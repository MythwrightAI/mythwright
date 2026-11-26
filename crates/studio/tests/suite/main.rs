//! One test binary for the crate; add modules here.

mod added;
mod apart;
mod attaching;
mod cards;
mod change;
mod collab;
mod counted;
mod documents;
mod first;
mod inside;
mod looked;
mod models;
mod moved;
mod ordered;
mod pages;
mod projects;
mod records;
mod redeclared;
mod references;
mod reworded;
mod rows;
mod search;
mod shown;
mod steered;
mod stepped;
mod stores;
mod takes;
mod trashed;
mod windowed;
mod workspaces;

use std::path::PathBuf;
use std::sync::Arc;

use journal::{Db, EventQuery, Journal};
use kernel::{Event, ManualClock, Principal, PrincipalKind};
use studio::Studio;

pub struct Rig {
    pub studio: Studio,
    pub journal: Journal,
    pub clock: ManualClock,
    /// Where new projects are created.
    pub dir: PathBuf,
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Whether the suite is played with every project kept in the factory's
/// own journal (`PROJECT_JOURNALS=one`, `journal::apart`), as a factory
/// from before keeps them. A test of a project's own journal holds
/// nothing then, and is not played: it asks first, and says so.
pub fn one_journal() -> bool {
    let one = journal::apart() == Some(false);
    if one {
        println!("not played: it is of a project's own journal, and the suite is played with one journal");
    }
    one
}

/// Leave to read and write any channel, for what a test here does to
/// the channels its own rig holds.
pub fn own() -> studio::Reader {
    studio::Channels::daemon("a test reads and writes what its own rig holds")
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
        let clock = ManualClock::new();
        let journal = Journal::memory(Arc::new(clock.clone())).await.unwrap();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("studio-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        let studio = Studio::open(
            journal.clone(),
            Db::memory().unwrap(),
            dir.join("projects"),
            dir.join("attachments"),
        )
        .await
        .unwrap();
        Rig { studio, journal, clock, dir }
    }

    /// A project with a board of the usual columns, kept where `store`
    /// says: what applying `std/board` to it makes.
    pub async fn board_in(&self, project: &str, store: studio::Store) {
        if self.studio.projects.dir_of(&blueprint::expr::slug(project)).is_none() {
            self.studio.projects.create(project, None, &ada()).await.unwrap();
        }
        let columns = studio::board::DEFAULT_COLUMNS.iter().map(|column| column.to_string()).collect();
        self.studio.board.make(&blueprint::expr::slug(project), columns, store, None, &ada()).await.unwrap();
    }

    pub async fn board(&self, project: &str) {
        self.board_in(project, studio::Store::Local).await;
    }

    pub async fn events(&self, topics: &str) -> Vec<Event> {
        let query = EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        self.journal.read(query).await.unwrap()
    }

    pub async fn topics(&self, topics: &str) -> Vec<String> {
        self.events(topics).await.into_iter().map(|event| event.topic).collect()
    }
}
