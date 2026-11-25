//! The simulation harness: a scenario played whole, and played again with
//! the daemon stopped and started at each place it could be.
//!
//! A scenario is what a person does and what they wait for, in order. It
//! is played once with nothing in its way, and what it came to is written
//! down: where every card is, what is on each branch, what was said where
//! a person looks, what is alive. That play also says where the daemon
//! could have been stopped: after each thing it said. Then the scenario is
//! played again once for each of those places it is asked to try, on a
//! factory of its own: the daemon is stopped there, started again on the
//! same data, and the play goes on to its end. What that came to is held
//! against the first, by one rule: everything is the same, but that an
//! agent may have been asked again.
//!
//! Where the application means something else at a place (a review that
//! was interrupted is left for a person, and says so), the scenario says
//! that outright (`Scenario::meant`), so the choice is written down and a
//! change to it is seen.
//!
//! A play is over when it has come to what was meant. One that never does
//! is a fault, and so that a fault is not waited for until patience runs
//! out, one that has said nothing for a while and is somewhere else is
//! taken to have come to rest there. But only where it is at rest: where
//! nothing is under way, which is asked and not told from the silence
//! (`observed::at_work`: no run of the runner under way beyond what was
//! meant to be, and no agent's turn with a program at work). A slow
//! commit says nothing for as long as it takes, and a play judged in the
//! middle of one was called a fault on a busy machine and never again.
//! While something is under way the play is waited for, as long as a
//! play is waited for at all; one still not at rest then did not come to
//! rest, which is said as that, apart from a fault, with what was under
//! way: the application came to nothing wrong that anybody saw, and
//! something of it never ended.
//!
//! The daemon is stopped at a place exactly. Its journal is told the
//! place (`Journal::shut_after`), keeps nothing said after it, and has its
//! database do nothing more for anybody, so what the daemon comes back to
//! is what one that went at that instant would have left: the same each
//! time the scenario is played.
//!
//! A play begins in a factory at rest, and so its places are those of what
//! a person does in it and of nothing else. Setting a factory up is asked
//! and answered, and what was asked for goes on a little after its answer
//! (a fleet whose apply has answered has stages still looking at their
//! columns). Counted from the answer, that was a place in one play and
//! had gone by before the next was listening: a place tried and never
//! reached, by chance. So the factory is waited for first (`rested`), and
//! what a fleet does as it comes up is tried by a scenario whose first
//! act is bringing one up. At rest is nothing said, nothing asked of the
//! factory's database, which whatever is on its way to saying something
//! does first (`Db::done`), and nothing under way; a play with nothing in
//! its way is over at the same, and so is one that never came to where it
//! was to be stopped.
//!
//! The stand-ins say when their program is there, and wait at a pipe to
//! be let go by the play. That a program is there is a place too
//! (`program.began`), though the daemon says nothing of it: an agent's turn
//! is said to begin a moment before its program is started, and a daemon
//! stopped exactly at that event never started it. Stopped at
//! `program.began` the daemon goes while the program waits, and what
//! becomes of that program is the scenario's to say (`Scenario::away`):
//! let go when the daemon is back, or while it is away.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use journal::{Journal, Places};
use kernel::Event;
use serde_json::Value;

use crate::Rig;
pub use crate::found::Grain;
use crate::found::{Found, Tried, Went};

mod overtaken;
mod passed;
use overtaken::{Next, Since, shut};
use passed::Passed;

