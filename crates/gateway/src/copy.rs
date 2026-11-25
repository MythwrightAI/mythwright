//! What of the gateway's does not go into a copy of a factory.
//!
//! A copy is a factory's database at one moment, for another factory to be
//! started from (`journal::copy`). A fleet is not copied: which devices
//! are members of a hub, and which hub a device has joined, are between
//! those devices and that factory.
//!
//! - A member is known to its hub by a record of the key it proved
//!   (`gateway.devices`), and is let in by an invitation whose hash the hub
//!   keeps (`gateway.invitations`). Both are removed here. A factory
//!   started from a copy of a hub has no members and has invited nobody;
//!   and it is another installation, with a secret and so a key of its own
//!   (`crate::own`), which no member of the first takes for its hub.
//! - When each member was last seen (`gateway.seen`) is removed here: it
//!   is said of devices the new factory does not have.
//! - A device's record of the hub it joined is a file beside the database.
//!   It is not in the database, and is not copied: a factory started from
//!   a copy of a member joins no hub.
//! - The installation's secret is a file beside the database too, and is
//!   never copied.
//!
//! The gateway keeps no tables of its own.

use journal::copy::{Connection, forget};

/// Where the hub keeps when it last heard from each member.
pub(crate) const SEEN: &str = "gateway.seen";
/// Where a device keeps that the hub it had joined took it out: which hub,
/// when, and what the hub said, for as long as it has joined no other. It
/// is this device's own history and no part of a copy.
pub(crate) const UNJOINED: &str = "gateway.unjoined";
/// Where a device keeps which instances its hub's steps started on it
/// (`blueprint.apply` sent there), by the instance: so that a device its
/// hub has taken out can say what of the hub's is still running on it.
/// This device's own, and no part of a copy.
pub(crate) const STARTED: &str = "gateway.started";

/// Removes the gateway's part of what does not go into a copy. Returns how
/// many devices were in the fleet or invited to it.
pub fn leave_out(connection: &Connection) -> rusqlite::Result<usize> {
    let members = forget(connection, crate::door::DEVICES)?;
    let invited = forget(connection, crate::door::INVITATIONS)?;
    forget(connection, SEEN)?;
    Ok(members + invited)
}
