//! Carrying a running instance over to another text: `on_update`.
//!
//! Applying over an instance runs its steps again, which is right for
//! something that is rolled out and wrong for something that lives a long
//! time and is in the middle of its work: a lane's steps claim its card.
//! A text may say instead how one that is running takes it: `on_update`,
//! a list of things to do *in place of* the steps. The instance carries
//! on from where it is, and what it was is handed to it as `was` (the
//! version, revision, state and params it had), so that state of one
//! shape can be made into state of another.
//!
//! It goes in this order, and nothing of the instance's ever runs beside
//! it:
//!
//! 1. Its handlers are told to take no more events, and each ends when it
//!    is next between runs: a run under way finishes on the text it began
//!    with. Until they all have, the instance is as it was. What happens
//!    meanwhile waits in the journal.
//! 2. The text, the params and the authority are replaced together, as any
//!    update replaces them, and the state is carried as any update carries
//!    it: a key still declared keeps its value, a new one starts at its
//!    default, one no longer declared is gone (and is still in
//!    `was.state`). What it had is kept as what to go back to, state and
//!    all.
//! 3. What the text attaches with `uses` is brought up, and `on_update`
//!    runs, each of its entries in turn and each seeing what the one
//!    before it set. The first that fails ends it.
//! 4. It settles as any update does. If it failed, the instance is back on
//!    the text, the authority *and the state* it had; if it did not, the
//!    new text's handlers listen from where the old ones had got to, and
//!    what is below is looked at as after any apply somebody asked for.
//!
//! The run is one of the instance's runs, called `on_update`. Its steps
//! having not been run, what a caller is given back is what they gave out
//! when they last ran.
//!
//! An instance is carried only if its steps have settled (it is active),
//! and never when its steps were asked for (`force`): then it is an update
//! like any other. A text that says nothing of this is one too.
//!
//! Whoever asks is answered as soon as the run is there, before 1 is
//! over: a step that asked goes on, and ends. So what the instance is to
//! be carried over to is kept with the run, in the write that makes it
//! (`Store::insert_carry`), until the write that takes the text or ends
//! the run. A conductor stopped in between has nobody to ask again; the
//! one that starts finds the run with what it carries, starts the
//! instance's handlers so that what was under way in them ends on the
//! text it began with, and carries it over from 1 ([`Engine::carries_owed`],
//! [`Engine::reached`]). Asked again for the same meanwhile (the step,
//! taken up before it had kept its answer), it is the same run that
//! answers; and once the text was taken, the step finds the run it asked
//! for by which step asked ([`asked_by`]) and asks nothing. And a text asked for as a conductor starts, before the start
//! has come to the instance and started its handlers, waits for that:
//! what they had under way was begun before the text was asked for.

use std::collections::BTreeMap;
use std::sync::Arc;

use blueprint::Blueprint;
use kernel::{Cancel, Error, Fields, Result, Source};
use runs::{Definition, Ending, InstanceInfo, InstanceStatus, RunInfo, RunStatus, Say, to_say};
use serde_json::{Value, json};

use crate::engine::{Applied, Engine, Live, initial_state};
use crate::hooks::Around;
use crate::run::{self, Halt, RunCx, Steps};
use crate::trigger;
use crate::update::Took;

/// Which step asked: its run and its name, kept with the run that
/// carries what it asked for over, so that the step done again after a
/// restart finds it was answered (`crate::child`).
pub(crate) fn asked_by(run: &str, step: &str) -> String {
    format!("{run}\n{step}")
}

/// What the run that carries an instance over is called.
pub(crate) const FLOW: &str = "on_update";

