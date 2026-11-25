//! One set of acts played against a sandbox, as it is played against
//! others beside it: what a side by side of several texts does in each.
//!
//! Each text has a sandbox of its own and nothing is shared between
//! them but the acts. The acts are what one sandbox takes, beside one
//! thing that only each sandbox can say: a stand-in answered with what
//! a stand-in for its action usually answers. An answer for a stand-in
//! that does not wait in this sandbox (its text asks other things) is
//! passed over, and said; an act this sandbox refuses is said, and
//! nothing after it is done here.

use kernel::{Error, Result};
use serde_json::{Value, json};

use crate::{Act, Sandbox};

/// Something done to each sandbox of a side by side.
#[derive(Clone, Debug)]
pub enum Done {
    /// An act, as a sandbox takes it.
    Act(Act),
    /// The stand-in asked so many things in, answered as usual.
    Usual(u64),
}

impl Done {
    /// What is done, as it is sent: an act, or `{ "answer": { asked, usual: true } }`.
    pub fn read(said: &Value) -> Result<Done> {
        let answer = &said["answer"];
        if answer.get("usual") == Some(&Value::Bool(true)) {
            let asked = answer.get("asked").and_then(Value::as_u64);
            return asked.map(Done::Usual).ok_or_else(|| {
                Error::invalid("an answer given as usual says which stand-in it is for (`asked`)")
            });
        }
        serde_json::from_value(said.clone())
            .map(Done::Act)
            .map_err(|err| Error::invalid(format!("`{said}` is not something done to a sandbox ({err})")))
    }
}

/// What came of playing in one sandbox, beside what the sandbox hands.
#[derive(Debug, Default)]
pub struct Played {
    /// Why the playing ended early: an act refused, in the daemon's words.
    pub refused: Option<String>,
    /// Each act that was passed over here, with why.
    pub skipped: Vec<String>,
}

impl Played {
    pub fn said(&self) -> Value {
        json!({ "refused": self.refused, "skipped": self.skipped })
    }
}

/// The stand-ins of a sandbox that wait, by which asking each was, with
/// what a stand-in for its action usually answers.
async fn waiting(sandbox: &Sandbox) -> Result<Vec<(u64, Value)>> {
    let stands = sandbox.stands().await?;
    let waits = stands.get("waiting").and_then(Value::as_array).cloned().unwrap_or_default();
    Ok(waits
        .iter()
        .filter_map(|one| Some((one.get("id")?.as_u64()?, one.get("usual").cloned().unwrap_or(json!({})))))
        .collect())
}

/// Plays `done` in `sandbox`, in order.
pub async fn play(sandbox: &Sandbox, done: &[Done]) -> Played {
    let mut played = Played::default();
    for (at, one) in done.iter().enumerate() {
        let mut act = match one {
            Done::Act(act) => act.clone(),
            Done::Usual(asked) => Act::Answer { asked: *asked, gives: None, fails: None },
        };
        if let Act::Answer { asked, gives, fails } = &mut act {
            let waits = match waiting(sandbox).await {
                Ok(waits) => waits,
                Err(why) => {
                    played.refused = Some(why.to_string());
                    return played;
                }
            };
            let Some((_, usual)) = waits.into_iter().find(|(id, _)| id == asked) else {
                played.skipped.push(format!(
                    "The answer for the stand-in asked {} in order was not given here: no stand-in asked so \
                     waits in this text's sandbox.",
                    nth(*asked)
                ));
                continue;
            };
            if matches!(one, Done::Usual(_)) {
                (*gives, *fails) = (Some(usual), None);
            }
        }
        if let Err(why) = sandbox.act(act).await {
            played.refused = Some(format!(
                "What was done {} in order was refused here, and what was to be done after it was not done: \
                 {why}",
                nth(at as u64 + 1)
            ));
            return played;
        }
    }
    played
}

/// `1st`, `2nd`, `3rd`, `4th`.
pub fn nth(count: u64) -> String {
    let ending = match (count % 10, count % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{count}{ending}")
}
