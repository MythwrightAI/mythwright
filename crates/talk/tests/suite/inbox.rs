//! The inbox (`talk::inbox`): a box of things addressed to somebody,
//! read by its owner and nobody else, filled through one door that is
//! the same done twice, each line written with what is said of it or
//! not at all; and what the channels put in it (`talk::inboxed`).

use std::sync::Arc;

use kernel::{ErrorKind, Principal};
use serde_json::json;
use talk::inbox::{Kind, Line, Tell};

use crate::kept::who;
use crate::{Rig, agent, own};

/// A key `maker` made, holding `scopes`: it acts as its maker.
fn key_of(maker: &str, scopes: &[&str]) -> Principal {
    Principal { key: Some(format!("cred_{maker}")), ..who(maker, scopes) }
}

/// Has the rig's channels take these people for who a word names: each
/// by the id they have here.
fn named(rig: &Rig, people: &[Principal]) {
    let people = people.to_vec();
    rig.channels.names_with(Arc::new(move |_, names| {
        Ok(people.iter().filter(|one| names.contains(&one.id)).cloned().collect())
    }));
}

/// What was said of anybody's box, as topic and what it carried.
async fn said(rig: &Rig) -> Vec<(String, serde_json::Value)> {
    rig.events("inbox.*").await.into_iter().map(|event| (event.topic, event.data)).collect()
}

fn told(owner: &str, about: &str, said_by: Option<&str>) -> Tell {
    Tell {
        owner: owner.into(),
        kind: Kind::Told,
        about: about.into(),
        place: "ch_1".into(),
        from: "bot:factory".into(),
        why: "told".into(),
        says: Some("the build is green".into()),
        said_by: said_by.map(str::to_string),
        parts: None,
    }
}

fn ids(lines: &[Line]) -> Vec<&str> {
    lines.iter().map(|line| line.id.as_str()).collect()
}

#[tokio::test]
async fn what_is_said_of_a_box_is_heard_by_its_owner_holding_the_scope_and_by_nobody_else() {
    let rig = Rig::new().await;
    let (inbox, journal) = (&rig.channels.inbox, &rig.journal);
    let line = inbox.tell(told("user:mira", "build", None)).await.unwrap().unwrap();
    let event = rig.events("inbox.told").await.pop().unwrap();
    assert_eq!(event.source.takes.as_deref(), Some("member:inbox:user:mira"));
    assert_eq!((&event.data["line"], &event.data["unread"]), (&json!(line.id), &json!(1)));
    // Its owner, and a key of theirs that was given the scope.
    assert!(journal.may_tell(&who("mira", &["inbox"]), &event));
    assert!(journal.may_tell(&key_of("mira", &["inbox:read"]), &event));
    // Not a key of theirs that was not, whatever else it holds; not
    // another member; not whoever holds everything, who is the
    // factory's owner or the factory itself; and not an agent called
    // what the account is.
    assert!(!journal.may_tell(&key_of("mira", &["channels", "admin", "private"]), &event));
    assert!(!journal.may_tell(&who("nell", &["inbox", "channels"]), &event));
    assert!(!journal.may_tell(&who("ada", &["*"]), &event));
    assert!(!journal.may_tell(&Principal::system(), &event));
    assert!(!journal.may_tell(&agent("mira", &["*"]), &event));
    // An agent's box is the agent's, the same way.
    inbox.tell(told("agent:scout", "build", None)).await.unwrap().unwrap();
    let event = rig.events("inbox.told").await.pop().unwrap();
    assert!(journal.may_tell(&agent("scout", &["inbox:read"]), &event));
    assert!(!journal.may_tell(&agent("scout", &["channels"]), &event));
    assert!(!journal.may_tell(&who("scout", &["*"]), &event));
    // No words are in what is said, and nobody that can own no box has one.
    assert!(!event.data.to_string().contains("green"));
    for nobody in ["bot:factory", "system", "user", "device:laptop"] {
        let refused = inbox.tell(told(nobody, "build", None)).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{nobody}");
    }
    assert_eq!(said(&rig).await.len(), 2);
}

