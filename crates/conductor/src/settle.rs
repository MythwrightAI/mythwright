//! How an instance's steps end, and how an instance does.
//!
//! When the steps of an instance finish, what becomes of it is written
//! down with how the run ended and said in the same write, so that nobody
//! who hears of the run reads an instance that has not yet caught up
//! with it, and a conductor stopped then leaves all of it or none. Steps that went well
//! leave it active (it has handlers, holds something or has children) or
//! done. Steps that failed leave it failed and take down what it started,
//! unless they were an update's: then it goes back to the text, the
//! authority and the state it had, hands back what the update started,
//! and stays up. One active for its children alone is done when the
//! last of them has ended, whichever came first ([`Engine::rests`]).
//!
//! Stopping an instance ends its work, stops its children first, and hands
//! back what it held.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use journal::EventQuery;
use kernel::{BoxFut, Error, Result, Source};
use runs::{Becomes, Definition, InstanceFilter, InstanceInfo, InstanceStatus, RunStatus, to_say};
use serde_json::json;

use crate::engine::{Engine, Live};
use crate::whole::Whole;
use crate::{run, trigger};

/// What an instance became when its steps finished.
pub(crate) enum Settled {
    Failed(String),
    /// An update failed; the previous definition is back in force, and
    /// what is under it is owed a look still where it was before.
    Reverted(bool),
    Active,
    Done,
}

