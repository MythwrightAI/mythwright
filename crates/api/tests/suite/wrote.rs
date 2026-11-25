//! Who added a record, as its history says it: where a blueprint's step
//! added it, the factory's bot, by the name it has as the line is read,
//! and where the step was told whose work it added it from, that agent
//! after; never whoever applied the blueprint, whose authority it was
//! added with all the same, and never the blueprint, which is kept with
//! the line as detail (`through`). A record somebody added says them.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::Rig;
use crate::fleets::{add, add_row, rig, row, shop};

const ARCHITECT: &str = "shop-architect-1";

/// The first line of a record's history: what it says, and who it names.
/// It is written a moment after what it says, which is waited for.
pub(crate) async fn first(rig: &Rig, id: &str) -> (String, String) {
    let record = id.to_string();
    rig.event("collection.history", move |event| event.data["record"] == record.as_str()).await;
    let history = rig.read(&format!("/api/records/{id}/history")).await;
    let line = &history["history"][0];
    let said = |name: &str| line[name].as_str().unwrap_or_else(|| panic!("{history}")).to_string();
    (said("said"), said("who"))
}

/// Writes a plan into the project through the factory, with `token`, and
/// waits for the card its one ticket came to: its id.
async fn plan(rig: &Rig, token: &str, path: &str, ticket: &str) -> String {
    let from = rig.mark();
    let content = format!("What there is to do.\n\n```tickets\n[{{ \"title\": \"{ticket}\" }}]\n```\n");
    let (status, answer) = rig
        .call(
            Method::PUT,
            "/api/projects/shop/file",
            Some(token),
            Some(json!({ "path": path, "content": content })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let ticket = ticket.to_string();
    let added = rig
        .event("board.item.created", move |event| event.seq > from && event.data["title"] == ticket.as_str())
        .await;
    added.data["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_card_a_plan_added_says_the_bot_as_it_is_called_now_and_the_agent_that_wrote_the_plan_and_one_a_person_added_says_the_person()
 {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    let architect =
        json!({ "title": "architect", "role": "architect", "adapter": "critic", "permission": "read" });
    add_row(&rig, "agents", architect).await;

    // The architect writes a plan through the factory: the card its ticket
    // came to was added by the plan's blueprint, from what it wrote, and
    // the event says so beside who lent the authority.
    let token = std::fs::read_to_string(rig.dir.join(format!("data/slots/{ARCHITECT}/token"))).unwrap();
    let from = rig.mark();
    let planned = plan(&rig, token.trim(), "docs/plans/cart.md", "The cart's model").await;
    let wrote = format!("conductor added it to draft from what {ARCHITECT} wrote");
    assert_eq!(first(&rig, &planned).await, (wrote, "conductor".into()));
    let id_ = planned.clone();
    let created = rig
        .event("board.item.created", move |event| event.seq > from && event.data["id"] == id_.as_str())
        .await;
    assert_eq!(
        (&created.data["through"], &created.data["wrote"]),
        (&json!("std/plan"), &json!(format!("agent:{ARCHITECT}"))),
        "{:?}",
        created.data
    );
    assert_eq!(created.source.actor.as_deref(), created.data["by"].as_str());
    assert!(created.data["by"].as_str().is_some_and(|by| by.starts_with("user:")), "{:?}", created.data);

    // A plan a person wrote names no agent: the blueprint added the card,
    // which is the bot's doing.
    let till = plan(&rig, &rig.token, "docs/plans/till.md", "The till").await;
    assert_eq!(first(&rig, &till).await, ("conductor added it to draft".into(), "conductor".into()));
    let line = &rig.read(&format!("/api/records/{till}/history")).await["history"][0];
    assert_eq!(line["data"]["through"], "std/plan", "which blueprint is kept with the line: {line}");

    // A card somebody added is theirs.
    let id = add(&rig, "The shelves", &[]).await;
    assert_eq!(first(&rig, &id).await, ("Ada added it to draft".into(), "Ada".into()));

    // So is any record a blueprint added: the fleet's first rows.
    let worker = row(&rig, "agents", "worker").await;
    assert_eq!(first(&rig, &worker).await, ("conductor added it".into(), "conductor".into()));

    // The bot called something else, every line of what a blueprint's step
    // did says the name it has now, written before or after; a person's
    // says the person still.
    let (status, answer) = rig
        .call(Method::POST, "/api/system/speaker", Some(&rig.token), Some(json!({ "name": "foreman" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let now = format!("foreman added it to draft from what {ARCHITECT} wrote");
    assert_eq!(first(&rig, &planned).await, (now, "foreman".into()));
    assert_eq!(first(&rig, &worker).await, ("foreman added it".into(), "foreman".into()));
    assert_eq!(first(&rig, &id).await, ("Ada added it to draft".into(), "Ada".into()));
}
