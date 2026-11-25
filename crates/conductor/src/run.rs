//! Running one flow: the scheduler.
//!
//! A flow is a graph. A step starts as soon as everything it needs has
//! settled, so independent branches run side by side without being told to.
//! The first step that fails (and is not tolerated) stops the run: nothing
//! new starts, what is running is cancelled, and what never started is
//! marked cancelled, so the record shows exactly how far the run got.
//!
//! The steps of a task are scheduled by the same loop as the run's own
//! ([`run_steps`]), so a graph means one thing wherever it is written.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use blueprint::{Blueprint, Flow, Labels, template, within};
use kernel::{Cancel, Fields, Principal, Source};
use runs::{Ending, RunStatus, Say, StepInfo, StepStatus, to_say};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::task::JoinSet;

use crate::engine::{Engine, Live};
use crate::held::{Quiet, holds};
use crate::hooks::Around;
use crate::step::{self, End, StepResult};
use crate::{carry, labels};

/// Steps of one flow that may run at once, unless the blueprint says otherwise.
const DEFAULT_PARALLEL: usize = 8;

/// What a run needs to know about itself.
pub(crate) struct RunCx {
    pub engine: Engine,
    pub live: Arc<Live>,
    pub blueprint: Arc<Blueprint>,
    /// On whose authority the run acts: whoever applied the text it runs.
    /// Taken with the text when the run begins and kept to its end, so what
    /// is left of a run when its instance is applied over by somebody else
    /// never acts as them.
    pub principal: Principal,
    pub run_id: String,
    /// `main`, `on.<handler>`, or `on_update`.
    pub flow_id: String,
    /// The event that fired the handler, or `null`.
    pub event: Fired,
    pub cancel: Cancel,
    /// For a main run: the generation of the instance it speaks for. When it
    /// finishes it settles the instance, unless it has been superseded.
    pub generation: Option<u64>,
    /// What everything measured in this graph of steps also carries: the
    /// instance's labels, and inside a task those the task added.
    pub labels: Labels,
    /// What these steps are inside of: each task around them, with which
    /// try of it this is and which element of its list. Nothing, for a
    /// blueprint's own steps. A step inside a task has the same name each
    /// time the task is done, and this is what tells one time from another
    /// ([`crate::ActionCx::doing`]).
    pub under: String,
}

impl Engine {
    /// A run with its steps, in the order they began. One that is still
    /// going and whose instance keeps nothing as it goes has its steps as
    /// they stand now, from where they are held.
    pub async fn run(&self, id: &str) -> kernel::Result<crate::RunInfo> {
        match self.run_here(id).await? {
            Some(run) => Ok(run),
            // Another conductor's: whichever keeps it answers.
            None => match self.elsewhere_run(id).await {
                Some(other) => Box::pin(other.run(id)).await,
                None => Err(kernel::Error::not_found(format!("there is no run `{id}`"))),
            },
        }
    }

    /// The run `id` where this conductor keeps it.
    pub(crate) async fn run_here(&self, id: &str) -> kernel::Result<Option<crate::RunInfo>> {
        // What is held of it and not yet written: all its steps, for one
        // that keeps nothing as it goes; those passed over since it last
        // wrote, for one that does. Read before what is written is: a step
        // written in between is then in both, and what is written is the
        // later word on it. The other way round it would be in neither.
        // Whose it is is not known until it is read, so every instance
        // is asked; this is somebody looking at one run, not the runner.
        let held: Vec<StepInfo> = {
            let live = self.inner.live.lock().expect("engine lock");
            live.values().flat_map(|live| live.held.of(id)).collect()
        };
        let Some(mut run) = self.inner.store.run(id).await? else { return Ok(None) };
        // What is held and not written began after everything that is
        // written: a step that is passed over is held only while none of
        // its run is at work, and the next to begin writes it first.
        let unwritten: Vec<StepInfo> =
            held.into_iter().filter(|one| !run.steps.iter().any(|step| step.step == one.step)).collect();
        run.steps.extend(unwritten);
        Ok(Some(run))
    }
}

