//! The documents a channel keeps to hand, through the daemon's routes
//! (`talk::docs`, `agents::docs`, `routes/channeldocs.rs`): a library's
//! record and a project's file added by whoever may write in the channel
//! and may read what they add, listed to everybody who reads it as each
//! may read what an entry names, taken away again, each change said with
//! the channel's other events; and the list of a channel kept to its
//! members its members' alone, kept in a copy of the factory, taken into
//! the Trash with the channel and given back with it.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, key, person, say};
use crate::snapshots::{Shared, taken};

async fn add(rig: &Rig, token: &str, channel: &str, doc: &str) -> (StatusCode, Value) {
    rig.call(Method::POST, &format!("/api/channels/{channel}/docs"), Some(token), Some(json!({ "doc": doc })))
        .await
}

async fn remove(rig: &Rig, token: &str, channel: &str, doc: &str) -> (StatusCode, Value) {
    let path = format!("/api/channels/{channel}/docs?doc={}", doc.replace('/', "%2F").replace(':', "%3A"));
    rig.call(Method::DELETE, &path, Some(token), None).await
}

async fn listed(rig: &Rig, token: &str, channel: &str) -> (StatusCode, Value) {
    rig.call(Method::GET, &format!("/api/channels/{channel}/docs"), Some(token), None).await
}

/// Each entry as `doc state title path`, in the order they were added.
fn lines(answer: &Value) -> Vec<String> {
    answer["docs"]
        .as_array()
        .unwrap_or_else(|| panic!("a list: {answer}"))
        .iter()
        .map(|one| {
            let said = |key: &str| one[key].as_str().unwrap_or("-").to_string();
            format!("{} {} {} {}", said("doc"), said("state"), said("title"), said("path"))
        })
        .collect()
}

/// A project `shop` with its library, and a file of its own at its top.
async fn shop(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["docs"] })).await;
    let body = Some(json!({ "path": "notes.md", "content": "# Notes\n\nThe quince is ripe.\n" }));
    let (status, wrote) = rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), body).await;
    assert_eq!(status, StatusCode::OK, "{wrote}");
}

