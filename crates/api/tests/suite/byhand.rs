//! A card somebody works themselves, outside the fleet.
//!
//! The columns a fleet's stages watch are the fleet's: a card that comes
//! to one is due what its stage does. The column of work in hand is the
//! exception a person needs, so that a board reads as a board does: a
//! card put straight there by somebody was never taken by the stage that
//! works, and is nobody's turn. Only a card that stage took, come back to
//! work in hand from somewhere else, is given to its agent again.

use serde_json::json;

use crate::fleets::{add, card, shop};
use crate::lanes::{move_card, moved};
use crate::rows::{alive, rig_whose_worker, said_of};

#[tokio::test]
async fn a_card_somebody_puts_straight_into_work_in_hand_is_theirs_and_no_agent_is_asked() {
    // A worker that leaves a mark if it is ever asked anything.
    let (rig, at) =
        rig_whose_worker(&["swallow", "append", "$DIR/asked", "asked", "say", "I did nothing."]).await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let id = add(&rig, "Done by hand", &[]).await;
    let before = alive(&rig).await;

    let from = rig.mark();
    move_card(&rig, &id, "doing", None).await;
    let at_move = moved(&rig, &id, "doing", from).await;
    // The stage that works has heard it: what it does with a card that
    // comes to work in hand from somewhere else has run, and is over.
    rig.event("run.finished", move |event| {
        event.seq > at_move && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.back"
    })
    .await;

    // It asked for no turn for the card, so nothing comes after.
    assert_eq!(said_of(&rig, "stage.again", "card", &id).await, 0, "the stage took it for its own");
    assert_eq!(said_of(&rig, "pool.asked", "as", &id).await, 0, "a turn was asked for it");
    assert!(!at.join("asked").exists(), "an agent was asked something");
    assert_eq!(alive(&rig).await, before, "something was started for it");
    let is = card(&rig, &id).await;
    assert_eq!(is["status"], "doing");
    assert_eq!(is["assignee"].as_str().unwrap_or_default(), "", "it was given to somebody");
}
