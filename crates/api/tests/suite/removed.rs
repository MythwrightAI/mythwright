//! A channel taken away: by its owner, into the Trash with everything
//! said in it, given back from there whole, and once it is deleted for
//! good with nothing left holding it.

use std::path::Path;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::snapshots::{Shared, counted, holds, stop_instance, taken};

/// A channel with two things said in it, the first with a file: its id,
/// and the file's.
async fn spoken_in(rig: &Rig, name: &str) -> (String, String) {
    let channel = rig.send("/api/channels", json!({ "name": name })).await;
    let channel = channel["id"].as_str().unwrap().to_string();
    let attach = format!("{}/api/channels/{channel}/attachments?name=plan.txt", rig.url);
    let sent = rig.client.post(attach).bearer_auth(&rig.token).body("a plan of quince and medlar");
    let attached: Value = sent.send().await.unwrap().json().await.unwrap();
    let file = attached["id"].as_str().unwrap().to_string();
    let said = json!({ "body": "the marmalade ledger is attached", "attachments": [file] });
    rig.send(&format!("/api/channels/{channel}/messages"), said).await;
    rig.send(&format!("/api/channels/{channel}/messages"), json!({ "body": "second thoughts on marmalade" }))
        .await;
    (channel, file)
}

async fn remove(rig: &Rig, token: &str, channel: &str) -> (StatusCode, Value) {
    rig.call(Method::DELETE, &format!("/api/channels/{channel}"), Some(token), None).await
}

