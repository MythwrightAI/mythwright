//! Running things on this device.
//!
//! This crate gives blueprints their hands: `shell` to run a command,
//! `service.*` to keep a process alive and healthy, `router.*` to put a
//! stable address in front of services and shift traffic between them, and
//! `system.check` to look before leaping. None of it knows about blueprints
//! beyond being registered as actions; all of it reports what it does as
//! events and measurements.

mod actions;
pub mod apart;
pub mod copy;
pub mod events;
pub mod group;
mod here;
pub mod kept;
pub mod named;
pub mod ports;
mod proc;
pub mod repository;
pub mod rollout;
mod router;
mod said;
pub mod script;
pub mod secrets;
mod service;
pub mod shares;
mod shell;
mod system;
pub mod unsaid;
pub mod wall;
mod watch;
pub mod wrapped;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use conductor::{Engine, InstanceInfo, Registry};
use journal::{Journal, Logs};
use kernel::{Result, Source};
use serde_json::json;
use tokio::task::AbortHandle;

pub use actions::HEALTH;
pub use here::end_under;
pub use proc::{ADDRESS_ENV, ENVIRONMENT_ENV, PROFILE_ENV, PROGRAM_ENV, cleared_env};
pub use router::{Backend, BackendInfo, RouterInfo, Stats};
pub use service::{Health, Probe, Restart, ServiceInfo, ServiceSpec, Status};
pub use system::{Resources, resources, unplaced, volume};
pub use wall::{Shown, Walls};
pub use watch::{Changes, Watch};

#[derive(Clone)]
pub struct Hosting {
    inner: Arc<Inner>,
}

struct Inner {
    journal: Journal,
    logs: Logs,
    /// Where output and exit files of running processes live.
    run_dir: PathBuf,
    /// How this device says that a file changed; nothing here looks on a
    /// timer to find out.
    watch: Watch,
    services: Mutex<HashMap<String, Arc<service::Service>>>,
    routers: Mutex<HashMap<String, Arc<router::Router>>>,
    tasks: Mutex<Vec<AbortHandle>>,
    /// Where the daemon answers, once whoever listens has said.
    address: OnceLock<String>,
    /// What the factory's profile is called, once whoever started the
    /// factory has said.
    profile: OnceLock<String>,
    /// Where the factory keeps its secrets and what they are sealed with,
    /// once whoever started it has said.
    secrets: OnceLock<secrets::Secrets>,
    /// What wraps the programs started here, when it is not this program.
    wraps: OnceLock<wrapped::Wraps>,
    /// How walls are held here, once whoever started the factory has
    /// said; by the system's own program, if they said nothing.
    walls: OnceLock<Walls>,
}

/// Adds the two blocks of a rollout (`rollout.next`, `rollout.watched`)
/// to `registry`: each a function of what it is handed, which no
/// `Hosting` is needed for. A sandbox test has these as
/// they are, and a stand-in for everything else here.
pub fn register_blocks(registry: &mut Registry) {
    rollout::register(registry);
}

impl Hosting {
    /// `run_dir` must be on a real disk and survive a restart of the
    /// conductor: it is how running processes are found again.
    pub async fn new(journal: Journal, logs: Logs, run_dir: impl Into<PathBuf>) -> Result<Hosting> {
        journal.db().migrate("hosting", service::SCHEMA).await?;
        let run_dir = run_dir.into();
        std::fs::create_dir_all(&run_dir)?;
        // Found now, for everything started later to be told of.
        proc::program();
        let watch = Watch::new();
        if watch.folder(&run_dir).is_none() {
            let reason = watch.problem().unwrap_or_default();
            let effect = "output is looked for a few times a second instead of arriving as it is written";
            let data = json!({ "what": "file notifications", "reason": reason, "effect": effect });
            let _ = journal.emit("system.degraded", Source::actor("system"), data).await;
        }
        Ok(Hosting {
            inner: Arc::new(Inner {
                journal,
                logs,
                run_dir,
                watch,
                services: Mutex::default(),
                routers: Mutex::default(),
                tasks: Mutex::default(),
                address: OnceLock::new(),
                profile: OnceLock::new(),
                secrets: OnceLock::new(),
                wraps: OnceLock::new(),
                walls: OnceLock::new(),
            }),
        })
    }

