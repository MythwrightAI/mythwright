//! An approved card whose branch no longer merges into the project's.
//!
//! Its worker works behind a wall that lets it read the repository's own
//! files and not write them, so it cannot merge anything itself. The
//! fleet merges the project's branch into the card's where it was worked,
//! and where files conflict leaves that merge under way and gives the
//! card back to its worker, told which files: the worker changes files
//! and nothing else, and the commit that keeps its work ends the merge.
//! Where nothing conflicts that way the merge is made at once and the
//! card's branch goes in, with nobody asked anything. Going back for that
//! is no verdict on the work, and is not counted against how often the
//! stage lets a card be sent back. And how often that is, raised, takes
//! up a card left for a person under the limit it had.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, card, change, given, hold, rig_told, says, shop};
use crate::lanes::{asking, git, move_card, moved};

/// A node whose worker, told that a merge is under way for it, writes the
/// file that conflicts whole, with no marker left; and otherwise does what
/// `first` says and adds a line to that file, as `fleets` has it.
async fn rig(first: &[&str]) -> (Rig, PathBuf) {
    let mut writes = vec![
        "read",
        "where",
        "is under way for you",
        "(",
        "write",
        "CHANGES.md",
        "both",
        "say",
        "I put it right.",
        ")",
    ];
    writes.extend(first);
    writes.extend(["append", "CHANGES.md", "changed", "say", "I changed a file."]);
    let writes = crate::standin::argv(&writes);
    rig_told(move |config| {
        config.adapters.retain(|(name, _)| name != "writer");
        config.adapters.push(("writer".into(), writes));
    })
    .await
}

/// A card made ready, worked, and approved once the project's own branch
/// has changed the same file while its reviewer looked: the card, the
/// project's folder, and the commit the project's branch is at. Every
/// review after that one is let through.
async fn approved_behind(rig: &Rig, at: &Path, more: Value) -> (String, PathBuf, String) {
    let pipe = hold(at);
    let dir = shop(rig, more).await;
    let id = add(rig, "Add a cart", &[]).await;
    move_card(rig, &id, "ready", None).await;
    rig.event("agent.turn.started", |event| event.data["slot"] == "shop-reviewer-1").await;
    std::fs::write(dir.join("CHANGES.md"), "something else\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "meanwhile"]);
    let meanwhile = git(&dir, &["rev-parse", "HEAD"]);
    crate::pass(&pipe, 8);
    (id, dir, meanwhile)
}

/// The merges made on a branch that its worker's commit ended: each with
/// its parents.
fn ended_by_the_worker(dir: &Path, branch: &str) -> Vec<String> {
    let merges = git(dir, &["log", "--merges", "--author=shop-worker-1", "--format=%P", branch]);
    merges.lines().map(str::to_string).collect()
}

/// How often the card was moved from review back to doing.
async fn sent_back(rig: &Rig, id: &str) -> usize {
    let all = journal::EventQuery { topics: "board.item.moved".into(), limit: 1000, ..Default::default() };
    let moves = rig.said(all).await;
    moves
        .iter()
        .filter(|event| {
            event.data["id"] == id && event.data["from"] == "review" && event.data["to"] == "doing"
        })
        .count()
}

#[tokio::test]
async fn an_approved_branch_that_no_longer_merges_has_the_projects_branch_merged_into_it_and_its_worker_puts_right_only_the_files()
 {
    let (rig, at) = rig(&[]).await;
    let (id, dir, meanwhile) = approved_behind(&rig, &at, json!({})).await;

    // The card goes back to whoever worked it, told the merge is under
    // way for it and which files conflict, and to change files and
    // nothing else; the project's branch is as it was.
    let back = rig
        .event("board.item.moved", |event| event.data["from"] == "review" && event.data["to"] == "doing")
        .await;
    let note = back.data["note"].as_str().unwrap();
    assert!(
        note.starts_with(&format!("It was approved, and its branch lane/{id} no longer merges"))
            && note.contains("that merge is under way for you: what conflicts is marked in CHANGES.md")
            && note.contains("do not run git"),
        "{note}"
    );
    assert_eq!(back.data["uncounted"], true, "{back:?}");
    let done = moved(&rig, &id, "done", back.seq).await;
    let worked = given(&rig, "shop-worker-1").await;
    assert_eq!(worked.len(), 2, "{worked:?}");
    assert!(worked[1].contains("is under way for you"), "{}", worked[1]);

    // What the worker left ended the merge begun for it: the card's branch
    // has the project's in its history, and it went in on its own, with
    // what the worker put right.
    let ended = ended_by_the_worker(&dir, "HEAD");
    assert_eq!(ended.len(), 1, "{ended:?}");
    assert!(ended[0].ends_with(&meanwhile), "the project's branch is the merge's second parent: {ended:?}");
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "both\n");
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), format!("{id}: Add a cart"));
    let reviewed = given(&rig, "shop-reviewer-1").await;
    assert_eq!(reviewed.len(), 2, "looked at again once it was put right");
    assert!(done > back.seq);
}

