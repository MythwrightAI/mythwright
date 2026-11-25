//! Segments: what more is done with a card where it is, said by a row of
//! the project's `segments` and done by a small blueprint the row names.
//!
//! The fleet that ships reviews a card that was worked. A row laid into
//! that list before the review (`std/run`, with a command of the
//! project's own) is run first, in the card's place: a card that passes
//! goes on to be looked at, one that does not goes back with what was
//! printed and nobody is asked to look, or is left for a person where the
//! row says so. A row may be held to one card. What a row says that its
//! blueprint has no param for, and a blueprint of the project's own that
//! nobody vouched for, are said in a sentence and leave the card for a
//! person; nothing is run that should not be. And a list that changes
//! while a card is in one of its segments changes what comes after.
//! A stage does so many things at most with a card in one going through
//! its list (`most` of its row): a card with more to do is left for a
//! person with the command that raises the number, and a number changed
//! while a list is gone through is for the next card.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, add_row, card, change, given, rig, row, shop};
use crate::held::{key, said as refused};
use crate::lanes::{move_card, moved, said};

/// A command that has the stand-in for a program do `words`: a line for
/// the system's shell, as a row's command is, which either system's
/// shell reads the same.
fn does(words: &[&str]) -> String {
    crate::standin::line(words)
}

/// A command either system's shell reads, which passes.
const PASSES: &str = "exit 0";

/// A row of `segments` that runs a command before a card is reviewed.
fn check(title: &str, order: u64, run: &str) -> Value {
    json!({ "title": title, "stage": "review", "order": order, "does": "std/run", "command": run })
}

/// What the journal kept on a topic.
async fn kept(rig: &Rig, topic: &str) -> Vec<kernel::Event> {
    let query = journal::EventQuery { topics: topic.into(), limit: 10_000, ..Default::default() };
    rig.said(query).await
}

/// What was said of what was done with a card before anybody looked, so
/// far: `(order, came)` for each going through, which is the review that
/// begins (`review`, at its order) or what said the word that ended it.
async fn came(rig: &Rig, id: &str) -> Vec<(u64, String)> {
    kept(rig, "stage.came")
        .await
        .iter()
        .filter(|event| event.data["card"] == id)
        .map(|event| {
            (event.data["order"].as_u64().unwrap(), event.data["came"].as_str().unwrap().to_string())
        })
        .collect()
}

/// Waits until what the things done before anybody looks came to has been
/// taken up, after `after` in the journal.
async fn taken_up(rig: &Rig, after: u64) {
    rig.event("run.finished", move |event| event.seq > after && event.data["flow"] == "on.decided").await;
}

#[tokio::test]
async fn a_command_a_row_names_is_run_where_the_card_was_worked_before_anybody_looks_at_it() {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    // What the worker leaves is in the card's own place and not in the
    // project's folder until it is merged: so this passes only there.
    add_row(&rig, "segments", check("Tests", 5, &does(&["has", "CHANGES.md"]))).await;
    assert!(!dir.join("CHANGES.md").exists());
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    moved(&rig, &id, "done", reviewed).await;

    assert_eq!(came(&rig, &id).await, [(10, "review".to_string())]);
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1, "it was looked at once, after its check");
    assert!(dir.join("CHANGES.md").exists(), "and merged");
    // What was run for it was an instance of the row's blueprint, called
    // by the stage for that card.
    let ran: Vec<String> = kept(&rig, "instance.started")
        .await
        .iter()
        .filter(|event| event.data["blueprint"] == "std/run")
        .map(|event| event.data["key"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ran.len(), 1, "{ran:?}");
    assert!(ran[0].starts_with(&format!("segment-{id}-")), "{ran:?}");
}

#[tokio::test]
async fn a_card_whose_command_does_not_pass_goes_back_with_what_was_printed_and_nobody_is_asked_to_look() {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    // The worker adds a line each time it is given the card: the first
    // time there is one, which this does not pass.
    let run = &does(&["fewer", "CHANGES.md", "2", "only one change"]);
    add_row(&rig, "segments", check("Tests", 5, run)).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    let back = moved(&rig, &id, "doing", reviewed).await;
    assert!(given(&rig, "shop-reviewer-1").await.is_empty(), "nobody looked at what did not pass");

    // Its worker is given it again, told what the command printed, and
    // the second time it passes and is looked at.
    let again = moved(&rig, &id, "review", back).await;
    moved(&rig, &id, "done", again).await;
    let worked = given(&rig, "shop-worker-1").await;
    assert_eq!(worked.len(), 2, "{worked:?}");
    assert!(worked[1].contains(&format!("`{run}` did not pass: only one change")), "{}", worked[1]);
    assert_eq!(came(&rig, &id).await, [(5, "back".to_string()), (10, "review".to_string())]);
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);
    assert_eq!(card(&rig, &id).await["more"]["rounds"], 1, "counted as a card sent back is");
}

