//! What a bot with no model behind it says back when it is spoken to.
//!
//! `std/bot` is something in a channel that answers a message that names
//! it: with what a command of its owner's prints, or with what was said
//! where it has no command. The blueprint runs the command and posts the
//! answer. What the answer is, of the few things it can be, is [`answer`]:
//! a function of what was said and what became of the command, which the
//! blueprint asks as `channel.answer`.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use kernel::BoxFut;
use serde_json::{Value, json};

blueprint::came! {
    /// What a bot's answer is.
    pub enum Came {
        /// It has no command: what was said, said back.
        Echo => "echo",
        /// What its command printed.
        Printed => "printed",
        /// Its command went well and printed nothing but space.
        Nothing => "nothing",
        /// Its command failed.
        Failed => "failed",
    }
}

/// What became of the command a bot runs to answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ran<'a> {
    /// There is none, and none was run.
    Not,
    /// It went well, and printed this.
    Well(&'a str),
    /// It failed, with this said of why where anything was.
    Badly(&'a str),
}

/// A bot's answer: which of the things it can be, and the words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub came: Came,
    pub says: String,
}

/// What a bot says back to `said`, its command having `ran` so.
pub fn answer(said: &str, ran: Ran<'_>) -> Answer {
    let (came, says) = match ran {
        Ran::Not => (Came::Echo, format!("You said: {said}")),
        Ran::Well(printed) if printed.trim().is_empty() => {
            (Came::Nothing, "(it printed nothing)".to_string())
        }
        // As it was printed, with the space about it: a command's lines are its own.
        Ran::Well(printed) => (Came::Printed, printed.to_string()),
        Ran::Badly("") => (Came::Failed, "I could not answer: the command failed".to_string()),
        Ran::Badly(why) => (Came::Failed, format!("I could not answer: {why}")),
    };
    Answer { came, says }
}

pub fn register(registry: &mut Registry) {
    let spec = ActionSpec::new(
        "channel.answer",
        "Say what a bot with a command answers: what the command printed, that it printed nothing, why it could \
         not, or what was said where there is no command.",
    )
    .arg("said", T::String, "what was said to it")
    .required("skipped", T::Bool, "whether no command was run, there being none")
    .arg("ok", T::Bool, "whether the command went well, where one was run")
    .arg("printed", T::String, "what the command printed")
    .arg("error", T::String, "why the command failed, where it did and anything was said of why")
    .output("came", &format!("{}: no command; what it printed; nothing but space; or it failed", Came::words()))
    .output("says", "the words of the answer")
    .idempotent();
    registry.add_fn(spec, |_cx, args| -> BoxFut<'static, conductor::ActionResult> {
        Box::pin(async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default();
            let flag = |key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
            let ran = match (flag("skipped"), flag("ok")) {
                (true, false) => Ran::Not,
                (false, true) => Ran::Well(text("printed")),
                (false, false) => Ran::Badly(text("error")),
                (true, true) => {
                    return Err(ActionError::new(
                        "`skipped` and `ok` are both so: a command that was not run did not go well",
                    ));
                }
            };
            let answer = answer(text("said"), ran);
            Ok(json!({ "came": answer.came.word(), "says": answer.says }))
        })
    });
}
