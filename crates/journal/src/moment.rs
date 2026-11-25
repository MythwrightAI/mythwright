//! Every journal of a factory, read as of one moment.
//!
//! A factory is several files, the factory's own and one for each
//! project that has a journal of its own ([`crate::set`]), each written
//! by a thread of its own. A copy of each taken whenever its turn came
//! is no factory that ever was: what a project's thread wrote in the
//! instant between rests on something the factory's file, taken a moment
//! earlier, does not hold. So a copy of a factory reads every file as of
//! one moment ([`Journal::moment`]):
//!
//! 1. **Every thread is parked between two pieces of work.** Each is
//!    asked for a piece of work that does nothing but wait to be let go,
//!    done apart (`Db::call_apart`): what was asked of the thread before
//!    is done, committed and answered first, and what is asked after
//!    waits. Every project's is asked at once, and the factory's only
//!    when all of those stand still; that order is what keeps it from
//!    standing still for good: a project's thread waits on the factory's
//!    in the middle of a piece of its own work, for a run of numbers
//!    ([`crate::giver`]), and never the other way round. So the
//!    factory's thread is free until every project's is between two
//!    pieces of work, where it waits for nothing.
//! 2. **A read is begun on each file**, on a connection of its own,
//!    while all of them are parked: the factory's, and each journal the
//!    factory's file lists at that moment. A read sees its file as it
//!    was when it began, whatever is written afterwards. A journal that
//!    was listed while the others were being parked has no thread parked
//!    here, and is read all the same: with every other thread standing
//!    still, whatever its own does in that instant rests on nothing the
//!    others have yet to write.
//! 3. **Every thread is let go**, and each file is written out from its
//!    read at leisure ([`Read::write`]), which is let go as its file is
//!    done.
//!
//! **Nothing waits for ever.** How long the threads are waited for, all
//! of them together, is bounded ([`BETWEEN_WITHIN`]): a thread that is
//! in the middle of something long, or whose database is to do nothing
//! more (`Db::hold`), is not waited for past that, the ones parked so
//! far are let go, and whoever asked is told which thread it was. A
//! thread that is parked goes on by itself where nobody lets it go
//! ([`PARKED_AT_MOST`]), and whoever parked it is then told that the
//! moment was not one. And whoever parked them going away lets all of
//! them go.
//!
//! **What it costs.** Every journal stands still from when the first
//! thread is parked until the reads are begun: as long as the slowest
//! piece of work any thread is in the middle of, and the opening of a
//! file each ([`Moment::stood_ms`]). And a read that is held open keeps
//! its file's log from being brought back past where the read began
//! ([`crate::checkpoint`]): the log grows by what is written while its
//! file, and the files written before it, are being copied, and is
//! brought back, and its file cut back to the bound, once the read is
//! let go. The figures are in `docs/architecture.md`, *A journal for
//! each project*.

use std::panic::Location;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use kernel::{Error, Result};
use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};

use crate::db::{How, db_error};
use crate::{Db, FACTORY, Journal, Listed};

/// How long every thread of a factory's journals is waited for to come
/// between two pieces of work, all of them together. A piece of work is
/// a millisecond or a few; what takes longer is rare (old events taken
/// out, once an hour) and a copy asked for then is asked for again.
pub const BETWEEN_WITHIN: Duration = Duration::from_secs(5);

/// How long a thread stays parked where nobody lets it go: longer than
/// the others are waited for, and no longer than a factory can stand
/// still with whoever parked it stuck.
const PARKED_AT_MOST: Duration = Duration::from_secs(15);

/// A parked thread, until it is let go or goes on by itself.
const PARKED: u8 = 0;
const LET_GO: u8 = 1;
const WENT_ON: u8 = 2;

/// One thread that is parked, or asked to be.
struct Park {
    /// What its journal is called.
    journal: String,
    /// Which of the three it is: decided once, by whoever comes first.
    state: Arc<AtomicU8>,
    /// What ends its wait; put away, that ends it too.
    go: mpsc::Sender<()>,
}

/// The threads parked for a moment. Put away, they are let go.
#[derive(Default)]
struct Parked {
    all: Vec<Park>,
    /// When the first of them was parked.
    since: Option<Instant>,
}

