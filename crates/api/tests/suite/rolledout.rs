//! A gradual rollout, over HTTP and with stand-in agents: a new setup (an
//! agent of a role of its own) given a share of a board's cards beside
//! the usual one by a comparison that shares them out, which grows by a
//! rule while its cards are sent back from review no more often than the
//! usual one's, and is held where they are sent back more often. The
//! factory counts and says when the share changes and why; the rule is
//! whoever wrote it's.

use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, given, rig_told, shop};
use crate::lanes::{move_card, moved};

/// A node whose `sloppy` writer leaves a file the reviewer (`judging`)
/// sends back wherever it sees it, and passes everything else.
async fn node() -> Rig {
    rig_told(|config| {
        let sloppy = ["swallow", "append", "SLOPPY.md", "sloppy", "say", "I changed a file."];
        config.adapters.push(("sloppy".into(), crate::standin::argv(&sloppy)));
        let judging = [
            "read",
            "where",
            "SLOPPY",
            "(",
            "say",
            "It leaves a sloppy file.",
            "say",
            "DECISION: send_back",
            ")",
            "say",
            "It does what the card asks.",
            "say",
            "DECISION: approve",
        ];
        config.adapters.push(("judging".into(), crate::standin::argv(&judging)));
    })
    .await
    .0
}

/// The usual worker, a new setup beside it (`slate`, run as `runs_on`), and
/// one who reviews.
fn agents(runs_on: &str) -> Value {
    json!([
        { "title": "worker", "role": "worker", "count": 1, "adapter": "writer" },
        { "title": "slate", "role": "slate", "count": 1, "adapter": runs_on },
        { "title": "reviewer", "role": "reviewer", "count": 1, "adapter": "judging", "permission": "read" },
    ])
}

/// `mw compare start rollout --project shop --of std/work --per item --way usual --way slate
///     --with slate.agent=shop-slate-1 --grows slate --against usual --holds verdict=failed --over 1
///     --step 25 --most 75`, with the usual counts of a board's cards.
async fn rollout(rig: &Rig) -> Value {
    rollout_falling(rig, None).await
}

/// The same, going back a step where it does not hold, to `least`, where one is said (`--least`).
async fn rollout_falling(rig: &Rig, least: Option<u64>) -> Value {
    let mut asked = json!({
        "name": "rollout", "project": "shop", "of": "std/work", "per": "item", "counts": "cards",
        "ways": [{ "name": "usual" }, { "name": "slate", "with": { "agent": "shop-slate-1" } }],
        "rule": {
            "grows": "slate", "against": "usual", "count": "verdict", "came": "failed",
            "over": 1, "step": 25, "most": 75,
        },
    });
    if let Some(least) = least {
        asked["rule"]["least"] = json!(least);
    }
    rig.send("/api/comparisons", asked).await
}

/// A card added, made ready and worked until it is done: its id.
async fn done(rig: &Rig, title: &str) -> String {
    let id = add(rig, title, &[]).await;
    move_card(rig, &id, "ready", None).await;
    moved(rig, &id, "done", 0).await;
    id
}

/// A card added and made ready, until it is done or left for a person.
async fn through(rig: &Rig, title: &str) {
    let id = add(rig, title, &[]).await;
    move_card(rig, &id, "ready", None).await;
    rig.event("card.left,board.item.moved", move |event| {
        let of = event.data["card"] == id.as_str() || event.data["id"] == id.as_str();
        of && (event.topic == "card.left" || event.data["to"] == "done")
    })
    .await;
}

/// What `compare show` lays out of the rollout's shares.
async fn rolled(rig: &Rig) -> Value {
    rig.read("/api/comparisons/rollout").await["beside"]["rolled"].clone()
}

/// Each way's share as it now is.
async fn shares(rig: &Rig) -> Vec<(String, u64)> {
    let one = rig.read("/api/comparisons/rollout").await;
    one["comparison"]["ways"]
        .as_array()
        .unwrap()
        .iter()
        .map(|way| (way["name"].as_str().unwrap().to_string(), way["share"].as_u64().unwrap()))
        .collect()
}

fn of(pairs: &[(&str, u64)]) -> Vec<(String, u64)> {
    pairs.iter().map(|(way, share)| (way.to_string(), *share)).collect()
}

