//! What waits for a person is said, and told to whoever the project's
//! `tells` says, wherever it waits: a card in review with nobody there to
//! look at it (`card.waits`), and a card whose agent could not do what it
//! needed (`blocked`).

use std::path::PathBuf;

use serde_json::json;

use crate::Rig;
use crate::fleets::{add, change, shop};
use crate::kept::person;
use crate::lanes::{history, move_card, moved};
use crate::tells::{heard, lines};

/// What a card that is to be blocked is called: its agent cannot do it.
const REFUSED: &str = "Run the check";

/// What that agent says it could not do.
const COULD_NOT: &str = "I could not run ./check: running it was refused me.";

/// A node whose fleet has stand-ins of its own: `writer` says it could
/// not do a card called `REFUSED` and ends `blocked`, and does any other
/// card; `critic` approves.
async fn rig() -> (Rig, PathBuf) {
    let mut at = None;
    let rig = Rig::with("api-waited", |config, dir| {
        let writes = [
            "read",
            "where",
            REFUSED,
            "(",
            "say",
            COULD_NOT,
            "say",
            "DECISION: blocked",
            ")",
            "append",
            "CHANGES.md",
            "changed",
            "say",
            "I changed a file.",
            "say",
            "DECISION: done",
        ];
        let looks = ["swallow", "say", "It does what the card asks.", "say", "DECISION: approve"];
        config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
        config.adapters.push(("critic".into(), crate::standin::argv(&looks)));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap())
}

/// How often `topic` was said of `card`.
async fn said_of(rig: &Rig, topic: &str, card: &str) -> usize {
    let all = journal::EventQuery { topics: topic.into(), limit: 1_000, ..Default::default() };
    rig.said(all).await.iter().filter(|event| event.data["card"] == card).count()
}

#[tokio::test]
async fn a_card_in_review_with_nobody_to_look_at_it_waits_for_a_person_who_is_told_once_and_is_reviewed_once_somebody_is_there()
 {
    let (owned, _) = rig().await;
    let rig = &owned;
    shop(rig, json!({ "reviewers": 0 })).await;

    // 1: worked, it comes to review, where the role has nobody. That it
    // waits for a person is said once, a line of its history, and the
    // owner, who applied the fleet, is told in those words.
    let id = add(rig, "Add a shelf", &[]).await;
    move_card(rig, &id, "ready", None).await;
    moved(rig, &id, "review", 0).await;
    let its = id.clone();
    let waits = rig.event("card.waits", move |event| event.data["card"] == its.as_str()).await;
    let sentence = format!(
        "{id} is in review, where nobody in the fleet has the role reviewer: it waits for a person to review it."
    );
    assert_eq!(
        (&waits.data["kind"], &waits.data["stage"], &waits.data["project"], &waits.data["why"]),
        (&json!("review"), &json!("review"), &json!("shop"), &json!(sentence)),
        "{waits:?}"
    );
    heard(rig, "on.waited", waits.seq).await;
    assert_eq!(lines(rig, &rig.token, "review").await, [(sentence.clone(), 1)]);
    history(rig, &id, &sentence).await;

    // 2: a second card that comes there is said to wait too, and the
    // first is not said again for it.
    let other = add(rig, "Add a lamp", &[]).await;
    move_card(rig, &other, "ready", None).await;
    let theirs = other.clone();
    let second = rig.event("card.waits", move |event| event.data["card"] == theirs.as_str()).await;
    heard(rig, "on.waited", second.seq).await;
    assert_eq!(said_of(rig, "card.waits", &id).await, 1);
    assert_eq!(lines(rig, &rig.token, "review").await.len(), 2);

    // 3: somebody is given the role: what waited is reviewed, approved
    // and goes on, and nothing more is said of it waiting.
    let before = rig.mark();
    change(rig, "agents", "reviewer", json!({ "count": 1 })).await;
    moved(rig, &id, "done", before).await;
    moved(rig, &other, "done", before).await;
    assert_eq!(said_of(rig, "card.waits", &id).await, 1);
    assert_eq!(said_of(rig, "card.waits", &other).await, 1);
}

#[tokio::test]
async fn a_card_whose_agent_could_not_do_what_it_needed_is_left_for_a_person_who_is_told_why_and_a_row_that_tells_of_cards_left_hears_it()
 {
    let (owned, _) = rig().await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(rig, "Grace").await;
    shop(rig, json!({})).await;
    rig.event("record.created", |event| event.data["collection"] == "tells").await;
    let lead = json!({ "fields": { "title": "lead", "whom": "@grace", "of": ["left"] } });
    rig.send("/api/collections/shop/tells/records", lead).await;

    // Its agent says what it could not do and ends `blocked`: the card is
    // left in doing, which is said with which way, and with what it said.
    let id = add(rig, REFUSED, &[]).await;
    move_card(rig, &id, "ready", None).await;
    let its = id.clone();
    let left = rig.event("card.left", move |event| event.data["card"] == its.as_str()).await;
    assert_eq!(left.data["as"], "blocked", "{left:?}");
    let why = left.data["why"].as_str().unwrap().to_string();
    assert!(why.contains(COULD_NOT), "{why}");
    heard(rig, "on.left", left.seq).await;
    let sentence = format!("{id}: its agent could not do what it needed, and it is left for a person: {why}");
    assert_eq!(lines(rig, &rig.token, "blocked").await, [(sentence.clone(), 1)]);
    assert_eq!(lines(rig, &grace.token, "blocked").await, [(sentence, 1)]);
    assert_eq!(crate::fleets::card(rig, &id).await["status"], "doing");
    history(rig, &id, &format!("it was left for a person: {why}")).await;

    // A card it can do is worked as ever: `done`, and nothing is left.
    let before = rig.mark();
    let shelf = add(rig, "Add a shelf", &[]).await;
    move_card(rig, &shelf, "ready", None).await;
    moved(rig, &shelf, "done", before).await;
    assert_eq!(said_of(rig, "card.left", &shelf).await, 0);
}

#[tokio::test]
async fn a_card_whose_review_is_stopped_with_nothing_said_waits_for_a_person_who_is_told_so() {
    // The fleet's own stand-ins: its reviewer waits at a hold, so that a
    // review is in session for as long as the test wants.
    let (owned, at) = crate::fleets::rig().await;
    let rig = &owned;
    let hold = at.join("hold");
    crate::hold(&hold);
    shop(rig, json!({})).await;

    let id = add(rig, "Add a shelf", &[]).await;
    let before = rig.mark();
    move_card(rig, &id, "ready", None).await;
    moved(rig, &id, "review", before).await;
    rig.event("agent.turn.started", move |event| {
        event.seq > before && event.data["slot"] == "shop-reviewer-1"
    })
    .await;
    let live = rig.read("/api/instances?project=shop&live=true").await;
    let review = live["instances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|one| one["blueprint"] == "std/review")
        .unwrap_or_else(|| panic!("a review in session: {live}"))["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Stopped by somebody, the review says nothing of how it came out:
    // its card is said to wait for a person, and the owner is told.
    rig.send(&format!("/api/instances/{review}/stop"), json!({})).await;
    let its = id.clone();
    let waits = rig.event("card.waits", move |event| event.data["card"] == its.as_str()).await;
    let sentence = format!(
        "{id} is in review, and its review ended with nothing said of how it came out (it was stopped, or it \
         failed): it waits for a person to review it."
    );
    assert_eq!((&waits.data["kind"], &waits.data["why"]), (&json!("review"), &json!(sentence)), "{waits:?}");
    heard(rig, "on.waited", waits.seq).await;
    assert_eq!(lines(rig, &rig.token, "review").await, [(sentence.clone(), 1)]);
    history(rig, &id, &sentence).await;
    assert_eq!(crate::fleets::card(rig, &id).await["status"], "review");
}
