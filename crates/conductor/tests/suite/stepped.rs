//! The database answered a turn at a time, so that which of two things
//! a conductor is doing comes to something first is what the test wrote
//! and not what the machine had to do.
//!
//! Everything a conductor does runs on the one thread a test has, and
//! goes forward by asking the database, whose own thread answers in the
//! order it is asked. Left to itself that is two things racing: on a
//! machine with processors to spare the answers come one at a time and
//! what the conductor has running takes turns, and on one with none the
//! answer is there before whoever asked has looked away, so the first
//! thing to run goes on to its end before the next has begun. A play
//! that is there for one order of two things (the step that asked for an
//! instance coming to it before the conductor's start does) came to it
//! on the first kind of machine and not on the second, and said so by
//! failing: with nothing wrong in what it holds.
//!
//! So where the order matters the database is held between turns
//! ([`Stepped`]). Whatever is running goes on until it has asked the
//! database something and waits; then all that was asked is answered, in
//! the order it was asked, and the database is held again. Each thing
//! the conductor has running goes forward by one answer a turn, on any
//! machine, and which comes first is how many answers each needs: what
//! the play put between them.
//!
//! A factory has several databases once a project has a journal of its
//! own, each with a thread, and every one of them is held: the factory's
//! own and each project's. In a turn the factory's answers what it was
//! asked, then each project's in the order of their names, and all are
//! held again. One thing a project's thread asks of the factory's in the
//! middle of its own work (a run of numbers, the first after it is
//! opened), so the factory's is left free while the projects' answer, and
//! held last. Nothing the conductor has running is let run on meanwhile:
//! the test's thread waits where it is, so that what a project's answer
//! wakes cannot ask the factory's database for a second answer in the
//! turn. A journal made meanwhile is held from the turn after.
//!
//! Nothing here waits on a clock, and nothing in the runner knows of it:
//! the database is held by a piece of work asked of it like any other,
//! which waits to be let go.
//!
//! A play holds its databases from when it is begun until it is over,
//! through each start of its conductor and whatever it asks itself
//! ([`Stepped::holding`], [`Stepped::through`]). Held only while it
//! waited to be at rest, a play had moments with the databases let go:
//! as a conductor's start began and as it ended, and whenever the play
//! did something. In such a moment the machine says again who goes
//! first, and more than that: an answer may be there before whoever
//! asked has looked for it, and then that one goes on without giving
//! way, through several answers, while what is beside it stands still.
//! What a conductor kept after a start came in another order from one
//! run to the next, and with it how many plays were made of a scenario;
//! and once in some thousands of plays somebody who stopped an instance
//! went on, in such a moment, from calling off what was under it to what
//! the first of those says as it stops, and what waited for that was
//! then looked at with both at hand ([`Stepped::at_once`] makes that
//! order, where a play is there for it).

use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::{Arc, mpsc};
use std::task::{Context, Poll, Wake};

use journal::{Db, Journal};
use tokio::sync::oneshot;

/// How often whatever is running is let run on, each turn, before what
/// it has asked is counted as all it will ask: one thing woken by
/// another runs a turn of the thread later.
const RUNS_ON: usize = 8;

/// One database that is held between turns.
struct Held {
    db: Db,
    /// What lets go the piece of work that holds it now.
    go: Option<mpsc::Sender<()>>,
    /// How much it had done when it was last held.
    done: u64,
}

/// A factory's databases answered a turn at a time, for as long as this
/// is kept. Put away, they are let go, and answer as they are asked.
pub struct Stepped {
    journal: Journal,
    /// Each database there was at the last turn; the factory's own last.
    held: Vec<Held>,
}

/// Wakes the thread that waits where it is.
struct Waits(std::thread::Thread);

