//! One test binary for the crate; add modules here.
//!
//! Two devices in one process, each with a journal, an engine and a gateway
//! of its own, joined by a link the test can cut. The link over HTTP is
//! exercised by the daemon's own tests, with two real daemons.

mod held;
mod invited;
mod link;
mod outage;
mod placing;
mod sealed;
mod standin;

/// The stand-in for a program that the tests here start
/// (`crates/hosting/standin`): this crate's own copy, built with this
/// suite.
pub const STANDIN: &str = env!("CARGO_BIN_EXE_gateway-standin");

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use conductor::{
    ActionError, ActionResult, Applied, Apply, Engine, MemoryLibrary, Registry, RunInfo, StepInfo,
};
use gateway::wire::{HELLO_PATH, REPORT_PATH};
use gateway::{Carrier, Gateway, LinkError, Shut};
use hosting::Hosting;
use journal::{Db, Journal, Logs};
use kernel::{BoxFut, ManualClock, Principal, PrincipalKind};
use serde_json::{Value, json};
use studio::Studio;
use tokio::sync::{Notify, Semaphore, mpsc, watch};

pub fn ada() -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: "ada".into(),
        name: "Ada".into(),
        scopes: vec!["*".into()],
        key: None,
    }
}

pub async fn soon<T>(what: &str, work: impl Future<Output = T>) -> T {
    match tokio::time::timeout(Duration::from_secs(30), work).await {
        Ok(value) => value,
        Err(_) => panic!("timed out waiting for: {what}"),
    }
}

/// What the test actions on both devices share with the test.
#[derive(Default)]
pub struct Shared {
    gates: Mutex<HashMap<String, Arc<Semaphore>>>,
    /// `<gate>@<device>` for every `test.gate` that has started.
    started: watch::Sender<Vec<String>>,
    pub cancelled: Mutex<Vec<String>>,
    /// What every device here reaches a hub by.
    pub wire: std::sync::OnceLock<Arc<Wire>>,
}

impl Shared {
    fn gate(&self, name: &str) -> Arc<Semaphore> {
        self.gates.lock().unwrap().entry(name.into()).or_insert_with(|| Arc::new(Semaphore::new(0))).clone()
    }

    /// Lets one `test.gate` called `name` finish.
    pub fn open(&self, name: &str) {
        self.gate(name).add_permits(1);
    }

    /// Where `test.gate` called `name` has started, in order.
    pub fn starts(&self, name: &str) -> Vec<String> {
        let prefix = format!("{name}@");
        self.started
            .borrow()
            .iter()
            .filter_map(|entry| entry.strip_prefix(&prefix).map(str::to_string))
            .collect()
    }

    /// Waits until `test.gate` called `name` has started `count` times.
    pub async fn reached(&self, name: &str, count: usize) {
        let prefix = format!("{name}@");
        let mut started = self.started.subscribe();
        soon(
            &format!("`{name}` to start {count} time(s)"),
            started.wait_for(|all| all.iter().filter(|entry| entry.starts_with(&prefix)).count() >= count),
        )
        .await
        .unwrap();
    }
}

