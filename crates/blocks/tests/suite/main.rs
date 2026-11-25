//! One test binary for the crate; add modules here.
//!
//! A conductor with nothing above it but these blocks: a journal in
//! memory, a library in memory, and the few stand-in actions these tests
//! have their blueprints do beside the blocks themselves. Nothing waits
//! on real time: what is waited for is an event, and a step that is to
//! be caught under way waits at a gate the test opens.

mod held;
mod pool;
mod prompts;
mod ranked;
mod variables;
mod words;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use blueprint::{ActionSpec, ValueType};
use conductor::{
    ActionCx, ActionError, ActionResult, Applied, Apply, Engine, InstanceInfo, MemoryLibrary, Registry,
    RunInfo, StepInfo,
};
use journal::{EventQuery, Following, Journal, Logs, Order};
use kernel::{BoxFut, Event, Fields, ManualClock, Principal, PrincipalKind};
use serde_json::{Value, json};
use tokio::sync::Semaphore;

/// State the stand-in actions share with the test.
#[derive(Default)]
pub struct Shared {
    counters: Mutex<HashMap<String, u64>>,
    gates: Mutex<HashMap<String, Arc<Semaphore>>>,
}

impl Shared {
    fn gate(&self, name: &str) -> Arc<Semaphore> {
        self.gates
            .lock()
            .unwrap()
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(0)))
            .clone()
    }

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

pub struct Rig {
    pub engine: Engine,
    pub journal: Journal,
    pub library: Arc<MemoryLibrary>,
    pub shared: Arc<Shared>,
    logs: Logs,
    dir: std::path::PathBuf,
    /// What was said, kept or passed on, for a test to read.
    order: Arc<Order>,
}

/// Each rig writes step logs to its own directory; take it away afterwards.
impl Drop for Rig {
    fn drop(&mut self) {
        self.engine.halt();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Waits at the named gate until the test opens it, having said that it
/// is there.
async fn at_gate(shared: Arc<Shared>, cx: ActionCx, args: Fields) -> ActionResult {
    let gate = shared.gate(args["name"].as_str().unwrap());
    tokio::select! {
        permit = gate.acquire() => permit.unwrap().forget(),
        () = cx.cancel().cancelled() => return Err(ActionError::cancelled()),
    }
    Ok(json!({ "passed": args["name"] }))
}

/// An action that is taken up where it was after a restart: it has kept
/// its place, and waits at its gate again.
struct Durable {
    shared: Arc<Shared>,
    spec: ActionSpec,
}

impl conductor::Action for Durable {
    fn spec(&self) -> &ActionSpec {
        &self.spec
    }

    fn run(&self, cx: ActionCx, args: Fields) -> BoxFut<'static, ActionResult> {
        let shared = self.shared.clone();
        Box::pin(async move {
            cx.checkpoint(json!({ "pid": 4242 })).await?;
            cx.emit("test.reached", json!({ "gate": args["name"] })).await?;
            at_gate(shared, cx, args).await
        })
    }

    fn resume(&self, cx: ActionCx, args: Fields, _saved: Value) -> BoxFut<'static, ActionResult> {
        Box::pin(at_gate(self.shared.clone(), cx, args))
    }
}

