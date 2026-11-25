//! A channel called something else, through the daemon's routes
//! (`talk::renamed`, `routes/renamed.rs`): an open one by its name, which
//! is then free, and one kept to its members by its title, which only its
//! members are told; by whoever manages it; refused for a name
//! another has, for one that is archived and while something running
//! finds it by its name; and what keeps something of a channel going on
//! by its id: what was said there, the search index, an agent's turns,
//! the Trash and a copy.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, key, person, say};
use crate::snapshots::{Shared, stop_instance, taken};

async fn rename(rig: &Rig, token: &str, channel: &str, to: &str) -> (StatusCode, Value) {
    rig.call(Method::PATCH, &format!("/api/channels/{channel}"), Some(token), Some(json!({ "name": to })))
        .await
}

/// The words the daemon refused with.
fn why(answer: &Value) -> &str {
    answer["error"]["message"].as_str().unwrap_or_default()
}

/// The titles of what a search for `words` finds among what was said,
/// once the index has taken in everything that has happened.
async fn found(rig: &Rig, words: &str) -> Vec<String> {
    rig.node().app.studio.search.catch_up().await.unwrap();
    let hits = rig.read(&format!("/api/search?q={words}&kind=message")).await;
    hits["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["title"].as_str().unwrap_or_default().into())
        .collect()
}

/// The channels listed by that name.
async fn called(rig: &Rig, name: &str) -> Vec<Value> {
    let listed = rig.read("/api/channels").await;
    listed["channels"].as_array().unwrap().iter().filter(|one| one["name"] == name).cloned().collect()
}

#[tokio::test]
async fn an_open_channel_is_renamed_by_whoever_manages_it_and_its_old_name_is_free() {
    let shared = Shared::new();
    let rig = shared.factory("prod").await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let mira = person(&rig, "Mira").await;
    let reads = key(&rig, &mira.token, &["channels:read"]).await;
    let made = rig.send("/api/channels", json!({ "name": "general" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    rig.send("/api/channels", json!({ "name": "elsewhere" })).await;
    rig.send(&format!("/api/channels/{id}/agents"), json!({ "name": "helper" })).await;
    say(&rig, &rig.token, "general", json!({ "body": "the quince ledger" })).await;
    // An agent asked by a step, of the channel by its name, which its turn keeps.
    let asking = "blueprint: asks\npermissions: [agents, channels]\nsteps:\n  ask: { do: agent.task, with: { slot: helper, prompt: quince, channel: general } }\n";
    // The assistant that is built in does no such work: what is held is the turn it was given.
    rig.send("/api/apply", json!({ "source": asking, "wait": true })).await;
    let turns = rig.read("/api/agents/helper/turns").await;
    let named: Vec<&str> =
        turns["turns"].as_array().unwrap().iter().filter_map(|turn| turn["channel"].as_str()).collect();
    assert!(named.contains(&"general"), "{turns}");

    // Whoever may only read it may not rename it, nor may somebody who
    // writes there and is none of its managers (`talk::managed`).
    let (status, refused) = rename(&rig, &reads, "general", "lobby").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Brought in by the owner (saying something would ask the channel's agent).
    rig.send(&format!("/api/channels/{id}/members"), json!({ "actor": mira.actor })).await;
    let (status, refused) = rename(&rig, &mira.token, "general", "lobby").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(why(&refused).starts_with("Mira is not one of its managers"), "{refused}");
    // Whoever made it makes her one.
    let manages = json!({ "member": mira.actor, "manages": true });
    let (status, named) = rig
        .call(Method::POST, &format!("/api/channels/{id}/managers"), Some(&rig.token), Some(manages))
        .await;
    assert_eq!(status, StatusCode::OK, "{named}");
    // A name another channel has is refused in a sentence, and so is one
    // that comes to no name.
    let (status, refused) = rename(&rig, &mira.token, "general", "Elsewhere").await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert_eq!(why(&refused), "there is already a channel called `elsewhere`");
    let (status, refused) = rename(&rig, &mira.token, "general", " -- ").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");

    // Somebody who may write there renames it, as its name is written.
    let mark = rig.mark();
    let (status, renamed) = rename(&rig, &mira.token, "general", "The Lobby").await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    assert_eq!((renamed["id"].as_str(), renamed["name"].as_str()), (Some(id.as_str()), Some("the-lobby")));
    let said = rig.event("channel.renamed", move |event| event.seq > mark).await;
    assert_eq!(
        (said.data["channel"].as_str(), said.data["was"].as_str(), said.data["name"].as_str()),
        (Some(id.as_str()), Some("general"), Some("the-lobby"))
    );
    assert_eq!(said.data["by"], mira.actor.as_str());
    // Called so again, nothing more is said.
    let again = rig.mark();
    assert_eq!(rename(&rig, &mira.token, &id, "the lobby").await.0, StatusCode::OK);
    rig.send("/api/channels/the-lobby/messages", json!({ "body": "after the medlar" })).await;
    rig.event("channel.message", move |event| event.seq > again).await;
    let renamed =
        journal::EventQuery { after: again, topics: "channel.renamed".into(), ..Default::default() };
    assert!(rig.said(renamed).await.is_empty());

    // Read and written in by its new name, with what was said under its old.
    let read = rig.read("/api/channels/the-lobby/messages").await;
    let bodies: Vec<&str> =
        read["messages"].as_array().unwrap().iter().filter_map(|message| message["body"].as_str()).collect();
    assert!(bodies.contains(&"the quince ledger") && bodies.contains(&"after the medlar"), "{read}");
    // Its turn names it by its id, so that it is still its own.
    let turns = rig.read("/api/agents/helper/turns").await;
    let named: Vec<&str> =
        turns["turns"].as_array().unwrap().iter().filter_map(|turn| turn["channel"].as_str()).collect();
    assert!(!named.contains(&"general") && named.contains(&id.as_str()), "{turns}");
    // The search index says what it is called now.
    assert_eq!(found(&rig, "quince").await.iter().filter(|title| title.starts_with("#the-lobby")).count(), 1);
    // Its old name is free: whoever makes a channel of it makes another.
    let other = rig.send("/api/channels", json!({ "name": "general" })).await;
    assert_ne!(other["id"].as_str(), Some(id.as_str()));
    assert_eq!(called(&rig, "general").await.len(), 1);

    // A copy holds it as it is called now.
    let copied = shared.snapshot("renamed");
    taken(&rig, &copied).await;
    let dev = shared.copy("dev", &copied).await;
    assert_eq!(called(&dev, "the-lobby").await[0]["id"], id.as_str());

    // Into the Trash and back, as it is called now.
    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/channels/{id}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!(gone["name"], "the-lobby");
    let trash = gone["trash"].as_str().unwrap();
    rig.send(&format!("/api/trash/{trash}/restore"), json!({})).await;
    assert_eq!(called(&rig, "the-lobby").await[0]["id"], id.as_str());
}

#[tokio::test]
async fn a_private_channel_is_renamed_by_its_title_for_its_members_alone_and_keeps_its_id() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, otto) = (person(&rig, "Mira").await, person(&rig, "Otto").await);
    let made = json!({ "name": "Pay review", "kept": true });
    let (_, kept) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(made)).await;
    let id = kept["id"].as_str().unwrap().to_string();
    rig.send("/api/channels", json!({ "name": "lobby" })).await;
    let (mut to_mira, mut to_otto) = (Told::of(&rig, &mira.token).await, Told::of(&rig, &otto.token).await);

    // Somebody who is not in it is answered as for a channel that is not there.
    let (status, refused) = rename(&rig, &otto.token, &id, "Gossip").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    // A title is one line, and something.
    let (status, refused) = rename(&rig, &mira.token, &id, "  ").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");

    let (status, renamed) = rename(&rig, &mira.token, &id, "Salary review").await;
    assert_eq!(status, StatusCode::OK, "{renamed}");
    assert_eq!(
        (renamed["id"].as_str(), renamed["name"].as_str(), renamed["title"].as_str()),
        (Some(id.as_str()), Some(id.as_str()), Some("Salary review"))
    );
    to_mira.until("Salary review").await;
    assert!(to_mira.heard.contains("\"was\":\"Pay review\""), "{}", to_mira.heard);
    // Otto is told what is said of the open channel after it, and nothing of this.
    let (status, _) = rename(&rig, &rig.token, "lobby", "hall").await;
    assert_eq!(status, StatusCode::OK);
    to_otto.until("\"was\":\"lobby\"").await;
    assert!(!to_otto.heard.contains("Salary") && !to_otto.heard.contains(&id), "{}", to_otto.heard);
    // Another may be called that too: a title is not a name channels share.
    let again = json!({ "name": "Salary review", "kept": true });
    let (status, _) = rig.call(Method::POST, "/api/channels", Some(&otto.token), Some(again)).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_channel_is_not_renamed_while_it_is_archived_or_something_running_finds_it_by_its_name() {
    let rig = Rig::new().await;
    let made = rig.send("/api/channels", json!({ "name": "general" })).await;
    let id = made["id"].as_str().unwrap().to_string();
    // A bot finds its channel by the name it was told.
    let params = json!({ "channel": "general", "name": "Parrot", "run": "exit 0" });
    let bot = json!({ "blueprint": "std/bot", "key": "bot-parrot", "params": params, "wait": true });
    let bot = rig.send("/api/apply", bot).await["instance"].as_str().unwrap().to_string();
    let (status, refused) = rename(&rig, &rig.token, "general", "lobby").await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(
        why(&refused).starts_with(&format!("#general is where `bot-parrot` (std/bot, {bot}) posts")),
        "{refused}"
    );
    assert!(why(&refused).contains(&format!("stop {bot}")), "{refused}");
    assert_eq!(called(&rig, "general").await.len(), 1);
    // One told it by its id finds it whatever it is called.
    stop_instance(&rig, &bot).await;
    let params = json!({ "channel": id, "name": "Parrot", "run": "exit 0" });
    let by_id = json!({ "blueprint": "std/bot", "key": "bot-parrot", "params": params, "wait": true });
    let by_id = rig.send("/api/apply", by_id).await["instance"].as_str().unwrap().to_string();
    assert_eq!(rename(&rig, &rig.token, "general", "lobby").await.0, StatusCode::OK);
    stop_instance(&rig, &by_id).await;

    // Archived, it is given back first.
    rig.send("/api/channels/lobby/archive", json!({})).await;
    let (status, refused) = rename(&rig, &rig.token, "lobby", "hall").await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(why(&refused).starts_with("#lobby is archived"), "{refused}");
    assert!(why(&refused).contains("channel unarchive lobby"), "{refused}");
    rig.send("/api/channels/lobby/unarchive", json!({})).await;
    assert_eq!(rename(&rig, &rig.token, "lobby", "hall").await.0, StatusCode::OK);
}
