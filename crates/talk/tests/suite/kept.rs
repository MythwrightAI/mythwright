//! A channel kept to its members (`talk::kept`): who it is there for, by
//! each door; what leave for one channel reads of another; who is told
//! what is said of it; and that nothing this crate offers hands a
//! channel, a message or something attached out unasked.

use std::path::Path;

use kernel::{Error, ErrorKind, Event, Principal, PrincipalKind};
use talk::{Attachments, Channels};

use crate::{Rig, ada, own};

/// Somebody signed in, holding `scopes`.
pub(crate) fn who(id: &str, scopes: &[&str]) -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: id.into(),
        name: id.into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    }
}

/// A key `maker` made, holding `scopes`: it acts as its maker.
fn key_of(maker: &str, scopes: &[&str]) -> Principal {
    Principal { key: Some(format!("cred_{maker}")), ..who(maker, scopes) }
}

/// What a refusal says, with what was asked for taken out of it.
fn said(refused: &Error, asked: &str) -> (ErrorKind, String) {
    (refused.kind, refused.message.replace(asked, "<it>"))
}

const NOWHERE: &str = "ch_nonesuch";

#[tokio::test]
async fn a_kept_channel_is_there_for_whoever_reads_it_and_answers_everybody_else_as_one_that_is_not_there() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let mira = who("mira", &["channels"]);
    let kept = channels.create_kept("Pay review", None, "who is paid what", &mira).await.unwrap();
    assert!(kept.kept && kept.name == kept.id && kept.title.as_deref() == Some("Pay review"), "{kept:?}");
    assert_eq!(kept.members.iter().map(|one| one.actor.as_str()).collect::<Vec<_>>(), ["user:mira"]);

    // Who it is there for: (who, reads, writes, is found with no scope of the channels' asked).
    let cases: &[(&str, Principal, bool, bool, bool)] = &[
        ("a member", mira.clone(), true, true, true),
        ("a member who may only read", who("mira", &["channels:read"]), true, false, true),
        ("a member who may write and not read", who("mira", &["channels:write"]), false, false, false),
        (
            "somebody else, holding what a person is given",
            who("nell", kernel::principal::MEMBER_SCOPES),
            false,
            false,
            false,
        ),
        (
            "somebody else who manages the factory",
            who("nils", &["admin", "channels", "projects"]),
            false,
            false,
            false,
        ),
        ("whoever holds what the owner holds", who("owen", &["channels", "private"]), true, true, true),
        (
            "whoever holds it and no scope of the channels'",
            who("owen", &["private", "admin"]),
            false,
            false,
            false,
        ),
        (
            "whoever holds only what is private under one word",
            who("nell", &["channels", "private:pay"]),
            false,
            false,
            false,
        ),
        ("a member's key that holds the scope", key_of("mira", &["channels:read"]), true, false, true),
        ("a member's key that does not", key_of("mira", &["board", "admin"]), false, false, false),
        ("the system", Principal::system(), true, true, true),
    ];
    for (what, by, reads, writes, found) in cases {
        for (door, opens) in [("read", *reads), ("write", *writes), ("find", *found)] {
            let asked = |channel: &'static str| {
                let (by, id) =
                    (by.clone(), if channel == NOWHERE { NOWHERE.to_string() } else { kept.id.clone() });
                async move {
                    match door {
                        "read" => channels.may_read(&id, &by).await,
                        "write" => channels.may_write(&id, &by).await,
                        _ => channels.may_find(&id, &by).await,
                    }
                }
            };
            let (here, nowhere) = (asked("kept").await, asked(NOWHERE).await.unwrap_err());
            match (opens, here) {
                (true, Ok(reader)) => assert_eq!(reader.channel(), Some(kept.id.as_str())),
                // Refused exactly as for a channel that is not there: the
                // scope's words where the scope is lacking, and where it
                // is not, that there is no such channel.
                (false, Err(refused)) => {
                    assert_eq!(said(&refused, &kept.id), said(&nowhere, NOWHERE), "{what}, to {door}");
                }
                (_, here) => panic!("{what}, to {door}: {here:?}"),
            }
        }
        // And it is listed for those who read it and for nobody else.
        let listed = channels.readable(by).await.map(|all| all.iter().any(|one| one.id == kept.id));
        assert_eq!(listed.unwrap_or(false), *reads, "{what}, listed");
        let left_out = channels.kept_from(Some(by)).await.unwrap().contains(&kept.id);
        assert_eq!(left_out, !found, "{what}, left out of what lists channels by id");
    }
    assert!(channels.kept_from(None).await.unwrap().contains(&kept.id));

    // What takes a principal asks the same: said, taken away, something attached.
    let nell = who("nell", &["channels", "admin", "search"]);
    let words = channels.post(&kept.id, &mira, "the marked word quince", None).await.unwrap();
    let file = channels
        .attachments
        .add(&channels.may_write(&kept.id, &mira).await.unwrap(), &kept.id, &mira, "pay.txt", None, vec![1])
        .await
        .unwrap();
    let refused = channels.post(&kept.id, &nell, "hello?", None).await.unwrap_err();
    assert_eq!(
        said(&refused, &kept.id),
        said(&channels.post(NOWHERE, &nell, "x", None).await.unwrap_err(), NOWHERE)
    );
    let refused = channels.remove(&kept.id, &nell).await.unwrap_err();
    assert_eq!(said(&refused, &kept.id), said(&channels.remove(NOWHERE, &nell).await.unwrap_err(), NOWHERE));
    let refused = channels.attached(&file.id, &nell).await.unwrap_err();
    let nothing = channels.attached("att_nonesuch", &nell).await.unwrap_err();
    assert_eq!(said(&refused, &file.id), said(&nothing, "att_nonesuch"));
    assert_eq!(channels.attached(&file.id, &mira).await.unwrap().0.name, "pay.txt");
    // Leave for another channel reads nothing of it either.
    let open = channels.create("general", None, "", &nell).await.unwrap();
    let elsewhere = channels.may_write(&open.id, &nell).await.unwrap();
    for refused in [
        channels.get(&elsewhere, &kept.id).await.map(|_| ()),
        channels.messages(&elsewhere, &kept.id, None, 10).await.map(|_| ()),
        channels.thread(&elsewhere, &words.id).await.map(|_| ()),
        channels.message(&elsewhere, &words.id).await.map(|_| ()),
        channels.join(&elsewhere, &kept.id, "user:nell", "nell").await,
        channels.leave(&elsewhere, &kept.id, "user:mira").await,
        channels.post_speaker(&elsewhere, &kept.id, "hello", talk::Place::default()).await.map(|_| ()),
    ] {
        assert_eq!(refused.unwrap_err().kind, ErrorKind::NotFound);
    }
    assert_eq!(channels.list(&elsewhere).await.unwrap(), std::slice::from_ref(&open));
    assert!(channels.all_messages(&elsewhere, 0, 100).await.unwrap().is_empty());
    assert_eq!(channels.answered_by(&elsewhere, &words.id).await.unwrap(), Vec::<String>::new());
    // And leave to read is no leave to write.
    let reads = channels.may_read(&kept.id, &mira).await.unwrap();
    assert!(!reads.writes());
    let refused = channels.post_speaker(&reads, &kept.id, "hello", talk::Place::default()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert_eq!(
        channels.join(&reads, &kept.id, "user:nell", "nell").await.unwrap_err().kind,
        ErrorKind::Denied
    );
    assert_eq!(channels.messages(&reads, &kept.id, None, 10).await.unwrap().len(), 1, "nothing was said");
}

