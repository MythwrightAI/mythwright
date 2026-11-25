//! An agent moved to another project, or to none, asked of the daemon
//! (`PATCH /api/agents/{name}` with `project` alone): nothing else of it
//! changed, one that was stopped left stopped, refused to a key that holds
//! less and for one that has work in hand or that a fleet counts, and one
//! this factory does not start moved as any is.

use std::path::PathBuf;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A node with two projects, and an agent's program that waits at a hold
/// before it answers, with the hold.
async fn rig() -> (Rig, PathBuf) {
    let mut gate = None;
    let rig = Rig::with("api-relocated", |config, dir| {
        let pipe = dir.join("gate");
        crate::hold(&pipe);
        let does = ["swallow", "held", &pipe.to_string_lossy(), "waiter", "say", "Here it is."];
        config.adapters.push(("waiter".into(), crate::standin::argv(&does)));
        config.adapters.push(("echo".into(), crate::standin::argv(&["swallow", "say", "done"])));
        gate = Some(pipe);
    })
    .await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/projects", json!({ "name": "Cafe" })).await;
    rig.send("/api/channels", json!({ "name": "shop", "project": "shop" })).await;
    (rig, gate.unwrap())
}

async fn moved(rig: &Rig, token: &str, name: &str, project: Value) -> (StatusCode, Value) {
    rig.call(Method::PATCH, &format!("/api/agents/{name}"), Some(token), Some(json!({ "project": project })))
        .await
}

/// A key that holds `scopes` and nothing else.
async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

fn why(refused: &Value) -> &str {
    refused["error"]["message"].as_str().unwrap_or_default()
}

/// An agent as the list of agents says it.
async fn agent(rig: &Rig, name: &str) -> Value {
    let all = rig.read("/api/agents").await;
    let found = all["agents"].as_array().unwrap().iter().find(|one| one["name"] == name).cloned();
    found.unwrap_or_else(|| panic!("no agent called {name}: {all}"))
}

/// Everything of an agent but what a move changes: its project, and the
/// text of its own it is applied as there (`std/agent`, one for a project).
fn rest(agent: &Value) -> serde_json::Map<String, Value> {
    let mut agent = agent.as_object().unwrap().clone();
    agent.retain(|name, _| !["project", "updated", "instance"].contains(&name.as_str()));
    agent
}