/// How long nothing has to be said before a play looks at where it is.
const LULL: Duration = Duration::from_millis(40);
/// How long nothing has to be said, and nothing asked of the factory's
/// database, for a factory with nothing under way to be taken as at rest:
/// before a play begins, and when one with nothing in its way is over.
const REST: Duration = Duration::from_millis(400);
/// How long a play that is somewhere else than was meant may say nothing
/// before it is taken to have come to rest there.
const STUCK: Duration = Duration::from_secs(2);
/// How long a play may take at all.
const PATIENCE: Duration = Duration::from_secs(40);
/// How many plays go on side by side in one process, whichever scenarios
/// they are of: each is a daemon of its own with its threads, its files
/// and what it follows. Each scenario's test holds one of the program's
/// threads until its last play is made, and the scenarios are begun
/// before the other tests (`abreast`), so how many go at once is how
/// soon those threads are the other tests'. Sixteen where the program
/// runs from five to sixteen tests at a time: its first tests are then
/// all scenarios, and with four at a time they held every thread while
/// half the processors stood idle (on eight processors the suite took
/// 91 to 97 s with four, 83 to 85 s with eight, 65 to 74 s with sixteen
/// and no less with more). Eight where it runs more, where sixteen was
/// slower; and four where it runs four or fewer, which is what the
/// hosted machines have, where nothing was measured
/// (docs/build-times.md, *The plays of one test are made several at a
/// time*).
fn at_once() -> usize {
    let said = std::env::var("RUST_TEST_THREADS").ok().and_then(|said| said.parse::<usize>().ok());
    let tests = said.or_else(|| std::thread::available_parallelism().ok().map(usize::from));
    match tests.unwrap_or(4) {
        0..=4 => 4,
        5..=16 => 16,
        _ => 8,
    }
}
static TURNS: std::sync::LazyLock<tokio::sync::Semaphore> =
    std::sync::LazyLock::new(|| tokio::sync::Semaphore::new(at_once()));
/// How long a play that goes on saying things may come no nearer to what
/// was meant before it is taken to be going nowhere.
const NOWHERE: Duration = Duration::from_secs(6);
/// How a play is waited for, and what becomes of a stand-in whose
/// program is there once the daemon is back. A scenario says another only
/// to hold the harness itself to its own rule.
#[derive(Clone, Copy, Debug)]
pub struct Pace {
    /// How long a play that is somewhere else may say nothing before it
    /// is looked at for being at rest there.
    pub stuck: Duration,
    /// How long it is waited for from then, while something is under way.
    pub patience: Duration,
    pub holds: Holds,
}

/// What becomes of a stand-in's program that is there after the daemon
/// has been stopped and started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holds {
    /// It is let go at once.
    Nothing,
    /// It is let go once the play has been waited for with it at work.
    UntilWaitedFor,
    /// It is never let go: something that does not end.
    ForEver,
}

impl Default for Pace {
    fn default() -> Pace {
        Pace { stuck: STUCK, patience: PATIENCE, holds: Holds::Nothing }
    }
}

/// How many more times an agent may be asked in a play that was
/// interrupted than in one that was not.
const ASKED_AGAIN: u64 = 2;

/// The place that is a stand-in's program saying it is there. Whom it is
/// about is which stand-in: `writer`, who works, or `critic`, who reviews.
pub const BEGAN: &str = "program.began";

/// Somewhere the daemon could be stopped: just after it said something.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Place {
    /// What was said.
    pub topic: String,
    /// The blueprint of the instance that said it; nothing for the daemon itself.
    pub blueprint: String,
    /// Which of the instance's runs: `main`, or `on.<handler>`.
    pub flow: String,
    /// Which step of that run.
    pub step: String,
    /// Whom it is about, where the topic is about an agent.
    pub who: String,
    /// Which time this was said so, from 1.
    pub nth: usize,
}

impl Place {
    /// The same thing said, whichever time.
    fn kind(&self) -> (String, String, String, String, String) {
        (self.topic.clone(), self.blueprint.clone(), self.flow.clone(), self.step.clone(), self.who.clone())
    }

    /// Whether this is something said of `blueprint` while it did `flow`.
    pub fn within(&self, blueprint: &str) -> bool {
        self.blueprint == blueprint
    }
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
        if !self.who.is_empty() {
            write!(f, ", {}", self.who)?;
        }
        if self.nth > 1 {
            write!(f, ", time {}", self.nth)?;
        }
        Ok(())
    }
}

/// What a play came to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Came {
    /// What has to be the same however the play was interrupted.
    pub same: BTreeMap<String, Value>,
    /// How often each agent was asked: more after an interruption is
    /// allowed, and fewer is not.
    pub asked: BTreeMap<String, u64>,
}

impl Came {
    pub fn with(mut self, what: &str, value: Value) -> Came {
        self.same.insert(what.to_string(), value);
        self
    }

