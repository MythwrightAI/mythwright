//! Writing a database from before out again as one is made now.
//!
//! Two things about a database are decided when it is made and cost the
//! disk at every write afterwards: how big its pages are ([`crate::db`]),
//! and whether the database numbers the events ([`crate::numbered`]). A
//! database made now has small pages and events the journal numbers; one
//! from before goes on as it was made, and is given both only by having
//! all of it written out again. That takes as long as the file is big and
//! as much room again while it is done, so it is never done by opening a
//! database: its owner asks, is told what it will take, and it is done
//! while nothing has the database open.
//!
//! It is written beside the one there is, whole, and put in its place in
//! one move once it is all there and was read back. Cut off at any
//! moment before that, the database is as it was and what was written
//! beside it is taken away the next time; nothing is ever changed in
//! place. Every table goes as it is, with every row, index and number:
//! the only table made otherwise is the events', which the database no
//! longer numbers.

use std::path::{Path, PathBuf};

use kernel::{Error, Result};
use rusqlite::Connection;

use crate::db::{PAGE, db_error};
use crate::numbered;

/// How much of a database is written out again in a second, in bytes, for
/// saying about how long it takes. Measured, a database of 296 MB that
/// was nearly all events took 2.7 s on the machine this was written on,
/// with other work going on beside it; this is a third of that, so that a
/// slower disk is not promised what it cannot do.
const A_SECOND: u64 = 40 * 1024 * 1024;

/// What writing a database out again comes to, said before it is begun.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// How big the database is now, with its log, in bytes.
    pub bytes: u64,
    /// How big its pages are, in bytes.
    pub page: u32,
    /// Whether the database numbers its events.
    pub numbered: bool,
}

impl Plan {
    /// Whether there is anything to do: one made now, or written out
    /// before, is as it would be afterwards.
    pub fn wanted(&self) -> bool {
        self.page != PAGE || self.numbered
    }

    /// How much room it needs while it is done, in bytes: all of it
    /// again, since what is written is about as big, and a tenth over.
    pub fn needs(&self) -> u64 {
        self.bytes + self.bytes / 10
    }

    /// About how long it takes, in seconds.
    pub fn seconds(&self) -> u64 {
        (self.bytes / A_SECOND).max(1)
    }

    /// Why it is not begun where `free` bytes are left on the disk, or
    /// nothing where there is the room.
    pub fn refused(&self, free: u64) -> Option<String> {
        let mb = |bytes: u64| bytes.div_ceil(1024 * 1024);
        (free < self.needs()).then(|| {
            format!(
                "there is not the room: writing it out again needs {} MB beside it while it is done, and {} MB \
                 are free. Nothing was changed",
                mb(self.needs()),
                mb(free)
            )
        })
    }
}

/// What writing a database out again came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Done {
    /// How big it was, with its log, and how big it is, in bytes.
    pub before: u64,
    pub after: u64,
    /// How many events it holds, as it did.
    pub events: u64,
    pub took_ms: u64,
}

fn text(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| {
        Error::invalid(format!(
            "{} cannot be written as text, which a database's place has to be",
            path.display()
        ))
    })
}

/// A file that goes with the database at `path`: its log, or what is
/// written beside it.
fn with(path: &Path, ending: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(ending);
    PathBuf::from(name)
}

/// Where the database at `path` is written out again, beside it, until
/// it is whole: what a rewrite that was cut off leaves, and nothing else
/// makes.
pub fn beside(path: &Path) -> PathBuf {
    with(path, ".rewriting")
}

fn size(path: &Path) -> u64 {
    path.metadata().map_or(0, |file| file.len())
}

fn open(path: &Path) -> Result<Connection> {
    if !path.is_file() {
        return Err(Error::not_found(format!("there is no database at {}", path.display())));
    }
    let wrong = |err: rusqlite::Error| db_error(err).context(path.display());
    let connection = Connection::open(path).map_err(wrong)?;
    connection.busy_timeout(std::time::Duration::from_secs(5)).map_err(wrong)?;
    Ok(connection)
}

/// What writing the database at `path` out again would come to. Nothing
/// is changed by asking.
pub fn look(path: &Path) -> Result<Plan> {
    let connection = open(path)?;
    let wrong = |err: rusqlite::Error| {
        db_error(err).context(format!("{} is not a factory's database", path.display()))
    };
    let page: u32 = connection.query_row("PRAGMA page_size", [], |row| row.get(0)).map_err(wrong)?;
    if numbered::unnumbered(&connection, "main").map_err(wrong)?.is_none() {
        return Err(Error::invalid(format!(
            "{} is not a factory's database: it keeps no events",
            path.display()
        )));
    }
    let numbered = numbered::by_the_database(&connection, "main").map_err(wrong)?;
    Ok(Plan { bytes: size(path) + size(&with(path, "-wal")), page, numbered })
}

