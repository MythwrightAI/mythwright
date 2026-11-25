//! How many things were said on each record of a table is one question,
//! answered for the table: what a list of records shows of each without
//! asking for each one's history. It counts what was said and nothing
//! else that happened, of that table's records and no other's, and is
//! read by whoever may read the records.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Sprints that keep what is said on them and what is changed of them,
/// and notes that are kept private.
const PLAN: &str = r#"
blueprint: plan
permissions: [projects, records]
steps:
  sprints:
    do: collection.ensure
    with:
      name: sprints
      fields: { title: { type: text, required: true }, size: { type: number } }
      history: { record.updated: "{actor} changed its {changed}", record.commented: "{actor} said: {body}" }
  notes:
    do: collection.ensure
    with:
      name: notes
      reads: private:notes
      fields: { title: { type: text, required: true } }
      history: [record.*]
"#;

async fn applied(rig: &Rig, asked: Value) {
    let applied = rig.send("/api/apply", asked).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// Says `words` on `record` and waits until the line is in its history:
/// the count is of lines, and a line is written after what it is of.
async fn say(rig: &Rig, record: &str, words: &str, public: bool) {
    rig.send(&format!("/api/records/{record}/comments"), json!({ "body": words, "public": public })).await;
    let (record, words) = (record.to_string(), words.to_string());
    rig.event("collection.history", move |event| {
        event.data["record"] == record.as_str()
            && event.data["said"].as_str().is_some_and(|said| said.contains(&words))
    })
    .await;
}

async fn counted(rig: &Rig, project: &str, table: &str) -> Value {
    rig.read(&format!("/api/collections/{project}/{table}/commented")).await["commented"].clone()
}

#[tokio::test]
async fn how_many_things_were_said_on_each_record_of_a_table_is_answered_for_the_table_and_counts_nothing_else()
 {
    let rig = Rig::new().await;
    // Two projects, the second called so that everything of it is known
    // by an id that begins as a card of the first's does.
    for name in ["Shop", "Shop 2"] {
        rig.send("/api/projects", json!({ "name": name })).await;
    }
    let dir = rig.node().app.studio.projects.dir_of("shop").unwrap();
    let _ = std::fs::remove_dir_all(dir.join("docs"));
    std::fs::create_dir_all(dir.join("docs/guides")).unwrap();
    for (path, text) in [
        ("docs/welcome.md", "# Welcome\n"),
        ("docs/guides/install.md", "# Installing it\n"),
        ("docs/quiet.md", "# Quiet\n"),
    ] {
        std::fs::write(dir.join(path), text).unwrap();
    }
    applied(&rig, json!({ "blueprint": "std/docs", "project": "shop", "wait": true })).await;
    applied(&rig, json!({ "source": PLAN, "project": "shop", "wait": true })).await;
    for project in ["shop", "shop-2"] {
        applied(&rig, json!({ "blueprint": "std/board", "project": project, "wait": true })).await;
        rig.send(&format!("/api/board/{project}/items"), json!({ "title": "Sell mugs" })).await;
    }
    for title in ["November", "December"] {
        rig.send("/api/collections/shop/sprints/records", json!({ "fields": { "title": title } })).await;
    }

    // Nothing was said yet: no record is named, by any table.
    for table in ["docs", "sprints", "board"] {
        assert_eq!(counted(&rig, "shop", table).await, json!({}), "{table}");
    }

    say(&rig, "shop-docs:guides:install", "Does this cover Windows?", false).await;
    say(&rig, "shop-docs:guides:install", "It does now.", true).await;
    say(&rig, "shop-docs:welcome", "A good start.", false).await;
    say(&rig, "shop-sprints-2", "Too short.", false).await;
    say(&rig, "shop-1", "Which mugs?", false).await;
    say(&rig, "shop-2-1", "The blue ones.", false).await;
    say(&rig, "shop-2-1", "And the green.", false).await;
    // Something that happened to a record, kept in its history, is not something said on it.
    let grown = json!({ "fields": { "size": 8 } });
    let (status, changed) =
        rig.call(Method::PATCH, "/api/records/shop-sprints-2", Some(&rig.token), Some(grown)).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    rig.event("collection.history", |event| {
        event.data["record"] == "shop-sprints-2" && event.data["topic"] == "record.updated"
    })
    .await;

    // Each table answers for its own records: a document by where it is,
    // marked public or not; a card of one project and not the card of
    // another whose id begins the same, nor a sprint of its own project.
    assert_eq!(
        counted(&rig, "shop", "docs").await,
        json!({ "shop-docs:guides:install": 2, "shop-docs:welcome": 1 }),
        "a document nothing was said on is not named"
    );
    assert_eq!(counted(&rig, "shop", "sprints").await, json!({ "shop-sprints-2": 1 }));
    assert_eq!(counted(&rig, "shop", "board").await, json!({ "shop-1": 1 }));
    assert_eq!(counted(&rig, "shop-2", "board").await, json!({ "shop-2-1": 2 }));
    // And it is what each record's own history holds of things said.
    let history = rig.read("/api/records/shop-docs:guides:install/history").await;
    let said = history["history"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|line| line["topic"] == "record.commented")
        .count();
    assert_eq!(said, 2, "{history}");
}

#[tokio::test]
async fn how_many_things_were_said_is_told_only_to_whoever_may_read_the_tables_records() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    applied(&rig, json!({ "source": PLAN, "project": "shop", "wait": true })).await;
    for table in ["sprints", "notes"] {
        let records = format!("/api/collections/shop/{table}/records");
        rig.send(&records, json!({ "fields": { "title": "November" } })).await;
    }
    say(&rig, "shop-sprints-1", "Too short.", false).await;
    say(&rig, "shop-notes-1", "The spare key is under the mat.", true).await;
    let key = |scopes: &'static [&'static str]| {
        let rig = &rig;
        async move {
            let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
            made["token"].as_str().unwrap().to_string()
        }
    };
    let (sprints, notes) =
        ("/api/collections/shop/sprints/commented", "/api/collections/shop/notes/commented");

    // Its owner is told of both.
    assert_eq!(rig.read(sprints).await["commented"], json!({ "shop-sprints-1": 1 }));
    assert_eq!(rig.read(notes).await["commented"], json!({ "shop-notes-1": 1 }));

    // A key that reads records is told of the table it may read, and of
    // the one kept private nothing, in the words its records are refused
    // in: not that a note was commented on, nor which.
    let reader = key(&["projects", "records:read"]).await;
    let (status, told) = rig.call(Method::GET, sprints, Some(&reader), None).await;
    assert_eq!((status, &told["commented"]), (StatusCode::OK, &json!({ "shop-sprints-1": 1 })), "{told}");
    let (status, refused) = rig.call(Method::GET, notes, Some(&reader), None).await;
    let (listing, listed) =
        rig.call(Method::GET, "/api/collections/shop/notes/records", Some(&reader), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!((status, &refused), (listing, &listed), "refused as its records are");
    assert!(!refused.to_string().contains("shop-notes-1"), "{refused}");

    // A key that reads no records is told of neither.
    let other = key(&["projects", "channels"]).await;
    for path in [sprints, notes] {
        let (status, refused) = rig.call(Method::GET, path, Some(&other), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {refused}");
        assert!(refused.get("commented").is_none(), "{refused}");
    }
    // Nor anybody who is not signed in; and a table there is not is not there.
    let (status, _) = rig.call(Method::GET, sprints, None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, missing) =
        rig.call(Method::GET, "/api/collections/shop/nothing/commented", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");
}
