//! The conductor: owns instances and drives them.
//!
//! An *instance* is a blueprint brought to life with params and durable
//! state: the "durable object" of the design. It runs its steps once when it
//! starts and then, if the blueprint has handlers or the instance holds
//! something (a service, attached children), stays alive reacting to events.
//! A parent's `call`/`spawn` steps and `uses` attachments create child
//! instances, so a fleet is a tree of them. Each node of that tree is what the
//! design calls a sub-conductor.
//!
//! Applying a blueprint whose instance already exists (same parent, same key)
//! updates that instance instead of starting another: the new revision's
//! steps run against the state the old one left. That is the IaC-like half of
//! the model, and it is what makes a rollout "apply the blueprint again".
//! Whose authority the instance then has is in [`crate::update`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use blueprint::{Blueprint, Labels, template};
use journal::{Journal, LogLine, Logs};
use kernel::{BoxFut, Cancel, Error, Fields, Principal, Result, Source};
use runs::{Definition, InstanceFilter, InstanceInfo, InstanceStatus, RunInfo, Store, to_say};
use serde_json::{Value, json};
use tokio::sync::oneshot;
use tokio::task::AbortHandle;

use crate::action::{ActionCx, ActionError, ActionResult, CxInner, Registry, RemoteCall, check_args};
pub use crate::asked::{Applied, Apply};
use crate::delivery::Deliveries;
use crate::library::Library;
use crate::sink::Sink;
use crate::update::Asker;
use crate::{sink, trigger};

/// Runs an action on another device. Installed by the gateway.
pub trait Remote: Send + Sync + 'static {
    fn run_action(
        &self,
        device: &str,
        action: &str,
        args: Fields,
        cx: ActionCx,
    ) -> BoxFut<'static, ActionResult>;
}

/// Says where a project's files are. Installed by whoever owns projects.
pub type Workdirs = Arc<dyn Fn(Option<&str>) -> Option<std::path::PathBuf> + Send + Sync>;

/// Called when an instance stops, so whoever gave it something to hold
/// (services, slots) can take it back.
pub type StopHook = Arc<dyn Fn(InstanceInfo) -> BoxFut<'static, ()> + Send + Sync>;

#[derive(Clone)]
pub struct Engine {
    pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
    /// The journal this conductor keeps what it runs in, and says its
    /// own events in: its project's, where it is a project's conductor.
    pub own: Journal,
    /// The factory's own journal, which keeps what is nobody's and the
    /// small durable state every conductor shares. The same as `own`
    /// for the factory's conductor.
    pub factory: Journal,
    /// The project this conductor runs what was applied for; none for
    /// the factory's own, which runs what is nobody's and every project
    /// that has no journal of its own.
    pub project: Option<String>,
    pub logs: Logs,
    pub store: Store,
    pub registry: Registry,
    pub library: Arc<dyn Library>,
    pub delivery: Deliveries,
    pub(crate) live: Mutex<HashMap<String, Arc<Live>>>,
    /// The project of instances no longer in hand ([`crate::whose`]).
    pub(crate) whose: Mutex<HashMap<String, Option<String>>>,
    /// Whether a step that is passed over is kept as an event ([`crate::owed`]).
    pub owed: Arc<crate::owed::Owed>,
    /// Whoever waits for a run to end, by the run ([`crate::settle`]).
    pub waiters: Mutex<HashMap<String, Vec<oneshot::Sender<()>>>>,
    /// What every conductor of a factory shares, so that one made
    /// later has what the first was given.
    stop_hooks: Arc<Mutex<Vec<StopHook>>>,
    remote: Arc<Mutex<Option<Arc<dyn Remote>>>>,
    workdirs: Arc<Mutex<Option<Workdirs>>>,
    tasks: Mutex<Vec<AbortHandle>>,
    halting: AtomicBool,
    /// Held while an instance is looked for by its key and, when there is
    /// none, made: two who ask for the same key at once are answered by
    /// one instance, never by one each.
    making: tokio::sync::Mutex<()>,
    /// The children `call` steps are waiting on ([`crate::why`]).
    pub calls: crate::why::Calls,
    /// The stops under way ([`crate::meets`]).
    pub stops: crate::meets::Stops,
    pub environment: Arc<crate::profile::Held>,
    /// A project's environments as what runs under one reads them, and
    /// the turn at changing them ([`crate::environs`]): one for every
    /// conductor of the factory.
    pub environs: Arc<environments::held::Held>,
    /// Which instances say they went on with an earlier text ([`crate::reach`]).
    pub kept: crate::reach::Kept,
    /// What this conductor keeps of texts beside what it runs (`texts`).
    pub texts: texts::Texts,
    /// What answers for comparisons, once installed ([`crate::comparing`]).
    pub compared: Arc<std::sync::OnceLock<crate::comparing::Compared>>,
    /// Which texts were said not to be taken ([`crate::taking`]).
    pub taking: Arc<crate::taking::Kept>,
    /// Every conductor of the factory ([`crate::set`]).
    pub set: Arc<crate::set::Conductors>,
    /// How far this one has come ([`crate::set::standing`]).
    pub standing: std::sync::atomic::AtomicU8,
}

