//! One test binary for the crate; add modules here.

mod asks;

use std::path::PathBuf;
use std::sync::Arc;

use journal::{Db, EventQuery, Journal};
use kernel::{Event, ManualClock, Principal, PrincipalKind};
use studio::Studio;

/// A factory of its own, over a journal in memory, with Ada (who owns it)
/// and Grace (a member) signed up.
pub struct Rig {
    pub studio: Studio,
    pub journal: Journal,
    pub clock: ManualClock,
    pub ada: Principal,
    pub grace: Principal,
    dir: PathBuf,
}

impl Drop for Rig {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// An agent called `name`, holding `scopes`.
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
            PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("told-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        let studio = Studio::open(
            journal.clone(),
            Db::memory().unwrap(),
            dir.join("projects"),
            dir.join("attachments"),
        )
        .await
        .unwrap();
        let identity = &studio.identity;
        let ada = identity.register("ada@example.com", "Ada", "correct horse", None, true).await.unwrap();
        let ada = ada.account.principal();
        identity.set_tier(&ada, studio::Tier::Premium).await.unwrap();
        let code = identity.invite(&ada, Some("grace@example.com")).await.unwrap();
        let grace = identity
            .register("grace@example.com", "Grace", "correct horse", Some(&code), false)
            .await
            .unwrap();
        let grace = grace.account.principal();
        Rig { studio, journal, clock, ada, grace, dir }
    }

    pub async fn events(&self, topics: &str) -> Vec<Event> {
        let query = EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        self.journal.read(query).await.unwrap()
    }
}
