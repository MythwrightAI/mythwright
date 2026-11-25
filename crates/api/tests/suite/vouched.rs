//! A text runs with the authority of whoever applied it, and of nobody who
//! only wrote a file.
//!
//! A project's own blueprints are files in the project. Whoever may write
//! the project's files can write them, and what a running fleet calls is
//! read by name when it is called: so a file is run for a fleet only as
//! somebody who holds what the fleet runs with applied or saved it.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{change, params, rig, shop};

/// A key of the rig's owner that holds `scopes` and nothing else.
async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// What an agent holds unless its slot says otherwise.
const AGENTS: &[&str] =
    &["projects", "channels", "board", "records", "blueprints", "agents:read", "fleet:read", "search"];

/// The arrangement that ships, with one step more: it says `shop.planted`.
fn planted() -> String {
    let shipped = include_str!("../../../../blueprints/std/arrangement.bp");
    let step = "steps:\n  planted: { do: emit, with: { topic: shop.planted } }\n\n";
    shipped.replacen("\nsteps:\n", &format!("\n{step}"), 1)
}

/// Writes the project's own `std/arrangement` with `token`.
async fn write(rig: &Rig, token: &str, text: &str) {
    write_as(rig, token, &brand::blueprint_file("std/arrangement"), text).await;
}

/// The same, under one of the names a blueprint's file may have.
async fn write_as(rig: &Rig, token: &str, file: &str, text: &str) {
    let path = format!("@blueprints/{file}");
    let body = json!({ "path": path, "content": text });
    let (status, answer) = rig.call(Method::PUT, "/api/projects/shop/file", Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

async fn events(rig: &Rig, topic: &str) -> Vec<Value> {
    let query = journal::EventQuery { topics: topic.to_string(), limit: 1000, ..Default::default() };
    rig.said(query).await.into_iter().map(|event| event.data).collect()
}

#[tokio::test]
async fn a_file_written_by_someone_who_holds_less_is_not_run_with_the_authority_of_whoever_applied_the_fleet()
{
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    // Somebody who holds what an agent does writes a file of the project's
    // own under the name of a blueprint the owner's fleet calls each time
    // a row of its tables changes.
    let agent = key(&rig, AGENTS).await;
    write(&rig, &agent, &planted()).await;
    change(&rig, "agents", "worker", json!({ "told": "Work the card." })).await;

    // The fleet was brought to what its tables say, by the arrangement
    // that ships; nothing of the file ran, and the fleet says why.
    assert_eq!(events(&rig, "shop.planted").await, Vec::<Value>::new());
    let kept = events(&rig, "instance.call_kept").await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0]["blueprint"], "std/arrangement");
    let why = kept[0]["error"].as_str().unwrap();
    assert!(
        why.contains("`shop` has a file of its own for `std/arrangement`")
            && why.contains("so the one that ships is run"),
        "{why}"
    );

    // Asking for the fleet to be applied, as one who holds less, is that
    // one's word and no more: what runs as the owner goes on as it was.
    let asked = json!({ "blueprint": "std/fleet", "project": "shop", "params": params(json!({})) });
    let (status, answer) = rig.call(Method::POST, "/api/apply", Some(&agent), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    change(&rig, "agents", "worker", json!({ "told": "Work the card well." })).await;
    assert_eq!(events(&rig, "shop.planted").await, Vec::<Value>::new());

    // The owner applies the fleet again, for whatever reason: the file is
    // left as it was, which is not there, and they are told it has changed.
    let asked = json!({ "blueprint": "std/fleet", "project": "shop", "params": params(json!({})) });
    let applied = rig.send("/api/apply", asked.clone()).await;
    assert_eq!(applied["vouched"], json!({ "left": ["std/arrangement"] }), "{applied}");
    change(&rig, "agents", "worker", json!({ "told": "Work the card." })).await;
    assert_eq!(events(&rig, "shop.planted").await, Vec::<Value>::new());

    // Having read it, they say to take the project's files as they are:
    // from then it is theirs to have run.
    let mut taking = asked;
    taking["take"] = json!(true);
    let applied = rig.send("/api/apply", taking).await;
    assert_eq!(applied["vouched"], json!({ "taken": ["std/arrangement"] }), "{applied}");
    change(&rig, "agents", "worker", json!({ "told": "Work the card well." })).await;
    assert!(!events(&rig, "shop.planted").await.is_empty());
}

#[tokio::test]
async fn a_file_saved_by_whoever_applied_the_fleet_is_the_one_its_next_call_runs() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    // The owner saves an arrangement of the project's own, as the editor
    // does: they hold what the fleet runs with, and could have applied it.
    write(&rig, &rig.token, &planted()).await;
    change(&rig, "agents", "worker", json!({ "told": "Work the card." })).await;
    assert_eq!(events(&rig, "shop.planted").await.len(), 1);
    assert_eq!(events(&rig, "instance.call_kept").await, Vec::<Value>::new());

    // Somebody who holds less changes it after them: it is run as the
    // owner saved it, and that is said.
    let agent = key(&rig, AGENTS).await;
    write(&rig, &agent, &planted().replace("shop.planted", "shop.taken")).await;
    change(&rig, "agents", "worker", json!({ "told": "Work the card well." })).await;
    assert_eq!(events(&rig, "shop.taken").await, Vec::<Value>::new());
    assert_eq!(events(&rig, "shop.planted").await.len(), 2);
    let kept = events(&rig, "instance.call_kept").await;
    assert!(kept[0]["error"].as_str().unwrap().contains("changed after"), "{kept:?}");
}

#[tokio::test]
async fn a_blueprint_whose_file_is_called_as_yaml_is_vouched_for_as_any_other() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    let file = "std/arrangement.bp.yml";
    // Somebody who holds what an agent does writes it: nothing of it runs
    // for the owner's fleet, and the fleet says which file and why.
    let agent = key(&rig, AGENTS).await;
    write_as(&rig, &agent, file, &planted()).await;
    change(&rig, "agents", "worker", json!({ "told": "Work the card." })).await;
    assert_eq!(events(&rig, "shop.planted").await, Vec::<Value>::new());
    let kept = events(&rig, "instance.call_kept").await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert!(kept[0]["error"].as_str().unwrap().contains("so the one that ships is run"), "{kept:?}");

    // The owner saves it, as the editor does: it is theirs to have run.
    write_as(&rig, &rig.token, file, &planted()).await;
    change(&rig, "agents", "worker", json!({ "told": "Work the card well." })).await;
    assert_eq!(events(&rig, "shop.planted").await.len(), 1);
}
