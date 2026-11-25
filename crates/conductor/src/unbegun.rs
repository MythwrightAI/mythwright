//! An instance that could not be begun.
//!
//! An instance is written down, what it attaches is brought up, and its
//! steps are begun: three things, and the second and third can fail for
//! reasons of the instance's own (a text it attaches that no longer
//! reads, a param that cannot be filled in). Whoever applied it is told.
//! But left at that, its row said it was starting for ever, with no run,
//! and the next conductor to start took it up, met the same failure, and
//! failed its own start with it: one instance kept a whole factory down.
//!
//! So an instance that cannot be begun is said to have failed, and is
//! settled as failed, wherever that is found out: as it is applied, or as
//! a conductor that starts takes up one that was stopped between being
//! written down and its first run. A start never fails for one instance.
//!
//! One that was running, and was being given another text when it could
//! not be begun, has not failed: the text has. It goes back to the text,
//! the authority and the state it had, as it does when the steps of an
//! update fail (`settle`), what was brought up for the update is handed
//! back, and what it had running is left running.
//!
//! A conductor stopped between an instance being written down and its
//! first run starts with an instance that is owed its beginning, and two
//! things come to it: the start, which goes over everything alive, and
//! the step that asked for it, taken up as its own run goes on. There is
//! one way of beginning it ([`Engine::begin_owed`]), asked by both under
//! the instance's own turn: whichever comes second finds the run the
//! first began, or the instance over, and begins nothing. Each for
//! itself, they gave it two runs of its steps, or failed it and then
//! made and failed another of its name.
//!
//! Whether it is owed its steps is two things, that it is written as
//! starting and that no run of its steps is under way, and they are read
//! as of one moment ([`runs::Store::standing`]). A run of steps takes no turn
//! to end, and its ending is one write: the run as over, the instance as
//! what it became. Read apart, an instance read as starting while its
//! steps were under way, whose run was then read as over, was taken to
//! be owed them again: whoever came to it second, in the moment its steps
//! ended, had them run once more.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use kernel::{BoxFut, Error, Result};
use runs::{InstanceStatus, RunInfo, RunStatus, to_say};
use serde_json::{Value, json};

use crate::engine::{Engine, Live};
use crate::run::{self, RunCx};
use crate::trigger;
use crate::whole::Whole;

/// What is measured where an instance owed its beginning is begun: how
/// long it had been owed, by who came to it (`by`: `start`, the conductor
/// as it started; `step`, the step that asked for it, taken up; `apply`,
/// somebody who asked for it again), and which steps it was owed (`owed`:
/// `first`, it had never run any; `again`, those of a text it had taken).
/// And where it was owed no steps but a text it was waiting to be
/// carried over to when the conductor stopped (`owed`: `text`), who came
/// to that first ([`crate::carry`]).
pub(crate) const BEGUN: &str = "instance.begun";

/// What an instance that could not be begun says it failed with, before
/// why.
const COULD_NOT: &str = "could not be begun: ";

/// Why an instance could not be begun, as whoever asked for it was
/// answered: what it says it failed with, without the words that say it
/// was never begun.
pub(crate) fn why(error: &str) -> &str {
    error.strip_prefix(COULD_NOT).unwrap_or(error)
}

impl Engine {
    /// Brings up what a new instance attaches and begins its steps: the
    /// run that was begun, or why it could not be, with the instance
    /// settled as failed.
    pub(crate) async fn begin(&self, live: &Arc<Live>, generation: u64) -> Result<String> {
        let begun = async {
            self.start_attachments(live, &live.blueprint()).await?;
            self.start_main(live, generation, None).await
        };
        match begun.await {
            Ok(run) => Ok(run),
            Err(err) => {
                self.could_not_begin(live, &err).await;
                Err(err)
            }
        }
    }

