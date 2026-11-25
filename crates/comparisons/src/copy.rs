//! What of the comparisons does not go into a copy of a factory.
//!
//! A copy is a factory's database at one moment, for another factory to
//! be started from (`journal::copy`), and nothing is running in it
//! (`conductor::copy`). A comparison says how a running tree's work is
//! started and counts what is measured of it, and a copy has neither: the
//! comparisons, the way each thing went, what each thing started under a
//! way reads, the place events are counted from and what was counted once
//! are all removed.

use journal::copy::{Connection, forget};
use runs::beside::Whose;

/// Every name comparisons keep something under in the small durable
/// state, with whose it is and why, as the runner says of its own
/// (`runs::beside::KEPT`). A comparison is one for the factory, started
/// from outside and counted from every journal; what a thing started
/// under a way reads is the instance's that started it.
pub const KEPT: &[(&str, Whose, &str)] = &[
    (
        crate::compared::NAMESPACE,
        Whose::Factory,
        "a comparison is said from outside, once, of the whole factory",
    ),
    (
        crate::compared::WENT,
        Whose::Factory,
        "which way a thing went is the comparison's to know, whoever runs the thing",
    ),
    (
        crate::counted::NAMESPACE,
        Whose::Factory,
        "what is counted is counted from every journal, from one place in each",
    ),
    (
        crate::counted::ONCE,
        Whose::Factory,
        "what was counted once of a thing is the comparison's, as which way the thing went is",
    ),
    (
        crate::instead::NAMESPACE,
        Whose::Instance,
        "what a thing reads in place of a file is settled as it is started, for what started it, and goes with that",
    ),
];

/// Removes every comparison from a copy, and what is kept beside them.
/// Done again it finds none.
pub fn leave_out(connection: &Connection) -> rusqlite::Result<()> {
    for namespace in [
        crate::compared::NAMESPACE,
        crate::compared::WENT,
        crate::counted::NAMESPACE,
        crate::counted::ONCE,
        crate::instead::NAMESPACE,
    ] {
        forget(connection, namespace)?;
    }
    Ok(())
}
