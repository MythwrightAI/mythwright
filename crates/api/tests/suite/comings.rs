//! A card is given what a stage does once for each time it comes to the
//! stage's column, and only while it is there.
//!
//! A stage's handlers are addressed by its row's column, and each has a
//! place in the journal that moves only for what was addressed to it. So
//! a stage whose row is given another column reads, from wherever its
//! handler for a card that arrives had last been handed anything, every
//! move to the new column there has been since: moves made before the row
//! said so, of cards that have gone on. What is done with a card that is
//! said to be due is the card's row's to say: it is in the column now,
//! and this coming of it has not been taken up already.
//!
//! The stage here does a word of its own, a blueprint of the project's
//! that says what it was started for and ends.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::Rig;
use crate::fleets::{add, change, rig, shop_with};
use crate::lanes::{move_card, moved};

/// What the stage starts for each card: it says which, and is over.
const CHECKS: &str = "\
blueprint: checks
params:
  project: { type: string }
  item: { type: string }
  channel: { type: string, default: \"\" }
  role: { type: string, default: \"\" }
  picks: { type: string, default: first }
  from: { type: string, default: \"\" }
  to: { type: string, default: \"\" }
  pool: { type: string, default: \"\" }
  note: { type: string, default: \"\" }
steps:
  said:
    do: emit
    with: { topic: shop.checked, card: \"${params.item}\", from: \"${params.from}\" }
";

/// A shop whose fleet is one agent and one stage, which starts `checks`
/// for each card that comes to `column`.
async fn checking(rig: &Rig, column: &str) {
    let agents = json!([{ "title": "worker", "role": "worker", "count": 1, "adapter": "writer" }]);
    let stages = json!([{ "title": "check", "column": column, "does": "checks", "role": "worker" }]);
    let columns = ["draft", "ready", "doing", "review", "done"];
    shop_with(rig, &columns, json!({ "agents": agents, "stages": stages })).await;
    let path = format!("@blueprints/{}", brand::blueprint_file("checks"));
    let body = json!({ "path": path, "content": CHECKS });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

/// Moves a card and waits for the move to have been said.
async fn put(rig: &Rig, id: &str, column: &'static str) -> u64 {
    let from = rig.mark();
    move_card(rig, id, column, None).await;
    moved(rig, id, column, from).await
}

/// Waits until `checks` has said it was started for a card after `after`.
async fn checked(rig: &Rig, id: &str, after: u64) -> u64 {
    let card = id.to_string();
    rig.event("shop.checked", move |event| event.seq > after && event.data["card"] == card.as_str()).await.seq
}

/// How many times something was started for a card: each time what the
/// stage starts was said to have been started under the card's name.
async fn started_for(rig: &Rig, id: &str) -> usize {
    crate::rows::said_of(rig, "instance.started", "key", &format!("checks-{id}")).await
}

/// Waits until the stage has looked at a card said to be due after
/// `after`: what it does with one that is due has run, and is over.
async fn looked_at(rig: &Rig, id: &str, after: u64) {
    let card = id.to_string();
    let due =
        rig.event("stage.due", move |event| event.seq > after && event.data["card"] == card.as_str()).await;
    let due = due.seq;
    rig.event("run.finished", move |event| {
        event.seq > due && event.data["blueprint"] == "std/planning" && event.data["flow"] == "on.begun"
    })
    .await;
}

/// The cards the stage keeps as begun for, each with which coming.
async fn kept(rig: &Rig) -> serde_json::Value {
    let all = rig.read("/api/instances?live=true&project=shop").await;
    let stage = all["instances"].as_array().unwrap().iter().find(|one| one["blueprint"] == "std/planning");
    let id = stage.unwrap_or_else(|| panic!("no stage is alive: {all}"))["id"].as_str().unwrap();
    rig.read(&format!("/api/instances/{id}")).await["instance"]["state"]["had"].clone()
}

#[tokio::test]
async fn a_stage_given_another_column_starts_nothing_for_a_card_that_went_through_it_before_and_once_for_one_that_is_there()
 {
    let (rig, _) = rig().await;
    checking(&rig, "ready").await;

    // A card comes to the stage's column and is given what the stage does.
    let first = add(&rig, "Add a cart", &[]).await;
    let at = put(&rig, &first, "ready").await;
    checked(&rig, &first, at).await;

    // Two cards go through another column, which is nobody's: one goes on
    // from there, and one stays.
    let gone = add(&rig, "Write the returns policy", &[]).await;
    put(&rig, &gone, "review").await;
    put(&rig, &gone, "done").await;
    let there = add(&rig, "Count the stock", &[]).await;
    put(&rig, &there, "review").await;
    assert_eq!((started_for(&rig, &gone).await, started_for(&rig, &there).await), (0, 0));

    // The stage's row is given that column. The card that is there is the
    // stage's now, and the one that went through it earlier is not.
    let before = rig.mark();
    change(&rig, "stages", "check", json!({ "column": "review" })).await;
    checked(&rig, &there, before).await;

    // A card that comes there afterwards is given what the stage does,
    // and by then everything said before it has been looked at: what
    // hears a card arrive takes the moves in the order they were made,
    // and what is done with a card that is due is done one at a time.
    let later = add(&rig, "Print the labels", &[]).await;
    let at = put(&rig, &later, "review").await;
    checked(&rig, &later, at).await;

    // This is there for a move from before being heard, and says whether
    // it was: the card that went on was said to be due where the stage
    // now is, after the row was changed, for a move made before it. A
    // runner that gave a handler another place with another `for` would
    // say nothing of that card, and this line would go.
    let said = journal::EventQuery { topics: "stage.due".into(), limit: 10_000, ..Default::default() };
    let said = rig.said(said).await;
    let stale = |event: &&kernel::Event| event.seq > before && event.data["card"] == gone.as_str();
    assert_eq!(said.iter().filter(stale).count(), 1, "no move from before the row was changed was heard");

    assert_eq!(started_for(&rig, &gone).await, 0, "something was started for a card that is not there");
    assert_eq!(started_for(&rig, &there).await, 1, "the card that is there was not started for once");
    assert_eq!((started_for(&rig, &first).await, started_for(&rig, &later).await), (1, 1));
}

#[tokio::test]
async fn a_card_is_given_what_a_stage_does_once_for_each_time_it_comes_however_often_it_is_said_to_be_due() {
    let (rig, _) = rig().await;
    checking(&rig, "ready").await;
    let id = add(&rig, "Add a cart", &[]).await;
    let at = put(&rig, &id, "ready").await;
    checked(&rig, &id, at).await;
    assert_eq!(kept(&rig).await, json!([{ "card": id, "at": "1" }]));

    // The role has nobody, and then somebody: the stage looks at its
    // column and says the card is due again. It came once, and what the
    // stage does was done for that.
    change(&rig, "agents", "worker", json!({ "count": 0 })).await;
    let before = rig.mark();
    change(&rig, "agents", "worker", json!({ "count": 1 })).await;
    looked_at(&rig, &id, before).await;
    assert_eq!(started_for(&rig, &id).await, 1, "it was started for twice for the one time it came");

    // Taken out of the column, nothing is kept of its having come.
    let left = put(&rig, &id, "review").await;
    rig.event("run.finished", move |event| {
        event.seq > left && event.data["blueprint"] == "std/planning" && event.data["flow"] == "on.went"
    })
    .await;
    assert_eq!(kept(&rig).await, json!([]));

    // Put there again it has come again, and is given it again.
    let at = put(&rig, &id, "ready").await;
    checked(&rig, &id, at).await;
    assert_eq!(started_for(&rig, &id).await, 2);
    assert_eq!(kept(&rig).await, json!([{ "card": id, "at": "3" }]));
}

#[tokio::test]
async fn a_card_added_where_a_stage_starts_something_for_each_is_started_for_once() {
    let (rig, _) = rig().await;
    checking(&rig, "draft").await;
    let before = rig.mark();
    let id = add(&rig, "Add a cart", &[]).await;
    checked(&rig, &id, before).await;
    change(&rig, "agents", "worker", json!({ "count": 0 })).await;
    let before = rig.mark();
    change(&rig, "agents", "worker", json!({ "count": 1 })).await;
    looked_at(&rig, &id, before).await;
    assert_eq!(started_for(&rig, &id).await, 1);
}
