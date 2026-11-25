//! A page of a wiki deleted takes every page under it into the Trash, over
//! HTTP: by the route that deletes a record and by a
//! blueprint's `record.remove`, in one write and one entry, which the
//! Trash lists as the page with how many pages it holds; given back in one
//! write under its page, or as a wiki of its own where its page is gone,
//! which is said; deleted for good with every page's revisions and what
//! is attached to each; refused to a key that holds less; and kept by a
//! copy of the factory and by a project moved to a journal of its own.

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

/// A wiki, `Handbook`, with `Deploying` under it holding `To staging`,
/// which holds `Rolling back`: the ids of the last three.
async fn handbook(rig: &Rig, place: Value) -> (String, String, String) {
    let mut asked = json!({ "title": "Handbook" });
    asked.as_object_mut().unwrap().extend(place.as_object().cloned().unwrap_or_default());
    let handbook = id(&rig.send("/api/topics", asked).await);
    let deploying = id(&rig.send("/api/topics", json!({ "title": "Deploying", "under": handbook })).await);
    let staging = id(&rig.send("/api/topics", json!({ "title": "To staging", "under": deploying })).await);
    let back = id(&rig.send("/api/topics", json!({ "title": "Rolling back", "under": staging })).await);
    (deploying, staging, back)
}

/// The titles of the topics there are, each with the title of the page it is under.
async fn tree(rig: &Rig) -> Vec<(String, Option<String>)> {
    let listed = rig.read("/api/topics").await;
    let topics = listed["topics"].as_array().unwrap();
    let title = |id: &str| -> Option<String> {
        let found = topics.iter().find(|topic| topic["id"] == json!(id))?;
        Some(found["data"]["title"].as_str()?.to_string())
    };
    let mut tree: Vec<(String, Option<String>)> = topics
        .iter()
        .map(|topic| {
            let above = topic["data"]["parent"].as_str().and_then(title);
            (topic["data"]["title"].as_str().unwrap().to_string(), above)
        })
        .collect();
    tree.sort();
    tree
}

fn pair(title: &str, above: Option<&str>) -> (String, Option<String>) {
    (title.to_string(), above.map(str::to_string))
}

