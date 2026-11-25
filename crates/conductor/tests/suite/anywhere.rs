//! Stopped anywhere, and started again: the language itself.
//!
//! A blueprint is played once with nothing in its way, and what it came to
//! is written down: every instance and its state, how each run ended, each
//! step's status and what it gave, what was said and how often, and how
//! often each stand-in action was asked. Then it is played again once for
//! every place it passed, a place being a thing the conductor kept in its
//! journal, with the conductor stopped just after that place
//! ([`journal::Journal::shut_after`]) and started again on what the journal
//! then held. Each of those plays is held to coming to the same. And a
//! conductor that goes down while it is coming up is ordinary: each of
//! those plays is played again for the things the conductor kept from
//! that start until what it took up had passed the step it was in (until
//! a step began that was not one taken up, or the play came to rest with
//! nothing done to it from outside), stopped a second time just after
//! such a thing and started a third. From there on a stop is one in a run
//! that is going, which is what the first stop of some play is.
//!
//! Where a play is stopped a second time is found by that play, as it
//! goes ([`crate::stops`]), and never read from what another play kept:
//! a kind of second stop is known when a play comes to it, each kind is
//! played once for each pair of ways, and a play that comes to rest
//! before any thing it was told to be stopped at is a play stopped once
//! and is held as one. `ANYWHERE=everything` plays every one instead: for
//! each first stop and each pair of ways a play stopped at the first
//! thing its conductor then keeps, one at the second, and so on until one
//! comes to rest first.
//!
//! What may differ is said in one place ([`differs`]): an action that is
//! safe to do twice, whose step was under way when the conductor stopped,
//! may have been asked once more; and what the conductor says of its own
//! going on (`run.resumed`, a step said to have started again) is not what
//! a blueprint came to. Nothing else may.
//!
//! Nothing here waits on a clock. A play is driven from outside by what a
//! person would do (apply a text, say an event, let time pass), each thing
//! done when the play has come to rest: when nothing the conductor does
//! has changed anything, or asked its database anything, through several
//! turns of everything it has running. The database answers in the
//! order it is asked, and while a play waits to be at rest it is
//! answered a turn at a time ([`crate::stepped`]): what was asked is
//! answered, whatever is running goes on until it has asked again, and
//! so on. So a conductor that only reads for a while, on its way to
//! something it will write, is not taken to be at rest; and what a
//! conductor that is started again takes up goes forward the same way
//! on any machine, which is what lets a play be there for one order of
//! two things.
//!
//! This is the conductor's suite: what is played is steps, handlers, waits
//! and children, and names no piece of work.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use conductor::InstanceFilter;
use kernel::Event;
use serde_json::{Map, Value, json};

use crate::Rig;
use crate::stepped::Stepped;
use crate::stops::{Again, Passed};

mod abreast;
mod acts;

/// How many turns in a row have to change nothing for a play to be at rest.
pub(crate) const STILL: usize = 12;
/// How many things a play may do from outside before it is taken to be
/// going round for ever.
const MOVES: usize = 4_000;
/// What a step that cannot be done twice fails with when it was under way.
pub const NOT_TWICE: &str = "is not safe to repeat automatically";

