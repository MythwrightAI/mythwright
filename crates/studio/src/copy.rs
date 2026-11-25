//! What of the studio's does not go into a copy of a factory, and where a
//! copy's projects are put.
//!
//! A copy is a factory's database at one moment, for another factory to be
//! started from (`journal::copy`). Most of what the studio keeps is what a
//! copy is for: accounts, channels and what was said in them, tables and
//! their records, projects. These go as they are, people's passwords
//! among them (as they are kept, hashed), so that the same people sign in
//! to the new factory with the passwords they have.
//!
//! What does not go ([`leave_out`]):
//!
//! - **sessions, credentials and invitations**, every one. Being signed in
//!   to one factory is not being signed in to another, and an agent's or a
//!   device's credential is for the factory that issued it.
//! - **the address of a store service.** A table kept by one is reached at
//!   an address, and in a copy that address is the other factory's store:
//!   the new factory's board would read and write the other's cards. So in
//!   a copy such a table has no address, and says so until it is given one.
//! - **what was attached and never posted**: it was waiting for a message
//!   somebody was still writing, somewhere else.
//! - **what a scan last saw of each project's files**: it is true of the
//!   directories of the factory copied. [`look`] writes it again from the
//!   directories the new factory has.
//! - **where the search index had read to** (`search`): the index is a
//!   file of its own that is not copied, and is built again in full.
//!
//! What stays of the small durable state is in [`KEPT`].
//!
//! **Projects.** A project is a directory, said by its whole path, and two
//! factories must never share one. In a copy a project's place is written
//! relative to a folder of projects, and the new factory puts each under
//! its own: that is the `projects` crate's (`projects::copied`, which came
//! out of this crate and is offered here under the names it had:
//! [`uproot`], [`plant`], [`look`]).

use journal::copy::{Connection, forget};
use serde::{Deserialize, Serialize};

pub use ::projects::copied::{Placed, look, plant, uproot};

/// The tables this crate keeps, by the name they are migrated under, and
/// the version of each this build knows.
pub const KNOWS: &[(&str, usize)] = &[
    people::KNOWS[0],
    people::KNOWS[1],
    people::KNOWS[2],
    people::KNOWS[3],
    people::KNOWS[4],
    people::KNOWS[5],
    ("projects", crate::projects::SCHEMA.len()),
    ("files", crate::tables::FILES.len()),
    talk::KNOWS[0],
    talk::KNOWS[1],
    talk::KNOWS[2],
    talk::KNOWS[3],
    ("collections", crate::collections::SCHEMA.len()),
    ("board", crate::board::LEGACY.len()),
    ("workspaces", crate::workspaces::SCHEMA.len()),
];

/// The tables this crate keeps, each of which goes into a copy as it is or
/// is spoken for above: one that no crate lists is emptied in a copy.
pub const TABLES: &[&str] = &[
    "accounts",
    "sessions",
    "credentials",
    "invites",
    "themes",
    "themes_chosen",
    "arranged",
    "clocked",
    "appearance",
    "groups",
    "group_members",
    "projects",
    "project_files",
    "attachments",
    "channels",
    "channel_members",
    "channel_docs",
    "discussing",
    "discussions",
    "messages",
    "inbox",
    "follows",
    "follow_changes",
    "collections",
    "records",
    "record_history",
    "board_items",
    "workspaces",
    "workspace_projects",
    "workspace_channels",
    "trash",
    "asked",
    "record_attachments",
];

/// The small durable state that does go into a copy, by its namespace: the
/// columns of a board from before boards were tables, which are what the
/// board is; and where the keeper of records' histories has read to. The
/// second is a place in the journal, and is kept because the journal is
/// copied with it and a line of history is written once however often its
/// event is read: kept, the histories carry on from where they were;
/// removed, the whole journal would be read again to no effect. And what
/// the factory's bot is called ([`crate::speaker`]): the channels go into a
/// copy with what was said in them, and go on under the name they had.
/// And what each fork of a blueprint that ships was forked from
/// (`conductor::forked`): a fork is a file of its project's, which the
/// copy's project still has, and what it was made from is said of that
/// file for as long as it is there. And a project's environments
/// (`conductor::environs`), with their variables, which are the project's.
/// And how long the detail of runs' steps is kept (`journal::letgo`), which is how the factory is kept;
/// where they write is not, since two factories never write in one
/// folder. And how long what is deleted is kept in the Trash
/// (`crate::trash`), whose rows and files go too. And where each project
/// keeps its own blueprints (`conductor::library::folders`), and how its
/// things are arranged in the side panel (`kernel::outline`).
pub const KEPT: &[&str] = &[
    "board.columns",
    "history",
    crate::speaker::NAMESPACE,
    conductor::taking::NAMESPACE,
    conductor::forked::NAMESPACE,
    conductor::environs::NAMESPACE,
    journal::letgo::KEEPS,
    crate::trash::KEEPS,
    conductor::library::folders::NAMESPACE,
    kernel::outline::NAMESPACE,
    // The setups new projects may be set up with (`setup::setups`), which `studio` names by its word.
    "project.setups",
    // Which of what is not yet whole the factory has turned on.
    kernel::experiments::NAMESPACE,
];

