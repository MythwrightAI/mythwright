//! Who a card is given to, and the workspace it is in.
//!
//! A card is given to an agent of its own workspace or to one that is no
//! project's; an agent of a project of another workspace is refused, in
//! a sentence, by the route every way of giving a card comes to. It is
//! asked of the giving alone: nothing else of a card is held to it, and
//! nobody is kept from reading anything.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

async fn give(rig: &Rig, card: &str, to: Value) -> (StatusCode, Value) {
    let path = format!("/api/board/items/{card}");
    rig.call(Method::PATCH, &path, Some(&rig.token), Some(json!({ "assignee": to }))).await
}

async fn card_of(rig: &Rig, card: &str) -> Value {
    let all = rig.read("/api/board/shop").await;
    all["items"].as_array().unwrap().iter().find(|one| one["id"] == card).cloned().unwrap()
}

async fn given_to(rig: &Rig, card: &str) -> Value {
    card_of(rig, card).await["assignee"].clone()
}

#[tokio::test]
async fn a_card_is_given_to_an_agent_of_its_own_workspace_or_of_no_project_and_to_none_of_another_workspaces()
{
    let rig = Rig::new().await;
    // Two projects in the usual workspace and one in another, each with
    // an agent, and an agent that is no project's.
    rig.send("/api/workspaces", json!({ "name": "Press" })).await;
    for (name, workspace) in [("Shop", None), ("Site", None), ("Print", Some("press"))] {
        rig.send("/api/projects", json!({ "name": name, "workspace": workspace })).await;
    }
    let board = json!({ "blueprint": "std/board", "project": "shop", "wait": true });
    assert_eq!(rig.send("/api/apply", board).await["finished"]["status"], "ok");
    for (name, project) in
        [("shop-hand", Some("shop")), ("site-hand", Some("site")), ("print-hand", Some("print"))]
    {
        rig.send("/api/agents", json!({ "name": name, "project": project })).await;
    }
    rig.send("/api/agents", json!({ "name": "rover" })).await;
    let card = rig.send("/api/board/shop/items", json!({ "title": "Hang the sign" })).await;
    let card = card["id"].as_str().unwrap().to_string();

    // Its own project's, another project's of its workspace, the
    // factory's own, a person, and a name that is no agent's: each as it
    // always was.
    for to in
        ["agent:shop-hand", "agent:site-hand", "agent:rover", "user:somebody", "agent:nobody-by-this-name"]
    {
        let (status, answer) = give(&rig, &card, json!(to)).await;
        assert_eq!(status, StatusCode::OK, "{to}: {answer}");
        assert_eq!(given_to(&rig, &card).await, to);
    }

    // An agent of another workspace's project is refused, in a sentence
    // that says why and where to look, and the card is whose it was.
    let (status, answer) = give(&rig, &card, json!("agent:print-hand")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    let why = answer["error"]["message"].as_str().unwrap_or_default();
    assert!(
        why.contains("`print-hand` works in `print`, which is in another workspace than `shop`")
            && why.contains("agent list"),
        "{why}"
    );
    assert_eq!(given_to(&rig, &card).await, "agent:nobody-by-this-name");

    // Its project put in the card's workspace, it is given the card; and
    // put back in the other, the card that names it is still changed in
    // its other cells, given to it again as it is, and given to nobody.
    let put = |workspace: &'static str| {
        let rig = &rig;
        async move {
            let to = Some(json!({ "workspace": workspace }));
            let (status, answer) =
                rig.call(Method::PUT, "/api/projects/print/workspace", Some(&rig.token), to).await;
            assert_eq!(status, StatusCode::OK, "{answer}");
        }
    };
    put("ada-s-workspace").await;
    assert_eq!(give(&rig, &card, json!("agent:print-hand")).await.0, StatusCode::OK);
    put("press").await;
    let path = format!("/api/board/items/{card}");
    let both = json!({ "title": "Hang the new sign", "assignee": "agent:print-hand" });
    let (status, answer) = rig.call(Method::PATCH, &path, Some(&rig.token), Some(both)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(card_of(&rig, &card).await["title"], "Hang the new sign");
    assert_eq!(given_to(&rig, &card).await, "agent:print-hand");
    assert_eq!(give(&rig, &card, Value::Null).await.0, StatusCode::OK);
    assert_eq!(give(&rig, &card, json!("agent:print-hand")).await.0, StatusCode::BAD_REQUEST);

    // Nobody is kept from reading by it: the card and the agents are
    // read as they always were.
    assert!(rig.read("/api/agents").await["agents"].as_array().unwrap().len() >= 4);
}
