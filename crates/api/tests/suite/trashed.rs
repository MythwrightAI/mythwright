//! The Trash over HTTP (`api::trash`, over `studio::trash` and the
//! `trash` crate): each kind deleted, left out of every route that lists
//! or reads what it was of, found there only, given back where it was,
//! deleted for good, emptied on the factory's clock, and kept by a copy.

use std::sync::Arc;
use std::time::Duration;

use kernel::ManualClock;
use kernel::trash::DAY_MS;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

const NOTES: &str = r#"
blueprint: notes
steps:
  notes:
    do: collection.ensure
    with:
      name: notes
      fields: { title: { type: text, required: true } }
      history: { record.commented: "{actor} said: {body}", "record.*": "" }
  secrets:
    do: collection.ensure
    with: { name: secrets, reads: private:secrets, fields: { title: { type: text, required: true } } }
"#;

async fn shop(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["docs"] })).await;
    rig.board("shop").await;
    let applied = rig.send("/api/apply", json!({ "source": NOTES, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

async fn write(rig: &Rig, path: &str, content: &str) {
    let body = Some(json!({ "path": path, "content": content }));
    let (status, body) = rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), body).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

async fn delete(rig: &Rig, path: &str) {
    let (status, body) = rig.call(Method::DELETE, path, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "DELETE {path}: {body}");
}

/// What is in the Trash for whoever holds `token`.
async fn trash(rig: &Rig, token: &str) -> Vec<Value> {
    let (status, body) = rig.call(Method::GET, "/api/trash", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["trash"].as_array().cloned().unwrap_or_default()
}

fn of<'a>(trash: &'a [Value], thing: &str) -> &'a Value {
    trash.iter().find(|entry| entry["thing"] == thing).unwrap_or_else(|| panic!("no `{thing}` in {trash:?}"))
}

fn ids(listed: &Value, at: &str) -> Vec<String> {
    let rows = listed.pointer(at).and_then(Value::as_array).cloned().unwrap_or_default();
    rows.iter().map(|row| row["id"].as_str().unwrap_or_default().to_string()).collect()
}

#[tokio::test]
async fn each_kind_deleted_is_in_the_trash_and_in_nothing_that_lists_what_it_was_of_until_it_is_given_back() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let card = rig.send("/api/board/shop/items", json!({ "title": "Add a cart", "status": "ready" })).await;
    let card = card["id"].as_str().unwrap().to_string();
    let note =
        rig.send("/api/collections/shop/notes/records", json!({ "fields": { "title": "Kept" } })).await;
    let note = note["id"].as_str().unwrap().to_string();
    rig.send(&format!("/api/records/{note}/comments"), json!({ "body": "Said on it." })).await;
    write(&rig, "docs/plan.md", "# Plan\n").await;
    let doc = "shop-docs:plan";
    rig.send(&format!("/api/records/{doc}/comments"), json!({ "body": "Said on the plan." })).await;
    let folder = rig.dir.join("data").join("projects").join("shop");

    // What deleting answers: what it is in the Trash, and how long it is kept there.
    let path = format!("/api/board/items/{card}");
    let (_, answer) = rig.call(Method::DELETE, &path, Some(&rig.token), None).await;
    assert!(answer["trash"].as_str().is_some_and(|id| id.starts_with("trash_")), "{answer}");
    assert_eq!(answer["kept"], 30, "{answer}");
    delete(&rig, &format!("/api/records/{note}")).await;
    delete(&rig, "/api/projects/shop/file?path=docs/plan.md").await;
    let listed = trash(&rig, &rig.token).await;
    assert_eq!(listed.len(), 3, "{listed:?}");
    assert_eq!(of(&listed, &card)["kind"], "record");
    assert_eq!(of(&listed, &card)["called"], "Add a cart");
    assert_eq!(of(&listed, &note)["table"], "notes");
    assert_eq!(of(&listed, "docs/plan.md")["kind"], "file");
    let entry = of(&listed, &card);
    assert_eq!(entry["goes"].as_i64(), Some(entry["at"].as_i64().unwrap() + 30 * DAY_MS));
    assert_eq!(entry["may"], true);

    // Nothing that lists or reads what each was of finds it.
    assert!(!ids(&rig.read("/api/board/shop").await, "/items").contains(&card), "on the board");
    assert!(!ids(&rig.read("/api/collections/shop/notes/records").await, "/records").contains(&note));
    let library = rig.read("/api/collections/shop/docs/records").await;
    assert!(!ids(&library, "/records").contains(&doc.to_string()), "in the library: {library}");
    for gone in [card.as_str(), note.as_str(), doc] {
        let (status, _) = rig.get(&format!("/api/records/{gone}")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{gone} is found by its id only by the Trash");
    }
    let tree = rig.read("/api/projects/shop/tree").await;
    assert!(!tree.to_string().contains("plan.md"), "{tree}");
    assert!(!folder.join("docs/plan.md").exists(), "out of the folder agents work in");
    let (status, _) = rig.get("/api/projects/shop/file?path=docs/plan.md").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Given back, each is where it was, with what was said on it.
    for thing in [card.as_str(), note.as_str(), "docs/plan.md"] {
        let id = of(&listed, thing)["id"].as_str().unwrap().to_string();
        rig.send(&format!("/api/trash/{id}/restore"), json!({})).await;
    }
    assert!(trash(&rig, &rig.token).await.is_empty());
    let back = rig.read(&format!("/api/records/{card}")).await;
    assert_eq!(back["data"]["status"], "ready", "{back}");
    assert!(ids(&rig.read("/api/collections/shop/notes/records").await, "/records").contains(&note));
    assert_eq!(std::fs::read_to_string(folder.join("docs/plan.md")).unwrap(), "# Plan\n");
    for (record, said) in [(note.as_str(), "Said on it."), (doc, "Said on the plan.")] {
        let history = rig.read(&format!("/api/records/{record}/history")).await;
        assert!(history.to_string().contains(said), "{record}: {history}");
    }
    // Each one's history says it was given back, and not added: in the
    // board's own words for a card, in the usual ones for a row.
    for (record, line) in [
        (card.clone(), "Ada gave it back from the Trash, to ready"),
        (note.clone(), "Ada gave it back from the Trash"),
    ] {
        let (of, wanted) = (record.clone(), line.to_string());
        rig.event("collection.history", move |event| {
            event.data["record"] == of.as_str() && event.data["said"] == wanted.as_str()
        })
        .await;
        let history = rig.read(&format!("/api/records/{record}/history")).await;
        let said: Vec<&str> =
            history["history"].as_array().unwrap().iter().filter_map(|line| line["said"].as_str()).collect();
        assert_eq!(said.last(), Some(&line), "{record}: {said:?}");
        assert!(
            said.iter().filter(|one| one.contains("added it")).count() <= 1,
            "added once, and given back after: {said:?}"
        );
    }
}

#[tokio::test]
async fn a_document_given_back_beside_what_took_its_place_takes_what_was_said_on_it_there() {
    let rig = Rig::new().await;
    shop(&rig).await;
    write(&rig, "docs/plan.md", "# Plan\n").await;
    rig.send("/api/records/shop-docs:plan/comments", json!({ "body": "Said on the plan." })).await;
    delete(&rig, "/api/projects/shop/file?path=docs/plan.md").await;
    write(&rig, "docs/plan.md", "# Another plan\n").await;
    let id = trash(&rig, &rig.token).await[0]["id"].as_str().unwrap().to_string();
    let (status, refused) = rig.post(&format!("/api/trash/{id}/restore"), json!({})).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    let back = rig.send(&format!("/api/trash/{id}/restore"), json!({ "beside": true })).await;
    assert_eq!(back["at"], "docs/plan (restored).md", "{back}");
    let library = rig.read("/api/collections/shop/docs/records").await;
    let now = ids(&library, "/records")
        .into_iter()
        .find(|one| one.contains("restored"))
        .unwrap_or_else(|| panic!("{library}"));
    let history = rig.read(&format!("/api/records/{now}/history")).await;
    assert!(history.to_string().contains("Said on the plan."), "what was said moved with it: {history}");
    let left = rig.read("/api/records/shop-docs:plan/history").await;
    assert!(
        !left.to_string().contains("Said on the plan."),
        "and is not said of what took its place: {left}"
    );
}

#[tokio::test]
async fn whoever_could_not_read_a_thing_does_not_see_it_and_whoever_could_not_delete_it_may_not_give_it_back()
{
    let rig = Rig::new().await;
    shop(&rig).await;
    let note =
        rig.send("/api/collections/shop/notes/records", json!({ "fields": { "title": "Open" } })).await;
    let note = note["id"].as_str().unwrap().to_string();
    let secret =
        rig.send("/api/collections/shop/secrets/records", json!({ "fields": { "title": "Shh" } })).await;
    let secret = secret["id"].as_str().unwrap().to_string();
    delete(&rig, &format!("/api/records/{note}")).await;
    delete(&rig, &format!("/api/records/{secret}")).await;
    let all = trash(&rig, &rig.token).await;
    let (open, kept) = (of(&all, &note)["id"].clone(), of(&all, &secret)["id"].clone());

    let reader = crate::held::key(&rig, &["records:read", "projects:read"]).await;
    let seen = trash(&rig, &reader).await;
    assert_eq!(seen.len(), 1, "a private table's row only to who may read it: {seen:?}");
    assert_eq!((seen[0]["thing"].as_str(), seen[0]["may"].as_bool()), (Some(note.as_str()), Some(false)));
    for (method, path) in [
        (Method::POST, format!("/api/trash/{}/restore", open.as_str().unwrap())),
        (Method::DELETE, format!("/api/trash/{}", open.as_str().unwrap())),
    ] {
        let (status, body) = rig.call(method, &path, Some(&reader), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {body}");
    }
    // What is kept from it answers as nothing that is there.
    let path = format!("/api/trash/{}/restore", kept.as_str().unwrap());
    let (status, _) = rig.call(Method::POST, &path, Some(&reader), Some(json!({}))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let nobody = crate::held::key(&rig, &["projects:read"]).await;
    assert!(trash(&rig, &nobody).await.is_empty());

    // The setting is the owner's.
    let member = crate::held::key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let (status, _) = rig.call(Method::GET, "/api/system/trash", Some(&member), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) =
        rig.call(Method::POST, "/api/system/trash", Some(&member), Some(json!({ "days": 1 }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Whoever could delete it may delete it for good.
    delete(&rig, &format!("/api/trash/{}", open.as_str().unwrap())).await;
    assert_eq!(trash(&rig, &rig.token).await.len(), 1);
}

/// A node on a clock the test moves.
async fn ticking() -> (Rig, ManualClock) {
    let clock = ManualClock::new();
    let moved = clock.clone();
    let rig =
        Rig::owned(Rig::with("api-trash", move |config, _| config.clock = Some(Arc::new(moved))).await).await;
    (rig, clock)
}

#[tokio::test]
async fn what_has_been_in_the_trash_as_long_as_the_owner_says_goes_on_the_factorys_clock_and_nought_deletes_at_once()
 {
    let (rig, clock) = ticking().await;
    shop(&rig).await;
    let said = rig.read("/api/system/trash").await;
    assert_eq!(said["days"], 30, "thirty unless said: {said}");
    let card = rig.send("/api/board/shop/items", json!({ "title": "Old" })).await;
    let card = card["id"].as_str().unwrap().to_string();
    delete(&rig, &format!("/api/board/items/{card}")).await;
    let kept = rig.send("/api/system/trash", json!({ "days": 7 })).await;
    assert_eq!((kept["days"].as_u64(), kept["gone"].as_u64()), (Some(7), Some(0)), "{kept}");
    clock.advance(Duration::from_millis((8 * DAY_MS) as u64));
    let gone = rig.event("trash.gone", |event| event.data["thing"] == card.as_str()).await;
    assert_eq!(gone.data["why"], "days");
    assert!(trash(&rig, &rig.token).await.is_empty());

    let refused = rig.post("/api/system/trash", json!({ "days": "soon" })).await;
    assert_eq!(refused.0, StatusCode::BAD_REQUEST, "{}", refused.1);
    rig.send("/api/system/trash", json!({ "days": "always" })).await;
    assert_eq!(rig.read("/api/system/trash").await["days"], Value::Null);
    rig.send("/api/system/trash", json!({ "days": 0 })).await;
    let now = rig.send("/api/board/shop/items", json!({ "title": "Now" })).await;
    delete(&rig, &format!("/api/board/items/{}", now["id"].as_str().unwrap())).await;
    assert!(trash(&rig, &rig.token).await.is_empty(), "nought days deletes at once");
}

#[tokio::test]
async fn a_project_forgotten_is_in_the_trash_with_its_files_where_they_were_and_comes_back_in_its_workspace()
{
    let rig = Rig::new().await;
    shop(&rig).await;
    let other = rig.send("/api/workspaces", json!({ "name": "Other" })).await;
    let other = other["id"].as_str().unwrap().to_string();
    let body = Some(json!({ "workspace": other }));
    let (status, placed) =
        rig.call(Method::PUT, "/api/projects/shop/workspace", Some(&rig.token), body).await;
    assert_eq!(status, StatusCode::OK, "{placed}");
    let forgot = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await.1;
    let id = forgot["trash"].as_str().unwrap().to_string();
    let (status, _) = rig.get("/api/projects/shop").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(rig.dir.join("data").join("projects").join("shop").is_dir());
    let listed = trash(&rig, &rig.token).await;
    assert_eq!(of(&listed, "shop")["kind"], "project");
    rig.send(&format!("/api/trash/{id}/restore"), json!({})).await;
    let back = rig.read("/api/projects/shop").await;
    assert_eq!(back["name"], "Shop");
    let workspace = rig.read(&format!("/api/workspaces/{other}")).await;
    assert!(workspace["projects"].to_string().contains("shop"), "back in its workspace: {workspace}");
}

#[tokio::test]
async fn a_copy_of_the_factory_takes_the_trash_with_its_files_and_how_long_it_is_kept() {
    let shared = crate::snapshots::Shared::new();
    let prod = shared.factory("prod").await;
    shop(&prod).await;
    prod.send("/api/system/trash", json!({ "days": 12 })).await;
    let card = prod.send("/api/board/shop/items", json!({ "title": "In the Trash" })).await;
    let card = card["id"].as_str().unwrap().to_string();
    delete(&prod, &format!("/api/board/items/{card}")).await;
    write(&prod, "notes.md", "kept\n").await;
    delete(&prod, "/api/projects/shop/file?path=notes.md").await;
    let to = shared.snapshot("prod-copy");
    crate::snapshots::taken(&prod, &to).await;
    let dev = shared.copy("dev", &to).await;
    assert_eq!(dev.read("/api/system/trash").await["days"], 12);
    let listed = trash(&dev, &dev.token).await;
    assert_eq!(listed.len(), 2, "{listed:?}");
    for thing in [card.as_str(), "notes.md"] {
        let id = of(&listed, thing)["id"].as_str().unwrap().to_string();
        dev.send(&format!("/api/trash/{id}/restore"), json!({})).await;
    }
    assert_eq!(dev.read(&format!("/api/records/{card}")).await["data"]["title"], "In the Trash");
    let notes = dev.read("/api/projects/shop/file?path=notes.md").await;
    assert_eq!(notes["content"], "kept\n");
}

#[tokio::test]
async fn a_private_channel_in_the_trash_is_seen_told_of_and_given_back_by_its_readers_alone() {
    use crate::kept::{Told, key, person, reads_none, say};
    let rig = Rig::with("api-trashed-kept", |_, _| {}).await;
    let owned = Rig::owned(rig).await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell) = (person(rig, "Mira").await, person(rig, "Nell").await);
    let mut manages: Vec<&str> = kernel::principal::MEMBER_SCOPES.to_vec();
    manages.push("admin");
    let admin = key(rig, &rig.token, &manages).await;
    let (mut to_nell, mut to_admin, mut to_mira) =
        (Told::of(rig, &nell.token).await, Told::of(rig, &admin).await, Told::of(rig, &mira.token).await);
    let made = json!({ "name": "Quince review", "kept": true });
    let (status, made) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(made)).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    let channel = made["id"].as_str().unwrap().to_string();
    say(rig, &mira.token, &channel, json!({ "body": "the greengage word" })).await;
    let open =
        rig.send("/api/channels", json!({ "name": "general" })).await["id"].as_str().unwrap().to_string();

    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/channels/{channel}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let entry = gone["trash"].as_str().unwrap().to_string();
    to_mira.until("trash.put").await;
    let marked = ["Quince", "greengage", channel.as_str(), entry.as_str()];

    // Whoever was not in it, and a key that manages the factory and holds
    // nothing private, are shown nothing of it and told nothing of it.
    for (who, token) in [("Nell", &nell.token), ("an administrator", &admin)] {
        assert!(trash(rig, token).await.is_empty(), "{who} sees it");
        for (method, path) in [
            (Method::POST, format!("/api/trash/{entry}/restore")),
            (Method::DELETE, format!("/api/trash/{entry}")),
        ] {
            let (status, body) = rig.call(method, &path, Some(token), Some(json!({}))).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{who} {path}: {body}");
            // As for something that is not there: the id it was asked by, and nothing of the channel.
            reads_none(&format!("by what {who} is answered"), &body.to_string(), &marked[..3]);
        }
    }
    // Who was in it sees it by what it is called, and may not give it back.
    let seen = trash(rig, &mira.token).await;
    assert_eq!(
        (seen.len(), seen[0]["called"].as_str(), seen[0]["may"].as_bool()),
        (1, Some("Quince review"), Some(false))
    );
    let (status, _) = rig
        .call(Method::POST, &format!("/api/trash/{entry}/restore"), Some(&mira.token), Some(json!({})))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Given back by the owner, it is kept to the same members.
    rig.send(&format!("/api/trash/{entry}/restore"), json!({})).await;
    to_mira.until("trash.restored").await;
    let said =
        rig.call(Method::GET, &format!("/api/channels/{channel}/messages"), Some(&mira.token), None).await;
    assert_eq!(
        (said.0, said.1["messages"][0]["body"].as_str()),
        (StatusCode::OK, Some("the greengage word"))
    );
    let (status, _) =
        rig.call(Method::GET, &format!("/api/channels/{channel}"), Some(&nell.token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    say(rig, &rig.token, &open, json!({ "body": "the open marker" })).await;
    for (who, told) in [("Nell", &mut to_nell), ("an administrator", &mut to_admin)] {
        told.until("the open marker").await;
        reads_none(&format!("on what {who} is told as it is said"), &told.heard, &marked);
    }
}