impl Engine {
    /// Works out what becomes of an instance now that its steps have
    /// finished, and what is said of it: for the one write that ends the
    /// run (`together::Ending`). `None` if this run was superseded by a
    /// re-apply or a stop and no longer speaks for the instance.
    ///
    /// What the instance becomes is written with how its run ended and
    /// said in the same write, so that nobody who hears that the run
    /// finished reads a stale instance, and a conductor stopped there
    /// leaves all of it or none. What has to be in place before that is
    /// done here, each thing the same done twice: stopped between this
    /// and the write, the run is taken up when the conductor starts, ends
    /// again, and comes here again.
    pub(crate) async fn settling(
        &self,
        live: &Arc<Live>,
        generation: u64,
        ran: &str,
        outcome: &run::Outcome,
        called: Option<(String, bool)>,
    ) -> Option<(Settled, Becomes)> {
        // Nor where a stop is under way: it is the stop's to end it, and
        // one the instance asked of itself as its last step may not have
        // called it off yet ([`crate::meets::Ahead`]).
        if live.generation.load(Ordering::SeqCst) != generation || self.inner.stops.under_way(&live.id) {
            return None;
        }
        let store = &self.inner.store;
        let source = self.instance_source(live);
        // Whether this run still speaks for the instance is asked again
        // in the write: what is done from here takes a while, and an
        // apply or a stop that comes meanwhile has it in hand.
        let speaks = || {
            let live = live.clone();
            Box::new(move || live.generation.load(Ordering::SeqCst) == generation)
                as Box<dyn Fn() -> bool + Send>
        };
        let becomes = |status, note, owes, says| Becomes {
            instance: live.id.clone(),
            status,
            note,
            update: None,
            owes,
            says,
            still: speaks(),
            then: None,
        };
        // What an update would go back to is left where it is until the
        // write that ends the update: an apply that comes meanwhile goes
        // by it (an update that may yet be undone is not added to).
        let previous = live.previous.lock().expect("live lock").clone();
        // The update is over in the write that says so, and what was in
        // hand to go back to is let go of there.
        let over = || {
            let live = live.clone();
            Box::new(move || *live.previous.lock().expect("live lock") = None) as Box<dyn FnOnce() + Send>
        };
        if outcome.status != RunStatus::Ok {
            let error = outcome.error.clone().unwrap_or_else(|| "the run did not finish".into());
            // A failed *update* does not take down what was working: the
            // instance goes back to its previous definition and stays up.
            let meta = live.meta();
            if let Some(previous) = previous
                && let Ok(blueprint) =
                    self.compile(&meta.blueprint, &previous.source, meta.project.as_deref())
            {
                let message = format!("update failed, still running the previous version: {error}");
                // Before it is written as gone back: stopped in between,
                // it is still an update under way when the conductor next
                // starts, fails again, and comes here again.
                self.give_back(live, &previous).await;
                let back = Some((Some(previous.clone()), Some(message)));
                let kept = previous.revision.clone();
                let looks = previous.before.as_ref().is_some_and(|stood| stood.below);
                // Said by whoever applied the version gone back to.
                let by = previous.principal.as_ref().map(|by| by.actor());
                let source = Source { actor: by.or(source.actor.clone()), ..source };
                let said = to_say("instance.update_failed", source, json!({ "error": error, "kept": kept }));
                // What is in hand goes back in the write that says so, and
                // not before it: with the authority that version was
                // applied with, not that of whoever applied the one that
                // failed, and with the state it had (what the failed text
                // dropped, and what its steps set, are as they were).
                // Gone back in hand first, an apply that came while what
                // the update had started was being handed back had its
                // text taken out from under it.
                let to = live.clone();
                let then = Box::new(move || {
                    let state = previous.state.clone();
                    to.redefine(blueprint, previous, state);
                    *to.previous.lock().expect("live lock") = None;
                });
                // Written as gone back with how the run ended, and said:
                // where it did go back. Stopped meanwhile it is stopped,
                // and nothing is said to have failed that changed nothing.
                let gone_back =
                    Becomes { update: back, then: Some(then), ..becomes(None, None, false, vec![said]) };
                return Some((Settled::Reverted(looks), gone_back));
            }
            // A step that called it is told too, and fails with it:
            // said here, so that whoever tells people says it once.
            let (by, lets) = called.unzip();
            // Which run it was, so that whoever tells of it can open that run
            // at the step that failed, and not only what it was a run of.
            let failed = json!({
                "error": error, "reason": crate::why::reason(&error), "called_by": by, "let_be": lets, "run": ran,
            });
            let says = vec![to_say("instance.failed", source, failed)];
            let failing = becomes(Some(InstanceStatus::Failed), Some(error.clone()), true, says);
            return Some((Settled::Failed(error), failing));
        }
        // An update that went well is taken for good as its run ends,
        // in that write: what it would have gone back to is kept until
        // then, for a conductor that stops before it.
        let taken = previous.is_some().then_some((None, None));
        let has_children = store
            .instances(InstanceFilter {
                parent: Some(live.id.clone()),
                live_only: true,
                ..Default::default()
            })
            .await
            .is_ok_and(|children| !children.is_empty());
        if live.blueprint().is_long_lived() || !live.meta().holds.is_empty() || has_children {
            let note = self.inner.kept.said_by(self, &live.id).await;
            // The handlers it has are those of the text it now runs: one
            // an earlier text had and this one has not keeps no place.
            let handlers = live.blueprint().triggers.iter().map(|trigger| trigger.name.clone()).collect();
            let _ = store.keep_places(&live.id, handlers).await;
            // Fix where handlers start before anyone is told it is active.
            trigger::prepare(self, live, None).await;
            let says = vec![to_say("instance.active", source, json!({ "outputs": outcome.outputs }))];
            let active = Becomes {
                update: taken,
                then: previous.is_some().then(over),
                ..becomes(Some(InstanceStatus::Active), note, false, says)
            };
            Some((Settled::Active, active))
        } else {
            let said = json!({ "status": "done", "outputs": outcome.outputs, "key": live.meta().key });
            let says = vec![to_say("instance.finished", source, said)];
            let done = Becomes {
                update: taken,
                then: previous.is_some().then(over),
                ..becomes(Some(InstanceStatus::Done), None, false, says)
            };
            Some((Settled::Done, done))
        }
    }

    /// Hands back what a failed update started, so that the instance goes
    /// back to holding what it held and to the children it had. What it
    /// came to hold since the update began is stopped as it is when an
    /// instance stops, and so is each child made since. What it held
    /// before is not touched, even where the update's steps did something
    /// to it under the same name; and a child that was there and was given
    /// another text or other params by those steps was applied in its own
    /// right, and keeps what it was given.
    pub(crate) async fn give_back(&self, live: &Arc<Live>, previous: &Definition) {
        // One written down before this was kept says nothing of it.
        let Some(stood) = &previous.before else { return };
        let (kept, started): (Vec<String>, Vec<String>) =
            live.meta().holds.into_iter().partition(|hold| stood.holds.contains(hold));
        if !started.is_empty()
            && let Ok(Some(info)) = self.inner.store.instance(&live.id).await
        {
            let handed = InstanceInfo { holds: started, ..info };
            for hook in self.stop_hooks() {
                hook(handed.clone()).await;
            }
            live.meta.lock().expect("live lock").holds = kept.clone();
            let now = self.inner.own.now_ms();
            let _ = self.inner.store.set_instance_holds(&live.id, kept, now).await;
        }
        for child in self.inner.store.children_since(&live.id, stood.newest).await.unwrap_or_default() {
            let _ = self.stop(&child).await;
        }
    }