#[tokio::test]
async fn what_a_kept_channel_is_called_is_not_among_the_names_channels_share() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let (mira, nell) = (who("mira", &["channels"]), who("nell", &["channels"]));
    let first = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    // Another may be called the same, and so may an open one, made or made sure of.
    let second = channels.create_kept("Pay review", None, "", &nell).await.unwrap();
    assert!(first.id != second.id && first.title == second.title);
    let open = channels.create("Pay review", None, "", &nell).await.unwrap();
    assert_eq!((open.name.as_str(), open.kept, open.title.as_deref()), ("pay-review", false, None));
    assert_eq!(channels.ensure("Pay review", None, "", &nell).await.unwrap().id, open.id);
    // Making one says the same whether or not a kept channel is called that.
    let taken = channels.create("pay review", None, "", &mira).await.unwrap_err();
    assert!(taken.message.contains("already a channel called `pay-review`"), "{taken}");
    // It is found by its id and by nothing it is called.
    for by in [&mira, &nell] {
        assert_eq!(channels.may_read("pay-review", by).await.unwrap().channel(), Some(open.id.as_str()));
        assert_eq!(channels.may_read("Pay review", by).await.unwrap_err().kind, ErrorKind::NotFound);
    }
    assert_eq!(channels.may_read(&first.id, &mira).await.unwrap().channel(), Some(first.id.as_str()));
    // Called nothing, or more than a line, it is not made.
    for title in ["   ", &"x".repeat(201)] {
        assert_eq!(channels.create_kept(title, None, "", &mira).await.unwrap_err().kind, ErrorKind::Invalid);
    }
    // An open channel is written as it always was: it says nothing of being kept.
    let written = serde_json::to_value(&open).unwrap();
    assert!(written.get("kept").is_none() && written.get("title").is_none(), "{written}");
    let written = serde_json::to_value(&first).unwrap();
    assert_eq!(
        (&written["kept"], &written["title"]),
        (&serde_json::json!(true), &serde_json::json!("Pay review"))
    );
}

