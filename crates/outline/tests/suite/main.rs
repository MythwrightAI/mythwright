//! One test binary for the crate; add modules here.

mod kept;

use std::path::PathBuf;
use std::sync::Arc;

use journal::{Db, EventQuery, Journal};
use kernel::{Event, ManualClock, Principal, PrincipalKind};
use outline::Outlines;
use studio::Studio;

/// A factory of its own, over a journal in memory, with its outlines.
pub struct Rig {
    pub studio: Studio,
    pub journal: Journal,
    pub outlines: Outlines,
    dir: PathBuf,
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Somebody called `name`, holding `scopes`, and by a key of theirs where `key` is said.
pub fn person(name: &str, scopes: &[&str], key: Option<&str>) -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: name.to_lowercase(),
        name: name.into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: key.map(str::to_string),
    }
}

/// The factory's owner, who holds everything.
pub fn ada() -> Principal {
    person("Ada", &["*"], None)
}

impl Rig {
    pub async fn new() -> Rig {
        let journal = Journal::memory(Arc::new(ManualClock::new())).await.unwrap();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("outline-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        let studio = Studio::open(
            journal.clone(),
            Db::memory().unwrap(),
            dir.join("projects"),
            dir.join("attachments"),
        )
        .await
        .unwrap();
        let outlines = Outlines::new(journal.clone(), studio.clone());
        Rig { studio, journal, outlines, dir }
    }

    pub async fn events(&self, topics: &str) -> Vec<Event> {
        let query = EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        self.journal.read(query).await.unwrap()
    }
}
