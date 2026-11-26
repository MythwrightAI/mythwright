//! When a stage that works cards takes the next one: as soon as the last
//! is handed on (the usual), or only once none it worked waits in review
//! unmerged (`next_after: merged` of its row in `stages`). A card's place
//! is cut from the project's branch as it is when its round begins, so a
//! card taken while the last is still in review is worked without that
//! one's work. Played with the stand-ins of the fleet's suite.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::Rig;
use crate::fleets::{add, card, change, given, rig, shop};
use crate::lanes::{move_card, moved};

#[tokio::test]
async fn usually_the_next_card_is_taken_while_the_last_is_in_review_and_is_worked_without_its_work() {
    let (rig, at) = rig().await;
    let hold = at.join("hold");
    crate::hold(&hold);
    let dir = shop(&rig, json!({})).await;
    let first = add(&rig, "Lay the floor", &[]).await;
    let second = add(&rig, "Put the shelves up", &[]).await;
    move_card(&rig, &first, "ready", None).await;
    move_card(&rig, &second, "ready", None).await;

    // The first is in review, its reviewer looking at it, and the second is
    // worked meanwhile: its place was cut from the project's branch, which
    // has nothing of the first's.
    moved(&rig, &first, "review", 0).await;
    rig.event("agent.turn.started", |event| event.data["slot"] == "shop-reviewer-1").await;
    moved(&rig, &second, "review", 0).await;
    assert_eq!(
        std::fs::read_to_string(dir.join(".lanes").join(&first).join("CHANGES.md")).unwrap(),
        "changed\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join(".lanes").join(&second).join("CHANGES.md")).unwrap(),
        "changed\n"
    );
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 2);
    crate::let_go("a reviewer to be waiting", &hold).await;
    moved(&rig, &first, "done", 0).await;
}

/// A shop whose stage of work takes the next card only once the last is
/// merged, with two who work and a reviewer waiting at a hold: the first
/// card worked and in review, its reviewer looking at it, and the second
/// made ready meanwhile and waiting behind it, saying so. The project's
/// folder, the hold, and the two cards.
async fn one_behind(rig: &Rig, at: &Path) -> (PathBuf, PathBuf, String, String) {
    let hold = at.join("hold");
    crate::hold(&hold);
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
    // Two who work: one card at a time all the same.
    let dir = shop(rig, json!({ "workers": 2, "stages": stages })).await;
    let first = add(rig, "Lay the floor", &[]).await;
    let second = add(rig, "Put the shelves up", &[]).await;
    move_card(rig, &first, "ready", None).await;
    moved(rig, &first, "review", 0).await;
    rig.event("agent.turn.started", |event| event.data["slot"] == "shop-reviewer-1").await;

    move_card(rig, &second, "ready", None).await;
    let waiting = rig.event("card.waiting", |_| true).await;
    let behind =
        format!("no work is being started here for now: {first}, worked from here, is not merged yet");
    assert_eq!(waiting.data, json!({ "card": second, "project": "shop", "what": behind }));
    assert_eq!(card(rig, &second).await["status"], "ready");
    assert!(!dir.join(".lanes").join(&second).exists());
    (dir, hold, first, second)
}

/// The first let go, passed and merged: the second is worked from what it
/// left, says it waits for nothing, and is merged after it.
async fn worked_after(rig: &Rig, dir: &Path, hold: &Path, first: &str, second: &str) {
    crate::let_go("a reviewer to be waiting", hold).await;
    moved(rig, first, "done", 0).await;
    moved(rig, second, "review", 0).await;
    let lane = dir.join(".lanes").join(second).join("CHANGES.md");
    assert_eq!(std::fs::read_to_string(lane).unwrap(), "changed\nchanged\n");
    assert_eq!(card(rig, second).await["more"]["waiting"], "");
    crate::let_go("a reviewer to be waiting", hold).await;
    moved(rig, second, "done", 0).await;
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "changed\nchanged\n");
}

#[tokio::test]
async fn a_stage_that_takes_the_next_card_once_the_last_is_merged_works_it_from_the_last_ones_work() {
    let (rig, at) = rig().await;
    let (dir, hold, first, second) = one_behind(&rig, &at).await;
    worked_after(&rig, &dir, &hold, &first, &second).await;
    // One worker was given both, one after the other; the other none.
    assert_eq!(given(&rig, "shop-worker-1").await.len() + given(&rig, "shop-worker-2").await.len(), 2);
}

#[tokio::test]
async fn a_card_waiting_behind_another_still_waits_after_the_daemon_is_started_again() {
    let (mut rig, at) = rig().await;
    let (dir, hold, first, second) = one_behind(&rig, &at).await;
    rig.restart().await;
    // The review of the first is begun again; the second has not been taken meanwhile.
    rig.event("agent.turn.started", |event| {
        event.data["slot"] == "shop-reviewer-1" && event.data["again"] == 1
    })
    .await;
    assert_eq!(card(&rig, &second).await["status"], "ready");
    worked_after(&rig, &dir, &hold, &first, &second).await;
}

#[tokio::test]
async fn a_card_waiting_behind_another_is_taken_once_the_row_says_the_next_is_taken_when_the_last_is_handed_on()
 {
    let (rig, at) = rig().await;
    let (dir, hold, first, second) = one_behind(&rig, &at).await;
    change(&rig, "stages", "work", json!({ "next_after": "handed" })).await;
    moved(&rig, &second, "review", 0).await;
    // Taken while the first was still in review, it has nothing of its work.
    let lane = dir.join(".lanes").join(&second).join("CHANGES.md");
    assert_eq!(std::fs::read_to_string(lane).unwrap(), "changed\n");
    assert_eq!(card(&rig, &first).await["status"], "review");
    assert_eq!(card(&rig, &second).await["more"]["waiting"], "");
    crate::let_go("a reviewer to be waiting", &hold).await;
}