/// The events said of `channel`, oldest first.
pub(crate) async fn of(rig: &Rig, channel: &str) -> Vec<Event> {
    let all = rig.events("channel.*").await;
    all.into_iter().filter(|event| event.data["channel"] == channel).collect()
}

#[tokio::test]
async fn what_is_said_of_a_kept_channel_is_told_to_its_members_as_they_are_when_it_is_handed_over() {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let (mira, nell) = (who("mira", &["channels"]), who("nell", &["channels"]));
    let admin = who("nils", &["channels", "admin", "blueprints"]);
    let owner = who("owen", &["channels", "private"]);
    let key = |scopes: &[&str]| key_of("mira", scopes);
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let open = channels.create("general", None, "", &mira).await.unwrap();
    let writes = channels.may_write(&kept.id, &mira).await.unwrap();
    channels.post(&kept.id, &mira, "quince", None).await.unwrap();
    channels.post_speaker(&writes, &kept.id, "noted", talk::Place::default()).await.unwrap();
    channels.post(&open.id, &mira, "hello", None).await.unwrap();

    // Every event of it takes being a member, and names only who it is also for.
    let events = of(&rig, &kept.id).await;
    let topics: Vec<&str> = events.iter().map(|event| event.topic.as_str()).collect();
    assert_eq!(
        topics,
        ["channel.joined", "channel.created", "channel.message", "channel.joined", "channel.message"]
    );
    let member = format!("member:{}", kept.id);
    let takes: Vec<&str> = events.iter().map(|event| event.source.takes.as_deref().unwrap()).collect();
    let (to_mira, to_the_bot) = (format!("{member};user:mira"), format!("{member};bot:conductor"));
    assert_eq!(takes, [&to_mira, &to_mira, &member, &to_the_bot, &member]);
    // An open channel's take nothing, as they always did.
    assert!(of(&rig, &open.id).await.iter().all(|event| event.source.takes.is_none()));

    let told = |who: &Principal, event: &Event| journal.may_tell(who, event);
    let message = &events[2];
    for (who, tells) in [
        (&mira, true),
        (&nell, false),
        (&admin, false),
        (&owner, true),
        (&key(&["channels:read"]), true),
        (&key(&["board", "blueprints"]), false),
        (&who("mira", &["channels:write"]), false),
    ] {
        assert_eq!(told(who, message), tells, "{} holding {:?}", who.name, who.scopes);
        assert_eq!(journal.may_tell_as(&who.actor(), &who.scopes, message), tells);
    }

    // Somebody is added: they are told that they joined, by name, and
    // from then what is said; and what was said before, when it is
    // handed over now.
    channels.join(&writes, &kept.id, "user:nell", "nell").await.unwrap();
    let joined = of(&rig, &kept.id).await.pop().unwrap();
    assert_eq!(joined.source.takes.as_deref(), Some(format!("{member};user:nell").as_str()));
    assert!(told(&nell, &joined) && told(&mira, &joined) && !told(&admin, &joined));
    channels.post(&kept.id, &nell, "medlar", None).await.unwrap();
    let after = of(&rig, &kept.id).await.pop().unwrap();
    assert!(told(&nell, &after) && told(&nell, message));

    // They leave: told once that they left, by name, and nothing after,
    // nor anything that was said while they were in it.
    channels.leave(&channels.may_write(&kept.id, &nell).await.unwrap(), &kept.id, "user:nell").await.unwrap();
    let left = of(&rig, &kept.id).await.pop().unwrap();
    assert_eq!(left.topic, "channel.left");
    assert_eq!(left.source.takes.as_deref(), Some(format!("{member};user:nell").as_str()));
    assert!(told(&nell, &left) && told(&mira, &left) && !told(&admin, &left));
    channels.post(&kept.id, &mira, "sloe", None).await.unwrap();
    let since = of(&rig, &kept.id).await.pop().unwrap();
    assert!(!told(&nell, &since) && !told(&nell, message) && !told(&nell, &after));
    assert!(told(&nell, &joined), "what names them is theirs whenever it is handed over: that they joined");
    assert!(channels.may_read(&kept.id, &nell).await.is_err());
    // Named in what an event takes, somebody who lacks the scope is told nothing all the same.
    assert!(!journal.may_tell(&who("nell", &["board"]), &left));

    // Taken away: whoever was in it is told, by name, and nobody else;
    // and nothing said of it before is told to them by that.
    let gone = who("mira", &["channels", "admin"]);
    channels.remove(&kept.id, &gone).await.unwrap();
    let removed = of(&rig, &kept.id).await.pop().unwrap();
    assert_eq!(removed.topic, "channel.removed");
    assert_eq!(
        removed.source.takes.as_deref(),
        Some(format!("{member};user:mira;bot:conductor").as_str()),
        "everybody who was in it as it went"
    );
    assert!(
        told(&mira, &removed) && told(&owner, &removed) && !told(&nell, &removed) && !told(&admin, &removed)
    );
    assert!(!told(&mira, &since), "nobody is a member of what is gone");
    assert!(told(&owner, &since), "whoever reads everything still does");
}