/// A table that was kept by a store service where the copy was taken, and
/// has no store in the copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unstored {
    pub project: String,
    pub collection: String,
}

/// What was taken out of a copy.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Removed {
    pub sessions: usize,
    pub credentials: usize,
    pub invites: usize,
    /// Things attached to a message that was never posted.
    pub unposted: usize,
    /// Every table of the copy that is kept by a store service: none of
    /// them has an address any more.
    pub stores: Vec<Unstored>,
}

/// Removes the studio's part of what does not go into a copy. Done again
/// it finds nothing to remove, and names the same tables without a store.
pub fn leave_out(connection: &Connection) -> rusqlite::Result<Removed> {
    let sessions = connection.execute("DELETE FROM sessions", [])?;
    let credentials = connection.execute("DELETE FROM credentials", [])?;
    let invites = connection.execute("DELETE FROM invites", [])?;
    let unposted = connection.execute("DELETE FROM attachments WHERE message IS NULL", [])?;
    connection.execute("DELETE FROM project_files", [])?;
    connection.execute("UPDATE collections SET at = NULL WHERE store = 'service'", [])?;
    forget(connection, "search")?;
    let mut statement = connection
        .prepare("SELECT project, name FROM collections WHERE store = 'service' ORDER BY project, name")?;
    let stores = statement
        .query_map([], |row| Ok(Unstored { project: row.get(0)?, collection: row.get(1)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Removed { sessions, credentials, invites, unposted, stores })
}

/// The small durable state of a project's own journal that goes into a
/// copy: where the keeper of records' histories has read to in that
/// journal, which is kept there with the lines it wrote
/// ([`crate::history`]); and what each of the project's forks of a
/// blueprint that ships was forked from, and its environments, which are
/// kept where what is run for the project is ([`KEPT`]).
pub const KEPT_BESIDE: &[&str] =
    &["history", conductor::forked::NAMESPACE, conductor::environs::NAMESPACE, BRANCHES];

/// Where each branch of a project was said to be last, which places keep
/// (`places::branches::NAMESPACE`, named here as places stand above this):
/// kept in a copy so that what moved since it was taken is said in the
/// factory started from it, as it would be in this one.
const BRANCHES: &str = "branches";

/// What the copy of a project's own journal holds of this crate's that
/// whoever takes the copy is told of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Beside {
    /// Its tables that are kept by a store service, none of which has an
    /// address any more.
    pub stores: Vec<Unstored>,
    /// Its tables that are kept as files of a project, each with its
    /// project.
    pub as_files: Vec<Unstored>,
}

/// Removes from the copy of a project's own journal the studio's part of
/// what does not go into a copy, as [`leave_out`] does from the
/// factory's: of the tables such a journal keeps
/// ([`crate::apart::KEPT_APART`]), the address of a store service and
/// what a scan last saw of the project's files. A journal that keeps
/// none of them has nothing of the studio's.
pub fn leave_out_beside(connection: &Connection) -> rusqlite::Result<Beside> {
    let made = |table: &str| -> rusqlite::Result<bool> {
        connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
            [table],
            |row| row.get(0),
        )
    };
    if made("project_files")? {
        connection.execute("DELETE FROM project_files", [])?;
    }
    if !made("collections")? {
        return Ok(Beside::default());
    }
    connection.execute("UPDATE collections SET at = NULL WHERE store = 'service'", [])?;
    let kept_in = |store: &str| -> rusqlite::Result<Vec<Unstored>> {
        let mut statement = connection
            .prepare("SELECT project, name FROM collections WHERE store = ?1 ORDER BY project, name")?;
        let rows = statement
            .query_map([store], |row| Ok(Unstored { project: row.get(0)?, collection: row.get(1)? }))?;
        rows.collect()
    };
    Ok(Beside { stores: kept_in("service")?, as_files: kept_in("files")? })
}

impl Removed {
    /// Counts in what the copy of a project's own journal held.
    pub fn with(&mut self, beside: &Beside) {
        self.stores.extend(beside.stores.iter().cloned());
        self.stores
            .sort_by(|one, other| (&one.project, &one.collection).cmp(&(&other.project, &other.collection)));
    }
}

/// Says of the projects of a copy which of their tables are kept as
/// files, where a project's own journal keeps its tables and so the
/// factory's database could not say.
pub fn kept_beside(placed: &mut [Placed], beside: &[Unstored]) {
    for table in beside {
        if let Some(project) = placed.iter_mut().find(|project| project.id == table.project)
            && !project.kept_as_files.contains(&table.collection)
        {
            project.kept_as_files.push(table.collection.clone());
            project.kept_as_files.sort();
        }
    }
}

/// The ids of everything attached to a message or a record in a copy: each is a file
/// of that name beside the database.
pub fn attached(connection: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection.prepare("SELECT id FROM attachments ORDER BY created")?;
    let rows = statement.query_map([], |row| row.get(0))?;
    let mut ids: Vec<String> = rows.collect::<rusqlite::Result<_>>()?;
    // And to records, where the factory's own keeps a project's records.
    ids.extend(crate::attaching::attached_ids(connection)?);
    Ok(ids)
}