#[tokio::test]
async fn a_card_sent_back_only_because_its_branch_no_longer_merges_is_not_counted_toward_its_stages_rounds() {
    let (rig, at) = rig(&[]).await;
    // Sent back once more than none is too often here: a sending back that
    // counted would leave the card for a person when it came back.
    let (id, _, _) = approved_behind(&rig, &at, json!({ "rounds": 1 })).await;
    moved(&rig, &id, "done", 0).await;
    assert_eq!(rig.count_of("card.left").await, 0);
    assert_eq!(sent_back(&rig, &id).await, 1);
    assert!(card(&rig, &id).await["more"]["rounds"].is_null(), "{}", card(&rig, &id).await);
}

/// Waits until the stage that reviews has been handed the row of
/// `segments` (as `segmented` waits for every stage): adding a row waits
/// for the fleet to say it was arranged, which an arrangement begun before
/// the row may say.
async fn handed_a_row(rig: &Rig) {
    crate::soon("the stage that reviews to be handed the row of segments", async {
        loop {
            let listed = rig.read("/api/instances?project=shop&live=true&limit=1000").await;
            let all = listed["instances"].as_array().cloned().unwrap_or_default();
            let reviewing = all.iter().find(|one| one["blueprint"] == "std/reviewing");
            if reviewing
                .is_some_and(|one| one["params"]["segments"].as_array().is_some_and(|rows| rows.len() == 1))
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
}

#[tokio::test]
async fn going_back_to_be_brought_up_to_date_is_not_counted_where_the_merge_is_among_the_projects_segments() {
    let (rig, at) = rig(&[]).await;
    let pipe = hold(&at);
    let dir = shop(&rig, json!({ "rounds": 1 })).await;
    // A command of the project's own, between the looking and the merge:
    // the merge is then gone through as one of a list.
    let row = json!({ "title": "Licences", "stage": "review", "order": 15, "does": "std/run", "command": "exit 0" });
    crate::fleets::add_row(&rig, "segments", row).await;
    handed_a_row(&rig).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    rig.event("agent.turn.started", |event| event.data["slot"] == "shop-reviewer-1").await;
    std::fs::write(dir.join("CHANGES.md"), "something else\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "meanwhile"]);
    crate::pass(&pipe, 8);

    let back = rig
        .event("board.item.moved", |event| event.data["from"] == "review" && event.data["to"] == "doing")
        .await;
    assert_eq!(back.data["uncounted"], true, "{back:?}");
    moved(&rig, &id, "done", back.seq).await;
    assert_eq!(rig.count_of("card.left").await, 0);
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "both\n");
}

#[tokio::test]
async fn where_the_projects_branch_merges_into_the_cards_without_conflict_the_card_goes_in_and_nobody_is_asked_again()
 {
    // Its worker says, where it works, that the file is merged by putting
    // both sides together: so the project's branch goes into the card's
    // where it was worked, and not the other way, in the project's folder.
    let (rig, at) = rig(&["write", ".gitattributes", "CHANGES.md merge=union"]).await;
    let (id, dir, meanwhile) = approved_behind(&rig, &at, json!({})).await;
    moved(&rig, &id, "done", 0).await;
    assert_eq!(sent_back(&rig, &id).await, 0);
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);
    let changes = std::fs::read_to_string(dir.join("CHANGES.md")).unwrap();
    assert!(changes.contains("changed") && changes.contains("something else"), "{changes}");
    // The card's branch had the project's merged into it, and then went in.
    let first = git(&dir, &["log", "-1", "--format=%P", "HEAD^2"]);
    assert!(first.ends_with(&meanwhile), "{first}");
}

#[tokio::test]
async fn a_worker_that_leaves_what_git_marked_as_conflicting_is_given_the_card_again_and_nothing_marked_is_committed()
 {
    // The worker of `fleets`, which adds a line to the file whatever it is told.
    let (rig, at) = crate::fleets::rig().await;
    let (id, dir, meanwhile) = approved_behind(&rig, &at, json!({})).await;
    let left = rig.event("card.left", |_| true).await;
    assert_eq!(left.data["card"], id.as_str());
    let worked = given(&rig, "shop-worker-1").await;
    assert!(worked.len() >= 3, "{worked:?}");
    assert!(worked[1].contains("is under way for you"), "{}", worked[1]);
    assert!(worked[2].contains("what git marked as conflicting is left in CHANGES.md"), "{}", worked[2]);
    // Nothing with a marker in it was committed, and the merge begun for
    // the worker is under way still, for whoever puts it right.
    let marked =
        asking(&dir).args(["grep", "-q", "<<<<<<<", &format!("lane/{id}"), "--", "CHANGES.md"]).output();
    assert!(!marked.unwrap().status.success(), "a marker was committed");
    let lane = dir.join(".lanes").join(&id);
    assert_eq!(git(&lane, &["rev-parse", "MERGE_HEAD"]), meanwhile);
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), "meanwhile");
    assert_eq!(card(&rig, &id).await["status"], "doing");
}