#[tokio::test]
async fn a_telling_done_twice_is_one_line_told_once_and_another_telling_raises_the_line_there_is() {
    let rig = Rig::new().await;
    let inbox = &rig.channels.inbox;
    let mira = who("mira", &["inbox"]);
    let first = inbox.tell(told("user:mira", "build", Some("run_1/notify"))).await.unwrap().unwrap();
    assert_eq!((first.count, first.read, first.away), (1, None, None));
    // The same step saying it again says nothing: one line, one event.
    assert_eq!(inbox.tell(told("user:mira", "build", Some("run_1/notify"))).await.unwrap(), None);
    assert_eq!(inbox.list(&mira, true).await.unwrap(), std::slice::from_ref(&first));
    assert_eq!(said(&rig).await.len(), 1);
    // Read and put away, it waits no longer and is listed only with everything.
    inbox.read(&mira, &first.id).await.unwrap();
    inbox.away(&mira, &first.id).await.unwrap();
    assert_eq!(inbox.unread(&mira).await.unwrap(), 0);
    assert!(inbox.list(&mira, false).await.unwrap().is_empty());
    // Another telling of the same thing raises that line: unread again,
    // back in the box, counted, with what the later one said, and no
    // second row.
    let mut again = told("user:mira", "build", Some("run_2/notify"));
    again.says = Some("the build is red".into());
    let raised = inbox.tell(again).await.unwrap().unwrap();
    assert_eq!(
        (raised.id.as_str(), raised.count, raised.read, raised.away),
        (first.id.as_str(), 2, None, None)
    );
    assert_eq!(raised.says.as_deref(), Some("the build is red"));
    assert_eq!(inbox.list(&mira, true).await.unwrap(), [raised]);
    assert_eq!(inbox.unread(&mira).await.unwrap(), 1);
    let last = said(&rig).await.pop().unwrap();
    assert_eq!(last.0, "inbox.told");
    assert_eq!((&last.1["count"], &last.1["unread"]), (&json!(2), &json!(1)));
    // The same thing is another line for somebody else, and for another kind.
    inbox.tell(told("user:nell", "build", Some("run_2/notify"))).await.unwrap().unwrap();
    let named = Tell::named("user:mira", "build", "ch_1", "user:nell");
    let line = inbox.tell(named.clone()).await.unwrap().unwrap();
    assert_ne!((line.id.as_str(), line.kind), (first.id.as_str(), Kind::Told));
    // Somebody is named in a message once: told again, nothing is.
    assert_eq!(inbox.tell(named).await.unwrap(), None);
}

