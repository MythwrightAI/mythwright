//! A card sent back to another column than the one cards are in while
//! they are in hand.
//!
//! Where a card that did not pass goes is a row's to say (`fail` of the
//! stage that reviews), and the row may say the column the stage that
//! works takes its cards from. Sent there, by the fleet's own reviewers
//! or by a person, the card is that stage's again: it is taken up as a
//! card that was sent back, by the agent that had it, told what was
//! said; and it has gone round once more, which its row counts, so that
//! what caps a loop between agents reads the same number whichever
//! column a card goes back to.

use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, card, given, rig, says, shop};
use crate::lanes::{history, move_card, moved};

/// The fleet's two stages, where the one that reviews sends a card that
/// did not pass back to the column the other works from.
fn back_to_ready() -> Value {
    json!([
        { "title": "work", "column": "ready", "does": "work", "role": "worker", "pass": "review" },
        {
            "title": "review", "column": "review", "does": "review", "role": "reviewer",
            "pass": "done", "fail": "ready", "rounds": 2, "merge": true,
        },
    ])
}

/// Waits until a card's row has counted its being sent back, after
/// `after` in the journal: how often it then says.
async fn counted(rig: &Rig, id: &str, after: u64) -> Value {
    let its = id.to_string();
    rig.event("board.item.updated", move |event| {
        event.seq > after && event.data["id"] == its.as_str() && event.data["changed"] == json!(["rounds"])
    })
    .await;
    card(rig, id).await["more"]["rounds"].clone()
}

#[tokio::test]
async fn a_card_its_reviewers_send_back_to_the_column_it_is_worked_from_is_counted_and_given_to_its_worker_again()
 {
    let (rig, at) = rig().await;
    shop(&rig, json!({ "stages": back_to_ready() })).await;
    says(&at, "shop-reviewer-1", "The total is wrong.", "send_back");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;

    // Worked, looked at and sent back: to where it is worked from, as
    // the row says, and not to where cards are in hand.
    let reviewed = moved(&rig, &id, "review", 0).await;
    let back = moved(&rig, &id, "ready", reviewed).await;
    assert_eq!(counted(&rig, &id, reviewed).await, 1);

    // It goes round as often as the row allows, counted each time, and
    // is then left in review for a person.
    let reviewed = moved(&rig, &id, "review", back).await;
    let back = moved(&rig, &id, "ready", reviewed).await;
    assert_eq!(counted(&rig, &id, reviewed).await, 2);
    moved(&rig, &id, "review", back).await;
    let its = id.clone();
    let left = rig.event("card.left", move |event| event.data["card"] == its.as_str()).await;
    assert_eq!(left.data["rounds"], 2);

    // The worker did it, and was given it again each time it came back,
    // told what whoever looked had said.
    let worked = given(&rig, "shop-worker-1").await;
    assert_eq!(worked.len(), 3, "{worked:?}");
    for again in &worked[1..] {
        assert!(
            again.starts_with("This card was sent back, for you to take up again")
                && again.contains("shop-reviewer-1: The total is wrong."),
            "{again}"
        );
    }
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 2, "it was looked at once it was left");
    let line = "conductor moved it from review to ready, saying: shop-reviewer-1: The total is wrong.";
    let said = history(&rig, &id, line).await;
    assert_eq!(said.iter().filter(|one| one.as_str() == line).count(), 2, "{said:?}");
    let is = card(&rig, &id).await;
    assert_eq!((&is["status"], &is["more"]["rounds"]), (&json!("review"), &json!(2)));
}

#[tokio::test]
async fn a_card_a_person_sends_back_where_its_stage_says_is_counted_and_given_to_its_worker_again() {
    let (rig, _) = rig().await;
    // Nobody reviews: a worked card waits in review for a person.
    shop(&rig, json!({ "reviewers": 0, "stages": back_to_ready() })).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;

    // Somebody sends it back, as the client does: to where the row of
    // the stage that has it says, with what is wrong.
    move_card(&rig, &id, "ready", Some("The total is wrong when the cart is empty.")).await;
    assert_eq!(counted(&rig, &id, reviewed).await, 1);
    moved(&rig, &id, "review", reviewed).await;
    let worked = given(&rig, "shop-worker-1").await;
    assert_eq!(worked.len(), 2, "{worked:?}");
    assert!(
        worked[1].starts_with("This card was sent back, for you to take up again")
            && worked[1].contains("The total is wrong when the cart is empty."),
        "{}",
        worked[1]
    );
}
