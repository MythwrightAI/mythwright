//! One giver of numbers for the whole factory.
//!
//! An event's number is its own in the factory: no other event has it,
//! whichever journal keeps the other. While there is one journal that is
//! true by counting; with a journal for each project it has to be true
//! across them, so the numbers come from one place, apart from any one
//! journal's events, and a journal draws a run of them at a time
//! ([`RUN`]) and gives them out itself, in order, to what it writes.
//!
//! What is promised of a number, and it is all that is: no other event in
//! the factory has it, and of two events of one journal the later has the
//! greater. Numbers have gaps: what is left of a run when its journal
//! stops is never given, and neither is a run whose write was not kept.
//!
//! **Nothing is given twice, wherever the program is stopped**, because
//! of one rule: the giver's mark (the highest number anybody has drawn)
//! is kept before any number at or below it is given to an event.
//!
//! - A journal that keeps the mark in its own database, which is every
//!   journal there is today, raises it in the transaction that writes
//!   the first event of the run ([`Giver::draw_here`]): one commit keeps
//!   both, so the mark is on the disk by the time the event is, and
//!   drawing a run is no write of its own. Where that commit is not
//!   made, neither was kept: the journal has the run no longer, and the
//!   giver has it back.
//! - A journal whose numbers come from a giver kept in another database
//!   (a project's, in a file of its own) asks that database to raise the
//!   mark and is answered once it is raised **and on the disk**
//!   ([`Giver::draw_apart`], `Db::call_to_the_disk`). In one file the
//!   mark and the events it covers are in one log, and a loss of power
//!   keeps both or neither; in two it could keep a project's events and
//!   lose the mark above them, and the next start would give their
//!   numbers again. So this mark is held to the disk as it is raised,
//!   once in a run, and the events are held to the disk when they
//!   always were.
//!
//! **Such a journal draws ahead** ([`Giver::draw_ahead`]). Asking
//! another database's thread and waiting for the disk is what it must
//! not do in the middle of a write: so when half the run in hand is
//! given it asks for the next and goes on, and by the time it has given
//! the rest the next is there. It waits only where it is not: for the
//! first run after it is opened, and where it gave half a run sooner
//! than the mark could be kept. What is drawn ahead and never used is a
//! gap, as what is left of any run is.
//!
//! The mark only ever goes up, whoever writes it and in whatever order
//! their writes land. And a giver starts above everything the journal it
//! is kept in ever held, whatever the mark says ([`Giver::kept_in`]): a
//! journal an earlier program made has no mark, and is taken up with
//! nothing written and nothing read but what opening it always read.
//!
//! **A journal that joins holding numbers of its own** (one that
//! numbered its own events and now draws here) says how far they go
//! ([`Giver::above`]), and nothing at or below that is given from then
//! on, by anybody: not from a run drawn later, and not from one another
//! journal already had in hand, which it lets go of as it next writes
//! ([`Giver::is_given_away`]). What another journal was in the middle of
//! writing as it joined is as what was written before it joined.

use std::panic::Location;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use kernel::Result;
use rusqlite::Connection;

use tokio::sync::oneshot;

use crate::Db;
use crate::db::How;

/// How many numbers a journal draws at a time.
///
/// It is what a restart leaves ungiven at most, and how often drawing
/// costs anything: one row written again in the write of one event in
/// this many, and for a journal whose giver is kept elsewhere one commit
/// held to the disk on another database's thread in this many, asked for
/// ahead of need. Numbers are plentiful (a gap
/// of this size at every restart for a million restarts is a thousand
/// million of nine million million million), so it is as big as leaves
/// what goes by an event's number unchanged: a multiple of 512, since one
/// number in 512 is in the index of when (`events_when`), and any whole
/// run then holds two of them however the runs of several journals fall.
pub const RUN: u64 = 1024;

/// Where the giver's mark is kept, of the small durable state of the
/// journal it is kept in: the highest number anybody has drawn.
pub const DRAWN: (&str, &str) = ("journal", "drawn");

/// The one giver of numbers: a journal draws from it, and several may.
#[derive(Clone)]
pub struct Giver {
    kept: Arc<Kept>,
}

struct Kept {
    /// The database its mark is kept in.
    db: Db,
    /// The highest number anybody has drawn, as this program knows it:
    /// what the next run begins above. Ahead of the mark on the disk for
    /// as long as the write that raises it is not kept, and for good
    /// where that write never was; never behind it.
    drawn: Mutex<u64>,
    /// The highest number a journal held as it began to draw here
    /// ([`Giver::above`]): nothing at or below it is given, though it is
    /// of a run somebody drew before.
    floor: AtomicU64,
}