    /// Everything in which this is not what was meant, a sentence each.
    pub fn differs_from(&self, meant: &Came) -> Vec<String> {
        let mut differs = Vec::new();
        let names: BTreeSet<&String> = self.same.keys().chain(meant.same.keys()).collect();
        for name in names {
            let (came, meant) = (self.same.get(name), meant.same.get(name));
            if came != meant {
                let show = |value: Option<&Value>| value.map_or("nothing".to_string(), Value::to_string);
                differs.push(format!("{name}: it came to {}, and {} was meant", show(came), show(meant)));
            }
        }
        for (agent, meant) in &meant.asked {
            let came = self.asked.get(agent).copied().unwrap_or(0);
            if came < *meant || came > meant + ASKED_AGAIN {
                differs.push(format!(
                    "{agent} was asked {came} times, where {meant} was meant and {ASKED_AGAIN} more are allowed"
                ));
            }
        }
        differs
    }
}

/// Since when a factory's databases have been asked nothing. Whatever is
/// on its way to saying something reads or writes first, so a count of
/// what the database has done that stands still says more than silence
/// does: a handler that has heard something and not yet come to it is
/// silent too.
struct Unasked {
    done: u64,
    since: Instant,
}

impl Unasked {
    fn from_now(journal: &Journal) -> Unasked {
        Unasked { done: Self::done(journal), since: Instant::now() }
    }

    /// What every database of the factory's has done: its own, and each
    /// project's journal.
    fn done(journal: &Journal) -> u64 {
        journal.dbs().iter().map(journal::Db::done).sum()
    }

    /// For how long nothing has been asked, as far as can be told by
    /// looking now: it is looked at every lull, or what was asked between
    /// two looks far apart would count from the second.
    fn lasted(&mut self, journal: &Journal) -> Duration {
        let done = Self::done(journal);
        if done != self.done {
            *self = Unasked { done, since: Instant::now() };
        }
        self.since.elapsed()
    }
}

/// Waits for a factory to be at rest: nothing said and nothing asked of
/// its database for a while, and nothing under way. What set it up has
/// then run its course, and what is said from here on is what is done to
/// it from here on.
pub(crate) async fn rested(rig: &Rig) {
    let journal = rig.node().app.journal.clone();
    let mut follow = journal.follow_every(journal.now(), "");
    let (mut heard_last, mut unasked) = (Instant::now(), Unasked::from_now(&journal));
    let began = Instant::now();
    let mut still = Vec::new();
    loop {
        if began.elapsed() >= PATIENCE {
            let said = rig.lately().await;
            panic!(
                "the factory a play was to begin in did not come to rest. Still under way when it was last \
                 quiet: {}. What was said last, the latest first:\n{said}",
                if still.is_empty() { "nothing".to_string() } else { still.join(", ") }
            );
        }
        if tokio::time::timeout(LULL, follow.next()).await.is_ok() {
            heard_last = Instant::now();
            continue;
        }
        if heard_last.elapsed().min(unasked.lasted(&journal)) < REST {
            continue;
        }
        let (runs, turns, under_way) = crate::observed::at_work(rig).await;
        if runs == 0 && turns == 0 {
            return;
        }
        // What was asked to know that was the harness's own asking.
        (still, unasked) = (under_way, Unasked::from_now(&journal));
    }
}

/// A play under way: the factory it is played in, and what it has heard.
pub struct Play {
    pub rig: Rig,
    /// The pipe the stand-ins wait at, held open so that nobody waits to
    /// open it. A line written lets one of them go on.
    hold: PathBuf,
    /// Where each stand-in says its program is there, a line each.
    there: crate::Listening,
    /// How many stand-ins were left waiting when the daemon was stopped,
    /// to be let go once it is back.
    pub owed: usize,
    /// Where the pipes are, and where a stand-in writes that it has ended.
    at: PathBuf,
    /// How far in each journal the play has read, and where it began.
    read: Places,
    from: Places,
    passed: Arc<Mutex<Passed>>,
    /// Whether the daemon has been stopped and started in this play.
    pub stopped: bool,
    heard_last: Instant,
    unasked: Unasked,
    began: Instant,
    /// What it had come to when last looked at, and since when that has
    /// not changed.
    looked: Option<(Came, Instant)>,
    /// Since when it has been waited for with something under way, and
    /// how many stand-ins are held meanwhile (`Pace::holds`).
    waited: Option<Instant>,
    held: usize,
    /// What the scenario keeps of its own play: a card's id, where a
    /// project is.
    pub notes: BTreeMap<&'static str, String>,
}