    /// Begins an instance that is written down as starting with none of
    /// its steps under way, where that is still so: the one way it is
    /// done. Whoever asks holds the instance's own turn
    /// (`Live::apply_lock`), and what is so is read here, inside it: the
    /// instance and the last run of its steps, as of one moment.
    ///
    /// What it attaches may not have been brought up either, so what is
    /// not there is. Gives the run of its steps that is under way: the
    /// one begun here, or the one somebody began before. Nothing, where
    /// it is no longer starting. Where it cannot be begun it has failed
    /// and says so, and that is what is answered.
    ///
    /// Who came to it first is measured (`instance.begun`, with how long
    /// it had been owed): nothing else says which of them began it, and
    /// each is a way it has to hold.
    pub(crate) fn begin_owed<'a>(
        &'a self,
        live: &'a Arc<Live>,
        by: &'static str,
    ) -> BoxFut<'a, Result<Option<String>>> {
        // In a box: a step asks this of what it started, and what is
        // begun here is steps.
        Box::pin(async move {
            let store = &self.inner.store;
            let Some((now, last)) = store.standing(&live.id).await? else { return Ok(None) };
            if now.status != InstanceStatus::Starting {
                return Ok(None);
            }
            // Its first steps, or those of a text it took since they ran.
            let owed = if last.is_some() { "again" } else { "first" };
            if let Some(going) = last.filter(|run| run.status == RunStatus::Running) {
                return Ok(Some(going.id));
            }
            let generation = live.generation.load(Ordering::SeqCst);
            let begun = async {
                self.start_missing_attachments(live).await?;
                self.start_main(live, generation, None).await
            };
            let begun = begun.await;
            let journal = &self.inner.own;
            let labels = [("blueprint", now.blueprint.as_str()), ("by", by), ("owed", owed)];
            journal.measure(BEGUN, &labels, (journal.now_ms() - now.updated).max(0) as f64, begun.is_ok());
            match begun {
                Ok(run) => Ok(Some(run)),
                Err(err) => {
                    self.could_not_begin(live, &err).await;
                    Err(err)
                }
            }
        })
    }

    /// The same as the conductor starts, for an instance it was stopped
    /// before the first run of. The step that asked for it may be taken
    /// up at the same moment and come to it first: it is begun in its own
    /// turn, by whichever does ([`Engine::begin_owed`]). Where it cannot
    /// be begun the instance has failed, and the conductor starts.
    pub(crate) async fn begin_unbegun(&self, live: &Arc<Live>) {
        let _turn = live.apply_lock.lock().await;
        let _ = self.begin_owed(live, "start").await;
    }

    /// Goes on with the steps of an instance whose run was under way when
    /// the conductor stopped; where that cannot be begun the instance has
    /// failed, and the conductor starts.
    pub(crate) async fn go_on(&self, live: &Arc<Live>, generation: u64, run: RunInfo) {
        if let Err(err) = self.start_main(live, generation, Some(run)).await {
            self.could_not_begin(live, &err).await;
        }
    }

    /// Puts an instance that was taking a text, and could not be begun on
    /// it, back on the one it had: true where it had one to go back to.
    /// Each thing here is the same done twice: stopped in the middle, it
    /// is still taking the text when a conductor starts, cannot be begun
    /// again, and comes here again.
    async fn gone_back(&self, live: &Arc<Live>, err: &Error) -> bool {
        let previous = live.previous.lock().expect("live lock").clone();
        let Some(previous) = previous else { return false };
        let meta = live.meta();
        let Ok(blueprint) = self.compile(&meta.blueprint, &previous.source, meta.project.as_deref()) else {
            return false;
        };
        live.previous.lock().expect("live lock").take();
        let message = format!("update failed, still running the previous version: {}", err.message);
        self.give_back(live, &previous).await;
        let store = &self.inner.store;
        let handlers = blueprint.triggers.iter().map(|trigger| trigger.name.clone()).collect();
        let _ = store.keep_places(&live.id, handlers).await;
        live.redefine(blueprint, previous.clone(), previous.state.clone());
        trigger::prepare(self, live, None).await;
        // Said by whoever applied the version gone back to.
        let said = json!({ "error": err.message, "kept": live.meta().revision });
        let says = vec![to_say("instance.update_failed", self.instance_source(live), said)];
        let now = self.inner.own.now_ms();
        let looks = previous.before.as_ref().is_some_and(|stood| stood.below);
        if store.go_back(&live.id, previous, message, now, says).await.unwrap_or(false) {
            trigger::arm(self, live, Vec::new());
            // A look owed from before what could not be begun is owed still.
            if looks {
                live.owes_look(live.generation.load(std::sync::atomic::Ordering::SeqCst));
                self.look_below(live).await;
            }
        }
        true
    }

    /// Settles as failed an instance that could not be begun, says so,
    /// and stops what was brought up for it before it failed. One that
    /// was running and was taking another text goes back to what it had.
    pub(crate) async fn could_not_begin(&self, live: &Arc<Live>, err: &Error) {
        if self.gone_back(live, err).await {
            return;
        }
        let error = format!("{COULD_NOT}{}", err.message);
        let now = self.inner.own.now_ms();
        let store = &self.inner.store;
        let source = self.instance_source(live);
        let key = live.meta().key;
        // A step that called it fails with what its apply answered: said
        // here too, so that whoever tells people says it once.
        let (by, lets) = self.inner.calls.of(live.meta().parent.as_deref(), &live.meta().key).unzip();
        let failed = json!({
            "error": error, "reason": crate::why::reason(&error), "called_by": by, "let_be": lets,
        });
        // Written as failed and said to be, in one write. Stopped
        // meanwhile, it is stopped, and whoever stopped it said so.
        let says = vec![to_say("instance.failed", source.clone(), failed)];
        if !store.fail_instance(&live.id, error.clone(), true, now, says).await.unwrap_or(false) {
            return;
        }
        self.teardown(&live.id).await;
        let said = json!({ "status": "failed", "error": error, "key": key });
        let _ = store.taken_down(&live.id, vec![to_say("instance.finished", source, said)]).await;
    }
}

