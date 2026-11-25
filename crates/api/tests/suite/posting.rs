//! Whoever says something in a channel is among its members.
//!
//! An agent says something only in a channel it is in: its own post with
//! the factory's command is refused elsewhere, in a sentence that says how
//! it is brought in. A fleet brings each agent it brings up into the
//! channel it reports in, to say there what it is asked there, and takes
//! it out when the agent goes. A person comes into an open channel by what
//! they say there, in the write that says it. An agent asked in a channel
//! it is not in (named there by a person, or asked by a blueprint to say
//! what it did there) is brought in by being asked, so that no answer is
//! lost: which is also how the agents of a fleet from before come into its
//! channel until the fleet is applied again taking what ships.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{key, person};
use crate::lanes::{move_card, moved};

/// The credential an agent's program reads each time it asks the factory anything.
fn token(rig: &Rig, name: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{name}/token"))).unwrap().trim().to_string()
}

/// Who is in a channel, each with whether it listens.
async fn members(rig: &Rig, channel: &str) -> Vec<(String, bool)> {
    let channel = rig.read(&format!("/api/channels/{channel}")).await;
    let listed = channel["members"].as_array().unwrap().iter();
    listed.map(|one| (one["actor"].as_str().unwrap().to_string(), one["listens"] != false)).collect()
}

/// Says `body` in `channel` with `token`: how it went, and the answer.
async fn says(rig: &Rig, token: &str, channel: &str, body: &str) -> (StatusCode, Value) {
    let path = format!("/api/channels/{channel}/messages");
    rig.call(Method::POST, &path, Some(token), Some(json!({ "body": body }))).await
}

/// The messages of a channel, each as who said it and what.
async fn messages(rig: &Rig, channel: &str) -> Vec<(String, String)> {
    let read = rig.read(&format!("/api/channels/{channel}/messages")).await;
    let listed = read["messages"].as_array().unwrap().iter();
    listed.map(|one| (one["author"].as_str().unwrap().into(), one["body"].as_str().unwrap().into())).collect()
}

#[tokio::test]
async fn an_agent_says_nothing_in_a_channel_it_is_not_in_and_is_told_how_it_is_brought_in() {
    let rig = Rig::new().await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/agents", json!({ "name": "helper" })).await;
    let helper = token(&rig, "helper");

    let (status, refused) = says(&rig, &helper, "general", "I have done it.").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let sentence = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(sentence.contains("`helper` is not in #general"), "{refused}");
    assert!(sentence.contains(&format!("{} channel add general helper", brand::CLI)), "{refused}");
    assert!(messages(&rig, "general").await.is_empty(), "nothing was said");

    // Brought in by somebody who may, it says what it says there.
    rig.send("/api/channels/general/members", json!({ "actor": "agent:helper" })).await;
    let (status, said) = says(&rig, &helper, "general", "I have done it.").await;
    assert_eq!(status, StatusCode::OK, "{said}");
}

#[tokio::test]
async fn a_person_comes_into_a_channel_by_saying_something_there_and_a_key_that_holds_less_does_not() {
    let rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    let nell = person(&rig, "Nell").await;
    assert!(!members(&rig, "general").await.iter().any(|(actor, _)| *actor == nell.actor));

    // A key of hers that may only read is refused, and she is not in it.
    let reads = key(&rig, &nell.token, &["channels:read"]).await;
    let (status, refused) = says(&rig, &reads, "general", "quince").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(!members(&rig, "general").await.iter().any(|(actor, _)| *actor == nell.actor));

    // One that may write says it as her, and she is in it from then.
    let writes = key(&rig, &nell.token, &["channels"]).await;
    let before = rig.mark();
    let (status, said) = says(&rig, &writes, "general", "quince").await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(members(&rig, "general").await.contains(&(nell.actor.clone(), true)));
    let joined = rig.event("channel.joined", |event| event.seq > before).await;
    assert_eq!(joined.data["member"], json!(nell.actor));
    // That she came in is said before what she said.
    let message = rig.event("channel.message", |event| event.seq > before).await;
    assert!(joined.seq < message.seq);
}

