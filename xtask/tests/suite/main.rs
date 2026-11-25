//! One test binary for the crate; add modules here.
//!
//! These run the real tooling, as `cargo xtask` does, in a tree made for
//! the test: nothing is built and nothing outside the tree is touched.
//! What a part of it works out from what it is given is asked of that
//! part directly (`xtask/src/lib.rs`).

mod changes;
mod clock;
// Unix's: what stands in for a test program here is a file of its shell
// made a program by its mode.
#[cfg(unix)]
mod crowd;
mod general;
mod generations;
// Unix's: what stands in for a build here is that system's (programs
// called with no ending), and the commands an install puts on a path
// are held to as links, which Windows does not give out freely.
#[cfg(unix)]
mod install;
mod left;
// Unix's: what stands in for the tool and the client's build here is a
// file of that system's shell.
#[cfg(unix)]
mod noticed;
mod notices;
// Unix's: a package's programs are told from its other files by their
// mode, and what stands in for cargo is a file of that system's shell.
#[cfg(unix)]
mod package;
mod plain;
mod played;
mod scope;
mod sha256;
mod sharing;
mod systems;
mod tools;
mod ways;
mod zip;
