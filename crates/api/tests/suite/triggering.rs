//! A pipeline run by hand, stopped anywhere in the running and started
//! again.
//!
//! The scenario is a project with its own pipeline (`pipelines/gates`,
//! which says that it ran) and a row of its triggers that runs it by
//! hand. Somebody asks for it from outside, and what runs a project's
//! pipelines (`std/pipeline`) hears that, asks whether they hold what the
//! pipeline runs with, says the run is due, asks the project's pool for
//! pipelines for a place by its key, runs the pipeline under that key and
//! gives the place back. Wherever the daemon goes in that, the pipeline
//! runs once, the place is asked for and given back once, and nothing
//! holds or waits for a place once it is over.

use kernel::Event;
use serde_json::{Value, json};

use crate::found::Grain;
use crate::observed::node_with_stand_ins;
use crate::pipelines::{row, shop};
use crate::simulation::{Came, Place, Play, Scenario, simulate};

#[derive(Clone)]
struct ByHand;

impl Scenario for ByHand {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-triggering").await;
        shop(&rig, None).await;
        row(&rig, json!({ "pipeline": "pipelines/gates", "on": "hand" })).await;
        Play::on(rig, &at).await
    }

    async fn act(&self, _: usize, play: &mut Play) {
        // Asked for once: where it was asked already, it is not again.
        if play.rig.count_of("pipeline.asked").await == 0 {
            let asked =
                json!({ "project": "shop", "pipeline": "pipelines/gates", "with": { "note": "once" } });
            play.rig.send("/api/emit/pipeline.asked", asked).await;
        }
    }

    fn heard(&self, _: usize, play: &Play, _: &Event) -> bool {
        play.times(|place| {
            place.topic == "run.finished" && place.within("std/pipeline") && place.flow == "on.go"
        }) > 0
    }

    async fn came_to(&self, play: &Play) -> Came {
        let mut counted = serde_json::Map::new();
        for topic in ["pipeline.fired", "gates.ran", "pool.asked", "pool.granted", "pool.released"] {
            counted.insert(topic.to_string(), json!(play.rig.count_of(topic).await));
        }
        Came::default().with("said", Value::Object(counted)).with("pool", pool(play).await)
    }
}

/// Who holds a place in the project's pool for pipelines, and who waits.
async fn pool(play: &Play) -> Value {
    let pools = play.rig.read("/api/instances?project=shop&live=true").await;
    let pool = pools.as_array().into_iter().flatten().find(|one| one["blueprint"] == "std/pool");
    pool.map(|pool| json!([pool["state"]["held"], pool["state"]["waiting"]])).unwrap_or(Value::Null)
}

/// Where the run is under way: anything of what runs the project's
/// pipelines, of its pool, of the pipeline, and what is said of them.
fn in_the_running(place: &Place) -> bool {
    ["std/pipeline", "std/pool", "pipelines/gates"].iter().any(|blueprint| place.within(blueprint))
        || place.topic.starts_with("pipeline.")
        || place.topic.starts_with("pool.")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_pipeline_run_by_hand_runs_once_and_gives_its_place_back_wherever_the_daemon_is_stopped_in_it() {
    let found = simulate("a pipeline run by hand", ByHand, Grain::Where(in_the_running)).await;
    found.holds();
}