/// Somewhere the conductor could be stopped: just after it kept something.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Place {
    pub topic: String,
    /// The blueprint of the instance that said it; nothing for what was
    /// said from outside.
    pub blueprint: String,
    /// Which of the instance's runs: `main`, `on.<handler>`, a hook.
    pub flow: String,
    pub step: String,
    /// Which time this was said so, from 1.
    pub nth: usize,
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}`", self.topic)?;
        if !self.blueprint.is_empty() {
            write!(f, " of {}", self.blueprint)?;
        }
        if !self.flow.is_empty() {
            write!(f, " ({})", self.flow)?;
        }
        if !self.step.is_empty() {
            write!(f, ", step `{}`", self.step)?;
        }
        if self.nth > 1 {
            write!(f, ", time {}", self.nth)?;
        }
        Ok(())
    }
}

/// How the conductor is stopped at a place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Way {
    /// The journal and its database take nothing after the place, as if
    /// the conductor had gone in the instant its event was kept.
    Shut,
    /// Nothing is said after the place, and what was being written down
    /// is written: each thing the conductor was doing goes on to the next
    /// thing it would say, and is stopped there. This is what stopping a
    /// daemon does to work that had asked for something to be written and
    /// was to say so afterwards.
    Unsaid,
}

impl fmt::Display for Way {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Way::Shut => "with nothing written after it",
            Way::Unsaid => "with what was being written kept and nothing said of it",
        })
    }
}

/// What has been kept so far, for whoever decides what to do next.
pub struct Seen(pub Vec<Event>);

impl Seen {
    /// How often something on `topic` was kept.
    pub fn said(&self, topic: &str) -> usize {
        self.0.iter().filter(|event| event.topic == topic).count()
    }

    /// How often `topic` was kept of the step called `step`.
    pub fn of_step(&self, topic: &str, step: &str) -> usize {
        self.0
            .iter()
            .filter(|event| event.topic == topic && event.source.step.as_deref() == Some(step))
            .count()
    }
}

/// Something done to a play from outside.
pub enum Does {
    /// A text is applied, as somebody would: again, if nothing came back.
    Apply(&'static str, Value),
    /// The same, for a project: what it starts is that project's.
    ApplyFor(&'static str, &'static str, Value),
    /// The same, under one of the project's environments, which the play
    /// makes before it begins, with a variable (`api`).
    ApplyUnder(&'static str, &'static str, &'static str, Value),
    /// The same, under none of them, with the one named made all the same:
    /// for what its steps start under it (`environment:`).
    ApplyBeside(&'static str, &'static str, &'static str, Value),
    /// A text is applied that cannot be begun: whoever applies it is
    /// told so, and that is no fault of the scenario's.
    Offer(&'static str, Value),
    /// A text is applied by somebody the play does not wait for: what
    /// they are answered is written down when it comes (`answered`). For
    /// an apply that waits on what only the play goes on to let happen.
    /// Asked again, looking first, where the conductor was stopped
    /// before it answered.
    Asks(&'static str, Value),
    /// An event is said, once: looked for first, by its `act`.
    Say(&'static str, Value),
    /// The instance applied under this blueprint's name is stopped.
    Stop(&'static str),
    /// The same, by somebody the play does not wait for: for a stop that
    /// waits on what only the play goes on to let happen. Asked again,
    /// where the conductor was stopped before it was over and the
    /// instance is still there.
    Stops(&'static str),
    /// The same, by somebody who goes on without giving way: each answer
    /// of the database is there before it is looked for, so nothing else
    /// that is running is looked at from when the stop begins until it
    /// waits for something that is no answer ([`Stepped::at_once`]).
    /// Where the suite is played with a journal for each project, the
    /// stop is the project's conductor's to do, as a task of its own,
    /// and goes a turn at a time: the order is made where a play is
    /// nobody's, which is how the gates play it.
    StopsAtOnce(&'static str),
    /// A text is put where texts are found by name, as somebody who
    /// changed a blueprint would: what runs an earlier one is behind it.
    Put(&'static str),
    /// The instance with this key, wherever it is in the tree, is applied
    /// again as it is by somebody (the button on one that is behind):
    /// looked for first, and done only where it does not run the text now
    /// there under its blueprint's name.
    Again(&'static str),
}

impl Does {
    /// The project what is applied is for, where it is for one.
    pub fn project(&self) -> Option<&'static str> {
        match self {
            Does::ApplyFor(project, ..) | Does::ApplyUnder(project, ..) | Does::ApplyBeside(project, ..) => {
                Some(project)
            }
            _ => None,
        }
    }
}

/// One thing done from outside, when the play is at rest and `when` holds.
pub struct Act {
    pub when: fn(&Seen) -> bool,
    pub does: Does,
}

/// Done as soon as the play is at rest.
pub fn then(does: Does) -> Act {
    Act { when: |_| true, does }
}

/// Done when the play is at rest and `when` holds of what was kept.
pub fn once(when: fn(&Seen) -> bool, does: Does) -> Act {
    Act { when, does }
}

/// A blueprint, and what is done to it.
pub struct Scenario {
    pub name: &'static str,
    /// Texts that are there to be called, spawned and attached.
    pub library: &'static [&'static str],
    pub acts: Vec<Act>,
    /// How much time passes in all, and how much at a time, while
    /// something sleeps. Nothing, where nothing is to sleep.
    pub time: (&'static str, &'static str),
    /// Where what a play stopped at a place came to is not what is meant
    /// for that place: [`differs`] from what it comes to with nothing in
    /// its way, unless the scenario means something else by design.
    pub judge: Judge,
    /// The places it is stopped at, where it is not every place it
    /// passes; and each of those once, with no second stop. For a
    /// scenario that is played many times over, each time with something
    /// about it another amount, to come to one moment that only some
    /// amount comes to.
    pub only: Option<fn(&Place) -> bool>,
    /// The projects it names have journals of their own, and so
    /// conductors, however the suite is played.
    pub apart: bool,
    /// Whether a play that held was where the scenario is there to be,
    /// by what it kept: for an order of two things nothing measures.
    pub there: fn(&[Event]) -> bool,
}

/// Says where a play stopped at a place did not come to what is meant
/// there, given what it comes to with nothing in its way, what it came
/// to, and the steps that were under way at the stop.
pub type Judge = fn(&Place, &Value, &Value, &BTreeMap<String, usize>) -> Vec<String>;

impl Scenario {
    pub fn of(name: &'static str, source: &'static str) -> Scenario {
        Scenario {
            name,
            library: &[],
            acts: vec![then(Does::Apply(source, json!({})))],
            time: ("0s", "1s"),
            judge: |_, usual, came, again| differs(usual, came, again),
            only: None,
            apart: false,
            there: |_| false,
        }
    }

    pub fn there(mut self, there: fn(&[Event]) -> bool) -> Scenario {
        self.there = there;
        self
    }

    pub fn apart(mut self) -> Scenario {
        self.apart = true;
        self
    }

    pub fn only(mut self, at: fn(&Place) -> bool) -> Scenario {
        self.only = Some(at);
        self
    }

    pub fn judged(mut self, judge: Judge) -> Scenario {
        self.judge = judge;
        self
    }

    pub fn with(mut self, library: &'static [&'static str]) -> Scenario {
        self.library = library;
        self
    }

    pub fn and(mut self, act: Act) -> Scenario {
        self.acts.push(act);
        self
    }

    pub fn over(mut self, all: &'static str, at_a_time: &'static str) -> Scenario {
        self.time = (all, at_a_time);
        self
    }
}

/// A play under way.
pub(crate) struct Play {
    pub rig: Rig,
    pub passed: Arc<Mutex<Passed>>,
    way: Way,
    done: usize,
    opened: BTreeMap<String, usize>,
    time_left: u64,
    tick: u64,
    pub stopped: bool,
    /// How it is stopped a second time, where it is: the journal its
    /// conductor is started on again is told this way.
    second: Way,
    /// How often it has been stopped and started.
    stops: usize,
    /// Whoever asked and is not waited for ([`Does::Asks`],
    /// [`Does::Stops`]), each with which of the things done it was.
    asking: Vec<(usize, tokio::task::JoinHandle<()>)>,
    /// Its databases, held between turns from when it is begun until it
    /// is over: nothing its conductor does is answered in between.
    pub stepped: Stepped,
}

/// How a play went.
pub enum Went {
    AsMeant,
    /// It came to rest without ever being where it was to be stopped.
    NeverThere,
    Fault(Vec<String>),
}

impl Play {
    async fn begin(
        scenario: &Scenario,
        stop: Option<&Place>,
        way: Way,
        second: Option<(Again, Way)>,
    ) -> Play {
        Play::on(Rig::on_a_file().await, scenario, stop, way, second).await
    }

    /// A play on `rig`: on files where its journals are to be shut and
    /// opened again, which is how a factory is stopped.
    pub(crate) async fn on(
        mut rig: Rig,
        scenario: &Scenario,
        stop: Option<&Place>,
        way: Way,
        second: Option<(Again, Way)>,
    ) -> Play {
        // For a project with a journal of its own, where the suite is
        // played so: every play is then of such a project.
        if crate::own_journals() {
            rig.applies_for(crate::PROJECT).await;
        }
        if crate::own_journals() || scenario.apart {
            rig.owns_each(&scenario.acts).await;
        }
        rig.makes_environments(&scenario.acts).await;
        for text in scenario.library {
            let name =
                blueprint::check(text, &blueprint::NoCatalog).blueprint.expect("a text that reads").name;
            rig.library.put(&name, text);
        }
        // What keeps nothing as it goes, and the steps it has.
        let mut plain = HashMap::new();
        let applied = scenario.acts.iter().filter_map(|act| match &act.does {
            Does::Apply(source, _)
            | Does::Asks(source, _)
            | Does::ApplyFor(_, source, _)
            | Does::ApplyUnder(_, _, source, _)
            | Does::ApplyBeside(_, _, source, _) => Some(*source),
            _ => None,
        });
        for text in scenario.library.iter().copied().chain(applied) {
            if let Some(read) =
                blueprint::check(text, &blueprint::NoCatalog).blueprint.filter(|read| !read.durable)
            {
                plain.insert(read.name.clone(), read.flow.steps.iter().map(|step| step.id.clone()).collect());
            }
        }
        let (again_at, second) = second.map_or((None, Way::Shut), |(at, way)| (Some(at), way));
        let passed = Arc::new(Mutex::new(Passed::stopped_at(stop.cloned(), again_at, plain)));
        let watching = passed.clone();
        let at = move |event: &Event| watching.lock().unwrap().take(event);
        match way {
            Way::Shut => rig.journal.shut_after(at),
            Way::Unsaid => rig.journal.shut_after_saying(at),
        }
        let millis = |text: &str| blueprint::duration::parse(text).unwrap();
        let stepped = Stepped::holding(&rig.journal);
        Play {
            stepped,
            rig,
            passed,
            way,
            done: 0,
            opened: BTreeMap::new(),
            time_left: millis(scenario.time.0),
            tick: millis(scenario.time.1).max(1),
            stopped: false,
            second,
            stops: 0,
            asking: Vec::new(),
        }
    }

    /// What tells whether anything is still happening.
    pub(crate) fn mark(&self) -> (u64, u64, usize, u64, usize) {
        let shared = &self.rig.shared;
        // Of every journal there is: what was last said in each, and
        // what each database has written.
        let every = self.rig.journal.every();
        (
            every.iter().map(journal::Journal::last_seq).sum(),
            every.iter().map(|journal| journal.db().writes()).sum(),
            shared.calls().len(),
            self.rig.clock.sleep_count(),
            self.rig.clock.sleeping(),
        )
    }

    /// Waits until the play is at rest, or its journal has shut. Where
    /// the journal shut and its database is open ([`Way::Unsaid`]), until
    /// what was being written has been: until that too is at rest.
    pub(crate) async fn rest(&mut self) {
        let (mut still, mut last) = (0, self.mark());
        let shut = self.way == Way::Shut;
        // The database is answered a turn at a time ([`Stepped`]): what
        // the conductor has running takes turns the same way on any
        // machine, and a turn in which nothing was asked of the database
        // is one in which nothing is on its way to changing anything,
        // where one in which nothing was written only looked so. A
        // journal that shuts and holds its database meanwhile answers
        // nothing more, a turn included.
        while still < STILL && !(shut && self.rig.journal.is_shut()) {
            let Some(answered) = self.stepped.turn(shut).await else { return };
            let now = self.mark();
            (still, last) = if now == last && answered == 0 { (still + 1, last) } else { (0, now) };
        }
    }

    async fn seen(&self) -> Seen {
        Seen(self.rig.events("").await)
    }

    /// Plays to the end: how many things it did, or that it never ended.
    async fn through(&mut self, scenario: &Scenario) -> Result<(), String> {
        for _ in 0..MOVES {
            self.rest().await;
            // At rest after its first start again: what its conductor
            // keeps from here is no part of its coming up. Said before
            // looking whether the journal shut, so that it is stopped a
            // second time before this or not at all.
            if self.stops == 1 {
                self.passed.lock().unwrap().rested();
            }
            if self.rig.journal.is_shut() {
                // The conductor is stopped where the journal shut, and
                // started on what was kept: on a journal that is told
                // where to shut again, for a play that is stopped a
                // second time while the conductor is coming up. What was
                // under way is counted once the journal that shut has put
                // its file away, with the whole of its last write in.
                // Whoever asked and was not answered has no answer
                // coming: they ask again, looking first.
                for (act, asking) in std::mem::take(&mut self.asking) {
                    if !asking.is_finished() {
                        self.done = self.done.min(act);
                    }
                    asking.abort();
                    let _ = asking.await;
                }
                let (watching, counting) = (self.passed.clone(), self.passed.clone());
                let at = move |event: &Event| watching.lock().unwrap().take(event);
                let way = self.second;
                // Let go, so that each file can be put away; and held
                // again from before the new conductor's start begins.
                self.stepped.let_go();
                self.stepped = self
                    .rig
                    .stopped_there_told(|journal| {
                        counting.lock().unwrap().started_again();
                        match way {
                            Way::Shut => journal.shut_after(at),
                            Way::Unsaid => journal.shut_after_saying(at),
                        }
                    })
                    .await;
                self.passed.lock().unwrap().stopping(self.rig.stopping);
                self.way = way;
                self.stops += 1;
                self.stopped = true;
                continue;
            }
            if let Some(ended) = self.next(scenario).await {
                return ended;
            }
        }
        self.stepped.let_go();
        Err(format!("it had not come to rest after {MOVES} things were done to it"))
    }

    /// At rest, does the next thing from outside: something a person
    /// would do, a stand-in let go, time let pass. How the play ended,
    /// where there is nothing more to do.
    pub(crate) async fn next(&mut self, scenario: &Scenario) -> Option<Result<(), String>> {
        // What the play asks itself is answered a turn at a time too:
        // nothing, where the journal shut before it was.
        let holds = self.way == Way::Shut;
        let seen = Seen(self.stepped.through(holds, self.rig.events("")).await?);
        if let Some(act) = scenario.acts.get(self.done).filter(|act| (act.when)(&seen)) {
            let may_be_refused = self.stopped || self.passed.lock().unwrap().stop.is_some();
            let doing = acts::does(&self.rig, &act.does, may_be_refused);
            let asking = match self.stepped.through(holds, doing).await? {
                acts::Did::It => None,
                acts::Did::Asked(asking) => Some(asking),
                // Whoever asked goes on with it where it is not over.
                acts::Did::AtOnce(mut work) => match self.stepped.at_once(work.as_mut()) {
                    std::task::Poll::Ready(()) => None,
                    std::task::Poll::Pending => Some(tokio::spawn(work)),
                },
            };
            self.asking.extend(asking.map(|asking| (self.done, asking)));
            self.done += 1;
            return None;
        }
        // A stand-in that waits to be let go is let go, the one whose
        // name comes first.
        let mut parked: BTreeMap<String, usize> = BTreeMap::new();
        for event in seen.0.iter().filter(|event| event.topic == "test.reached") {
            *parked.entry(event.data["gate"].as_str().unwrap_or_default().to_string()).or_default() += 1;
        }
        // And one that was taken up where it was, after a restart,
        // waits at its gate again without saying so.
        for (gate, times) in self.rig.shared.counted("taken-up:") {
            *parked.entry(gate).or_default() += times as usize;
        }
        let waiting =
            parked.iter().find(|(gate, times)| self.opened.get(*gate).copied().unwrap_or(0) < **times);
        if let Some((gate, _)) = waiting {
            *self.opened.entry(gate.clone()).or_default() += 1;
            self.rig.open(gate);
            return None;
        }
        // Something sleeps, and there is time left to let pass.
        if self.rig.clock.sleeping() > 0 && self.time_left > 0 {
            let by = self.tick.min(self.time_left);
            self.time_left -= by;
            self.rig.clock.advance(Duration::from_millis(by));
            return None;
        }
        // Over: its databases answer as they are asked from here, for
        // whoever reads what it came to.
        self.stepped.let_go();
        if self.asking.iter().any(|(_, asking)| !asking.is_finished()) {
            return Some(Err(
                "it came to rest with somebody who had asked for something never answered".into()
            ));
        }
        Some(match scenario.acts.get(self.done) {
            Some(_) => Err(format!(
                "it came to rest with {} of the {} things done to it still to do: what the next waits for never came",
                scenario.acts.len() - self.done,
                scenario.acts.len()
            )),
            None => Ok(()),
        })
    }

    /// What the play came to, with nothing in it that is another every time.
    pub(crate) async fn came(&self) -> Value {
        let rig = &self.rig;
        let filter = InstanceFilter { limit: 10_000, ..Default::default() };
        let all = rig.engine.instances(filter).await.unwrap();
        let by_id: HashMap<&str, &conductor::InstanceInfo> =
            all.iter().map(|one| (one.id.as_str(), one)).collect();
        let path = |id: &str| -> String {
            let (mut names, mut at) = (Vec::new(), by_id.get(id).copied());
            while let Some(one) = at {
                names.push(format!("{}[{}]", one.blueprint, one.key));
                at = one.parent.as_deref().and_then(|parent| by_id.get(parent).copied());
            }
            names.reverse();
            names.join(" > ")
        };
        let mut names: HashMap<String, String> = HashMap::new();
        for one in &all {
            names.insert(one.id.clone(), format!("<{}>", path(&one.id)));
        }
        let (mut instances, mut runs) = (Map::new(), Map::new());
        // The oldest first: of two that were started under one name, one
        // after the other, what it came to is the one that is there now,
        // and the ones before it are held too, in the order they came.
        let mut earlier: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for one in all.iter().rev() {
            let mut flows: BTreeMap<String, Vec<Value>> = BTreeMap::new();
            let mut theirs = rig.engine.runs(&one.id, 10_000).await.unwrap();
            theirs.sort_by_key(|run| run.started);
            for run in &theirs {
                names.insert(run.id.clone(), "<a run>".to_string());
            }
            for run in theirs {
                let whole = rig.engine.run(&run.id).await.unwrap();
                let steps: Map<String, Value> = whole
                    .steps
                    .iter()
                    .map(|step| {
                        let said = json!({
                            "status": step.status.as_str(), "tolerated": step.tolerated,
                            "gave": step.output, "error": step.error,
                        });
                        (step.step.clone(), said)
                    })
                    .collect();
                flows.entry(whole.flow.clone()).or_default().push(json!({
                    "status": whole.status.as_str(), "error": whole.error, "outputs": whole.outputs, "steps": steps,
                }));
            }
            // With the text it runs and whose word it runs on: one that
            // went back to a text, or was left on another, is not the same.
            let runs_as = format!("{} holding {}", one.principal.actor(), one.principal.scopes.join(" "));
            let now = json!({
                "status": one.status.as_str(), "state": one.state, "error": one.error, "holds": one.holds,
                "text": one.revision, "as": runs_as,
            });
            if let (Some(was), Some(ran)) =
                (instances.insert(path(&one.id), now), runs.insert(path(&one.id), json!(flows)))
            {
                earlier.entry(path(&one.id)).or_default().push(json!({ "instance": was, "runs": ran }));
            }
        }
        // What was said that is a blueprint's to say, and of the
        // conductor's own what is said once of a thing: not that a run was
        // taken up again, or a step begun a second time.
        let mut said: BTreeMap<String, usize> = BTreeMap::new();
        // And what each ending said, whole: one said late, when the
        // conductor is back, carries what one said in time carries.
        let mut ended: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for event in rig.events("").await {
            if event.topic == "instance.finished" {
                let of = path(event.source.instance.as_deref().unwrap_or_default());
                ended.entry(of).or_default().push(event.data.clone());
            }
            let own = ["step.", "run.", "log.", "system.", "test.", "handler.", "hook.", "poll."];
            let counted = !own.iter().any(|family| event.topic.starts_with(family))
                || ["run.finished", "run.failed"].contains(&event.topic.as_str());
            if counted && event.topic != "instance.state" {
                *said.entry(event.topic.clone()).or_default() += 1;
            }
        }
        let mut asked: BTreeMap<String, usize> = BTreeMap::new();
        for call in rig.shared.calls() {
            *asked.entry(call).or_default() += 1;
        }
        let amiss = crate::amiss::amiss(rig, &all, &path).await;
        // What whoever asked and was not waited for was answered: read
        // of a play with nothing in its way, and held of no other, since
        // somebody whose asking got no answer asks again or finds it done.
        let answered = rig.shared.noted("answered");
        let came = json!({
            "instances": instances, "runs": runs, "earlier": earlier, "said": said, "asked": asked, "ended": ended,
            "amiss": amiss, "answered": answered,
        });
        named(came, &names)
    }
}

/// `value` with every id in it put as what it is the id of, and the place
/// in the journal something was said left out: both are another each play.
fn named(value: Value, names: &HashMap<String, String>) -> Value {
    match value {
        Value::String(text) => Value::String(names.get(&text).cloned().unwrap_or(text)),
        Value::Array(items) => Value::Array(items.into_iter().map(|item| named(item, names)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .filter(|(name, _)| {
                    !["seq", "ts", "started", "finished", "duration_ms"].contains(&name.as_str())
                })
                .map(|(name, value)| (name, named(value, names)))
                .collect(),
        ),
        other => other,
    }
}

/// Where what a play came to is not what was meant: nothing, if it is.
/// `again` is how many steps of each name were under way at the stop:
/// that many more times its action may have been asked.
pub fn differs(meant: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    let mut found = Vec::new();
    for part in ["instances", "runs", "earlier", "said", "ended", "amiss"] {
        apart(part, &meant[part], &came[part], &mut found);
    }
    let count = |of: &Value, key: &str| of["asked"][key].as_u64().unwrap_or(0);
    let keys: BTreeSet<&String> = [meant, came]
        .iter()
        .flat_map(|of| of["asked"].as_object().into_iter().flat_map(|asked| asked.keys()))
        .collect();
    // What is done for one step (its action, and what is done around it)
    // is counted together: of all of it, one thing was under way for each
    // time the step was, and only that may have been asked once more.
    let mut more: BTreeMap<&str, u64> = BTreeMap::new();
    for key in keys {
        let (was, is) = (count(meant, key), count(came, key));
        let step = key.split(':').next().unwrap_or_default();
        *more.entry(step).or_default() += is.saturating_sub(was);
        let most = again.get(step).copied().unwrap_or(0) as u64;
        if is < was || more[step] > most {
            found.push(format!("`{key}` was asked {is} times, and {was} with nothing in its way"));
        }
    }
    found
}

/// Says each thing in which two values are apart, by where it is.
fn apart(at: &str, meant: &Value, came: &Value, found: &mut Vec<String>) {
    match (meant, came) {
        (Value::Object(meant), Value::Object(came)) => {
            let keys: BTreeSet<&String> = meant.keys().chain(came.keys()).collect();
            for key in keys {
                let (was, is) =
                    (meant.get(key).unwrap_or(&Value::Null), came.get(key).unwrap_or(&Value::Null));
                apart(&format!("{at} > {key}"), was, is, found);
            }
        }
        (Value::Array(meant), Value::Array(came)) if meant.len() == came.len() => {
            for (index, (was, is)) in meant.iter().zip(came).enumerate() {
                apart(&format!("{at} > {}", index + 1), was, is, found);
            }
        }
        (was, is) if was != is => {
            found.push(format!("{at}: it came to {is}, and {was} with nothing in its way"))
        }
        _ => {}
    }
}

/// What playing a scenario everywhere found.
pub struct Found {
    pub name: &'static str,
    /// What it came to with nothing in its way.
    pub usual: Value,
    pub places: Vec<Place>,
    pub tried: Vec<Tried>,
    /// In how many plays something written down and not yet begun was
    /// begun, since the conductor was last started, by the blueprint of
    /// what it was, who came to it (the conductor as it started, `start`,
    /// or the step that had asked for it, `step`) and what it was owed
    /// (`first`, its first steps; `again`, those of a text it had taken).
    pub begun: BTreeMap<(String, String, String), usize>,
    /// In how many plays that held it was there ([`Scenario::there`]).
    pub noted: usize,
}

/// One play that was stopped: where and how, and where and how a second
/// time, for one that was.
pub struct Tried {
    pub place: Place,
    pub way: Way,
    /// After how many things kept since it was started again, how, and
    /// where that was.
    pub again: Option<(usize, Way, Place)>,
    /// It came to rest before a second stop that a play before it, stopped
    /// at the same place the same way, looked to lead to: it is a play
    /// stopped once, and was held as one.
    pub rested: bool,
    pub went: Went,
}

impl Found {
    /// A play whose fault shows only in one order of two things says so
    /// when the order was not come to: here, that in some play the step
    /// that had asked for an instance of `blueprint`, taken up, came to
    /// it before the conductor's start did. A play that holds without
    /// ever being there has held nothing of what it is there for.
    /// `owed` is which steps it was owed: `first`, or `again`.
    pub fn came_by_the_step(&self, blueprint: &str, owed: &str) {
        let key = |who: &str| (blueprint.to_string(), who.to_string(), owed.to_string());
        let by = |who: &str| self.begun.get(&key(who)).copied().unwrap_or(0);
        assert!(
            by("step") > 0,
            "{}: in no play did the step that had asked for `{blueprint}` come to it, written down and its \
             steps ({owed}) not yet begun, before the conductor's start did (the start came first in {} \
             plays): the order this play is there for was not come to, and it has held nothing of it. A \
             start has too little to do for what is made between the two; give it more.",
            self.name,
            by("start"),
        );
    }

    /// Every place that was played came to what was meant: or the test
    /// fails, saying where and how.
    pub fn held(&self) {
        let mut faults = Vec::new();
        for Tried { place, way, again, went, .. } in &self.tried {
            if let Went::Fault(why) = went {
                let twice = again.as_ref().map_or(String::new(), |(after, way, at)| {
                    format!("; started, and stopped again after thing {after} it then kept ({at}), {way}")
                });
                faults
                    .push(format!("  stopped after {place}, {way}{twice}:\n      {}", why.join("\n      ")));
            }
        }
        // A play that was to be stopped somewhere and never was has held
        // nothing: it is no pass. That is its first stop, a place every
        // play passes; a second is found by the play that is stopped at
        // it, and one that rests before it was held as stopped once.
        let mut never = Vec::new();
        for Tried { place, way, went, .. } in &self.tried {
            if matches!(went, Went::NeverThere) {
                never.push(format!("  to be stopped after {place}, {way}"));
            }
        }
        let twice = self.tried.iter().filter(|one| one.again.is_some()).count();
        let rested = self.tried.iter().filter(|one| one.rested).count();
        crate::counted::played(self.places.len(), self.tried.len() + 1);
        eprintln!(
            "{}: {} places, {} played, {} of them stopped a second time, {} at rest before a second stop",
            self.name,
            self.places.len(),
            self.tried.len(),
            twice,
            rested,
        );
        for ((blueprint, by, owed), plays) in &self.begun {
            eprintln!(
                "{}: `{blueprint}`, written down and its steps ({owed}) not yet begun, was begun by the {by} in {plays} plays",
                self.name
            );
        }
        assert!(
            never.is_empty(),
            "{}: {} of {} plays never came to where they were to be stopped, and so held nothing:\n{}",
            self.name,
            never.len(),
            self.tried.len(),
            never.join("\n")
        );
        assert!(
            faults.is_empty(),
            "{}: stopped and started, it did not come to what it comes to with nothing in its way, at {} of {} places:\n{}",
            self.name,
            faults.len(),
            self.tried.len(),
            faults.join("\n")
        );
        // And it made the plays it always makes: once it has held, so
        // that a fault is said before a count is.
        crate::counted::held(self.name, self.places.len(), self.tried.len() + 1);
    }
}

/// How a play that was to be stopped went, against what the scenario
/// comes to with nothing in its way.
async fn went(scenario: &Scenario, place: &Place, usual: &Value, play: &mut Play) -> Went {
    match play.through(scenario).await {
        // What it kept is written down for this too: why something a
        // play waits for never came is read there and nowhere else.
        Err(why) => {
            let kept = play.written_down(scenario.name, place).await;
            Went::Fault(vec![why, format!("everything kept in that play is in {kept}")])
        }
        Ok(()) if !play.stopped => Went::NeverThere,
        Ok(()) => {
            let (again, second, owed) = {
                let passed = play.passed.lock().unwrap();
                (passed.under_way.clone(), passed.again.clone(), passed.owed())
            };
            // It kept the thing it was to be stopped at a second time,
            // and went on: a place that was to be played and was not.
            if let (Some(owed), true) = (owed, play.stops < 2) {
                return Went::Fault(vec![format!(
                    "started again, it kept thing {owed} of what it was to be stopped at a second time, and was not stopped there"
                )]);
            }
            let mut came = play.came().await;
            // For a scenario that means something by how often it was
            // stopped: what goes by the clock counts again from each start.
            came["stops"] = json!(play.stops);
            let mut apart = (scenario.judge)(place, usual, &came, &again);
            // What a scenario means by design for a place holds for the
            // place it was stopped at the second time as well.
            if let (false, Some(second)) = (apart.is_empty(), &second)
                && (scenario.judge)(second, usual, &came, &again).is_empty()
            {
                apart.clear();
            }
            if apart.is_empty() {
                return Went::AsMeant;
            }
            let kept = play.written_down(scenario.name, place).await;
            apart.push(format!("everything kept in that play is in {kept}"));
            Went::Fault(apart)
        }
    }
}

/// Plays a scenario with nothing in its way, then once for each place it
/// passed with the conductor stopped and started there, each way a
/// conductor stops; and, for each of those, again with the conductor
/// stopped a second time just after a thing it kept from that start,
/// until it came to rest or a step began that was not one taken up, each
/// way: a conductor that goes down while it is coming up, or while what
/// it took up is going on.
///
/// A play finds its own second stop. Each is told what to be stopped at
/// (the first thing it keeps of a kind not yet played for its pair of
/// ways: what was said, by which run of what, of which step), and the
/// kind is played once it has been come to; so each kind is played once
/// for each pair of ways, as the simulation of a factory plays each kind
/// of place. The one that comes to rest with nothing new kept is the
/// play stopped once at that place. What that one kept says only whether
/// stopping the second time the other way is worth a play; a play so
/// begun that rests first is held as stopped once and counted.
/// `ANYWHERE=everything` tells each the first thing, then the second,
/// until one rests first: it is run by hand before and after a change to
/// what is taken up, and is some six times the plays.
pub async fn anywhere(scenario: Scenario) -> Found {
    let everything = std::env::var("ANYWHERE").is_ok_and(|all| all == "everything");
    let (usual, places) = abreast::first(&scenario);
    // Every other play, in that order and told what it was always told,
    // made several at a time: each on a thread and a conductor of its
    // own, while this thread waits for them ([`abreast`]).
    let abreast::Played { tried, begun, noted } = abreast::played(&scenario, &usual, &places, everything);
    Found { name: scenario.name, usual, places, tried, begun, noted }
}
