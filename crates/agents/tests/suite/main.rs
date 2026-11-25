//! One test binary for the crate; add modules here.

mod actions;
mod apart;
mod archived;
mod blueprints;
mod branches;
mod briefed;
mod came;
mod contended;
mod conversation;
mod council;
mod docsed;
mod door;
mod extensions;
mod external;
mod filed;
mod given;
mod homes;
mod kept;
mod keys;
mod leaving;
mod moved;
mod outlives;
mod present;
mod real;
mod removed;
mod repositories;
mod seen;
mod signins;
mod slots;
mod standin;
mod together;
mod walls;

/// The stand-in for a program that the tests here start
/// (`crates/hosting/standin`): this crate's own copy, built with this
/// suite.
pub const STANDIN: &str = env!("CARGO_BIN_EXE_agents-standin");

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agents::{Adapter, Agents, Config, Failure, Reply, SlotSpec, Turn};
use conductor::{Applied, Apply, Engine, MemoryLibrary, Registry, RunInfo};
use journal::{Db, EventQuery, Journal, Logs};
use kernel::{BoxFut, Event, ManualClock, Principal, PrincipalKind};
use serde_json::Value;
use studio::Studio;
use tokio::sync::Semaphore;

/// What an adapter was asked to do.
#[derive(Clone, Debug)]
pub struct Seen {
    pub slot: String,
    pub system: String,
    pub prompt: String,
    pub session: Option<String>,
    pub env: BTreeMap<String, String>,
    pub workdir: PathBuf,
}

/// An adapter that records what it is asked and answers from a script.
#[derive(Default)]
pub struct Fake {
    /// Programs that stand in for an agent's, each an adapter's name and what
    /// it runs: added again whenever the agents are started.
    pub programs: Mutex<Vec<(String, Vec<String>)>>,
    pub seen: Mutex<Vec<Seen>>,
    replies: Mutex<VecDeque<Result<String, String>>>,
    gates: Mutex<HashMap<String, Arc<Semaphore>>>,
    sessions: Mutex<u32>,
    /// The model the program says it used, when it says.
    pub names_its_model: Mutex<Option<String>>,
    /// That the program says nothing of what a turn cost, as a package
    /// that reports only tokens does.
    pub says_no_cost: Mutex<bool>,
    /// Calls made to the `test.flaky` action since it last succeeded.
    sessions_flaky: Mutex<u64>,
    /// How many turns have reached the adapter.
    reached: tokio::sync::watch::Sender<usize>,
}

impl Fake {
    /// Queues what the next turn will answer. With nothing queued a turn
    /// answers `done`.
    pub fn will_say(&self, reply: &str) {
        self.replies.lock().unwrap().push_back(Ok(reply.to_string()));
    }

    pub fn will_fail(&self, error: &str) {
        self.replies.lock().unwrap().push_back(Err(error.to_string()));
    }

    fn gate(&self, name: &str) -> Arc<Semaphore> {
        self.gates
            .lock()
            .unwrap()
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(0)))
            .clone()
    }

    /// Lets a turn whose prompt contains `[gate:<name>]` finish; one that
    /// says `[gate:slot]`, the turn of the agent called `name`.
    pub fn open(&self, name: &str) {
        self.gate(name).add_permits(1);
    }

    /// Waits until `count` turns have reached the adapter.
    pub async fn saw(&self, count: usize) {
        let mut reached = self.reached.subscribe();
        soon(&format!("{count} turns to reach the adapter"), reached.wait_for(|seen| *seen >= count))
            .await
            .unwrap();
    }

    pub fn prompts(&self) -> Vec<String> {
        self.seen.lock().unwrap().iter().map(|seen| seen.prompt.clone()).collect()
    }
}

/// The fake as the agents service holds it; the test keeps the other handle.
struct FakeAdapter(Arc<Fake>);