    /// Says how walls are held on this device ([`wall`]). Said once, as
    /// the factory starts.
    pub fn holds_walls(&self, walls: Walls) {
        let _ = self.inner.walls.set(walls);
    }

    /// How walls are held on this device.
    pub fn walls(&self) -> Walls {
        self.inner.walls.get().cloned().unwrap_or_else(Walls::system)
    }

    /// Says where the factory keeps its secrets, a folder of its data
    /// folder which no copy of the factory carries, and the key of this
    /// installation they are sealed with ([`secrets`]).
    pub fn keeps_secrets_in(&self, dir: impl Into<PathBuf>, seal: link::kept::Seal) {
        let _ = self.inner.secrets.set(secrets::Secrets::new(dir.into(), seal));
    }

    /// The journal a service or a router of `project` says what happens
    /// to it in. What either says is said by the instance that owns it
    /// and names no project, so it is said where that project's events
    /// are kept (`Journal::said_of`): a service that stopped and the
    /// instance that owned it being over are then in one order, for a
    /// handler that hears both and for whoever reads what happened.
    pub(crate) fn said_of(&self, project: &Option<String>) -> journal::Journal {
        self.inner.journal.said_of(project.as_deref())
    }

    /// The secrets the factory keeps.
    pub fn secrets(&self) -> Result<secrets::Secrets> {
        match self.inner.secrets.get() {
            Some(kept) => Ok(kept.clone()),
            None => Err(kernel::Error::unavailable(
                "this factory was not told where to keep secrets, and keeps none",
            )),
        }
    }

    /// Keeps a secret and says that one was set: its name and who set it,
    /// never what it is.
    pub async fn set_secret(&self, name: &str, value: &str, by: &kernel::Principal) -> Result<()> {
        self.secrets()?.set(name, value)?;
        let said = json!({ "name": name, "by_name": by.name });
        let _ = self.inner.journal.emit("secret.set", Source::actor(by.actor()), said).await;
        Ok(())
    }

    /// Takes a secret away, and says so. A service that names it goes on
    /// with what it was handed, and is not started again without it.
    pub async fn remove_secret(&self, name: &str, by: &kernel::Principal) -> Result<()> {
        if !self.secrets()?.remove(name)? {
            return Err(kernel::Error::not_found(format!("there is no secret called `{name}`")));
        }
        let said = json!({ "name": name, "by_name": by.name });
        let _ = self.inner.journal.emit("secret.removed", Source::actor(by.actor()), said).await;
        Ok(())
    }

    /// What a program is handed beside its environment: each variable in
    /// `named` with the secret it names, read now, as the program starts.
    pub(crate) fn hand_over(
        &self,
        named: &std::collections::BTreeMap<String, String>,
        env: &mut std::collections::BTreeMap<String, String>,
    ) -> Result<()> {
        if named.is_empty() {
            return Ok(());
        }
        let kept = self.secrets()?;
        for (variable, name) in named {
            env.insert(variable.clone(), kept.handed(name)?);
        }
        Ok(())
    }

    /// Says where the daemon answers, as `http://127.0.0.1:7700`, so that
    /// what is started from here on is told ([`ADDRESS_ENV`]). Hosting does
    /// not listen for anyone itself; whoever does says so once.
    pub fn answers_at(&self, url: impl Into<String>) {
        let _ = self.inner.address.set(url.into());
    }

    /// Where the daemon answers, if whoever listens has said.
    pub fn address(&self) -> Option<String> {
        self.inner.address.get().cloned()
    }

    /// Has the factory's own process stand between a program and its
    /// files, in the wrapper's place ([`wrapped::Wraps::Here`]): a thread
    /// that is not the factory's waits for the program, says how it
    /// ended, and writes what it prints without what it was handed. For
    /// a factory that is its own test, and nothing else: there this
    /// program is the test's, and cannot be started as the wrapper. Said
    /// once, before anything is started.
    pub fn stands_between(&self) {
        let _ = self.inner.wraps.set(wrapped::Wraps::Here);
    }

    pub(crate) fn wraps(&self) -> wrapped::Wraps {
        self.inner.wraps.get().copied().unwrap_or_default()
    }