/// Takes a channel away into the Trash, and deletes it for good from there.
async fn gone_for_good(rig: &Rig, channel: &str) {
    let (status, gone) = remove(rig, &rig.token, channel).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let trash = gone["trash"].as_str().unwrap();
    let (status, forgot) =
        rig.call(Method::DELETE, &format!("/api/trash/{trash}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{forgot}");
}

/// How many messages a search for `words` finds, once the index has
/// taken in everything that has happened.
async fn found(rig: &Rig, words: &str) -> usize {
    rig.node().app.studio.search.catch_up().await.unwrap();
    let hits = rig.read(&format!("/api/search?q={words}&kind=message")).await;
    hits["hits"].as_array().unwrap().len()
}

/// Whether any file under `dir` holds `text`.
fn held_in(dir: &Path, text: &str) -> bool {
    std::fs::read_dir(dir).unwrap().flatten().any(|entry| match entry.path() {
        path if path.is_dir() => held_in(&path, text),
        path => holds(&std::fs::read(path).unwrap_or_default(), text),
    })
}

#[tokio::test]
async fn a_channel_is_removed_by_the_owner_into_the_trash_with_what_was_said_in_it_and_attached_to_it() {
    let rig = Rig::new().await;
    let (channel, file) = spoken_in(&rig, "general").await;
    let kept = rig.node().app.studio.channels.attachments.path(&file);
    assert!(kept.exists() && found(&rig, "marmalade").await == 2);

    // A member may read and write a channel, and may not take it away.
    let member = json!({ "name": "member", "scopes": kernel::principal::MEMBER_SCOPES });
    let member = rig.send("/api/credentials", member).await["token"].as_str().unwrap().to_string();
    let (status, refused) = remove(&rig, &member, "general").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap().contains("(needs `admin`)"), "{refused}");
    assert_eq!(rig.read("/api/channels/general/messages").await["messages"].as_array().unwrap().len(), 2);

    let (status, gone) = remove(&rig, &rig.token, "general").await;
    assert_eq!(
        (status, gone["name"].as_str(), gone["kept"].as_u64()),
        (StatusCode::OK, Some("general"), Some(30)),
        "{gone}"
    );
    let trash = gone["trash"].as_str().unwrap().to_string();
    let said = rig.event("channel.removed", |_| true).await;
    assert_eq!(said.data["channel"], channel.as_str());
    assert_eq!((said.data["messages"].as_i64(), said.data["attachments"].as_i64()), (Some(2), Some(1)));
    assert_eq!(said.data["trash"], trash.as_str());

    // It is not there, by its name or by its id, and neither is its file.
    for path in ["/api/channels/general", "/api/channels/general/messages"] {
        assert_eq!(rig.get(path).await.0, StatusCode::NOT_FOUND, "{path}");
    }
    assert_eq!(rig.get(&format!("/api/channels/{channel}")).await.0, StatusCode::NOT_FOUND);
    assert_eq!(rig.read("/api/channels").await["channels"], json!([]));
    assert_eq!(rig.get(&format!("/api/attachments/{file}")).await.0, StatusCode::NOT_FOUND);
    assert!(!kept.exists(), "the file went with it");
    // Nothing the factory keeps goes on holding what was said there: not
    // the journal, where each message was an event, nor the search index.
    assert_eq!(counted(&rig, "SELECT COUNT(*) FROM events WHERE topic = 'channel.message'").await, 0);
    assert_eq!(counted(&rig, "SELECT COUNT(*) FROM messages").await, 0);
    assert_eq!(counted(&rig, "SELECT COUNT(*) FROM channel_members").await, 0);
    assert_eq!(counted(&rig, "SELECT COUNT(*) FROM attachments").await, 0);
    assert_eq!(found(&rig, "marmalade").await, 0);

    // It is in the Trash, and comes back from there whole: what was said,
    // what was attached, found again by a search.
    let listed = rig.read("/api/trash").await;
    let entry =
        listed["trash"].as_array().unwrap().iter().find(|entry| entry["id"] == trash.as_str()).unwrap();
    assert_eq!(
        (entry["kind"].as_str(), entry["called"].as_str()),
        (Some("channel"), Some("#general")),
        "{entry}"
    );
    let back = rig.send(&format!("/api/trash/{trash}/restore"), json!({})).await;
    assert_eq!(back["at"], channel.as_str());
    let said = rig.read("/api/channels/general/messages").await;
    assert_eq!(said["messages"].as_array().unwrap().len(), 2, "{said}");
    assert!(kept.exists() && found(&rig, "marmalade").await == 2);
    assert_eq!(rig.get(&format!("/api/attachments/{file}")).await.0, StatusCode::OK);

    // Deleted for good, nothing holds any of it.
    gone_for_good(&rig, "general").await;
    assert!(rig.read("/api/trash").await["trash"].as_array().unwrap().is_empty());
    assert!(!kept.exists(), "the file went with it");
    assert_eq!(counted(&rig, "SELECT COUNT(*) FROM events WHERE topic = 'channel.message'").await, 0);
    assert_eq!(counted(&rig, "SELECT COUNT(*) FROM messages").await, 0);
    assert_eq!(counted(&rig, "SELECT COUNT(*) FROM trash").await, 0);
    assert_eq!(found(&rig, "marmalade").await, 0);

    // A channel made under the name again is another, with nothing in it.
    let again = rig.send("/api/channels", json!({ "name": "general" })).await;
    assert_ne!(again["id"], channel.as_str());
    assert_eq!(rig.read("/api/channels/general/messages").await["messages"], json!([]));
    let (status, missing) = remove(&rig, &rig.token, "nonesuch").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");
}