fn test_actions(registry: &mut Registry, shared: &Arc<Shared>) {
    use blueprint::{ActionSpec, ValueType as T};
    registry.add_fn(
        ActionSpec::new("test.where", "Say which device this ran on.").arg("say", T::String, "a line to log"),
        |cx, args| -> BoxFut<'static, ActionResult> {
            Box::pin(async move {
                let device = cx.engine().journal().device().to_string();
                cx.log('o', &format!("hello from {device}"));
                if let Some(say) = args.get("say").and_then(Value::as_str) {
                    cx.log('e', say);
                }
                Ok(json!({ "device": device, "by": cx.principal().actor(), "attempt": cx.attempt() }))
            })
        },
    );
    let gates = shared.clone();
    registry.add_fn(
        ActionSpec::new("test.gate", "Wait until the test lets it finish.").required(
            "name",
            T::String,
            "which gate",
        ),
        move |cx, args| -> BoxFut<'static, ActionResult> {
            let shared = gates.clone();
            Box::pin(async move {
                let name = args["name"].as_str().unwrap_or_default().to_string();
                let device = cx.engine().journal().device().to_string();
                shared.started.send_modify(|all| all.push(format!("{name}@{device}")));
                let gate = shared.gate(&name);
                tokio::select! {
                    permit = gate.acquire() => permit.unwrap().forget(),
                    () = cx.cancel().cancelled() => {
                        shared.cancelled.lock().unwrap().push(name);
                        return Err(ActionError::cancelled());
                    }
                }
                Ok(json!({ "device": device }))
            })
        },
    );
    let gates = shared.clone();
    registry.add_fn(
        ActionSpec::new("test.big", "Give out this many bytes, once a gate is opened if one is named.")
            .required("bytes", T::Int, "how many")
            .arg("gate", T::String, "a gate to wait at first"),
        move |cx, args| -> BoxFut<'static, ActionResult> {
            let shared = gates.clone();
            Box::pin(async move {
                if let Some(name) = args.get("gate").and_then(Value::as_str) {
                    let device = cx.engine().journal().device().to_string();
                    shared.started.send_modify(|all| all.push(format!("{name}@{device}")));
                    shared.gate(name).acquire().await.unwrap().forget();
                }
                let bytes = args.get("bytes").and_then(Value::as_u64).unwrap_or(0) as usize;
                Ok(json!({ "said": "a".repeat(bytes) }))
            })
        },
    );
    registry.add_fn(
        ActionSpec::new("test.who", "Say which device this ran on, and as which key."),
        |cx, _| -> BoxFut<'static, ActionResult> {
            Box::pin(async move {
                let device = cx.engine().journal().device().to_string();
                Ok(json!({ "device": device, "key": cx.principal().key }))
            })
        },
    );
    registry.add_fn(
        ActionSpec::new("test.env", "Say what environment this was given."),
        |cx, _| -> BoxFut<'static, ActionResult> { Box::pin(async move { Ok(json!({ "env": cx.env() })) }) },
    );
    registry.add_fn(
        ActionSpec::new("test.secrets", "Say which secrets this was to hand over, by name."),
        |cx, _| -> BoxFut<'static, ActionResult> {
            Box::pin(async move { Ok(json!({ "secrets": cx.secrets() })) })
        },
    );
    registry.add_fn(
        ActionSpec::new("test.hold", "Try to leave something running."),
        |cx, _| -> BoxFut<'static, ActionResult> {
            Box::pin(async move {
                cx.hold("thing:1").await?;
                Ok(Value::Null)
            })
        },
    );
}

/// One device: everything a daemon has, without the daemon.
pub struct Device {
    pub journal: Journal,
    pub logs: Logs,
    pub studio: Studio,
    pub hosting: Hosting,
    pub engine: Engine,
    pub gateway: Gateway,
    pub dir: PathBuf,
    library: Arc<MemoryLibrary>,
    shared: Arc<Shared>,
}

