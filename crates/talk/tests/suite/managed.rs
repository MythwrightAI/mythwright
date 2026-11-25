//! Who changes a channel's details (`talk::managed`): the rule, held to
//! the table the client's own is held to; and what is name, what it says
//! it is for and who manages it, each changed by a manager and refused to
//! anybody else, the last manager staying.

use kernel::{ErrorKind, Principal};
use serde::Deserialize;
use serde_json::json;
use talk::{Channel, Channels, Conversing, Member, managed};

use crate::kept::{of, who};
use crate::{Rig, own};

#[derive(Deserialize)]
struct Case {
    said: String,
    channel: Shape,
    actor: String,
    scopes: Vec<String>,
    manages: bool,
}

#[derive(Deserialize)]
struct Shape {
    #[serde(default)]
    kept: bool,
    #[serde(default)]
    conversation: bool,
    members: Vec<Seat>,
}

#[derive(Deserialize)]
struct Seat {
    actor: String,
    #[serde(default)]
    manages: bool,
}

fn cases() -> Vec<Case> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/managedcases.json");
    let kept = std::fs::read_to_string(path).expect("the table of who manages is in the client's sources");
    serde_json::from_str(&kept).expect("the table is a list of cases")
}

fn channel_of(shape: &Shape) -> Channel {
    let members = shape
        .members
        .iter()
        .map(|seat| Member { manages: seat.manages, ..Member::new(&seat.actor, &seat.actor) })
        .collect();
    Channel {
        id: "ch_1".into(),
        name: "general".into(),
        project: None,
        topic: String::new(),
        created: 0,
        members,
        kept: shape.kept,
        title: shape.kept.then(|| "Pay review".into()),
        alone: false,
        conversation: shape.conversation,
        archived: None,
    }
}

#[test]
fn who_manages_a_channel_is_the_one_rule_the_client_is_held_to_case_for_case() {
    let cases = cases();
    assert!(cases.len() > 10, "the table is there");
    for case in &cases {
        let channel = channel_of(&case.channel);
        assert_eq!(managed::manages_as(&channel, &case.actor, &case.scopes), case.manages, "{}", case.said);
    }
    let has = |wanted: &dyn Fn(&Case) -> bool| cases.iter().any(wanted);
    assert!(
        has(&|case| case.manages && case.scopes.iter().any(|one| one == "admin")),
        "whoever manages the factory"
    );
    assert!(
        has(&|case| !case.manages && case.channel.members.iter().any(|one| one.actor == case.actor)),
        "a member"
    );
    assert!(has(&|case| case.channel.conversation), "a conversation");
    assert!(has(&|case| case.actor.starts_with("agent:")), "an agent");
}

async fn writes(channels: &Channels, channel: &str, by: &Principal) -> talk::Reader {
    channels.may_write(channel, by).await.unwrap()
}

