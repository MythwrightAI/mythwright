//! An agent taken away for good, asked of the daemon
//! (`/api/agents/{name}/remove`): by a person who manages the factory and
//! by nobody else, refused for one that something running counts and for
//! one with work in hand, and gone with what was its own while what it
//! said and did stays. What goes and what stays, thing by thing, is the
//! agents' own suite's (`removed.rs` there).

use std::path::PathBuf;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A node with a project and its channel, and an agent's program that
/// waits at a hold before it answers, with the hold.
async fn rig() -> (Rig, PathBuf) {
    let mut gate = None;
    let rig = Rig::with("api-away", |config, dir| {
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
    rig.send("/api/channels", json!({ "name": "shop", "project": "shop" })).await;
    (rig, gate.unwrap())
}

/// A key that holds `scopes` and nothing else.
async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// The credential an agent's program reads each time it asks the factory anything.
fn token(rig: &Rig, name: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{name}/token"))).unwrap().trim().to_string()
}

async fn remove(rig: &Rig, token: &str, name: &str) -> (StatusCode, Value) {
    rig.call(Method::POST, &format!("/api/agents/{name}/remove"), Some(token), None).await
}

/// Why something was refused, as the daemon said it.
fn why(refused: &Value) -> &str {
    refused["error"]["message"].as_str().unwrap_or_default()
}

/// The agents there are, by name.
async fn names(rig: &Rig) -> Vec<String> {
    let all = rig.read("/api/agents").await;
    all["agents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|agent| agent["name"].as_str().unwrap().to_string())
        .collect()
}

/// What `token` is told was said on `topics`.
async fn heard(rig: &Rig, token: &str, topics: &str) -> Vec<Value> {
    let path = format!("/api/events/recent?topics={topics}&limit=100");
    let (status, said) = rig.call(Method::GET, &path, Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    said["events"].as_array().cloned().unwrap_or_default()
}

#[tokio::test]
async fn an_agent_is_taken_away_by_a_person_who_manages_the_factory_and_is_gone_with_what_was_its_own() {
    let (rig, _gate) = rig().await;
    // An agent that holds as much as an agent can be given: changing
    // agents, and managing the factory.
    let holds = json!(["channels", "agents", "admin"]);
    rig.send(
        "/api/agents",
        json!({ "name": "helper", "project": "shop", "channel": "shop", "scopes": holds }),
    )
    .await;
    rig.send("/api/agents", json!({ "name": "other", "scopes": holds })).await;
    let before = rig.mark();
    let asked = rig.send("/api/channels/shop/messages", json!({ "body": "@helper what can you do?" })).await;
    let ended = rig
        .event("agent.turn.finished", move |event| event.seq > before && event.data["slot"] == "helper")
        .await;
    let turn = ended.data["turn"].as_str().unwrap().to_string();
    let (its_own, another) = (token(&rig, "helper"), token(&rig, "other"));
    let path = "/api/agents/helper/remove";

    // Not by an agent, its own credential included, whatever it holds;
    // not by a key that holds less, in the words that always says.
    for agent in [&its_own, &another] {
        for how in [Method::GET, Method::POST] {
            let (status, refused) = rig.call(how, path, Some(agent), None).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
            assert!(why(&refused).starts_with("taking an agent away is a person's to do"), "{refused}");
        }
    }
    let reads = key(&rig, &["agents:read", "admin"]).await;
    let (status, refused) = remove(&rig, &reads, "helper").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(why(&refused), "narrow is not allowed to do this (needs `agents:write`)");
    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let (status, refused) = remove(&rig, &member, "helper").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(why(&refused), "narrow is not allowed to do this (needs `admin`)");
    assert_eq!(names(&rig).await, ["helper", "other"]);

    // Asked about first, by whoever may: what would go, and nothing does.
    let looked = rig.read(path).await;
    assert_eq!(
        looked,
        json!({
            "name": "helper", "project": "shop", "removed": false, "refused": null, "active": true,
            "channels": ["shop"], "turns": 1, "cards": [], "folder_left": null, "withdrawn": 0,
        })
    );
    assert_eq!(names(&rig).await, ["helper", "other"]);

    // Taken away, by a key that holds what it takes and no more, which a
    // person made: it says who.
    let manages = key(&rig, &["agents:write", "admin"]).await;
    let before = rig.mark();
    let (status, gone) = remove(&rig, &manages, "helper").await;
    assert_eq!(
        (status, &gone["removed"], &gone["channels"]),
        (StatusCode::OK, &json!(true), &json!(["shop"]))
    );
    let said = rig.event("agent.slot.removed", move |event| event.seq > before).await;
    assert_eq!(said.data, json!({ "slot": "helper", "turns": 1, "channels": ["shop"], "project": "shop" }));
    let left = rig.event("channel.left", move |event| event.seq > before).await;
    assert_eq!(left.data["member"], "agent:helper");

    // Who may see the agents is told, and nobody else.
    let sees = key(&rig, &["agents:read"]).await;
    let told = heard(&rig, &sees, "agent.slot.removed").await;
    assert_eq!(told.len(), 1, "{told:?}");
    assert_eq!(told[0]["data"]["slot"], "helper");
    let elsewhere = key(&rig, &["board:read", "channels:read"]).await;
    assert_eq!(heard(&rig, &elsewhere, "agent.slot.removed").await, Vec::<Value>::new());

    // It is not there, and its credential is nobody's at the next thing
    // it asks.
    assert_eq!(names(&rig).await, ["other"]);
    let (status, _) = rig.call(Method::GET, "/api/agents", Some(&its_own), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let channel = rig.read("/api/channels/shop").await;
    let members: Vec<&str> =
        channel["members"].as_array().unwrap().iter().filter_map(|member| member["actor"].as_str()).collect();
    assert!(!members.contains(&"agent:helper"), "{members:?}");
    assert!(!rig.dir.join("data/slots/helper").exists());
    for how in [Method::GET, Method::POST] {
        let (status, refused) = rig.call(how, path, Some(&rig.token), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    }

    // What it said stays, and what it was asked; and an agent that is
    // later called what it was called has none of it for its own.
    let said = rig.read("/api/channels/shop/messages").await;
    let by: Vec<&str> =
        said["messages"].as_array().unwrap().iter().filter_map(|one| one["author_name"].as_str()).collect();
    assert_eq!(by, ["Ada", "helper"], "{said}");
    assert_eq!(said["messages"][1]["reply_to"], asked["id"]);
    rig.send("/api/agents", json!({ "name": "helper", "project": "shop" })).await;
    assert_eq!(rig.read("/api/agents/helper/turns").await["turns"], json!([]));
    assert_eq!(rig.node().app.agents.turn(&turn).await.unwrap().slot, "helper");
    // The conversation the first one held, in the channel, is held with nobody.
    let channel = rig.read("/api/channels/shop").await;
    let conversation = format!("channel:{}", channel["id"].as_str().unwrap());
    assert_eq!(rig.node().app.agents.holding(&conversation).await.unwrap(), None);
    assert!(
        !channel["members"].to_string().contains("agent:helper"),
        "it is in no channel it was not put in"
    );
    assert_ne!(token(&rig, "helper"), its_own);

    // Stopping one is what it always was: it is stopped, and is there.
    let (status, _) = rig.call(Method::DELETE, "/api/agents/other", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    let all = rig.read("/api/agents").await;
    let other = all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == "other").unwrap();
    assert_eq!(other["active"], false);
}

#[tokio::test]
async fn an_agent_with_work_in_hand_or_that_a_fleet_counts_is_not_taken_away() {
    let (rig, gate) = rig().await;
    rig.send(
        "/api/agents",
        json!({ "name": "maker", "adapter": "waiter", "project": "shop", "channel": "shop" }),
    )
    .await;

    // In the middle of answering, it has that in hand.
    let before = rig.mark();
    rig.send("/api/channels/shop/messages", json!({ "body": "@maker three palettes, please" })).await;
    rig.event("agent.turn.started", move |event| event.seq > before && event.data["slot"] == "maker").await;
    let (status, refused) = remove(&rig, &rig.token, "maker").await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(why(&refused).starts_with("`maker` has 1 piece of work in hand"), "{refused}");
    assert!(why(&refused).contains("agent stop maker"), "it says what to do: {refused}");
    let looked = rig.read("/api/agents/maker/remove").await;
    assert_eq!((looked["removed"].as_bool(), looked["refused"].as_str()), (Some(false), Some(why(&refused))));
    crate::let_go("the agent to be waiting at its hold", &gate).await;
    rig.event("agent.turn.finished", move |event| event.seq > before && event.data["slot"] == "maker").await;
    // With that answered it has nothing in hand, and goes.
    let (status, gone) = remove(&rig, &rig.token, "maker").await;
    assert_eq!((status, &gone["removed"]), (StatusCode::OK, &json!(true)), "{gone}");

    // Two that a role counts are changed where they are counted.
    let role = |count: i64| {
        json!({
            "blueprint": "std/role", "project": "shop", "key": "workers", "wait": true,
            "params": { "role": "worker", "name": "shop-worker", "count": count, "adapter": "echo" },
        })
    };
    let applied = rig.send("/api/apply", role(2)).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let (status, refused) = remove(&rig, &rig.token, "shop-worker-2").await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    let said = why(&refused);
    assert!(said.starts_with("`shop-worker-2` is brought up by `workers` (std/role, in_"), "{said}");
    assert!(said.contains("in the `agents` table of the project `shop`, lower the row's count"), "{said}");
    // One added by hand that is called as a fleet's are is nobody's but
    // its own: it is what holds an agent that is asked, not its name.
    rig.send("/api/agents", json!({ "name": "shop-worker-7", "project": "shop" })).await;
    assert_eq!(remove(&rig, &rig.token, "shop-worker-7").await.0, StatusCode::OK);
    // No longer counted, the second has left and is taken away; the
    // first is counted still.
    let applied = rig.send("/api/apply", role(1)).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(remove(&rig, &rig.token, "shop-worker-2").await.0, StatusCode::OK);
    assert_eq!(remove(&rig, &rig.token, "shop-worker-1").await.0, StatusCode::CONFLICT);
    assert_eq!(names(&rig).await, ["shop-worker-1"]);
}
