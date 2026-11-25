//! A channel kept to its members: read by nobody else, by any way of
//! reading there is (`talk::kept`).
//!
//! One journey, numbered by the ways a channel's words are read as the
//! plan of this work lists them: somebody who is in the channel says a
//! marked word there, attaches a file and answers in a thread; then
//! somebody who is not, somebody who manages the factory and holds
//! nothing private, and a key of the member's own that does not hold the
//! channels' scope each try every way there is, and read the word, what
//! the channel is called, its id and who is in it nowhere. Each refusal
//! is what a channel that is not there is refused with, word for word.
//! Then the other side: who does read it, somebody added and somebody
//! who left, and a copy of the factory.
//!
//! The ways a channel comes to be kept, and stops being, are the door's
//! (`kept_door.rs`).
//!
//! And two tests read the source: for each place the daemon reads a
//! channel for itself, asking nobody (`Channels::daemon`), and for each
//! place this crate reads or follows what was said, each on a list with
//! why nothing of a kept channel leaves the daemon by it.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::snapshots::{Shared, taken};
use crate::told::{held_to, lines_of};
use crate::{Rig, soon};

mod listed;
use listed::{READS_AS_THE_DAEMON, READS_WHAT_WAS_SAID};

/// Somebody with an account: their session and who they are.
pub(crate) struct Person {
    pub(crate) token: String,
    pub(crate) actor: String,
}

/// Registers `name`, invited by the owner.
pub(crate) async fn person(rig: &Rig, name: &str) -> Person {
    let email = format!("{}@example.com", name.to_lowercase());
    let code =
        rig.send("/api/invites", json!({ "email": email })).await["code"].as_str().unwrap().to_string();
    let body = json!({ "email": email, "name": name, "password": "correct horse", "invite": code });
    let (status, made) = rig.call(Method::POST, "/api/auth/register", None, Some(body)).await;
    assert_eq!((status, made["account"]["role"].as_str()), (StatusCode::OK, Some("member")), "{made}");
    Person {
        token: made["token"].as_str().unwrap().to_string(),
        actor: format!("user:{}", made["account"]["id"].as_str().unwrap()),
    }
}

