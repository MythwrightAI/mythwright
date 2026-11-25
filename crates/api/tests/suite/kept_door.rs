//! The door of a channel kept to its members, through the daemon's
//! routes (`talk::door`, `agents::door`, `routes/kept.rs`): one made
//! with who is in it and what it belongs to, an open one kept, a kept
//! one opened again by the factory's owner and nobody else, somebody
//! added and somebody leaving, and whether its agents are asked nowhere
//! else.
//!
//! Each route of the door is tried by somebody who is not in the
//! channel, by somebody who manages the factory and holds nothing
//! private, and by a key of a member's own that holds less: the first
//! two are answered as for a channel that is not there, word for word,
//! and the third in the words its scope is always refused in, which are
//! the same for a channel that is not there.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, asked, key, person, reads_none, say};

/// What is the kept channel's and nobody else's to read.
const WORDS: [&str; 4] = ["Quince", "greengage", "medlar", "damson"];

/// Asks as `token` and is answered well: what was answered.
async fn does(rig: &Rig, token: &str, method: Method, path: &str, body: Option<Value>) -> Value {
    let (status, answer) = rig.call(method, path, Some(token), body).await;
    assert_eq!(status, StatusCode::OK, "{path}: {answer}");
    answer
}

/// What `token` is answered at each way through the door of `channel`,
/// beside the same asked of a channel that is not there: the two are
/// the same, word for word, and none is answered well.
async fn shut_to(rig: &Rig, who: &str, token: &str, channel: &str, someone: &str) {
    type Way = (&'static str, Method, &'static str, Option<Value>);
    let ways: Vec<Way> = vec![
        ("keeping it", Method::POST, "/api/channels/{channel}/keep", None),
        ("opening it again", Method::POST, "/api/channels/{channel}/open", None),
        (
            "opening it under a name",
            Method::POST,
            "/api/channels/{channel}/open",
            Some(json!({ "name": "open-now" })),
        ),
        (
            "saying its agents are asked nowhere else",
            Method::PUT,
            "/api/channels/{channel}/alone",
            Some(json!({ "alone": true })),
        ),
        ("leaving it", Method::POST, "/api/channels/{channel}/leave", None),
        (
            "taking an agent out of it",
            Method::POST,
            "/api/channels/{channel}/leave",
            Some(json!({ "agent": "helper" })),
        ),
        ("joining it", Method::POST, "/api/channels/{channel}/members", Some(json!({ "actor": someone }))),
        (
            "bringing an agent that is there into it",
            Method::POST,
            "/api/channels/{channel}/members",
            Some(json!({ "actor": "agent:helper" })),
        ),
        (
            "trying an agent's program there",
            Method::POST,
            "/api/agents/helper/try",
            Some(json!({ "channel": "{channel}" })),
        ),
    ];
    for (way, method, path, body) in ways {
        let ask = |of: &str| {
            let body = body.as_ref().map(|body| {
                serde_json::from_str::<Value>(&body.to_string().replace("{channel}", of)).unwrap()
            });
            let (path, method) = (path.replace("{channel}", of), method.clone());
            async move { asked(rig, token, method, &path, body).await }
        };
        let (here, there) = (ask(channel).await, ask("ch_nonesuch").await);
        reads_none(&format!("by {who}, {way}"), &here.1, &WORDS);
        assert_eq!(
            (here.0, here.1.replace(channel, "<it>")),
            (there.0, there.1.replace("ch_nonesuch", "<it>")),
            "{who}, {way}"
        );
        assert_ne!(here.0, StatusCode::OK, "{who}, {way}: {here:?}");
    }
}