impl Engine {
    /// Starts (or resumes) the main flow as a task and returns its run id.
    ///
    /// Its run is written down and its task begun as one thing, done
    /// whole once it is begun whoever was waiting for it
    /// ([`crate::whole`]): a step that asked for a child and was cut short
    /// in the moment its run was written (what the child was started
    /// under being stopped) left that run written as under way with
    /// nothing to run it, and nothing ever ended it. Begun, it is cut as
    /// anything of a stopped instance is, and ends saying so.
    pub(crate) async fn start_main(
        &self,
        live: &Arc<Live>,
        generation: u64,
        resume: Option<RunInfo>,
    ) -> Result<String> {
        let (engine, live) = (self.clone(), live.clone());
        Whole::of(self, async move { engine.main_begun(&live, generation, resume).await }).await
    }

    async fn main_begun(&self, live: &Arc<Live>, generation: u64, resume: Option<RunInfo>) -> Result<String> {
        let (blueprint, principal) = live.acting();
        let taken_up = resume.is_some();
        let (run_id, prior) = match resume {
            Some(run) => (run.id, run.steps),
            None => {
                let id = self.inner.own.new_id("run");
                self.inner
                    .store
                    .insert_run(RunInfo {
                        id: id.clone(),
                        instance: live.id.clone(),
                        flow: "main".into(),
                        revision: live.meta().revision,
                        status: RunStatus::Running,
                        event: Value::Null,
                        outputs: Value::Null,
                        error: None,
                        started: self.inner.own.now_ms(),
                        finished: None,
                        steps: Vec::new(),
                    })
                    .await?;
                (id, Vec::new())
            }
        };
        // Taken up with no step written down, it is taken up all the same
        // where it had said it started: what it does as it ends goes on
        // from where it was, and it does not say it started a second time.
        let resumed = !prior.is_empty() || (taken_up && run::said_started(self, &run_id).await);
        let cx = Arc::new(RunCx {
            engine: self.clone(),
            live: live.clone(),
            blueprint,
            principal,
            run_id: run_id.clone(),
            flow_id: "main".into(),
            event: Value::Null.into(),
            cancel: live.work().child(),
            generation: Some(generation),
            labels: live.meta().labels,
            under: String::new(),
        });
        // Started by a step of a run that is handling an event, its steps
        // are part of that event's chain, and so is what it says as it
        // ends: a handler that starts a child whenever one finishes is
        // going round as surely as one that answers itself.
        let chain = kernel::chain::current();
        self.spawn(kernel::chain::within(chain, async move {
            run::run_flow(cx, prior, resumed).await;
        }));
        Ok(run_id)
    }
}
