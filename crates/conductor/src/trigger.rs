//! Handlers: what an instance does while it is alive.
//!
//! Each `on` handler gets one task. An event handler has a place in the
//! journal and is handed what is for it from there on ([`crate::delivery`]),
//! so events that happen while the conductor is down are handled when it
//! comes back, each exactly once: its place moves in the same transaction
//! that creates the handler's run. Runs of one handler never overlap; a
//! burst of events queues behind the run in progress.
//!
//! A handler is written in one text and stays in it. When its instance is
//! given another text where it is ([`crate::current`]), each handler is
//! told to stop taking events and ends when it is next between runs; the
//! new text's handler of the same name waits for that and then listens
//! from where the old one had got to. So a run under way finishes on the
//! text it began with, and two runs of one handler still never overlap.
//!
//! A poller is the only thing here that runs on a timer against the outside
//! world. It is for sources that cannot push, it has a floor on its interval,
//! it backs off when the source fails, and every poll is measured, so a
//! poller that costs too much is visible rather than a mystery.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use blueprint::{Address, Blueprint, Trigger, TriggerScope, TriggerSource, template};
use journal::{Journal, Places};
use kernel::{Cancel, Chain, Event, Fields, Principal};
use runs::{RunInfo, RunStatus};
use serde_json::{Value, json};

use crate::delivery::{Inboxes, PLACES_KEPT, Wanted};
use crate::engine::{Engine, Live};
use crate::run::{self, RunCx};
use crate::turns::Again;

mod at;
mod places;
mod polled;
mod started;

pub(crate) use places::{begins, places, places_from, prepare};
pub(crate) use started::{arm, arm_of, leave, rearm, retire, unclaimed};

/// How far a handler's place may fall behind what it has been handed and
/// passed over before it is written, so a restart never has to hand it
/// more than this stretch of the journal that turned out not to matter.
const CURSOR_SAVE_EVERY: u64 = 256;

/// Whether a trigger hears every project's events, and so every journal.
pub(crate) fn everywhere(trigger: &Trigger) -> bool {
    trigger.scope == TriggerScope::Fleet
}

/// The handlers of an instance that are listening.
#[derive(Default)]
pub(crate) struct Listening {
    /// Whether the instance's handlers have been started since it was last
    /// applied. Until they are, nothing here is to be handed over: whoever
    /// starts them reads the text then in force.
    armed: bool,
    heard: HashMap<String, Heard>,
    /// Handlers the text in force no longer has, until their places in the
    /// journal have been taken away. One that a later text brings back
    /// before then is found here by the handler that takes its name.
    gone: HashMap<String, Heard>,
    /// Runs of its handlers that were under way when the conductor
    /// stopped, where the instance was not listening as it was taken up
    /// ([`leave`]): theirs when its handlers are next started.
    left: Vec<RunInfo>,
    /// Told to take no more by whoever is to carry the instance over
    /// ([`retire`]): none is started until that has it in hand (another
    /// generation) or lets it be ([`rearm`]). One that had only just
    /// been written as active had none yet to be told: they were started
    /// a moment later, heard that the text was taken, and were ended
    /// with the generation in the middle of that. What they had begun
    /// was cancelled, and its event was nobody's.
    retired: bool,
    /// Whether a starting was held back for that, which `rearm` then
    /// owes where the text was not taken after all; and since when, for
    /// the measurement ([`LATE`]).
    owed: bool,
    late: Option<i64>,
}

/// How long the starting of an instance's handlers was held back for a
/// text it was being carried over to, asked for as it became active. It
/// is all that says this order of the two came about.
pub(crate) const LATE: &str = "instance.listening_late";

/// One handler, as whatever takes its place knows it.
#[derive(Clone, Default)]
struct Heard {
    /// Cancelled to have it take no more events.
    retire: Cancel,
    /// Cancelled by the handler itself when it has ended.
    ended: Cancel,
    /// Its recent runs, left here as it ends for the one that follows it.
    left: Arc<Mutex<VecDeque<String>>>,
    /// Cancelled where the text dropped it and a later text has a handler
    /// of its name again: its place is then not taken away after it, since
    /// by then it is that handler's.
    back: Cancel,
    /// Cancelled when what is done with its place once the text has
    /// dropped it is done: taken away, or left to the one that came back.
    cleared: Cancel,
}

impl Heard {
    /// Whether this is that very handler, and not another of its name.
    fn is(&self, other: &Heard) -> bool {
        Arc::ptr_eq(&self.left, &other.left)
    }
}