#[tokio::test]
async fn a_fleets_agents_are_in_its_channel_say_what_they_did_there_and_leave_it_as_they_go() {
    let (rig, _at) = crate::fleets::rig().await;
    crate::fleets::shop(&rig, json!({})).await;
    // Each agent it brought up is in the channel it reports in, asked there
    // by its name and not by what names nobody.
    let within = members(&rig, "shop").await;
    for agent in ["agent:shop-worker-1", "agent:shop-reviewer-1"] {
        assert!(within.contains(&(agent.to_string(), false)), "{agent} in {within:?}");
    }

    // A card worked: what the worker did is said there, by the worker.
    let id = crate::fleets::add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "done", 0).await;
    let said = messages(&rig, "shop").await;
    assert!(said.iter().any(|(author, _)| author == "agent:shop-worker-1"), "{said:?}");

    // One more who works is brought in; counted out again, it goes.
    crate::fleets::change(&rig, "agents", "worker", json!({ "count": 2 })).await;
    assert!(members(&rig, "shop").await.contains(&("agent:shop-worker-2".into(), false)));
    crate::fleets::change(&rig, "agents", "worker", json!({ "count": 1 })).await;
    let within = members(&rig, "shop").await;
    assert!(!within.iter().any(|(actor, _)| actor == "agent:shop-worker-2"), "{within:?}");
    assert!(within.contains(&("agent:shop-worker-1".into(), false)), "{within:?}");
}

#[tokio::test]
async fn an_agent_asked_in_a_channel_it_is_not_in_is_brought_in_and_its_answer_is_said_there() {
    // A fleet from before brought its agents into no channel: the worker
    // is taken out of it here, which is where such a fleet's worker is.
    let (rig, _at) = crate::fleets::rig().await;
    crate::fleets::shop(&rig, json!({})).await;
    let out = json!({ "agent": "shop-worker-1" });
    let (status, left) =
        rig.call(Method::POST, "/api/channels/shop/leave", Some(&rig.token), Some(out)).await;
    assert_eq!(status, StatusCode::OK, "{left}");
    assert!(!members(&rig, "shop").await.iter().any(|(actor, _)| actor == "agent:shop-worker-1"));

    // Asked to say what it did there, it is brought in, and its answer is said.
    let id = crate::fleets::add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "done", 0).await;
    assert!(members(&rig, "shop").await.contains(&("agent:shop-worker-1".into(), false)));
    let said = messages(&rig, "shop").await;
    assert!(said.iter().any(|(author, _)| author == "agent:shop-worker-1"), "{said:?}");
}

#[tokio::test]
async fn an_agent_a_person_names_in_a_channel_it_is_not_in_is_brought_in_and_answers_there() {
    let rig = crate::lanes::rig().await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/agents", json!({ "name": "helper", "adapter": "writer" })).await;
    let before = rig.mark();
    rig.send("/api/channels/general/messages", json!({ "body": "@helper a line, please" })).await;
    rig.event("agent.turn.finished", move |event| event.seq > before && event.data["slot"] == "helper").await;
    assert!(members(&rig, "general").await.contains(&("agent:helper".into(), false)));
    let said = crate::soon("the helper's answer said in #general", async {
        loop {
            let said = messages(&rig, "general").await;
            if said.iter().any(|(author, _)| author == "agent:helper") {
                return said;
            }
            rig.event("channel.message", |event| event.data["author"] == "agent:helper").await;
        }
    })
    .await;
    assert!(said.iter().any(|(_, body)| body == "I changed a file."), "{said:?}");

    // Taken away, it is in no channel.
    let taken = rig.send("/api/agents/helper/remove", json!({})).await;
    assert!(taken.is_object(), "{taken}");
    assert!(!members(&rig, "general").await.iter().any(|(actor, _)| actor == "agent:helper"));
}

#[tokio::test]
async fn an_agent_that_may_not_write_in_channels_answers_where_a_person_named_it_and_its_own_post_is_still_refused()
 {
    let rig = crate::lanes::rig().await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/agents", json!({ "name": "helper", "adapter": "writer", "scopes": ["records"] })).await;
    let before = rig.mark();
    rig.send("/api/channels/general/messages", json!({ "body": "@helper a line, please" })).await;
    rig.event("agent.turn.finished", move |event| event.seq > before && event.data["slot"] == "helper").await;
    let said = crate::soon("the helper's answer said in #general", async {
        loop {
            let said = messages(&rig, "general").await;
            if said.iter().any(|(author, _)| author == "agent:helper") {
                return said;
            }
            rig.event("channel.message", |event| event.data["author"] == "agent:helper").await;
        }
    })
    .await;
    assert!(said.contains(&("agent:helper".into(), "I changed a file.".into())), "{said:?}");
    // What it says by its own credential is asked of it as ever.
    let (status, refused) = says(&rig, &token(&rig, "helper"), "general", "and another").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
}
