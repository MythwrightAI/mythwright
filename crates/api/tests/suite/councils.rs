//! What a row of a project's `segments` says of a column where cards are
//! reviewed, beyond something run before anybody looks: something done
//! once a card is approved and before its work is merged, and after it;
//! a second review with seats of its own; and a review for the cards
//! with a label in place of the usual one. Each is a row and nothing
//! else.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add_row, card, change, given, rig, shop};
use crate::lanes::{move_card, moved, said};

/// A row of `segments` that runs a command for a card in review.
fn runs(title: &str, order: u64, command: &str) -> Value {
    json!({ "title": title, "stage": "review", "order": order, "does": "std/run", "command": command })
}

async fn kept(rig: &Rig, topic: &str) -> Vec<kernel::Event> {
    let query = journal::EventQuery { topics: topic.into(), limit: 10_000, ..Default::default() };
    rig.said(query).await
}

async fn add(rig: &Rig, title: &str, labels: &[&str]) -> String {
    let card = json!({ "title": title, "labels": labels });
    rig.send("/api/board/shop/items", card).await["id"].as_str().unwrap().to_string()
}

/// How many times either reviewer was asked to look.
async fn looked(rig: &Rig) -> usize {
    given(rig, "shop-reviewer-1").await.len() + given(rig, "shop-reviewer-2").await.len()
}

/// A card made ready, worked and gone to `to`.
async fn through(rig: &Rig, id: &str, to: &'static str) {
    move_card(rig, id, "ready", None).await;
    let reviewed = moved(rig, id, "review", 0).await;
    moved(rig, id, to, reviewed).await;
}

#[tokio::test]
async fn what_a_row_lays_after_the_review_is_done_once_the_card_is_approved_before_and_after_its_work_is_merged()
 {
    let (rig, at) = rig().await;
    let dir = shop(&rig, json!({})).await;
    // The worker's file is in the project's folder only once the card's
    // branch is merged: the first of these passes only before that, and
    // the second only after it.
    let (changes, ran) = (dir.join("CHANGES.md"), at.join("ran"));
    let (changes, ran) = (changes.to_string_lossy(), ran.to_string_lossy());
    let before = crate::standin::line(&["lacks", &changes, "append", &ran, "before"]);
    let after = crate::standin::line(&["has", &changes, "append", &ran, "after"]);
    add_row(&rig, "segments", runs("Licences", 15, &before)).await;
    add_row(&rig, "segments", runs("Announce", 25, &after)).await;
    let id = add(&rig, "Add a cart", &[]).await;
    through(&rig, &id, "done").await;

    assert_eq!(std::fs::read_to_string(at.join("ran")).unwrap(), "before\nafter\n");
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1, "it was looked at once");
    assert!(dir.join("CHANGES.md").exists(), "and merged");
    // The merge's commit says which card, by what the card is called: what
    // is so of the card comes before what its stage is called.
    let merged = crate::lanes::git(&dir, &["log", "-1", "--format=%s"]);
    assert_eq!(merged, format!("{id}: Add a cart"));
    // Nothing was laid before the review: it was begun at once, and the
    // one going through of the list is the one after it.
    assert!(kept(&rig, "stage.came").await.is_empty());
    let lists = kept(&rig, "instance.started").await;
    assert_eq!(lists.iter().filter(|event| event.data["blueprint"] == "std/segments").count(), 1);
}

#[tokio::test]
async fn a_card_that_what_is_done_after_its_review_does_not_pass_is_not_merged_and_goes_back_or_is_left() {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    let fails = crate::standin::line(&["say", "no licence header", "exit", "1"]);
    let mut row = runs("Licences", 15, &fails);
    row["with"] = json!(["fails=left"]);
    add_row(&rig, "segments", row).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    rig.event("run.finished", move |event| event.seq > reviewed && event.data["flow"] == "on.decided").await;

    assert_eq!(card(&rig, &id).await["status"], "review", "left where it is");
    assert!(!dir.join("CHANGES.md").exists(), "and not merged");
    let line = format!("{id}: `{fails}` did not pass, and it is left for a person: no licence header");
    assert!(said(&rig).await.contains(&line), "{}", said(&rig).await);
    assert_eq!(kept(&rig, "card.approved").await.len(), 1, "it was approved all the same, and that is said");

    // Where the row does not say so, it goes back to whoever worked it,
    // with what was printed, and is counted as a card sent back is.
    change(
        &rig,
        "segments",
        "Licences",
        json!({ "with": [], "command": crate::standin::line(&["needs", "../../pass", "no licence header"]) }),
    )
    .await;
    let other = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &other, "ready", None).await;
    let reviewed = moved(&rig, &other, "review", 0).await;
    let back = moved(&rig, &other, "doing", reviewed).await;
    std::fs::write(dir.join("pass"), "").unwrap();
    let again = moved(&rig, &other, "review", back).await;
    moved(&rig, &other, "done", again).await;
    assert_eq!(card(&rig, &other).await["more"]["rounds"], 1, "counted as a card sent back is");
    let worked = given(&rig, "shop-worker-1").await;
    assert!(worked.last().unwrap().contains("did not pass: no licence header"), "{worked:?}");
}