impl Play {
    /// A play in `rig`, whose stand-ins wait at `pipe`, begun once the
    /// factory is at rest: what set it up is no part of the play. From
    /// here on every thing the daemon says is a place the play has passed.
    pub async fn on(rig: Rig, at: &Path) -> Play {
        let hold = at.join("hold");
        crate::hold(&hold);
        let began = crate::Listening::at(&at.join("began"));
        rested(&rig).await;
        let read = rig.node().app.journal.now();
        let unasked = Unasked::from_now(&rig.node().app.journal);
        let play = Play {
            rig,
            hold,
            there: began,
            owed: 0,
            at: at.to_path_buf(),
            from: read.clone(),
            read,
            passed: Arc::default(),
            stopped: false,
            heard_last: Instant::now(),
            unasked,
            began: Instant::now(),
            looked: None,
            waited: None,
            held: 0,
            notes: BTreeMap::new(),
        };
        play.watch(false).await;
        play
    }

    /// What a note says, which the scenario wrote down earlier in the play.
    pub fn note(&self, name: &str) -> &str {
        self.notes.get(name).map(String::as_str).unwrap_or_else(|| panic!("the play has no note of `{name}`"))
    }

    /// Waits until the daemon has said such a thing on `topic` in this
    /// play, from its beginning: for an act that is not done until what
    /// it set off has come to something. Nothing is taken in by it; the
    /// play hears the same again where it listens.
    pub async fn said(&self, topic: &str, such: impl Fn(&Event) -> bool) {
        let mut follow = self.rig.order().follow_from(&self.journal(), self.from.clone(), topic);
        crate::soon(&format!("the play to have said such a thing on `{topic}`"), async {
            while !such(&follow.next().await.unwrap()) {}
        })
        .await;
    }

    /// How many of the places passed so far are such a one.
    pub fn times(&self, such: impl Fn(&Place) -> bool) -> usize {
        self.passed.lock().unwrap().places.iter().filter(|place| such(place)).count()
    }

    /// Every place passed so far, in order.
    fn places(&self) -> Vec<Place> {
        self.passed.lock().unwrap().places.clone()
    }

    fn journal(&self) -> Journal {
        self.rig.node().app.journal.clone()
    }

    /// Has the daemon that is running tell the play of everything it
    /// says, and stop taking anything at the place the play stops at;
    /// `catching` where what it said before is still to be read back.
    async fn watch(&self, catching: bool) {
        // Which blueprint each instance runs: what was alive before this
        // daemon was listened to said so to nobody here.
        let filter = conductor::InstanceFilter { live_only: false, limit: 100_000, ..Default::default() };
        let alive = self.rig.node().app.engine.instances(filter).await.unwrap();
        {
            let mut passed = self.passed.lock().unwrap();
            for instance in alive {
                passed.instances.insert(instance.id.clone(), instance.blueprint.clone());
            }
        }
        if catching {
            self.passed.lock().unwrap().catching = Some(Vec::new());
        }
        let passed = self.passed.clone();
        self.journal().shut_after(move |event| passed.lock().unwrap().hooked(event));
    }

    /// Writes down everything the daemon said in this play, a line each,
    /// for whoever has to find out why it went as it did: where it is.
    async fn written_down(&self, name: &str) -> PathBuf {
        let events = self.rig.order().read_after(&self.journal(), &self.from, "", 100_000).await;
        let events = events.unwrap_or_default().0;
        let known = self.passed.lock().unwrap();
        let mut lines = String::new();
        for event in &events {
            let whose = event.source.instance.as_ref().and_then(|id| known.instances.get(id)).cloned();
            let mut data = event.data.to_string();
            if data.len() > 300 {
                let cut = (0..=300).rev().find(|at| data.is_char_boundary(*at)).unwrap_or(0);
                data.truncate(cut);
                data.push('…');
            }
            lines.push_str(&format!(
                "{:5} {:22} {:18} {:14} {}\n",
                event.seq,
                event.topic,
                whose.unwrap_or_default(),
                event.source.step.clone().unwrap_or_default(),
                data
            ));
        }
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("simulation");
        std::fs::create_dir_all(&dir).unwrap();
        let file: String = name.chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
        let path = dir.join(format!("{file}.txt"));
        std::fs::write(&path, lines).unwrap();
        path
    }