impl Wake for Waits {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Waits for `asked` where the thread is, with nothing else that runs on
/// it let run meanwhile: only what other threads do goes forward.
fn still<T>(asked: impl Future<Output = T>) -> T {
    let waker = Arc::new(Waits(std::thread::current())).into();
    let mut cx = Context::from_waker(&waker);
    // What is waited on here is asked many times in one turn of the
    // thread, and is not to be told to give way to what is not let run.
    let mut asked = pin!(tokio::task::unconstrained(asked));
    loop {
        if let Poll::Ready(came) = asked.as_mut().poll(&mut cx) {
            return came;
        }
        std::thread::park();
    }
}

/// Asks `db` for a piece of work that holds it until it is let go, behind
/// everything asked of it so far: what says it is held, and what lets go.
fn hold(db: &Db) -> (oneshot::Receiver<()>, mpsc::Sender<()>) {
    let (entered, enters) = oneshot::channel();
    let (let_go, go) = mpsc::channel::<()>();
    drop(db.call_apart(move |_| {
        let _ = entered.send(());
        // Let go, or put away: either ends the wait.
        let _ = go.recv();
        Ok(())
    }));
    (enters, let_go)
}

impl Stepped {
    /// A factory's databases held from now, with nothing let run on
    /// first: whoever is about to begin something that is to go forward
    /// a turn at a time holds them before it is first looked at. Begun
    /// with nothing held, it may have each answer before it looks for
    /// it and go on to its end at once, on a busy machine: a test that
    /// held only from its first turn failed its own check that its
    /// order had come about, once in six runs of the suite held to four
    /// processors.
    pub fn holding(journal: &Journal) -> Stepped {
        let mut stepped = Stepped { journal: journal.clone(), held: Vec::new() };
        stepped.answer(true);
        stepped
    }

    /// The databases there are now, each with what was kept of it: one
    /// made since the last turn is held from this one.
    fn every(&mut self) -> Vec<Held> {
        let mut before = std::mem::take(&mut self.held);
        let mut every = Vec::new();
        for db in self.journal.dbs() {
            let at = before.iter().position(|held| held.db.is(&db));
            every.push(match at {
                Some(at) => before.swap_remove(at),
                None => Held { done: db.done(), db, go: None },
            });
        }
        every
    }

    /// Lets every database go: each answers as it is asked, until the
    /// next turn.
    pub fn let_go(&mut self) {
        self.held.clear();
    }

    /// One turn: what is running goes on until it waits, what it asked
    /// of each database is answered, and every database is held again.
    /// Gives how many things were answered, which is none where nothing
    /// is waiting on any database any more.
    ///
    /// Nothing, where the journal shut and `shut_ends`: a journal that
    /// shuts and holds its databases answers nothing more, this included.
    pub async fn turn(&mut self, shut_ends: bool) -> Option<u64> {
        for _ in 0..RUNS_ON {
            tokio::task::yield_now().await;
        }
        // Nothing was asked of any of them: there is nothing to answer,
        // and each stays held as it is. Let go and held again, each
        // answered nothing all the same, at the cost of a round trip to
        // its thread and two commits of nothing; a play waits a dozen
        // such turns each time it is to be at rest, and they were a
        // quarter of all its turns and a fifth of what the suite took.
        if self.asked_nothing() {
            return Some(0);
        }
        self.answer(shut_ends)
    }

    /// Whether every database there is, is held, with nothing asked of
    /// it behind what holds it. Not said of a journal that has shut,
    /// whose databases answer nothing, nor where one was made since the
    /// last turn, which is not held yet.
    fn asked_nothing(&self) -> bool {
        let dbs = self.journal.dbs();
        let held = |db: &Db| self.held.iter().any(|held| held.go.is_some() && held.db.is(db));
        !self.journal.is_shut()
            && dbs.len() == self.held.len()
            && dbs.iter().all(|db| held(db) && db.waiting() == 0)
    }

