//! A round as a list: what is done with a card where it is worked is the
//! work, the keeping of it and the sending, each a small blueprint, and
//! what the project's `segments` lays between them or in the place of one.
//!
//! A row between the work and the keeping (`std/run`, with a command of
//! the project's own) is run where the card was worked before anything is
//! committed, so that what it leaves is in the commit. Its command is
//! handed the card (`CARD`, `CARD_TITLE`, `CARD_LABELS`), and is run in
//! the project's folder where its row says `in=project`; one laid before
//! the work, where the card has no place yet, is not run at all. A card whose
//! command does not pass goes back to whoever worked it, told what was
//! printed, and is left for a person when that has been as often as a
//! card is given again. A row at the place of the work, held to a label,
//! has cards with that label worked the way it says: by one agent, here.
//! And a row of an agent's turn between the work and the keeping
//! (`std/turn`) has a second agent look at the work, ending with the
//! words the row offers it, each coming to what the row says; and one
//! that says `seats` has two look at once, each for itself (a council),
//! what they come to together coming to what the row says of that.

use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add_row, card, given, rig, says, shop};
use crate::lanes::{git, move_card, moved, said};

/// A row of `segments` that runs a command in a round, after the work.
fn tidy(run: &str) -> Value {
    json!({ "title": "Tidy", "stage": "work", "order": 15, "does": "std/run", "command": run })
}

async fn add(rig: &Rig, title: &str, labels: &[&str]) -> String {
    let card = json!({ "title": title, "labels": labels });
    rig.send("/api/board/shop/items", card).await["id"].as_str().unwrap().to_string()
}

/// The blueprints that were started for a card's round, by what each is called.
async fn segments_of(rig: &Rig, id: &str) -> Vec<(String, String)> {
    let query =
        journal::EventQuery { topics: "instance.started".into(), limit: 10_000, ..Default::default() };
    let its = format!("segment-{id}-");
    rig.said(query)
        .await
        .iter()
        .filter(|event| event.data["key"].as_str().is_some_and(|key| key.starts_with(&its)))
        .map(|event| {
            let text = |name: &str| event.data[name].as_str().unwrap().to_string();
            (text("key"), text("blueprint"))
        })
        .collect()
}

#[tokio::test]
async fn a_command_a_row_lays_between_the_work_and_the_keeping_is_run_before_anything_is_committed() {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    // It passes only where the worker left its change, and what it leaves
    // itself is to be in the commit.
    let tidies = crate::standin::line(&["has", "CHANGES.md", "write", "TIDIED.md", "tidied"]);
    add_row(&rig, "segments", tidy(&tidies)).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;

    // What the command left was committed with the work, on the card's branch.
    let kept = git(&dir, &["show", "--stat", "--format=%an", &format!("lane/{id}")]);
    assert!(kept.contains("TIDIED.md") && kept.contains("CHANGES.md"), "{kept}");
    assert!(kept.starts_with("shop-worker-1"), "the commit is the agent's: {kept}");
    moved(&rig, &id, "done", reviewed).await;
    assert!(dir.join("TIDIED.md").exists(), "and merged");

    // The round was the work, the row's command, and the keeping, in that order.
    let did: Vec<String> = segments_of(&rig, &id).await.into_iter().map(|(_, blueprint)| blueprint).collect();
    assert_eq!(did, ["std/work", "std/run", "std/keep"]);
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);
}