#[tokio::test]
async fn who_is_in_a_kept_channel_is_remembered_from_the_tables_and_by_every_opening_of_one_journal() {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let (mira, nell, otto) =
        (who("mira", &["channels"]), who("nell", &["channels"]), who("otto", &["channels"]));
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    channels.post(&kept.id, &mira, "quince", None).await.unwrap();
    let message = of(&rig, &kept.id).await.pop().unwrap();
    assert!(journal.may_tell(&mira, &message) && !journal.may_tell(&nell, &message));

    // The tables are what is so: somebody in them whom nothing here put
    // there is remembered by the channels that are opened next, which
    // answer from the one memory this journal asks.
    let row = kept.id.clone();
    let planted = journal.db().call(move |connection| {
        connection.execute(
            "INSERT INTO channel_members (channel, member, name, joined) VALUES (?1, 'user:nell', 'nell', 0)",
            [&row],
        )
    });
    planted.await.unwrap();
    assert!(!journal.may_tell(&nell, &message), "nothing has read the tables again");
    let files = Attachments::open(journal.clone(), rig.dir.join("attachments")).await.unwrap();
    let again = Channels::open(journal.clone(), files).await.unwrap();
    assert!(journal.may_tell(&nell, &message) && journal.may_tell(&mira, &message));

    // Whichever of the two changes who is in it, the journal answers by it.
    again.leave(&again.may_write(&kept.id, &nell).await.unwrap(), &kept.id, "user:nell").await.unwrap();
    assert!(!journal.may_tell(&nell, &message));
    channels
        .join(&channels.may_write(&kept.id, &mira).await.unwrap(), &kept.id, "user:otto", "otto")
        .await
        .unwrap();
    assert!(journal.may_tell(&otto, &message));
    again.leave(&own(), &kept.id, "user:otto").await.unwrap();
    assert!(!journal.may_tell(&otto, &message) && journal.may_tell(&mira, &message));

    // Somebody who leaves every channel in a write of somebody else's is
    // let go before it, and is back where the write is not made.
    let open = channels.create("general", None, "", &mira).await.unwrap();
    let leaving = channels.leaving("user:mira").await;
    assert!(!journal.may_tell(&mira, &message), "before anything is written");
    channels.stayed(leaving);
    assert!(journal.may_tell(&mira, &message));
    let leaving = channels.leaving("user:mira").await;
    let left = leaving.left();
    let wrote = journal.write(move |writing| Channels::leave_all(writing, &left));
    assert_eq!(wrote.await.unwrap().0, ["general"], "one that is kept is named to nobody");
    drop(leaving);
    assert!(!journal.may_tell(&mira, &message));
    let said: Vec<Event> = rig.events("channel.left").await.into_iter().rev().take(2).collect();
    let took = |channel: &str| {
        let event = said.iter().find(|event| event.data["channel"] == channel).unwrap();
        event.source.takes.clone()
    };
    assert_eq!((took(&open.id), took(&kept.id)), (None, Some(format!("member:{};user:mira", kept.id))));
    assert!(channels.get(&own(), &kept.id).await.unwrap().members.is_empty());
}