    /// Says that an instance is active, where a conductor of a program
    /// from before this stopped after writing that down and before saying
    /// it. Nothing else would ever say it, and what begins on an
    /// instance's own `instance.active` would never begin: it would sit
    /// alive, listening, with nothing started.
    ///
    /// Asked once, of each instance that was active when this program
    /// first opened the journal (`owes`, noted by the migration), by the
    /// first conductor to start on it. What this program writes as active
    /// it says in the same write, and nothing is looked for.
    ///
    /// Told from the journal. An instance says it is active straight after
    /// its steps' run is said to have finished, before anything else of
    /// its own: so one that has said nothing since the last thing kept of
    /// that run was never said to be active. One that has, was; and one
    /// whose run left nothing in the journal settled long ago. Said in the
    /// write that takes the note off, so once.
    pub(crate) async fn say_active_unsaid(&self, live: &Arc<Live>) {
        let Ok(Some(run)) = self.inner.store.last_main(&live.id).await else { return };
        if run.status != RunStatus::Ok {
            return;
        }
        let journal = &self.inner.own;
        let Ok(after) = self.inner.store.run_place(&run.id).await else { return };
        let of_run = EventQuery {
            run: Some(run.id.clone()),
            after,
            newest_first: true,
            limit: 1,
            ..Default::default()
        };
        // What the run said last, where the journal kept it, as a program
        // from before kept all of it; where it kept none of it (this
        // program passes on what a run says of itself), whether the
        // instance was said to be active since the run was written.
        let since = match journal.read(of_run).await.ok().and_then(|kept| kept.into_iter().next()) {
            Some(last) => EventQuery {
                instance: Some(live.id.clone()),
                after: last.seq,
                limit: 1,
                ..Default::default()
            },
            None => EventQuery {
                instance: Some(live.id.clone()),
                topics: "instance.active".into(),
                after,
                limit: 1,
                ..Default::default()
            },
        };
        if journal.read(since).await.is_ok_and(|said| said.is_empty()) {
            let source = self.instance_source(live);
            let says = vec![to_say("instance.active", source, json!({ "outputs": run.outputs }))];
            let _ = self.inner.store.taken_down(&live.id, says).await;
        }
    }

    /// Acts on what an instance became, once that is written and said
    /// ([`Engine::settling`], `Store::end_run`).
    ///
    /// What is done here is done to what is in hand, after the write. An
    /// apply or a stop that came since has the instance in hand (it began
    /// another generation), and what is its is left to it: its handlers
    /// are not started for it, and it is not let go of under it. What an
    /// instance that failed had started is taken down all the same: that
    /// is written, and owed.
    pub(crate) async fn settled(&self, live: &Arc<Live>, settled: Settled, generation: u64) {
        let still = || live.generation.load(Ordering::SeqCst) == generation;
        match settled {
            Settled::Failed(error) => {
                self.teardown(&live.id).await;
                // Finished is said after any ending, and last: what it
                // started is stopped and what it held is given back.
                let said = json!({ "status": "failed", "error": error, "key": live.meta().key });
                let over = to_say("instance.finished", self.instance_source(live), said);
                let _ = self.inner.store.taken_down(&live.id, vec![over]).await;
                self.rests(live.meta().parent);
            }
            Settled::Reverted(looks) if still() => {
                // A handler only the text that failed has, which was given
                // a place to listen from, has none in the text gone back to.
                let handlers = live.blueprint().triggers.iter().map(|trigger| trigger.name.clone()).collect();
                let _ = self.inner.store.keep_places(&live.id, handlers).await;
                trigger::prepare(self, live, None).await;
                trigger::arm_of(self, live, generation);
                if looks {
                    self.look_below(live).await;
                }
            }
            Settled::Active if still() => {
                trigger::arm_of(self, live, generation);
                self.look_below(live).await;
                // What it started may have ended since it was counted.
                self.rests(Some(live.id.clone()));
            }
            Settled::Done if still() => {
                self.forget(&live.id);
                self.rests(live.meta().parent);
            }
            Settled::Reverted(_) | Settled::Active | Settled::Done => {}
        }
    }

