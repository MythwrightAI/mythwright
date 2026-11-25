//! A factory's data folder, worked on as files.
//!
//! A factory is one daemon on one data folder, and most of what is done
//! to the folder is done with no daemon there, or before one has opened
//! anything in it: who has the folder ([`folder`]), what the factory is
//! called and where it listens ([`profile`]), which factories there
//! are beside it on this device ([`registry`]), a copy of it to start
//! another from ([`snapshot`]) and a factory started from one
//! ([`arrival`]), a project of a factory from before moved to a journal
//! of its own ([`moved`]), its database written out again
//! ([`repack`]), a project's own blueprints as a layer of the library
//! ([`library`]), where a text stands among those that ship
//! ([`forked`]), where the running program is ([`place`]), and the
//! factory started again by a step of its own ([`restarting`]); and,
//! until a crate of what a package says is made, what this program and
//! each package offer a blueprint, held to what each offered before
//! ([`contract`]). None of it serves anything: the daemon (`api`) asks it as it starts and
//! stops, and offers each under the name it always had
//! (`api::snapshot`, `api::folder`, …), and the command line asks it for
//! what is done to a factory that is stopped.

pub mod across;
pub mod arrival;
pub mod contract;
mod events;
pub mod folder;
pub mod forked;
pub mod library;
pub mod moved;
pub mod place;
pub mod profile;
pub mod registry;
pub mod repack;
pub mod restarting;
pub mod snapshot;
mod tree;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use conductor::Registry;

pub use events::events;

/// The file in a data folder that is the factory's database. A folder
/// that has one is a factory's data, which is how one is known on disk.
pub const DATABASE_FILE: &str = "journal.db";
/// How long a starting node, and whatever else comes to hold a factory's
/// folder, waits for the data directory's lock.
pub const LOCK_PATIENCE: Duration = Duration::from_secs(2);

/// Where something on this device reaches the daemon. One that listens on
/// every address has none of its own to give out, and is reached on this
/// device's own.
pub fn own_url(addr: SocketAddr) -> String {
    let mut addr = addr;
    if addr.ip().is_unspecified() {
        addr.set_ip(match addr.ip() {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
        });
    }
    format!("http://{addr}")
}

/// Adds what a blueprint may ask of the folder (a snapshot of the
/// factory, `factory.snapshot`), with the events said of it.
pub fn register(registry: &mut Registry, source: snapshot::Source, projects: studio::Projects) {
    snapshot::register(registry, source, projects);
    registry.emits(events());
}
