//! Actions: the verbs a blueprint can `do`.
//!
//! The conductor knows how to run a graph; it does not know how to build a
//! crate, start a service or talk to an agent. Those are actions, registered
//! by the crates above this one. That inversion is the seam the whole system
//! hangs on: adding a capability is adding an action, and nothing here needs
//! to change or recompile for it.

use std::collections::BTreeMap;
use std::sync::Arc;

use blueprint::{ActionList, ActionSpec, Catalog, EventList, EventSpec, Labels};
use kernel::{BoxFut, Cancel, Clock, Fields, Principal, Source};
use serde_json::Value;

use crate::engine::Engine;
use crate::sink::Sink;

/// Why an action did not succeed.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionError {
    pub message: String,
    /// Anything structured worth showing or branching on, e.g. an exit code.
    pub data: Value,
    pub kind: FailureKind,
}

/// "It failed", "it ran out of time" and "it was told to stop" are different
/// facts and are kept apart all the way to the journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureKind {
    Failed,
    TimedOut,
    Cancelled,
}

impl ActionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into(), data: Value::Null, kind: FailureKind::Failed }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    pub fn cancelled() -> Self {
        Self { message: "cancelled".into(), data: Value::Null, kind: FailureKind::Cancelled }
    }

    pub fn timed_out(after: &str) -> Self {
        Self { message: format!("timed out after {after}"), data: Value::Null, kind: FailureKind::TimedOut }
    }
}

impl ActionError {
    /// Whether this is a database saying that it has ended: what was
    /// asked of it and not done when its thread went is refused in
    /// these words (`journal::Db`), to whoever is still there to hear.
    /// That is the daemon going, where this conductor's journal has
    /// shut or it was halted, and nothing the action did.
    pub(crate) fn gone(&self) -> bool {
        self.message.ends_with("the database thread has stopped")
    }
}

impl From<kernel::Error> for ActionError {
    fn from(err: kernel::Error) -> Self {
        ActionError::new(err.message)
    }
}

impl From<String> for ActionError {
    fn from(message: String) -> Self {
        ActionError::new(message)
    }
}

impl From<&str> for ActionError {
    fn from(message: &str) -> Self {
        ActionError::new(message)
    }
}

pub type ActionResult = Result<Value, ActionError>;

/// One action to run here for a step of a run on another device.
///
/// There is no instance or run of this device's behind it: the run, its
/// retries, its timeout and its hooks all belong to the device that asked.
/// This device only does the work and says how it went.
pub struct RemoteCall {
    pub action: String,
    pub args: Fields,
    /// On whose authority, as vouched for by the device that asked.
    pub principal: Principal,
    pub project: Option<String>,
    /// The step this is for, on the device that asked.
    pub origin: Source,
    pub attempt: u32,
    pub env: BTreeMap<String, String>,
    /// The secrets its program is to be handed, each by the variable it is
    /// to be in: names of what this device keeps.
    pub secrets: BTreeMap<String, String>,
    pub cancel: Cancel,
    /// Receives every line the action writes, to send back.
    pub on_log: crate::sink::Forward,
}

/// Everything an action is given about where it is running.
#[derive(Clone)]
pub struct ActionCx {
    pub(crate) inner: Arc<CxInner>,
}

pub(crate) struct CxInner {
    pub engine: Engine,
    pub instance: String,
    pub blueprint: String,
    pub project: Option<String>,
    pub run: String,
    pub step: String,
    pub attempt: u32,
    pub principal: Principal,
    pub cancel: Cancel,
    pub env: BTreeMap<String, String>,
    pub sink: Arc<Sink>,
    pub labels: Labels,
    /// Set when the step is done once for each of a list: which element
    /// this is.
    pub element: Option<Element>,
    /// The secrets the step is to be handed, each by the variable it is
    /// handed in. The names: a value is read by what starts a program, as
    /// it starts it, and is never here.
    pub secrets: BTreeMap<String, String>,
    /// Whether what the step does is written down ([`crate::run::RunCx::kept`]).
    pub kept: bool,
    /// Whether the step was under way when the conductor stopped, and is
    /// being done again with nothing of its own saved.
    pub again: bool,
    /// Which entry of what is done around its step, or as its run ends,
    /// this is (`on_fail 0`); nothing, for a step's own action.
    pub around: Option<String>,
    /// Which of everything done under its step's name in its run this is.
    pub doing: String,
    /// How long the step is given, in milliseconds, where it says.
    pub limit: Option<u64>,
}

