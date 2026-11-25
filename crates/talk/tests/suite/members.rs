//! Whoever says something in a channel is in it: a person comes into an
//! open one by what they say, in the write that says it; an agent says
//! something only in one it is in, but for its answer to what it was
//! asked there; a bot, and what is said as the system, as they were.

use kernel::{ErrorKind, Principal, PrincipalKind};
use serde_json::json;

use crate::{Rig, ada, agent, own};

fn nell() -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: "nell".into(),
        name: "Nell".into(),
        scopes: vec!["channels".into()],
        key: None,
    }
}

fn members(channel: &talk::Channel) -> Vec<(&str, bool)> {
    channel.members.iter().map(|member| (member.actor.as_str(), member.listens)).collect()
}

#[tokio::test]
async fn an_agent_says_nothing_in_an_open_channel_it_is_not_in_and_is_told_how_it_is_brought_in() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    channels.create("general", None, "", &ada()).await.unwrap();
    let scout = agent("scout", &["channels"]);
    let quietly = rig.events("**").await.len();

    let refused = channels.post("general", &scout, "sloe", None).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert!(refused.message.contains("`scout` is not in #general"), "{}", refused.message);
    assert!(refused.message.contains("channel add general scout"), "{}", refused.message);
    // Nothing written and nothing said: not the message, and not that it came in.
    assert_eq!(rig.events("**").await.len(), quietly);
    assert!(channels.messages(&own(), "general", None, 10).await.unwrap().is_empty());
    assert_eq!(members(&channels.get(&own(), "general").await.unwrap()), [("user:ada", true)]);

    // Brought in, it says what it says.
    channels.bring(&own(), "general", "scout").await.unwrap();
    channels.post("general", &scout, "sloe", None).await.unwrap();
}

#[tokio::test]
async fn a_person_comes_into_an_open_channel_by_what_they_say_in_the_write_that_says_it() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    channels.create("general", None, "", &ada()).await.unwrap();
    let before = rig.events("channel.*").await.len();

    let said = channels.post("general", &nell(), "quince", None).await.unwrap();
    let general = channels.get(&own(), "general").await.unwrap();
    assert_eq!(members(&general), [("user:ada", true), ("user:nell", true)]);
    // That she came in is said first, by her, and then what she said.
    let after = rig.events("channel.*").await;
    let new: Vec<(&str, Option<&str>)> =
        after[before..].iter().map(|event| (event.topic.as_str(), event.source.actor.as_deref())).collect();
    assert_eq!(new, [("channel.joined", Some("user:nell")), ("channel.message", Some("user:nell"))]);
    assert_eq!(after[before].data["member"], "user:nell");
    assert_eq!(after[before + 1].data["id"], json!(said.id));

    // Saying something again is no second coming in.
    channels.post("general", &nell(), "medlar", None).await.unwrap();
    assert_eq!(rig.topics("channel.joined").await.len(), 2);
}

#[tokio::test]
async fn an_agents_answer_brings_it_into_an_open_channel_to_say_what_it_is_asked_there() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    channels.create("general", None, "", &ada()).await.unwrap();
    let asked = channels.post("general", &ada(), "@scout what is ripe?", None).await.unwrap();
    let scout = agent("scout", &["channels"]);

    let place = talk::Place { reply_to: Some(&asked.id), answers: true, ..talk::Place::default() };
    channels.post_at("general", &scout, "the quinces", place).await.unwrap();
    let general = channels.get(&own(), "general").await.unwrap();
    // In, asked there by its name: said so of it, and of nobody else.
    assert_eq!(members(&general), [("user:ada", true), ("agent:scout", false)]);
    let shown = serde_json::to_value(&general).unwrap();
    // Whoever made it manages it, said where it is so.
    assert_eq!(shown["members"][0], json!({ "actor": "user:ada", "name": "Ada", "manages": true }));
    assert_eq!(shown["members"][1], json!({ "actor": "agent:scout", "name": "scout", "listens": false }));
    assert_eq!(channels.asked_into("scout").await.unwrap(), [general.id.as_str()]);

    // Brought in to listen, it listens; brought in to be asked again, it
    // is not made quieter.
    channels.bring(&own(), "general", "scout").await.unwrap();
    channels.bring_asked(&own(), "general", "scout").await.unwrap();
    let general = channels.get(&own(), "general").await.unwrap();
    assert_eq!(members(&general), [("user:ada", true), ("agent:scout", true)]);
    assert!(channels.asked_into("scout").await.unwrap().is_empty());
    assert_eq!(rig.topics("channel.joined").await.len(), 2, "it came in once");
}

#[tokio::test]
async fn an_agent_brought_in_to_be_asked_leaves_by_the_door_anybody_leaves_by() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let general = channels.create("general", None, "", &ada()).await.unwrap();
    channels.bring_asked(&own(), "general", "scout").await.unwrap();
    assert_eq!(channels.asked_into("scout").await.unwrap(), [general.id.as_str()]);
    channels.leave(&own(), "general", "agent:scout").await.unwrap();
    assert!(channels.asked_into("scout").await.unwrap().is_empty());
    assert_eq!(members(&channels.get(&own(), "general").await.unwrap()), [("user:ada", true)]);
    let refused = channels.post("general", &agent("scout", &["channels"]), "sloe", None).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
}

#[tokio::test]
async fn a_bot_and_what_is_said_as_the_system_are_said_as_they_were() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    channels.create("general", None, "", &ada()).await.unwrap();
    let place = talk::Place::default;
    channels.post_bot(&own(), "general", "watch", "a card moved", place()).await.unwrap();
    channels.post_system(&own(), "general", "factory", "started").await.unwrap();
    // The factory's own bot comes in by what it says, as it always did.
    channels.post_speaker(&own(), "general", "the build passed", place()).await.unwrap();
    let general = channels.get(&own(), "general").await.unwrap();
    let speaker = talk::channels::bot_actor(&channels.speaker.name());
    assert_eq!(members(&general), [("user:ada", true), (speaker.as_str(), true)]);
}

#[tokio::test]
async fn somebody_who_reads_a_kept_channel_without_being_in_it_is_not_brought_in_by_saying_something() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let kept = channels.create_kept("plans", None, "", &nell()).await.unwrap();
    // Ada holds what the factory's owner holds, which reads every kept channel.
    channels.post(&kept.id, &ada(), "noted", None).await.unwrap();
    let kept = channels.get(&own(), &kept.id).await.unwrap();
    assert_eq!(members(&kept), [("user:nell", true)]);
}
