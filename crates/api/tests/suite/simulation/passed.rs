//! What a play has passed, as the daemon says it: asked of every event
//! as it is kept, in the order they are kept, which is what makes a place
//! the same place each time the scenario is played.

use std::collections::HashMap;

use kernel::Event;
use serde_json::Value;

use super::{BEGAN, Place};

/// Where a play has got to, as the journal says it: asked of every event as
/// it is kept, in the order they are kept, which is what makes a place
/// the same place each time the scenario is played.
#[derive(Default)]
pub(super) struct Passed {
    /// The blueprint each instance runs, and the flow each run is.
    pub(super) instances: HashMap<String, String>,
    pub(super) runs: HashMap<String, String>,
    pub(super) times: HashMap<(String, String, String, String, String), usize>,
    /// Every place passed, in order.
    pub(super) places: Vec<Place>,
    /// Where the daemon is to be stopped, until it has been; and then
    /// where it was.
    pub(super) stop: Option<Place>,
    pub(super) stopped_at: Option<Place>,
    /// Which events were taken in, by their numbers: one that is handed
    /// over twice is one place.
    pub(super) taken: std::collections::HashSet<u64>,
    /// While a daemon that was started again is being caught up with:
    /// what it said as the play listened again, kept back until what it
    /// said before that has been read back and taken in.
    pub(super) catching: Option<Vec<Event>>,
}

impl Passed {
    /// Takes in a stand-in's program saying it is there: whether that is
    /// where the daemon stops.
    pub(super) fn began(&mut self, who: &str) -> bool {
        let mut place = Place {
            topic: BEGAN.to_string(),
            blueprint: String::new(),
            flow: String::new(),
            step: String::new(),
            who: who.to_string(),
            nth: 0,
        };
        let times = self.times.entry(place.kind()).or_insert(0);
        *times += 1;
        place.nth = *times;
        let here = self.stop.as_ref() == Some(&place);
        self.places.push(place);
        if here {
            self.stopped_at = self.stop.take();
        }
        here
    }

    /// Takes in one thing the daemon said: whether that is where it stops.
    pub(super) fn take(&mut self, event: &Event) -> bool {
        // What is passed on and not kept is no place: it is said by
        // whoever says it, in no order that is the same twice.
        if event.seq == 0 || !self.taken.insert(event.seq) {
            return false;
        }
        let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
        if let (Some(instance), "instance.started") = (&event.source.instance, event.topic.as_str()) {
            self.instances.insert(instance.clone(), text(&event.data["blueprint"]));
        }
        if let (Some(run), "run.started" | "run.resumed") = (&event.source.run, event.topic.as_str()) {
            self.runs.insert(run.clone(), text(&event.data["flow"]));
        }
        let known = |map: &HashMap<String, String>, id: &Option<String>| {
            id.as_ref().and_then(|id| map.get(id)).cloned().unwrap_or_default()
        };
        let mut place = Place {
            topic: event.topic.clone(),
            blueprint: known(&self.instances, &event.source.instance),
            flow: known(&self.runs, &event.source.run),
            step: event.source.step.clone().unwrap_or_default(),
            who: if event.topic.starts_with("agent.turn.") {
                text(&event.data["slot"])
            } else {
                String::new()
            },
            nth: 0,
        };
        let times = self.times.entry(place.kind()).or_insert(0);
        *times += 1;
        place.nth = *times;
        let here = self.stop.as_ref() == Some(&place);
        self.places.push(place);
        if here {
            self.stopped_at = self.stop.take();
        }
        here
    }

    /// Takes in one thing the daemon says as it says it, while the play
    /// listens: as it comes where nothing is still to be read back, and
    /// otherwise kept back until it has been (`caught_up`). A daemon
    /// started again says things before the play listens to it, which
    /// are read back after; something it says once the play listens
    /// may be of what it said before (a run that ends, of the run that
    /// began), and taken in first it was placed as nothing it knew of.
    /// Only where the play has been stopped, so nowhere to stop is left.
    pub(super) fn hooked(&mut self, event: &Event) -> bool {
        match (&mut self.catching, &self.stop) {
            (Some(meanwhile), None) => {
                meanwhile.push(event.clone());
                false
            }
            _ => self.take(event),
        }
    }

    /// Takes in what was read back of what a daemon started again said
    /// before the play listened to it, and then what it said since, in
    /// that order.
    pub(super) fn caught_up(&mut self, read: &[Event]) {
        let meanwhile = self.catching.take().unwrap_or_default();
        for event in read.iter().chain(&meanwhile) {
            self.take(event);
        }
    }
}

/// The order is made here: what the daemon says once the play listens
/// again is handed over before what it said before that is read back, as
/// it was where a run of `std/told` that began as the daemon came up
/// ended a moment later. Taken in as it came, its ending was placed as a
/// run nobody had said was begun, and what the play waited for (that run
/// over) was never so (`climbed.rs`, crowded: half the playings of every
/// place). Taken in the order it was said in, it is the run it is.
#[test]
fn a_run_that_ends_while_a_daemon_started_again_is_caught_up_with_is_placed_as_the_run_that_began() {
    let said = |seq: u64, topic: &str| Event {
        seq,
        ts: 0,
        topic: topic.to_string(),
        source: kernel::Source { run: Some("run_1".into()), ..kernel::Source::instance("in_1") },
        data: serde_json::json!({ "blueprint": "std/told", "flow": "on.waited" }),
    };
    let mut passed = Passed::default();
    passed.instances.insert("in_1".into(), "std/told".into());
    // The play listens again with what was said before still to be read.
    passed.catching = Some(Vec::new());
    assert!(!passed.hooked(&said(12, "run.finished")), "nothing is a place before it is taken in");
    assert!(passed.places.is_empty());
    passed.caught_up(&[said(11, "run.started")]);
    let placed: Vec<(String, String, String)> = passed
        .places
        .iter()
        .map(|place| (place.topic.clone(), place.blueprint.clone(), place.flow.clone()))
        .collect();
    let run = |topic: &str| (topic.to_string(), "std/told".to_string(), "on.waited".to_string());
    assert_eq!(placed, [run("run.started"), run("run.finished")]);
    // Caught up, what is said is taken in as it comes.
    passed.hooked(&said(13, "step.started"));
    assert_eq!(passed.places.last().map(|place| place.flow.as_str()), Some("on.waited"));
}
