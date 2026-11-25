use kernel::ErrorKind;
use serde_json::json;
use talk::channels::mentions;

use crate::{Rig, ada, agent};

#[tokio::test]
async fn a_channel_has_members_and_messages_are_events() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let channel =
        channels.create("Riverside Shop", Some("riverside-shop"), "Work on the shop", &ada()).await.unwrap();
    assert_eq!(
        (channel.name.as_str(), channel.project.as_deref()),
        ("riverside-shop", Some("riverside-shop"))
    );
    assert_eq!(
        channel.members.iter().map(|m| m.actor.as_str()).collect::<Vec<_>>(),
        ["user:ada"],
        "its creator is in it"
    );

    channels.join(&crate::own(), "riverside-shop", "agent:builder", "builder").await.unwrap();
    channels.join(&crate::own(), &channel.id, "agent:builder", "builder").await.unwrap();
    let members: Vec<String> = channels
        .get(&crate::own(), "#riverside-shop")
        .await
        .unwrap()
        .members
        .into_iter()
        .map(|m| m.actor)
        .collect();
    assert_eq!(members, ["user:ada", "agent:builder"], "joining twice is joining once");

    let first =
        channels.post("riverside-shop", &ada(), "@builder please set up the project", None).await.unwrap();
    let builder = agent("builder", &["channels"]);
    let reply = channels.post(&channel.id, &builder, "On it.", Some(&first.id)).await.unwrap();
    assert_eq!(
        (reply.author.as_str(), reply.reply_to.as_deref()),
        ("agent:builder", Some(first.id.as_str()))
    );

    let events = rig.events("channel.message").await;
    assert_eq!(events[0].data["mentions"], json!(["builder"]));
    assert_eq!(events[0].data["channel_name"], "riverside-shop");
    assert_eq!(events[0].data["body"], "@builder please set up the project");
    assert_eq!(events[0].source.actor.as_deref(), Some("user:ada"));
    assert_eq!(events[1].data["author_name"], "builder");
    assert_eq!(
        rig.topics("channel.*").await,
        ["channel.joined", "channel.created", "channel.joined", "channel.message", "channel.message"]
    );
}

#[tokio::test]
async fn messages_are_read_newest_page_first_in_order() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    channels.create("general", None, "", &ada()).await.unwrap();
    let mut ids = Vec::new();
    for n in 1..=7 {
        ids.push(channels.post("general", &ada(), &format!("message {n}"), None).await.unwrap().id);
    }
    let bodies = |messages: Vec<talk::Message>| messages.into_iter().map(|m| m.body).collect::<Vec<_>>();
    assert_eq!(
        bodies(channels.messages(&crate::own(), "general", None, 3).await.unwrap()),
        ["message 5", "message 6", "message 7"]
    );
    assert_eq!(
        bodies(channels.messages(&crate::own(), "general", Some(&ids[4]), 3).await.unwrap()),
        ["message 2", "message 3", "message 4"]
    );
    assert_eq!(
        bodies(channels.messages(&crate::own(), "general", Some(&ids[1]), 3).await.unwrap()),
        ["message 1"]
    );
}

#[tokio::test]
async fn channels_refuse_what_makes_no_sense() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    channels.create("general", None, "", &ada()).await.unwrap();
    assert_eq!(channels.create("General!", None, "", &ada()).await.unwrap_err().kind, ErrorKind::Conflict);
    assert_eq!(channels.create("!!!", None, "", &ada()).await.unwrap_err().kind, ErrorKind::Invalid);
    assert_eq!(channels.post("general", &ada(), "   ", None).await.unwrap_err().kind, ErrorKind::Invalid);
    assert_eq!(channels.post("nowhere", &ada(), "hello", None).await.unwrap_err().kind, ErrorKind::NotFound);
    let lurker = agent("lurker", &["channels:read"]);
    assert_eq!(channels.post("general", &lurker, "hello", None).await.unwrap_err().kind, ErrorKind::Denied);

    // `ensure` is create-or-get.
    let again = channels.ensure("general", None, "ignored", &ada()).await.unwrap();
    assert_eq!(channels.list(&crate::own()).await.unwrap(), [again]);
}

#[test]
fn mentions_are_found_and_email_addresses_are_not() {
    assert_eq!(mentions("@builder and @Reviewer-2, then @builder again"), ["builder", "reviewer-2"]);
    assert_eq!(mentions("mail ada@example.com about it"), Vec::<String>::new());
    assert_eq!(mentions("(@ops) @"), ["ops"]);
}