#[tokio::test]
async fn an_agent_in_a_kept_channel_is_of_it_only_while_a_turn_asked_there_is_in_hand_as_whoever_keeps_agents_says()
 {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let mira = who("mira", &["channels"]);
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let other = channels.create_kept("Hiring", None, "", &mira).await.unwrap();
    let writes = channels.may_write(&kept.id, &mira).await.unwrap();
    channels.post(&kept.id, &mira, "quince", None).await.unwrap();
    let message = of(&rig, &kept.id).await.pop().unwrap();

    // An agent is not added as a person or a bot is: it is brought in,
    // by whoever keeps agents. In an open channel it is added as anybody.
    let refused = channels.join(&writes, &kept.id, "agent:scribe", "scribe").await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert!(refused.message.contains("comes into a channel kept to its members by being brought in"));
    assert_eq!(channels.kept_with("agent:scribe"), Vec::<String>::new());
    channels.bring(&writes, &kept.id, "scribe").await.unwrap();
    let elsewhere = channels.may_write(&other.id, &mira).await.unwrap();
    channels.bring(&elsewhere, &other.id, "scribe").await.unwrap();
    assert_eq!(channels.kept_with("agent:scribe").len(), 2);
    let open = channels.create("general", None, "", &mira).await.unwrap();
    channels.join(&own(), &open.id, "agent:scribe", "scribe").await.unwrap();

    let scribe = crate::agent("scribe", &["channels"]);
    // What the factory's owner holds reads nothing for an agent.
    let trusted = crate::agent("scribe", &["channels", "private"]);
    let reads = |by: &Principal, channel: &str| {
        let (by, channel) = (by.clone(), channel.to_string());
        async move { channels.may_read(&channel, &by).await }
    };
    let closed = |refused: Error, asked: &str| {
        let nowhere = Error::not_found(format!("there is no channel `{NOWHERE}`"));
        assert_eq!(said(&refused, asked), said(&nowhere, NOWHERE));
    };

    // With nobody to say whether a turn asked there is in hand, it is
    // of neither: in it, and read by its key at no time. Nor is it told
    // that it joined, which names it: that the channel is there is not
    // an agent's to be told while it is asked nothing in it.
    let joined = of(&rig, &kept.id).await.pop().unwrap();
    assert_eq!(joined.source.takes, Some(format!("member:{};agent:scribe", kept.id)));
    for by in [&scribe, &trusted] {
        closed(reads(by, &kept.id).await.unwrap_err(), &kept.id);
        assert!(!journal.may_tell(by, &message) && !journal.may_tell(by, &joined));
        assert!(channels.readable(by).await.unwrap().iter().all(|one| !one.kept));
    }
    assert!(reads(&scribe, &open.id).await.is_ok(), "an open channel is there for it as it always was");

    // Whoever keeps agents says which turns are in hand, and where.
    let in_hand: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>> = Default::default();
    let asked = in_hand.clone();
    channels.asked_with(std::sync::Arc::new(move |channel, actor| {
        asked.lock().unwrap().contains(&(channel.to_string(), actor.to_string()))
    }));
    closed(reads(&scribe, &kept.id).await.unwrap_err(), &kept.id);

    // A turn asked there is in hand: it reads that channel, writes
    // there and is told what is said there, and the other is as closed
    // as it was.
    in_hand.lock().unwrap().push((kept.id.clone(), "agent:scribe".into()));
    assert_eq!(reads(&scribe, &kept.id).await.unwrap().channel(), Some(kept.id.as_str()));
    assert!(channels.post(&kept.id, &scribe, "noted", None).await.is_ok());
    assert!(journal.may_tell(&scribe, &message) && journal.may_tell(&trusted, &message));
    assert!(journal.may_tell(&scribe, &joined));
    assert_eq!(channels.readable(&scribe).await.unwrap().iter().filter(|one| one.kept).count(), 1);
    closed(reads(&scribe, &other.id).await.unwrap_err(), &other.id);
    closed(reads(&trusted, &other.id).await.unwrap_err(), &other.id);
    // Another agent's turn there opens nothing for one that is not in it.
    let spy = crate::agent("spy", &["channels", "private"]);
    in_hand.lock().unwrap().push((kept.id.clone(), "agent:spy".into()));
    closed(reads(&spy, &kept.id).await.unwrap_err(), &kept.id);
    assert!(!journal.may_tell(&spy, &message));
    // A person in it reads it whatever anybody has in hand.
    assert!(reads(&mira, &kept.id).await.is_ok() && journal.may_tell(&mira, &message));

    // The turn has ended: closed again, for what was said meanwhile too.
    in_hand.lock().unwrap().clear();
    closed(reads(&scribe, &kept.id).await.unwrap_err(), &kept.id);
    assert!(!journal.may_tell(&scribe, &message));
    closed(channels.post(&kept.id, &scribe, "more", None).await.unwrap_err(), &kept.id);

    // A conversation named by a message is read by whoever reads the
    // channel it was said in, and is not there for anybody else.
    let said_there = channels.messages(&writes, &kept.id, None, 10).await.unwrap().remove(0);
    let nell = who("nell", &["channels"]);
    assert_eq!(
        channels.may_read_message(&said_there.id, &mira).await.unwrap().channel(),
        Some(kept.id.as_str())
    );
    let refused = channels.may_read_message(&said_there.id, &nell).await.unwrap_err();
    let nothing = channels.may_read_message("msg_nonesuch", &nell).await.unwrap_err();
    assert_eq!(said(&refused, &said_there.id), said(&nothing, "msg_nonesuch"));
    assert_eq!(nothing.kind, ErrorKind::NotFound);
    let lacks = channels.may_read_message(&said_there.id, &who("mira", &["board"])).await.unwrap_err();
    assert_eq!(lacks.kind, ErrorKind::Denied);
}