/// What hearing the event that set a run off takes, from the event as
/// its run keeps it (`RunInfo::event`, whole, so a run taken up after a
/// restart reads what it read before): what is said of the run takes
/// the same. Nothing for a run no event set off (its `event` is nothing,
/// or what a handler made up for a timer, which names no source) and for
/// one whose event took nothing. Where what is kept cannot be read as
/// where an event came from, that is not taken for nothing: it is said
/// as kept to the members of nothing, which is nobody.
///
/// It is read to be said again and never to decide who is told.
pub(crate) fn took(event: &Value) -> Option<String> {
    let said = event.get("source").filter(|said| !said.is_null())?;
    match Source::deserialize(said) {
        Ok(source) => source.takes,
        Err(_) => Some(kernel::principal::MEMBER.to_string()),
    }
}

/// What set a run off: the event, whole, or nothing; and what hearing it
/// took ([`took`]), read from it once, where the run's context is made.
/// A run says something of itself for each step and each line a step
/// prints, and what its event took is the same at every one of them.
#[derive(Clone)]
pub(crate) struct Fired {
    event: Value,
    took: Option<String>,
}

impl From<Value> for Fired {
    fn from(event: Value) -> Fired {
        let took = took(&event);
        Fired { event, took }
    }
}

impl std::ops::Deref for Fired {
    type Target = Value;

    fn deref(&self) -> &Value {
        &self.event
    }
}

pub(crate) struct Outcome {
    pub status: RunStatus,
    pub outputs: Value,
    pub error: Option<String>,
}

impl RunCx {
    pub fn flow(&self) -> &Flow {
        match self.flow_id.strip_prefix("on.") {
            Some(name) => self
                .blueprint
                .triggers
                .iter()
                .find(|trigger| trigger.name == name)
                .map_or(&self.blueprint.flow, |trigger| &trigger.flow),
            None => &self.blueprint.flow,
        }
    }

    /// Whether what this run does is written down as it goes: it is,
    /// unless its instance's text says the instance keeps nothing
    /// (`durable: false`). Then its steps have no record, and nothing is
    /// saved for a restart to pick up, since no restart picks it up.
    pub fn kept(&self) -> bool {
        self.blueprint.durable
    }

    /// Says something of a step of this run, by itself. Kept, where the
    /// run is, only where a text could be owed it ([`crate::owed`]), and
    /// otherwise passed on in its order; passed on to whoever is watching
    /// and no more where the instance keeps nothing.
    pub async fn say(&self, topic: &'static str, source: Source, data: Value) {
        match self.kept() {
            true => self.engine.own_say(topic, source, data).tell(self.engine.own()).await,
            false => {
                self.engine.own().emit_ephemeral(topic, source, data);
            }
        }
    }

    /// Writes down what a step of this run did: now, where the run is
    /// kept; with the run's ending, where it is not ([`crate::held::Held`]). What an
    /// action saved to go on from is no part of that: nothing goes on.
    ///
    /// And says what the record says of the step (`says`: that it started,
    /// how it ended), in the same write where the run is kept; passed on
    /// to whoever is watching and no more where it is not.
    pub async fn put_step(&self, info: StepInfo, says: Vec<Say>) {
        self.write_step(info, says, false).await;
    }

    /// The same, saying whether it is written now only for another step
    /// of its run at work (`beside`), which is measured as such.
    async fn write_step(&self, info: StepInfo, says: Vec<Say>, beside: bool) {
        match self.kept() {
            true => {
                let Some((info, says, only_beside)) = self.live.held.quietly(info, says) else { return };
                self.live.held.notes(&info);
                // With it, in the one write, the steps that were passed
                // over since this run last wrote ([`RunCx::pass_over`]).
                let (live, run) = (self.live.clone(), self.run_id.clone());
                let began = std::time::Instant::now();
                let wrote =
                    self.engine.inner.store.put_step_saying(info, move || live.held.take(&run), says).await;
                if (beside || only_beside) && wrote.is_ok() {
                    let took = began.elapsed().as_secs_f64() * 1000.0;
                    self.engine.own().measure(crate::held::BESIDE, &[], took, true);
                }
            }
            false => {
                self.live.held.put(StepInfo { saved: Value::Null, ..info });
                for one in says {
                    one.pass_on(self.engine.own());
                }
            }
        }
    }