    /// Where the daemon is to be stopped in this play.
    fn stop_at(&self, place: &Place) {
        self.passed.lock().unwrap().stop = Some(place.clone());
    }

    /// Lets so many stand-ins that are waiting, or the next to wait, go on.
    pub fn let_go(&mut self, how_many: usize) {
        crate::pass(&self.hold, how_many);
    }

    /// Waits until so many stand-ins' programs have said they ended, in
    /// all: for a scenario that has one end while the daemon is away.
    /// Looked for and not waited on: the daemon that could say is not there.
    pub async fn ended(&self, how_many: usize) {
        let said = self.at.join("ended");
        crate::soon("a stand-in's program to have ended", async {
            while std::fs::read_to_string(&said).unwrap_or_default().lines().count() < how_many {
                tokio::task::yield_now().await;
            }
        })
        .await;
    }

    /// Stops the daemon, now that its journal has shut at the place it was
    /// to be stopped at, and starts it again on what it had. What happens
    /// while there is none is the scenario's; a stand-in that was left
    /// waiting is let go once the daemon is back, if nothing let it go
    /// while it was away.
    async fn stop_and_start<S: Scenario>(&mut self, scenario: &S) {
        self.rig.stop().await;
        scenario.away(self).await;
        if let Err(why) = self.rig.try_start().await {
            let at = self.passed.lock().unwrap().places.last().map(Place::to_string).unwrap_or_default();
            panic!("stopped after {at}: {why}");
        }
        self.stopped = true;
        self.heard_last = Instant::now();
        // A daemon goes on with what it had as it comes up, before anybody
        // here is listening to it: what it kept meanwhile is taken in too,
        // or a play that came to its end in that moment never heard that
        // it had (a card tidied, counted among the places passed).
        // Read from where it had read to, kept or passed on: one place each.
        // What it says from here is taken in once what it said before is.
        self.watch(true).await;
        let read = self.rig.order().read_after(&self.journal(), &self.read, "", 100_000).await;
        self.passed.lock().unwrap().caught_up(&read.unwrap_or_default().0);
        let owed = std::mem::take(&mut self.owed);
        // Where the scenario holds what is there once the daemon is back,
        // what is owed is held too: a line written now would be read by
        // the program that is begun again.
        if scenario.pace().holds == Holds::Nothing {
            self.let_go(owed);
        } else {
            self.held += owed;
        }
    }
}

/// What a person does and waits for, and what it comes to.
pub trait Scenario: Clone + 'static {
    /// How many things a person does in it.
    const ACTS: usize;

    /// A factory ready for the play.
    fn stage(&self) -> impl Future<Output = Play>;

    /// Does the `nth` thing a person does, from 0. Where the daemon went
    /// while they were doing it they do it again when it is back, so it is
    /// written as a person does things: looking first at what is there.
    fn act(&self, nth: usize, play: &mut Play) -> impl Future<Output = ()>;

    /// Whether what a person waits for after the `nth` thing they did has
    /// now been said.
    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool;

    /// What the play has come to, as far as anybody can see from outside.
    fn came_to(&self, play: &Play) -> impl Future<Output = Came>;

    /// What a play stopped at `place` is meant to come to, where that is
    /// not what one with nothing in its way came to.
    fn meant(&self, _place: &Place, usual: &Came) -> Came {
        usual.clone()
    }

    /// Why a play stopped at `place` does not come to what is meant today,
    /// where it is known not to: a sentence saying what goes wrong. Such a
    /// place is held to nothing, and is played only where it is asked for
    /// (`SIMULATE=everything`, or a test that names it), which says how
    /// it went.
    fn not_yet(&self, _place: &Place) -> Option<&'static str> {
        None
    }

    /// Why a play may go by `place` without the daemon ever saying it,
    /// where two things in the scenario come in either order by design: a
    /// sentence saying which two. Such a place is played as any other,
    /// and again where a play never came to it (`found::PLAYS`); one no
    /// play came to is then said, with the sentence, and is no failure.
    /// Any other place no play came to is a failure of its own.
    fn either(&self, _place: &Place) -> Option<&'static str> {
        None
    }

    /// What happens while the daemon is away, in a play that stops it:
    /// nothing, unless the scenario is of something that goes on without
    /// it (an agent's program that outlives the daemon, let go and ended
    /// before the daemon is back).
    fn away(&self, _play: &mut Play) -> impl Future<Output = ()> {
        async {}
    }

    /// How the play is waited for: the usual, but for a scenario that is
    /// there to hold the harness to its own rule.
    fn pace(&self) -> Pace {
        Pace::default()
    }

    /// Puts away what the play left running that the daemon would leave.
    fn strike(&self, _play: &mut Play) -> impl Future<Output = ()> {
        async {}
    }
}

