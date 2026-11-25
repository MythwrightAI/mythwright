//! Asking the same again.
//!
//! What asks an agent for work can be taken away by a restart while the
//! turn it asked for is still there: waiting, in hand, or ended with its
//! answer. Whatever asks again is then given that turn, and the agent is
//! not asked a second time. There are two ways of knowing it is the same
//! asking.
//!
//! - **It was in hand when the daemon stopped.** A turn an earlier daemon
//!   left in hand is given to the first that asks exactly the same of the
//!   same agent in the same conversation, once. Nothing has to be said
//!   for this, and nothing is known beyond it: a turn that had ended is
//!   not found, since the same words asked again are as likely a new
//!   asking (a card sent back twice with nothing said).
//! - **The asker says which asking it is** (`asking` of `agent.task`): a
//!   word of its own that it would say again for this asking and for no
//!   other, such as the event that set it off. The turn that was asked
//!   for in that word by the same asker is this asking's, whatever has
//!   become of it: still waiting, in hand, or ended, well or not. The
//!   asker is whoever applied the instance, under the key it is known
//!   by: an instance that keeps nothing is another one when it is begun
//!   again, and is known by the same key. Nobody is handed an answer
//!   they did not ask for by saying somebody else's word.
//!
//! Either way what is asked has to be the same: the agent, the
//! conversation, the words, where it is done and how it may end.
//!
//! A turn left in hand is begun again as the daemon starts, before
//! anybody asks for it, so that the work goes on. Whatever was to ask
//! for it again may come to an end without asking (its card was moved
//! away meanwhile, and it found nothing to do): the turn is then nobody's.
//! Whatever started that asker lets go of it once the asker is over
//! ([`Agents::let_go_under`], asked as `agent.let_go`), and a turn left
//! in hand, or waiting in a queue (which is begun as the daemon starts
//! just as well), that nobody has asked for since this daemon began, asked by
//! that asker or anything it started, is called off and its program
//! ended. One somebody asked for again is theirs, and is called off by
//! their step being stopped, as any turn is.

use journal::optional;
use rusqlite::params;

use crate::filed::NTH;
use crate::{Agents, TurnInfo, TurnRequest};

/// Whether `turn` was asked exactly what `like` asks.
fn same(turn: &TurnInfo, like: &TurnRequest) -> bool {
    turn.slot == like.slot
        && turn.thread == like.thread
        && turn.kind == like.kind
        && turn.prompt == like.prompt
        && turn.cwd == like.cwd
        && turn.options == like.options
}

/// What has become of a turn an earlier daemon left in hand.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Left {
    /// Nobody has asked for it since this daemon began.
    Open,
    /// It was waiting in its agent's queue, asked for in a word of its
    /// asker's own, and nobody has asked for it since this daemon began:
    /// it is found again by that word alone, and is begun as any turn in
    /// a queue is.
    Waiting,
    /// Somebody asked for it again, and has it.
    Claimed,
    /// What was to ask for it again came to an end without asking, and
    /// it was called off ([`Agents::let_go_under`]).
    LetGo,
}

impl Agents {
    /// The turn that is already this asking's, for whoever asks the same
    /// again: the one the asker's own word names, as it is by now, or
    /// else one an earlier daemon left in hand that nobody has asked for
    /// since this daemon began. `None` is a new asking.
    pub(crate) async fn claim(&self, like: &TurnRequest) -> Option<String> {
        if let Some(id) = self.asked_before(like).await {
            // It is somebody's again, where it was left in hand.
            self.claimed(&id);
            return Some(id);
        }
        let open: Vec<String> = {
            let left = self.inner.left_in_hand.lock().expect("agents lock");
            left.iter().filter(|(_, left)| **left == Left::Open).map(|(id, _)| id.clone()).collect()
        };
        for id in open {
            let Ok(turn) = self.turn(&id).await else { continue };
            // One asked for in another word is another asking, whatever
            // its words; one asked for in none is told by its words alone.
            let another = match (self.asking_of(&id).await, &like.asking) {
                (Some(was), Some(now)) => was != *now,
                _ => false,
            };
            if same(&turn, like) && !another && self.claimed(&id) {
                return Some(id);
            }
        }
        None
    }

