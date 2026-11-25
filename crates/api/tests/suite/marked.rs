//! What is said on a record is private unless whoever says it marks it
//! public: by the route, by a blueprint's step, and whatever key says it.
//! The mark is kept, said with the event and shown in the history, and it
//! is no way of reading anything: a table that is kept private is as kept
//! as it was.
//!
//! And what tells something on outside the factory asks what whoever did
//! it holds, so what is done with a key somebody made is said to be the
//! key's doing: something said on a record, a card added, a card changed.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Sprints that keep what is said on them, in their own words; notes that
/// are kept private and keep it in the usual words; and something that
/// hears only what was marked public on a sprint and says it on, as
/// whatever keeps a record somewhere else as well would.
const PLAN: &str = r#"
blueprint: plan
permissions: [projects, records]
steps:
  sprints:
    do: collection.ensure
    with:
      name: sprints
      fields: { title: { type: text, required: true } }
      history: { record.created: "{actor} planned it", record.commented: "{actor} said: {body}" }
  notes:
    do: collection.ensure
    with:
      name: notes
      reads: private:notes
      fields: { title: { type: text, required: true } }
      history: [record.*]
on:
  marked:
    event: record.commented
    for: { project: shop, collection: sprints, public: true }
    steps:
      tell:
        do: emit
        with: { topic: shop.told, record: "${event.data.id}", body: "${event.data.body}" }
"#;

