//! What an agent may ask of the factory is changed by saying what it
//! holds, whole: by itself, for one that is in no row of a fleet, with
//! nothing else of it changed; by whoever holds what they give; and it
//! holds the new list from the next thing it asks.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

const USUAL: &[&str] =
    &["projects", "channels", "board", "records", "blueprints:read", "agents:read", "fleet:read", "search"];

/// Something anybody who may apply a blueprint can apply.
const HELLO: &str = "blueprint: hello\nsteps:\n  rest: { do: wait, with: { for: 0s } }\n";

fn message(answer: &Value) -> &str {
    answer["error"]["message"].as_str().unwrap_or_default()
}

/// An agent as the factory lists it.
async fn agent(rig: &Rig, name: &str) -> Value {
    let listed = rig.read("/api/agents").await;
    let found = listed["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name).cloned();
    found.unwrap_or_else(|| panic!("no agent called {name}: {listed}"))
}

/// The credential an agent's program reads each time it asks the factory anything.
fn token(rig: &Rig, name: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{name}/token"))).unwrap().trim().to_string()
}

async fn applies(rig: &Rig, token: &str) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/apply", Some(token), Some(json!({ "source": HELLO, "wait": true }))).await
}

#[tokio::test]
async fn an_agent_given_leave_to_apply_blueprints_applies_one_from_the_next_thing_it_asks_and_one_it_was_taken_from_is_refused()
 {
    let rig = Rig::new().await;
    rig.send("/api/agents", json!({ "name": "helper", "instructions": "Be brief.", "role": "scout" })).await;
    let before = agent(&rig, "helper").await;
    assert_eq!(before["scopes"], json!(USUAL), "{before}");
    let first = token(&rig, "helper");
    let (status, refused) = applies(&rig, &first).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(message(&refused).contains("needs `blueprints:run`"), "{refused}");

    // Told what it holds, with that in it, it holds that, and nothing
    // else of it is another thing than it was.
    let mut more: Vec<&str> = USUAL.iter().copied().filter(|scope| *scope != "blueprints:read").collect();
    more.push("blueprints");
    let given = json!({ "scopes": more });
    let (status, now) = rig.call(Method::PATCH, "/api/agents/helper", Some(&rig.token), Some(given)).await;
    assert_eq!(status, StatusCode::OK, "{now}");
    let after = agent(&rig, "helper").await;
    assert_eq!(after["scopes"], json!(more), "{after}");
    let rest = |agent: &Value| {
        let mut agent = agent.as_object().unwrap().clone();
        agent.retain(|name, _| !["scopes", "updated"].contains(&name.as_str()));
        agent
    };
    assert_eq!(rest(&before), rest(&after), "only what it holds changed");

    // Its program reads its credential each time it asks: the one it had
    // is nobody's any more, and the one it now reads holds the new list,
    // in a piece of work it has in hand as in its next.
    let second = token(&rig, "helper");
    assert_ne!(first, second);
    assert_eq!(applies(&rig, &first).await.0, StatusCode::UNAUTHORIZED, "what it held before is withdrawn");
    let (status, applied) = applies(&rig, &second).await;
    assert_eq!((status, &applied["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{applied}");

    // Taken from it again, it is refused again.
    let (status, back) = rig
        .call(Method::PATCH, "/api/agents/helper", Some(&rig.token), Some(json!({ "scopes": USUAL })))
        .await;
    assert_eq!(status, StatusCode::OK, "{back}");
    assert_eq!(applies(&rig, &second).await.0, StatusCode::UNAUTHORIZED);
    let (status, refused) = applies(&rig, &token(&rig, "helper")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(message(&refused).contains("needs `blueprints:run`"), "{refused}");

    // One that is stopped stays stopped when only what it holds is said,
    // which adding it again would not leave it.
    let (status, _) = rig.call(Method::DELETE, "/api/agents/helper", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, told) = rig
        .call(Method::PATCH, "/api/agents/helper", Some(&rig.token), Some(json!({ "scopes": more })))
        .await;
    assert_eq!((status, &told["active"]), (StatusCode::OK, &json!(false)), "{told}");
    assert_eq!(agent(&rig, "helper").await["scopes"], json!(more));
    let again = rig.send("/api/agents", json!({ "name": "helper", "scopes": USUAL })).await;
    assert_eq!(
        (&again["active"], &again["scopes"], &again["role"]),
        (&json!(true), &json!(USUAL), &json!("scout"))
    );

    // Nothing but what it holds is said this way, and of nobody who is not there.
    let (status, refused) = rig
        .call(Method::PATCH, "/api/agents/helper", Some(&rig.token), Some(json!({ "role": "worker" })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(message(&refused).starts_with("an agent is changed here by `scopes`"), "{refused}");
    assert_eq!(agent(&rig, "helper").await["role"], "scout");
    let (status, nobody) = rig
        .call(Method::PATCH, "/api/agents/nobody", Some(&rig.token), Some(json!({ "scopes": USUAL })))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{nobody}");
    assert!(message(&nobody).contains("there is no agent called `nobody`"), "{nobody}");
    assert!(
        rig.read("/api/agents").await["agents"].as_array().unwrap().iter().all(|one| one["name"] != "nobody")
    );
}

#[tokio::test]
async fn nobody_gives_an_agent_what_they_do_not_hold_by_either_way_of_saying_what_it_holds() {
    let rig = Rig::new().await;
    rig.send("/api/agents", json!({ "name": "helper" })).await;
    let key = |scopes: Value| {
        let rig = &rig;
        async move {
            let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
            made["token"].as_str().unwrap().to_string()
        }
    };
    // One who may change agents, and so apply what an agent is set up by
    // (`std/agent`), and holds nothing else of blueprints.
    let mut holds: Vec<&str> = USUAL.to_vec();
    holds.extend(["agents", "blueprints:run"]);
    let arranger = key(json!(holds)).await;
    let mut more: Vec<&str> = USUAL.to_vec();
    more.push("blueprints");
    for (method, path, body) in [
        (Method::PATCH, "/api/agents/helper", json!({ "scopes": more })),
        (Method::POST, "/api/agents", json!({ "name": "helper", "scopes": more })),
        (Method::POST, "/api/agents", json!({ "name": "another", "scopes": more })),
    ] {
        let (status, refused) = rig.call(method.clone(), path, Some(&arranger), Some(body)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {refused}");
        assert!(
            message(&refused).contains("cannot give an agent `blueprints`, not holding it"),
            "{method} {path}: {refused}"
        );
    }
    assert_eq!(agent(&rig, "helper").await["scopes"], json!(USUAL), "nothing was changed");
    // What they do hold they give, and take away.
    let less = json!({ "scopes": ["board:read", "search"] });
    let (status, told) = rig.call(Method::PATCH, "/api/agents/helper", Some(&arranger), Some(less)).await;
    assert_eq!((status, &told["scopes"]), (StatusCode::OK, &json!(["board:read", "search"])), "{told}");

    // One who may not change agents says nothing of what one holds.
    let member = key(json!(USUAL)).await;
    let (status, refused) = rig
        .call(Method::PATCH, "/api/agents/helper", Some(&member), Some(json!({ "scopes": ["search"] })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(message(&refused).contains("needs `agents:write`"), "{refused}");
    assert_eq!(agent(&rig, "helper").await["scopes"], json!(["board:read", "search"]));
}
