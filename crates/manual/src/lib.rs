//! What the program says of itself: the help of its command line, the
//! guides it carries, and all of it written out as pages.
//!
//! It came out of the command line so that the two things that write the
//! pages, the command (`docs export`, which asks no daemon) and the action
//! a daemon runs (`docs.export`), are one function of plain values
//! ([`pages::pages`]). It stands on the language and on nothing that runs:
//! what a device has of actions and events, and the blueprints that ship,
//! are handed to it.

mod help;
mod help_copies;
mod help_first;
mod help_more;
mod help_who;

pub mod commands;
pub mod guides;
pub mod pages;

pub use commands::EVERYWHERE;
pub use help::{overview, topic};

/// This program's version, which every page written out says.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The notes of each release, as they were when this program was built
/// (`docs/releases.md`): an installed copy has them with no checkout.
pub const RELEASES: &str = include_str!("../../../docs/releases.md");