impl Drop for Device {
    fn drop(&mut self) {
        self.engine.halt();
        self.gateway.halt();
        self.hosting.halt();
        // A test ends what it started, however it ended: what a device
        // hosts is left running by its factory, on purpose.
        for pid in hosting::end_under(&self.dir) {
            eprintln!(
                "the test was over with a program its factory started still running; its group {pid} was ended"
            );
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

async fn assemble(
    journal: &Journal,
    logs: &Logs,
    studio: &Studio,
    dir: &std::path::Path,
    library: &Arc<MemoryLibrary>,
    shared: &Arc<Shared>,
) -> (Hosting, Gateway, Engine) {
    let hosting = Hosting::new(journal.clone(), logs.clone(), dir.join(".run")).await.unwrap();
    hosting.stands_between();
    let wired = shared.clone();
    let gateway = Gateway::carried_by(
        journal.clone(),
        studio.identity.clone(),
        hosting.clone(),
        dir.to_path_buf(),
        Box::new(move |_| Arc::new(WireEnd(wired.wire.get().expect("the wire").clone()))),
    )
    .unwrap();
    let mut registry = Registry::new();
    hosting.register(&mut registry);
    gateway.register(&mut registry);
    test_actions(&mut registry, shared);
    let engine = Engine::new(journal.clone(), logs.clone(), registry, library.clone()).await.unwrap();
    hosting.attach(&engine);
    gateway.attach(&engine);
    // Commands run in the device's own directory, as a project's would.
    let home = dir.to_path_buf();
    engine.set_workdirs(Arc::new(move |_| Some(home.clone())));
    // These tests move the clock a long way to bring links back; a step is
    // only given up on where a test says so.
    gateway.hub().set_grace(Duration::from_secs(10 * 365 * 24 * 3600));
    (hosting, gateway, engine)
}

impl Device {
    pub async fn new(name: &str, clock: &ManualClock, shared: &Arc<Shared>) -> Device {
        let journal = Journal::open(Db::memory().unwrap(), Arc::new(clock.clone()), name).await.unwrap();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("gateway-{name}-{}", &kernel::id::secret()[..12]));
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
        let library = Arc::new(conductor::standard());
        let (hosting, gateway, engine) = assemble(&journal, &logs, &studio, &dir, &library, shared).await;
        Device { journal, logs, studio, hosting, engine, gateway, dir, library, shared: shared.clone() }
    }

    /// Stops the engine and gateway as a crash would and starts new ones on
    /// what was recorded.
    pub async fn restart(&mut self) {
        self.engine.halt();
        self.gateway.halt();
        self.hosting.halt();
        let (hosting, gateway, engine) =
            assemble(&self.journal, &self.logs, &self.studio, &self.dir, &self.library, &self.shared).await;
        self.hosting = hosting;
        self.gateway = gateway;
        self.engine = engine;
    }

    pub async fn apply(&self, source: &str) -> Applied {
        self.apply_under(source, None).await
    }

    /// The same, for `shop` under its environment where one is said.
    pub async fn apply_under(&self, source: &str, environment: Option<&str>) -> Applied {
        let name =
            blueprint::check(source, &blueprint::NoCatalog).blueprint.expect("the test's blueprint").name;
        self.engine
            .apply(Apply {
                blueprint: name,
                source: Some(source.to_string()),
                params: Default::default(),
                key: None,
                parent: None,
                target: None,
                project: environment.map(|_| "shop".to_string()),
                principal: ada(),
                force: false,
                take: false,
                labels: Default::default(),
                environment: environment.map(str::to_string),
            })
            .await
            .unwrap()
    }

    /// Applies a blueprint and waits for its run to finish.
    pub async fn run(&self, source: &str) -> RunInfo {
        let applied = self.apply(source).await;
        self.finished(&applied).await
    }

    pub async fn finished(&self, applied: &Applied) -> RunInfo {
        let run = applied.run.clone().expect("a run was started");
        soon("the run to finish", self.engine.wait_run(&run)).await.unwrap()
    }

    pub async fn topics(&self, pattern: &str) -> Vec<String> {
        let query = journal::EventQuery { topics: pattern.to_string(), limit: 1000, ..Default::default() };
        self.journal.read(query).await.unwrap().into_iter().map(|event| event.topic).collect()
    }

    /// Devices coming and going, in order. (`device.changed`, which says
    /// only that a member's account of itself is newer, is left out: when
    /// it arrives depends on how quickly the member gets round to it.)
    pub async fn comings_and_goings(&self) -> Vec<String> {
        self.topics("device.*").await.into_iter().filter(|topic| topic != "device.changed").collect()
    }
}

pub fn step<'a>(run: &'a RunInfo, name: &str) -> &'a StepInfo {
    run.steps.iter().find(|step| step.step == name).unwrap_or_else(|| panic!("no step `{name}` in {run:?}"))
}

