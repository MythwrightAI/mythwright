//! What of the runner's rows does not go into a copy of a factory.
//!
//! A copy is a factory's database at one moment, for another factory to be
//! started from (`journal::copy`). The rows kept here say what is running,
//! and after a restart they are taken as the truth: an instance that says
//! `active` is brought back, a run that says `running` is picked up. In a
//! copy none of that is true. Nothing is running there, and the work those
//! rows describe is another factory's, still being done by it.
//!
//! So in a copy everything that was in progress is ended, where it stands
//! and in so many words, and stays as history:
//!
//! - an instance that was live is `stopped`, holds nothing, and has no
//!   update in progress;
//! - a run that had not finished is `cancelled`, and so is a step of one,
//!   and how far through a loop such a step had got is forgotten;
//! - the places of event handlers (`places`) are removed. A
//!   handler belongs to a live instance and there is none; one that starts
//!   to listen in the new factory starts from the end of the journal, as
//!   any new handler does, so nothing that already happened fires it.
//!   And nothing is kept as alive or under way (`crate::apart`): the
//!   database follows the rows as they are ended.
//!
//! Whose each row was, it still says: an instance its project, and a run
//! with it.
//!
//! What the runner keeps beside its rows, in the small durable state, is
//! the runner's to leave out (`conductor::copy`), which asks this first.
//!
//! No hook runs and nothing is torn down: that would be done to what the
//! factory the copy was taken from is running.

use journal::copy::Connection;
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// The tables this crate keeps, by the name they are migrated under, and
/// the version of each this build knows. The name is the runner's, which
/// they were always migrated under ([`crate::Store::open`]).
pub const KNOWS: &[(&str, usize)] = &[("conductor", crate::schema::SCHEMA.len())];

/// The tables this crate keeps, each spoken for above: one that no crate
/// lists is emptied in a copy. The last is the one a handler's place was
/// kept in before it said which journal it is a place in, which a copy
/// an earlier program made has, and a journal from before that has kept
/// much ([`crate::SMALL`]).
pub const TABLES: &[&str] = &[
    "instances",
    "instance_versions",
    "runs",
    "steps",
    "step_elements",
    "places",
    "alive",
    "under_way",
    "trigger_cursors",
];

/// What is said of an instance that was running where the copy was taken.
pub const NOT_STARTED: &str = "it was running in the factory this one was copied from, and a copy starts nothing; \
                               apply its blueprint again to run it here";
/// What is said of a run, or a step, that had not finished there.
pub const NOT_FINISHED: &str =
    "it had not finished in the factory this one was copied from, and is not carried on in a copy";

/// What is said of the instance, the run and the step by which the copy
/// itself was being made: they were in progress too, and are not what
/// anyone means by what the factory was running.
pub const MAKING_THE_COPY: &str = "it was making the copy this factory was started from";

/// What was in progress in a copy, and no longer is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ended {
    /// Instances that were live.
    pub instances: usize,
    /// Runs that had not finished.
    pub runs: usize,
    /// Steps that were running.
    pub steps: usize,
}

/// Ends everything the rows of a copy say is in progress, as of `at`
/// (milliseconds). Done again it finds nothing in progress and changes
/// nothing. `making` is the instance and the run by which the copy is
/// being made, when it is being made by one: they are ended with the
/// rest, say what they were, and are not counted among what was running.
pub fn leave_out(connection: &Connection, at: i64, making: Option<(&str, &str)>) -> rusqlite::Result<Ended> {
    let steps = connection.execute(
        "UPDATE steps SET status = 'cancelled', error = ?1, finished = coalesce(finished, ?2)
         WHERE status = 'running'",
        params![NOT_FINISHED, at],
    )?;
    let runs = connection.execute(
        "UPDATE runs SET status = 'cancelled', error = ?1, finished = coalesce(finished, ?2)
         WHERE status = 'running'",
        params![NOT_FINISHED, at],
    )?;
    let instances = connection.execute(
        "UPDATE instances SET status = 'stopped', error = ?1, holds = '[]', previous = NULL, updated = ?2
         WHERE status IN ('starting', 'active')",
        params![NOT_STARTED, at],
    )?;
    // Where handlers had got to, in the table a place is kept in now and
    // in the one it was kept in before: a copy may be one an earlier
    // program made, whose tables this one brings up when it is started on.
    for places in ["places", "trigger_cursors"] {
        let there: bool = connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [places],
            |row| row.get(0),
        )?;
        if there {
            connection.execute(&format!("DELETE FROM {places}"), [])?;
        }
    }
    // How far through a loop each step under way had got: of no use to a
    // run that is not carried on.
    connection.execute("DELETE FROM step_elements", [])?;
    let mut ended = Ended { instances, runs, steps };
    if let Some((instance, run)) = making {
        ended.steps -= connection.execute(
            "UPDATE steps SET error = ?1 WHERE run = ?2 AND error = ?3",
            params![MAKING_THE_COPY, run, NOT_FINISHED],
        )?;
        ended.runs -= connection.execute(
            "UPDATE runs SET error = ?1 WHERE id = ?2 AND error = ?3",
            params![MAKING_THE_COPY, run, NOT_FINISHED],
        )?;
        ended.instances -= connection.execute(
            "UPDATE instances SET error = ?1 WHERE id = ?2 AND error = ?3",
            params![MAKING_THE_COPY, instance, NOT_STARTED],
        )?;
    }
    Ok(ended)
}