/// An instance that is loaded and being driven.
pub(crate) struct Live {
    pub id: String,
    /// Cancelled when the instance stops.
    pub cancel: Cancel,
    blueprint: Mutex<Arc<Blueprint>>,
    pub meta: Mutex<Meta>,
    /// Serialises applies to this instance. Held apart from the
    /// instance, so that work an asker let go of keeps the turn until it
    /// is done ([`crate::whole`]).
    pub apply_lock: Arc<tokio::sync::Mutex<()>>,
    /// Bumped by every re-apply and by stop. Work started under an older
    /// generation finishes quietly instead of deciding the instance's fate.
    pub generation: AtomicU64,
    /// Cancels the current generation's runs and handlers.
    work: Mutex<Cancel>,
    /// The definition to return to if the update in progress fails.
    pub previous: Mutex<Option<Definition>>,
    /// Its handlers, while they listen ([`crate::trigger`]).
    pub listening: Mutex<trigger::Listening>,
    /// The generation whose steps, once they have settled, are followed by
    /// a look at everything below, and one more; 0 for none
    /// (`Live::owes_look` in [`crate::current`]).
    pub look_below: AtomicU64,
    /// How many texts it has taken owing a look, and the turn at those
    /// writes and at taking the note off: a look that was owed again
    /// while it went on does not take it off (`Engine::look_below`).
    pub looks: (AtomicU64, tokio::sync::Mutex<()>),
    /// Cancelled to call off a text it is waiting to be carried over to
    /// ([`crate::carry`]).
    pub carrying: Mutex<crate::carry::Carries>,
    /// Held while a `set` changes its state: one at a time, from working
    /// out what is set to having written it down and said so.
    pub(crate) setting: tokio::sync::Mutex<()>,
    /// The turns its handlers take: all of them one at a time, where its
    /// text says `one_at_a_time`, or by a key ([`crate::turns`]).
    pub turn: crate::turns::Turns,
    /// What the steps of its runs have done, for one that keeps nothing as
    /// it goes ([`crate::held::Held`]).
    pub held: crate::held::Held,
    /// Whether its text could be owed a step's being passed over.
    pub owed: crate::owed::Hearing,
    /// When it was last given something to run (`Live::given`).
    pub given: AtomicU64,
}

#[derive(Clone)]
pub(crate) struct Meta {
    pub blueprint: String,
    pub revision: String,
    pub params: Fields,
    pub state: Fields,
    pub principal: Principal,
    pub project: Option<String>,
    pub parent: Option<String>,
    pub key: String,
    pub target: Option<String>,
    pub holds: Vec<String>,
    pub labels: Labels,
    /// How the text in force hears where it says no `scope` ([`crate::whose`]).
    pub heard: Option<String>,
    /// The environment of its project's it runs under, empty for none.
    pub environment: String,
}

impl Live {
    pub fn blueprint(&self) -> Arc<Blueprint> {
        self.blueprint.lock().expect("live lock").clone()
    }

    pub fn meta(&self) -> Meta {
        self.meta.lock().expect("live lock").clone()
    }

    pub fn work(&self) -> Cancel {
        self.work.lock().expect("live lock").clone()
    }

    /// The text in force and the authority it was applied with, read
    /// together: what starts work takes both from here, so that no text is
    /// ever run with the authority of whoever applied another.
    pub fn acting(&self) -> (Arc<Blueprint>, Principal) {
        let meta = self.meta.lock().expect("live lock");
        (self.blueprint.lock().expect("live lock").clone(), meta.principal.clone())
    }