/// `fresh` is said by whoever has an instance that is listening go over
/// to another text ([`rearm`]): the place a handler that text brings, and
/// the one before it had not, starts from. `of` is the generation whose
/// steps having settled is why they are started, where that is why.
fn listen(
    engine: &Engine,
    live: &Arc<Live>,
    mut unfinished: Vec<RunInfo>,
    fresh: Option<Places>,
    of: Option<u64>,
) {
    // The text is read where nobody else is starting handlers, so that of
    // two who start them the later one has the later text.
    let mut listening = live.listening.lock().expect("live lock");
    let another = of.is_some_and(|of| live.generation.load(Ordering::SeqCst) != of);
    if another || (fresh.is_some() && !listening.armed) {
        return;
    }
    if fresh.is_none() {
        let now = engine.own().now_ms();
        if listening.retired {
            // Whoever carries it over starts them, in the text it takes.
            listening.left.append(&mut unfinished);
            listening.owed = true;
            listening.late.get_or_insert(now);
            return;
        }
        if let Some(since) = listening.late.take() {
            let text = live.blueprint();
            let waited = (now - since).max(0) as f64;
            engine.own().measure(LATE, &[("blueprint", text.name.as_str())], waited, true);
        }
    }
    // What was kept for them from the conductor's start is theirs now.
    if fresh.is_none() {
        unfinished.append(&mut listening.left);
    }
    let (blueprint, principal) = live.acting();
    let cancel = live.work().child();
    let before = std::mem::take(&mut listening.heard);
    for heard in before.values() {
        heard.retire.cancel();
    }
    listening.armed = true;
    let by_key = |run: &&RunInfo| {
        blueprint.triggers.iter().any(|one| one.turn.is_some() && run.flow == format!("on.{}", one.name))
    };
    // Each handler takes up its own in this order too, so that none waits
    // here for one that is further down its own list.
    unfinished.sort_by_key(Again::first);
    let again = Again::of(unfinished.iter().filter(by_key));
    for trigger in blueprint.triggers.clone() {
        let flow_id = format!("on.{}", trigger.name);
        let (mine, rest): (Vec<RunInfo>, Vec<RunInfo>) =
            unfinished.into_iter().partition(|run| run.flow == flow_id);
        unfinished = rest;
        let heard = Heard::default();
        listening.heard.insert(trigger.name.clone(), heard.clone());
        // One the text before had goes on from where that one had got to.
        // One it had not is the text's to begin: from the place it was
        // given, whatever a handler of its name that an earlier text
        // dropped left behind. That one may not have ended yet, and its
        // place may not have been taken away: it is told that its name is
        // taken, and is waited for.
        let (after, fresh) = match before.get(&trigger.name) {
            Some(before) => (Some(before.clone()), None),
            None => {
                let dropped = listening.gone.remove(&trigger.name);
                if let Some(dropped) = &dropped {
                    dropped.back.cancel();
                }
                (dropped, fresh.clone())
            }
        };
        let handler = Handler {
            engine: engine.clone(),
            live: live.clone(),
            blueprint: blueprint.clone(),
            principal: principal.clone(),
            after,
            fresh,
            trigger,
            flow_id,
            cancel: cancel.clone(),
            heard,
            again: again.clone(),
            own_runs: VecDeque::new(),
        };
        engine.spawn(handler.run(mine));
    }
    // A handler the text in force no longer has gives up its place in the
    // journal once it has ended. Left there, it would be where a handler a
    // later text brings back under that name began, and that one would be
    // handed everything since.
    let gone: Vec<(String, Heard)> = before
        .into_iter()
        .filter(|(name, _)| !blueprint.triggers.iter().any(|trigger| &trigger.name == name))
        .collect();
    //
    // A later text may bring the name back before that: the handler it
    // then starts takes this one out of `gone`, and the place is left to
    // it. Taken away after that, it would be the new handler's own place
    // that went; left as this one had it, the new handler would begin
    // where this one stopped, and be handed everything since.
    for (name, heard) in gone {
        listening.gone.insert(name.clone(), heard.clone());
        let (engine, live) = (engine.clone(), live.clone());
        engine.clone().spawn(async move {
            heard.ended.cancelled().await;
            if !heard.back.is_cancelled() {
                let _ = engine.inner.store.drop_places(&live.id, &name).await;
            }
            let mut listening = live.listening.lock().expect("live lock");
            if listening.gone.get(&name).is_some_and(|kept| kept.is(&heard)) {
                listening.gone.remove(&name);
            }
            drop(listening);
            heard.cleared.cancel();
        });
    }
    // A run of a handler the text in force no longer has cannot be carried
    // on by anything; it is ended, and says why.
    if !unfinished.is_empty() {
        let engine = engine.clone();
        engine.clone().spawn(async move {
            // As a run that was cancelled ends: its steps that were under
            // way say so, and it says that it finished (`Engine::end_left`).
            for run in unfinished {
                let why = "its handler is not in the text its instance now runs";
                let _ = engine.end_left(&run, why, engine.own().now_ms()).await;
            }
        });
    }
}

