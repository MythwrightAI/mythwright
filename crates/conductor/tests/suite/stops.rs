//! What a play passes through the journal's keeping, and where it is
//! stopped: the half of [`crate::anywhere`] that is asked on the thread
//! that keeps events, of each one as it is kept.
//!
//! Where a play is stopped a second time is found here, by the play
//! itself as it goes. It is told which of the things its conductor keeps
//! by itself after being started again it is to be stopped at ([`Again`]):
//! the first of a kind not yet played, or the n-th. It is never told a
//! place another play passed after its start, because what is kept after
//! a start is not held to come in one order from one play to the next;
//! a play that comes to rest before the thing it was told is a play
//! stopped once.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kernel::Event;
use serde_json::Value;

use crate::anywhere::Place;

/// A kind of thing the conductor keeps: a place, whichever time it is.
pub type Kind = (String, String, String, String);

impl Place {
    pub fn kind(&self) -> Kind {
        (self.topic.clone(), self.blueprint.clone(), self.flow.clone(), self.step.clone())
    }
}

/// Which thing a play is stopped at a second time, of those its conductor
/// keeps by itself after it was started again.
#[derive(Clone, PartialEq)]
pub enum Again {
    /// The first of a kind that is not one of these.
    New(BTreeSet<Kind>),
    /// This one of them, from 1.
    Each(usize),
}

/// Which of a play's stops it is at.
#[derive(Clone, Copy, PartialEq)]
enum Stop {
    First,
    Second,
}

/// What every play of a scenario passes through the journal's keeping.
#[derive(Default)]
pub struct Passed {
    instances: HashMap<String, String>,
    runs: HashMap<String, String>,
    times: HashMap<Kind, usize>,
    pub places: Vec<Place>,
    pub stop: Option<Place>,
    /// How many steps of each name had started and not ended when the
    /// journal shut; and every step of a run that keeps nothing and had
    /// not ended, which is begun again whole.
    pub under_way: BTreeMap<String, usize>,
    started: Vec<(String, String)>,
    ended: BTreeSet<(String, String)>,
    /// Runs that began and have not ended, each with its instance.
    going: BTreeMap<String, String>,
    /// The steps of each blueprint that keeps nothing as it goes.
    plain: HashMap<String, Vec<String>>,
    /// How often the conductor has been stopped at a place it was told.
    stops: usize,
    /// Which thing it is stopped at a second time, where it is to be: a
    /// conductor that goes down while it is coming up.
    again_at: Option<Again>,
    /// Where that was.
    pub again: Option<Place>,
    /// What the conductor kept by itself since it was started again, each
    /// thing by its kind: in coming up and in going on with what was
    /// under way, until that had passed the step it was in.
    pub kinds: Vec<Kind>,
    /// That is over: a step began that was not one taken up, or the play
    /// came to rest. From here a stop is one in a run that is going.
    passed_it: bool,
    /// It has been stopped at a place and not yet started again. What is
    /// kept meanwhile was written with the place, in the one write: it is
    /// before the stop, and is nothing the conductor kept in coming up.
    between: Option<Stop>,
    /// The one project whose conductor is stopped, where one is stopped
    /// alone: only what was under way in it may be asked once more.
    pub alone: Option<&'static str>,
    /// The instances of that project.
    its: BTreeSet<String>,
}

impl Passed {
    /// For a play that is stopped at `stop`, and a second time at `again_at`;
    /// `plain` is the steps of each blueprint that keeps nothing as it goes.
    pub fn stopped_at(
        stop: Option<Place>,
        again_at: Option<Again>,
        plain: HashMap<String, Vec<String>>,
    ) -> Passed {
        Passed { stop, again_at, plain, ..Passed::default() }
    }