    /// Puts a definition in force together with the authority of whoever
    /// applied it; `state` when it is being applied, not gone back to.
    pub fn redefine(&self, blueprint: Arc<Blueprint>, definition: Definition, state: Option<Fields>) {
        let mut meta = self.meta.lock().expect("live lock");
        self.given();
        self.owed.hear(&blueprint);
        *self.blueprint.lock().expect("live lock") = blueprint;
        meta.revision = definition.revision;
        meta.params = definition.params;
        // A text gone back to hears as it did when it was taken.
        meta.heard = definition.heard;
        // One written down before its authority was kept with it has the
        // instance's, which nothing but this ever changed.
        if let Some(principal) = definition.principal {
            meta.principal = principal;
        }
        if let Some(state) = state {
            meta.state = state;
        }
    }

    /// Cancels everything running under the current generation and starts a
    /// new one.
    pub fn next_generation(&self) -> u64 {
        // Its handlers are ended, and are started again when the steps
        // of the new generation settle: none is started for this one now.
        let generation = self.listening.lock().expect("live lock").ended(&self.generation);
        {
            let mut work = self.work.lock().expect("live lock");
            work.cancel();
            *work = self.cancel.child();
        }
        if let Some(waiting) = self.carrying.lock().expect("live lock").waiting.take() {
            waiting.cancel.cancel();
        }
        generation
    }
}

impl Engine {
    pub async fn new(
        journal: Journal,
        logs: Logs,
        registry: Registry,
        library: Arc<dyn Library>,
    ) -> Result<Engine> {
        let store = Store::open(&journal).await?;
        let texts = texts::Texts::open(journal.clone(), store.clone(), library.clone()).await;
        let front = Engine {
            inner: Arc::new(Inner {
                environment: Arc::new(crate::profile::Held::load(&journal).await?),
                environs: Arc::default(),
                own: journal.clone(),
                factory: journal,
                project: None,
                logs,
                store,
                registry,
                library,
                delivery: Deliveries::default(),
                live: Mutex::default(),
                whose: Mutex::default(),
                owed: Arc::new(crate::owed::Owed::new(&crate::owed::OWN)),
                waiters: Mutex::default(),
                stop_hooks: Arc::default(),
                remote: Arc::default(),
                workdirs: Arc::default(),
                tasks: Mutex::default(),
                halting: AtomicBool::new(false),
                making: tokio::sync::Mutex::new(()),
                calls: Default::default(),
                stops: Default::default(),
                kept: Default::default(),
                texts,
                compared: Default::default(),
                taking: Arc::default(),
                set: Arc::default(),
                standing: Default::default(),
            }),
        };
        front.with_each().await
    }

    /// A conductor for `project`, beside this one, the factory's own:
    /// what it runs it keeps in `own`, the project's journal, and what
    /// every conductor shares it is handed.
    pub(crate) async fn beside(&self, project: &str, own: Journal) -> Result<Engine> {
        let inner = &self.inner;
        let store = Store::open(&own).await?;
        Ok(Engine {
            inner: Arc::new(Inner {
                texts: inner.texts.beside(project, own.clone(), store.clone()),
                store,
                own,
                factory: inner.factory.clone(),
                project: Some(project.to_string()),
                logs: inner.logs.clone(),
                registry: inner.registry.clone(),
                library: inner.library.clone(),
                delivery: Deliveries::default(),
                live: Mutex::default(),
                whose: Mutex::default(),
                owed: inner.owed.clone(),
                waiters: Mutex::default(),
                stop_hooks: inner.stop_hooks.clone(),
                remote: inner.remote.clone(),
                workdirs: inner.workdirs.clone(),
                tasks: Mutex::default(),
                halting: AtomicBool::new(inner.halting.load(Ordering::SeqCst)),
                making: tokio::sync::Mutex::new(()),
                calls: Default::default(),
                stops: Default::default(),
                environment: inner.environment.clone(),
                environs: inner.environs.clone(),
                kept: Default::default(),
                compared: inner.compared.clone(),
                taking: inner.taking.clone(),
                set: inner.set.clone(),
                standing: Default::default(),
            }),
        })
    }

    /// The factory's own journal, whichever conductor is asked: what
    /// reads and follows the events of the whole factory asks it for
    /// every journal there is (`journal::every`).
    pub fn journal(&self) -> &Journal {
        &self.inner.factory
    }

    /// The journal this conductor keeps what it runs in.
    pub(crate) fn own(&self) -> &Journal {
        &self.inner.own
    }