impl Engine {
    /// Begins carrying `live` over to `definition`, whose text is
    /// `blueprint`, and returns at once with the run that will do it.
    /// Called where nobody else is applying over the instance.
    pub(crate) async fn carry(
        &self,
        live: &Arc<Live>,
        instance: String,
        blueprint: Arc<Blueprint>,
        definition: Definition,
        (asked, during): (bool, Option<u64>),
        by: Option<String>,
    ) -> Result<Applied> {
        let applied = |run: String| Applied {
            instance: instance.clone(),
            run: Some(run),
            created: false,
            changed: true,
            took: Some(Took::Carried),
            vouched: Default::default(),
            reached: Default::default(),
        };
        // Asked for what it is already waiting to be carried over to, by
        // whoever asked before: the run that will do it is the answer. A
        // second run beside it would only call the first off.
        if let Some((run, first)) = waits_for(live, &definition, asked, during) {
            if let Some(since) = first {
                self.came_to(live, if asked { "apply" } else { "step" }, since);
            }
            return Ok(applied(run));
        }
        let run = self.inner.own.new_id("run");
        let started = self.inner.own.now_ms();
        let info = RunInfo {
            id: run.clone(),
            instance: instance.clone(),
            flow: FLOW.into(),
            revision: definition.revision.clone(),
            status: RunStatus::Running,
            event: Value::Null,
            outputs: Value::Null,
            error: None,
            started,
            finished: None,
            steps: Vec::new(),
        };
        // With what it is to be carried over to, as one write: whoever
        // asked is answered next, and may be gone before it is taken.
        let carries = json!({ "definition": definition, "asked": asked, "during": during });
        self.inner.store.insert_carry(info, carries, by).await?;
        let carrying = Carrying { blueprint, definition, run: run.clone(), asked, during };
        self.wait_to_carry(live, carrying, None);
        Ok(applied(run))
    }

    /// Has `live` wait to be carried over: a text it was already waiting
    /// to be carried over to is called off, the later asking being the
    /// one that stands; its handlers are told to take no more, and it is
    /// carried over when each has ended. `kept` is when the run was
    /// made, for one kept from before this conductor started.
    ///
    /// Where a conductor that is starting has not come to the instance
    /// yet, its handlers have not been started and what they had under
    /// way has not been taken up: it waits for that first
    /// ([`Engine::reached`]), so that those runs end on the text they
    /// began with, as they do with nothing stopped. Carried over at
    /// once, what the text says is done as it is taken was done before
    /// work that had begun before it was asked for.
    fn wait_to_carry(&self, live: &Arc<Live>, carrying: Carrying, kept: Option<i64>) {
        let waiting = Cancel::new();
        let run = carrying.run.clone();
        let latch = {
            let mut carries = live.carrying.lock().expect("live lock");
            let now = Waiting { cancel: waiting.clone(), carrying: carrying.clone(), kept };
            if let Some(earlier) = carries.waiting.replace(now) {
                earlier.cancel.cancel();
            }
            carries.unreached.clone()
        };
        let ended = latch.is_none().then(|| trigger::retire(live));
        let (engine, live) = (self.clone(), live.clone());
        // Carried over because a step of a run asked, it is part of the
        // chain of events that run is part of.
        let chain = kernel::chain::current();
        self.spawn(kernel::chain::within(chain, async move {
            if let Some(latch) = latch {
                tokio::select! {
                    () = latch.cancelled() => {}
                    () = waiting.cancelled() => {}
                    () = live.cancel.cancelled() => {}
                }
            }
            // Told to take no more by the start, in the moment it had
            // started them, where this is the one that then stood.
            let ended = ended.unwrap_or_else(|| {
                let mut carries = live.carrying.lock().expect("live lock");
                let mine = carries.waiting.as_ref().is_some_and(|one| one.carrying.run == run);
                if mine { std::mem::take(&mut carries.retired) } else { Vec::new() }
            });
            engine.carry_over(live, carrying, waiting, ended).await;
        }));
    }