    /// The last turn this asker asked for in this word, where it asked
    /// exactly this.
    async fn asked_before(&self, like: &TurnRequest) -> Option<String> {
        let (word, by) = (like.asking.clone()?, like.asked_by.clone()?);
        let (slot, thread, kind) = (like.slot.clone(), like.thread.clone(), like.kind.clone());
        let held = self.may_hold(&slot);
        let asked = move |connection: &mut rusqlite::Connection| {
            optional(connection.query_row(
                &format!(
                    "SELECT id, {NTH} FROM turns
                     WHERE thread = ?1 AND slot = ?2 AND kind = ?3 AND asking = ?4 AND asked_by = ?5
                     ORDER BY rowid DESC LIMIT 1"
                ),
                params![thread, slot, kind, word, by],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            ))
        };
        let newest = self.of_each(held, asked).await.ok()?;
        // One asked of an agent that was taken away was not asked of
        // whoever is called that now.
        let since = self.since(&like.slot).await;
        let found = newest.into_iter().flatten().filter(|(_, nth)| *nth > since);
        let found = found.max_by_key(|(_, nth)| *nth).map(|(id, _)| id);
        let turn = self.turn(&found?).await.ok()?;
        same(&turn, like).then_some(turn.id)
    }

    /// The word a turn was asked for in, where it was asked for in one.
    async fn asking_of(&self, id: &str) -> Option<String> {
        let lookup = id.to_string();
        let asked = move |connection: &mut rusqlite::Connection| {
            optional(connection.query_row("SELECT asking FROM turns WHERE id = ?1", [&lookup], |row| {
                row.get::<_, Option<String>>(0)
            }))
        };
        let word = self.of_each(self.may_keep(id), asked).await.ok()?;
        word.into_iter().flatten().next().flatten()
    }

    /// Says that somebody has asked for a turn that was left in hand:
    /// whether it was theirs to take.
    pub(crate) fn claimed(&self, id: &str) -> bool {
        self.taken_as(id, Left::Claimed)
    }

    /// Takes a turn left in hand, or waiting, that nobody has asked for,
    /// as `as_`: whether it was there to take.
    fn taken_as(&self, id: &str, as_: Left) -> bool {
        match self.inner.left_in_hand.lock().expect("agents lock").get_mut(id) {
            Some(left) if matches!(*left, Left::Open | Left::Waiting) => {
                *left = as_;
                true
            }
            _ => false,
        }
    }

    /// Whether a turn left in hand was let go of ([`Agents::let_go_under`]).
    pub(crate) fn was_let_go(&self, id: &str) -> bool {
        self.inner.left_in_hand.lock().expect("agents lock").get(id) == Some(&Left::LetGo)
    }

    /// Calls off each turn an earlier daemon left in hand or waiting, that
    /// nobody has asked for since, which `asker` asked for or anything it
    /// started:
    /// `asker` is over, and will not ask for it again. Who asked is as a
    /// turn keeps it (`asked_by` of `agent.task`): whoever applied it, the
    /// instance that keeps what it does above it, and the names of those
    /// under that which keep nothing. What is called off is said as any
    /// turn called off is, and its program is ended; one not yet begun by
    /// this daemon is left as it is begun (`turns::run`).
    pub(crate) async fn let_go_under(&self, asker: &str) -> Vec<String> {
        let open: Vec<String> = {
            let left = self.inner.left_in_hand.lock().expect("agents lock");
            let open = |left: &Left| matches!(left, Left::Open | Left::Waiting);
            left.iter().filter(|(_, left)| open(left)).map(|(id, _)| id.clone()).collect()
        };
        let mut called_off = Vec::new();
        for id in open {
            let Ok(turn) = self.turn(&id).await else { continue };
            let Some(by) = turn.asked_by.as_deref() else { continue };
            let under = by == asker || by.strip_prefix(asker).is_some_and(|rest| rest.starts_with('/'));
            if !under || turn.is_finished() || !self.taken_as(&id, Left::LetGo) {
                continue;
            }
            // Begun by this daemon already, it is stopped where it is; and
            // where it is not yet, it is left as it is begun.
            let at_work = self.inner.running.lock().expect("agents lock").get(&id).cloned();
            if let Some(cancel) = at_work {
                cancel.cancel();
            }
            called_off.push(id);
        }
        called_off
    }
}
