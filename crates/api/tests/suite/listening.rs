//! A place asked for before its pool was listening is given all the same.
//!
//! A pool hears what asks it only from when it is listening, and a stage
//! and its pool are started side by side: a card that was in the column
//! already asked nobody, and stayed there for good. Here the order is the
//! test's own and not how fast anything was: the stage's pool is taken
//! away, a card comes to the column and its place is asked of nobody, and
//! then a pool of that name is there and says that it is listening.

use serde_json::json;

use crate::Rig;
use crate::fleets::{add, given, rig, shop};
use crate::lanes::{move_card, moved};

/// Takes away the pool that answers to `name`, as it is while a fleet is
/// coming up and its stage is there first: nothing hears what asks it.
async fn no_pool(rig: &Rig, name: &str) {
    let all = rig.read("/api/instances?live=true&project=shop").await;
    for one in all["instances"].as_array().unwrap().iter().filter(|one| one["blueprint"] == "std/pool") {
        let id = one["id"].as_str().unwrap();
        if rig.read(&format!("/api/instances/{id}")).await["instance"]["params"]["name"] == name {
            rig.send(&format!("/api/instances/{id}/stop"), json!({})).await;
            return;
        }
    }
    panic!("no pool answers to {name}");
}

/// A pool that answers to `name` and gives one place: once it has said
/// that it is listening, which is where the journal had got to then.
async fn a_pool(rig: &Rig, name: &'static str) -> u64 {
    let pool = json!({
        "blueprint": "std/pool", "project": "shop", "wait": true,
        "params": { "name": name, "project": "shop", "limit": 1 },
    });
    let before = rig.mark();
    let applied = rig.send("/api/apply", pool).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let said =
        rig.event("pool.listening", move |event| event.seq > before && event.data["pool"] == name).await;
    assert_eq!(said.data, json!({ "pool": name, "held": [], "waiting": [] }));
    said.seq
}

#[tokio::test]
async fn a_card_whose_place_was_asked_for_before_its_pool_was_listening_is_worked_when_the_pool_says_it_is() {
    let (rig, _) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    no_pool(&rig, "shop/ready").await;

    // Made ready, the card asks for its turn, and nobody hears it.
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let card = id.clone();
    let unheard = rig.event("pool.asked", move |event| event.data["as"] == card.as_str()).await.seq;
    assert_eq!(rig.count_of("pool.granted").await, 0);

    // A pool is there and says that it is listening: the stage says the
    // card is due again, it asks again, and it is worked.
    let listening = a_pool(&rig, "shop/ready").await;
    assert!(listening > unheard);
    moved(&rig, &id, "review", listening).await;
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1, "and its agent was asked once");
}

#[tokio::test]
async fn a_review_that_asked_for_its_turn_before_its_pool_was_listening_is_begun_again_when_the_pool_says_it_is()
 {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    no_pool(&rig, "shop/review").await;

    // Worked, the card comes to review. A review of it is begun and asks
    // for its turn, and nobody hears it.
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;
    let unheard = rig.event("pool.asked", |event| event.data["pool"] == "shop/review").await;
    let first = unheard.source.instance.clone().expect("a review asks as itself");
    assert_eq!(rig.count_of("agent.turn.queued").await, 1, "nobody has been asked to look");

    // A pool is there, says that it is listening, and has not got that
    // review: it ends, and the review begun in its place asks a pool that
    // hears. The card is looked at once, passes and is merged.
    let listening = a_pool(&rig, "shop/review").await;
    let again = rig
        .event("pool.asked", move |event| event.seq > listening && event.data["pool"] == "shop/review")
        .await;
    assert_ne!(again.source.instance.as_deref(), Some(first.as_str()), "another review, and not the first");
    moved(&rig, &id, "done", listening).await;
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1, "and whoever looks was asked once");
    let begun = rig.read("/api/instances?project=shop").await;
    let reviews: Vec<&str> = begun["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|one| one["blueprint"] == "std/review")
        .map(|one| one["status"].as_str().unwrap())
        .collect();
    assert_eq!(reviews.len(), 2, "{begun}");
}