impl Adapter for FakeAdapter {
    fn run(&self, turn: Turn) -> BoxFut<'static, Result<Reply, Failure>> {
        let fake = self.0.clone();
        Box::pin(async move {
            fake.seen.lock().unwrap().push(Seen {
                slot: turn.slot.clone(),
                system: turn.system.clone(),
                prompt: turn.prompt.clone(),
                session: turn.session.clone(),
                env: turn.env.clone(),
                workdir: turn.workdir.clone(),
            });
            fake.reached.send_modify(|seen| *seen += 1);
            // Only the newest message can hold a turn; the same marker in
            // the history it is shown does not.
            let newest =
                turn.prompt.rsplit_once("is for you:\n").map_or(turn.prompt.as_str(), |(_, body)| body);
            if let Some(name) = newest.split("[gate:").nth(1).and_then(|rest| rest.split(']').next()) {
                // `[gate:slot]` holds each agent at a gate of its own name:
                // several asked the same at once are let go one by one.
                let gate = fake.gate(if name == "slot" { &turn.slot } else { name });
                tokio::select! {
                    permit = gate.acquire() => permit.unwrap().forget(),
                    () = turn.cancel.cancelled() => return Err("the turn was cancelled".into()),
                }
            }
            let scripted = fake.replies.lock().unwrap().pop_front();
            let text = scripted.unwrap_or_else(|| Ok("done".to_string()))?;
            (turn.progress)(&text);
            // A new conversation gets a new session; a continued one keeps it.
            let session = turn.session.unwrap_or_else(|| {
                let mut sessions = fake.sessions.lock().unwrap();
                *sessions += 1;
                format!("session-{sessions}")
            });
            let model = fake.names_its_model.lock().unwrap().clone();
            Ok(Reply {
                text,
                session: Some(session),
                model,
                cost_usd: (!*fake.says_no_cost.lock().unwrap()).then_some(0.01),
                input_tokens: 100,
                output_tokens: 20,
                refused: Vec::new(),
            })
        })
    }
}

/// Writes `text` as a program at `path`, by a process of its own.
///
/// A program whose file anybody has open to write is refused when it is
/// started (the system says the text file is busy). Written by the test
/// itself, the file is open in this process for that moment, and a
/// program any other test starts in the moment is this process again
/// until it has become itself, holding what this one had open: the
/// program written here was then refused, now and then, by whichever
/// test started something as it was written. Written by a process that
/// starts nothing, nobody else ever holds it.
#[cfg(unix)]
pub fn program(path: &std::path::Path, text: &str) {
    use std::io::Write;
    let mut writes = std::process::Command::new("sh")
        .args(["-c", "cat > \"$0\" && chmod 755 \"$0\""])
        .arg(path)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    writes.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    assert!(writes.wait().unwrap().success(), "could not write the program {}", path.display());
}

