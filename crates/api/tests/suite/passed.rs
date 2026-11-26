//! A card a person passes themselves, moving it on from where it is
//! reviewed to where its stage hands what passes. On a stage whose row
//! says `merge`, its branch is merged into the project's as one that
//! agents approved is, through the same merge (`std/merge`): where it no
//! longer merges, the project's branch is merged into the card's for its
//! worker and the card goes back to them, uncounted. A move anywhere else
//! merges nothing, and a card with no branch moves as it always did.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;

use crate::Rig;
use crate::fleets::{add, card, change, rig, shop};
use crate::lanes::{git, history, move_card, moved};

/// A card made ready and worked, now waiting in review for a person,
/// nobody of the stage's role being there to look at it: its id.
pub(crate) async fn waiting(rig: &Rig, title: &str) -> String {
    let id = add(rig, title, &[]).await;
    move_card(rig, &id, "ready", None).await;
    moved(rig, &id, "review", 0).await;
    id
}

/// Whether the project's branch has the card's work, merged as a commit
/// that says which card.
pub(crate) fn on_the_projects_branch(dir: &Path, id: &str) -> bool {
    git(dir, &["log", "--format=%s"]).lines().any(|line| line.starts_with(&format!("{id}:")))
}

#[tokio::test]
async fn a_card_a_person_passes_from_a_stage_that_merges_is_merged_and_its_history_says_who_passed_it() {
    let (rig, _at) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    let id = waiting(&rig, "Add a cart").await;
    move_card(&rig, &id, "done", None).await;
    let merged = id.clone();
    rig.event("card.merged", move |event| event.data["card"] == merged.as_str()).await;
    assert!(on_the_projects_branch(&dir, &id));
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), format!("{id}: Add a cart"));
    history(&rig, &id, &format!("its branch lane/{id} was merged into the project's as Ada passed it")).await;
}

/// Waits for the stage that reviews to have heard a move of `id` out of
/// its column after `after`, and done with it whatever it does.
async fn heard_leaving(rig: &Rig, id: &str, after: u64) {
    let id = id.to_string();
    let moved = rig
        .event("board.item.moved", move |event| {
            event.seq > after && event.data["id"] == id.as_str() && event.data["from"] == "review"
        })
        .await;
    rig.event("run.finished", move |event| {
        event.seq > moved.seq
            && event.data["blueprint"] == "std/reviewing"
            && event.data["flow"] == "on.handed"
    })
    .await;
}

/// How many instances of a blueprint were started.
async fn started(rig: &Rig, blueprint: &'static str) -> usize {
    let all = journal::EventQuery { topics: "instance.started".into(), limit: 1000, ..Default::default() };
    rig.said(all).await.iter().filter(|event| event.data["blueprint"] == blueprint).count()
}

#[tokio::test]
async fn a_card_a_person_moves_from_review_anywhere_but_where_it_passes_to_merges_nothing() {
    let (rig, _at) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    let id = waiting(&rig, "Add a cart").await;

    // Put back among what is not begun, and then sent back to be worked.
    let now = rig.mark();
    move_card(&rig, &id, "draft", None).await;
    heard_leaving(&rig, &id, now).await;
    move_card(&rig, &id, "review", None).await;
    let now = rig.mark();
    move_card(&rig, &id, "doing", Some("The total is wrong.")).await;
    heard_leaving(&rig, &id, now).await;
    moved(&rig, &id, "review", now).await;
    assert_eq!(started(&rig, "std/pass-by-hand").await, 0);
    assert_eq!(rig.count_of("card.merged").await, 0);
    assert!(!on_the_projects_branch(&dir, &id));
}

#[tokio::test]
async fn a_card_a_person_passes_from_a_stage_that_does_not_merge_is_done_and_its_branch_left_as_it_is() {
    let (rig, _at) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    change(&rig, "stages", "review", json!({ "merge": false })).await;
    let id = waiting(&rig, "Add a cart").await;
    let now = rig.mark();
    move_card(&rig, &id, "done", None).await;
    rig.event("run.finished", move |event| event.seq > now && event.data["flow"] == "on.tidied").await;
    assert_eq!(started(&rig, "std/pass-by-hand").await, 0);
    assert!(!on_the_projects_branch(&dir, &id));
    assert_eq!(git(&dir, &["branch", "--list", &format!("lane/{id}")]), format!("lane/{id}"));
    assert_eq!(card(&rig, &id).await["status"], "done");
}