#[tokio::test]
async fn a_box_is_read_and_changed_by_its_owner_and_a_line_of_anothers_is_as_one_that_is_not_there() {
    let rig = Rig::new().await;
    let inbox = &rig.channels.inbox;
    let (mira, nell) = (who("mira", &["inbox"]), who("nell", &["inbox"]));
    let mut lines = Vec::new();
    for (about, place) in [("a", "ch_1"), ("b", "ch_1"), ("c", "ch_2"), ("d", "ch_2")] {
        let tell = Tell { place: place.into(), ..told("user:mira", about, None) };
        lines.push(inbox.tell(tell).await.unwrap().unwrap());
    }
    let hers = inbox.tell(told("user:nell", "a", None)).await.unwrap().unwrap();
    // The latest told is first; each reads their own and nothing else.
    assert_eq!(
        ids(&inbox.list(&mira, false).await.unwrap()),
        [&lines[3].id, &lines[2].id, &lines[1].id, &lines[0].id]
    );
    assert_eq!(inbox.list(&nell, false).await.unwrap(), std::slice::from_ref(&hers));
    assert_eq!((inbox.unread(&mira).await.unwrap(), inbox.unread(&nell).await.unwrap()), (4, 1));

    // A line of somebody else's is refused word for word as one that
    // was never there is, whoever asks and whatever they hold.
    let ada = who("ada", &["*"]);
    for by in [&nell, &ada] {
        let (read, away) = (inbox.read(by, &lines[0].id).await, inbox.away(by, &lines[0].id).await);
        let never = inbox.read(by, "inb_never").await.unwrap_err();
        assert_eq!(never.kind, ErrorKind::NotFound);
        for refused in [read.unwrap_err(), away.unwrap_err()] {
            assert_eq!(refused.kind, ErrorKind::NotFound);
            assert_eq!(refused.message.replace(&lines[0].id, "inb_never"), never.message);
        }
        assert!(inbox.list(by, true).await.unwrap().iter().all(|line| line.owner == by.actor()));
    }
    assert_eq!(inbox.read_in(&ada, None).await.unwrap(), 0);
    assert_eq!(inbox.unread(&mira).await.unwrap(), 4);

    // Reading takes its scope, and changing its own: a key that reads
    // marks nothing, one given neither reads nothing.
    let reads = key_of("mira", &["inbox:read"]);
    assert_eq!(inbox.list(&reads, false).await.unwrap().len(), 4);
    assert_eq!(inbox.read(&reads, &lines[0].id).await.unwrap_err().kind, ErrorKind::Denied);
    assert_eq!(inbox.away(&reads, &lines[0].id).await.unwrap_err().kind, ErrorKind::Denied);
    assert_eq!(inbox.read_in(&reads, None).await.unwrap_err().kind, ErrorKind::Denied);
    let neither = key_of("mira", &["channels", "admin"]);
    assert_eq!(inbox.list(&neither, false).await.unwrap_err().kind, ErrorKind::Denied);
    assert_eq!(inbox.unread(&neither).await.unwrap_err().kind, ErrorKind::Denied);

    // One read: said once, with what waits, and the same asked again.
    let before = said(&rig).await.len();
    let read = inbox.read(&mira, &lines[0].id).await.unwrap();
    assert!(read.read.is_some());
    assert_eq!(inbox.read(&mira, &lines[0].id).await.unwrap(), read);
    let events = said(&rig).await;
    assert_eq!(events.len(), before + 1);
    let expected = json!({ "owner": "user:mira", "lines": [lines[0].id], "place": "ch_1", "unread": 3 });
    assert_eq!(events[before], ("inbox.read".to_string(), expected));
    // Every line of one place: only what was unread there is said.
    assert_eq!(inbox.read_in(&mira, Some("ch_1")).await.unwrap(), 1);
    assert_eq!(inbox.read_in(&mira, Some("ch_1")).await.unwrap(), 0);
    let events = said(&rig).await;
    let expected = json!({ "owner": "user:mira", "lines": [lines[1].id], "place": "ch_1", "unread": 2 });
    assert_eq!((events.len(), &events[before + 1].1), (before + 2, &expected));
    // One put away: listed no longer, and waiting no longer.
    let away = inbox.away(&mira, &lines[2].id).await.unwrap();
    assert_eq!(inbox.away(&mira, &lines[2].id).await.unwrap(), away);
    assert_eq!(ids(&inbox.list(&mira, false).await.unwrap()), [&lines[3].id, &lines[1].id, &lines[0].id]);
    assert_eq!(inbox.list(&mira, true).await.unwrap().len(), 4);
    let events = said(&rig).await;
    let expected = json!({ "owner": "user:mira", "lines": [lines[2].id], "place": "ch_2", "unread": 1 });
    assert_eq!(events[before + 2], ("inbox.away".to_string(), expected));
    // And every line there is, the one put away with them.
    assert_eq!(inbox.read_in(&mira, None).await.unwrap(), 2);
    assert_eq!(inbox.unread(&mira).await.unwrap(), 0);
    let last = said(&rig).await.pop().unwrap().1;
    assert_eq!((last.get("place"), &last["unread"]), (None, &json!(0)));
    // Nothing of hers was touched by any of it.
    assert_eq!(inbox.list(&nell, false).await.unwrap(), [hers]);
}

