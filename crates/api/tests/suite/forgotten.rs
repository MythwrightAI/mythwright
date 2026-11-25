//! A project deleted for good takes what was its own above its files with
//! it: its tables with their records and what was
//! said of each, what of it was in the Trash, what was applied for it and
//! still ran, and its agents. A project made again under its name finds
//! none of it.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// What is kept of `shop` in every journal's records, by table: the rows
/// of each table that tells rows apart by project.
async fn rows_of_shop(rig: &Rig) -> Vec<(String, i64)> {
    let mut found = Vec::new();
    for journal in rig.node().app.journal.every() {
        let counted: Vec<(String, i64)> = journal
            .db()
            .call(|connection| {
                let mut counted = Vec::new();
                for (table, whose) in [
                    ("collections", "project = 'shop'"),
                    ("records", "project = 'shop'"),
                    ("record_history", "record LIKE 'shop-%'"),
                    ("trash", "project = 'shop'"),
                ] {
                    let sql = format!("SELECT count(*) FROM {table} WHERE {whose}");
                    let count: i64 = connection.query_row(&sql, [], |row| row.get(0)).unwrap_or(0);
                    if count > 0 {
                        counted.push((table.to_string(), count));
                    }
                }
                Ok(counted)
            })
            .await
            .unwrap();
        found.extend(counted);
    }
    found
}

/// What asks for every event said on `topic`.
fn said(topic: &str) -> journal::EventQuery {
    journal::EventQuery { topics: topic.into(), limit: 100, ..Default::default() }
}

/// The agents there are, by name.
async fn agents(rig: &Rig) -> Vec<String> {
    let all = rig.read("/api/agents").await;
    let names = all["agents"].as_array().into_iter().flatten();
    let mut names: Vec<String> =
        names.map(|agent| agent["name"].as_str().unwrap_or_default().to_string()).collect();
    names.sort();
    names
}

/// What runs for `shop`, by key.
async fn running_for_shop(rig: &Rig) -> Vec<String> {
    let filter = conductor::InstanceFilter {
        live_only: true,
        project: Some("shop".to_string()),
        limit: 100,
        ..conductor::InstanceFilter::default()
    };
    let live = rig.node().app.engine.instances(filter).await.unwrap();
    live.into_iter().map(|one| one.key).collect()
}