/// Which of the things done under its step's name a call is: which entry
/// of what is done around the step, where it is one, and the whole of it
/// ([`ActionCx::doing`]): inside which steps done as one, at which try,
/// for which of a list, as which entry.
pub(crate) fn doing(under: &str, call: &crate::step::Call<'_>) -> (Option<String>, String) {
    let around = match (call.list, call.around) {
        (Some((list, at)), _) => Some(format!("{list} {at}")),
        (None, true) => Some("a rule".to_string()),
        (None, false) => None,
    };
    let element = call.element.as_ref().map_or(String::new(), |element| format!(" of {}", element.at));
    let entry = around.as_ref().map_or(String::new(), |entry| format!(", {entry}"));
    let doing = format!("{under}try {}{element}{entry}", call.attempt);
    (around, doing)
}

/// Which element of a `for_each` an action is being done for, and the list
/// the loop is over. A checkpoint the action saves is written down with
/// both, so that a conductor that starts again knows the list the loop
/// began with and whose checkpoint it is ([`Left`]).
#[derive(Clone)]
pub(crate) struct Element {
    pub items: Arc<Vec<Value>>,
    pub at: usize,
}

impl Element {
    /// What is saved for a loop that is at this element: nothing more when
    /// `saved` is nothing, which is how a loop says what it is over as it
    /// begins.
    pub fn around(items: &[Value], at: Option<(usize, Value)>) -> Value {
        match at {
            Some((at, saved)) => serde_json::json!({ "each": { "items": items, "at": at, "saved": saved } }),
            None => serde_json::json!({ "each": { "items": items } }),
        }
    }
}

/// What a `for_each` step had saved when the conductor stopped.
pub(crate) struct Left {
    /// The list the loop began with.
    pub items: Vec<Value>,
    /// Where the action of the element it was at had saved a checkpoint:
    /// which element that was, and what it saved.
    pub own: Option<(usize, Value)>,
}

impl Left {
    pub fn of(saved: &Value) -> Option<Left> {
        let each = saved.get("each")?;
        let items = each.get("items")?.as_array()?.clone();
        let at = each.get("at").and_then(Value::as_u64).map(|at| at as usize);
        Some(Left { items, own: at.zip(each.get("saved").cloned()) })
    }
}

impl ActionCx {
    pub fn engine(&self) -> &Engine {
        &self.inner.engine
    }

    pub fn instance(&self) -> &str {
        &self.inner.instance
    }

    pub fn blueprint(&self) -> &str {
        &self.inner.blueprint
    }

    /// The project this instance belongs to, if any.
    pub fn project(&self) -> Option<&str> {
        self.inner.project.as_deref()
    }

    pub fn run(&self) -> &str {
        &self.inner.run
    }

    pub fn step(&self) -> &str {
        &self.inner.step
    }

    /// Which try this is, starting at 1.
    pub fn attempt(&self) -> u32 {
        self.inner.attempt
    }

    /// Whether this step was under way when the conductor stopped and is
    /// being done again from its start. An action that says it is the
    /// same done twice is asked again then, and one that leaves something
    /// behind each time it is done (a message, say) looks here for what
    /// it left before it leaves another.
    pub fn again(&self) -> bool {
        self.inner.again
    }

    /// Which entry of what is done around its step, or as its run ends,
    /// this is being done as (`on_fail 0`: the first of what is done when
    /// it fails); nothing, for a step's own action.
    pub fn around(&self) -> Option<&str> {
        self.inner.around.as_deref()
    }