fn message(answer: &Value) -> &str {
    answer["error"]["message"].as_str().unwrap_or_default()
}

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// The project, its tables, a sprint and a note.
async fn planned() -> Rig {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = rig.send("/api/apply", json!({ "source": PLAN, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    for table in ["sprints", "notes"] {
        let records = format!("/api/collections/shop/{table}/records");
        rig.send(&records, json!({ "fields": { "title": "November" } })).await;
    }
    rig
}

/// What was said on a record, oldest first, as its events said it: the
/// words, and the mark as it was said (nothing, where none was).
async fn said_on(rig: &Rig, record: &str) -> Vec<(String, Value)> {
    let query = journal::EventQuery { topics: "record.commented".into(), limit: 1000, ..Default::default() };
    let mut events = rig.said(query).await;
    events.sort_by_key(|event| event.seq);
    events
        .iter()
        .filter(|event| event.data["id"] == record)
        .map(|event| (event.data["body"].as_str().unwrap().to_string(), event.data["public"].clone()))
        .collect()
}

/// The lines of a record's history that are of something said, once there
/// are `lines` of them: the sentence, and the mark the line keeps.
async fn history(rig: &Rig, record: &str, lines: usize) -> Vec<(String, Value)> {
    let mut written = rig.follow("collection.history");
    crate::soon(&format!("{lines} things said in the history of {record}"), async {
        loop {
            let kept = rig.read(&format!("/api/records/{record}/history")).await;
            let said: Vec<(String, Value)> = kept["history"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|line| line["topic"] == "record.commented")
                .map(|line| (line["said"].as_str().unwrap().to_string(), line["data"]["public"].clone()))
                .collect();
            if said.len() >= lines {
                return said;
            }
            written.next().await.unwrap();
        }
    })
    .await
}

#[tokio::test]
async fn what_is_said_on_a_record_is_private_unless_it_is_marked_public_and_the_event_and_the_history_say_which()
 {
    let rig = planned().await;
    let sprint = "/api/records/shop-sprints-1/comments";

    // Said with nothing more, it is private: the event says no mark.
    rig.send(sprint, json!({ "body": "Two weeks is too short." })).await;
    // Marked, it says so; and said not to be, it is as if nothing was said.
    let on = rig.send(sprint, json!({ "body": "Three it is.", "public": true })).await;
    assert_eq!(on["id"], "shop-sprints-1", "the record, as it always answered: {on}");
    rig.send(sprint, json!({ "body": "Between us.", "public": false })).await;
    rig.send(sprint, json!({ "body": "Nothing said of it.", "public": null })).await;
    assert_eq!(
        said_on(&rig, "shop-sprints-1").await,
        [
            ("Two weeks is too short.".to_string(), Value::Null),
            ("Three it is.".to_string(), json!(true)),
            ("Between us.".to_string(), Value::Null),
            ("Nothing said of it.".to_string(), Value::Null),
        ]
    );

    // A mark that is neither is refused, and nothing is said: what was
    // meant to be kept in does not go out on a guess.
    for mark in [json!("yes"), json!("false"), json!(1), json!([])] {
        let words = json!({ "body": "Which is it?", "public": mark });
        let (status, refused) = rig.call(Method::POST, sprint, Some(&rig.token), Some(words)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        assert!(
            message(&refused).starts_with("`public` says whether what is said is marked public")
                && message(&refused).ends_with("Left out, it is private."),
            "{refused}"
        );
    }
    assert_eq!(rig.count_of("record.commented").await, 4);

    // The history keeps the mark, and the line says it whatever words the
    // table has for something said.
    assert_eq!(
        history(&rig, "shop-sprints-1", 4).await,
        [
            ("Ada said: Two weeks is too short.".to_string(), Value::Null),
            ("Ada said: Three it is. (public)".to_string(), json!(true)),
            ("Ada said: Between us.".to_string(), Value::Null),
            ("Ada said: Nothing said of it.".to_string(), Value::Null),
        ]
    );

    // What hears only what was marked heard one thing.
    let told = rig.event("shop.told", |_| true).await;
    assert_eq!(told.data["body"], "Three it is.", "{:?}", told.data);
    assert_eq!(rig.count_of("shop.told").await, 1);
}

#[tokio::test]
async fn marking_what_is_said_takes_what_saying_it_takes_and_a_step_marks_it_as_a_person_does() {
    let rig = planned().await;
    let sprint = "/api/records/shop-sprints-1/comments";
    let words = json!({ "body": "Three it is.", "public": true });

    // A key that may not say anything on a sprint may not say it
    // publicly either, and is told what it always was.
    let reader = key(&rig, &["records:read", "board"]).await;
    let (status, refused) = rig.call(Method::POST, sprint, Some(&reader), Some(words.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(message(&refused).contains("needs `records:write`"), "{refused}");
    assert_eq!(rig.count_of("record.commented").await, 0);

    // One that may, marks it: the mark asks nothing more of whoever says it.
    let writer = key(&rig, &["records"]).await;
    let (status, on) = rig.call(Method::POST, sprint, Some(&writer), Some(words)).await;
    assert_eq!(status, StatusCode::OK, "{on}");

    // A blueprint's step says something each way, as whoever applied it.
    let steps = r#"
blueprint: season
permissions: [records]
steps:
  kept:
    do: record.comment
    with: { id: shop-sprints-1, body: "The season says nothing of it." }
  told:
    do: record.comment
    with: { id: shop-sprints-1, body: "The season says so.", public: true }
"#;
    let applied = rig.send("/api/apply", json!({ "source": steps, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(
        said_on(&rig, "shop-sprints-1").await,
        [
            ("Three it is.".to_string(), json!(true)),
            ("The season says nothing of it.".to_string(), Value::Null),
            ("The season says so.".to_string(), json!(true)),
        ]
    );
    assert_eq!(
        history(&rig, "shop-sprints-1", 3).await,
        [
            ("Ada (with the key narrow) said: Three it is. (public)".to_string(), json!(true)),
            ("conductor said: The season says nothing of it.".to_string(), Value::Null),
            ("conductor said: The season says so. (public)".to_string(), json!(true)),
        ]
    );
    // A mark that is no flag is refused where the blueprint is read.
    let wrong = steps.replace("public: true", "public: sometimes");
    let (status, refused) = rig
        .call(
            Method::POST,
            "/api/apply",
            Some(&rig.token),
            Some(json!({ "source": wrong, "project": "shop" })),
        )
        .await;
    assert_ne!(status, StatusCode::OK, "{refused}");
    assert!(message(&refused).contains("public"), "{refused}");
}

#[tokio::test]
async fn what_is_marked_public_on_a_record_of_a_table_kept_private_is_read_and_heard_by_nobody_it_was_kept_from()
 {
    let rig = planned().await;
    // Everything of the project but what the notes are kept under.
    let helper = key(&rig, &["projects", "channels", "board", "records", "blueprints:read", "search"]).await;
    let note = "/api/records/shop-notes-1/comments";
    rig.send(note, json!({ "body": "The spare key is under the mat.", "public": true })).await;
    rig.send("/api/records/shop-sprints-1/comments", json!({ "body": "Three it is.", "public": true })).await;

    // Its owner reads it in the history, in the usual words, which say
    // everything the event does and the mark with it.
    let lines = history(&rig, "shop-notes-1", 1).await;
    assert_eq!(lines[0].1, json!(true), "{lines:?}");
    assert!(
        lines[0].0.contains("The spare key is under the mat.")
            && lines[0].0.contains("public true")
            && !lines[0].0.ends_with("(public)"),
        "{lines:?}"
    );

    // Whoever the table is kept from is refused its history as before,
    // says nothing on it, marked or not, and hears nothing of it.
    let (status, refused) =
        rig.call(Method::GET, "/api/records/shop-notes-1/history", Some(&helper), None).await;
    assert_ne!(status, StatusCode::OK, "{refused}");
    assert!(!refused.to_string().contains("spare key"), "{refused}");
    for mark in [true, false] {
        let words = json!({ "body": "And the other one?", "public": mark });
        let (status, refused) = rig.call(Method::POST, note, Some(&helper), Some(words)).await;
        assert_ne!(status, StatusCode::OK, "{refused}");
    }
    // What follows from the sprint's has happened by now: its line, and
    // what hears what was marked.
    history(&rig, "shop-sprints-1", 1).await;
    rig.event("shop.told", |_| true).await;
    let recent = "/api/events/recent?topics=record.**,collection.history,shop.told&limit=100";
    let (status, heard) = rig.call(Method::GET, recent, Some(&helper), None).await;
    assert_eq!(status, StatusCode::OK, "{heard}");
    let heard = heard.to_string();
    assert!(
        heard.contains("Ada said: Three it is. (public)") && heard.contains("shop.told"),
        "what it may read, it hears, and the line of its history with it: {heard}"
    );
    // Not what was said on the note, nor the line that was written of it
    // in the note's history, which says the same in words.
    assert!(!heard.contains("spare key") && !heard.contains("shop-notes-1"), "{heard}");
    let owner = rig.read(recent).await.to_string();
    assert!(owner.contains("spare key") && owner.contains("public true"), "its owner hears both: {owner}");
    assert_eq!(rig.count_of("record.commented").await, 2);
    assert_eq!(rig.count_of("shop.told").await, 1, "and only what was marked on a sprint was said on");
}

/// Hears what was marked public on a sprint and asks what whoever said it
/// holds, as something that tells it on outside the factory would before
/// it did: only what was said by somebody who may write the project's
/// files goes out.
const OUTWARD: &str = r#"
blueprint: outward
permissions: [records, projects]
on:
  marked:
    event: record.commented
    for: { project: shop, collection: sprints, public: true }
    steps:
      may:
        do: actor.holds
        with: { actor: "${event.source.actor}", scope: "projects:write" }
      tell:
        do: emit
        with: { topic: shop.asked, body: "${event.data.body}", by: "${event.data.by}", holds: "${steps.may.out.holds}", known: "${steps.may.out.known}" }
"#;

#[tokio::test]
async fn what_is_said_with_a_key_is_said_by_the_key_and_whatever_asks_what_the_speaker_holds_is_answered_for_it()
 {
    let rig = planned().await;
    let applied = rig.send("/api/apply", json!({ "source": OUTWARD, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let sprint = "/api/records/shop-sprints-1/comments";
    // A key of the owner's that may say things on records and no more:
    // its maker holds what writing the project's files takes, and it
    // does not.
    let made = rig.send("/api/credentials", json!({ "name": "notes", "scopes": ["records"] })).await;
    let (key, id) = (made["token"].as_str().unwrap().to_string(), made["id"].as_str().unwrap().to_string());
    let (status, on) = rig
        .call(Method::POST, sprint, Some(&key), Some(json!({ "body": "By the key.", "public": true })))
        .await;
    assert_eq!(status, StatusCode::OK, "{on}");
    let by_key = rig.event("shop.asked", |event| event.data["body"] == "By the key.").await;
    assert_eq!(
        (&by_key.data["by"], &by_key.data["holds"], &by_key.data["known"]),
        (&json!(format!("key:{id}")), &json!(false), &json!(true)),
        "asked of the key, which holds less than whoever made it: {:?}",
        by_key.data
    );
    // The event names the key, in who it says said it and in `by`.
    let said = rig.event("record.commented", |event| event.data["body"] == "By the key.").await;
    assert_eq!(said.source.actor.as_deref(), Some(format!("key:{id}").as_str()), "{:?}", said.source);

    // The same person, signed in, says it as themselves, and holds it.
    rig.send(sprint, json!({ "body": "By hand.", "public": true })).await;
    let by_hand = rig.event("shop.asked", |event| event.data["body"] == "By hand.").await;
    assert_eq!(
        (&by_hand.data["holds"], &by_hand.data["known"]),
        (&json!(true), &json!(true)),
        "{:?}",
        by_hand.data
    );
    assert!(by_hand.data["by"].as_str().is_some_and(|by| by.starts_with("user:")), "{:?}", by_hand.data);

    // And the history names both in words a person can read.
    assert_eq!(
        history(&rig, "shop-sprints-1", 2).await,
        [
            ("Ada (with the key notes) said: By the key. (public)".to_string(), json!(true)),
            ("Ada said: By hand. (public)".to_string(), json!(true)),
        ]
    );
}

/// Hears a card added and a card changed, and asks what whoever did it
/// holds, as something that keeps a card somewhere outside the factory
/// as well would before it sent one there.
const LINKS: &str = r#"
blueprint: links
permissions: [board, projects]
on:
  added:
    event: board.item.created
    for: { project: shop }
    steps:
      may:
        do: actor.holds
        with: { actor: "${event.source.actor}", scope: "projects:write" }
      tell:
        do: emit
        with: { topic: shop.asked, of: added, card: "${event.data.id}", by: "${event.data.by}", holds: "${steps.may.out.holds}", known: "${steps.may.out.known}" }
  changed:
    event: board.item.updated
    for: { project: shop }
    steps:
      may:
        do: actor.holds
        with: { actor: "${event.source.actor}", scope: "projects:write" }
      tell:
        do: emit
        with: { topic: shop.asked, of: "${event.data.changed}", card: "${event.data.id}", by: "${event.data.by}", holds: "${steps.may.out.holds}", known: "${steps.may.out.known}" }
"#;

#[tokio::test]
async fn a_card_added_or_changed_with_a_key_is_said_to_be_the_keys_doing_and_whoever_asks_what_it_holds_is_answered_for_it()
 {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let applied = rig.send("/api/apply", json!({ "source": LINKS, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    // A key of the owner's for the board, and to apply a blueprint: its
    // maker holds what writing the project's files takes, and it does not.
    let made =
        rig.send("/api/credentials", json!({ "name": "cards", "scopes": ["board", "blueprints"] })).await;
    let (key, id) = (made["token"].as_str().unwrap().to_string(), made["id"].as_str().unwrap().to_string());
    let the_key = format!("key:{id}");
    let asked = |of: Value, card: &'static str| {
        let rig = &rig;
        async move {
            rig.event("shop.asked", move |event| event.data["of"] == of && event.data["card"] == card).await
        }
    };

    // Added with the key, it is the key that added it.
    let (status, added) = rig
        .call(Method::POST, "/api/board/shop/items", Some(&key), Some(json!({ "title": "Sell mugs" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let said = rig.event("board.item.created", |event| event.data["id"] == "shop-1").await;
    assert_eq!(
        (said.source.actor.as_deref(), &said.data["by"]),
        (Some(the_key.as_str()), &json!(the_key)),
        "{:?} {:?}",
        said.source,
        said.data
    );
    let by_key = asked(json!("added"), "shop-1").await;
    assert_eq!(
        (&by_key.data["by"], &by_key.data["holds"], &by_key.data["known"]),
        (&json!(the_key), &json!(false), &json!(true)),
        "asked of the key, which holds less than whoever made it: {:?}",
        by_key.data
    );

    // Changed with it, in what it says and in where it stands in its
    // column, the same.
    let labels = json!({ "labels": ["out"] });
    let (status, changed) =
        rig.call(Method::PATCH, "/api/board/items/shop-1", Some(&key), Some(labels)).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    let by_key = asked(json!(["labels"]), "shop-1").await;
    assert_eq!(
        (&by_key.data["by"], &by_key.data["holds"], &by_key.data["known"]),
        (&json!(the_key), &json!(false), &json!(true)),
        "{:?}",
        by_key.data
    );
    rig.send("/api/board/shop/items", json!({ "title": "Pack boxes" })).await;
    let place = json!({ "status": "draft", "before": "shop-1" });
    let (status, placed) = rig.call(Method::PATCH, "/api/board/items/shop-2", Some(&key), Some(place)).await;
    assert_eq!(status, StatusCode::OK, "{placed}");
    let by_key = asked(json!(["position"]), "shop-2").await;
    assert_eq!(
        (&by_key.data["by"], &by_key.data["holds"]),
        (&json!(the_key), &json!(false)),
        "{:?}",
        by_key.data
    );

    // The same person, signed in, does each as themselves, and holds it.
    let by_hand = asked(json!("added"), "shop-2").await;
    assert_eq!(
        (&by_hand.data["holds"], &by_hand.data["known"]),
        (&json!(true), &json!(true)),
        "{:?}",
        by_hand.data
    );
    assert!(by_hand.data["by"].as_str().is_some_and(|by| by.starts_with("user:")), "{:?}", by_hand.data);
    rig.call(
        Method::PATCH,
        "/api/board/items/shop-2",
        Some(&rig.token),
        Some(json!({ "body": "Twelve to a box." })),
    )
    .await;
    let by_hand = asked(json!(["body"]), "shop-2").await;
    assert_eq!(
        (&by_hand.data["holds"], &by_hand.data["known"]),
        (&json!(true), &json!(true)),
        "{:?}",
        by_hand.data
    );

    // What a blueprint's step adds and changes is said as it always was:
    // by whoever applied it, whatever they applied it with.
    let steps = r#"
blueprint: stocker
permissions: [board]
steps:
  add:
    do: board.add
    with: { project: shop, title: "Count stock" }
  change:
    do: board.update
    with: { id: "${steps.add.out.id}", labels: [counted] }
"#;
    let (status, ran) = rig
        .call(
            Method::POST,
            "/api/apply",
            Some(&key),
            Some(json!({ "source": steps, "project": "shop", "wait": true })),
        )
        .await;
    assert_eq!((status, &ran["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{ran}");
    let stocked = asked(json!("added"), "shop-3").await;
    assert!(stocked.data["by"].as_str().is_some_and(|by| by.starts_with("user:")), "{:?}", stocked.data);

    // And the card's history names each in words a person can read.
    let mut written = rig.follow("collection.history");
    let lines = crate::soon("the card's history", async {
        loop {
            let kept = rig.read("/api/records/shop-1/history").await;
            let said: Vec<String> = kept["history"]
                .as_array()
                .unwrap()
                .iter()
                .map(|line| line["said"].as_str().unwrap().into())
                .collect();
            if said.len() >= 2 {
                return said;
            }
            written.next().await.unwrap();
        }
    })
    .await;
    assert_eq!(
        lines,
        ["Ada (with the key cards) added it to draft", "Ada (with the key cards) changed its labels"]
    );
}
