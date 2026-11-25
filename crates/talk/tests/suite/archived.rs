//! An archived channel (`talk::archived`): read and found by whoever
//! read it, written in by nobody (a person, an agent, the factory's bot
//! and a bot of a blueprint's alike), counted as unread by nobody's box,
//! and given back as it was; each one write with what is said of it.

use std::sync::Arc;

use kernel::{ErrorKind, Principal};
use serde_json::json;
use talk::{Attachments, Channels, Conversing, Member};

use crate::kept::{of, who};
use crate::{Rig, agent, own};

async fn writes(channels: &Channels, channel: &str, by: &Principal) -> talk::Reader {
    channels.may_write(channel, by).await.unwrap()
}

#[tokio::test]
async fn an_archived_channel_is_read_as_before_and_nothing_is_said_there_until_it_is_given_back() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let (mira, nell, otto) =
        (who("mira", &["channels"]), who("nell", &["channels"]), who("otto", &["channels:read"]));
    let open = channels.create("general", Some("shop"), "everything", &mira).await.unwrap();
    channels.post("general", &mira, "quince", None).await.unwrap();
    let before = of(&rig, &open.id).await.len();

    // Archived by somebody who writes there: one thing said, to whoever
    // may read channels, with when and by whom on the row.
    let archived =
        channels.archive(&writes(channels, "general", &nell).await, "general", &nell).await.unwrap();
    let on = archived.archived.clone().expect("archived");
    assert_eq!((on.by.as_str(), on.name.as_str()), ("user:nell", "nell"));
    assert_eq!(channels.get(&own(), &open.id).await.unwrap(), archived, "as the row has it");
    let said = of(&rig, &open.id).await;
    assert_eq!(said.len(), before + 1);
    let event = said.last().unwrap();
    assert_eq!((event.topic.as_str(), event.source.takes.as_deref()), ("channel.archived", None));
    let data = json!({
        "channel": open.id, "name": "general", "by": "user:nell", "by_name": "nell", "at": on.at,
        "project": "shop",
    });
    assert_eq!(event.data, data);
    assert!(rig.journal.may_tell(&otto, event));
    assert!(
        channels.is_archived("general") && channels.is_archived(&open.id) && channels.is_archived("#general")
    );

    // Read and found as before, by whoever read it.
    assert!(channels.may_read("general", &otto).await.is_ok());
    let reader = channels.may_read("general", &otto).await.unwrap();
    assert_eq!(channels.messages(&reader, "general", None, 10).await.unwrap().len(), 1);

    // Nobody says anything there, each told it is archived and how it is
    // given back, with nothing written and nothing said.
    let quietly = rig.events("**").await.len();
    let refusals = [
        channels.post("general", &mira, "medlar", None).await.unwrap_err(),
        channels.post("general", &agent("scout", &["channels"]), "sloe", None).await.unwrap_err(),
        channels.post_speaker(&own(), "general", "the build", talk::Place::default()).await.unwrap_err(),
        channels.post_bot(&own(), "general", "builder", "built", talk::Place::default()).await.unwrap_err(),
    ];
    for refused in refusals {
        assert_eq!(refused.kind, ErrorKind::Conflict);
        assert!(refused.message.contains("#general is archived"), "{}", refused.message);
        assert!(refused.message.contains("channel unarchive general"), "{}", refused.message);
    }
    assert_eq!(rig.events("**").await.len(), quietly);
    let now = channels.get(&own(), "general").await.unwrap();
    assert_eq!(now.members, open.members, "the bot was not put in it by what it was refused");

    // A blueprint's step finds it and says so: it makes no second of the
    // name, and does not give it back.
    let found = channels.ensure("general", Some("shop"), "", &mira).await.unwrap();
    assert_eq!((found.id.as_str(), found.archived.is_some()), (open.id.as_str(), true));
    assert_eq!(rig.events("**").await.len(), quietly);
    // Archived again, nothing changes and nothing is said.
    channels.archive(&writes(channels, "general", &mira).await, "general", &mira).await.unwrap();
    assert_eq!(rig.events("**").await.len(), quietly);

    // Remembered as archived by every opening of the journal.
    let files = Attachments::open(rig.journal.clone(), rig.dir.join("attachments")).await.unwrap();
    assert!(Channels::open(rig.journal.clone(), files).await.unwrap().is_archived("general"));

    // Given back, as it was: written in again, and said once.
    let back = channels.unarchive(&writes(channels, "general", &mira).await, "general", &mira).await.unwrap();
    assert_eq!(back.archived, None);
    let event = of(&rig, &open.id).await.pop().unwrap();
    assert_eq!(event.topic, "channel.unarchived");
    assert_eq!(
        event.data,
        json!({ "channel": open.id, "name": "general", "by": "user:mira", "project": "shop" })
    );
    assert!(!channels.is_archived("general"));
    channels.post("general", &mira, "medlar", None).await.unwrap();
    let again =
        channels.unarchive(&writes(channels, "general", &mira).await, "general", &mira).await.unwrap();
    assert_eq!(again, channels.get(&own(), "general").await.unwrap());

    // Archived and taken away, another made under its name is not archived.
    channels.archive(&writes(channels, "general", &mira).await, "general", &mira).await.unwrap();
    channels.remove("general", &who("ada", &["*"])).await.unwrap();
    channels.create("general", None, "", &mira).await.unwrap();
    assert!(!channels.is_archived("general"));
    channels.post("general", &mira, "quince", None).await.unwrap();
}