    /// Finishes taking down each instance that failed and that a
    /// conductor was stopped in the middle of taking down: what it
    /// started is stopped, what it held is given back, and it says that
    /// it is over. Asked as a conductor starts, before anything is taken
    /// up, of the rows that say this is owed (`Store::owing`).
    ///
    /// What such an instance had started may still be alive, and nothing
    /// is in hand yet. It is stopped as anything is stopped, and not
    /// merely written as stopped: brought into hand first, so that what
    /// its text says is done as it stops is done (`on_stop`). The runs it
    /// had under way are ended when everything has been looked at, as
    /// runs that were cancelled ([`Engine::end_left`]).
    pub(crate) async fn take_down_owed(&self) {
        for info in self.inner.store.owing("down").await.unwrap_or_default() {
            self.hand_below(&info.id).await;
            self.teardown(&info.id).await;
            let source = Source {
                instance: Some(info.id.clone()),
                actor: Some(info.principal.actor()),
                ..Source::default()
            };
            let said = json!({ "status": "failed", "error": info.error, "key": info.key });
            let _ =
                self.inner.store.taken_down(&info.id, vec![to_say("instance.finished", source, said)]).await;
        }
    }

    /// Finishes each stop that had begun when the conductor stopped:
    /// those of instances that do something as they stop, each of which
    /// notes that its stop is owed before the first of it
    /// (`run::run_stop_hooks`). Asked as a conductor starts, before
    /// anything is taken up. Such an instance had done part of what it
    /// does as it stops, and its work was called off: it is not brought
    /// back to go on as if nobody had asked. It is brought into hand to
    /// be stopped, with what is alive under it, and its stop goes on from
    /// the entry it had come to, said to be for the reason it was begun.
    pub(crate) async fn finish_stops(&self) {
        for info in self.inner.store.owing("stop").await.unwrap_or_default() {
            // Stopped already, under one finished before it: as it is now.
            let alive = self.inner.store.instance(&info.id).await;
            if !alive.is_ok_and(|now| now.is_some_and(|now| now.status.is_live())) {
                continue;
            }
            let why = match self.inner.store.kept_stop(&info.id).await.unwrap_or_default()["why"].as_str() {
                Some("parent") => "parent",
                Some("restarted") => "restarted",
                _ => "asked",
            };
            if self.live(&info.id).is_none()
                && let Ok(text) = self.compile(&info.blueprint, &info.source, info.project.as_deref())
            {
                self.register(&info, text);
            }
            self.hand_below(&info.id).await;
            let _ = self.stop_as(&info.id, why).await;
        }
    }

    /// Brings into hand everything alive under an instance, and under
    /// that, for a conductor that has just started and has to stop it
    /// before anything is taken up. None of it is begun or listens: it
    /// is in hand to be stopped, and is let go of as it is. One whose
    /// text no longer reads is left as it is, and is written as stopped
    /// with nothing done.
    fn hand_below<'a>(&'a self, id: &'a str) -> BoxFut<'a, ()> {
        Box::pin(async move {
            let filter =
                InstanceFilter { parent: Some(id.to_string()), live_only: true, ..Default::default() };
            for child in self.inner.store.instances(filter).await.unwrap_or_default() {
                if self.live(&child.id).is_none()
                    && let Ok(text) = self.compile(&child.blueprint, &child.source, child.project.as_deref())
                {
                    self.register(&child, text);
                }
                self.hand_below(&child.id).await;
            }
        })
    }