    /// Has records of `step` held until this run next writes, where
    /// nothing is done around it: no hook, no rule that watches it, no
    /// list it goes over, no other device. Such a step has no write of
    /// its own for what is held, as one passed over has none. A step that
    /// is taken up after a stop, one that fails, and one beside another
    /// at work are written as every step is.
    ///
    /// Where its action only decides ([`blueprint::ActionSpec::decides`]),
    /// its beginning and its ending are held. Nothing it gives is acted on
    /// before it is written (whatever acts takes its turn to write first,
    /// and the step's records with it), and stopped before then it has no
    /// record and is done from its start, which comes to what it would
    /// have come to.
    ///
    /// Where its action is the same done twice and does more than decide
    /// ([`blueprint::ActionSpec::idempotent`]: it may leave something
    /// behind, a message, a row, an event), its ending alone is held, and
    /// its beginning is written as every step's is. That is the rule, and
    /// it holds for every such action, whatever it does, because it asks
    /// nothing of it that being the same done twice does not already say.
    /// Stopped while its ending is held, its record says it is under way,
    /// so it is taken up as a step stopped in the middle of its action is,
    /// after whatever it leaves was left: done again, told so
    /// ([`crate::ActionCx::again`]), which is what tells one that looks
    /// for what it left (`channel.post`, `emit`) to look; from what it
    /// saved, where it saved anything; and with its `when` not looked at
    /// again, against what its own doing may have changed. Its beginning
    /// is never held, for those three reasons and a fourth: what is held
    /// of its run before it (a step that only decides, one passed over)
    /// is written with its beginning, before it acts on any of it. Held,
    /// none of that would be so: a stop after it had left a message would
    /// find no record, do the step from its start as though it were new,
    /// and leave a second.
    pub fn hush(&self, step: &blueprint::Step) {
        let quiet = match &step.kind {
            // The language's own first, as a step is done.
            blueprint::StepKind::Do(name) => match crate::held::own(name) {
                Some(quiet) => quiet,
                None => {
                    self.engine.inner.registry.get(name).map_or(Quiet::Not, |action| holds(action.spec()))
                }
            },
            _ => Quiet::Not,
        };
        let alone = [&step.on_start, &step.on_success, &step.on_fail, &step.on_timeout, &step.on_finish]
            .iter()
            .all(|hooks| hooks.is_empty())
            && step.watch.is_empty()
            && step.for_each.is_none()
            && step.device.is_none();
        if quiet != Quiet::Not && self.kept() && alone {
            self.live.held.hush(&self.run_id, &step.id, quiet);
        }
    }

    /// Says that a step of this run was passed over (its `when` did not
    /// hold, or its run ended before its turn), and has that written down,
    /// without a write of its own where it can be.
    ///
    /// A blueprint's handlers pass over as many steps as they run, and a
    /// write for each was as much again on the disk for nothing having
    /// happened. What it came to is held, and written with the next thing
    /// this run writes down: the next step it runs, or its ending.
    ///
    /// It is held only while no step of the run is at work. Then nothing
    /// of the run has happened since that a restart could pick up, and a
    /// run picked up again looks at the step's `when` exactly as it would
    /// have, had the daemon stopped a moment sooner. Beside a step that is
    /// at work it is written at once, as it always was: that step may be
    /// at work for hours, and a `when` looked at again after it, against
    /// what had changed meanwhile, could run what was passed over.
    ///
    /// It is said to whoever is watching, and kept as an event only where
    /// the text of some instance could be owed it ([`crate::owed`]). Kept,
    /// it is said in the write that keeps the step's record, and is held
    /// with it until then: said apart, a conductor stopped between the
    /// two looked at the step's `when` again and said it a second time,
    /// or had the record and never said it.
    pub async fn pass_over(&self, info: StepInfo, source: Source, data: Value) {
        if !self.kept() {
            self.live.held.put(StepInfo { saved: Value::Null, ..info });
            self.engine.own().emit_ephemeral("step.skipped", source, data);
            return;
        }
        let say = match self.engine.inner.owed.keeps("step.skipped") {
            true => Some(to_say("step.skipped", source, data)),
            false => {
                self.engine.own().emit_ephemeral("step.skipped", source, data);
                None
            }
        };
        if let Some((beside, say)) = self.live.held.pass(info, say) {
            self.write_step(beside, say.into_iter().collect(), true).await;
        }
    }