/// How a stretch of listening ended.
enum Ended {
    Heard,
    AsMeant,
    Rested,
    Unrested(Vec<String>),
    Fault(Vec<String>),
}

/// Listens until what a person waits for after their `nth` act is said
/// (or, with no act named, until the play is over), starting the daemon
/// again where it stopped on the way.
async fn listen<S: Scenario>(
    scenario: &S,
    play: &mut Play,
    nth: Option<usize>,
    meant: Option<&Came>,
) -> Ended {
    let mut follow = play.rig.order().follow_from(&play.journal(), play.read.clone(), "");
    // The last thing heard, to ask of again when nothing more is said.
    let mut last = None;
    // What was under way when the play was last quiet and not at rest.
    let mut still: Vec<String> = Vec::new();
    // What was heard as the play was looked at, to be taken first.
    let mut waiting = None;
    loop {
        let next = match waiting.take() {
            Some(next) => next,
            None => tokio::select! {
                () = shut(play.journal(), meant.is_some() && !play.stopped) => Next::Gone,
                line = play.there.next_line() => Next::Began(line.ok().flatten().unwrap_or_default()),
                next = tokio::time::timeout(LULL, follow.next()) => Next::Said(next),
            },
        };
        match next {
            Next::Gone => {
                drop(follow);
                play.stop_and_start(scenario).await;
                follow = play.rig.order().follow_from(&play.journal(), play.read.clone(), "");
            }
            // A program is there: the daemon is stopped while it waits, or
            // it is let go.
            Next::Began(who) => {
                play.heard_last = Instant::now();
                let here = play.passed.lock().unwrap().began(who.trim());
                if here {
                    play.owed += 1;
                    play.journal().shut_now();
                } else if play.stopped && scenario.pace().holds != Holds::Nothing {
                    play.held += 1;
                } else {
                    play.let_go(1);
                }
            }
            Next::Said(Ok(event)) => {
                let event = event.unwrap();
                play.read = follow.places();
                play.heard_last = Instant::now();
                if nth.is_some_and(|nth| scenario.heard(nth, play, &event)) {
                    return Ended::Heard;
                }
                last = Some(event);
            }
            // Nothing is being said just now: a moment to look.
            Next::Said(Err(_)) => {
                // And to ask again whether what a person waits for has
                // come. The journal hands an event to whoever follows it
                // before it counts the event among the places passed, so
                // what goes by that count (a card tidied) may be asked of
                // the last event a moment too soon, and say no. Nothing
                // more is said then until the node samples its device,
                // half a minute on: a play in fifty waited that long.
                if let (Some(nth), Some(event)) = (nth, &last)
                    && scenario.heard(nth, play, event)
                {
                    return Ended::Heard;
                }
                let silent = play.heard_last.elapsed();
                let unasked = play.unasked.lasted(&play.journal());
                match meant {
                    Some(meant) if play.stopped => {
                        let mark = play.rig.mark();
                        let came = scenario.came_to(play).await;
                        let differs = came.differs_from(meant);
                        if differs.is_empty() {
                            return Ended::AsMeant;
                        }
                        // It has come to rest somewhere else; or it goes on
                        // saying things and comes no nearer.
                        let same_since = match &play.looked {
                            Some((before, since)) if *before == came => *since,
                            _ => Instant::now(),
                        };
                        play.looked = Some((came, same_since));
                        let pace = scenario.pace();
                        if silent >= pace.stuck || same_since.elapsed() >= NOWHERE {
                            // Judged only where it is at rest: nothing is
                            // under way but what was meant to be.
                            let (runs, turns, under_way) = crate::observed::at_work(&play.rig).await;
                            let meant_runs =
                                meant.same.get("runs under way").and_then(Value::as_u64).unwrap_or(0);
                            if runs as u64 <= meant_runs && turns == 0 {
                                // What it came to was read before that was
                                // asked, and the daemon may have gone on
                                // between the two: what it has said since
                                // is heard first, and it is looked at again.
                                match overtaken::since(&play.rig, &mut follow, mark, LULL).await {
                                    Since::Heard(next) => waiting = Some(next),
                                    // Bounded as any play is: given up on, it
                                    // is judged by what it was read to be.
                                    Since::Writing if play.began.elapsed() < PATIENCE => {}
                                    Since::Writing | Since::Nothing => return Ended::Fault(differs),
                                }
                                continue;
                            }
                            let since = *play.waited.get_or_insert_with(Instant::now);
                            if pace.holds == Holds::UntilWaitedFor {
                                let held = std::mem::take(&mut play.held);
                                play.let_go(held);
                            }
                            if since.elapsed() >= pace.patience {
                                let mut why = vec![format!("still under way: {}", under_way.join(", "))];
                                why.extend(differs);
                                why.push(format!(
                                    "what was said last, the latest first:\n{}",
                                    play.rig.lately().await
                                ));
                                return Ended::Unrested(why);
                            }
                        }
                    }
                    // With nothing in its way, or not yet where it is to
                    // be stopped, it is over where it is at rest, and not
                    // where it is only silent: what a slow commit left
                    // unsaid is a place too, and a play that has not come
                    // to its place while something is under way may yet.
                    _ if nth.is_none() && silent.min(unasked) >= REST => {
                        // The journal may shut as this is asked, and then
                        // answers nothing: the daemon is started again
                        // the next time round.
                        let at_work = tokio::select! {
                            () = shut(play.journal(), meant.is_some() && !play.stopped) => None,
                            at_work = crate::observed::at_work(&play.rig) => Some(at_work),
                        };
                        match at_work {
                            Some((0, 0, _)) => return Ended::Rested,
                            Some((_, _, under_way)) => {
                                // What was asked to know that was the
                                // harness's own asking.
                                still = under_way;
                                play.unasked = Unasked::from_now(&play.journal());
                            }
                            None => {}
                        }
                    }
                    _ => {}
                }
                // A journal that has just shut is asked nothing: the daemon
                // is started again the next time round.
                if play.began.elapsed() >= PATIENCE && !play.journal().is_shut() {
                    let waited = match nth {
                        Some(nth) => format!("what a person waits for after the {}th thing they do", nth + 1),
                        None => "the play to be over".to_string(),
                    };
                    // And where it was held against what was meant, in
                    // what it differed when it was given up on: what was
                    // said last does not say that.
                    let mut why = vec![format!(
                        "it was never said: {waited}. What was said last, the latest first:\n{}",
                        play.rig.lately().await
                    )];
                    if !still.is_empty() {
                        why.push(format!("still under way when it was last quiet: {}", still.join(", ")));
                    }
                    for waits in crate::observed::waiting(&play.rig).await {
                        why.push(format!("under way: {waits}"));
                    }
                    if let (None, Some(meant)) = (nth, meant) {
                        why.extend(scenario.came_to(play).await.differs_from(meant));
                    }
                    return Ended::Fault(why);
                }
            }
        }
    }
}

