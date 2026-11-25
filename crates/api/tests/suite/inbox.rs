//! The inbox (`talk::inbox`): the box of things addressed to somebody,
//! by the routes that read and change it.
//!
//! One journey in one factory, with its owner, two people, keys of
//! theirs that hold less, and an agent the factory does not start:
//! somebody named in a channel finds a line in their box and is told so
//! as it is said, and nobody else is, by any route or stream; the line
//! is read, read by its place, put away and raised again; a message of
//! a channel kept to its members names only those who read it; a line
//! of a channel that was kept from its owner since is handed with none
//! of its words; a line goes with its channel and a box with its agent;
//! and a copy of the factory keeps what was in each box. Then a factory
//! as a program from before left it, which starts on this one.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, asked, key, person, reads_none, say, signed_in};
use crate::snapshots::{Shared, taken};

/// What `token`'s box holds: its lines, and how many wait unread.
async fn inbox(rig: &Rig, token: &str, all: bool) -> (Vec<Value>, u64) {
    let path = if all { "/api/inbox?all=true" } else { "/api/inbox" };
    let (status, found) = rig.call(Method::GET, path, Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    (found["lines"].as_array().unwrap().clone(), found["unread"].as_u64().unwrap())
}

/// Asks something of a box as `token`, which is to go well: the answer.
async fn does(rig: &Rig, token: &str, path: &str, body: Value) -> Value {
    let (status, answer) = rig.call(Method::POST, path, Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{path}: {answer}");
    answer
}

/// The words of each line that says any, as they were handed.
fn words(lines: &[Value]) -> Vec<&str> {
    lines.iter().filter_map(|line| line["said"]["body"].as_str()).collect()
}

/// A channel `token` makes: its id.
async fn channel(rig: &Rig, token: &str, asked: Value) -> String {
    let (status, made) = rig.call(Method::POST, "/api/channels", Some(token), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    made["id"].as_str().unwrap().to_string()
}

/// How many events there have been since `mark`, of any topic.
async fn since(rig: &Rig, mark: u64) -> Vec<String> {
    let query = journal::EventQuery { topics: "**".into(), after: mark, limit: 10_000, ..Default::default() };
    rig.said(query).await.into_iter().map(|event| event.topic).collect()
}

#[tokio::test]
async fn somebody_named_in_a_channel_finds_it_in_their_inbox_which_nobody_else_reads_hears_or_counts() {
    let shared = Shared::new();
    let registry = shared.registry();
    let rig = Rig::with("api-inbox", move |config, _| {
        config.profile = Some("prod".into());
        config.registry = Some(registry);
    })
    .await;
    let owned = Rig::owned(rig).await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell) = (person(rig, "Mira").await, person(rig, "Nell").await);
    // Keys that hold less: one of Nell's that reads channels and no box,
    // one of hers that reads her box and marks nothing in it, and one of
    // the owner's that manages the factory and reads no box.
    let narrow = key(rig, &nell.token, &["channels"]).await;
    let reads = key(rig, &nell.token, &["inbox:read", "channels:read"]).await;
    let manages = key(rig, &rig.token, &["channels", "admin", "private", "agents", "projects"]).await;
    let general = channel(rig, &mira.token, json!({ "name": "general" })).await;
    let ops = channel(rig, &mira.token, json!({ "name": "ops" })).await;
    let (mut to_nell, mut to_mira, mut to_owner) =
        (Told::of(rig, &nell.token).await, Told::of(rig, &mira.token).await, Told::of(rig, &rig.token).await);
    let (mut to_narrow, mut to_reads, mut to_manages) =
        (Told::of(rig, &narrow).await, Told::of(rig, &reads).await, Told::of(rig, &manages).await);

    // 1: what names nobody is what it always was, one thing said and
    // nothing written beside it.
    let mark = rig.mark();
    say(rig, &mira.token, &general, json!({ "body": "nobody is named, not user@example.com nor @nobody" }))
        .await;
    assert_eq!(since(rig, mark).await, ["channel.message"]);
    assert_eq!(inbox(rig, &nell.token, true).await, (Vec::new(), 0));

    // 2: named, Nell has a line in her box, with the words as she may
    // read them, where it was said and by whom; she is not in the
    // channel, and reads it as anybody may. Whoever said it has none,
    // though it names them too.
    let mark = rig.mark();
    let first = say(rig, &mira.token, &general, json!({ "body": "@nell @mira the quince is in" })).await;
    assert_eq!(since(rig, mark).await, ["channel.message", "inbox.told"]);
    let (lines, unread) = inbox(rig, &nell.token, false).await;
    assert_eq!((lines.len(), unread), (1, 1), "{lines:?}");
    let line = &lines[0];
    let id = line["id"].as_str().unwrap().to_string();
    assert_eq!(
        (&line["kind"], &line["about"], &line["place"]),
        (&json!("named"), &json!(first), &json!(general))
    );
    assert_eq!(
        (&line["owner"], &line["from"], &line["why"]),
        (&json!(nell.actor), &json!(mira.actor), &json!("named"))
    );
    assert_eq!((&line["read"], &line["away"], &line["count"]), (&Value::Null, &Value::Null, &json!(1)));
    let said = &line["said"];
    assert_eq!(
        (&said["body"], &said["author_name"]),
        (&json!("@nell @mira the quince is in"), &json!("Mira"))
    );
    assert_eq!(
        (&said["channel_name"], &said["thread"], said.get("channel_kept")),
        (&json!("general"), &Value::Null, None)
    );
    assert_eq!(inbox(rig, &mira.token, true).await, (Vec::new(), 0));

    // 3: nobody else reads it, by any route: the factory's owner and a
    // key that manages the factory read their own box, which is empty;
    // a key of Nell's that reads no box is refused for the scope; and
    // her line is, to each of them, one that is not there, in the words
    // one that never was is refused in.
    assert_eq!(inbox(rig, &rig.token, true).await, (Vec::new(), 0));
    let (status, refused) = asked(rig, &manages, Method::GET, "/api/inbox", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused.contains("needs `inbox:read`"), "{refused}");
    let (status, refused) = asked(rig, &narrow, Method::GET, "/api/inbox?all=true", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    reads_none("by a key that reads no box", &refused, &["quince", &id]);
    for (who, token) in [("another person", &mira.token), ("the factory's owner", &rig.token)] {
        for does in ["read", "away"] {
            let hers = asked(rig, token, Method::POST, &format!("/api/inbox/{id}/{does}"), None).await;
            let never = asked(rig, token, Method::POST, &format!("/api/inbox/inb_never/{does}"), None).await;
            assert_eq!(hers.0, StatusCode::NOT_FOUND, "{who}: {}", hers.1);
            assert_eq!((hers.0, hers.1.replace(&id, "inb_never")), never, "{who}, asking to {does} it");
        }
        // Nor is it read by saying that everything, or everything of
        // its place, was read: that is of the box of whoever says so.
        let all = does_as(rig, token, json!({})).await;
        let there = does_as(rig, token, json!({ "place": general })).await;
        assert_eq!((&all["read"], &there["read"]), (&json!(0), &json!(0)), "{who}");
    }
    for (who, token) in
        [("a key of hers that marks nothing", &reads), ("a key of hers that reads no box", &narrow)]
    {
        for path in
            [format!("/api/inbox/{id}/read"), format!("/api/inbox/{id}/away"), "/api/inbox/read".into()]
        {
            let (status, refused) = asked(rig, token, Method::POST, &path, Some(json!({}))).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{who}: {refused}");
            assert!(refused.contains("needs `inbox:write`"), "{who}: {refused}");
        }
    }
    assert_eq!(inbox(rig, &nell.token, false).await.1, 1, "nothing anybody else asked changed it");
    // Her key that reads her box reads it, words and all: it reads channels too.
    assert_eq!(words(&inbox(rig, &reads, false).await.0), ["@nell @mira the quince is in"]);

    // 4: and nobody else hears of it. Nell and her key that reads her
    // box were told as it was said; everybody else heard what was said
    // in the channel and nothing of a box.
    say(rig, &mira.token, &general, json!({ "body": "marker-one" })).await;
    for (who, told) in [("Nell", &mut to_nell), ("her key that reads her box", &mut to_reads)] {
        told.until("marker-one").await;
        assert!(told.heard.contains("inbox.told") && told.heard.contains(&id), "{who}: {}", told.heard);
        reads_none(&format!("in what {who} is told of the box"), &of_a_box(&told.heard), &["quince"]);
    }
    for (who, told) in [
        ("Mira, who said it", &mut to_mira),
        ("the factory's owner", &mut to_owner),
        ("a key of Nell's that reads no box", &mut to_narrow),
        ("a key that manages the factory", &mut to_manages),
    ] {
        told.until("marker-one").await;
        assert!(told.heard.contains("the quince is in"), "{who} hears what is said in the channel");
        reads_none(&format!("on what {who} is told"), &told.heard, &["inbox.", &id]);
    }
    // Nor is it found among what happened lately, by whoever may ask that.
    for (who, token) in [("the owner", &rig.token), ("Mira", &mira.token), ("the managing key", &manages)] {
        let (_, recent) = asked(rig, token, Method::GET, "/api/events/recent?limit=1000", None).await;
        reads_none(&format!("by {who}, in what happened lately"), &recent, &["inbox.", &id]);
    }
    let (_, recent) = asked(rig, &nell.token, Method::GET, "/api/events/recent?limit=1000", None).await;
    assert!(recent.contains("inbox.told"), "Nell reads back what she was told: {recent}");

    // 5: read, it waits no longer, and she is told so; the same asked
    // again changes nothing and says nothing.
    let mark = rig.mark();
    let read = does(rig, &nell.token, &format!("/api/inbox/{id}/read"), json!({})).await;
    assert!(read["line"]["read"].is_i64() && read["unread"] == 0, "{read}");
    assert_eq!(does(rig, &nell.token, &format!("/api/inbox/{id}/read"), json!({})).await, read);
    assert_eq!(since(rig, mark).await, ["inbox.read"]);
    let event = rig.event("inbox.read", |event| event.seq > mark).await;
    assert_eq!(event.data, json!({ "owner": nell.actor, "lines": [id], "place": general, "unread": 0 }));

    // 6: by its place. Two more in the one channel, one of them in a
    // thread, and one in another; every line of the first channel is
    // read in one asking, and then every line there is.
    say(rig, &mira.token, &general, json!({ "body": "@nell greengage", "thread": first })).await;
    say(rig, &mira.token, &general, json!({ "body": "@nell damson" })).await;
    say(rig, &mira.token, &ops, json!({ "body": "@nell sloe" })).await;
    let (lines, unread) = inbox(rig, &nell.token, false).await;
    assert_eq!(
        words(&lines),
        ["@nell sloe", "@nell damson", "@nell greengage", "@nell @mira the quince is in"]
    );
    assert_eq!(
        (unread, &lines[2]["said"]["thread"], &lines[0]["said"]["channel_name"]),
        (3, &json!(first), &json!("ops"))
    );
    let read = does(rig, &nell.token, "/api/inbox/read", json!({ "place": general })).await;
    assert_eq!(read, json!({ "read": 2, "unread": 1 }));
    assert_eq!(does(rig, &nell.token, "/api/inbox/read", json!({ "place": general })).await["read"], 0);
    let (status, refused) =
        asked(rig, &nell.token, Method::POST, "/api/inbox/read", Some(json!({ "place": 4 }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert_eq!(does(rig, &nell.token, "/api/inbox/read", json!({})).await, json!({ "read": 1, "unread": 0 }));

    // 7: put away, it is listed only with everything; and raised again
    // by something else telling the same thing, it is back, unread and
    // counted, one line still.
    let sloe = lines[0]["id"].as_str().unwrap().to_string();
    let away = does(rig, &nell.token, &format!("/api/inbox/{sloe}/away"), json!({})).await;
    assert!(away["line"]["away"].is_i64(), "{away}");
    assert_eq!(words(&inbox(rig, &nell.token, false).await.0).len(), 3);
    assert_eq!(inbox(rig, &nell.token, true).await.0.len(), 4);
    let keeper = &rig.node().app.studio.channels.inbox;
    let build = |said_by: &str| studio::inbox::Tell {
        owner: nell.actor.clone(),
        kind: studio::inbox::Kind::Told,
        about: "build".into(),
        place: general.clone(),
        from: "bot:factory".into(),
        why: "told".into(),
        says: Some(format!("the build is green ({said_by})")),
        said_by: Some(said_by.into()),
        parts: None,
    };
    let told = keeper.tell(build("run_1/tell")).await.unwrap().unwrap();
    assert_eq!(keeper.tell(build("run_1/tell")).await.unwrap(), None, "the same telling done twice");
    does(rig, &nell.token, &format!("/api/inbox/{}/away", told.id), json!({})).await;
    assert_eq!(inbox(rig, &nell.token, false).await.1, 0);
    let mark = rig.mark();
    keeper.tell(build("run_2/tell")).await.unwrap().unwrap();
    let (lines, unread) = inbox(rig, &nell.token, false).await;
    assert_eq!((unread, &lines[0]["id"], &lines[0]["count"]), (1, &json!(told.id), &json!(2)));
    assert_eq!(
        (&lines[0]["says"], &lines[0]["read"]),
        (&json!("the build is green (run_2/tell)"), &Value::Null)
    );
    assert_eq!((lines.len(), inbox(rig, &nell.token, true).await.0.len()), (4, 5));
    assert_eq!(since(rig, mark).await, ["inbox.told"]);
    does(rig, &nell.token, "/api/inbox/read", json!({})).await;

    // 8: a channel kept to its members names only who reads it. Nell is
    // not in this one: no line, nothing said. Added, she is named like
    // anybody. Out of it again, the line she had is passed over, as the
    // channel is to her one that is not there: not listed, not counted,
    // not found by its id, and nothing said of it; and back in it, the
    // line is as it was.
    let review = channel(rig, &mira.token, json!({ "name": "Medlar review", "kept": true })).await;
    let mark = rig.mark();
    say(rig, &mira.token, &review, json!({ "body": "@nell bullace" })).await;
    assert_eq!(since(rig, mark).await, ["channel.message"]);
    let add = async |to: &str| {
        does(rig, &mira.token, &format!("/api/channels/{to}/members"), json!({ "actor": nell.actor })).await
    };
    add(&review).await;
    say(rig, &mira.token, &review, json!({ "body": "@nell mirabelle" })).await;
    let (lines, unread) = inbox(rig, &nell.token, false).await;
    assert_eq!((unread, &lines[0]["said"]["body"]), (1, &json!("@nell mirabelle")));
    assert_eq!(
        (&lines[0]["said"]["channel_title"], &lines[0]["said"]["channel_kept"]),
        (&json!("Medlar review"), &json!(true))
    );
    let of_review = lines[0]["id"].as_str().unwrap().to_string();
    does(rig, &nell.token, &format!("/api/channels/{review}/leave"), json!({})).await;
    let not_there = async |line: &str, words: [&str; 2], why: &str| {
        let (lines, unread) = inbox(rig, &nell.token, true).await;
        reads_none(why, &json!(lines).to_string(), &[line, words[0], words[1]]);
        for does in ["read", "away"] {
            let hers =
                asked(rig, &nell.token, Method::POST, &format!("/api/inbox/{line}/{does}"), None).await;
            let never =
                asked(rig, &nell.token, Method::POST, &format!("/api/inbox/inb_never/{does}"), None).await;
            assert_eq!((hers.0, hers.1.replace(line, "inb_never")), never, "{why}, asking to {does} it");
        }
        unread
    };
    let mark = rig.mark();
    assert_eq!(not_there(&of_review, ["mirabelle", "Medlar"], "in her box, of a channel she left").await, 0);
    assert_eq!(does(rig, &nell.token, "/api/inbox/read", json!({ "place": review })).await["read"], 0);
    assert_eq!(does(rig, &nell.token, "/api/inbox/read", json!({})).await, json!({ "read": 0, "unread": 0 }));
    let said: Vec<String> =
        since(rig, mark).await.into_iter().filter(|topic| topic.starts_with("inbox.")).collect();
    assert_eq!(said, Vec::<String>::new(), "nothing is said of a line that is passed over");
    add(&review).await;
    let (lines, unread) = inbox(rig, &nell.token, false).await;
    assert_eq!(
        (unread, &lines[0]["id"], &lines[0]["said"]["body"]),
        (1, &json!(of_review), &json!("@nell mirabelle"))
    );

    // 9: and an open channel made private without her: the line she had
    // of it is gone from her list and her count, and what is told of her
    // box from then counts it no longer; added to the channel, she has
    // the line again, unread as it was.
    let plans = channel(rig, &mira.token, json!({ "name": "plans" })).await;
    say(rig, &mira.token, &plans, json!({ "body": "@nell the whitebeam ledger" })).await;
    let (lines, unread) = inbox(rig, &nell.token, false).await;
    assert_eq!((words(&lines)[0], unread), ("@nell the whitebeam ledger", 2));
    let of_plans = lines[0]["id"].as_str().unwrap().to_string();
    does(rig, &mira.token, &format!("/api/channels/{plans}/keep"), json!({})).await;
    assert_eq!(
        not_there(&of_plans, ["whitebeam", &plans], "in her box, of a channel kept from her since").await,
        1
    );
    let mark = rig.mark();
    say(rig, &mira.token, &general, json!({ "body": "@nell the rowan is in" })).await;
    let told = rig.event("inbox.told", |event| event.seq > mark).await;
    assert_eq!((&told.data["owner"], &told.data["unread"]), (&json!(nell.actor), &json!(2)));
    add(&plans).await;
    let (lines, unread) = inbox(rig, &nell.token, false).await;
    assert_eq!(unread, 3, "{lines:?}");
    let back = lines.iter().find(|line| line["id"] == json!(of_plans)).unwrap();
    assert_eq!(
        (&back["said"]["body"], &back["read"], &back["said"]["channel_title"]),
        (&json!("@nell the whitebeam ledger"), &Value::Null, &json!("plans"))
    );
    does(rig, &nell.token, "/api/inbox/read", json!({ "place": general })).await;
    // What she is told of her box says no word of either, as ever.
    say(rig, &mira.token, &general, json!({ "body": "marker-two" })).await;
    to_nell.until("marker-two").await;
    reads_none(
        "in what Nell is told of her box",
        &of_a_box(&to_nell.heard),
        &["mirabelle", "whitebeam", "Medlar", "damson"],
    );

    // 10: an agent the factory does not start is told the same way, by
    // whoever hears what is said for agents: named where it may read,
    // it finds the message in its own box, which it reads as itself.
    let invited = does(rig, &rig.token, "/api/agent/invite", json!({ "name": "scout" })).await;
    let (status, joined) = rig
        .call(Method::POST, "/api/agent/join", None, Some(json!({ "invitation": invited["invitation"] })))
        .await;
    assert_eq!(status, StatusCode::OK, "{joined}");
    let scout = joined["token"].as_str().unwrap().to_string();
    say(rig, &mira.token, &review, json!({ "body": "@scout not for you" })).await;
    let gate = say(rig, &mira.token, &general, json!({ "body": "@scout hang the gate" })).await;
    rig.event("inbox.told", |event| {
        event.data["owner"] == "agent:scout" && event.data["about"] == json!(gate)
    })
    .await;
    let (lines, unread) = inbox(rig, &scout, false).await;
    assert_eq!((words(&lines), unread), (vec!["@scout hang the gate"], 1), "{lines:?}");
    assert_eq!((&lines[0]["owner"], &lines[0]["from"]), (&json!("agent:scout"), &json!(mira.actor)));
    let read =
        does(rig, &scout, &format!("/api/inbox/{}/read", lines[0]["id"].as_str().unwrap()), json!({})).await;
    assert_eq!(read["unread"], 0);
    // Whoever invited it reads nothing of its box, nor does anybody.
    for token in [&rig.token, &mira.token, &nell.token] {
        let (lines, _) = inbox(rig, token, true).await;
        assert!(lines.iter().all(|line| line["owner"] != "agent:scout"), "{lines:?}");
    }

    // 11: a copy of the factory keeps what was in each box, as it was.
    let to = shared.snapshot("copy");
    taken(rig, &to).await;
    let copy = shared.copy("dev1", &to).await;
    let nell_there = signed_in(&copy, "Nell").await;
    assert_eq!(inbox(&copy, &nell_there, true).await, inbox(rig, &nell.token, true).await);
    assert_eq!(inbox(&copy, &copy.token, true).await, (Vec::new(), 0));

    // 12: a line goes with the channel it opens, its owner told which
    // went and what then waits; and a box with the agent that is taken
    // away, so that one invited under its name later finds nothing.
    let mark = rig.mark();
    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/channels/{plans}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let event = rig.event("inbox.away", |event| event.seq > mark).await;
    assert_eq!(
        (&event.data["owner"], &event.data["place"], &event.data["unread"]),
        (&json!(nell.actor), &json!(plans), &json!(1))
    );
    let (lines, unread) = inbox(rig, &nell.token, true).await;
    assert!(unread == 1 && lines.iter().all(|line| line["place"] != json!(plans)), "{lines:?}");
    say(rig, &mira.token, &general, json!({ "body": "@scout and the latch" })).await;
    rig.event("inbox.told", |event| event.seq > mark && event.data["owner"] == "agent:scout").await;
    does(rig, &rig.token, "/api/agents/scout/remove", json!({})).await;
    let again = does(rig, &rig.token, "/api/agent/invite", json!({ "name": "scout" })).await;
    let (_, joined) = rig
        .call(Method::POST, "/api/agent/join", None, Some(json!({ "invitation": again["invitation"] })))
        .await;
    assert_eq!(inbox(rig, joined["token"].as_str().unwrap(), true).await, (Vec::new(), 0));
    // The owner heard none of it, from first to last.
    say(rig, &mira.token, &general, json!({ "body": "marker-three" })).await;
    to_owner.until("marker-three").await;
    reads_none("on what the owner is told", &to_owner.heard, &["inbox."]);
}

/// Says as `token` that lines were read, which is to go well.
async fn does_as(rig: &Rig, token: &str, body: Value) -> Value {
    does(rig, token, "/api/inbox/read", body).await
}

/// The frames of a stream that are of somebody's box.
fn of_a_box(heard: &str) -> String {
    heard.split("\n\n").filter(|frame| frame.contains("inbox.")).collect::<Vec<_>>().join("\n\n")
}

#[tokio::test]
async fn a_factory_from_before_starts_on_this_program_with_a_box_for_everybody_and_nothing_in_any() {
    let mut rig = Rig::new().await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let mira = person(&rig, "Mira").await;
    let general = channel(&rig, &mira.token, json!({ "name": "general" })).await;
    say(&rig, &mira.token, &general, json!({ "body": "@ada said before there was a box" })).await;
    rig.stop().await;
    // As a program from before left it: no table of lines, and nothing
    // that says its tables were ever made.
    let file = rig.dir.join("data/journal.db");
    let database = rusqlite::Connection::open(&file).unwrap();
    // What was said, by topic, but for what every start says: of
    // itself, and that what is searched was read again, which is said
    // whenever that is done and may be before this looks or after.
    let events = |database: &rusqlite::Connection| -> Vec<(String, i64)> {
        let mut said = database
            .prepare(
                "SELECT topic, count(*) FROM events WHERE topic NOT LIKE 'system.%' AND topic != 'search.indexed'
                 GROUP BY topic ORDER BY topic",
            )
            .unwrap();
        let rows = said.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        rows.collect::<rusqlite::Result<_>>().unwrap()
    };
    database
        .execute_batch(
            "DROP TABLE inbox;
             DELETE FROM schema_versions WHERE namespace = 'inbox';
             DELETE FROM events WHERE topic LIKE 'inbox.%';",
        )
        .unwrap();
    let before = events(&database);
    drop(database);
    rig.start().await;

    // It starts, and the start says nothing of anybody's box and puts
    // nothing in one: what was said before is not gone back over.
    let database = rusqlite::Connection::open(&file).unwrap();
    assert_eq!(events(&database), before);
    let made: i64 = database
        .query_row("SELECT version FROM schema_versions WHERE namespace = 'inbox'", [], |row| row.get(0))
        .unwrap();
    // Made by every step this program has for it.
    let steps = studio::copy::KNOWS.iter().find(|(name, _)| *name == "inbox").map(|(_, steps)| *steps as i64);
    assert_eq!(
        (
            Some(made),
            database.query_row("SELECT count(*) FROM inbox", [], |row| row.get::<_, i64>(0)).unwrap()
        ),
        (steps, 0)
    );
    drop(database);
    assert_eq!(inbox(&rig, &rig.token, true).await, (Vec::new(), 0));
    // The owner signed in before there was an inbox reads theirs with
    // the session they had, and what names them from here is in it.
    let mira = signed_in(&rig, "Mira").await;
    say(&rig, &mira, &general, json!({ "body": "@ada and after" })).await;
    let (lines, unread) = inbox(&rig, &rig.token, false).await;
    assert_eq!((words(&lines), unread), (vec!["@ada and after"], 1));
}
