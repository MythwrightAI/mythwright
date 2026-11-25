//! A conversation (`talk::conversed`): a channel kept to its members
//! that says it is one, written with its members and its first message
//! as one write, one line of each member's box raised by each thing
//! somebody else says there, and opened as a channel by one of them.
//! And an agent the factory does not start, which is of a kept channel
//! it is in as a person is (`talk::kept`).

use std::sync::Arc;

use kernel::{Error, ErrorKind, Principal};
use serde_json::json;
use talk::inbox::Kind;
use talk::{Conversing, Member};

use crate::kept::who;
use crate::{Rig, agent, own};

fn member(actor: &str) -> Member {
    Member::new(actor, actor.split_once(':').unwrap().1)
}

/// Has the rig's channels take these people for who a word names.
fn named(rig: &Rig, people: &[Principal]) {
    let people = people.to_vec();
    rig.channels.names_with(Arc::new(move |_, names| {
        Ok(people.iter().filter(|one| names.contains(&one.id)).cloned().collect())
    }));
}

/// Has the rig's channels take these agents for ones the factory does
/// not start.
fn outside(rig: &Rig, agents: &[&str]) {
    let agents: Vec<String> = agents.iter().map(|name| format!("agent:{name}")).collect();
    rig.channels.outside_with(Arc::new(move |actor| agents.iter().any(|one| one == actor)));
}

/// How many rows a table of the rig's has.
async fn rows(rig: &Rig, table: &'static str) -> i64 {
    let counted = rig.journal.db().call(move |connection| {
        connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))
    });
    counted.await.unwrap()
}

/// What a refusal says, with what was asked for taken out of it.
fn said(refused: &Error, asked: &str) -> (ErrorKind, String) {
    (refused.kind, refused.message.replace(asked, "<it>"))
}

const NOWHERE: &str = "ch_nonesuch";

/// How a conversation that is none is refused.
async fn refused(rig: &Rig, by: &Principal, asked: Conversing<'_>) -> ErrorKind {
    rig.channels.converse(asked, by).await.unwrap_err().kind
}

#[tokio::test]
async fn a_conversation_who_is_in_it_its_first_message_and_the_lines_it_raises_are_one_write_or_none_of_them()
{
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let mira = who("mira", &["channels", "inbox"]);
    let to = [member("user:nell"), member("user:odo")];
    let asked = Conversing { subject: "The quay", text: "quince", people: &to, agents: &[] };
    // Cut where each part of it is said or written: the line of somebody
    // it is to, the message, somebody in it, and the message's own row.
    for cut in [
        "BEFORE INSERT ON events WHEN NEW.topic = 'inbox.told'",
        "BEFORE INSERT ON events WHEN NEW.topic = 'channel.message'",
        "BEFORE INSERT ON events WHEN NEW.topic = 'channel.joined'",
        "BEFORE INSERT ON messages",
        "BEFORE INSERT ON inbox WHEN NEW.owner = 'user:odo'",
    ] {
        let planted = journal.db().call(move |connection| {
            connection.execute_batch(&format!(
                "CREATE TRIGGER cut {cut} BEGIN SELECT RAISE(ABORT, 'cut here'); END;"
            ))
        });
        planted.await.unwrap();
        assert!(channels.converse(asked, &mira).await.is_err(), "{cut}");
        for table in ["channels", "channel_members", "messages", "inbox"] {
            assert_eq!(rows(&rig, table).await, 0, "{table}, cut {cut}");
        }
        assert!(rig.events("**").await.is_empty(), "{cut}");
        // And the memory that answers who is told has nobody in anything.
        assert!(!channels.keeps_any(), "{cut}");
        assert!(channels.kept_with("user:nell").is_empty(), "{cut}");
        journal.db().call(|connection| connection.execute_batch("DROP TRIGGER cut")).await.unwrap();
    }
    // Uncut, all of it is there, said in the order it was written.
    let (conversation, first) = channels.converse(asked, &mira).await.unwrap();
    assert_eq!((rows(&rig, "channels").await, rows(&rig, "channel_members").await), (1, 3));
    assert_eq!((rows(&rig, "messages").await, rows(&rig, "inbox").await), (1, 2));
    assert_eq!(
        rig.topics("**").await,
        [
            "channel.joined",
            "channel.joined",
            "channel.joined",
            "channel.created",
            "channel.message",
            "inbox.told",
            "inbox.told"
        ]
    );
    assert_eq!((first.channel.as_str(), first.body.as_str()), (conversation.id.as_str(), "quince"));
    // Everything said of it takes being in it. What was said there is
    // told to whoever is in it as it is handed over, and names nobody:
    // somebody who has left reads it no longer.
    let (nell, pru) = (who("nell", &["channels", "inbox"]), who("pru", &["channels", "inbox"]));
    for event in rig.events("channel.*").await {
        assert!(journal.may_tell(&nell, &event) && journal.may_tell(&mira, &event), "{}", event.topic);
        assert!(!journal.may_tell(&pru, &event), "{}", event.topic);
    }
    let first_said = rig.events("channel.message").await.remove(0);
    assert_eq!(first_said.source.takes, Some(format!("member:{}", conversation.id)));
    channels.leave(&own(), &conversation.id, "user:nell").await.unwrap();
    assert!(!journal.may_tell(&nell, &first_said) && journal.may_tell(&mira, &first_said));
}