    /// Takes in one thing the journal kept: whether that is where it stops.
    pub fn take(&mut self, event: &Event) -> bool {
        if event.seq == 0 {
            return false;
        }
        let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
        if let (Some(instance), "instance.started") = (&event.source.instance, event.topic.as_str()) {
            self.instances.insert(instance.clone(), text(&event.data["blueprint"]));
            if self.alone.is_some_and(|project| event.data["project"] == project) {
                self.its.insert(instance.clone());
            }
        }
        // Where one conductor is stopped alone, what the others have
        // under way is not stopped, and is held to being asked once.
        let stopped =
            self.alone.is_none() || event.source.instance.as_ref().is_some_and(|by| self.its.contains(by));
        if let (Some(run), "run.finished") = (&event.source.run, event.topic.as_str()) {
            self.going.remove(run);
        }
        if let (Some(run), "run.started" | "run.resumed") = (&event.source.run, event.topic.as_str()) {
            if stopped {
                self.going.insert(run.clone(), event.source.instance.clone().unwrap_or_default());
            }
            self.runs.insert(run.clone(), text(&event.data["flow"]));
            if let Some(instance) = &event.source.instance {
                self.instances.entry(instance.clone()).or_insert_with(|| text(&event.data["blueprint"]));
            }
        }
        let known = |map: &HashMap<String, String>, id: &Option<String>| {
            id.as_ref().and_then(|id| map.get(id)).cloned().unwrap_or_default()
        };
        let step = event.source.step.clone().unwrap_or_default();
        if let Some(run) = &event.source.run {
            match event.topic.as_str() {
                "step.started" if stopped => self.started.push((run.clone(), step.clone())),
                "step.finished" | "step.cancelled" => {
                    self.ended.insert((run.clone(), step.clone()));
                }
                _ => {}
            }
        }
        let kind = (
            event.topic.clone(),
            known(&self.instances, &event.source.instance),
            known(&self.runs, &event.source.run),
            step,
        );
        let times = self.times.entry(kind.clone()).or_insert(0);
        *times += 1;
        let place = Place { topic: kind.0, blueprint: kind.1, flow: kind.2, step: kind.3, nth: *times };
        // Kept in the write the place was kept in, after it: part of what
        // was there when the conductor stopped, and no place of its own.
        if self.between.is_some() {
            return false;
        }
        let first = self.stop.as_ref() == Some(&place);
        // Counted from when the conductor was started after its first stop.
        let second = self.stops == 1 && self.by_itself(event, &place);
        self.places.push(place.clone());
        if first || second {
            self.stops += 1;
            match first {
                true => (self.stop, self.between) = (None, Some(Stop::First)),
                false => (self.again, self.between) = (Some(place), Some(Stop::Second)),
            }
        }
        first || second
    }

    /// One thing kept since the conductor was started after its first
    /// stop: whether it is the one the play is stopped at a second time.
    fn by_itself(&mut self, event: &Event, place: &Place) -> bool {
        // A step began that was not one taken up: what was under way has
        // passed the step it was in.
        if event.topic == "step.started" && event.data["resumed"] != true {
            self.passed_it = true;
        }
        if self.passed_it {
            return false;
        }
        self.kinds.push(place.kind());
        self.owed() == Some(self.kinds.len())
    }

    /// The play has come to rest: whatever its conductor keeps from here
    /// is set off from outside, and is no part of its coming up.
    pub fn rested(&mut self) {
        self.passed_it = true;
    }

    /// Which of the things the conductor kept by itself the play was to
    /// be stopped at a second time, from 1: none, where it kept none that
    /// it was told. Worked out from the whole of what was kept, apart from
    /// the looking as each thing came, so that a play which passed the
    /// thing it was told and went on is found.
    pub fn owed(&self) -> Option<usize> {
        match self.again_at.as_ref()? {
            Again::New(played) => self.kinds.iter().position(|kind| !played.contains(kind)).map(|at| at + 1),
            Again::Each(nth) => (self.kinds.len() >= *nth).then_some(*nth),
        }
    }

    /// The conductor is started again on what was kept: what was under
    /// way when it stopped is counted now, with everything the write of
    /// the place kept taken in, so that a step which ended in that write
    /// is not one that may be asked once more.
    ///
    /// Asked when the journal that shut has put its file away, and not
    /// when it is seen to have shut. It shuts at the place, and the rest
    /// of that write is taken in after it, on the journal's own thread:
    /// asked in between, what the write kept after the place was counted
    /// as kept by a conductor that had not been started yet.
    pub fn started_again(&mut self) {
        let Some(stop) = self.between.take() else { return };
        let mut under_way: BTreeMap<String, usize> = BTreeMap::new();
        match stop {
            Stop::First => {
                for (_, step) in self.started.iter().filter(|begun| !self.ended.contains(*begun)) {
                    *under_way.entry(step.clone()).or_default() += 1;
                }
            }
            // A step that was taken up says it started a second time:
            // it is one step under way, however often it said so.
            Stop::Second => {
                let begun: BTreeSet<&(String, String)> =
                    self.started.iter().filter(|begun| !self.ended.contains(*begun)).collect();
                for (_, step) in begun {
                    *under_way.entry(step.clone()).or_default() += 1;
                }
            }
        }
        for instance in self.going.values() {
            let steps = self.instances.get(instance).and_then(|blueprint| self.plain.get(blueprint));
            for step in steps.into_iter().flatten() {
                *under_way.entry(step.clone()).or_default() += 1;
            }
            // What a run does as it ends is asked of no step: the
            // entry it was in the middle of may be asked once more.
            *under_way.entry(String::new()).or_default() += 1;
        }
        // Each stop may have what was under way at it asked once more.
        for (step, times) in under_way {
            *self.under_way.entry(step).or_default() += times;
        }
    }

    /// So many instances were in the middle of being stopped when the
    /// conductor was: of what each does as it stops, which is asked of no
    /// step and in no run, the entry it had come to may be asked once
    /// more. Nothing kept in the journal says a stop has begun, so this
    /// is read from what the conductor is started on.
    pub fn stopping(&mut self, instances: usize) {
        if instances > 0 {
            *self.under_way.entry(String::new()).or_default() += instances;
        }
    }
}
