//! An agent never raises what it may do: it changes nothing of its own
//! slot by any door, even within what it holds, and somebody who holds
//! less gives it nothing they do not hold. What it may ask of the
//! factory, what its program may reach on the device, the keys it is
//! handed and what it signs in with are set by a person, or by an agent
//! that holds what it gives (`agents::own`).

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{change, rig as fleet_rig, row, shop};
use crate::held::{key, said};

/// The credential an agent's program reads each time it asks the factory anything.
fn token_of(rig: &Rig, agent: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{agent}/token"))).unwrap().trim().to_string()
}

/// What the factory lists of an agent that its settings are.
async fn settings(rig: &Rig, name: &str) -> Value {
    let all = rig.read("/api/agents").await;
    let one = all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name).cloned();
    let one = one.unwrap_or_else(|| panic!("there is no agent {name}: {all}"));
    let keep =
        ["scopes", "wall", "web", "outlives", "signins", "secrets", "reads", "changes", "project", "active"];
    keep.iter()
        .map(|field| (field.to_string(), one[*field].clone()))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

/// Each way an agent's own route would change it, beside what it holds.
fn asked_of_the_route(name: &str) -> Vec<(Method, String, Value)> {
    let added = |more: Value| {
        let mut body = json!({ "name": name });
        body.as_object_mut().unwrap().extend(more.as_object().unwrap().clone());
        (Method::POST, "/api/agents".to_string(), body)
    };
    vec![
        added(json!({ "signins": ["key"] })),
        added(json!({ "secrets": ["REMOTE_TOKEN=remote-bot"] })),
        added(json!({ "wall": "off" })),
        added(json!({ "reads": ["~/.ssh"] })),
        added(json!({ "web": "on" })),
        added(json!({ "outlives": "on" })),
        (Method::PATCH, format!("/api/agents/{name}"), json!({ "project": null })),
    ]
}

