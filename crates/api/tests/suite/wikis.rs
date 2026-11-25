//! Topics as wikis, pages in a tree, over HTTP: a page made
//! under another and moved under one, to the top and before another, each
//! said; refused to a key that only reads and where it cannot go; and the
//! topics a factory from before made read as wikis, each of its own, with
//! nothing written for it at a start or the next.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "helper", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

fn id(page: &Value) -> String {
    page["id"].as_str().unwrap().to_string()
}

/// The titles of the pages directly under `parent` (the wikis, for none),
/// in the order they stand.
async fn under(rig: &Rig, parent: Option<&str>) -> Vec<String> {
    let listed = rig.read("/api/topics").await;
    let mut pages: Vec<&Value> = listed["topics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|page| page["data"]["parent"].as_str() == parent)
        .collect();
    pages.sort_by(|one, other| {
        one["position"].as_f64().unwrap().total_cmp(&other["position"].as_f64().unwrap())
    });
    pages.iter().map(|page| page["data"]["title"].as_str().unwrap().to_string()).collect()
}

#[tokio::test]
async fn a_page_is_made_under_another_and_moved_under_one_to_the_top_and_before_another_each_said() {
    let rig = Rig::new().await;
    let handbook = id(&rig.send("/api/topics", json!({ "title": "Handbook" })).await);
    let deploying = rig.send("/api/topics", json!({ "title": "Deploying", "under": handbook })).await;
    assert_eq!(deploying["data"]["parent"], json!(handbook), "{deploying}");
    assert_eq!(deploying["data"]["workspace"], json!("ada-s-workspace"), "where its wiki is");
    let deploying = id(&deploying);
    let started =
        id(&rig.send("/api/topics", json!({ "title": "Getting started", "under": "handbook" })).await);
    let (status, both) = rig
        .post("/api/topics", json!({ "title": "Astray", "under": handbook, "workspace": "ada-s-workspace" }))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{both}");
    assert!(both.to_string().contains("say `under`, or where it is"), "{both}");
    assert_eq!(under(&rig, Some(&handbook)).await, ["Deploying", "Getting started"]);

    let mark = rig.mark();
    let moved = rig
        .send(&format!("/api/topics/{started}/move"), json!({ "under": handbook, "before": deploying }))
        .await;
    assert_eq!(moved["data"]["parent"], json!(handbook), "{moved}");
    assert_eq!(under(&rig, Some(&handbook)).await, ["Getting started", "Deploying"]);
    let said = rig.event("topic.moved", |event| event.seq > mark).await;
    assert_eq!(said.data["record"], json!(started));
    assert_eq!(
        (said.data["parent"].clone(), said.data["was"]["parent"].clone()),
        (json!(handbook), json!(handbook))
    );
    assert!(said.source.actor.as_deref().is_some_and(|actor| actor.starts_with("user:")), "{said:?}");

    let rolling = id(&rig.send("/api/topics", json!({ "title": "Rolling back", "under": deploying })).await);
    let read = rig.read(&format!("/api/topics/{rolling}")).await;
    let path: Vec<&str> =
        read["path"].as_array().unwrap().iter().map(|one| one["title"].as_str().unwrap()).collect();
    assert_eq!(path, ["Handbook", "Deploying"], "{read}");
    let held = rig.read(&format!("/api/topics/{deploying}")).await;
    assert_eq!(held["pages"], json!([{ "id": rolling, "title": "Rolling back", "status": null }]));

    let top = rig.send(&format!("/api/topics/{deploying}/move"), json!({ "under": null })).await;
    assert_eq!(top["data"]["parent"], Value::Null, "a wiki of its own: {top}");
    assert_eq!(under(&rig, None).await, ["Handbook", "Deploying"]);
    assert_eq!(under(&rig, Some(&deploying)).await, ["Rolling back"], "its page went with it");

    let (status, nowhere) = rig.post(&format!("/api/topics/{rolling}/move"), json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{nowhere}");
    assert!(nowhere.to_string().contains("say where it goes"), "{nowhere}");
    let itself = json!({ "under": rolling });
    let (status, refused) = rig.post(&format!("/api/topics/{deploying}/move"), itself).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused.to_string().contains("cannot go under itself or a page that is under it"), "{refused}");
}

#[tokio::test]
async fn a_key_that_only_reads_topics_moves_none_and_makes_none_under_another() {
    let rig = Rig::new().await;
    let handbook = id(&rig.send("/api/topics", json!({ "title": "Handbook" })).await);
    let page = id(&rig.send("/api/topics", json!({ "title": "Deploying", "under": handbook })).await);
    let reads = key(&rig, &["records:read"]).await;
    let (status, refused) = rig
        .call(Method::POST, &format!("/api/topics/{page}/move"), Some(&reads), Some(json!({ "under": null })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let asked = json!({ "title": "Rolling back", "under": handbook });
    let (status, refused) = rig.call(Method::POST, "/api/topics", Some(&reads), Some(asked)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, read) = rig.call(Method::GET, &format!("/api/topics/{page}"), Some(&reads), None).await;
    assert_eq!(status, StatusCode::OK, "it reads them: {read}");
    assert_eq!(read["path"][0]["title"], json!("Handbook"));
    assert_eq!(under(&rig, Some(&handbook)).await, ["Deploying"], "nothing moved");

    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let (status, moved) = rig
        .call(
            Method::POST,
            &format!("/api/topics/{page}/move"),
            Some(&member),
            Some(json!({ "under": null })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "a member who writes records moves one: {moved}");
}

/// The lines of `token`'s box of something said on a record.
async fn commented(rig: &Rig, token: &str) -> Vec<Value> {
    let (status, found) = rig.call(Method::GET, "/api/inbox?all=true", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    found["lines"].as_array().unwrap().iter().filter(|line| line["kind"] == "commented").cloned().collect()
}

#[tokio::test]
async fn somebody_named_on_a_page_is_told_with_where_the_page_is_now_and_a_wiki_says_none() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let nell = crate::kept::person(&rig, "Nell").await;
    let handbook = id(&rig.send("/api/topics", json!({ "title": "Handbook" })).await);
    let deploying = id(&rig.send("/api/topics", json!({ "title": "Deploying", "under": handbook })).await);
    let back = id(&rig.send("/api/topics", json!({ "title": "Rolling back", "under": deploying })).await);
    let mark = rig.mark();
    rig.send(&format!("/api/records/{back}/comments"), json!({ "body": "@nell which tag?" })).await;
    let told = nell.actor.clone();
    rig.event("inbox.told", move |event| event.seq > mark && event.data["owner"] == told.as_str()).await;
    let lines = commented(&rig, &nell.token).await;
    let on = &lines[0]["on_record"];
    assert_eq!(
        (on["title"].clone(), on["path"].clone()),
        (json!("Rolling back"), json!(["Handbook", "Deploying"])),
        "{on}"
    );

    // Moved, the line says where it is now: it is read as the line is read.
    rig.send(&format!("/api/topics/{back}/move"), json!({ "under": handbook })).await;
    assert_eq!(commented(&rig, &nell.token).await[0]["on_record"]["path"], json!(["Handbook"]));
    // And a wiki is where it is: nothing above it.
    rig.send(&format!("/api/topics/{back}/move"), json!({ "under": null })).await;
    let on = commented(&rig, &nell.token).await[0]["on_record"].clone();
    assert!(on.get("path").is_none(), "{on}");
}

/// Each topic's row as the database keeps it: id, what it holds, where it
/// stands and when it was written.
async fn rows(file: &std::path::Path) -> Vec<(String, String, f64, i64)> {
    let db = journal::Db::open(file).unwrap();
    let rows = db
        .call(|connection| {
            let mut asked = connection.prepare(
                "SELECT id, data, position, updated FROM records WHERE project = '@' AND collection = 'topics' ORDER BY id",
            )?;
            let rows = asked.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?;
            rows.collect()
        })
        .await
        .unwrap();
    db.close();
    db.closed().await;
    rows
}

#[tokio::test]
async fn the_topics_a_factory_from_before_made_are_each_a_wiki_of_its_own_with_nothing_written_for_it() {
    let mut rig = Rig::new().await;
    rig.send("/api/topics", json!({ "title": "Payment provider options" })).await;
    rig.send("/api/topics", json!({ "title": "Why the build is slow" })).await;
    let file = rig.kept_in("@");
    rig.stop().await;
    // As a program from before left them: a topic said nothing of a page above it.
    let db = journal::Db::open(&file).unwrap();
    let taken = db
        .call(|connection| {
            connection.execute(
                "UPDATE records SET data = json_remove(data, '$.parent') WHERE project = '@' AND collection = 'topics'",
                [],
            )
        })
        .await
        .unwrap();
    assert_eq!(taken, 2);
    db.close();
    db.closed().await;
    let before = rows(&file).await;
    assert!(before.iter().all(|(_, data, _, _)| !data.contains("parent")), "{before:?}");

    rig.start().await;
    let listed = rig.read("/api/topics").await;
    let topics = listed["topics"].as_array().unwrap();
    assert_eq!(topics.len(), 2);
    assert!(topics.iter().all(|topic| topic["data"]["parent"].is_null()), "each a wiki: {listed}");
    assert_eq!(
        under(&rig, None).await,
        ["Payment provider options", "Why the build is slow"],
        "in the order made"
    );
    let read = rig.read("/api/topics/Why%20the%20build%20is%20slow").await;
    assert_eq!((read["path"].clone(), read["pages"].clone()), (json!([]), json!([])), "{read}");
    rig.restart().await;
    rig.read("/api/topics").await;
    rig.stop().await;
    assert_eq!(rows(&file).await, before, "nothing was written for them at either start");

    rig.start().await;
    let page = rig.send("/api/topics", json!({ "title": "Linking", "under": "Why the build is slow" })).await;
    assert_eq!(page["data"]["workspace"], json!("ada-s-workspace"), "a page under one, where it is: {page}");
}
