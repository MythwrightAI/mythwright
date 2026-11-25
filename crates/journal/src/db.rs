//! The database, behind one thread.
//!
//! SQLite allows one writer at a time, so rather than a pool that queues on a
//! lock nobody can see, there is exactly one connection on one thread and
//! every operation is a closure sent to it. Order is the order of arrival,
//! nothing is ever "busy", and a slow query shows up as queue time that can be
//! measured rather than as a mystery.
//!
//! The thread has a transaction open for each piece of work, begun as one
//! that writes, and where it has fallen behind keeps what is already
//! waiting in the same one ([`crate::gather`]): that work is committed
//! together, and answered once it is. And the log its commits go into is
//! brought back into the database by another thread
//! ([`crate::checkpoint`]).
//!
//! Each crate owns its own tables and declares them with [`Db::migrate`]
//! under its own namespace, so adding a table to one crate never touches
//! another ([`crate::steps`]).
//!
//! It is measured: every piece of work is told to whoever asked to be told
//! ([`Db::measured`]) as a [`Turn`]: whose it was, whether it wrote, how
//! long it waited for the work ahead of it and how long it then took. That
//! is what says whether one crate's work is being held up by another's.

use std::panic::Location;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::Instant;

use kernel::{Error, ErrorKind, Result};
use rusqlite::Connection;
use tokio::sync::{oneshot, watch};

use crate::checkpoint::{Kept, Log, Logged};
use crate::disk::Disk;

/// One piece of work the database thread did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turn {
    /// Whose work it was: the crate whose code asked for it, by the folder
    /// it is kept in. The journal's own work for another crate (writing an
    /// event down, say) is the journal's.
    pub by: &'static str,
    /// Whether it changed anything, or only read.
    pub wrote: bool,
    /// How long it waited for the work ahead of it, in milliseconds.
    pub waited_ms: f64,
    /// How long it then took, in milliseconds.
    pub took_ms: f64,
    /// How many pieces of work the commit that kept it kept, itself
    /// counted: one, unless the thread had fallen behind
    /// ([`crate::gather`]).
    pub shared: usize,
    /// How long it was done before its commit was made and it was
    /// answered, in milliseconds: what it waited for the others its commit
    /// kept.
    pub held_ms: f64,
    /// Whether it worked. Looking for something that is not there is not a failure.
    pub ok: bool,
}

type Meter = Box<dyn Fn(Turn) + Send + Sync>;

/// Handing back what a piece of work came to, to whoever asked for it:
/// or, where what it wrote could not be kept, why not.
type Answer = Box<dyn FnOnce(Option<Error>) + Send>;

/// A piece of work for the database thread: it says whether it worked, and
/// gives back how its answer is handed over.
type Work = Box<dyn FnOnce(&mut Connection) -> (bool, Answer) + Send>;

/// Work waiting its turn: when it was asked for, where from, and the work,
/// which says whether it worked and gives back how its answer is handed
/// over. The answer is handed over after the work has been told of, so
/// that whoever asked never hears of it before whoever measures has.
struct Job {
    asked: Instant,
    from: &'static Location<'static>,
    work: Work,
    /// Done even while the database is held ([`Db::hold`]).
    though_held: bool,
    /// Done by itself, in no commit shared with anything
    /// ([`Db::call_apart`]).
    apart: bool,
    /// Its commit is held to the disk before it is answered
    /// ([`Db::call_to_the_disk`]), and is a commit of its own.
    to_disk: bool,
}

/// How a piece of work is done, beyond being done in its turn.
#[derive(Clone, Copy, Default)]
pub(crate) struct How {
    pub(crate) though_held: bool,
    pub(crate) apart: bool,
    pub(crate) to_disk: bool,
}

/// A piece of work that was done and waits for its commit: what is told
/// of it, and how its answer is handed over.
struct Done {
    from: &'static Location<'static>,
    waited_ms: f64,
    took_ms: f64,
    /// When it was done.
    at: Instant,
    ok: bool,
    changed: bool,
    answer: Answer,
}