/// A run asked for ahead of need: its numbers, and where it is said
/// that the mark above them is kept and on the disk.
pub(crate) struct Ahead {
    run: Run,
    kept: Raised,
}

/// Where it is said that the mark was raised: once the commit that
/// raised it is made and on the disk, or that it could not be.
type Raised = oneshot::Receiver<Result<Result<()>>>;

/// Numbers a journal has drawn and not yet given: its own, in order.
/// One is before another where it is the same run with more of it left,
/// or a run drawn earlier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Run {
    /// The next of them.
    next: u64,
    /// One past the last of them.
    end: u64,
}

impl Run {
    /// The next number of the run, which it has where one is left.
    pub(crate) fn first(&self) -> u64 {
        self.next
    }

    /// The last number of the run, for one that was drawn.
    pub(crate) fn last(&self) -> Option<u64> {
        self.end.checked_sub(1)
    }

    /// How many numbers of the run are left.
    pub(crate) fn left(&self) -> u64 {
        self.end - self.next
    }

    /// The next number of the run, where one is left.
    pub(crate) fn take(&mut self) -> Option<u64> {
        (self.next < self.end).then(|| {
            self.next += 1;
            self.next - 1
        })
    }
}

/// The mark as `schema` keeps it, on a connection to the database that
/// does: nought where none was ever kept.
pub(crate) fn mark(connection: &Connection, schema: &str) -> rusqlite::Result<i64> {
    connection.query_row(
        &format!(
            "SELECT coalesce((SELECT CAST(value AS INTEGER) FROM {schema}.kv WHERE namespace = ?1 AND key = ?2), 0)"
        ),
        [DRAWN.0, DRAWN.1],
        |row| row.get(0),
    )
}

/// Raises the mark to `drawn`, and leaves one that is there already and
/// is higher as it is: whoever raised it that far drew those numbers.
fn raise(connection: &Connection, drawn: u64) -> rusqlite::Result<()> {
    connection
        .execute(
            "INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3)
             ON CONFLICT (namespace, key) DO UPDATE SET value = excluded.value
             WHERE CAST(kv.value AS INTEGER) < CAST(excluded.value AS INTEGER)",
            rusqlite::params![DRAWN.0, DRAWN.1, drawn.to_string()],
        )
        .map(|_| ())
}

/// What a journal is told where a run could not be drawn, in the words
/// of the database it was writing to: the write it was for fails with it.
fn undrawn(why: &kernel::Error) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
        Some(format!(
            "no number could be drawn for an event, so nothing was written: the giver of numbers is kept in \
             another database, which did not keep how far it has given ({})",
            why.message
        )),
    )
}

impl Giver {
    /// The giver whose mark is kept in `db`, which the journal's tables
    /// are in. It begins above the mark, and above `held`, the highest
    /// number the journal there ever held: a journal from before has no
    /// mark, and is gone on from all the same. Nothing is written: the
    /// mark is first kept with the first run drawn.
    pub(crate) async fn kept_in(db: &Db, held: u64) -> Result<Giver> {
        let mark = db.call(|connection| mark(connection, "main")).await?;
        let drawn = Mutex::new(held.max(mark as u64));
        Ok(Giver { kept: Arc::new(Kept { db: db.clone(), drawn, floor: AtomicU64::new(0) }) })
    }

    /// Whether its mark is kept in `db`.
    pub(crate) fn is_kept_in(&self, db: &Db) -> bool {
        self.kept.db.is(db)
    }

    /// The highest number anybody has drawn: nothing at or below it is
    /// drawn from now on.
    pub fn drawn(&self) -> u64 {
        *self.kept.drawn.lock().expect("giver lock")
    }

    /// The numbers of a run nobody else has or will have.
    fn next(&self) -> Run {
        let mut drawn = self.kept.drawn.lock().expect("giver lock");
        let run = Run { next: *drawn + 1, end: *drawn + 1 + RUN };
        *drawn += RUN;
        run
    }

    /// Draws a run for the journal whose database keeps the mark: on that
    /// database's thread, on its connection, in the transaction the
    /// journal is writing in. The mark is raised there, so it is kept if
    /// and when what uses the run is. Whoever asks gives nothing of the
    /// run once that transaction has not been kept.
    pub(crate) fn draw_here(&self, connection: &Connection) -> rusqlite::Result<Run> {
        let run = self.next();
        raise(connection, run.end - 1)?;
        Ok(run)
    }