#[tokio::test]
async fn a_conversation_is_one_line_in_the_box_of_each_of_its_members_which_what_somebody_else_says_there_raises()
 {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let scopes = ["channels", "inbox"];
    let (mira, nell, odo) = (who("mira", &scopes), who("nell", &scopes), who("odo", &scopes));
    let ada = who("ada", &["*"]);
    named(&rig, &[mira.clone(), nell.clone(), odo.clone(), ada.clone()]);
    outside(&rig, &["scout"]);

    // Written to two people, an agent the factory does not start and
    // one it starts: a channel kept to them that says what it is, called
    // by its subject and by nothing among the names channels share.
    let to = [member("user:nell"), member("user:odo")];
    let agents = [member("agent:scout"), member("agent:builder")];
    let asked = Conversing { subject: " The quay ", text: "quince for all", people: &to, agents: &agents };
    let (conversation, first) = channels.converse(asked, &mira).await.unwrap();
    assert!(conversation.kept && conversation.conversation && !conversation.alone);
    assert_eq!((conversation.title.as_deref(), &conversation.name), (Some("The quay"), &conversation.id));
    assert_eq!(json!(conversation)["conversation"], json!(true));
    assert_eq!(conversation.members.len(), 5);
    let read = channels.may_read(&conversation.id, &nell).await.unwrap();
    assert_eq!(channels.get(&read, &conversation.id).await.unwrap(), conversation);

    // Each of those it is to who has a box and reads it has its line:
    // not whoever wrote it, and not the agent the factory starts, which
    // is given a turn by what is said to it.
    let scout = agent("scout", &["channels", "inbox"]);
    let line_of = |by: &Principal| {
        let by = by.clone();
        async move { channels.inbox_of(&by, false).await.unwrap() }
    };
    assert_eq!(line_of(&mira).await, (Vec::new(), 0));
    assert!(channels.inbox.list(&agent("builder", &["channels", "inbox"]), true).await.unwrap().is_empty());
    for by in [&nell, &odo, &scout] {
        let (shown, unread) = line_of(by).await;
        assert_eq!((shown.len(), unread), (1, 1), "{}", by.id);
        let (line, said) = (&shown[0].line, shown[0].said.as_ref().unwrap());
        assert_eq!(
            (line.kind, line.about.as_str(), line.place.as_str()),
            (Kind::Message, &*first.channel, &*first.channel)
        );
        assert_eq!(
            (line.said_by.as_deref(), line.from.as_str(), line.count),
            (Some(&*first.id), "user:mira", 1)
        );
        // Shown with its subject, who is in it, who said the latest and its words.
        assert_eq!((said.channel_title.as_deref(), said.conversation), (Some("The quay"), true));
        assert_eq!((said.author_name.as_str(), said.body.as_str()), ("mira", "quince for all"));
        assert_eq!(said.members, conversation.members);
    }

    // Answered: the writer has a line now, and the line of everybody
    // else is raised, never a second row. One that names somebody in it
    // raises that line and no other; one it names who is not in it, and
    // reads it all the same, is told as anybody named is.
    let hers = line_of(&nell).await.0.remove(0).line;
    channels.inbox.read(&nell, &hers.id).await.unwrap();
    channels.inbox.away(&odo, &line_of(&odo).await.0[0].line.id).await.unwrap();
    let answer = channels.post(&conversation.id, &nell, "@mira @odo @ada medlar", None).await.unwrap();
    let (shown, unread) = line_of(&mira).await;
    assert_eq!((shown.len(), unread, shown[0].line.kind), (1, 1, Kind::Message));
    assert_eq!(shown[0].said.as_ref().unwrap().body, "@mira @odo @ada medlar");
    // Put away, it is back; and whoever said it has theirs as it was.
    let (shown, unread) = line_of(&odo).await;
    assert_eq!((shown.len(), unread, shown[0].line.count), (1, 1, 2));
    assert_eq!((shown[0].line.said_by.as_deref(), shown[0].line.away), (Some(&*answer.id), None));
    let (shown, unread) = line_of(&nell).await;
    assert_eq!((shown.len(), unread, shown[0].line.count, shown[0].line.read.is_some()), (1, 0, 1, true));
    let (shown, _) = line_of(&ada).await;
    assert_eq!(
        (shown.len(), shown[0].line.kind, shown[0].line.about.as_str()),
        (1, Kind::Named, &*answer.id)
    );
    assert_eq!(rows(&rig, "inbox").await, 5, "a line each for four in it, and one for somebody named");
    // What its writer says next raises the line of each of the others.
    let before = rig.events("inbox.*").await.len();
    channels.post(&conversation.id, &mira, "sloe", None).await.unwrap();
    assert_eq!(rig.events("inbox.*").await.len(), before + 3);
    assert_eq!(line_of(&nell).await.1, 1);

    // Somebody who leaves has their line passed over: they read the
    // conversation no longer, and nothing of their box says it is there.
    channels.leave(&own(), &conversation.id, "user:odo").await.unwrap();
    assert_eq!(line_of(&odo).await, (Vec::new(), 0));
    channels.post(&conversation.id, &mira, "damson", None).await.unwrap();
    assert_eq!(line_of(&odo).await, (Vec::new(), 0));
    assert_eq!(line_of(&scout).await.0[0].line.count, 4);

    // Somebody added to it has its line from the write that brings them
    // in, by the latest thing somebody else said; and one who comes back
    // with nothing said since has the line they had, as it was.
    let pru = who("pru", &scopes);
    let before = rig.events("inbox.told").await.len();
    channels.join(&own(), &conversation.id, "user:pru", "pru").await.unwrap();
    let (shown, unread) = line_of(&pru).await;
    assert_eq!((shown.len(), unread, shown[0].line.count), (1, 1, 1));
    assert_eq!(shown[0].said.as_ref().unwrap().body, "damson");
    assert_eq!(rig.events("inbox.told").await.len(), before + 1);
    channels.leave(&own(), &conversation.id, "user:pru").await.unwrap();
    channels.join(&own(), &conversation.id, "user:pru", "pru").await.unwrap();
    assert_eq!(line_of(&pru).await.0[0].line.count, 1);
    assert_eq!(rig.events("inbox.told").await.len(), before + 1);
    // An agent the factory starts that is brought in has none.
    let writes = channels.may_write(&conversation.id, &mira).await.unwrap();
    channels.bring(&writes, &conversation.id, "clerk").await.unwrap();
    assert!(channels.inbox.list(&agent("clerk", &["channels", "inbox"]), true).await.unwrap().is_empty());

    // What is said in a channel that is no conversation raises no such
    // line, kept to its members or not, and costs what it always did.
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    channels.join(&own(), &kept.id, "user:nell", "nell").await.unwrap();
    let before = rig.events("**").await.len();
    channels.post(&kept.id, &mira, "greengage", None).await.unwrap();
    assert_eq!(rig.events("**").await.len(), before + 1);
    assert_eq!(line_of(&nell).await.0.len(), 1);

    // The same people and the same subject again are another conversation.
    let (again, _) = channels.converse(asked, &mira).await.unwrap();
    assert_ne!(again.id, conversation.id);
    assert_eq!(line_of(&nell).await.0.len(), 2);

    // Taken away, a conversation takes the lines that open it.
    channels.remove(&again.id, &ada).await.unwrap();
    assert_eq!(line_of(&nell).await.0.len(), 1);
    channels.remove(&conversation.id, &ada).await.unwrap();
    assert_eq!(rows(&rig, "inbox").await, 0);
}