#[tokio::test]
async fn a_thread_is_a_message_and_what_is_said_under_it() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    channels.create("general", None, "", &ada()).await.unwrap();
    channels.create("other", None, "", &ada()).await.unwrap();
    fn under(message: &str) -> talk::Place<'_> {
        talk::Place { thread: Some(message), ..talk::Place::default() }
    }

    let first = channels.post("general", &ada(), "Shall we ship on Friday?", None).await.unwrap();
    let second = channels.post("general", &ada(), "Lunch?", None).await.unwrap();
    let yes = channels.post_at("general", &ada(), "Yes, if the tests pass.", under(&first.id)).await.unwrap();
    // Said under a message that is itself under one, it joins the same thread: a thread is one deep.
    let and = channels.post_at("general", &ada(), "They do.", under(&yes.id)).await.unwrap();
    assert_eq!(
        (yes.thread.as_deref(), and.thread.as_deref()),
        (Some(first.id.as_str()), Some(first.id.as_str()))
    );

    // The channel itself shows neither; the message they are under says how many there are.
    let shown = channels.messages(&crate::own(), "general", None, 10).await.unwrap();
    let told: Vec<(&str, u32)> =
        shown.iter().map(|message| (message.body.as_str(), message.replies)).collect();
    assert_eq!(told, [("Shall we ship on Friday?", 2), ("Lunch?", 0)]);

    // The thread, read from any message of it.
    for from in [&first.id, &and.id] {
        let thread = channels.thread(&crate::own(), from).await.unwrap();
        let bodies: Vec<&str> = thread.iter().map(|message| message.body.as_str()).collect();
        assert_eq!(bodies, ["Shall we ship on Friday?", "Yes, if the tests pass.", "They do."]);
    }
    let told = rig.events("channel.message").await;
    assert_eq!(
        told.last().unwrap().data["thread"],
        first.id.as_str(),
        "whoever is following is told where it was said"
    );

    // It stays in the channel of the message it is under: a message of
    // another channel is as much there as one that was never said, to
    // say something under and to answer, and is refused in the same words.
    let said = |refused: kernel::Error, id: &str| (refused.kind, refused.message.replace(id, "<id>"));
    let nowhere = channels.post_at("general", &ada(), "Hello?", under("msg_nonesuch")).await.unwrap_err();
    assert_eq!(nowhere.kind, ErrorKind::NotFound);
    let elsewhere = channels.post_at("other", &ada(), "Wrong room.", under(&second.id)).await.unwrap_err();
    assert_eq!(said(elsewhere, &second.id), said(nowhere.clone(), "msg_nonesuch"));
    let answers = channels.post("other", &ada(), "Wrong room.", Some(&second.id)).await.unwrap_err();
    assert_eq!(said(answers, &second.id), said(nowhere.clone(), "msg_nonesuch"));
    let nothing = channels.post("general", &ada(), "Hello?", Some("msg_nonesuch")).await.unwrap_err();
    assert_eq!(said(nothing, "msg_nonesuch"), said(nowhere, "msg_nonesuch"));
    assert_eq!(channels.messages(&crate::own(), "other", None, 10).await.unwrap(), []);
    // Nor is a thread read by leave for another channel than its own.
    let other = channels.may_read("other", &ada()).await.unwrap();
    assert_eq!(channels.thread(&other, &first.id).await.unwrap_err().kind, ErrorKind::NotFound);
    assert_eq!(channels.message(&other, &first.id).await.unwrap_err().kind, ErrorKind::NotFound);
    let general = channels.may_read("general", &ada()).await.unwrap();
    assert_eq!(channels.thread(&general, &first.id).await.unwrap().len(), 3);
}

