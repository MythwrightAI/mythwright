//! How a card is finished where it is worked, as its stage's row says:
//! each step its agent sees to before it says it is done
//! (`finish`), what is checked later by somebody else (`later`), and how
//! long one turn may take (`lasts`). Played against the fleet that
//! ships with a stand-in that ends each way: nearly done and given again
//! once, none of it done and left, and cut by its time and asked once.

use std::sync::Arc;
use std::time::Duration;

use kernel::ManualClock;
use serde_json::json;

use crate::Rig;
use crate::fleets::{add, change, given, shop};
use crate::lanes::{WORKER, move_card, moved};

/// The steps a card is finished by here, and what is checked later.
fn finishing() -> serde_json::Value {
    json!({
        "finish": ["its tests pass", "the docs it changes say what is new"],
        "later": ["the whole gate, run when the work lands"],
    })
}

/// A node whose worker says, of a card whose time ran out, that the first
/// step is done and the second waits on what is checked later; of the
/// second time it is given a card, that every step is done; of a card
/// that is broken, that nothing is; of a slow card, nothing, waiting
/// until it is stopped; and of any other, that the docs are not written.
/// Its reviewer approves.
async fn rig(clock: Option<ManualClock>) -> Rig {
    Rig::owned(
        Rig::with("api-finishing", |config, _| {
            let writes = [
                "read",
                "where",
                "ran out",
                "(",
                "say",
                "The cart is made; the gate is not mine to run.",
                "say",
                "STEP 1: done",
                "say",
                "STEP 2: later",
                "say",
                "DECISION: done",
                ")",
                "where",
                "Take it up again",
                "(",
                "say",
                "I wrote the docs.",
                "say",
                "STEP 1: done",
                "say",
                "STEP 2: done",
                "say",
                "DECISION: done",
                ")",
                "where",
                "broken",
                "(",
                "say",
                "STEP 1: not done: the tests do not build",
                "say",
                "STEP 2: not done",
                "say",
                "DECISION: done",
                ")",
                "where",
                "slow",
                "(",
                "idle",
                ")",
                "say",
                "I made the cart.",
                "say",
                "STEP 1: done",
                "say",
                "STEP 2: not done: no time",
                "say",
                "DECISION: done",
            ];
            let looks = ["swallow", "say", "It does what the card asks.", "say", "DECISION: approve"];
            config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
            config.adapters.push(("critic".into(), crate::standin::argv(&looks)));
            if let Some(clock) = clock {
                config.clock = Some(Arc::new(clock));
            }
        })
        .await,
    )
    .await
}

#[tokio::test]
async fn a_card_its_agent_says_is_nearly_finished_is_given_again_once_with_what_is_left_and_one_with_none_of_it_done_is_left()
 {
    let rig = &rig(None).await;
    shop(rig, json!({})).await;
    change(rig, "stages", "work", finishing()).await;

    // Nearly done: the docs are not written. It is given again once, told
    // what is left, in the same conversation; done then, it goes on.
    let cart = add(rig, "Add a cart; the gate passes", &[]).await;
    move_card(rig, &cart, "ready", None).await;
    moved(rig, &cart, "done", 0).await;
    let asked = given(rig, WORKER).await;
    let first = &asked[0];
    assert!(first.starts_with(&format!("Before you say {cart} is done, see to each of these")), "{first}");
    assert!(first.contains("1. its tests pass\n2. the docs it changes say what is new\n"), "{first}");
    assert!(
        first.contains("never yours to wait on or to run: the whole gate, run when the work lands"),
        "{first}"
    );
    assert!(first.contains(&format!("What {cart} says asks for the whole gate")), "{first}");
    let again = &asked[1];
    assert!(again.starts_with(&format!("You said {cart} was done")), "{again}");
    assert!(
        again.contains("what is left: 2. the docs it changes say what is new (not done: no time)"),
        "{again}"
    );
    assert_eq!(asked.len(), 2, "{asked:?}");

    // None of it done: left for a person, saying what is not done, and
    // its agent is not asked again.
    let broken = add(rig, "Fix the broken till", &[]).await;
    move_card(rig, &broken, "ready", None).await;
    let its = broken.clone();
    let left = rig.event("card.left", move |event| event.data["card"] == its.as_str()).await;
    assert_eq!(left.data["as"], "unfinished", "{left:?}");
    let why = left.data["why"].as_str().unwrap();
    assert!(
        why.starts_with("none of how it is finished is done: 1. its tests pass (not done: the tests do"),
        "{why}"
    );
    assert_eq!(given(rig, WORKER).await.len(), 3);
}

#[tokio::test]
async fn a_turn_past_its_time_has_its_program_stopped_and_its_agent_asked_once_what_is_done_which_is_what_it_comes_to()
 {
    let clock = ManualClock::new();
    let rig = &rig(Some(clock.clone())).await;
    shop(rig, json!({})).await;
    let mut row = finishing();
    row["lasts"] = json!("20m");
    change(rig, "stages", "work", row).await;

    let slow = add(rig, "Add a slow cart", &[]).await;
    move_card(rig, &slow, "ready", None).await;
    let thread = format!("lane:{slow}");
    rig.event("agent.turn.started", move |event| event.data["thread"] == thread.as_str()).await;
    clock.advance(Duration::from_secs(21 * 60));

    // Its program was stopped, and asked once what it had done it said
    // the first step is done and the second waits only on what is checked
    // later: it goes on, and is reviewed as any card is.
    moved(rig, &slow, "done", 0).await;
    let asked = given(rig, WORKER).await;
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert!(
        asked[1].starts_with(&format!(
            "The time you were given for {slow} ran out, and your program was stopped"
        )),
        "{}",
        asked[1]
    );
    // How long the turn took, and that it was cut, is measured.
    let measured = rig.read("/api/metrics?name=turn.ended&labels=time:cut").await;
    let total = &measured["series"][0]["total"];
    assert_eq!((&total["count"], &total["max"]), (&json!(1), &json!(21.0 * 60_000.0)), "{measured}");
}
