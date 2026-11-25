//! A project and a table called something else for people (`PATCH
//! /api/projects/{id}`, `PUT /api/collections/{project}/{name}/title`), by
//! whoever may write the project's files, each said in the write that
//! keeps it, with what everything knows them by as it was.

use reqwest::{Method, StatusCode};
use serde_json::json;

use crate::Rig;

/// Shop with a table of runs, declared as called `Test runs`, and a diary kept private.
const TABLES: &str = r#"
blueprint: tables
permissions: [projects]
steps:
  runs:
    do: collection.ensure
    with:
      name: runs
      title: Test runs
      fields: { title: { type: text, required: true } }
  diary:
    do: collection.ensure
    with:
      name: diary
      reads: private:journal
      fields: { title: { type: text, required: true } }
"#;

async fn shop(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    let applied = rig.send("/api/apply", json!({ "source": TABLES, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "helper", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

async fn renamed(rig: &Rig, token: Option<&str>, id: &str, name: &str) -> (StatusCode, serde_json::Value) {
    let token = token.unwrap_or(&rig.token).to_string();
    rig.call(Method::PATCH, &format!("/api/projects/{id}"), Some(&token), Some(json!({ "name": name }))).await
}

#[tokio::test]
async fn a_project_is_called_something_else_by_whoever_may_write_its_files_its_id_as_it_was() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let (status, project) = renamed(&rig, None, "shop", "  Shop front ").await;
    assert_eq!(status, StatusCode::OK, "{project}");
    assert_eq!((project["id"].clone(), project["name"].clone()), (json!("shop"), json!("Shop front")));
    let said = rig.event("project.renamed", |event| event.data["project"] == "shop").await;
    assert_eq!(said.data["name"], "Shop front");
    assert_eq!(said.data["was"], "Shop");
    assert_eq!(rig.read("/api/projects/shop").await["name"], "Shop front");

    // Called what another project is, whatever its capitals, or nothing, it is refused and nothing is written.
    let (status, refused) = renamed(&rig, None, "shop", "depot").await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(
        refused["error"]["message"].as_str().unwrap().contains("another project is called depot"),
        "{refused}"
    );
    let (status, refused) = renamed(&rig, None, "shop", "   ").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert_eq!(rig.read("/api/projects/shop").await["name"], "Shop front");

    // A key that only reads projects is refused, and one that writes them is not.
    let reader = key(&rig, &["projects:read"]).await;
    let (status, refused) = renamed(&rig, Some(&reader), "shop", "Mine").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let writer = key(&rig, &["projects"]).await;
    let (status, project) = renamed(&rig, Some(&writer), "shop", "Shop").await;
    assert_eq!((status, project["name"].clone()), (StatusCode::OK, json!("Shop")));
}

#[tokio::test]
async fn a_table_is_called_something_else_for_everybody_and_keeps_it_when_it_is_declared_again() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let read = || async { rig.read("/api/collections?project=shop").await };
    let runs = |all: &serde_json::Value| {
        all["collections"].as_array().unwrap().iter().find(|one| one["name"] == "runs").cloned().unwrap()
    };
    assert_eq!(runs(&read().await)["title"], "Test runs", "what its blueprint calls it");
    let title = "/api/collections/shop/runs/title";
    let (status, table) =
        rig.call(Method::PUT, title, Some(&rig.token), Some(json!({ "title": "Runs of tests" }))).await;
    assert_eq!(status, StatusCode::OK, "{table}");
    assert_eq!(
        (table["name"].clone(), table["title_chosen"].clone()),
        (json!("runs"), json!("Runs of tests"))
    );
    let said = rig.event("collection.titled", |event| event.data["name"] == "runs").await;
    assert_eq!(said.data["title"], "Runs of tests");
    // Declared again, what somebody called it stands; and taken away, what is declared is said again.
    let declared = TABLES.replace("title: Test runs", "title: Runs we ran");
    let applied =
        rig.send("/api/apply", json!({ "source": declared, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(runs(&read().await)["title_chosen"], "Runs of tests");
    assert_eq!(runs(&read().await)["title"], "Runs we ran");
    let (status, table) = rig.call(Method::DELETE, title, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{table}");
    assert!(table.get("title_chosen").is_none(), "{table}");

    // A key that holds less is refused: one that does not write projects, and one that does not read the diary.
    let reader = key(&rig, &["projects:read", "records"]).await;
    let (status, refused) =
        rig.call(Method::PUT, title, Some(&reader), Some(json!({ "title": "Mine" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let helper = key(&rig, kernel::principal::AGENT_SCOPES).await;
    let diary = "/api/collections/shop/diary/title";
    let (status, refused) =
        rig.call(Method::PUT, diary, Some(&helper), Some(json!({ "title": "Mine" }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    // A board keeps its words.
    rig.send("/api/apply", json!({ "blueprint": "std/board", "project": "shop", "wait": true })).await;
    let board = "/api/collections/shop/board/title";
    let (status, refused) =
        rig.call(Method::PUT, board, Some(&rig.token), Some(json!({ "title": "Cards" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
}