    /// Takes back a run that was drawn and none of which is the number of
    /// an event that was kept: what was written with it was not kept, its
    /// mark neither where that was raised with it. Its numbers are the
    /// next drawn; where somebody has drawn since, it is left a gap.
    pub(crate) fn take_back(&self, run: Run) {
        let mut drawn = self.kept.drawn.lock().expect("giver lock");
        if *drawn + 1 == run.end {
            *drawn = run.end - 1 - RUN;
        }
    }

    /// Asks the database that keeps the mark to raise it to `last`, and
    /// gives where it is said once that is kept and on the disk. Nothing
    /// is waited for here.
    fn raise_apart(&self, last: u64, though_held: bool) -> Result<Raised> {
        let how = How { though_held, apart: true, to_disk: true };
        self.kept.db.ask(Location::caller(), how, move |connection| {
            let did = raise(connection, last);
            (did.is_ok(), did.map_err(crate::db_error))
        })
    }

    /// Draws a run for a journal kept in another database than the mark
    /// is: asks that database to raise the mark, and waits until it has
    /// kept it and it is on the disk. Asked from the journal's own
    /// thread, which is no thread of the mark's database and waits for
    /// nothing else; and only where no run was drawn ahead
    /// ([`Giver::draw_ahead`]). `though_held` for the one thing a journal
    /// says though it has shut (`Journal::emit_last`): the database the
    /// mark is in may be doing nothing more by then (`Db::hold`), and
    /// whoever stops the daemon is not to wait for ever.
    pub(crate) fn draw_apart(&self, though_held: bool) -> rusqlite::Result<Run> {
        let run = self.next();
        let kept = self
            .raise_apart(run.end - 1, though_held)
            .and_then(|kept| kept.blocking_recv().map_err(|_| crate::db::stopped())??);
        match kept {
            Ok(()) => Ok(run),
            Err(why) => {
                self.take_back(run);
                Err(undrawn(&why))
            }
        }
    }

    /// Asks for a run ahead of need, for a journal kept in another
    /// database than the mark is, and waits for nothing: the run is the
    /// journal's once the mark above it is kept ([`Giver::ahead`]).
    pub(crate) fn draw_ahead(&self) -> Option<Ahead> {
        let run = self.next();
        match self.raise_apart(run.end - 1, false) {
            Ok(kept) => Some(Ahead { run, kept }),
            Err(_) => {
                self.take_back(run);
                None
            }
        }
    }

    /// The run that was asked for ahead, now that it is needed: waited
    /// for, where the mark above it is not kept yet. Nothing where it
    /// could not be kept: the run is given back, and whoever asks draws
    /// as if none had been asked for.
    pub(crate) fn ahead(&self, ahead: Ahead) -> Option<Run> {
        match ahead.kept.blocking_recv() {
            Ok(Ok(Ok(()))) => Some(ahead.run),
            _ => {
                self.take_back(ahead.run);
                None
            }
        }
    }

    /// Whether the run asked for ahead is there to be used without
    /// waiting: its mark is kept. Where it is not yet, it is handed back
    /// as it was.
    pub(crate) fn ahead_if_kept(&self, mut ahead: Ahead) -> std::result::Result<Option<Run>, Ahead> {
        match ahead.kept.try_recv() {
            Ok(Ok(Ok(()))) => Ok(Some(ahead.run)),
            Err(oneshot::error::TryRecvError::Empty) => Err(ahead),
            _ => {
                self.take_back(ahead.run);
                Ok(None)
            }
        }
    }

    /// Whether the numbers of `run` are no longer its journal's to give:
    /// a journal that joined since holds numbers as high
    /// ([`Giver::above`]). A run with nothing left is nobody's concern.
    pub(crate) fn is_given_away(&self, run: &Run) -> bool {
        run.left() > 0 && run.first() <= self.kept.floor.load(Ordering::SeqCst)
    }

    /// Has the giver give nothing at or below `held`, and keeps that: for
    /// a journal that begins to draw from a giver kept elsewhere, and
    /// holds events numbered before it did. It is asked before that
    /// journal gives a number. Whatever other journals have in hand of
    /// runs that reach no higher is theirs no longer
    /// ([`Giver::is_given_away`]).
    pub(crate) async fn above(&self, held: u64) -> Result<()> {
        self.kept.floor.fetch_max(held, Ordering::SeqCst);
        {
            let mut drawn = self.kept.drawn.lock().expect("giver lock");
            if held <= *drawn {
                return Ok(());
            }
            *drawn = held;
        }
        self.kept.db.call(move |connection| raise(connection, held)).await
    }
}