    /// Waits for what whoever holds the databases asks itself, a turn at
    /// a time: what the conductor has running goes forward beside it as
    /// it does in any turn. Nothing, where the journal shut before it
    /// was answered: where it shut and holds its databases (`holds`), it
    /// never is.
    pub async fn through<T>(&mut self, holds: bool, asked: impl Future<Output = T>) -> Option<T> {
        let mut asked = pin!(asked);
        loop {
            let once = std::future::poll_fn(|cx| Poll::Ready(asked.as_mut().poll(cx)));
            if let Poll::Ready(came) = once.await {
                return Some(came);
            }
            if self.journal.is_shut() {
                return None;
            }
            self.turn(holds).await?;
        }
    }

    /// Does `work` without giving way, the databases let go meanwhile:
    /// each answer is there before it is looked for, so nothing else
    /// that is running is looked at from when it begins until it waits
    /// for something that is no answer of a database, or is over. Which
    /// is what a busy machine does to whoever asks the database several
    /// things in a row. The databases are held again from there, with
    /// what its answers woke not yet looked at; whoever asked goes on
    /// with `work` where it is not over.
    pub fn at_once<T>(&mut self, mut work: Pin<&mut (dyn Future<Output = T> + Send)>) -> Poll<T> {
        self.let_go();
        let mut cx = Context::from_waker(std::task::Waker::noop());
        let came = loop {
            let dbs = self.journal.dbs();
            let before: u64 = dbs.iter().map(Db::done).sum();
            if let Poll::Ready(came) = work.as_mut().poll(&mut cx) {
                break Poll::Ready(came);
            }
            // Behind what it asked of each: a mark, whose answer is
            // waited for where the thread is. A database has counted a
            // thing done by the time it answers it, so what was done
            // beyond the marks is what `work` asked. A journal that shut
            // answers none.
            let journal = self.journal.clone();
            let shut = still(async {
                for db in &dbs {
                    tokio::select! {
                        biased;
                        _ = db.call_apart(|_| Ok(())) => {}
                        () = journal.shut() => return true,
                    }
                }
                false
            });
            let asked = dbs.iter().map(Db::done).sum::<u64>() - before > dbs.len() as u64;
            if shut || !asked {
                break Poll::Pending;
            }
        };
        if !self.journal.is_shut() {
            self.answer(true);
        }
        came
    }

    /// What was asked of each database is answered, and every database
    /// is held again: how many things were answered.
    fn answer(&mut self, shut_ends: bool) -> Option<u64> {
        let mut every = self.every();
        let journal = self.journal.clone();
        // Waited for where the thread is: whatever an answer wakes runs
        // once every database is held again, and asks behind that.
        let answered = still(async {
            let mut ours = 0;
            // Either what was waited for, or nothing where the journal
            // shut meanwhile and that ends the turn.
            let or_shut = async |held: oneshot::Receiver<()>| {
                tokio::select! {
                    biased;
                    held = held => held.ok(),
                    () = journal.shut(), if shut_ends => None,
                }
            };
            let (own, projects) = every.split_last_mut().expect("a factory has a database of its own");
            if !projects.is_empty() {
                // The factory's own answers what it was asked, and is
                // left free: a mark is all that is asked behind that.
                let (marked, mark) = oneshot::channel();
                drop(own.db.call_apart(move |_| {
                    let _ = marked.send(());
                    Ok(())
                }));
                ours += 1 + u64::from(own.go.take().is_some());
                or_shut(mark).await?;
                for project in projects {
                    let (enters, let_go) = hold(&project.db);
                    // The one before is let go only now, so that there
                    // is never a moment with nothing holding it behind
                    // what was asked.
                    ours += u64::from(project.go.replace(let_go).is_some());
                    or_shut(enters).await?;
                }
            }
            let (enters, let_go) = hold(&own.db);
            ours += u64::from(own.go.replace(let_go).is_some());
            or_shut(enters).await?;
            // While each is held, what it has done is what was done
            // before it was: what was answered this turn, and what held
            // it before.
            let mut answered = 0;
            for held in every.iter_mut() {
                let done = held.db.done();
                answered += done - held.done;
                held.done = done;
            }
            Some(answered.saturating_sub(ours))
        });
        self.held = every;
        answered
    }
}
