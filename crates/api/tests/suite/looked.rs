//! The simulation's own rule for when a play is called a fault, held by a
//! play made for it: a play is not held to a look the daemon has
//! overtaken.
//!
//! A play that has said nothing for a while and is not as meant is looked
//! at, and is a fault where nothing is under way. What it has come to is
//! read first and whether anything is under way is asked after, and the
//! daemon goes on between the two: a fleet still coming up as it was read
//! was up and at rest by the time that was asked, on a machine with much
//! else to do, and the play was called a fault with the thing it waited
//! for said and not yet heard (once in twenty-four runs of the two
//! scenarios of a fleet brought up, four at once on two processors). So
//! what the daemon said since the play last listened is heard first.
//!
//! Here the order is the play's own. A run waits to be nudged, and where
//! the daemon was stopped and started nobody nudges it but whoever looks:
//! what it has come to is read, then it is nudged and waited for until it
//! is over, and what was read is handed back, as a look that was slow is.

use std::time::Duration;

use kernel::{Event, Source};
use serde_json::json;

use crate::found::Went;
use crate::observed::{at_work, node_with_stand_ins, under_way};
use crate::simulation::{Came, Grain, Pace, Place, Play, Scenario, simulate};

/// Says that it has begun, waits to be nudged, and says that it is done.
const NUDGED: &str = "\
blueprint: nudged
on:
  go:
    event: looked.go
    steps:
      begun: { do: emit, with: { topic: looked.begun } }
      nudged: { needs: [begun], do: wait, with: { event: looked.nudge } }
      done: { needs: [nudged], do: emit, with: { topic: looked.done } }
";

async fn say(play: &Play, topic: &str) {
    play.rig.node().app.journal.emit(topic, Source::default(), json!({})).await.unwrap();
}

#[derive(Clone)]
struct Overtaken;

impl Scenario for Overtaken {
    const ACTS: usize = 2;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-looked").await;
        rig.send("/api/projects", json!({ "name": "Shop" })).await;
        let applied =
            rig.send("/api/apply", json!({ "source": NUDGED, "project": "shop", "wait": true })).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
        Play::on(rig, &at).await
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        match nth {
            0 if play.rig.count_of("looked.go").await == 0 => say(play, "looked.go").await,
            // Where the daemon was stopped and started, nobody nudges it
            // but whoever looks at what the play has come to.
            1 if !play.stopped && play.rig.count_of("looked.nudge").await == 0 => {
                say(play, "looked.nudge").await;
            }
            _ => {}
        }
    }

    /// The wait says that it waits once what is said from then on is
    /// something it hears, so that is when it can be nudged.
    fn heard(&self, nth: usize, _: &Play, event: &Event) -> bool {
        event.topic == ["step.waiting", "looked.done"][nth]
    }

    async fn came_to(&self, play: &Play) -> Came {
        let rig = &play.rig;
        let came = Came::default()
            .with("done", json!(rig.count_of("looked.done").await))
            .with("runs under way", under_way(rig).await);
        let waits = rig.count_of("step.waiting").await > 0;
        if play.stopped && waits && rig.count_of("looked.nudge").await == 0 {
            // The daemon goes on while it is looked at: nudged, the run
            // says it is done and is over before whoever looked is back
            // with what they read.
            let mut said = rig.follow("run.finished");
            say(play, "looked.nudge").await;
            rig.event("looked.done", |_| true).await;
            crate::soon("the run that was nudged to be over", async {
                while at_work(rig).await.0 > 0 {
                    said.next().await.unwrap();
                }
            })
            .await;
        }
        came
    }

    /// Looked at the moment it is quiet, and not once it has been quiet
    /// for a while: the look that is overtaken is the first there is.
    fn pace(&self) -> Pace {
        Pace { stuck: Duration::ZERO, ..Pace::default() }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_play_is_not_called_a_fault_on_a_look_the_daemon_overtook_what_was_said_since_is_heard_first() {
    let begun: fn(&Place) -> bool = |place| place.topic == "looked.begun";
    let found = simulate("a run nudged as it was looked at", Overtaken, Grain::Where(begun)).await;
    found.holds();
    assert_eq!(found.tried.len(), 1, "stopped once, as it said it had begun");
    let tried = &found.tried[0];
    assert!(matches!(tried.went, Went::AsMeant), "stopped after {}: {:?}", tried.place, tried.went);
}
