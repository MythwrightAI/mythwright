//! What of hosting's does not go into a copy of a factory.
//!
//! A copy is a factory's database at one moment, for another factory to be
//! started from (`journal::copy`). Hosting's rows are the services and
//! routers that are running, and they are how a daemon finds its processes
//! again after a restart: by pid and start time, from the row.
//!
//! A new factory that had those rows would find the processes of the
//! factory the copy was taken from, take them for its own, and later stop
//! them. So none of them goes: both tables are emptied in a copy. Nothing
//! is stopped by it. The processes are another factory's, and stay so.

use journal::copy::Connection;
use serde::{Deserialize, Serialize};

/// The tables this crate keeps, by the name they are migrated under, and
/// the version of each this build knows.
pub const KNOWS: &[(&str, usize)] = &[("hosting", crate::service::SCHEMA.len())];

/// The tables this crate keeps, each spoken for above: one that no crate
/// lists is emptied in a copy.
pub const TABLES: &[&str] = &["services", "routers"];

/// What was running where a copy was taken, and is not in it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unhosted {
    pub services: usize,
    pub routers: usize,
}

/// Removes every service and router from a copy. Done again it finds none.
pub fn leave_out(connection: &Connection) -> rusqlite::Result<Unhosted> {
    let services = connection.execute("DELETE FROM services", [])?;
    let routers = connection.execute("DELETE FROM routers", [])?;
    Ok(Unhosted { services, routers })
}