    /// Waits for the instance to be between runs, then replaces what it
    /// is and runs `on_update`.
    async fn carry_over(&self, live: Arc<Live>, carrying: Carrying, waiting: Cancel, ended: Vec<Cancel>) {
        for handler in ended {
            tokio::select! {
                () = handler.cancelled() => {}
                () = waiting.cancelled() => break,
                () = live.cancel.cancelled() => break,
            }
        }
        let applying = live.apply_lock.lock().await;
        let still = {
            let mut carries = live.carrying.lock().expect("live lock");
            let still = !waiting.is_cancelled() && !live.cancel.is_cancelled();
            if still {
                carries.waiting = None;
            }
            still
        };
        if !still {
            // Whatever called it off has the instance's handlers in hand.
            self.end_carry(&live, &carrying.run, RunStatus::Cancelled, CALLED_OFF.into(), None).await;
            return;
        }
        let run = carrying.run.clone();
        match self.replace(&live, carrying).await {
            Ok(generation) => {
                drop(applying);
                run::run_flow(Arc::new(self.carry_cx(&live, run, generation)), Vec::new(), false).await;
            }
            Err(refused) => {
                // Nothing of it was changed: it listens again as it was.
                trigger::rearm(self, &live, None).await;
                let kept = live.meta().revision;
                // That the text was not taken is said in the write that
                // ends the run that was to carry it over: said before it, a
                // conductor stopped between the two came back to that run
                // under way and ended it as one it was stopped before.
                let said = json!({ "error": refused.message, "kept": kept });
                let not_taken = to_say("instance.update_failed", self.instance_source(&live), said);
                self.end_carry(&live, &run, RunStatus::Failed, refused.message, Some(not_taken)).await;
            }
        }
    }

    /// Puts the new text, params, authority and carried state in force,
    /// keeping what there was as what to go back to. The generation that
    /// then begins.
    async fn replace(&self, live: &Arc<Live>, carrying: Carrying) -> Result<u64> {
        let Carrying { blueprint, definition, asked, during, .. } = carrying;
        let existing = self.instance(&live.id).await?;
        if existing.status != InstanceStatus::Active {
            return Err(Error::conflict(format!("`{}` is no longer running as it was", existing.key)));
        }
        let had = live.meta().state;
        let seen = self.seen_by(live);
        let state = initial_state(self, &blueprint, (&definition.params, seen), &had)?;
        let stood = self.stood(live).await?;
        let before = Definition {
            revision: existing.revision.clone(),
            source: existing.source,
            params: existing.params,
            principal: Some(existing.principal),
            state: Some(had),
            before: Some(stood),
            heard: existing.heard,
        };
        // The text is written as taken and said to be, as one write.
        let said = json!({
            "blueprint": existing.blueprint,
            "revision": definition.revision,
            "from": existing.revision,
            "carried": true,
        });
        let says = vec![to_say("instance.updated", self.source_taking(live, &definition), said)];
        // What the text adds listens from here: from the event that
        // brought this about, where one did, and otherwise from now,
        // before `on_update` is done. So a handler the text brings hears
        // what its own `on_update` says (which is how something done as
        // a text is taken asks what only a handler can), and what
        // happened meanwhile, as the handlers it had do. Its place is
        // written with the text, in the one write: written after it, a
        // conductor stopped between the two gave the handler its place
        // when it started, from then, and the event that had brought the
        // text was never heard by the handler the text brought for it.
        let places = trigger::places(self, live, &blueprint, during).await;
        let taken = runs::Taken {
            definition: definition.clone(),
            state: state.clone(),
            previous: Some(before.clone()),
            below: asked,
            places,
        };
        let Some(below) = self.take_owing(live, taken, says).await? else {
            return Err(crate::update::ended_meanwhile(&existing.key));
        };
        *live.previous.lock().expect("live lock") = Some(crate::update::keeping(before, below));
        let generation = live.next_generation();
        live.redefine(blueprint, definition, Some(state));
        if below.owed() {
            live.owes_look(generation);
        }
        Ok(generation)
    }

    fn carry_cx(&self, live: &Arc<Live>, run: String, generation: u64) -> RunCx {
        let (blueprint, principal) = live.acting();
        RunCx {
            engine: self.clone(),
            live: live.clone(),
            blueprint,
            principal,
            run_id: run,
            flow_id: FLOW.into(),
            event: Value::Null.into(),
            cancel: live.work().child(),
            generation: Some(generation),
            labels: live.meta().labels,
            under: String::new(),
        }
    }