/// Signs `name` in to a factory that has their account: their session.
pub(crate) async fn signed_in(rig: &Rig, name: &str) -> String {
    let body =
        json!({ "email": format!("{}@example.com", name.to_lowercase()), "password": "correct horse" });
    let (status, signed) = rig.call(Method::POST, "/api/auth/login", None, Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    signed["token"].as_str().unwrap().to_string()
}

/// A key `maker` makes, holding `scopes`.
pub(crate) async fn key(rig: &Rig, maker: &str, scopes: &[&str]) -> String {
    let asked = json!({ "name": "key", "scopes": scopes });
    let (status, made) = rig.call(Method::POST, "/api/credentials", Some(maker), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    made["token"].as_str().unwrap().to_string()
}

/// What `token` is answered, as it was sent: how it went and every word.
pub(crate) async fn asked(
    rig: &Rig,
    token: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, String) {
    let (status, answer) = rig.call(method, path, Some(token), body).await;
    (status, answer.to_string())
}

/// What somebody is told as it is said: their stream, and what has come
/// down it.
pub(crate) struct Told {
    stream: reqwest::Response,
    pub(crate) heard: String,
}

impl Told {
    pub(crate) async fn of(rig: &Rig, token: &str) -> Told {
        let asked = reqwest::Client::new().get(format!("{}/api/events", rig.url)).bearer_auth(token);
        let stream = asked.send().await.unwrap();
        assert_eq!(stream.status(), StatusCode::OK);
        Told { stream, heard: String::new() }
    }

    /// Reads on until `words` have come: everything said before them has
    /// come by then, or was not for whoever this is.
    pub(crate) async fn until(&mut self, words: &str) {
        let from = self.heard.len();
        soon(&format!("`{words}` to be told"), async {
            while !self.heard[from..].contains(words) {
                let more = self.stream.chunk().await.unwrap().expect("the stream goes on");
                self.heard.push_str(&String::from_utf8_lossy(&more));
            }
        })
        .await;
    }
}

/// Puts a file in a channel for a message that is about to be said: its id.
async fn attach(rig: &Rig, token: &str, channel: &str, name: &str, holds: &str) -> String {
    let to = format!("{}/api/channels/{channel}/attachments?name={name}", rig.url);
    let sent = reqwest::Client::new().post(to).bearer_auth(token).body(holds.to_string());
    let kept: Value = sent.send().await.unwrap().json().await.unwrap();
    kept["id"].as_str().unwrap_or_else(|| panic!("the file is kept: {kept}")).to_string()
}

/// Says something in a channel as `token`: the message's id.
pub(crate) async fn say(rig: &Rig, token: &str, channel: &str, said: Value) -> String {
    let (status, message) =
        rig.call(Method::POST, &format!("/api/channels/{channel}/messages"), Some(token), Some(said)).await;
    assert_eq!(status, StatusCode::OK, "{message}");
    message["id"].as_str().unwrap().to_string()
}

/// What is the kept channel's and nobody else's to read: what it is
/// called, what it is for, and what was said and attached there.
const WORDS: [&str; 7] = ["Quince", "greengage", "medlar", "damson", "sloe", "bullace", "mirabelle"];

/// Fails where `answer` holds any of `marked`.
#[track_caller]
pub(crate) fn reads_none(of: &str, answer: &str, marked: &[&str]) {
    for one in marked {
        assert!(!answer.contains(one), "`{one}` is read {of}: {answer}");
    }
}

/// A text that listens: a handler that hears what is said in every
/// project's channels and says it on, and one that says what a step
/// that waits for something said was handed.
const EARS: &str = r#"
blueprint: ears
on:
  heard:
    event: channel.message
    scope: fleet
    steps:
      again: { do: emit, with: { topic: ears.heard, body: "${event.data.body}", where: "${event.data.channel}" } }
"#;
const WAITS: &str = r#"
blueprint: waits
steps:
  hold: { do: wait, with: { event: channel.message, scope: fleet } }
  tell: { do: emit, with: { topic: waits.heard, body: "${steps.hold.out.event.data.body}" } }
"#;

/// A text that tries what a blueprint can do in a channel, each step let fail.
fn pries(channel: &str) -> String {
    format!(
        r#"
blueprint: pries
steps:
  sure: {{ do: channel.ensure, with: {{ name: "Quince review" }} }}
  post: {{ needs: [sure], do: channel.post, allow_failure: true, with: {{ channel: "{channel}", body: "let me in" }} }}
  join: {{ needs: [post], do: channel.join, allow_failure: true, with: {{ channel: "{channel}", name: spy }} }}
  leave: {{ needs: [join], do: channel.leave, allow_failure: true, with: {{ channel: "{channel}", name: conductor }} }}
  present: {{ needs: [leave], do: agent.present, allow_failure: true, with: {{ channel: "{channel}" }} }}
"#
    )
}

#[tokio::test]
async fn a_channel_kept_to_its_members_is_read_by_them_and_by_nobody_else_by_any_way_of_reading() {
    let shared = Shared::new();
    let registry = shared.registry();
    let rig = Rig::with("api-kept", move |config, _| {
        config.profile = Some("prod".into());
        config.registry = Some(registry);
    })
    .await;
    let owned = Rig::owned(rig).await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    // Mira, who will be in the channel; Nell, who holds what a person is
    // given and is not; somebody who manages the factory with a key that
    // holds nothing private; and two keys of Mira's own.
    let (mira, nell) = (person(rig, "Mira").await, person(rig, "Nell").await);
    let mut manages: Vec<&str> = kernel::principal::MEMBER_SCOPES.to_vec();
    manages.push("admin");
    let admin = key(rig, &rig.token, &manages).await;
    let reads = key(rig, &mira.token, &["channels:read"]).await;
    let board = key(rig, &mira.token, &["board"]).await;
    let others: [(&str, &str); 3] = [
        ("somebody who is not in it", &nell.token),
        ("an administrator", &admin),
        ("a member's key", &board),
    ];

    // Everybody else is listening from before there is anything to hear.
    let (mut to_nell, mut to_admin, mut to_mira) =
        (Told::of(rig, &nell.token).await, Told::of(rig, &admin).await, Told::of(rig, &mira.token).await);
    rig.send("/api/workspaces", json!({ "name": "Front" })).await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    let mira_asks = |method: Method, path: String, body: Option<Value>| {
        let token = mira.token.clone();
        async move {
            let (status, answer) = rig.call(method, &path, Some(&token), body).await;
            assert_eq!(status, StatusCode::OK, "{path}: {answer}");
            answer
        }
    };
    // Nell's own texts listen too: a handler, and a step that waits.
    for (text, wait) in [(EARS, true), (WAITS, false)] {
        let applied = json!({ "source": text, "wait": wait });
        let (status, applied) = rig.call(Method::POST, "/api/apply", Some(&nell.token), Some(applied)).await;
        assert_eq!(status, StatusCode::OK, "{applied}");
    }
    rig.event("step.started", |event| event.source.step.as_deref() == Some("hold")).await;

    // Mira makes the channel, kept, says what it belongs to, and speaks
    // there: a marked word, a file, a page, and an answer in a thread.
    let made = json!({ "name": "Quince review", "topic": "the greengage accounts", "kept": true });
    let kept = mira_asks(Method::POST, "/api/channels".into(), Some(made)).await;
    let channel = kept["id"].as_str().unwrap().to_string();
    assert_eq!(
        (&kept["kept"], &kept["title"], &kept["name"], kept["members"].as_array().unwrap().len()),
        (&json!(true), &json!("Quince review"), &json!(channel), 1),
        "{kept}"
    );
    let front = json!({ "workspace": "front" });
    mira_asks(Method::PUT, format!("/api/channels/{channel}/belongs"), Some(front)).await;
    let first = say(rig, &mira.token, &channel, json!({ "body": "the medlar ledger" })).await;
    let file = attach(rig, &mira.token, &channel, "sloe.txt", "a plan of bullace").await;
    let page = attach(rig, &mira.token, &channel, "page.html", "<p>mirabelle</p>").await;
    say(rig, &mira.token, &channel, json!({ "body": "attached", "attachments": [file, page] })).await;
    let under = say(rig, &mira.token, &channel, json!({ "body": "and the damson", "thread": first })).await;
    // The factory's bot is spoken to there, and answers there.
    say(rig, &mira.token, &channel, json!({ "body": "@conductor help" })).await;
    let of_the_bot =
        |event: &kernel::Event| event.data["channel"] == channel && event.data["author_kind"] == "bot";
    let answered = rig.event("channel.message", of_the_bot).await;
    assert!(answered.data["body"].as_str().unwrap().contains("status"), "{:?}", answered.data);
    // And something is said in the open, which everybody is told: what
    // was said before it has reached whoever it was for by then.
    let open = say(rig, &mira.token, "general", json!({ "body": "the open word" })).await;
    say(rig, &mira.token, "general", json!({ "body": "under it", "thread": open })).await;
    say(rig, &mira.token, "general", json!({ "body": "first-marker" })).await;
    rig.node().app.studio.search.catch_up().await.unwrap();

    // What is its alone: the words, and the ids of everything of it.
    let mut marked: Vec<&str> = WORDS.to_vec();
    let ids = [channel.as_str(), first.as_str(), under.as_str(), file.as_str(), page.as_str()];
    marked.extend(ids);

    // ── Nobody else reads it, by any way ────────────────────────────────
    // Each way that names it, beside the same asked of one that is not
    // there: the two answers are the same, word for word.
    let nowhere = ["ch_nonesuch", "msg_nonesuch", "msg_nonesuch", "att_nonesuch", "att_nonesuch"];
    type Way = (&'static str, Method, &'static str, Option<Value>);
    let ways: Vec<Way> = vec![
        ("2: the channel", Method::GET, "/api/channels/{channel}", None),
        ("3: what was said in it", Method::GET, "/api/channels/{channel}/messages", None),
        (
            "3: a thread of it, by its own path",
            Method::GET,
            "/api/channels/{channel}/messages?thread={first}",
            None,
        ),
        (
            "4: a thread of it, by another channel's path",
            Method::GET,
            "/api/channels/general/messages?thread={first}",
            None,
        ),
        (
            "4: a thread of it, by a message under it",
            Method::GET,
            "/api/channels/general/messages?thread={under}",
            None,
        ),
        (
            "3: what was said before a message of it",
            Method::GET,
            "/api/channels/general/messages?before={first}",
            None,
        ),
        (
            "5: saying something in it",
            Method::POST,
            "/api/channels/{channel}/messages",
            Some(json!({ "body": "hello?" })),
        ),
        (
            "5: saying something under a message of it, elsewhere",
            Method::POST,
            "/api/channels/general/messages",
            Some(json!({ "body": "hello?", "thread": "{first}" })),
        ),
        (
            "5: answering a message of it, elsewhere",
            Method::POST,
            "/api/channels/general/messages",
            Some(json!({ "body": "hello?", "reply_to": "{first}" })),
        ),
        (
            "6: attaching something there",
            Method::POST,
            "/api/channels/{channel}/attachments?name=x.txt",
            Some(json!("x")),
        ),
        ("7: something attached there", Method::GET, "/api/attachments/{file}", None),
        ("8: a page attached there", Method::GET, "/api/attachments/{page}/view", None),
        ("8: something attached there, as a page", Method::GET, "/api/attachments/{file}/view", None),
        (
            "9: adding somebody to it",
            Method::POST,
            "/api/channels/{channel}/members",
            Some(json!({ "actor": nell.actor })),
        ),
        (
            "10: bringing an agent into it",
            Method::POST,
            "/api/channels/{channel}/agents",
            Some(json!({ "name": "spy" })),
        ),
        (
            "11: adding an agent that is to be in it",
            Method::POST,
            "/api/agents",
            Some(json!({ "name": "spy", "channel": "{channel}" })),
        ),
        ("12: taking it away", Method::DELETE, "/api/channels/{channel}", None),
        (
            "12: taking it away whatever posts there",
            Method::DELETE,
            "/api/channels/{channel}?force=true",
            None,
        ),
        ("14: saying it belongs to nothing", Method::PUT, "/api/channels/{channel}/belongs", Some(json!({}))),
        (
            "14: saying what it belongs to",
            Method::PUT,
            "/api/channels/{channel}/belongs",
            Some(json!({ "workspace": "front" })),
        ),
    ];
    let named = ["{channel}", "{first}", "{under}", "{file}", "{page}"];
    let fill = |text: &str, with: &[&str]| {
        named.iter().zip(with).fold(text.to_string(), |text, (name, with)| text.replace(name, with))
    };
    for (who, token) in others {
        for (way, method, path, body) in &ways {
            let ask = |with: &[&str]| {
                let body =
                    body.as_ref().map(|body| serde_json::from_str(&fill(&body.to_string(), with)).unwrap());
                let (path, method) = (fill(path, with), method.clone());
                async move { asked(rig, token, method, &path, body).await }
            };
            let (here, there) = (ask(&ids).await, ask(&nowhere).await);
            reads_none(&format!("by {who}, {way}"), &here.1, &WORDS);
            let plain = |answer: &(StatusCode, String), with: &[&str]| {
                (answer.0, with.iter().fold(answer.1.clone(), |text, one| text.replace(one, "<it>")))
            };
            assert_eq!(plain(&here, &ids), plain(&there, &nowhere), "{who}, {way}");
            // Read before a message that is not of the channel asked is
            // read as from its end; everything else here is refused.
            assert!(here.0 != StatusCode::OK || way.contains("before a message"), "{who}, {way}: {here:?}");
        }
        // 1, 15, 18, 19, 21, 22, 24: what lists, finds or recounts, which
        // leaves it out and says nothing of it.
        let mut lists = vec![
            "/api/channels".to_string(),
            "/api/workspaces".into(),
            "/api/workspaces/front".into(),
            "/api/events/recent?limit=1000".into(),
            "/api/events/recent?limit=1000&topics=channel.*".into(),
            "/api/instances".into(),
            "/api/runs?limit=100".into(),
            "/api/fleet".into(),
            "/api/people".into(),
            "/api/system/speaker".into(),
            "/api/system/speaker/history?limit=200".into(),
        ];
        lists.extend(WORDS.iter().map(|word| format!("/api/search?q={word}")));
        lists.extend(WORDS.iter().map(|word| format!("/api/search?q={word}&kind=message")));
        for path in lists {
            let (_, answer) = asked(rig, token, Method::GET, &path, None).await;
            reads_none(&format!("by {who}, in {path}"), &answer, &marked);
        }
    }
    // 4, 5, for every channel: a thread of an open channel is read in
    // that channel too, and nothing is said under a message of another.
    let elsewhere = format!("/api/channels/{channel}/messages?thread={open}");
    let (status, refused) = rig.call(Method::GET, &elsewhere, Some(&mira.token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    let stray = json!({ "body": "astray", "thread": open });
    let (status, refused) = rig
        .call(Method::POST, &format!("/api/channels/{channel}/messages"), Some(&mira.token), Some(stray))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    let own = rig.read(&format!("/api/channels/general/messages?thread={open}")).await;
    assert_eq!(own["messages"].as_array().unwrap().len(), 2, "{own}");

    // 13: making a channel says the same whether or not a kept one is
    // called that, and what is made is another channel.
    let (status, mine) = rig
        .call(Method::POST, "/api/channels", Some(&nell.token), Some(json!({ "name": "Quince Review" })))
        .await;
    assert_eq!((status, &mine["name"]), (StatusCode::OK, &json!("quince-review")), "{mine}");
    assert!(mine["id"] != json!(channel) && mine.get("kept").is_none(), "{mine}");
    let (status, again) =
        rig.call(Method::POST, "/api/channels", Some(&admin), Some(json!({ "name": "quince review" }))).await;
    assert_eq!(status, StatusCode::CONFLICT, "{again}");
    assert!(again.to_string().contains("already a channel called `quince-review`"), "{again}");

    // 25, 27: the factory's bot is called something else, and leaves and
    // joins every channel it is in under the new name, that one too.
    rig.send("/api/system/speaker", json!({ "name": "herald" })).await;
    let renamed =
        |event: &kernel::Event| event.data["channel"] == channel && event.data["member"] == "bot:herald";
    rig.event("channel.joined", renamed).await;

    // 20, 26, 27, 32: nothing of it was told to anybody else as it was
    // said: not on a stream, not to a handler, not to a step that waited.
    say(rig, &mira.token, "general", json!({ "body": "second-marker" })).await;
    for (who, told) in [("Nell", &mut to_nell), ("an administrator", &mut to_admin)] {
        told.until("second-marker").await;
        reads_none(&format!("on what {who} is told as it is said"), &told.heard, &marked);
        assert!(
            told.heard.contains("the open word") && told.heard.contains("first-marker"),
            "{}",
            told.heard
        );
    }
    to_mira.until("second-marker").await;
    for word in ["medlar", "damson", "channel.created", "bot:herald", "the open word"] {
        assert!(to_mira.heard.contains(word), "Mira is told `{word}`: {}", to_mira.heard);
    }
    rig.event("ears.heard", |event| event.data["body"] == "second-marker").await;
    let handed =
        rig.said(journal::EventQuery { topics: "ears.heard,waits.heard".into(), ..Default::default() }).await;
    let handed: Vec<String> = handed.iter().map(|event| event.data.to_string()).collect();
    reads_none("by a handler or a wait of Nell's", &handed.join("\n"), &marked);
    assert!(handed.iter().any(|said| said.contains("the open word")), "{handed:?}");
    assert_eq!(
        handed.iter().filter(|said| !said.contains("where")).count(),
        1,
        "the wait was handed something: {handed:?}"
    );
    // 22: and no run of hers was set off by anything of it.
    let (_, runs) = asked(rig, &nell.token, Method::GET, "/api/runs?limit=200", None).await;
    reads_none("in the runs of Nell's texts", &runs, &marked);

    // 35, 36, 37, 40: a text of Nell's is not a way in. (It is tried once
    // everything above was read: her own steps say the id she wrote,
    // which is what a refusal echoes and nothing she was told.) Making sure of a
    // channel called that finds her own; saying something there, taking
    // part, leaving and asking which agents are in it are refused as of
    // a channel that is not there.
    let tried = |named: &'static str| {
        let (token, source) =
            (nell.token.clone(), pries(if named == "kept" { &channel } else { "ch_nonesuch" }));
        async move {
            let asked = json!({ "source": source, "wait": true });
            let (status, applied) = rig.call(Method::POST, "/api/apply", Some(&token), Some(asked)).await;
            assert_eq!(status, StatusCode::OK, "{applied}");
            applied["finished"]["steps"].clone()
        }
    };
    let (here, there) = (tried("kept").await, tried("nowhere").await);
    let step = |steps: &Value, id: &str| {
        steps
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["step"] == id)
            .unwrap_or_else(|| panic!("{steps}"))
            .clone()
    };
    assert_eq!(step(&here, "sure")["output"]["id"], mine["id"], "{here}");
    for id in ["post", "join", "leave", "present"] {
        let (here, there) = (step(&here, id), step(&there, id));
        let refused = here["error"].as_str().unwrap_or_else(|| panic!("`{id}` is refused: {here}"));
        reads_none(&format!("by a step of Nell's that would `{id}`"), refused, &WORDS);
        assert_eq!(
            refused.replace(&channel, "<it>"),
            there["error"].as_str().unwrap().replace("ch_nonesuch", "<it>")
        );
        assert!(refused.contains("there is no channel"), "{refused}");
    }

    // ── Those who do read it ────────────────────────────────────────────
    // The owner reads everything, for now: listed, read, searched, told.
    let listed = rig.read("/api/channels").await;
    let its =
        |listed: &Value| listed["channels"].as_array().unwrap().iter().any(|one| one["id"] == json!(channel));
    assert!(its(&listed), "{listed}");
    let said = rig.read(&format!("/api/channels/{channel}/messages")).await.to_string();
    assert!(said.contains("the medlar ledger") && said.contains("sloe.txt"), "{said}");
    let thread = rig.read(&format!("/api/channels/{channel}/messages?thread={under}")).await;
    assert_eq!(thread["messages"].as_array().unwrap().len(), 2, "{thread}");
    let (status, bytes) =
        asked(rig, &rig.token, Method::GET, &format!("/api/attachments/{file}"), None).await;
    assert_eq!((status, bytes.contains("a plan of bullace")), (StatusCode::OK, true), "{bytes}");
    let found = |hits: &Value| {
        hits["hits"].as_array().unwrap().iter().any(|hit| hit["link"]["channel"] == json!(channel))
    };
    assert!(found(&rig.read("/api/search?q=medlar").await));
    assert!(
        rig.read("/api/workspaces/front").await["channels"].as_array().unwrap().contains(&json!(channel))
    );
    assert!(
        rig.read("/api/events/recent?limit=1000&topics=channel.message").await.to_string().contains("medlar")
    );
    // Mira reads it, and finds what she said there; her key that holds
    // the channels' scope reads it as she does, and the one that does
    // not is refused as it is refused any channel.
    for token in [&mira.token, &reads] {
        let (status, said) =
            asked(rig, token, Method::GET, &format!("/api/channels/{channel}/messages"), None).await;
        assert_eq!((status, said.contains("the medlar ledger")), (StatusCode::OK, true), "{said}");
        let (status, listed) = rig.call(Method::GET, "/api/channels", Some(token), None).await;
        assert_eq!((status, its(&listed)), (StatusCode::OK, true), "{listed}");
    }
    let (_, hits) = rig.call(Method::GET, "/api/search?q=medlar", Some(&mira.token), None).await;
    assert!(found(&hits), "{hits}");
    let (status, refused) = asked(rig, &board, Method::GET, &format!("/api/channels/{channel}"), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused.contains("(needs `channels:read`)"), "{refused}");
    // A key that may only read does not write there, as anywhere.
    let (status, refused) = asked(
        rig,
        &reads,
        Method::POST,
        &format!("/api/channels/{channel}/messages"),
        Some(json!({ "body": "x" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // 9: Mira adds Nell, who is told that she joined, reads from the
    // first word, and hears what is said from then.
    let add = json!({ "actor": nell.actor });
    let added = mira_asks(Method::POST, format!("/api/channels/{channel}/members"), Some(add)).await;
    assert_eq!(added["members"].as_array().unwrap().len(), 3, "Mira, the bot and Nell: {added}");
    to_nell.until("channel.joined").await;
    say(rig, &mira.token, &channel, json!({ "body": "welcome to the elderberry" })).await;
    to_nell.until("elderberry").await;
    let (status, said) =
        asked(rig, &nell.token, Method::GET, &format!("/api/channels/{channel}/messages"), None).await;
    assert_eq!((status, said.contains("the medlar ledger")), (StatusCode::OK, true), "{said}");
    let (_, listed) = rig.call(Method::GET, "/api/channels", Some(&nell.token), None).await;
    assert!(its(&listed), "{listed}");
    rig.node().app.studio.search.catch_up().await.unwrap();
    let (_, hits) = rig.call(Method::GET, "/api/search?q=medlar", Some(&nell.token), None).await;
    assert!(found(&hits), "{hits}");
    // Her handler hears from then too, being hers.
    rig.event("ears.heard", |event| event.data["body"] == "welcome to the elderberry").await;

    // Nell leaves. She is told so once, and from then hears and reads
    // nothing: not what is said next, and not what she read a moment ago.
    let channels = &rig.node().app.studio.channels;
    channels.leave(&crate::own(), &channel, &nell.actor).await.unwrap();
    say(rig, &mira.token, &channel, json!({ "body": "now the rowan" })).await;
    say(rig, &mira.token, "general", json!({ "body": "third-marker" })).await;
    let before = to_nell.heard.len();
    to_nell.until("third-marker").await;
    let since = &to_nell.heard[before..];
    assert_eq!(since.matches("channel.left").count(), 1, "{since}");
    reads_none("on what Nell is told once she has left", since, &["rowan"]);
    let (here, there) = (
        asked(rig, &nell.token, Method::GET, &format!("/api/channels/{channel}/messages"), None).await,
        asked(rig, &nell.token, Method::GET, "/api/channels/ch_nonesuch/messages", None).await,
    );
    assert_eq!((here.0, here.1.replace(&channel, "<it>")), (there.0, there.1.replace("ch_nonesuch", "<it>")));
    rig.node().app.studio.search.catch_up().await.unwrap();
    for path in ["/api/channels", "/api/search?q=medlar", "/api/search?q=rowan", "/api/workspaces"] {
        let (_, answer) = asked(rig, &nell.token, Method::GET, path, None).await;
        reads_none(&format!("by Nell, having left, in {path}"), &answer, &marked);
    }
    // What is still hers to be told is that she joined and that she
    // left, which name her; nothing that was said there.
    // (What her own handler said on of what it heard while she was in
    // it is hers, and is said as anything a blueprint says.)
    let (_, recent) = asked(rig, &nell.token, Method::GET, "/api/events/recent?limit=1000", None).await;
    reads_none("by Nell, having left, in what was said lately", &recent, &WORDS);
    reads_none("by Nell, having left, in what was said lately", &recent, &["rowan", first.as_str()]);
    let recent: Value = serde_json::from_str(&recent).unwrap();
    let of_it: Vec<&Value> = recent["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["data"]["channel"] == json!(channel))
        .collect();
    let topics: Vec<&str> = of_it.iter().map(|event| event["topic"].as_str().unwrap()).collect();
    assert_eq!(topics, ["channel.left", "channel.joined"], "{of_it:?}");
    assert!(of_it.iter().all(|event| event["data"]["member"] == json!(nell.actor)), "{of_it:?}");
    rig.event("ears.heard", |event| event.data["body"] == "third-marker").await;
    let heard = rig.said(journal::EventQuery { topics: "ears.heard".into(), ..Default::default() }).await;
    assert!(heard.iter().all(|event| event.data["body"] != "now the rowan"), "her handler heard on");

    // 53: a copy of the factory holds the channel, still kept: Nell is
    // refused there as here, and Mira reads it there.
    let to = shared.snapshot("copy");
    taken(rig, &to).await;
    let copy = shared.copy("dev1", &to).await;
    let (nell_there, mira_there) = (signed_in(&copy, "Nell").await, signed_in(&copy, "Mira").await);
    let (here, there) = (
        asked(&copy, &nell_there, Method::GET, &format!("/api/channels/{channel}/messages"), None).await,
        asked(&copy, &nell_there, Method::GET, "/api/channels/ch_nonesuch/messages", None).await,
    );
    assert_eq!((here.0, here.1.replace(&channel, "<it>")), (there.0, there.1.replace("ch_nonesuch", "<it>")));
    copy.node().app.studio.search.catch_up().await.unwrap();
    for path in ["/api/channels", "/api/search?q=medlar", "/api/workspaces", "/api/events/recent?limit=1000"]
    {
        let (_, answer) = asked(&copy, &nell_there, Method::GET, path, None).await;
        reads_none(&format!("by Nell, in the copy, in {path}"), &answer, &WORDS);
    }
    let (status, said) =
        asked(&copy, &mira_there, Method::GET, &format!("/api/channels/{channel}/messages"), None).await;
    assert_eq!((status, said.contains("the medlar ledger")), (StatusCode::OK, true), "{said}");
    // And what is said there from then is told to its members there.
    let mut to_nell_there = Told::of(&copy, &nell_there).await;
    say(&copy, &mira_there, &channel, json!({ "body": "the copied whitebeam" })).await;
    say(&copy, &mira_there, "general", json!({ "body": "copy-marker" })).await;
    to_nell_there.until("copy-marker").await;
    reads_none("on what Nell is told in the copy", &to_nell_there.heard, &["whitebeam", channel.as_str()]);

    // 28: the workspace it was of is taken away, which is said to
    // whoever reads projects, without it.
    // What it held, the kept channel among it, goes to the workspace
    // the factory always has, and that it moved is said to its members.
    let to = Some(json!({ "to": "ada-s-workspace" }));
    let (status, removed) = rig.call(Method::DELETE, "/api/workspaces/front", Some(&rig.token), to).await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    let gone = rig.event("workspace.removed", |_| true).await;
    reads_none("in what is said of a workspace taken away", &gone.data.to_string(), &marked);
    // 12, 27: and the channel is, by the owner: whoever was in it is
    // told, and nobody else.
    let (status, removed) =
        rig.call(Method::DELETE, &format!("/api/channels/{channel}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    say(rig, &mira.token, "general", json!({ "body": "last-marker" })).await;
    to_mira.until("last-marker").await;
    assert!(to_mira.heard.contains("channel.removed"), "{}", to_mira.heard);
    let before = to_nell.heard.len();
    to_nell.until("last-marker").await;
    reads_none(
        "on what Nell is told of its going",
        &to_nell.heard[before..],
        &["channel.removed", channel.as_str()],
    );
    to_admin.until("last-marker").await;
    reads_none("on what an administrator was ever told", &to_admin.heard, &WORDS);
    // (But for what Nell's own handler said on while she was in it,
    // which is hers and is said as anything a blueprint says.)
    let said = ["rowan", "channel.removed", first.as_str(), under.as_str(), file.as_str(), page.as_str()];
    reads_none("on what an administrator was ever told", &to_admin.heard, &said);
}

/// A text that prints what is said in any channel, and then fails with it.
const PRINTS: &str = r#"
blueprint: prints
on:
  heard:
    event: channel.message
    scope: fleet
    steps:
      said: { do: log, with: { message: "heard ${event.data.body}" } }
      lost: { needs: [said], do: fail, with: { message: "lost ${event.data.body}" } }
"#;

#[tokio::test]
async fn what_is_said_of_a_run_that_something_said_in_a_kept_channel_set_off_is_for_its_members() {
    let rig = Rig::with("api-kept-runs", |_, _| {}).await;
    let owned = Rig::owned(rig).await;
    let rig = &owned;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell) = (person(rig, "Mira").await, person(rig, "Nell").await);
    rig.send("/api/channels", json!({ "name": "general" })).await;
    // Mira's own text hears what is said wherever she may read, prints
    // it and fails with it; she and Nell, who holds what she holds and
    // is in no kept channel, each follow what is said.
    let applied = json!({ "source": PRINTS, "wait": true });
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(&mira.token), Some(applied)).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    let (mut to_nell, mut to_mira) = (Told::of(rig, &nell.token).await, Told::of(rig, &mira.token).await);

    let made = json!({ "name": "Quince review", "kept": true });
    let (status, kept) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(made)).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    let channel = kept["id"].as_str().unwrap().to_string();
    let mut runs = rig.follow("run.finished");
    let ended = async |runs: &mut crate::Followed| {
        soon("the handler's run to end", async {
            while runs.next().await.unwrap().data["flow"] != "on.heard" {}
        })
        .await
    };
    say(rig, &mira.token, &channel, json!({ "body": "the medlar ledger" })).await;
    ended(&mut runs).await;
    // Said where everybody reads, once that run is over: whoever has
    // heard this has been told everything before it that is theirs.
    say(rig, &mira.token, "general", json!({ "body": "last-marker" })).await;
    ended(&mut runs).await;
    say(rig, &mira.token, "general", json!({ "body": "and-after" })).await;

    // Mira is told what her run printed and why it failed. Nell is told
    // neither: what is said of a run takes what the event that set it
    // off took, which is being of the channel.
    to_mira.until("and-after").await;
    for said in ["heard the medlar ledger", "lost the medlar ledger", "lost last-marker"] {
        assert!(to_mira.heard.contains(said), "Mira is told `{said}`: {}", to_mira.heard);
    }
    to_nell.until("and-after").await;
    reads_none("on what Nell is told", &to_nell.heard, &["medlar", "Quince", channel.as_str()]);
    assert!(to_nell.heard.contains("lost last-marker"), "what an open channel set off is hers to hear");
    let (_, recent) = asked(rig, &nell.token, Method::GET, "/api/events/recent?limit=1000", None).await;
    reads_none("by Nell in what was said lately", &recent, &["medlar", "Quince", channel.as_str()]);
    // The run is listed to her without what was said. (Where its event
    // came from is still there, and what it took names the channel by
    // its id: that is not held, and is in `docs/status.md`'s list.)
    let (_, listed) = asked(rig, &nell.token, Method::GET, "/api/runs?limit=100", None).await;
    reads_none("by Nell in the runs there are", &listed, &["medlar", "Quince"]);
}

#[tokio::test]
async fn a_channel_from_before_is_open_as_it_always_was_and_kept_is_said_as_so_or_not_so() {
    // Said as anything but so or not so, no channel is made.
    let mut rig = Rig::new().await;
    let (status, refused) =
        rig.post("/api/channels", json!({ "name": "Quince review", "kept": "yes" })).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "an open channel is not made where a kept one was meant: {refused}"
    );
    assert_eq!(rig.read("/api/channels").await["channels"], json!([]));

    // A channel as a program from before left it: made, spoken in, an
    // agent asked there, and then the daemon stopped and its tables put
    // back as they were before a channel could be kept.
    let made = rig.send("/api/channels", json!({ "name": "general", "topic": "everything" })).await;
    let first = rig.send("/api/channels/general/messages", json!({ "body": "the first word" })).await;
    let under = json!({ "body": "under it", "thread": first["id"] });
    rig.send("/api/channels/general/messages", under).await;
    rig.send("/api/agents", json!({ "name": "helper", "channel": "general" })).await;
    let before = rig.mark();
    rig.send("/api/channels/general/messages", json!({ "body": "@helper are you there?" })).await;
    rig.event("agent.turn.finished", |event| event.seq > before).await;
    let member = json!({ "name": "member", "scopes": kernel::principal::MEMBER_SCOPES });
    let member = rig.send("/api/credentials", member).await["token"].as_str().unwrap().to_string();
    let read = |rig: &Rig| {
        let (url, member) = (rig.url.clone(), member.clone());
        async move {
            let mut answers = Vec::new();
            for path in [
                "/api/channels",
                "/api/channels/general",
                "/api/channels/general/messages",
                "/api/agents/helper/turns",
            ] {
                let asked = reqwest::Client::new().get(format!("{url}{path}")).bearer_auth(&member);
                answers.push(asked.send().await.unwrap().text().await.unwrap());
            }
            answers
        }
    };
    let before = read(&rig).await;
    assert!(before[2].contains("the first word") && !before[0].contains("kept"), "{before:?}");
    rig.stop().await;
    let database = rusqlite::Connection::open(rig.dir.join("data/journal.db")).unwrap();
    database
        .execute_batch(
            "DROP TABLE discussions;
             DROP TABLE discussing;
             ALTER TABLE messages DROP COLUMN mark;
             ALTER TABLE channel_members DROP COLUMN manages;
             ALTER TABLE channel_members DROP COLUMN listens;
             DROP TABLE channel_docs;
             ALTER TABLE channels DROP COLUMN archived;
             ALTER TABLE channels DROP COLUMN conversation;
             ALTER TABLE channels DROP COLUMN alone;
             ALTER TABLE channels DROP COLUMN kept;
             ALTER TABLE channels DROP COLUMN title;
             UPDATE schema_versions SET version = 4 WHERE namespace = 'channels';
             ALTER TABLE slots DROP COLUMN owes;
             ALTER TABLE slots DROP COLUMN environment;
             ALTER TABLE turns DROP COLUMN kept;
             ALTER TABLE turns DROP COLUMN refused;
             ALTER TABLE turns DROP COLUMN hidden;
             ALTER TABLE turns DROP COLUMN cost_said;
             UPDATE schema_versions SET version = version - 6 WHERE namespace = 'agents';",
        )
        .unwrap();
    drop(database);
    rig.start().await;

    // It is open, and reads as it did, word for word, to the same key:
    // what was said there, and what its agent was asked and answered;
    // but that a program from before kept no managers, so its maker
    // manages it no longer.
    let before: Vec<String> = before.iter().map(|one| one.replace("\"manages\":true,", "")).collect();
    assert_eq!(read(&rig).await, before);
    assert!(before[3].contains("are you there") && !before[3].contains("kept_from"), "{before:?}");
    let said = rig.send("/api/channels/general/messages", json!({ "body": "and after" })).await;
    assert_eq!(said["channel"], made["id"]);
    let event = rig.event("channel.message", |event| event.data["body"] == "and after").await;
    assert_eq!(event.source.takes, None, "what is said in it takes nothing to hear");
    let kept = rusqlite::Connection::open(rig.dir.join("data/journal.db")).unwrap();
    let row: (i64, Option<String>, i64) = kept
        .query_row("SELECT kept, title, alone FROM channels WHERE name = 'general'", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    assert_eq!(row, (0, None, 0));
}

#[test]
fn where_the_daemon_reads_a_channel_for_itself_is_listed_with_why_nothing_kept_leaves_by_it() {
    let lines = lines_of(&["crates", "xtask", "desktop", "examples"]);
    let wrong = held_to(&lines, &["daemon("], |_| false, READS_AS_THE_DAEMON);
    assert!(
        wrong.is_empty(),
        "these read a channel as the daemon itself (`Channels::daemon`), which asks nobody whether they may \
         read it. A channel may be kept to its members (`talk::kept`): what is read for somebody is read by \
         their leave (`Channels::may_read`, `may_write`, `may_find`), and what the daemon reads for its own \
         work is handed to nobody as it is. Ask as whoever it is for; or, where it is the daemon's own, add \
         the line to `READS_AS_THE_DAEMON` with why nothing of a kept channel leaves the daemon by it.\n{}",
        wrong.join("\n")
    );
}

#[test]
fn an_agents_turns_leave_the_daemon_only_as_whoever_asks_may_be_shown_them() {
    let hands_out = [
        "crates/api/src/",
        "crates/served/src/",
        "crates/folders/src/",
        "crates/cli/src/",
        "crates/gateway/src/",
    ];
    let lines: Vec<_> = lines_of(&["crates"])
        .into_iter()
        .filter(|(file, ..)| hands_out.iter().any(|at| file.starts_with(at)))
        .collect();
    let needles = ["agents.turns(", "agents.turn(", "agents.wait(", "TurnInfo"];
    let wrong = held_to(&lines, &needles, |_| false, &[]);
    assert!(
        wrong.is_empty(),
        "these hand out an agent's turn as it is kept, or wait for one. A turn asked in a channel kept to its \
         members holds what was said there, and is shown whole only to whoever reads that channel \
         (`agents::kept`): what leaves the daemon of an agent's turns leaves through `Agents::turns_for`, \
         which is handed whoever asks and leaves the words out for anybody else.\n{}",
        wrong.join("\n")
    );
}

#[test]
fn where_the_node_reads_what_was_said_is_listed_with_why_nothing_kept_is_handed_on_by_it() {
    // The node, what it serves beside its routes (`served`), and what it
    // asks of its data folder (`folders`).
    let node = ["crates/api/src/", "crates/served/src/", "crates/folders/src/"];
    let lines: Vec<_> = lines_of(&["crates"])
        .into_iter()
        .filter(|(file, ..)| node.iter().any(|at| file.starts_with(at)))
        .collect();
    let needles = [
        "follow_every(",
        ".read_every(",
        ".read_within(",
        ".read_after(",
        "journal.read(",
        "journal.follow(",
    ];
    let wrong = held_to(&lines, &needles, |_| false, READS_WHAT_WAS_SAID);
    assert!(
        wrong.is_empty(),
        "these read or follow what was said, in the crate that hands things to people. An event may be of \
         something kept from whoever would be handed it (a private table's record, a channel kept to its \
         members): what is handed on is asked of `App::may_hear` first, event by event. What follows topics \
         that are nobody's to keep, or hands nothing on, is listed in `READS_WHAT_WAS_SAID` with the reason.\n{}",
        wrong.join("\n")
    );
}
