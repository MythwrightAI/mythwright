//! What the runner hands the machinery that came out of it in place of
//! itself.
//!
//! Handing events to handlers ([`crate::delivery`]), turns taken by key
//! ([`crate::turns`]) and work done whole ([`crate::whole`]) begin tasks
//! of their own, and a task the runner begins is the conductor's: one
//! that was halted begins none, and what it began is ended with it. So
//! they are handed what begins one, and nothing else of the engine.

use std::future::Future;

use crate::Store;

/// What begins a task that is the conductor's own, and keeps the rows
/// it writes: the runner's engine answers it (`impl Hands for Engine`).
pub trait Hands: Clone + Send + Sync + 'static {
    /// Begins `work` as a task of the conductor's own, as part of
    /// whatever its asker is part of. Nothing is begun by a conductor
    /// that was halted: what would have been is found written down by
    /// the one that starts next.
    fn spawn(&self, work: impl Future<Output = ()> + Send + 'static);

    /// Where the conductor keeps its rows.
    fn store(&self) -> &Store;
}