/// How deep a chain of events may go within [`CHAIN_WITHIN_MS`] before the
/// next of it is not handled: more rounds than any chain that does work
/// gets through in that time, and few enough to cost nothing to speak of.
pub(crate) const CHAIN_DEPTH: u32 = 100;
/// How long a chain is counted for. One that has taken longer than this
/// begins again from the next event, so a round that is slow goes on for
/// as long as it likes.
pub(crate) const CHAIN_WITHIN_MS: i64 = 10_000;

struct Handler {
    engine: Engine,
    live: Arc<Live>,
    /// The text this handler is written in and the authority it was applied
    /// with, as they were when it began to listen. Applying over the
    /// instance ends this handler and starts the new text's; until it has
    /// ended, what it does is still this text's, as whoever applied it.
    blueprint: Arc<Blueprint>,
    principal: Principal,
    trigger: Trigger,
    flow_id: String,
    cancel: Cancel,
    /// How it is told to take no more events, and says that it has ended.
    heard: Heard,
    /// The handler of the same name in the text before, when this one
    /// takes its place: it is waited for.
    after: Option<Heard>,
    /// Where it begins, when the text has just brought it to an instance
    /// that is listening: it says so itself once whatever had its name
    /// before has ended, so that nothing that one left is where it begins.
    fresh: Option<Places>,
    /// The order in which runs that take turns by a key and were under
    /// way when the conductor last stopped are given their turns again.
    again: Arc<Again>,
    /// Recent runs of this handler, so it does not react to its own events.
    own_runs: VecDeque<String>,
}

impl Handler {
    async fn run(mut self, unfinished: Vec<RunInfo>) {
        if self.took_over().await {
            for run in unfinished {
                self.own_runs.push_back(run.id.clone());
                let began = self.began_with(&run.revision).await;
                if let Some(key) = self.key(&run.event) {
                    // Its turn is by the key it came to, as it was: it is
                    // carried on beside the others that were under way,
                    // and this handler goes on to what it hears next. It
                    // is given its turn after those that had theirs before
                    // it, unless this handler is ended first: then it is
                    // given it as it comes, rather than not at all.
                    tokio::select! {
                        () = self.again.turn_of(&run.id) => {}
                        () = self.cancel.cancelled() => {}
                        () = self.heard.retire.cancelled() => {}
                    }
                    let id = run.id.clone();
                    let gone_on = run::gone_on(&self.engine, &run.id, &run.steps).await;
                    let cx = Arc::new(self.lasting(self.run_cx(run.id, run.event, began)));
                    self.live.turn.keyed.take(&self.engine, key, async move {
                        run::run_flow(cx, run.steps, gone_on).await;
                    });
                    self.again.given(&id);
                    continue;
                }
                // A run that was under way had its turn, and has it still.
                let Ok(_turn) = self.turn().await else { return };
                self.execute(run.id, run.event, run.steps, began).await;
            }
            match self.trigger.source.clone() {
                TriggerSource::Event(pattern) => self.follow(&pattern).await,
                TriggerSource::Every(every_ms) => self.tick(every_ms).await,
                TriggerSource::At(at) => self.at(&at).await,
                TriggerSource::Poll { every_ms, action, with } => self.poll(every_ms, &action, &with).await,
            }
        }
        *self.heard.left.lock().expect("handler lock") = std::mem::take(&mut self.own_runs);
        self.heard.ended.cancel();
    }

