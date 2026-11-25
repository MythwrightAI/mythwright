//! A table made from the side panel (`std/table`): applied for a project as whoever asked, under the key
//! `table-<name>`, and never declared over a table the project has that
//! something else declared (`anew` of `collection.ensure`).

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// `std/table` applied for Shop under its usual key, as whoever `token` is.
fn made(name: &str, layout: &str) -> Value {
    let params = json!({ "name": name, "layout": layout });
    json!({ "blueprint": "std/table", "project": "shop", "key": format!("table-{name}"), "params": params, "wait": true })
}

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "helper", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// Shop's tables, by name.
async fn tables(rig: &Rig) -> Vec<Value> {
    rig.read("/api/collections?project=shop").await["collections"].as_array().cloned().unwrap_or_default()
}

async fn table(rig: &Rig, name: &str) -> Option<Value> {
    tables(rig).await.into_iter().find(|one| one["name"] == name)
}

#[tokio::test]
async fn a_table_made_from_the_side_panel_is_applied_as_whoever_asked_and_refused_without_blueprints_run() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;

    // A member, who may write projects and apply: the table is applied as her, and not as the factory's owner.
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let nell = crate::kept::person(&rig, "Nell").await;
    let (status, applied) =
        rig.call(Method::POST, "/api/apply", Some(&nell.token), Some(made("reading", "list"))).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let instance = applied["instance"].as_str().unwrap().to_string();
    let read = rig.read(&format!("/api/instances/{instance}")).await;
    assert_eq!(read["instance"]["key"], "table-reading", "{read}");
    assert_eq!(read["instance"]["blueprint"], "std/table", "{read}");
    let ensured = rig.event("collection.ensured", |event| event.data["name"] == "reading").await;
    assert_eq!(ensured.source.actor.as_deref(), Some(nell.actor.as_str()), "declared as whoever asked");
    let reading = table(&rig, "reading").await.expect("the table is there");
    assert_eq!(reading["views"][0]["kind"], "list", "{reading}");
    assert_eq!(reading["fields"][0]["name"], "title", "{reading}");

    // A table kept from her is refused her in words that say nothing of it.
    let diary = r#"
blueprint: diary
permissions: [projects]
steps:
  diary:
    do: collection.ensure
    with: { name: diary, reads: "private:journal", fields: { title: { type: text } } }
"#;
    rig.send("/api/apply", json!({ "source": diary, "project": "shop", "wait": true })).await;
    let (_, refused) =
        rig.call(Method::POST, "/api/apply", Some(&nell.token), Some(made("diary", "list"))).await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    let words = refused.to_string();
    assert!(words.contains("cannot be made in") && !words.contains("already"), "{refused}");

    // Somebody who may write projects but not apply is refused, and nothing is made.
    let writer = key(&rig, &["projects", "records"]).await;
    let (status, refused) =
        rig.call(Method::POST, "/api/apply", Some(&writer), Some(made("notes", "table"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused.to_string().contains("blueprints:run"), "{refused}");
    assert!(table(&rig, "notes").await.is_none(), "nothing made for them");
}

#[tokio::test]
async fn a_table_made_anew_is_the_same_done_twice_under_its_own_key_and_never_declared_over_another() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;

    let first = rig.send("/api/apply", made("reading", "table")).await;
    assert_eq!(first["finished"]["status"], "ok", "{first}");
    rig.send("/api/collections/shop/reading/records", json!({ "fields": { "title": "The Waves" } })).await;
    // Applied again under its own key, with another way of laying it out: the same table, and its row kept.
    let again = rig.send("/api/apply", made("reading", "gallery")).await;
    assert_eq!(again["finished"]["status"], "ok", "{again}");
    assert_eq!(again["instance"], first["instance"], "the same instance: {again}");
    let read = rig.read("/api/collections/shop/reading/records").await;
    assert_eq!(read.to_string().matches("The Waves").count(), 1, "{read}");

    // Under another key, a name the project has a table of is refused, and that table is as it was.
    let mut other = made("reading", "list");
    other["key"] = json!("table-reading-2");
    let refused = rig.send("/api/apply", other).await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    assert!(
        refused.to_string().contains("has a table called `reading` already"),
        "said in the sentence that refuses it: {refused}"
    );
    let still = table(&rig, "reading").await.unwrap();
    assert_eq!(still["views"][0]["kind"], "gallery", "{still}");

    // Nor is the board, which another blueprint declared, declared over.
    let board = rig.send("/api/apply", made("board", "list")).await;
    assert_eq!(board["finished"]["status"], "failed", "{board}");
    let kept = table(&rig, "board").await.unwrap();
    assert_eq!(kept["model"], "card", "the board as it was: {kept}");
}

#[tokio::test]
async fn a_table_made_anew_is_never_declared_over_a_fleets_table() {
    let rig = crate::lanes::rig().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    crate::lanes::fleet(&rig, json!({})).await;
    let was = table(&rig, "agents").await.expect("the fleet's agents");

    let refused = rig.send("/api/apply", made("agents", "table")).await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    assert!(refused.to_string().contains("which something else declared"), "{refused}");
    let now = table(&rig, "agents").await.unwrap();
    assert_eq!(now["fields"], was["fields"], "the fleet's table as it was");
    assert_eq!(now["views"], was["views"], "the fleet's table as it was");
}
