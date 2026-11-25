//! Somebody named with `@` in what is said on a record (`talk::commented`,
//! `studio::named`): a line in their box, by every door that says
//! something on a record, where they may read the record as things stand.
//!
//! One journey in one factory, with its owner, two people and keys of
//! theirs that hold less: a comment on a card names both people, and
//! each is told but whoever said it, the line keeping no words and
//! handed with them from the card's history, which holds the comment at
//! the place the line names; a key of the one told that reads her box and
//! not the board is handed no such line and counts none; a comment on a
//! record of a table kept private names her and tells her nothing; a
//! comment said with a key of the other names its maker and the first,
//! and only the first is told; and a blueprint's step that says something
//! on a record names her the same way.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{key, person};

/// Notes kept private, where what is said is kept.
const NOTES: &str = r#"
blueprint: notes
permissions: [projects, records]
steps:
  notes:
    do: collection.ensure
    with:
      name: notes
      reads: private:notes
      fields: { title: { type: text, required: true } }
      history: [record.*]
"#;

/// A blueprint whose step says something on a card.
fn saying(card: &str, body: &str) -> String {
    format!(
        "blueprint: says\npermissions: [board]\nsteps:\n  say:\n    do: record.comment\n    with: {{ id: {card}, body: \"{body}\" }}\n"
    )
}

/// What `token`'s box holds that is of something said on a record: its lines, and how many of all wait unread.
async fn commented(rig: &Rig, token: &str) -> (Vec<Value>, u64) {
    let (status, found) = rig.call(Method::GET, "/api/inbox?all=true", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    let lines = found["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|line| line["kind"] == "commented")
        .cloned()
        .collect();
    (lines, found["unread"].as_u64().unwrap())
}

/// Waits until `owner` is told a line of something said on `record`.
async fn told(rig: &Rig, owner: &str, record: &str) {
    let (owner, record) = (owner.to_string(), record.to_string());
    rig.event("inbox.told", move |event| {
        event.data["owner"] == owner.as_str()
            && event.data["kind"] == "commented"
            && event.data["place"] == record.as_str()
    })
    .await;
}

/// Says `body` on `record` as `token`.
async fn say(rig: &Rig, token: &str, record: &str, body: &str) {
    let (status, said) = rig
        .call(
            Method::POST,
            &format!("/api/records/{record}/comments"),
            Some(token),
            Some(json!({ "body": body })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{said}");
}

#[tokio::test]
async fn somebody_named_on_a_record_they_may_read_is_told_in_their_box_by_every_door_and_nobody_else_is() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell) = (person(&rig, "Mira").await, person(&rig, "Nell").await);
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    for asked in [
        json!({ "blueprint": "std/board", "project": "shop", "wait": true }),
        json!({ "source": NOTES, "project": "shop", "wait": true }),
    ] {
        let applied = rig.send("/api/apply", asked).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    }
    let card = rig.send("/api/board/shop/items", json!({ "title": "Fix the login" })).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    // A key of Nell's that reads her box and not the board, and one of Mira's that writes on the board.
    let boxed = key(&rig, &nell.token, &["inbox:read"]).await;
    let writes = key(&rig, &mira.token, &["board", "inbox"]).await;

    // 1: the owner names both on the card; each is told, and the owner, though they read everything, is not.
    say(&rig, &rig.token, &card, "@nell and @mira, the login fails on Safari").await;
    told(&rig, &nell.actor, &card).await;
    told(&rig, &mira.actor, &card).await;
    let (lines, unread) = commented(&rig, &nell.token).await;
    assert_eq!((lines.len(), unread), (1, 1), "{lines:?}");
    let line = &lines[0];
    assert_eq!((&line["place"], &line["why"], line.get("says")), (&json!(card), &json!("named"), None));
    let on = &line["on_record"];
    assert_eq!(
        (&on["record"], &on["project"], &on["table"], &on["title"]),
        (&json!(card), &json!("shop"), &json!("board"), &json!("Fix the login"))
    );
    assert_eq!(on["body"], "@nell and @mira, the login fails on Safari");
    // The line is about the comment by its place in the card's history, which holds it there.
    assert_eq!(line["about"], json!(on["seq"].as_u64().unwrap().to_string()));
    let history = rig.read(&format!("/api/records/{card}/history")).await;
    let kept =
        history["history"].as_array().unwrap().iter().find(|one| one["seq"] == on["seq"]).cloned().unwrap();
    assert_eq!(kept["topic"], "record.commented");
    assert_eq!(commented(&rig, &rig.token).await.0, Vec::<Value>::new());
    // What is said of her box carries no words.
    let event = rig.event("inbox.told", |event| event.data["kind"] == "commented").await;
    assert!(!event.data.to_string().contains("Safari"));

    // 2: her key that holds no reading of the board is handed no such line, and counts none.
    assert_eq!(commented(&rig, &boxed).await, (Vec::new(), 0));
    let id = line["id"].as_str().unwrap();
    let (status, _) = rig.call(Method::GET, &format!("/api/inbox/{id}"), Some(&boxed), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // 3: named on a note kept private, she is told nothing; what is said next on the card, which tells Mira, is after it.
    let note = rig
        .send("/api/collections/shop/notes/records", json!({ "fields": { "title": "Pricing" } }))
        .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    say(&rig, &rig.token, &note, "@nell the prices for next year").await;
    say(&rig, &rig.token, &card, "@mira one more thing").await;
    let to = |who: &str, unread: u64| {
        let who = who.to_string();
        move |event: &kernel::Event| {
            event.data["owner"] == who.as_str()
                && event.data["kind"] == "commented"
                && event.data["unread"] == unread
        }
    };
    rig.event("inbox.told", to(&mira.actor, 2)).await;
    assert!(commented(&rig, &nell.token).await.0.iter().all(|line| line["place"] == json!(card)));
    assert_eq!(commented(&rig, &nell.token).await.0.len(), 1);

    // 4: said with a key of Mira's, it names its maker and Nell: only Nell is told.
    say(&rig, &writes, &card, "@mira @nell it is fixed").await;
    rig.event("inbox.told", to(&nell.actor, 2)).await;
    let (lines, _) = commented(&rig, &nell.token).await;
    let newest = &lines[0];
    assert_eq!(newest["on_record"]["body"], "@mira @nell it is fixed");
    assert!(newest["from"].as_str().unwrap().starts_with("key:"), "{newest}");
    assert_eq!(commented(&rig, &mira.token).await.0.len(), 2);

    // 5: a blueprint's step says something on the card that names her: the same.
    let applied = rig
        .send(
            "/api/apply",
            json!({ "source": saying(&card, "@nell from a blueprint"), "project": "shop", "wait": true }),
        )
        .await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    rig.event("inbox.told", to(&nell.actor, 3)).await;
    let (lines, unread) = commented(&rig, &nell.token).await;
    assert_eq!((lines.len(), unread), (3, 3));
    assert_eq!(lines[0]["on_record"]["body"], "@nell from a blueprint");
}
