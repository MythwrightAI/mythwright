//! One test binary for the crate; add modules here.
//!
//! Nothing in this suite waits on real time. Steps that "take five minutes"
//! park on a [`ManualClock`] that the test advances, and steps that need to
//! be caught mid-flight park on a named gate that the test opens.

mod addressed;
mod alone;
mod amiss;
mod answered;
mod anywhere;
mod apart;
mod as_taken;
mod asking;
mod authority;
mod begun;
mod below;
mod beside;
mod brought;
mod called_off;
mod carried;
mod chained;
mod chains;
mod compiled;
mod compose;
mod costs;
mod counted;
mod current;
mod cut_first;
mod decides;
mod deepest;
mod elsewhere;
mod endings;
mod failure;
mod flow;
mod forked;
mod handlers;
mod heard;
mod held;
mod in_turn;
mod judged;
mod labels;
mod left;
mod lent;
mod letgo;
mod lifecycle;
mod linked;
mod loops;
mod meets;
mod ordered;
mod owed;
mod passed;
mod pinned;
mod plain;
mod plainly;
mod plays;
mod profile;
mod ran_out;
mod reached;
mod refused;
mod rested;
mod rule;
mod said;
mod settled;
mod several;
mod started;
mod state;
mod stepped;
mod stops;
mod taken;
mod taken_whole;
mod taking;
mod task;
mod time;
mod told;
mod turns;
mod unbegun;
mod under;
mod under_way;
mod waited;
mod ways;
mod whose;
mod written;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use blueprint::{ActionSpec, ValueType};
use conductor::{
    ActionCx, ActionError, ActionResult, Applied, Apply, Engine, InstanceInfo, MemoryLibrary, Registry,
    RunInfo, StepInfo,
};
use journal::{EventQuery, Journal, Logs, Order};
use kernel::{BoxFut, Event, Fields, ManualClock, Principal, PrincipalKind};
use serde_json::{Value, json};
use tokio::sync::Semaphore;

/// State the test actions share with the test.
#[derive(Default)]
pub struct Shared {
    /// Every `test.*` call, as `step:action`, in the order it began.
    pub calls: Mutex<Vec<String>>,
    counters: Mutex<HashMap<String, u64>>,
    gates: Mutex<HashMap<String, Arc<Semaphore>>>,
    /// Whether the actions that come and go are here now.
    pub lent: Mutex<bool>,
    /// What stand-ins wrote down under a key, in the order they did.
    notes: Mutex<HashMap<String, Vec<String>>>,
    /// A conductor is coming up: nothing a play would let go is waited for.
    pub starting: std::sync::atomic::AtomicBool,
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

    pub fn note(&self, key: &str, what: &str) {
        self.notes.lock().unwrap().entry(key.to_string()).or_default().push(what.to_string());
    }

    pub fn noted(&self, key: &str) -> Vec<String> {
        self.notes.lock().unwrap().get(key).cloned().unwrap_or_default()
    }

    pub fn count(&self, key: &str) -> u64 {
        self.counters.lock().unwrap().get(key).copied().unwrap_or(0)
    }