    /// Ends a run that never came to run `on_update`, and says so as the
    /// end of any run is said; before that, in the same write, `first`:
    /// that the text was not taken, where it was refused.
    async fn end_carry(&self, live: &Live, run: &str, status: RunStatus, why: String, first: Option<Say>) {
        let now = self.inner.own.now_ms();
        let source =
            Source { instance: Some(live.id.clone()), run: Some(run.to_string()), ..Source::default() };
        let mut says: Vec<Say> = first.into_iter().collect();
        if status == RunStatus::Failed {
            let failed = json!({ "flow": FLOW, "blueprint": live.meta().blueprint, "error": why, "reason": crate::why::reason(&why) });
            says.push(self.own_say("run.failed", source.clone(), failed));
        }
        let said = json!({
            "flow": FLOW,
            "blueprint": live.meta().blueprint,
            "status": status.as_str(),
            "error": why,
        });
        says.push(self.own_say("run.finished", source, said));
        let ending = Ending {
            run: run.to_string(),
            status,
            outputs: Value::Null,
            error: Some(why),
            now,
            steps: Vec::new(),
            says,
            becomes: None,
        };
        let _ = self.inner.store.end_run(ending).await;
        self.notify_run_finished(run);
    }

    /// At start, before anything goes on: says of each instance that
    /// the start has not come to it yet, and has each that is owed a
    /// text, one it was waiting to be carried over to when the conductor
    /// stopped, wait for it again. Whoever asks for the same before the
    /// start has come to the instance is answered with the run there is;
    /// and it is carried over once the start has come to it and started
    /// its handlers ([`Engine::reached`]).
    pub(crate) async fn carries_owed(
        &self,
        known: &[(InstanceInfo, Arc<Live>)],
        unfinished: &[RunInfo],
    ) -> Reaching {
        for (_, live) in known {
            live.carrying.lock().expect("live lock").unreached = Some(Cancel::new());
        }
        // Whatever way the start is left, each of them is come to.
        let reaching = Reaching(self.clone(), known.iter().map(|(_, live)| live.clone()).collect());
        for run in unfinished.iter().filter(|run| run.flow == FLOW) {
            let of = |(info, _): &&(InstanceInfo, Arc<Live>)| {
                info.id == run.instance && info.status == InstanceStatus::Active
            };
            let Some((info, live)) = known.iter().find(of) else { continue };
            let Ok(Some(kept)) = self.inner.store.carries(&run.id).await else { continue };
            let Ok(definition) = serde_json::from_value::<Definition>(kept["definition"].clone()) else {
                continue;
            };
            // A text this program can no longer read is not taken: its
            // run ends as one nothing was kept for, saying to apply it.
            let Ok(blueprint) = self.compile(&info.blueprint, &definition.source, info.project.as_deref())
            else {
                continue;
            };
            let carrying = Carrying {
                blueprint,
                definition,
                run: run.id.clone(),
                asked: kept["asked"] == true,
                during: kept["during"].as_u64(),
            };
            live.carrying.lock().expect("live lock").kept = Some(run.id.clone());
            self.wait_to_carry(live, carrying, Some(run.started));
        }
        reaching
    }

    /// At start, where it has come to an instance and its handlers have
    /// been started, if they were to be: whatever waits to carry it over
    /// goes on from here, the handlers told to take no more in this
    /// moment, as they are where a text is asked for with nothing
    /// stopped.
    pub(crate) fn reached(&self, live: &Arc<Live>) {
        let first = {
            let mut carries = live.carrying.lock().expect("live lock");
            let carries = &mut *carries;
            let Some(latch) = carries.unreached.take() else { return };
            let mut first = None;
            if let Some(one) = carries.waiting.as_mut().filter(|one| !one.cancel.is_cancelled()) {
                carries.retired = trigger::retire(live);
                first = one.kept.take();
            }
            latch.cancel();
            first
        };
        if let Some(since) = first {
            self.came_to(live, "start", since);
        }
    }

    /// Measures who came first to a text an instance was owed, kept from
    /// before the conductor started: the start, or whoever asked for the
    /// same again before the start had come to it. Nothing else says
    /// which, and each is a way it has to hold ([`crate::unbegun::BEGUN`]).
    fn came_to(&self, live: &Live, by: &'static str, since: i64) {
        let journal = &self.inner.own;
        let blueprint = live.meta().blueprint;
        let labels = [("blueprint", blueprint.as_str()), ("by", by), ("owed", "text")];
        journal.measure(crate::unbegun::BEGUN, &labels, (journal.now_ms() - since).max(0) as f64, true);
    }

