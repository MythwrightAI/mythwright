//! A daemon started again answers before it has taken up what it was
//! doing: what is read at once, from what is written, and what changes
//! anything once it has, after `system.started`. Meanwhile it says that
//! it is taking up, and how far it has come (`api::resuming`).

use std::sync::Arc;

use api::Node;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::{Rig, in_time, soon};

/// An instance that keeps what it does and waits to be told to go on: what
/// a daemon started again has to take up.
const WAITS: &str = "blueprint: waits\nsteps:\n  held:\n    do: wait\n    with: { event: shop.go }\n";

/// A request to the node answering at `at`, as the rig's owner.
async fn asked(rig: &Rig, at: &str, method: Method, path: &str) -> (StatusCode, Value) {
    let client = reqwest::Client::new();
    let response = in_time(
        &format!("{method} {path} to be answered while the node takes up"),
        client.request(method.clone(), format!("{at}{path}")).bearer_auth(&rig.token).json(&json!({})).send(),
    )
    .await
    .unwrap();
    let status = response.status();
    (status, response.json().await.unwrap_or(Value::Null))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_started_again_answers_reads_before_it_has_taken_up_and_holds_writes_until_it_has() {
    let mut rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = rig.send("/api/apply", json!({ "source": WAITS, "project": "shop" })).await;
    let instance = applied["instance"].as_str().unwrap().to_string();
    rig.event("step.waiting", |event| event.source.instance.as_deref() == Some(instance.as_str())).await;
    rig.stop().await;

    // Started again, held as it is about to take up what it was doing.
    let held = Arc::new(api::resuming::Held::default());
    let mut config = rig.config.clone();
    config.held = Some(held.clone());
    let starting = tokio::spawn(Node::start(config));
    let at = format!("http://{}", soon("the start to reach its taking up", held.reached()).await);

    // It answers what is read, and says it is taking up.
    let (status, meta) = asked(&rig, &at, Method::GET, "/api/meta").await;
    assert_eq!(status, StatusCode::OK, "{meta}");
    let says = meta["taking_up"]["says"].as_str().unwrap_or_default();
    assert!(says.contains("still taking up what it was doing"), "{meta}");
    let (status, fleet) = asked(&rig, &at, Method::GET, "/api/fleet").await;
    assert_eq!(status, StatusCode::OK, "{fleet}");
    assert_eq!(fleet["taking_up"], meta["taking_up"], "{fleet}");
    let shown = fleet["instances"].as_array().unwrap();
    assert!(shown.iter().any(|one| one["id"] == instance.as_str()), "what is kept is read: {fleet}");
    let (status, one) = asked(&rig, &at, Method::GET, &format!("/api/instances/{instance}")).await;
    // As written by the daemon that stopped: its steps had not settled.
    assert_eq!((status, &one["instance"]["status"]), (StatusCode::OK, &json!("starting")), "{one}");

    // What changes anything waits until it has taken up.
    let (token, told) = (rig.token.clone(), at.clone());
    let go = tokio::spawn(async move {
        let client = reqwest::Client::new();
        let response = client.post(format!("{told}/api/emit/shop.go")).bearer_auth(token).json(&json!({}));
        response.send().await.unwrap().status()
    });
    soon("the write to be held", held.holds_one()).await;
    assert!(!go.is_finished(), "a write is answered only once the daemon has taken up");
    held.go();

    let node = in_time("the node to take up what it was doing", starting).await.unwrap().unwrap();
    assert_eq!(in_time("the held write to be answered", go).await.unwrap(), StatusCode::OK);
    rig.url = node.url();
    rig.node = Some(node);

    // `system.started` says it is done, before what waited was let through,
    // and the instance taken up hears what that said and goes on.
    let started = rig.event("system.started", |event| event.data["resumed"].as_u64() >= Some(1)).await;
    let said = rig.event("shop.go", |_| true).await;
    assert!(started.seq < said.seq, "the write came after the daemon said it was up");
    rig.event("instance.finished", |event| event.source.instance.as_deref() == Some(instance.as_str())).await;
    assert_eq!(rig.read("/api/meta").await["taking_up"], Value::Null);
}