#[tokio::test]
async fn an_agent_is_moved_to_another_project_or_none_with_nothing_else_of_it_changed() {
    let (rig, _gate) = rig().await;
    rig.send("/api/agents", json!({ "name": "helper", "project": "shop", "instructions": "Be brief." }))
        .await;
    let token = || std::fs::read_to_string(rig.dir.join("data/slots/helper/token")).unwrap();
    let held = token();
    let before = agent(&rig, "helper").await;
    assert_eq!(before["held_by"], Value::Null, "nothing running holds one added by hand: {before}");

    let mark = rig.mark();
    let (status, now) = moved(&rig, &rig.token, "helper", json!("cafe")).await;
    assert_eq!((status, &now["project"]), (StatusCode::OK, &json!("cafe")), "{now}");
    let after = agent(&rig, "helper").await;
    assert_eq!(rest(&before), rest(&after), "only its project changed");
    let said = rig.event("agent.slot.updated", move |event| event.seq > mark).await;
    assert_eq!((&said.data["slot"], &said.data["project"]), (&json!("helper"), &json!("cafe")), "{said:?}");
    assert_eq!(token(), held, "what it may ask says no project, so its credential is the one it had");

    // Stopped, it stays stopped: to none, and back; to where it is already, nothing.
    let (status, _) = rig.call(Method::DELETE, "/api/agents/helper", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, none) = moved(&rig, &rig.token, "helper", Value::Null).await;
    assert_eq!(
        (status, &none["project"], &none["active"]),
        (StatusCode::OK, &Value::Null, &json!(false)),
        "{none}"
    );
    let (status, again) = moved(&rig, &rig.token, "helper", json!("shop")).await;
    assert_eq!(
        (status, &again["project"], &again["active"]),
        (StatusCode::OK, &json!("shop"), &json!(false))
    );
    let (status, same) = moved(&rig, &rig.token, "helper", json!("shop")).await;
    assert_eq!((status, &same["project"]), (StatusCode::OK, &json!("shop")), "{same}");

    // To a project that is not there, or with something else said beside it.
    let (status, nowhere) = moved(&rig, &rig.token, "helper", json!("attic")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{nowhere}");
    let beside = json!({ "project": "cafe", "role": "worker" });
    let (status, refused) =
        rig.call(Method::PATCH, "/api/agents/helper", Some(&rig.token), Some(beside)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(why(&refused).starts_with("an agent is changed here by `scopes`"), "{refused}");
    assert_eq!(agent(&rig, "helper").await["project"], "shop");
}

#[tokio::test]
async fn an_agent_is_not_moved_by_a_key_that_holds_less_than_it_or_than_changing_agents_takes() {
    let (rig, _gate) = rig().await;
    rig.send("/api/agents", json!({ "name": "helper", "project": "shop" })).await;
    let reads = key(&rig, &["agents:read", "projects"]).await;
    let (status, refused) = moved(&rig, &reads, "helper", json!("cafe")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(why(&refused).contains("`agents:write`"), "{refused}");
    // One that may change agents and holds less than this one does.
    let changes = key(&rig, &["agents", "projects:read"]).await;
    let (status, refused) = moved(&rig, &changes, "helper", json!("cafe")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(why(&refused).contains("cannot move `helper` to another project: it holds `"), "{refused}");
    assert_eq!(agent(&rig, "helper").await["project"], "shop");
    // Holding all it holds, it may: applying what it is set up by there.
    let mut all: Vec<&str> = kernel::principal::AGENT_SCOPES.to_vec();
    all.extend(["agents", "blueprints:run"]);
    let enough = key(&rig, &all).await;
    let (status, now) = moved(&rig, &enough, "helper", json!("cafe")).await;
    assert_eq!((status, &now["project"]), (StatusCode::OK, &json!("cafe")), "{now}");
}

#[tokio::test]
async fn an_agent_with_work_in_hand_or_that_a_fleet_counts_is_not_moved_and_one_from_outside_is() {
    let (rig, gate) = rig().await;
    rig.send(
        "/api/agents",
        json!({ "name": "maker", "adapter": "waiter", "project": "shop", "channel": "shop" }),
    )
    .await;

    // In the middle of answering, it has that in hand, and is moved once it has answered.
    let before = rig.mark();
    rig.send("/api/channels/shop/messages", json!({ "body": "@maker three palettes, please" })).await;
    rig.event("agent.turn.started", move |event| event.seq > before && event.data["slot"] == "maker").await;
    let (status, refused) = moved(&rig, &rig.token, "maker", json!("cafe")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(why(&refused).starts_with("`maker` has 1 piece of work in hand"), "{refused}");
    assert!(why(&refused).contains("agent stop maker"), "it says what to do: {refused}");
    assert_eq!(agent(&rig, "maker").await["project"], "shop");
    crate::let_go("the agent to be waiting at its hold", &gate).await;
    rig.event("agent.turn.finished", move |event| event.seq > before && event.data["slot"] == "maker").await;
    let (status, now) = moved(&rig, &rig.token, "maker", json!("cafe")).await;
    assert_eq!((status, &now["project"]), (StatusCode::OK, &json!("cafe")), "{now}");

    // One a role counts has its project from that, and says so in the list.
    let role = json!({
        "blueprint": "std/role", "project": "shop", "key": "workers", "wait": true,
        "params": { "role": "worker", "name": "shop-worker", "count": 1, "adapter": "echo" },
    });
    let applied = rig.send("/api/apply", role).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(agent(&rig, "shop-worker-1").await["held_by"], "workers");
    let (status, refused) = moved(&rig, &rig.token, "shop-worker-1", json!("cafe")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(why(&refused).starts_with("`shop-worker-1` is brought up by `workers` (in_"), "{refused}");
    assert!(why(&refused).contains("in the `agents` table of the project `shop`"), "{refused}");
    assert_eq!(agent(&rig, "shop-worker-1").await["project"], "shop");

    // One this factory does not start moves as any does.
    let invited = rig.send("/api/agent/invite", json!({ "name": "walls", "project": "shop" })).await;
    assert_eq!(invited["agent"]["name"], "walls", "{invited}");
    let (status, now) = moved(&rig, &rig.token, "walls", json!("cafe")).await;
    assert_eq!(
        (status, &now["project"], &now["adapter"]),
        (StatusCode::OK, &json!("cafe"), &json!(agents::EXTERNAL))
    );
}

#[tokio::test]
async fn an_agent_moved_to_another_project_works_under_none_of_the_environments_of_the_one_it_left() {
    let (rig, _gate) = rig().await;
    rig.send("/api/projects/shop/environments", json!({ "name": "staging" })).await;
    let under = json!({ "name": "helper", "project": "shop", "environment": "staging" });
    rig.send("/api/agents", under).await;
    assert_eq!(agent(&rig, "helper").await["environment"], "staging");
    let (status, now) = moved(&rig, &rig.token, "helper", json!("cafe")).await;
    assert_eq!(
        (status, &now["project"], &now["environment"]),
        (StatusCode::OK, &json!("cafe"), &json!("")),
        "{now}"
    );
    assert_eq!(agent(&rig, "helper").await["environment"], "", "staging is Shop's, and Cafe has none");
}

#[tokio::test]
async fn an_agent_added_by_hand_and_in_is_moved_to_none_and_works_in_none() {
    let (rig, _gate) = rig().await;
    rig.send("/api/agents", json!({ "name": "helper", "project": "shop", "instructions": "Be brief." }))
        .await;
    let before = agent(&rig, "helper").await;
    assert_eq!(before["active"], true, "{before}");
    // Its text is applied for no project, and the agent it brings up works in none: not the project it had.
    let (status, none) = moved(&rig, &rig.token, "helper", Value::Null).await;
    assert_eq!(
        (status, &none["project"], &none["active"]),
        (StatusCode::OK, &Value::Null, &json!(true)),
        "{none}"
    );
    let after = agent(&rig, "helper").await;
    assert_eq!(after["project"], Value::Null, "{after}");
    assert_eq!(rest(&before), rest(&after), "only its project changed");
    // And back into a project, where it works again.
    let (status, back) = moved(&rig, &rig.token, "helper", json!("cafe")).await;
    assert_eq!(
        (status, &back["project"], &back["active"]),
        (StatusCode::OK, &json!("cafe"), &json!(true)),
        "{back}"
    );
}
