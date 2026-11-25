//! Links between topics over HTTP: a page names another by its id
//! (`[words](topic:@-topics-3)`), so a new title or a move of the page
//! linked to keeps every link leading to it; the pages that link to a page
//! are read from what they say as they are now, each with its wiki; and
//! nobody who may not read topics is told the title of one that links.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "reader", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

fn id(page: &Value) -> String {
    page["id"].as_str().unwrap().to_string()
}

async fn set(rig: &Rig, page: &str, fields: Value) {
    let path = format!("/api/records/{page}");
    let (status, said) =
        rig.call(Method::PATCH, &path, Some(&rig.token), Some(json!({ "fields": fields }))).await;
    assert_eq!(status, StatusCode::OK, "{said}");
}

/// The pages that link to `page`, each its title and its wiki's title.
async fn linking(rig: &Rig, page: &str) -> Vec<(String, Option<String>)> {
    let read = rig.read(&format!("/api/topics/{page}/linked")).await;
    read["linked"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| {
            (one["title"].as_str().unwrap().to_string(), one["wiki"]["title"].as_str().map(str::to_string))
        })
        .collect()
}

#[tokio::test]
async fn the_pages_that_link_to_a_page_are_said_with_their_wiki_and_follow_what_they_say_now() {
    let rig = Rig::new().await;
    let amphibians = id(&rig.send("/api/topics", json!({ "title": "Amphibians" })).await);
    let fungus =
        id(&rig.send("/api/topics", json!({ "title": "Chytrid fungus", "under": amphibians })).await);
    let link = format!("[the fungus](topic:{fungus})");
    rig.send(
        "/api/topics",
        json!({ "title": "Frogs", "under": amphibians, "body": format!("Many died of {link}.") }),
    )
    .await;
    let decline = id(&rig.send("/api/topics", json!({ "title": "Decline", "body": "" })).await);
    set(&rig, &decline, json!({ "summary": format!("Mostly {link}.") })).await;
    // Written as code, and the page's own link to itself, are no link to it from another.
    rig.send("/api/topics", json!({ "title": "Writing links", "body": format!("Write `{link}`.") })).await;
    set(&rig, &fungus, json!({ "body": format!("This page: {link}.") })).await;
    assert_eq!(
        linking(&rig, &fungus).await,
        [("Decline".to_string(), None), ("Frogs".to_string(), Some("Amphibians".to_string()))],
        "each by its title, a page under a wiki with its wiki"
    );
    assert_eq!(linking(&rig, "chytrid FUNGUS").await.len(), 2, "the page asked of by its title");

    // A link taken out is gone from the next reading, and one written is in it.
    set(&rig, &decline, json!({ "summary": "Nobody knows." })).await;
    set(&rig, &amphibians, json!({ "body": format!("See [it](topic:{fungus}#12).") })).await;
    assert_eq!(
        linking(&rig, &fungus).await,
        [("Amphibians".to_string(), None), ("Frogs".to_string(), Some("Amphibians".to_string()))],
        "a link to one of its replies is a link to the page"
    );
    assert!(linking(&rig, &decline).await.is_empty());
    let (status, _) = rig.get("/api/topics/nothing-called-so/linked").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_link_leads_to_its_page_through_a_new_title_and_a_move() {
    let rig = Rig::new().await;
    let handbook = id(&rig.send("/api/topics", json!({ "title": "Handbook" })).await);
    let fungus = id(&rig.send("/api/topics", json!({ "title": "Chytrid fungus", "under": handbook })).await);
    let body = format!("Many died of [Chytrid fungus](topic:{fungus}).");
    let frogs = id(&rig.send("/api/topics", json!({ "title": "Frogs", "body": body })).await);

    set(&rig, &fungus, json!({ "title": "Batrachochytrium" })).await;
    rig.send(&format!("/api/topics/{fungus}/move"), json!({ "under": null })).await;
    set(&rig, &frogs, json!({ "title": "Frogs and toads" })).await;

    let read = rig.read(&format!("/api/topics/{frogs}")).await;
    assert_eq!(read["topic"]["data"]["body"], json!(body), "the link is written as it was");
    let led = rig.read(&format!("/api/topics/{fungus}")).await;
    assert_eq!(
        led["topic"]["data"]["title"],
        json!("Batrachochytrium"),
        "and leads to the page as it is now"
    );
    assert_eq!(linking(&rig, &fungus).await, [("Frogs and toads".to_string(), None)]);
}

#[tokio::test]
async fn nobody_who_may_not_read_topics_is_told_the_title_of_a_page_that_links() {
    let rig = Rig::new().await;
    let fungus = id(&rig.send("/api/topics", json!({ "title": "Chytrid fungus" })).await);
    let body = format!("Many died of [it](topic:{fungus}).");
    rig.send("/api/topics", json!({ "title": "A secret study", "body": body })).await;

    let talker = key(&rig, &["channels:read", "channels:write"]).await;
    for path in
        [format!("/api/topics/{fungus}/linked"), format!("/api/topics/{fungus}"), "/api/topics".to_string()]
    {
        let (status, said) = rig.call(Method::GET, &path, Some(&talker), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {said}");
        assert!(!said.to_string().contains("A secret study"), "{path}: {said}");
    }
    let reader = key(&rig, &["records:read"]).await;
    let (status, said) =
        rig.call(Method::GET, &format!("/api/topics/{fungus}/linked"), Some(&reader), None).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said["linked"][0]["title"], json!("A secret study"));
}

#[tokio::test]
async fn a_page_that_links_only_through_a_series_box_says_which_series() {
    let rig = Rig::new().await;
    let practice = id(&rig.send("/api/topics", json!({ "title": "Practice" })).await);
    let series = |title: &str| {
        format!(
            "Words.\n\n```series\ntitle: {title}\n[Practice](topic:{practice})\n[Breath](topic:@-topics-90)\n```\n"
        )
    };
    rig.send("/api/topics", json!({ "title": "Breath", "body": series("Meditation") })).await;
    rig.send(
        "/api/topics",
        json!({ "title": "Body scan", "body": format!("{}\nSee [practice](topic:{practice}).", series("Meditation")) }),
    )
    .await;
    let walking = id(&rig.send("/api/topics", json!({ "title": "Walking", "body": "Nothing yet." })).await);
    set(&rig, &walking, json!({ "summary": series("Moving") })).await;
    rig.send(
        "/api/topics",
        json!({ "title": "Sitting", "body": format!("```infobox\ntitle: Sitting\nSee: [p](topic:{practice})\n```\n{}", series("Meditation")) }),
    )
    .await;
    let read = rig.read(&format!("/api/topics/{practice}/linked")).await;
    let said: Vec<(String, Value)> = read["linked"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| (one["title"].as_str().unwrap().to_string(), one["series"].clone()))
        .collect();
    assert_eq!(
        said,
        [
            ("Body scan".to_string(), Value::Null),
            ("Breath".to_string(), json!("Meditation")),
            ("Sitting".to_string(), Value::Null),
            ("Walking".to_string(), json!("Moving")),
        ],
        "a page that links from its words or another box is said as any other, one whose only link is a series box with its title"
    );
}
