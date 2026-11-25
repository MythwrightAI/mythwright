//! Who is next for a card that waits on somebody, as the rows of the
//! project's `chain` say (`std/chain`): each rung told in turn
//! while the card waits as it did, and nothing more once it is moved.

use std::sync::Arc;
use std::time::Duration;

use kernel::ManualClock;
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, shop};
use crate::kept::person;
use crate::lanes::{move_card, moved};
use crate::tells::lines;

/// A node on a clock the test moves, whose fleet's stand-ins work any
/// card and approve it.
async fn ticking() -> (Rig, ManualClock) {
    let clock = ManualClock::new();
    let moved = clock.clone();
    let rig = Rig::with("api-chained", move |config, _| {
        config.clock = Some(Arc::new(moved));
        let writes = [
            "swallow",
            "append",
            "CHANGES.md",
            "changed",
            "say",
            "I changed a file.",
            "say",
            "DECISION: done",
        ];
        config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
        config
            .adapters
            .push(("critic".into(), crate::standin::argv(&["swallow", "say", "DECISION: approve"])));
    })
    .await;
    (Rig::owned(rig).await, clock)
}

/// Adds a rung to the chain, and waits for the chain to have read it.
async fn rung(rig: &Rig, fields: Value) {
    let before = rig.mark();
    rig.send("/api/collections/shop/chain/records", json!({ "fields": fields })).await;
    rig.event("run.finished", move |event| {
        event.seq > before && event.data["blueprint"] == "std/chain" && event.data["flow"] == "on.changed"
    })
    .await;
}

/// Waits until what the chain does with what was said at `said`, by its
/// handler `flow`, is over.
async fn heard(rig: &Rig, flow: &'static str, said: u64) {
    rig.event("run.finished", move |event| {
        event.seq > said && event.data["blueprint"] == "std/chain" && event.data["flow"] == flow
    })
    .await;
}

/// Waits until the rung's time is being waited out (`std/later` has
/// begun its wait), so that the clock moved after it moves past it.
async fn timing(rig: &Rig, after: u64) {
    rig.event("step.waiting", move |event| event.seq > after && event.data.get("until").is_some()).await;
}

