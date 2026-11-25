//! The door of a channel kept to its members (`talk::door`): an open
//! one kept and a kept one opened again, who is in one from the first,
//! somebody leaving, whether its agents are asked nowhere else, and how
//! a blueprint's step finds the kept channel it asked for before. Each
//! with what is said of it, to whom, and what the memory that answers
//! for events says on each side of the write.

use kernel::{ErrorKind, Event, Principal};
use talk::{Asked, Attachments, Channels, Member};

use crate::kept::{of, who};
use crate::{Rig, own};

/// Leave for `by` to write in `channel`.
async fn writes(channels: &Channels, channel: &str, by: &Principal) -> talk::Reader {
    channels.may_write(channel, by).await.unwrap()
}

#[tokio::test]
async fn an_open_channel_is_kept_to_whoever_is_in_it_and_a_kept_one_is_opened_again_under_a_name_channels_share()
 {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let (mira, nell, otto) =
        (who("mira", &["channels"]), who("nell", &["channels"]), who("otto", &["channels"]));
    let owner = who("owen", &["channels", "private", "admin"]);
    let told = |who: &Principal, event: &Event| journal.may_tell(who, event);
    let open = channels.create("general", Some("shop"), "everything", &mira).await.unwrap();
    channels.join(&writes(channels, "general", &mira).await, "general", "user:nell", "nell").await.unwrap();
    channels.post("general", &nell, "quince", None).await.unwrap();
    let while_open = of(&rig, &open.id).await.pop().unwrap();
    assert!(told(&otto, &while_open) && channels.may_read("general", &otto).await.is_ok());

    // Kept, by somebody who writes in it: it is called what its name
    // was, its name is its own id, and whoever was in it is in it.
    let kept = channels.keep(&writes(channels, "general", &nell).await, "general", &nell).await.unwrap();
    assert_eq!(
        (kept.kept, kept.title.as_deref(), &kept.name, kept.alone),
        (true, Some("general"), &open.id, false)
    );
    assert_eq!(kept.members, open_members(channels, &open.id).await);
    assert_eq!(channels.get(&own(), &open.id).await.unwrap(), kept, "as the tables have it");
    // That it was kept is said to whoever may read channels, once: they
    // read it until then, and stop showing it.
    let said = of(&rig, &open.id).await.pop().unwrap();
    assert_eq!((said.topic.as_str(), said.source.takes.as_deref()), ("channel.kept", None));
    assert_eq!(said.source.actor.as_deref(), Some("user:nell"));
    let about = serde_json::json!({
        "channel": open.id, "name": open.id, "kept": true, "alone": false, "title": "general",
        "was": "general", "project": "shop",
    });
    assert_eq!(said.data, about);
    assert!(told(&otto, &said) && told(&mira, &said));

    // It is its members' from there: read by them and by nobody else,
    // and under the name it had by nobody at all.
    for member in [&mira, &nell, &owner] {
        assert!(channels.may_read(&open.id, member).await.is_ok(), "{}", member.name);
    }
    let refused = channels.may_read(&open.id, &otto).await.unwrap_err();
    let nowhere = channels.may_read("ch_nonesuch", &otto).await.unwrap_err();
    assert_eq!(refused.message.replace(&open.id, "<it>"), nowhere.message.replace("ch_nonesuch", "<it>"));
    assert_eq!(channels.may_read("general", &mira).await.unwrap_err().kind, ErrorKind::NotFound);
    channels.post(&open.id, &mira, "medlar", None).await.unwrap();
    let after = of(&rig, &open.id).await.pop().unwrap();
    assert_eq!(after.source.takes.as_deref(), Some(format!("member:{}", open.id).as_str()));
    assert!(told(&nell, &after) && told(&owner, &after) && !told(&otto, &after));
    // What was said of it while it was open was said to everybody, and
    // is as it was said wherever it is handed over again: not held.
    assert!(told(&otto, &while_open));
    // The name is free: somebody who makes a channel is not told that
    // one of that name is there.
    let another = channels.create("general", None, "", &otto).await.unwrap();
    assert_ne!(another.id, open.id);
    // Kept again, nothing changes and nothing more is said.
    let again = channels.keep(&writes(channels, &open.id, &mira).await, &open.id, &mira).await.unwrap();
    assert_eq!(again, channels.get(&own(), &open.id).await.unwrap());
    assert_eq!(of(&rig, &open.id).await.pop().unwrap().seq, after.seq);
    // Channels opened on the same journal remember it as kept.
    let files = Attachments::open(journal.clone(), rig.dir.join("attachments")).await.unwrap();
    assert!(Channels::open(journal.clone(), files).await.unwrap().is_kept(&open.id));

    // Opened again: it takes a name among those channels share, and is
    // refused the one another has, with nothing changed.
    let leave = writes(channels, &open.id, &owner).await;
    let taken = channels.open_again(&leave, &open.id, None, &owner).await.unwrap_err();
    assert_eq!(taken.kind, ErrorKind::Conflict);
    assert!(taken.message.contains("there is already a channel called `general`"), "{}", taken.message);
    assert!(channels.is_kept(&open.id) && told(&nell, &after) && !told(&otto, &after));
    assert_eq!(channels.get(&own(), &open.id).await.unwrap(), again);
    let nameless = channels.open_again(&leave, &open.id, Some("!!"), &owner).await.unwrap_err();
    assert_eq!(nameless.kind, ErrorKind::Invalid);
    let opened = channels.open_again(&leave, &open.id, Some("Old general"), &owner).await.unwrap();
    assert_eq!((opened.kept, opened.title.as_deref(), opened.name.as_str()), (false, None, "old-general"));
    assert_eq!(channels.get(&own(), "old-general").await.unwrap(), opened);
    let said = of(&rig, &open.id).await.pop().unwrap();
    assert_eq!((said.topic.as_str(), said.source.takes.as_deref()), ("channel.kept", None));
    assert_eq!((&said.data["kept"], &said.data["name"]), (&false.into(), &"old-general".into()));
    // What it was called while it was kept was its members', and is not said.
    assert!(said.data.get("title").is_none() && said.data.get("was").is_none() && told(&otto, &said));
    // Anybody who may read channels reads it, and is told what is said
    // there from here; nobody is a member of it as of a kept one.
    assert!(channels.may_read("old-general", &otto).await.is_ok() && !channels.is_kept(&open.id));
    channels.post("old-general", &otto, "sloe", None).await.unwrap();
    let since = of(&rig, &open.id).await.pop().unwrap();
    assert!(since.source.takes.is_none() && told(&otto, &since));
    assert!(!told(&nell, &after), "what was said to its members is told to nobody by who is in it now");
    assert!(told(&owner, &after), "whoever reads everything still does");
    // Otto came into it by what he said there, as anybody does into an open channel.
    let opened = channels.get(&own(), &open.id).await.unwrap();
    assert!(opened.members.iter().any(|member| member.actor == "user:otto"));
    // Opened again, nothing changes.
    let same = channels.open_again(&writes(channels, &open.id, &owner).await, &open.id, None, &owner).await;
    assert_eq!(same.unwrap(), opened);
    assert_eq!(of(&rig, &open.id).await.pop().unwrap().seq, since.seq);
}