    /// Gives a command that something other than this crate starts the
    /// environment a `shell` step's command has: where the product's own
    /// program and the daemon are, what the factory's profile is
    /// called, none of the conductor's credentials, and then `env`, which
    /// is what the blueprint and the step said.
    pub fn dress(
        &self,
        command: &mut tokio::process::Command,
        env: &std::collections::BTreeMap<String, String>,
    ) {
        proc::dress(command, self.address().as_deref(), self.profile().as_deref(), env);
    }

    /// The same for a program started for a step, which is also handed the
    /// secrets the step's environment names (`cx.secrets()`), read now.
    /// One that is not kept is why the program is not started.
    ///
    /// What comes back is what was handed, to be taken out of everything
    /// the program says before anything is done with it
    /// ([`unsaid::Unsaid::from`]): whoever starts the program reads what
    /// it prints, and what is read is written down.
    pub fn dress_handed(
        &self,
        command: &mut tokio::process::Command,
        env: &std::collections::BTreeMap<String, String>,
        named: &std::collections::BTreeMap<String, String>,
    ) -> Result<unsaid::Unsaid> {
        let mut env = env.clone();
        // Last, so that nothing written in the step stands in for one.
        self.hand_over(named, &mut env)?;
        self.dress(command, &env);
        Ok(unsaid::Unsaid::named(named, &env))
    }

    /// Says what the factory's profile is called, the empty text for one
    /// with none, so that what is started from here on is told
    /// ([`PROFILE_ENV`]). Hosting does not decide it; whoever starts the
    /// factory says so once.
    pub fn in_profile(&self, name: impl Into<String>) {
        let _ = self.inner.profile.set(name.into());
    }

    pub(crate) fn profile(&self) -> Option<String> {
        self.inner.profile.get().cloned()
    }

    /// Where the files of a program that goes on without the daemon are
    /// kept, under `tag`: in the folder of everything the daemon runs.
    pub fn kept(&self, tag: &str) -> kept::Files {
        kept::Files::new(self.run_dir(), tag).wrapped(self.wraps())
    }

    pub(crate) fn run_dir(&self) -> PathBuf {
        self.inner.run_dir.clone()
    }

    /// This device's file notifications, for anything else that follows a
    /// file: one watcher serves the whole daemon.
    pub fn watch(&self) -> &Watch {
        &self.inner.watch
    }

    /// Adds this crate's actions to `registry`.
    pub fn register(&self, registry: &mut Registry) {
        registry.add(shell::Shell::new(self.clone()));
        system::register(registry);
        actions::register(registry, self);
        register_blocks(registry);
        registry.emits(events::events());
    }

    /// Ties services and routers to the instances that hold them: when an
    /// instance stops, what it held is stopped with it.
    pub fn attach(&self, engine: &Engine) {
        let hosting = self.clone();
        engine.on_stop(Arc::new(move |info: InstanceInfo| {
            let hosting = hosting.clone();
            Box::pin(async move {
                for hold in &info.holds {
                    if let Some(name) = hold.strip_prefix("service:") {
                        hosting.stop_service(name).await;
                        hosting.forget_service(name).await;
                    } else if let Some(name) = hold.strip_prefix("router:") {
                        hosting.remove_router(name).await;
                    }
                }
            })
        }));
    }

    /// Finds what was running when the conductor last stopped. Call before
    /// resuming the engine, so resumed steps find their services.
    pub async fn resume(&self) -> Result<()> {
        self.resume_services().await?;
        self.resume_routers().await?;
        Ok(())
    }

    /// Stops watching, without stopping anything: services keep running for
    /// the next conductor to adopt.
    pub fn halt(&self) {
        for task in self.inner.tasks.lock().expect("hosting lock").drain(..) {
            task.abort();
        }
        // Nothing here starts a service's program again, so no port is
        // kept from anybody: a conductor that finds the services again
        // speaks for theirs, and in a test's process, which goes on, a
        // factory that was put away would hold them for good.
        for service in self.inner.services.lock().expect("hosting lock").values() {
            service.let_go();
        }
        // Whoever followed a file has just been stopped: the system's
        // notifications are given back with them.
        self.inner.watch.close();
    }
}
