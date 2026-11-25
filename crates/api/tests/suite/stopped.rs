//! A fleet that a person stops and applies again, and an agent that a
//! person stops, each under a card in hand with another in review.
//!
//! Stopping a fleet stops everything under it, one thing after another
//! and the newest first: its agents, whose turns are called off, and
//! the stages that had asked for them. Nothing has failed by that:
//! somebody stopped the fleet. Applied again, the stage that is first
//! there takes up each card the fleet had in hand, its agent told that
//! the card is taken up again and not that anybody sent it back, and a
//! card that was being looked at is looked at again from the start;
//! nothing is said to have failed, and no card is left where it was
//! with nobody told. So the play is held against the same cards with no
//! stop in it, whole: but for each agent having been asked once more,
//! for the turn that was called off, it comes to the same.
//!
//! That is so whichever of the fleet's agents and stages came first.
//! What is under a fleet is stopped one thing after another, the newest
//! first, and the play makes both orders: the agents there before the
//! stages, as in a fleet as it is first made, and after them (`later`),
//! by rows of `agents` added once the fleet is up. Stopped first, an
//! agent has its turn called off while the stage that asked for the
//! card's round has not been come to; the round used to hear that, take
//! it for its agent having failed, and write on the card's row that the
//! card was left, so that the fleet applied again left it in `doing`
//! with nobody told (twenty-nine of thirty plays, six at once on two
//! processors). Everything under what is stopped is now called off
//! before any of it is taken down, which is the runner's
//! (`conductor::meets`, `Engine::call_off`): the round is cut before
//! its agent's turn is called off.
//!
//! An agent that somebody stops by itself, under a card in hand, is the
//! same thing heard by a fleet that is running: the card is left for a
//! person, its row says why, and the fleet says so where it reports,
//! once. Its agent brought back and the card made ready, it is worked
//! by the same agent and goes on.

use std::collections::BTreeMap;
use std::path::PathBuf;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::fleets::{add, add_row, card, change, fleet, given, shop};
use crate::lanes::{move_card, moved};
use crate::observed::{alive, asked, cards, repository, said};
use crate::reapplied::{REVIEWER, WORKER, began, let_go, played_out, stand_ins};
use crate::{Listening, Rig};

/// What a play came to.
#[derive(Debug, PartialEq)]
struct Came {
    /// Where each card is, what is on each branch, what the channel was
    /// told and what is alive.
    same: BTreeMap<&'static str, Value>,
    /// How often each agent was asked.
    asked: BTreeMap<String, u64>,
    /// What was said to have failed, from the stop on: the topic, the
    /// blueprint and the reason of each.
    failed: Vec<String>,
}

/// A factory in the middle of two cards.
struct Held {
    rig: Rig,
    /// Where the stand-ins' holds are, and where they say they began.
    at: PathBuf,
    there: Listening,
    /// The project's folder.
    dir: PathBuf,
    /// The card in hand.
    second: String,
    /// What the fleet was applied with.
    rows: Value,
}

/// A fleet with a card in review, its reviewer's program there and
/// held, and a card in hand, its worker's program there and held. Who
/// is in the fleet is said as it is made, or by rows added once it is
/// up (`later`): whatever is under a fleet is stopped the newest first,
/// so agents that came later are stopped before the stages that ask
/// them.
async fn held(later: bool) -> Held {
    let (rig, at) = stand_ins("api-stopped", None).await;
    let mut there = Listening::at(&at.join("began"));
    for who in [WORKER, REVIEWER] {
        crate::hold(&at.join(format!("hold-{who}")));
    }
    let rows = match later {
        true => json!({ "agents": [] }),
        false => json!({}),
    };
    let dir = shop(&rig, rows.clone()).await;
    if later {
        let reviewer = json!({
            "title": "reviewer", "role": "reviewer", "count": 1, "adapter": "critic", "permission": "read",
        });
        add_row(&rig, "agents", reviewer).await;
        let worker = json!({ "title": "worker", "role": "worker", "count": 1, "adapter": "writer" });
        add_row(&rig, "agents", worker).await;
    }

    let first = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &first, "ready", None).await;
    began(&mut there, WORKER).await;
    let_go(&at, WORKER).await;
    moved(&rig, &first, "review", 0).await;
    began(&mut there, REVIEWER).await;
    let second = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &second, "ready", None).await;
    began(&mut there, WORKER).await;
    Held { rig, at, there, dir, second, rows }
}

