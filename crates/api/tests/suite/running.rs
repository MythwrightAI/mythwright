//! What is running for a channel, as its Channel tab lists it
//! (`GET /api/channels/{id}/running`): what says it posts there, held to
//! whoever reads the channel and may read what is applied.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A project's own kind of thing that says something in a channel now and then, and stays.
const UPDATES: &str = "\
blueprint: status-updates
params:
  channel: { type: string }
  every: { type: string, default: 10m }
on:
  asked:
    event: status.asked
    do: set
    with: { asked: true }
";

/// Applies `UPDATES` under `key` with `params`, and the labels everything measured under it carries.
async fn apply_labelled(rig: &Rig, key: &str, params: Value, labels: Value) -> Value {
    let asked = json!({ "source": UPDATES, "key": key, "params": params, "labels": labels, "wait": true });
    rig.send("/api/apply", asked).await
}

/// Applies `UPDATES` under `key` with `params`.
async fn apply(rig: &Rig, key: &str, params: Value) -> Value {
    apply_labelled(rig, key, params, json!({})).await
}

/// What the route answers `token` for `channel`.
async fn running(rig: &Rig, token: &str, channel: &str) -> (StatusCode, Value) {
    rig.call(Method::GET, &format!("/api/channels/{channel}/running"), Some(token), None).await
}

/// The keys of what is listed, in the order it is listed.
fn keys(answer: &Value) -> Vec<String> {
    let listed = answer["instances"].as_array().cloned().unwrap_or_default();
    listed.iter().map(|one| one["key"].as_str().unwrap_or_default().to_string()).collect()
}

#[tokio::test]
async fn a_channel_runs_what_says_it_posts_there_as_it_is_applied_again_with_its_labels_and_until_it_is_stopped()
 {
    let rig = Rig::new().await;
    for name in ["design", "quiet"] {
        rig.send("/api/channels", json!({ "name": name })).await;
    }
    let labels = json!({ "experiment": "weekly" });
    let first =
        apply_labelled(&rig, "updates", json!({ "channel": "#design", "every": "10m" }), labels.clone())
            .await;
    let (status, said) = running(&rig, &rig.token, "design").await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(keys(&said), ["updates"], "{said}");
    let listed = &said["instances"][0];
    assert_eq!(
        (&listed["blueprint"], &listed["params"]["every"]),
        (&json!("status-updates"), &json!("10m")),
        "{said}"
    );
    // Nothing says it posts in a channel beside it.
    assert_eq!(keys(&running(&rig, &rig.token, "quiet").await.1), Vec::<String>::new());

    // Applied again with other params and the labels it has, as its
    // settings are applied from the channel, it is the same thing, said
    // otherwise, and what is measured under it is still labelled so.
    assert_eq!(listed["labels"], labels, "{said}");
    apply_labelled(&rig, "updates", json!({ "channel": "#design", "every": "5m" }), listed["labels"].clone())
        .await;
    let said = running(&rig, &rig.token, "design").await.1;
    let listed = &said["instances"][0];
    assert_eq!((&listed["id"], &listed["params"]["every"]), (&first["instance"], &json!("5m")), "{said}");
    assert_eq!(listed["labels"], labels, "{said}");

    // Stopped, nothing runs there.
    let stopped = format!("/api/instances/{}/stop", listed["id"].as_str().unwrap());
    rig.send(&stopped, json!({})).await;
    assert_eq!(keys(&running(&rig, &rig.token, "design").await.1), Vec::<String>::new());
}

#[tokio::test]
async fn what_a_channel_runs_is_told_only_to_whoever_reads_the_channel_and_what_is_applied() {
    let rig = Rig::new().await;
    rig.send("/api/channels", json!({ "name": "design" })).await;
    let kept = rig.send("/api/channels", json!({ "name": "accounts", "kept": true })).await;
    let kept = kept["id"].as_str().unwrap().to_string();
    for (key, channel) in [("design-updates", "design"), ("accounts-updates", kept.as_str())] {
        apply(&rig, key, json!({ "channel": channel })).await;
    }
    assert_eq!(keys(&running(&rig, &rig.token, &kept).await.1), ["accounts-updates"]);
    let key = |name: &str, scopes: Value| {
        let rig = &rig;
        let asked = json!({ "name": name, "scopes": scopes });
        async move { rig.send("/api/credentials", asked).await["token"].as_str().unwrap().to_string() }
    };
    // A member reads what runs in an open channel, and of one kept from
    // them hears what they would of a channel that is not there.
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let member = crate::kept::person(&rig, "Mira").await.token;
    assert_eq!(keys(&running(&rig, &member, "design").await.1), ["design-updates"]);
    let (kept_from, refused) = running(&rig, &member, &kept).await;
    let (nowhere, missing) = running(&rig, &member, "ch_nonesuch").await;
    assert_eq!((kept_from, nowhere), (StatusCode::NOT_FOUND, StatusCode::NOT_FOUND), "{refused} {missing}");
    // Somebody who reads channels and not what is applied is told nothing of it.
    let reader = key("reader", json!(["channels:read"])).await;
    let (status, refused) = running(&rig, &reader, "design").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(!refused.to_string().contains("design-updates"), "{refused}");
}
