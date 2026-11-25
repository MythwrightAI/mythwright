//! What the daemon serves over HTTP beside the routes of its API, and what
//! every request is held to before a route is looked at.
//!
//! The client's files and the notice that they were replaced ([`client`]);
//! whether a request was addressed to this daemon and comes from where it
//! may ([`guard`], with the names it answers to); what is sent past what a
//! route takes, read through before a refusal goes back ([`sent`]); a
//! library's documents followed where they are and served as pages
//! ([`library`]); mockups served under a way in ([`mockups`]); and the
//! store program, a store service that keeps records in one database file
//! and is run as a program of its own ([`store`]).
//!
//! It came out of the daemon (`api`) when that crate was at its ceiling:
//! none of it asked anything of the routes, and what it needs of the node
//! it is handed (the client, the names, the studio, the journal, what
//! follows files, the ways in). `api` offers each under the name it
//! always had (`api::ui`, `api::client_file`, `api::store`, `api::sent`),
//! keeps the routes of a library's pages and of mockups, each a call of
//! what is here, and adds what is registered here where it always was
//! ([`register`]).

pub mod client;
pub mod events;
pub mod guard;
pub mod library;
pub mod mockups;
pub mod sent;
pub mod store;

use std::sync::Arc;

use conductor::Registry;

pub use events::events;
pub use guard::Names;

/// Adds the action by which a blueprint tells the daemon a name it is
/// asked under (`system.answer_as`), with the events said here.
pub fn register(registry: &mut Registry, names: Arc<Names>) {
    guard::register(registry, names);
    registry.emits(events());
}