#[tokio::test]
async fn a_card_whose_command_does_not_pass_is_left_for_a_person_where_its_row_says_so() {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    let run = does(&["say", "the build is broken", "exit", "3"]);
    let mut row = check("Tests", 5, &run);
    row["with"] = json!(["fails=left"]);
    add_row(&rig, "segments", row).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    taken_up(&rig, reviewed).await;

    assert_eq!(card(&rig, &id).await["status"], "review");
    assert_eq!(came(&rig, &id).await, [(5, "left".to_string())]);
    let line = format!("{id}: `{run}` did not pass, and it is left for a person: the build is broken");
    assert!(said(&rig).await.contains(&line), "{}", said(&rig).await);
    assert!(given(&rig, "shop-reviewer-1").await.is_empty());
}

#[tokio::test]
async fn a_row_held_to_one_card_is_run_for_that_card_and_no_other() {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    let one = add(&rig, "Take payments", &[]).await;
    let other = add(&rig, "Fix a typo", &[]).await;
    let mut row = check("Tests", 5, PASSES);
    row["item"] = json!(one);
    add_row(&rig, "segments", row).await;
    for id in [&one, &other] {
        move_card(&rig, id, "ready", None).await;
        let reviewed = moved(&rig, id, "review", 0).await;
        moved(&rig, id, "done", reviewed).await;
    }
    assert_eq!(came(&rig, &one).await, [(10, "review".to_string())]);
    let ran = kept(&rig, "instance.started").await;
    assert_eq!(ran.iter().filter(|event| event.data["blueprint"] == "std/run").count(), 1);
    // The other has nothing before its review: its review is begun at
    // once, and no list is gone through for it.
    assert_eq!(came(&rig, &other).await, [], "the other is looked at with nothing run");
    assert_eq!(ran.iter().filter(|event| event.data["blueprint"] == "std/segments").count(), 1);
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 2, "each was looked at once");
}

#[tokio::test]
async fn a_row_added_while_a_cards_list_is_gone_through_is_for_the_next_card() {
    let (rig, at) = rig().await;
    shop(&rig, json!({})).await;
    let pipe = at.join("hold-check");
    crate::hold(&pipe);
    // The command waits there until it is let go.
    let held = does(&["held", &pipe.to_string_lossy(), "check"]);
    add_row(&rig, "segments", check("Tests", 5, &held)).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    let its = format!("segment-{id}-");
    rig.event("instance.started", move |event| {
        event.seq > reviewed && event.data["key"].as_str().is_some_and(|key| key.starts_with(&its))
    })
    .await;

    // A second thing is added to the list while the first is under way.
    // The list is gone through with the rows it began with: the card is
    // looked at with one thing done, and the next card with both.
    add_row(&rig, "segments", check("Lint", 7, PASSES)).await;
    // The one that waits is let go, and so is every one after it.
    crate::let_go("the check to be waiting", &pipe).await;
    crate::pass(&pipe, 16);
    moved(&rig, &id, "done", reviewed).await;
    let ran = |all: &[kernel::Event]| all.iter().filter(|event| event.data["blueprint"] == "std/run").count();
    assert_eq!(ran(&kept(&rig, "instance.started").await), 1);
    assert_eq!(came(&rig, &id).await, [(10, "review".to_string())]);

    let next = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &next, "ready", None).await;
    let reviewed = moved(&rig, &next, "review", 0).await;
    moved(&rig, &next, "done", reviewed).await;
    assert_eq!(ran(&kept(&rig, "instance.started").await), 3, "the next card has both done");
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 2);
}

const OWN: &str = "\
blueprint: check
description: The project's own check of a card.
params:
  project: { type: string }
  item: { type: string }