    pub fn logs(&self) -> &Logs {
        &self.inner.logs
    }

    pub fn registry(&self) -> &Registry {
        &self.inner.registry
    }

    pub fn library(&self) -> &Arc<dyn Library> {
        &self.inner.library
    }

    /// What is told when an instance stops.
    pub(crate) fn stop_hooks(&self) -> Vec<StopHook> {
        self.inner.stop_hooks.lock().expect("engine lock").clone()
    }

    /// Registers a callback for when an instance stops.
    pub fn on_stop(&self, hook: StopHook) {
        self.inner.stop_hooks.lock().expect("engine lock").push(hook);
    }

    /// Installs the way to run actions on other devices.
    pub fn set_remote(&self, remote: Arc<dyn Remote>) {
        *self.inner.remote.lock().expect("engine lock") = Some(remote);
    }

    /// Installs the way to find a project's directory.
    pub fn set_workdirs(&self, workdirs: Workdirs) {
        *self.inner.workdirs.lock().expect("engine lock") = Some(workdirs);
    }

    /// The directory work for `project` happens in: the project's own, or
    /// the conductor's current directory when there is no project.
    pub fn workdir(&self, project: Option<&str>) -> std::path::PathBuf {
        let workdirs = self.inner.workdirs.lock().expect("engine lock").clone();
        workdirs
            .and_then(|resolve| resolve(project))
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    }

    /// Runs one action here for a step on another device, and returns how
    /// it went. Called by the gateway when the device that owns the run asks.
    pub async fn run_remote(&self, call: RemoteCall) -> ActionResult {
        // By the conductor that runs the project here; the factory's
        // own where this device has no such project.
        if let Ok(conductor) = self.of(call.project.as_deref()).await
            && !Arc::ptr_eq(&conductor.inner, &self.inner)
        {
            return Box::pin(conductor.run_remote(call)).await;
        }
        if blueprint::intrinsic(&call.action).is_some() {
            return Err(ActionError::new(format!(
                "`{}` is part of the blueprint language and happens where the blueprint runs; \
                 it cannot be sent to another device",
                call.action
            )));
        }
        let Some(handler) = self.registry().get(&call.action) else {
            let device = self.inner.own.device();
            return Err(ActionError::new(match self.registry().absent(&call.action, false) {
                Some(why) => format!("on device `{device}`, {why}"),
                None => format!("there is no action called `{}` on device `{device}`", call.action),
            }));
        };
        if let Some(capability) = &handler.spec().capability
            && !call.principal.can(capability)
        {
            return Err(ActionError::new(format!("{} may not use `{capability}`", call.principal.name)));
        }
        check_args(handler.spec(), &call.args)?;

        let step = call.origin.step.clone().unwrap_or_default();
        let run = call.origin.run.clone().unwrap_or_default();
        // Kept under a name of its own: this device has no such run.
        let log_id = sink::log_id(&format!("remote.{run}"), &step);
        let (sink, _matched) = Sink::new(
            self.inner.own.clone(),
            self.inner.logs.clone(),
            log_id,
            call.origin.clone(),
            Vec::new(),
        );
        let cx = ActionCx {
            inner: Arc::new(CxInner {
                engine: self.clone(),
                instance: String::new(),
                blueprint: String::new(),
                project: call.project,
                run: format!("remote.{run}"),
                step,
                attempt: call.attempt,
                principal: call.principal,
                cancel: call.cancel,
                env: call.env,
                sink: Arc::new(sink.forwarding(call.on_log)),
                labels: Labels::new(),
                element: None,
                secrets: call.secrets,
                kept: true,
                again: false,
                around: None,
                doing: format!("try {}", call.attempt),
                limit: None,
            }),
        };
        handler.run(cx, call.args).await
    }

    /// Whether every task this conductor started is gone.
    pub(crate) fn tasks_gone(&self) -> bool {
        self.inner.tasks.lock().expect("engine lock").iter().all(AbortHandle::is_finished)
    }

    pub(crate) fn remote(&self) -> Option<Arc<dyn Remote>> {
        self.inner.remote.lock().expect("engine lock").clone()
    }

    /// True once the conductor is shutting down. An action whose future is
    /// dropped while this is set must leave what it started running, so the
    /// next conductor can pick it up; dropped otherwise, it must clean up.
    pub fn is_halting(&self) -> bool {
        self.inner.halting.load(Ordering::SeqCst)
    }

