//! What a project's `segments` lays about the one thing a stage begins
//! for a card where it plans, or starts a blueprint of its row's naming.
//!
//! Such a stage begins what is usual there for each card that comes, and
//! that has the card for as long as it is in the column. A row ordered
//! before it is called first, and the usual thing is begun once that has
//! passed; a row ordered after it is called once what was begun says it
//! has done and leaves the handing on to its stage, which then moves the
//! card. Nothing is sent back from such a column: a thing that says so
//! leaves the card for a person, with what it said.

use std::path::Path;

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, add_row, card, given, rig, says, shop_with};
use crate::lanes::{moved, said};

const ARCHITECT: &str = "shop-architect-1";

/// A shop with a column nobody has (`planned`), and a stage that does
/// `does` with the cards in `draft` and sends them on to it.
async fn shop(rig: &Rig, does: &str) {
    shop_with(rig, &["draft", "planned", "ready", "doing", "review", "done"], json!({})).await;
    let architect =
        json!({ "title": "architect", "role": "architect", "adapter": "critic", "permission": "read" });
    add_row(rig, "agents", architect).await;
    let first =
        json!({ "title": "first", "column": "draft", "does": does, "role": "architect", "pass": "planned" });
    add_row(rig, "stages", first).await;
}

/// A row of the first stage's list that runs a command, which writes a
/// line where the test reads it.
///
/// The command is a line for the system's shell, which names a program
/// (`crates/hosting/standin`): it writes the line and then does `then`.
fn runs(title: &str, order: u64, at: &Path, line: &str, then: &[&str]) -> Value {
    let ran = at.join("ran").to_string_lossy().into_owned();
    let command = crate::standin::line(&[&["append", &ran, line][..], then].concat());
    json!({ "title": title, "stage": "first", "order": order, "does": "std/run", "command": command })
}

fn ran(at: &Path) -> String {
    std::fs::read_to_string(at.join("ran")).unwrap_or_default()
}

/// What was said of the things done with a card in its column, in order:
/// the word each came to.
async fn came(rig: &Rig, id: &str) -> Vec<String> {
    let query = journal::EventQuery { topics: "stage.came".into(), limit: 10_000, ..Default::default() };
    let all = rig.said(query).await;
    all.iter()
        .filter(|event| event.data["card"] == id)
        .map(|event| event.data["came"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn what_is_laid_before_a_plan_is_done_before_anybody_is_asked_and_what_is_laid_after_it_before_the_card_goes_on()
 {
    let (rig, at) = rig().await;
    shop(&rig, "plan").await;
    add_row(&rig, "segments", runs("Triage", 5, &at, "before", &[])).await;
    add_row(&rig, "segments", runs("Lint", 15, &at, "after", &[])).await;
    says(&at, ARCHITECT, "First the cart's model, then its page.", "propose");

    let id = add(&rig, "Add a cart", &[]).await;
    let on = {
        let id = id.clone();
        rig.event("board.item.moved", move |event| event.data["id"] == id.as_str()).await
    };
    // Its stage handed it on, once what is laid after the plan was done,
    // saying who planned it as whoever planned it would have.
    assert_eq!(
        (on.data["to"].as_str(), on.data["note"].as_str()),
        (Some("planned"), Some("planned by shop-architect-1"))
    );
    assert_eq!(ran(&at), "before\nafter\n");
    assert_eq!(given(&rig, ARCHITECT).await.len(), 1, "asked once, after what is laid before it");
    let plan = rig.read(&format!("/api/records/{id}")).await["data"]["plan"].clone();
    assert_eq!(plan, "First the cart's model, then its page.");
    // What was said of it: the plan was begun once the first thing had
    // passed, and whoever planned it said that it had.
    assert_eq!(came(&rig, &id).await, ["review", "on"]);
    assert!(!said(&rig).await.contains("Move the card on to accept it"), "{}", said(&rig).await);
}

#[tokio::test]
async fn a_card_that_what_is_laid_before_its_plan_does_not_pass_is_left_for_a_person_and_nobody_is_asked() {
    let (rig, at) = rig().await;
    shop(&rig, "plan").await;
    let fails = ["say", "it has no body", "exit", "1"];
    add_row(&rig, "segments", runs("Triage", 5, &at, "before", &fails)).await;

    let id = add(&rig, "Add a cart", &[]).await;
    let line = format!(
        "{id}: what is done with a card in draft sent it back, and nothing is sent back from there: it is left for \
         a person."
    );
    let mut posted = rig.follow("channel.message");
    crate::soon("the stage to say the card is left", async {
        while !said(&rig).await.contains(&line) {
            posted.next().await.unwrap();
        }
    })
    .await;
    assert!(said(&rig).await.contains("it has no body"), "{}", said(&rig).await);
    assert_eq!(card(&rig, &id).await["status"], "draft");
    assert!(given(&rig, ARCHITECT).await.is_empty(), "nobody was asked to plan it");
    assert_eq!(came(&rig, &id).await, ["back"], "what the thing done said, which here leaves the card");
}

/// A blueprint of the project's own that a stage starts for each card:
/// it says what it came to and leaves the handing on to its stage.
const SORTS: &str = "\
blueprint: sorts
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
    with:
      topic: stage.came
      card: \"${params.item}\"
      project: \"${params.project}\"
      stage: \"${params.from}\"
      came: on
      says: sorted
      hands: true
";

#[tokio::test]
async fn a_blueprint_of_the_rows_naming_that_leaves_the_handing_on_to_its_stage_has_what_is_laid_about_it_done()
 {
    let (rig, at) = rig().await;
    let path = format!("@blueprints/{}", brand::blueprint_file("sorts"));
    shop(&rig, "sorts").await;
    let body = json!({ "path": path, "content": SORTS });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    add_row(&rig, "segments", runs("Triage", 5, &at, "before", &[])).await;
    add_row(&rig, "segments", runs("Lint", 15, &at, "after", &[])).await;

    let id = add(&rig, "Add a cart", &[]).await;
    let on = moved(&rig, &id, "planned", 0).await;
    assert_eq!(ran(&at), "before\nafter\n");
    let started =
        journal::EventQuery { topics: "instance.started".into(), limit: 10_000, ..Default::default() };
    let begun = rig.said(started).await;
    let key = format!("sorts-{id}");
    assert_eq!(begun.iter().filter(|event| event.data["key"] == key.as_str()).count(), 1, "started once");
    assert_eq!(came(&rig, &id).await, ["review", "on"]);
    let moves = journal::EventQuery { topics: "board.item.moved".into(), limit: 100, ..Default::default() };
    let moves = rig.said(moves).await;
    let note = moves.iter().find(|event| event.seq == on).unwrap().data["note"].clone();
    assert_eq!(note, "sorted");
}