#[tokio::test]
async fn a_channels_documents_are_added_and_taken_away_by_whoever_may_write_there_and_listed_as_each_may_read_them()
 {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    shop(&rig).await;
    let mira = person(&rig, "Mira").await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    // Whoever may only read the channel adds nothing; nor does somebody who may write there and read nothing of the project.
    let reads = key(&rig, &mira.token, &["channels:read"]).await;
    let talks = key(&rig, &mira.token, &["channels"]).await;
    let (status, refused) = add(&rig, &reads, "general", "shop-docs:overview").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    for doc in ["shop-docs:overview", "shop/notes.md"] {
        let (status, refused) = add(&rig, &talks, "general", doc).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{doc}: {refused}");
    }
    // What is not there, and what is no document or no file of a project, is refused in a sentence.
    for (doc, wanted) in [
        ("shop-docs:nowhere", StatusCode::NOT_FOUND),
        ("shop/missing.md", StatusCode::NOT_FOUND),
        ("shop/../elsewhere.md", StatusCode::BAD_REQUEST),
        ("", StatusCode::BAD_REQUEST),
    ] {
        let (status, refused) = add(&rig, &mira.token, "general", doc).await;
        assert_eq!(status, wanted, "{doc}: {refused}");
    }
    let (_, none) = listed(&rig, &mira.token, "general").await;
    assert!(lines(&none).is_empty(), "nothing refused was kept: {none}");

    // Somebody who may write there and read both adds a document of the library and a file of the project.
    let mark = rig.mark();
    let (status, now) = add(&rig, &mira.token, "general", "shop-docs:overview").await;
    assert_eq!(status, StatusCode::OK, "{now}");
    let said = rig.event("channel.docs", move |event| event.seq > mark).await;
    assert_eq!(
        (said.data["doc"].as_str(), said.data["did"].as_str()),
        (Some("shop-docs:overview"), Some("added"))
    );
    assert_eq!(said.data["by"], mira.actor.as_str());
    let (status, now) = rig
        .call(
            Method::POST,
            "/api/channels/general/docs",
            Some(&mira.token),
            Some(json!({ "project": "shop", "path": "notes.md" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{now}");
    assert_eq!(
        lines(&now),
        ["shop-docs:overview there Shop: overview docs/overview.md", "shop/notes.md there notes.md notes.md"]
    );
    // Added again, it is there once and nothing more is said.
    let again = rig.mark();
    assert_eq!(add(&rig, &mira.token, "general", "shop-docs:overview").await.0, StatusCode::OK);
    say(&rig, &rig.token, "general", json!({ "body": "a mark" })).await;
    rig.event("channel.message", move |event| event.seq > again).await;
    let more = journal::EventQuery { after: again, topics: "channel.docs".into(), ..Default::default() };
    assert!(rig.said(more).await.is_empty());

    // The same list for everybody who reads the channel, each entry as they may read it: kept from a key that reads only channels, with nothing of it said.
    let (status, theirs) = listed(&rig, &reads, "general").await;
    assert_eq!(status, StatusCode::OK, "{theirs}");
    assert_eq!(lines(&theirs), ["shop-docs:overview kept - -", "shop/notes.md kept - notes.md"]);
    assert!(!theirs.to_string().contains("Shop: overview"), "{theirs}");
    let (_, owners) = listed(&rig, &rig.token, "general").await;
    assert_eq!(lines(&owners), lines(&now));

    // A file that goes is said to be gone, and stays listed until it is taken away.
    let (status, gone) =
        rig.call(Method::DELETE, "/api/projects/shop/file?path=notes.md", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let (_, now) = listed(&rig, &mira.token, "general").await;
    assert_eq!(lines(&now)[1], "shop/notes.md gone - notes.md");

    // Taken away by whoever may write there, and said; not by whoever may only read it.
    assert_eq!(remove(&rig, &reads, "general", "shop/notes.md").await.0, StatusCode::FORBIDDEN);
    let mark = rig.mark();
    let (status, now) = remove(&rig, &mira.token, "general", "shop/notes.md").await;
    assert_eq!(status, StatusCode::OK, "{now}");
    let said = rig.event("channel.docs", move |event| event.seq > mark).await;
    assert_eq!(
        (said.data["doc"].as_str(), said.data["did"].as_str()),
        (Some("shop/notes.md"), Some("removed"))
    );
    assert_eq!(lines(&now), ["shop-docs:overview there Shop: overview docs/overview.md"]);
    // Taken away when it is not there, nothing is.
    assert_eq!(remove(&rig, &mira.token, "general", "shop/notes.md").await.0, StatusCode::OK);

    // An archived channel is given back before anything is added to it.
    rig.send("/api/channels/general/archive", json!({})).await;
    let (status, refused) = add(&rig, &mira.token, "general", "shop-docs:architecture").await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
}

#[tokio::test]
async fn a_private_channels_documents_are_its_members_alone_and_go_with_it_into_a_copy_and_the_trash() {
    let shared = Shared::new();
    let rig = shared.factory("prod").await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    shop(&rig).await;
    let (mira, otto) = (person(&rig, "Mira").await, person(&rig, "Otto").await);
    let made = json!({ "name": "Pay review", "kept": true });
    let (_, kept) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(made)).await;
    let id = kept["id"].as_str().unwrap().to_string();
    rig.send("/api/channels", json!({ "name": "lobby" })).await;
    let mut to_otto = Told::of(&rig, &otto.token).await;

    let (status, now) = add(&rig, &mira.token, &id, "shop-docs:architecture").await;
    assert_eq!(status, StatusCode::OK, "{now}");
    // Somebody who is not in it reads no list, adds nothing, and is told nothing of it.
    assert_eq!(listed(&rig, &otto.token, &id).await.0, StatusCode::NOT_FOUND);
    assert_eq!(add(&rig, &otto.token, &id, "shop-docs:overview").await.0, StatusCode::NOT_FOUND);
    say(&rig, &rig.token, "lobby", json!({ "body": "after the greengage" })).await;
    to_otto.until("after the greengage").await;
    assert!(!to_otto.heard.contains("channel.docs") && !to_otto.heard.contains("shop-docs:architecture"));

    // A copy of the factory keeps it with the channel.
    let copied = shared.snapshot("docsed");
    taken(&rig, &copied).await;
    let dev = shared.copy("dev", &copied).await;
    let (status, there) = listed(&dev, &dev.token, &id).await;
    assert_eq!(status, StatusCode::OK, "{there}");
    assert_eq!(lines(&there), ["shop-docs:architecture there Shop: architecture docs/architecture.md"]);

    // Into the Trash with the channel, nothing of it left behind, and given back with it.
    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/channels/{id}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let left = rig
        .node()
        .app
        .journal
        .db()
        .call(|connection| {
            connection.query_row("SELECT COUNT(*) FROM channel_docs", [], |row| row.get::<_, i64>(0))
        })
        .await
        .unwrap();
    assert_eq!(left, 0);
    let trash = gone["trash"].as_str().unwrap();
    rig.send(&format!("/api/trash/{trash}/restore"), json!({})).await;
    let (status, back) = listed(&rig, &mira.token, &id).await;
    assert_eq!(status, StatusCode::OK, "{back}");
    assert_eq!(lines(&back), ["shop-docs:architecture there Shop: architecture docs/architecture.md"]);
}