    /// At start, picks up an instance that was being carried over when
    /// the conductor last stopped: true when it has it in hand. `taking`
    /// says the instance was in the middle of taking a text and no run of
    /// its steps was under way: then it was this that it was doing.
    pub(crate) async fn resume_carry(
        &self,
        live: &Arc<Live>,
        generation: u64,
        mut runs: Vec<RunInfo>,
        taking: bool,
    ) -> bool {
        let before = live.previous.lock().expect("live lock").clone();
        let taken = before.filter(|_| taking).and_then(|before| runs.pop().map(|run| (before, run)));
        // It had not yet been between runs, and is as it was (or it was
        // applied over since). What it was to take is kept with the run
        // that was to carry it over, and something waits to do that, or
        // to end the run where it was called off (`carries_owed`). A run
        // with nothing kept is one a program from before this made, one
        // whose text this program cannot read, or one that was called
        // off before the conductor stopped: it ends, saying which.
        let kept = live.carrying.lock().expect("live lock").kept.take();
        for run in runs.into_iter().filter(|run| Some(&run.id) != kept.as_ref()) {
            // Called off before the conductor stopped, by a text the
            // instance took or another asked for, and stopped before the
            // run had ended: it is not to be applied again.
            let why = match self.inner.store.carries(&run.id).await {
                Ok(Some(Value::Bool(false))) => CALLED_OFF,
                _ => "the conductor stopped before it had been carried over to this text; apply it again",
            };
            self.end_carry(live, &run.id, RunStatus::Cancelled, why.into(), None).await;
        }
        let Some((before, run)) = taken else { return false };
        // Where `on_update` had begun it goes on from the entry it had
        // come to, with what the entries before it set as they left it:
        // how far it came is kept with its run (`hooks::Around::alone`).
        // Where nothing was kept it had not begun, or was begun by a
        // program from before this, which kept nothing: it is begun from
        // what the instance was, so that nothing it set is counted twice.
        let begun = self.inner.store.kept_hooks(&run.id, None).await.is_ok_and(|kept| !kept.is_null());
        let (blueprint, meta) = (live.blueprint(), live.meta());
        if !begun
            && let Ok(state) = initial_state(
                self,
                &blueprint,
                (&meta.params, self.seen_by(live)),
                &before.state.unwrap_or_default(),
            )
        {
            live.meta.lock().expect("live lock").state = state.clone();
            let _ = self.inner.store.set_instance_state(&live.id, state, self.inner.own.now_ms()).await;
        }
        // What the text adds has its place written with the text
        // (`replace`). A text that a program from before this wrote as
        // taken may have none for a handler it brought: it is given one
        // now, from here.
        trigger::prepare(self, live, None).await;
        // Stopped before the run said that it started, it had done none
        // of it, and begins now.
        let gone_on = run::said_started(self, &run.id).await;
        let cx = Arc::new(self.carry_cx(live, run.id, generation));
        self.spawn(async move {
            run::run_flow(cx, Vec::new(), gone_on).await;
        });
        true
    }
}

/// Why the run that was to carry an instance over ends, where something
/// came to the instance first.
const CALLED_OFF: &str = "it was applied over, or stopped, before it had been carried over to this text";

/// Held by a conductor's start from when it has said of everything
/// alive that it has not come to it yet ([`Engine::carries_owed`]) until
/// it is over, however it ends. A start comes to each instance where it
/// has taken it up, found it at rest, settled it as failed or put it
/// back on the text it had ([`Engine::reached`], asked at the end of
/// what the start does for one instance, whichever of those it was).
/// One that the start could not go on from (it could not read what it
/// kept) leaves the rest not come to, and whatever waits for one of
/// them would wait for ever: put away, this comes to each of them.
pub(crate) struct Reaching(Engine, Vec<Arc<Live>>);

impl Drop for Reaching {
    fn drop(&mut self) {
        for live in &self.1 {
            self.0.reached(live);
        }
    }
}

