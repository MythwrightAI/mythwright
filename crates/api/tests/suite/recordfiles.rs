//! What is attached to a record, a card included (`studio::attaching`):
//! added by whoever may change it, read by whoever may read it, at every
//! door, and kept from whoever may not read a table kept private.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Notes that are kept private, beside the board.
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

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

fn message(answer: &Value) -> &str {
    answer["error"]["message"].as_str().unwrap_or_default()
}

/// Sends `bytes` as a file to be attached to `record`, with `token`.
async fn attach(
    rig: &Rig,
    token: &str,
    record: &str,
    name: &str,
    media: &str,
    bytes: &[u8],
) -> (StatusCode, Value) {
    let answer = rig
        .client
        .post(format!("{}/api/records/{record}/attachments?name={name}", rig.url))
        .bearer_auth(token)
        .header("content-type", media)
        .body(bytes.to_vec())
        .send()
        .await
        .unwrap();
    let status = answer.status();
    (status, answer.json().await.unwrap_or(Value::Null))
}

/// What is attached, read back at its address, with `token`: its status,
/// its kind and how it is handed over, and what it holds.
async fn fetched(rig: &Rig, token: &str, id: &str) -> (StatusCode, String, String, Vec<u8>) {
    let answer =
        rig.client.get(format!("{}/api/attachments/{id}", rig.url)).bearer_auth(token).send().await.unwrap();
    let header = |name: &str| {
        answer.headers().get(name).and_then(|value| value.to_str().ok()).unwrap_or_default().to_string()
    };
    let (status, kind, handed) = (answer.status(), header("content-type"), header("content-disposition"));
    (status, kind, handed, answer.bytes().await.unwrap().to_vec())
}