/// Writes the database at `path` out again as one is made now, and puts
/// it in its place. Nothing else may have the database open: whoever
/// asks holds whatever says so. Cut off, or refused, it leaves the
/// database as it was.
pub fn rewrite(path: &Path) -> Result<Done> {
    let began = std::time::Instant::now();
    let before = size(path) + size(&with(path, "-wal"));
    let beside = beside(path);
    // What an earlier one that was cut off left.
    let _ = std::fs::remove_file(&beside);
    let events = write_out(path, &beside).and_then(|events| {
        // Everything it held is in the one file by now, and its log is
        // empty. One that is not was written to meanwhile, by something
        // that should not have had it open.
        if size(&with(path, "-wal")) > 0 {
            return Err(Error::conflict(format!(
                "{} was written to while it was being written out again: something else has it open. \
                 Nothing was changed",
                path.display()
            )));
        }
        // On the disk before it is the database, and not only on its way.
        // (Through a handle that may write: not every system puts a file
        // on the disk for one that may only read.)
        std::fs::OpenOptions::new().write(true).open(&beside)?.sync_all()?;
        Ok(events)
    });
    let events = match events {
        Ok(events) => events,
        Err(err) => {
            let _ = std::fs::remove_file(&beside);
            return Err(err);
        }
    };
    // Who may read the database is as it was: what takes its place is
    // given the permissions of the one before, whatever a new file gets.
    if let Ok(was) = path.metadata() {
        std::fs::set_permissions(&beside, was.permissions())?;
    }
    std::fs::rename(&beside, path)?;
    // The log of the one that was there says nothing of this one.
    for ending in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(with(path, ending));
    }
    #[cfg(unix)]
    if let Some(folder) = path.parent() {
        let _ = std::fs::File::open(folder).and_then(|folder| folder.sync_all());
    }
    Ok(Done { before, after: size(path), events, took_ms: began.elapsed().as_millis() as u64 })
}

/// Writes everything the database at `path` holds into a new one at
/// `beside`, and says how many events that was.
fn write_out(path: &Path, beside: &Path) -> Result<u64> {
    let wrong = |err: rusqlite::Error| db_error(err).context(format!("writing {} out again", path.display()));
    let in_use = || {
        Error::conflict(format!(
            "{} is in use: something else has it open. Nothing was changed",
            path.display()
        ))
    };
    // Its log is folded back first, so that what is read below is one
    // file. Then its turn to write is taken and kept until what was
    // written out is whole: whoever asked holds the factory's folder, and
    // this is what holds the database itself, so that nothing written to
    // it meanwhile (by a daemon still on its way out) could be left
    // behind in the one that is replaced.
    let old = open(path)?;
    let busy: i64 = old.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0)).map_err(wrong)?;
    if busy != 0 {
        return Err(in_use());
    }
    old.execute_batch("BEGIN IMMEDIATE;").map_err(|_| in_use())?;
    let mut new = Connection::open(beside).map_err(wrong)?;
    // Nothing of it counts until it is whole, so it keeps no journal of
    // its own and waits for no disk: it is put on the disk once, at the end.
    new.execute_batch(&format!(
        "PRAGMA page_size = {PAGE};
         PRAGMA journal_mode = OFF;
         PRAGMA synchronous = OFF;"
    ))
    .map_err(wrong)?;
    new.execute("ATTACH DATABASE ?1 AS old", [text(path)?]).map_err(wrong)?;
    let Some(events) = numbered::unnumbered(&new, "old").map_err(wrong)? else {
        return Err(Error::invalid(format!(
            "{} is not a factory's database: it keeps no events",
            path.display()
        )));
    };
    let last = numbered::last(&new, "old").map_err(wrong)?;
    let listed = |connection: &Connection, tables: bool| -> rusqlite::Result<Vec<(String, String)>> {
        let which = if tables { "type = 'table'" } else { "type != 'table'" };
        let mut statement = connection.prepare(&format!(
            "SELECT name, sql FROM old.sqlite_master
             WHERE {which} AND sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY rowid"
        ))?;
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect()
    };
    let transaction = new.transaction().map_err(wrong)?;
    let tables = listed(&transaction, true).map_err(wrong)?;
    for (name, made) in &tables {
        // Every table as it is, but the events', which the database
        // numbers no longer; and every row of each, in its order.
        let made = if name == "events" { &events } else { made };
        transaction.execute_batch(made).map_err(wrong)?;
        transaction
            .execute(&format!("INSERT INTO main.\"{name}\" SELECT * FROM old.\"{name}\""), [])
            .map_err(wrong)?;
    }
    // What is found by is made once the rows are there, which is quicker
    // and leaves each as small as it can be; but for what a journal made
    // now does not keep (`events::UNKEPT`).
    for (name, made) in listed(&transaction, false).map_err(wrong)? {
        if !crate::events::UNKEPT.contains(&name.as_str()) {
            transaction.execute_batch(&made).map_err(wrong)?;
        }
    }
    // The database's own count of the events does not go; what it was
    // for does: no number is given again.
    numbered::keep_said(&transaction, last).map_err(wrong)?;
    for (name, _) in &tables {
        let same: bool = transaction
            .query_row(
                &format!(
                    "SELECT (SELECT count(*) FROM main.\"{name}\") = (SELECT count(*) FROM old.\"{name}\")"
                ),
                [],
                |row| row.get(0),
            )
            .map_err(wrong)?;
        // `kv` may have gained the one row written just above.
        if !same && name != "kv" {
            return Err(Error::internal(format!(
                "`{name}` did not come out as it went in, so {} is left as it was",
                path.display()
            )));
        }
    }
    let held: i64 =
        transaction.query_row("SELECT count(*) FROM main.events", [], |row| row.get(0)).map_err(wrong)?;
    transaction.commit().map_err(wrong)?;
    new.execute_batch("DETACH DATABASE old;").map_err(wrong)?;
    let sound: String = new.query_row("PRAGMA quick_check", [], |row| row.get(0)).map_err(wrong)?;
    if sound != "ok" {
        return Err(Error::internal(format!(
            "what was written out did not read back sound ({sound}), so {} is left as it was",
            path.display()
        )));
    }
    new.close().map_err(|(_, err)| wrong(err))?;
    // Let go of with nothing written to it, and closed before the other
    // takes its place: closing it tidies its own log away, by name.
    old.execute_batch("ROLLBACK;").map_err(wrong)?;
    old.close().map_err(|(_, err)| wrong(err))?;
    Ok(held as u64)
}