    /// Writes down what is held of this run, where it is kept: before
    /// anything that takes time comes after its last step.
    ///
    /// Held until the run's ending, a step that was passed over had no
    /// record while the run's hooks ran, and they may run for as long as
    /// anything does. Stopped then, the run was taken up from the steps
    /// that were written: a step after an `end` was run, since that the
    /// run had ended is not itself written down, and a `when` was looked
    /// at again against what a hook had set since.
    pub async fn write_held(&self) {
        if self.kept() && self.live.held.any(&self.run_id) {
            let (live, run) = (self.live.clone(), self.run_id.clone());
            let _ = self.engine.inner.store.put_held_saying(move || live.held.take(&run)).await;
        }
    }

    /// Takes away what was written down of the steps of this run whose
    /// names begin with `prefix`: a task's, when it is to run them again.
    pub async fn forget_steps(&self, prefix: &str) -> kernel::Result<()> {
        self.forget_held(prefix);
        match self.kept() {
            true => self.engine.inner.store.forget_steps(&self.run_id, prefix).await,
            false => Ok(()),
        }
    }

    /// Holds no longer what is held of the steps of this run whose names
    /// begin with `prefix`. Before whatever takes away what was written of
    /// them is asked for: a write that came between would put them back.
    pub fn forget_held(&self, prefix: &str) {
        self.live.held.forget(&self.run_id, prefix);
    }

    /// Where what a blueprint has this run say comes from: an `emit` step
    /// and what a step starts. That is its text's own to say, as whoever
    /// applied it, and takes nothing. What the runner says of the run
    /// for itself is said with [`RunCx::about`].
    pub fn source(&self, step: Option<&str>) -> Source {
        Source {
            device: None,
            instance: Some(self.live.id.clone()),
            run: Some(self.run_id.clone()),
            step: step.map(str::to_string),
            actor: Some(self.principal.actor()),
            takes: None,
        }
    }

    /// Where what the runner itself says of this run comes from: that it
    /// started, each step beginning and ending, why one failed or is
    /// tried again, each line one prints, and how the run ended.
    ///
    /// It takes what the event that set the run off took ([`Fired`]). A
    /// handler hears as whoever applied its text, and what its steps
    /// print and fail with is full of what they heard: said as taking
    /// nothing, a record of a table kept private was read, a line at a
    /// time, by whoever may hear that runs go on. So whatever is said of
    /// a step or a run for the runner's own part is said with this, and
    /// is then handed out by the question the event itself was
    /// (`Journal::may_tell`). A run no event set off, and one whose
    /// event took nothing, says as [`RunCx::source`] does.
    pub fn about(&self, step: Option<&str>) -> Source {
        self.source(step).taking(self.event.took.as_deref())
    }

    /// What an expression sees once `steps` have settled.
    pub fn scope(&self, steps: &Fields) -> Fields {
        let mut scope = self.engine.base_scope(&self.live);
        scope.insert("steps".into(), Value::Object(steps.clone()));
        if !self.event.is_null() {
            scope.insert("event".into(), Value::clone(&self.event));
        }
        scope
    }

    /// The same run, for the steps of a task: stopping them is `cancel`,
    /// which leaves the rest of the run as it is.
    pub fn within(&self, cancel: Cancel) -> RunCx {
        RunCx {
            engine: self.engine.clone(),
            live: self.live.clone(),
            blueprint: self.blueprint.clone(),
            principal: self.principal.clone(),
            run_id: self.run_id.clone(),
            flow_id: self.flow_id.clone(),
            event: self.event.clone(),
            cancel,
            generation: None,
            labels: self.labels.clone(),
            under: self.under.clone(),
        }
    }
}

pub(crate) enum Halt {
    Failed(String),
    Ended(End),
}

/// Where a graph of steps is: the run's own, or inside a task.
#[derive(Default)]
pub(crate) struct Place {
    /// What goes before a step's name to make the name it has in the run:
    /// nothing for the run's own steps, `build.` inside the task `build`.
    pub prefix: String,
    /// The steps outside the graph that had settled when it began, which
    /// its steps may read beside each other.
    pub outside: Fields,
    /// `item` and `index`, inside a task that runs once for each of a list.
    pub each: Fields,
}