#[tokio::test]
async fn a_rows_command_is_handed_its_card_and_is_run_where_the_card_was_worked_or_in_the_projects_folder() {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    // After the work, where the card was worked: what it writes there is
    // committed with the work. And once the card is approved and merged,
    // a line in the project's own folder, as a changelog is kept.
    let sees = crate::standin::line(&[
        "write",
        "SEEN.md",
        "card %CARD% called %CARD_TITLE% labelled %CARD_LABELS% in %:cwd%",
    ]);
    add_row(&rig, "segments", tidy(&sees)).await;
    let logs = crate::standin::line(&["append", "CHANGELOG.md", "done %CARD% %CARD_TITLE% in %:cwd%"]);
    let changelog = json!({
        "title": "Changelog", "stage": "review", "order": 25, "does": "std/run", "command": logs,
        "with": ["in=project"],
    });
    add_row(&rig, "segments", changelog).await;
    let id = add(&rig, "Add a cart", &["cart", "docs"]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    moved(&rig, &id, "done", reviewed).await;

    let seen = std::fs::read_to_string(dir.join("SEEN.md")).expect("what it saw was committed and merged");
    let (said, cwd) = seen.trim().split_once(" in ").unwrap();
    assert_eq!(said, format!("card {id} called Add a cart labelled cart,docs"));
    assert!(std::path::Path::new(cwd).ends_with(std::path::Path::new(".lanes").join(&id)), "{cwd}");
    let changelog = std::fs::read_to_string(dir.join("CHANGELOG.md")).unwrap();
    let (said, cwd) = changelog.trim().split_once(" in ").unwrap();
    assert_eq!(said, format!("done {id} Add a cart"));
    assert_eq!(kernel::whole::whole(std::path::Path::new(cwd)).unwrap(), kernel::whole::whole(&dir).unwrap());
}

#[tokio::test]
async fn a_rows_command_laid_before_the_work_where_the_card_has_no_place_yet_is_not_run_and_the_card_is_left()
{
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    let run = crate::standin::line(&["write", "RAN.md", "ran"]);
    let row = json!({ "title": "Early", "stage": "work", "order": 5, "does": "std/run", "command": run });
    add_row(&rig, "segments", row).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let left = rig
        .event("channel.message", move |event| {
            event.data["body"].as_str().is_some_and(|body| body.contains("it is left for a person"))
        })
        .await;
    crate::lanes::at_rest(&rig, left.seq).await;

    // Nothing was run in the project's folder in its place, and nobody
    // was given the card: the row says what to write to have it run there.
    let told = said(&rig).await;
    assert!(
        told.contains(&format!("{id}: `{run}` was not run, for it has no place of its own here")),
        "{told}"
    );
    assert!(told.contains("A row that says `in=project` in `with`"), "{told}");
    assert!(!dir.join("RAN.md").exists());
    assert!(given(&rig, "shop-worker-1").await.is_empty());
}

#[tokio::test]
async fn a_card_whose_command_does_not_pass_goes_back_to_whoever_worked_it_with_what_was_printed() {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    // The worker adds a line each time it is given the card: the first
    // time there is one, which this does not pass.
    let run = &crate::standin::line(&["fewer", "CHANGES.md", "2", "only one change"]);
    add_row(&rig, "segments", tidy(run)).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    moved(&rig, &id, "done", reviewed).await;

    // It was given to its worker twice, the second time told what the
    // command printed, as a card sent back is told what was said; nothing
    // was committed or handed on between the two.
    let worked = given(&rig, "shop-worker-1").await;
    assert_eq!(worked.len(), 2, "{worked:?}");
    assert!(worked[1].contains("This card was sent back"), "{}", worked[1]);
    assert!(worked[1].contains(&format!("`{run}` did not pass: only one change")), "{}", worked[1]);
    assert!(!worked[1].contains("could not be committed"), "{}", worked[1]);
    let did: Vec<String> = segments_of(&rig, &id).await.into_iter().map(|(_, blueprint)| blueprint).collect();
    assert_eq!(did, ["std/work", "std/run", "std/work", "std/run", "std/keep"]);
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1, "looked at once, when it had passed");
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap().lines().count(), 2);
    assert_eq!(card(&rig, &id).await["more"]["rounds"].as_u64().unwrap_or(0), 0, "no review sent it back");
}