/// What carries a link straight into the hub's gateway, which the test can
/// cut. It carries what it is given and knows nothing of it: the handshake
/// and the sealing are the two gateways' own.
pub struct Wire {
    hub: Mutex<Gateway>,
    /// The member, as the hub's gateway is asked of it directly.
    pub who: Principal,
    up: AtomicBool,
    cut: Notify,
    /// Asked of the member before its next attempt, when set.
    pub retry_after: Mutex<Option<Duration>>,
    /// While set, asking for the stream of commands is taken and never
    /// answered.
    pub mute: AtomicBool,
    /// Handshakes begun.
    pub attempts: AtomicUsize,
    /// Each first message of a handshake, as it crossed.
    pub hellos: Mutex<Vec<Vec<u8>>>,
}

impl Wire {
    pub fn cut(&self) {
        self.up.store(false, Ordering::SeqCst);
        self.cut.notify_waiters();
    }

    pub fn restore(&self) {
        self.up.store(true, Ordering::SeqCst);
    }

    /// Points the wire at a hub that has been restarted, or at another.
    pub fn rewire(&self, hub: &Gateway) {
        *self.hub.lock().unwrap() = hub.clone();
        self.cut.notify_waiters();
    }

    fn down(&self) -> LinkError {
        LinkError { retry_after: *self.retry_after.lock().unwrap(), ..LinkError::new("the wire is cut") }
    }
}

/// Words where something sealed was wanted, as a hub's refusal over HTTP is.
fn said(err: kernel::Error) -> LinkError {
    LinkError { said: true, ..LinkError::new(err.message) }
}

/// The member's end of a [`Wire`].
pub struct WireEnd(pub Arc<Wire>);

impl Carrier for WireEnd {
    fn place(&self) -> String {
        "http://hub".into()
    }

    fn exchange(&self, path: &'static str, body: Vec<u8>) -> BoxFut<'static, Result<Vec<u8>, LinkError>> {
        let wire = self.0.clone();
        Box::pin(async move {
            if path == HELLO_PATH {
                wire.attempts.fetch_add(1, Ordering::SeqCst);
                wire.hellos.lock().unwrap().push(body.clone());
            }
            if !wire.up.load(Ordering::SeqCst) {
                return Err(wire.down());
            }
            let hub = wire.hub.lock().unwrap().clone();
            match path {
                HELLO_PATH => match hub.hello(&body, [127, 0, 0, 1].into()).await {
                    Ok(answer) => Ok(answer),
                    Err(Shut::Unread(message)) => Err(LinkError { said: true, ..LinkError::new(message) }),
                    Err(Shut::Later { wait, message }) => {
                        Err(LinkError { retry_after: Some(wait), ..LinkError::new(message) })
                    }
                    Err(Shut::Failed(err)) => Err(LinkError::new(err.message)),
                },
                REPORT_PATH => hub.sealed_report(&body).await.map_err(said),
                other => Err(LinkError::new(format!("nothing is exchanged at {other}"))),
            }
        })
    }

    fn stream(
        &self,
        _path: &'static str,
        body: Vec<u8>,
    ) -> BoxFut<'static, Result<mpsc::Receiver<Vec<u8>>, LinkError>> {
        let wire = self.0.clone();
        Box::pin(async move {
            if !wire.up.load(Ordering::SeqCst) {
                return Err(wire.down());
            }
            if wire.mute.load(Ordering::SeqCst) {
                // Taken, and never answered.
                std::future::pending::<()>().await;
            }
            let hub = wire.hub.lock().unwrap().clone();
            let mut link = hub.sealed_link(&body).await.map_err(said)?;
            let (pieces, stream) = mpsc::channel(64);
            tokio::spawn(async move {
                loop {
                    // Listen for a cut before looking, so none is missed.
                    let cut = wire.cut.notified();
                    tokio::pin!(cut);
                    cut.as_mut().enable();
                    tokio::select! {
                        line = link.next() => {
                            let Some(piece) = line.and_then(|line| gateway::door::decode(&line)) else { return };
                            if pieces.send(piece).await.is_err() {
                                return;
                            }
                        }
                        () = &mut cut => return,
                        // The member went away, as when its daemon dies.
                        () = pieces.closed() => return,
                    }
                }
            });
            Ok(stream)
        })
    }
}