#[tokio::test]
async fn a_channel_something_running_posts_in_is_not_removed_until_that_is_stopped_or_it_is_said_all_the_same()
 {
    let rig = Rig::new().await;
    spoken_in(&rig, "general").await;
    spoken_in(&rig, "elsewhere").await;
    // A bot is a blueprint that says which channel it speaks in.
    let params = json!({ "channel": "general", "name": "Parrot", "run": "exit 0" });
    let bot = json!({ "blueprint": "std/bot", "key": "bot-parrot", "params": params, "wait": true });
    let bot = rig.send("/api/apply", bot).await["instance"].as_str().unwrap().to_string();

    let (status, refused) = remove(&rig, &rig.token, "general").await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    let why = refused["error"]["message"].as_str().unwrap();
    assert!(
        why.starts_with(&format!(
            "#general is where `bot-parrot` (std/bot, {bot}) posts, and it is running."
        )),
        "{why}"
    );
    assert!(why.contains(&format!("stop {bot}")) && why.contains("--force"), "{why}");
    assert_eq!(rig.read("/api/channels/general/messages").await["messages"].as_array().unwrap().len(), 2);
    // A channel nothing posts in is nobody's to hold up.
    assert_eq!(remove(&rig, &rig.token, "elsewhere").await.0, StatusCode::OK);

    // Stopped, it holds the channel no longer.
    stop_instance(&rig, &bot).await;
    assert_eq!(remove(&rig, &rig.token, "general").await.0, StatusCode::OK);

    // Or it is removed all the same, and what posts there is left running.
    spoken_in(&rig, "general").await;
    let again = json!({ "blueprint": "std/bot", "key": "bot-parrot", "params": params, "wait": true });
    let again = rig.send("/api/apply", again).await["instance"].as_str().unwrap().to_string();
    let (status, forced) =
        rig.call(Method::DELETE, "/api/channels/general?force=true", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{forced}");
    assert_eq!(rig.read(&format!("/api/instances/{again}")).await["instance"]["status"], "active");
    stop_instance(&rig, &again).await;
}

/// How many turns answer something said in a channel, in every file
/// that keeps turns: an agent of a project has its turns in the
/// project's own.
async fn turns_asked_in_a_channel(rig: &Rig) -> i64 {
    let mut kept = 0;
    for journal in rig.node().app.journal.every_keeping(journal::keeps::TURNS) {
        let counted = journal.db().call(|database| {
            database.query_row("SELECT COUNT(*) FROM turns WHERE kind = 'message'", [], |row| {
                row.get::<_, i64>(0)
            })
        });
        kept += counted.await.unwrap();
    }
    kept
}

#[tokio::test]
async fn a_copy_holds_a_channel_in_the_trash_as_the_trash_keeps_it_and_nothing_of_one_deleted_for_good() {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    let (channel, _file) = spoken_in(&prod, "general").await;
    // An agent in the channel was asked, and what it was asked was kept
    // with what had been said there.
    prod.send(&format!("/api/channels/{channel}/agents"), json!({ "name": "helper" })).await;
    prod.send(
        &format!("/api/channels/{channel}/messages"),
        json!({ "body": "@helper is the marmalade set?" }),
    )
    .await;
    prod.event("agent.turn.finished", |_| true).await;
    assert_eq!(turns_asked_in_a_channel(&prod).await, 1);

    // A copy made while the channel is there holds it: so the looking
    // below would find it.
    let before = shared.snapshot("before");
    taken(&prod, &before).await;
    assert!(held_in(&before, "marmalade") && held_in(&before, "quince and medlar"));

    let (status, gone) = remove(&prod, &prod.token, "general").await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let trash = gone["trash"].as_str().unwrap().to_string();
    // What agents were asked there goes as the channel does: it is no
    // part of what is given back.
    assert_eq!(turns_asked_in_a_channel(&prod).await, 0);

    // A copy made while it is in the Trash has it as the Trash keeps it,
    // and the factory started from the copy gives it back whole.
    let trashed = shared.snapshot("trashed");
    taken(&prod, &trashed).await;
    assert!(held_in(&trashed, "marmalade") && held_in(&trashed, "quince and medlar"));
    let dev = shared.copy("dev", &trashed).await;
    dev.send(&format!("/api/trash/{trash}/restore"), json!({})).await;
    let said = dev.read("/api/channels/general/messages").await["messages"].clone();
    let bodies: Vec<&str> =
        said.as_array().unwrap().iter().filter_map(|message| message["body"].as_str()).collect();
    assert!(bodies.contains(&"the marmalade ledger is attached"), "{said}");
    let attached =
        said.as_array().unwrap().iter().find_map(|message| message["attachments"][0]["id"].as_str());
    let file = attached.unwrap().to_string();
    let (status, _) = dev.get(&format!("/api/attachments/{file}")).await;
    assert_eq!(status, StatusCode::OK);

    let (status, forgot) =
        prod.call(Method::DELETE, &format!("/api/trash/{trash}"), Some(&prod.token), None).await;
    assert_eq!(status, StatusCode::OK, "{forgot}");
    let after = shared.snapshot("after");
    taken(&prod, &after).await;
    for said in ["marmalade", "quince and medlar"] {
        assert!(!held_in(&after, said), "the copy still holds `{said}`");
    }
}
