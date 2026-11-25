//! What of a project's agents is kept here, for whoever moves a project
//! of a factory from before to a journal of its own (`journal::moved`).
//!
//! The move knows no table. It is handed the turns ([`tables`]) with how
//! a turn says whose it is ([`crate::apart::TURNS`], which is what the
//! rows are held to): by what it says, and one kept before a turn said
//! so by the agent it names, as the list of agents stands when the
//! project is moved. An agent is the factory's and is not moved, so a
//! turn goes with the project it is of whoever its agent is of by now,
//! and one that is nobody's stays.
//!
//! Two things a turn from before says only by where it is are written
//! out as it goes, since where it goes it is neither. Which turn of all
//! it is ([`crate::filed::NTH`]): one that says no number is ordered by
//! where it came in the factory's file, which was the one file such
//! turns were in, so that is the number it is given. And whose it is:
//! the list of agents it was read through is not in a project's file.
//! The turns are written in the order of their numbers, which is the
//! order a file's rows are relied on to be in; where the project's file
//! holds turns already, those are written again with the ones that
//! come, all in that order.
//!
//! And what is in the way ([`in_hand`]): a turn that is waiting, running
//! or answered and not yet said to have ended is work in the middle of
//! itself. The run that asked for it is under way, or its program goes
//! on without the daemon and is taken up when one starts, or somebody
//! asked for it in a channel and nothing is under way for it at all. A
//! project is moved with none of it in hand, so the move asks, of a
//! factory that is stopped.

use journal::moved::Kept;
use rusqlite::Connection;

use crate::apart::{KEPT_APART, TABLES, TURNS};

/// Which turn of all a row called `kept` is, as the database is asked
/// it ([`crate::filed::NTH`]).
const NTH: &str = "coalesce(kept.nth, kept.rowid)";

/// The turns, as the move is handed them, with the level a journal
/// keeps them from (`journal::keeps`): each written saying its project
/// and its number, in the order of the numbers.
pub fn tables() -> Vec<(i64, Kept)> {
    let lines = KEPT_APART.iter().filter_map(|(table, level)| {
        let told = journal::apart::told(TABLES, table)?;
        // The list of agents a turn may be read through stays.
        // And a turn may be the project's in the factory's file though
        // the project's journal keeps turns: it was asked of an agent
        // that is another project's by now, or was taken away, and the
        // journal was made when no agent was the project's any more
        // (`crate::apart::held_here`, which asks the agents).
        let kept = Kept::rows(table, told.reads(), 1).saying().writing("nth", NTH).ordered(NTH).straying();
        Some((*level, kept))
    });
    lines.collect()
}

/// A turn of one of a project's agents that is waiting or under way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InHand {
    pub turn: String,
    /// The agent whose turn it is, by name.
    pub agent: String,
    /// `queued`, `running` or `answered`, as its row says.
    pub status: String,
}

/// The turns of `project` that are queued, running or answered and not
/// yet ended, the oldest first, on a connection to a factory's database
/// that is stopped: told apart as the turns that are moved are, so one
/// that says the project is its own though its agent was given to
/// another since, and one of an agent that was taken away. Asked of the
/// index of turns by agent and status, an agent at a time: no turn is
/// read but those in hand. None, in a database that keeps no agents.
pub fn in_hand(connection: &Connection, project: &str) -> rusqlite::Result<Vec<InHand>> {
    let there: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN ('slots', 'turns')",
        [],
        |row| row.get(0),
    )?;
    if there < 2 {
        return Ok(Vec::new());
    }
    let mut statement = connection.prepare(&format!(
        "SELECT kept.id, kept.slot, kept.status FROM (SELECT DISTINCT slot FROM turns) each
         CROSS JOIN turns kept ON kept.slot = each.slot AND kept.status IN ('queued', 'running', 'answered')
         WHERE {} IS ?1 ORDER BY kept.created, kept.id",
        TURNS.reads()
    ))?;
    let rows = statement.query_map([project], |row| {
        Ok(InHand { turn: row.get(0)?, agent: row.get(1)?, status: row.get(2)? })
    })?;
    rows.collect()
}