#[tokio::test]
async fn a_message_tells_the_people_it_names_who_read_its_channel_in_the_write_that_keeps_it() {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let (mira, nell, otto) =
        (who("mira", &["channels", "inbox"]), who("nell", &["channels", "inbox"]), who("otto", &["inbox"]));
    let ada = who("ada", &["channels", "inbox", "private"]);
    named(&rig, &[mira.clone(), nell.clone(), otto.clone(), ada.clone()]);
    let open = channels.create("general", None, "", &mira).await.unwrap();
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();

    // A message that names nobody is what it always was: one write that
    // says one thing.
    channels.post(&open.id, &mira, "nobody is named here, not even user@example.com", None).await.unwrap();
    assert!(said(&rig).await.is_empty());
    assert_eq!(rig.topics("channel.message").await.len(), 1);
    let before = rig.events("**").await.len();
    channels.post(&open.id, &mira, "nor does @nobody name anybody", None).await.unwrap();
    assert_eq!(rig.events("**").await.len(), before + 1);

    // In an open channel: whoever is named and reads channels, in it or
    // not, and never whoever said it.
    let message = channels.post(&open.id, &mira, "@nell @otto @mira look, @nell", None).await.unwrap();
    let hers = channels.inbox.list(&nell, false).await.unwrap();
    assert_eq!(hers.len(), 1);
    let line = &hers[0];
    assert_eq!(
        (line.kind, line.about.as_str(), line.place.as_str()),
        (Kind::Named, message.id.as_str(), open.id.as_str())
    );
    assert_eq!(
        (line.from.as_str(), line.why.as_str(), &line.says, line.created),
        ("user:mira", "named", &None, message.created)
    );
    assert!(channels.inbox.list(&mira, true).await.unwrap().is_empty());
    assert!(channels.inbox.list(&otto, true).await.unwrap().is_empty(), "otto does not read channels");
    let events = said(&rig).await;
    assert_eq!(events.len(), 1);
    let expected = json!({
        "owner": "user:nell", "line": line.id, "kind": "named", "place": open.id, "about": message.id,
        "count": 1, "unread": 1,
    });
    assert_eq!(events[0], ("inbox.told".to_string(), expected));
    let event = rig.events("inbox.told").await.pop().unwrap();
    assert_eq!(event.source.actor.as_deref(), Some("user:mira"));
    assert!(
        journal.may_tell(&nell, &event)
            && !journal.may_tell(&mira, &event)
            && !journal.may_tell(&ada, &event)
    );

    // In one kept to its members: whoever reads it, by the one question
    // every reading of it asks. Somebody in it, and for now whoever
    // holds what reads every kept channel; not somebody who is neither.
    channels.post(&kept.id, &mira, "@nell @ada greengage", None).await.unwrap();
    assert_eq!(channels.inbox.list(&nell, false).await.unwrap().len(), 1, "nell is not in it");
    assert_eq!(
        channels.inbox.list(&ada, false).await.unwrap().len(),
        1,
        "who reads every kept channel reads this"
    );
    channels.join(&own(), &kept.id, "user:nell", "nell").await.unwrap();
    let within = channels.post(&kept.id, &mira, "@nell damson", None).await.unwrap();
    let hers = channels.inbox.list(&nell, false).await.unwrap();
    assert_eq!(
        (hers.len(), hers[0].about.as_str(), hers[0].place.as_str()),
        (2, within.id.as_str(), kept.id.as_str())
    );
    // What the bot says names somebody as what anybody says does.
    channels
        .post_speaker(&own(), &open.id, "@nell the build is green", talk::Place::default())
        .await
        .unwrap();
    assert_eq!(channels.inbox.unread(&nell).await.unwrap(), 3);
    // No word of anything said is in what is said of a box.
    for (_, data) in said(&rig).await {
        let data = data.to_string();
        assert!(
            !data.contains("greengage") && !data.contains("damson") && !data.contains("Pay review"),
            "{data}"
        );
    }
}