#[tokio::test]
async fn what_is_attached_goes_with_the_message_it_was_put_here_for() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let general = channels.create("general", None, "", &ada()).await.unwrap();
    let files = &channels.attachments;

    // Put here first, as it arrives before the message: it says what it
    // is, or its name does.
    let picture = files
        .add(&crate::own(), &general.id, &ada(), "shots/Board (1).png", None, vec![1, 2, 3])
        .await
        .unwrap();
    let notes = files
        .add(&crate::own(), &general.id, &ada(), "notes.txt", Some("text/plain"), b"hello".to_vec())
        .await
        .unwrap();
    assert_eq!(
        (picture.name.as_str(), picture.media.as_str(), picture.size),
        ("Board (1).png", "image/png", 3),
        "only the name of the file is kept, not where it was"
    );
    assert_eq!(std::fs::read(files.path(&notes.id)).unwrap(), b"hello", "it is a file, to be read as one");

    // Posted with a message, it is part of it: in the message, in its
    // event, and wherever it is read back.
    let ids = [picture.id.clone(), notes.id.clone()];
    let place = talk::Place { attachments: &ids, ..talk::Place::default() };
    let said = channels.post_at(&general.id, &ada(), "the board as it is", place).await.unwrap();
    let names =
        |message: &talk::Message| message.attachments.iter().map(|one| one.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&said), ["Board (1).png", "notes.txt"]);
    assert_eq!(
        names(&channels.message(&crate::own(), &said.id).await.unwrap()),
        ["Board (1).png", "notes.txt"]
    );
    assert_eq!(
        names(&channels.messages(&crate::own(), &general.id, None, 10).await.unwrap()[0]),
        ["Board (1).png", "notes.txt"]
    );
    let event = rig.events("channel.message").await.pop().unwrap();
    assert_eq!(
        event.data["attachments"][0],
        json!({ "id": picture.id, "name": "Board (1).png", "media": "image/png", "size": 3 })
    );

    // With something attached a message need say nothing; with nothing, it must.
    let alone = files.add(&crate::own(), &general.id, &ada(), "plan.pdf", None, vec![9]).await.unwrap();
    let only = [alone.id.clone()];
    let silent = channels
        .post_at(&general.id, &ada(), "  ", talk::Place { attachments: &only, ..Default::default() })
        .await;
    assert_eq!(silent.unwrap().attachments[0].media, "application/pdf");
    let nothing = channels.post(&general.id, &ada(), "  ", None).await.unwrap_err();
    assert!(nothing.message.contains("some text, or something attached"), "{nothing}");
    // In a thread too.
    let reply = files.add(&crate::own(), &general.id, &ada(), "fix.patch", None, vec![7]).await.unwrap();
    let with = [reply.id.clone()];
    let under = talk::Place { thread: Some(&said.id), attachments: &with, ..Default::default() };
    channels.post_at(&general.id, &ada(), "and the fix", under).await.unwrap();
    assert_eq!(names(&channels.thread(&crate::own(), &said.id).await.unwrap()[1]), ["fix.patch"]);
}

#[tokio::test]
async fn only_what_somebody_put_here_themselves_and_has_not_used_can_be_attached() {
    let rig = Rig::new().await;
    let channels = &rig.channels;
    let general = channels.create("general", None, "", &ada()).await.unwrap();
    let other = channels.create("other", None, "", &ada()).await.unwrap();
    let files = &channels.attachments;
    let builder = agent("builder", &["channels"]);
    let post = async |by: &kernel::Principal, channel: &str, ids: &[String]| {
        channels
            .post_at(channel, by, "see this", talk::Place { attachments: ids, ..Default::default() })
            .await
    };

    let mine = files.add(&crate::own(), &general.id, &ada(), "a.png", None, vec![1]).await.unwrap();
    let theirs = files.add(&crate::own(), &general.id, &builder, "b.png", None, vec![1]).await.unwrap();
    let refused = async |by: &kernel::Principal, channel: &str, ids: &[String]| {
        let error = post(by, channel, ids).await.unwrap_err();
        assert_eq!(error.kind, ErrorKind::Invalid, "{error}");
        assert!(error.message.contains("not something you attached here"), "{error}");
    };
    // Somebody else's, something put here for another channel, and something made up.
    refused(&ada(), &general.id, std::slice::from_ref(&theirs.id)).await;
    refused(&ada(), &other.id, std::slice::from_ref(&mine.id)).await;
    refused(&ada(), &general.id, &["att_made_up".to_string()]).await;
    // One that cannot be attached refuses the lot, and nothing is said.
    refused(&ada(), &general.id, &[mine.id.clone(), theirs.id.clone()]).await;
    assert!(channels.messages(&crate::own(), &general.id, None, 10).await.unwrap().is_empty());
    // Its own, once.
    post(&ada(), &general.id, std::slice::from_ref(&mine.id)).await.unwrap();
    refused(&ada(), &general.id, std::slice::from_ref(&mine.id)).await;

    // Nothing in it, too much in it, and somebody who may not write here.
    let empty =
        files.add(&crate::own(), &general.id, &ada(), "empty.txt", None, Vec::new()).await.unwrap_err();
    assert!(empty.message.contains("nothing in that file"), "{empty}");
    let huge = vec![0u8; talk::attachments::MAX_ATTACHMENT_BYTES + 1];
    let large = files.add(&crate::own(), &general.id, &ada(), "huge.bin", None, huge).await.unwrap_err();
    assert!(large.message.contains("too large to attach"), "{large}");
    let reader = agent("reader", &["channels:read"]);
    assert_eq!(
        files.add(&crate::own(), &general.id, &reader, "c.png", None, vec![1]).await.unwrap_err().kind,
        ErrorKind::Denied
    );
    assert!(matches!(channels.attached("att_made_up", &ada()).await.unwrap_err().kind, ErrorKind::NotFound));
}