/// Who is in the channel `id`, as the tables have it.
async fn open_members(channels: &Channels, id: &str) -> Vec<Member> {
    channels.get(&own(), id).await.unwrap().members
}

#[tokio::test]
async fn whether_a_kept_channels_agents_are_asked_nowhere_else_is_said_of_it_afterwards_and_held_as_at_its_making()
 {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let (mira, otto) = (who("mira", &["channels"]), who("otto", &["channels"]));
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    let other = channels.create_kept("Hiring", None, "", &mira).await.unwrap();
    let lobby = channels.create("lobby", None, "", &mira).await.unwrap();
    let (to_kept, to_other) =
        (writes(channels, &kept.id, &mira).await, writes(channels, &other.id, &mira).await);
    channels.bring(&to_kept, &kept.id, "scribe").await.unwrap();
    channels.bring(&to_other, &other.id, "scribe").await.unwrap();
    channels.bring(&writes(channels, "lobby", &mira).await, "lobby", "scribe").await.unwrap();

    // Not while one of its agents is in another kept channel: an agent
    // is held so to one, and the other is named to nobody.
    let refused = channels.hold_alone(&to_kept, &kept.id, true, &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("`scribe` would be asked nowhere but here"), "{}", refused.message);
    assert!(!refused.message.contains(&other.id) && !refused.message.contains("Hiring"));
    assert_eq!(channels.alone_in("agent:scribe"), None);
    assert!(!channels.get(&own(), &kept.id).await.unwrap().alone);
    // Nor is it said of an open channel.
    let open = channels.hold_alone(&writes(channels, "lobby", &mira).await, "lobby", true, &mira).await;
    assert_eq!(open.unwrap_err().kind, ErrorKind::Invalid);

    // Once it is in no other, it is: said to the channel's members, and
    // the agent is held from there, an open channel being no matter.
    channels.leave(&to_other, &other.id, "agent:scribe").await.unwrap();
    let held = channels.hold_alone(&to_kept, &kept.id, true, &mira).await.unwrap();
    assert!(held.alone && channels.get(&own(), &kept.id).await.unwrap().alone);
    assert_eq!(channels.alone_in("agent:scribe"), Some(kept.id.clone()));
    let said = of(&rig, &kept.id).await.pop().unwrap();
    assert_eq!(said.topic, "channel.kept");
    assert_eq!(said.source.takes.as_deref(), Some(format!("member:{}", kept.id).as_str()));
    assert_eq!(
        (&said.data["kept"], &said.data["alone"], &said.data["title"]),
        (&true.into(), &true.into(), &"Pay review".into())
    );
    assert!(journal.may_tell(&mira, &said) && !journal.may_tell(&otto, &said));
    assert!(channels.bring(&to_other, &other.id, "scribe").await.is_err(), "held to the one");
    // Said again it is as it was, and nothing more is said.
    channels.hold_alone(&to_kept, &kept.id, true, &mira).await.unwrap();
    assert_eq!(of(&rig, &kept.id).await.pop().unwrap().seq, said.seq);
    // Remembered by channels opened on the same journal.
    let files = Attachments::open(journal.clone(), rig.dir.join("attachments")).await.unwrap();
    let again = Channels::open(journal.clone(), files).await.unwrap();
    assert_eq!(again.alone_in("agent:scribe"), Some(kept.id.clone()));

    // An open channel an agent so held is in is not kept: it would be
    // in a second kept one.
    let refused = channels.keep(&writes(channels, "lobby", &mira).await, "lobby", &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(
        refused.message.contains("`scribe` is in a channel it is asked nowhere but in"),
        "{}",
        refused.message
    );
    assert!(refused.message.contains("channel leave lobby --agent scribe"), "{}", refused.message);
    assert!(!refused.message.contains(&kept.id) && !refused.message.contains("Pay review"));
    assert!(!channels.is_kept(&lobby.id) && !channels.get(&own(), "lobby").await.unwrap().kept);

    // And let go: the agent is asked anywhere again, and brought into
    // another kept channel as any is.
    let free = channels.hold_alone(&to_kept, &kept.id, false, &mira).await.unwrap();
    assert!(!free.alone && channels.alone_in("agent:scribe").is_none());
    channels.bring(&to_other, &other.id, "scribe").await.unwrap();
    // Opened again, one that held its agents holds nobody.
    channels.leave(&to_other, &other.id, "agent:scribe").await.unwrap();
    channels.hold_alone(&to_kept, &kept.id, true, &mira).await.unwrap();
    let owner = who("owen", &["channels", "private"]);
    let opened = channels.open_again(&writes(channels, &kept.id, &owner).await, &kept.id, None, &owner).await;
    assert!(!opened.unwrap().alone && channels.alone_in("agent:scribe").is_none());
    assert!(!channels.get(&own(), "pay-review").await.unwrap().alone);
}

#[tokio::test]
async fn a_channel_is_made_with_who_is_in_it_and_somebody_leaves_by_asking_but_not_the_last_person_of_a_kept_one()
 {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let (mira, nell, otto) =
        (who("mira", &["channels"]), who("nell", &["channels"]), who("otto", &["channels"]));
    let person = |id: &str| Member::new(format!("user:{id}"), id);
    let bot = Member::new("bot:minutes", "minutes");

    // A kept channel, made with somebody and a bot in it: one write, in
    // which each is said to have joined, to all of them by name.
    let with = [person("nell"), bot.clone(), person("mira"), person("nell")];
    let asked = Asked { name: " Pay review ", topic: "pay", kept: true, members: &with, ..Asked::default() };
    let kept = channels.create_as(asked, &mira).await.unwrap();
    let within: Vec<&str> = kept.members.iter().map(|member| member.actor.as_str()).collect();
    assert_eq!(within, ["user:mira", "user:nell", "bot:minutes"], "its maker first, and nobody twice");
    assert_eq!((kept.title.as_deref(), kept.name.as_str()), (Some("Pay review"), kept.id.as_str()));
    assert_eq!(channels.get(&own(), &kept.id).await.unwrap(), kept);
    let events = of(&rig, &kept.id).await;
    let topics: Vec<&str> = events.iter().map(|event| event.topic.as_str()).collect();
    assert_eq!(topics, ["channel.joined", "channel.joined", "channel.joined", "channel.created"]);
    let all = format!("member:{};user:mira;user:nell;bot:minutes", kept.id);
    assert!(events.iter().all(|event| event.source.takes.as_deref() == Some(all.as_str())));
    assert!(events.iter().all(|event| journal.may_tell(&nell, event) && !journal.may_tell(&otto, event)));
    // Whoever it was made with reads it from the first.
    channels.post(&kept.id, &nell, "quince", None).await.unwrap();
    assert!(journal.may_tell(&nell, &of(&rig, &kept.id).await.pop().unwrap()));

    // What makes no sense is refused, and nothing is made.
    let agent = [Member::new("agent:scribe", "scribe")];
    for (asked, why) in [
        (Asked { name: "Hiring", kept: true, members: &agent, ..Asked::default() }, "an agent is brought in"),
        (Asked { name: "hiring", alone: true, ..Asked::default() }, "`alone` is said of a kept one"),
        (Asked { name: "  ", kept: true, ..Asked::default() }, "a kept channel is called something"),
        (Asked { name: "!!", ..Asked::default() }, "an open one has a name"),
    ] {
        assert_eq!(channels.create_as(asked, &mira).await.unwrap_err().kind, ErrorKind::Invalid, "{why}");
    }
    assert_eq!(channels.list(&own()).await.unwrap().len(), 1);
    // An open one is made with people too, and is refused a name another has.
    let with = [person("nell")];
    let open =
        channels.create_as(Asked { name: "Lobby", members: &with, ..Asked::default() }, &mira).await.unwrap();
    assert_eq!((open.name.as_str(), open.kept, open.members.len()), ("lobby", false, 2));
    let taken = channels.create_as(Asked { name: "lobby", ..Asked::default() }, &otto).await.unwrap_err();
    assert_eq!(
        (taken.kind, taken.message.as_str()),
        (ErrorKind::Conflict, "there is already a channel called `lobby`")
    );

    // Somebody leaves a kept channel by asking; whoever is not in it
    // has no leave to ask with; and the last person in it does not.
    assert!(channels.may_write(&kept.id, &otto).await.is_err());
    channels.step_out(&writes(channels, &kept.id, &nell).await, &kept.id, &nell).await.unwrap();
    let left = of(&rig, &kept.id).await.pop().unwrap();
    assert_eq!(left.topic, "channel.left");
    assert_eq!(left.source.takes.as_deref(), Some(format!("member:{};user:nell", kept.id).as_str()));
    assert!(channels.may_read(&kept.id, &nell).await.is_err());
    let last =
        channels.step_out(&writes(channels, &kept.id, &mira).await, &kept.id, &mira).await.unwrap_err();
    assert_eq!(last.kind, ErrorKind::Conflict);
    assert!(last.message.contains("nobody else is in it: add somebody first"), "{}", last.message);
    assert_eq!(open_members(channels, &kept.id).await.len(), 2, "its maker and the bot");
    assert!(journal.may_tell(&mira, &left), "still told, being in it still");
    // Whoever reads everything and is not in it is not in it to leave.
    let owner = who("owen", &["channels", "private"]);
    let outside = channels.step_out(&writes(channels, &kept.id, &owner).await, &kept.id, &owner).await;
    assert_eq!(outside.unwrap_err().kind, ErrorKind::Invalid);
    assert_eq!(of(&rig, &kept.id).await.pop().unwrap().seq, left.seq, "nothing is said of a leaving refused");

    // An open channel is left by anybody in it, the last among them.
    channels.step_out(&writes(channels, "lobby", &nell).await, "lobby", &nell).await.unwrap();
    channels.step_out(&writes(channels, "lobby", &mira).await, "lobby", &mira).await.unwrap();
    assert!(open_members(channels, "lobby").await.is_empty());
    let outside = channels.step_out(&writes(channels, "lobby", &otto).await, "lobby", &otto).await;
    assert_eq!(outside.unwrap_err().kind, ErrorKind::Invalid);

    // Channels are listed by what each is called, kept or open.
    channels.create_kept("apple pie", None, "", &mira).await.unwrap();
    channels.create_kept("Zebra", None, "", &mira).await.unwrap();
    channels.create("melon", None, "", &mira).await.unwrap();
    let listed = channels.readable(&mira).await.unwrap();
    let called: Vec<&str> = listed.iter().map(|one| one.title.as_deref().unwrap_or(&one.name)).collect();
    assert_eq!(called, ["apple pie", "lobby", "melon", "Pay review", "Zebra"]);
}

#[tokio::test]
async fn a_step_finds_the_kept_channel_it_asked_for_before_or_one_kept_since_and_is_refused_one_where_an_open_one_has_the_name()
 {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let (mira, nell) = (who("mira", &["channels"]), who("nell", &["channels"]));
    let owner = who("owen", &["channels", "private"]);
    let kept = |name: &'static str, project: Option<&'static str>| Asked {
        name,
        project,
        topic: "",
        kept: true,
        ..Asked::default()
    };

    // Asked for kept: made, and found again by what it is called, in
    // its project, by whoever is in it. The same done twice.
    let made = channels.ensure_as(kept("Pay review", Some("shop")), &mira).await.unwrap();
    assert!(
        made.kept && made.title.as_deref() == Some("Pay review") && made.project.as_deref() == Some("shop")
    );
    assert_eq!(channels.ensure_as(kept("Pay review", Some("shop")), &mira).await.unwrap().id, made.id);
    assert_eq!(channels.ensure_as(kept("pay-review", Some("shop")), &mira).await.unwrap().id, made.id);
    // Asked for by a step that says nothing of kept, it is the one found.
    assert_eq!(channels.ensure("Pay review", Some("shop"), "", &mira).await.unwrap().id, made.id);
    // Not one of another project, and not one whoever asks is not in,
    // whatever they read: each of those is made its own.
    let elsewhere = channels.ensure_as(kept("Pay review", Some("yard")), &mira).await.unwrap();
    let nobodys = channels.ensure_as(kept("Pay review", None), &mira).await.unwrap();
    let hers = channels.ensure_as(kept("Pay review", Some("shop")), &nell).await.unwrap();
    let owners = channels.ensure_as(kept("Pay review", Some("shop")), &owner).await.unwrap();
    let all = [&made.id, &elsewhere.id, &nobodys.id, &hers.id, &owners.id];
    assert_eq!(all.iter().collect::<std::collections::BTreeSet<_>>().len(), 5);
    // Of two it is in that are called the same, the one made first.
    let reader = channels.may_write(&hers.id, &nell).await.unwrap();
    channels.join(&reader, &hers.id, "user:mira", "mira").await.unwrap();
    assert_eq!(channels.ensure_as(kept("Pay review", Some("shop")), &mira).await.unwrap().id, made.id);

    // An open channel is found by its name as it always was, and is not
    // what a step that asks for a kept one is handed.
    let open = channels.ensure("general", Some("shop"), "", &mira).await.unwrap();
    assert_eq!(channels.ensure("General", None, "", &nell).await.unwrap().id, open.id);
    let refused = channels.ensure_as(kept("General", Some("shop")), &mira).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("#general is an open channel"), "{}", refused.message);
    assert!(!channels.get(&own(), "general").await.unwrap().kept);
    // Kept since by somebody, it is found by the step that made it, by
    // what it was called; and by whoever is not in it, it is not: they
    // are handed an open channel of the name, made for them.
    channels.keep(&channels.may_write("general", &mira).await.unwrap(), "general", &mira).await.unwrap();
    assert_eq!(channels.ensure("general", Some("shop"), "", &mira).await.unwrap().id, open.id);
    assert_eq!(channels.ensure_as(kept("general", Some("shop")), &mira).await.unwrap().id, open.id);
    let theirs = channels.ensure("general", Some("shop"), "", &nell).await.unwrap();
    assert!(theirs.id != open.id && !theirs.kept && theirs.name == "general");
}