#[tokio::test]
async fn the_door_of_a_kept_channel_is_its_members_and_answers_everybody_else_as_a_channel_that_is_not_there()
{
    let rig = Rig::owned(Rig::with("api-kept-door", |_, _| {}).await).await;
    let rig = &rig;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell, otto) =
        (person(rig, "Mira").await, person(rig, "Nell").await, person(rig, "Otto").await);
    let mut manages: Vec<&str> = kernel::principal::MEMBER_SCOPES.to_vec();
    manages.push("admin");
    let admin = key(rig, &rig.token, &manages).await;
    let board = key(rig, &mira.token, &["board"]).await;
    let talks = key(rig, &mira.token, &["channels"]).await;
    let mut to_nell = Told::of(rig, &nell.token).await;
    rig.send("/api/workspaces", json!({ "name": "Front" })).await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/agents", json!({ "name": "helper" })).await;
    let listed = |token: &str| {
        let token = token.to_string();
        async move { does(rig, &token, Method::GET, "/api/channels", None).await["channels"].clone() }
    };

    // ── Made ────────────────────────────────────────────────────────────
    // Kept, with somebody in it and what it belongs to, in one asking.
    let made = json!({
        "name": "Quince review", "topic": "the greengage accounts", "kept": true, "workspace": "front",
        "members": [otto.actor, mira.actor, otto.actor],
    });
    let kept = does(rig, &mira.token, Method::POST, "/api/channels", Some(made)).await;
    let channel = kept["id"].as_str().unwrap().to_string();
    let within: Vec<&str> =
        kept["members"].as_array().unwrap().iter().map(|one| one["actor"].as_str().unwrap()).collect();
    assert_eq!(within, [mira.actor.as_str(), otto.actor.as_str()], "{kept}");
    assert_eq!(
        (&kept["kept"], &kept["title"], &kept["name"]),
        (&json!(true), &json!("Quince review"), &json!(channel))
    );
    let front = does(rig, &mira.token, Method::GET, "/api/workspaces/front", None).await;
    assert!(front.to_string().contains(&channel), "it is that workspace's own: {front}");
    // Whoever it was made with reads it from the first, and says so.
    say(rig, &otto.token, &channel, json!({ "body": "the medlar ledger" })).await;
    // What makes no sense is refused, and nothing is made.
    for (members, status) in [
        (json!(["bot:spy"]), StatusCode::BAD_REQUEST),
        (json!(["user:nobody"]), StatusCode::NOT_FOUND),
        (json!(["agent:nobody"]), StatusCode::NOT_FOUND),
        (json!("everybody"), StatusCode::BAD_REQUEST),
    ] {
        let wrong = json!({ "name": "Hiring", "kept": true, "members": members });
        let (answered, refused) =
            rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(wrong)).await;
        assert_eq!(answered, status, "{refused}");
    }
    let alone = json!({ "name": "hiring", "alone": true });
    let (status, refused) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(alone)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert_eq!(listed(&mira.token).await.as_array().unwrap().len(), 2, "the kept one and the open one");

    // ── Nobody else comes through the door ──────────────────────────────
    let mut marked: Vec<&str> = WORDS.to_vec();
    marked.push(&channel);
    for (who, token, someone) in [
        ("somebody who is not in it", &nell.token, &nell.actor),
        ("an administrator", &admin, &nell.actor),
        ("a member's key that holds less", &board, &mira.actor),
    ] {
        shut_to(rig, who, token, &channel, someone).await;
        for path in ["/api/channels", "/api/workspaces", "/api/workspaces/front", "/api/people"] {
            let (_, answer) = asked(rig, token, Method::GET, path, None).await;
            reads_none(&format!("by {who}, in {path}"), &answer, &marked);
        }
    }
    // A member's key that may speak in channels and may not change
    // agents brings none in and takes none out.
    for body in [json!({ "actor": "agent:helper" }), json!({ "agent": "helper" })] {
        let path = if body.get("actor").is_some() { "members" } else { "leave" };
        let (status, refused) = rig
            .call(Method::POST, &format!("/api/channels/{channel}/{path}"), Some(&talks), Some(body))
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    }

    // ── Its agents asked nowhere else ───────────────────────────────────
    let alone = format!("/api/channels/{channel}/alone");
    let held = does(rig, &otto.token, Method::PUT, &alone, Some(json!({ "alone": true }))).await;
    assert_eq!(held["alone"], true, "said by anybody who is in it: {held}");
    let free = does(rig, &mira.token, Method::PUT, &alone, Some(json!({ "alone": false }))).await;
    assert!(free.get("alone").is_none(), "{free}");
    for (path, said) in [(alone.as_str(), json!({})), (alone.as_str(), json!({ "alone": "yes" }))] {
        let (status, refused) = rig.call(Method::PUT, path, Some(&mira.token), Some(said)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    }
    let of_an_open = json!({ "alone": true });
    let (status, refused) =
        rig.call(Method::PUT, "/api/channels/general/alone", Some(&mira.token), Some(of_an_open)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");

    // ── Leaving ──────────────────────────────────────────────────────────
    // Somebody leaves by asking, and reads nothing of it from there;
    // the last person in it does not.
    let leave = format!("/api/channels/{channel}/leave");
    let left = does(rig, &otto.token, Method::POST, &leave, None).await;
    assert_eq!(left, json!({ "channel": channel, "left": otto.actor }));
    shut_to(rig, "somebody who left", &otto.token, &channel, &otto.actor).await;
    let (status, last) = rig.call(Method::POST, &leave, Some(&mira.token), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{last}");
    assert!(last.to_string().contains("nobody else is in it: add somebody first"), "{last}");
    // Added again by somebody who is in it, they read it from the first word.
    let add = format!("/api/channels/{channel}/members");
    does(rig, &mira.token, Method::POST, &add, Some(json!({ "actor": otto.actor }))).await;
    let read = does(rig, &otto.token, Method::GET, &format!("/api/channels/{channel}/messages"), None).await;
    assert!(read.to_string().contains("the medlar ledger"), "{read}");

    // ── Opened again ─────────────────────────────────────────────────────
    // By the factory's owner and by nobody else: somebody who is in it
    // is told so, and it stays as it was.
    let open = format!("/api/channels/{channel}/open");
    let (status, refused) = rig.call(Method::POST, &open, Some(&mira.token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, refused) = rig.call(Method::POST, &open, Some(&talks), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    reads_none("by Nell, in the channels there are", &listed(&nell.token).await.to_string(), &marked);
    // Said by its maker: Nell, saying it, would be in it from then.
    say(rig, &rig.token, "general", json!({ "body": "first-marker" })).await;
    to_nell.until("first-marker").await;
    reads_none("on what Nell was told as it was said", &to_nell.heard, &marked);
    let opened = does(rig, &rig.token, Method::POST, &open, None).await;
    assert_eq!(
        (&opened["name"], opened.get("kept"), opened.get("title")),
        (&json!("quince-review"), None, None)
    );
    // Anybody who may read channels reads it from there, what was said
    // in it while it was kept with the rest, and is told that it is open.
    to_nell.until("channel.kept").await;
    let read = does(rig, &nell.token, Method::GET, "/api/channels/quince-review/messages", None).await;
    assert!(read.to_string().contains("the medlar ledger"), "{read}");
    say(rig, &nell.token, "quince-review", json!({ "body": "and I am here" })).await;
    assert!(listed(&nell.token).await.to_string().contains("quince-review"));

    // ── An open one kept ─────────────────────────────────────────────────
    // By somebody who is in it: Mira is not, joins as anybody joins an
    // open channel, and keeps it.
    // Said by its maker: Nell, saying it, would be in it from then.
    say(rig, &rig.token, "general", json!({ "body": "the open word" })).await;
    let (status, outside) =
        rig.call(Method::POST, "/api/channels/general/keep", Some(&mira.token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{outside}");
    assert!(outside.to_string().contains("is not in #general"), "{outside}");
    does(
        rig,
        &mira.token,
        Method::POST,
        "/api/channels/general/members",
        Some(json!({ "actor": mira.actor })),
    )
    .await;
    let general = does(rig, &mira.token, Method::POST, "/api/channels/general/keep", None).await;
    let id = general["id"].as_str().unwrap().to_string();
    assert_eq!(
        (&general["kept"], &general["title"], &general["name"]),
        (&json!(true), &json!("general"), &json!(id))
    );
    say(rig, &mira.token, &id, json!({ "body": "the damson figures" })).await;
    rig.node().app.studio.search.catch_up().await.unwrap();
    // Whoever is not in it is told that it was kept, and from there
    // reads nothing of it by any way: not what is said in it, and not
    // what was said in it while it was open.
    to_nell.until("channel.kept").await;
    shut_to(rig, "somebody who was reading it while it was open", &nell.token, &id, &nell.actor).await;
    let (status, by_name) =
        asked(rig, &nell.token, Method::GET, "/api/channels/general/messages", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{by_name}");
    for path in ["/api/channels", "/api/search?q=damson", "/api/search?q=open", "/api/workspaces"] {
        let (_, answer) = asked(rig, &nell.token, Method::GET, path, None).await;
        reads_none(&format!("by Nell, in {path}"), &answer, &["damson", "the open word", &id]);
    }
    let found = does(rig, &mira.token, Method::GET, "/api/search?q=damson", None).await;
    assert!(found.to_string().contains("damson"), "its members find what is said there: {found}");
    say(rig, &nell.token, "quince-review", json!({ "body": "second-marker" })).await;
    to_nell.until("second-marker").await;
    reads_none("on what Nell was told once it was kept", &to_nell.heard, &["damson"]);

    // Its name is free, and it is opened again only under one that is.
    let theirs =
        does(rig, &nell.token, Method::POST, "/api/channels", Some(json!({ "name": "general" }))).await;
    assert!(theirs["id"] != json!(id) && theirs.get("kept").is_none(), "{theirs}");
    let open = format!("/api/channels/{id}/open");
    let (status, taken) = rig.call(Method::POST, &open, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{taken}");
    assert!(taken.to_string().contains("there is already a channel called `general`"), "{taken}");
    let named = does(rig, &rig.token, Method::POST, &open, Some(json!({ "name": "Old general" }))).await;
    assert_eq!((&named["name"], named.get("kept")), (&json!("old-general"), None));
    let read = does(rig, &nell.token, Method::GET, "/api/channels/old-general/messages", None).await;
    assert!(read.to_string().contains("the damson figures") && read.to_string().contains("the open word"));
}

#[cfg(unix)]
#[tokio::test]
async fn agents_come_through_the_door_of_a_kept_channel_only_as_they_may_be_in_one() {
    use crate::kept_agents::holder;
    let rig = Rig::with("api-kept-door-agents", |config, dir| {
        config.walls = Some(api::Walls::Held(holder(dir)));
    })
    .await;
    let rig = &Rig::owned(rig).await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell) = (person(rig, "Mira").await, person(rig, "Nell").await);
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/agents", json!({ "name": "helper" })).await;
    rig.send("/api/agents", json!({ "name": "roamer", "wall": false })).await;
    let channels = || async {
        does(rig, &mira.token, Method::GET, "/api/channels", None).await["channels"].as_array().unwrap().len()
    };
    let turns = |token: &str| {
        let token = token.to_string();
        async move { asked(rig, &token, Method::GET, "/api/agents/helper/turns", None).await.1 }
    };
    let finished = |before: u64| async move {
        rig.event("agent.turn.finished", |event| event.seq > before && event.data["slot"] == "helper").await;
    };

    // Made with an agent in it, which is then asked nowhere else.
    let made = json!({ "name": "Board minutes", "kept": true, "alone": true, "members": ["agent:helper"] });
    let closest = does(rig, &mira.token, Method::POST, "/api/channels", Some(made)).await;
    let first = closest["id"].as_str().unwrap().to_string();
    assert_eq!(closest["alone"], true, "{closest}");
    assert_eq!(closest["members"].as_array().unwrap().len(), 2, "{closest}");
    assert_eq!(closest["members"][1]["actor"], "agent:helper");
    // One that could be in no kept channel, and one there is not, are
    // refused before anything is made.
    for (agent, status, why) in [
        ("agent:roamer", StatusCode::CONFLICT, "`roamer` works with no wall"),
        ("agent:nobody", StatusCode::NOT_FOUND, "nobody"),
    ] {
        let wrong = json!({ "name": "Hiring", "kept": true, "members": [agent] });
        let (answered, refused) =
            rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(wrong)).await;
        assert_eq!(answered, status, "{refused}");
        assert!(refused.to_string().contains(why), "{refused}");
    }
    assert_eq!(channels().await, 2);
    // Held to the one, it is brought into no other kept channel until
    // the first says otherwise; and then the first holds it no more
    // while it is in another.
    let other = does(
        rig,
        &mira.token,
        Method::POST,
        "/api/channels",
        Some(json!({ "name": "Hiring", "kept": true })),
    )
    .await["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (add, alone) = (format!("/api/channels/{other}/members"), format!("/api/channels/{first}/alone"));
    let helper = json!({ "actor": "agent:helper" });
    let (status, refused) = rig.call(Method::POST, &add, Some(&mira.token), Some(helper.clone())).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(refused.to_string().contains("asked nowhere but in"), "{refused}");
    assert!(!refused.to_string().contains(&first) && !refused.to_string().contains("Board"), "{refused}");
    does(rig, &mira.token, Method::PUT, &alone, Some(json!({ "alone": false }))).await;
    let with = does(rig, &mira.token, Method::POST, &add, Some(helper)).await;
    assert_eq!(with["members"][1]["actor"], "agent:helper", "{with}");
    let (status, refused) =
        rig.call(Method::PUT, &alone, Some(&mira.token), Some(json!({ "alone": true }))).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(refused.to_string().contains("`helper` would be asked nowhere but here"), "{refused}");
    // Taken out of the other by somebody who is in it; and not twice.
    let leave = format!("/api/channels/{other}/leave");
    let left = does(rig, &mira.token, Method::POST, &leave, Some(json!({ "agent": "helper" }))).await;
    assert_eq!(left, json!({ "channel": other, "left": "agent:helper" }));
    let (status, twice) =
        rig.call(Method::POST, &leave, Some(&mira.token), Some(json!({ "agent": "helper" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{twice}");

    // An open channel with an agent in it that could not stay is not
    // kept, and the sentence says which and the way out; with that one
    // out it is, and what the other was asked there is its members'.
    for actor in ["agent:helper", "agent:roamer", mira.actor.as_str()] {
        does(
            rig,
            &mira.token,
            Method::POST,
            "/api/channels/general/members",
            Some(json!({ "actor": actor })),
        )
        .await;
    }
    let before = rig.mark();
    say(rig, &mira.token, "general", json!({ "body": "@helper count the quince" })).await;
    finished(before).await;
    assert!(turns(&nell.token).await.contains("count the quince"));
    let (status, refused) =
        rig.call(Method::POST, "/api/channels/general/keep", Some(&mira.token), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    let why = refused.to_string();
    assert!(why.contains("`roamer` works with no wall") && why.contains("#general is not kept"), "{why}");
    assert!(why.contains(&format!("{} channel leave general --agent roamer", brand::CLI)), "{why}");
    does(rig, &mira.token, Method::POST, "/api/channels/general/leave", Some(json!({ "agent": "roamer" })))
        .await;
    let general = does(rig, &mira.token, Method::POST, "/api/channels/general/keep", None).await;
    let id = general["id"].as_str().unwrap().to_string();
    assert_eq!(general["members"].as_array().unwrap().len(), 3, "the owner, the agent and Mira: {general}");
    let (to_nell, to_mira) = (turns(&nell.token).await, turns(&mira.token).await);
    assert!(!to_nell.contains("quince") && to_nell.contains("kept_from"), "{to_nell}");
    assert!(to_mira.contains("count the quince") && !to_mira.contains("kept_from"), "{to_mira}");
    // Asked there now, it answers there, and that turn is its members' too.
    let before = rig.mark();
    say(rig, &mira.token, &id, json!({ "body": "@helper and the damson" })).await;
    finished(before).await;
    assert!(!turns(&nell.token).await.contains("damson") && turns(&mira.token).await.contains("damson"));

    // Its program is tried where a turn asked there starts it, by
    // somebody who is in the channel: one that no package runs has
    // nothing to try, and an open channel is no place to try one in.
    let try_in = |channel: &str| json!({ "channel": channel });
    does(rig, &rig.token, Method::POST, "/api/channels", Some(json!({ "name": "lobby" }))).await;
    let (status, open) =
        rig.call(Method::POST, "/api/agents/helper/try", Some(&mira.token), Some(try_in("lobby"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{open}");
    assert!(open.to_string().contains("that channel is open"), "{open}");
    let (status, plain) =
        rig.call(Method::POST, "/api/agents/helper/try", Some(&mira.token), Some(try_in(&id))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{plain}");
    assert!(plain.to_string().contains("is not run by a package's program"), "{plain}");

    // Opened again by the owner, what it was asked there is anybody's
    // who may see agents, as what was said there is.
    does(rig, &rig.token, Method::POST, &format!("/api/channels/{id}/open"), None).await;
    let to_nell = turns(&nell.token).await;
    assert!(to_nell.contains("count the quince") && to_nell.contains("damson"), "{to_nell}");
}

/// A text that makes sure of a channel and says something there.
fn minutes(name: &str, kept: bool) -> String {
    format!(
        r#"
blueprint: minutes
steps:
  sure: {{ do: channel.ensure, allow_failure: true, with: {{ name: "{name}", kept: {kept} }} }}
  post: {{ needs: [sure], do: channel.post, allow_failure: true, with: {{ channel: "${{steps.sure.out.id}}", body: "the minutes of {name}" }} }}
"#
    )
}

#[tokio::test]
async fn a_blueprint_asks_for_a_kept_channel_and_finds_the_one_it_made_or_one_kept_since_each_time_it_is_applied()
 {
    let rig = Rig::owned(Rig::with("api-kept-ensure", |_, _| {}).await).await;
    let rig = &rig;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell) = (person(rig, "Mira").await, person(rig, "Nell").await);
    // What applying the text as `token` came to: the step that makes
    // sure of the channel, as it ended.
    let applied = |token: &str, name: &'static str, kept: bool| {
        let token = token.to_string();
        async move {
            let asked = json!({ "source": minutes(name, kept), "wait": true });
            let applied = does(rig, &token, Method::POST, "/api/apply", Some(asked)).await;
            // By what the step is called: two that began in one moment are listed in no promised order.
            let steps = applied["finished"]["steps"].as_array().unwrap_or_else(|| panic!("{applied}"));
            steps.iter().find(|step| step["step"] == "sure").unwrap_or_else(|| panic!("{applied}")).clone()
        }
    };
    let said_in = |token: &str, channel: &str| {
        let (token, path) = (token.to_string(), format!("/api/channels/{channel}/messages"));
        async move { asked(rig, &token, Method::GET, &path, None).await }
    };

    // Asked for kept: made with whoever applied the text in it, and
    // found again when it is applied again; what the text says is said
    // there both times, to its members.
    let first = applied(&mira.token, "Pay review", true).await;
    let channel = first["output"]["id"].as_str().unwrap_or_else(|| panic!("{first}")).to_string();
    assert_eq!(first["output"]["name"], json!(channel), "a kept channel's name is its id: {first}");
    assert_eq!(applied(&mira.token, "Pay review", true).await["output"]["id"], json!(channel));
    let (status, said) = said_in(&mira.token, &channel).await;
    assert_eq!((status, said.matches("the minutes of Pay review").count()), (StatusCode::OK, 2), "{said}");
    assert_eq!(said_in(&nell.token, &channel).await.0, StatusCode::NOT_FOUND);
    // Somebody else's text is handed a channel of its own, called the same.
    let hers = applied(&nell.token, "Pay review", true).await;
    assert!(hers["output"]["id"] != json!(channel) && hers["output"]["id"].is_string(), "{hers}");

    // An open channel a text made is kept by somebody since: the text,
    // applied again as it was, finds it and goes on saying things there.
    let open = applied(&mira.token, "lobby", false).await["output"]["id"].as_str().unwrap().to_string();
    assert_eq!(said_in(&nell.token, "lobby").await.0, StatusCode::OK);
    does(rig, &mira.token, Method::POST, "/api/channels/lobby/keep", None).await;
    assert_eq!(applied(&mira.token, "lobby", false).await["output"]["id"], json!(open));
    let (_, said) = said_in(&mira.token, &open).await;
    assert_eq!(said.matches("the minutes of lobby").count(), 2, "{said}");
    assert_eq!(said_in(&nell.token, "lobby").await.0, StatusCode::NOT_FOUND, "and no open one was made");

    // Asked for kept where an open channel has the name, nothing is
    // made and nothing kept: the step is refused, saying what to do.
    does(rig, &nell.token, Method::POST, "/api/channels", Some(json!({ "name": "yard" }))).await;
    let refused = applied(&mira.token, "Yard", true).await;
    let why = refused["error"].as_str().unwrap_or_else(|| panic!("{refused}"));
    assert!(why.contains("#yard is an open channel") && why.contains("channel keep yard"), "{why}");
    let yard = does(rig, &nell.token, Method::GET, "/api/channels/yard", None).await;
    assert!(yard.get("kept").is_none(), "{yard}");
}