#[tokio::test]
async fn a_card_sent_back_by_its_command_as_often_as_a_card_is_given_again_is_left_for_a_person() {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    let fails = crate::standin::line(&["say", "it is not laid out", "exit", "1"]);
    add_row(&rig, "segments", tidy(&fails)).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let left = rig
        .event("channel.message", move |event| {
            event.data["body"].as_str().is_some_and(|body| body.contains("it is left for a person"))
        })
        .await;
    crate::lanes::at_rest(&rig, left.seq).await;

    // Given at first and then twice more, which is as often as the stage
    // that works gives a card again; then left where it is, and said so.
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 3);
    let card = card(&rig, &id).await;
    assert_eq!(card["status"], "doing");
    assert_eq!(card["more"]["again"][0], "left", "{card}");
    let told = said(&rig).await;
    assert!(
        told.contains(&format!(
            "{id}: Tidy sent it back, and it has been given again 2 times already: it is left for a person."
        )),
        "{told}"
    );
    assert!(given(&rig, "shop-reviewer-1").await.is_empty());
}

#[tokio::test]
async fn a_card_with_a_label_is_worked_by_the_agent_a_row_names_for_that_label_and_the_rest_as_the_stage_says()
 {
    let (rig, _) = rig().await;
    shop(&rig, json!({ "workers": 2 })).await;
    let row = json!({
        "title": "Docs", "stage": "work", "order": 10, "does": "std/work", "labelled": "docs",
        "with": ["agent=shop-worker-2"],
    });
    add_row(&rig, "segments", row).await;
    let docs = add(&rig, "Write the guide", &["docs"]).await;
    move_card(&rig, &docs, "ready", None).await;
    let reviewed = moved(&rig, &docs, "review", 0).await;
    let done = moved(&rig, &docs, "done", reviewed).await;
    let other = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &other, "ready", None).await;
    let reviewed = moved(&rig, &other, "review", done).await;
    moved(&rig, &other, "done", reviewed).await;

    let second = given(&rig, "shop-worker-2").await;
    assert_eq!(second.len(), 1, "{second:?}");
    assert!(second[0].contains("Write the guide"), "{}", second[0]);
    let first = given(&rig, "shop-worker-1").await;
    assert_eq!(first.len(), 1, "{first:?}");
    assert!(first[0].contains("Add a cart"), "{}", first[0]);
}

#[tokio::test]
async fn a_whole_round_as_it_was_named_in_a_row_does_all_of_it_and_hands_its_card_on_itself() {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    // A project's copy of the round from when a round was one text, kept
    // under a name of its own beside the ones that ship, so that what it
    // tells its agent is the files that ship beside those.
    let before = include_str!("../fixtures/before/std/lane-round.bp")
        .replace("blueprint: std/lane-round", "blueprint: std/round-of-before");
    let path = format!("@blueprints/std/{}", brand::blueprint_file("round-of-before"));
    let body = json!({ "path": path, "content": before });
    let (status, answer) =
        rig.call(reqwest::Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    let row = json!({ "title": "Round", "stage": "work", "order": 10, "does": "std/round-of-before" });
    add_row(&rig, "segments", row).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    moved(&rig, &id, "done", reviewed).await;

    // It was the one thing done in the card's round: it worked the card,
    // kept the work and handed the card on, and the keeping that is usual
    // after the work was not done a second time for it.
    assert_eq!(
        segments_of(&rig, &id).await,
        [(format!("segment-{id}-1"), "std/round-of-before".to_string())]
    );
    let query =
        journal::EventQuery { topics: "board.item.moved".into(), limit: 10_000, ..Default::default() };
    let moves = rig.said(query).await;
    let handed_on =
        moves.iter().find(|event| event.data["id"] == id.as_str() && event.data["to"] == "review");
    assert_eq!(handed_on.unwrap().data["through"], "std/round-of-before");
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);
    assert!(dir.join("CHANGES.md").exists(), "merged");
}

/// A row of `segments` that has one of the role that reviews look at the
/// work in the card's place before it is kept, offered two words.
fn look() -> Value {
    json!({
        "title": "Look", "stage": "work", "order": 15, "does": "std/turn", "role": "reviewer",
        "ends": ["approve=on send_back=back"],
        "with": ["told=Look at what was changed here and say whether it may be kept"],
    })
}