#[tokio::test]
async fn a_card_with_no_branch_of_its_own_passed_by_a_person_is_done_with_nothing_merged() {
    let (rig, _at) = rig().await;
    // A repository with no history yet: the card is worked in the
    // project's own folder, and has no branch.
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = std::path::PathBuf::from(project["path"].as_str().unwrap());
    git(&dir, &["init", "--quiet", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    rig.board("shop").await;
    crate::fleets::fleet(&rig, json!({ "reviewers": 0 })).await;
    let id = waiting(&rig, "Add a cart").await;
    assert_eq!(git(&dir, &["branch", "--list", &format!("lane/{id}")]), "");

    let now = rig.mark();
    move_card(&rig, &id, "done", None).await;
    rig.event("run.finished", move |event| event.seq > now && event.data["flow"] == "on.tidied").await;
    let by_hand = format!("by-hand-{id}");
    let ended = rig.event("instance.finished", move |event| event.data["key"] == by_hand.as_str()).await;
    assert_eq!(ended.data["status"], "done", "{ended:?}");
    assert_eq!(rig.count_of("card.merged").await, 0);
    assert_eq!(card(&rig, &id).await["status"], "done");
    assert!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap().contains("changed"));
}

#[tokio::test]
async fn a_card_a_person_passes_whose_branch_no_longer_merges_goes_back_to_its_worker_uncounted_and_goes_in_when_put_right()
 {
    let (rig, _at) = crate::conflicted::rig(&[]).await;
    let dir = shop(&rig, json!({ "reviewers": 0, "rounds": 1 })).await;
    let id = waiting(&rig, "Add a cart").await;
    std::fs::write(dir.join("CHANGES.md"), "something else\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "meanwhile"]);
    let meanwhile = git(&dir, &["rev-parse", "HEAD"]);

    let now = rig.mark();
    move_card(&rig, &id, "done", None).await;
    let back = rig
        .event("board.item.moved", move |event| {
            event.seq > now && event.data["from"] == "done" && event.data["to"] == "doing"
        })
        .await;
    let note = back.data["note"].as_str().unwrap();
    assert!(
        note.starts_with(&format!("It was passed, and its branch lane/{id} no longer merges"))
            && note.contains("that merge is under way for you: what conflicts is marked in CHANGES.md"),
        "{note}"
    );
    assert_eq!(back.data["uncounted"], true, "{back:?}");
    assert_eq!(git(&dir, &["rev-parse", "HEAD"]), meanwhile, "the project's branch is as it was");

    // Put right by its worker, it waits for a person again; passed again,
    // it goes in with what was put right, and was never counted as sent
    // back.
    moved(&rig, &id, "review", back.seq).await;
    let worked = crate::fleets::given(&rig, "shop-worker-1").await;
    assert_eq!(worked.len(), 2, "{worked:?}");
    assert!(worked[1].contains("is under way for you"), "{}", worked[1]);
    move_card(&rig, &id, "done", None).await;
    let merged = id.clone();
    rig.event("card.merged", move |event| event.data["card"] == merged.as_str()).await;
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "both\n");
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), format!("{id}: Add a cart"));
    assert_eq!(card(&rig, &id).await["status"], "done");
    assert!(card(&rig, &id).await["more"]["rounds"].is_null(), "{}", card(&rig, &id).await);
    assert_eq!(rig.count_of("card.left").await, 0);
}

/// Whether the daemon is to stop at an event, with whether the person's
/// pass has been made by then.
type AtPlace = fn(&kernel::Event, &AtomicBool) -> bool;