#[tokio::test]
async fn a_channel_is_kept_opened_and_told_of_its_agents_with_what_is_said_of_it_or_not_at_all() {
    let rig = Rig::new().await;
    let (channels, journal) = (&rig.channels, &rig.journal);
    let (mira, otto) = (who("mira", &["channels"]), who("otto", &["channels"]));
    let owner = who("owen", &["channels", "private"]);
    let open = channels.create("general", None, "", &mira).await.unwrap();
    let kept = channels.create_kept("Pay review", None, "", &mira).await.unwrap();
    channels.bring(&writes(channels, &kept.id, &mira).await, &kept.id, "scribe").await.unwrap();
    channels.post(&kept.id, &mira, "quince", None).await.unwrap();
    let message = of(&rig, &kept.id).await.pop().unwrap();
    // Nothing of a channel can be said from here on.
    let unsaid = journal.db().call(|connection| {
        connection.execute_batch(
            "CREATE TRIGGER unsaid BEFORE INSERT ON events WHEN NEW.topic LIKE 'channel.%'
             BEGIN SELECT RAISE(ABORT, 'nothing of a channel is said'); END;",
        )
    });
    unsaid.await.unwrap();

    // An open channel that cannot be said to have been kept is open, in
    // the tables and for whoever is told an event.
    assert!(channels.keep(&writes(channels, "general", &mira).await, "general", &mira).await.is_err());
    assert_eq!(channels.get(&own(), "general").await.unwrap(), open);
    assert!(!channels.is_kept(&open.id) && channels.may_read("general", &otto).await.is_ok());
    // A kept one that cannot be said to be open again is kept, its
    // members told what is said of it and nobody else.
    let leave = writes(channels, &kept.id, &owner).await;
    assert!(channels.open_again(&leave, &kept.id, None, &owner).await.is_err());
    assert!(channels.get(&own(), &kept.id).await.unwrap().kept && channels.is_kept(&kept.id));
    assert!(journal.may_tell(&mira, &message) && !journal.may_tell(&otto, &message));
    assert!(channels.may_read(&kept.id, &otto).await.is_err());
    // And one that cannot be said to hold its agents holds none.
    assert!(
        channels.hold_alone(&writes(channels, &kept.id, &mira).await, &kept.id, true, &mira).await.is_err()
    );
    assert!(!channels.get(&own(), &kept.id).await.unwrap().alone);
    assert_eq!(channels.alone_in("agent:scribe"), None);
    // Nor is one made with anybody in it.
    let with = [Member::new("user:otto", "otto")];
    let asked = Asked { name: "Hiring", kept: true, members: &with, ..Asked::default() };
    assert!(channels.create_as(asked, &mira).await.is_err());
    assert_eq!(channels.list(&own()).await.unwrap().len(), 2);
    assert!(channels.kept_with("user:otto").is_empty());
}

#[tokio::test]
async fn two_steps_that_make_sure_of_one_open_channel_at_once_both_find_the_one_that_was_made() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let mira = who("mira", &["channels"]);
    let asked = || Asked { name: "Shop", project: Some("shop"), topic: "", ..Asked::default() };
    // Both look before either has made it, and both are answered with the one channel.
    let (one, other) = tokio::join!(channels.ensure_as(asked(), &mira), channels.ensure_as(asked(), &mira));
    let (one, other) = (one.unwrap(), other.unwrap());
    assert_eq!((one.id.as_str(), one.name.as_str()), (other.id.as_str(), "shop"));
    assert_eq!(rig.topics("channel.created").await.len(), 1);
    // Asked for kept where the open one is, it is still refused.
    let kept = Asked { kept: true, ..asked() };
    assert_eq!(channels.ensure_as(kept, &mira).await.unwrap_err().kind, kernel::ErrorKind::Conflict);
}
