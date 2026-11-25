//! One test binary for the crate; add modules here.
//!
//! What is here needs no conductor: what is kept of the profile's
//! variables, how a change of them and of a project's environment is
//! worked out, and what is refused. What
//! an expression reads of them, and what is said when they change, is the
//! runner's and is played in its suite (`profile.rs`) and in the suite
//! of `blocks`, whose action sets them (`variables.rs`).

mod factory;
mod held;
mod last;
mod project;
mod rules;

use std::sync::Arc;

use journal::Journal;
use kernel::ManualClock;

/// A journal of a factory's own, in memory.
pub async fn journal() -> Journal {
    Journal::memory(Arc::new(ManualClock::new())).await.unwrap()
}