#[tokio::test]
async fn whoever_made_a_channel_manages_it_and_a_member_who_is_not_a_manager_changes_none_of_its_details() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let (ada, mira) = (who("ada", &["channels"]), who("mira", &["channels"]));
    let owen = who("owen", &["channels", "admin"]);
    let made = channels.create("general", None, "everything", &ada).await.unwrap();
    assert!(made.members[0].manages, "whoever made it manages it");
    channels.join(&writes(channels, "general", &ada).await, "general", "user:mira", "mira").await.unwrap();
    let now = channels.get(&own(), "general").await.unwrap();
    assert_eq!(managed::managers(&now), vec!["ada"], "whoever joins manages nothing");

    // Mira writes there and manages nothing: refused, saying who does.
    let quietly = rig.events("**").await.len();
    let refusals = [
        channels
            .rename(&writes(channels, "general", &mira).await, "general", "lobby", &mira)
            .await
            .unwrap_err(),
        channels
            .describe(&writes(channels, "general", &mira).await, "general", "quinces", &mira)
            .await
            .unwrap_err(),
        channels
            .manage(&writes(channels, "general", &mira).await, "general", "user:mira", true, &mira)
            .await
            .unwrap_err(),
    ];
    for refused in refusals {
        assert_eq!(refused.kind, ErrorKind::Denied, "{}", refused.message);
        assert!(
            refused.message.starts_with("mira is not one of its managers: #general is managed by ada"),
            "{}",
            refused.message
        );
        assert!(refused.message.contains("(needs `admin`)"), "{}", refused.message);
    }
    assert_eq!(rig.events("**").await.len(), quietly, "nothing written, nothing said");

    // Its manager says what it is for, as one line, said once.
    let described = channels
        .describe(&writes(channels, "general", &ada).await, "general", "  the quince orchard  ", &ada)
        .await
        .unwrap();
    assert_eq!(described.topic, "the quince orchard");
    assert_eq!(channels.get(&own(), "general").await.unwrap().topic, "the quince orchard");
    let said = of(&rig, &made.id).await.pop().unwrap();
    assert_eq!(said.topic, "channel.described");
    let data = json!({
        "channel": made.id, "name": "general", "topic": "the quince orchard", "was": "everything",
        "by": "user:ada", "by_name": "ada",
    });
    assert_eq!(said.data, data);
    let again = rig.events("**").await.len();
    channels
        .describe(&writes(channels, "general", &ada).await, "general", "the quince orchard", &ada)
        .await
        .unwrap();
    assert_eq!(rig.events("**").await.len(), again, "said so already, nothing is said");
    let refused = channels
        .describe(&writes(channels, "general", &ada).await, "general", "two\nlines", &ada)
        .await
        .unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    let long = "q".repeat(managed::MAX_TOPIC + 1);
    let refused = channels
        .describe(&writes(channels, "general", &ada).await, "general", &long, &ada)
        .await
        .unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    // Nothing at all is a description too: it says nothing.
    let emptied =
        channels.describe(&writes(channels, "general", &ada).await, "general", "", &ada).await.unwrap();
    assert_eq!(emptied.topic, "");

    // She makes Mira one, who renames it; somebody not in it is no manager.
    let refused = channels
        .manage(&writes(channels, "general", &ada).await, "general", "user:nell", true, &ada)
        .await
        .unwrap_err();
    assert!(refused.message.contains("is not in #general"), "{}", refused.message);
    let named = channels
        .manage(&writes(channels, "general", &ada).await, "general", "user:mira", true, &ada)
        .await
        .unwrap();
    assert_eq!(managed::managers(&named), vec!["ada", "mira"]);
    let said = of(&rig, &made.id).await.pop().unwrap();
    assert_eq!(said.topic, "channel.managers");
    assert_eq!(
        (said.data["member"].clone(), said.data["manages"].clone()),
        (json!("user:mira"), json!(true))
    );
    assert_eq!(said.data["managers"], json!(["user:ada", "user:mira"]));
    let renamed =
        channels.rename(&writes(channels, "general", &mira).await, "general", "lobby", &mira).await.unwrap();
    assert_eq!(renamed.name, "lobby");

    // The last manager stays: Ada is let go only while Mira manages it.
    channels
        .manage(&writes(channels, "lobby", &mira).await, "lobby", "user:ada", false, &mira)
        .await
        .unwrap();
    let refused = channels
        .manage(&writes(channels, "lobby", &mira).await, "lobby", "user:mira", false, &mira)
        .await
        .unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("last manager stays"), "{}", refused.message);
    // Whoever manages the factory does all of it, in it or not.
    let by_owen =
        channels.describe(&writes(channels, "lobby", &owen).await, "lobby", "fruit", &owen).await.unwrap();
    assert_eq!(by_owen.topic, "fruit");
    let ada_again = channels
        .manage(&writes(channels, "lobby", &owen).await, "lobby", "user:ada", true, &owen)
        .await
        .unwrap();
    assert_eq!(managed::managers(&ada_again), vec!["ada", "mira"]);
}

#[tokio::test]
async fn a_private_channels_managers_are_told_to_its_members_alone_and_a_conversations_people_all_manage_it()
{
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let (mira, nell, otto) = (
        who("mira", &["channels", "inbox"]),
        who("nell", &["channels", "inbox"]),
        who("otto", &["channels"]),
    );
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let reader = writes(channels, &kept.id, &mira).await;
    channels.join(&reader, &kept.id, "user:nell", "nell").await.unwrap();
    channels.manage(&reader, &kept.id, "user:nell", true, &mira).await.unwrap();
    channels.describe(&reader, &kept.id, "salaries", &mira).await.unwrap();
    for event in of(&rig, &kept.id)
        .await
        .iter()
        .filter(|one| one.topic == "channel.managers" || one.topic == "channel.described")
    {
        assert!(rig.journal.may_tell(&nell, event), "{}", event.topic);
        assert!(!rig.journal.may_tell(&otto, event), "{}", event.topic);
    }

    // Every person in a conversation manages it, and goes on doing so
    // once it is opened as a channel.
    let people = [Member::new("user:nell", "nell")];
    let asked = Conversing { subject: "Quinces", text: "how many?", people: &people, agents: &[] };
    let (talk, _) = channels.converse(asked, &mira).await.unwrap();
    let talk_reader = writes(channels, &talk.id, &nell).await;
    assert_eq!(
        channels.describe(&talk_reader, &talk.id, "the orchard", &nell).await.unwrap().topic,
        "the orchard"
    );
    let refused = channels.manage(&talk_reader, &talk.id, "user:nell", true, &nell).await.unwrap_err();
    assert!(refused.message.contains("while it is a conversation"), "{}", refused.message);
    let opened = channels.open_as_channel(&talk_reader, &talk.id, None, &nell).await.unwrap();
    assert!(opened.members.iter().all(|one| one.manages), "{:?}", opened.members);
    let read = channels.get(&own(), &talk.id).await.unwrap();
    assert_eq!(managed::managers(&read), vec!["mira", "nell"]);
    let renamed =
        channels.rename(&writes(channels, &talk.id, &nell).await, &talk.id, "Orchard", &nell).await.unwrap();
    assert_eq!(renamed.title.as_deref(), Some("Orchard"));
}