#[derive(Clone)]
pub struct Db {
    jobs: mpsc::Sender<Job>,
    meter: Arc<OnceLock<Meter>>,
    /// Set by the work that closes it ([`Db::close`]), on its own thread.
    closed: Arc<AtomicBool>,
    /// While set, nothing that is asked is done ([`Db::hold`]).
    held: Arc<AtomicBool>,
    /// Says so once the thread has ended and let the file go ([`Db::closed`]).
    gone: watch::Receiver<bool>,
    /// How many pieces of work have changed anything ([`Db::writes`]).
    writes: Arc<AtomicU64>,
    /// How many pieces of work have been done at all ([`Db::done`]).
    done: Arc<AtomicU64>,
    /// How many transactions its thread has begun ([`Db::begun`]).
    begun: Arc<AtomicU64>,
    /// How many pieces of work are waiting to be begun: asked, and not
    /// yet taken up by the thread.
    waiting: Arc<AtomicUsize>,
    /// How long its log is, in pages, as its last commit said
    /// ([`Db::log_pages`]). Nothing, for one that has no log.
    log: Arc<AtomicI64>,
    /// How many times its log has been brought back ([`Db::brought_back`]).
    brought: Arc<AtomicU64>,
    /// What has gone into its log ([`Db::commits`], [`Db::logged`]).
    logged: Arc<Logged>,
    /// How many commits were held to the disk before they were answered
    /// ([`Db::kept_to_the_disk`]).
    synced: Arc<AtomicU64>,
    /// Which file it is in and what its thread is at ([`Db::standing`]).
    at: Arc<At>,
    /// Whether it waits for the disk ([`Db::disk`]).
    disk: Disk,
}

/// Where a database is, and what its thread is at: for whoever waits
/// for the thread to say what was waited for, where it did not come.
struct At {
    /// The file the database is in; none for one in memory.
    file: Option<PathBuf>,
    /// What the thread is doing, for a piece of work where it was asked,
    /// and since when.
    doing: Mutex<(&'static str, Option<&'static Location<'static>>, Instant)>,
}

impl At {
    fn now(&self, doing: &'static str, asked: Option<&'static Location<'static>>) {
        *self.doing.lock().expect("database lock") = (doing, asked, Instant::now());
    }
}

/// The crate a source file is in, by the folder it is kept in under
/// `crates`. Anything kept elsewhere is `other`.
fn whose(file: &'static str) -> &'static str {
    // A run of numbers drawn ahead for a journal kept in another file is
    // the giver's, and falls wherever half a run has been given: it is
    // told apart from what the journal does for whoever asks.
    if file.ends_with("giver.rs") {
        return "giver";
    }
    let mut parts = file.split(['/', '\\']);
    while let Some(part) = parts.next() {
        if part == "crates" {
            return parts.next().unwrap_or("other");
        }
    }
    "other"
}

/// How big a page of a new database is, in bytes.
///
/// What a write costs the disk is whole pages: a row or two is changed, and
/// every page they and their places in the indexes are on is written out
/// again, whole. Nearly everything written here is that small (an event, a
/// step, a run), so the size of a page is what most writes cost, several
/// times over. At half the size the database usually takes, a piece of
/// work taken through a fleet had the disk written two fifths less, for
/// the same processor time and a file no bigger. Nothing is kept less
/// safely: what is written, and when it is said to be kept, are as they
/// were. A value longer than a page is spread over more of them, and a
/// table of many millions of rows is a level deeper to look into.
pub(crate) const PAGE: u32 = 2048;

pub(crate) fn stopped() -> Error {
    Error::internal("the database thread has stopped")
}

impl Db {
    /// Opens the database at `path`, creating it if it does not exist.
    ///
    /// A file that exists but cannot be read is an error, never a reason to
    /// start empty: serving an empty state over a damaged one is how real
    /// state gets overwritten.
    pub fn open(path: &Path) -> Result<Db> {
        Self::open_keeping(path, Kept::default())
    }

    /// Like [`Db::open`], saying how long its log is let grow and when it
    /// is brought back ([`crate::checkpoint`]): for a test of that, which
    /// cannot write as much as a factory at work does.
    pub fn open_keeping(path: &Path, kept: Kept) -> Result<Db> {
        Self::open_as(path, kept, crate::disk::said())
    }

    /// Like [`Db::open`], for a factory made for a test: nothing it
    /// writes is waited for on the disk ([`crate::disk`]). Never asked
    /// for a factory that is run.
    pub fn open_unwaited(path: &Path) -> Result<Db> {
        Self::open_as(path, Kept::default(), Disk::Unwaited)
    }

    /// Opens the database at `path`, saying how its log is kept and
    /// whether the disk is waited for: a project's journal is opened as
    /// the factory's own was.
    pub(crate) fn open_as(path: &Path, kept: Kept, disk: Disk) -> Result<Db> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path).map_err(|err| db_error(err).context(path.display()))?;
        // How long to wait for the file is said first: a daemon started as
        // the one before it is still putting the file away would otherwise
        // be told at once that it is locked, by the very next line.
        //
        // Then how big a page is ([`PAGE`]), before anything is written:
        // the only time it is heard. A database made before goes on with
        // the pages it has, and nothing here writes it out again.
        //
        // One that does not wait for the disk is told so before its file
        // is made, which is a wait of its own; one that does is told
        // nothing it was not always told.
        let first = match disk {
            Disk::Waited => "",
            Disk::Unwaited => "PRAGMA synchronous = OFF;",
        };
        connection
            .execute_batch(&format!(
                "PRAGMA busy_timeout = 5000;
                 {first}
                 PRAGMA page_size = {PAGE};
                 PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = {};
                 PRAGMA foreign_keys = ON;",
                disk.synchronous(),
            ))
            .map_err(|err| db_error(err).context(path.display()))?;
        // Touch the schema so a corrupt or foreign file fails here, loudly.
        connection
            .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
            .map_err(|err| db_error(err).context(format!("{} is not a usable database", path.display())))?;
        Self::start(connection, Some((path, kept)), disk)
    }