/// How a graph of steps went.
pub(crate) struct Steps {
    /// Every step that ran, by its name in the graph.
    pub settled: BTreeMap<String, StepResult>,
    /// What an expression after the graph sees as `steps`.
    pub views: Fields,
    pub halt: Option<Halt>,
}

/// Runs the steps of `flow` until all have settled or one stops the rest,
/// and gives every one of them a status. `prior` is what the run's steps
/// were recorded as before a restart; only this graph's are looked at.
pub(crate) async fn run_steps(cx: &Arc<RunCx>, flow: &Flow, place: &Place, prior: Vec<StepInfo>) -> Steps {
    let journal = cx.engine.own();
    let mut settled: BTreeMap<String, StepResult> = BTreeMap::new();
    let mut interrupted: HashMap<String, StepInfo> = HashMap::new();
    for info in prior {
        // A step further in is a step of a task in this graph, and is that
        // task's to pick up.
        let Some(id) = within::own(&place.prefix, &info.step) else {
            continue;
        };
        if info.status.is_settled() {
            settled.insert(id.to_string(), StepResult::from_info(&info));
        } else {
            interrupted.insert(id.to_string(), info);
        }
    }
    let mut views = place.outside.clone();
    for (id, result) in &settled {
        views.insert(id.clone(), result.view());
    }
    let mut begun: BTreeSet<String> = settled.keys().cloned().collect();
    let mut running: JoinSet<(String, StepResult)> = JoinSet::new();
    let limit = cx.blueprint.max_parallel.map_or(DEFAULT_PARALLEL, |max| max as usize);
    // Steps that were done before a restart are not done again, and one
    // of them may have ended the run, or failed it: then it is still
    // ended, and as it was (by the first to have done so, where several
    // did). A step that had failed, with nothing to let it, and whose run
    // was stopped before it was written down as failed, left a run that
    // went on past it when it was taken up, and ended well.
    let ended = settled.iter().filter_map(|(id, result)| {
        let how = match &result.end {
            Some(end) => Halt::Ended(end.clone()),
            None if result.status == StepStatus::Failed && !result.tolerated => {
                Halt::Failed(crate::why::within(id, result.error.as_deref().unwrap_or_default()))
            }
            None => return None,
        };
        Some((result.finished, how))
    });
    let mut halt: Option<Halt> = ended.min_by_key(|(at, _)| *at).map(|(_, how)| how);
    // And one of them may have been cancelled: then the run was being
    // cancelled when the conductor stopped (what it is a run of was being
    // stopped, or applied over), and it is cancelled still. A step is
    // written as cancelled for nothing else. Something half stopped was
    // brought back when a conductor started, and its run, taken up before
    // anybody stopped it again, went on to the step after the one that was
    // cancelled and ended well. (Of the run's own steps: those inside
    // steps done as one are theirs, and may have been cancelled by one of
    // them failing.)
    let was_cancelled = |result: &StepResult| result.status == StepStatus::Cancelled;
    if halt.is_none() && place.prefix.is_empty() && settled.values().any(was_cancelled) {
        cx.cancel.cancel();
    }

    loop {
        if halt.is_none() && !cx.cancel.is_cancelled() {
            for step in &flow.steps {
                if running.len() >= limit {
                    break;
                }
                let ready =
                    step.needs.iter().all(|need| settled.contains_key(need) || flow.step(need).is_none());
                if begun.contains(&step.id) || !ready {
                    continue;
                }
                begun.insert(step.id.clone());
                let mut scope = cx.scope(&views);
                scope.extend(place.each.clone());
                let prior = interrupted.remove(&step.id);
                let id = step.id.clone();
                let (cx, mut step) = (cx.clone(), step.clone());
                step.id = within::name(&place.prefix, &id);
                // A step is a task of its own, and what it says is as deep in
                // a chain of events as what its run is handling.
                let chain = kernel::chain::current();
                running.spawn(crate::meets::carried(kernel::chain::within(chain, async move {
                    (id, step::exec_step(cx, step, scope, prior).await)
                })));
            }
        }
        let Some(joined) = running.join_next().await else { break };
        let (id, result) = match joined {
            Ok(finished) => finished,
            Err(err) => {
                // A step's task died. That is a bug in an action; the run
                // still has to end in a known state.
                halt.get_or_insert(Halt::Failed(format!("a step crashed: {err}")));
                cx.cancel.cancel();
                continue;
            }
        };
        if let Some(end) = &result.end {
            halt.get_or_insert(Halt::Ended(end.clone()));
            cx.cancel.cancel();
        } else if result.status == StepStatus::Failed && !result.tolerated {
            let error = result.error.clone().unwrap_or_default();
            halt.get_or_insert(Halt::Failed(crate::why::within(&id, &error)));
            cx.cancel.cancel();
        }
        views.insert(id.clone(), result.view());
        settled.insert(id, result);
    }

    // Everything that never ran gets a status, so the record is complete.
    let ended_early = matches!(halt, Some(Halt::Ended(_)));
    for step in &flow.steps {
        if settled.contains_key(&step.id) {
            continue;
        }
        let now = journal.now_ms();
        let name = within::name(&place.prefix, &step.id);
        match ended_early {
            // Its run ended before its turn, which is being passed over
            // as much as a `when` that did not hold: no write of its own.
            true => {
                let info = StepInfo {
                    run: cx.run_id.clone(),
                    step: name.clone(),
                    status: StepStatus::Skipped,
                    attempt: 0,
                    tolerated: false,
                    output: Value::Null,
                    error: None,
                    saved: Value::Null,
                    started: now,
                    finished: Some(now),
                };
                cx.pass_over(info, cx.about(Some(&name)), json!({})).await;
            }
            false => {
                let cancelled = StepStatus::Cancelled;
                let says = vec![cx.engine.own_say("step.cancelled", cx.about(Some(&name)), json!({}))];
                record(cx, &name, cancelled, 0, false, Value::Null, None, Value::Null, now, Some(now), says)
                    .await;
            }
        }
    }
    Steps { settled, views, halt }
}

