//! A forum's topics over HTTP (`topics`): set up by
//! whoever manages the factory as the first is made, read whole with
//! their replies nested, changed at the door any record is changed by
//! with a revision all the same, and heard by a project's handlers as
//! what is nobody's.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "helper", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// A topic whole once `replies` replies of it are in its history.
async fn with_replies(rig: &Rig, topic: &str, replies: usize) -> Value {
    loop {
        let read = rig.read(&format!("/api/topics/{topic}")).await;
        if read["replies"].as_array().map_or(0, Vec::len) >= replies {
            return read;
        }
        let mark = rig.mark();
        rig.event("collection.history", |event| event.seq > mark).await;
    }
}

#[tokio::test]
async fn the_first_topic_is_made_by_whoever_manages_the_factory_and_sets_the_topics_up() {
    let rig = Rig::new().await;
    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let asked = json!({ "title": "Which store" });
    let (status, refused) = rig.call(Method::POST, "/api/topics", Some(&member), Some(asked.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused.to_string().contains("whoever manages it makes the first one"), "{refused}");
    assert_eq!(rig.read("/api/topics").await["topics"], json!([]), "there are none, and nothing was set up");

    let made =
        rig.send("/api/topics", json!({ "title": "Payment provider options", "body": "# Which" })).await;
    assert_eq!(made["data"]["status"], Value::Null, "made with nothing said, it has no status: {made}");
    let (status, theirs) = rig.call(Method::POST, "/api/topics", Some(&member), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "set up, a member makes one: {theirs}");
    let listed = rig.read("/api/topics").await;
    let titles: Vec<&str> = listed["topics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| one["data"]["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["Payment provider options", "Which store"]);
    // The table is the factory's own, and is no project's.
    let (status, _) = rig.get("/api/collections/@/topics/records").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn replies_nest_under_what_they_answer_and_what_a_topic_says_is_revised_by_any_door() {
    let rig = Rig::new().await;
    let made = rig.send("/api/topics", json!({ "title": "Why the build is slow", "status": "open" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    let first = rig.send(&format!("/api/topics/{id}/replies"), json!({ "body": "Linking." })).await;
    let under = json!({ "body": "Which linker?", "parent": first["seq"] });
    let second = rig.send("/api/topics/why%20the%20build%20is%20slow/replies", under).await;
    let read = with_replies(&rig, &id, 2).await;
    let replies = read["replies"].as_array().unwrap();
    assert_eq!(
        (replies[0]["parent"].clone(), replies[1]["parent"].clone()),
        (Value::Null, first["seq"].clone())
    );
    assert_eq!(replies[1]["seq"], second["seq"]);

    // Changed as any record is, it is revised all the same; its place is not changed so.
    let (status, _) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{id}"),
            Some(&rig.token),
            Some(json!({ "fields": { "body": "Mostly linking." } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, refused) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{id}"),
            Some(&rig.token),
            Some(json!({ "fields": { "workspace": "x" } })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused.to_string().contains("written by the topic's own commands"), "{refused}");
    let revisions = rig.read(&format!("/api/topics/{id}/revisions")).await;
    let bodies: Vec<&str> =
        revisions["revisions"].as_array().unwrap().iter().map(|one| one["body"].as_str().unwrap()).collect();
    assert_eq!(bodies, ["", "Mostly linking."]);
    // Each says who made it by the name on their account, as a record's history does.
    let me = rig.read("/api/me").await;
    for revision in revisions["revisions"].as_array().unwrap() {
        assert_eq!(revision["by"], me["actor"], "{revision}");
        assert_eq!(revision["who"], me["account"]["name"], "{revision}");
    }

    let answered = rig.send(&format!("/api/topics/{id}/answer"), json!({ "reply": first["seq"] })).await;
    assert_eq!(answered["data"]["status"], json!("answered"));
    rig.event("topic.answered", |event| event.data["record"] == json!(id)).await;
}

const HEARS: &str = r#"
blueprint: hears-topics
permissions: [records]
on:
  closed:
    event: topic.closed
    steps:
      noted: { do: emit, with: { topic: test.heard, title: "${event.data.title}" } }
  changed:
    event: record.updated
    for: { collection: topics }
    steps:
      noted: { do: emit, with: { topic: test.heard, changed: "${event.data.id}" } }
"#;

#[tokio::test]
async fn what_is_said_of_a_workspaces_topic_is_nobodys_and_a_projects_handlers_hear_it() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = rig.send("/api/apply", json!({ "source": HEARS, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let made = rig.send("/api/topics", json!({ "title": "Retired stores" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    let (status, _) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{id}"),
            Some(&rig.token),
            Some(json!({ "fields": { "status": "closed" } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    rig.event("test.heard", |event| event.data["title"] == json!("Retired stores")).await;
    rig.event("test.heard", |event| event.data["changed"] == json!(id)).await;
}

#[tokio::test]
async fn an_agent_named_in_a_reply_is_given_the_topic_where_it_is_in_its_wiki_and_the_chain_and_answers_under_that_reply()
 {
    let rig = Rig::with("api-topics-asked", |config, _| {
        // Says back what it was handed: what it was given is what it answers.
        config.adapters.push(("echo".into(), crate::standin::argv(&["cat"])));
    })
    .await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/agents", json!({ "name": "scout", "adapter": "echo" })).await;
    let channel = rig.send("/api/channels", json!({ "name": "lobby" })).await;
    rig.send(
        &format!("/api/channels/{}/messages", channel["id"].as_str().unwrap()),
        json!({ "body": "Nothing of this." }),
    )
    .await;
    // A page three deep in a wiki, with pages under it.
    rig.send("/api/topics", json!({ "title": "Handbook" })).await;
    rig.send("/api/topics", json!({ "title": "Building", "under": "Handbook" })).await;
    let made = rig
        .send(
            "/api/topics",
            json!({ "title": "Which linker", "body": "The build links for minutes.", "under": "Building" }),
        )
        .await;
    let id = made["id"].as_str().unwrap().to_string();
    for title in ["Gold", "Mold"] {
        rig.send(
            "/api/topics",
            json!({ "title": title, "body": format!("All about {title}."), "under": id }),
        )
        .await;
    }
    let top =
        rig.send(&format!("/api/topics/{id}/replies"), json!({ "body": "Linking is most of it." })).await;
    let asking = json!({ "body": "@scout which linker is faster?", "parent": top["seq"] });
    let asked = rig.send(&format!("/api/topics/{id}/replies"), asking).await;
    rig.event("agent.turn.finished", |event| event.data["slot"] == "scout").await;
    let read = with_replies(&rig, &id, 3).await;
    let replies = read["replies"].as_array().unwrap();
    let answer = replies.iter().find(|reply| reply["by"] == json!("agent:scout")).expect("scout answered");
    assert_eq!(answer["parent"], asked["seq"], "under the reply that asked it");
    let said = answer["body"].as_str().unwrap();
    for given in [
        // It has no status, and none is named.
        "\"Which linker\" (in ",
        "The build links for minutes.",
        "Linking is most of it.",
        "which linker is faster?",
        "It is a page of the wiki \"Handbook\", under Handbook › Building.",
        "The pages under it: \"Gold\", \"Mold\";",
        // It may read and change records, so it is offered what does.
        &format!("topic reply {id} --to <reply>"),
    ] {
        assert!(said.contains(given), "it was given {given}: {said}");
    }
    assert!(!said.contains("All about Gold."), "the pages under it by title alone: {said}");
    assert!(!said.contains("Nothing of this."), "nothing of any channel: {said}");
    // One turn for the reply that asked it, and none for its own answer.
    assert_eq!(rig.count_of("agent.turn.queued").await, 1);
}

#[tokio::test]
async fn an_agents_answer_in_a_topic_said_as_the_daemon_went_is_said_once_when_it_is_back() {
    let rig = Rig::with("api-topics-again", |config, _| {
        config
            .adapters
            .push(("echo".into(), crate::standin::argv(&["swallow", "say", "Use the gold linker."])));
    })
    .await;
    let mut rig = Rig::owned(rig).await;
    rig.send("/api/agents", json!({ "name": "scout", "adapter": "echo" })).await;
    let made = rig.send("/api/topics", json!({ "title": "Which linker" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    // The daemon goes as the answer is said, before the turn is kept as ended.
    let journal = rig.node().app.journal.clone();
    journal.shut_after(|event| event.topic == "record.commented" && event.data["by"] == "agent:scout");
    rig.send(&format!("/api/topics/{id}/replies"), json!({ "body": "@scout which?" })).await;
    journal.shut().await;
    rig.stop().await;
    rig.start().await;
    rig.event("agent.turn.finished", |event| event.data["slot"] == "scout").await;
    let read = with_replies(&rig, &id, 2).await;
    let answers: Vec<&Value> = read["replies"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|reply| reply["by"] == json!("agent:scout"))
        .collect();
    assert_eq!(answers.len(), 1, "said once: {read}");
    assert_eq!(rig.count_of("agent.turn.queued").await, 1, "and asked once");
}

/// The replies of the topic `id` said by `by`, once there are `replies` in all.
async fn said_by(rig: &Rig, id: &str, replies: usize, by: &str) -> Vec<Value> {
    let read = with_replies(rig, id, replies).await;
    read["replies"].as_array().unwrap().iter().filter(|reply| reply["by"] == json!(by)).cloned().collect()
}

#[tokio::test]
async fn an_agent_that_may_not_change_records_answers_where_a_person_asked_it_in_a_topic_offered_no_command_it_may_not_run()
 {
    let rig = Rig::with("api-topics-unheld", |config, _| {
        // Says back what it was handed: what it was given is what it answers.
        config.adapters.push(("echo".into(), crate::standin::argv(&["cat"])));
    })
    .await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/agents", json!({ "name": "scout", "adapter": "echo", "scopes": ["channels"] })).await;
    let made = rig.send("/api/topics", json!({ "title": "Which linker" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    let asked = rig.send(&format!("/api/topics/{id}/replies"), json!({ "body": "@scout which?" })).await;
    rig.event("agent.turn.finished", |event| event.data["slot"] == "scout").await;
    let answers = said_by(&rig, &id, 2, "agent:scout").await;
    assert_eq!(answers.len(), 1, "{answers:?}");
    let said = answers[0]["body"].as_str().unwrap();
    assert!(said.contains("Your answer is said for you as a reply"), "{said}");
    assert!(!said.contains("topic show") && !said.contains("topic reply"), "nothing it is refused: {said}");
    assert_eq!(answers[0]["parent"], asked["seq"], "under the reply that asked it");
    let agents = rig.read("/api/agents").await;
    let agent = agents["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == "scout").unwrap();
    assert_eq!(agent["scopes"], json!(["channels"]), "it holds what it held: {agent}");
}

#[tokio::test]
async fn an_agent_that_answers_nothing_in_a_topic_is_said_there_by_the_factory_to_have_given_no_answer() {
    let rig = Rig::with("api-topics-nothing", |config, _| {
        config.adapters.push(("quiet".into(), crate::standin::argv(&["swallow"])));
    })
    .await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/agents", json!({ "name": "scout", "adapter": "quiet" })).await;
    let made = rig.send("/api/topics", json!({ "title": "Which linker" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    let asked = rig.send(&format!("/api/topics/{id}/replies"), json!({ "body": "@scout which?" })).await;
    rig.event("agent.turn.finished", |event| event.data["slot"] == "scout").await;
    let said = said_by(&rig, &id, 2, "system").await;
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(said[0]["body"], json!("scout gave no answer."));
    assert_eq!(said[0]["parent"], asked["seq"], "under the reply that asked it");
}

#[tokio::test]
async fn an_answer_that_cannot_be_said_in_a_topic_is_said_there_by_the_factory_to_have_not_been() {
    let long = "gold ".repeat(4_000);
    let rig = Rig::with("api-topics-unsaid", |config, _| {
        config.adapters.push(("long".into(), crate::standin::argv(&["swallow", "say", &long])));
    })
    .await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/agents", json!({ "name": "scout", "adapter": "long" })).await;
    let made = rig.send("/api/topics", json!({ "title": "Which linker" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    let asked = rig.send(&format!("/api/topics/{id}/replies"), json!({ "body": "@scout which?" })).await;
    rig.event("agent.turn.finished", |event| event.data["slot"] == "scout").await;
    let said = said_by(&rig, &id, 2, "system").await;
    assert_eq!(said.len(), 1, "{said:?}");
    let body = said[0]["body"].as_str().unwrap();
    assert!(body.starts_with("scout answered, and its answer could not be said here: "), "{body}");
    assert!(body.contains("too much to say"), "and why: {body}");
    assert_eq!(said[0]["parent"], asked["seq"], "under the reply that asked it");
}

#[tokio::test]
async fn that_an_agent_gave_no_answer_in_a_topic_said_as_the_daemon_went_is_said_once_when_it_is_back() {
    let rig = Rig::with("api-topics-nothing-again", |config, _| {
        config.adapters.push(("quiet".into(), crate::standin::argv(&["swallow"])));
    })
    .await;
    let mut rig = Rig::owned(rig).await;
    rig.send("/api/agents", json!({ "name": "scout", "adapter": "quiet" })).await;
    let made = rig.send("/api/topics", json!({ "title": "Which linker" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    let journal = rig.node().app.journal.clone();
    journal.shut_after(|event| event.topic == "record.commented" && event.data["by"] == "system");
    rig.send(&format!("/api/topics/{id}/replies"), json!({ "body": "@scout which?" })).await;
    journal.shut().await;
    rig.stop().await;
    rig.start().await;
    rig.event("agent.turn.finished", |event| event.data["slot"] == "scout").await;
    let said = said_by(&rig, &id, 2, "system").await;
    assert_eq!(said.len(), 1, "said once: {said:?}");
    assert_eq!(said[0]["body"], json!("scout gave no answer."));
    assert_eq!(rig.count_of("agent.turn.queued").await, 1, "and asked once");
}

/// The lines of `token`'s box of something said on a record.
async fn commented(rig: &Rig, token: &str) -> Vec<Value> {
    let (status, found) = rig.call(Method::GET, "/api/inbox?all=true", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    found["lines"].as_array().unwrap().iter().filter(|line| line["kind"] == "commented").cloned().collect()
}

#[tokio::test]
async fn a_person_named_in_a_reply_is_told_in_their_box_once_an_agent_named_there_is_asked_and_told_nothing_and_a_name_of_nobody_tells_nobody()
 {
    let rig = Rig::with("api-topics-named", |config, _| {
        config.adapters.push(("echo".into(), crate::standin::argv(&["cat"])));
    })
    .await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let nell = crate::kept::person(&rig, "Nell").await;
    rig.send("/api/agents", json!({ "name": "scout", "adapter": "echo" })).await;
    let id =
        rig.send("/api/topics", json!({ "title": "Which linker" })).await["id"].as_str().unwrap().to_string();
    let replies = format!("/api/topics/{id}/replies");
    let top = rig.send(&replies, json!({ "body": "Linking is most of it." })).await;

    // 1: named in a reply under another, with a name of nobody beside hers: she is told, of that reply.
    let named = json!({ "body": "@nell @nobody which linker is faster?", "parent": top["seq"] });
    let asked = rig.send(&replies, named).await;
    let her = nell.actor.clone();
    rig.event("inbox.told", move |event| {
        event.data["owner"] == her.as_str() && event.data["kind"] == "commented"
    })
    .await;
    let lines = commented(&rig, &nell.token).await;
    assert_eq!(lines.len(), 1, "{lines:?}");
    let on = &lines[0]["on_record"];
    assert_eq!((&lines[0]["place"], &lines[0]["why"]), (&json!(id), &json!("named")), "{lines:?}");
    assert_eq!(
        (&on["title"], &on["body"]),
        (&json!("Which linker"), &json!("@nell @nobody which linker is faster?"))
    );
    assert_eq!(on["seq"], asked["seq"], "the line is of the reply that named her");

    // 2: an agent the factory starts, named in a chain of its own, answers there and is given no line;
    // a reply naming nobody tells nobody; the next that names her is the next thing told.
    let mark = rig.mark();
    rig.send(&replies, json!({ "body": "@scout which linker is faster?" })).await;
    rig.event("agent.turn.finished", |event| event.data["slot"] == "scout").await;
    let read = with_replies(&rig, &id, 4).await;
    assert!(
        read["replies"].as_array().unwrap().iter().any(|reply| reply["by"] == json!("agent:scout")),
        "{read}"
    );
    rig.send(&replies, json!({ "body": "@nobody is anybody there?" })).await;
    rig.send(&replies, json!({ "body": "@nell one more thing" })).await;
    let her = nell.actor.clone();
    rig.event("inbox.told", move |event| event.data["owner"] == her.as_str() && event.data["unread"] == 2)
        .await;
    let told = rig
        .said(journal::EventQuery { topics: "inbox.told".into(), after: mark, ..Default::default() })
        .await;
    let owners: Vec<&Value> = told.iter().map(|event| &event.data["owner"]).collect();
    assert_eq!(owners, [&json!(nell.actor)], "nobody else was told anything: {told:?}");
    assert_eq!(commented(&rig, &nell.token).await.len(), 2);
}
