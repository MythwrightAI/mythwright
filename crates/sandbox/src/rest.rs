//! Waiting for a sandbox to be at rest, and holding it to its limits.
//!
//! An act is answered when what came of it has come to rest: when
//! nothing the sandbox runs can go on unless whoever runs the test does
//! something more (a stand-in answered, the clock moved on, an event
//! said). That is worked out, and no clock is looked at for it.
//!
//! Everything a sandbox runs is a task of its own thread's runtime, and
//! a task goes forward only when it is woken: by another task of that
//! runtime, by a database answering, or by file work coming back from
//! the one thread that does it. So rest is three things being so at
//! one moment, each of which is known exactly:
//!
//! - **No task can run.** The runtime says when it has nothing left to
//!   run and is about to wait ([`Parked`]): the sandbox's own thread is
//!   the only one that runs its tasks.
//! - **No action of the sandbox's own is in the middle of its work.**
//!   Every action that is the real one counts itself in and out
//!   ([`crate::offered::Working`]); file work is only ever done inside
//!   one, and such an action always comes to its end by itself. A step
//!   that waits, one a stand-in was asked for, and one that started
//!   something and waits for it are in no action of this kind.
//! - **No database has anything asked of it.** A piece of work asked
//!   now is done after everything asked before it, so when it is
//!   answered and no other piece was done meanwhile, there was none;
//!   and so twice running, since a database counts a piece as done a
//!   moment before it answers whoever asked for it.
//!
//! Until all three are so the sandbox is let go on, a round at a time,
//! and counted as it goes: one that has come to more steps, events or
//! bytes than a sandbox may is stopped.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use conductor::InstanceFilter;
use rusqlite::Connection;
use tokio::sync::Notify;

use crate::inside::Inside;
use crate::stood::Answers;

/// How many rounds a sandbox that is stopped is given to have stopped.
const STOPPING: usize = 20_000;

/// How many quiet rounds one after the other are rest.
const QUIET: usize = 2;

/// How many rounds in a row with nothing asked of a database an action
/// that is in the middle of its work is waited for: more pieces of file
/// work than any action does one after another. One still at it after
/// that waits for something this does not know of, and is let be.
const WORKING: usize = 256;

/// Told by the runtime of the sandbox's thread each time it has nothing
/// left to run.
#[derive(Clone, Default)]
pub(crate) struct Parked {
    times: Arc<AtomicU64>,
    told: Arc<Notify>,
}

impl Parked {
    /// One that counts in `times`, which whoever holds the sandbox reads
    /// to say how it stands ([`crate::at::At`]).
    pub fn counting(times: Arc<AtomicU64>) -> Parked {
        Parked { times, told: Arc::default() }
    }

    /// The runtime has nothing left to run: said by the runtime itself,
    /// on its own thread, before it waits.
    pub fn now(&self) {
        self.times.fetch_add(1, Ordering::SeqCst);
        self.told.notify_one();
    }

    /// Completes when the runtime next has nothing left to run but
    /// whoever asks: every task that could go on has gone on as far as
    /// it can.
    async fn next(&self) {
        let before = self.times.load(Ordering::SeqCst);
        // One told of before this was asked is not the one waited for.
        while self.times.load(Ordering::SeqCst) == before {
            self.told.notified().await;
        }
    }
}

/// What a database keeps, in bytes, and how many steps its runs have.
fn kept(connection: &mut Connection) -> rusqlite::Result<(u64, u64)> {
    let bytes: i64 = connection.query_row(
        "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
        [],
        |row| row.get(0),
    )?;
    // The table of steps is the runner's; a database that keeps none of
    // what is run has none.
    let steps: i64 = connection.query_row("SELECT count(*) FROM steps", [], |row| row.get(0)).unwrap_or(0);
    Ok((bytes.max(0) as u64, steps.max(0) as u64))
}

impl Inside {
    /// How many pieces of work the sandbox's databases have done.
    fn done(&self) -> u64 {
        self.dbs().iter().map(journal::Db::done).sum()
    }

    /// How many events the sandbox has said.
    pub fn said(&self) -> u64 {
        self.order.so_far().saturating_sub(self.begins)
    }

    /// Asks every database what it keeps, which each answers once it has
    /// done everything it was asked before: what the sandbox keeps, in
    /// bytes, with what was written to its steps' logs, and how many
    /// steps it has run.
    async fn asks_each(&self) -> (u64, u64) {
        let (mut bytes, mut steps) = (0, 0);
        for (at, db) in self.dbs().into_iter().enumerate() {
            self.at.now(format!("asking its database {at} what it keeps"));
            let (more, ran) = db.call(kept).await.unwrap_or_default();
            (bytes, steps) = (bytes + more, steps + ran);
        }
        // And what its steps' logs were written, which are files of its
        // folder: counted as they are written, so nothing is looked at.
        (bytes + self.logs.written(), steps)
    }