#[tokio::test]
async fn a_conversation_is_refused_what_makes_none_and_is_called_by_its_first_words_where_it_has_no_subject()
{
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let mira = who("mira", &["channels", "inbox"]);
    let to = [member("user:nell")];
    let ask =
        |subject: &'static str, text: &'static str| Conversing { subject, text, people: &to, agents: &[] };
    // Left empty, the subject is the first words of what was said.
    let (short, _) = channels.converse(ask("", "  quince\n and   medlar "), &mira).await.unwrap();
    assert_eq!(short.title.as_deref(), Some("quince and medlar"));
    let long =
        "the quay was to be mended by the end of the month and nobody has yet said who is to pay for it";
    let (called, _) = channels.converse(ask("  ", long), &mira).await.unwrap();
    assert_eq!(called.title.as_deref(), Some("the quay was to be mended by the end of the month and…"));
    assert_eq!(talk::conversed::first_words("überlänge".repeat(20).as_str()).chars().count(), 61);

    // Nothing said, a subject too long, nobody it is to, only whoever
    // writes it, a bot, an agent among the people or a person among the
    // agents: none is a conversation, and nothing is made.
    let made = rows(&rig, "channels").await;
    assert_eq!(refused(&rig, &mira, ask("Quay", "  ")).await, ErrorKind::Invalid);
    let endless = "q".repeat(201);
    let too_long = Conversing { subject: &endless, text: "quince", people: &to, agents: &[] };
    assert_eq!(refused(&rig, &mira, too_long).await, ErrorKind::Invalid);
    let nobody = Conversing { subject: "Quay", text: "quince", people: &[], agents: &[] };
    assert_eq!(refused(&rig, &mira, nobody).await, ErrorKind::Invalid);
    for (people, agents) in [
        (vec![member("user:mira")], vec![]),
        (vec![member("bot:factory")], vec![]),
        (vec![member("agent:scout")], vec![]),
        (vec![], vec![member("user:nell")]),
    ] {
        let asked = Conversing { subject: "Quay", text: "quince", people: &people, agents: &agents };
        assert_eq!(refused(&rig, &mira, asked).await, ErrorKind::Invalid, "{people:?} {agents:?}");
    }
    // Writing one takes changing channels and changing one's own box.
    for scopes in [&["channels", "inbox:read"][..], &["channels:read", "inbox"][..]] {
        let lacks = channels.converse(ask("Quay", "quince"), &who("mira", scopes)).await.unwrap_err();
        assert_eq!(lacks.kind, ErrorKind::Denied, "{scopes:?}");
    }
    assert_eq!(rows(&rig, "channels").await, made);

    // An agent that a channel holds to itself is brought into none.
    let alone = channels.create_kept_alone("Closest", None, "", &mira).await.unwrap();
    let writes = channels.may_write(&alone.id, &mira).await.unwrap();
    channels.bring(&writes, &alone.id, "clerk").await.unwrap();
    let clerk = [member("agent:clerk")];
    let held = Conversing { subject: "Quay", text: "quince", people: &[], agents: &clerk };
    let refused = channels.converse(held, &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("asked nowhere but in"), "{}", refused.message);
    assert_eq!(rows(&rig, "channels").await, made + 1);
    // One the factory does not start is held by nothing of that: it is
    // given no work anywhere.
    outside(&rig, &["clerk"]);
    channels.converse(held, &mira).await.unwrap();

    // A step that asks for a kept channel called what a conversation is
    // about finds no conversation: it makes its own.
    let ensured = channels
        .ensure_as(talk::Asked { name: "quince and medlar", kept: true, ..Default::default() }, &mira)
        .await
        .unwrap();
    assert!(ensured.id != short.id && !ensured.conversation);
}