/// Plays a scenario, stopping the daemon at `stop` if anywhere: how it
/// went, what it came to, and every place it passed.
async fn play<S: Scenario>(scenario: &S, stop: Option<(&Place, &Came)>) -> (Went, Came, Vec<Place>) {
    let mut play = Box::pin(scenario.stage()).await;
    let meant = stop.map(|(_, meant)| meant);
    if let Some((place, _)) = stop {
        play.stop_at(place);
    }
    let mut ended = Ended::Heard;
    for nth in 0..S::ACTS {
        // Where the daemon goes while a person is doing something, what
        // they were doing gets no answer: they do it again when it is back.
        loop {
            let gone = shut(play.journal(), stop.is_some() && !play.stopped);
            let cut = {
                let acting = scenario.act(nth, &mut play);
                tokio::pin!(acting);
                tokio::pin!(gone);
                tokio::select! {
                    () = &mut acting => false,
                    () = &mut gone => true,
                }
            };
            if !cut {
                break;
            }
            play.stop_and_start(scenario).await;
        }
        ended = listen(scenario, &mut play, Some(nth), meant).await;
        if !matches!(ended, Ended::Heard) {
            break;
        }
    }
    if matches!(ended, Ended::Heard) {
        ended = listen(scenario, &mut play, None, meant).await;
    }
    let came = scenario.came_to(&play).await;
    let called = stop.map_or("with nothing in its way".to_string(), |(place, _)| place.to_string());
    let name = format!("{}-{called}", std::any::type_name::<S>());
    let went = match ended {
        Ended::Fault(mut why) => {
            let at = play.written_down(&name).await;
            why.push(format!("everything the daemon said in that play is in {}", at.display()));
            Went::Fault(why)
        }
        Ended::Unrested(mut why) => {
            let at = play.written_down(&name).await;
            why.push(format!("everything the daemon said in that play is in {}", at.display()));
            Went::Unrested(why)
        }
        Ended::AsMeant => Went::AsMeant,
        // It came to rest without the daemon having been stopped: it was
        // never where it was to be stopped. What it said instead is
        // written down as a play that went wrong is, under the place's
        // name: which of two things came first this time is read there.
        Ended::Rested | Ended::Heard if stop.is_some() && !play.stopped => {
            play.written_down(&format!("{name}, never reached")).await;
            Went::NeverThere
        }
        Ended::Rested | Ended::Heard => Went::AsMeant,
    };
    // What is put away from here on is no part of the play: the places it
    // passed are those up to now.
    let passed = play.places();
    scenario.strike(&mut play).await;
    // Whoever is still waiting to be let go, is.
    play.let_go(16);
    // And the daemon is stopped, which gives back what it held: a process
    // that plays hundreds of these keeps none of them.
    play.rig.stop().await;
    (went, came, passed)
}