#[tokio::test]
async fn what_several_messages_said_is_read_at_once_by_leave_for_their_channel_and_for_no_other() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let mira = who("mira", &["channels"]);
    let open = channels.create("general", None, "", &ada()).await.unwrap();
    let kept = channels.create_kept("Pay review", None, "who is paid what", &mira).await.unwrap();
    let mut ids = Vec::new();
    for (channel, body) in [(&open, "the open word"), (&kept, "count the quince"), (&open, "and another")] {
        ids.push(channels.post(&channel.id, &mira, body, None).await.unwrap().id);
    }
    let asked: Vec<String> = ids.iter().cloned().chain(["msg_nonesuch".to_string()]).collect();
    let said = async |reader: &talk::Reader| {
        let mut said: Vec<(String, String)> =
            channels.said_in(reader, &asked).await.unwrap().into_iter().collect();
        said.sort();
        said
    };
    let of = |which: &[usize], words: &[&str]| {
        let mut all: Vec<(String, String)> =
            which.iter().zip(words).map(|(at, word)| (ids[*at].clone(), word.to_string())).collect();
        all.sort();
        all
    };
    // Leave for a channel reads what was said in it, each by its
    // message, and nothing of the other; one that names nothing is not
    // among them.
    let reads_open = channels.may_read(&open.id, &mira).await.unwrap();
    assert_eq!(said(&reads_open).await, of(&[0, 2], &["the open word", "and another"]));
    let reads_kept = channels.may_read(&kept.id, &mira).await.unwrap();
    assert_eq!(said(&reads_kept).await, of(&[1], &["count the quince"]));
    // Somebody else has leave for the open one, which reads nothing of the kept.
    let nell = who("nell", &["channels"]);
    let theirs = channels.may_read(&open.id, &nell).await.unwrap();
    assert!(said(&theirs).await.iter().all(|(_, words)| words != "count the quince"));
    assert!(channels.may_read(&kept.id, &nell).await.is_err());
    // The daemon's own reads them all, and nothing asked is nothing read.
    assert_eq!(said(&own()).await.len(), 3);
    assert!(channels.said_in(&own(), &[]).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_kept_channel_may_say_its_agents_are_asked_nowhere_else_and_an_agent_is_held_so_to_one_channel() {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let mira = who("mira", &["channels", "admin"]);
    let closest = channels.create_kept_alone("Board minutes", None, "", &mira).await.unwrap();
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let other = channels.create_kept("Hiring", None, "", &mira).await.unwrap();
    let open = channels.create("general", None, "", &mira).await.unwrap();
    assert!(closest.alone && closest.kept && !kept.alone && !open.alone);
    // Only one that says so says anything of it, as it is handed out.
    assert_eq!(serde_json::to_value(&closest).unwrap()["alone"], true);
    assert!(serde_json::to_value(&kept).unwrap().get("alone").is_none());
    assert!(serde_json::to_value(&open).unwrap().get("alone").is_none());
    let into = |channel: &talk::Channel| {
        let (id, mira) = (channel.id.clone(), mira.clone());
        async move { channels.may_write(&id, &mira).await.unwrap() }
    };
    let (to_closest, to_kept, to_other) = (into(&closest).await, into(&kept).await, into(&other).await);

    // Kept channels that hold nobody: an agent is in as many as it is
    // brought into, and is asked anywhere.
    channels.bring(&to_kept, &kept.id, "scribe").await.unwrap();
    channels.bring(&to_other, &other.id, "scribe").await.unwrap();
    assert_eq!(channels.alone_in("agent:scribe"), None);
    // It is not brought into one that would hold it, being in others.
    let refused = channels.bring(&to_closest, &closest.id, "scribe").await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("`scribe` would be asked nowhere but here"), "{}", refused.message);
    assert_eq!(channels.may_bring(&to_closest, "scribe").unwrap_err().message, refused.message);
    for name in [kept.id.as_str(), other.id.as_str(), "Pay review", "Hiring"] {
        assert!(!refused.message.contains(name), "the others are named to nobody: {}", refused.message);
    }
    assert_eq!(channels.kept_with("agent:scribe").len(), 2);

    // One that is in the channel that says so is held to it: it is
    // brought into no other kept one, and into an open one as anybody.
    channels.bring(&to_closest, &closest.id, "clerk").await.unwrap();
    assert_eq!(channels.alone_in("agent:clerk"), Some(closest.id.clone()));
    let refused = channels.bring(&to_kept, &kept.id, "clerk").await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(
        refused.message.contains("`clerk` is in a channel it is asked nowhere but in"),
        "{}",
        refused.message
    );
    assert!(!refused.message.contains(&closest.id) && !refused.message.contains("Board minutes"));
    assert!(channels.may_bring(&to_kept, "clerk").is_err());
    channels.bring(&into(&open).await, &open.id, "clerk").await.unwrap();
    assert_eq!(channels.kept_with("agent:clerk"), std::slice::from_ref(&closest.id));
    // Brought in again where it is, nothing changes and nothing is refused.
    channels.bring(&to_closest, &closest.id, "clerk").await.unwrap();

    // Channels opened again on the same journal remember which hold.
    let files = Attachments::open(journal.clone(), rig.dir.join("attachments")).await.unwrap();
    let again = Channels::open(journal.clone(), files).await.unwrap();
    assert_eq!(again.alone_in("agent:clerk"), Some(closest.id.clone()));
    assert_eq!(again.alone_in("agent:scribe"), None);
    assert!(again.get(&own(), &closest.id).await.unwrap().alone);

    // Having left it, it is held no longer; nor by one that is taken away.
    channels.leave(&to_closest, &closest.id, "agent:clerk").await.unwrap();
    assert_eq!(channels.alone_in("agent:clerk"), None);
    channels.bring(&to_kept, &kept.id, "clerk").await.unwrap();
    channels.leave(&to_kept, &kept.id, "agent:clerk").await.unwrap();
    channels.bring(&to_closest, &closest.id, "clerk").await.unwrap();
    channels.remove(&closest.id, &mira).await.unwrap();
    assert_eq!(channels.alone_in("agent:clerk"), None);
    assert!(channels.may_bring(&to_kept, "clerk").is_ok());
}

