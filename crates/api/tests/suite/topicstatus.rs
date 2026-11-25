//! A topic's status over HTTP, which a topic may not have: a wiki's page
//! or a thread that gathers what is known is no question, and stands
//! nowhere. A topic is made with none unless one is said, takes one and
//! loses it again at the door any record is changed by, and is answered
//! only where it is tracked as a question.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

async fn patched(rig: &Rig, id: &str, fields: Value) -> (StatusCode, Value) {
    rig.call(
        Method::PATCH,
        &format!("/api/records/{id}"),
        Some(&rig.token),
        Some(json!({ "fields": fields })),
    )
    .await
}

/// How the topic `id` stands as each door that reads it hands it: whole,
/// in the list of every topic, and among the pages of the page it is under.
async fn standing(rig: &Rig, id: &str) -> Value {
    let whole = rig.read(&format!("/api/topics/{id}")).await;
    let listed = rig.read("/api/topics").await;
    let line =
        listed["topics"].as_array().unwrap().iter().find(|one| one["id"] == json!(id)).unwrap().clone();
    assert_eq!(whole["topic"]["data"]["status"], line["data"]["status"], "{whole} {line}");
    whole["topic"]["data"]["status"].clone()
}

#[tokio::test]
async fn a_topic_made_with_nothing_said_has_no_status_nor_does_a_page_made_under_it() {
    let rig = Rig::new().await;
    let made = rig.send("/api/topics", json!({ "title": "Handbook" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    assert_eq!(made["data"]["status"], Value::Null, "{made}");
    assert_eq!(standing(&rig, &id).await, Value::Null);

    let page = rig.send("/api/topics", json!({ "title": "Deploying", "under": id })).await;
    assert_eq!(page["data"]["status"], Value::Null, "{page}");
    let held = rig.read(&format!("/api/topics/{id}")).await;
    assert_eq!(held["pages"], json!([{ "id": page["id"], "title": "Deploying", "status": null }]));
}

#[tokio::test]
async fn a_topic_is_made_open_or_standing_where_that_is_said_and_never_answered_or_anything_else() {
    let rig = Rig::new().await;
    let open = rig.send("/api/topics", json!({ "title": "Which store", "status": "open" })).await;
    assert_eq!(open["data"]["status"], json!("open"), "{open}");
    let id = open["id"].as_str().unwrap().to_string();
    rig.event("topic.opened", |event| event.data["record"] == json!(id)).await;
    let page = rig
        .send("/api/topics", json!({ "title": "How we name branches", "under": id, "status": "standing" }))
        .await;
    assert_eq!(page["data"]["status"], json!("standing"), "{page}");

    for wrong in ["answered", "sometimes"] {
        let (status, refused) = rig.post("/api/topics", json!({ "title": "Nothing", "status": wrong })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        assert!(refused.to_string().contains("`open` or `standing`"), "{refused}");
    }
    let titles = rig.read("/api/topics").await["topics"].as_array().unwrap().len();
    assert_eq!(titles, 2, "nothing was made of what was refused");
}

#[tokio::test]
async fn marking_an_answer_in_a_topic_with_no_status_is_refused_saying_how_to_make_it_a_question() {
    let rig = Rig::new().await;
    let made = rig.send("/api/topics", json!({ "title": "Payments" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    let reply = rig.send(&format!("/api/topics/{id}/replies"), json!({ "body": "Cards first." })).await;
    for asked in [json!({ "reply": reply["seq"] }), json!({ "reply": null })] {
        let (status, refused) = rig.post(&format!("/api/topics/{id}/answer"), asked).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        let said = refused.to_string();
        assert!(said.contains("not tracked as a question") && said.contains("status=open"), "{said}");
    }
    assert_eq!(standing(&rig, &id).await, Value::Null, "refused, it is still not tracked");
    assert_eq!(rig.read(&format!("/api/topics/{id}")).await["topic"]["data"]["answer"], Value::Null);
}

#[tokio::test]
async fn a_status_given_and_then_cleared_takes_a_topic_back_to_none_and_each_is_said() {
    let rig = Rig::new().await;
    let made = rig.send("/api/topics", json!({ "title": "Why the build is slow" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    let (status, _) = patched(&rig, &id, json!({ "status": "open" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(standing(&rig, &id).await, json!("open"));
    rig.event("topic.opened", |event| event.data["record"] == json!(id)).await;

    for cleared in [Value::Null, json!("")] {
        patched(&rig, &id, json!({ "status": "closed" })).await;
        let mark = rig.mark();
        let (status, answer) = patched(&rig, &id, json!({ "status": cleared })).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        assert_eq!(standing(&rig, &id).await, Value::Null, "cleared with {cleared}");
        rig.event("topic.untracked", |event| event.seq > mark && event.data["record"] == json!(id)).await;
    }
    let reply = rig.send(&format!("/api/topics/{id}/replies"), json!({ "body": "Linking." })).await;
    let (status, refused) =
        rig.post(&format!("/api/topics/{id}/answer"), json!({ "reply": reply["seq"] })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "untracked again, it is answered no more: {refused}");
}

#[tokio::test]
async fn a_topic_that_stands_open_as_every_topic_made_before_did_is_open_and_answered_as_it_was() {
    let rig = Rig::new().await;
    let made = rig.send("/api/topics", json!({ "title": "Which linker" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    // Its row says `open`, as every topic's row did that was made before a topic could have no status.
    patched(&rig, &id, json!({ "status": "open" })).await;
    assert_eq!(standing(&rig, &id).await, json!("open"));
    patched(&rig, &id, json!({ "summary": "Most of it is linking." })).await;
    assert_eq!(standing(&rig, &id).await, json!("open"), "a change of what it says leaves how it stands");

    let reply = rig.send(&format!("/api/topics/{id}/replies"), json!({ "body": "mold." })).await;
    let answered = rig.send(&format!("/api/topics/{id}/answer"), json!({ "reply": reply["seq"] })).await;
    assert_eq!(answered["data"]["status"], json!("answered"), "{answered}");
    let opened = rig.send(&format!("/api/topics/{id}/answer"), json!({ "reply": null })).await;
    assert_eq!(
        (opened["data"]["status"].clone(), opened["data"]["answer"].clone()),
        (json!("open"), Value::Null)
    );
}