/// Plays a scenario with nothing in its way, and then once for each place
/// of `grain` with the daemon stopped and started there.
pub async fn simulate<S: Scenario>(name: &'static str, scenario: S, grain: Grain) -> Found {
    let began = Instant::now();
    let (went, usual, passed) = {
        let _turn = TURNS.acquire().await.unwrap();
        Box::pin(play(&scenario, None)).await
    };
    if let Went::Fault(why) = went {
        panic!("{name}: with nothing in its way the play did not come to its end:\n{}", why.join("\n"));
    }
    // `SIMULATE_AT=<words>` tries only the places whose description has
    // those words in it, whatever the grain: for looking at one.
    let only = std::env::var("SIMULATE_AT").ok();
    let places: Vec<Place> = passed
        .iter()
        .enumerate()
        .filter(|(nth, place)| match &only {
            Some(words) => place.to_string().contains(words.as_str()),
            None => grain.takes(*nth, place),
        })
        .map(|(_, place)| place.clone())
        .collect();
    // What is not held yet is played only where everything is, or where
    // it was asked for by name.
    let all = only.is_some() || !matches!(grain, Grain::Kinds);
    let local = tokio::task::LocalSet::new();
    let mut plays = Vec::new();
    for place in places {
        let (not_yet, either) = (scenario.not_yet(&place), scenario.either(&place));
        let (scenario, usual) = (scenario.clone(), usual.clone());
        plays.push(local.spawn_local(async move {
            if let (Some(why), false) = (not_yet, all) {
                return Tried { place, went: Went::NotYet(why), not_yet, either, plays: 0 };
            }
            let _turn = TURNS.acquire().await.unwrap();
            let meant = scenario.meant(&place, &usual);
            // Whatever it is waiting in: a play that has not ended in three
            // times its patience is not going to. (Patience may be begun
            // late: it runs from when the play was first waited for.)
            let once = async || {
                let stopped = Box::pin(play(&scenario, Some((&place, &meant))));
                match tokio::time::timeout(PATIENCE * 3, stopped).await {
                    Ok((went, _, _)) => went,
                    Err(_) => Went::Unrested(vec!["the play did not end".to_string()]),
                }
            };
            // A play that never came to the place is played again.
            let (went, plays) = crate::found::until_there(once).await;
            Tried { place, went, not_yet, either, plays }
        }));
    }
    let mut tried = Vec::new();
    local
        .run_until(async {
            for play in plays {
                tried.push(play.await.unwrap());
            }
        })
        .await;
    Found { name, passed, tried, took: began.elapsed() }
}
