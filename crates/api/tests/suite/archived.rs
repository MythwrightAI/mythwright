//! A channel archived and given back, through the daemon's routes
//! (`talk::archived`, `routes/archived.rs`): by whoever may write in it,
//! read and searched as before, written in by nobody, a blueprint's step
//! refused there and let be where it says so, no agent asked there, and
//! refused while something running posts there.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{key, person};
use crate::snapshots::stop_instance;

async fn ask(rig: &Rig, token: &str, path: &str) -> (StatusCode, Value) {
    rig.call(Method::POST, path, Some(token), Some(json!({}))).await
}

/// The words the daemon refused with.
fn why(answer: &Value) -> &str {
    answer["error"]["message"].as_str().unwrap_or_default()
}

/// How many messages a search for `words` finds, once the index has
/// taken in everything that has happened.
async fn found(rig: &Rig, words: &str) -> usize {
    rig.node().app.studio.search.catch_up().await.unwrap();
    let hits = rig.read(&format!("/api/search?q={words}&kind=message")).await;
    hits["hits"].as_array().unwrap().len()
}

/// Applies `source` and waits for its steps: how they ended, and each
/// step's run as the daemon hands it.
async fn applied(rig: &Rig, source: &str) -> Value {
    rig.send("/api/apply", json!({ "source": source, "wait": true })).await
}

#[tokio::test]
async fn an_archived_channel_is_read_and_searched_as_before_and_nobody_says_anything_there_until_it_is_given_back()
 {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let mira = person(&rig, "Mira").await;
    let reads = key(&rig, &mira.token, &["channels:read"]).await;
    let channel = rig.send("/api/channels", json!({ "name": "general" })).await;
    let id = channel["id"].as_str().unwrap().to_string();
    rig.send(&format!("/api/channels/{id}/agents"), json!({ "name": "helper" })).await;
    rig.send("/api/channels/general/messages", json!({ "body": "the quince ledger" })).await;
    rig.event("agent.turn.finished", |_| true).await;
    let turns = rig.read("/api/agents/helper/turns").await;

    // Whoever may only read it may not archive it, nor give it back.
    let (status, refused) = ask(&rig, &reads, "/api/channels/general/archive").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // A member who may write there archives it.
    let (status, archived) = ask(&rig, &mira.token, "/api/channels/general/archive").await;
    assert_eq!(status, StatusCode::OK, "{archived}");
    assert_eq!(archived["archived"]["by"], mira.actor.as_str());
    let said = rig.event("channel.archived", |_| true).await;
    assert_eq!(
        (said.data["channel"].as_str(), said.data["by_name"].as_str()),
        (Some(id.as_str()), Some("Mira"))
    );
    // Listed and read as before, by whoever read it, and searched.
    let listed = rig.read("/api/channels").await;
    let listed = listed["channels"].as_array().unwrap().iter().find(|one| one["id"] == id.as_str()).cloned();
    assert!(listed.is_some_and(|one| one["archived"]["by"] == mira.actor.as_str()));
    let (status, read) = rig.call(Method::GET, "/api/channels/general/messages", Some(&reads), None).await;
    assert_eq!((status, read["messages"].as_array().map(Vec::len)), (StatusCode::OK, Some(2)));
    assert_eq!(found(&rig, "quince").await, 1);

    // Nobody says anything there, each told why and how it is given back.
    for token in [&rig.token, &mira.token] {
        let (status, refused) = rig
            .call(
                Method::POST,
                "/api/channels/general/messages",
                Some(token),
                Some(json!({ "body": "@helper sloe" })),
            )
            .await;
        assert_eq!(status, StatusCode::CONFLICT, "{refused}");
        assert!(why(&refused).starts_with("#general is archived"), "{refused}");
        assert!(why(&refused).contains("channel unarchive general"), "{refused}");
    }
    // A blueprint's step is refused the same, and let be where it says so;
    // making sure of the channel finds it, says so, and makes no other.
    let posting = "blueprint: says\npermissions: [channels]\nsteps:\n  sure: { do: channel.ensure, with: { name: general } }\n  say: { needs: [sure], do: channel.post, with: { channel: general, body: medlar } }\n";
    let failed = applied(&rig, posting).await;
    assert_eq!(failed["finished"]["status"], "failed", "{failed}");
    assert!(failed.to_string().contains("#general is archived"), "{failed}");
    let letting = posting
        .replace("do: channel.post,", "do: channel.post, allow_failure: true,")
        .replace("says", "lets");
    let let_be = applied(&rig, &letting).await;
    assert_eq!(let_be["finished"]["status"], "ok", "{let_be}");
    let steps = let_be["finished"]["steps"].as_array().unwrap();
    let sure = steps.iter().find(|step| step["step"] == "sure").cloned().unwrap();
    assert_eq!(
        (sure["output"]["id"].as_str(), &sure["output"]["archived"]),
        (Some(id.as_str()), &json!(true))
    );
    let named: Vec<Value> = rig.read("/api/channels").await["channels"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|one| one["name"] == "general")
        .cloned()
        .collect();
    assert_eq!(named.len(), 1, "no second channel of the name");
    // No agent is asked anything there.
    let asking = "blueprint: asks\npermissions: [agents, channels]\nsteps:\n  ask: { do: agent.task, with: { slot: helper, prompt: quince, channel: general } }\n";
    let asked = applied(&rig, asking).await;
    assert_eq!(asked["finished"]["status"], "failed", "{asked}");
    assert!(asked.to_string().contains("nobody is asked anything in an archived channel"), "{asked}");
    assert_eq!(rig.read("/api/agents/helper/turns").await, turns);
    assert_eq!(rig.read("/api/channels/general/messages").await["messages"].as_array().unwrap().len(), 2);

    // Given back, as it was, by whoever may write there.
    let (status, back) = ask(&rig, &mira.token, "/api/channels/general/unarchive").await;
    assert_eq!((status, back.get("archived")), (StatusCode::OK, None), "{back}");
    rig.event("channel.unarchived", |_| true).await;
    // Its agent is asked again, and answers there: what is read once it
    // has is the four, and not whichever of three and four came first.
    let now = rig.mark();
    rig.send("/api/channels/general/messages", json!({ "body": "the medlar ledger" })).await;
    rig.event("agent.turn.finished", move |event| event.seq > now).await;
    let said = rig.read("/api/channels/general/messages").await;
    assert_eq!(said["messages"].as_array().unwrap().len(), 4, "{said}");
    assert!(said.to_string().contains("the medlar ledger"), "{said}");
}

