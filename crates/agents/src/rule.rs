//! The rule for choosing a sign-in: which of those a slot may draw a turn
//! is handed, or that it waits, or that it gives up.
//!
//! [`crate::signins`] is the mechanism: it knows what the factory holds,
//! who has which and what is set aside, and it holds, gives back and hands
//! over. It decides nothing. Before each turn of a slot that draws
//! sign-ins it asks here, with everything a choice could turn on
//! ([`Asked`]), and does what it is told ([`Decision`]), having checked
//! that what was chosen is one of the free sign-ins it offered.
//!
//! The rule is a blueprint, `std/signin`, so that it is its owner's to
//! change: a project that has a file of that name is asked its own. It is
//! asked and not run. It has `params` and `outputs` and nothing else, so
//! asking it starts nothing, takes no time worth measuring, and needs
//! nobody's authority: whatever it says, the most it can do is choose
//! among the sign-ins its slot may draw anyway, or make its slot wait.

use blueprint::template;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{Agents, Slot};

/// The blueprint that is the rule.
pub const RULE: &str = "std/signin";

/// A sign-in a slot may draw, as the rule is shown it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Offered {
    pub name: String,
    /// `key`, `token` or `folder`.
    pub kind: &'static str,
    /// Which of the slot's choices it comes under, the first being 0.
    pub choice: usize,
    /// How many turns may hold it at once, and how many do.
    pub at_once: u32,
    pub held: usize,
    /// Whether it can be taken now: held by fewer than may, and not set aside.
    pub free: bool,
    /// When it was last handed out, in milliseconds since 1970; 0 if never.
    pub last_used: i64,
    /// Set aside until then, if it is.
    pub aside_until: Option<i64>,
}

/// What the rule is asked: the `params` of the blueprint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Asked {
    /// The slot whose turn is about to start, and the package it runs on.
    pub slot: String,
    pub package: String,
    pub turn: String,
    /// The sign-in under which the conversation this turn would carry on
    /// was held, if there is one: a conversation is carried on only there.
    pub conversation: Option<String>,
    /// Now, in milliseconds since 1970.
    pub now: i64,
    /// Every sign-in the slot may draw; never empty.
    pub signins: Vec<Offered>,
}

/// Those of the offered that can be taken now, in the order the rule that
/// ships takes them: the earliest choice first, and of one choice whichever
/// was handed out longest ago. Equals stay in the order they were offered.
///
/// The rule is handed this beside everything it could choose by
/// (`params.in_turn`), so that the one that ships names the first and
/// works nothing out; a rule of somebody's own may go by it or not.
pub fn in_turn(signins: &[Offered]) -> Vec<Offered> {
    let mut free: Vec<Offered> = signins.iter().filter(|one| one.free).cloned().collect();
    free.sort_by_key(|one| (one.choice, one.last_used));
    free
}

/// What the rule says: the `outputs` of the blueprint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Hand the turn this one, by name (`take`).
    Take(String),
    /// Wait until one is given back or is good again, and ask again (`wait`).
    Wait,
    /// Fail the turn, with this sentence (`says`).
    GiveUp(String),
}

/// Works out what a rule says of what it is asked.
pub fn ask(source: &str, asked: &Asked) -> Result<Decision, String> {
    let checked = blueprint::check(source, &blueprint::NoCatalog);
    let wrong: Vec<String> = checked.errors().map(|problem| problem.message.clone()).collect();
    if !wrong.is_empty() {
        return Err(format!("does not check: {}", wrong.join("; ")));
    }
    let Some(rule) = checked.blueprint else { return Err("does not read as a blueprint".to_string()) };
    if !rule.flow.steps.is_empty() || !rule.triggers.is_empty() || !rule.uses.is_empty() {
        let why = "has steps, handlers or attachments. It is asked before every turn and never run, so it \
                   can have `params` and `outputs` and nothing that does anything";
        return Err(why.to_string());
    }
    // What it says of its own params stands where nothing is said here:
    // that is how a copy of it is given a setting of its owner's.
    let mut params: Map<String, Value> =
        rule.params.iter().filter_map(|param| Some((param.name.clone(), param.default.clone()?))).collect();
    if let Ok(Value::Object(given)) = serde_json::to_value(asked) {
        params.extend(given);
    }
    if let Ok(free) = serde_json::to_value(in_turn(&asked.signins)) {
        params.insert("in_turn".to_string(), free);
    }
    let mut scope = Map::new();
    scope.insert("params".to_string(), Value::Object(params));
    let gave = template::render(&Value::Object(rule.outputs.clone()), &scope)
        .map_err(|err| format!("could not be worked out: {err}"))?;
    Ok(match (&gave["take"], &gave["wait"]) {
        (Value::String(name), _) if !name.is_empty() => Decision::Take(name.clone()),
        (Value::Null | Value::String(_), Value::Bool(true)) => Decision::Wait,
        (Value::Null | Value::String(_), _) => Decision::GiveUp(
            gave["says"].as_str().filter(|says| !says.trim().is_empty()).map(str::to_string).unwrap_or_else(
                || {
                    format!(
                        "says that `{}` is to be handed no sign-in, and is not to wait for one",
                        asked.slot
                    )
                },
            ),
        ),
        (other, _) => {
            return Err(format!("gave `take` as {other}, which is not the name of a sign-in"));
        }
    })
}

impl Agents {
    /// Asks the rule that holds for `slot`: its project's own, if the
    /// project has one, and the one that ships otherwise.
    pub(crate) fn decide(&self, slot: &Slot, asked: &Asked) -> Decision {
        let library = self.inner.library.get().and_then(std::sync::Weak::upgrade);
        let source = library.and_then(|library| library.load(RULE, slot.project.as_deref()));
        let of = match &slot.project {
            Some(project) => format!("`{RULE}` (as the project `{project}` sees it)"),
            None => format!("`{RULE}`"),
        };
        let Some(source) = source else {
            return Decision::GiveUp(format!(
                "there is no blueprint {of} to say which sign-in `{}` is handed",
                slot.name
            ));
        };
        match ask(&source, asked) {
            Ok(Decision::GiveUp(why)) if why.starts_with("says that") => {
                Decision::GiveUp(format!("the rule for choosing a sign-in, {of}, {why}"))
            }
            Ok(decision) => decision,
            Err(why) => Decision::GiveUp(format!("the rule for choosing a sign-in, {of}, {why}")),
        }
    }
}