    /// Lets the file work that is under way come back: one thread does
    /// that work in the order it is asked, so when this is done, so is
    /// what was asked before it.
    async fn file_work(&self) {
        let _ = tokio::task::spawn_blocking(|| ()).await;
    }

    /// Whether an action of the sandbox's own is in the middle of its work.
    fn working(&self) -> bool {
        self.working.load(Ordering::SeqCst) > 0
    }

    /// Lets the sandbox go on until it is at rest, or, where it is
    /// `limited`, until it has come to more than it may, which is
    /// answered with why.
    ///
    /// A round is: wait until no task can run; note whether an action is
    /// at work; ask each database, and count what the databases did
    /// since the round before that was not this asking. A round in which
    /// no action was at work and they did nothing else is quiet: nothing
    /// was asked of them when it began (it would have been done before
    /// what this asked), and no file work was under way to come back.
    ///
    /// Rest is two quiet rounds, one after the other, and both are
    /// needed. A database counts a piece of work as done a moment before
    /// it answers whoever asked, so a piece counted in one round may
    /// wake its task in the next, after this has asked that database,
    /// and what the task then asks is behind this round's asking and is
    /// not seen by it. It is seen by the round after: asked there first,
    /// it is done first. In a round that follows a quiet one nothing is
    /// left to answer but this itself, so no task is woken in it, and
    /// where it is quiet too none ran and none can.
    async fn rests(&mut self, limited: bool) -> Result<(), String> {
        let (mut last, mut quiet, mut still) = (self.done(), 0, 0);
        // One that is being stopped is given so many rounds to stop in.
        for round in 0..if limited { usize::MAX } else { STOPPING } {
            self.at.now(format!(
                "waiting for its runtime to have nothing left to run, in round {round} of coming to rest \
                 ({quiet} quiet)"
            ));
            self.parked.next().await;
            let working = self.working();
            let asked_of = self.dbs().len() as u64;
            self.at.now(format!(
                "asking each of its databases what it keeps, in round {round} of coming to rest"
            ));
            let (bytes, steps) = self.asks_each().await;
            let now = self.done();
            let asked = now.saturating_sub(last).saturating_sub(asked_of) > 0;
            last = now;
            (self.keeps, self.ran) = (bytes, steps);
            if limited && let Some(over) = self.limits.over(steps, self.said(), bytes) {
                return Err(over);
            }
            match (asked, working) {
                (false, false) if quiet + 1 >= QUIET => return Ok(()),
                (false, false) => quiet += 1,
                (true, _) => (quiet, still) = (0, 0),
                // In the middle of file work, which comes back.
                (false, true) if still < WORKING => {
                    (quiet, still) = (0, still + 1);
                    self.at
                        .now(format!("letting its file work come back, in round {round} of coming to rest"));
                    self.file_work().await;
                }
                (false, true) => return Ok(()),
            }
        }
        Ok(())
    }

    /// Waits until the sandbox is at rest, or stops it where it has come
    /// to more than it may.
    pub async fn settle(&mut self) {
        if let Err(over) = self.rests(true).await {
            self.stop(over).await;
        }
    }

    /// Stops everything the sandbox runs, for good: what it kept is
    /// still read, and nothing more is done to it. Each thing it has
    /// running is stopped as anything is, with what it does as it stops
    /// done (a stand-in asked then answers at once); what is still going
    /// after a good many rounds of that is halted where it stands.
    async fn stop(&mut self, why: String) {
        self.at.now("stopping what it runs, for it came to more than it may");
        self.over = Some(why);
        self.stood.answers(Answers::Default);
        let filter = InstanceFilter { live_only: true, roots_only: true, ..InstanceFilter::default() };
        let running = self.engine.instances(filter).await.unwrap_or_default();
        let engine = self.engine.clone();
        let stopping = tokio::spawn(async move {
            for instance in running {
                let _ = engine.stop(&instance.id).await;
            }
        });
        let _ = self.rests(false).await;
        if stopping.is_finished() {
            return;
        }
        // At rest and not stopped: what is being stopped waits for
        // something that will not come. Nothing of it goes on, and less
        // of it is read.
        stopping.abort();
        self.engine.halt();
    }
}

impl crate::Limits {
    /// Why a sandbox that has run `steps` steps, said `events` events and
    /// keeps `bytes` bytes is stopped, where it is.
    pub fn over(&self, steps: u64, events: u64, bytes: u64) -> Option<String> {
        let came = if steps > self.steps {
            format!("it has run {steps} steps, and a sandbox test runs {} at most", self.steps)
        } else if events > self.events {
            format!("it has said {events} events, and a sandbox test says {} at most", self.events)
        } else if bytes > self.bytes {
            format!(
                "what it keeps, in its journals and the logs of its steps, has come to {} MB, and a sandbox \
                 test keeps {} at most",
                bytes >> 20,
                self.bytes >> 20
            )
        } else {
            return None;
        };
        Some(format!(
            "this sandbox test was stopped: {came}. It is to be light. What it did until then is still read; to go \
             on, run it again with less (a text that goes round for ever does so here too)"
        ))
    }
}
