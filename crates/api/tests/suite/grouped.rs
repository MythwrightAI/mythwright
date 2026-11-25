//! Groups of people and agents through the daemon (`people::groups`):
//! what a group holds held by its people and agents at their next
//! request, and gone with their place in it; nobody given by one what
//! whoever put them there does not hold; a key that holds less changing
//! nothing; an agent putting itself in no group; and an agent taken away
//! leaving each it was in.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::said;
use crate::kept::{key, person};

/// The credential an agent's program reads each time it asks the factory anything.
fn token_of(rig: &Rig, agent: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{agent}/token"))).unwrap().trim().to_string()
}

/// What `token` holds as things stand, as the daemon says it.
async fn holds(rig: &Rig, token: &str) -> Vec<String> {
    let (status, me) = rig.call(Method::GET, "/api/me", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    me["principal"]["scopes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| one.as_str().unwrap().to_string())
        .collect()
}

fn group<'a>(listed: &'a Value, name: &str) -> &'a Value {
    listed["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|one| one["name"] == name)
        .unwrap_or_else(|| panic!("{listed}"))
}

#[tokio::test]
async fn what_a_group_holds_is_held_by_its_people_and_agents_from_their_next_request_and_goes_with_their_place()
 {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(&rig, "Grace").await;
    rig.send("/api/agents", json!({ "name": "helper" })).await;
    let helper = token_of(&rig, "helper");
    for token in [&grace.token, &helper] {
        assert!(!holds(&rig, token).await.contains(&"environments:secrets".to_string()));
    }

    // A key that holds less makes and changes none, and reads who is in each without what each holds.
    let less = key(&rig, &rig.token, &["channels:read"]).await;
    let asked = json!({ "name": "deploys", "holds": ["environments:secrets"] });
    let (status, refused) = rig.call(Method::POST, "/api/groups", Some(&less), Some(asked.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let made = rig.send("/api/groups", asked).await;
    let id = made["group"]["id"].as_str().unwrap().to_string();
    let members = format!("/api/groups/{id}/members");
    let (status, refused) =
        rig.call(Method::POST, &members, Some(&less), Some(json!({ "who": "grace" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // A person by her handle, an agent by its name.
    rig.send(&members, json!({ "who": "grace" })).await;
    rig.send("/api/groups/deploys/members", json!({ "who": "helper" })).await;
    for token in [&grace.token, &helper] {
        assert!(holds(&rig, token).await.contains(&"environments:secrets".to_string()), "held at once");
    }
    let (status, listed) = rig.call(Method::GET, "/api/groups", Some(&less), None).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(group(&listed, "deploys")["members"], json!(["agent:helper", grace.actor]));
    assert!(group(&listed, "deploys").get("holds").is_none(), "what it holds is the manager's: {listed}");
    let listed = rig.read("/api/groups").await;
    assert_eq!(group(&listed, "deploys")["holds"], json!(["environments:secrets"]));
    let accounts = rig.read("/api/accounts").await;
    let theirs = accounts["accounts"].as_array().unwrap().iter().find(|one| one["name"] == "Grace").unwrap();
    assert_eq!(theirs["groups"], json!([id]));
    assert!(theirs["holds"].as_array().unwrap().contains(&json!("environments:secrets")), "{theirs}");
    let joined = rig.event("identity.group.joined", |event| event.data["member"] == "agent:helper").await;
    assert_eq!(joined.data["name"], "deploys");

    // Nobody, and an agent that is not there, is put in no group.
    for (who, status) in [("nobody-at-all", StatusCode::BAD_REQUEST), ("agent:ghost", StatusCode::NOT_FOUND)]
    {
        let (got, refused) =
            rig.call(Method::POST, &members, Some(&rig.token), Some(json!({ "who": who }))).await;
        assert_eq!(got, status, "{who}: {refused}");
    }

    // Her key made with it holds it; out of the group, neither she nor it does.
    let hers = key(&rig, &grace.token, &["environments:secrets", "projects"]).await;
    let (status, out) = rig.call(Method::DELETE, &format!("{members}/grace"), Some(&rig.token), None).await;
    assert_eq!((status, &out["keys"]), (StatusCode::OK, &json!(1)), "{out}");
    for token in [&grace.token, &hers] {
        assert!(!holds(&rig, token).await.contains(&"environments:secrets".to_string()));
    }
    assert!(holds(&rig, &hers).await.contains(&"projects".to_string()), "what it held besides it stays");

    // Narrowed, and taken away: the agent holds it no longer either.
    let (status, narrowed) = rig
        .call(Method::PATCH, &format!("/api/groups/{id}"), Some(&rig.token), Some(json!({ "holds": [] })))
        .await;
    assert_eq!(status, StatusCode::OK, "{narrowed}");
    assert!(!holds(&rig, &helper).await.contains(&"environments:secrets".to_string()));
    let (status, gone) = rig.call(Method::DELETE, &format!("/api/groups/{id}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert!(rig.read("/api/groups").await["groups"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn nobody_gives_by_a_group_what_they_do_not_hold_and_an_agent_puts_itself_in_none() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let grace = person(&rig, "Grace").await;
    let id = grace.actor.trim_start_matches("user:");
    let (status, given) =
        rig.call(Method::PUT, &format!("/api/accounts/{id}/given/admin"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{given}");
    rig.send("/api/groups", json!({ "name": "deploys", "holds": ["environments:secrets"] })).await;
    // She manages the factory and does not hold the environments' secrets.
    let asked = json!({ "name": "hers", "holds": ["environments:secrets"] });
    let (status, refused) = rig.call(Method::POST, "/api/groups", Some(&grace.token), Some(asked)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("nobody gives what they do not hold"), "{refused}");
    let owner = rig.read("/api/me").await["actor"].as_str().unwrap().to_string();
    let put = json!({ "who": owner });
    let (status, refused) =
        rig.call(Method::POST, "/api/groups/deploys/members", Some(&grace.token), Some(put)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // An agent that manages the factory changes a group it is not in, and never its own place.
    let scopes = ["admin", "agents", "channels", "environments:secrets"];
    rig.send("/api/agents", json!({ "name": "boss", "scopes": scopes })).await;
    let boss = token_of(&rig, "boss");
    let (status, refused) = rig
        .call(Method::POST, "/api/groups/deploys/members", Some(&boss), Some(json!({ "who": "boss" })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("never changes a group it is in"), "{refused}");
    rig.send("/api/agents", json!({ "name": "helper" })).await;
    let (status, put) = rig
        .call(Method::POST, "/api/groups/deploys/members", Some(&boss), Some(json!({ "who": "helper" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{put}");

    // Taken away, an agent leaves each group it was in: a later agent of its name is no heir to it.
    let removed = rig.send("/api/agents/helper/remove", json!({})).await;
    assert_eq!(removed["removed"], true, "{removed}");
    let listed = rig.read("/api/groups").await;
    assert_eq!(group(&listed, "deploys")["members"], json!([]), "{listed}");
    let left = rig.event("identity.group.left", |event| event.data["member"] == "agent:helper").await;
    assert_eq!(left.data["name"], "deploys");
    rig.send("/api/agents", json!({ "name": "helper" })).await;
    assert!(!holds(&rig, &token_of(&rig, "helper")).await.contains(&"environments:secrets".to_string()));
}