    /// Which of the things done under its step's name in its run this is.
    /// Several are: the step's action, at each try of it; each entry of
    /// each list of what is done as it starts and ends; and, for a step
    /// inside steps done as one, all of that again for each try of those
    /// and each element of their list. One that leaves something behind
    /// and looks for it when it is done again ([`ActionCx::again`]) keeps
    /// this with what it leaves and looks by it, beside the run and the
    /// step, or it finds what another of them left and leaves nothing.
    pub fn doing(&self) -> &str {
        &self.inner.doing
    }

    /// When the try of its step this is being done in began, as the
    /// step's row says: for an action done again ([`ActionCx::again`])
    /// that looks for what it left among what was said since, by the
    /// time, in a journal its run's own place means nothing in. Nothing
    /// where the row says nothing of it (a program from before wrote it).
    /// It reads the row, so it is asked only where the step is done again.
    pub async fn began_at(&self) -> Option<i64> {
        let store = &self.inner.engine.inner.store;
        Some(store.step_began(&self.inner.run, &self.inner.step).await.ok()??.1)
    }

    /// Which element of the list this is being done for, where the step
    /// is done once for each of one.
    pub fn at(&self) -> Option<usize> {
        self.inner.element.as_ref().map(|element| element.at)
    }

    /// On whose authority the action runs.
    pub fn principal(&self) -> &Principal {
        &self.inner.principal
    }

    /// Raised when the step is cancelled or times out. Long-running actions
    /// must watch it and clean up; a turn that ends leaves nothing running.
    pub fn cancel(&self) -> &Cancel {
        &self.inner.cancel
    }

    /// What the step's measurements also carry: the labels of its instance
    /// and of any task it is in. An action that has something measured on
    /// the step's behalf (an agent's turn) hands these on to it.
    pub fn labels(&self) -> &Labels {
        &self.inner.labels
    }

    /// The secrets the blueprint and the step say a program is to be
    /// handed, each by the variable it is handed in: `{ "STRIPE_KEY":
    /// "stripe" }`. Whatever starts a program reads each as it starts it.
    pub fn secrets(&self) -> &BTreeMap<String, String> {
        &self.inner.secrets
    }

    /// Environment declared by the blueprint and the step, already filled in.
    pub fn env(&self) -> &BTreeMap<String, String> {
        &self.inner.env
    }

    /// The directory this step works in: its project's, unless the
    /// blueprint is not part of one.
    pub fn workdir(&self) -> std::path::PathBuf {
        self.inner.engine.workdir(self.project())
    }

    pub fn clock(&self) -> &Arc<dyn Clock> {
        self.inner.engine.own().clock()
    }

    /// Where what is said of this step for the runner's own part comes
    /// from: what its lines are said with, which takes what the event
    /// that set its run off took (`RunCx::about`). For whoever has the
    /// step done somewhere else, where its lines are said too.
    pub fn about(&self) -> Source {
        self.inner.sink.said_from().clone()
    }

    /// Where events from this step come from: what an action says for
    /// the blueprint that asked it, as whoever applied that.
    pub fn source(&self) -> Source {
        Source {
            device: None,
            instance: Some(self.inner.instance.clone()).filter(|instance| !instance.is_empty()),
            run: Some(self.inner.run.clone()).filter(|run| !run.is_empty()),
            step: Some(self.inner.step.clone()).filter(|step| !step.is_empty()),
            actor: Some(self.inner.principal.actor()),
            takes: None,
        }
    }

    /// Writes a line to the step's log. `stream` is `o`, `e` or `s`.
    pub fn log(&self, stream: char, text: &str) {
        self.inner.sink.line(stream, text);
    }

    /// Publishes an event attributed to this step.
    pub async fn emit(&self, topic: &str, data: Value) -> kernel::Result<()> {
        // Kept where what it names is kept ([`Engine::kept_in`]).
        self.inner.engine.kept_in(&data).emit(topic, self.source(), data).await.map(|_| ())
    }

    /// How long the step is given, where it says (`timeout`). The runner
    /// holds the step to it while it runs; an action whose work goes on
    /// while no runner is there holds what it started to the same.
    pub fn limit(&self) -> Option<std::time::Duration> {
        self.inner.limit.map(std::time::Duration::from_millis)
    }

