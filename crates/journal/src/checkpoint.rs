//! The log brought back into the database, off the writer's turn.
//!
//! What is written goes into a log beside the database first, and is
//! brought back into the database itself now and then: copied over, and
//! both held to the disk. Left to the database, that is done by whoever
//! commits the write that makes the log long enough, on the one thread
//! that does everything: every thousand pages, the writer stopped for
//! two waits on the disk while all that was asked of it meanwhile waited
//! behind. Measured with a board of 120 cards worked by eight at once,
//! that was five sixths of what a write took, and the longest waits
//! anybody had (a quarter of a second and more) were these.
//!
//! So the writer does not do it. It is told how long the log is by each
//! commit ([`Log::watch`]), and when the log has grown by [`ASK`] pages
//! since it last said so it tells a thread of its own kind, with a
//! connection of its own, which brings back as much as it can without
//! holding anybody ([`Other`]). That is being told, and no timer: a
//! journal at rest tells nobody, and nothing is written.
//!
//! What a loss of power can take is what it was. The log is held to the
//! disk when it is brought back and not when a write is answered, so what
//! was answered since it was last brought back is what a loss of power,
//! or of the system, can take: about [`ASK`] pages of it, as before. A
//! daemon that is killed loses nothing: what it wrote is in the log.
//!
//! The other thread can bring back only what was there when it began,
//! and the log begins again only when all of it has been brought back
//! at a moment nothing is being written. Under writes that never pause
//! there is no such moment, and the log would grow for as long as they
//! went on. So at [`BOUND`] pages the writer brings back what is left
//! itself, between two pieces of work, where nothing is being written:
//! a wait on the disk once in that many pages, where it was once in a
//! thousand. It holds nobody for it either: where something else has the
//! log (a reader that began before, the other thread in the middle of
//! its own) it does what it can and looks again later.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;

use rusqlite::Connection;

use crate::disk::Disk;

/// How many pages the log grows by before the other thread is told to
/// bring it back. What the database itself went by, so that what a loss
/// of power can take is what it was.
pub(crate) const ASK: i64 = 1000;

/// How many pages long the log is when the writer brings back what is
/// left itself: 64 MB of it, with pages as they are made now. Reached
/// only where writes do not pause for as long as it takes to bring a
/// thousand pages back, or something holds the log.
pub(crate) const BOUND: i64 = 32_000;

/// How long a log is let grow, and when it is brought back.
#[derive(Clone, Copy, Debug)]
pub struct Kept {
    /// Pages it grows by before the other thread is told.
    pub ask: i64,
    /// Pages at which the writer brings back what is left itself.
    pub bound: i64,
}

impl Default for Kept {
    fn default() -> Kept {
        Kept { ask: ASK, bound: BOUND }
    }
}

/// What has gone into the log since the database was opened: how many
/// commits wrote to it, and how many frames (a page each) they wrote.
/// Counted from what each commit says of the log's length: a commit
/// that finds the log begun again wrote all of what it then says, and
/// one that does not wrote what the log grew by. The log begins again
/// only once a thousand pages and more have been brought back, so a
/// commit of more than that just after is the one that would be counted
/// short, and none writes so much. For whoever counts what something
/// costs the disk ([`crate::Db::commits`], [`crate::Db::logged`]).
#[derive(Default)]
pub(crate) struct Logged {
    pub(crate) commits: AtomicU64,
    pub(crate) frames: AtomicU64,
}

/// The writer's side: how long the log is, and who was told when.
pub(crate) struct Log {
    path: PathBuf,
    kept: Kept,
    /// Whether the disk is waited for, by this thread and by the other.
    disk: Disk,
    /// How long the log is, as its last commit said.
    pages: Arc<AtomicI64>,
    /// What has gone into it since the database was opened.
    logged: Arc<Logged>,
    /// How long it was when the other thread was last told.
    told: i64,
    /// How long it was when the writer last tried to bring it back.
    tried: i64,
    /// Set once the database is to do nothing more (`Db::hold`).
    held: Arc<AtomicBool>,
    /// How many times it has been brought back, by either thread.
    brought: Arc<AtomicU64>,
    other: Option<Other>,
}

/// The thread that brings the log back, and how it is told to.
struct Other {
    tell: mpsc::Sender<()>,
    thread: JoinHandle<()>,
}

