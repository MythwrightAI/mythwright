//! What an agent posts for itself goes where it is being asked.
//!
//! An agent's answer is said where the question was, by the daemon. A file
//! it has made it posts itself, with the factory's own command, and that
//! asks this route: while it is answering somebody in a thread of the
//! channel, what it posts there with no thread said is said in that
//! thread, unless it says the channel itself. Nobody else's post is put
//! anywhere it was not said to be.
//!
//! The agent here reads what it was asked and then waits to be let go,
//! so that its turn is in hand while the test posts as it, with its own
//! credential, as its program would.

use std::path::PathBuf;

use reqwest::StatusCode;
use serde_json::{Value, json};

use crate::Rig;

/// A node with an agent that waits at a hold before it answers, and the hold.
async fn rig() -> (Rig, PathBuf) {
    let mut gate = None;
    let rig = Rig::with("api-threaded", |config, dir| {
        let pipe = dir.join("gate");
        crate::hold(&pipe);
        let does = ["swallow", "held", &pipe.to_string_lossy(), "waiter", "say", "Here they are."];
        config.adapters.push(("waiter".into(), crate::standin::argv(&does)));
        gate = Some(pipe);
    })
    .await;
    (Rig::owned(rig).await, gate.unwrap())
}

/// The credential an agent's program reads each time it asks the factory anything.
fn token(rig: &Rig, name: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{name}/token"))).unwrap().trim().to_string()
}

/// Posts in a channel as whoever holds `token`: a file put there first,
/// and then the message that names it, as the command line does.
async fn posts(rig: &Rig, token: &str, channel: &str, more: Value) -> Value {
    let url = format!("{}/api/channels/{channel}/attachments?name=palettes.html", rig.url);
    let kept =
        rig.client.post(url).bearer_auth(token).body(b"<h1>Six ways</h1>".to_vec()).send().await.unwrap();
    assert_eq!(kept.status(), StatusCode::OK);
    let kept: Value = kept.json().await.unwrap();
    let mut body = json!({ "body": "The palettes.", "attachments": [kept["id"]] });
    body.as_object_mut().unwrap().extend(more.as_object().cloned().unwrap_or_default());
    let path = format!("/api/channels/{channel}/messages");
    let (status, said) = rig.call(reqwest::Method::POST, &path, Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said["attachments"][0]["name"], "palettes.html", "{said}");
    said
}

/// Waits for `slot`'s next turn to be in hand, after `after` in the journal.
async fn in_hand(rig: &Rig, slot: &'static str, after: u64) {
    rig.event("agent.turn.started", move |event| event.seq > after && event.data["slot"] == slot).await;
}

async fn answered(rig: &Rig, slot: &'static str, after: u64) {
    rig.event("agent.turn.finished", move |event| event.seq > after && event.data["slot"] == slot).await;
}