#[tokio::test]
async fn a_card_that_waits_is_told_to_each_rung_of_the_chain_in_turn_while_it_waits_as_it_did() {
    let (rig, clock) = ticking().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(&rig, "Grace").await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    rig.event("record.created", |event| event.data["collection"] == "tells").await;

    // 1: the fleet keeps its chain beside its board, with no row: a card
    // that waits is told to whom `tells` names, and nothing is run for
    // it here.
    let tables = rig.read("/api/collections?project=shop").await;
    let table = tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == "chain");
    assert_eq!(table.expect("the chain")["beside"], "board", "{tables}");
    let quiet = add(&rig, "Add a shelf", &[]).await;
    move_card(&rig, &quiet, "ready", None).await;
    let its = quiet.clone();
    let waits = rig.event("card.waits", move |event| event.data["card"] == its.as_str()).await;
    crate::tells::heard(&rig, "on.waited", waits.seq).await;
    let chained =
        |event: &kernel::Event| event.topic == "run.started" && event.data["blueprint"] == "std/chain";
    let all = journal::EventQuery { topics: "run.started".into(), limit: 10_000, ..Default::default() };
    let ran: Vec<_> = rig.said(all).await.into_iter().filter(|event| chained(event)).collect();
    assert!(ran.iter().all(|event| event.data["flow"] == "main"), "{ran:?}");
    assert_eq!(lines(&rig, &grace.token, "review").await, []);

    // 2: two rungs: Grace first, and after four hours the owner. A card
    // that comes to review is told Grace at once.
    rung(&rig, json!({ "title": "lead", "waits": ["review"], "step": 1, "who": "@grace", "after": "4h" }))
        .await;
    rung(&rig, json!({ "title": "owner", "waits": ["review"], "step": 2, "who": "applier" })).await;
    let id = add(&rig, "Add a lamp", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let its = id.clone();
    let waits = rig.event("card.waits", move |event| event.data["card"] == its.as_str()).await;
    heard(&rig, "on.waited", waits.seq).await;
    timing(&rig, waits.seq).await;
    let sentence = format!(
        "{id} is in review, where nobody in the fleet has the role reviewer: it waits for a person to review it."
    );
    assert_eq!(lines(&rig, &grace.token, "review").await, [(sentence.clone(), 1)]);
    // The owner, told by `tells`, has one line of it.
    let owner =
        || async { lines(&rig, &rig.token, "review").await.into_iter().find(|(says, _)| *says == sentence) };
    assert_eq!(owner().await.map(|(_, count)| count), Some(1));

    // 3: four hours on, it still waits: the owner is told by the second
    // rung, which raises the line they have, and nobody after.
    let before = rig.mark();
    clock.advance(Duration::from_secs(4 * 3600));
    let due = rig.event("later.over", move |event| event.seq > before).await;
    heard(&rig, "on.due", due.seq).await;
    assert_eq!(owner().await.map(|(_, count)| count), Some(2));
    assert_eq!(lines(&rig, &grace.token, "review").await, [(sentence.clone(), 1)]);

    // 4: a card moved on before its time tells no second rung.
    let lamp = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &lamp, "ready", None).await;
    let its = lamp.clone();
    let waits = rig.event("card.waits", move |event| event.data["card"] == its.as_str()).await;
    heard(&rig, "on.waited", waits.seq).await;
    timing(&rig, waits.seq).await;
    let before = rig.mark();
    move_card(&rig, &lamp, "done", None).await;
    moved(&rig, &lamp, "done", before).await;
    let before = rig.mark();
    clock.advance(Duration::from_secs(4 * 3600));
    let due = rig.event("later.over", move |event| event.seq > before).await;
    heard(&rig, "on.due", due.seq).await;
    let of_till = format!("{lamp} is in review");
    let told = lines(&rig, &rig.token, "review").await;
    let till =
        told.iter().find(|(says, _)| says.starts_with(&of_till)).expect("the owner was told by `tells`");
    assert_eq!(till.1, 1, "{told:?}");

    // 5: a column no stage works waits on somebody only where a rung
    // names it.
    rung(&rig, json!({ "title": "parked", "waits": ["idle"], "column": "draft", "who": "@grace" })).await;
    let before = rig.mark();
    move_card(&rig, &quiet, "draft", None).await;
    let moved_at = moved(&rig, &quiet, "draft", before).await;
    heard(&rig, "on.moved", moved_at).await;
    let idle = lines(&rig, &grace.token, "idle").await;
    assert_eq!(
        idle,
        [(format!("{quiet} is in draft, where nobody works its cards: it waits for a person."), 1)]
    );
}

/// A node whose fleet has an agent of the role `lead` beside its worker
/// and nobody to review: `looker` says the card goes back, `critic`
/// approves what it is shown.
async fn led(lead: &str) -> Rig {
    let rig = Rig::with("api-chained-led", |config, _| {
        let writes = [
            "swallow",
            "append",
            "CHANGES.md",
            "changed",
            "say",
            "I changed a file.",
            "say",
            "DECISION: done",
        ];
        let looks = ["swallow", "say", "It cannot be checked here without leave.", "say", "DECISION: back"];
        config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
        config.adapters.push(("looker".into(), crate::standin::argv(&looks)));
        config
            .adapters
            .push(("critic".into(), crate::standin::argv(&["swallow", "say", "DECISION: approve"])));
    })
    .await;
    let rig = Rig::owned(rig).await;
    let agents = json!([
        { "title": "worker", "role": "worker", "count": 1, "adapter": "writer" },
        { "title": "reviewer", "role": "reviewer", "count": 0, "adapter": "critic", "permission": "read" },
        { "title": "lead", "role": "lead", "count": 1, "adapter": lead, "permission": "read" },
    ]);
    shop(&rig, json!({ "agents": agents })).await;
    rig.event("record.created", |event| event.data["collection"] == "tells").await;
    rig
}