impl Log {
    /// Has `connection` say how long the log is at each commit, and take
    /// no part in bringing it back unless asked ([`Log::look`]): what
    /// the database would have done for itself is put aside by this.
    pub(crate) fn watch(
        connection: &mut Connection,
        path: &Path,
        (kept, disk): (Kept, Disk),
        held: Arc<AtomicBool>,
        brought: Arc<AtomicU64>,
    ) -> rusqlite::Result<Log> {
        let pages: Arc<AtomicI64> = Arc::default();
        let logged: Arc<Logged> = Arc::default();
        let (said, counted) = (pages.clone(), logged.clone());
        connection.wal_hook(Some(move |_: &mut rusqlite::hooks::Wal, pages: std::ffi::c_int| {
            let (now, was) = (i64::from(pages), said.swap(i64::from(pages), Ordering::SeqCst));
            let wrote = if now >= was { now - was } else { now };
            counted.commits.fetch_add(1, Ordering::SeqCst);
            counted.frames.fetch_add(wrote.max(0) as u64, Ordering::SeqCst);
            Ok(())
        }))?;
        // A log that grew past the bound while something held it is cut
        // back to the bound when it next begins again.
        let limit = kept.bound.saturating_mul(i64::from(crate::db::PAGE));
        connection.pragma_update(None, "journal_size_limit", limit)?;
        let (told, tried, other) = (0, 0, None);
        Ok(Log { path: path.to_path_buf(), kept, disk, pages, logged, told, tried, held, brought, other })
    }

    /// How long the log is, in pages, as its last commit said.
    pub(crate) fn pages(&self) -> Arc<AtomicI64> {
        self.pages.clone()
    }

    /// What has gone into it since the database was opened.
    pub(crate) fn logged(&self) -> Arc<Logged> {
        self.logged.clone()
    }

    /// Asked by the writer between two pieces of work: tells the other
    /// thread where the log has grown enough, and brings back what is
    /// left itself where it has reached the bound.
    pub(crate) fn look(&mut self, connection: &Connection) {
        let pages = self.pages.load(Ordering::SeqCst);
        // Shorter than it was: it began again since.
        if pages < self.told {
            self.told = 0;
        }
        if pages < self.tried {
            self.tried = 0;
        }
        if pages >= self.kept.bound && pages - self.tried >= self.kept.ask.min(self.kept.bound) {
            self.tried = pages;
            self.told = pages;
            // As much as can be without waiting for anybody. Where all
            // of it is brought back, the next write begins the log again.
            if bring_back(connection) {
                self.brought.fetch_add(1, Ordering::SeqCst);
            }
        } else if pages - self.told >= self.kept.ask {
            self.told = pages;
            self.tell();
        }
    }

    /// Tells the other thread, starting it the first time: a database
    /// that never writes a thousand pages never has one.
    fn tell(&mut self) {
        if self.other.is_none() {
            let (tell, told) = mpsc::channel::<()>();
            let (path, held, brought) = (self.path.clone(), self.held.clone(), self.brought.clone());
            let disk = self.disk;
            let thread = std::thread::Builder::new()
                .name("journal-log".into())
                .spawn(move || other(&path, &told, &held, &brought, disk));
            // With no thread to be had the writer does it at the bound.
            let Ok(thread) = thread else { return };
            self.other = Some(Other { tell, thread });
        }
        if let Some(other) = &self.other {
            let _ = other.tell.send(());
        }
    }

    /// Ends the other thread and waits for it to have let the file go:
    /// asked by the writer as it ends, before it lets go itself.
    pub(crate) fn end(&mut self) {
        if let Some(Other { tell, thread }) = self.other.take() {
            drop(tell);
            let _ = thread.join();
        }
    }
}

/// Brings back as much of the log as can be without waiting for anybody.
/// True where something was done; where something else has the log just
/// now, nothing is, and that is no fault.
fn bring_back(connection: &Connection) -> bool {
    connection
        .query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| row.get::<_, i64>(0))
        .is_ok_and(|busy| busy == 0)
}

/// The other thread: a connection of its own to the same file, which
/// brings the log back each time it is told, until nobody can tell it
/// any more.
fn other(path: &Path, told: &mpsc::Receiver<()>, held: &AtomicBool, brought: &AtomicU64, disk: Disk) {
    let Ok(connection) = Connection::open(path) else { return };
    // It waits for nobody, and holds the disk as the writer's does.
    let as_the_writer = format!("PRAGMA busy_timeout = 0; PRAGMA synchronous = {};", disk.synchronous());
    if connection.execute_batch(&as_the_writer).is_err() {
        return;
    }
    while told.recv().is_ok() {
        // Told several times while it was at it: once is enough.
        while told.try_recv().is_ok() {}
        // A database that is to do nothing more is not written to here either.
        if held.load(Ordering::SeqCst) {
            continue;
        }
        if bring_back(&connection) {
            brought.fetch_add(1, Ordering::SeqCst);
        }
    }
}
