//! Where a turn is kept, and which turn of all it is.
//!
//! A turn is its project's (`crate::apart`), and is kept in the file of
//! its project's own journal where that journal was made to keep its
//! agents' turns (`journal::keeps::TURNS`), with what is said of it: one
//! write, by that project's thread. A turn of an agent that is no
//! project's, of a project of a factory from before, and of one whose
//! journal was made keeping less, is in the factory's own. So the turns
//! of a factory are in several files, and everything here that reads or
//! writes one asks which:
//!
//! - **a turn in hand** is in the file this daemon queued it in or found
//!   it in, which is that of the project its row says
//!   ([`Agents::keeps`]);
//! - **a turn by its id** is looked for where this daemon queued or found
//!   it, and otherwise in each file ([`Agents::keeping`]): an id is a
//!   name, and says nothing of where its turn is;
//! - **an agent's turns** are in the files known to hold one of its
//!   ([`Agents::may_hold`]): the one its project's journal keeps, and
//!   whichever kept them when it was another project's, or nobody's;
//! - **a conversation's turns** may be anybody's, and are asked of every
//!   file ([`Agents::every_turns`]).
//!
//! **Which turn of all it is** ([`NTH`]) is what puts turns of different
//! files in order: where a row comes in its file says nothing of a row
//! in another, and neither does when it was written, to the millisecond.
//! Every turn is given a number as it is queued, from one count for the
//! whole factory ([`Filed::next`]), in the write that queues it and on
//! the thread that writes its file: so within a file the numbers go as
//! the rows do, and the newest row of a file is its highest number,
//! which the index finds without reading a turn. The count only goes up.
//! It is not kept anywhere but in the turns: a start reads the highest
//! number each file holds and goes on above them all, which is also
//! what a factory started from a copy does, so no number a turn has is
//! given again. A number given to a turn that was never kept (its write
//! failed, or the daemon went) may be: no turn has it. A turn from
//! before says no number and is ordered by where it came in the
//! factory's file, which is the one file such turns are in, below any
//! number given since. Nothing counts turns by it.
//!
//! The journal's own giver of numbers is not asked for it. An event's
//! number is drawn a run at a time by each journal, so of two events of
//! two journals the later may have the lesser: it says no other event
//! has it, and nothing of order across files, which is all this number
//! is for.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, Ordering};

use journal::{Journal, keeps};
use kernel::Result;
use rusqlite::Connection;

use crate::{Agents, TurnInfo};

/// Which turn of all a row is, as the database is asked it: the number
/// it was given, and for one from before where it came in its file.
pub(crate) const NTH: &str = "coalesce(nth, rowid)";

/// How many turns each agent was given, and which was the last.
pub(crate) type Given = HashMap<String, (i64, i64)>;

/// What is known in memory of where turns are kept, and the count their
/// numbers are given from.
#[derive(Default)]
pub(crate) struct Filed {
    /// The number the last turn was given.
    nth: AtomicI64,
    /// For each agent, the journals whose files hold a turn of it.
    held: Mutex<HashMap<String, Vec<Journal>>>,
    /// The journal that keeps each turn this daemon queued or found in
    /// hand, until the turn has ended.
    at: Mutex<HashMap<String, Journal>>,
}

