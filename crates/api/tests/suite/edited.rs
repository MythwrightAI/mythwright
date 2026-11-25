//! An agent changed whole, as the Agents page's Edit… changes one: one
//! added by hand said again with everything its form says under the
//! text's own params, what it is told and its model emptied among them,
//! and one this factory does not start given what it holds and its
//! project in one request; each answered with what the change comes to
//! for the work it has in hand (`said`). And a fleet's row whose model
//! and what it is told are emptied says nothing, which its agents take
//! as none: the provider's usual model, and nothing told of its own.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// What the list of agents says of `name`.
async fn listed(rig: &Rig, name: &str) -> Value {
    let all = rig.read("/api/agents").await;
    let found = all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name);
    found.cloned().unwrap_or_else(|| panic!("no agent called {name}: {all}"))
}

fn said(answer: &Value) -> &str {
    answer["said"].as_str().unwrap_or_else(|| panic!("nothing said of the change: {answer}"))
}

#[tokio::test]
async fn an_agent_added_by_hand_is_set_up_again_whole_and_what_it_is_told_and_its_model_are_emptied_there() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let first = json!({ "name": "helper", "project": "shop", "instructions": "Be brief.", "model": "small" });
    let added = rig.send("/api/agents", first).await;
    assert_eq!((&added["model"], &added["said"]), (&json!("small"), &Value::Null), "a new one: {added}");

    // Everything its form says, under the text's own params, as the form sends it.
    let params = json!({
        "instructions": "", "model": "", "permission": "read", "holds": ["board:read", "search"],
        "wall": "on", "reads": [], "changes": [], "git": "read", "web": "off", "outlives": "off",
        "secrets": [], "signins": [],
    });
    let body = json!({ "name": "helper", "project": "shop", "params": params });
    let changed = rig.send("/api/agents", body.clone()).await;
    assert_eq!((&changed["instructions"], &changed["model"]), (&json!(""), &Value::Null), "{changed}");
    assert_eq!(
        (&changed["permission_mode"], &changed["scopes"]),
        (&json!("read"), &json!(["board:read", "search"]))
    );
    let words = said(&changed);
    assert!(words.starts_with("`helper` is changed: "), "{words}");
    assert!(words.contains("its program is started as it is now set up from its next turn"), "{words}");
    assert!(words.contains("it is told what it is now told from its next turn"), "{words}");
    assert!(
        words.contains("the new list from the next thing it asks, in work it has in hand too"),
        "{words}"
    );
    let listed = listed(&rig, "helper").await;
    assert_eq!((&listed["instructions"], &listed["model"]), (&json!(""), &Value::Null), "{listed}");
    let text = rig.read(&format!("/api/instances/{}", listed["instance"].as_str().unwrap())).await;
    let instance = if text["instance"].is_object() { &text["instance"] } else { &text };
    assert_eq!(
        (&instance["params"]["instructions"], &instance["params"]["model"]),
        (&json!(""), &json!("")),
        "{text}"
    );

    // Saved as it is, nothing changed, and that is said.
    let again = rig.send("/api/agents", body).await;
    assert_eq!(said(&again), "Nothing of `helper` changed.");

    // Moved by the same form, it works in the other project, under none of its environments.
    rig.send("/api/projects", json!({ "name": "Cafe" })).await;
    let moved =
        rig.send("/api/agents", json!({ "name": "helper", "project": "cafe", "params": params })).await;
    assert_eq!((&moved["project"], &moved["environment"]), (&json!("cafe"), &json!("")), "{moved}");
    assert!(said(&moved).contains("it works in `cafe` from its next turn"), "{moved}");
}

#[tokio::test]
async fn an_agent_this_factory_does_not_start_is_given_what_it_holds_and_its_project_in_one_request() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/projects", json!({ "name": "Cafe" })).await;
    let invited = json!({ "name": "walls", "project": "shop", "scopes": ["board:read"] });
    let (status, answer) = rig.call(Method::POST, "/api/agent/invite", Some(&rig.token), Some(invited)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    let both = json!({ "scopes": ["board:read", "search"], "project": "cafe" });
    let (status, changed) = rig.call(Method::PATCH, "/api/agents/walls", Some(&rig.token), Some(both)).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    assert_eq!((&changed["scopes"], &changed["project"]), (&json!(["board:read", "search"]), &json!("cafe")));
    let words = said(&changed);
    assert!(
        words.contains("the new list from the next thing it asks") && words.contains("`cafe`"),
        "{words}"
    );
    assert!(!words.contains("its program"), "nothing is started for it: {words}");
    let is = listed(&rig, "walls").await;
    assert_eq!((&is["scopes"], &is["project"]), (&json!(["board:read", "search"]), &json!("cafe")), "{is}");

    // Saying neither, or something else beside them, is refused, saying what it takes.
    let (status, refused) =
        rig.call(Method::PATCH, "/api/agents/walls", Some(&rig.token), Some(json!({ "model": "big" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused.to_string().contains("or by both"), "{refused}");
    let beside = json!({ "project": "shop", "model": "big" });
    let (status, refused) =
        rig.call(Method::PATCH, "/api/agents/walls", Some(&rig.token), Some(beside)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert_eq!(listed(&rig, "walls").await["project"], "cafe", "nothing of it changed");
}

#[tokio::test]
async fn a_fleet_rows_model_and_what_it_is_told_emptied_say_nothing_and_its_agent_goes_by_the_usual() {
    let (rig, _at) = crate::fleets::rig().await;
    crate::fleets::shop(&rig, json!({})).await;
    let id = crate::fleets::row(&rig, "agents", "worker").await;
    let path = format!("/api/records/{id}");
    let mut seen = Vec::new();
    for fields in [json!({ "model": "big", "told": "Work slowly." }), json!({ "model": "", "told": "" })] {
        let before = rig.mark();
        let (status, answer) =
            rig.call(Method::PATCH, &path, Some(&rig.token), Some(json!({ "fields": fields }))).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        crate::taken::by_fleet(&rig, before).await;
        let worker = listed(&rig, "shop-worker-1").await;
        seen.push((
            worker["model"].clone(),
            worker["instructions"].as_str().unwrap_or_default().trim().to_string(),
        ));
    }
    assert_eq!(seen[0], (json!("big"), "Work slowly.".to_string()), "what the row said: {seen:?}");
    // Emptied, the row says nothing: the provider's usual model and nothing of its own to be told, never what it said before.
    assert_eq!(seen[1], (Value::Null, String::new()), "what the row says once emptied: {seen:?}");
}