/// A hub called `hub` and a member it knows as `builder-1`.
pub struct Rig {
    pub hub: Device,
    pub member: Device,
    /// What lets the member in, once.
    pub invitation: String,
    pub wire: Arc<Wire>,
    pub clock: ManualClock,
    pub shared: Arc<Shared>,
}

impl Rig {
    /// The two devices, the member invited to the hub's fleet and not yet
    /// joined.
    pub async fn apart() -> Rig {
        let clock = ManualClock::new();
        let shared = Arc::new(Shared::default());
        let hub = Device::new("hub", &clock, &shared).await;
        let member = Device::new("member-box", &clock, &shared).await;
        let invited = hub.gateway.invite("builder-1", None, vec!["http://hub".into()], &ada()).await.unwrap();
        let who = Principal {
            kind: PrincipalKind::Device,
            id: "builder-1".into(),
            name: "builder-1".into(),
            scopes: vec!["fleet".into()],
            key: None,
        };
        let wire = Arc::new(Wire {
            hub: Mutex::new(hub.gateway.clone()),
            who,
            up: AtomicBool::new(true),
            cut: Notify::new(),
            retry_after: Mutex::new(None),
            mute: AtomicBool::new(false),
            attempts: AtomicUsize::new(0),
            hellos: Mutex::default(),
        });
        shared.wire.set(wire.clone()).ok().expect("one wire");
        Rig { hub, member, invitation: invited.invitation, wire, clock, shared }
    }

    /// The two devices, linked.
    pub async fn linked() -> Rig {
        let rig = Rig::apart().await;
        rig.link(vec!["*".into()]).await;
        rig
    }

    /// Has the member join the hub with its invitation, letting the hub do
    /// `allow` on it.
    pub async fn link(&self, allow: Vec<String>) {
        self.member.gateway.join(&self.invitation, Some(allow), &ada()).await.unwrap();
        self.until_linked().await;
    }

    fn is_linked(&self) -> bool {
        let member_sees = self.member.gateway.uplink().is_some_and(|link| link.status.connected);
        member_sees && self.hub.gateway.hub().linked_since("builder-1").is_some()
    }

    /// Lets time pass until both ends agree the link is up. A member waits
    /// before each new attempt; time here only moves when a test moves it.
    pub async fn until_linked(&self) {
        self.until("the member to link to the hub", || self.is_linked()).await;
    }

    /// Lets time pass, a member's wait at a time, until `done`.
    pub async fn until(&self, what: &str, done: impl Fn() -> bool) {
        soon(what, async {
            let mut idle = 0u32;
            while !done() {
                // Move time only as far as the member's next attempt, and
                // only when it is waiting for one: a test that leaps hours
                // ahead to bring a link back has tested something else.
                let link = self.member.gateway.uplink();
                let trying = link.as_ref().is_some_and(|link| !link.status.connected)
                    && link.as_ref().is_some_and(|link| link.status.retry_at.is_none());
                let waiting_until =
                    link.filter(|link| !link.status.connected).and_then(|link| link.status.retry_at);
                let now = self.member.journal.now_ms();
                match waiting_until {
                    Some(at) if at > now => self.clock.advance(Duration::from_millis((at - now) as u64)),
                    // In the middle of an attempt: it is not time it waits
                    // for, and time that ran on here would be the hub taking
                    // a minute to answer it.
                    None if trying => {}
                    // Its timer may have been set just after we looked.
                    _ if idle >= 200 => {
                        idle = 0;
                        self.clock.advance(Duration::from_secs(1));
                    }
                    _ => idle += 1,
                }
                tokio::task::yield_now().await;
            }
        })
        .await;
    }

    pub async fn until_unlinked(&self) {
        soon("both ends to notice the link is gone", async {
            while self.member.gateway.uplink().is_some_and(|link| link.status.connected)
                || self.hub.gateway.hub().linked_since("builder-1").is_some()
            {
                tokio::task::yield_now().await;
            }
        })
        .await;
    }
}
