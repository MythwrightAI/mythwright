//! A fleet having taken what was written to one of its tables.
//!
//! A change to a row asks the fleet to bring itself to its tables
//! (`std/fleet`'s `changed`), and what does that says `fleet.arranged`
//! with what it was asked for: `row` and the number of the change. Any
//! `fleet.arranged` said after the write is not the fleet having taken
//! it: one asked for by an earlier change may end after the write,
//! having read the tables before it. So a test waits for the one asked
//! for by its own change, or by a later one of the same project, which
//! read the tables after it: of two changes made together, the first
//! may be called off by the second, which then takes both.
//!
//! A row of a table that steers or is steered, written through the API,
//! is answered once the fleet has taken it (`api::taken`), so that the
//! next request goes the new way: a card moved at once is worked as a
//! stage's cell and a segment now say, before and after a restart. A
//! hearer still at it does not hold the answer for longer than a while,
//! which then says who had not taken it.

use std::sync::Arc;

use kernel::Event;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, card, hold, let_go, rig, row, shop};
use crate::lanes::{move_card, moved};

/// The fleet's own tables: a change to any of them is heard.
const TABLES: [&str; 4] = ["agents", "stages", "segments", "actions"];

/// Waits for the first change to one of the fleet's tables written
/// since `before` to have been taken by the fleet, and hands what it
/// said of itself then.
pub(crate) async fn by_fleet(rig: &Rig, before: u64) -> Value {
    let written = rig
        .event("record.*", move |event| {
            event.seq > before
                && event.data["collection"].as_str().is_some_and(|table| TABLES.contains(&table))
        })
        .await;
    arranged_for(rig, before, &written).await
}

/// Waits for an arrangement asked for by `written`, or by a later
/// change of its project.
pub(crate) async fn arranged_for(rig: &Rig, before: u64, written: &Arc<Event>) -> Value {
    let number = rig.number(written.seq);
    let project = written.data["project"].clone();
    rig.event("fleet.arranged", move |event| {
        event.seq > before
            && event.data["project"] == project
            && asked(event).is_some_and(|asked| asked >= number)
    })
    .await
    .data
    .clone()
}

/// The number of the change an arrangement was asked for by, where a
/// change asked for it.
pub(crate) fn asked(event: &Event) -> Option<u64> {
    event.data["asked"].as_str()?.strip_prefix("row ")?.parse().ok()
}

/// What `std/run` was started for, by the key of each: a segment of a
/// card's list is `segment-<card>-…`.
async fn ran(rig: &Rig) -> Vec<String> {
    let query =
        journal::EventQuery { topics: "instance.started".into(), limit: 10_000, ..Default::default() };
    let started = rig.said(query).await;
    let ran = started.iter().filter(|event| event.data["blueprint"] == "std/run");
    ran.map(|event| event.data["key"].as_str().unwrap_or_default().to_string()).collect()
}

#[tokio::test]
async fn a_row_laid_into_a_cards_list_is_in_effect_for_a_card_moved_as_soon_as_the_write_answers() {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    // Written and nothing waited for but the answer: the card is moved in
    // the very next request, as a script or an agent would.
    let row =
        json!({ "title": "Tidy", "stage": "work", "order": 15, "does": "std/run", "command": "exit 0" });
    rig.send("/api/collections/shop/segments/records", json!({ "fields": row })).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;
    let ran = ran(&rig).await;
    assert!(ran.iter().any(|key| key.starts_with(&format!("segment-{id}-"))), "it was tidied: {ran:?}");
}

