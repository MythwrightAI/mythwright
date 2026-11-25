//! The building blocks the runner used to bring itself.
//!
//! Three actions belong to no part of the factory above the runner, and
//! were registered by it: `pool.turn` ([`pool`]), `text.put` ([`words`])
//! and `profile.set` ([`profile`]); a fourth asks the runner what a
//! project's environments are and makes those it lacks,
//! `environment.ensure` ([`environed`]), which is what a new project's
//! environments are made ready by; and a fifth says a project's feature
//! flag in one of its environments, `flag.set` ([`flagged`]). Each is an action like any
//! other crate's: it is handed an [`conductor::ActionCx`] and asks the
//! engine what anybody may ask it. So they are here, outside the runner,
//! and whatever makes a registry for a factory adds them ([`register`]).
//!
//! What each works out is not here either. Who a pool gives a place to
//! is `kernel::pool`, and what parts come to is `blueprint::parts`:
//! functions of plain values, which the runner still offers under the
//! names it always gave them (`conductor::pool`, `conductor::words`). The
//! variables of the profile are the engine's to keep
//! (`Engine::set_variables`). What is here is the asking: what a
//! blueprint hands, what it is handed back, and the little each needs of
//! the engine to answer.

pub mod environed;
pub mod flagged;
pub mod pool;
pub mod profile;
pub mod words;

use conductor::Registry;

/// Adds all of them to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    profile::register(registry);
    environed::register(registry);
    flagged::register(registry);
    pool::register(registry);
    words::register(registry);
}