impl Filed {
    /// The next number, for a turn being queued. Asked in the write
    /// that queues it.
    pub(crate) fn next(&self) -> i64 {
        self.nth.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Says that `journal` holds a turn of `slot`.
    fn holds(&self, slot: &str, journal: &Journal) {
        let mut held = self.held.lock().expect("agents lock");
        let of = held.entry(slot.to_string()).or_default();
        if of.iter().all(|one| one.name() != journal.name()) {
            of.push(journal.clone());
        }
    }
}

/// What one file holds of turns: those left in hand, those waiting in an
/// agent's queue that a step asked for in a word of its own (which may
/// be let go of, `crate::letgo`), and for each agent
/// how many it was given there and which row is the newest, with the
/// number that row has. Read from the index of turns by agent and
/// status, an agent at a time: no turn is read but those in hand and
/// the newest of each agent. Asked for by status alone, the turns in
/// hand were found by reading every turn there ever was, which no index
/// begins with: twenty seconds of a start, on 1.6 GB of turns read from
/// the disk.
type Held = (Vec<InHand>, Vec<String>, Vec<(String, i64, i64)>);

/// A turn left in hand: its id, whose it is, and the channel kept to its
/// members that it is of, where it is of one (`crate::kept`).
pub(crate) type InHand = (String, String, Option<String>);

fn held(connection: &mut Connection) -> rusqlite::Result<Held> {
    let counted = connection
        .prepare("SELECT slot, COUNT(*), MAX(rowid) FROM turns GROUP BY slot")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<Vec<(String, i64, i64)>>>()?;
    let mut in_hand = connection
        .prepare("SELECT id, kept FROM turns WHERE slot = ?1 AND status IN ('running', 'answered')")?;
    let mut queued = connection
        .prepare("SELECT id FROM turns WHERE slot = ?1 AND status = 'queued' AND asked_by IS NOT NULL")?;
    let (mut left, mut waiting, mut given) = (Vec::new(), Vec::new(), Vec::new());
    for (slot, count, newest) in counted {
        for found in in_hand.query_map([&slot], |row| Ok((row.get(0)?, row.get(1)?)))? {
            let (id, kept) = found?;
            left.push((id, slot.clone(), kept));
        }
        for found in queued.query_map([&slot], |row| row.get::<_, String>(0))? {
            waiting.push(found?);
        }
        let nth =
            connection.query_row(&format!("SELECT {NTH} FROM turns WHERE rowid = ?1"), [newest], |row| {
                row.get(0)
            })?;
        given.push((slot, count, nth));
    }
    Ok((left, waiting, given))
}

/// Counts again what was given to each agent whose name another had
/// before it, an agent that was taken away (`crate::removed`): only the
/// turns numbered above the last its name had when it was made are its
/// own. Such a name's turns are read for it, and no other's: there are
/// as many such names as agents that were taken away and made again.
async fn own(factory: &Journal, journals: &[Journal], given: &mut Given) -> Result<()> {
    let made_again: Vec<(String, i64)> = factory
        .db()
        .call(|connection| {
            let mut statement = connection.prepare("SELECT name, since FROM slots WHERE since > 0")?;
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            rows.collect()
        })
        .await?;
    for (name, since) in made_again {
        let (mut all, mut last) = (0, 0);
        for journal in journals {
            let lookup = name.clone();
            let (count, newest): (i64, i64) = journal
                .db()
                .call(move |connection| {
                    connection.query_row(
                        &format!(
                            "SELECT COUNT(*), coalesce(MAX({NTH}), 0) FROM turns WHERE slot = ?1 AND {NTH} > ?2"
                        ),
                        rusqlite::params![lookup, since],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                })
                .await?;
            (all, last) = (all + count, last.max(newest));
        }
        match all {
            0 => given.remove(&name),
            _ => given.insert(name, (all, last)),
        };
    }
    Ok(())
}

/// Every journal of `factory`'s that keeps turns, its own first.
fn every(factory: &Journal) -> Vec<Journal> {
    factory.every_keeping(keeps::TURNS)
}

/// Reads what every file holds of turns, as agents are opened: the
/// turns an earlier daemon left in hand (none of them asked for yet),
/// those it left waiting that a step asked for in a word of its own,
/// what each agent was given, and where turns are kept, with the count
/// set above every number there is.
pub(crate) async fn read(factory: &Journal) -> Result<(Vec<InHand>, Vec<String>, Given, Filed)> {
    let (mut left_in_hand, mut left_waiting) = (Vec::new(), Vec::new());
    let (mut given, filed) = (Given::new(), Filed::default());
    for journal in every(factory) {
        let (left, waiting, counted) = journal.db().call(held).await?;
        for one in left {
            filed.at.lock().expect("agents lock").insert(one.0.clone(), journal.clone());
            left_in_hand.push(one);
        }
        for id in waiting {
            filed.at.lock().expect("agents lock").insert(id.clone(), journal.clone());
            left_waiting.push(id);
        }
        for (slot, count, nth) in counted {
            filed.holds(&slot, &journal);
            filed.nth.fetch_max(nth, Ordering::SeqCst);
            let (all, last) = given.entry(slot).or_insert((0, 0));
            (*all, *last) = (*all + count, (*last).max(nth));
        }
    }
    own(factory, &every(factory), &mut given).await?;
    Ok((left_in_hand, left_waiting, given, filed))
}

impl Agents {
    /// The journal whose file keeps the turns of an agent of `project`:
    /// the project's own where it was made to keep them, and the
    /// factory's otherwise, and for an agent that is nobody's.
    pub(crate) fn turns_of(&self, project: Option<&str>) -> Journal {
        self.inner.journal.kept_at(project.filter(|project| !project.is_empty()), keeps::TURNS)
    }

    /// The journal that keeps `turn`: the one this daemon queued it in
    /// or found it in, while it is in hand, and otherwise its
    /// project's, as its row says.
    pub(crate) fn keeps(&self, turn: &TurnInfo) -> Journal {
        let known = self.inner.filed.at.lock().expect("agents lock").get(&turn.id).cloned();
        known.unwrap_or_else(|| self.turns_of(turn.project.as_deref()))
    }

    /// Notes that the turn `id`, in hand, was found in `journal`.
    pub(crate) fn found_in(&self, id: &str, journal: &Journal) {
        self.inner.filed.at.lock().expect("agents lock").insert(id.to_string(), journal.clone());
    }

    /// Every journal that keeps turns, the factory's own first.
    pub(crate) fn every_turns(&self) -> Vec<Journal> {
        every(&self.inner.journal)
    }

    /// The journals that may hold a turn of `slot`: those known to hold
    /// one, and every one for an agent nothing is known of.
    pub(crate) fn may_hold(&self, slot: &str) -> Vec<Journal> {
        let known = self.inner.filed.held.lock().expect("agents lock").get(slot).cloned();
        known.unwrap_or_else(|| self.every_turns())
    }

    /// Notes that the turn `id` of `slot` was queued in `journal`.
    pub(crate) fn queued_in(&self, slot: &str, id: &str, journal: &Journal) {
        self.inner.filed.holds(slot, journal);
        self.inner.filed.at.lock().expect("agents lock").insert(id.to_string(), journal.clone());
    }

    /// Forgets where a turn that has ended was kept: it is found again
    /// by asking.
    pub(crate) fn ended_in(&self, id: &str) {
        self.inner.filed.at.lock().expect("agents lock").remove(id);
    }

    /// The journals to look for the turn `id` in: the one this daemon
    /// queued or found it in, or every one.
    pub(crate) fn may_keep(&self, id: &str) -> Vec<Journal> {
        let known = self.inner.filed.at.lock().expect("agents lock").get(id).cloned();
        known.map_or_else(
            || {
                let every = self.every_turns();
                let more = every.len().saturating_sub(1) as f64;
                self.inner.journal.measure(TURN_LOOKED_UP, &[], more, true);
                every
            },
            |journal| vec![journal],
        )
    }

    /// Asks each of `journals` the same thing, and gives what each
    /// answered, in their order.
    pub(crate) async fn of_each<T, F>(&self, journals: Vec<Journal>, ask: F) -> Result<Vec<T>>
    where
        T: Send + 'static,
        F: Fn(&mut Connection) -> rusqlite::Result<T> + Clone + Send + 'static,
    {
        let mut answers = Vec::with_capacity(journals.len());
        for journal in journals {
            answers.push(journal.db().call(ask.clone()).await?);
        }
        Ok(answers)
    }

    /// Counts again what each agent was given, from the turns there are
    /// in every file: where turns were taken away. The count their
    /// numbers are given from is left where it is.
    pub(crate) async fn count_given(&self) -> Result<()> {
        let mut given = Given::new();
        for (_, _, counted) in self.of_each(self.every_turns(), held).await? {
            for (slot, count, nth) in counted {
                let (all, last) = given.entry(slot).or_insert((0, 0));
                (*all, *last) = (*all + count, (*last).max(nth));
            }
        }
        own(&self.inner.journal, &self.every_turns(), &mut given).await?;
        *self.inner.given.lock().expect("agents lock") = given;
        Ok(())
    }
}

/// What each asking for a turn by its id is measured as that is not
/// known to be in hand and so goes to every file that keeps turns, by how
/// many files more than one it asks (`_count`): a turn is forgotten from
/// where it was kept as it ends, so whether one is asked of every file
/// goes by whether it had ended in that moment, and what one piece of
/// work cost the factory is read without them (`costed.rs` in the API
/// suite).
pub const TURN_LOOKED_UP: &str = "agent.turn_looked_up_count";

/// What each reading of a turn is measured as by whoever waits for it to
/// end: it is read as the wait begins and again each time whoever waits
/// is woken by something said of it, and whether a turn is waited for at
/// all (a council's seat that has ended as the council first looks is
/// not), and how many times it is read before it has ended, go by how
/// the work about it fell (`costed.rs` in the API suite).
pub const TURN_WAITED: &str = "agent.turn_waited_count";