    /// Stops an instance, its children, and gives back what it held:
    /// because somebody, or a step, said to.
    pub fn stop(&self, id: &str) -> BoxFut<'_, Result<()>> {
        self.stop_as(id, "asked")
    }

    /// The same, saying why (`why` of `instance.stopped`): `asked`;
    /// `parent`, what it was started under has ended; `restarted`, it
    /// keeps nothing as it goes and the conductor was started again; or
    /// `withdrawn`, the key it was applied with was withdrawn.
    ///
    /// A stop is done whole once it has begun, whoever was waiting for
    /// it ([`crate::whole`]): a step that asked and was cut short, or a
    /// request let go of, left what it was stopping called off in hand,
    /// alive where it is written, with what it does as it stops half
    /// done, until a conductor next started.
    pub(crate) fn stop_as(&self, id: &str, why: &'static str) -> BoxFut<'_, Result<()>> {
        let (engine, id) = (self.clone(), id.to_string());
        Box::pin(Whole::of(self, async move { engine.stopping(id, why).await }))
    }

    async fn stopping(&self, id: String, why: &'static str) -> Result<()> {
        let Some(info) = self.inner.store.instance(&id).await? else {
            // Another conductor's: it is that one's to stop.
            return match self.elsewhere(&id).await {
                Some(other) => other.stop_for_another(&id, why).await,
                None => Err(Error::not_found(format!("there is no instance `{id}`"))),
            };
        };
        if !info.status.is_live() {
            return Ok(());
        }
        // Said to be under way from before anything is done until it
        // is written: an apply that comes to it meanwhile waits for
        // that, and one asked from inside it is refused
        // ([`crate::meets`]).
        let _stopping = self.inner.stops.begin(&id);
        let stop = async {
            if let Some(live) = self.live(&id) {
                // That the stop has begun is kept for the top of
                // what is stopped, whatever it does as it stops:
                // what is under it has been called off, and a
                // conductor that starts finishes the stop before it
                // takes anything up ([`Engine::finish_stops`]).
                // Brought back instead, what was under it listened
                // again while what it had been stopped beside went
                // on being stopped.
                if self.call_off(&live, true) {
                    let _ = self.inner.store.save_stop(&id, json!({ "why": why })).await;
                }
                run::run_stop_hooks(self, &live, why).await;
            }
            self.teardown(&id).await;
            let now = self.inner.own.now_ms();
            // Written as stopped and said to be, and to be over: one write.
            let stopped = json!({ "blueprint": info.blueprint, "why": why });
            let over = json!({ "status": "stopped", "key": info.key });
            let says = vec![
                to_say("instance.stopped", Source::instance(&id), stopped),
                to_say("instance.finished", Source::instance(&id), over),
            ];
            self.inner.store.stop_instance(&id, now, says).await
        };
        let stopped = crate::meets::within(&id, stop).await;
        // Stopped for its parent's stopping, the parent is ending anyway.
        if why != "parent" {
            self.rests(info.parent);
        }
        stopped
    }

    /// One with no handlers that holds nothing is active for its children
    /// alone, and done once the last has ended, whichever came first
    /// (`docs/architecture.md`, *Instances and runs*): decided in the
    /// write (`Store::rest_instance`), as a task nothing waits for.
    pub(crate) fn rests(&self, id: Option<String>) {
        let Some(id) = id else { return };
        let engine = self.clone();
        self.spawn(async move { engine.resting(id).await });
    }

    fn resting(&self, id: String) -> BoxFut<'_, ()> {
        Box::pin(async move {
            let Some(live) = self.live(&id) else { return };
            if live.blueprint().is_long_lived() || !live.meta().holds.is_empty() {
                return;
            }
            // No apply part of the way through it; one after is looked for.
            let turn = live.apply_lock.clone().lock_owned().await;
            let generation = live.generation.load(Ordering::SeqCst);
            let ran = self.inner.store.last_main(&id).await.ok().flatten();
            let said =
                json!({ "status": "done", "outputs": ran.map(|run| run.outputs), "key": live.meta().key });
            let says = vec![to_say("instance.finished", self.instance_source(&live), said)];
            let (engine, here, now) = (self.clone(), live.clone(), self.inner.own.now_ms());
            let still = Box::new(move || {
                here.generation.load(Ordering::SeqCst) == generation
                    && !engine.inner.stops.under_way(&here.id)
            });
            let rested = self.inner.store.rest_instance(&id, now, still, says).await.unwrap_or(false);
            drop(turn);
            if rested {
                self.forget(&id);
                if let Some(parent) = live.meta().parent {
                    self.resting(parent).await;
                }
            }
        })
    }

    /// Stops an instance's children and hands back its holds. The instance's
    /// own status is the caller's to set.
    pub(crate) fn teardown<'a>(&'a self, id: &'a str) -> BoxFut<'a, ()> {
        Box::pin(async move {
            // Before its children are counted: from here nothing more is
            // started under it (`Store::insert_instance`).
            if let Some(live) = self.live(id) {
                self.call_off(&live, false);
                // Runs of its handlers that a conductor's start kept for
                // them, and that none had taken up, end with it.
                for run in trigger::unclaimed(&live) {
                    let _ = self.end_left(&run, "cancelled", self.inner.own.now_ms()).await;
                }
            }
            let children = self
                .inner
                .store
                .instances(InstanceFilter {
                    parent: Some(id.to_string()),
                    live_only: true,
                    ..Default::default()
                })
                .await
                .unwrap_or_default();
            for child in children {
                let _ = self.stop_as(&child.id, "parent").await;
            }
            if let Ok(Some(info)) = self.inner.store.instance(id).await {
                let hooks = self.stop_hooks();
                for hook in hooks {
                    hook(info.clone()).await;
                }
            }
            if let Some(live) = self.live(id) {
                live.cancel.cancel();
            }
            self.forget(id);
        })
    }

    /// Stops `id`, which this conductor keeps, for another that asked:
    /// as a task of this one's own, once it has taken up what it keeps,
    /// so that the instance is stopped in hand and the stop is cut
    /// short where this conductor is stopped, not where whoever asked
    /// is. It is then asked again of the one started in this one's
    /// place, which finishes a stop that had begun as it starts.
    ///
    /// Asked by what a start does itself (something being stopped as
    /// the factory went down), it is asked and not waited for: this
    /// conductor may start after that one (`crate::set::standing`).
    async fn stop_for_another(&self, id: &str, why: &'static str) -> Result<()> {
        let mut keeper = self.clone();
        loop {
            let (done, answer) = tokio::sync::oneshot::channel();
            let (theirs, id) = (keeper.clone(), id.to_string());
            let chain = kernel::chain::current();
            keeper.spawn(kernel::chain::within(chain, async move {
                let _ = done.send(theirs.when_up().await.stop_as(&id, why).await);
            }));
            if Engine::is_within_a_start() {
                return Ok(());
            }
            if let Ok(answered) = answer.await {
                return answered;
            }
            let next = keeper.when_up().await;
            if Arc::ptr_eq(&next.inner, &keeper.inner) {
                return Err(Error::unavailable(
                    "the conductor was stopped as it was asked to stop an instance",
                ));
            }
            keeper = next;
        }
    }

    /// Waits for a run to finish and returns it.
    pub async fn wait_run(&self, id: &str) -> Result<crate::RunInfo> {
        loop {
            // Stopped alone, this conductor ends no run: it is waited
            // for where the one started in its place has taken it up.
            let now = self.up(self.clone()).await;
            if !Arc::ptr_eq(&now.inner, &self.inner) {
                return Box::pin(now.wait_run(id)).await;
            }
            // Register before looking, so a finish in between is not missed.
            let (done, finished) = tokio::sync::oneshot::channel();
            self.inner.waiters.lock().expect("engine lock").entry(id.to_string()).or_default().push(done);
            let Some(run) = self.run_here(id).await? else {
                // Another conductor's run: nobody here will ever say it
                // has ended, so it is waited for where it is.
                self.inner.waiters.lock().expect("engine lock").remove(id);
                return match self.elsewhere_run(id).await {
                    Some(other) => Box::pin(other.wait_run(id)).await,
                    None => Err(Error::not_found(format!("there is no run `{id}`"))),
                };
            };
            if run.status != RunStatus::Running {
                return Ok(run);
            }
            let _ = finished.await;
        }
    }

    pub(crate) fn notify_run_finished(&self, id: &str) {
        if let Some(waiting) = self.inner.waiters.lock().expect("engine lock").remove(id) {
            for waiter in waiting {
                let _ = waiter.send(());
            }
        }
    }
}