#[tokio::test]
async fn a_picture_attached_to_a_card_is_listed_read_back_and_said_in_its_history_and_taken_away() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig.send("/api/board/shop/items", json!({ "title": "Fix the till" })).await;

    let (status, kept) =
        attach(&rig, &rig.token, "shop-1", "till.png", "image/png", b"\x89PNG the till").await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    let id = kept["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("ratt_"), "{id}");
    assert_eq!(
        (kept["name"].as_str(), kept["media"].as_str(), kept["size"].as_u64()),
        (Some("till.png"), Some("image/png"), Some(13))
    );
    assert_eq!(kept["who"], "Ada");

    let listed = rig.read("/api/records/shop-1/attachments").await;
    assert_eq!(listed["attachments"].as_array().unwrap().len(), 1, "{listed}");
    // A picture is shown where it is read; a log is read in place too.
    let (status, kind, handed, bytes) = fetched(&rig, &rig.token, &id).await;
    assert_eq!(
        (status, kind.as_str(), bytes.as_slice()),
        (StatusCode::OK, "image/png", &b"\x89PNG the till"[..])
    );
    assert!(handed.starts_with("inline"), "{handed}");

    // Said in one write with its row, and a line of the card's history.
    let said = rig.event("record.attached", |event| event.data["id"] == "shop-1").await;
    assert_eq!(
        (said.data["name"].as_str(), said.data["attachment"].as_str()),
        (Some("till.png"), Some(id.as_str()))
    );
    rig.event("collection.history", |event| {
        event.data["record"] == "shop-1" && event.data["topic"] == "record.attached"
    })
    .await;
    let lines = rig.read("/api/records/shop-1/history").await;
    let attached =
        lines["history"].as_array().unwrap().iter().find(|line| line["topic"] == "record.attached");
    assert_eq!(attached.unwrap()["said"], "Ada attached till.png", "{lines}");

    // Nothing, and too much, is refused, and nothing is kept of either.
    let (status, refused) = attach(&rig, &rig.token, "shop-1", "empty.txt", "text/plain", b"").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let too_much = vec![b'x'; studio::attachments::MAX_ATTACHMENT_BYTES + 1];
    let (status, _) = attach(&rig, &rig.token, "shop-1", "big.log", "text/plain", &too_much).await;
    assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::PAYLOAD_TOO_LARGE, "{status}");
    assert_eq!(rig.read("/api/records/shop-1/attachments").await["attachments"].as_array().unwrap().len(), 1);

    // Taken away, it is said, and its file is gone.
    let path = format!("/api/records/shop-1/attachments/{id}");
    let (status, gone) = rig.call(Method::DELETE, &path, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    rig.event("record.detached", |event| event.data["attachment"] == id.as_str()).await;
    assert_eq!(fetched(&rig, &rig.token, &id).await.0, StatusCode::NOT_FOUND);
    assert!(rig.read("/api/records/shop-1/attachments").await["attachments"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn what_is_attached_is_added_by_whoever_may_change_the_record_and_read_by_whoever_may_read_it() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let applied = rig.send("/api/apply", json!({ "source": NOTES, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    rig.send("/api/board/shop/items", json!({ "title": "Fix the till" })).await;
    rig.send("/api/collections/shop/notes/records", json!({ "fields": { "title": "Who owes what" } })).await;
    let (_, card) = attach(&rig, &rig.token, "shop-1", "till.log", "text/plain", b"the till said no").await;
    let (_, note) = attach(&rig, &rig.token, "shop-notes-1", "owed.txt", "text/plain", b"a great deal").await;
    let (card, note) = (card["id"].as_str().unwrap().to_string(), note["id"].as_str().unwrap().to_string());

    // A key that may read the board and the records, and change neither:
    // it reads what is attached to the card, and adds nothing.
    let reader = key(&rig, &["board:read", "records:read"]).await;
    let listed = rig.call(Method::GET, "/api/records/shop-1/attachments", Some(&reader), None).await;
    assert_eq!(listed.0, StatusCode::OK, "{}", listed.1);
    let (status, _, _, bytes) = fetched(&rig, &reader, &card).await;
    assert_eq!((status, bytes.as_slice()), (StatusCode::OK, &b"the till said no"[..]));
    let (status, refused) = attach(&rig, &reader, "shop-1", "mine.txt", "text/plain", b"mine").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(message(&refused).contains("board:write"), "{refused}");
    let (status, _) = rig
        .call(Method::DELETE, &format!("/api/records/shop-1/attachments/{card}"), Some(&reader), None)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // The note is of a table kept private: whoever may not read it reads
    // nothing attached to it, at any door, nor hears that it was.
    for door in ["/api/records/shop-notes-1/attachments".to_string(), format!("/api/attachments/{note}")] {
        let (status, refused) = rig.call(Method::GET, &door, Some(&reader), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{door}: {refused}");
    }
    let heard = |events: &Value| -> Vec<String> {
        let events = events["events"].as_array().unwrap().iter();
        events.map(|event| event["data"]["id"].as_str().unwrap_or("-").to_string()).collect()
    };
    let topics = "/api/events/recent?topics=record.attached&limit=100";
    let (_, events) = rig.call(Method::GET, topics, Some(&reader), None).await;
    assert_eq!(heard(&events), ["shop-1"], "{events}");
    let all = rig.read(topics).await;
    assert_eq!(heard(&all).len(), 2, "{all}");

    // A key given the note's scope reads it.
    let private = key(&rig, &["records:read", "private:notes"]).await;
    let (status, _, _, bytes) = fetched(&rig, &private, &note).await;
    assert_eq!((status, bytes.as_slice()), (StatusCode::OK, &b"a great deal"[..]));
    // And a key that reads nothing of records reads nothing attached to one.
    let nothing = key(&rig, &["channels:read"]).await;
    assert_eq!(fetched(&rig, &nothing, &card).await.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_card_deleted_keeps_what_is_attached_in_the_trash_gives_it_back_with_it_and_takes_it_away_for_good()
{
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig.send("/api/board/shop/items", json!({ "title": "Fix the till" })).await;
    let (_, kept) = attach(&rig, &rig.token, "shop-1", "till.png", "image/png", b"\x89PNG").await;
    let id = kept["id"].as_str().unwrap().to_string();

    // In the Trash, nothing reads it.
    let (status, gone) = rig.call(Method::DELETE, "/api/board/items/shop-1", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let trash = gone["trash"].as_str().unwrap().to_string();
    assert_eq!(fetched(&rig, &rig.token, &id).await.0, StatusCode::NOT_FOUND);
    // Given back, it is there again.
    rig.send(&format!("/api/trash/{trash}/restore"), json!({})).await;
    let (status, _, _, bytes) = fetched(&rig, &rig.token, &id).await;
    assert_eq!((status, bytes.as_slice()), (StatusCode::OK, &b"\x89PNG"[..]));
    // Deleted for good, it goes with it.
    let (_, gone) = rig.call(Method::DELETE, "/api/board/items/shop-1", Some(&rig.token), None).await;
    let trash = gone["trash"].as_str().unwrap().to_string();
    let (status, forgot) =
        rig.call(Method::DELETE, &format!("/api/trash/{trash}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{forgot}");
    let files = rig.config.data_dir.join("attachments");
    assert!(!files.join(&id).exists(), "its file went with it");
}

#[tokio::test]
async fn a_blueprints_step_attaches_a_file_of_the_project_to_a_card_as_whoever_applied_it() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig.send("/api/board/shop/items", json!({ "title": "Fix the till" })).await;
    let written = json!({ "path": "shots/after.png", "content": "after" });
    let (status, _) = rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(written)).await;
    assert_eq!(status, StatusCode::OK);
    let source = kernel::asked::RECORD_ATTACH;
    let body = json!({ "source": source, "project": "shop", "params": { "card": "shop-1" }, "wait": true });
    let applied = rig.send("/api/apply", body).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let listed = rig.read("/api/records/shop-1/attachments").await;
    let one = &listed["attachments"][0];
    assert_eq!((one["name"].as_str(), one["size"].as_u64()), (Some("after.png"), Some(5)), "{listed}");
    let said = rig.event("record.attached", |event| event.data["id"] == "shop-1").await;
    assert_eq!(said.data["through"], "shown-on-the-card", "{:?}", said.data);
}

/// A library and a gallery of mockups, each kept as files of the project.
const AS_FILES: &str = r#"
blueprint: kept-as-files
permissions: [projects, records]
steps:
  notes:
    do: collection.ensure
    with: { name: notes, store: documents, at: notes, fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } } }
  shots:
    do: collection.ensure
    with: { name: shots, store: mockups, at: shots, fields: { title: { type: text, required: true }, path: { type: text } } }
"#;

/// Writes `content` at `path` of `project`, as its owner.
async fn written(rig: &Rig, project: &str, path: &str, content: &str) {
    let body = json!({ "path": path, "content": content });
    let (status, answer) =
        rig.call(Method::PUT, &format!("/api/projects/{project}/file"), Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

/// Attaches a few bytes to `record`, and gives back where its file is kept.
async fn kept_at(rig: &Rig, record: &str) -> std::path::PathBuf {
    let (status, kept) =
        attach(rig, &rig.token, &record.replace(':', "%3A"), "shot.png", "image/png", b"png").await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    let file = rig.config.data_dir.join("attachments").join(kept["id"].as_str().unwrap());
    assert!(file.exists(), "{}", file.display());
    file
}

#[tokio::test]
async fn a_project_deleted_for_good_from_the_trash_or_at_once_takes_the_files_attached_to_its_records_with_it()
 {
    let rig = Rig::new().await;
    for name in ["Shop", "Till"] {
        rig.send("/api/projects", json!({ "name": name })).await;
        rig.board(&name.to_lowercase()).await;
        rig.send(&format!("/api/board/{}/items", name.to_lowercase()), json!({ "title": "Fix it" })).await;
    }
    let (shop, till) = (kept_at(&rig, "shop-1").await, kept_at(&rig, "till-1").await);

    // Into the Trash, its records' files are kept for it to be given back with; deleted for good, they go.
    let (status, gone) = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert!(shop.exists(), "kept while the project is in the Trash");
    let trash = gone["trash"].as_str().unwrap().to_string();
    let (status, forgot) =
        rig.call(Method::DELETE, &format!("/api/trash/{trash}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{forgot}");
    assert!(!shop.exists(), "gone with the project for good");
    assert!(till.exists(), "and another project's left as it was");

    // Where the factory keeps nothing that is deleted, at once.
    rig.send("/api/system/trash", json!({ "days": 0 })).await;
    let (status, gone) = rig.call(Method::DELETE, "/api/projects/till", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert!(!till.exists(), "gone with the project at once");
}

#[tokio::test]
async fn a_document_or_a_mockup_deleted_through_its_file_takes_what_is_attached_to_it_and_a_file_inside_a_mockup_does_not()
 {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied =
        rig.send("/api/apply", json!({ "source": AS_FILES, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    written(&rig, "shop", "notes/till.md", "# The till\n").await;
    written(&rig, "shop", "shots/pricing/index.html", "<p>Prices</p>").await;
    written(&rig, "shop", "shots/pricing/page.css", "p {}").await;
    let (note, shot) = (kept_at(&rig, "shop-notes:till").await, kept_at(&rig, "shop-shots:pricing").await);
    rig.send("/api/system/trash", json!({ "days": 0 })).await;
    let delete = async |path: &str| {
        let at = format!("/api/projects/shop/file?path={path}");
        let (status, answer) = rig.call(Method::DELETE, &at, Some(&rig.token), None).await;
        assert_eq!(status, StatusCode::OK, "{path}: {answer}");
    };

    delete("notes/till.md").await;
    assert!(!note.exists(), "a document deleted at once takes what is attached to it");
    delete("shots/pricing/page.css").await;
    assert!(shot.exists(), "a file inside a mockup is no mockup");
    delete("shots/pricing").await;
    assert!(!shot.exists(), "the mockup's folder deleted at once takes what is attached to it");
}

/// A library whose folder holds a gallery of mockups.
const NESTED: &str = r#"
blueprint: kept-as-files
permissions: [projects, records]
steps:
  notes:
    do: collection.ensure
    with: { name: notes, store: documents, at: notes, fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } } }
  shots:
    do: collection.ensure
    with: { name: shots, store: mockups, at: notes/shots, fields: { title: { type: text, required: true }, path: { type: text } } }
"#;

#[tokio::test]
async fn a_folder_deleted_for_good_or_at_once_takes_what_is_attached_to_every_document_and_mockup_under_it() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = rig.send("/api/apply", json!({ "source": NESTED, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let filled = async || {
        written(&rig, "shop", "notes/till.md", "# The till\n").await;
        written(&rig, "shop", "notes/old/cart.md", "# The cart\n").await;
        written(&rig, "shop", "notes/shots/pricing/index.html", "<p>Prices</p>").await;
        let mut kept = Vec::new();
        for record in ["shop-notes:till", "shop-notes:old:cart", "shop-shots:pricing"] {
            kept.push(kept_at(&rig, record).await);
        }
        kept
    };
    let delete = async || {
        let (status, answer) =
            rig.call(Method::DELETE, "/api/projects/shop/file?path=notes", Some(&rig.token), None).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
        answer
    };

    // Into the Trash, kept with the folder; deleted for good from there, each goes.
    let kept = filled().await;
    let trash = delete().await["trash"].as_str().unwrap().to_string();
    assert!(kept.iter().all(|file| file.exists()), "kept while the folder is in the Trash");
    let (status, forgot) =
        rig.call(Method::DELETE, &format!("/api/trash/{trash}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{forgot}");
    assert!(kept.iter().all(|file| !file.exists()), "every one under the folder gone with it for good");

    // At once, the same.
    let kept = filled().await;
    rig.send("/api/system/trash", json!({ "days": 0 })).await;
    delete().await;
    assert!(kept.iter().all(|file| !file.exists()), "every one under the folder gone with it at once");
}

#[tokio::test]
async fn an_attachment_taken_away_goes_into_the_trash_and_is_given_back_whole_or_deleted_for_good_by_whoever_may_change_its_record()
 {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig.send("/api/board/shop/items", json!({ "title": "Fix the till" })).await;
    let (_, kept) = attach(&rig, &rig.token, "shop-1", "till.png", "image/png", b"\x89PNG the till").await;
    let id = kept["id"].as_str().unwrap().to_string();
    let file = rig.config.data_dir.join("attachments").join(&id);
    let history = async |topic: &str| -> Vec<String> {
        let mut written = rig.follow("collection.history");
        crate::soon(&format!("{topic} in the card's history"), async {
            loop {
                let lines = rig.read("/api/records/shop-1/history").await;
                let said: Vec<String> = lines["history"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|line| line["topic"] == topic)
                    .map(|line| line["said"].as_str().unwrap().to_string())
                    .collect();
                if !said.is_empty() {
                    return said;
                }
                written.next().await.unwrap();
            }
        })
        .await
    };

    // Taken away, it is in the Trash for the days it keeps things, its file in the entry's folder.
    let (status, gone) = rig
        .call(Method::DELETE, &format!("/api/records/shop-1/attachments/{id}"), Some(&rig.token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let trash = gone["trash"].as_str().unwrap().to_string();
    assert_eq!(
        (gone["kept"].as_u64(), gone["attachment"]["name"].as_str()),
        (Some(30), Some("till.png")),
        "{gone}"
    );
    assert_eq!(fetched(&rig, &rig.token, &id).await.0, StatusCode::NOT_FOUND);
    assert!(!file.exists() && rig.config.data_dir.join("trash").join(&trash).join("kept").exists());
    assert_eq!(history("record.detached").await, ["Ada took away till.png, moving it to the Trash"]);
    let listed = rig.read("/api/trash").await;
    let entry =
        listed["trash"].as_array().unwrap().iter().find(|one| one["id"] == trash.as_str()).unwrap().clone();
    assert_eq!(
        (entry["kind"].as_str(), entry["thing"].as_str(), entry["called"].as_str()),
        (Some("attachment"), Some("shop-1"), Some("till.png")),
        "{entry}"
    );

    // Whoever may read the card sees it there, and may neither give it back nor delete it for good.
    let reader = key(&rig, &["board:read", "records:read"]).await;
    let (_, theirs) = rig.call(Method::GET, "/api/trash", Some(&reader), None).await;
    let seen = theirs["trash"].as_array().unwrap().iter().find(|one| one["id"] == trash.as_str()).cloned();
    assert_eq!(seen.map(|one| one["may"].clone()), Some(json!(false)), "{theirs}");
    let (status, _) =
        rig.call(Method::POST, &format!("/api/trash/{trash}/restore"), Some(&reader), Some(json!({}))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = rig.call(Method::DELETE, &format!("/api/trash/{trash}"), Some(&reader), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Given back, it is attached again, its file whole, and the history says so.
    rig.send(&format!("/api/trash/{trash}/restore"), json!({})).await;
    let (status, _, _, bytes) = fetched(&rig, &rig.token, &id).await;
    assert_eq!((status, bytes.as_slice()), (StatusCode::OK, &b"\x89PNG the till"[..]));
    assert!(file.exists() && !rig.config.data_dir.join("trash").join(&trash).exists());
    assert_eq!(history("record.attached").await.last().unwrap(), "Ada gave till.png back from the Trash");

    // Taken away again and deleted for good, it is gone, file and all.
    let (_, gone) = rig
        .call(Method::DELETE, &format!("/api/records/shop-1/attachments/{id}"), Some(&rig.token), None)
        .await;
    let trash = gone["trash"].as_str().unwrap().to_string();
    let (status, _) = rig.call(Method::DELETE, &format!("/api/trash/{trash}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!file.exists() && !rig.config.data_dir.join("trash").join(&trash).exists());
    assert!(rig.read("/api/records/shop-1/attachments").await["attachments"].as_array().unwrap().is_empty());
}