#[tokio::test]
async fn what_an_agent_posts_while_it_answers_in_a_thread_is_said_in_that_thread_unless_it_says_the_channel_itself()
 {
    let (rig, gate) = rig().await;
    let seq = || rig.mark();
    let channel = rig.send("/api/channels", json!({ "name": "studio" })).await;
    let id = channel["id"].as_str().unwrap().to_string();
    rig.send("/api/channels", json!({ "name": "elsewhere" })).await;
    rig.send("/api/agents", json!({ "name": "maker", "adapter": "waiter", "channel": "studio" })).await;
    rig.send("/api/agents", json!({ "name": "idler", "channel": "studio" })).await;
    // In the other channel too: an agent says something only where it is in.
    rig.send("/api/channels/elsewhere/members", json!({ "actor": "agent:maker" })).await;
    let (maker, idler) = (token(&rig, "maker"), token(&rig, "idler"));
    let say = async |body: Value| rig.send(&format!("/api/channels/{id}/messages"), body).await;

    // Asked in the channel itself, what it posts while it answers is in
    // the channel, as its answer is.
    let before = seq();
    let asked = say(json!({ "body": "@maker three palettes, please" })).await;
    in_hand(&rig, "maker", before).await;
    assert_eq!(posts(&rig, &maker, &id, json!({})).await["thread"], Value::Null);
    crate::let_go("the agent to be waiting at its pipe", &gate).await;
    answered(&rig, "maker", before).await;

    // Asked under that message, and still answering: what it posts with
    // no thread said is said under it too, the file with it.
    let before = seq();
    say(json!({ "body": "@maker three more of each", "thread": asked["id"] })).await;
    in_hand(&rig, "maker", before).await;
    let page = posts(&rig, &maker, &id, json!({})).await;
    assert_eq!(page["thread"], asked["id"], "it was put where the agent is being asked: {page}");

    // Saying the channel itself, it is said there; and saying a thread,
    // in that thread, whichever it is being asked in.
    assert_eq!(posts(&rig, &maker, &id, json!({ "top": true })).await["thread"], Value::Null);
    let other = say(json!({ "body": "Something else." })).await;
    let there = posts(&rig, &maker, &id, json!({ "thread": other["id"] })).await;
    assert_eq!(there["thread"], other["id"], "{there}");

    // In another channel it is being asked nothing, and posts in that channel.
    assert_eq!(posts(&rig, &maker, "elsewhere", json!({})).await["thread"], Value::Null);
    // An agent with nothing in hand here, and a person, post where they say.
    assert_eq!(posts(&rig, &idler, &id, json!({})).await["thread"], Value::Null);
    assert_eq!(posts(&rig, &rig.token, &id, json!({})).await["thread"], Value::Null);

    // Its answer is said in the thread, to the message that asked, as it always was.
    crate::let_go("the agent to be waiting at its pipe", &gate).await;
    answered(&rig, "maker", before).await;
    let thread =
        rig.read(&format!("/api/channels/{id}/messages?thread={}", asked["id"].as_str().unwrap())).await;
    let said: Vec<(&str, &str)> = thread["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|message| (message["author_name"].as_str().unwrap(), message["body"].as_str().unwrap()))
        .collect();
    assert_eq!(
        said,
        [
            ("Ada", "@maker three palettes, please"),
            ("Ada", "@maker three more of each"),
            ("maker", "The palettes."),
            ("maker", "Here they are."),
        ],
        "the page it made and its answer, under what was asked"
    );

    // With that answered it has nothing in hand, and posts in the channel again.
    assert_eq!(posts(&rig, &maker, &id, json!({})).await["thread"], Value::Null);
}

#[tokio::test]
async fn what_an_agent_posts_while_a_turn_is_in_hand_there_says_the_model_its_answer_says() {
    let (rig, gate) = rig().await;
    let channel = rig.send("/api/channels", json!({ "name": "studio" })).await;
    let id = channel["id"].as_str().unwrap().to_string();
    rig.send("/api/channels", json!({ "name": "elsewhere" })).await;
    rig.send(
        "/api/agents",
        json!({ "name": "maker", "adapter": "waiter", "model": "m-1", "channel": "studio" }),
    )
    .await;
    rig.send("/api/channels/elsewhere/members", json!({ "actor": "agent:maker" })).await;
    let maker = token(&rig, "maker");

    let before = rig.mark();
    rig.send(&format!("/api/channels/{id}/messages"), json!({ "body": "@maker three palettes, please" }))
        .await;
    in_hand(&rig, "maker", before).await;
    let page = posts(&rig, &maker, &id, json!({})).await;
    assert_eq!(page["model"], "m-1", "what it posts beside its answer says what wrote it: {page}");
    // Where it has nothing in hand, it says nothing of what wrote it, as before.
    assert_eq!(posts(&rig, &maker, "elsewhere", json!({})).await["model"], Value::Null);
    crate::let_go("the agent to be waiting at its pipe", &gate).await;
    answered(&rig, "maker", before).await;

    let said = rig.read(&format!("/api/channels/{id}/messages")).await;
    let models: Vec<(&str, &Value)> = said["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["author_name"] == "maker")
        .map(|message| (message["body"].as_str().unwrap(), &message["model"]))
        .collect();
    assert_eq!(
        models,
        [("The palettes.", &json!("m-1")), ("Here they are.", &json!("m-1"))],
        "the page and its answer, one model"
    );
    // With that answered it has nothing in hand, and what it posts says nothing of what wrote it.
    assert_eq!(posts(&rig, &maker, &id, json!({})).await["model"], Value::Null);
}