#[tokio::test]
async fn a_new_setup_is_given_more_of_the_cards_while_its_cards_are_sent_back_no_more_often_than_the_usual() {
    let rig = node().await;
    shop(&rig, json!({ "agents": agents("writer") })).await;
    let started = rollout(&rig).await;
    assert_eq!(started["rule"]["grows"], "slate", "{started}");

    // One card each, turn about, and both pass their review: `slate` is given three of every four next.
    let before = rig.mark();
    done(&rig, "Add a cart").await;
    done(&rig, "Add a till").await;
    let said = rig.event("system.comparison.shared", move |event| event.seq > before).await;
    assert_eq!(said.data["by"], "rule");
    assert_eq!(said.data["now"], json!({ "usual": 25, "slate": 75 }));
    assert_eq!(
        said.data["why"],
        "`slate` came to `failed` of its `verdict` for 0 of the 1 things read since its share was last set, and \
         `usual` for 0 of 1: no more often, so its share goes from 50% to 75%, as far as its rule takes it."
    );
    assert_eq!(shares(&rig).await, of(&[("usual", 25), ("slate", 75)]));

    for title in ["Add a shelf", "Add a door", "Add a sign", "Add a bell"] {
        done(&rig, title).await;
    }
    // Of the four after the change, one to the usual worker and three to the new setup.
    assert_eq!((given(&rig, "shop-worker-1").await.len(), given(&rig, "shop-slate-1").await.len()), (2, 4));
    let laid = rolled(&rig).await;
    assert_eq!(laid["rule"]["done"], true, "{laid}");
    assert_eq!(laid["shared"].as_array().map(Vec::len), Some(1), "said once: {laid}");
    assert_eq!(laid["parts"][1], json!({ "way": "slate", "share": 75, "percent": 75 }));
    assert!(!laid.to_string().contains("better"), "it counts and never scores");
}

#[tokio::test]
async fn a_new_setup_sent_back_more_often_than_the_usual_is_held_where_it_is_and_said_so() {
    let rig = node().await;
    shop(&rig, json!({ "agents": agents("sloppy"), "rounds": 1 })).await;
    rollout(&rig).await;
    let before = rig.mark();
    done(&rig, "Add a cart").await;
    // The new setup's card is sent back, and left for a person once it has gone round as often as it may.
    let id = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let card = id.clone();
    rig.event("card.left", move |event| event.data["card"] == card.as_str()).await;
    let said = rig.event("system.comparison.shared", move |event| event.seq > before).await;
    assert_eq!(said.data["was"], said.data["now"], "held: {}", said.data);
    let why = said.data["why"].as_str().unwrap();
    assert!(
        why.ends_with(
            "`usual` for 0 of 1: more often, so its share stays at 50%, and what comes next is read again."
        ),
        "{why}"
    );
    assert_eq!(shares(&rig).await, of(&[("usual", 1), ("slate", 1)]));

    // Somebody says the shares again, with why, and the rule goes on from there; then takes the rule away.
    let why = "Held back while the sloppy file is looked into.";
    let shared =
        rig.send("/api/comparisons/rollout/shares", json!({ "shares": { "usual": 9 }, "why": why })).await;
    assert_eq!(shared["shared"].as_array().map(Vec::len), Some(2), "{shared}");
    let laid = rolled(&rig).await;
    assert_eq!(laid["shared"][1]["why"], why);
    assert_eq!(laid["shared"][1]["by"].as_str().map(|by| by.starts_with("user:")), Some(true), "{laid}");
    assert_eq!(laid["parts"][0], json!({ "way": "usual", "share": 9, "percent": 90 }));
    let ruled = rig.call(
        axum::http::Method::PUT,
        "/api/comparisons/rollout/rule",
        Some(&rig.token),
        Some(json!({ "rule": null })),
    );
    let (status, ruled) = ruled.await;
    assert!(status.is_success(), "{ruled}");
    assert!(ruled["rule"].is_null(), "{ruled}");
    // A share of nothing, and a way it has not got, are refused with what is wrong.
    let (status, refused) = rig
        .call(
            axum::http::Method::POST,
            "/api/comparisons/rollout/shares",
            Some(&rig.token),
            Some(json!({ "shares": { "otter": 1 } })),
        )
        .await;
    assert_eq!(status.as_u16(), 400, "{refused}");
    assert!(refused.to_string().contains("has no way called `otter`"), "{refused}");
}

#[tokio::test]
async fn a_new_setup_sent_back_more_often_goes_back_a_step_where_its_rule_says_how_far() {
    let rig = node().await;
    shop(&rig, json!({ "agents": agents("sloppy"), "rounds": 1 })).await;
    let started = rollout_falling(&rig, Some(25)).await;
    assert_eq!(started["rule"]["least"], 25, "{started}");
    let before = rig.mark();
    done(&rig, "Add a cart").await;
    let id = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let card = id.clone();
    rig.event("card.left", move |event| event.data["card"] == card.as_str()).await;
    let said = rig.event("system.comparison.shared", move |event| event.seq > before).await;
    assert_eq!(said.data["now"], json!({ "usual": 75, "slate": 25 }), "{}", said.data);
    let why = said.data["why"].as_str().unwrap();
    assert!(
        why.ends_with("more often, so its share goes back from 50% to 25%, the least its rule gives it."),
        "{why}"
    );
    // Back at its least, the usual worker has three of the next four cards, and the new setup one.
    for title in ["Add a shelf", "Add a door", "Add a sign", "Add a bell"] {
        through(&rig, title).await;
    }
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 4);
    let laid = rolled(&rig).await;
    assert_eq!(laid["parts"][1], json!({ "way": "slate", "share": 25, "percent": 25 }), "{laid}");
    assert!(laid["rule"]["said"].as_str().unwrap().ends_with("to 25 at least"), "{laid}");
}
