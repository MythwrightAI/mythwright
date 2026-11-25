//! What the language's own actions do, said with what each is handed.
//!
//! An action that comes from outside says what it does in one sentence
//! for every step that asks it. The actions that are part of the language
//! ([`crate::catalog::intrinsics`]) are asked for most of all, and one
//! sentence for all of their steps says little: that an event is
//! published, where a person wants to know which. So the language says
//! its own by what the step hands them. One it has no words for here is
//! said by its sentence, as any action is.

use serde_json::Value;

use super::words::{joined, lasting, value};
use crate::template::HANDED_ON;

/// What a step that asks one of the language's own actions does, in a
/// sentence; nothing for an action that is not one of them, or that is
/// said well enough by its own sentence.
pub(crate) fn own(action: &str, with: &Value) -> Option<String> {
    let given = |name: &str| with.get(name).filter(|value| !value.is_null());
    Some(match action {
        "emit" => match given("topic") {
            Some(topic) => format!("Say {}.", value(topic)),
            None => "Say an event.".to_string(),
        },
        "set" => {
            let keys: Vec<String> = with
                .as_object()
                .into_iter()
                .flat_map(|given| given.keys())
                .filter(|key| *key != HANDED_ON)
                .map(|key| format!("`{key}`"))
                .collect();
            match keys.as_slice() {
                [] => "Change what it keeps.".to_string(),
                keys => format!("Keep {}.", joined(keys, "and")),
            }
        }
        "keep" => format!("Keep an entry in its list {}.", value(given("in")?)),
        "drop" => format!("Take an entry out of its list {}.", value(given("from")?)),
        "measure" => format!("Measure {}.", value(given("name")?)),
        "wait" => {
            let most =
                given("timeout").map(|most| format!(", for {} at most", lasting(most))).unwrap_or_default();
            let asks = given("say")
                .and_then(|say| say.get("topic"))
                .map(|topic| format!("Say {}, and ", value(topic)));
            match (given("event"), given("for"), asks) {
                (Some(event), _, Some(asks)) => format!("{asks}wait until {} is said{most}.", value(event)),
                (Some(event), _, None) => format!("Wait until {} is said{most}.", value(event)),
                (None, Some(long), _) => format!("Wait {}.", lasting(long)),
                _ => return None,
            }
        }
        "end" => match given("status").and_then(Value::as_str) {
            Some("failed") => "End this run here, as failed.".to_string(),
            _ => "End this run here.".to_string(),
        },
        "fail" => "Fail here, on purpose.".to_string(),
        "noop" => "Do nothing.".to_string(),
        "log" => "Write a line in the run's log.".to_string(),
        "stop" => match given("instance") {
            Some(instance) => format!("Stop {} and everything it started.", value(instance)),
            None => "Stop this instance and everything it started.".to_string(),
        },
        _ => return None,
    })
}
