//! A comment on a few words of a document: what it is about (the words
//! quoted, a few either side and the headings over them) is kept with the
//! comment, in its line of the document's history, and said with it. The
//! file holds nothing of it. Each part is checked where it is written, and
//! somebody who may not say anything on the document says nothing on a
//! few words of it either.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

const DOC: &str = "shop-docs:guides:install";
const TEXT: &str = "# Installing it\n\n## On Linux\n\nThen run the second step, which is short.\n";

async fn library(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = rig.node().app.studio.projects.dir_of("shop").unwrap();
    let _ = std::fs::remove_dir_all(dir.join("docs"));
    std::fs::create_dir_all(dir.join("docs/guides")).unwrap();
    std::fs::write(dir.join("docs/guides/install.md"), TEXT).unwrap();
    let applied =
        rig.send("/api/apply", json!({ "blueprint": "std/docs", "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// The lines of the document's history that are something said.
async fn said(rig: &Rig) -> Vec<Value> {
    let history = rig.read(&format!("/api/records/{DOC}/history")).await;
    history["history"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|line| line["topic"] == "record.commented")
        .cloned()
        .collect()
}

async fn token(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn what_a_comment_on_a_few_words_is_about_is_kept_with_it_and_said_with_it() {
    let rig = Rig::new().await;
    library(&rig).await;
    let anchor = json!({ "quote": "the second step", "before": "Then run ", "after": ", which is", "under": ["Installing it", "On Linux"] });
    rig.send(
        &format!("/api/records/{DOC}/comments"),
        json!({ "body": "Which one is that?", "anchor": anchor }),
    )
    .await;
    let heard = rig.event("record.commented", |event| event.data["id"] == DOC).await;
    assert_eq!(heard.data["anchor"], anchor, "said with the comment: {}", heard.data);
    rig.event("collection.history", |event| event.data["record"] == DOC).await;
    // And on the whole of it, with nothing said of any words.
    rig.send(&format!("/api/records/{DOC}/comments"), json!({ "body": "Read it all." })).await;
    rig.event("collection.history", |event| {
        event.data["record"] == DOC
            && event.data["said"].as_str().is_some_and(|said| said.contains("Read it all"))
    })
    .await;

    let lines = said(&rig).await;
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(lines[0]["data"]["anchor"], anchor, "kept with its line: {}", lines[0]);
    assert!(lines[1]["data"].get("anchor").is_none(), "{}", lines[1]);
    // The usual words say what was said, and nothing of the anchor's parts.
    assert!(!lines[0]["said"].as_str().unwrap().contains("quote"), "{}", lines[0]);
    // The file holds nothing of it: other programs write it.
    let dir = rig.node().app.studio.projects.dir_of("shop").unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("docs/guides/install.md")).unwrap(), TEXT);
}

#[tokio::test]
async fn each_part_of_what_a_comment_is_about_is_checked_where_it_is_written_and_nothing_is_kept_of_one_refused()
 {
    let rig = Rig::new().await;
    library(&rig).await;
    let long = "x".repeat(kernel::anchor::MOST_QUOTED + 1);
    let cases = [
        (json!("the second step"), "says which words"),
        (json!({ "quote": "" }), "is something"),
        (json!({ "quote": "one\ntwo" }), "on one line"),
        (json!({ "quote": long }), "400 characters at most"),
        (json!({ "quote": "x", "before": "y".repeat(41) }), "`anchor.before` is 40 characters"),
        (json!({ "quote": "x", "under": ["a", "b", "c", "d", "e", "f", "g"] }), "6 headings at most"),
        (json!({ "quote": "x", "under": "On Linux" }), "as a list"),
        (json!({ "quote": "x", "line": 4 }), "not `line`"),
    ];
    let path = format!("/api/records/{DOC}/comments");
    for (anchor, says) in cases {
        let (status, refused) = rig
            .call(Method::POST, &path, Some(&rig.token), Some(json!({ "body": "Here.", "anchor": anchor })))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{anchor}: {refused}");
        assert!(refused.to_string().contains(says), "{anchor}: {refused} says `{says}`");
    }
    // Something said after them is the first line there is.
    rig.send(&path, json!({ "body": "Fine.", "anchor": { "quote": "short" } })).await;
    rig.event("collection.history", |event| event.data["record"] == DOC).await;
    let lines = said(&rig).await;
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0]["data"]["anchor"], json!({ "quote": "short" }));
}

#[tokio::test]
async fn a_key_that_may_not_say_anything_on_a_document_says_nothing_on_a_few_words_of_it() {
    let rig = Rig::new().await;
    library(&rig).await;
    let path = format!("/api/records/{DOC}/comments");
    let anchored = json!({ "body": "Which one?", "anchor": { "quote": "the second step" } });
    // A key that reads records and writes none is refused, as it is for the whole document.
    let reader = token(&rig, &["projects", "records:read"]).await;
    let (status, refused) = rig.call(Method::POST, &path, Some(&reader), Some(anchored.clone())).await;
    let (whole, refused_whole) =
        rig.call(Method::POST, &path, Some(&reader), Some(json!({ "body": "Which one?" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!((status, &refused), (whole, &refused_whole), "refused in the same words either way");
    // One with no records at all is told nothing of the document, anchor or not.
    let other = token(&rig, &["projects", "channels"]).await;
    let (status, refused) = rig.call(Method::POST, &path, Some(&other), Some(anchored.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&rig).await.is_empty());

    // A key that may write records says it, and the line says the key said it.
    let writer = token(&rig, &["projects", "records"]).await;
    let (status, kept) = rig.call(Method::POST, &path, Some(&writer), Some(anchored)).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    let heard = rig.event("record.commented", |event| event.data["id"] == DOC).await;
    assert!(heard.data["by"].as_str().unwrap().starts_with("key:"), "{}", heard.data);
    assert_eq!(heard.data["anchor"]["quote"], "the second step");
}