/// The lines of this crate's source that begin a public function, each
/// with the whole of how it is declared, up to its body.
fn offered() -> Vec<(String, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&src).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(entry.path()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            let code = line.trim_start();
            if !(code.starts_with("pub fn ") || code.starts_with("pub async fn ")) {
                continue;
            }
            let mut declared = String::new();
            for more in &lines[at..] {
                declared.push_str(more.trim());
                declared.push(' ');
                if more.trim_end().ends_with('{') || more.trim_end().ends_with(';') {
                    break;
                }
            }
            found.push((name.clone(), declared));
        }
    }
    assert!(found.len() > 30, "found only {} public functions: is this still where they are?", found.len());
    found
}

/// What hands out something of a channel and takes neither leave to read
/// it nor whoever asks, and why nothing is read by it unasked: the file,
/// words of how it is declared, and the reason.
const UNASKED: &[(&str, &str, &str)] = &[(
    "channels.rs",
    "pub fn new(actor: impl Into<String>, name: impl Into<String>) -> Member",
    "makes a member of what it is handed, for whoever is about to put somebody in a channel: it reads nothing",
)];

#[test]
fn nothing_of_a_channel_is_handed_out_by_a_public_method_that_takes_no_leave_to_read_it() {
    // What is of a channel: the channel with who is in it, a message,
    // something attached, a member.
    let of_a_channel = ["Channel", "Message", "Attachment", "Member"];
    let mut wrong = Vec::new();
    let mut listed = vec![0; UNASKED.len()];
    for (file, declared) in offered() {
        let Some((takes, gives)) = declared.split_once("->") else { continue };
        let hands = of_a_channel.iter().any(|one| {
            // The word itself, not one that only begins with it (`Channels`, `Attachments`).
            gives
                .match_indices(one)
                .any(|(at, _)| !gives[at + one.len()..].starts_with(|next: char| next.is_alphanumeric()))
        });
        if !hands || takes.contains("&Reader") || takes.contains("&Principal") {
            continue;
        }
        match UNASKED.iter().position(|(at, words, _)| *at == file && declared.contains(words)) {
            Some(at) => listed[at] += 1,
            None => wrong.push(format!("{file}: {declared}")),
        }
    }
    for ((file, words, _), found) in UNASKED.iter().zip(&listed) {
        if *found != 1 {
            wrong.push(format!("`{words}` in {file} is listed and is there {found} times"));
        }
    }
    assert!(
        wrong.is_empty(),
        "these public methods of `talk` hand out a channel, a message, a member or something attached and take \
         neither leave to read it (`&Reader`, which `Channels::may_read`, `may_write`, `may_find` and \
         `Channels::daemon` make) nor whoever asks (`&Principal`, to ask one of those themselves). A channel may \
         be kept to its members, and what reads one without asking reads it for anybody: take a `&Reader` and \
         hold what is read to it (`Reader::holds`), or be `pub(crate)` for a method that asks. What hands \
         nothing of a channel out is listed in `UNASKED` with the reason.\n{}",
        wrong.join("\n")
    );
}