pub struct Rig {
    pub agents: Agents,
    pub studio: Studio,
    pub engine: Engine,
    pub journal: Journal,
    /// The order everything was written in, whichever journal keeps it.
    order: Arc<journal::Order>,
    pub clock: ManualClock,
    pub fake: Arc<Fake>,
    pub dir: PathBuf,
    /// Blueprints a test puts where they are found before the ones that ship.
    pub library: Arc<MemoryLibrary>,
    logs: Logs,
    pub hosting: hosting::Hosting,
    walls: hosting::Walls,
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.engine.halt();
        self.agents.halt();
        self.hosting.halt();
        // A test ends what it started, however it ended: what its
        // factory hosts is left running by the factory, on purpose.
        for pid in hosting::end_under(&self.dir) {
            eprintln!(
                "the test was over with a program its factory started still running; its group {pid} was ended"
            );
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
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

pub async fn soon<T>(what: &str, work: impl Future<Output = T>) -> T {
    match tokio::time::timeout(Duration::from_secs(30), work).await {
        Ok(value) => value,
        Err(_) => panic!("timed out waiting for: {what}"),
    }
}

/// The same, for a wait whose giving up says too little by itself: it
/// says `was` as well, which is looked at only then.
pub async fn soon_or<T>(what: &str, work: impl Future<Output = T>, was: impl Future<Output = String>) -> T {
    match tokio::time::timeout(Duration::from_secs(30), work).await {
        Ok(value) => value,
        Err(_) => panic!("timed out waiting for: {what}; {}", was.await),
    }
}

/// Whether a process with this number is running. One that has ended and
/// has not been asked how by whatever started it (a zombie) is not.
#[cfg(target_os = "linux")]
pub fn alive(pid: &str) -> bool {
    match std::fs::read_to_string(std::path::Path::new("/proc").join(pid.trim()).join("stat")) {
        Ok(stat) => !stat.rsplit_once(')').is_some_and(|(_, rest)| rest.trim_start().starts_with(['Z', 'X'])),
        Err(_) => false,
    }
}

/// The same where the system keeps no such file: asked of the system as
/// the factory asks it.
#[cfg(not(target_os = "linux"))]
pub fn alive(pid: &str) -> bool {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System};
    let (Ok(mut system), Ok(pid)) = (System::new(), pid.trim().parse::<u32>()) else { return false };
    let pid = Pid::from_u32(pid);
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing());
    system
        .process(pid)
        .is_some_and(|process| !matches!(process.status(), ProcessStatus::Zombie | ProcessStatus::Dead))
}