steps:
  seen: { do: emit, with: { topic: shop.checked, card: \"${params.item}\" } }
outputs:
  came: on
";

#[tokio::test]
async fn what_a_row_says_wrongly_and_a_blueprint_nobody_vouched_for_leave_the_card_for_a_person_and_run_nothing()
 {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    // A file put among the project's blueprints by something that is not
    // the factory: nobody who holds what the fleet runs with said it is
    // theirs.
    std::fs::create_dir_all(dir.join("blueprints")).unwrap();
    std::fs::write(dir.join("blueprints/check.bp"), OWN).unwrap();
    add_row(&rig, "segments", json!({ "title": "Ours", "stage": "review", "order": 5, "does": "check" }))
        .await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    taken_up(&rig, reviewed).await;
    let line = format!("{id}: Ours was not run: there is no blueprint called `check` that may be run here");
    assert!(said(&rig).await.contains(&line), "{}", said(&rig).await);
    assert_eq!(card(&rig, &id).await["status"], "review");
    assert!(kept(&rig, "shop.checked").await.is_empty(), "it was not run");
    assert!(given(&rig, "shop-reviewer-1").await.is_empty());

    // Saved through the factory by its owner, it is theirs, and is run
    // for the card when it is put there again.
    let saved = json!({ "path": "@blueprints/check.bp", "content": OWN });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    move_card(&rig, &id, "draft", None).await;
    let before = rig.mark();
    move_card(&rig, &id, "review", None).await;
    moved(&rig, &id, "done", before).await;
    assert_eq!(kept(&rig, "shop.checked").await.len(), 1);

    // And a row that says something its blueprint has no param for is
    // refused in a sentence that says what it takes.
    let id = add(&rig, "Add a wish list", &[]).await;
    let mut row = check("Tests", 3, PASSES);
    row["with"] = json!(["colour=red"]);
    row["item"] = json!(id);
    add_row(&rig, "segments", row).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    taken_up(&rig, reviewed).await;
    let line = format!(
        "{id}: Tests was not run: Tests says `colour`, and `std/run` takes nothing called that: what it takes \
         is asking, command, fails"
    );
    assert!(said(&rig).await.contains(&line), "{}", said(&rig).await);
    assert_eq!(card(&rig, &id).await["status"], "review");
}

#[tokio::test]
async fn a_row_that_will_never_be_run_as_written_is_said_where_the_fleet_reports() {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    // A row that names nothing where nothing is usual takes nothing out,
    // and the stage whose row it is says so when it has taken it.
    add_row(&rig, "segments", json!({ "title": "Gone", "stage": "work", "order": 12 })).await;
    let begins =
        "The row Gone of `segments` names nothing to do (`does`), and nothing is usually done at 12 in ready";
    rig.event("channel.message", move |event| {
        event.data["body"].as_str().is_some_and(|body| body.starts_with(begins))
    })
    .await;
}

#[tokio::test]
async fn a_command_is_written_into_a_row_only_by_somebody_who_holds_shell() {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    // Somebody who may change the fleet's rows and holds no `shell`.
    let holds = ["projects", "channels", "board", "records", "blueprints", "agents", "fleet", "search"];
    let narrow = key(&rig, &holds).await;
    let add = |fields: Value| {
        let (rig, narrow) = (&rig, narrow.clone());
        async move {
            let body = json!({ "fields": fields });
            rig.call(Method::POST, "/api/collections/shop/segments/records", Some(&narrow), Some(body)).await
        }
    };
    let (status, why) = add(check("Tests", 5, "cargo test")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{why}");
    assert!(refused(&why).contains("`shell`"), "{why}");
    // A row that says no command is theirs to add, as a stage's row is.
    let (status, added) =
        add(json!({ "title": "Ours", "stage": "review", "order": 5, "does": "check" })).await;
    assert_eq!(status, StatusCode::OK, "{added}");
}

#[tokio::test]
async fn a_stage_that_does_fewer_things_at_most_than_a_cards_list_holds_leaves_it_saying_how_to_raise_that() {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    // A round where cards are worked is four things here: the work, two
    // commands, and the keeping of it; the stage does two at most.
    let work = |title: &str, order: u64| json!({ "title": title, "stage": "work", "order": order, "does": "std/run", "command": PASSES });
    add_row(&rig, "segments", work("Tidy", 12)).await;
    add_row(&rig, "segments", work("Lint", 14)).await;
    change(&rig, "stages", "work", json!({ "most": 2 })).await;
    let stage = row(&rig, "stages", "work").await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let left = rig.event("card.left", |event| event.data["card"] == id).await;
    let line = format!(
        "{id}: there is more to do with it in ready than is gone through at once (2 things at most), and Lint \
         was not begun: it is left for a person. Take a row of `segments` away, put two of them in one \
         blueprint, or raise how many are done: `{} record set {stage} most=3`",
        brand::CLI
    );
    assert_eq!(left.data["why"], line.as_str());
    assert!(said(&rig).await.contains(&line), "{}", said(&rig).await);
    assert_eq!(card(&rig, &id).await["status"], "doing", "left where it is");
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1, "its work was done all the same");

    // Raised, the card made ready again has every thing done, and goes on.
    change(&rig, "stages", "work", json!({ "most": 4 })).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    moved(&rig, &id, "done", reviewed).await;
}

#[tokio::test]
async fn a_stage_given_fewer_things_at_most_while_a_cards_list_is_gone_through_holds_the_next_card_to_that() {
    let (rig, at) = rig().await;
    shop(&rig, json!({})).await;
    let pipe = at.join("hold-check");
    crate::hold(&pipe);
    let held = does(&["held", &pipe.to_string_lossy(), "check"]);
    add_row(&rig, "segments", check("Tests", 5, &held)).await;
    add_row(&rig, "segments", check("Lint", 7, PASSES)).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    let its = format!("segment-{id}-");
    rig.event("instance.started", move |event| {
        event.seq > reviewed && event.data["key"].as_str().is_some_and(|key| key.starts_with(&its))
    })
    .await;

    // The stage is told to do one thing at most while the first of two is
    // under way. The list under way is gone through as it was begun, with
    // the number it was begun with: both are done and the card is looked at.
    change(&rig, "stages", "review", json!({ "most": 1 })).await;
    crate::let_go("the check to be waiting", &pipe).await;
    crate::pass(&pipe, 16);
    moved(&rig, &id, "done", reviewed).await;
    let ran = |all: &[kernel::Event]| all.iter().filter(|event| event.data["blueprint"] == "std/run").count();
    assert_eq!(ran(&kept(&rig, "instance.started").await), 2);

    // The next card's list is begun with the number as it is now: one
    // thing is done, and it is left for a person, saying how to raise it.
    let stage = row(&rig, "stages", "review").await;
    let next = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &next, "ready", None).await;
    let left = rig.event("card.left", |event| event.data["card"] == next).await;
    assert!(
        left.data["why"].as_str().unwrap().ends_with(&format!("`{} record set {stage} most=2`", brand::CLI)),
        "{}",
        left.data
    );
    assert_eq!(ran(&kept(&rig, "instance.started").await), 3);
    assert_eq!(card(&rig, &next).await["status"], "review", "left where it is");
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1, "and nobody was asked to look");
}