/// Whether a run said that it started: one that was written down and
/// found with no step may have been stopped before it had. Asked only as
/// such a run is taken up.
pub(crate) async fn said_started(engine: &Engine, run: &str) -> bool {
    // Its row says so where its beginning was only passed on; otherwise
    // it is looked for from where its journal had got to as the run was
    // written.
    let Ok((begun, after)) = engine.inner.store.run_begun(run).await else { return false };
    if begun {
        return true;
    }
    let started = journal::EventQuery {
        run: Some(run.to_string()),
        topics: "run.started".into(),
        after,
        limit: 1,
        ..Default::default()
    };
    engine.own().read(started).await.is_ok_and(|said| !said.is_empty())
}

/// Whether a run that is being taken up had begun: it has a step, or it
/// said that it started. A run is written down before it begins (a
/// handler's when it is given its place, which may be long before its
/// turn; any run a moment before), so one found under way with nothing
/// said of it was never begun: it is begun now, and says that it
/// started, as it would have. Taken for one that goes on, it said that
/// it was taken up and never that it started, and whatever hears a run
/// start never heard this one.
pub(crate) async fn gone_on(engine: &Engine, run: &str, steps: &[StepInfo]) -> bool {
    !steps.is_empty() || said_started(engine, run).await
}

pub(crate) async fn run_flow(cx: Arc<RunCx>, prior: Vec<StepInfo>, resumed: bool) -> Outcome {
    let engine = &cx.engine;
    let journal = engine.own();
    let flow = cx.flow().clone();
    let started = journal.now_ms();
    let topic = if resumed { "run.resumed" } else { "run.started" };
    engine.begin_run(&cx, topic).await;

    // An instance being carried over to another text runs what that text
    // says in place of its steps ([`crate::carry`]).
    let carried = cx.flow_id == carry::FLOW;
    let Steps { settled, views, halt } = match carried {
        true => carry::steps(&cx, resumed).await,
        false => run_steps(&cx, &flow, &Place::default(), prior).await,
    };

    // A run is cancelled where something of it was cut short: a step
    // stopped in the middle, or never come to. One whose every step had
    // ended did all it was to do, and a stop that came after them took
    // nothing from it: a handler whose last step stops its own instance,
    // which is how a review ends, read as cancelled or as having gone
    // well by which of the two the database answered first.
    let ended = |id: &String| settled.get(id).is_some_and(|step| step.status != StepStatus::Cancelled);
    let whole = !carried && flow.steps.iter().all(|step| ended(&step.id));
    // Stopped or applied over by now, whatever the run reads as: what is
    // done as its steps end is not done for it.
    let overtaken = halt.is_none() && cx.cancel.is_cancelled();
    let ended_early = matches!(halt, Some(Halt::Ended(_)));
    let (mut status, mut error) = match halt {
        Some(Halt::Failed(error)) => (RunStatus::Failed, Some(error)),
        Some(Halt::Ended(End { failed: true, reason })) => {
            (RunStatus::Failed, Some(reason.unwrap_or_else(|| "ended as failed".into())))
        }
        Some(Halt::Ended(_)) => (RunStatus::Ok, None),
        None if overtaken && !whole => (RunStatus::Cancelled, Some("cancelled".into())),
        None => (RunStatus::Ok, None),
    };

    // What was passed over and is still held is written before anything
    // that can take time comes between the last step and the run's
    // ending ([`RunCx::write_held`]): its hooks, what a text does as it
    // is carried over; and always where the run ended early, which only
    // those records say. Otherwise it goes with the ending, in its write.
    let own = &cx.blueprint;
    let hooks = !(own.on_success.is_empty() && own.on_fail.is_empty() && own.on_finish.is_empty());
    if ended_early || carried || (cx.flow_id == "main" && hooks) {
        cx.write_held().await;
    }

    let scope = cx.scope(&views);
    let mut outputs = Value::Null;
    if status == RunStatus::Ok && carried {
        outputs = carry::outputs(&cx).await;
    }
    if status == RunStatus::Ok && cx.flow_id == "main" {
        match template::render(&Value::Object(cx.blueprint.outputs.clone()), &scope) {
            Ok(rendered) => outputs = rendered,
            Err(err) => {
                status = RunStatus::Failed;
                error = Some(format!("`outputs` could not be filled in: {err}"));
            }
        }
    }

    if cx.flow_id == "main" && !overtaken {
        // What is done by how its steps went, and then what is done when
        // they have ended however they went.
        let by_outcome =
            if status == RunStatus::Ok { &cx.blueprint.on_success } else { &cx.blueprint.on_fail };
        let mut scope = scope.clone();
        scope.insert("run".into(), json!({ "id": cx.run_id, "status": status.as_str(), "error": error }));
        // Taken up among them it goes on from the next: which were done
        // is kept with the run, and read only when a run is taken up.
        let after = &cx.blueprint.on_finish;
        let any = !(by_outcome.is_empty() && after.is_empty());
        let mut around = match resumed && cx.kept() && any {
            true => Around::kept(&cx, "").await,
            false => Around::default(),
        };
        if any {
            around.begin(&cx).await;
        }
        let list = if status == RunStatus::Ok { "on_success" } else { "on_fail" };
        around.ending(&cx, "", 0, list, by_outcome, &scope, false, after.is_empty()).await;
        around.ending(&cx, "", by_outcome.len(), "on_finish", after, &scope, false, true).await;
    }

    let finished = journal.now_ms();
    let outcome = Outcome { status, outputs, error };
    // Asked before the run reads as finished: a step that called for this
    // is still waiting then, and may be gone a moment after.
    let called = engine.inner.calls.of(cx.live.meta().parent.as_deref(), &cx.live.meta().key);
    // What becomes of its instance, where this run speaks for it.
    let (settled, becomes) = match cx.generation {
        Some(generation) => engine.settling(&cx.live, generation, &cx.run_id, &outcome, called).await.unzip(),
        None => (None, None),
    };
    // Measured before it reads as finished: whoever waits for a run and
    // then asks what was measured is told of this one too. One that was
    // cancelled is counted, and not among those that failed: its instance
    // was stopped or applied over, which is nothing of the run's going
    // wrong, and a count of failures that took those in said that reviews
    // failed where every one of them had decided and ended itself.
    let ok = outcome.status != RunStatus::Failed;
    let project = cx.live.meta().project.unwrap_or_default();
    let of =
        [("blueprint", cx.blueprint.name.as_str()), ("flow", cx.flow_id.as_str()), ("project", &project)];
    journal.measure("run", &labels::beside(&of, &cx.labels), (finished - started) as f64, ok);
    // A run that failed says so, and then that it finished, as a run says
    // after any ending: what is to be done about a failure listens for a
    // failure, and does not have to read how each finished run went.
    // First, what is still to be said of steps that were passed over:
    // their records go with this write.
    let mut says = cx.live.held.says(&cx.run_id);
    if outcome.status == RunStatus::Failed {
        let failed = json!({
            "flow": cx.flow_id,
            "blueprint": cx.blueprint.name,
            "duration_ms": finished - started,
            "error": outcome.error,
            "reason": outcome.error.as_deref().map(crate::why::reason),
        });
        says.push(engine.own_say("run.failed", cx.about(None), failed));
    }
    let ended = json!({
        "flow": cx.flow_id,
        "blueprint": cx.blueprint.name,
        "status": outcome.status.as_str(),
        "duration_ms": finished - started,
        "error": outcome.error,
    });
    says.push(engine.own_say("run.finished", cx.about(None), ended));
    // One write: how the run ended, what its steps did where that was not
    // written as they went, what its instance became, and all that is
    // said of it. Whoever hears that the run finished reads both as
    // settled; a conductor stopped here leaves all of it or none.
    let ending = Ending {
        run: cx.run_id.clone(),
        status: outcome.status,
        outputs: outcome.outputs.clone(),
        error: outcome.error.clone(),
        now: finished,
        steps: cx.live.held.of(&cx.run_id),
        says,
        becomes,
    };
    let written = engine.inner.store.end_run(ending).await;
    // Held until they are there to be read, so that nobody who asks in
    // between is told the run had no steps.
    cx.live.held.written(&cx.run_id);
    if let (Some(settled), Ok(true), Some(generation)) = (settled, written, cx.generation) {
        engine.settled(&cx.live, settled, generation).await;
    }
    engine.notify_run_finished(&cx.run_id);
    outcome
}