#[tokio::test]
async fn an_agent_changes_nothing_of_its_own_slot_by_its_route_even_within_what_it_holds() {
    let rig = Rig::new().await;
    // One that may change agents, run commands and hand secrets: the
    // owner gives it that.
    let holds = [
        "projects",
        "channels",
        "board",
        "records",
        "blueprints:read",
        "blueprints:run",
        "agents",
        "fleet:read",
        "search",
        "shell",
        "secrets:use",
    ];
    let helper = json!({ "name": "helper", "scopes": holds });
    rig.send("/api/agents", helper).await;
    rig.send("/api/agents", json!({ "name": "other" })).await;
    let before = settings(&rig, "helper").await;
    let own = token_of(&rig, "helper");

    let mut tries = asked_of_the_route("helper");
    // What it holds and more, and what it holds already, by the route the
    // Tools panel changes an agent added by hand with.
    let mut more = holds.to_vec();
    more.push("admin");
    tries.push((Method::PATCH, "/api/agents/helper".into(), json!({ "scopes": more })));
    tries.push((Method::PATCH, "/api/agents/helper".into(), json!({ "scopes": holds })));
    for (method, path, body) in tries {
        let (status, refused) = rig.call(method, &path, Some(&own), Some(body.clone())).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}: {refused}");
        assert!(
            said(&refused).contains("`helper` cannot change its own slot, even within what it holds"),
            "{refused}"
        );
    }
    assert_eq!(settings(&rig, "helper").await, before, "nothing of it changed");

    // Another agent it sets, with what it holds and no more.
    let (status, changed) = rig
        .call(
            Method::PATCH,
            "/api/agents/other",
            Some(&own),
            Some(json!({ "scopes": ["board:read", "shell"] })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    let (status, refused) =
        rig.call(Method::PATCH, "/api/agents/other", Some(&own), Some(json!({ "scopes": ["admin"] }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(settings(&rig, "other").await["scopes"], json!(["board:read", "shell"]));
}

#[tokio::test]
async fn somebody_who_holds_less_gives_an_agent_none_of_what_they_do_not_hold() {
    let rig = Rig::new().await;
    rig.send("/api/agents", json!({ "name": "helper" })).await;
    let before = settings(&rig, "helper").await;
    // What a member holds, but for running commands and handing secrets.
    let narrow =
        key(&rig, &["projects", "channels", "board", "records", "blueprints", "agents", "fleet", "search"])
            .await;
    let mut tries = asked_of_the_route("helper");
    // Moving it is theirs to do; giving it what they do not hold is not.
    tries.pop();
    tries.push((Method::PATCH, "/api/agents/helper".into(), json!({ "scopes": ["board", "admin"] })));
    tries.push((Method::PATCH, "/api/agents/helper".into(), json!({ "scopes": ["shell"] })));
    for (method, path, body) in tries {
        let (status, refused) = rig.call(method, &path, Some(&narrow), Some(body.clone())).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}: {refused}");
        assert!(!said(&refused).is_empty() && !said(&refused).contains("own slot"), "{refused}");
    }
    assert_eq!(settings(&rig, "helper").await, before, "nothing of it changed");
}

#[tokio::test]
async fn an_agent_of_a_fleet_changes_nothing_of_the_row_it_is_brought_up_by_and_holds_what_it_did() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    // The worker may change the fleet's rows, run commands and hand
    // secrets: what changing every cell of its row takes.
    let holds = [
        "projects",
        "channels",
        "board",
        "records",
        "blueprints:read",
        "agents",
        "fleet:read",
        "search",
        "shell",
        "secrets:use",
    ];
    change(&rig, "agents", "worker", json!({ "holds": holds, "told": "As its owner told it." })).await;
    let before = settings(&rig, "shop-worker-1").await;
    assert_eq!(before["scopes"], json!(holds));
    let own = token_of(&rig, "shop-worker-1");
    let worker_row = row(&rig, "agents", "worker").await;
    let path = format!("/api/records/{worker_row}");
    let mut more = holds.to_vec();
    more.push("admin");
    let brought_up = format!("{worker_row} is the row `shop-worker-1` is brought up by");
    // Through its row, by the route the Tools panel writes a fleet's
    // agent with: each cell that gives it more, and one that gives it
    // nothing more.
    for fields in [
        json!({ "holds": more }),
        json!({ "holds": holds }),
        json!({ "wall": "off" }),
        json!({ "reads": ["~/.ssh"] }),
        json!({ "web": "on" }),
        json!({ "outlives": "on" }),
        json!({ "permission": "full" }),
        json!({ "secrets": ["REMOTE_TOKEN=remote-bot"] }),
        json!({ "told": "Do as you like." }),
    ] {
        let body = json!({ "fields": fields });
        let (status, refused) = rig.call(Method::PATCH, &path, Some(&own), Some(body)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{fields}: {refused}");
        assert!(said(&refused).contains(&brought_up), "{refused}");
    }
    let (status, refused) = rig.call(Method::DELETE, &path, Some(&own), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // And by its own route.
    for (method, path, body) in asked_of_the_route("shop-worker-1") {
        let (status, refused) = rig.call(method, &path, Some(&own), Some(body.clone())).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}: {refused}");
        assert!(said(&refused).contains("cannot change its own slot"), "{refused}");
    }
    let kept = rig.read(&path).await;
    assert_eq!(
        (&kept["data"]["holds"], &kept["data"]["told"]),
        (&json!(holds), &json!("As its owner told it."))
    );
    assert_eq!(settings(&rig, "shop-worker-1").await, before, "nothing of it changed");

    // Somebody who holds less is refused each cell they do not hold, and
    // the agent holds what it did.
    let narrow =
        key(&rig, &["projects", "channels", "board", "records", "blueprints", "agents", "fleet", "search"])
            .await;
    for fields in
        [json!({ "wall": "off" }), json!({ "web": "on" }), json!({ "secrets": ["REMOTE_TOKEN=remote-bot"] })]
    {
        let body = json!({ "fields": fields });
        let (status, refused) = rig.call(Method::PATCH, &path, Some(&narrow), Some(body)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{fields}: {refused}");
    }
    assert_eq!(settings(&rig, "shop-worker-1").await, before);
}