    /// A private in-memory database, for tests.
    pub fn memory() -> Result<Db> {
        let connection = Connection::open_in_memory().map_err(db_error)?;
        connection.execute_batch("PRAGMA foreign_keys = ON;").map_err(db_error)?;
        Self::start(connection, None, Disk::Waited)
    }

    fn start(mut connection: Connection, file: Option<(&Path, Kept)>, disk: Disk) -> Result<Db> {
        // A transaction takes its turn to write when it begins. One that
        // read first and asked to write afterwards is refused at once,
        // however long it was told to wait, if anything else wrote to the
        // file in between: a daemon started on data whose last holder is
        // still putting its work away would be told the database is locked.
        // The thread begins its own that way, for every piece of work
        // (below); this is for whatever is begun before the thread is.
        connection.set_transaction_behavior(rusqlite::TransactionBehavior::Immediate);
        crate::steps::kept_in(&connection).map_err(db_error)?;
        let (jobs, queue) = mpsc::channel::<Job>();
        let meter: Arc<OnceLock<Meter>> = Arc::default();
        let told = meter.clone();
        let closed: Arc<AtomicBool> = Arc::default();
        let over = closed.clone();
        let held: Arc<AtomicBool> = Arc::default();
        let holding = held.clone();
        let (going, gone) = watch::channel(false);
        let writes: Arc<AtomicU64> = Arc::default();
        let wrote = writes.clone();
        let done: Arc<AtomicU64> = Arc::default();
        let did = done.clone();
        let begun: Arc<AtomicU64> = Arc::default();
        let beginning = begun.clone();
        let waiting: Arc<AtomicUsize> = Arc::default();
        let behind = waiting.clone();
        // A database in a file has a log, which is brought back into it
        // off this thread's turn ([`crate::checkpoint`]).
        let brought: Arc<AtomicU64> = Arc::default();
        let mut log = match file {
            Some((path, kept)) => Some(
                Log::watch(&mut connection, path, (kept, disk), held.clone(), brought.clone())
                    .map_err(db_error)?,
            ),
            None => None,
        };
        let pages = log.as_ref().map(Log::pages).unwrap_or_default();
        let logged = log.as_ref().map(Log::logged).unwrap_or_default();
        let synced: Arc<AtomicU64> = Arc::default();
        let held_to_disk = synced.clone();
        let at = Arc::new(At {
            file: file.map(|(path, _)| path.to_path_buf()),
            doing: Mutex::new(("waiting for work", None, Instant::now())),
        });
        let says = at.clone();
        std::thread::Builder::new()
            .name("journal-db".into())
            .spawn(move || {
                // What was asked while the database was held, and is
                // never done: kept, so that whoever asked is not told
                // anything while there is anything they could do about it.
                let mut never_done = Vec::new();
                crate::gather::here();
                let (most, deep) = crate::gather::said();
                // Taken from the queue and not yet done: what is done
                // apart from the commit that was open when it came.
                let mut next: Option<Job> = None;
                loop {
                    let first = match next.take() {
                        Some(job) => job,
                        None => match queue.recv() {
                            Ok(job) => {
                                behind.fetch_sub(1, Ordering::SeqCst);
                                job
                            }
                            Err(_) => break,
                        },
                    };
                    if holding.load(Ordering::SeqCst) && !first.though_held {
                        never_done.push(first);
                        continue;
                    }
                    // One commit: this piece of work, and where the
                    // thread is behind whatever else is waiting already
                    // ([`crate::gather`]), unless it
                    // is to be done apart. It takes its turn to write as
                    // it begins, whatever it then does.
                    let alone = first.apart || first.to_disk;
                    // One that is to be on the disk before it is answered
                    // has the log held to the disk as its commit is made,
                    // which the database does where it is told to before
                    // the transaction is begun, and is told otherwise
                    // again once it is made: no other commit pays for it.
                    // A database that does not wait for the disk does
                    // not wait here either, and counts none as held.
                    let to_disk = first.to_disk && disk == Disk::Waited;
                    if to_disk {
                        let _ = connection.execute_batch("PRAGMA synchronous = FULL");
                    }
                    let began = connection.execute_batch("BEGIN IMMEDIATE").is_ok();
                    if began {
                        beginning.fetch_add(1, Ordering::SeqCst);
                    }
                    let opened = Instant::now();
                    let mut done: Vec<Done> = Vec::new();
                    let mut job = Some(first);
                    // Keeping up, each piece of work is committed and
                    // answered as it comes; behind, what is waiting
                    // shares this commit. Asked once, as the commit is
                    // begun: one that shares takes all that waits.
                    let shares = behind.load(Ordering::SeqCst) >= deep;
                    while let Some(one) = job.take() {
                        let started = Instant::now();
                        let before = connection.total_changes();
                        says.now("doing the work asked at", Some(one.from));
                        let (ok, answer) = (one.work)(&mut connection);
                        done.push(Done {
                            from: one.from,
                            waited_ms: started.duration_since(one.asked).as_secs_f64() * 1000.0,
                            took_ms: started.elapsed().as_secs_f64() * 1000.0,
                            at: Instant::now(),
                            ok,
                            changed: connection.total_changes() != before,
                            answer,
                        });
                        let full = done.len() >= most || opened.elapsed() >= crate::gather::FOR;
                        if alone || !began || !shares || full || crate::gather::is_cut() {
                            break;
                        }
                        // What else is waiting, and nothing that is not.
                        while let Ok(more) = queue.try_recv() {
                            behind.fetch_sub(1, Ordering::SeqCst);
                            if holding.load(Ordering::SeqCst) && !more.though_held {
                                never_done.push(more);
                            } else if more.apart {
                                next = Some(more);
                                break;
                            } else {
                                job = Some(more);
                                break;
                            }
                        }
                    }
                    let committing = Instant::now();
                    says.now("keeping what was written", None);
                    let kept = match began {
                        true => connection.execute_batch("COMMIT"),
                        false => Ok(()),
                    };
                    if kept.is_err() {
                        // Nothing of it is kept, and the next begins clean.
                        let _ = connection.execute_batch("ROLLBACK");
                    }
                    if to_disk {
                        let _ = connection.execute_batch("PRAGMA synchronous = NORMAL");
                        if began && kept.is_ok() {
                            held_to_disk.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    let lost =
                        kept.err().map(|err| db_error(err).context("what was written could not be kept"));
                    // What follows from its being kept, before anybody is answered.
                    crate::gather::follow(lost.is_none());
                    // The commit is the work of those that wrote, in equal parts.
                    let writers = done.iter().filter(|one| one.changed).count();
                    let commit_ms = committing.elapsed().as_secs_f64() * 1000.0 / writers.max(1) as f64;
                    let changed = writers > 0;
                    let (shared, made) = (done.len(), Instant::now());
                    for one in done {
                        if one.changed {
                            wrote.fetch_add(1, Ordering::SeqCst);
                        }
                        did.fetch_add(1, Ordering::SeqCst);
                        if let Some(meter) = told.get() {
                            meter(Turn {
                                by: whose(one.from.file()),
                                wrote: one.changed,
                                waited_ms: one.waited_ms,
                                took_ms: one.took_ms + if one.changed { commit_ms } else { 0.0 },
                                shared,
                                held_ms: made.duration_since(one.at).as_secs_f64() * 1000.0,
                                ok: one.ok && lost.is_none(),
                            });
                        }
                        (one.answer)(lost.clone());
                    }
                    // Whether the log has grown enough for somebody to
                    // be told, once whoever asked has their answer.
                    if let Some(log) = log.as_mut().filter(|_| changed) {
                        log.look(&connection);
                    }
                    // Closed: the thread ends here, and the file is let go
                    // with the connection. Work still waiting is never done,
                    // and whoever asked for it is told so.
                    if over.load(Ordering::SeqCst) {
                        break;
                    }
                    says.now("waiting for work", None);
                }
                // The file is let go before anyone is told it has been:
                // whoever opens it next is not met by this one putting it away.
                drop(never_done);
                // The thread that brings the log back lets the file go
                // first: the last to let go puts the log away.
                says.now("waiting for the thread that brings its log back to end", None);
                if let Some(log) = log.as_mut() {
                    log.end();
                }
                says.now("putting its file away", None);
                drop(connection);
                says.now("gone", None);
                let _ = going.send(true);
            })
            .map_err(|err| Error::internal(format!("cannot start the database thread: {err}")))?;
        Ok(Db {
            jobs,
            meter,
            closed,
            held,
            gone,
            writes,
            done,
            begun,
            waiting,
            log: pages,
            brought,
            logged,
            synced,
            at,
            disk,
        })
    }

    /// Whether the thread has ended and let its file go ([`Db::closed`]):
    /// it said so, or is there no longer to say anything.
    pub fn is_closed(&self) -> bool {
        *self.gone.borrow() || self.gone.has_changed().is_err()
    }

    /// How the database stands, in words: which file it is in, what its
    /// thread is at and for how long it has been, and for a piece of work
    /// where the work was asked for. For whoever waited for the database
    /// and has to say what it waited for: a wait that ends by running out
    /// of patience and names nothing is found again only by chance.
    pub fn standing(&self) -> String {
        let file = match &self.at.file {
            Some(file) => file.display().to_string(),
            None => "a database in memory".to_string(),
        };
        let (doing, asked, since) = *self.at.doing.lock().expect("database lock");
        let asked = asked.map(|asked| format!(" {}:{}", asked.file(), asked.line())).unwrap_or_default();
        let behind = self.waiting.load(Ordering::SeqCst);
        // One that was told to do nothing more is at none of what it was
        // asked since, and whoever asked that is waiting on its ending.
        let held = match self.held.load(Ordering::SeqCst) {
            true => "; it does nothing more of what it is asked, having shut at a place it was told",
            false => "",
        };
        format!(
            "{file}: its thread is {doing}{asked}, and has been for {:.1} s, with {behind} asked behind that{held}",
            since.elapsed().as_secs_f64()
        )
    }

    /// The file the database is in: none, for one in memory. For whoever
    /// reads it on a connection of their own, as a copy does.
    pub fn file(&self) -> Option<&Path> {
        self.at.file.as_deref()
    }

    /// How many pieces of work are waiting to be begun: asked, and not
    /// yet taken up by the thread. For a test to know that something was
    /// asked behind what the thread is at.
    pub fn waiting(&self) -> usize {
        self.waiting.load(Ordering::SeqCst)
    }

    /// Whether the database waits for the disk ([`crate::disk`]): every
    /// one a factory that is run opens does.
    pub fn disk(&self) -> Disk {
        self.disk
    }

    /// How many commits were held to the disk before whoever asked was
    /// answered ([`Db::call_to_the_disk`]), since the database was opened.
    pub fn kept_to_the_disk(&self) -> u64 {
        self.synced.load(Ordering::SeqCst)
    }

    /// How long the database's log is, in pages, as its last commit
    /// said: what has been written and not yet brought back into the
    /// database's own file, or was brought back with the log not yet
    /// begun again. Nothing for a database in memory.
    pub fn log_pages(&self) -> i64 {
        self.log.load(Ordering::SeqCst)
    }

    /// How many commits have written to the log since the database was
    /// opened: pieces of work that changed anything and were kept
    /// together, which is each such piece by itself unless the thread had
    /// fallen behind ([`crate::gather`]). Nothing for a database in
    /// memory. For whoever counts what something costs.
    pub fn commits(&self) -> u64 {
        self.logged.commits.load(Ordering::SeqCst)
    }

    /// How many frames, a page each, have been written to the log since
    /// the database was opened, whether or not they have been brought
    /// back since: what its writes cost the disk before anything is
    /// brought back. Nothing for a database in memory.
    pub fn logged(&self) -> u64 {
        self.logged.frames.load(Ordering::SeqCst)
    }

    /// How many times the log has been brought back into the database
    /// since it was opened, by the thread that does it or by the writer
    /// at the bound.
    pub fn brought_back(&self) -> u64 {
        self.brought.load(Ordering::SeqCst)
    }

    /// Has the database do nothing more of what it is asked, and answer
    /// nobody, from now on. Asked from work the database is doing, that
    /// work is the last it does.
    ///
    /// For a journal that has shut at a place it was told
    /// (`Journal::shut_after`): nothing is written after that place, by
    /// anybody, as if the process had gone at that instant. What is asked
    /// from then is never done, and whoever asked waits until the database
    /// is closed. It is never let go: what is still about of a daemon that
    /// is stopping would get in what it was in the middle of. Only what
    /// says that the daemon stopped ([`Db::call_though_held`]) and what
    /// closes the database are done.
    pub fn hold(&self) {
        self.held.store(true, Ordering::SeqCst);
    }

    /// Ends the database's thread once the work ahead of this has been
    /// done, and lets its file go. Whatever is asked of it afterwards is
    /// told that the thread has stopped.
    ///
    /// For a daemon that is stopping. The thread otherwise lives for as
    /// long as anything at all holds the database, and something long gone
    /// usually does: a process that starts and stops many daemons kept a
    /// thread and an open file for every one.
    #[track_caller]
    pub fn close(&self) {
        let closed = self.closed.clone();
        // Apart: it is the last thing the thread does, and nothing asked
        // after it is done with it.
        let how = How { though_held: true, apart: true, to_disk: false };
        let _ = self.ask(Location::caller(), how, move |_| {
            closed.store(true, Ordering::SeqCst);
            (true, ())
        });
    }

    /// Completes when the thread has ended and let its file go: after
    /// [`Db::close`], or when the last to hold the database has gone.
    ///
    /// A daemon that stops waits for this, so that it has stopped only
    /// when the file is free. Closing asks the thread to end and does not
    /// wait for it, and a daemon started in the moment between was told
    /// the database was locked, by the one before it putting the file away.
    pub async fn closed(&self) {
        let mut gone = self.gone.clone();
        let _ = gone.wait_for(|gone| *gone).await;
    }

    /// How many pieces of work have changed anything since the database
    /// was opened, whatever each wrote and whether it was committed by
    /// itself or with others ([`crate::gather`]). For whoever counts what
    /// something costs, and for a test to know that nothing is being
    /// written any more.
    pub fn writes(&self) -> u64 {
        self.writes.load(Ordering::SeqCst)
    }

    /// How many pieces of work the database has done since it was
    /// opened, whatever each was: something read counts as something
    /// written does. For a test to know that nothing is being asked of
    /// the database any more, which says more than that nothing is being
    /// written: whatever is on its way to a write reads first.
    pub fn done(&self) -> u64 {
        self.done.load(Ordering::SeqCst)
    }

    /// How many transactions the thread has begun since the database was
    /// opened: one for each commit, whether it kept one piece of work or
    /// several ([`crate::gather`]), and whether or not anything was
    /// written. What beginning one and keeping it take of the database's
    /// own steps is the same every time, so for whoever counts those
    /// steps for what a piece of work does (a card's cost): where
    /// gathering is forced how many transactions there are is how the
    /// machine's work fell, and says nothing of what was asked.
    pub fn begun(&self) -> u64 {
        self.begun.load(Ordering::SeqCst)
    }

    /// Has every piece of work told to `meter` once it is done, on the
    /// database thread: what is done there holds up the work behind it, so
    /// it should be little. The first to ask is the one that is told.
    pub fn measured(&self, meter: impl Fn(Turn) + Send + Sync + 'static) {
        let _ = self.meter.set(Box::new(meter));
    }

    /// Puts `work` in the queue, from wherever it was asked for, and gives
    /// back where its answer will come. It waits from now, not from when
    /// whoever asked first looks for the answer.
    pub(crate) fn ask<T, F>(
        &self,
        from: &'static Location<'static>,
        how: How,
        work: F,
    ) -> Result<oneshot::Receiver<Result<T>>>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> (bool, T) + Send + 'static,
    {
        let (reply, answer) = oneshot::channel();
        let work = Box::new(move |connection: &mut Connection| {
            let (ok, did) = work(connection);
            // What it came to: or, where the commit it was done in
            // could not be made, that nothing of it was kept.
            let answer: Answer = Box::new(move |lost| {
                let _ = reply.send(match lost {
                    None => Ok(did),
                    Some(why) => Err(why),
                });
            });
            (ok, answer)
        });
        let How { though_held, apart, to_disk } = how;
        let job = Job { asked: Instant::now(), from, work, though_held, apart, to_disk };
        // Counted as waiting before it can be taken up.
        self.waiting.fetch_add(1, Ordering::SeqCst);
        self.jobs.send(job).map_err(|_| {
            self.waiting.fetch_sub(1, Ordering::SeqCst);
            stopped()
        })?;
        Ok(answer)
    }

    /// Runs `work` on the database thread and returns its result.
    #[track_caller]
    pub fn call<T, F>(&self, work: F) -> impl Future<Output = Result<T>> + Send + use<T, F>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        self.call_as(Location::caller(), How::default(), work)
    }

    /// Like [`Db::call`], counted as the work of whoever asked `from`.
    pub(crate) fn call_from<T, F>(
        &self,
        from: &'static Location<'static>,
        work: F,
    ) -> impl Future<Output = Result<T>> + Send + use<T, F>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        self.call_as(from, How::default(), work)
    }