#[tokio::test]
async fn an_agent_a_rung_names_looks_at_a_card_that_waits_and_sends_it_back_as_its_own_decision() {
    let rig = led("looker").await;
    rung(&rig, json!({ "title": "lead", "waits": ["review"], "who": "role:lead", "does": "look" })).await;
    let id = add(&rig, "Add a lamp", &[]).await;
    let before = rig.mark();
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", before).await;

    // It looks, and sends the card back to be worked with what it said,
    // as its own decision: the move says so, and the card's history.
    let its = id.clone();
    let back = rig
        .event("board.item.moved", move |event| {
            event.data["id"] == its.as_str() && event.data["from"] == "review" && event.data["to"] == "ready"
        })
        .await;
    assert_eq!(
        (&back.data["decided"], &back.data["note"]),
        (&json!("agent:shop-lead-1"), &json!("It cannot be checked here without leave.")),
        "{back:?}"
    );
    // The card's history says the agent moved it, and the blueprint
    // through which it did as detail: the bot said nothing of its own.
    let said = "shop-lead-1 moved it from review to ready, saying: It cannot be checked here without leave.";
    crate::lanes::history(&rig, &id, said).await;
    let history = rig.read(&format!("/api/records/{id}/history")).await;
    let line = history["history"].as_array().unwrap().iter().find(|one| one["said"] == said).unwrap();
    assert_eq!(
        (&line["who"], &line["data"]["through"]),
        (&json!("shop-lead-1"), &json!("std/chain-wait")),
        "{line}"
    );
    // Worked again and sent back again, it is sent back as often as it may
    // be and left for a person: a loop between agents has a number in it.
    let its = id.clone();
    let left = rig.event("card.left", move |event| event.data["card"] == its.as_str()).await;
    assert_eq!(left.data["rounds"], 2, "{left:?}");
}

#[tokio::test]
async fn an_agent_a_rung_names_reviews_a_card_that_waits_as_its_stage_would_and_its_verdict_is_its_own() {
    let rig = led("critic").await;
    rung(&rig, json!({ "title": "lead", "waits": ["review"], "who": "agent:shop-lead-1", "does": "review" }))
        .await;
    let id = add(&rig, "Add a lamp", &[]).await;
    let before = rig.mark();
    move_card(&rig, &id, "ready", None).await;

    // The agent the rung names reviews it alone, the stage merges it as it
    // would for one of its role, and it goes on, approved by that agent.
    let its = id.clone();
    let passed = rig.event("review.passed", move |event| event.data["card"] == its.as_str()).await;
    assert_eq!(passed.data["by"], "shop-lead-1", "{passed:?}");
    moved(&rig, &id, "done", before).await;
    crate::lanes::history(&rig, &id, "shop-lead-1 approved it").await;
    // Merged by the stage, as a review of its own role's would have been.
    let after = passed.seq;
    rig.event("run.finished", move |event| {
        event.seq > after && event.data["blueprint"] == "std/merge" && event.data["status"] == "ok"
    })
    .await;
}

#[tokio::test]
async fn a_rung_that_names_the_agent_that_worked_the_card_is_passed_over_and_the_next_is_told() {
    let rig = led("looker").await;
    // The worker is named first to look at its own card, and then the owner.
    rung(&rig, json!({ "title": "self", "waits": ["review"], "step": 1, "who": "agent:shop-worker-1", "does": "look" }))
        .await;
    rung(&rig, json!({ "title": "owner", "waits": ["review"], "step": 2, "who": "applier" })).await;
    let id = add(&rig, "Add a lamp", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let its = id.clone();
    let waits = rig.event("card.waits", move |event| event.data["card"] == its.as_str()).await;
    heard(&rig, "on.waited", waits.seq).await;

    // Nobody judges its own work: the worker was asked nothing, and the
    // owner, the rung after it, was told (on the line `tells` told them on).
    let all = journal::EventQuery { topics: "agent.turn.started".into(), limit: 1_000, ..Default::default() };
    let looked =
        rig.said(all).await.into_iter().filter(|event| event.data["thread"] == format!("chain:{id}")).count();
    assert_eq!(looked, 0);
    let told = lines(&rig, &rig.token, "review").await;
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(told[0].1, 2, "told by `tells` and by the rung after the one passed over: {told:?}");
    let card = crate::fleets::card(&rig, &id).await;
    assert_eq!(card["more"]["rung"], json!(["review", card["moved"][3].clone(), "2"]), "{card}");
}