#[tokio::test]
async fn a_card_reviewed_while_another_cards_gate_runs_after_its_review_is_approved_without_waiting_for_it() {
    // Two who work and two who review, each card looked at by one: two cards are in review at once.
    let (rig, at) = rig().await;
    shop(&rig, json!({ "workers": 2, "reviewers": 2, "seats": 1 })).await;
    // Each card's work is the one file changed, so nothing is merged: two cards worked side by side
    // would otherwise each be merged over the other.
    change(&rig, "stages", "review", json!({ "merge": false })).await;
    let pipe = at.join("hold-gate");
    crate::hold(&pipe);
    let held = does(&["held", &pipe.to_string_lossy(), "gate"]);
    let first = add(&rig, "Take payments", &[]).await;
    let second = add(&rig, "Fix a typo", &[]).await;
    // A gate after the review, of the first card alone, that waits until it is let go.
    let mut row = check("Gate", 15, &held);
    row["item"] = json!(first);
    add_row(&rig, "segments", row).await;

    move_card(&rig, &first, "ready", None).await;
    let approved = |card: String| move |event: &kernel::Event| event.data["card"] == card;
    rig.event("card.approved", approved(first.clone())).await;
    rig.event("instance.started", |event| event.data["blueprint"] == "std/run").await;
    // While the first card's gate is under way, the second is worked, reviewed and approved: what a
    // review of a card comes to takes its turn by that card, and not behind what another card's does
    // (an approval was said seconds after its review passed, behind two gates).
    move_card(&rig, &second, "ready", None).await;
    rig.event("card.approved", approved(second.clone())).await;
    moved(&rig, &second, "done", 0).await;
    assert_eq!(card(&rig, &first).await["status"], "review", "the first waits for its gate");

    crate::let_go("the gate to be waiting", &pipe).await;
    moved(&rig, &first, "done", 0).await;
}

