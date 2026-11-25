//! What is kept of something that is put to an agent a number of times at
//! most.
//!
//! Something that goes round between an agent and whoever answers it (a
//! plan that is asked about, sent back and said again) is put to one agent
//! each time, and a loop between an agent and anybody always has a number
//! in it. Whatever keeps the rounds keeps how many there were and who has
//! had them, and has to know four things more each time: who the next is
//! put to, as what that agent acts (so that what it does itself is not
//! taken for an answer), whether there may be another, and what to call
//! whoever has it in a sentence. Those are [`round`], a function of what
//! is kept and what was said of who it is put to. A blueprint asks it as
//! `agent.round` and keeps what it is told.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use crate::add;

/// What is kept of the rounds there have been.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round {
    /// How many there have been.
    pub rounds: i64,
    /// Who has had them: whoever took the last, where it was taken.
    pub agent: String,
    /// Who the next is put to: whoever has had them, or else whoever was named.
    pub next: String,
    /// The actor that agent is, as an event says who did a thing.
    pub its: String,
    /// Whether there may be another.
    pub open: bool,
    /// What whoever has it is called in a sentence.
    pub whom: String,
}

/// What is kept once `had` rounds are one more (`counts`), or as they are;
/// `most` is how many there may be. `took` is who took the round there
/// just was, nothing where it did not come to an end, and the empty text
/// where there was nobody to take it; `has` is who had them until now, and
/// `named`, who they were to be put to where nobody has had one.
pub fn round(
    had: i64,
    most: i64,
    counts: bool,
    took: Option<&str>,
    has: &str,
    named: &str,
) -> Result<Round, String> {
    for (name, many) in [("had", had), ("most", most)] {
        if many < 0 {
            return Err(format!("`{name}` is a number of rounds, and is not less than none; it is {many}"));
        }
    }
    let rounds = had + i64::from(counts);
    let agent = took.unwrap_or(has);
    Ok(Round {
        rounds,
        agent: agent.to_string(),
        next: if agent.is_empty() { named } else { agent }.to_string(),
        its: format!("agent:{agent}"),
        open: rounds < most,
        whom: if agent.is_empty() { "its agent" } else { agent }.to_string(),
    })
}

blueprint::came! {
    /// What a change to something an agent has in hand is to that agent.
    pub enum News {
        /// Nothing it is to be told of.
        None => "none",
        /// What the thing asks was changed.
        Changed => "changed",
        /// It was put back where its agent has it.
        Back => "back",
    }
}

/// A change to something an agent has in hand, as whoever keeps its
/// rounds sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change<'a> {
    /// Whether it may be put to its agent again, and whether it is past
    /// being put to anyone (accepted, say).
    pub open: bool,
    pub settled: bool,
    /// Who made the change, and the actor its own agent is.
    pub by: &'a str,
    pub its: &'a str,
    /// How many rounds there have been.
    pub rounds: i64,
    /// Whether what it asks was changed; whether where it is was; and
    /// whether it is now where its agent has it.
    pub reworded: bool,
    pub moved: bool,
    pub here: bool,
}

/// Whether a change is news to the agent that has the thing in hand.
///
/// Nothing is, of something settled, or that may not be put to its agent
/// again, or that its agent changed itself. Otherwise it is told where
/// what the thing asks was changed, and where the thing was put back where
/// its agent has it, having been there before; and a change that moved it
/// is told as its having come back.
pub fn news(change: Change<'_>) -> News {
    let Change { open, settled, by, its, rounds, reworded, moved, here } = change;
    if settled || !open || by == its {
        return News::None;
    }
    match (reworded || (moved && here && rounds > 0), moved) {
        (false, _) => News::None,
        (true, true) => News::Back,
        (true, false) => News::Changed,
    }
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "agent.round",
            "Say what is kept of something put to an agent a number of times at most: how many rounds there \
             have been, who has it, and whether there may be another.",
        )
        .deciding()
        .arg("had", T::Int, "how many rounds there were before (default 0)")
        .required("most", T::Int, "how many there may be")
        .arg("counts", T::Bool, "whether there has just been one more (default true)")
        .arg("took", T::String, "who took the round there just was; nothing where it came to no end")
        .arg("has", T::String, "who had the rounds until now")
        .arg("named", T::String, "who they were to be put to, where nobody has had one")
        .output("rounds", "how many there have been")
        .output("agent", "who has had them: whoever took the last, where it was taken")
        .output("next", "who the next is put to: whoever has had them, or else whoever was named")
        .output("its", "the actor that agent is (`agent:<name>`), as an event says who did a thing")
        .output("open", "whether there may be another")
        .output("whom", "what whoever has it is called in a sentence: its name, or `its agent`")
        .idempotent(),
        |_, args| async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str);
            let number = |key: &str| match args.get(key) {
                None | Some(Value::Null) => Ok(0),
                Some(said) => said.as_i64().ok_or_else(|| {
                    ActionError::new(format!("`{key}` is a whole number, and was given {said}"))
                }),
            };
            let counts = match args.get("counts") {
                None | Some(Value::Null) => true,
                Some(Value::Bool(counts)) => *counts,
                Some(said) => {
                    return Err(ActionError::new(format!("`counts` is true or false, and was given {said}")));
                }
            };
            let kept = round(
                number("had")?,
                number("most")?,
                counts,
                text("took"),
                text("has").unwrap_or_default(),
                text("named").unwrap_or_default(),
            )
            .map_err(ActionError::new)?;
            Ok(json!({
                "rounds": kept.rounds, "agent": kept.agent, "next": kept.next,
                "its": kept.its, "open": kept.open, "whom": kept.whom,
            }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "agent.news",
            "Say whether a change to something an agent has in hand is news to that agent, and as what.",
        )
        .deciding()
        .arg("open", T::Bool, "whether it may be put to its agent again (default true)")
        .arg("settled", T::Bool, "whether it is past being put to anyone (default false)")
        .arg("by", T::String, "who made the change, as the event says (`source.actor`)")
        .arg("its", T::String, "the actor its own agent is (`its` of `agent.round`)")
        .arg("rounds", T::Int, "how many rounds there have been (default 0)")
        .arg("reworded", T::Bool, "whether what it asks was changed")
        .arg("moved", T::Bool, "whether where it is was changed")
        .arg("here", T::Bool, "whether it is now where its agent has it")
        .output(
            "came",
            &format!(
                "{}: nothing to tell; what it asks was changed; or it was put back where its agent has it",
                News::words()
            ),
        )
        .idempotent(),
        |_, args| async move {
            let flag = |key: &str, otherwise: bool| match args.get(key) {
                None | Some(Value::Null) => Ok(otherwise),
                Some(Value::Bool(said)) => Ok(*said),
                Some(said) => {
                    Err(ActionError::new(format!("`{key}` is true or false, and was given {said}")))
                }
            };
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default();
            let rounds = match args.get("rounds") {
                None | Some(Value::Null) => 0,
                Some(said) => said.as_i64().ok_or_else(|| {
                    ActionError::new(format!("`rounds` is a whole number, and was given {said}"))
                })?,
            };
            let came = news(Change {
                open: flag("open", true)?,
                settled: flag("settled", false)?,
                by: text("by"),
                its: text("its"),
                rounds,
                reworded: flag("reworded", false)?,
                moved: flag("moved", false)?,
                here: flag("here", false)?,
            });
            Ok(json!({ "came": came.word() }))
        },
    );
}