/// What is in hand of an instance being carried over.
#[derive(Default)]
pub(crate) struct Carries {
    /// The text it is waiting to be carried over to.
    pub waiting: Option<Waiting>,
    /// There while a conductor that is starting has not come to the
    /// instance, and cancelled when it has ([`Engine::reached`]).
    unreached: Option<Cancel>,
    /// What each of its handlers says when it has ended, where the
    /// start told them to take no more for what was waiting then.
    retired: Vec<Cancel>,
    /// The run kept from before the conductor started that something
    /// here has in hand, and will end: the start leaves it alone.
    kept: Option<String>,
}

/// A text an instance is waiting to be carried over to.
pub(crate) struct Waiting {
    /// Cancelled to call it off.
    pub cancel: Cancel,
    carrying: Carrying,
    /// When its run was made, for one kept from before the conductor
    /// started that nobody has come to since.
    kept: Option<i64>,
}

/// The run that will carry `live` over to this very thing, asked this
/// way by this bearer, where it is waiting to be; and, where it is one
/// kept from before the conductor started that nobody had come to yet,
/// when its run was made.
fn waits_for(
    live: &Live,
    definition: &Definition,
    asked: bool,
    during: Option<u64>,
) -> Option<(String, Option<i64>)> {
    let mut carries = live.carrying.lock().expect("live lock");
    let waiting = carries.waiting.as_mut().filter(|waiting| !waiting.cancel.is_cancelled())?;
    let Carrying { definition: wanted, run, asked: was_asked, during: was_during, .. } = &waiting.carrying;
    let same = wanted == definition && *was_asked == asked && *was_during == during;
    same.then(|| (run.clone(), waiting.kept.take()))
}

/// What an instance is to be carried over to, and the run that does it.
#[derive(Clone)]
struct Carrying {
    blueprint: Arc<Blueprint>,
    definition: Definition,
    run: String,
    /// Somebody asked for the apply, so what is below is looked at after.
    asked: bool,
    /// The place of the event that brought the asking about, when a step
    /// of a handler's run asked: what the text adds hears from there.
    during: Option<u64>,
}

/// What `was` is in `on_update`: the instance as it was before it took
/// the text, read from what it would go back to.
fn was(live: &Live) -> Value {
    match live.previous.lock().expect("live lock").clone() {
        Some(before) => json!({
            "version": blueprint::version(&before.source),
            "revision": before.revision,
            "state": before.state.unwrap_or_default(),
            "params": before.params,
        }),
        None => json!({ "version": null, "revision": null, "state": {}, "params": {} }),
    }
}

/// What the run does in place of a graph of steps: brings up what the
/// text attaches, then `on_update`, entry by entry.
pub(crate) async fn steps(cx: &RunCx, resumed: bool) -> Steps {
    let halt = match entries(cx, resumed).await {
        Ok(()) => None,
        // Applied over or stopped as it ran: that is not its failure.
        Err(_) if cx.cancel.is_cancelled() => None,
        Err(error) => Some(Halt::Failed(error)),
    };
    Steps { settled: BTreeMap::new(), views: Fields::new(), halt }
}

async fn entries(cx: &RunCx, resumed: bool) -> std::result::Result<(), String> {
    cx.engine.start_attachments(&cx.live, &cx.blueprint).await.map_err(|err| err.message)?;
    let was = was(&cx.live);
    // Taken up among them after a restart it goes on from the next:
    // which were done is kept with the run, and read only then.
    let mut around = match resumed && cx.kept() {
        true => Around::kept(cx, "").await,
        false => Around::default(),
    };
    // Read afresh for each, so that one sees what the one before set.
    let scope = || {
        let mut scope = cx.scope(&Fields::new());
        scope.insert("was".into(), was.clone());
        scope
    };
    let entries = cx.blueprint.on_update.as_deref().unwrap_or_default();
    around.alone(cx, "on_update", entries, scope, cx.cancel.child(), true).await
}

/// What a caller is given back by an instance that was carried over: what
/// its steps gave out when they last ran, since they were not run now.
pub(crate) async fn outputs(cx: &RunCx) -> Value {
    match cx.engine.inner.store.last_main_run(&cx.live.id).await {
        Ok(Some(run)) if run.status == RunStatus::Ok => run.outputs,
        _ => Value::Null,
    }
}
