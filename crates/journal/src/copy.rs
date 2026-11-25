//! A copy of the database, to start another factory from.
//!
//! Factories share nothing, and nothing copies one into another by itself.
//! A person may ask for a copy of one, to start a new one from: the copy is
//! a file of its own, taken whole at one moment, and everything that must
//! not travel is then removed from the copy and never from the database it
//! was taken from.
//!
//! This module is the database's part of that, and every crate that keeps
//! tables has a `copy` module of its own beside it that says what of its
//! tables does not go. None of it runs on the database thread: the copy is
//! taken on a second connection that only reads, which in WAL mode holds
//! nobody up, and is changed on a connection of its own. All of it blocks,
//! and belongs off the async threads.
//!
//! A factory is several files, its own and one for each project that
//! has a journal of its own, and a copy of it is all of them as of the
//! same moment: that is [`crate::moment`], which reads each as this does
//! and stands every journal's thread still for the instant the reads
//! are begun in. [`take`] is one file by itself.
//!
//! What of the journal's own goes and does not:
//!
//! - **events** go as they are. They are what happened, and the sequence
//!   numbers the rest of the copy refers to. With them goes how far their
//!   numbers have been given: the giver's mark ([`crate::giver`]) and
//!   the highest number written down as events were taken out, so that
//!   a factory started from the copy gives none of them again. What it
//!   gives is above what the copy came with; it and the factory it was
//!   copied from share nothing, their numbers neither.
//! - **measurements** do not: the new factory's start clean.
//! - **small durable state** (`kv`) is mostly places in the journal and
//!   things remembered about what was running. Each crate removes its own,
//!   and what is left is held to a list of what is known to travel
//!   ([`keep_state`]), so that a cursor somebody adds later is not copied
//!   by being forgotten here.

use std::path::Path;

use kernel::{Error, Result};
use rusqlite::OpenFlags;
use serde_json::Value;

use crate::db::db_error;

pub use rusqlite::Connection;

/// The tables this crate keeps, by the name they are migrated under, and
/// the version of each this build knows.
pub const KNOWS: &[(&str, usize)] = &[("journal", crate::SCHEMA.len())];

/// The tables this crate keeps, each of which it has said goes into a copy
/// or is emptied in one. A table that no crate lists is emptied
/// ([`keep_tables`]): one added later does not travel by being forgotten.
pub const TABLES: &[&str] = &["schema_versions", "events", "kv", "metrics", "journals"];

fn text(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| {
        Error::invalid(format!(
            "{} cannot be written as text, which a database's place has to be",
            path.display()
        ))
    })
}

/// Writes the database at `from` to `to`, as it was at one moment, whatever
/// is being written to it meanwhile. `to` must not exist.
///
/// It only reads `from`, on a connection of its own: the thread that does
/// the factory's work is not asked for anything, and waits for nothing.
/// One file by itself: the files of a factory, each as of the same
/// moment, are read by [`crate::Journal::moment`].
pub fn take(from: &Path, to: &Path) -> Result<()> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let connection =
        Connection::open_with_flags(from, flags).map_err(|err| db_error(err).context(from.display()))?;
    connection.busy_timeout(std::time::Duration::from_secs(5)).map_err(db_error)?;
    connection
        .execute("VACUUM INTO ?1", [text(to)?])
        .map_err(|err| db_error(err).context(format!("copying {} to {}", from.display(), to.display())))?;
    Ok(())
}

/// Opens a copy, to take out of it what does not travel. It must exist.
///
/// A copy is kept with a rollback journal and not a write-ahead log, so
/// that when it is closed there is one file and nothing beside it.
pub fn open(path: &Path) -> Result<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let wrong =
        |err: rusqlite::Error| db_error(err).context(format!("{} is not a usable database", path.display()));
    let connection = Connection::open_with_flags(path, flags).map_err(wrong)?;
    connection.pragma_update(None, "journal_mode", "DELETE").map_err(wrong)?;
    connection.pragma_update(None, "secure_delete", "ON").map_err(wrong)?;
    connection
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
        .map_err(wrong)?;
    Ok(connection)
}

/// Closes a copy so that what was removed from it is gone from the file,
/// and not only from its tables: a removed row otherwise stays in a page
/// that is no longer used, for anyone who reads the file's bytes.
pub fn seal(connection: Connection) -> Result<()> {
    connection.execute_batch("VACUUM;").map_err(db_error)?;
    connection.close().map_err(|(_, err)| db_error(err))
}

/// The version each crate's tables are at in a copy, by the name they are
/// migrated under.
pub fn versions(connection: &Connection) -> rusqlite::Result<Vec<(String, i64)>> {
    let mut statement =
        connection.prepare("SELECT namespace, version FROM schema_versions ORDER BY namespace")?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

/// Removes the journal's own part of what does not go into a copy: the
/// measurements, and what this factory has yet to say of its own journal
/// (that its index of when was made anew), which is not so of the copy's.
/// Returns how many rows that was.
pub fn leave_out(connection: &Connection) -> rusqlite::Result<usize> {
    let noted = connection.execute(
        "DELETE FROM kv WHERE namespace = ?1 AND key = ?2",
        [crate::REINDEXED.0, crate::REINDEXED.1],
    )?;
    Ok(noted + connection.execute("DELETE FROM metrics", [])?)
}

/// Removes everything a copy has kept under `namespace` of the small
/// durable state: a place in the journal, or something remembered about
/// what was running, that is not true of a new factory.
pub fn forget(connection: &Connection, namespace: &str) -> rusqlite::Result<usize> {
    connection.execute("DELETE FROM kv WHERE namespace = ?1", [namespace])
}

/// Holds the small durable state of a copy to what is known to travel:
/// every namespace not in `kept` is removed. Each crate removes its own
/// with [`forget`]; this is for the ones nobody thought of.
pub fn keep_state(connection: &Connection, kept: &[&str]) -> rusqlite::Result<usize> {
    let mut statement = connection.prepare("SELECT DISTINCT namespace FROM kv")?;
    let found: Vec<String> = statement.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    let mut removed = 0;
    // The journal's own stays: how far the giver of numbers has given and
    // the highest number an event was given, which go with the events.
    let own = [crate::numbered::SAID.0, crate::giver::DRAWN.0];
    let kept = |namespace: &str| kept.contains(&namespace) || own.contains(&namespace);
    for namespace in found.iter().filter(|namespace| !kept(namespace)) {
        removed += forget(connection, namespace)?;
    }
    Ok(removed)
}

/// Empties every table of a copy that is not in `known`, and says which
/// those were. The tables stay, as the crates that made them expect; what
/// was in them does not travel until its crate says that it does.
pub fn keep_tables(connection: &Connection, known: &[&str]) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")?;
    let tables: Vec<String> = statement.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    let mut emptied = Vec::new();
    for table in tables.into_iter().filter(|table| !known.contains(&table.as_str())) {
        connection.execute(&format!("DELETE FROM \"{}\"", table.replace('"', "\"\"")), [])?;
        emptied.push(table);
    }
    Ok(emptied)
}

/// Writes a small durable value into a copy, as [`crate::Journal::kv_set`]
/// writes one into the database of a running factory.
pub fn note(connection: &Connection, namespace: &str, key: &str, value: &Value) -> rusqlite::Result<()> {
    connection
        .execute(
            "INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3)
             ON CONFLICT (namespace, key) DO UPDATE SET value = excluded.value",
            [namespace, key, &value.to_string()],
        )
        .map(|_| ())
}