    /// Waits for the handler this one takes the place of to end, and takes
    /// what it knew of its own runs. False when the instance was stopped
    /// or applied over while waiting.
    async fn took_over(&mut self) -> bool {
        let Some(before) = self.after.take() else { return true };
        tokio::select! {
            () = before.ended.cancelled() => {}
            () = self.cancel.cancelled() => return false,
        }
        // One the text had dropped: what was to be done with its place is
        // done before this one says where it begins.
        if self.fresh.is_some() {
            tokio::select! {
                () = before.cleared.cancelled() => {}
                () = self.cancel.cancelled() => return false,
            }
        }
        self.own_runs = std::mem::take(&mut *before.left.lock().expect("handler lock"));
        true
    }

    /// The text a run of this handler began with and the authority that
    /// text had, when it is not the one in force: the instance took
    /// another text where it was while the run was under way, and the
    /// conductor was then stopped. The run is finished as it began.
    async fn began_with(&self, revision: &str) -> Option<(Arc<Blueprint>, Principal)> {
        let meta = self.live.meta();
        if revision == meta.revision {
            return None;
        }
        let versions = self.engine.inner.store.versions(&self.live.id, 1000).await.ok()?;
        let version = versions.into_iter().find(|version| version.revision == revision)?;
        let blueprint =
            self.engine.compile(&meta.blueprint, &version.source, meta.project.as_deref()).ok()?;
        // Only if that text still has this handler; otherwise it is the
        // text in force that finishes it.
        blueprint.triggers.iter().any(|trigger| trigger.name == self.trigger.name).then(|| {
            let principal = version.principal.unwrap_or_else(|| self.principal.clone());
            (blueprint, principal)
        })
    }

    /// Whether to take no more events: the instance has another text now.
    fn retired(&self) -> bool {
        self.heard.retire.is_cancelled()
    }

    fn scope(&self, event: &Value) -> Fields {
        let mut scope = self.engine.base_scope(&self.live);
        scope.insert("event".into(), event.clone());
        scope
    }

    async fn in_scope(&self, heard_in: &str, event: &Event) -> bool {
        let from = event.source.instance.as_deref();
        match (self.trigger.scope, from) {
            // Said aloud, the whole factory. With nothing said, what is
            // its own project's and what is nobody's (`crate::whose`).
            (TriggerScope::Fleet, _) => true,
            (TriggerScope::Project, _) => self.engine.within_reach(&self.live, heard_in, event).await,
            (TriggerScope::Own, Some(id)) => self.engine.is_within(id, &self.live.id).await,
            // An attachment watches its target, and it is itself part of its
            // target's tree. Leaving its own subtree out is what stops it
            // reacting to what it causes: a log feed must not grow a feed.
            (TriggerScope::Target, Some(id)) => match &self.live.meta().target {
                Some(target) => {
                    self.engine.is_within(id, target).await && !self.engine.is_within(id, &self.live.id).await
                }
                None => false,
            },
            (_, None) => false,
        }
    }

    /// Whether this event, heard in the journal called `heard_in`, should
    /// start a run.
    async fn wants(&self, heard_in: &str, event: &Event) -> Option<Value> {
        if !event.is_durable() || !self.in_scope(heard_in, event).await {
            return None;
        }
        // Being told is one of the things a credential allows: a handler
        // hears what whoever applied its text may read.
        let (_, by) = self.live.acting();
        if !self.engine.hears(&by, &self.live.id, event).await {
            return None;
        }
        if event.source.run.as_ref().is_some_and(|run| self.own_runs.contains(run)) {
            return None;
        }
        let value = serde_json::to_value(event).unwrap_or(Value::Null);
        match &self.trigger.when {
            Some(when) => when.test(&self.scope(&value)).unwrap_or(false).then_some(value),
            None => Some(value),
        }
    }

    /// What this handler's measurements are labelled with.
    fn labels(&self) -> Vec<(String, String)> {
        let meta = self.live.meta();
        vec![
            ("blueprint".to_string(), meta.blueprint),
            ("handler".to_string(), self.trigger.name.clone()),
            ("project".to_string(), meta.project.unwrap_or_default()),
        ]
    }

    /// What `for:` comes to for this instance. Only what is fixed for as
    /// long as the handler lives can be read there, which is what lets it
    /// be worked out once, here, and not for every event.
    fn address(&self) -> Result<Address, String> {
        let mut scope = self.engine.base_scope(&self.live);
        scope.retain(|name, _| matches!(name.as_str(), "params" | "instance" | "target"));
        Address::of(&self.trigger.address, &scope)
    }

