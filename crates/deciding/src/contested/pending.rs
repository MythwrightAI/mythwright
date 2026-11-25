//! Which of a card's contests is to be done now.
//!
//! A card may have several contests, done one after another, the oldest
//! first, a round each. A contest under way is taken up before anything
//! else (the daemon may have stopped in the middle of it), and else the
//! oldest still waiting is next. One that came to something, or was left
//! for a person, is done.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use super::{said, text};
use crate::add;

/// What a contest's row says of how it stands: waiting, under way, or done.
pub const WAITING: &[&str] = &["", "waiting"];
pub const UNDER_WAY: &[&str] = &["working", "judging", "asking"];
pub const DONE: &[&str] = &["came", "left"];

/// A contest as its row says it: what it is called, the card it is
/// about and how it stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contest {
    pub id: String,
    pub card: String,
    pub state: String,
}

/// What is to be done now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pending {
    /// `next`; `done`, where nothing is left to do and the newest came to a
    /// winner; or `none`.
    pub came: &'static str,
    /// The contest to do, by its place among those handed.
    pub at: Option<usize>,
    /// How many of the card's wait after it.
    pub waiting: usize,
}

/// Every word [`pending`] can answer.
pub const PENDING_WORDS: &[&str] = &["next", "done", "none"];

/// Of `contests`, in the order they were added, which of `card`'s is to
/// be done now: one under way, else the oldest waiting. Another card's are
/// passed by; a state that is none of those a contest can be in is
/// refused, saying which.
pub fn pending(card: &str, contests: &[Contest]) -> Result<Pending, String> {
    let mut under_way = None;
    let mut waiting = Vec::new();
    for (at, one) in contests.iter().enumerate().filter(|(_, one)| one.card == card) {
        let state = one.state.as_str();
        if UNDER_WAY.contains(&state) {
            under_way = under_way.or(Some(at));
        } else if WAITING.contains(&state) {
            waiting.push(at);
        } else if !DONE.contains(&state) {
            return Err(format!(
                "the contest `{}` says it is `{state}`, which a contest is not: it is waiting, working, \
                 judging, asking, came or left",
                one.id
            ));
        }
    }
    let at = under_way.or_else(|| waiting.first().copied());
    let left = waiting.iter().filter(|other| Some(**other) != at).count();
    if at.is_some() {
        return Ok(Pending { came: "next", at, waiting: left });
    }
    let newest = contests.iter().enumerate().rfind(|(_, one)| one.card == card);
    match newest {
        Some((at, one)) if one.state == "came" => Ok(Pending { came: "done", at: Some(at), waiting: 0 }),
        _ => Ok(Pending { came: "none", at: None, waiting: 0 }),
    }
}

/// Adds `contest.pending` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "contest.pending",
            "Say which of an item's contests is to be done now: one under way, else the oldest waiting, or none.",
        )
        .deciding()
        .required("item", T::String, "the item the contests are about")
        .required("contests", T::List, "the contests' rows as the table gives them, the oldest first: each with `id`, the item it is about (`card`) and its `state`")
        .output("came", "`next`; `done`, where nothing is left to do and the newest came to a winner; or `none`")
        .output("ends", "where it is `done`, `on`: the winner's work is the item's already")
        .output("agent", "where it is `done`, the winner's agent, as the contest's row says")
        .output("says", "where it is `done`, a sentence that says so")
        .output("contest", "the row of the contest to do now, whole")
        .output("id", "its id")
        .output("waiting", "how many of the item's wait after it")
        .idempotent(),
        |_, args| async move {
            let rows = args.get("contests").and_then(Value::as_array).cloned().unwrap_or_default();
            let contests: Vec<Contest> = rows
                .iter()
                .map(|row| Contest { id: text(row, "id"), card: text(row, "card"), state: text(row, "state") })
                .collect();
            let came = pending(&said(&args, "item"), &contests).map_err(ActionError::new)?;
            let row = came.at.map(|at| rows[at].clone()).unwrap_or(Value::Null);
            let id = came.at.map(|at| contests[at].id.clone()).unwrap_or_default();
            // Nothing where it is not done, so that what reads it goes on to what it says otherwise.
            let (ends, agent, says) = match came.came == "done" {
                true => (
                    json!("on"),
                    json!(text(&row, "agent")),
                    json!(format!("its A/B test came to {} already, and that work is the card's", text(&row, "winner"))),
                ),
                false => (Value::Null, Value::Null, Value::Null),
            };
            Ok(json!({
                "came": came.came, "contest": row, "id": id, "waiting": came.waiting, "ends": ends, "agent": agent,
                "says": says,
            }))
        },
    );
}