    /// Like [`Db::call`], and done even while the database is held: for
    /// the one thing said past the place a journal shut at, that its
    /// daemon stopped.
    #[track_caller]
    pub fn call_though_held<T, F>(&self, work: F) -> impl Future<Output = Result<T>> + Send + use<T, F>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        self.call_as(Location::caller(), How { though_held: true, ..How::default() }, work)
    }

    /// Like [`Db::call`], and done apart: in a commit of its own, shared
    /// with no other work ([`crate::gather`]). What was asked before it
    /// is committed and answered before it is begun, and what is asked
    /// after it is not begun until it is over. For work that holds the
    /// thread until it is let go: whoever asked something before it
    /// would otherwise wait for it too.
    #[track_caller]
    pub fn call_apart<T, F>(&self, work: F) -> impl Future<Output = Result<T>> + Send + use<T, F>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        self.call_as(Location::caller(), How { apart: true, ..How::default() }, work)
    }

    /// Like [`Db::call`], and on the disk before it is answered: the log
    /// is held to the disk as its commit is made, which is a commit of
    /// its own. Everything else is answered once it is in the log, and
    /// held to the disk when the log is next brought back
    /// ([`crate::checkpoint`]): a daemon that is killed loses none of
    /// it, and a loss of power can take what was answered since. This is
    /// for the little that has to outlast that too, because something
    /// kept in another file rests on it: how far the giver of numbers
    /// has given, before a journal in a file of its own gives any of
    /// them ([`crate::giver`]). It costs a wait on the disk, on the
    /// database's one thread.
    #[track_caller]
    pub fn call_to_the_disk<T, F>(&self, work: F) -> impl Future<Output = Result<T>> + Send + use<T, F>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        self.call_as(Location::caller(), How { to_disk: true, ..How::default() }, work)
    }

    fn call_as<T, F>(
        &self,
        from: &'static Location<'static>,
        how: How,
        work: F,
    ) -> impl Future<Output = Result<T>> + Send + use<T, F>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let answer = self.ask(from, how, move |connection| {
            let did = work(connection);
            let ok = !matches!(did, Err(ref err) if !matches!(err, rusqlite::Error::QueryReturnedNoRows));
            // Said here, on the database's thread: what it refuses for
            // the transaction it has open is said in a sentence.
            (ok, did.map_err(db_error))
        });
        async move { answer?.await.map_err(|_| stopped())?? }
    }

    /// Whether this is `other`: the same database behind the same thread.
    pub fn is(&self, other: &Db) -> bool {
        Arc::ptr_eq(&self.closed, &other.closed)
    }

    /// Like [`Db::call`], for work that can fail for its own reasons.
    #[track_caller]
    pub fn try_call<T, F>(&self, work: F) -> impl Future<Output = Result<T>> + Send + use<T, F>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    {
        let answer = self.ask(Location::caller(), How::default(), move |connection| {
            let did = work(connection);
            (!matches!(did, Err(ref err) if err.kind != ErrorKind::NotFound), did)
        });
        async move { answer?.await.map_err(|_| stopped())?? }
    }
}

/// Maps a database error onto the kinds callers can act on. On the
/// database's own thread, what was refused for the transaction the
/// thread has open is said as what to write in its place
/// ([`crate::gather::refused`]).
pub fn db_error(err: rusqlite::Error) -> Error {
    if let Some(refused) = crate::gather::is_here().then(|| crate::gather::refused(&err)).flatten() {
        return refused;
    }
    let kind = match &err {
        rusqlite::Error::QueryReturnedNoRows => ErrorKind::NotFound,
        rusqlite::Error::SqliteFailure(failure, _) => match failure.code {
            rusqlite::ErrorCode::ConstraintViolation => ErrorKind::Conflict,
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked => ErrorKind::Unavailable,
            _ => ErrorKind::Internal,
        },
        _ => ErrorKind::Internal,
    };
    Error::new(kind, format!("database: {err}"))
}

/// `Ok(None)` instead of an error when a single-row query finds nothing.
pub fn optional<T>(result: rusqlite::Result<T>) -> rusqlite::Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err),
    }
}