#[tokio::test]
async fn a_stages_cell_is_in_effect_for_a_card_moved_as_soon_as_the_write_answers() {
    let (rig, at) = rig().await;
    let hold = hold(&at);
    // Two who work, so that only the row holds the second card back.
    shop(&rig, json!({ "workers": 2 })).await;
    let first = add(&rig, "Lay the floor", &[]).await;
    let second = add(&rig, "Put the shelves up", &[]).await;
    move_card(&rig, &first, "ready", None).await;
    moved(&rig, &first, "review", 0).await;
    rig.event("agent.turn.started", |event| event.data["slot"] == "shop-reviewer-1").await;

    let stage = row(&rig, "stages", "work").await;
    let fields = json!({ "fields": { "next_after": "merged" } });
    let (status, answer) =
        rig.call(Method::PATCH, &format!("/api/records/{stage}"), Some(&rig.token), Some(fields)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    move_card(&rig, &second, "ready", None).await;
    // What becomes of it first: it waits, saying so, or it is taken.
    let it = second.clone();
    let came = rig
        .event("card.waiting,board.item.moved", move |event| {
            event.data["card"] == it.as_str()
                || (event.data["id"] == it.as_str() && event.data["to"] == "doing")
        })
        .await;
    assert_eq!(came.topic, "card.waiting", "the second waits behind the first: {}", came.data);
    assert_eq!(card(&rig, &second).await["status"], "ready");
    let_go(&hold).await;
    moved(&rig, &first, "done", 0).await;
    moved(&rig, &second, "review", 0).await;
    let_go(&hold).await;
    moved(&rig, &second, "done", 0).await;
}

#[tokio::test]
async fn a_row_taken_before_its_write_answered_is_still_in_effect_after_the_daemon_is_started_again() {
    let (mut rig, _) = rig().await;
    shop(&rig, json!({})).await;
    let row =
        json!({ "title": "Tidy", "stage": "work", "order": 15, "does": "std/run", "command": "exit 0" });
    rig.send("/api/collections/shop/segments/records", json!({ "fields": row })).await;
    rig.restart().await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;
    let ran = ran(&rig).await;
    assert!(ran.iter().any(|key| key.starts_with(&format!("segment-{id}-"))), "it was tidied: {ran:?}");
}

/// A project's own pair of tables, one steering the other, and a handler
/// that hears a row of the one that steers and is still at it when asked.
const STEERED: &str = "\
blueprint: crewed
permissions: [projects, records]
steps:
  crew: { do: collection.ensure, with: { name: crew, fields: { title: { type: text } } } }
  orders: { needs: [crew], do: collection.ensure, with: { name: orders, steers: crew, fields: { title: { type: text } } } }
on:
  heard:
    event: record.*
    for: { collection: orders }
    steps:
      hold: { do: wait, with: { event: shop.go } }
";

#[tokio::test]
async fn a_write_whose_hearer_is_still_at_it_is_answered_all_the_same_after_a_while_saying_what_had_not_taken_it()
 {
    let clock = kernel::ManualClock::new();
    let theirs = clock.clone();
    let rig = Rig::with("api-taken", move |config, _| config.clock = Some(std::sync::Arc::new(theirs))).await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = rig.send("/api/apply", json!({ "source": STEERED, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");

    // A table nothing steers, nor is steered by, is answered as it always was.
    let crew = rig.send("/api/collections/shop/crew/records", json!({ "fields": { "title": "Ada" } })).await;
    assert!(crew.get("untaken").is_none(), "{crew}");

    let slept = clock.sleep_count();
    let url = "/api/collections/shop/orders/records";
    let asking = rig.post(url, json!({ "fields": { "title": "Mind the till" } }));
    let answered = async {
        rig.event("step.started", |event| event.source.step.as_deref() == Some("hold")).await;
        clock.sleeps_started(slept + 1).await;
        clock.advance(crate::PATIENCE);
    };
    let ((status, answer), ()) = tokio::join!(asking, answered);
    assert_eq!(status, StatusCode::OK, "{answer}");
    let id = answer["id"].as_str().unwrap();
    let said = format!(
        "{id} is written, but `heard` of `{}` had not taken it after 10 seconds: until it does, what the row \
         steers goes as it did. `{} runs` shows what is under way.",
        applied["instance"].as_str().unwrap(),
        brand::CLI
    );
    assert_eq!(answer["untaken"], said.as_str());
}
