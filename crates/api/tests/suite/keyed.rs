//! A key that applied a fleet, withdrawn while the fleet has a card in
//! hand: what the key applied is stopped as a person stopping it would
//! stop it (its agents with it, their turns called off), and what it did
//! stays done: the card is where it was.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Listening;
use crate::fleets::{add, card, params};
use crate::lanes::{git, move_card};
use crate::reapplied::{WORKER, began, stand_ins};

/// Everything alive that is the project's: its blueprint and the key it runs as.
async fn alive_in_shop(rig: &crate::Rig) -> Vec<(String, Option<String>)> {
    let filter = conductor::InstanceFilter {
        live_only: true,
        project: Some("shop".into()),
        limit: 1000,
        ..Default::default()
    };
    let found = rig.node().app.engine.instances(filter).await.unwrap();
    found.into_iter().map(|one| (one.blueprint, one.principal.key)).collect()
}

#[tokio::test]
async fn withdrawing_the_key_that_applied_a_fleet_stops_it_and_calls_its_turn_off_and_leaves_the_card_where_it_is()
 {
    let (rig, at) = stand_ins("api-keyed", None).await;
    let mut there = Listening::at(&at.join("began"));
    crate::hold(&at.join(format!("hold-{WORKER}")));
    // The board is the owner's; the fleet is applied with a key of theirs.
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = std::path::PathBuf::from(project["path"].as_str().unwrap());
    for args in [
        &["init", "--quiet", "."][..],
        &["config", "user.name", "Ada"],
        &["config", "user.email", "ada@example.com"],
        &["add", "-A"],
        &["commit", "--quiet", "-m", "first"],
    ] {
        git(&dir, args);
    }
    let columns = ["draft", "ready", "doing", "review", "done"];
    let board = json!({ "blueprint": "std/board", "project": "shop", "params": { "columns": columns }, "wait": true });
    assert_eq!(rig.send("/api/apply", board).await["finished"]["status"], "ok");
    let key: Value = rig.send("/api/credentials", json!({ "name": "deploys" })).await;
    let (id, token) = (key["id"].as_str().unwrap(), key["token"].as_str().unwrap());
    let fleet =
        json!({ "blueprint": "std/fleet", "project": "shop", "params": params(json!({})), "wait": true });
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(token), Some(fleet)).await;
    assert_eq!((status, &applied["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{applied}");
    let fleet_id = applied["instance"].as_str().unwrap().to_string();

    // A card in hand, its worker's program there and held.
    let card_id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &card_id, "ready", None).await;
    began(&mut there, WORKER).await;
    assert_eq!(card(&rig, &card_id).await["status"], "doing");
    let from = rig.mark();

    // Asked first, the fleet is what withdrawing would stop.
    let look = rig.read(&format!("/api/credentials/{id}/withdrawing")).await;
    let named: Vec<&str> =
        look["running"].as_array().unwrap().iter().map(|one| one["id"].as_str().unwrap()).collect();
    assert_eq!(named, [fleet_id.as_str()], "{look}");

    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/credentials/{id}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let stopped: Vec<&str> =
        gone["stopped"].as_array().unwrap().iter().map(|one| one["id"].as_str().unwrap()).collect();
    assert_eq!(stopped, [fleet_id.as_str()], "{gone}");

    // Nothing the key applied is alive, the board and what looks after
    // its cards, which are the owner's, are, and its worker's turn was
    // called off, which is no failure of the agent's.
    let left = alive_in_shop(&rig).await;
    assert!(left.iter().all(|(_, key)| key.as_deref() != Some(id)), "{left:?}");
    assert!(left.iter().any(|(blueprint, _)| blueprint == "std/board"), "{left:?}");
    let ended =
        rig.event("agent.turn.finished", |event| event.seq > from && event.data["slot"] == WORKER).await;
    assert_eq!(ended.data["called_off"], true, "{:?}", ended.data);
    // What it did stays done: the card is where the fleet put it.
    assert_eq!(card(&rig, &card_id).await["status"], "doing");
}
