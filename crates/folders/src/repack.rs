//! A factory from before, given the database one is made with now.
//!
//! What a database costs the disk at every write is decided when it is
//! made (`journal::rewrite` says what, and why it is not changed by
//! opening one). A factory made by an earlier program goes on exactly as
//! it was until its owner asks for this: the whole database written out
//! again beside itself and put in its place. It is asked of a factory
//! that is stopped, and holds the factory's folder as a daemon would
//! while it is done, so none is started in the middle of it. Before it
//! begins it says how long it takes and how much room it needs, and where
//! there is not the room it does nothing.
//!
//! What was done is kept in the database and said once by the daemon that
//! next starts on it (`system.repacked`), since nothing was running to
//! say so when it happened.

use std::path::{Path, PathBuf};

use journal::rewrite::{Done, Plan};
use kernel::{Error, Result, Source};
use serde_json::json;

use crate::DATABASE_FILE;

/// Where what was done is kept until a daemon says it.
const REMEMBERED: (&str, &str) = ("factory", "repacked");

/// What a factory is held for here, said where it is running.
const HELD_FOR: &str = "its database is written out again";

/// What writing a factory's database out again would come to.
#[derive(Clone, Debug)]
pub struct Looked {
    pub database: PathBuf,
    pub plan: Plan,
    /// How much room is left on the disk it is on, in bytes, where that
    /// can be told.
    pub free: Option<u64>,
}

impl Looked {
    /// Why it would not be begun, where there is not the room.
    pub fn refused(&self) -> Option<String> {
        self.free.and_then(|free| self.plan.refused(free))
    }
}

