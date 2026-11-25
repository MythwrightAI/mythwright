//! A stage that plans: a card in its column is put to an agent of its
//! role, which asks what it needs to know or says how the card would be
//! done, and has the card for as long as it is there.
//!
//! Nothing here is the daemon's: the stage is a row of the fleet's
//! `stages` that says `plan`, and what is done with a card is
//! `std/proposal`.

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, add_row, card, given, rig, says, shop};
use crate::lanes::{move_card, moved};

const ARCHITECT: &str = "shop-architect-1";

/// The shop's fleet with an architect, and a stage that plans the cards in
/// `draft`; a planned card goes to `pass`, or stays where `pass` is none.
async fn planning(rig: &Rig, pass: &str) {
    shop(rig, json!({})).await;
    let architect =
        json!({ "title": "architect", "role": "architect", "adapter": "critic", "permission": "read" });
    add_row(rig, "agents", architect).await;
    let plan =
        json!({ "title": "plan", "column": "draft", "does": "plan", "role": "architect", "pass": pass });
    add_row(rig, "stages", plan).await;
}

async fn say(rig: &Rig, id: &str, words: &str) {
    let (status, answer) = rig
        .call(
            Method::POST,
            &format!("/api/records/{id}/comments"),
            Some(&rig.token),
            Some(json!({ "body": words })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

async fn plan_of(rig: &Rig, id: &str) -> Value {
    rig.read(&format!("/api/records/{id}")).await["data"]["plan"].clone()
}

#[tokio::test]
async fn a_card_where_a_stage_plans_is_put_to_an_agent_that_asks_and_then_says_how_it_would_be_done() {
    let (rig, at) = rig().await;
    planning(&rig, "").await;
    // It reads the card and cannot tell what is wanted: it asks, and the card waits with its question.
    says(&at, ARCHITECT, "Which shop is the cart for?", "ask");
    let id = add(&rig, "Add a cart", &[]).await;
    let asked = {
        let id = id.clone();
        rig.event("card.asked", move |event| event.data["card"] == id.as_str()).await
    };
    assert_eq!(asked.data["asker"], ARCHITECT);
    assert_eq!(asked.data["question"], "Which shop is the cart for?");
    let first = given(&rig, ARCHITECT).await;
    assert!(
        first[0].starts_with("This card is yours to plan.") && first[0].contains("Add a cart"),
        "{first:?}"
    );

    // What somebody says on the card is put to the same agent, which now
    // says how it would be done. That is kept on the card, and the card
    // stays where it is: its stage says no column for it to go to.
    says(
        &at,
        ARCHITECT,
        "First the cart's model, then its page. Done when a thing can be put in it.",
        "propose",
    );
    say(&rig, &id, "The one on the high street.").await;
    let planned = {
        let id = id.clone();
        rig.event("card.planned", move |event| event.data["card"] == id.as_str()).await
    };
    assert_eq!(planned.data["agent"], ARCHITECT);
    assert_eq!(
        plan_of(&rig, &id).await,
        "First the cart's model, then its page. Done when a thing can be put in it."
    );
    assert_eq!(card(&rig, &id).await["status"], "draft");
    let told = given(&rig, ARCHITECT).await;
    assert_eq!(told.len(), 2, "{told:?}");
    assert!(
        told[1].contains("Somebody said this on the card:") && told[1].contains("the high street"),
        "{told:?}"
    );

    // Given to the one who planned it, the card is still worked by one
    // of the role that works: who a card names is one stage's agent, and
    // each stage takes the name only where it is one of its own.
    let named = json!({ "assignee": format!("agent:{ARCHITECT}") });
    let (status, answer) =
        rig.call(Method::PATCH, &format!("/api/board/items/{id}"), Some(&rig.token), Some(named)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    // Accepted by being moved on, it is worked by one who is shown the
    // plan with the card; and the one who planned it has it no longer.
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;
    let worker = given(&rig, "shop-worker-1").await;
    assert!(
        worker[0].contains("How it is to be done, as was said when it was planned:")
            && worker[0].contains("First the cart's model"),
        "{worker:?}"
    );
    let live = rig.read("/api/instances?project=shop").await;
    let still = live["instances"].as_array().unwrap().iter().any(|one| {
        one["key"] == format!("proposal-{id}").as_str()
            && (one["status"] == "active" || one["status"] == "starting")
    });
    assert!(!still, "{live}");
    assert_eq!(given(&rig, ARCHITECT).await.len(), 2, "nothing more was put to it");
}

#[tokio::test]
async fn a_planned_card_goes_where_its_stage_says_and_sent_back_it_is_planned_again_with_what_was_said() {
    let (rig, at) = rig().await;
    // Planned, a card here is ready to be worked without anybody accepting it.
    planning(&rig, "ready").await;
    says(&at, ARCHITECT, "One piece: the cart.", "propose");
    let id = add(&rig, "Add a cart", &[]).await;
    moved(&rig, &id, "review", 0).await;
    assert_eq!(plan_of(&rig, &id).await, "One piece: the cart.");

    // Sent back to be planned again, its agent is told why, and what it said before.
    says(&at, ARCHITECT, "Two pieces: the model, then the page.", "propose");
    let before = rig.mark();
    move_card(&rig, &id, "draft", Some("Too big for one piece.")).await;
    {
        let id = id.clone();
        rig.event("card.planned", move |event| event.seq > before && event.data["card"] == id.as_str()).await;
    }
    let told = given(&rig, ARCHITECT).await;
    let last = told.last().unwrap();
    assert!(
        last.starts_with("This card was sent back to you") && last.contains("Too big for one piece."),
        "{told:?}"
    );
    assert!(
        last.contains("What you last said of how it would be done:") && last.contains("One piece: the cart.")
    );
    assert_eq!(plan_of(&rig, &id).await, "Two pieces: the model, then the page.");
}

#[tokio::test]
async fn a_card_moved_or_added_where_a_stage_plans_as_the_daemon_went_with_nobody_told_is_planned_when_it_is_back()
 {
    let (mut rig, at) = rig().await;
    planning(&rig, "").await;
    says(&at, ARCHITECT, "First the cart's model, then its page.", "propose");
    // A card that was done, and said to be, is put back where cards are
    // planned, and another is added there. The daemon goes before either
    // is said: the rows are there, and the stage heard of neither.
    let new = json!({ "title": "Add a cart", "status": "done" });
    let put_back = rig.send("/api/board/shop/items", new).await["id"].as_str().unwrap().to_string();
    crate::late::moved_and_not_said(&rig, &put_back, "draft").await;
    let added = crate::late::added_and_not_said(&rig, &put_back, "Add a till", "draft").await;

    rig.restart().await;

    // Each is said late when the daemon is back, and the stage that plans
    // puts each to its architect as it does a card that comes: once.
    for id in [&put_back, &added] {
        let its = id.clone();
        let planned = rig.event("card.planned", move |event| event.data["card"] == its.as_str()).await;
        assert_eq!(planned.data["agent"], ARCHITECT);
    }
    assert_eq!(given(&rig, ARCHITECT).await.len(), 2);
}

#[tokio::test]
async fn a_card_is_planned_while_what_is_laid_before_another_cards_plan_is_under_way_without_waiting_for_it()
{
    let (rig, at) = rig().await;
    planning(&rig, "").await;
    says(&at, ARCHITECT, "One piece: the cart.", "propose");
    // Added where nothing is planned, so that the command is laid before its plan first.
    let new = json!({ "title": "Take payments", "status": "done" });
    let first = rig.send("/api/board/shop/items", new).await["id"].as_str().unwrap().to_string();
    // A command before the first card's plan, that waits until it is let go.
    let pipe = at.join("hold-gate");
    crate::hold(&pipe);
    let held = crate::standin::line(&["held", &pipe.to_string_lossy(), "gate"]);
    let gate = json!({
        "title": "Gate", "stage": "plan", "order": 5, "does": "std/run", "command": held, "item": first,
    });
    add_row(&rig, "segments", gate).await;

    move_card(&rig, &first, "draft", None).await;
    rig.event("instance.started", |event| event.data["blueprint"] == "std/run").await;
    // While the first card's command waits, the second comes and is planned, beside it and not behind it
    // (the stage that plans took up nothing else until one card's command was over).
    let second = add(&rig, "Fix a typo", &[]).await;
    let planned = second.clone();
    rig.event("card.planned", move |event| event.data["card"] == planned.as_str()).await;
    assert_eq!(given(&rig, ARCHITECT).await.len(), 1, "the first is not planned yet");

    crate::let_go("the gate to be waiting", &pipe).await;
    rig.event("card.planned", move |event| event.data["card"] == first.as_str()).await;
    assert_eq!(given(&rig, ARCHITECT).await.len(), 2);
}
