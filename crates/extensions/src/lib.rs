//! Extension packages: what a package says, and how a turn of its
//! program is run.
//!
//! A package is what the application is not, read from its manifest
//! ([`extension::MANIFEST`]): the program it runs and how one turn is put
//! to it and read ([`extension`], [`adapter`], [`running`]), the actions it
//! declares and how one is done ([`declared`], [`program`],
//! [`performed`]), the blueprints and prompts it brings ([`brought`]), what
//! it shows of a repository ([`shown`]), how its program is handed a
//! sign-in ([`handed`]), and what that program needs behind a wall
//! ([`walled`]). None of it asks anything of the agents.
//!
//! It came out of `agents` whole when that crate was at its ceiling: what
//! of it needs the agents (which packages are installed, an agent's wall
//! and home, the library and the actions a package brings while it is
//! installed) stayed there, and `agents` offers everything here under the
//! names it always had (`agents::extension`, `agents::adapter`,
//! `agents::Turn`, `agents::walled::Behind`, ...). Nothing here says an
//! event.

pub mod adapter;
pub mod brought;
pub mod declared;
pub mod extension;
pub mod handed;
pub mod performed;
pub mod program;
pub mod running;
pub mod shown;
pub mod walled;

pub use adapter::{Adapter, Failure, Reply, Turn};
pub use extension::{Extension, MANIFEST, Model};

/// How much an agent may do on its own. A package says what each means to
/// its program.
pub const PERMISSIONS: &[(&str, &str)] = &[
    ("read", "look, and change nothing"),
    ("edit", "change files where it works"),
    ("full", "anything, unasked"),
];

/// What the slot of an agent this factory does not start says runs it: a
/// word no package may be called (`agents::external`).
pub const EXTERNAL: &str = "external";
