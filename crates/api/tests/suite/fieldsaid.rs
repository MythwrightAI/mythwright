//! What is said of a record carries the fields its table says to carry
//! (`said: true`, `studio::fieldsaid`): a card's move and change, a row added,
//! changed, spoken on and taken away, each with what the record holds of
//! them under `fields`. So a handler is chosen by such a field with
//! nothing run (`for:` naming `fields.<name>`, a plain `when`), with no
//! rebuild for a field a blueprint adds.
//!
//! What hears it is what hears the event, so a table kept private
//! carries nothing to whoever it is kept from; a field kept for itself
//! and one that says what giving it a value takes are refused the word;
//! a long text is cut; and a table that says it of nothing says what it
//! always did.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A board whose cards say which sprint they are of and carry it, with
/// notes they carry and a size they do not; a table of tasks that carries
/// where each is; and two handlers chosen by the sprint, one by what the
/// conductor is told it is addressed by and one by a plain `when`.
const PLAN: &str = r#"
blueprint: plan
permissions: [projects, records, board]
steps:
  board:
    do: collection.ensure
    with:
      name: board
      fields:
        title: { type: text, required: true }
        body: { type: text }
        status: { type: choice, of: [todo, doing, done], required: true }
        sprint: { type: text, said: true }
        notes: { type: text, said: true }
        size: { type: text }
      views: { board: { kind: board, by: status } }
  tasks:
    do: collection.ensure
    with:
      name: tasks
      fields:
        title: { type: text, required: true }
        area: { type: choice, of: [garden, kitchen], said: true }
        effort: { type: number }
      history: [record.*]
on:
  autumn:
    event: board.item.moved
    for: { project: shop, fields.sprint: autumn }
    steps:
      tell: { do: emit, with: { topic: shop.autumn, card: "${event.data.id}" } }
  spring:
    event: board.item.moved
    when: ${ event.data.fields.sprint == "spring" }
    steps:
      tell: { do: emit, with: { topic: shop.spring, card: "${event.data.id}" } }
"#;

