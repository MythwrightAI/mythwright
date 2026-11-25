//! The rule for who work goes to, of those that could take it.
//!
//! `agents::picking` is the mechanism: it knows who is in a role, who has
//! what in hand and who was given what, and it puts those who could take a
//! piece of work in the order the asker's word says (`pick`). It decides
//! nothing beyond that order. Where work is asked of a role and nobody is
//! named, it asks here, with everything a choice could turn on
//! (`Asked`), and gives the work to whoever it is told, having checked
//! that the one chosen is one of those it offered.
//!
//! The rule is a blueprint, `std/dispatch`, so that it is its owner's to
//! change: a project that has a file of that name is asked its own (as it
//! was vouched for, like any text an instance runs). It is asked and not
//! run. It has `params` and `outputs` and nothing else, so asking it
//! starts nothing and takes no time worth measuring. Whatever it says, the
//! most it can do is choose among the agents that could have been given
//! the work anyway.
//!
//! It uses no agent: what the rule is shown is plain values, and the
//! agents' crate offers it as `agents::dispatch`, making each `Offered`
//! from one of those who could take the work.

use blueprint::template;
use serde::Serialize;
use serde_json::{Map, Value};

/// The blueprint that is the rule.
pub const RULE: &str = "std/dispatch";

/// One who could take the work, as the rule is shown them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Offered {
    pub name: String,
    pub role: String,
    pub rank: i64,
    pub model: String,
    pub adapter: String,
    /// How many turns it has in hand, how many it was ever given, and
    /// which was the last of them (0 for none).
    pub in_hand: usize,
    pub given: i64,
    pub last: i64,
    /// Whether a conversation beside this one is held with it.
    pub apart: bool,
}

/// What the rule is asked: the `params` of the blueprint.
#[derive(Clone, Debug, Serialize)]
pub struct Asked {
    pub project: String,
    pub role: String,
    /// The asker's word for the order (`pick`): `agents` is in it.
    pub picks: String,
    /// What the asker says the work is: a card, its title, its labels.
    pub about: Value,
    /// Everyone who could take it, the one the order puts first first; never empty.
    pub agents: Vec<Offered>,
}

/// Who a rule says the work goes to: the name it gives as `take`, where
/// that is one of those offered. A rule that names nobody, or somebody who
/// was not offered, has said nothing, and is told so by `Err`.
pub fn ask(source: &str, asked: &Asked) -> Result<Option<String>, String> {
    let checked = blueprint::check(source, &blueprint::NoCatalog);
    let wrong: Vec<String> = checked.errors().map(|problem| problem.message.clone()).collect();
    if !wrong.is_empty() {
        return Err(format!("does not check: {}", wrong.join("; ")));
    }
    let Some(rule) = checked.blueprint else { return Err("does not read as a blueprint".to_string()) };
    if !rule.flow.steps.is_empty() || !rule.triggers.is_empty() || !rule.uses.is_empty() {
        let why = "has steps, handlers or attachments. It is asked each time work is given and never run, so it \
                   can have `params` and `outputs` and nothing that does anything";
        return Err(why.to_string());
    }
    // What it says of its own params stands where nothing is said here.
    let mut params: Map<String, Value> =
        rule.params.iter().filter_map(|param| Some((param.name.clone(), param.default.clone()?))).collect();
    if let Ok(Value::Object(given)) = serde_json::to_value(asked) {
        params.extend(given);
    }
    let mut scope = Map::new();
    scope.insert("params".to_string(), Value::Object(params));
    let gave = template::render(&Value::Object(rule.outputs.clone()), &scope)
        .map_err(|err| format!("could not be worked out: {err}"))?;
    match &gave["take"] {
        Value::Null => Ok(None),
        Value::String(name) if name.is_empty() => Ok(None),
        Value::String(name) if asked.agents.iter().any(|one| one.name == *name) => Ok(Some(name.clone())),
        Value::String(name) => {
            Err(format!("gave the work to `{name}`, who is not one of those who could take it"))
        }
        other => Err(format!("gave `take` as {other}, which is not the name of an agent")),
    }
}