    pub(crate) fn live(&self, id: &str) -> Option<Arc<Live>> {
        self.inner.live.lock().expect("engine lock").get(id).cloned()
    }

    pub(crate) fn register(&self, info: &InstanceInfo, blueprint: Arc<Blueprint>) -> Arc<Live> {
        let cancel = Cancel::new();
        let live = Arc::new(Live {
            id: info.id.clone(),
            work: Mutex::new(cancel.child()),
            cancel,
            owed: crate::owed::Hearing::of(&self.inner.owed, &blueprint),
            blueprint: Mutex::new(blueprint),
            meta: Mutex::new(Meta {
                blueprint: info.blueprint.clone(),
                revision: info.revision.clone(),
                params: info.params.clone(),
                state: info.state.clone(),
                principal: info.principal.clone(),
                project: info.project.clone(),
                parent: info.parent.clone(),
                key: info.key.clone(),
                target: info.target.clone(),
                holds: info.holds.clone(),
                labels: info.labels.clone(),
                heard: info.heard.clone(),
                environment: info.environment.clone(),
            }),
            apply_lock: Default::default(),
            generation: AtomicU64::new(0),
            previous: Mutex::new(info.previous.clone()),
            listening: Mutex::default(),
            look_below: AtomicU64::new(0),
            looks: Default::default(),
            carrying: Mutex::default(),
            setting: tokio::sync::Mutex::new(()),
            turn: Default::default(),
            held: Default::default(),
            given: AtomicU64::new(0),
        });
        live.given();
        self.inner.live.lock().expect("engine lock").insert(info.id.clone(), live.clone());
        live
    }

    pub(crate) fn forget(&self, id: &str) {
        self.inner.live.lock().expect("engine lock").remove(id);
    }

    /// Runs `work` as a task that [`Engine::halt`] will abort.
    pub fn spawn(&self, work: impl Future<Output = ()> + Send + 'static) {
        // Nothing is begun by a conductor that was halted: what would
        // have been is found written down by the one that starts next.
        if self.is_halting() {
            return;
        }
        let handle = tokio::spawn(crate::meets::carried(work)).abort_handle();
        let mut tasks = self.inner.tasks.lock().expect("engine lock");
        tasks.retain(|task| !task.is_finished());
        tasks.push(handle);
    }

    pub async fn emit(&self, topic: &str, source: Source, data: Value) {
        // The journal failing is not something a run can recover from, and it
        // must not turn a successful step into a failed one.
        let _ = self.inner.own.emit(topic, source, data).await;
    }

    /// What an instance says as it takes `definition`: said by whoever
    /// applied that, which the instance acts as from then.
    pub(crate) fn source_taking(&self, live: &Live, definition: &runs::Definition) -> Source {
        let mut source = self.instance_source(live);
        if let Some(principal) = &definition.principal {
            source.actor = Some(principal.actor());
        }
        source
    }

    pub(crate) fn instance_source(&self, live: &Live) -> Source {
        Source {
            instance: Some(live.id.clone()),
            actor: Some(live.meta().principal.actor()),
            ..Source::default()
        }
    }