async fn planned() -> Rig {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = rig.send("/api/apply", json!({ "source": PLAN, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    rig
}

async fn card(rig: &Rig, title: &str, fields: Value) -> String {
    let added = rig.send("/api/board/shop/items", json!({ "title": title, "fields": fields })).await;
    added["id"].as_str().unwrap().to_string()
}

async fn change(rig: &Rig, path: &str, body: Value) {
    let (status, answer) = rig.call(Method::PATCH, path, Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "PATCH {path}: {answer}");
}

/// What the first event on `topic` about `id` carries.
async fn carried(rig: &Rig, topic: &str, id: &str) -> Value {
    rig.event(topic, |event| event.data["id"] == id).await.data["fields"].clone()
}

#[tokio::test]
async fn a_field_a_table_says_is_carried_is_in_every_event_about_its_records_and_chooses_a_handler_with_nothing_run()
 {
    let rig = planned().await;
    let long = "é".repeat(2000);
    let quiet = card(&rig, "Nothing said", json!({})).await;
    let spring = card(&rig, "Sow", json!({ "sprint": "spring", "size": "small" })).await;
    let autumn = card(&rig, "Reap", json!({ "sprint": "autumn", "notes": long })).await;

    // Added, each says what it holds of the two, and nothing of its size.
    assert_eq!(carried(&rig, "board.item.created", &quiet).await, json!({ "sprint": null, "notes": null }));
    assert_eq!(
        carried(&rig, "board.item.created", &spring).await,
        json!({ "sprint": "spring", "notes": null })
    );
    // A long text is cut, where a letter ends, and says so by how it ends.
    let notes = carried(&rig, "board.item.created", &autumn).await["notes"].as_str().unwrap().to_string();
    assert!(notes.len() <= 1024 && notes.ends_with('…'), "{} bytes: {notes}", notes.len());
    assert!(long.starts_with(notes.trim_end_matches('…')), "{notes}");

    // Moved, each is heard by the handler its sprint chooses and no other.
    for id in [&quiet, &spring, &autumn] {
        change(&rig, &format!("/api/board/items/{id}"), json!({ "status": "doing" })).await;
    }
    assert_eq!(carried(&rig, "board.item.moved", &quiet).await, json!({ "sprint": null, "notes": null }));
    assert_eq!(rig.event("shop.autumn", |_| true).await.data["card"], json!(autumn));
    assert_eq!(rig.event("shop.spring", |_| true).await.data["card"], json!(spring));

    // Changed in what it does not carry, it says what it does.
    change(&rig, &format!("/api/board/items/{spring}"), json!({ "fields": { "size": "large" } })).await;
    let updated = rig.event("board.item.updated", |event| event.data["id"] == spring.as_str()).await;
    assert_eq!(updated.data["changed"], json!(["size"]), "{:?}", updated.data);
    assert_eq!(updated.data["fields"], json!({ "sprint": "spring", "notes": null }), "{:?}", updated.data);

    // A row of any table: added, changed, spoken on and taken away.
    let added = rig
        .send(
            "/api/collections/shop/tasks/records",
            json!({ "fields": { "title": "Weed", "area": "garden" } }),
        )
        .await;
    let task = added["id"].as_str().unwrap().to_string();
    change(&rig, &format!("/api/records/{task}"), json!({ "fields": { "effort": 3 } })).await;
    rig.send(&format!("/api/records/{task}/comments"), json!({ "body": "The beds first." })).await;
    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/records/{task}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    for topic in ["record.created", "record.updated", "record.commented", "record.removed"] {
        assert_eq!(carried(&rig, topic, &task).await, json!({ "area": "garden" }), "{topic}");
    }

    // Nothing run for what no handler is for: the handlers' runs are the
    // two that were addressed to them.
    assert_eq!(rig.count_of("shop.autumn").await, 1);
    assert_eq!(rig.count_of("shop.spring").await, 1);
}

/// A person's moods, kept private, carrying how each day went.
const MOODS: &str = r#"
blueprint: moods
permissions: [projects, records]
steps:
  moods:
    do: collection.ensure
    with:
      name: moods
      reads: private:journal
      fields: { title: { type: text, required: true }, went: { type: text, said: true } }
      history: [record.*]
"#;

#[tokio::test]
async fn what_a_table_kept_private_carries_is_heard_by_nobody_it_is_kept_from() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Life" })).await;
    let applied = rig.send("/api/apply", json!({ "source": MOODS, "project": "life", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let made =
        rig.send("/api/credentials", json!({ "name": "helper", "scopes": kernel::principal::AGENT_SCOPES }));
    let helper = made.await["token"].as_str().unwrap().to_string();
    rig.send(
        "/api/collections/life/moods/records",
        json!({ "fields": { "title": "Saturday", "went": "marmalade" } }),
    )
    .await;
    change(&rig, "/api/records/life-moods-1", json!({ "fields": { "went": "marmalade again" } })).await;
    rig.event("record.updated", |event| event.data["id"] == "life-moods-1").await;

    let recent = "/api/events/recent?topics=record.**&limit=100";
    let owner = rig.read(recent).await.to_string();
    assert!(owner.contains("marmalade again") && owner.contains("\"fields\""), "its owner hears it: {owner}");
    let (status, heard) = rig.call(Method::GET, recent, Some(&helper), None).await;
    assert_eq!(status, StatusCode::OK, "{heard}");
    let heard = heard.to_string();
    assert!(!heard.contains("marmalade") && !heard.contains("life-moods-1"), "{heard}");
}

#[tokio::test]
async fn a_field_kept_for_itself_or_guarded_is_never_carried_and_a_table_that_carries_nothing_says_what_it_did()
 {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    for (field, why) in [
        ("{ type: text, quiet: true, said: true }", "quiet"),
        ("{ type: text, takes: shell, said: true }", "takes"),
        ("{ type: flag, takes: { true: shell }, said: true }", "takes"),
    ] {
        let source = format!(
            "blueprint: guarded\npermissions: [projects, records]\nsteps:\n  t:\n    do: collection.ensure\n    \
             with: {{ name: kept, fields: {{ title: {{ type: text }}, held: {field} }} }}\n"
        );
        let (status, refused) = rig
            .call(
                Method::POST,
                "/api/apply",
                Some(&rig.token),
                Some(json!({ "source": source, "project": "shop", "wait": true })),
            )
            .await;
        let said = refused.to_string();
        assert!(
            status != StatusCode::OK || refused["finished"]["status"] != "ok",
            "{why}: a field that is {why} is refused `said`: {refused}"
        );
        assert!(said.contains("says `said`") && said.contains("never carried"), "{why}: {said}");
    }

    // Saying it of nothing, a table's events say what they always did.
    let plain = "blueprint: plain\npermissions: [projects, records]\nsteps:\n  t:\n    do: collection.ensure\n    \
                 with: { name: plain, fields: { title: { type: text }, area: { type: text } } }\n";
    let applied = rig.send("/api/apply", json!({ "source": plain, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    rig.send(
        "/api/collections/shop/plain/records",
        json!({ "fields": { "title": "One", "area": "garden" } }),
    )
    .await;
    change(&rig, "/api/records/shop-plain-1", json!({ "fields": { "area": "kitchen" } })).await;
    let created = rig.event("record.created", |event| event.data["id"] == "shop-plain-1").await;
    let keys: Vec<&String> = created.data.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["collection", "id", "project"], "{:?}", created.data);
    let updated = rig.event("record.updated", |event| event.data["id"] == "shop-plain-1").await;
    assert!(updated.data.get("fields").is_none(), "{:?}", updated.data);
}
