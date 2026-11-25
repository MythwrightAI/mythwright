//! What a sandbox's thread is at, for whoever waits on it.
//!
//! Whoever holds a sandbox waits on its thread for every answer, and a
//! wait that runs out of patience and names nothing is found again only
//! by chance. So the thread says, as it goes, what it is doing and
//! since when, and hands over its databases once it has them: whoever
//! gave up waiting can then say what the sandbox was at, and what each
//! of its databases was at ([`journal::Db::standing`]). Nothing here is
//! read by the sandbox itself, and nothing waits on it.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use journal::Db;

#[derive(Default)]
struct Seen {
    doing: String,
    since: Option<Instant>,
    dbs: Vec<Db>,
}

/// Said by the sandbox's thread, read by whoever holds the sandbox.
#[derive(Clone, Default)]
pub(crate) struct At {
    seen: Arc<Mutex<Seen>>,
    /// How many times its runtime has had nothing left to run, and how
    /// many of its own actions are at work: shared with what says when
    /// it is at rest, and only read here.
    pub(crate) parked: Arc<AtomicU64>,
    pub(crate) working: Arc<AtomicUsize>,
}

impl At {
    /// The thread is now doing `what`.
    pub(crate) fn now(&self, what: impl Into<String>) {
        let mut seen = self.seen.lock().expect("sandbox lock");
        seen.doing = what.into();
        seen.since = Some(Instant::now());
    }

    /// The databases the sandbox keeps anything in, once it has them.
    pub(crate) fn keeps_in(&self, dbs: Vec<Db>) {
        self.seen.lock().expect("sandbox lock").dbs = dbs;
    }

    /// In words: what the thread is at and for how long, how often its
    /// runtime has had nothing to run, how many of its actions are at
    /// work, and how each of its databases stands.
    pub(crate) fn said(&self) -> String {
        let seen = self.seen.lock().expect("sandbox lock");
        let since = seen.since.map(|since| since.elapsed().as_millis()).unwrap_or_default();
        let mut said = format!(
            "its thread is {} (for {since} ms); its runtime has had nothing left to run {} times; {} of its \
             own actions are at work",
            if seen.doing.is_empty() { "starting" } else { &seen.doing },
            self.parked.load(Ordering::SeqCst),
            self.working.load(Ordering::SeqCst),
        );
        for (at, db) in seen.dbs.iter().enumerate() {
            said.push_str(&format!("; database {at}: {}", db.standing()));
        }
        said
    }
}