#[tokio::test]
async fn a_second_agent_a_row_lays_after_the_work_looks_at_it_before_it_is_kept_offered_the_words_the_row_says()
 {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({})).await;
    add_row(&rig, "segments", look()).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;

    // The round was the work, the look and the keeping, in that order:
    // the look approved, and the work was kept on the card's branch.
    let did: Vec<String> = segments_of(&rig, &id).await.into_iter().map(|(_, blueprint)| blueprint).collect();
    assert_eq!(did, ["std/work", "std/turn", "std/keep"]);
    let kept = git(&dir, &["show", "--stat", "--format=%an", &format!("lane/{id}")]);
    assert!(
        kept.contains("CHANGES.md") && kept.starts_with("shop-worker-1"),
        "the commit is the worker's: {kept}"
    );
    // Whoever looked was told the row's words with the card, offered the
    // two words the row says, and asked in a conversation of its own.
    let turns = rig.read("/api/agents/shop-reviewer-1/turns").await;
    let first = turns["turns"].as_array().unwrap().last().unwrap().clone();
    let prompt = first["prompt"].as_str().unwrap();
    assert!(prompt.starts_with("Look at what was changed here") && prompt.contains("Add a cart"), "{prompt}");
    assert_eq!(first["options"], json!(["approve", "send_back"]), "{first}");
    assert_eq!(first["thread"], format!("turn:{id}"));
}

#[tokio::test]
async fn work_a_second_agent_sends_back_goes_to_whoever_worked_it_with_what_was_said_and_is_left_after_as_often_as_a_card_is_given_again()
 {
    let (rig, at) = rig().await;
    shop(&rig, json!({})).await;
    add_row(&rig, "segments", look()).await;
    says(&at, "shop-reviewer-1", "The total is wrong.", "send_back");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let left = rig
        .event("channel.message", move |event| {
            event.data["body"].as_str().is_some_and(|body| body.contains("it is left for a person"))
        })
        .await;
    crate::lanes::at_rest(&rig, left.seq).await;

    // Looked at after each round and sent back each time, with what was
    // said; then left in doing, and never reviewed.
    let worked = given(&rig, "shop-worker-1").await;
    assert_eq!(worked.len(), 3, "{worked:?}");
    assert!(worked[1].contains("The total is wrong."), "{}", worked[1]);
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 3);
    let card = card(&rig, &id).await;
    assert_eq!(card["status"], "doing");
    assert_eq!(card["more"]["again"][0], "left", "{card}");
}

/// A row of `segments` that has two of the role that reviews look at the
/// work at once in the card's place before it is kept, both of whom have
/// to approve: a council, laid into the work stage's list.
fn council() -> Value {
    json!({
        "title": "Second look", "stage": "work", "order": 15, "does": "std/turn", "role": "reviewer",
        "seats": 2, "needed": 2, "ends": ["passed=on failed=back"],
    })
}

/// What a turn of an agent's asked, the latest first.
async fn turns_of(rig: &Rig, agent: &str) -> Vec<Value> {
    let turns = rig.read(&format!("/api/agents/{agent}/turns")).await;
    turns["turns"].as_array().unwrap().clone()
}