    /// What was counted under each key that begins `prefix`, by the rest
    /// of the key.
    pub fn counted(&self, prefix: &str) -> Vec<(String, u64)> {
        let counters = self.counters.lock().unwrap();
        counters
            .iter()
            .filter_map(|(key, times)| Some((key.strip_prefix(prefix)?.to_string(), *times)))
            .collect()
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

/// Takes the runner's tables back to what they were before a row said
/// whose it is and a place which journal it is in: the last of what was
/// done to them, undone, for a test that plays a journal as a program
/// from before left it and takes off what came after that program
/// itself. What was kept of what is alive and under way goes, a
/// handler's place is one number again, and the two indexes of status
/// are there as they were.
pub const BEFORE_ROWS_WERE_APART: &str = "
    DROP TRIGGER alive_begins;
    DROP TRIGGER alive_ends;
    DROP TRIGGER alive_again;
    DROP TRIGGER alive_gone;
    DROP TRIGGER under_way_begins;
    DROP TRIGGER under_way_ends;
    DROP TRIGGER under_way_again;
    DROP TRIGGER under_way_gone;
    DROP TRIGGER instances_of_one_project;
    DROP TRIGGER runs_say_whose;
    DROP TRIGGER places_say_whose;
    DROP TRIGGER instances_project_stays;
    DROP TRIGGER runs_project_stays;
    DROP TRIGGER alive_stays;
    DROP TRIGGER under_way_stays;
    DROP TRIGGER places_project_stays;
    DROP TABLE alive;
    DROP TABLE under_way;
    CREATE TABLE trigger_cursors (
        instance TEXT NOT NULL,
        trigger TEXT NOT NULL,
        seq INTEGER NOT NULL,
        PRIMARY KEY (instance, trigger)
    ) WITHOUT ROWID;
    INSERT INTO trigger_cursors (instance, trigger, seq) SELECT instance, handler, seq FROM places;
    DROP TABLE places;
    ALTER TABLE runs DROP COLUMN project;
    CREATE INDEX instances_status ON instances (status);
    CREATE INDEX runs_status ON runs (status);
    UPDATE schema_versions SET version = version - 1 WHERE namespace = 'conductor';
";

/// Undoes the step of the runner's tables after that one, which notes
/// in a journal that had something alive that what is kept beside its
/// rows may still be in the factory's: the note goes, and the tables
/// are a version back. Said before [`BEFORE_ROWS_WERE_APART`] by what
/// plays a journal from further back, and after what undoes the step
/// after it (`ordered::BEFORE_STEPS_SAID_THEIR_PLACE`).
pub const BEFORE_KEPT_BESIDE: &str = "
    DELETE FROM kv WHERE namespace = 'beside';
    UPDATE schema_versions SET version = version - 1 WHERE namespace = 'conductor';
";

/// How many instances were in the middle of what they do as they stop,
/// in a database a conductor is about to start on: one of which only
/// that its stop had begun is kept had done none of it.
pub const STOPPING: &str =
    "SELECT count(*) FROM instances WHERE owes = 'stop' AND json_extract(hooks, '$.end') IS NOT NULL";

/// The project a play is applied for where the suite is played with a
/// journal for each project.
pub const PROJECT: &str = "yard";

pub use ways::{one_journal, own_journals};

pub struct Rig {
    /// How many instances were in the middle of being stopped, with
    /// something done as they stop, when the conductor was last stopped
    /// where it was and started ([`Rig::stopped_there_told`]): what each
    /// was doing then is done in no run, and nothing said tells of it.
    pub stopping: usize,
    pub engine: Engine,
    pub clock: ManualClock,
    pub journal: Journal,
    pub library: Arc<MemoryLibrary>,
    pub shared: Arc<Shared>,
    logs: Logs,
    dir: std::path::PathBuf,
    /// The order everything was written in, whichever journal keeps it:
    /// what the rig hands over as an event's `seq` ([`Rig::event`]).
    order: Arc<Order>,
    /// The project what the rig applies is of ([`Rig::applies_for`]);
    /// nobody's, unless it was told.
    project: Option<String>,
    /// A start that the journal shut in the middle of, and that waits for
    /// ever on what it would have said next.
    starting: Option<tokio::task::JoinHandle<()>>,
}

/// Each rig writes step logs to its own directory; take it away afterwards.
impl Drop for Rig {
    fn drop(&mut self) {
        if let Some(starting) = self.starting.take() {
            starting.abort();
        }
        self.engine.halt();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn action<F, Fut>(registry: &mut Registry, spec: ActionSpec, shared: &Arc<Shared>, run: F)
where
    F: Fn(ActionCx, Fields, Arc<Shared>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ActionResult> + Send + 'static,
{
    let shared = shared.clone();
    let name = spec.name.clone();
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        // By the step and the action, and by which entry of what is
        // done around the step it is: one entry passed over and another
        // done twice are not the same as each done once.
        let entry = cx.around().map_or(String::new(), |entry| format!(" ({entry})"));
        shared.calls.lock().unwrap().push(format!("{}:{name}{entry}", cx.step()));
        Box::pin(run(cx, args, shared.clone()))
    });
}

fn registry(shared: &Arc<Shared>) -> Registry {
    use ValueType::*;
    let mut registry = Registry::new();
    action(
        &mut registry,
        ActionSpec::new("test.echo", "Return what it was given.").open().idempotent(),
        shared,
        |_, args, _| async move { Ok(Value::Object(args)) },
    );
    action(
        &mut registry,
        ActionSpec::new("test.decides", "Give back what it was handed, and do nothing else.")
            .open()
            .deciding(),
        shared,
        |_, args, _| async move { Ok(Value::Object(args)) },
    );
    action(
        &mut registry,
        ActionSpec::new("test.flaky", "Fail the first `times` calls, then succeed.")
            .arg("times", Int, "how many failures")
            .arg("message", String, "the error"),
        shared,
        |cx, args, shared| async move {
            let calls = shared.bump(&format!("flaky:{}", cx.step()));
            let times = args.get("times").and_then(Value::as_u64).unwrap_or(u64::MAX);
            if calls <= times {
                let message = args.get("message").and_then(Value::as_str).unwrap_or("boom");
                return Err(ActionError::new(message).with_data(json!({ "call": calls })));
            }
            Ok(json!({ "calls": calls }))
        },
    );
    action(
        &mut registry,
        ActionSpec::new("test.sleep", "Take this long.").required("for", Duration, "how long").idempotent(),
        shared,
        |cx, args, _| async move {
            let millis = blueprint::duration::parse(args["for"].as_str().unwrap()).unwrap();
            cx.sleep(std::time::Duration::from_millis(millis)).await?;
            Ok(json!({ "slept_ms": millis }))
        },
    );
    action(
        &mut registry,
        ActionSpec::new("test.gate", "Wait until the test opens the named gate.")
            .required("name", String, "which gate")
            .arg("lines", List, "lines to log first"),
        shared,
        |cx, args, shared| async move {
            for line in args.get("lines").and_then(Value::as_array).into_iter().flatten() {
                cx.log('o', line.as_str().unwrap_or_default());
            }
            let gate = shared.gate(args["name"].as_str().unwrap());
            cx.emit("test.reached", json!({ "gate": args["name"] })).await?;
            tokio::select! {
                permit = gate.acquire() => permit.unwrap().forget(),
                () = cx.cancel().cancelled() => return Err(ActionError::cancelled()),
            }
            Ok(json!({ "passed": args["name"] }))
        },
    );
    action(
        &mut registry,
        ActionSpec::new("test.count", "Count calls under a key.")
            .required("key", String, "the counter")
            .idempotent(),
        shared,
        |_, args, shared| async move { Ok(json!({ "count": shared.bump(args["key"].as_str().unwrap()) })) },
    );
    action(
        &mut registry,
        ActionSpec::new("test.decide", "Answer with a decision, like an agent would.").required(
            "decision",
            String,
            "the answer",
        ),
        shared,
        |_, args, _| async move { Ok(json!({ "decision": args["decision"] })) },
    );
    action(
        &mut registry,
        ActionSpec::new("test.shell", "Needs the `shell` capability.").capability("shell").idempotent(),
        shared,
        |cx, _, _| async move { Ok(json!({ "ran_as": cx.principal().name })) },
    );
    action(
        &mut registry,
        ActionSpec::new("test.hold", "Hold something that outlives the run.")
            .required("what", String, "its name"),
        shared,
        |cx, args, _| async move {
            cx.hold(args["what"].as_str().unwrap()).await?;
            Ok(Value::Null)
        },
    );
    action(
        &mut registry,
        ActionSpec::new("test.mark", "Be counted under a key, and give nothing that says how often.")
            .required("key", String, "the counter")
            .idempotent(),
        shared,
        |_, args, shared| async move {
            shared.bump(args["key"].as_str().unwrap());
            Ok(json!({ "marked": args["key"] }))
        },
    );
    action(
        &mut registry,
        ActionSpec::new(
            "test.told",
            "Be counted under a key; under `again:<key>` when told it is being done again, and under \
             `around:<key>` when it is done around a step and not as its action.",
        )
        .required("key", String, "the counter")
        .idempotent(),
        shared,
        |cx, args, shared| async move {
            let key = args["key"].as_str().unwrap();
            shared.bump(key);
            if cx.again() {
                shared.bump(&format!("again:{key}"));
            }
            if cx.around().is_some() {
                shared.bump(&format!("around:{key}"));
            }
            Ok(json!({ "again": cx.again() }))
        },
    );
    left::register(&mut registry, shared);
    action(
        &mut registry,
        ActionSpec::new(
            "test.which",
            "Be written down as which of the things done under its step's name it is, under a key.",
        )
        .required("key", String, "what to write it under")
        .idempotent(),
        shared,
        |cx, args, shared| async move {
            let which = format!("{}: {}", cx.step(), cx.doing());
            shared.note(args["key"].as_str().unwrap(), &which);
            Ok(json!({}))
        },
    );
    action(
        &mut registry,
        ActionSpec::new("test.fails", "Fail, however often it is asked: safe to ask again.")
            .arg("message", String, "the error")
            .idempotent(),
        shared,
        |_, args, _| async move {
            Err(ActionError::new(args.get("message").and_then(Value::as_str).unwrap_or("no")))
        },
    );
    action(
        &mut registry,
        ActionSpec::new(
            "test.until",
            "Fail until it has been asked `times` times under a key: safe to ask again.",
        )
        .required("key", String, "the counter")
        .required("times", Int, "how many askings it takes")
        .idempotent(),
        shared,
        |_, args, shared| async move {
            let asked = shared.bump(&format!("until:{}", args["key"].as_str().unwrap()));
            if asked < args["times"].as_u64().unwrap_or(1) {
                return Err(ActionError::new("not yet"));
            }
            Ok(json!({ "there": true }))
        },
    );
    action(
        &mut registry,
        ActionSpec::new("test.speak", "Print lines, then take a while: safe to ask again.")
            .arg("lines", List, "what to print")
            .arg("for", Duration, "how long to take")
            .idempotent(),
        shared,
        |cx, args, _| async move {
            for line in args.get("lines").and_then(Value::as_array).into_iter().flatten() {
                cx.log('o', line.as_str().unwrap_or_default());
            }
            if let Some(long) = args.get("for").and_then(Value::as_str) {
                let millis = blueprint::duration::parse(long).unwrap();
                cx.sleep(std::time::Duration::from_millis(millis)).await?;
            }
            Ok(json!({ "spoke": true }))
        },
    );
    action(&mut registry, ActionSpec::new("test.panic", "Crash."), shared, |_, _, _| async move {
        panic!("this action has a bug");
    });
    registry.add(Durable {
        shared: shared.clone(),
        spec: ActionSpec::new("test.durable", "Checkpoints, then waits at a gate.").required(
            "name",
            String,
            "which gate",
        ),
    });
    registry.also(Arc::new(lent::Lent(shared.clone())));
    meets::stand_ins(&mut registry, shared);
    handlers::stand_ins(&mut registry, shared);
    registry
}

/// An action that can be picked up after a restart.
struct Durable {
    shared: Arc<Shared>,
    spec: ActionSpec,
}

impl Durable {
    async fn finish(shared: Arc<Shared>, cx: ActionCx, args: Fields, resumed_from: Value) -> ActionResult {
        let gate = shared.gate(args["name"].as_str().unwrap());
        tokio::select! {
            permit = gate.acquire() => permit.unwrap().forget(),
            () = cx.cancel().cancelled() => return Err(ActionError::cancelled()),
        }
        Ok(json!({ "resumed_from": resumed_from }))
    }
}

impl conductor::Action for Durable {
    fn spec(&self) -> &ActionSpec {
        &self.spec
    }

    fn run(&self, cx: ActionCx, args: Fields) -> BoxFut<'static, ActionResult> {
        let shared = self.shared.clone();
        Box::pin(async move {
            shared.bump("durable:run");
            cx.checkpoint(json!({ "pid": 4242 })).await?;
            cx.emit("test.reached", json!({ "gate": args["name"] })).await?;
            Self::finish(shared, cx, args, Value::Null).await
        })
    }

    fn resume(&self, cx: ActionCx, args: Fields, saved: Value) -> BoxFut<'static, ActionResult> {
        let shared = self.shared.clone();
        Box::pin(async move {
            shared.bump("durable:resume");
            // It waits at its gate again, and says nothing of it: whoever
            // lets stand-ins go is told here.
            shared.bump(&format!("taken-up:{}", args["name"].as_str().unwrap()));
            Self::finish(shared, cx, args, saved).await
        })
    }
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

/// How many turns in a row a start may ask nothing of any database and
/// not be over, before it is taken to wait for what cannot come.
pub const STALLED: usize = 400;

/// How long something is waited for before the test fails for it.
const PATIENCE: Duration = Duration::from_secs(20);

/// How long a database's file is waited for once it was asked to close:
/// its thread brings its log back and holds the file to the disk, which
/// takes seconds on a machine with much else to write.
const PUT_AWAY: Duration = Duration::from_secs(60);

/// Fails the test instead of hanging it when something never happens.
/// Where there is a rig to ask, [`Rig::soon`] says what did happen.
pub async fn soon<T>(what: &str, work: impl Future<Output = T>) -> T {
    match tokio::time::timeout(PATIENCE, work).await {
        Ok(value) => value,
        Err(_) => panic!("timed out waiting for: {what}"),
    }
}

impl Rig {
    pub async fn new() -> Rig {
        Rig::made(false).await
    }

    /// A rig whose journal is a file, so that it can be shut at a place
    /// and opened again, as the journal of a conductor stopped there is.
    pub async fn on_a_file() -> Rig {
        Rig::made(true).await
    }

    /// A rig on the journal it is to have and no other first: a play
    /// makes a rig for each of its hundreds of plays.
    async fn made(on_a_file: bool) -> Rig {
        let clock = ManualClock::new();
        let order = Arc::new(Order::default());
        let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("conductor-{}", kernel::id::secret()));
        let journal = match on_a_file {
            true => Rig::opened(&dir, &clock, &order).await,
            false => Journal::memory(Arc::new(clock.clone())).await.unwrap(),
        };
        journal.orders_by(order.clone());
        let logs = Logs::new(dir.clone());
        let shared = Arc::new(Shared::default());
        let library = Arc::new(MemoryLibrary::new());
        let engine =
            Engine::new(journal.clone(), logs.clone(), registry(&shared), library.clone()).await.unwrap();
        let (stopping, project, starting) = (0, None, None);
        Rig { stopping, engine, clock, journal, library, shared, logs, dir, order, project, starting }
    }

    /// The journal in a rig's file, opened, with the journals of its
    /// projects that are kept in files beside it: each a database of its
    /// own, as a factory's are.
    async fn opened(dir: &std::path::Path, clock: &ManualClock, order: &Arc<Order>) -> Journal {
        // A conductor is stopped here by its journal being shut, never
        // by the power going: nothing waits for the disk, which was
        // most of what a play took (`journal::disk`).
        let db = journal::Db::open_unwaited(&dir.join("journal.db")).unwrap();
        let journal = Journal::open(db, Arc::new(clock.clone()), "test-device").await.unwrap();
        journal.orders_by(order.clone());
        journal.keeps_projects_in(dir.join("journals")).await.unwrap();
        journal
    }

    /// Has the journal of a rig in memory keep its projects' beside it,
    /// in memory: for a test of a project with a journal of its own.
    pub async fn keeps_projects(&self) {
        self.journal.keeps_projects_in_memory().await.unwrap();
    }

    /// Has a rig on a file apply what it applies for `project`, which is
    /// given a journal of its own: a file beside the factory's, opened
    /// again with it each time the conductor is stopped and started.
    pub async fn applies_for(&mut self, project: &str) {
        self.journal.own(project).await.unwrap();
        self.project = Some(project.to_string());
    }

    /// Gives each project a play names for something it applies a
    /// journal of its own: that is then run by a conductor of its own,
    /// beside the factory's and the other's, and stopped with them.
    pub async fn owns_each(&self, acts: &[anywhere::Act]) {
        for act in acts {
            if let Some(project) = act.does.project() {
                self.journal.own(project).await.unwrap();
            }
        }
    }

    /// For a rig on a file whose journal shut at a place it was told:
    /// stops the conductor there, and starts a new one on what the file
    /// holds, which is what a conductor stopped at that place left.
    pub async fn stopped_there(&mut self) {
        drop(self.stopped_there_told(|_| {}).await);
    }

    /// The same, with the journal the new conductor starts on told
    /// something first: a place to shut at, so that the conductor is
    /// stopped a second time while it is coming up. A start the journal
    /// shuts in the middle of is left where it is, as the start of a
    /// daemon that went down then would be, until the rig is stopped
    /// again. What holds the databases is given back still holding, for
    /// whoever goes on a turn at a time; put away, they are let go.
    pub async fn stopped_there_told(&mut self, tell: impl FnOnce(&Journal)) -> stepped::Stepped {
        if let Some(starting) = self.starting.take() {
            starting.abort();
            let _ = starting.await;
        }
        self.engine.halt();
        self.engine.halted().await;
        // Closed when its files are free, and not when they were asked to
        // be: opened in the moment between, one is locked by the one
        // before. Every database of the factory's, each asked before any
        // is waited for: a project's whose thread waits on the factory's
        // for a run of numbers is answered by the factory's ending.
        let dbs = self.journal.dbs();
        for db in &dbs {
            db.close();
        }
        for db in &dbs {
            // One that never is says which, and what its thread is at.
            if tokio::time::timeout(PUT_AWAY, db.closed()).await.is_err() {
                panic!("timed out waiting for a database's file to be free: {}", db.standing());
            }
        }
        self.journal = Rig::opened(&self.dir, &self.clock, &self.order).await;
        tell(&self.journal);
        // Read before the conductor is there to finish them, in each
        // database that keeps what runs.
        self.stopping = 0;
        for db in self.journal.dbs() {
            let owed = db.call(|connection| connection.query_row(crate::STOPPING, [], |row| row.get(0)));
            self.stopping += owed.await.unwrap_or(0i64) as usize;
        }
        self.engine = Engine::new(
            self.journal.clone(),
            self.logs.clone(),
            registry(&self.shared),
            self.library.clone(),
        )
        .await
        .unwrap();
        // A start goes over everything alive, and what it takes up goes
        // on beside it. Which of them comes to something first is not
        // left to the machine: the database is held from before the
        // start begins, and answered a turn at a time until the start
        // is over, or the journal shut in the middle.
        let mut stepped = stepped::Stepped::holding(&self.journal);
        let (engine, shared) = (self.engine.clone(), self.shared.clone());
        shared.starting.store(true, std::sync::atomic::Ordering::SeqCst);
        let starting = tokio::spawn(async move {
            engine.resume().await.unwrap();
            shared.starting.store(false, std::sync::atomic::Ordering::SeqCst);
        });
        let mut idle = 0;
        loop {
            if starting.is_finished() {
                starting.await.unwrap();
                break;
            }
            let answered = match self.journal.is_shut() {
                true => None,
                false => stepped.turn(true).await,
            };
            let Some(answered) = answered else {
                self.starting = Some(starting);
                break;
            };
            // A start goes forward by what it asks of a database. One
            // that asks nothing more and is not over waits on something
            // that cannot come while it starts.
            idle = if answered == 0 { idle + 1 } else { 0 };
            assert!(
                idle < STALLED,
                "the conductors' start did not come to its end, and asks nothing of any database any more: it \
                 waits for something that cannot come while it starts. What a start does itself does not wait \
                 on a conductor that starts after it (`standing` in `crates/conductor/src/set.rs`)"
            );
        }
        stepped
    }

    /// Stops the conductor as a crash would, then starts a new one on the
    /// same journal and lets it pick up where the old one was.
    pub async fn restart(&mut self) {
        self.engine.halt();
        // Nothing of the old one is left to take what the test then gives
        // (a gate it opens, time it lets pass) in place of the new.
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
        // The project's, where the rig applies for one.
        let project = self.project.clone();
        Apply {
            source: Some(source.to_string()),
            params: kernel::fields(params),
            project,
            ..Apply::new(name, user())
        }
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
        self.soon("the run to finish", self.engine.wait_run(run)).await.unwrap()
    }

    /// As [`soon`], and a wait that runs out says the last things the
    /// journal held with what never came: a wait that fails once in fifty
    /// runs, on a machine nobody can look at, has nothing else to go by.
    pub async fn soon<T>(&self, what: &str, work: impl Future<Output = T>) -> T {
        self.within(PATIENCE, what, work).await
    }

    /// The same, waited for as long as is said: what holds the rig to
    /// what it says of a wait that ran out, without waiting.
    pub async fn within<T>(&self, patience: Duration, what: &str, work: impl Future<Output = T>) -> T {
        match tokio::time::timeout(patience, work).await {
            Ok(value) => value,
            Err(_) => panic!(
                "timed out waiting for: {what}\nwhat the journal held last, the latest first:\n{}",
                self.lately().await
            ),
        }
    }

    /// The last events the journal held, the latest first, a line each.
    pub async fn lately(&self) -> String {
        let query = EventQuery { limit: 60, newest_first: true, ..Default::default() };
        // A journal that has shut at a place it was told answers nothing:
        // a wait that ran out on one says so, and is not kept from failing
        // by asking what it held.
        let Ok(events) =
            tokio::time::timeout(Duration::from_secs(5), self.order.read_every(&self.journal, query)).await
        else {
            return "  (the journal answered nothing in five seconds: it has shut, or is held up)"
                .to_string();
        };
        let line = |event: &Event| {
            let mut data = event.data.to_string();
            if data.len() > 160 {
                let cut = (0..=160).rev().find(|at| data.is_char_boundary(*at)).unwrap_or(0);
                data.truncate(cut);
                data.push('…');
            }
            let step = event.source.step.as_deref().map(|step| format!(" [{step}]")).unwrap_or_default();
            format!("  {} {}{step} {data}", event.seq, event.topic)
        };
        match events.unwrap_or_default().as_slice() {
            [] => "  (nothing)".to_string(),
            events => events.iter().map(line).collect::<Vec<_>>().join("\n"),
        }
    }

    pub async fn instance(&self, id: &str) -> InstanceInfo {
        self.engine.instance(id).await.unwrap()
    }

    /// The first stored event on `topic` that satisfies `wanted`, waiting for
    /// it if it has not happened yet.
    ///
    /// **Its `seq` is where it came among everything that was written**,
    /// whichever journal keeps it, and not its number ([`journal::Order`]):
    /// so that one event is after another, or after a [`Rig::mark`], where
    /// its `seq` is the greater, though a project's journal keeps the one
    /// and the factory's the other. So are those of everything the rig
    /// hands over. The number an event has in its journal is
    /// [`Rig::number`] of that.
    pub async fn event(&self, topic: &str, wanted: impl Fn(&Event) -> bool) -> Arc<Event> {
        let mut follow = self.follow(topic);
        self.soon(&format!("an event on `{topic}`"), async {
            loop {
                let event = follow.next().await.unwrap();
                if wanted(&event) {
                    return event;
                }
            }
        })
        .await
    }

    /// Waits until `step` of any run has started.
    pub async fn started(&self, step: &str) -> Arc<Event> {
        self.event("step.started", |event| event.source.step.as_deref() == Some(step)).await
    }

    /// Waits until an action is parked at the named gate. Unlike
    /// [`Rig::started`], this means the action itself is running (and, for a
    /// durable one, has saved its checkpoint).
    pub async fn reached(&self, gate: &str) -> Arc<Event> {
        self.event("test.reached", |event| event.data["gate"] == gate).await
    }

    /// Topics of every stored event matching `topics`, in order.
    pub async fn topics(&self, topics: &str) -> Vec<String> {
        self.events(topics).await.into_iter().map(|event| event.topic).collect()
    }

    /// Every stored event matching `topics`, of every journal, in the
    /// order they were written; each with where it came as its `seq`
    /// ([`Rig::event`]).
    pub async fn events(&self, topics: &str) -> Vec<Event> {
        let query = EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        let mut events: Vec<(u64, Event)> = Vec::new();
        // What was passed on and not kept with what was kept: what was
        // said, which is what a test holds the runner to.
        for event in self.order.read_every(&self.journal, query).await.unwrap() {
            events.push((event.seq, placed(&self.order, &event)));
        }
        events.sort_by_key(|(number, event)| (event.seq, *number));
        events.into_iter().map(|(_, event)| event).collect()
    }

    /// Follows what every journal says on `topics`, from the beginning.
    pub fn follow(&self, topics: &str) -> Followed {
        self.follow_after(0, topics)
    }

    /// Follows what every journal says on `topics` that was written
    /// after `mark`: a [`Rig::mark`], or the `seq` of an event the rig
    /// handed over.
    pub fn follow_after(&self, mark: u64, topics: &str) -> Followed {
        let follow = self.order.follow_every(&self.journal, topics);
        Followed { follow, order: self.order.clone(), after: mark }
    }

    /// Where the last thing written came: everything said from here on,
    /// in whichever journal, is handed over with a greater `seq`.
    pub fn mark(&self) -> u64 {
        self.order.so_far()
    }

    /// The number an event has in its journal, given the `seq` the rig
    /// handed it over with.
    pub fn number(&self, seq: u64) -> u64 {
        self.order.number(seq).unwrap_or(seq)
    }

    /// Where the event numbered `number` came: the `seq` the rig hands
    /// it over with.
    pub fn came(&self, number: u64) -> u64 {
        self.order.of(number).unwrap_or(0)
    }

    pub fn open(&self, gate: &str) {
        self.shared.gate(gate).add_permits(1);
    }

    /// Advances the clock once the code under test has started `sleeps`
    /// more sleeps than it had at `since`.
    pub async fn advance_after(&self, since: u64, sleeps: u64, by: &str) {
        self.soon("the code under test to sleep", self.clock.sleeps_started(since + sleeps)).await;
        self.clock.advance(Duration::from_millis(blueprint::duration::parse(by).unwrap()));
    }
}

/// An event as the rig hands it over: with where it came among
/// everything written as its `seq`. One that was not kept has none, and
/// one written before any order was kept comes before everything.
fn placed(order: &Order, event: &Event) -> Event {
    let mut placed = event.clone();
    placed.seq = order.of(event.seq).unwrap_or(0);
    placed
}

/// What every journal says, followed, each event with where it came as
/// its `seq` ([`Rig::event`]).
pub struct Followed {
    follow: journal::Following,
    order: Arc<Order>,
    after: u64,
}

impl Followed {
    /// The next matching event of any journal that was written after
    /// what this follows from. Waits if there is none yet.
    pub async fn next(&mut self) -> kernel::Result<Arc<Event>> {
        loop {
            let event = self.follow.next().await?;
            let placed = placed(&self.order, &event);
            if placed.seq > self.after || !event.is_durable() {
                return Ok(Arc::new(placed));
            }
        }
    }
}

pub fn step<'a>(run: &'a RunInfo, id: &str) -> &'a StepInfo {
    run.steps.iter().find(|step| step.step == id).unwrap_or_else(|| panic!("run has no step `{id}`"))
}

/// `(step, status)` for every step of a run, in the blueprint's order.
pub fn statuses(run: &RunInfo, order: &[&str]) -> Vec<(String, &'static str)> {
    order.iter().map(|id| (id.to_string(), step(run, id).status.as_str())).collect()
}