#[tokio::test]
async fn a_conversation_is_opened_as_a_channel_by_one_of_its_members_with_what_is_said_of_it_or_not_at_all() {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let scopes = ["channels", "inbox"];
    let (mira, nell) = (who("mira", &scopes), who("nell", &scopes));
    let ada = who("ada", &["*"]);
    let to = [member("user:nell")];
    let asked = Conversing { subject: "The quay", text: "quince", people: &to, agents: &[] };
    let (conversation, _) = channels.converse(asked, &mira).await.unwrap();
    channels.post(&conversation.id, &nell, "medlar", None).await.unwrap();
    let writes = |by: &Principal| {
        let (by, id) = (by.clone(), conversation.id.clone());
        async move { channels.may_write(&id, &by).await.unwrap() }
    };

    // Not by somebody who reads it and is not in it, and not opened to
    // everybody while it is a conversation.
    let outsider = channels.open_as_channel(&writes(&ada).await, &conversation.id, None, &ada).await;
    assert_eq!(outsider.unwrap_err().kind, ErrorKind::Denied);
    let refused = channels.open_again(&writes(&ada).await, &conversation.id, None, &ada).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("opens it as a channel first"), "{}", refused.message);
    // Nor where that cannot be said.
    let unsaid = journal.db().call(|connection| {
        connection.execute_batch(
            "CREATE TRIGGER unsaid BEFORE INSERT ON events WHEN NEW.topic = 'channel.kept'
             BEGIN SELECT RAISE(ABORT, 'nothing of a channel is said'); END;",
        )
    });
    unsaid.await.unwrap();
    assert!(channels.open_as_channel(&writes(&nell).await, &conversation.id, None, &nell).await.is_err());
    assert!(channels.get(&own(), &conversation.id).await.unwrap().conversation);
    journal.db().call(|connection| connection.execute_batch("DROP TRIGGER unsaid")).await.unwrap();

    // By one of its members: a channel kept to its members like any,
    // called what was asked, with everything said so far, said to those
    // who are in it and to nobody else.
    let before = rig.events("**").await.len();
    let opened = channels
        .open_as_channel(&writes(&nell).await, &conversation.id, Some(" Quay works "), &nell)
        .await
        .unwrap();
    assert!(opened.kept && !opened.conversation);
    assert_eq!((opened.title.as_deref(), &opened.name), (Some("Quay works"), &opened.id));
    assert_eq!(channels.get(&own(), &conversation.id).await.unwrap(), opened);
    assert_eq!(channels.messages(&own(), &opened.id, None, 10).await.unwrap().len(), 2);
    let said = rig.events("**").await.split_off(before);
    assert_eq!(said.len(), 1);
    let said = &said[0];
    assert_eq!(
        (said.topic.as_str(), &said.data["conversation"], &said.data["was"]),
        ("channel.kept", &json!(false), &json!("The quay"))
    );
    assert_eq!((&said.data["kept"], &said.data["title"]), (&json!(true), &json!("Quay works")));
    assert!(journal.may_tell(&mira, said) && !journal.may_tell(&who("pru", &scopes), said));

    // The lines it raised are as they were, and open it still; what is
    // said there from now on raises none, and names whom it names.
    let (shown, unread) = channels.inbox_of(&mira, false).await.unwrap();
    assert_eq!((shown.len(), unread, shown[0].line.kind), (1, 1, Kind::Message));
    let of_it = shown[0].said.as_ref().unwrap();
    assert_eq!(
        (of_it.body.as_str(), of_it.conversation, of_it.channel_title.as_deref()),
        ("medlar", false, Some("Quay works"))
    );
    let before = rig.events("inbox.*").await.len();
    channels.post(&opened.id, &nell, "sloe", None).await.unwrap();
    assert_eq!(rig.events("inbox.*").await.len(), before);
    assert_eq!(channels.inbox_of(&mira, false).await.unwrap().0[0].line.count, 1);
    // Done twice it is as it was, and nothing is said; it is not undone.
    let again =
        channels.open_as_channel(&writes(&mira).await, &opened.id, Some("Other"), &mira).await.unwrap();
    assert_eq!(again, opened);
    assert_eq!(rig.events("channel.kept").await.len(), 1);
    // And the factory's owner opens it to everybody as any kept channel.
    let open = channels.open_again(&writes(&ada).await, &opened.id, None, &ada).await.unwrap();
    assert_eq!((open.kept, open.name.as_str()), (false, "quay-works"));
}