fn registry(shared: &Arc<Shared>) -> Registry {
    use ValueType::String;
    let mut registry = Registry::new();
    registry.add_fn(
        ActionSpec::new("test.echo", "Return what it was given.").open().idempotent(),
        |_, args| -> BoxFut<'static, ActionResult> { Box::pin(async move { Ok(Value::Object(args)) }) },
    );
    let counts = shared.clone();
    registry.add_fn(
        ActionSpec::new("test.count", "Count calls under a key.")
            .required("key", String, "the counter")
            .idempotent(),
        move |_, args| -> BoxFut<'static, ActionResult> {
            let counts = counts.clone();
            Box::pin(async move { Ok(json!({ "count": counts.bump(args["key"].as_str().unwrap()) })) })
        },
    );
    let gates = shared.clone();
    registry.add_fn(
        ActionSpec::new("test.gate", "Wait until the test opens the named gate.").required(
            "name",
            String,
            "which gate",
        ),
        move |cx, args| -> BoxFut<'static, ActionResult> {
            let gates = gates.clone();
            Box::pin(async move {
                cx.emit("test.reached", json!({ "gate": args["name"] })).await?;
                at_gate(gates, cx, args).await
            })
        },
    );
    registry.add(Durable {
        shared: shared.clone(),
        spec: ActionSpec::new("test.durable", "Checkpoints, then waits at a gate.").required(
            "name",
            String,
            "which gate",
        ),
    });
    blocks::register(&mut registry);
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
        let journal = Journal::memory(Arc::new(clock)).await.unwrap();
        let order = Arc::new(Order::default());
        journal.orders_by(order.clone());
        let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("blocks-{}", kernel::id::secret()));
        let logs = Logs::new(dir.clone());
        let shared = Arc::new(Shared::default());
        let library = Arc::new(MemoryLibrary::new());
        let engine =
            Engine::new(journal.clone(), logs.clone(), registry(&shared), library.clone()).await.unwrap();
        Rig { engine, journal, library, shared, logs, dir, order }
    }

    /// Stops the conductor as a crash would, then starts a new one on the
    /// same journal and lets it pick up where the old one was.
    pub async fn restart(&mut self) {
        self.engine.halt();
        // Nothing of the old one is left to take what the test then gives
        // (a gate it opens) in place of the new.
        self.engine.halted().await;
        self.engine = Engine::new(
            self.journal.clone(),
            self.logs.clone(),
            registry(&self.shared),
            self.library.clone(),
        )
        .await
        .unwrap();
        self.engine.resume().await.unwrap();
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

    /// Applies `source` and waits for its steps to finish.
    pub async fn run(&self, source: &str, params: Value) -> RunInfo {
        let applied = self.apply(source, params).await;
        self.finished(&applied).await
    }

    pub async fn finished(&self, applied: &Applied) -> RunInfo {
        let run = applied.run.as_ref().expect("a run was started");
        soon("the run to finish", self.engine.wait_run(run)).await.unwrap()
    }

    pub async fn instance(&self, id: &str) -> InstanceInfo {
        self.engine.instance(id).await.unwrap()
    }

    /// The first stored event on `topic` that satisfies `wanted`, waiting
    /// for it if it has not happened yet. There is one journal here, so
    /// one event is after another, or after a [`Rig::mark`], where its
    /// `seq` is the greater.
    pub async fn event(&self, topic: &str, wanted: impl Fn(&Event) -> bool) -> Arc<Event> {
        let mut follow = self.follow(topic);
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

    /// Waits until an action is waiting at the named gate.
    pub async fn reached(&self, gate: &str) -> Arc<Event> {
        self.event("test.reached", |event| event.data["gate"] == gate).await
    }

    /// Every stored event matching `topics`, in order.
    pub async fn events(&self, topics: &str) -> Vec<Event> {
        // What was said, kept or passed on.
        let query = EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        self.order.read_every(&self.journal, query).await.unwrap()
    }

    /// Follows what is said on `topics`, from the beginning.
    pub fn follow(&self, topics: &str) -> Followed {
        self.follow_after(0, topics)
    }

    /// Follows what is said on `topics` after `mark`: a [`Rig::mark`],
    /// or the `seq` of an event the rig handed over.
    pub fn follow_after(&self, mark: u64, topics: &str) -> Followed {
        Followed { follow: self.order.follow_every(&self.journal, topics), after: mark }
    }

    /// Where the last thing said came: everything said from here on has
    /// a greater `seq`.
    pub fn mark(&self) -> u64 {
        self.journal.last_seq()
    }

    pub fn open(&self, gate: &str) {
        self.shared.gate(gate).add_permits(1);
    }
}

pub fn step<'a>(run: &'a RunInfo, id: &str) -> &'a StepInfo {
    run.steps.iter().find(|step| step.step == id).unwrap_or_else(|| panic!("run has no step `{id}`"))
}

/// What the tests of handlers wait for, under the names the runner's
/// suite gives them.
pub(crate) mod handlers {
    use crate::{Rig, soon};

    /// Waits until `instance` says it is active.
    pub(crate) async fn active(rig: &Rig, instance: &str) {
        rig.event("instance.active", |event| event.source.instance.as_deref() == Some(instance)).await;
    }

    /// Waits until `count` runs of `flow` have finished.
    pub(crate) async fn handled(rig: &Rig, flow: &str, count: usize) {
        let mut follow = rig.follow("run.finished");
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
}

/// What is said on some topics after a mark, kept or passed on: there is
/// one journal here, so an event is after the mark where its number is
/// the greater.
pub struct Followed {
    follow: Following,
    after: u64,
}

impl Followed {
    pub async fn next(&mut self) -> kernel::Result<Arc<Event>> {
        loop {
            let event = self.follow.next().await?;
            if event.seq > self.after || !event.is_durable() {
                return Ok(event);
            }
        }
    }
}
