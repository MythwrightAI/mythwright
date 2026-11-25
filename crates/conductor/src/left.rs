//! A run left under way by a conductor that stopped, which nothing will
//! take up: its instance is over.
//!
//! An instance that is stopped, or fails, cancels the runs it has under
//! way, and each of them ends itself: the step it was in is written as
//! cancelled and says so, the steps it never came to are written as
//! cancelled, and the run says that it finished. That takes a moment, and
//! the instance is written as over without waiting for it. A conductor
//! that stops in that moment, or before the instance was stopped at all
//! and is started to find it must be (what an instance that failed had
//! started, [`Engine::take_down_owed`]), leaves such a run written as
//! under way with nobody to end it.
//!
//! It is ended here as it would have ended itself, in one write: so that
//! what is read of it afterwards is what is read of a run that was
//! cancelled with nothing in its way, whoever heard `run.finished` for
//! every other run hears it for this one, and none of its steps is left
//! reading as under way for good. Which steps those are, and what each is
//! written as, is a function of the text and the steps written
//! ([`runs::left`]).

use kernel::{Result, Source};
use runs::left::{cancelled, flow_of};
use runs::{Ending, RunInfo, RunStatus, to_say};
use serde_json::{Value, json};

use crate::engine::Engine;

impl Engine {
    /// Ends a run that was under way and that nothing will take up, as a
    /// run ends that is cancelled while it goes, and tells whoever waits
    /// on it. `why` is what it says it ended with: that it was cancelled,
    /// for one whose instance is over; that its handler is no longer in
    /// the text, for one a text left behind.
    pub(crate) async fn end_left(&self, run: &RunInfo, why: &str, now: i64) -> Result<()> {
        let store = &self.inner.store;
        let whole = store.run(&run.id).await?.unwrap_or_else(|| run.clone());
        let of = store.instance(&run.instance).await?;
        let text = of
            .as_ref()
            .and_then(|info| self.compile(&info.blueprint, &info.source, info.project.as_deref()).ok());
        let actor = of.as_ref().map(|info| info.principal.actor());
        // Said as the run would have said it: taking what the event that
        // set it off took ([`crate::run::took`]).
        let took = crate::run::took(&whole.event);
        let source = |step: Option<&str>| Source {
            device: None,
            instance: Some(whole.instance.clone()),
            run: Some(whole.id.clone()),
            step: step.map(str::to_string),
            actor: actor.clone(),
            takes: took.clone(),
        };
        // A run of something that keeps nothing as it goes wrote down no
        // step, and none is made up for it now.
        let (steps, mut says) = match text.as_deref().filter(|text| text.durable) {
            Some(text) => cancelled(&whole, flow_of(text, &whole.flow), now, &source),
            None => (Vec::new(), Vec::new()),
        };
        let blueprint = match (&text, &of) {
            (Some(text), _) => text.name.clone(),
            (None, Some(info)) => info.blueprint.clone(),
            (None, None) => String::new(),
        };
        let error = why;
        let ended = json!({
            "flow": whole.flow,
            "blueprint": blueprint,
            "status": RunStatus::Cancelled.as_str(),
            "duration_ms": now - whole.started,
            "error": error,
        });
        says.push(to_say("run.finished", source(None), ended));
        let ending = Ending {
            run: whole.id.clone(),
            status: RunStatus::Cancelled,
            outputs: Value::Null,
            error: Some(error.into()),
            now,
            steps,
            says,
            becomes: None,
        };
        store.end_run(ending).await?;
        self.notify_run_finished(&whole.id);
        Ok(())
    }
}

/// What an instance that keeps nothing says it ended with when a
/// conductor starts and finds it there.
const KEPT_NOTHING: &str =
    "it keeps nothing across a restart, and the conductor was started again; apply it to run it";

impl Engine {
    /// An instance that keeps nothing as it goes, found alive by a
    /// conductor that starts: it is over, with what it started and what
    /// it held, and the runs it had under way with it.
    ///
    /// That it is stopped, why, how each of its runs ended and all that
    /// is said of it are one write. They were three (the instance as
    /// stopped, then why, then its runs, of which nothing was said), and
    /// a conductor stopped a second time between them came back to an
    /// instance stopped with nothing said of why, or to runs of it that
    /// whoever had called it took for under way.
    ///
    /// Asked before anything that is kept goes on. A `call` that was
    /// under way looks at the run of the child it had made to know
    /// whether to wait for it or to call again (`child::had`), so that
    /// run is ended here, with the instance, and whoever waits on it is
    /// told.
    pub(crate) async fn over_at_start(&self, info: &crate::InstanceInfo) -> Result<()> {
        self.teardown(&info.id).await;
        let now = self.inner.own.now_ms();
        let actor = Some(info.principal.actor());
        let mut runs = Vec::new();
        for run in self.inner.store.unfinished_runs().await? {
            if run.instance != info.id {
                continue;
            }
            let source = Source {
                device: None,
                instance: Some(info.id.clone()),
                run: Some(run.id.clone()),
                step: None,
                actor: actor.clone(),
                takes: crate::run::took(&run.event),
            };
            let ended = json!({
                "flow": run.flow,
                "blueprint": info.blueprint,
                "status": RunStatus::Cancelled.as_str(),
                "duration_ms": now - run.started,
                "error": KEPT_NOTHING,
            });
            runs.push((run.id, to_say("run.finished", source, ended)));
        }
        let ids: Vec<String> = runs.iter().map(|(id, _)| id.clone()).collect();
        let stopped = json!({ "blueprint": info.blueprint, "why": "restarted" });
        let over = json!({ "status": "stopped", "key": info.key });
        let says = vec![
            to_say("instance.stopped", Source::instance(&info.id), stopped),
            to_say("instance.finished", Source::instance(&info.id), over),
        ];
        self.inner.store.stop_with_its_runs(&info.id, KEPT_NOTHING.into(), now, runs, says).await?;
        for id in ids {
            self.notify_run_finished(&id);
        }
        Ok(())
    }
}
