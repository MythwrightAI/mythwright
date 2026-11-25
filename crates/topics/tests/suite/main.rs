//! One test binary for the crate; add modules here.

mod context;
mod kept;
mod linked;
mod trashed;
mod tree;

use std::path::PathBuf;
use std::sync::Arc;

use journal::{Db, EventQuery, Journal};
use kernel::{Event, ManualClock, Principal, PrincipalKind};
use serde_json::json;
use studio::Studio;
use studio::collections::Spec;
use topics::Topics;

/// A factory of its own, over a journal in memory, owned by Ada, with
/// its topics' tables declared as `std/topics` declares them.
pub struct Rig {
    pub studio: Studio,
    pub journal: Journal,
    pub topics: Topics,
    dir: PathBuf,
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.studio.halt();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Somebody called `name`, holding `scopes`.
pub fn person(name: &str, scopes: &[&str]) -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: name.to_lowercase(),
        name: name.into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

/// The factory's owner, who holds everything.
pub fn ada() -> Principal {
    person("Ada", &["*"])
}

/// The fields of the factory's topics, as `std/topics` writes them.
fn fields() -> serde_json::Value {
    json!({
        "title": { "type": "text", "required": true },
        "body": { "type": "document" },
        "summary": { "type": "document" },
        "status": { "type": "choice", "of": ["open", "answered", "closed", "standing"] },
        "workspace": { "type": "text", "quiet": true, "hidden": true },
        "project": { "type": "text", "quiet": true, "hidden": true },
        "followers": { "type": "list", "quiet": true, "hidden": true },
        "answer": { "type": "text", "quiet": true },
    })
}

impl Rig {
    pub async fn new() -> Rig {
        let journal = Journal::memory(Arc::new(ManualClock::new())).await.unwrap();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("topics-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        let studio =
            Studio::open(journal.clone(), Db::memory().unwrap(), dir.join("projects"), dir.join("attached"))
                .await
                .unwrap();
        studio.workspaces.settle(Some("Ada")).await.unwrap();
        Topics::open(&journal, &studio).await.unwrap();
        let history = json!(["record.*", "topic.*"]);
        let spec = Spec::read(&fields(), &json!(null), None).unwrap().modelled(Some("topic"));
        let spec = spec.with_history(&history).unwrap();
        studio.collections.ensure("@", "topics", spec, None, &ada()).await.unwrap();
        let topics = Topics::new(journal.clone(), studio.clone(), None);
        Rig { studio, journal, topics, dir }
    }

    pub async fn events(&self, topics: &str) -> Vec<Event> {
        let query = EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        self.journal.read(query).await.unwrap()
    }
}