    /// Records what is needed to pick this step up again if the conductor
    /// restarts while it is running. Without a checkpoint an interrupted step
    /// is run again from the start (if the action is idempotent) or failed.
    pub async fn checkpoint(&self, saved: Value) -> kernel::Result<()> {
        // Nothing picks up a step of an instance that keeps nothing. And
        // what is done around a step, or by a rule that watches it, is
        // never picked up where it was: it is done again from its start.
        // The step's record is where its own action's place is kept, and
        // one written there by anything else would be taken for it.
        if !self.inner.kept || self.inner.around.is_some() {
            return Ok(());
        }
        let saved = match &self.inner.element {
            Some(element) => Element::around(&element.items, Some((element.at, saved))),
            None => saved,
        };
        self.inner.engine.save_checkpoint(&self.inner.run, &self.inner.step, saved).await
    }

    /// Marks the instance as owning something that outlives this run, so it
    /// stays alive after its steps finish. Release it with [`ActionCx::release`].
    pub async fn hold(&self, what: &str) -> kernel::Result<()> {
        self.inner.engine.set_hold(&self.inner.instance, what, true).await
    }

    pub async fn release(&self, what: &str) -> kernel::Result<()> {
        self.inner.engine.set_hold(&self.inner.instance, what, false).await
    }

    /// Completes after `duration`, or fails early if the step is cancelled.
    pub async fn sleep(&self, duration: std::time::Duration) -> Result<(), ActionError> {
        tokio::select! {
            () = self.clock().sleep(duration) => Ok(()),
            () = self.inner.cancel.cancelled() => Err(ActionError::cancelled()),
        }
    }
}

pub trait Action: Send + Sync + 'static {
    fn spec(&self) -> &ActionSpec;

    fn run(&self, cx: ActionCx, args: Fields) -> BoxFut<'static, ActionResult>;

    /// Called instead of [`Action::run`] when the conductor restarted while
    /// this step was running and it had saved a checkpoint.
    fn resume(&self, cx: ActionCx, args: Fields, saved: Value) -> BoxFut<'static, ActionResult> {
        let _ = saved;
        self.run(cx, args)
    }
}

struct FnAction<F> {
    spec: ActionSpec,
    run: F,
}

impl<F> Action for FnAction<F>
where
    F: Fn(ActionCx, Fields) -> BoxFut<'static, ActionResult> + Send + Sync + 'static,
{
    fn spec(&self) -> &ActionSpec {
        &self.spec
    }

    fn run(&self, cx: ActionCx, args: Fields) -> BoxFut<'static, ActionResult> {
        (self.run)(cx, args)
    }
}

/// Actions that are not fixed when the conductor starts.
///
/// What a device can do is mostly settled by the crates built into it, and
/// is registered once. Some of it is not: an extension package is installed
/// and removed while the conductor runs, and what it brings has to be there
/// from that moment and gone from the other. So such actions are not copied
/// into the registry; whoever has them is asked each time.
pub trait Brought: Send + Sync + 'static {
    /// The action of this name, if it is here now.
    fn action(&self, name: &str) -> Option<Arc<dyn Action>>;

    /// The specs of every action that is here now.
    fn specs(&self) -> Vec<ActionSpec>;

    /// What to say of an action that is not here, when more is known than
    /// that: that it comes with something which is not installed, and how
    /// to get it. `once_here` is said of a step of a blueprint that was
    /// checked when it was applied: the action was here then.
    fn absent(&self, name: &str, once_here: bool) -> Option<String> {
        let _ = (name, once_here);
        None
    }

    /// The events that are emitted while these actions are here: what a
    /// package says happens, as it says what can be done.
    fn events(&self) -> Vec<EventSpec> {
        Vec::new()
    }
}

/// The actions available to blueprints on this device.
#[derive(Clone, Default)]
pub struct Registry {
    actions: BTreeMap<String, Arc<dyn Action>>,
    specs: ActionList,
    brought: Vec<Arc<dyn Brought>>,
    /// The events the crates built in say they emit.
    events: Vec<EventSpec>,
    /// Whether that is all of them: see [`Registry::emits_no_more`].
    whole: bool,
}

