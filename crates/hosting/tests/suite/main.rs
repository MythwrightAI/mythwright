//! One test binary for the crate; add modules here.
//!
//! These tests start real processes, so they take real (small) amounts of
//! time, but none of them waits by sleeping: each waits for the event or log
//! line that says the thing it needs has happened.

mod apart;
mod cleared;
mod group;
mod kept;
mod named;
mod ports;
mod rigs;
mod rollout;
mod rollouts;
mod routing;
mod script;
mod secrets;
mod services;
mod shares;
mod shell;
mod standin;
mod volumes;
mod walls;
mod watch;
mod wrapped;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use conductor::{Applied, Apply, Engine, MemoryLibrary, Registry, RunInfo, StepInfo};
use hosting::Hosting;
use journal::{Follow, Journal, Logs};
use kernel::{Event, ManualClock, Principal, PrincipalKind};
use serde_json::Value;

/// The stand-in for a program, which the tests here start where they
/// need one that says, reads, writes, waits and ends as they tell it
/// (`crates/hosting/standin`): built with this suite, and found by what
/// cargo says of where.
pub const STANDIN: &str = env!("CARGO_BIN_EXE_hosting-standin");

pub struct Rig {
    pub engine: Engine,
    pub hosting: Hosting,
    pub clock: ManualClock,
    pub journal: Journal,
    /// The directory steps run in.
    pub dir: PathBuf,
    logs: Logs,
    library: Arc<MemoryLibrary>,
}

/// A test that fails part-way must not leave its services running, so the
/// rig takes down whatever is left when it goes out of scope, pass or panic.
impl Drop for Rig {
    fn drop(&mut self) {
        self.engine.halt();
        self.hosting.halt();
        // Every program this rig's factory started and that is still
        // running, a service as a step's, whether or not the factory
        // had kept that it started it: each leads its own group, and is
        // ended with it as the factory ends one, by asking the system.
        for pid in hosting::end_under(&self.dir) {
            eprintln!(
                "the test was over with a program its factory started still running; its group {pid} was ended"
            );
        }
        let _ = std::fs::remove_dir_all(&self.dir);
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

pub async fn soon<T>(what: &str, work: impl Future<Output = T>) -> T {
    match tokio::time::timeout(Duration::from_secs(30), work).await {
        Ok(value) => value,
        Err(_) => panic!("timed out waiting for: {what}"),
    }
}

async fn build(
    journal: &Journal,
    logs: &Logs,
    dir: &std::path::Path,
    library: &Arc<MemoryLibrary>,
    says: bool,
) -> (Engine, Hosting) {
    let hosting = Hosting::new(journal.clone(), logs.clone(), dir.join(".run")).await.unwrap();
    // This program is the test's: it cannot be started again as the
    // wrapper of what the factory starts, so the test's own process
    // stands in for one. A rig says so, or is told to (`wrapped.rs`).
    if says {
        hosting.stands_between();
    }
    let own = link::own::Own::open(dir).unwrap();
    hosting.keeps_secrets_in(dir.join(".secrets"), own.at_rest());
    let mut registry = Registry::new();
    hosting.register(&mut registry);
    let engine = Engine::new(journal.clone(), logs.clone(), registry, library.clone()).await.unwrap();
    hosting.attach(&engine);
    let workdir = dir.to_path_buf();
    engine.set_workdirs(Arc::new(move |_| Some(workdir.clone())));
    (engine, hosting)
}

impl Rig {
    pub async fn new() -> Rig {
        Rig::made(true).await
    }

    /// A factory inside the test's program that did not say that it is.
    pub async fn that_said_nothing() -> Rig {
        Rig::made(false).await
    }

    async fn made(says: bool) -> Rig {
        let clock = ManualClock::new();
        let journal = Journal::memory(Arc::new(clock.clone())).await.unwrap();
        // Under the build's target directory: real disk, and cleaned with it.
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("hosting-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        let logs = Logs::new(dir.join(".logs"));
        let library = Arc::new(MemoryLibrary::new());
        let (engine, hosting) = build(&journal, &logs, &dir, &library, says).await;
        Rig { engine, hosting, clock, journal, dir, logs, library }
    }

    /// Stops the conductor without stopping what it started, then brings up
    /// a new one that has to find it all again.
    pub async fn restart(&mut self) {
        self.engine.halt();
        self.hosting.halt();
        let (engine, hosting) = build(&self.journal, &self.logs, &self.dir, &self.library, true).await;
        hosting.resume().await.unwrap();
        engine.resume().await.unwrap();
        self.engine = engine;
        self.hosting = hosting;
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

    pub async fn run(&self, source: &str, params: Value) -> RunInfo {
        let applied = self.apply(source, params).await;
        self.finished(&applied).await
    }

    pub async fn finished(&self, applied: &Applied) -> RunInfo {
        soon("the run to finish", self.engine.wait_run(applied.run.as_ref().expect("a run was started")))
            .await
            .unwrap()
    }

    pub async fn event(&self, topic: &str, wanted: impl Fn(&Event) -> bool) -> Arc<Event> {
        let mut follow = self.journal.follow(0, topic);
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

    /// Follows output lines from now. Make this before starting whatever
    /// will print, since lines are not stored as events.
    pub fn lines(&self) -> Follow {
        self.journal.follow(self.journal.last_seq(), "log.line")
    }

    pub async fn events(&self, topics: &str) -> Vec<Event> {
        let query = journal::EventQuery { topics: topics.to_string(), limit: 10_000, ..Default::default() };
        self.journal.read(query).await.unwrap()
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

/// Waits for a line of output containing `text`.
pub async fn line(lines: &mut Follow, text: &str) -> Arc<Event> {
    soon(&format!("a line containing `{text}`"), async {
        loop {
            let event = lines.next().await.unwrap();
            if event.data["text"].as_str().is_some_and(|line| line.contains(text)) {
                return event;
            }
        }
    })
    .await
}

pub fn step<'a>(run: &'a RunInfo, id: &str) -> &'a StepInfo {
    run.steps.iter().find(|step| step.step == id).unwrap_or_else(|| panic!("run has no step `{id}`"))
}

/// Whether a process with this pid is running (a zombie is not).
#[cfg(target_os = "linux")]
pub fn alive(pid: u32) -> bool {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => !stat.rsplit_once(')').is_some_and(|(_, rest)| rest.trim_start().starts_with(['Z', 'X'])),
        Err(_) => false,
    }
}

/// The same where the system keeps no such file: asked of the system as
/// the factory asks it.
#[cfg(not(target_os = "linux"))]
pub fn alive(pid: u32) -> bool {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System};
    let Ok(mut system) = System::new() else { return false };
    let pid = Pid::from_u32(pid);
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing());
    system
        .process(pid)
        .is_some_and(|process| !matches!(process.status(), ProcessStatus::Zombie | ProcessStatus::Dead))
}

/// Waits, without sleeping, until every pid is gone.
pub async fn gone(pids: &[u32]) {
    soon("the processes to die", async {
        while pids.iter().any(|pid| alive(*pid)) {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

pub fn pids(path: &std::path::Path) -> Vec<u32> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}
