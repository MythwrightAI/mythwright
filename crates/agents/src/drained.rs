//! The agents of a daemon that is draining to stop (`mw down --drain`):
//! what of theirs is still in hand, and what becomes of it where the
//! daemon stops before it is over.
//!
//! While it drains, a turn that no run asked for (somebody said
//! something to an agent in a channel) is not begun: it waits where it
//! is, queued, and the next daemon begins it. A turn a run under way
//! asks for is that run's work, and is begun as any is, since the drain
//! waits for what is under way to end.

use std::sync::atomic::Ordering;

use kernel::Result;
use serde::Serialize;

use crate::Agents;
use crate::turns::{TURN, read_turn};

/// A turn in hand, and what becomes of it where the daemon stops now.
#[derive(Clone, Debug, Serialize)]
pub struct InHand {
    pub turn: String,
    pub slot: String,
    /// What a stop does to it, in words.
    pub becomes: String,
}

/// What becomes of a turn in hand where the daemon stops now, as a
/// sentence: taken up where its program goes on without the daemon (its
/// slot `outlives` it, and the program is started by its wrapper), and
/// otherwise as whoever asked for it said (`turns::run`). One whose
/// program is on pipes (`piped`: it was handed something in no variable,
/// which only the daemon can keep out of what it says) ends with the
/// daemon whatever its slot says, and that is said.
pub fn becomes(outlives: bool, piped: bool, again: bool, restarts: u32, most: u32) -> String {
    let then = match again && restarts < most {
        true => "it is begun again by the next start",
        false => "it is left for a person: the next start does not begin it again",
    };
    match (outlives, piped) {
        (true, false) => "its program goes on, and the next start takes it up where it is".to_string(),
        (true, true) => format!(
            "its program ends with the daemon though its agent outlives it, since it was handed something only \
             the daemon can keep out of what it says; {then}"
        ),
        (false, _) => then.to_string(),
    }
}

impl Agents {
    /// Begins no turn that no run asked for, from here on.
    pub fn drain(&self) {
        self.inner.draining.store(true, Ordering::SeqCst);
    }

    /// Whether a turn nobody's run asked for waits ([`Agents::drain`]).
    pub(crate) fn draining(&self) -> bool {
        self.inner.draining.load(Ordering::SeqCst)
    }

    /// Every turn in hand (at work, or answered and not yet said to have
    /// ended), of every file that keeps turns, with what becomes of it
    /// where the daemon stops now.
    pub async fn in_hand_now(&self) -> Result<Vec<InHand>> {
        let asked = |connection: &mut rusqlite::Connection| {
            let mut statement = connection
                .prepare(&format!("SELECT {TURN} FROM turns WHERE status IN ('running', 'answered')"))?;
            let turns = statement.query_map([], read_turn)?;
            turns.collect::<rusqlite::Result<Vec<_>>>()
        };
        let mut all = Vec::new();
        for turns in self.of_each(self.every_turns(), asked).await? {
            for turn in turns {
                let outlives = self.inner.outliving.of(&turn.slot);
                let piped = self.inner.piped.lock().expect("agents lock").contains(&turn.id);
                let becomes = match turn.status.as_str() {
                    "answered" => {
                        "its answer is kept, and the next start says it where it was asked".to_string()
                    }
                    _ => becomes(outlives, piped, turn.again, turn.restarts, turn.most),
                };
                all.push(InHand { turn: turn.id, slot: turn.slot, becomes });
            }
        }
        Ok(all)
    }
}