    async fn follow(&mut self, pattern: &str) {
        let journal = self.engine.own().clone();
        let address = match self.address() {
            Ok(address) => address,
            Err(error) => {
                // Listening to everything instead would run it for events
                // that were never its; it listens to nothing, and says so.
                let error = format!(
                    "the handler `{}` hears nothing, because its `for` could not be worked out: {error}",
                    self.trigger.name
                );
                let source = kernel::Source::instance(&self.live.id);
                let data = json!({ "handler": self.trigger.name, "error": error });
                self.engine.emit("handler.failed", source, data).await;
                return;
            }
        };
        let (id, name) = (&self.live.id.clone(), &self.trigger.name.clone());
        let store = self.engine.inner.store.clone();
        let labels = self.labels();
        let wanted = |after: u64| Wanted {
            instance: id.clone(),
            handler: name.clone(),
            pattern: pattern.to_string(),
            address: address.clone(),
            after,
            labels: labels.clone(),
        };
        // It reads from each journal it hears, and has a place in each:
        // no number of one is ever its place in another.
        let fresh = self.fresh.take();
        // The factory's own journal is what says when another is made.
        let mut inbox = Inboxes::new(&self.engine.inner.factory, Vec::new());
        // Where its place in each was last written.
        let mut saved: HashMap<String, u64> = HashMap::new();
        self.join(&mut inbox, &mut saved, fresh.as_ref(), &wanted).await;
        let labels: Vec<(&str, &str)> =
            labels.iter().map(|(name, value)| (name.as_str(), value.as_str())).collect();
        // How long an event waited before this handler looked at it; how
        // many there are is how often the handler was woken.
        let woken = |event: &Event| {
            journal.measure("handler.event", &labels, (journal.now_ms() - event.ts).max(0) as f64, true);
        };
        // Writes its place in each journal where it has moved on by
        // `least` or more since it was last written. Kept for having
        // passed over that much, it is measured as such a keeping is.
        let keep = async |inbox: &Inboxes, saved: &mut HashMap<String, u64>, least: u64, but: &str| {
            for each in inbox.each().iter().filter(|each| each.journal().name() != but) {
                let (of, settled) = (each.journal().name(), each.settled());
                if settled >= saved.get(of).copied().unwrap_or(0) + least {
                    saved.insert(of.to_string(), settled);
                    let began = std::time::Instant::now();
                    let kept = store.set_place(id, name, of, settled).await;
                    if kept.is_ok() && least == CURSOR_SAVE_EVERY {
                        let took = began.elapsed().as_secs_f64() * 1000.0;
                        journal.measure(PLACES_KEPT, &[("kind", "passed")], took, true);
                    }
                }
            }
        };
        loop {
            // Whatever was in hand has a run recorded for it, or was not
            // wanted: from here its place can be moved without it.
            inbox.rest();
            if self.retired() {
                // What follows it starts from where this one had got to;
                // one that came back after the text had dropped this one
                // starts from its own place, which is by now what is kept.
                if !self.heard.back.is_cancelled() {
                    keep(&inbox, &mut saved, 1, "").await;
                }
                return;
            }
            let event = tokio::select! {
                event = inbox.next() => event,
                () = self.cancel.cancelled() => return,
                () = self.heard.retire.cancelled() => continue,
            };
            let Ok(event) = event else { return };
            // A journal was made: it may be one this handler hears.
            let Some(mut event) = event else {
                self.join(&mut inbox, &mut saved, None, &wanted).await;
                continue;
            };
            let heard_in = inbox.last().journal().clone();
            // Told to take no more in the moment this came. The two can be
            // ready together, and which is seen first is chance: so it is
            // asked again here. The event has no run and its place has not
            // moved, so it is the next handler's of this name, and nobody's
            // where the text now in force has none.
            if self.retired() {
                // Its place is left just before this one: everything
                // earlier was handed over or was not its.
                let before = event.seq.saturating_sub(1);
                if !self.heard.back.is_cancelled() {
                    if before > saved.get(heard_in.name()).copied().unwrap_or(0) {
                        let _ = store.set_place(id, name, heard_in.name(), before).await;
                    }
                    // And in every other, where it had got to.
                    keep(&inbox, &mut saved, 1, heard_in.name()).await;
                }
                return;
            }
            woken(&event);
            let Some(mut value) = self.wants(heard_in.name(), &event).await else {
                keep(&inbox, &mut saved, CURSOR_SAVE_EVERY, "").await;
                continue;
            };
            let mut heard_in = heard_in;
            // Debounce: keep taking matching events until it has been quiet
            // for the window, then run once with the latest.
            if let Some(quiet) = self.trigger.debounce_ms {
                let clock = journal.clock().clone();
                'burst: loop {
                    // The quiet starts again with each event that is wanted
                    // and with no other: what is not this handler's must not
                    // hold it back, whether it was passed over here or
                    // never handed to it at all.
                    let mut quiet = clock.sleep(Duration::from_millis(quiet));
                    loop {
                        tokio::select! {
                            () = &mut quiet => break 'burst,
                            next = inbox.next() => {
                                let Ok(next) = next else { return };
                                let Some(next) = next else {
                                    self.join(&mut inbox, &mut saved, None, &wanted).await;
                                    continue;
                                };
                                woken(&next);
                                let next_in = inbox.last().journal().clone();
                                if let Some(newer) = self.wants(next_in.name(), &next).await {
                                    (event, value) = (next, newer);
                                    heard_in = next_in;
                                    continue 'burst;
                                }
                            }
                            () = self.cancel.cancelled() => return,
                            // The event in hand has no run yet and its
                            // place has not moved: it is the next one's.
                            () = self.heard.retire.cancelled() => return,
                        }
                    }
                }
            }
            // What a burst took from another journal and let go for a
            // later event is not to be handed over again after a restart:
            // its place there is written before the run is.
            if self.trigger.debounce_ms.is_some() {
                keep(&inbox, &mut saved, 1, heard_in.name()).await;
            }
            // Its place in the journal the event is of moves with the run.
            let at = inbox.each().iter().find(|each| each.journal().name() == heard_in.name());
            let place = at.map_or(event.seq, |each| each.settled().max(event.seq));
            saved.insert(heard_in.name().to_string(), place);
            if let Some(chain) = self.runaway(&heard_in, &event) {
                self.refuse(&heard_in, &event, chain, place).await;
                continue;
            }
            self.start(value, &heard_in, place).await;
        }
    }

    /// Has the handler read from each journal it hears that it does not
    /// read from yet, from its place there: the one the text that brought
    /// it gave (`fresh`, written over whatever is kept under its name: a
    /// handler an earlier text dropped may have written its own last
    /// place after `prepare` looked), else the one that is kept. With
    /// none kept there and one kept in another journal, the journal was
    /// made since the handler was given its places, and is read from its
    /// beginning; with none kept anywhere its places are somehow gone,
    /// and it starts from now, not replaying what there is.
    async fn join(
        &self,
        inbox: &mut Inboxes,
        saved: &mut HashMap<String, u64>,
        fresh: Option<&Places>,
        wanted: &impl Fn(u64) -> Wanted,
    ) {
        let (id, name, store) = (&self.live.id, &self.trigger.name, &self.engine.inner.store);
        let mut hears = Vec::new();
        for journal in self.engine.hears_from(&self.live, everywhere(&self.trigger)) {
            if !inbox.has(journal.name()) {
                let kept = store.place(id, name, journal.name()).await.ok().flatten();
                hears.push((journal, kept));
            }
        }
        let somewhere = !inbox.each().is_empty() || hears.iter().any(|(_, kept)| kept.is_some());
        let there: HashMap<String, u64> =
            hears.iter().filter_map(|(journal, kept)| Some((journal.name().to_string(), (*kept)?))).collect();
        for (journal, kept) in hears {
            let cursor = match (fresh, kept) {
                (Some(fresh), _) => fresh.of(journal.name()),
                (None, Some(cursor)) => cursor,
                // One made since is read from its beginning; one its
                // project was moved to, from where this had got to in
                // the journal it was moved from (`Journal::joins`).
                (None, None) if somewhere => {
                    let kept_in = |from: &str| saved.get(from).or_else(|| there.get(from)).copied();
                    self.engine.inner.factory.joins(&journal, kept_in)
                }
                (None, None) => journal.last_seq(),
            };
            if fresh.is_some() || (kept.is_none() && somewhere) {
                let _ = store.set_place(id, name, journal.name(), cursor).await;
            }
            saved.insert(journal.name().to_string(), cursor);
            inbox.add(self.engine.inner.delivery.of(&journal).listen(&self.engine, wanted(cursor)));
        }
    }

    /// Where `event` is in a chain, when that is too deep in too short a
    /// time for the chain to be anything but handlers going round.
    fn runaway(&self, journal: &Journal, event: &Event) -> Option<Chain> {
        journal
            .chain_of(event.seq)
            .filter(|chain| chain.depth >= CHAIN_DEPTH && journal.now_ms() - chain.since <= CHAIN_WITHIN_MS)
    }

    /// Passes over an event at the end of a runaway chain, which ends the
    /// chain, and says so. Its place is moved past it at once: it is not
    /// to be come back to when the conductor next starts.
    async fn refuse(&self, journal: &Journal, event: &Event, chain: Chain, place: u64) {
        let (id, name) = (&self.live.id, &self.trigger.name);
        let _ = self.engine.inner.store.set_place(id, name, journal.name(), place).await;
        let within_ms = (self.engine.own().now_ms() - chain.since).max(0);
        let error = format!(
            "this `{topic}` is the {depth}th in a chain of events each said while the one before was being \
             handled, all within {within}: `{name}` and what it answers are going round without end. It was \
             not handled, which ends the chain. If every round is meant, give each something to wait for \
             (`wait`), or have one run go through the list (`for_each`).",
            topic = event.topic,
            depth = chain.depth,
            within = blueprint::duration::format(within_ms as u64),
        );
        let said = json!({
            "handler": name,
            "topic": event.topic,
            "seq": event.seq,
            "depth": chain.depth,
            "within_ms": within_ms,
            "error": error,
        });
        self.engine.emit("handler.runaway", kernel::Source::instance(id), said).await;
    }

    async fn tick(&mut self, every_ms: u64) {
        let clock = self.engine.own().clock().clone();
        loop {
            tokio::select! {
                () = clock.sleep(Duration::from_millis(every_ms)) => {}
                () = self.cancel.cancelled() => return,
                () = self.heard.retire.cancelled() => return,
            }
            let event =
                json!({ "topic": "timer", "ts": clock.now_ms(), "data": { "handler": self.trigger.name } });
            self.start(event, &self.engine.own().clone(), 0).await;
        }
    }

    /// What a run of this handler runs in: this handler's text and
    /// authority, or the ones the run `began` with.
    fn run_cx(&self, run_id: String, event: Value, began: Option<(Arc<Blueprint>, Principal)>) -> RunCx {
        let (blueprint, principal) =
            began.unwrap_or_else(|| (self.blueprint.clone(), self.principal.clone()));
        RunCx {
            engine: self.engine.clone(),
            live: self.live.clone(),
            blueprint,
            principal,
            run_id,
            flow_id: self.flow_id.clone(),
            event: event.into(),
            cancel: self.cancel.child(),
            generation: None,
            labels: self.live.meta().labels,
            under: String::new(),
        }
    }

    /// What a run that takes its turn by a key runs in. Such a run is not
    /// waited for when its instance takes another text where it is: the
    /// instance goes over at once, and what is in hand for a key is
    /// finished beside it, as the text it began in. So it is ended by its
    /// instance being stopped, and by nothing less.
    fn lasting(&self, cx: RunCx) -> RunCx {
        RunCx { cancel: self.live.cancel.child(), ..cx }
    }

    /// The key an event comes to, where this handler's text says what its
    /// events take turns by (`turn:`); nothing where it says none. A key
    /// that cannot be worked out from the event is the empty one, so that
    /// such events at least take turns with each other.
    fn key(&self, event: &Value) -> Option<String> {
        let turn = self.trigger.turn.as_ref()?;
        let key = template::render(&Value::String(turn.clone()), &self.scope(event));
        Some(match key {
            Ok(Value::String(key)) => key,
            Ok(Value::Null) | Err(_) => String::new(),
            Ok(other) => other.to_string(),
        })
    }

    /// Waits for this instance's turn, where its text says its handlers
    /// take turns (`one_at_a_time`), and holds it for as long as what is
    /// given back is kept. Nothing to wait for where it does not say so.
    /// An error when the handler is ended, or told to take no more, while
    /// it waits: then there is no turn to take.
    async fn turn(&self) -> Result<Option<tokio::sync::OwnedMutexGuard<()>>, ()> {
        if !self.blueprint.one_at_a_time {
            return Ok(None);
        }
        tokio::select! {
            turn = self.live.turn.whole.clone().lock_owned() => Ok(Some(turn)),
            () = self.cancel.cancelled() => Err(()),
            () = self.heard.retire.cancelled() => Err(()),
        }
    }

    /// Records a new run for `event` (moving its place in `heard_in`, the
    /// journal the event is of, to `seq` with it) and executes it.
    async fn start(&mut self, event: Value, heard_in: &Journal, seq: u64) {
        self.begin(event, heard_in, seq, None).await;
    }

    /// [`Handler::start`], for what a poll `read` too where one did, which
    /// is kept as what it last read in the write that makes the run.
    /// Whether what set it off is done with: begun, or turned down by its
    /// `when`; not where nothing was begun because the handler was ended
    /// or told to take no more, or the run could not be written, which
    /// leaves it to be come to again.
    async fn begin(&mut self, event: Value, heard_in: &Journal, seq: u64, read: Option<&Value>) -> bool {
        // One that takes its turn by a key is written down first and waits
        // for its turn afterwards (below): this handler must not wait with
        // it, or what comes to every other key would.
        let key = self.key(&event);
        // Given up while waiting, the event has no run and its place has
        // not moved: it is the next handler's of this name to take.
        let turn = match key {
            Some(_) => None,
            None => match self.turn().await {
                Ok(turn) => turn,
                Err(()) => return false,
            },
        };
        // What it waited behind may have changed what its own `when` reads:
        // it is asked again now that nothing else of this instance is running.
        if turn.is_some()
            && let Some(when) = &self.trigger.when
            && !when.test(&self.scope(&event)).unwrap_or(false)
        {
            if let Some(read) = read {
                let _ = self.engine.own().kv_set(runs::beside::POLL, &self.memory(), read).await;
            }
            return true;
        }
        // Ended, or told to take no more, by the time it comes to this:
        // it begins nothing. It may have been handed the event in the
        // moment it was ended, and which of the two it saw first was
        // chance; or been ended while it worked out whether the event
        // was its to take. The event has no run and its place has not
        // moved ([`crate::meets`]).
        if self.cancel.is_cancelled() || self.retired() {
            return false;
        }
        // The daemon is draining to stop: nothing new is begun, and this
        // handler takes nothing more. The event has no run and its place
        // has not moved, so the next daemon hands it over again.
        if self.engine.holds_new_work() {
            return std::future::pending().await;
        }
        let journal = self.engine.own();
        let run = RunInfo {
            id: journal.new_id("run"),
            instance: self.live.id.clone(),
            flow: self.flow_id.clone(),
            revision: self.live.meta().revision,
            status: RunStatus::Running,
            event: event.clone(),
            outputs: Value::Null,
            error: None,
            started: journal.now_ms(),
            finished: None,
            steps: Vec::new(),
        };
        let id = run.id.clone();
        let store = &self.engine.inner.store;
        let written = match read {
            Some(read) => store.insert_polled_run(run, &self.memory(), read).await,
            None => store.insert_triggered_run(run, &self.trigger.name, heard_in.name(), seq).await,
        };
        if written.is_err() {
            return false;
        }
        self.own_runs.push_back(id.clone());
        if self.own_runs.len() > 32 {
            self.own_runs.pop_front();
        }
        // What the run says is one deeper than the event it is for, in the
        // chain that event is part of; or the first of a chain, where the
        // event is part of none or of one that has taken its time.
        let now = journal.now_ms();
        let chain = event.get("seq").and_then(Value::as_u64).filter(|seq| *seq > 0).map(|seq| match heard_in
            .chain_of(seq)
        {
            Some(chain) if now - chain.since <= CHAIN_WITHIN_MS => {
                Chain { depth: chain.depth + 1, since: chain.since }
            }
            _ => Chain { depth: 1, since: now },
        });
        let cx = self.run_cx(id, event, None);
        let cx = Arc::new(if key.is_some() { self.lasting(cx) } else { cx });
        let run = kernel::chain::within(chain, run::run_flow(cx, Vec::new(), false));
        match key {
            // Done when everything that came to its key before it has been,
            // what had a turn when the conductor stopped first of all.
            Some(key) => {
                tokio::select! {
                    () = self.again.all_given() => {}
                    () = self.cancel.cancelled() => {}
                    () = self.heard.retire.cancelled() => {}
                }
                self.live.turn.keyed.take(&self.engine, key, async move {
                    run.await;
                })
            }
            None => {
                run.await;
                drop(turn);
            }
        }
        true
    }

    /// Carries on a run that was under way when the conductor last stopped.
    async fn execute(
        &self,
        run_id: String,
        event: Value,
        prior: Vec<runs::StepInfo>,
        began: Option<(Arc<Blueprint>, Principal)>,
    ) {
        let gone_on = run::gone_on(&self.engine, &run_id, &prior).await;
        let cx = Arc::new(self.run_cx(run_id, event, began));
        run::run_flow(cx, prior, gone_on).await;
    }
}