/// Waits, without sleeping, until none of these processes is running.
///
/// One look is not enough, even after whatever stopped them has said so.
/// What is written down for a program is its wrapper, which ends a moment
/// after its program has; and a program taken up after a restart is said
/// to be over by the code its wrapper wrote, not by the wrapper being
/// gone. Here the daemon that started it is this test's own process, so
/// until that asks, the wrapper is still listed by the system.
pub async fn gone(what: &str, pids: &[&str]) {
    soon(what, async {
        while pids.iter().any(|pid| alive(pid)) {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

async fn build(
    (journal, logs, studio): (&Journal, &Logs, &Studio),
    dir: &std::path::Path,
    fake: &Arc<Fake>,
    library: &Arc<MemoryLibrary>,
    walls: &hosting::Walls,
    started: bool,
) -> (Agents, Engine, hosting::Hosting) {
    let config = Config {
        api_url: "http://127.0.0.1:7700".into(),
        home: dir.join("slots"),
        bin: dir.join("bin"),
        default_adapter: Some("fake".into()),
        extensions: dir.join("extensions"),
        // Where a test puts a package that came with the product.
        offered: Some(dir.join("offered")),
        signins: dir.join("signins"),
        seal: link::own::Own::open(dir).unwrap().at_rest(),
        data: dir.to_path_buf(),
        // Never the home of whoever runs the tests.
        user_home: Some(dir.join("home")),
    };
    // What a program started here is told about where it runs.
    let hosting = hosting::Hosting::new(journal.clone(), logs.clone(), dir.join("run")).await.unwrap();
    hosting.stands_between();
    hosting.answers_at("http://127.0.0.1:7700");
    hosting.holds_walls(walls.clone());
    hosting.keeps_secrets_in(dir.join("secrets"), link::own::Own::open(dir).unwrap().at_rest());
    let agents = Agents::open(journal.clone(), studio.clone(), hosting.clone(), config).await.unwrap();
    agents.add_adapter("fake", Arc::new(FakeAdapter(fake.clone())));
    for (name, argv) in fake.programs.lock().unwrap().iter() {
        agents.add_adapter(name, Arc::new(agents::adapter::Command { argv: argv.clone() }));
    }
    let mut registry = Registry::new();
    studio.register(&mut registry);
    agents.register(&mut registry);
    blocks::words::register(&mut registry);
    registry.also(agents.actions(&hosting));
    // A step that fails its first `times` runs, to give failure hooks something to do.
    let failures = fake.clone();
    registry.add_fn(
        blueprint::ActionSpec::new("test.flaky", "Fail the first `times` calls.").arg(
            "times",
            blueprint::ValueType::Int,
            "failures",
        ),
        move |_, args| -> BoxFut<'static, conductor::ActionResult> {
            let failures = failures.clone();
            Box::pin(async move {
                let mut calls = failures.sessions_flaky.lock().unwrap();
                *calls += 1;
                if *calls <= args.get("times").and_then(Value::as_u64).unwrap_or(1) {
                    return Err(conductor::ActionError::new("not yet"));
                }
                *calls = 0;
                Ok(Value::Null)
            })
        },
    );
    // What a test puts there, and then what ships.
    let library = conductor::Layers::new().then(library.clone()).then(Arc::new(conductor::standard()));
    let engine = Engine::new(journal.clone(), logs.clone(), registry, Arc::new(library)).await.unwrap();
    studio.attach(&engine);
    agents.attach(&engine);
    if started {
        agents.start();
    }
    (agents, engine, hosting)
}

impl Rig {
    /// A factory that holds no walls: what a test starts as an agent's
    /// program sees the device, as a test's stand-in needs to. The tests
    /// of walls say how theirs are held (`Rig::holding`).
    pub async fn new() -> Rig {
        Rig::holding(hosting::Walls::Open).await
    }

    pub async fn holding(walls: hosting::Walls) -> Rig {
        let clock = ManualClock::new();
        let journal = Journal::memory(Arc::new(clock.clone())).await.unwrap();
        // A project that is added has a journal of its own, as a
        // daemon's has: what its agents are asked is kept there. Told
        // `PROJECT_JOURNALS=one`, the suite is played as a factory from
        // before keeps it, all in the factory's own.
        // And the order things were written in is kept, across them:
        // an event's number says nothing of that between two journals.
        let order = Arc::new(journal::Order::default());
        journal.orders_by(order.clone());
        journal.keeps_projects_in_memory().await.unwrap();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("agents-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        let logs = Logs::new(dir.join("logs"));
        let studio = Studio::open(
            journal.clone(),
            Db::memory().unwrap(),
            dir.join("projects"),
            dir.join("attachments"),
        )
        .await
        .unwrap();
        let fake = Arc::new(Fake::default());
        let library = Arc::new(MemoryLibrary::new());
        let (agents, engine, hosting) =
            build((&journal, &logs, &studio), &dir, &fake, &library, &walls, true).await;
        Rig { agents, studio, engine, journal, order, clock, fake, dir, library, logs, hosting, walls }
    }

    /// Stops the agents and the engine as a crash would, and starts new ones.
    pub async fn restart(&mut self) {
        self.stop();
        self.start().await;
    }

    /// The first half of that: nothing is there to hear what goes on.
    pub fn stop(&self) {
        self.engine.halt();
        self.agents.halt();
        self.hosting.halt();
    }

    /// New ones on what the old ones kept, which are opened and take
    /// nothing up: no turn is begun, gone on with or ended. For what is
    /// so of a daemon that has come back before it does anything.
    pub async fn open_again(&mut self) {
        (self.agents, self.engine, self.hosting) = self.build(false).await;
    }

    async fn build(&self, started: bool) -> (Agents, Engine, hosting::Hosting) {
        let kept = (&self.journal, &self.logs, &self.studio);
        build(kept, &self.dir, &self.fake, &self.library, &self.walls, started).await
    }

    /// And the second: new ones, on what the old ones kept.
    pub async fn start(&mut self) {
        let (agents, engine, hosting) = self.build(true).await;
        engine.resume().await.unwrap();
        self.agents = agents;
        self.engine = engine;
        self.hosting = hosting;
    }

    /// A project, a channel for it, and an agent called `name` in both.
    pub async fn agent(&self, name: &str, adapter: &str) {
        if self.studio.projects.dir_of("shop").is_none() {
            self.studio.projects.create("Shop", None, &ada()).await.unwrap();
            // With a board, as a project has once `std/board` is applied to it.
            let columns = studio::board::DEFAULT_COLUMNS.iter().map(|column| column.to_string()).collect();
            self.studio.board.make("shop", columns, studio::Store::Local, None, &ada()).await.unwrap();
            self.studio.channels.create("shop", Some("shop"), "", &ada()).await.unwrap();
        }
        let spec = SlotSpec {
            name: name.into(),
            adapter: Some(adapter.into()),
            project: Some("shop".into()),
            channel: Some("shop".into()),
            ..SlotSpec::default()
        };
        self.agents.ensure(spec, &ada()).await.unwrap();
    }

    pub async fn say(&self, body: &str) -> studio::Message {
        self.studio.channels.post("shop", &ada(), body, None).await.unwrap()
    }

    pub async fn event(&self, topic: &str, wanted: impl Fn(&Event) -> bool) -> Arc<Event> {
        // What was said, kept or passed on.
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

    /// Waits until `count` turns have finished.
    pub async fn turns_finished(&self, count: usize) -> Vec<Event> {
        let mut follow = self.journal.follow_every(journal::Places::default(), "agent.turn.finished");
        let mut seen = Vec::new();
        soon(&format!("{count} turns to finish"), async {
            while seen.len() < count {
                seen.push((*follow.next().await.unwrap()).clone());
            }
        })
        .await;
        seen
    }

    /// What was said on `topics`, in every journal, in the order it was
    /// written.
    pub async fn events(&self, topics: &str) -> Vec<Event> {
        let query = EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        let mut said = self.order.read_every(&self.journal, query).await.unwrap();
        said.sort_by_key(|event| self.order.of(event.seq).unwrap_or(0));
        said
    }

    /// Follows `topic` in every journal from what is said from now on.
    pub fn from_now(&self, topic: &str) -> journal::FollowEvery {
        self.journal.follow_every(self.journal.now(), topic)
    }

    /// Asks every database that keeps turns the same thing, and gives
    /// what each answered: the factory's own first, then each project's
    /// that keeps its agents' turns.
    pub async fn of_turns<T: Send + 'static>(
        &self,
        ask: impl Fn(&mut rusqlite::Connection) -> rusqlite::Result<T> + Clone + Send + 'static,
    ) -> Vec<T> {
        let mut answers = Vec::new();
        for journal in self.journal.every_keeping(journal::keeps::TURNS) {
            answers.push(journal.db().call(ask.clone()).await.unwrap());
        }
        answers
    }

    /// The journal that keeps the turns of `project`'s agents: its own,
    /// or the factory's where the suite is played with one journal.
    pub fn turns_of(&self, project: &str) -> Journal {
        self.journal.kept_at(Some(project), journal::keeps::TURNS)
    }

    /// The channel's messages as `author: body`.
    pub async fn transcript(&self) -> Vec<String> {
        let messages = self.studio.channels.messages(&crate::own(), "shop", None, 100).await.unwrap();
        messages.into_iter().map(|message| format!("{}: {}", message.author_name, message.body)).collect()
    }

    pub async fn apply(&self, source: &str, params: Value) -> Applied {
        let name =
            blueprint::check(source, &blueprint::NoCatalog).blueprint.map(|b| b.name).unwrap_or_default();
        let request = Apply {
            source: Some(source.to_string()),
            params: kernel::fields(params),
            project: Some("shop".into()),
            ..Apply::new(name, ada())
        };
        self.engine.apply(request).await.unwrap_or_else(|err| panic!("apply failed: {}", err.message))
    }

    pub async fn run(&self, source: &str, params: Value) -> RunInfo {
        let applied = self.apply(source, params).await;
        soon("the run to finish", self.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap()
    }
}