#[tokio::test]
async fn an_agent_the_factory_does_not_start_is_of_a_kept_channel_it_is_in_as_a_person_is_and_of_no_other() {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let mira = who("mira", &["channels", "inbox"]);
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let other = channels.create_kept("Hiring", None, "", &mira).await.unwrap();
    let writes = channels.may_write(&kept.id, &mira).await.unwrap();
    channels.post(&kept.id, &mira, "quince", None).await.unwrap();
    let message = rig.events("channel.message").await.pop().unwrap();
    for name in ["scout", "builder"] {
        channels.bring(&writes, &kept.id, name).await.unwrap();
    }
    let (scout, builder) = (agent("scout", &["channels"]), agent("builder", &["channels"]));
    let closed = |refused: Error, asked: &str| {
        let nowhere = Error::not_found(format!("there is no channel `{NOWHERE}`"));
        assert_eq!(said(&refused, asked), said(&nowhere, NOWHERE));
    };

    // With nobody to say which agents the factory does not start, none
    // is of a kept channel by being in it.
    closed(channels.may_read(&kept.id, &scout).await.unwrap_err(), &kept.id);
    assert!(!journal.may_tell(&scout, &message));

    // Said to be one, it reads the channel it is in with its own
    // credential, by every door, writes there, lists it and is handed
    // what is said there; and an agent the factory starts, in the same
    // channel with nothing in hand, is as closed out as it was.
    outside(&rig, &["scout", "stray"]);
    assert_eq!(channels.may_read(&kept.id, &scout).await.unwrap().channel(), Some(kept.id.as_str()));
    assert!(channels.may_write(&kept.id, &scout).await.unwrap().writes());
    assert!(channels.may_find(&kept.id, &scout).await.unwrap().writes());
    let listed = channels.readable(&scout).await.unwrap();
    assert_eq!(listed.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), [kept.id.as_str()]);
    assert!(!channels.kept_from(Some(&scout)).await.unwrap().contains(&kept.id));
    let posted = channels.post(&kept.id, &scout, "noted", None).await.unwrap();
    assert!(channels.may_read_message(&posted.id, &scout).await.is_ok());
    assert!(journal.may_tell(&scout, &message));
    closed(channels.may_read(&kept.id, &builder).await.unwrap_err(), &kept.id);
    assert!(!journal.may_tell(&builder, &message));
    assert!(channels.readable(&builder).await.unwrap().is_empty());

    // Not a kept channel it is not in, by any of them; nor is one that
    // is said to be outside and is in nothing of anything.
    let stray = agent("stray", &["channels", "private"]);
    for (by, channel) in [(&scout, &other.id), (&stray, &kept.id), (&stray, &other.id)] {
        closed(channels.may_read(channel, by).await.unwrap_err(), channel);
        closed(channels.may_write(channel, by).await.unwrap_err(), channel);
        closed(channels.may_find(channel, by).await.unwrap_err(), channel);
        assert!(channels.kept_from(Some(by)).await.unwrap().contains(channel));
    }
    let elsewhere = channels.post(&other.id, &mira, "medlar", None).await.unwrap();
    let nothing = channels.may_read_message("msg_nonesuch", &scout).await.unwrap_err();
    let refused = channels.may_read_message(&elsewhere.id, &scout).await.unwrap_err();
    assert_eq!(said(&refused, &elsewhere.id), said(&nothing, "msg_nonesuch"));
    let of_other = rig.events("channel.message").await.pop().unwrap();
    assert!(!journal.may_tell(&scout, &of_other) && !journal.may_tell(&stray, &message));

    // One whose credential lacks the channels' scope is refused in the
    // words a scope is always refused in, which say nothing of what is
    // there: the same for the channel it is in as for one that is not.
    let lacks = agent("scout", &["inbox", "board"]);
    let here = channels.may_read(&kept.id, &lacks).await.unwrap_err();
    let nowhere = channels.may_read(NOWHERE, &lacks).await.unwrap_err();
    assert_eq!((here.kind, said(&here, &kept.id)), (ErrorKind::Denied, said(&nowhere, NOWHERE)));
    assert!(!journal.may_tell(&lacks, &message));
    // What the factory's owner holds reads nothing for it, and it is
    // told nothing by being named: that it left is said to who is in it.
    let trusted = agent("scout", &["channels", "private"]);
    closed(channels.may_read(&other.id, &trusted).await.unwrap_err(), &other.id);
    channels.leave(&writes, &kept.id, "agent:scout").await.unwrap();
    let left = rig.events("channel.left").await.pop().unwrap();
    assert_eq!(left.source.takes, Some(format!("member:{};agent:scout", kept.id)));
    assert!(!journal.may_tell(&scout, &left) && !journal.may_tell(&trusted, &message));
    closed(channels.may_read(&kept.id, &scout).await.unwrap_err(), &kept.id);
    closed(channels.post(&kept.id, &scout, "more", None).await.unwrap_err(), &kept.id);
}