/// The fleet of `shop`, which is alive: its id.
async fn the_fleet(rig: &Rig) -> String {
    let filter = conductor::InstanceFilter {
        live_only: true,
        project: Some("shop".into()),
        blueprint: Some("std/fleet".into()),
        limit: 10,
        ..Default::default()
    };
    let found = rig.node().app.engine.instances(filter).await.unwrap();
    assert_eq!(found.len(), 1, "one fleet is alive");
    found[0].id.clone()
}

/// What was said from `from` on, on these topics.
async fn since(rig: &Rig, from: u64, topics: &str) -> Vec<kernel::Event> {
    let query =
        journal::EventQuery { topics: topics.into(), after: from, limit: 10_000, ..Default::default() };
    rig.said(query).await
}

/// What was said to have failed from `from` on.
async fn failed(rig: &Rig, from: u64) -> Vec<String> {
    let said = since(rig, from, "run.failed,instance.failed").await;
    let text =
        |event: &kernel::Event, field: &str| event.data[field].as_str().unwrap_or_default().to_string();
    said.iter()
        .map(|event| format!("{} of {}: {}", event.topic, text(event, "blueprint"), text(event, "reason")))
        .collect()
}

/// Whose turn was called off from `from` on, by name.
async fn called_off(rig: &Rig, from: u64) -> Vec<String> {
    let ended = since(rig, from, "agent.turn.finished").await;
    let mut whose: Vec<String> = ended
        .iter()
        .filter(|event| event.data["called_off"] == true)
        .map(|event| event.data["slot"].as_str().unwrap().to_string())
        .collect();
    whose.sort();
    whose
}

/// What the fleet said in its channel of something having failed, each
/// time it said so.
async fn said_to_fail(rig: &Rig) -> Vec<String> {
    let messages = rig.read("/api/channels/shop/messages").await;
    let bodies =
        messages["messages"].as_array().unwrap().iter().filter_map(|message| message["body"].as_str());
    bodies.filter(|body| body.starts_with("Something in the fleet failed")).map(str::to_string).collect()
}

/// The play: the two cards of [`held`]; then the fleet is stopped, as a
/// person stops it, and applied again (`stops`), or not. Each stand-in
/// is let go as its program is there, and both cards are let run to
/// their end. With what it came to: whose turn was called off, and what
/// the worker was last told.
async fn play(stops: bool, later: bool) -> (Came, Vec<String>, String) {
    let Held { rig, at, mut there, dir, second, rows } = held(later).await;
    let from = rig.mark();
    if stops {
        // Somebody stops the fleet, which is answered when it is stopped
        // with everything under it, and applies it again.
        let id = the_fleet(&rig).await;
        rig.send(&format!("/api/instances/{id}/stop"), json!({})).await;
        assert_eq!(card(&rig, &second).await["status"], "doing", "stopped in hand");
        fleet(&rig, rows).await;
    } else {
        for who in [WORKER, REVIEWER] {
            let_go(&at, who).await;
        }
    }
    played_out(&rig, &mut there, &at, from, 2).await;
    crate::simulation::rested(&rig).await;

    let same = BTreeMap::from([
        ("cards", cards(&rig, "shop").await),
        ("repository", repository(&dir)),
        ("said in the channel", said(&rig, "shop").await),
        ("alive", alive(&rig, Some("shop")).await),
    ]);
    let came = Came { same, asked: asked(&rig, &[WORKER, REVIEWER]).await, failed: failed(&rig, from).await };
    let told = given(&rig, WORKER).await.last().cloned().unwrap_or_default();
    (came, called_off(&rig, from).await, told)
}

#[tokio::test]
async fn a_fleet_stopped_and_applied_again_takes_up_the_card_it_had_in_hand_and_looks_again_at_the_one_in_review()
 {
    taken_up_again(false).await;
}

// The other order: the agents came after the stages, and are stopped
// before them. Everything under the fleet is called off before any of
// it is taken down, so the round is cut before its agent's turn is
// called off, and writes nothing on the card's row.
#[tokio::test]
async fn a_fleet_whose_agents_came_after_its_stages_is_stopped_and_applied_again_the_same() {
    taken_up_again(true).await;
}