/// The folder of a factory, held as its daemon holds it for as long as
/// what is given back is kept: nothing else is running on it, and nothing
/// is started. `what` is what is done only while it is stopped, for the
/// sentence that says so where it is running.
pub(crate) fn hold(data: &Path, what: &str) -> Result<std::fs::File> {
    if !crate::arrival::is_factory(data) {
        return Err(Error::not_found(format!("there is no factory at {}", data.display())));
    }
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(data.join(crate::folder::LOCK))?;
    // A lock outlives its holder for an instant in any child that was
    // being started as the holder exited, so one that is held is waited
    // for as a starting node waits for it, before it is taken to mean
    // that a daemon really is running.
    let began = std::time::Instant::now();
    while lock.try_lock().is_err() {
        if began.elapsed() >= crate::LOCK_PATIENCE {
            return Err(Error::conflict(format!(
                "the factory at {} is running, and {what} only while it is \
                 stopped: stop it first (`{cli} down`), and start it again afterwards",
                data.display(),
                cli = brand::CLI
            )));
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    // No daemon has the folder, so none is serving it, whatever one that
    // was killed left saying so: read while this holds the folder, it
    // would be taken for this.
    crate::folder::forget(data);
    Ok(lock)
}

fn looked(data: &Path) -> Result<Looked> {
    let database = data.join(DATABASE_FILE);
    let plan = journal::rewrite::look(&database)?;
    let free = hosting::resources(data)
        .ok()
        .and_then(|found| found.free_disk_mb)
        .map(|mb| mb.saturating_mul(1024 * 1024));
    Ok(Looked { database, plan, free })
}

/// The files of the journals kept beside the factory's own at `data`:
/// one for each project that has a journal of its own, as the factory's
/// database lists them. Read with nothing running, the folder held.
fn beside(data: &Path) -> Result<Vec<PathBuf>> {
    let wrong = |err| journal::db_error(err).context("reading which projects have a journal of their own");
    let connection = journal::copy::Connection::open(data.join(DATABASE_FILE)).map_err(wrong)?;
    let listed = journal::listed(&connection).map_err(wrong)?;
    let folder = data.join(crate::snapshot::JOURNALS);
    Ok(listed.into_iter().map(|listed| folder.join(listed.file)).filter(|file| file.is_file()).collect())
}

/// What writing the database of the factory at `data` out again would
/// take. Nothing is changed by asking; a factory that is running is not
/// asked about.
pub fn look(data: &Path) -> Result<Looked> {
    let _held = hold(data, HELD_FOR)?;
    looked(data)
}

/// Writes the database of the factory at `data` out again as one is made
/// now, having been shown what it would take (`begin` is told, and what
/// it prints is what whoever asked reads before anything is done).
/// `None` where there was nothing to do.
pub fn run(data: &Path, begin: impl FnOnce(&Looked)) -> Result<Option<Done>> {
    let _held = hold(data, HELD_FOR)?;
    let asked = looked(data)?;
    begin(&asked);
    let mut done = None;
    if asked.plan.wanted() {
        if let Some(why) = asked.refused() {
            return Err(Error::conflict(why));
        }
        done = Some(journal::rewrite::rewrite(&asked.database)?);
    }
    // And the journal of each project that has one of its own, beside
    // the factory's: each is a database made as one is made now, since
    // only this program makes them, so there is nothing to do for one
    // unless that is no longer so.
    for beside in beside(data)? {
        if journal::rewrite::look(&beside)?.wanted() {
            let more = journal::rewrite::rewrite(&beside)?;
            done = Some(match done {
                None => more,
                Some(done) => Done {
                    before: done.before + more.before,
                    after: done.after + more.after,
                    events: done.events + more.events,
                    took_ms: done.took_ms + more.took_ms,
                },
            });
        }
    }
    let Some(done) = done else { return Ok(None) };
    // Kept for the daemon that next starts here to say.
    let connection = journal::copy::Connection::open(&asked.database)
        .map_err(|err| journal::db_error(err).context("repack"))?;
    let said = json!({
        "before": done.before, "after": done.after, "events": done.events, "took_ms": done.took_ms,
    });
    journal::copy::note(&connection, REMEMBERED.0, REMEMBERED.1, &said)
        .map_err(|err| journal::db_error(err).context("repack"))?;
    Ok(Some(done))
}

/// Takes away what a `repack` that was cut off left beside the database:
/// a copy of it, as far as it had been written, which may be as big as
/// the database and is nothing to anybody. Only `repack` makes it, under
/// the folder's lock, and whoever calls this holds that lock: the daemon,
/// as it starts, before it opens the database. How big it was, where
/// there was one.
pub fn clear(data: &Path) -> Option<u64> {
    // Beside the factory's own database, and beside the journal of any
    // of its projects: whatever is in their folder under that name.
    let mut left = vec![journal::rewrite::beside(&data.join(DATABASE_FILE))];
    let journals = std::fs::read_dir(data.join(crate::snapshot::JOURNALS)).into_iter().flatten().flatten();
    left.extend(
        journals
            .map(|entry| entry.path())
            .filter(|file| {
                file.extension().is_some_and(|ending| ending == "db")
                    && journal::rewrite::beside(file).is_file()
            })
            .map(|file| journal::rewrite::beside(&file)),
    );
    let mut cleared = None;
    for left in left {
        if let Ok(bytes) = left.metadata().map(|file| file.len())
            && std::fs::remove_file(&left).is_ok()
        {
            cleared = Some(cleared.unwrap_or(0) + bytes);
        }
    }
    cleared
}

/// Says that it was taken away, in the journal of the factory it was
/// beside.
pub async fn announce_cleared(journal: &journal::Journal, bytes: Option<u64>) {
    if let Some(bytes) = bytes {
        let _ =
            journal.emit("system.repack_cleared", Source::actor("system"), json!({ "bytes": bytes })).await;
    }
}

/// Says, once, in the factory's own journal, that the index of when its
/// events were said was made anew as this program first opened it, and
/// measures how long that took: the one thing done to a journal from
/// before without its owner asking, since it takes a second or so.
pub async fn announce_reindexed(journal: &journal::Journal) {
    let (namespace, key) = journal::REINDEXED;
    let Some(done) = journal.kv_get(namespace, key).await.ok().flatten() else { return };
    let took = done["took_ms"].as_u64();
    if let Some(took) = took {
        journal.measure("db.reindex", &[], took as f64, true);
    }
    let said = took.map_or_else(|| json!({}), |took| json!({ "took_ms": took }));
    let _ = journal.emit("system.reindexed", Source::actor("system"), said).await;
    let _ = journal.kv_set(namespace, key, &serde_json::Value::Null).await;
}

/// Says, once, in the factory's own journal, that its database was
/// written out again while it was stopped, and measures how long it took.
pub async fn announce(journal: &journal::Journal) {
    let Some(done) = journal.kv_get(REMEMBERED.0, REMEMBERED.1).await.ok().flatten() else { return };
    let took = done["took_ms"].as_f64().unwrap_or_default();
    journal.measure("db.repack", &[], took, true);
    let _ = journal.emit("system.repacked", Source::actor("system"), done).await;
    let _ = journal.kv_set(REMEMBERED.0, REMEMBERED.1, &serde_json::Value::Null).await;
}
