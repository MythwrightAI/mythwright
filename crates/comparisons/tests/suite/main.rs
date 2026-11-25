//! One test binary for the crate; add modules here.
//!
//! A conductor with nothing above it but comparisons: a journal in
//! memory, a library in memory, and the few actions these tests have
//! their blueprints do. Nothing waits on real time: the clock is the
//! test's, and what is waited for is an event.

mod accepted;
mod asked;
mod beside;
mod compared;
mod once;
mod rolled;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use blueprint::ActionSpec;
use comparisons::Comparisons;
use conductor::{ActionResult, Applied, Apply, Engine, MemoryLibrary, Registry};
use journal::{EventQuery, Journal, Logs};
use kernel::{BoxFut, Event, ManualClock, Principal, PrincipalKind};
use serde_json::{Value, json};

/// What the test actions count, by the key each was handed.
#[derive(Default)]
pub struct Shared {
    counters: Mutex<HashMap<String, u64>>,
}

impl Shared {
    fn bump(&self, key: &str) -> u64 {
        let mut counters = self.counters.lock().unwrap();
        let count = counters.entry(key.to_string()).or_default();
        *count += 1;
        *count
    }

    pub fn count(&self, key: &str) -> u64 {
        self.counters.lock().unwrap().get(key).copied().unwrap_or(0)
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

pub struct Rig {
    pub engine: Engine,
    /// What is asked of comparisons, attached to the engine there is now.
    pub compared: Comparisons,
    pub clock: ManualClock,
    pub journal: Journal,
    pub library: Arc<MemoryLibrary>,
    pub shared: Arc<Shared>,
    logs: Logs,
    dir: std::path::PathBuf,
    /// The order things were said in, with what was passed on and not
    /// kept: what a test reads of what was said.
    pub order: Arc<journal::Order>,
}

/// Each rig writes step logs to its own directory; take it away afterwards.
impl Drop for Rig {
    fn drop(&mut self) {
        self.engine.halt();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn registry(shared: &Arc<Shared>) -> Registry {
    let mut registry = Registry::new();
    registry.add_fn(
        ActionSpec::new("test.echo", "Return what it was given.").open().idempotent(),
        |_, args| -> BoxFut<'static, ActionResult> { Box::pin(async move { Ok(Value::Object(args)) }) },
    );
    let counts = shared.clone();
    registry.add_fn(
        ActionSpec::new("test.count", "Count calls under a key.")
            .required("key", blueprint::ValueType::String, "the counter")
            .idempotent(),
        move |_, args| -> BoxFut<'static, ActionResult> {
            let counts = counts.clone();
            Box::pin(async move { Ok(json!({ "count": counts.bump(args["key"].as_str().unwrap()) })) })
        },
    );
    // What a blueprint links is read by the runner's own action, which is
    // where a way's other words are given in place of a file.
    blocks::words::register(&mut registry);
    comparisons::register(&mut registry);
    registry
}

pub fn user() -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: "ada".into(),
        name: "Ada".into(),
        scopes: vec!["*".into()],
        key: None,
    }
}

/// Fails the test instead of hanging it when something never happens.
pub async fn soon<T>(what: &str, work: impl Future<Output = T>) -> T {
    match tokio::time::timeout(Duration::from_secs(20), work).await {
        Ok(value) => value,
        Err(_) => panic!("timed out waiting for: {what}"),
    }
}

impl Rig {
    pub async fn new() -> Rig {
        let clock = ManualClock::new();
        let journal = Journal::memory(Arc::new(clock.clone())).await.unwrap();
        // What was said, kept or passed on, for the test to read.
        let order = Arc::new(journal::Order::default());
        journal.orders_by(order.clone());
        let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("comparisons-{}", kernel::id::secret()));
        let logs = Logs::new(dir.clone());
        let shared = Arc::new(Shared::default());
        let library = Arc::new(MemoryLibrary::new());
        let (engine, compared) = conductor(&journal, &logs, &shared, &library).await;
        Rig { engine, compared, clock, journal, library, shared, logs, dir, order }
    }

    /// Stops the conductor as a crash would, then starts a new one on the
    /// same journal, with comparisons attached as a daemon attaches them,
    /// and lets it pick up where the old one was.
    pub async fn restart(&mut self) {
        self.engine.halt();
        // Nothing of the old one is left to take what the test then gives
        // in place of the new: whoever counted is gone with it.
        self.engine.halted().await;
        (self.engine, self.compared) =
            conductor(&self.journal, &self.logs, &self.shared, &self.library).await;
        self.engine.resume().await.unwrap();
    }

    /// Stops the conductor as a crash would, and starts a new one on the
    /// same journal with comparisons attached, and does not have it take
    /// up anything yet: nothing counts until something asks it to.
    pub async fn stopped(&mut self) {
        self.engine.halt();
        self.engine.halted().await;
        (self.engine, self.compared) =
            conductor(&self.journal, &self.logs, &self.shared, &self.library).await;
    }

    pub fn request(&self, source: &str, params: Value) -> Apply {
        let name =
            blueprint::check(source, &blueprint::NoCatalog).blueprint.map(|b| b.name).unwrap_or_default();
        Apply { source: Some(source.to_string()), params: kernel::fields(params), ..Apply::new(name, user()) }
    }

    pub async fn apply(&self, source: &str, params: Value) -> Applied {
        self.engine
            .apply(self.request(source, params))
            .await
            .unwrap_or_else(|err| panic!("apply failed: {}", err.message))
    }

    /// The first stored event on `topic` that satisfies `wanted`, waiting for
    /// it if it has not happened yet.
    pub async fn event(&self, topic: &str, wanted: impl Fn(&Event) -> bool) -> Arc<Event> {
        let mut follow = self.order.follow_every(&self.journal, topic);
        soon(&format!("an event on `{topic}`"), async {
            loop {
                let event = follow.next().await.unwrap();
                if wanted(&event) {
                    return event;
                }
            }
        })
        .await
    }

    pub async fn events(&self, topics: &str) -> Vec<Event> {
        // What was said, kept or passed on.
        let query = EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        self.order.read_every(&self.journal, query).await.unwrap()
    }
}

/// A conductor on `journal`, and comparisons attached to it.
async fn conductor(
    journal: &Journal,
    logs: &Logs,
    shared: &Arc<Shared>,
    library: &Arc<MemoryLibrary>,
) -> (Engine, Comparisons) {
    let engine = Engine::new(journal.clone(), logs.clone(), registry(shared), library.clone()).await.unwrap();
    let compared = Comparisons::attach(&engine);
    (engine, compared)
}

/// Waits until `instance` says it is active.
pub async fn active(rig: &Rig, instance: &str) {
    rig.event("instance.active", |event| event.source.instance.as_deref() == Some(instance)).await;
}

/// Waits until `count` runs of `flow` have finished.
pub async fn handled(rig: &Rig, flow: &str, count: usize) {
    let mut follow = rig.order.follow_every(&rig.journal, "run.finished");
    let mut seen = 0;
    soon(&format!("{count} runs of {flow}"), async {
        while seen < count {
            if follow.next().await.unwrap().data["flow"] == flow {
                seen += 1;
            }
        }
    })
    .await;
}
