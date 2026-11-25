//! Pictures in a topic's text: a file put in the post or a reply is
//! attached to the topic, which is a record, and named in the text by its
//! id (`attachment:<id>`). So it is read by whoever may read the topic and
//! by nobody else, at every door; an SVG is drawn and runs nothing; a page
//! is shown as a view; and the text that names it keeps naming it through
//! revisions and a new title.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "reader", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// Attaches `bytes` to the topic `topic` as `name`, saying it is `media`: its id.
async fn attached(rig: &Rig, topic: &str, name: &str, media: &str, bytes: &[u8]) -> String {
    let answer = rig
        .client
        .post(format!("{}/api/records/{topic}/attachments?name={name}", rig.url))
        .bearer_auth(&rig.token)
        .header("content-type", media)
        .body(bytes.to_vec())
        .send()
        .await
        .unwrap();
    assert_eq!(answer.status(), StatusCode::OK);
    let kept: Value = answer.json().await.unwrap();
    kept["id"].as_str().unwrap().to_string()
}

/// What `path` answers `token`: its status, a header of it, and what it holds.
async fn fetched(rig: &Rig, token: &str, path: &str, header: &str) -> (StatusCode, String, String) {
    let answer = rig.client.get(format!("{}{path}", rig.url)).bearer_auth(token).send().await.unwrap();
    let said =
        answer.headers().get(header).and_then(|value| value.to_str().ok()).unwrap_or_default().to_string();
    (answer.status(), said, answer.text().await.unwrap_or_default())
}

#[tokio::test]
async fn a_picture_in_a_topic_is_read_by_whoever_may_read_the_topic_and_by_nobody_else() {
    let rig = Rig::new().await;
    let made = rig.send("/api/topics", json!({ "title": "How the till works" })).await;
    let topic = made["id"].as_str().unwrap().to_string();
    let png = attached(&rig, &topic, "till.png", "image/png", b"\x89PNG the till").await;
    let drawn = "<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script><rect/></svg>";
    let svg = attached(&rig, &topic, "flow.svg", "image/svg+xml", drawn.as_bytes()).await;
    let page = attached(&rig, &topic, "chart.html", "text/html", b"<p>Sales</p>").await;

    // Whoever may read records reads the topic, and each thing in its text.
    let reader = key(&rig, &["records:read"]).await;
    let (status, kind, _) = fetched(&rig, &reader, &format!("/api/attachments/{png}"), "content-type").await;
    assert_eq!((status, kind.as_str()), (StatusCode::OK, "image/png"));
    // An SVG is handed over to be drawn in place, held by a policy under which nothing in it runs.
    let (status, kind, text) =
        fetched(&rig, &reader, &format!("/api/attachments/{svg}"), "content-type").await;
    assert_eq!((status, kind.as_str(), text.as_str()), (StatusCode::OK, "image/svg+xml", drawn));
    let (_, policy, _) =
        fetched(&rig, &reader, &format!("/api/attachments/{svg}"), "content-security-policy").await;
    assert!(policy.starts_with("sandbox;") && policy.contains("default-src 'none'"), "{policy}");
    let (_, handed, _) =
        fetched(&rig, &reader, &format!("/api/attachments/{svg}"), "content-disposition").await;
    assert!(handed.starts_with("inline"), "{handed}");
    // A page is shown as a view, in a box of its own, as a channel's is.
    let (status, policy, shown) =
        fetched(&rig, &reader, &format!("/api/attachments/{page}/view"), "content-security-policy").await;
    assert_eq!(status, StatusCode::OK, "{shown}");
    assert!(policy.starts_with("sandbox allow-scripts;"), "{policy}");
    assert!(shown.ends_with("<p>Sales</p>"), "{shown}");
    // A picture is no page.
    let (status, _, refused) = fetched(&rig, &reader, &format!("/api/attachments/{png}/view"), "x").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");

    // Whoever may not read records reads none of it, at any door.
    let outside = key(&rig, &["channels:read"]).await;
    for door in [
        format!("/api/topics/{topic}"),
        format!("/api/records/{topic}/attachments"),
        format!("/api/attachments/{png}"),
        format!("/api/attachments/{svg}"),
        format!("/api/attachments/{page}/view"),
    ] {
        let (status, _, refused) = fetched(&rig, &outside, &door, "x").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{door}: {refused}");
    }
}

#[tokio::test]
async fn a_picture_named_in_a_topics_text_is_still_named_and_read_after_revisions_and_a_new_title() {
    let rig = Rig::new().await;
    let made = rig.send("/api/topics", json!({ "title": "How the till works" })).await;
    let topic = made["id"].as_str().unwrap().to_string();
    let svg =
        attached(&rig, &topic, "flow.svg", "image/svg+xml", b"<svg xmlns='http://www.w3.org/2000/svg'/>")
            .await;
    let named = format!("The till, step by step:\n\n![How a sale goes](attachment:{svg} \"side\")\n");
    let change = |fields: Value| {
        let (rig, topic) = (&rig, &topic);
        async move {
            let path = format!("/api/records/{topic}");
            let (status, answer) =
                rig.call(Method::PATCH, &path, Some(&rig.token), Some(json!({ "fields": fields }))).await;
            assert_eq!(status, StatusCode::OK, "{answer}");
        }
    };
    change(json!({ "body": named })).await;
    change(json!({ "title": "The till, as it works now" })).await;
    change(json!({ "body": format!("{named}\nAnd then it prints.\n") })).await;
    let reply = format!("Here it is drawn:\n![](attachment:{svg})");
    rig.send(&format!("/api/topics/{topic}/replies"), json!({ "body": reply })).await;

    // Read whole, by its new title, the text names the picture as it was written, and the picture is there.
    let read = loop {
        let read = rig.read("/api/topics/the%20till%2C%20as%20it%20works%20now").await;
        if read["replies"].as_array().is_some_and(|replies| !replies.is_empty()) {
            break read;
        }
        let mark = rig.mark();
        rig.event("collection.history", |event| event.seq > mark).await;
    };
    assert!(
        read["topic"]["data"]["body"].as_str().unwrap().contains(&format!("(attachment:{svg} \"side\")")),
        "{read}"
    );
    assert_eq!(read["replies"][0]["body"], json!(reply));
    let revisions = rig.read(&format!("/api/topics/{topic}/revisions")).await;
    let naming = revisions["revisions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|one| one["body"].as_str().unwrap_or_default().contains(&format!("attachment:{svg}")))
        .count();
    // Each revision after it was put in names it: the one that put it in, the new title, and the words added.
    assert_eq!(naming, 3, "{revisions}");
    let (status, kind, _) =
        fetched(&rig, &rig.token, &format!("/api/attachments/{svg}"), "content-type").await;
    assert_eq!((status, kind.as_str()), (StatusCode::OK, "image/svg+xml"));
}
