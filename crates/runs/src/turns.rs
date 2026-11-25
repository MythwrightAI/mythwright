//! Turns taken by key.
//!
//! A handler whose text says `turn:` has what it hears done in turns, by a
//! key worked out from each event. What comes to one key is done one at a
//! time, and what one handler hears of it in the order it was said; what
//! comes to another is done beside it. That is what lets one instance look
//! after many things of a kind (the rows of a table, say) without one of
//! them holding up the rest, and without two things said of the same one
//! being done at once.
//!
//! The turns are the instance's and not a handler's. Every handler of it
//! that comes to the same key takes turns with the others; and a handler
//! that takes the place of one of the same name in the text before joins
//! the same turns, behind whatever of the old one's is still being done.
//!
//! Where the conductor stopped with turns taken and waited for, they are
//! given again in the order they were first given ([`Again`]): what was
//! in hand for a key goes on first, and what waited behind it after,
//! whichever handler each was of.
//!
//! It came out of the runner, which offers it under the name it always
//! gave (`crate::turns` there), and hands it what begins a task.

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio::sync::watch;
use tokio::task::JoinSet;

use crate::{Hands, RunInfo};

type Job = Pin<Box<dyn Future<Output = ()> + Send>>;

/// The turns an instance's handlers take.
#[derive(Default)]
pub struct Turns {
    /// Held by the run of a handler of one whose text says
    /// `one_at_a_time`, for as long as it runs: one run of any of its
    /// handlers at a time.
    pub whole: Arc<tokio::sync::Mutex<()>>,
    /// The turns taken by key, where a handler says `turn:`.
    pub keyed: Arc<Keyed>,
}

/// What waits its turn, by key. A key is here for as long as something
/// that came to it is being done.
#[derive(Default)]
pub struct Keyed {
    waiting: Mutex<HashMap<String, VecDeque<Job>>>,
}

impl Keyed {
    /// Has `job` done when everything that came to `key` before it has
    /// been done: at once, where nothing of that key is in hand.
    pub fn take(
        self: &Arc<Self>,
        hands: &impl Hands,
        key: String,
        job: impl Future<Output = ()> + Send + 'static,
    ) {
        let mut waiting = self.waiting.lock().expect("turns lock");
        if let Some(behind) = waiting.get_mut(&key) {
            behind.push_back(Box::pin(job));
            return;
        }
        waiting.insert(key.clone(), VecDeque::new());
        drop(waiting);
        let turns = self.clone();
        // The conductor's own task, so that one that halts takes what is
        // being done for every key with it.
        hands.spawn(async move {
            let mut job: Job = Box::pin(job);
            loop {
                // Each in a task of its own: one that panics takes only
                // itself away, and what waits behind it has its turn.
                let mut one = JoinSet::new();
                one.spawn(job);
                let _ = one.join_next().await;
                let Some(next) = turns.next(&key) else { return };
                job = next;
            }
        });
    }

    /// What came to `key` next; with nothing left, the key is let go.
    fn next(&self, key: &str) -> Option<Job> {
        let mut waiting = self.waiting.lock().expect("turns lock");
        let next = waiting.get_mut(key).and_then(VecDeque::pop_front);
        if next.is_none() {
            waiting.remove(key);
        }
        next
    }
}

/// The order in which the runs that had or waited for a turn when the
/// conductor stopped are given their turns again.
///
/// Each handler takes up its own runs, in a task of its own, so left to
/// themselves two handlers that came to one key would be given their
/// turns in whichever order their tasks got there, and what only waited
/// could begin before what was in hand went on. Each waits here for the
/// runs before it to have been given their turns first.
pub struct Again {
    place: HashMap<String, usize>,
    given: watch::Sender<usize>,
}

impl Again {
    /// What runs that were under way are put in order by: when each was
    /// first given its place, and of two given theirs in the same instant,
    /// the one that had begun first.
    pub fn first(run: &RunInfo) -> (i64, bool, String) {
        (run.started, run.steps.is_empty(), run.id.clone())
    }

    /// The order of `runs`, which is the order they are handed in.
    pub fn of<'a>(runs: impl Iterator<Item = &'a RunInfo>) -> Arc<Self> {
        let place = runs.enumerate().map(|(place, run)| (run.id.clone(), place)).collect();
        Arc::new(Self { place, given: watch::Sender::new(0) })
    }

    /// Waits until every run before `run` has been given its turn.
    pub async fn turn_of(&self, run: &str) {
        let Some(place) = self.place.get(run).copied() else { return };
        let mut given = self.given.subscribe();
        let _ = given.wait_for(|given| *given >= place).await;
    }

    /// Waits until every run that had or waited for a turn when the
    /// conductor stopped has been given its turn again. What a handler
    /// hears afterwards waits here before it takes a turn of its own: a
    /// handler with nothing under way begins to listen at once, and what
    /// it heard first took a key ahead of what had been in hand for it.
    pub async fn all_given(&self) {
        let mut given = self.given.subscribe();
        let _ = given.wait_for(|given| *given >= self.place.len()).await;
    }

    /// `run` has been given its turn: the next may be given its own.
    pub fn given(&self, run: &str) {
        if self.place.contains_key(run) {
            self.given.send_modify(|given| *given += 1);
        }
    }
}