#[tokio::test]
async fn a_private_channel_is_archived_by_its_members_and_is_not_there_for_anybody_else() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, otto) = (person(&rig, "Mira").await, person(&rig, "Otto").await);
    let made = json!({ "name": "Pay review", "kept": true });
    let (_, kept) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(made)).await;
    let id = kept["id"].as_str().unwrap().to_string();
    // Somebody not in it is answered as for a channel that is not there.
    let here = ask(&rig, &otto.token, &format!("/api/channels/{id}/archive")).await;
    let there = ask(&rig, &otto.token, "/api/channels/ch_nonesuch/archive").await;
    assert_eq!(here.0, StatusCode::NOT_FOUND);
    assert_eq!(why(&here.1).replace(&id, "<it>"), why(&there.1).replace("ch_nonesuch", "<it>"));
    let (status, archived) = ask(&rig, &mira.token, &format!("/api/channels/{id}/archive")).await;
    assert_eq!((status, archived["kept"].as_bool()), (StatusCode::OK, Some(true)), "{archived}");
    let (_, theirs) = rig.call(Method::GET, "/api/channels", Some(&otto.token), None).await;
    assert!(!theirs.to_string().contains(&id), "{theirs}");
    // The factory's owner gives it back, as its door says.
    let (status, back) = ask(&rig, &rig.token, &format!("/api/channels/{id}/unarchive")).await;
    assert_eq!(status, StatusCode::OK, "{back}");
}

#[tokio::test]
async fn a_channel_something_running_posts_in_is_not_archived_until_that_is_stopped() {
    let rig = Rig::new().await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    let params = json!({ "channel": "general", "name": "Parrot", "run": "exit 0" });
    let bot = json!({ "blueprint": "std/bot", "key": "bot-parrot", "params": params, "wait": true });
    let bot = rig.send("/api/apply", bot).await["instance"].as_str().unwrap().to_string();
    let (status, refused) = ask(&rig, &rig.token, "/api/channels/general/archive").await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(
        why(&refused).starts_with(&format!("#general is where `bot-parrot` (std/bot, {bot}) posts")),
        "{refused}"
    );
    assert!(why(&refused).contains(&format!("stop {bot}")), "{refused}");
    stop_instance(&rig, &bot).await;
    assert_eq!(ask(&rig, &rig.token, "/api/channels/general/archive").await.0, StatusCode::OK);
    // An archived channel is taken away like any.
    let (status, gone) = rig.call(Method::DELETE, "/api/channels/general", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
}