/// Whether the daemon is to stop at an event, with whether the card was
/// approved by then.
type AtPlace = fn(&kernel::Event, &AtomicBool) -> bool;

#[tokio::test]
async fn stopped_once_the_merge_is_begun_for_the_worker_it_ends_as_one_never_stopped() {
    // Stopped as the merge into the card's branch is begun and before the
    // card goes back; and once it has gone back and before its worker is
    // asked.
    let places: [(&str, AtPlace); 2] = [
        ("as the merge is begun", |event, approved| {
            if event.topic == "review.passed" {
                approved.store(true, Ordering::SeqCst);
            }
            approved.load(Ordering::SeqCst)
                && event.topic == "step.finished"
                && event.source.step.as_deref() == Some("merged")
        }),
        ("as the card goes back", |event, _| {
            event.topic == "board.item.moved" && event.data["from"] == "review" && event.data["to"] == "doing"
        }),
    ];
    for (place, at_place) in places {
        let (mut rig, at) = rig(&[]).await;
        let journal = rig.node().app.journal.clone();
        let approved = Arc::new(AtomicBool::new(false));
        journal.shut_after(move |event| at_place(event, &approved));
        let (id, dir, meanwhile) = approved_behind(&rig, &at, json!({})).await;
        journal.shut().await;
        let lane = dir.join(".lanes").join(&id);
        assert_eq!(git(&lane, &["rev-parse", "MERGE_HEAD"]), meanwhile, "{place}: begun before the stop");
        rig.restart().await;

        moved(&rig, &id, "done", 0).await;
        assert_eq!(sent_back(&rig, &id).await, 1, "{place}");
        let worked = given(&rig, "shop-worker-1").await;
        assert_eq!(worked.len(), 2, "{place}: {worked:?}");
        let ended = ended_by_the_worker(&dir, "HEAD");
        assert_eq!(ended.len(), 1, "{place}: {ended:?}");
        assert!(ended[0].ends_with(&meanwhile), "{place}: {ended:?}");
        assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "both\n", "{place}");
        assert_eq!(rig.count_of("card.left").await, 0, "{place}");
    }
}

#[tokio::test]
async fn a_card_left_for_a_person_having_been_sent_back_as_often_as_allowed_is_taken_up_again_when_that_is_raised()
 {
    let (rig, at) = crate::fleets::rig().await;
    shop(&rig, json!({ "rounds": 1 })).await;
    says(&at, "shop-reviewer-1", "The total is wrong.", "send_back");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let left = rig.event("card.left", |_| true).await;
    assert_eq!(card(&rig, &id).await["status"], "review");

    // Raised, the card is looked at again; and passes, this time.
    std::fs::remove_file(at.join("says-shop-reviewer-1")).unwrap();
    change(&rig, "stages", "review", json!({ "rounds": 3 })).await;
    moved(&rig, &id, "done", left.seq).await;
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 2);
}