#[tokio::test]
async fn a_daemon_stopped_between_a_persons_pass_and_its_merge_ends_as_one_never_stopped() {
    // Stopped as the pass is said; once whoever passed it was asked what
    // they hold; once the merge itself is made, before what it came to is
    // said; and once the card is said to be merged, before its place is
    // tidied.
    let places: [(&str, AtPlace); 4] = [
        ("as the pass is said", |event, _| passing(event)),
        ("once whoever passed it was asked", |event, passed| {
            passed_by_then(event, passed) && finished_step(event, "may")
        }),
        ("once the merge is made", |event, passed| {
            passed_by_then(event, passed) && finished_step(event, "merged")
        }),
        ("once it is said to be merged", |event, _| event.topic == "card.merged"),
    ];
    for (place, at_place) in places {
        let (mut rig, _at) = rig().await;
        let dir = shop(&rig, json!({ "reviewers": 0 })).await;
        let id = waiting(&rig, "Add a cart").await;
        let journal = rig.node().app.journal.clone();
        let passed = Arc::new(AtomicBool::new(false));
        journal.shut_after(move |event| at_place(event, &passed));
        move_card(&rig, &id, "done", None).await;
        journal.shut().await;
        rig.restart().await;

        let merged = id.clone();
        rig.event("card.merged", move |event| event.data["card"] == merged.as_str()).await;
        rig.event("run.finished", |event| event.data["flow"] == "on.tidied").await;
        let merges = git(&dir, &["log", "--merges", "--format=%s"]);
        assert_eq!(
            merges.lines().filter(|line| line.starts_with(&format!("{id}:"))).count(),
            1,
            "{place}: {merges}"
        );
        assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), format!("{id}: Add a cart"), "{place}");
        assert_eq!(card(&rig, &id).await["status"], "done", "{place}");
        assert_eq!(rig.count_of("card.merged").await, 1, "{place}");
        assert_eq!(rig.count_of("card.left").await, 0, "{place}");
        history(&rig, &id, &format!("its branch lane/{id} was merged into the project's as Ada passed it"))
            .await;
    }
}

/// The person's pass, as it is said.
fn passing(event: &kernel::Event) -> bool {
    event.topic == "board.item.moved"
        && event.data["from"] == "review"
        && event.data["to"] == "done"
        && event.data["through"].is_null()
}

/// Whether the pass was said by this event or an earlier one.
fn passed_by_then(event: &kernel::Event, passed: &AtomicBool) -> bool {
    if passing(event) {
        passed.store(true, Ordering::SeqCst);
    }
    passed.load(Ordering::SeqCst)
}

/// Whether the event says a step of this name finished.
fn finished_step(event: &kernel::Event, step: &str) -> bool {
    event.topic == "step.finished" && event.source.step.as_deref() == Some(step)
}

#[tokio::test]
async fn where_the_next_card_is_taken_once_the_last_is_merged_one_a_person_passed_is_merged_before_the_next_is_cut()
 {
    let (rig, at) = rig().await;
    let stages = json!([
        {
            "title": "work", "column": "ready", "does": "work", "role": "worker", "pass": "review",
            "next_after": "merged",
        },
        {
            "title": "review", "column": "review", "does": "review", "role": "reviewer",
            "pass": "done", "fail": "doing", "rounds": 2, "merge": true,
        },
    ]);
    let dir = shop(&rig, json!({ "reviewers": 0, "stages": stages })).await;
    // The hand merge waits at a gate until it is let go, so the next card
    // could be taken in between if anything took it then.
    let gate = at.join("hold-gate");
    crate::hold(&gate);
    let fork = crate::segments::held_first(&rig, "std/pass-by-hand", &gate);
    let fork = json!({ "path": "@blueprints/std/pass-by-hand.bp", "content": fork });
    let (status, answer) =
        rig.call(axum::http::Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(fork)).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{answer}");

    let first = waiting(&rig, "Lay the floor").await;
    let second = add(&rig, "Put the shelves up", &[]).await;
    move_card(&rig, &second, "ready", None).await;
    rig.event("card.waiting", |_| true).await;
    move_card(&rig, &first, "done", None).await;
    rig.event("instance.started", |event| event.data["blueprint"] == "std/pass-by-hand").await;
    crate::let_go("the hand merge to be waiting at its gate", &gate).await;

    // The second was worked from the first's work, merged before it was taken.
    moved(&rig, &second, "review", 0).await;
    let lane = dir.join(".lanes").join(&second).join("CHANGES.md");
    assert_eq!(std::fs::read_to_string(lane).unwrap(), "changed\nchanged\n");
    assert!(on_the_projects_branch(&dir, &first));
}
