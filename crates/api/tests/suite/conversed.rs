//! A conversation (`talk::conversed`): a message written to one or
//! several people and agents, by the routes that write one, read it and
//! open it as a channel.
//!
//! One journey in one factory, with its owner, three people, somebody
//! it is not written to, keys that hold less and an agent the factory
//! does not start: written to two people and the agent, each has its
//! line and whoever wrote it has none until it is answered; answered,
//! the line is raised, never a second and never one for being named;
//! read by its place; the agent reads its box and the conversation and
//! answers with its own credential; one who leaves has their line
//! passed over; somebody who is not in it finds nothing by any route,
//! list, search or stream, in the words a channel that is not there is
//! refused in; the factory's owner reads it and has no line; an agent
//! that may be in no kept channel, and somebody who is not there, are
//! refused in a sentence with nothing made; a copy of the factory keeps
//! it; opened as a channel it is listed for its members with everything
//! said so far and raises no further line; and one taken away takes its
//! lines. Then a factory as a program from before left it.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, asked, key, person, reads_none, say, signed_in};
use crate::snapshots::{Shared, taken};

/// What is the conversation's and nobody else's to read.
const WORDS: [&str; 6] = ["quay", "Quay", "mends", "medlar", "damson", "sloe"];

/// What `token`'s box holds: its lines, and how many wait unread.
async fn inbox(rig: &Rig, token: &str, all: bool) -> (Vec<Value>, u64) {
    let path = if all { "/api/inbox?all=true" } else { "/api/inbox" };
    let (status, found) = rig.call(Method::GET, path, Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    (found["lines"].as_array().unwrap().clone(), found["unread"].as_u64().unwrap())
}

/// Asks something as `token`, which is to go well: the answer.
async fn does(rig: &Rig, token: &str, method: Method, path: &str, body: Option<Value>) -> Value {
    let (status, answer) = rig.call(method, path, Some(token), body).await;
    assert_eq!(status, StatusCode::OK, "{path}: {answer}");
    answer
}

/// The topics said since `mark`, in the order they were said.
async fn since(rig: &Rig, mark: u64) -> Vec<String> {
    let query = journal::EventQuery { topics: "**".into(), after: mark, limit: 10_000, ..Default::default() };
    rig.said(query).await.into_iter().map(|event| event.topic).collect()
}

/// The channels `token` is listed, by id.
async fn listed(rig: &Rig, token: &str) -> Vec<Value> {
    does(rig, token, Method::GET, "/api/channels", None).await["channels"].as_array().unwrap().clone()
}

/// What `token` is answered by each way of reaching `channel`, beside
/// the same asked of a channel that is not there: the two are the same,
/// word for word, none is answered well, and none says a word of it.
async fn not_there(rig: &Rig, who: &str, token: &str, channel: &str, message: &str) {
    shut_to(rig, who, token, channel, message, false).await;
}

/// The same; `was_in` for somebody who was in it and left, who was told
/// that they left, by its id and nothing else of it.
async fn shut_to(rig: &Rig, who: &str, token: &str, channel: &str, message: &str, was_in: bool) {
    type Way = (&'static str, Method, &'static str, Option<Value>);
    let ways: Vec<Way> = vec![
        ("reading it", Method::GET, "/api/channels/{channel}", None),
        ("reading what was said", Method::GET, "/api/channels/{channel}/messages", None),
        ("reading a thread of it", Method::GET, "/api/channels/{channel}/messages?thread={message}", None),
        (
            "saying something",
            Method::POST,
            "/api/channels/{channel}/messages",
            Some(json!({ "body": "hello" })),
        ),
        ("attaching to it", Method::POST, "/api/channels/{channel}/attachments?name=a.txt", None),
        ("adding to it", Method::POST, "/api/channels/{channel}/members", Some(json!({ "actor": "user:x" }))),
        ("leaving it", Method::POST, "/api/channels/{channel}/leave", None),
        ("keeping it", Method::POST, "/api/channels/{channel}/keep", None),
        ("opening it to everybody", Method::POST, "/api/channels/{channel}/open", None),
        (
            "saying what it belongs to",
            Method::PUT,
            "/api/channels/{channel}/belongs",
            Some(json!({ "workspace": "front" })),
        ),
        ("opening it as a channel", Method::POST, "/api/inbox/conversations/{channel}/channel", None),
        ("reading its lines", Method::POST, "/api/inbox/read", Some(json!({ "place": "{channel}" }))),
        ("taking it away", Method::DELETE, "/api/channels/{channel}", None),
    ];
    for (way, method, path, body) in ways {
        let ask = |of: &str, said: &str| {
            let put = |text: &str| text.replace("{channel}", of).replace("{message}", said);
            let body =
                body.as_ref().map(|body| serde_json::from_str::<Value>(&put(&body.to_string())).unwrap());
            let (path, method) = (put(path), method.clone());
            async move { asked(rig, token, method, &path, body).await }
        };
        let (here, there) = (ask(channel, message).await, ask("ch_nonesuch", "msg_nonesuch").await);
        reads_none(&format!("by {who}, {way}"), &here.1, &WORDS);
        let plain = |answer: &str| answer.replace(channel, "<it>").replace(message, "<said>");
        assert_eq!(
            (here.0, plain(&here.1)),
            (there.0, there.1.replace("ch_nonesuch", "<it>").replace("msg_nonesuch", "<said>")),
            "{who}, {way}"
        );
    }
    // Nor is it listed, found or counted for them.
    let mut lists: Vec<String> =
        ["/api/channels", "/api/workspaces", "/api/inbox?all=true", "/api/events/recent?limit=1000"]
            .iter()
            .map(|path| path.to_string())
            .collect();
    lists.extend(WORDS.iter().map(|word| format!("/api/search?q={word}")));
    for path in lists {
        let (_, answer) = asked(rig, token, Method::GET, &path, None).await;
        reads_none(&format!("by {who}, at {path}"), &answer, &WORDS);
        if !was_in {
            reads_none(&format!("by {who}, at {path}"), &answer, &[channel, message]);
        }
    }
}

#[tokio::test]
async fn a_conversation_is_in_the_inbox_of_whoever_it_is_to_and_is_there_for_nobody_else_by_any_way() {
    let shared = Shared::new();
    let registry = shared.registry();
    let rig = Rig::with("api-conversed", move |config, _| {
        config.profile = Some("prod".into());
        config.registry = Some(registry);
    })
    .await;
    let owned = Rig::owned(rig).await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    rig.send("/api/workspaces", json!({ "name": "Front" })).await;
    let (mira, nell, odo) = (person(rig, "Mira").await, person(rig, "Nell").await, person(rig, "Odo").await);
    let pru = person(rig, "Pru").await;
    // An agent the factory does not start, with what one usually holds;
    // one it starts, which in a factory that holds no walls may be in no
    // channel kept to its members; a key of Nell's that reads no channel;
    // and somebody who manages the factory with a key that holds
    // nothing private.
    let invited =
        does(rig, &rig.token, Method::POST, "/api/agent/invite", Some(json!({ "name": "scout" }))).await;
    let joined = json!({ "invitation": invited["invitation"] });
    let (_, joined) = rig.call(Method::POST, "/api/agent/join", None, Some(joined)).await;
    let scout = joined["token"].as_str().unwrap().to_string();
    rig.send("/api/agents", json!({ "name": "clerk" })).await;
    let board = key(rig, &nell.token, &["board", "inbox"]).await;
    let mut manages: Vec<&str> = kernel::principal::MEMBER_SCOPES.to_vec();
    manages.push("admin");
    let admin = key(rig, &rig.token, &manages).await;
    let (mut to_pru, mut to_nell) = (Told::of(rig, &pru.token).await, Told::of(rig, &nell.token).await);
    let (mut to_scout, mut to_owner) = (Told::of(rig, &scout).await, Told::of(rig, &rig.token).await);
    let write = |token: &str, asked: Value| {
        let token = token.to_string();
        async move { rig.call(Method::POST, "/api/inbox/conversations", Some(&token), Some(asked)).await }
    };

    // 1: what is said in a channel that is no conversation is what it
    // always was, kept to its members or not: one thing said, and
    // nothing written beside it.
    let review = does(
        rig,
        &mira.token,
        Method::POST,
        "/api/channels",
        Some(json!({ "name": "Pay review", "kept": true })),
    )
    .await;
    let review = review["id"].as_str().unwrap().to_string();
    does(
        rig,
        &mira.token,
        Method::POST,
        &format!("/api/channels/{review}/members"),
        Some(json!({ "actor": nell.actor })),
    )
    .await;
    let mark = rig.mark();
    say(rig, &mira.token, &review, json!({ "body": "the pay round" })).await;
    assert_eq!(since(rig, mark).await, ["channel.message"]);

    // 2: written to two people and the agent, by a handle, as an actor
    // and by its name: one request makes the conversation with all four
    // in it and says its first message, each of those it is to told in
    // their box. It is a channel kept to them that says what it is.
    let mark = rig.mark();
    let to = json!({ "to": ["@nell", odo.actor, "scout"], "subject": "The quay", "text": "who mends it?" });
    let (status, written) = write(&mira.token, to).await;
    assert_eq!(status, StatusCode::OK, "{written}");
    let (channel, first) = (&written["channel"], written["message"]["id"].as_str().unwrap().to_string());
    let id = channel["id"].as_str().unwrap().to_string();
    assert_eq!(
        (&channel["conversation"], &channel["kept"], &channel["title"], &channel["name"]),
        (&json!(true), &json!(true), &json!("The quay"), &json!(id))
    );
    let within: Vec<&str> =
        channel["members"].as_array().unwrap().iter().map(|one| one["actor"].as_str().unwrap()).collect();
    assert_eq!(within, [mira.actor.as_str(), nell.actor.as_str(), odo.actor.as_str(), "agent:scout"]);
    let mut said = since(rig, mark).await;
    said.sort();
    assert_eq!(
        said,
        [
            "channel.created",
            "channel.joined",
            "channel.joined",
            "channel.joined",
            "channel.joined",
            "channel.message",
            "channel.placed",
            "inbox.told",
            "inbox.told",
            "inbox.told"
        ]
    );
    for (who, token, owner) in [
        ("Nell", &nell.token, nell.actor.as_str()),
        ("Odo", &odo.token, &odo.actor),
        ("the agent", &scout, "agent:scout"),
    ] {
        let (lines, unread) = inbox(rig, token, false).await;
        assert_eq!((lines.len(), unread), (1, 1), "{who}: {lines:?}");
        let (line, said) = (&lines[0], &lines[0]["said"]);
        assert_eq!(
            (&line["kind"], &line["about"], &line["place"]),
            (&json!("message"), &json!(id), &json!(id)),
            "{who}"
        );
        assert_eq!(
            (&line["owner"], &line["from"], &line["count"]),
            (&json!(owner), &json!(mira.actor), &json!(1)),
            "{who}"
        );
        assert_eq!(
            (&said["channel_title"], &said["conversation"], &said["body"], &said["author_name"]),
            (&json!("The quay"), &json!(true), &json!("who mends it?"), &json!("Mira")),
            "{who}"
        );
        assert_eq!(said["members"].as_array().unwrap().len(), 4, "{who}");
    }
    // Whoever wrote it has no line, and neither has the factory's owner,
    // who reads it as every channel kept to its members, for now.
    assert_eq!(inbox(rig, &mira.token, true).await, (Vec::new(), 0));
    assert_eq!(inbox(rig, &rig.token, true).await, (Vec::new(), 0));
    assert!(listed(rig, &rig.token).await.iter().any(|one| one["id"] == json!(id)));
    // It is handed to its members among their channels, with its mark,
    // and no workspace lists it for anybody.
    for token in [&mira.token, &nell.token, &scout] {
        let found = listed(rig, token).await;
        let it = found.iter().find(|one| one["id"] == json!(id)).unwrap();
        assert_eq!(it["conversation"], true);
    }
    let spaces = does(rig, &nell.token, Method::GET, "/api/workspaces", None).await.to_string();
    reads_none("in what a member is listed of the workspaces", &spaces, &[&id]);

    // 3: somebody it is not written to, a key of a member's that reads
    // no channel, and an administrator who is not the owner find nothing
    // of it, by any route, list or search, and were told nothing.
    rig.node().app.studio.search.catch_up().await.unwrap();
    for (who, token) in [("somebody it is not to", &pru.token), ("an administrator", &admin)] {
        not_there(rig, who, token, &id, &first).await;
    }
    let (status, refused) = asked(rig, &board, Method::GET, &format!("/api/channels/{id}"), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    reads_none("by a member's key that reads no channel", &refused, &WORDS);
    assert_eq!(
        inbox(rig, &board, true).await,
        (Vec::new(), 0),
        "its line is passed over for a key that reads no channel"
    );
    // A member finds what was said there.
    let (_, hits) = asked(rig, &nell.token, Method::GET, "/api/search?q=mends", None).await;
    assert!(hits.contains(&first), "{hits}");

    // 4: answered, with two of those in it named: whoever wrote it has
    // a line now, the line of each of the others is raised and is the
    // one line still, and nobody in it has a line for being named.
    let mark = rig.mark();
    let answer = say(rig, &nell.token, &id, json!({ "body": "@mira @odo the medlar yard does" })).await;
    assert_eq!(since(rig, mark).await, ["channel.message", "inbox.told", "inbox.told", "inbox.told"]);
    let (lines, unread) = inbox(rig, &mira.token, false).await;
    assert_eq!((lines.len(), unread, &lines[0]["kind"]), (1, 1, &json!("message")), "{lines:?}");
    assert_eq!(lines[0]["said"]["body"], "@mira @odo the medlar yard does");
    let (lines, unread) = inbox(rig, &odo.token, true).await;
    assert_eq!(
        (lines.len(), unread, &lines[0]["count"], &lines[0]["said_by"]),
        (1, 1, &json!(2), &json!(answer))
    );
    assert_eq!(inbox(rig, &nell.token, false).await.1, 1, "her own answer raises nothing of hers");

    // 5: read by its place, as a channel that comes into view is; and
    // said again, it waits again.
    let read = does(rig, &odo.token, Method::POST, "/api/inbox/read", Some(json!({ "place": id }))).await;
    assert_eq!(read, json!({ "read": 1, "unread": 0 }));
    let hers = inbox(rig, &nell.token, false).await.0[0]["id"].as_str().unwrap().to_string();
    does(rig, &nell.token, Method::POST, &format!("/api/inbox/{hers}/away"), Some(json!({}))).await;
    assert_eq!(inbox(rig, &nell.token, false).await, (Vec::new(), 0));

    // 6: the agent reads its box and the conversation with its own
    // credential, and answers there: put away, Nell's line is back, and
    // nothing is started for the agent, which has no turn.
    let (lines, unread) = inbox(rig, &scout, false).await;
    assert_eq!(
        (unread, &lines[0]["count"], &lines[0]["said"]["body"]),
        (1, &json!(2), &json!("@mira @odo the medlar yard does"))
    );
    let read = does(rig, &scout, Method::GET, &format!("/api/channels/{id}/messages"), None).await;
    assert_eq!(read["messages"].as_array().unwrap().len(), 2);
    does(rig, &scout, Method::POST, "/api/inbox/read", Some(json!({ "place": id }))).await;
    let mark = rig.mark();
    say(rig, &scout, &id, json!({ "body": "the damson gate is hung" })).await;
    assert_eq!(since(rig, mark).await, ["channel.message", "inbox.told", "inbox.told", "inbox.told"]);
    let (lines, unread) = inbox(rig, &nell.token, false).await;
    assert_eq!((unread, &lines[0]["id"], &lines[0]["away"]), (1, &json!(hers), &Value::Null));
    assert_eq!(
        (&lines[0]["from"], &lines[0]["said"]["author_name"]),
        (&json!("agent:scout"), &json!("scout"))
    );
    assert_eq!(inbox(rig, &scout, false).await.1, 0);
    let turns = does(rig, &rig.token, Method::GET, "/api/agents/scout/turns", None).await;
    assert_eq!(turns["turns"], json!([]), "{turns}");
    // It writes one too, to a person, who has it in their box.
    let (status, back) =
        write(&scout, json!({ "to": [pru.actor], "text": "the bullace hedge is cut" })).await;
    assert_eq!(status, StatusCode::OK, "{back}");
    let hedge = back["channel"]["id"].as_str().unwrap().to_string();
    assert_eq!(back["channel"]["title"], "the bullace hedge is cut");
    let (lines, unread) = inbox(rig, &pru.token, false).await;
    assert_eq!((lines.len(), unread, &lines[0]["place"]), (1, 1, &json!(hedge)));

    // 7: one who leaves has their line passed over: not listed, not
    // counted, not found by its id, and not raised by what is said
    // next. The conversation is to them one that is not there.
    let his = inbox(rig, &odo.token, true).await.0[0]["id"].as_str().unwrap().to_string();
    does(rig, &odo.token, Method::POST, &format!("/api/channels/{id}/leave"), None).await;
    say(rig, &mira.token, &id, json!({ "body": "the sloe is late" })).await;
    assert_eq!(inbox(rig, &odo.token, true).await, (Vec::new(), 0));
    let gone = asked(rig, &odo.token, Method::POST, &format!("/api/inbox/{his}/read"), None).await;
    let never = asked(rig, &odo.token, Method::POST, "/api/inbox/inb_never/read", None).await;
    assert_eq!((gone.0, gone.1.replace(&his, "inb_never")), never);
    shut_to(rig, "somebody who left it", &odo.token, &id, &first, true).await;

    // 8: what the daemon refuses is said in its own words, and nothing
    // is made: an agent that may be in no channel kept to its members,
    // somebody who is not there, a bot, nobody at all, nothing said,
    // and whoever holds less than writing one takes.
    let no_box = key(rig, &nell.token, &["channels"]).await;
    let (made, mark) = (listed(rig, &rig.token).await.len(), rig.mark());
    for (to, status, why) in [
        (
            json!(["@nell", "clerk"]),
            StatusCode::CONFLICT,
            "an agent with no wall is in no channel kept to its members, which a conversation is",
        ),
        (json!(["@nobody"]), StatusCode::NOT_FOUND, "nobody here is called `@nobody`"),
        (json!(["user:nobody"]), StatusCode::NOT_FOUND, "nobody with an account here is `user:nobody`"),
        (json!(["agent:nobody"]), StatusCode::NOT_FOUND, "there is no agent called `nobody`"),
        (json!(["bot:factory"]), StatusCode::BAD_REQUEST, "never to a bot"),
        (json!([mira.actor]), StatusCode::BAD_REQUEST, "a conversation is written to somebody"),
        (json!([]), StatusCode::BAD_REQUEST, "`to` is needed"),
    ] {
        let (answered, refused) =
            write(&mira.token, json!({ "to": to, "subject": "Quay", "text": "medlar" })).await;
        assert_eq!(answered, status, "{refused}");
        assert!(refused.to_string().contains(why), "{refused}");
    }
    let (status, refused) = write(&mira.token, json!({ "to": ["@nell"], "subject": "Quay" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    for (token, lacks) in [(&board, "channels:write"), (&no_box, "inbox:write")] {
        let (status, refused) = write(token, json!({ "to": [mira.actor], "text": "medlar" })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert!(refused.to_string().contains(lacks), "{refused}");
    }
    assert_eq!((listed(rig, &rig.token).await.len(), since(rig, mark).await), (made, Vec::<String>::new()));

    // 9: nobody outside it was told anything of it as it was said, the
    // owner heard what was said there and nothing of anybody's box, and
    // those in it heard each thing with their own line.
    let general =
        does(rig, &mira.token, Method::POST, "/api/channels", Some(json!({ "name": "general" }))).await;
    say(rig, &mira.token, general["id"].as_str().unwrap(), json!({ "body": "marker-one" })).await;
    to_pru.until("marker-one").await;
    reads_none(
        "on what somebody it is not to is told",
        &to_pru.heard,
        &["quay", "mends", "medlar", "damson", &id],
    );
    to_owner.until("marker-one").await;
    assert!(to_owner.heard.contains("who mends it?"), "the owner hears what is said in it");
    reads_none("on what the owner is told", &to_owner.heard, &["inbox."]);
    for (who, told) in [("Nell", &mut to_nell), ("the agent", &mut to_scout)] {
        told.until("marker-one").await;
        assert!(told.heard.contains("who mends it?") && told.heard.contains("the sloe is late"), "{who}");
        assert!(told.heard.contains("inbox.told"), "{who}");
    }

    // 10: a copy of the factory keeps the conversation and the lines.
    let to = shared.snapshot("copy");
    taken(rig, &to).await;
    let copy = shared.copy("dev1", &to).await;
    let nell_there = signed_in(&copy, "Nell").await;
    assert_eq!(inbox(&copy, &nell_there, true).await, inbox(rig, &nell.token, true).await);
    let there = listed(&copy, &nell_there).await;
    assert!(there.iter().any(|one| one["id"] == json!(id) && one["conversation"] == true), "{there:?}");

    // 11: opened as a channel by one of its members, not by the owner,
    // who reads it and is not in it: called what was asked, in the
    // workspace that was said, kept to its members with everything said
    // so far, and listed for them. Its lines are as they were; what is
    // said there from then on raises none.
    let at = format!("/api/inbox/conversations/{id}/channel");
    let (status, refused) = asked(rig, &rig.token, Method::POST, &at, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let mark = rig.mark();
    let asked_for = json!({ "name": "Quay works", "workspace": "front" });
    let opened = does(rig, &nell.token, Method::POST, &at, Some(asked_for)).await;
    assert_eq!(
        (
            &opened["channel"]["title"],
            &opened["channel"]["kept"],
            opened["channel"].get("conversation"),
            &opened["workspace"]
        ),
        (&json!("Quay works"), &json!(true), None, &json!("front"))
    );
    assert_eq!(since(rig, mark).await, ["channel.placed", "channel.kept"]);
    let said = rig.event("channel.kept", |event| event.seq > mark).await;
    assert_eq!((&said.data["conversation"], &said.data["was"]), (&json!(false), &json!("The quay")));
    let front = does(rig, &mira.token, Method::GET, "/api/workspaces/front", None).await;
    assert!(front["channels"].as_array().unwrap().contains(&json!(id)), "{front}");
    let read = does(rig, &mira.token, Method::GET, &format!("/api/channels/{id}/messages"), None).await;
    assert_eq!(read["messages"].as_array().unwrap().len(), 4);
    let before = inbox(rig, &mira.token, true).await;
    assert_eq!(
        (before.0.len(), &before.0[0]["said"]["channel_title"], before.0[0]["said"].get("conversation")),
        (1, &json!("Quay works"), None)
    );
    let mark = rig.mark();
    say(rig, &nell.token, &id, json!({ "body": "the works begin" })).await;
    assert_eq!(since(rig, mark).await, ["channel.message"]);
    assert_eq!(inbox(rig, &mira.token, true).await, before);
    // It is not a conversation any more, to be opened so again; and to
    // whoever is not in it it is as shut as it was.
    let (status, refused) = asked(rig, &nell.token, Method::POST, &at, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused.contains("is no conversation"), "{refused}");
    rig.node().app.studio.search.catch_up().await.unwrap();
    not_there(rig, "somebody who is not in the channel it became", &pru.token, &id, &first).await;

    // 12: a conversation that is taken away takes the lines that open
    // it, each owner told which went.
    let mark = rig.mark();
    does(rig, &rig.token, Method::DELETE, &format!("/api/channels/{hedge}"), None).await;
    let away = rig.event("inbox.away", |event| event.seq > mark).await;
    assert_eq!(
        (&away.data["owner"], &away.data["place"], &away.data["unread"]),
        (&json!(pru.actor), &json!(hedge), &json!(0))
    );
    assert_eq!(inbox(rig, &pru.token, true).await, (Vec::new(), 0));
}

#[tokio::test]
async fn a_factory_from_before_starts_on_this_program_with_its_channels_as_they_were_and_none_a_conversation()
{
    let mut rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let mira = person(&rig, "Mira").await;
    let general =
        does(&rig, &mira.token, Method::POST, "/api/channels", Some(json!({ "name": "general" }))).await;
    let review = json!({ "name": "Pay review", "kept": true, "topic": "the round" });
    let review = does(&rig, &mira.token, Method::POST, "/api/channels", Some(review)).await;
    say(&rig, &mira.token, review["id"].as_str().unwrap(), json!({ "body": "quince" })).await;
    let mut before = listed(&rig, &mira.token).await;
    rig.stop().await;
    // As a program from before left it: no column that says a channel is
    // a conversation, nor one that says it is archived, no documents kept
    // to hand, no mark on a message nor discussions, no word of whether
    // a member listens or manages, and the channels' steps six fewer.
    let file = rig.dir.join("data/journal.db");
    let database = rusqlite::Connection::open(&file).unwrap();
    let steps: i64 = database
        .query_row("SELECT version FROM schema_versions WHERE namespace = 'channels'", [], |row| row.get(0))
        .unwrap();
    database
        .execute_batch(&format!(
            "DROP TABLE discussions;
             DROP TABLE discussing;
             ALTER TABLE messages DROP COLUMN mark;
             ALTER TABLE channel_members DROP COLUMN manages;
             ALTER TABLE channel_members DROP COLUMN listens;
             DROP TABLE channel_docs;
             ALTER TABLE channels DROP COLUMN archived;
             ALTER TABLE channels DROP COLUMN conversation;
             UPDATE schema_versions SET version = {} WHERE namespace = 'channels';",
            steps - 6
        ))
        .unwrap();
    let said = |database: &rusqlite::Connection| -> i64 {
        let of_channels = "SELECT count(*) FROM events WHERE topic LIKE 'channel.%' OR topic LIKE 'inbox.%'";
        database.query_row(of_channels, [], |row| row.get(0)).unwrap()
    };
    let was = said(&database);
    drop(database);
    rig.start().await;

    // It starts, with every channel as it was and nothing said of any.
    let database = rusqlite::Connection::open(&file).unwrap();
    assert_eq!(said(&database), was);
    let marked: i64 = database
        .query_row("SELECT count(*) FROM channels WHERE conversation != 0", [], |row| row.get(0))
        .unwrap();
    assert_eq!(marked, 0);
    drop(database);
    let mira = signed_in(&rig, "Mira").await;
    // As they were, but that a program from before kept no managers, so its maker manages neither now.
    for channel in &mut before {
        for member in channel["members"].as_array_mut().unwrap() {
            member.as_object_mut().unwrap().remove("manages");
        }
    }
    assert_eq!(listed(&rig, &mira).await, before);
    assert_eq!(before.len(), 2, "{general}");
    // And a conversation is written on it.
    let to = json!({ "to": ["@ada"], "text": "after the upgrade" });
    let (status, written) = rig.call(Method::POST, "/api/inbox/conversations", Some(&mira), Some(to)).await;
    assert_eq!(status, StatusCode::OK, "{written}");
    assert_eq!(inbox(&rig, &rig.token, false).await.1, 1);
}
