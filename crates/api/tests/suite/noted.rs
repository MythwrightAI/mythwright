//! What a person says as they put a card where it is worked, and who is
//! told: whoever works it, with the card.

use serde_json::json;

use crate::Rig;
use crate::fleets::{add, shop};
use crate::lanes::{git, move_card, moved};

#[tokio::test]
async fn what_is_said_as_a_card_is_put_where_it_is_worked_is_told_to_whoever_works_it() {
    // Whoever works writes down what it was asked, where it works.
    let rig = Rig::with("api-noted", move |config, _| {
        let reads = ["read", "write", "ASKED.md", "%:read%", "say", "I read it."];
        config.adapters.push(("writer".into(), crate::standin::argv(&reads)));
        config
            .adapters
            .push(("critic".into(), crate::standin::argv(&["swallow", "say", "DECISION: approve"])));
    });
    let rig = Rig::owned(rig.await).await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    let asked = |id: &str| git(&dir, &["show", &format!("lane/{id}:ASKED.md")]);

    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", Some("Mind the totals.")).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    let first = asked(&id);
    assert!(
        first.contains("Please take this card. What was said as it was put where you work:")
            && first.contains("Mind the totals."),
        "{first}"
    );

    // The card has come to rest in review: the stage's run for it is over.
    // Sent back sooner than that, on a slow machine, it was once given a
    // round with nothing said and only then the round that says why: a
    // turn that had come for it twice found it in doing before what was
    // said as it was sent back had been written on it. That window is the
    // stage's to close and is not what this test is of.
    rig.event("run.finished", move |event| event.seq > reviewed && event.data["flow"] == "on.go").await;
    let reviewed = rig.mark();
    // Sent back with something said, it is told that as one sent back is,
    // and not what was said when it was first put there.
    move_card(&rig, &id, "doing", Some("The total is wrong.")).await;
    moved(&rig, &id, "review", reviewed).await;
    let again = asked(&id);
    assert!(again.contains("This card was sent back") && again.contains("The total is wrong."), "{again}");
    assert!(!again.contains("Mind the totals."), "{again}");

    // And one put there with nothing said is asked as it always was.
    let plain = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &plain, "ready", None).await;
    moved(&rig, &plain, "review", 0).await;
    let plainly = asked(&plain);
    assert!(
        plainly.contains("\nPlease take this card.\n") && !plainly.contains("What was said as"),
        "{plainly}"
    );
}
