//! What an agent is told is a file a blueprint links, and a project's own
//! is read for what is running only as it was vouched for: saved through
//! the daemon by somebody who holds what the instance runs with, it is
//! what the next asking says, and nobody applies; written by somebody who
//! holds less, or with no daemon, it is not; and where the owner has said
//! texts are followed, it is taken as it changes.

use reqwest::{Method, StatusCode};
use serde_json::json;

use crate::Rig;

/// What an agent holds unless its slot says otherwise.
const AGENTS: &[&str] =
    &["projects", "channels", "board", "records", "blueprints", "agents:read", "fleet:read", "search"];

/// Says what its linked file says, each time it is poked.
const ASKS: &str = "\
blueprint: asks
on:
  poke:
    event: shop.poke
    steps:
      words: { do: text.put, with: { parts: [{ from: ask.md, with: { item: shop-12 } }] } }
      say: { do: emit, with: { topic: shop.told, text: \"${steps.words.out.text}\" } }
";

const FILE: &str = "@blueprints/prompts/ask.md";

/// Writes a file of the project's with `token`, through the daemon.
async fn write(rig: &Rig, token: &str, path: &str, text: &str) {
    let body = json!({ "path": path, "content": text });
    let (status, answer) = rig.call(Method::PUT, "/api/projects/shop/file", Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

/// A shop whose own blueprint links its own prompt, applied by the owner.
async fn shop(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    write(rig, &rig.token, FILE, "Look at ${item}.\n").await;
    let path = format!("@blueprints/{}", brand::blueprint_file("asks"));
    write(rig, &rig.token, &path, ASKS).await;
    let applied = rig.send("/api/apply", json!({ "project": "shop", "path": path, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// What it says when it is next poked.
async fn told(rig: &Rig) -> String {
    let before = rig.mark();
    rig.send("/api/emit/shop.poke", json!({})).await;
    let said = rig.event("shop.told", move |event| event.seq > before).await;
    said.data["text"].as_str().unwrap().to_string()
}

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_prompt_saved_by_whoever_applied_is_what_the_next_asking_says_and_nobody_applies() {
    let rig = Rig::new().await;
    shop(&rig).await;
    assert_eq!(told(&rig).await, "Look at shop-12.");
    // The owner saves other words, as the editor does. Nothing is applied.
    write(&rig, &rig.token, FILE, "Look hard at ${item}.\n").await;
    assert_eq!(told(&rig).await, "Look hard at shop-12.");
}

#[tokio::test]
async fn a_prompt_written_by_somebody_who_holds_less_or_with_no_daemon_is_not_what_is_said() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // Somebody who holds what an agent does saves the file: what is running
    // goes on saying what the owner vouched for.
    let agent = key(&rig, AGENTS).await;
    write(&rig, &agent, FILE, "Say nothing of ${item}.\n").await;
    assert_eq!(told(&rig).await, "Look at shop-12.");
    // And written where no daemon saw who wrote it.
    std::fs::write(
        rig.placed("shop").join(FILE.trim_start_matches("@blueprints/")),
        "Planted for ${item}.\n",
    )
    .unwrap();
    assert_eq!(told(&rig).await, "Look at shop-12.");
    // The owner applies again saying to take the files as they are: then it is.
    let path = format!("@blueprints/{}", brand::blueprint_file("asks"));
    let applied =
        rig.send("/api/apply", json!({ "project": "shop", "path": path, "wait": true, "take": true })).await;
    // Nothing of the blueprint changed, so nothing is run again: the file is taken, and that is said.
    assert_eq!(applied["vouched"]["taken"], json!(["prompts/ask.md"]), "{applied}");
    assert_eq!(told(&rig).await, "Planted for shop-12.");
}

#[tokio::test]
async fn where_texts_are_followed_a_changed_prompt_is_what_is_said_next_with_no_apply() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let (status, now) = rig
        .call(Method::POST, "/api/system/texts", Some(&rig.token), Some(json!({ "come": "follow" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{now}");
    // Whoever may write the file writes it, and it is taken as it changes.
    let agent = key(&rig, AGENTS).await;
    let before = rig.mark();
    write(&rig, &agent, FILE, "Look twice at ${item}.\n").await;
    rig.event("project.file.changed", move |event| event.seq > before).await;
    assert_eq!(told(&rig).await, "Look twice at shop-12.");
}

/// A package that brings a blueprint and the words it links, as it is
/// released with `words`.
fn package(words: &str) -> String {
    format!(
        r#"
extension: reach
title: Reach
description: Not a real product.
program: reach-which-is-not-installed
blueprints:
  ask: |
    blueprint: reach/ask
    version: 1
    on:
      poke:
        event: shop.poke
        steps:
          words: {{ do: text.put, with: {{ parts: [{{ from: ask.md, with: {{ item: shop-12 }} }}] }} }}
          say: {{ do: emit, with: {{ topic: shop.told, text: "${{steps.words.out.text}}" }} }}
prompts:
  ask.md: |
    {words}
"#
    )
}

#[tokio::test]
async fn a_prompt_that_ships_is_held_with_its_tree_said_to_have_changed_taken_and_put_back() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/extensions", json!({ "source": package("Look at ${item}.") })).await;
    let ask = |more: serde_json::Value| {
        let mut body = json!({ "blueprint": "reach/ask", "project": "shop", "wait": true });
        body.as_object_mut().unwrap().extend(more.as_object().unwrap().clone());
        rig.send("/api/apply", body)
    };
    ask(json!({})).await;
    assert_eq!(told(&rig).await, "Look at shop-12.");

    // The package is released again with other words: what is running goes on with the ones it
    // reached, and whoever applies again is told the prompt changed. Its blueprint is the version it was.
    rig.send("/api/extensions", json!({ "source": package("Look hard at ${item}.") })).await;
    assert_eq!(told(&rig).await, "Look at shop-12.");
    let again = ask(json!({})).await;
    let moved = json!([{ "name": "reach/prompts/ask.md", "was": null, "now": null, "changed": ["its words changed"] }]);
    assert_eq!((&again["reached"]["left"], &again["reached"]["taken"]), (&moved, &json!([])), "{again}");
    assert_eq!(told(&rig).await, "Look at shop-12.");
    let shown = rig.read(&format!("/api/instances/{}", again["instance"].as_str().unwrap())).await;
    assert_eq!(
        shown["behind"],
        serde_json::Value::Null,
        "a prompt that changed is no other version of its blueprint"
    );

    // Taken with the rest, and put back with it.
    let taken = ask(json!({ "take": true })).await;
    assert_eq!(taken["reached"]["taken"], moved, "{taken}");
    assert_eq!(told(&rig).await, "Look hard at shop-12.");
    let back = ask(json!({ "back": true })).await;
    assert_eq!(back["reached"]["taken"][0]["name"], "reach/prompts/ask.md", "{back}");
    assert_eq!(told(&rig).await, "Look at shop-12.");
}