#[tokio::test]
async fn a_project_deleted_for_good_takes_its_tables_what_runs_for_it_and_its_agents_with_it() {
    let rig = Rig::new().await;
    let files = rig.dir.join("data").join("projects").join("shop");
    // From the Trash, and where the factory keeps nothing that is deleted.
    for days in [json!(30), json!(0)] {
        rig.send("/api/system/trash", json!({ "days": days })).await;
        rig.send("/api/projects", json!({ "name": "shop" })).await;
        // A board, a card said something on and one in the Trash, and a table of its own.
        rig.board("shop").await;
        let first = rig.send("/api/board/shop/items", json!({ "title": "Hang the gate" })).await;
        let first = first["id"].as_str().unwrap().to_string();
        rig.send(&format!("/api/records/{first}/comments"), json!({ "body": "Twice, carefully." })).await;
        let second = rig.send("/api/board/shop/items", json!({ "title": "Paint it" })).await;
        let second = second["id"].as_str().unwrap().to_string();
        let path = format!("/api/board/items/{second}");
        let (status, gone) = rig.call(Method::DELETE, &path, Some(&rig.token), None).await;
        assert_eq!(status, StatusCode::OK, "{gone}");
        let table = json!({
            "blueprint": "std/table", "project": "shop", "key": "table-reading", "wait": true,
            "params": { "name": "reading" },
        });
        let applied = rig.send("/api/apply", table).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
        rig.send("/api/collections/shop/reading/records", json!({ "fields": { "title": "Dune" } })).await;
        // Two that a role brings up, and one added by hand.
        let role = json!({
            "blueprint": "std/role", "project": "shop", "key": "workers", "wait": true,
            "params": { "role": "worker", "name": "shop-worker", "count": 2 },
        });
        let applied = rig.send("/api/apply", role).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
        rig.send("/api/agents", json!({ "name": "helper", "project": "shop" })).await;
        assert!(rows_of_shop(&rig).await.iter().any(|(table, _)| table == "records"), "{days}");
        assert_eq!(agents(&rig).await, ["helper", "shop-worker-1", "shop-worker-2"], "{days}");
        assert!(running_for_shop(&rig).await.contains(&"workers".to_string()), "{days}");

        let (status, forgot) = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await;
        assert_eq!(status, StatusCode::OK, "{forgot}");
        if let Some(id) = forgot["trash"].as_str() {
            // Kept while it is in the Trash, so that it is given back whole.
            assert!(rows_of_shop(&rig).await.iter().any(|(table, _)| table == "records"));
            assert_eq!(agents(&rig).await.len(), 3);
            assert!(!running_for_shop(&rig).await.is_empty());
            let (status, gone) =
                rig.call(Method::DELETE, &format!("/api/trash/{id}"), Some(&rig.token), None).await;
            assert_eq!(status, StatusCode::OK, "{gone}");
        }
        assert_eq!(rows_of_shop(&rig).await, Vec::<(String, i64)>::new(), "{days}");
        assert_eq!(agents(&rig).await, Vec::<String>::new(), "{days}");
        assert_eq!(running_for_shop(&rig).await, Vec::<String>::new(), "{days}");
        let removed = rig.said(said("agent.slot.removed")).await;
        assert!(removed.iter().any(|event| event.data["slot"] == "shop-worker-2"), "{days}");
        let tables = rig.said(said("collection.forgotten")).await;
        assert!(tables.iter().any(|event| event.data["name"] == "reading"), "{days}");
        assert!(tables.iter().any(|event| event.data["name"] == "board"), "{days}");
        // Taken away, as its owner would, so that it is a project made anew.
        std::fs::remove_dir_all(&files).unwrap();

        // A project added again under its name finds none of it.
        rig.send("/api/projects", json!({ "name": "shop" })).await;
        let (status, missing) =
            rig.call(Method::GET, "/api/collections/shop/reading/records", Some(&rig.token), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");
        assert_eq!(rows_of_shop(&rig).await, Vec::<(String, i64)>::new(), "{days}");
        let trash = rig.read("/api/trash").await;
        let of_shop = |entry: &&Value| entry["project"] == "shop" && entry["kind"] != "project";
        assert_eq!(trash["trash"].as_array().unwrap().iter().filter(of_shop).count(), 0, "{trash}");
        let (status, forgot) = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await;
        assert_eq!(status, StatusCode::OK, "{forgot}");
        if let Some(id) = forgot["trash"].as_str() {
            rig.call(Method::DELETE, &format!("/api/trash/{id}"), Some(&rig.token), None).await;
        }
        std::fs::remove_dir_all(&files).unwrap();
    }
}

/// A credential that holds `scopes`.
async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn whoever_deletes_a_project_for_good_holds_what_stopping_what_runs_for_it_and_taking_its_agents_away_take()
 {
    let rig = Rig::new().await;
    // Writes projects and runs nothing; a member, who does not manage the factory.
    let writes = key(&rig, &["projects"]).await;
    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    // From the Trash, and where the factory keeps nothing that is deleted.
    for days in [json!(30), json!(0)] {
        rig.send("/api/system/trash", json!({ "days": days })).await;
        rig.send("/api/projects", json!({ "name": "shop" })).await;
        rig.board("shop").await;
        rig.send("/api/board/shop/items", json!({ "title": "Hang the gate" })).await;
        let role = json!({
            "blueprint": "std/role", "project": "shop", "key": "workers", "wait": true,
            "params": { "role": "worker", "name": "shop-worker", "count": 1 },
        });
        let applied = rig.send("/api/apply", role).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
        let running = running_for_shop(&rig).await;
        assert!(running.contains(&"workers".to_string()), "{days}");
        let rows = rows_of_shop(&rig).await;
        // Where it is kept, it is put there by its owner, who may: deleting it for good is asked.
        let trashed = match days == json!(30) {
            true => {
                let (status, forgot) =
                    rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await;
                assert_eq!(status, StatusCode::OK, "{forgot}");
                forgot["trash"].as_str().map(str::to_string)
            }
            false => None,
        };
        let path = match &trashed {
            Some(id) => format!("/api/trash/{id}"),
            None => "/api/projects/shop".to_string(),
        };
        let (status, refused) = rig.call(Method::DELETE, &path, Some(&writes), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{days} {refused}");
        let said = refused.to_string();
        assert!(said.contains("blueprints:run") && said.contains("std/role"), "{said}");
        let (status, refused) = rig.call(Method::DELETE, &path, Some(&member), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{days} {refused}");
        let said = refused.to_string();
        assert!(said.contains("shop-worker-1") && said.contains("admin"), "{said}");
        // Nothing was deleted, stopped or taken away.
        assert_eq!(running_for_shop(&rig).await, running, "{days}");
        assert_eq!(rows_of_shop(&rig).await, rows, "{days}");
        assert_eq!(agents(&rig).await, ["shop-worker-1"], "{days}");
        let projects = rig.read("/api/projects").await.to_string();
        assert_eq!(projects.contains("\"shop\""), trashed.is_none(), "{projects}");
        if let Some(id) = &trashed {
            let trash = rig.read("/api/trash").await.to_string();
            assert!(trash.contains(id.as_str()), "{trash}");
        }
        // Whoever manages the factory may.
        let (status, gone) = rig.call(Method::DELETE, &path, Some(&rig.token), None).await;
        assert_eq!(status, StatusCode::OK, "{gone}");
        assert_eq!(agents(&rig).await, Vec::<String>::new(), "{days}");
        assert_eq!(running_for_shop(&rig).await, Vec::<String>::new(), "{days}");
        std::fs::remove_dir_all(rig.dir.join("data").join("projects").join("shop")).unwrap();
    }
}
