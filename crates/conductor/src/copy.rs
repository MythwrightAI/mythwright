//! What of the conductor's does not go into a copy of a factory.
//!
//! A copy is a factory's database at one moment, for another factory to be
//! started from (`journal::copy`). The conductor's rows say what is
//! running, and after a restart they are taken as the truth: an instance
//! that says `active` is brought back, a run that says `running` is picked
//! up. In a copy none of that is true. Nothing is running there, and the
//! work those rows describe is another factory's, still being done by it.
//!
//! So in a copy everything that was in progress is ended, where it stands
//! and in so many words, and stays as history. What the rows say of it is
//! ended where the rows are kept ([`runs::copy`]): a live instance is
//! `stopped`, a run or a step that had not finished is `cancelled`, and
//! the places of event handlers are removed. And of what the conductor
//! keeps beside its rows:
//!
//! - what `poll:` handlers last read (`poll` in the small durable state)
//!   is removed with the instances it was remembered for;
//! - the variables of the profile (`crate::profile`) are removed.
//!   They were said of the factory copied: where its services answer,
//!   what it may do. A copy that kept them would read the other factory's
//!   addresses as its own, so it has none until they are set in it.
//!
//! - the last text of each blueprint that checked (`crate::reach`) is
//!   removed. It is kept so that an instance that is running can go on
//!   when a text it calls no longer checks, and nothing is running here:
//!   what is applied in the new factory is read and checked as it then is.
//!
//! - what each running tree reached of the texts that ship
//!   (`crate::pinned`) is removed with the trees. The comparisons, which
//!   say how a running tree's work is started and count measurements,
//!   are taken out by the crate that keeps them (`comparisons::copy`).
//!
//! What a project's fork of a blueprint that ships was forked from
//! (`crate::forked`) is not removed: it is said of a file of the
//! project's, and whoever keeps the small state that goes into a copy
//! lists it (`studio::copy::KEPT`).
//!
//! No hook runs and nothing is torn down: that would be done to what the
//! factory the copy was taken from is running.

use journal::copy::{Connection, forget};
pub use runs::copy::{Ended, KNOWS, MAKING_THE_COPY, NOT_FINISHED, NOT_STARTED, TABLES};

/// Ends everything a copy says is in progress, as of `at` (milliseconds).
/// Done again it finds nothing in progress and changes nothing. `making`
/// is the instance and the run by which the copy is being made, when it
/// is being made by one: they are ended with the rest, say what they
/// were, and are not counted among what was running.
pub fn leave_out(connection: &Connection, at: i64, making: Option<(&str, &str)>) -> rusqlite::Result<Ended> {
    let ended = runs::copy::leave_out(connection, at, making)?;
    forget(connection, runs::beside::POLL)?;
    forget(connection, runs::beside::AT)?;
    forget(connection, runs::beside::VOUCHED)?;
    forget(connection, runs::beside::OWED.0)?;
    forget(connection, crate::profile::NAMESPACE)?;
    forget(connection, runs::beside::FITS)?;
    forget(connection, runs::beside::PINNED)?;
    forget(connection, runs::beside::TEXTS)?;
    Ok(ended)
}