#[tokio::test]
async fn two_a_row_lays_after_the_work_look_at_it_at_once_and_it_is_kept_once_both_approve() {
    let (rig, _) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 2, "seats": 1 })).await;
    add_row(&rig, "segments", council()).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;

    // The round was the work, the council and the keeping, in that order:
    // both approved, and the work was kept on the card's branch.
    let did: Vec<String> = segments_of(&rig, &id).await.into_iter().map(|(_, blueprint)| blueprint).collect();
    assert_eq!(did, ["std/work", "std/turn", "std/keep"]);
    let kept = git(&dir, &["show", "--stat", "--format=%an", &format!("lane/{id}")]);
    assert!(kept.contains("CHANGES.md") && kept.starts_with("shop-worker-1"), "{kept}");
    // Each looked for itself, in a conversation of its own, offered the
    // two words it is counted by and asked to look at what was done.
    for (seat, reviewer) in ["shop-reviewer-1", "shop-reviewer-2"].iter().enumerate() {
        let turns = turns_of(&rig, reviewer).await;
        let look = turns
            .iter()
            .find(|turn| turn["thread"] == format!("turn:{id}:{}", seat + 1))
            .unwrap_or_else(|| panic!("{reviewer} looked in its own seat: {turns:?}"));
        assert_eq!(look["options"], json!(["approve", "send_back"]), "{look}");
        let prompt = look["prompt"].as_str().unwrap();
        assert!(prompt.contains("what was done for it is in the folder you are in"), "{prompt}");
        assert!(prompt.contains("choose `approve`") && prompt.contains("Add a cart"), "{prompt}");
    }
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);
}

#[tokio::test]
async fn work_one_of_two_who_look_at_once_sends_back_goes_to_whoever_worked_it_with_what_that_one_said() {
    let (rig, at) = rig().await;
    shop(&rig, json!({ "reviewers": 2, "seats": 1 })).await;
    add_row(&rig, "segments", council()).await;
    says(&at, "shop-reviewer-2", "The total is wrong.", "send_back");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let left = rig
        .event("channel.message", move |event| {
            event.data["body"].as_str().is_some_and(|body| body.contains("it is left for a person"))
        })
        .await;
    crate::lanes::at_rest(&rig, left.seq).await;

    // Both are needed, so one sending it back is enough: the card went
    // back to its worker after each round, told who said what, and was
    // left in doing once it had been given again as often as it may be.
    let worked = given(&rig, "shop-worker-1").await;
    assert_eq!(worked.len(), 3, "{worked:?}");
    assert!(worked[1].contains("This card was sent back"), "{}", worked[1]);
    assert!(worked[1].contains("shop-reviewer-2: The total is wrong."), "{}", worked[1]);
    let card = card(&rig, &id).await;
    assert_eq!(card["status"], "doing");
    assert_eq!(card["more"]["again"][0], "left", "{card}");
    let did: Vec<String> = segments_of(&rig, &id).await.into_iter().map(|(_, blueprint)| blueprint).collect();
    assert!(!did.contains(&"std/keep".to_string()), "nothing was kept: {did:?}");
}

#[tokio::test]
async fn two_who_look_at_once_are_offered_the_words_a_row_says_and_are_counted_by_them() {
    let (rig, at) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 2, "seats": 1 })).await;
    let row = json!({
        "title": "Release", "stage": "work", "order": 15, "does": "std/turn", "role": "reviewer",
        "seats": 2, "needed": 2, "yes": "ship", "no": "hold",
    });
    add_row(&rig, "segments", row).await;
    for reviewer in ["shop-reviewer-1", "shop-reviewer-2"] {
        says(&at, reviewer, "Ready to go out.", "ship");
    }
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;

    // Both chose the row's word for the work, which passed it: it was kept.
    let kept = git(&dir, &["show", "--stat", "--format=%an", &format!("lane/{id}")]);
    assert!(kept.contains("CHANGES.md"), "{kept}");
    for reviewer in ["shop-reviewer-1", "shop-reviewer-2"] {
        let turns = turns_of(&rig, reviewer).await;
        let look = turns
            .iter()
            .find(|turn| {
                turn["thread"].as_str().is_some_and(|thread| thread.starts_with(&format!("turn:{id}:")))
            })
            .unwrap_or_else(|| panic!("{reviewer} looked: {turns:?}"));
        assert_eq!(look["options"], json!(["ship", "hold"]), "{look}");
        let prompt = look["prompt"].as_str().unwrap();
        assert!(prompt.contains("choose `ship`") && prompt.contains("choose `hold`"), "{prompt}");
    }
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);
}