#[tokio::test]
async fn a_row_that_takes_the_reviews_place_has_the_card_merged_and_done_by_the_usual_merge_after_it() {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    // A command of the project's own where the review usually is: nobody is asked to look, and what is
    // usually done after the review (the merge, at 20) is done after it, in the going through that began
    // as the card came (that one was handed no column, and the merge refused to be called).
    add_row(&rig, "segments", check("Checked", 10, PASSES)).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    moved(&rig, &id, "done", reviewed).await;
    assert!(dir.join("CHANGES.md").exists(), "merged");
    assert!(given(&rig, "shop-reviewer-1").await.is_empty(), "nobody was asked to look");
}

/// The text that ships under `name`, with a step put first that waits at `pipe` until it is let go.
fn held_first(rig: &Rig, name: &str, pipe: &std::path::Path) -> String {
    let ships = rig.node().app.engine.library().shipped(name, None).unwrap();
    let wait = crate::standin::cmd(&["held", &pipe.to_string_lossy(), "gate"]);
    let first = format!("steps:\n  gate:\n    do: shell\n    with: {{ cmd: {wait} }}\n");
    assert!(ships.contains("\nsteps:\n"), "{ships}");
    ships.replacen("\nsteps:\n", &format!("\n{first}"), 1)
}

#[tokio::test]
async fn a_card_reviewed_while_another_cards_hand_merge_is_under_way_is_pressed_reviewed_and_done_without_waiting_for_it()
 {
    let (rig, at) = rig().await;
    shop(&rig, json!({})).await;
    // A card passed by hand is merged, and a card in review waits until Signed is pressed. Nothing is
    // merged after a review, so that the two cards' work, each the one file changed, is not merged over
    // the other's.
    change(&rig, "stages", "review", json!({ "by_hand": "merge", "merge": false })).await;
    add_row(&rig, "actions", json!({ "title": "Signed", "column": "review", "holds": true })).await;
    // The project's own hand merge, which waits at the gate until it is let go before it merges.
    let pipe = at.join("hold-gate");
    crate::hold(&pipe);
    let fork = json!({ "path": "@blueprints/std/pass-by-hand.bp", "content": held_first(&rig, "std/pass-by-hand", &pipe) });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(fork)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    let first = add(&rig, "Take payments", &[]).await;
    move_card(&rig, &first, "ready", None).await;
    moved(&rig, &first, "review", 0).await;
    rig.event("board.item.held", {
        let first = first.clone();
        move |event| event.data["id"] == first.as_str()
    })
    .await;
    move_card(&rig, &first, "done", None).await;
    rig.event("instance.started", |event| event.data["blueprint"] == "std/pass-by-hand").await;

    // While the first card's merge waits, the second comes to review, is held there, and is pressed:
    // what is pressed is taken up, and the card reviewed and done, beside the hand merge and not behind
    // it (the stage took up nothing else until a hand merge was over).
    let second = add(&rig, "Fix a typo", &[]).await;
    move_card(&rig, &second, "ready", None).await;
    moved(&rig, &second, "review", 0).await;
    rig.event("board.item.held", {
        let second = second.clone();
        move |event| event.data["id"] == second.as_str()
    })
    .await;
    let body = Some(json!({ "action": "signed", "back": false }));
    let (status, answer) =
        rig.call(Method::POST, &format!("/api/board/items/{second}/press"), Some(&rig.token), body).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    rig.event("card.approved", {
        let second = second.clone();
        move |event| event.data["card"] == second.as_str()
    })
    .await;
    moved(&rig, &second, "done", 0).await;

    crate::let_go("the gate to be waiting", &pipe).await;
    rig.event("card.merged", move |event| event.data["card"] == first.as_str()).await;
}