#[tokio::test]
async fn a_message_and_the_lines_it_raises_are_written_with_what_is_said_of_them_or_not_at_all() {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let (mira, nell) = (who("mira", &["channels", "inbox"]), who("nell", &["channels", "inbox"]));
    named(&rig, &[mira.clone(), nell.clone()]);
    let open = channels.create("general", None, "", &mira).await.unwrap();
    channels.post(&open.id, &mira, "@nell quince", None).await.unwrap();
    let before = rig.events("**").await.len();
    let unsaid = journal.db().call(|connection| {
        connection.execute_batch(
            "CREATE TRIGGER unsaid BEFORE INSERT ON events WHEN NEW.topic LIKE 'inbox.%'
             BEGIN SELECT RAISE(ABORT, 'nothing of a box is said'); END;",
        )
    });
    unsaid.await.unwrap();
    // A message whose line cannot be said is not there, and neither is
    // the line, nor anything said of either.
    assert!(channels.post(&open.id, &mira, "@nell medlar", None).await.is_err());
    assert_eq!(channels.messages(&own(), &open.id, None, 10).await.unwrap().len(), 1);
    assert_eq!(channels.inbox.list(&nell, true).await.unwrap().len(), 1);
    assert_eq!(rig.events("**").await.len(), before);
    // One that names nobody is said as ever, and so is nothing of a box.
    channels.post(&open.id, &mira, "sloe", None).await.unwrap();
    assert_eq!(rig.events("**").await.len(), before + 1);
    // Nor is a line read, put away or told where that cannot be said.
    let line = channels.inbox.list(&nell, false).await.unwrap().remove(0);
    assert!(channels.inbox.read(&nell, &line.id).await.is_err());
    assert!(channels.inbox.read_in(&nell, None).await.is_err());
    assert!(channels.inbox.away(&nell, &line.id).await.is_err());
    assert!(channels.inbox.tell(told("user:nell", "build", None)).await.is_err());
    assert_eq!(channels.inbox.list(&nell, true).await.unwrap(), [line]);
    // And a channel whose lines cannot be said to have gone is still
    // there, with everybody in it and everything said in it.
    assert!(channels.remove(&open.id, &who("ada", &["*"])).await.is_err());
    assert_eq!(channels.messages(&own(), &open.id, None, 10).await.unwrap().len(), 2);
    assert_eq!(channels.inbox.unread(&nell).await.unwrap(), 1);
}

#[tokio::test]
async fn a_line_goes_with_the_channel_it_opens_and_a_box_with_whoever_is_taken_away_for_good() {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let (mira, nell, otto) = (
        who("mira", &["channels", "inbox"]),
        who("nell", &["channels", "inbox"]),
        who("otto", &["channels", "inbox"]),
    );
    named(&rig, &[mira.clone(), nell.clone(), otto.clone()]);
    let (going, staying) = (
        channels.create("going", None, "", &mira).await.unwrap(),
        channels.create("staying", None, "", &mira).await.unwrap(),
    );
    channels.post(&going.id, &mira, "@nell @otto one", None).await.unwrap();
    channels.post(&going.id, &mira, "@nell two", None).await.unwrap();
    channels.post(&staying.id, &mira, "@nell three", None).await.unwrap();
    let scout = agent("scout", &["inbox"]);
    let told_in = |channel: &str| Tell { place: channel.into(), ..told("agent:scout", channel, None) };
    channels.inbox.tell(told_in(&going.id)).await.unwrap().unwrap();
    channels.inbox.tell(told_in(&staying.id)).await.unwrap().unwrap();
    let before = said(&rig).await.len();
    let gone: Vec<String> = channels
        .inbox
        .list(&nell, false)
        .await
        .unwrap()
        .into_iter()
        .filter(|l| l.place == going.id)
        .map(|l| l.id)
        .collect();

    // The channel taken away takes every line that opens it, each owner
    // told which of theirs went and what now waits.
    channels.remove(&going.id, &who("ada", &["*"])).await.unwrap();
    let hers = channels.inbox.list(&nell, true).await.unwrap();
    assert_eq!((hers.len(), hers[0].place.as_str()), (1, staying.id.as_str()));
    assert!(channels.inbox.list(&otto, true).await.unwrap().is_empty());
    assert_eq!(channels.inbox.list(&scout, true).await.unwrap().len(), 1);
    let events = said(&rig).await.split_off(before);
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|(topic, data)| topic == "inbox.away" && data["place"] == json!(going.id)));
    let of_nell = events.iter().find(|(_, data)| data["owner"] == "user:nell").unwrap();
    let mut went: Vec<String> = serde_json::from_value(of_nell.1["lines"].clone()).unwrap();
    went.sort();
    let mut expected = gone;
    expected.sort();
    assert_eq!((went, &of_nell.1["unread"]), (expected, &json!(1)));
    let heard = rig.events("inbox.away").await;
    let hers = heard.iter().find(|event| event.data["owner"] == "user:nell").unwrap();
    assert!(
        journal.may_tell(&nell, hers) && !journal.may_tell(&otto, hers) && !journal.may_tell(&mira, hers)
    );

    // Whoever is taken away for good takes their box, in the write that
    // takes them: nobody later called the same finds it.
    let wrote = journal.write(|writing| talk::Inbox::forget_owner(writing, "agent:scout"));
    assert_eq!(wrote.await.unwrap().0, 1);
    assert!(channels.inbox.list(&scout, true).await.unwrap().is_empty());
    assert_eq!(channels.inbox.unread(&nell).await.unwrap(), 1);
}