impl Registry {
    /// A registry with what the conductor itself does and emits.
    pub fn new() -> Self {
        Self { events: crate::events::events(), ..Self::default() }
    }

    /// Says what is emitted, beside the actions that were added: each
    /// crate, once, for the events that are its own.
    pub fn emits(&mut self, events: Vec<EventSpec>) -> &mut Self {
        self.events.extend(events);
        self
    }

    /// Says that every crate has now said what it emits, so that a topic
    /// of this program's own that is not among them is a mistake in the
    /// blueprint that names it. Without this nothing is held to the events:
    /// a registry put together for a test knows only some.
    pub fn emits_no_more(&mut self) -> &mut Self {
        self.whole = true;
        self
    }

    /// Whether [`Registry::emits_no_more`] was said.
    pub fn knows_events(&self) -> bool {
        self.whole
    }

    /// The events built in, for checking blueprints. Those that come and
    /// go with a package are not among them: see [`Registry::brought_events`].
    pub fn own_events(&self) -> &[EventSpec] {
        &self.events
    }

    /// The events that come and go, as they are now.
    pub fn brought_events(&self) -> Vec<EventSpec> {
        self.brought.iter().flat_map(|layer| layer.events()).collect()
    }

    /// Every event there is to hear on this device now.
    pub fn events(&self) -> EventList {
        let mut all = self.events.clone();
        all.extend(self.brought_events());
        EventList::new(all)
    }

    pub fn add(&mut self, action: impl Action) -> &mut Self {
        let spec = action.spec().clone();
        self.actions.insert(spec.name.clone(), Arc::new(action));
        self.specs.push(spec);
        self
    }

    /// Registers an action written as a function.
    pub fn add_fn<F>(&mut self, spec: ActionSpec, run: F) -> &mut Self
    where
        F: Fn(ActionCx, Fields) -> BoxFut<'static, ActionResult> + Send + Sync + 'static,
    {
        self.add(FnAction { spec, run })
    }

    /// Adds actions that come and go, asked for each time after the ones
    /// registered here: a name registered here is never theirs.
    pub fn also(&mut self, brought: Arc<dyn Brought>) -> &mut Self {
        self.brought.push(brought);
        self
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Action>> {
        self.actions.get(name).cloned().or_else(|| self.brought.iter().find_map(|layer| layer.action(name)))
    }

    /// What there is to say of a name [`Registry::get`] did not find.
    pub fn absent(&self, name: &str, once_here: bool) -> Option<String> {
        self.brought.iter().find_map(|layer| layer.absent(name, once_here))
    }

    /// The specs of the actions registered here, for checking blueprints.
    /// Those that come and go are not among them: see [`Registry::brought`].
    pub fn specs(&self) -> &ActionList {
        &self.specs
    }

    /// The specs of the actions that come and go, as they are now.
    pub fn brought(&self) -> ActionList {
        let taken = |spec: &ActionSpec| self.actions.contains_key(&spec.name);
        ActionList::new(
            self.brought.iter().flat_map(|layer| layer.specs()).filter(|spec| !taken(spec)).collect(),
        )
    }

    /// Every action a blueprint can use here now: the ones built into the
    /// language, the ones registered, and the ones that come and go.
    pub fn all_specs(&self) -> Vec<ActionSpec> {
        let mut specs = blueprint::intrinsics();
        specs.extend(self.specs.specs().iter().cloned());
        specs.extend(self.brought().specs().iter().cloned());
        specs
    }
}

/// Checks filled-in arguments against a spec (`texts::ran::checked`).
pub fn check_args(spec: &ActionSpec, args: &Fields) -> Result<(), ActionError> {
    texts::ran::checked(spec, args).map_err(ActionError::new)
}

/// Lets the registry stand in wherever a blueprint needs checking.
impl Catalog for Registry {
    fn action(&self, name: &str) -> Option<&ActionSpec> {
        self.specs.action(name)
    }

    fn action_names(&self) -> Vec<&str> {
        self.specs.action_names()
    }

    fn events(&self) -> Vec<&EventSpec> {
        self.events.iter().collect()
    }

    fn knows_events(&self) -> bool {
        self.whole
    }
}