#[tokio::test]
async fn a_message_of_a_kept_channel_carries_what_its_reader_is_shown_and_an_open_one_is_as_it_was() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let kept = channels.create_kept("Pay review", Some("shop"), "", &ada()).await.unwrap();
    let said = channels.post(&kept.id, &ada(), "quince", None).await.unwrap();
    let event = of(&rig, &kept.id).await.pop().unwrap();
    // Its name, where one is said, is its id: what it is called is read from the channel.
    assert_eq!(
        (&event.data["channel_name"], &event.data["id"]),
        (&serde_json::json!(kept.id), &serde_json::json!(said.id))
    );
    assert_eq!(event.data["project"], "shop");
    assert!(!event.data.to_string().contains("Pay review"));
}

#[tokio::test]
async fn a_channel_and_who_is_in_it_are_written_with_what_is_said_of_them_or_not_at_all() {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let (mira, nell) = (who("mira", &["channels"]), who("nell", &["channels"]));
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let writes = channels.may_write(&kept.id, &mira).await.unwrap();
    channels.join(&writes, &kept.id, "user:nell", "nell").await.unwrap();
    channels.post(&kept.id, &mira, "quince", None).await.unwrap();
    let message = of(&rig, &kept.id).await.pop().unwrap();
    let members = async || {
        let found = channels.get(&own(), &kept.id).await.unwrap();
        found.members.into_iter().map(|one| one.actor).collect::<Vec<_>>()
    };

    // Nothing of a channel can be said from here on.
    let unsaid = journal.db().call(|connection| {
        connection.execute_batch(
            "CREATE TRIGGER unsaid BEFORE INSERT ON events WHEN NEW.topic LIKE 'channel.%'
             BEGIN SELECT RAISE(ABORT, 'nothing of a channel is said'); END;",
        )
    });
    unsaid.await.unwrap();
    // Somebody who cannot be said to have joined has not: not in the
    // tables, and not told anything.
    assert!(channels.join(&writes, &kept.id, "user:otto", "otto").await.is_err());
    assert_eq!(members().await, ["user:mira", "user:nell"]);
    assert!(!journal.may_tell(&who("otto", &["channels"]), &message));
    // Somebody who cannot be said to have left is still in it, and is
    // told as before: the memory let them go and has them back.
    assert!(channels.leave(&writes, &kept.id, "user:nell").await.is_err());
    assert_eq!(members().await, ["user:mira", "user:nell"]);
    assert!(journal.may_tell(&nell, &message));
    // And a channel that cannot be said to have been made is not there.
    assert!(channels.create_kept("Another", None, "", &nell).await.is_err());
    assert!(channels.create("another", None, "", &nell).await.is_err());
    let all: Vec<String> = channels.list(&own()).await.unwrap().into_iter().map(|one| one.id).collect();
    assert_eq!(all, std::slice::from_ref(&kept.id));
    assert!(channels.post(&kept.id, &mira, "medlar", None).await.is_err());
    assert_eq!(channels.messages(&own(), &kept.id, None, 10).await.unwrap().len(), 1);
}