#[tokio::test]
async fn a_line_is_shown_with_the_words_its_owner_may_read_now_and_with_none_where_they_may_not() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let (mira, nell) = (who("mira", &["channels", "inbox"]), who("nell", &["channels", "inbox"]));
    named(&rig, &[mira.clone(), nell.clone()]);
    let open = channels.create("general", Some("shop"), "", &mira).await.unwrap();
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    channels.join(&own(), &kept.id, "user:nell", "nell").await.unwrap();
    let root = channels.post(&open.id, &mira, "the plan", None).await.unwrap();
    let place = talk::Place { thread: Some(&root.id), ..talk::Place::default() };
    let under = channels.post_at(&open.id, &mira, "@nell quince", place).await.unwrap();
    let within = channels.post(&kept.id, &mira, "@nell greengage", None).await.unwrap();
    channels.inbox.tell(told("user:nell", "build", None)).await.unwrap().unwrap();

    // The latest first, each of something said with what was said,
    // where and by whom; one of words of its own with those.
    let (shown, unread) = channels.inbox_of(&nell, false).await.unwrap();
    assert_eq!((shown.len(), unread), (3, 3));
    assert_eq!(
        (shown[0].line.kind, shown[0].line.says.as_deref(), &shown[0].said),
        (Kind::Told, Some("the build is green"), &None)
    );
    let of_kept = shown[1].said.as_ref().unwrap();
    assert_eq!(
        (of_kept.body.as_str(), of_kept.channel_title.as_deref(), of_kept.channel_kept),
        ("@nell greengage", Some("Pay review"), true)
    );
    assert_eq!(of_kept.channel_name, kept.id);
    let of_open = shown[2].said.as_ref().unwrap();
    assert_eq!(
        (of_open.body.as_str(), of_open.thread.as_deref(), of_open.channel_name.as_str()),
        ("@nell quince", Some(root.id.as_str()), "general")
    );
    assert_eq!(
        (of_open.author.as_str(), of_open.author_name.as_str(), of_open.project.as_deref()),
        ("user:mira", "mira", Some("shop"))
    );
    assert_eq!(shown[2].line.about.as_str(), under.id.as_str());

    // Out of the kept channel, the line she had of it is passed over:
    // to her the channel is one that is not there, and nothing of her
    // box says it is. Not listed, not counted, not found by its id,
    // not read with everything, and nothing said of any of that.
    let hers = shown[1].line.id.clone();
    channels.leave(&own(), &kept.id, "user:nell").await.unwrap();
    let before = said(&rig).await.len();
    let (shown, unread) = channels.inbox_of(&nell, true).await.unwrap();
    assert_eq!((shown.len(), unread), (2, 2));
    assert!(channels.inbox.list(&nell, true).await.unwrap().iter().all(|line| line.id != hers));
    let handed = json!(shown).to_string();
    assert!(
        !handed.contains("greengage") && !handed.contains("Pay review") && !handed.contains(&kept.id),
        "{handed}"
    );
    assert!(handed.contains("quince") && !handed.contains(within.id.as_str()));
    let never = channels.inbox.read(&nell, "inb_never").await.unwrap_err();
    for refused in [channels.inbox.read(&nell, &hers).await, channels.inbox.away(&nell, &hers).await] {
        assert_eq!(refused.unwrap_err().message.replace(&hers, "inb_never"), never.message);
    }
    assert_eq!(channels.inbox.read_in(&nell, Some(&kept.id)).await.unwrap(), 0);
    assert_eq!(said(&rig).await.len(), before);
    // What is said of her box as something else is put in it counts
    // what she is shown: the line that is passed over is not among it.
    channels.post(&open.id, &mira, "@nell damson", None).await.unwrap();
    let told = said(&rig).await.pop().unwrap().1;
    assert_eq!((&told["unread"], channels.inbox.unread(&nell).await.unwrap()), (&json!(3), 3));
    // Back in the channel, the line is as it was: listed, and waiting.
    channels.join(&own(), &kept.id, "user:nell", "nell").await.unwrap();
    let (shown, unread) = channels.inbox_of(&nell, false).await.unwrap();
    assert_eq!((shown.len(), unread), (4, 4));
    let back = shown.iter().find(|one| one.line.id == hers).unwrap();
    assert_eq!((back.said.as_ref().unwrap().body.as_str(), back.line.read), ("@nell greengage", None));
    // Read with everything while she is out of it, it is not: it waits
    // when she is back.
    channels.leave(&own(), &kept.id, "user:nell").await.unwrap();
    assert_eq!(channels.inbox.read_in(&nell, None).await.unwrap(), 3);
    assert_eq!(channels.inbox.unread(&nell).await.unwrap(), 0);
    channels.join(&own(), &kept.id, "user:nell", "nell").await.unwrap();
    assert_eq!(channels.inbox.unread(&nell).await.unwrap(), 1);
    // The same of a channel taken away while it is kept from her: she
    // is told nothing of its going, and whoever read it is.
    let other = channels.create_kept("Sloe review", None, "", &mira).await.unwrap();
    channels.join(&own(), &other.id, "user:nell", "nell").await.unwrap();
    channels.post(&other.id, &nell, "@mira bullace", None).await.unwrap();
    channels.post(&other.id, &mira, "@nell mirabelle", None).await.unwrap();
    channels.leave(&own(), &other.id, "user:nell").await.unwrap();
    let before = said(&rig).await.len();
    channels.remove(&other.id, &who("ada", &["*"])).await.unwrap();
    let went = said(&rig).await.split_off(before);
    assert_eq!(went.len(), 1, "{went:?}");
    assert_eq!((&went[0].1["owner"], &went[0].1["place"]), (&json!("user:mira"), &json!(other.id)));
    // Nor does a key of hers that reads no channels read a line of
    // something said by its box, or count one.
    let narrow = key_of("nell", &["inbox"]);
    let (shown, unread) = channels.inbox_of(&narrow, true).await.unwrap();
    assert_eq!((shown.iter().map(|one| one.line.kind).collect::<Vec<_>>(), unread), (vec![Kind::Told], 0));
    assert!(!json!(shown).to_string().contains("quince"));
    // Somebody else's box holds none of it, whatever they hold.
    let (shown, unread) = channels.inbox_of(&who("ada", &["*"]), true).await.unwrap();
    assert_eq!((shown.len(), unread), (0, 0));

    // A message that is gone took what was said with it: its line is
    // passed over. (Nothing takes one message away; a journal from
    // before, or a hand, may have.)
    let gone = under.id.clone();
    let taken = rig
        .journal
        .db()
        .call(move |connection| connection.execute("DELETE FROM messages WHERE id = ?1", [&gone]));
    taken.await.unwrap();
    let (shown, unread) = channels.inbox_of(&nell, false).await.unwrap();
    assert!(shown.iter().all(|one| one.line.about != under.id) && shown.len() == 3, "{shown:?}");
    assert_eq!(unread, 1, "what waits is what would be listed");
}