    /// Starts an instance of a blueprint, or updates the one that already
    /// exists under the same parent and key.
    pub fn apply(&self, request: Apply) -> BoxFut<'_, Result<Applied>> {
        self.apply_read(request, None)
    }

    /// [`Engine::apply`]; `again` for an instance applied again as it
    /// is, which says when what is asked was read from it.
    pub(crate) fn apply_read(&self, request: Apply, again: Option<u64>) -> BoxFut<'_, Result<Applied>> {
        Box::pin(async move {
            self.not_while_held(&request.blueprint)?;
            // By the conductor that runs what is applied for the project.
            let conductor = self.of(request.project.as_deref()).await?;
            let applied = conductor.apply_from(request, None, None, again).await?;
            conductor.keeps(&applied.instance);
            if let Some(run) = &applied.run {
                conductor.keeps(run);
            }
            Ok(applied)
        })
    }

    /// The same, asked by something `instance` does that is no `call` or
    /// `spawn` (an action that applies, a text attached): the project's own
    /// texts are those `instance` may run, and nothing is vouched for.
    pub fn apply_within(&self, request: Apply, instance: &str) -> BoxFut<'_, Result<Applied>> {
        let instance = instance.to_string();
        Box::pin(async move {
            let conductor = self.of(request.project.as_deref()).await?;
            // What an instance may run is asked of the conductor that
            // keeps it, which is the one that runs its project.
            if !Arc::ptr_eq(&conductor.inner, &self.keeper(&instance).await.inner) {
                return Err(Error::invalid(format!(
                    "`{}` was not applied: the instance `{instance}` that asked is not of the project it was \
                     asked for, and an instance applies only for its own",
                    request.blueprint
                )));
            }
            conductor.apply_from(request, None, Some(instance), None).await
        })
    }

    /// The same, for a `call` or `spawn` of a running instance when
    /// `caller` says which step it is, and which of the two: see
    /// [`crate::reach`] for the text it is given, and [`crate::update`]
    /// for what becomes of a child that is there, where
    /// [`Engine::apply_from`] is, which asks this.
    pub(crate) fn apply_once(
        &self,
        request: Apply,
        caller: Option<(Source, Asker)>,
        within: Option<String>,
        again: Option<u64>,
    ) -> BoxFut<'_, Result<Applied>> {
        Box::pin(async move {
            request.principal.require("blueprints:run")?;
            let (caller, asker) = match caller {
                Some((step, asker)) => (Some(step), asker),
                None => (None, Asker::Person),
            };
            // Whoever asks vouches for the project's own texts as they
            // stand; what an instance does is held to what was vouched for.
            let asked = (caller.is_none(), within.clone());
            let request_take = request.take;
            // Somebody asking, and no instance on its own behalf: what is
            // reached is pinned for the instance.
            let person = caller.is_none() && within.is_none();
            // Noted before the text is read: one that is there may be
            // given another before this has its turn ([`crate::update`]).
            let at = again.unwrap_or_else(crate::update::mark);
            let read = self.read(&request, caller.clone(), within.as_deref()).await?;
            let project = request.project.as_deref();
            let key = request.key.clone().unwrap_or_else(|| read.blueprint.name.clone());

            // Looking and making are one thing, done by one asker at a
            // time. Two handlers that hear one event and both spawn the
            // child for it would otherwise each find none and each make
            // one.
            let making = self.inner.making.lock().await;
            if let Some(existing) =
                self.inner.store.live_instance_by_key(request.parent.as_deref(), &key, project).await?
            {
                drop(making);
                if let Some(asked) =
                    request.environment.as_deref().filter(|asked| *asked != existing.environment)
                {
                    return Err(Error::conflict(environments::held::not_another(
                        &key,
                        &existing.environment,
                        asked,
                    )));
                }
                self.held_naming(&request, within.as_deref(), &existing, caller.as_ref()).await?;
                let read = (at, again.is_some(), read);
                return self.apply_over(existing, request, read, (caller, asker), within).await;
            }
            let crate::reach::Read { source, blueprint, reaches, params } = read;
            let revision = blueprint::revision(&source);
            let parent = request.parent.as_deref().or(within.as_deref());
            let (environment, enters) =
                self.environment_for(&request, parent, person && request.parent.is_none()).await?;
            let seen = self.environment_view(project, &environment, &request.principal);

            let now = self.inner.own.now_ms();
            let info = InstanceInfo {
                id: self.inner.own.new_id("in"),
                blueprint: blueprint.name.clone(),
                revision,
                source,
                state: initial_state(self, &blueprint, (&params, seen), &Fields::new())?,
                params,
                status: InstanceStatus::Starting,
                parent: request.parent,
                key,
                target: request.target,
                project: request.project,
                principal: request.principal,
                holds: Vec::new(),
                error: None,
                created: now,
                updated: now,
                previous: None,
                labels: request.labels,
                heard: crate::whose::taken(),
                environment,
            };
            // What it is started under may be stopping as this is asked:
            // then it is not started (`Store::insert_instance`).
            let stopped = || {
                Error::conflict(format!(
                    "`{}` was not started: what it was to be started under was stopped as it was asked for",
                    info.blueprint
                ))
            };
            let under = match info.parent.as_deref() {
                None => None,
                Some(parent) => match self.live(parent) {
                    Some(parent) => Some(parent.cancel.clone()),
                    // Stopped, and put away since: a step of its that was
                    // still on its way here finds nothing to ask under.
                    None => return Err(stopped()),
                },
            };
            // Written down and said to have started as one write.
            let source = Source {
                instance: Some(info.id.clone()),
                actor: Some(info.principal.actor()),
                ..Source::default()
            };
            // Under an environment, held until its row is written: it is not
            // taken away in between (`environments::held::Held::holding`).
            let environs = self.hold_made(&info, enters, caller.as_ref()).await?;
            let started = to_say(
                "instance.started",
                source,
                json!({
                    "blueprint": info.blueprint,
                    "key": info.key,
                    "parent": info.parent,
                    "project": info.project,
                    "revision": info.revision,
                }),
            );
            let inserted =
                self.inner.store.insert_instance(info.clone(), under.clone(), vec![started]).await?;
            drop(environs);
            if !inserted {
                return Err(stopped());
            }
            let live = self.register(&info, blueprint.clone());
            // Whoever asks for this key next finds it, and waits here until
            // its first run is written down: so that they find its steps
            // under way, and not an instance that has yet to begin, which
            // they would begin a second time ([`Engine::reapply`]).
            let _applying = live.apply_lock.lock().await;
            drop(making);
            // Written down just before the stopping began, it was counted
            // among the children and stopped where it was written: before
            // it was known here, so that going on from this line would
            // start it again over that. Nothing is waited for between
            // making it known and looking, so it is one or the other.
            if under.as_ref().is_some_and(Cancel::is_cancelled) {
                let _ = self.stop(&info.id).await;
                live.cancel.cancel();
                self.forget(&info.id);
                return Err(stopped());
            }

            let top = (info.parent.is_none(), (info.blueprint.as_str(), request_take));
            let vouched = self.answered(&info.id, info.project.as_deref(), top, &info.principal, asked).await;
            if person {
                self.pin(&info.id, info.project.as_deref(), &reaches, None).await;
            }
            let generation = live.generation.load(Ordering::SeqCst);
            let run = self.begin(&live, generation).await?;
            Ok(Applied {
                instance: info.id,
                run: Some(run),
                created: true,
                changed: true,
                took: None,
                vouched,
                reached: Default::default(),
            })
        })
    }

    /// Stops driving everything, at once, without recording anything further.
    /// What was in progress stays marked as in progress, to be picked up by
    /// [`Engine::resume`]. This is what a clean shutdown and a crash have in
    /// common, which is the point: there is one recovery path, and it is
    /// exercised every time the conductor restarts.
    pub fn halt(&self) {
        self.halt_own();
        // The factory's own takes the others with it, and lets go of them.
        if self.inner.project.is_none() {
            self.let_go();
        }
    }

    /// Halts this conductor and no other.
    pub(crate) fn halt_own(&self) {
        self.inner.halting.store(true, Ordering::SeqCst);
        // Kept, for whoever waits until each is gone ([`Engine::halted_one`]).
        for task in self.inner.tasks.lock().expect("engine lock").iter() {
            task.abort();
        }
        self.inner.live.lock().expect("engine lock").clear();
        // Whoever waits for a run of its to end is let go: no run ends
        // here any more, and whoever waits on another's start looks again.
        self.inner.waiters.lock().expect("engine lock").clear();
        self.inner.set.told.notify_waiters();
    }

    /// Waits until nothing a halted conductor started is still about.
    ///
    /// [`Engine::halt`] stops each task when it next comes to be run, and a
    /// step's task only once its run's has been: for a moment afterwards
    /// something of the old conductor can still be woken, by a wait that
    /// ends or a lock that is let go, and it would go on and write down
    /// what it did as if nothing had happened. A program that exits after
    /// halting never sees that moment. One that goes on, as a test does
    /// that starts another conductor on the same journal to see what it
    /// picks up, waits here first, so that what it then lets happen
    /// happens to the new conductor and to nothing else.
    pub async fn halted(&self) {
        // Everything it started holds it. Bounded, in case something else does.
        for _ in 0..100_000 {
            if Arc::strong_count(&self.inner) <= 1 && !self.still_held() {
                return;
            }
            tokio::task::yield_now().await;
        }
    }

    pub async fn instance(&self, id: &str) -> Result<InstanceInfo> {
        let Some(mut info) = self.inner.store.instance(id).await? else {
            // Not this conductor's: whichever keeps it answers.
            return match self.elsewhere(id).await {
                Some(other) => Box::pin(other.instance(id)).await,
                None => Err(Error::not_found(format!("there is no instance `{id}`"))),
            };
        };
        // One that keeps nothing has its state in hand and not written
        // down: it is shown as it is, for as long as it lives.
        if let Some(live) = self.live(id).filter(|live| !live.blueprint().durable) {
            info.state = live.meta().state;
        }
        Ok(info)
    }

    pub async fn instances(&self, filter: InstanceFilter) -> Result<Vec<InstanceInfo>> {
        crate::routed::instances(self, filter).await
    }

    /// A run with its steps.
    pub async fn runs(&self, instance: &str, limit: usize) -> Result<Vec<RunInfo>> {
        self.keeper(instance).await.inner.store.runs(instance, limit).await
    }

    /// The definitions an instance has been given, the latest first.
    pub async fn versions(&self, instance: &str, limit: usize) -> Result<Vec<crate::Version>> {
        self.keeper(instance).await.inner.store.versions(instance, limit).await
    }

    pub async fn recent_runs(&self, limit: usize, project: Option<String>) -> Result<Vec<RunInfo>> {
        crate::routed::recent_runs(self, limit, project).await
    }

    /// Lines from a step's log after line `after`.
    pub fn step_log(&self, run: &str, step: &str, after: u64, limit: usize) -> Vec<LogLine> {
        self.inner.logs.read("runs", &sink::log_id(run, step), after, limit)
    }

    pub(crate) async fn save_checkpoint(&self, run: &str, step: &str, saved: Value) -> Result<()> {
        self.inner.store.save_checkpoint(run, step, saved).await
    }

    pub(crate) async fn set_hold(&self, instance: &str, what: &str, held: bool) -> Result<()> {
        if instance.is_empty() {
            // Work done for a step on another device has no instance here.
            return Err(Error::conflict(
                "this action leaves something running, which needs an instance on this device to own it; \
                 apply a blueprint here instead (`do: blueprint.apply` with the same `device:`)",
            ));
        }
        let Some(live) = self.live(instance) else {
            return Err(Error::conflict(format!("instance `{instance}` is not running")));
        };
        let holds = {
            let mut meta = live.meta.lock().expect("live lock");
            meta.holds.retain(|hold| hold != what);
            if held {
                meta.holds.push(what.to_string());
            }
            meta.holds.clone()
        };
        self.inner.store.set_instance_holds(instance, holds, self.inner.own.now_ms()).await
    }

    /// What every expression in an instance can see, before steps and events
    /// are added.
    pub(crate) fn base_scope(&self, live: &Live) -> Fields {
        let meta = live.meta();
        kernel::fields(json!({
            "params": meta.params,
            "state": meta.state,
            "instance": {
                "id": live.id,
                "name": meta.blueprint,
                "key": meta.key,
                "parent": meta.parent,
                "project": meta.project,
                "device": self.inner.own.device(),
            },
            "target": meta.target.map(|id| json!({ "id": id })),
            "profile": crate::profile::view(self),
            "environment": self.environment_view(meta.project.as_deref(), &meta.environment, &meta.principal),
        }))
    }

    /// Whether `candidate` is `root` or one of its descendants.
    ///
    /// An instance that is not in hand is looked up where it is written
    /// down. One that fails, finishes or is stopped says so and is then
    /// let go of, and a handler gets to what it said a moment later, or
    /// after the conductor has started again: it was below its parent
    /// then, and what it said as it ended is still its parent's to hear.
    pub(crate) async fn is_within(&self, candidate: &str, root: &str) -> bool {
        let mut current = Some(candidate.to_string());
        for _ in 0..64 {
            let Some(id) = current else { return false };
            if id == root {
                return true;
            }
            let in_hand = self.live(&id).map(|live| live.meta.lock().expect("live lock").parent.clone());
            current = match in_hand {
                Some(parent) => parent,
                None => self.inner.store.parent_of(&id).await.ok().flatten(),
            };
        }
        false
    }
}

/// The state an instance starts with: declared initial values, overridden by
/// whatever it already had.
pub(crate) fn initial_state(
    engine: &Engine,
    blueprint: &Blueprint,
    (params, environment): (&Fields, Value),
    existing: &Fields,
) -> Result<Fields> {
    let scope = kernel::fields(json!({
        "params": params,
        "profile": crate::profile::view(engine),
        "environment": environment,
    }));
    let mut state = Fields::new();
    for (key, initial) in &blueprint.state {
        let value = match existing.get(key) {
            Some(kept) => kept.clone(),
            None => template::render(initial, &scope)
                .map_err(|err| Error::invalid(format!("state `{key}`: {err}")))?,
        };
        state.insert(key.clone(), value);
    }
    Ok(state)
}
