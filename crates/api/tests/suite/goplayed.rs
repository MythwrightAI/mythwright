//! A run that waits for a person to let it go on, stopped anywhere in the
//! asking and the approving and started again (`approval.ask`,
//! `told::goes`).
//!
//! Ada applies a text that builds, waits for a go-ahead, and then ships;
//! Grace, who may let it go on and did not start it, is told by a line of
//! her inbox and approves it. The ask's row and what is said of it are
//! one write, its line is owed by the row until it is told, and the
//! decision is one write that the waiting step hears; the step is the
//! same done twice, finding its ask by its run, its step and which of
//! everything done under its name it is. Wherever the daemon goes, there
//! is one ask, Grace has one line of it, told once, it is approved once,
//! and the run ships once. Declined, nothing ships, and the run fails
//! with why.

use kernel::Event;
use reqwest::Method;
use serde_json::{Value, json};

use crate::found::Grain;
use crate::kept::person;
use crate::observed::node_with_stand_ins;
use crate::simulation::{Came, Place, Play, Scenario, simulate};

const SHIP: &str = "blueprint: ship
steps:
  built:
    do: emit
    with: { topic: shop.built }
  approve:
    needs: [built]
    do: approval.ask
    with: { says: Ship the build? }
  shipped:
    needs: [approve]
    do: emit
    with: { topic: shop.shipped, by: \"${steps.approve.out.approved_by_name}\" }
";

/// The apply, and then Grace approving it, or declining it with why.
#[derive(Clone)]
struct Went {
    approve: bool,
}

impl Scenario for Went {
    const ACTS: usize = 2;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-goplayed").await;
        rig.send("/api/projects", json!({ "name": "Shop" })).await;
        rig.send("/api/tier", json!({ "tier": "premium" })).await;
        let grace = person(&rig, "Grace").await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("grace", grace.token);
        play
    }

    /// Applying the same text under the same key is the instance there
    /// is, and deciding what is decided is as it was decided: so an act
    /// the daemon went away in the middle of is done again as it is.
    async fn act(&self, nth: usize, play: &mut Play) {
        match nth {
            0 => {
                let body = json!({ "source": SHIP, "project": "shop", "key": "ship" });
                let (status, came) = play.rig.post("/api/apply", body).await;
                assert_eq!(status, reqwest::StatusCode::OK, "{came}");
            }
            _ => {
                // Found by the line she was told by, which is there however
                // it was decided: done again once decided, it is as it was.
                let grace = play.note("grace").to_string();
                let (_, inbox) = play.rig.call(Method::GET, "/api/inbox?all=true", Some(&grace), None).await;
                let line = inbox["lines"].as_array().into_iter().flatten().find_map(|line| {
                    line["why"].as_str().and_then(|why| why.strip_prefix("asked ")).map(str::to_string)
                });
                let ask = line.unwrap_or_else(|| panic!("Grace was told of no run that waits: {inbox}"));
                let how = if self.approve { "approve" } else { "decline" };
                let path = format!("/api/asked/{ask}/{how}");
                let body = json!({ "why": "not on a Friday" });
                let (status, came) = play.rig.call(Method::POST, &path, Some(&grace), Some(body)).await;
                assert_eq!(status, reqwest::StatusCode::OK, "{came}");
            }
        }
    }

    fn heard(&self, nth: usize, _: &Play, event: &Event) -> bool {
        match nth {
            0 => event.topic == "inbox.told",
            _ if self.approve => event.topic == "shop.shipped",
            _ => event.topic == "project.approval.decided",
        }
    }

    async fn came_to(&self, play: &Play) -> Came {
        let rig = &play.rig;
        let grace = play.note("grace");
        // Ada's, which lists every ask she made however it was decided.
        let asked = rig.read("/api/asked").await;
        let asks: Vec<Value> = asked["asked"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|ask| json!([ask["kind"], ask["status"], ask["says"]]))
            .collect();
        let (_, inbox) = rig.call(Method::GET, "/api/inbox?all=true", Some(grace), None).await;
        let lines: Vec<Value> = inbox["lines"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|line| json!([line["says"], line["count"]]))
            .collect();
        let shipped: Vec<Value> = rig
            .said(journal::EventQuery { topics: "shop.shipped".into(), limit: 100, ..Default::default() })
            .await
            .into_iter()
            .map(|event| event.data["by"].clone())
            .collect();
        let instances = rig.read("/api/instances?project=shop").await;
        let ran: Vec<Value> = instances["instances"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|one| json!([one["blueprint"], one["status"]]))
            .collect();
        Came::default()
            .with("asks", json!(asks))
            .with("lines", json!(lines))
            .with("shipped", json!(shipped))
            .with("ran", json!(ran))
    }
}

/// Where a go ask is kept, told, decided and heard, and the run goes on.
fn in_the_going(place: &Place) -> bool {
    place.topic.starts_with("project.approval")
        || ["inbox.told", "shop.shipped"].contains(&place.topic.as_str())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_run_let_go_on_ships_once_and_is_told_once_wherever_the_daemon_is_stopped_in_it() {
    let grain = Grain::asked_or(Grain::Where(in_the_going));
    simulate("a run let go on", Went { approve: true }, grain).await.holds();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_run_declined_ships_nothing_and_is_told_once_wherever_the_daemon_is_stopped_in_it() {
    let grain = Grain::asked_or(Grain::Where(in_the_going));
    simulate("a run declined", Went { approve: false }, grain).await.holds();
}