#[tokio::test]
async fn a_second_review_a_row_names_is_begun_once_the_first_has_passed_with_the_seats_its_row_says() {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 2 })).await;
    // The usual review asks one of the two; the second, which a row
    // names, asks both, and both have to approve.
    change(&rig, "stages", "review", json!({ "seats": 1, "needed": 1 })).await;
    let second = json!({
        "title": "Second look", "stage": "review", "order": 15, "does": "std/review", "seats": 2, "needed": 2,
    });
    add_row(&rig, "segments", second).await;
    let id = add(&rig, "Add a cart", &[]).await;
    through(&rig, &id, "done").await;

    assert_eq!(looked(&rig).await, 3, "one looked, and then both");
    let passed: Vec<u64> = kept(&rig, "review.passed")
        .await
        .iter()
        .filter(|event| event.data["card"] == id.as_str())
        .map(|event| event.data["seats"].as_u64().unwrap_or(1))
        .collect();
    assert_eq!(passed, [1, 2], "each review said that it passed, once");
    assert!(dir.join("CHANGES.md").exists(), "and it was merged after the second");
    let moves = kept(&rig, "board.item.moved").await;
    assert_eq!(moves.iter().filter(|event| event.data["to"] == "done").count(), 1);
}

#[tokio::test]
async fn cards_with_a_label_are_reviewed_as_a_row_held_to_the_label_says_and_the_rest_as_the_stage_says() {
    let (rig, _) = rig().await;
    shop(&rig, json!({ "reviewers": 2 })).await;
    change(&rig, "stages", "review", json!({ "seats": 1, "needed": 1 })).await;
    // In the place of the usual review, for cards that hold the label:
    // both look, and both have to approve.
    let payments = json!({
        "title": "Payments", "stage": "review", "order": 10, "does": "std/review", "labelled": "payments",
        "seats": 2, "needed": 2,
    });
    add_row(&rig, "segments", payments).await;
    let plain = add(&rig, "Fix a typo", &[]).await;
    through(&rig, &plain, "done").await;
    assert_eq!(looked(&rig).await, 1, "one looked at a card with no label");

    let paid = add(&rig, "Take payments", &["payments"]).await;
    through(&rig, &paid, "done").await;
    assert_eq!(looked(&rig).await, 3, "and both at the one that holds it");
    let seats: Vec<u64> = kept(&rig, "review.passed")
        .await
        .iter()
        .filter(|event| event.data["card"] == paid.as_str())
        .map(|event| event.data["seats"].as_u64().unwrap_or(1))
        .collect();
    assert_eq!(seats, [2], "one review of it, the row's");
}

/// A blueprint of the project's own that a row names: it waits until
/// somebody has said something on the card, and then the card goes on.
const SIGN_OFF: &str = "\
blueprint: sign-off
description: A person says on the card that it may go on.
params:
  project: { type: string }
  item: { type: string }
steps:
  signed:
    do: wait
    with:
      event: record.commented
      where: event.data.id == params.item
outputs:
  came: on
  says: signed off
";

#[tokio::test]
async fn a_persons_sign_off_is_a_row_that_names_a_blueprint_of_the_projects_own_which_waits_for_them() {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    let saved = json!({ "path": "@blueprints/sign-off.bp", "content": SIGN_OFF });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let row = json!({ "title": "Sign-off", "stage": "review", "order": 15, "does": "sign-off" });
    add_row(&rig, "segments", row).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    // Approved, it waits where it is for a person, and nothing is merged.
    rig.event("instance.started", move |event| event.seq > reviewed && event.data["blueprint"] == "sign-off")
        .await;
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);
    assert_eq!(card(&rig, &id).await["status"], "review");
    assert!(!dir.join("CHANGES.md").exists(), "not merged before it is signed off");

    rig.send(&format!("/api/records/{id}/comments"), json!({ "body": "It may go out." })).await;
    moved(&rig, &id, "done", reviewed).await;
    assert!(dir.join("CHANGES.md").exists(), "and merged once it was");
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1, "nobody was asked to look again");
}

#[tokio::test]
async fn a_stage_that_names_its_own_review_has_its_cards_reviewed_by_that_while_another_stage_has_the_fleets()
{
    let (rig, _) = rig().await;
    let stages = json!([
        { "title": "work", "column": "ready", "does": "work", "role": "worker", "pass": "review" },
        // Looked over by the project's own way, and then by the fleet's.
        {
            "title": "check", "column": "review", "does": "review", "role": "reviewer",
            "review": "looked-over", "pass": "checked",
        },
        { "title": "final", "column": "checked", "does": "review", "role": "reviewer", "pass": "done", "merge": true },
    ]);
    let columns = ["draft", "ready", "doing", "review", "checked", "done"];
    let dir = crate::fleets::shop_with(&rig, &columns, json!({ "stages": stages })).await;
    // The project's own way of reviewing, saved by its owner: it passes a
    // card as it is, asking nobody.
    let saved = json!({
        "path": format!("@blueprints/{}", brand::blueprint_file("looked-over")),
        "content": crate::rows::LOOKED_OVER,
    });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    let checked = moved(&rig, &id, "checked", reviewed).await;
    moved(&rig, &id, "done", checked).await;

    // Each review was of the text its stage says: the first stage's the
    // project's own, which asked nobody, and the second's the fleet's, which
    // asked the one reviewer once.
    let all = rig.read("/api/instances?project=shop&all=true").await;
    let mut reviews = Vec::new();
    for one in all["instances"].as_array().unwrap() {
        if one["key"].as_str().is_some_and(|key| key.starts_with(&format!("review-{id}-"))) {
            let one = rig.read(&format!("/api/instances/{}", one["id"].as_str().unwrap())).await["instance"]
                .clone();
            reviews.push(format!("{} in {}", one["blueprint"].as_str().unwrap(), one["params"]["stage"]));
        }
    }
    reviews.sort();
    assert_eq!(reviews, [r#"looked-over in "review""#, r#"std/review in "checked""#]);
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);
    assert!(dir.join("CHANGES.md").exists(), "merged where the fleet's review passed it");
}