/// Why a factory was not read: the thread of the journal called
/// `journal` did not come to stand still, for the reason `why`.
fn not_parked(journal: &str, why: &str) -> Error {
    Error::unavailable(format!(
        "the factory was not read at one moment: the database of the journal `{journal}` {why}. Nothing \
         was held up for longer than that, and nothing was copied; ask again"
    ))
}

impl Parked {
    /// Asks the thread of `db` to park between two pieces of work, behind
    /// all that was asked of it so far, and gives where it says that it
    /// has. Nothing is waited for here.
    fn ask(&mut self, db: &Db, journal: &str) -> Result<mpsc::Receiver<()>> {
        let (entered, enters) = mpsc::channel::<()>();
        let (go, gone) = mpsc::channel::<()>();
        let state = Arc::new(AtomicU8::new(PARKED));
        let its = state.clone();
        // Apart: what was asked before it is committed and answered
        // before it begins, and nothing is done with it.
        let how = How { apart: true, ..How::default() };
        let asked = db.ask(Location::caller(), how, move |_| {
            let _ = entered.send(());
            // Let go, or whoever parked it gone: either ends the wait.
            // And so does nobody coming: a factory does not stand still
            // for good because a copy of it went wrong.
            if gone.recv_timeout(PARKED_AT_MOST) == Err(mpsc::RecvTimeoutError::Timeout) {
                let _ = its.compare_exchange(PARKED, WENT_ON, Ordering::SeqCst, Ordering::SeqCst);
            }
            (true, ())
        });
        // Kept from here, so that it is let go however this ends.
        self.all.push(Park { journal: journal.to_string(), state, go });
        asked.map_err(|_| not_parked(journal, "has stopped"))?;
        Ok(enters)
    }

    /// Waits until the thread of the journal called `journal` says it is
    /// parked: `until` at the latest, which is `within` of when the
    /// first was asked.
    fn wait_for(
        &mut self,
        journal: &str,
        enters: &mpsc::Receiver<()>,
        until: Instant,
        within: Duration,
    ) -> Result<()> {
        match enters.recv_timeout(until.saturating_duration_since(Instant::now())) {
            Ok(()) => {
                self.since.get_or_insert_with(Instant::now);
                Ok(())
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Err(not_parked(
                journal,
                &format!(
                    "did not come between two pieces of its work in the {} ms every journal is waited \
                     for: it is in the middle of something long, or is to do nothing more, its journal \
                     having shut",
                    within.as_millis()
                ),
            )),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(not_parked(journal, "has stopped")),
        }
    }

    /// Lets every thread go, and says whether each was still parked: one
    /// that had gone on by itself was writing while the others were read.
    fn let_go(&mut self) -> Result<f64> {
        let stood = self.since.map(|since| since.elapsed().as_secs_f64() * 1000.0).unwrap_or_default();
        let mut went_on = None;
        for park in self.all.drain(..) {
            if park.state.compare_exchange(PARKED, LET_GO, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                went_on.get_or_insert(park.journal);
            }
            let _ = park.go.send(());
        }
        match went_on {
            None => Ok(stood),
            Some(journal) => Err(Error::unavailable(format!(
                "the factory was not read at one moment: reading its files took longer than a journal is \
                 held still for ({} s), and the journal `{journal}` went on meanwhile. Nothing was copied; \
                 ask again",
                PARKED_AT_MOST.as_secs()
            ))),
        }
    }
}

impl Drop for Parked {
    fn drop(&mut self) {
        let _ = self.let_go();
    }
}

/// One file of a factory, being read as it was at a moment: for as long
/// as this is kept, whatever is written to the file since.
pub struct Read {
    connection: Connection,
    file: PathBuf,
}

impl Read {
    /// Begins a read of `file`, on a connection of its own that only
    /// reads: the thread that writes the file is asked nothing.
    fn begin(file: &Path) -> Result<Read> {
        let wrong = |err: rusqlite::Error| db_error(err).context(format!("reading {}", file.display()));
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let connection = Connection::open_with_flags(file, flags).map_err(wrong)?;
        connection.busy_timeout(Duration::from_secs(5)).map_err(wrong)?;
        // A read begins when something is first read, and not when it
        // is said to: so something is.
        connection.execute_batch("BEGIN").map_err(wrong)?;
        connection
            .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get::<_, i64>(0))
            .map_err(wrong)?;
        Ok(Read { connection, file: file.to_path_buf() })
    }

