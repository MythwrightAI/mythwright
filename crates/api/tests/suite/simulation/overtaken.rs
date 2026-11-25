//! What a play hears, and whether a look at it was overtaken.
//!
//! A play at rest somewhere else than was meant is a fault, and what it
//! has come to is read before it is asked whether anything is under way:
//! the daemon may go on between the two, and what it said meanwhile is
//! heard before the play is judged (`looked.rs`). That was asked by each
//! journal's last number against how far the play had read, and the two
//! need never meet. An event passed on and not kept is a journal's last,
//! and the play reads no further for it where the order hands it over
//! ahead of the stream it follows (`journal::Following`: what was passed
//! on is handed from the order and moves no place); nor where the play
//! follows from a place again (each act, a restart) and the stream reads
//! back from the log, which has none of it. Nothing more being said, the
//! play listened for what it had heard already until it was cut off, and
//! what it differed in was never said.
//!
//! So a look asks two things that do come. Whether anything is there to
//! be heard, by listening for a moment; and whether anything was written
//! since what the play came to was read, by the order the rig writes down
//! (`Rig::mark`), which counts every event as it is written, kept or
//! passed on, and nothing else: where neither is so, what was read is
//! what the play has come to.

use std::sync::Arc;
use std::time::Duration;

use journal::{Following, Journal};
use kernel::Event;

use crate::Rig;

/// What a play hears next.
pub(super) enum Next {
    /// The journal shut at the place the daemon is to be stopped at.
    Gone,
    /// A stand-in's program said it is there.
    Began(String),
    /// Something the daemon said, or nothing for a moment.
    Said(Result<kernel::Result<Arc<Event>>, tokio::time::error::Elapsed>),
}

/// Completes when the journal has shut at the place the play stops at; in a
/// play that stops nowhere, or has stopped already, never.
pub(super) async fn shut(journal: Journal, stops: bool) {
    if stops {
        journal.shut().await;
    } else {
        std::future::pending::<()>().await;
    }
}

/// What the daemon did while a play was looked at.
pub(super) enum Since {
    /// It said something the play had not heard, which is heard before
    /// the play is looked at again.
    Heard(Next),
    /// It wrote something since what the play came to was read, which is
    /// not there to be heard yet: the play listens, and is looked at again.
    Writing,
    /// Nothing: what was read is what the play has come to.
    Nothing,
}

/// Whether the daemon went on while a play was looked at, from `mark`,
/// where the order stood before what the play came to was read.
pub(super) async fn since(rig: &Rig, follow: &mut Following, mark: u64, lull: Duration) -> Since {
    if let Ok(said) = tokio::time::timeout(lull, follow.next()).await {
        return Since::Heard(Next::Said(Ok(said)));
    }
    if rig.mark() > mark { Since::Writing } else { Since::Nothing }
}
