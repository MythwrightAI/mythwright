//! A card that somebody moves out of doing while its agent is at it.
//!
//! Its round is stopped at once, as the stage's row says (`moved_away`,
//! `stop` the usual): its agent's turn is called off, nothing it did is
//! counted as failed, what it changed is left in the card's worktree,
//! committed by nobody and thrown away by nobody, and the card keeps
//! that it was taken out of its agent's hands. Brought back to where it
//! waits, it is taken up where it was left, its agent told so and what
//! was said as it came back, and not given it again as new; moved to
//! done, it is done. A row that says `finish` lets the round go on to
//! its end as it always did: it sees the card has moved and hands
//! nothing on.

use std::path::Path;

use serde_json::json;

use crate::fleets::{add, card, change, given, shop};
use crate::lanes::{move_card, moved};
use crate::observed::asked;
use crate::reapplied::{REVIEWER, WORKER, began, let_go, stand_ins};
use crate::{Listening, Rig};

/// What was said on these topics from `from` on.
async fn since(rig: &Rig, from: u64, topics: &str) -> Vec<kernel::Event> {
    let query =
        journal::EventQuery { topics: topics.into(), after: from, limit: 10_000, ..Default::default() };
    rig.said(query).await
}

/// How many of the worker's turns were called off from `from` on, and
/// how many ended otherwise without going well.
async fn turns_ended(rig: &Rig, from: u64) -> (usize, usize) {
    let ended = since(rig, from, "agent.turn.finished").await;
    let worker = ended.iter().filter(|event| event.data["slot"] == WORKER);
    let (off, failed): (Vec<_>, Vec<_>) =
        worker.filter(|event| event.data["ok"] == false).partition(|event| event.data["called_off"] == true);
    (off.len(), failed.len())
}

/// The stage's run that asked for the card's round is over.
async fn round_over(rig: &Rig, after: u64) {
    rig.event("run.finished", move |event| {
        event.seq > after && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.go"
    })
    .await;
}

/// The card's worktree, with something its agent changed and did not commit.
fn half_done(dir: &Path, id: &str) -> std::path::PathBuf {
    let place = dir.join(".lanes").join(id);
    std::fs::write(place.join("half.md"), "half done\n").unwrap();
    place
}

#[tokio::test]
async fn a_card_moved_out_of_doing_while_its_agent_is_at_it_has_its_round_stopped_and_brought_back_is_taken_up_where_it_was_left()
 {
    let (rig, at) = stand_ins("api-out-of-hand", None).await;
    let mut there = Listening::at(&at.join("began"));
    for who in [WORKER, REVIEWER] {
        crate::hold(&at.join(format!("hold-{who}")));
    }
    let dir = shop(&rig, json!({})).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    began(&mut there, WORKER).await;
    let place = half_done(&dir, &id);
    let from = rig.mark();

    // Moved back to the drafts, further than where it waited: its round is stopped at
    // once, and its agent's turn with it, called off and not failed.
    move_card(&rig, &id, "draft", Some("Not this week.")).await;
    round_over(&rig, from).await;
    assert_eq!(turns_ended(&rig, from).await, (1, 0));
    let away = card(&rig, &id).await;
    assert_eq!(away["status"], "draft");
    assert_eq!(away["more"]["again"], json!(["away", "draft"]), "{away}");
    // What its agent changed is where it left it, and nothing was committed.
    assert_eq!(std::fs::read_to_string(place.join("half.md")).unwrap(), "half done\n");
    assert_eq!(crate::lanes::git(&dir, &["log", "--format=%s", &format!("lane/{id}")]), "first");
    assert_eq!(asked(&rig, &[WORKER]).await[WORKER], 1, "nobody is asked anything more");

    // Brought back to where it waits: taken up where it was left, told
    // so and what was said, and not given to its agent as new.
    let back = rig.mark();
    move_card(&rig, &id, "ready", Some("Go on with it.")).await;
    began(&mut there, WORKER).await;
    moved(&rig, &id, "doing", back).await;
    let told = given(&rig, WORKER).await.last().cloned().unwrap();
    assert!(told.contains("Nobody sent it back: take it up where you left it"), "{told}");
    assert!(told.contains("Go on with it."), "{told}");
    assert!(!told.contains("This card was sent back"), "{told}");
    assert_eq!(std::fs::read_to_string(place.join("half.md")).unwrap(), "half done\n");

    // Moved straight back to where it waits while its agent is at it
    // again: stopped, and taken up where it was left once more, and not
    // worked a second time as new.
    let again = rig.mark();
    move_card(&rig, &id, "ready", None).await;
    began(&mut there, WORKER).await;
    moved(&rig, &id, "doing", again).await;
    let told = given(&rig, WORKER).await.last().cloned().unwrap();
    assert!(told.contains("Nobody sent it back: take it up where you left it"), "{told}");
    assert_eq!(asked(&rig, &[WORKER]).await[WORKER], 3);

    // And moved to done while it is at it: stopped, and done.
    let done = rig.mark();
    move_card(&rig, &id, "done", None).await;
    round_over(&rig, done).await;
    assert_eq!(card(&rig, &id).await["status"], "done");

    // Three turns called off, and none of it said to have failed or to
    // have been left for a person.
    assert_eq!(turns_ended(&rig, from).await, (3, 0));
    let failed = since(&rig, from, "run.failed,instance.failed,card.left").await;
    assert!(
        failed.is_empty(),
        "nothing failed: {:?}",
        failed.iter().map(|e| (&e.topic, &e.data)).collect::<Vec<_>>()
    );
    crate::simulation::rested(&rig).await;
}

#[tokio::test]
async fn a_stage_whose_row_says_finish_lets_the_round_of_a_card_moved_away_go_on_to_its_end() {
    let (rig, at) = stand_ins("api-out-of-hand-finish", None).await;
    let mut there = Listening::at(&at.join("began"));
    for who in [WORKER, REVIEWER] {
        crate::hold(&at.join(format!("hold-{who}")));
    }
    shop(&rig, json!({})).await;
    change(&rig, "stages", "work", json!({ "moved_away": "finish" })).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    began(&mut there, WORKER).await;
    let from = rig.mark();

    move_card(&rig, &id, "draft", None).await;
    // Its agent goes on: let go, it finishes, and the round hands nothing on.
    let_go(&at, WORKER).await;
    round_over(&rig, from).await;
    assert_eq!(turns_ended(&rig, from).await, (0, 0), "nothing is called off");
    let away = card(&rig, &id).await;
    assert_eq!(away["status"], "draft", "{away}");
    assert_eq!(away["more"]["again"][0], "heard", "{away}");
    crate::simulation::rested(&rig).await;
}