    /// Writes the file to `to`, as it was when the read began, and lets
    /// the read go. `to` must not exist. It blocks for as long as the
    /// copying takes.
    ///
    /// The pages are copied as they are, the ones no longer used among
    /// them: what is then taken out of the copy is taken out of the
    /// file itself as the copy is closed ([`crate::copy::seal`]).
    pub fn write(self, to: &Path) -> Result<()> {
        let wrong = |err: rusqlite::Error| {
            db_error(err).context(format!("copying {} to {}", self.file.display(), to.display()))
        };
        // Made here and by nobody else: a copy is never written over
        // something that is there.
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(to)
            .map_err(|err| Error::from(err).context(to.display()))?;
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let mut copy = Connection::open_with_flags(to, flags).map_err(wrong)?;
        {
            // The read that is open is the one the pages are copied
            // through: all of them in one step, as of where it began.
            let pages = Backup::new(&self.connection, &mut copy).map_err(wrong)?;
            match pages.step(-1).map_err(wrong)? {
                StepResult::Done => {}
                _ => {
                    return Err(Error::unavailable(format!(
                        "{} could not be copied to {}: something else has the copy open",
                        self.file.display(),
                        to.display()
                    )));
                }
            }
        }
        copy.close().map_err(|(_, err)| wrong(err))
    }
}

/// A factory read as of one moment: its own file, and the journal of
/// each project that has one, as the factory's file listed them then.
pub struct Moment {
    /// The factory's own database.
    pub factory: Read,
    /// Each journal kept beside it, in the order of their names.
    pub beside: Vec<(Listed, Read)>,
    /// How long every journal's thread stood still for it, in
    /// milliseconds: from when the first was parked until all were let go.
    pub stood_ms: f64,
}

impl Journal {
    /// Begins a read of every file of this factory's journals as of one
    /// moment ([`crate::moment`]), from which each is then written out.
    /// Asked of the factory's own journal. It blocks, briefly: until
    /// every journal's thread has come between two pieces of work, which
    /// is waited for [`BETWEEN_WITHIN`] at most.
    pub fn moment(&self) -> Result<Moment> {
        self.moment_within(BETWEEN_WITHIN)
    }

    /// Like [`Journal::moment`], saying how long the threads are waited
    /// for: for a test of what is said where one never comes.
    pub fn moment_within(&self, within: Duration) -> Result<Moment> {
        if self.name() != FACTORY {
            return Err(Error::invalid(format!(
                "`{}` is a project's journal, and a factory is read as of one moment from its own",
                self.name()
            )));
        }
        let Some(file) = self.db().file() else {
            return Err(Error::invalid("this factory's journal is in memory, and has no file to read"));
        };
        let until = Instant::now() + within;
        let mut parked = Parked::default();
        // Every project's first, all asked at once so that none stands
        // still for longer than the slowest of them takes to come; and
        // the factory's own only when they all stand. A project's thread
        // may be waiting for the factory's, in the middle of its own
        // work, and the factory's waits for none of theirs.
        let every = self.every();
        let mut asked = Vec::new();
        for journal in every.iter().skip(1) {
            asked.push((journal.name(), parked.ask(journal.db(), journal.name())?));
        }
        for (journal, enters) in &asked {
            parked.wait_for(journal, enters, until, within)?;
        }
        let enters = parked.ask(self.db(), self.name())?;
        parked.wait_for(self.name(), &enters, until, within)?;
        let factory = Read::begin(file)?;
        let mut beside = Vec::new();
        for listed in crate::listed(&factory.connection).map_err(db_error)? {
            let Some(folder) = self.inner.set.get().and_then(|set| set.folder()) else {
                return Err(Error::invalid(format!(
                    "this factory lists a journal of the project `{}`, and was not told where its projects' \
                     journals are kept",
                    listed.project
                )));
            };
            let read = Read::begin(&folder.join(&listed.file))
                .map_err(|err| err.context(format!("the journal of the project `{}`", listed.project)))?;
            beside.push((listed, read));
        }
        let stood_ms = parked.let_go()?;
        Ok(Moment { factory, beside, stood_ms })
    }
}