/// The play with the fleet stopped and applied again, held whole
/// against the same cards with no stop; `later` as [`held`] takes it.
async fn taken_up_again(later: bool) {
    let ((mut stopped, called_off, told), (usual, ..)) = tokio::join!(play(true, later), play(false, later));

    // Both agents were at work, and the turn of each was called off.
    assert_eq!(called_off, [REVIEWER, WORKER]);
    // Each is asked once more for the card it had: the worker told that
    // the card is taken up again, and not that anybody sent it back.
    assert_eq!((stopped.asked[WORKER], stopped.asked[REVIEWER]), (3, 3));
    assert!(told.contains("Nobody sent it back: take it up where you left it"), "{told}");
    stopped.asked.clone_from(&usual.asked);
    if later {
        // Applied again with its agents there, the fleet says who is in
        // it: the first time it was up, nobody was. That one thing more
        // is said, once.
        let said = stopped.same.get_mut("said in the channel").and_then(Value::as_array_mut).unwrap();
        let up = |line: &Value| {
            line.as_str().is_some_and(|line| line.starts_with("The fleet for shop is up: one"))
        };
        assert_eq!(said.iter().filter(|line| up(line)).count(), 1, "{said:?}");
        said.retain(|line| !up(line));
    }

    // And everything else is as it is where nobody stopped the fleet:
    // both cards done and merged, none sent back, the same alive, the
    // channel told nothing more but that, and nothing said to have failed.
    assert_eq!(stopped, usual);
    assert_eq!(usual.failed, [] as [&str; 0]);
    let done = json!({ "in": "done", "sent back": 0 });
    assert_eq!(usual.same["cards"], json!({ "shop-1": done, "shop-2": done }));
}

#[tokio::test]
async fn an_agent_stopped_under_a_card_in_hand_leaves_the_card_for_a_person_and_the_fleet_says_so_once() {
    let Held { rig, at, mut there, dir, second, .. } = held(false).await;
    let from = rig.mark();

    // Somebody stops the worker (`agent stop`): its turn is called off,
    // and the round that waited on it has failed.
    let (status, answer) =
        rig.call(Method::DELETE, &format!("/api/agents/{WORKER}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // The stage's run for the card takes that in, and the fleet says it.
    for (of, flow) in [("std/working", "on.go"), ("std/fleet", "on.let_be")] {
        rig.event("run.finished", move |event| {
            event.seq > from && event.data["blueprint"] == of && event.data["flow"] == flow
        })
        .await;
    }
    assert_eq!(called_off(&rig, from).await, [WORKER]);

    // The card is where it was, and says that it is left and why; and
    // the fleet, which is running, says so where it reports.
    let left = card(&rig, &second).await;
    let why = format!("{WORKER} could not do it: the turn was cancelled");
    assert_eq!(left["status"], "doing");
    assert_eq!(left["more"]["again"][0], "left", "{left}");
    assert!(left["more"]["again"][1].as_str().unwrap().ends_with(&why), "{left}");
    assert_eq!(said_to_fail(&rig).await, [format!("Something in the fleet failed: {why}")]);
    // And keeps the run that failed, which a person opens from it at the step that failed.
    let messages = rig.read("/api/channels/shop/messages").await;
    let report = messages["messages"]
        .as_array()
        .unwrap()
        .iter()
        .rfind(|one| one["body"].as_str().is_some_and(|body| body.ends_with(&why)))
        .unwrap();
    let about =
        report["said_by"]["about_run"].as_str().unwrap_or_else(|| panic!("no run it is about: {report}"));
    let ran = rig.read(&format!("/api/runs/{about}")).await;
    assert_eq!(ran["status"], "failed", "{ran}");
    assert!(ran["steps"].as_array().unwrap().iter().any(|step| step["status"] == "failed"), "{ran}");
    assert_eq!(asked(&rig, &[WORKER]).await[WORKER], 2, "nobody was asked again");

    // A row of the fleet's agents that is changed has the fleet bring
    // its agents up again; and the card made ready is worked by the one
    // that had it, and goes on: both cards are done and merged.
    change(&rig, "agents", "worker", json!({ "rank": 1 })).await;
    move_card(&rig, &second, "ready", None).await;
    let_go(&at, REVIEWER).await;
    played_out(&rig, &mut there, &at, from, 2).await;
    crate::simulation::rested(&rig).await;
    let done = json!({ "in": "done", "sent back": 0 });
    assert_eq!(cards(&rig, "shop").await, json!({ "shop-1": done, "shop-2": done }));
    let merged = repository(&dir);
    assert_eq!(merged["branches"]["main"], json!(["shop-2: Add a till", "shop-1: Add a cart", "first"]));
    assert_eq!(asked(&rig, &[WORKER]).await[WORKER], 3, "asked once more, for the card made ready");
    // And what failed was said the once.
    assert_eq!(said_to_fail(&rig).await.len(), 1);
}
