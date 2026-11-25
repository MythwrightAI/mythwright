//! The simulation's own rule for a play that comes to rest somewhere else
//! than was meant, held by a play made for it: it is called a fault at
//! once, saying what differed, and is not waited for until it is given up
//! on.
//!
//! Before a play at rest is called a fault, what the daemon said since it
//! was looked at is heard first (`looked.rs` holds why). That used to be
//! asked by each journal's last number against how far the play had read,
//! and the two need never meet: an event that is passed on and not kept
//! is a journal's last, and a play that is handed it from the order the
//! rig writes down (said in the moment it next listens, before the stream
//! it follows comes to it) has read no further for it. Nothing more being
//! said, the play went on listening for what had been heard already,
//! until it was cut off two minutes on as "the play did not end", and
//! what differed was never said: another fault was hidden behind it.
//!
//! Here the order is the play's own. The daemon is stopped as the run
//! begins and comes back to finish it, which is all it does, and the
//! play is meant to have come to more than that: it rests wrongly. As it
//! is first looked at, something is passed on and not kept, in the
//! moment between the look and its listening again.

use std::time::Duration;

use kernel::{Event, Source};
use serde_json::json;

use crate::found::Went;
use crate::observed::{node_with_stand_ins, under_way};
use crate::simulation::{Came, Grain, Pace, Place, Play, Scenario, simulate};

/// Says that it has begun, and that it is done.
const BEGUN: &str = "\
blueprint: begun
on:
  go:
    event: differed.go
    steps:
      begun: { do: emit, with: { topic: differed.begun } }
      done: { needs: [begun], do: emit, with: { topic: differed.done } }
";

/// What is passed on as the play is looked at, and kept nowhere.
const PASSED: &str = "differed.passed";

#[derive(Clone)]
struct RestsWrongly;

impl Scenario for RestsWrongly {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-differed").await;
        rig.send("/api/projects", json!({ "name": "Shop" })).await;
        let applied =
            rig.send("/api/apply", json!({ "source": BEGUN, "project": "shop", "wait": true })).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
        Play::on(rig, &at).await
    }

    async fn act(&self, _: usize, play: &mut Play) {
        if play.rig.count_of("differed.go").await == 0 {
            play.rig.node().app.journal.emit("differed.go", Source::default(), json!({})).await.unwrap();
        }
    }

    fn heard(&self, _: usize, _: &Play, event: &Event) -> bool {
        event.topic == "differed.done"
    }

    async fn came_to(&self, play: &Play) -> Came {
        let rig = &play.rig;
        let came = Came::default()
            .with("done", json!(rig.count_of("differed.done").await))
            .with("runs under way", under_way(rig).await);
        let passed = rig.order().said_passing().iter().any(|event| event.topic == PASSED);
        if play.stopped && !passed {
            rig.node().app.journal.pass_on(PASSED, Source::default(), json!({})).await.unwrap();
        }
        came
    }

    /// Done twice, which it never is: it rests where it was not meant to.
    fn meant(&self, _: &Place, usual: &Came) -> Came {
        usual.clone().with("done", json!(2))
    }

    /// Looked at the moment it is quiet, as `looked.rs` is.
    fn pace(&self) -> Pace {
        Pace { stuck: Duration::ZERO, ..Pace::default() }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_play_at_rest_where_it_was_not_meant_to_be_is_called_a_fault_at_once_saying_what_differed() {
    let begun: fn(&Place) -> bool = |place| place.topic == "differed.begun";
    let found = simulate("a run that comes to less than was meant", RestsWrongly, Grain::Where(begun)).await;
    assert_eq!(found.tried.len(), 1, "stopped once, as it said it had begun");
    let tried = &found.tried[0];
    let Went::Fault(why) = &tried.went else {
        panic!("stopped after {}, it was to be a fault saying what differed: {:?}", tried.place, tried.went);
    };
    assert_eq!(why[0], "done: it came to 1, and 2 was meant", "{why:#?}");
    // A play is cut off two minutes on, and patience runs out at forty
    // seconds: this one is judged once it rests.
    assert!(found.took < Duration::from_secs(30), "it took {:?}: {why:#?}", found.took);
}