/// What is in the Trash for whoever holds `token`.
async fn trash(rig: &Rig, token: &str) -> Vec<Value> {
    let (status, body) = rig.call(Method::GET, "/api/trash", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["trash"].as_array().cloned().unwrap_or_default()
}

async fn revisions(rig: &Rig, page: &str) -> i64 {
    let page = page.to_string();
    let asked = move |connection: &mut rusqlite::Connection| {
        connection
            .query_row("SELECT count(*) FROM topic_revisions WHERE topic = ?1", [&page], |row| row.get(0))
    };
    rig.node().app.journal.db().call(asked).await.unwrap()
}

async fn attach(rig: &Rig, record: &str) -> String {
    let answer = rig
        .client
        .post(format!("{}/api/records/{record}/attachments?name=steps.txt", rig.url))
        .bearer_auth(&rig.token)
        .header("content-type", "text/plain")
        .body(b"run it twice".to_vec())
        .send()
        .await
        .unwrap();
    assert_eq!(answer.status(), StatusCode::OK);
    let kept: Value = answer.json().await.unwrap();
    id(&kept)
}

#[tokio::test]
async fn a_page_deleted_by_the_route_takes_every_page_under_it_into_one_entry_and_comes_back_with_them() {
    let rig = Rig::new().await;
    let (deploying, staging, back) = handbook(&rig, json!({})).await;
    let started = rig.send("/api/topics", json!({ "title": "Getting started", "under": "Handbook" })).await;
    let reads = key(&rig, &["records:read"]).await;
    let path = format!("/api/records/{deploying}");
    let (status, refused) = rig.call(Method::DELETE, &path, Some(&reads), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    let mark = rig.mark();
    let (status, gone) = rig.call(Method::DELETE, &path, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!(gone["with"], json!(2), "{gone}");
    assert!(gone["trash"].as_str().is_some_and(|trash| trash.starts_with("trash_")), "{gone}");
    assert_eq!(tree(&rig).await, [pair("Getting started", Some("Handbook")), pair("Handbook", None)]);
    for page in [&deploying, &staging, &back] {
        let (status, _) = rig.get(&format!("/api/topics/{page}")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{page} is no topic while it is in the Trash");
    }
    let said = rig.event("record.removed", |event| event.seq > mark && event.data["id"] == json!(back)).await;
    assert_eq!(
        (said.data["with"].clone(), said.data["trash"].clone()),
        (json!(deploying), gone["trash"].clone())
    );

    // One entry, said as the page with how many it holds; seen by a key that reads, which gives nothing back.
    let listed = trash(&rig, &reads).await;
    assert_eq!(listed.len(), 1, "{listed:?}");
    let entry = &listed[0];
    assert_eq!(
        (entry["thing"].clone(), entry["called"].clone(), entry["with"].clone()),
        (json!(deploying), json!("Deploying"), json!(2))
    );
    assert_eq!((entry["project"].clone(), entry["table"].clone()), (json!("@"), json!("topics")));
    assert_eq!(entry["may"], false);
    let restore = format!("/api/trash/{}/restore", entry["id"].as_str().unwrap());
    let (status, refused) = rig.call(Method::POST, &restore, Some(&reads), Some(json!({}))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(trash(&rig, &rig.token).await[0]["may"], true);

    let mark = rig.mark();
    let back_again = rig.send(&restore, json!({})).await;
    assert_eq!(back_again["restored"]["with"], json!(2), "{back_again}");
    assert!(back_again.get("said").is_none(), "where it was: {back_again}");
    assert_eq!(
        tree(&rig).await,
        [
            pair("Deploying", Some("Handbook")),
            pair("Getting started", Some("Handbook")),
            pair("Handbook", None),
            pair("Rolling back", Some("To staging")),
            pair("To staging", Some("Deploying")),
        ]
    );
    let restored = rig.event("trash.restored", |event| event.seq > mark).await;
    assert_eq!(restored.data["with"], json!(2));
    let read = rig.read(&format!("/api/topics/{deploying}")).await;
    assert_eq!(read["pages"][0]["title"], json!("To staging"), "{read}");
    assert_eq!(read["path"][0]["title"], json!("Handbook"));
    assert!(
        read["topic"]["position"].as_f64() < started["position"].as_f64(),
        "before the page made after it, where it stood"
    );
}

#[tokio::test]
async fn a_page_deleted_by_a_blueprint_takes_its_pages_and_given_back_after_its_page_is_a_wiki_of_its_own() {
    let rig = Rig::new().await;
    let (deploying, _, _) = handbook(&rig, json!({})).await;
    let remove = format!(
        "blueprint: prune\nsteps:\n  gone: {{ do: record.remove, with: {{ id: \"{deploying}\" }} }}\n"
    );
    let applied = rig.send("/api/apply", json!({ "source": remove, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(tree(&rig).await, [pair("Handbook", None)]);
    let entry = trash(&rig, &rig.token).await.remove(0);
    assert_eq!(entry["with"], json!(2), "{entry}");

    // Its page deleted after it: it comes back as a wiki of its own, with its pages under it.
    let handbook = rig.read("/api/topics/Handbook").await;
    let path = format!("/api/records/{}", handbook["topic"]["id"].as_str().unwrap());
    let (status, _) = rig.call(Method::DELETE, &path, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    let back = rig.send(&format!("/api/trash/{}/restore", entry["id"].as_str().unwrap()), json!({})).await;
    assert_eq!(
        back["said"],
        json!("The page `Deploying` was under is gone, so `Deploying` is a wiki of its own now."),
        "{back}"
    );
    assert_eq!(
        tree(&rig).await,
        [
            pair("Deploying", None),
            pair("Rolling back", Some("To staging")),
            pair("To staging", Some("Deploying"))
        ]
    );
    // And its page, given back after it, is a wiki with none of them under it.
    let last = trash(&rig, &rig.token).await.remove(0);
    assert_eq!(last["called"], json!("Handbook"));
    assert!(last.get("with").is_none(), "nothing went with it: {last}");
    rig.send(&format!("/api/trash/{}/restore", last["id"].as_str().unwrap()), json!({})).await;
    assert_eq!(tree(&rig).await.len(), 4);
}

#[tokio::test]
async fn a_page_deleted_for_good_takes_every_pages_revisions_and_attachments_with_it() {
    let rig = Rig::new().await;
    let (deploying, staging, back) = handbook(&rig, json!({})).await;
    for page in [&deploying, &back] {
        rig.call(
            Method::PATCH,
            &format!("/api/records/{page}"),
            Some(&rig.token),
            Some(json!({ "fields": { "body": "Run it." } })),
        )
        .await;
    }
    let attached = attach(&rig, &back).await;
    assert!(revisions(&rig, &back).await > 0);
    let (status, _) =
        rig.call(Method::DELETE, &format!("/api/records/{deploying}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(revisions(&rig, &back).await > 0, "kept while it is in the Trash, to come back with it");
    let entry = trash(&rig, &rig.token).await.remove(0);
    let forget = format!("/api/trash/{}", entry["id"].as_str().unwrap());
    let writes = key(&rig, &["records:read"]).await;
    let (status, refused) = rig.call(Method::DELETE, &forget, Some(&writes), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    let (status, gone) = rig.call(Method::DELETE, &forget, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert!(trash(&rig, &rig.token).await.is_empty());
    for page in [&deploying, &staging, &back] {
        assert_eq!(revisions(&rig, page).await, 0, "{page}'s revisions went with it");
    }
    let (status, _) = rig.get(&format!("/api/attachments/{attached}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "what was attached to a page under it went too");
    assert_eq!(tree(&rig).await, [pair("Handbook", None)]);
}

#[tokio::test]
async fn where_nothing_deleted_is_kept_a_page_goes_at_once_with_every_page_under_it() {
    let rig = Rig::new().await;
    rig.send("/api/system/trash", json!({ "days": 0 })).await;
    let (deploying, _, back) = handbook(&rig, json!({})).await;
    rig.call(
        Method::PATCH,
        &format!("/api/records/{back}"),
        Some(&rig.token),
        Some(json!({ "fields": { "body": "Run it." } })),
    )
    .await;
    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/records/{deploying}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!((gone["trash"].clone(), gone["with"].clone()), (Value::Null, json!(2)), "{gone}");
    assert!(trash(&rig, &rig.token).await.is_empty());
    assert_eq!(tree(&rig).await, [pair("Handbook", None)]);
    assert_eq!(revisions(&rig, &back).await, 0);
}

#[tokio::test]
async fn a_copy_of_the_factory_keeps_a_page_in_the_trash_with_the_pages_under_it_and_gives_them_back() {
    let shared = crate::snapshots::Shared::new();
    let prod = shared.factory("prod").await;
    let (deploying, _, back) = handbook(&prod, json!({})).await;
    let attached = attach(&prod, &back).await;
    let (status, _) =
        prod.call(Method::DELETE, &format!("/api/records/{deploying}"), Some(&prod.token), None).await;
    assert_eq!(status, StatusCode::OK);
    let to = shared.snapshot("prod-copy");
    crate::snapshots::taken(&prod, &to).await;
    let dev = shared.copy("dev", &to).await;
    let entry = trash(&dev, &dev.token).await.remove(0);
    assert_eq!((entry["called"].clone(), entry["with"].clone()), (json!("Deploying"), json!(2)), "{entry}");
    // A page made in the copy is numbered after them all, so that they come back under the ids they had.
    let next = dev.send("/api/topics", json!({ "title": "Next" })).await;
    assert!(id(&next) != back && id(&next) != deploying, "{next}");
    dev.send(&format!("/api/trash/{}/restore", entry["id"].as_str().unwrap()), json!({})).await;
    assert_eq!(tree(&dev).await.len(), 5);
    let (status, _) = dev.get(&format!("/api/attachments/{attached}")).await;
    assert_eq!(status, StatusCode::OK, "what was attached to a page under it came with the copy");
}

#[tokio::test]
async fn a_page_in_the_trash_with_pages_under_it_is_kept_by_a_project_moved_apart() {
    if crate::one_journal() {
        return;
    }
    let (mut rig, data) = crate::apart::rig("wikitrash-apart").await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.stop().await;
    crate::moved::from_before(&data);
    rig.start().await;
    let (deploying, _, _) = handbook(&rig, json!({ "project": "shop" })).await;
    let (status, _) =
        rig.call(Method::DELETE, &format!("/api/records/{deploying}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    rig.stop().await;
    let moved = api::moved::run(&data, Some("shop"), None, |_| {}).await.unwrap();
    assert_eq!(moved.len(), 1, "shop is moved to a journal of its own");
    rig.start().await;
    let entry = trash(&rig, &rig.token).await.remove(0);
    assert_eq!(
        entry["with"],
        json!(2),
        "kept with the factory's own, whatever project its pages are of: {entry}"
    );
    rig.send(&format!("/api/trash/{}/restore", entry["id"].as_str().unwrap()), json!({})).await;
    assert_eq!(tree(&rig).await.len(), 4);
    let read = rig.read("/api/topics/Rolling%20back").await;
    assert_eq!(read["topic"]["data"]["project"], json!("shop"), "{read}");
}
