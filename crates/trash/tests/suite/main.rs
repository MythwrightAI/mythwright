//! One test binary for the crate; add modules here.

mod channels;
mod trashed;

use std::path::PathBuf;
use std::sync::Arc;

use journal::{Db, EventQuery, Journal};
use kernel::{Event, ManualClock, Principal, PrincipalKind};
use studio::Studio;
use trash::Trash;

pub struct Rig {
    pub studio: Studio,
    pub trash: Trash,
    pub journal: Journal,
    pub clock: ManualClock,
    /// Where new projects are created, and where the factory's own data is.
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
        let dir =
            PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("trash-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        let studio = Studio::open(
            journal.clone(),
            Db::memory().unwrap(),
            dir.join("projects"),
            dir.join("attachments"),
        )
        .await
        .unwrap();
        // The factory's own data folder, where files are kept while they are in the Trash.
        studio.projects.keeps_out_of(&dir.join("data"));
        let trash = Trash::of(&studio, &journal);
        Rig { studio, trash, journal, clock, dir }
    }

    /// A project with a board of the usual columns.
    pub async fn board(&self, project: &str) {
        if self.studio.projects.dir_of(&blueprint::expr::slug(project)).is_none() {
            self.studio.projects.create(project, None, &ada()).await.unwrap();
        }
        let columns = studio::board::DEFAULT_COLUMNS.iter().map(|column| column.to_string()).collect();
        let local = studio::Store::Local;
        self.studio.board.make(&blueprint::expr::slug(project), columns, local, None, &ada()).await.unwrap();
    }

    pub async fn events(&self, topics: &str) -> Vec<Event> {
        let query = EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        self.journal.read(query).await.unwrap()
    }

    pub async fn topics(&self, topics: &str) -> Vec<String> {
        self.events(topics).await.into_iter().map(|event| event.topic).collect()
    }
}