/// Does what a blueprint says is done as its instance stops (`on_stop`),
/// on a context that is not cancelled even though the instance is.
///
/// It is done in no run, so how far it has come is kept in the
/// instance's own row, with why it is being stopped and a note that the
/// stop is owed, from before the first of it: a stop that is asked for
/// again, or finished by a conductor that starts in the middle of one
/// (`Engine::finish_stops`), goes on from the entry it had come to and
/// does none twice. Nothing is kept for an instance that keeps nothing.
pub(crate) async fn run_stop_hooks(engine: &Engine, live: &Arc<Live>, why: &str) {
    let (blueprint, principal) = live.acting();
    if blueprint.on_stop.is_empty() {
        return;
    }
    let cx = RunCx {
        engine: engine.clone(),
        live: live.clone(),
        blueprint: blueprint.clone(),
        principal,
        run_id: String::new(),
        flow_id: "main".into(),
        event: Value::Null.into(),
        cancel: Cancel::new(),
        generation: None,
        labels: live.meta().labels,
        under: String::new(),
    };
    let mut around = match cx.kept() {
        true => Around::kept(&cx, "").await,
        false => Around::default(),
    };
    around.stopping(why);
    let scope = || cx.scope(&Fields::new());
    let _ = around.alone(&cx, "on_stop", &blueprint.on_stop, scope, Cancel::new(), false).await;
}

/// Writes down what a step did so far ([`RunCx::put_step`]).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn record(
    cx: &RunCx,
    step: &str,
    status: StepStatus,
    attempt: u32,
    tolerated: bool,
    output: Value,
    error: Option<String>,
    saved: Value,
    started: i64,
    finished: Option<i64>,
    says: Vec<Say>,
) {
    let info = StepInfo {
        run: cx.run_id.clone(),
        step: step.to_string(),
        status,
        attempt,
        tolerated,
        output,
        error,
        saved,
        started,
        finished,
    };
    cx.put_step(info, says).await;
}
