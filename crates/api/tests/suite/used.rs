//! Which blueprints a project uses: the ones applied there that are still
//! alive, and the ones that are files of its own. A project's list is that
//! and no more; every blueprint the installation has is another list.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Stays alive: it listens.
const WATCHER: &str = "\
blueprint: watcher
on:
  wanted:
    event: shop.wanted
    steps:
      say: { do: emit, with: { topic: shop.heard } }
";

fn names(listed: &Value) -> Vec<&str> {
    listed.as_array().unwrap().iter().filter_map(Value::as_str).collect()
}

#[tokio::test]
async fn a_project_uses_the_blueprints_applied_there_that_are_alive_and_the_files_of_its_own() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    let path = format!("@blueprints/{}", brand::blueprint_file("watcher"));
    let body = json!({ "path": path, "content": WATCHER });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    // A file of its own is the project's before anything is applied; a
    // project with neither uses nothing, whatever the installation has.
    let used = rig.read("/api/blueprints/used").await;
    let shop = &used["projects"]["shop"];
    assert_eq!(shop["own"], json!([{ "name": "watcher", "file": path, "links": [] }]));
    assert_eq!(shop["applied"], json!([]));
    assert_eq!(used["projects"]["depot"], json!({ "applied": [], "own": [], "written": "@blueprints" }));
    let all = rig.read("/api/blueprints").await;
    assert!(names(&all["names"]).contains(&"std/board"), "the installation has more than any project uses");

    // Applied, and alive: the board holds its cards, with what looks
    // after all of them started under it, and the watcher listens. What
    // is used is what is alive at any depth, each said once.
    let board =
        rig.send("/api/apply", json!({ "blueprint": "std/board", "project": "shop", "wait": true })).await;
    rig.send("/api/apply", json!({ "path": path, "project": "shop", "wait": true })).await;
    rig.send("/api/apply", json!({ "blueprint": "std/board", "project": "depot", "wait": true })).await;
    let used = rig.read("/api/blueprints/used").await;
    assert_eq!(names(&used["projects"]["shop"]["applied"]), ["std/board", "std/cards", "watcher"]);
    assert_eq!(names(&used["projects"]["depot"]["applied"]), ["std/board", "std/cards"]);

    // A card is a row of the board's table: adding one starts nothing,
    // and the project uses what it did.
    rig.send("/api/board/shop/items", json!({ "title": "A till" })).await;
    rig.send("/api/board/shop/items", json!({ "title": "A shelf" })).await;
    let used = rig.read("/api/blueprints/used").await;
    assert_eq!(names(&used["projects"]["shop"]["applied"]), ["std/board", "std/cards", "watcher"]);

    // Stopped, it is no longer used there; the file is still the project's.
    let id = board["instance"].as_str().unwrap();
    rig.send(&format!("/api/instances/{id}/stop"), json!({})).await;
    let used = rig.read("/api/blueprints/used").await;
    assert_eq!(names(&used["projects"]["shop"]["applied"]), ["watcher"]);
    assert_eq!(used["projects"]["shop"]["own"][0]["name"], "watcher");
}
