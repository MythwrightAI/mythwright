//! One test binary for the crate; add modules here.

mod answered;
mod archived;
mod channels;
mod commented;
mod conversed;
mod door;
mod inbox;
mod kept;
mod managed;
mod members;
mod taken;
mod telling;

use std::path::PathBuf;
use std::sync::Arc;

use journal::{EventQuery, Journal};
use kernel::{Event, ManualClock, Principal, PrincipalKind};
use talk::{Attachments, Channels};

pub struct Rig {
    pub channels: Channels,
    pub journal: Journal,
    /// Where what is attached to messages is kept.
    pub dir: PathBuf,
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub fn ada() -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: "ada".into(),
        name: "Ada".into(),
        scopes: vec!["*".into()],
        key: None,
    }
}

/// Leave to read and write any channel, for what a test here does to
/// the channels its own rig holds.
pub fn own() -> talk::Reader {
    Channels::daemon("a test reads and writes what its own rig holds")
}

pub fn agent(name: &str, scopes: &[&str]) -> Principal {
    Principal {
        kind: PrincipalKind::Agent,
        id: name.into(),
        name: name.into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

impl Rig {
    pub async fn new() -> Rig {
        let journal = Journal::memory(Arc::new(ManualClock::new())).await.unwrap();
        let dir =
            PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("talk-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        let attachments = Attachments::open(journal.clone(), dir.join("attachments")).await.unwrap();
        let channels = Channels::open(journal.clone(), attachments).await.unwrap();
        Rig { channels, journal, dir }
    }

    pub async fn events(&self, topics: &str) -> Vec<Event> {
        let query = EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        self.journal.read(query).await.unwrap()
    }

    pub async fn topics(&self, topics: &str) -> Vec<String> {
        self.events(topics).await.into_iter().map(|event| event.topic).collect()
    }
}