#[tokio::test]
async fn a_private_channel_archived_stays_private_and_is_archived_by_its_members_alone() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let (mira, otto) = (who("mira", &["channels"]), who("otto", &["channels"]));
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    // Somebody who is not in it finds no channel to archive.
    let refused = channels.may_write(&kept.id, &otto).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::NotFound);
    let archived = channels.archive(&writes(channels, &kept.id, &mira).await, &kept.id, &mira).await.unwrap();
    assert!(archived.kept && archived.archived.is_some());
    let event = of(&rig, &kept.id).await.pop().unwrap();
    assert_eq!(event.topic, "channel.archived");
    assert_eq!(event.source.takes.as_deref(), Some(format!("member:{}", kept.id).as_str()));
    assert!(rig.journal.may_tell(&mira, &event) && !rig.journal.may_tell(&otto, &event));
    assert_eq!(event.data["title"], "Pay review");
    assert!(channels.may_read(&kept.id, &otto).await.is_err(), "still kept from whoever is not in it");
    let refused = channels.post(&kept.id, &mira, "quince", None).await.unwrap_err();
    assert!(refused.message.contains("`Pay review` is archived"), "{}", refused.message);
}

#[tokio::test]
async fn a_line_of_an_archived_channel_stays_and_opens_it_and_waits_unread_for_nobody_until_it_is_given_back()
{
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let (mira, nell) = (who("mira", &["channels", "inbox"]), who("nell", &["channels", "inbox"]));
    let people = [mira.clone(), nell.clone()];
    channels.names_with(Arc::new(move |_, names| {
        Ok(people.iter().filter(|one| names.contains(&one.id)).cloned().collect())
    }));
    let open = channels.create("general", None, "", &mira).await.unwrap();
    channels.post("general", &mira, "@nell quince", None).await.unwrap();
    let to = [Member::new("user:nell", "nell")];
    let asked = Conversing { subject: "The quay", text: "medlar", people: &to, agents: &[] };
    let (talked, _) = channels.converse(asked, &mira).await.unwrap();
    assert_eq!(channels.inbox.unread(&nell).await.unwrap(), 2);

    for (channel, by) in [(&open.id, &mira), (&talked.id, &nell)] {
        channels.archive(&writes(channels, channel, by).await, channel, by).await.unwrap();
    }
    // Listed, opening its channel, and waiting for nobody.
    assert_eq!(channels.inbox.unread(&nell).await.unwrap(), 0);
    let (shown, unread) = channels.inbox_of(&nell, false).await.unwrap();
    assert_eq!(unread, 0);
    assert_eq!(shown.len(), 2);
    assert!(shown.iter().all(|line| line.said.as_ref().is_some_and(|said| said.channel_archived)));
    let places: Vec<&str> = shown.iter().map(|line| line.line.place.as_str()).collect();
    assert!(places.contains(&open.id.as_str()) && places.contains(&talked.id.as_str()));
    // Nothing raises it: nothing is said there, and whoever comes into
    // an archived conversation is told nothing of it.
    let lines = rig.events("inbox.*").await.len();
    channels.join(&own(), &talked.id, "user:otto", "otto").await.unwrap();
    assert_eq!(rig.events("inbox.*").await.len(), lines);

    // Given back, each waits again as it did.
    for (channel, by) in [(&open.id, &mira), (&talked.id, &mira)] {
        channels.unarchive(&writes(channels, channel, by).await, channel, by).await.unwrap();
    }
    assert_eq!(channels.inbox.unread(&nell).await.unwrap(), 2);
}

#[tokio::test]
async fn a_channel_is_archived_with_what_is_said_of_it_or_not_at_all() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let mira = who("mira", &["channels"]);
    channels.create("general", None, "", &mira).await.unwrap();
    let unsaid = rig.journal.db().call(|connection| {
        connection.execute_batch(
            "CREATE TRIGGER unsaid BEFORE INSERT ON events WHEN NEW.topic LIKE 'channel.%archived'
             BEGIN SELECT RAISE(ABORT, 'nothing of archiving is said'); END;",
        )
    });
    unsaid.await.unwrap();
    assert!(channels.archive(&writes(channels, "general", &mira).await, "general", &mira).await.is_err());
    assert_eq!(channels.get(&own(), "general").await.unwrap().archived, None);
    assert!(!channels.is_archived("general"));
    channels.post("general", &mira, "quince", None).await.unwrap();
}
