//! An agent in a channel kept to its members (`agents::kept`): the half
//! of the journey of `kept.rs` that is an agent's, numbered by the same
//! ways a channel's words are read.
//!
//! An agent is in the kept channel and another is not. What the first is
//! asked there, what it answers and what is said of its turn are read by
//! the channel's members and by nobody else, its own key reads the
//! channel only while a turn asked there is in hand, a daemon that comes
//! back in the middle of that turn opens the channel for that turn and
//! no longer, and an agent with no wall is not in it. Each refusal is
//! what a channel that is not there is refused with, word for word.
//!
//! Unix's alone: a kept channel has in it only agents that work behind
//! a wall, and what holds the wall here is a file of a Unix shell, which
//! starts the agent's program as it is.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::kept::{Told, asked, key, person, reads_none, say};

/// Writes `text` as a program at `path`, by a process of its own: a
/// file this process had open to write is refused as a program while
/// anything it starts in that moment has not yet become itself.
fn program(path: &Path, text: &str) {
    use std::io::Write;
    let mut writes = std::process::Command::new("sh")
        .args(["-c", "cat > \"$0\" && chmod 755 \"$0\""])
        .arg(path)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    writes.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    assert!(writes.wait().unwrap().success(), "could not write {}", path.display());
}

/// Something to hold walls with that starts the command as it is.
pub(crate) fn holder(dir: &Path) -> PathBuf {
    let path = dir.join("holder");
    program(&path, "#!/bin/sh\nwhile [ \"$1\" != \"--\" ]; do shift; done\nshift\nexec \"$@\"\n");
    path
}

/// The credential an agent's program reads each time it asks the factory anything.
fn token(rig: &Rig, name: &str) -> String {
    std::fs::read_to_string(rig.dir.join(format!("data/slots/{name}/token"))).unwrap().trim().to_string()
}

/// What `token` is answered when it asks for what was said in `channel`,
/// and in a channel that is not there, each with the channel taken out.
async fn reads(rig: &Rig, token: &str, channel: &str) -> ((StatusCode, String), (StatusCode, String)) {
    let of = |id: &str| format!("/api/channels/{id}/messages");
    let (here, there) = (
        asked(rig, token, Method::GET, &of(channel), None).await,
        asked(rig, token, Method::GET, &of("ch_nonesuch"), None).await,
    );
    ((here.0, here.1.replace(channel, "<it>")), (there.0, there.1.replace("ch_nonesuch", "<it>")))
}

/// Everything `token` can list or look for, which leaves the kept
/// channel out: none of `marked` is read in any of it.
async fn finds_none(rig: &Rig, who: &str, token: &str, marked: &[&str]) {
    let mut lists = vec![
        "/api/channels".to_string(),
        "/api/events/recent?limit=1000".into(),
        "/api/events/recent?limit=1000&topics=agent.*,channel.*".into(),
        "/api/agents/scribe/turns".into(),
        "/api/workspaces".into(),
    ];
    lists.extend(["medlar", "damson", "quince"].map(|word| format!("/api/search?q={word}")));
    for path in lists {
        let (_, answer) = asked(rig, token, Method::GET, &path, None).await;
        reads_none(&format!("by {who}, in {path}"), &answer, marked);
    }
}

/// A text that has the agent go on with a conversation, the step let fail.
fn goes_on(thread: &str) -> String {
    format!(
        "blueprint: reads-out\nsteps:\n  ask: {{ do: agent.task, allow_failure: true, with: {{ slot: scribe, \
         prompt: \"What was said there?\", thread: \"{thread}\" }} }}\n"
    )
}

/// Brings the agent `name` into the channel `to`, as `by`: how it went.
async fn bring(rig: &Rig, name: &str, adapter: &str, to: &str, by: &str) -> (StatusCode, Value) {
    let body = json!({ "name": name, "adapter": adapter });
    rig.call(Method::POST, &format!("/api/channels/{to}/agents"), Some(by), Some(body)).await
}

/// The turns of the agent in the kept channel, as `token` is shown them.
async fn turns(rig: &Rig, token: &str) -> String {
    asked(rig, token, Method::GET, "/api/agents/scribe/turns", None).await.1
}

/// Applies, as `token`, a text that has the agent go on with the
/// conversation `thread`: how its step came out.
async fn tried(rig: &Rig, thread: &str, token: &str) -> Value {
    let asked = json!({ "source": goes_on(thread), "wait": true });
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(token), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    applied["finished"]["steps"][0].clone()
}

#[tokio::test]
async fn an_agent_in_a_kept_channel_reads_it_only_while_asked_there_and_what_it_is_asked_is_its_members_to_read()
 {
    let mut gate = None;
    let rig = Rig::with("api-kept-agents", |config, dir| {
        // A wall nothing is kept behind: what is asked of the system is
        // the same wherever the tests run.
        config.walls = Some(api::Walls::Held(holder(dir)));
        let pipe = dir.join("gate");
        crate::hold(&pipe);
        let does = ["swallow", "held", &pipe.to_string_lossy(), "waiter", "say", "The plum answer."];
        config.adapters.push(("waiter".into(), crate::standin::argv(&does)));
        gate = Some(pipe);
    })
    .await;
    let (mut rig, gate) = (Rig::owned(rig).await, gate.unwrap());
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell) = (person(&rig, "Mira").await, person(&rig, "Nell").await);
    let mut manages: Vec<&str> = kernel::principal::MEMBER_SCOPES.to_vec();
    manages.push("admin");
    let admin = key(&rig, &rig.token, &manages).await;
    rig.send("/api/channels", json!({ "name": "general" })).await;

    // Mira makes the channel, kept, speaks there, and brings an agent
    // in. Another agent is in the open channel and not in this one.
    let made = json!({ "name": "Quince review", "topic": "the greengage accounts", "kept": true });
    let (status, kept) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(made)).await;
    assert_eq!(status, StatusCode::OK, "{kept}");
    let channel = kept["id"].as_str().unwrap().to_string();
    let first = say(&rig, &mira.token, &channel, json!({ "body": "the medlar ledger" })).await;
    let (status, brought) = bring(&rig, "scribe", "waiter", &channel, &mira.token).await;
    assert_eq!(status, StatusCode::OK, "{brought}");
    assert!(brought["channel"]["members"].to_string().contains("agent:scribe"), "{brought}");
    for (name, adapter) in [("scribe", "waiter"), ("clerk", "builtin")] {
        let (status, brought) = bring(&rig, name, adapter, "general", &rig.token).await;
        assert_eq!(status, StatusCode::OK, "{brought}");
    }
    let (scribe, clerk) = (token(&rig, "scribe"), token(&rig, "clerk"));
    // What is its alone. (What the agent answers is the same words
    // wherever it is asked, being a stand-in's, and is no mark of it.)
    let marked = ["Quince", "greengage", "medlar", "damson", channel.as_str(), first.as_str()];

    // 48: an agent with no wall is not brought into it, by somebody who
    // is in it and may take a wall away; nor one that is in it already.
    let bare = json!({ "name": "roamer", "wall": "off" });
    let (status, refused) = rig
        .call(Method::POST, &format!("/api/channels/{channel}/agents"), Some(&rig.token), Some(bare))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(refused.to_string().contains("an agent with no wall is in no channel kept to its members"));
    assert!(!rig.read("/api/agents").await.to_string().contains("roamer"), "nothing was made of it");

    // 46: with nothing in hand its own key reads the channel no more
    // than the other agent's does, or anybody's: as one that is not there.
    for (who, token) in [("the agent in it", &scribe), ("an agent that is not", &clerk)] {
        let (here, there) = reads(&rig, token, &channel).await;
        assert_eq!(here, there, "{who}");
        assert_eq!(here.0, StatusCode::NOT_FOUND);
        finds_none(&rig, who, token, &marked).await;
    }

    // It is asked there. Its turn is in hand while its program waits.
    let (mut nell_hears, mut mira_hears) =
        (Told::of(&rig, &nell.token).await, Told::of(&rig, &mira.token).await);
    let before = rig.mark();
    say(&rig, &mira.token, &channel, json!({ "body": "@scribe what of the damson?" })).await;
    let started =
        rig.event("agent.turn.started", |event| event.seq > before && event.data["slot"] == "scribe").await;
    let turn = started.data["turn"].as_str().unwrap().to_string();
    assert_eq!(started.source.takes, Some(format!("member:{channel}")));
    // 46: while it is, its key reads the channel, and says something there.
    let (here, _) = reads(&rig, &scribe, &channel).await;
    assert_eq!(here.0, StatusCode::OK, "{here:?}");
    assert!(here.1.contains("the medlar ledger") && here.1.contains("what of the damson"), "{here:?}");
    let (_, listed) = asked(&rig, &scribe, Method::GET, "/api/channels", None).await;
    assert!(listed.contains(&channel), "{listed}");
    say(&rig, &scribe, &channel, json!({ "body": "looking into it" })).await;
    // The other agent's reads nothing of it, and neither does a turn of
    // this one that is only waiting behind another.
    let (here, there) = reads(&rig, &clerk, &channel).await;
    assert_eq!(here, there);

    // 16: its turns, as each is shown them. A member reads what it was
    // asked; anybody else when, how long and what it cost.
    for whole in [turns(&rig, &mira.token).await, turns(&rig, &rig.token).await] {
        assert!(whole.contains("what of the damson") && whole.contains(&channel), "{whole}");
        assert!(!whole.contains("kept_from"), "{whole}");
        // And, apart from all its agent was sent, the words that asked for it.
        let shown: Value = serde_json::from_str(&whole).unwrap();
        assert_eq!(shown["turns"][0]["asked"], "@scribe what of the damson?", "{whole}");
    }
    for (who, token) in [("Nell", &nell.token), ("an administrator", &admin), ("another agent", &clerk)] {
        let bare = turns(&rig, token).await;
        reads_none(&format!("by {who}, in the agent's turns"), &bare, &marked);
        let shown: Value = serde_json::from_str(&bare).unwrap();
        let one = &shown["turns"][0];
        assert_eq!(
            (&one["id"], &one["status"], &one["kept_from"]),
            (&json!(turn), &json!("running"), &json!(true))
        );
        assert!(one["started"].is_i64() && one["cost_usd"].is_number() && one["prompt"] == "", "{one}");
        assert!(one.get("asked").is_none(), "what was asked is handed to {who}: {one}");
    }

    // 29: that it was queued and began was told to a member as it was
    // said, and to nobody else.
    say(&rig, &mira.token, "general", json!({ "body": "began-marker" })).await;
    nell_hears.until("began-marker").await;
    reads_none("on what Nell is told as the turn begins", &nell_hears.heard, &marked);
    assert!(
        !nell_hears.heard.contains(&turn) && !nell_hears.heard.contains("agent.turn"),
        "{}",
        nell_hears.heard
    );
    mira_hears.until("began-marker").await;
    for said in ["agent.turn.queued", "agent.turn.started", turn.as_str(), "looking into it"] {
        assert!(mira_hears.heard.contains(said), "Mira is told `{said}`: {}", mira_hears.heard);
    }
    drop((nell_hears, mira_hears));

    // The daemon stops with the turn in hand, and comes back: the turn
    // is gone on with, and the channel is open to the agent's key for
    // it again.
    let before = rig.mark();
    rig.restart().await;
    rig.event("agent.turn.started", |event| event.seq > before && event.data["again"] == 1).await;
    let (here, _) = reads(&rig, &scribe, &channel).await;
    assert_eq!(here.0, StatusCode::OK, "{here:?}");
    let (here, there) = reads(&rig, &clerk, &channel).await;
    assert_eq!(here, there);
    // Everybody listens, to the daemon that came back.
    let (mut to_nell, mut to_admin, mut to_mira) =
        (Told::of(&rig, &nell.token).await, Told::of(&rig, &admin).await, Told::of(&rig, &mira.token).await);
    crate::let_go("the agent to be waiting at its hold", &gate).await;
    rig.event("agent.turn.finished", |event| event.seq > before && event.data["turn"] == turn.as_str()).await;
    // Its answer was said there, by it; and with that the channel is
    // closed to its key again.
    let said = rig.read(&format!("/api/channels/{channel}/messages")).await.to_string();
    assert!(said.contains("The plum answer.") && said.contains("looking into it"), "{said}");
    let (here, there) = reads(&rig, &scribe, &channel).await;
    assert_eq!(here, there, "with the turn ended, its key opens nothing");
    finds_none(&rig, "the agent in it, its turn ended", &scribe, &marked).await;

    // 46: at work on something asked elsewhere, its key opens nothing
    // of the kept channel either.
    let before = rig.mark();
    say(&rig, &rig.token, "general", json!({ "body": "@scribe and the open word?" })).await;
    rig.event("agent.turn.started", |event| event.seq > before && event.data["slot"] == "scribe").await;
    let (here, there) = reads(&rig, &scribe, &channel).await;
    assert_eq!(here, there, "a turn asked elsewhere opens nothing");
    assert_eq!(reads(&rig, &scribe, "general").await.0.0, StatusCode::OK);
    crate::let_go("the agent to be waiting at its hold", &gate).await;
    rig.event("agent.turn.finished", |event| event.seq > before && event.data["slot"] == "scribe").await;

    // 29, 30: of all that, nobody else was told anything as it was
    // said: not that a turn was queued, began or ended, nor what its
    // program printed. A member was.
    say(&rig, &mira.token, "general", json!({ "body": "agents-marker" })).await;
    for (who, told) in [("Nell", &mut to_nell), ("an administrator", &mut to_admin)] {
        told.until("agents-marker").await;
        reads_none(&format!("on what {who} is told as it is said"), &told.heard, &marked);
        assert!(!told.heard.contains(&turn), "{who} is told of the turn: {}", told.heard);
        assert!(told.heard.contains("and the open word"), "{}", told.heard);
    }
    to_mira.until("agents-marker").await;
    assert!(
        to_mira.heard.contains(&turn) && to_mira.heard.contains("agent.turn.finished"),
        "{}",
        to_mira.heard
    );
    assert!(to_mira.heard.contains("The plum answer."), "{}", to_mira.heard);
    for (who, token) in [("Nell", &nell.token), ("an administrator", &admin), ("another agent", &clerk)] {
        finds_none(&rig, who, token, &marked).await;
    }
    // And its turns are still shown them without their words, ended.
    let bare: Value = serde_json::from_str(&turns(&rig, &nell.token).await).unwrap();
    let ended = bare["turns"].as_array().unwrap().iter().find(|one| one["id"] == json!(turn)).unwrap();
    assert_eq!(
        (&ended["status"], &ended["reply"], &ended["kept_from"]),
        (&json!("done"), &json!(""), &json!(true))
    );
    assert!(ended["finished"].as_i64() >= ended["started"].as_i64(), "{ended}");

    // 39: a text of Nell's has the agent go on with the conversation,
    // by the channel and by a message of it: refused as one that is not
    // there, and the agent is asked nothing.
    let before = rig.mark();
    for (named, nowhere, id, nothing) in [
        (format!("channel:{channel}"), "channel:ch_nonesuch", channel.as_str(), "ch_nonesuch"),
        (format!("thread:{first}"), "thread:msg_nonesuch", first.as_str(), "msg_nonesuch"),
    ] {
        // By her, and by somebody who manages the factory and holds
        // nothing that is kept private.
        for token in [&nell.token, &admin] {
            let (here, there) = (tried(&rig, &named, token).await, tried(&rig, nowhere, token).await);
            let refused = here["error"].as_str().unwrap_or_else(|| panic!("it is refused: {here}"));
            reads_none("by a step that would have the agent go on", refused, &["medlar", "Quince", "damson"]);
            assert_eq!(
                refused.replace(id, "<it>"),
                there["error"].as_str().unwrap().replace(nothing, "<it>")
            );
            assert!(refused.contains("there is no"), "{refused}");
        }
    }
    let asked_since =
        rig.said(journal::EventQuery { topics: "agent.turn.queued".into(), ..Default::default() }).await;
    assert!(asked_since.iter().all(|event| event.seq < before), "the agent was asked nothing");

    // 47: what its program kept of the channel is in a folder of that
    // channel's own, where it worked on what it was asked there.
    let its_own = rig.dir.join("data/slots/scribe/kept").join(&channel);
    assert!(its_own.join("home").is_dir(), "{}", its_own.display());

    // 48: told to work with no wall, the agent leaves the channel, which
    // is said there; what it kept of it goes; and it is asked nothing
    // there from then.
    let before = rig.mark();
    rig.send("/api/agents", json!({ "name": "scribe", "wall": "off" })).await;
    rig.event("channel.left", |event| event.seq > before && event.data["member"] == "agent:scribe").await;
    assert!(!its_own.exists(), "{}", its_own.display());
    let (_, there) =
        rig.call(Method::GET, &format!("/api/channels/{channel}"), Some(&mira.token), None).await;
    assert!(!there["members"].to_string().contains("agent:scribe"), "{there}");
    let said = rig.read(&format!("/api/channels/{channel}/messages")).await.to_string();
    assert!(said.contains("`scribe` works with no wall") && said.contains("it has left this one"), "{said}");
    say(&rig, &mira.token, &channel, json!({ "body": "@scribe are you there?" })).await;
    say(&rig, &mira.token, "general", json!({ "body": "last-marker" })).await;
    to_mira.until("last-marker").await;
    let asked_since =
        rig.said(journal::EventQuery { topics: "agent.turn.queued".into(), ..Default::default() }).await;
    assert!(asked_since.iter().all(|event| event.seq < before), "it is not asked there once it has left");
    // (Her own text's steps say the ids she wrote in it, which a refusal
    // echoes: nothing she was told.)
    to_nell.until("last-marker").await;
    reads_none("on what Nell is told of its leaving", &to_nell.heard, &marked[..4]);
    assert!(
        !to_nell.heard.contains("channel.left") && !to_nell.heard.contains("no wall"),
        "{}",
        to_nell.heard
    );

    // 12, 16: the channel is taken away. What the agent was asked there
    // by a message goes with it, and nothing of its words is shown to
    // anybody, who was in it or not.
    let (status, removed) =
        rig.call(Method::DELETE, &format!("/api/channels/{channel}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    for (who, token) in [("Mira", &mira.token), ("the owner", &rig.token), ("Nell", &nell.token)] {
        let shown = turns(&rig, token).await;
        reads_none(&format!("by {who}, in the agent's turns once the channel is gone"), &shown, &marked);
        assert!(shown.contains("and the open word"), "what it was asked in the open stays: {shown}");
    }
}

#[tokio::test]
async fn a_kept_channel_says_its_agents_are_asked_nowhere_else_and_they_are_not() {
    let rig = Rig::with("api-kept-alone", |config, dir| {
        config.walls = Some(api::Walls::Held(holder(dir)));
    })
    .await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let (mira, nell) = (person(&rig, "Mira").await, person(&rig, "Nell").await);
    rig.send("/api/channels", json!({ "name": "general" })).await;

    // Said of a kept channel, and of no other.
    let open = json!({ "name": "wide open", "alone": true });
    let (status, refused) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(open)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused.to_string().contains("`alone` is said of a channel kept to its members"), "{refused}");
    let odd = json!({ "name": "Minutes", "kept": true, "alone": "yes" });
    let (status, refused) = rig.call(Method::POST, "/api/channels", Some(&mira.token), Some(odd)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let make = |body: Value| {
        let (rig, token) = (&rig, mira.token.clone());
        async move {
            let (status, made) = rig.call(Method::POST, "/api/channels", Some(&token), Some(body)).await;
            assert_eq!(status, StatusCode::OK, "{made}");
            made
        }
    };
    let closest = make(json!({ "name": "Board minutes", "kept": true, "alone": true })).await;
    let kept = make(json!({ "name": "Pay review", "kept": true })).await;
    assert_eq!((&closest["alone"], kept.get("alone")), (&json!(true), None));
    let (closest, kept) = (closest["id"].as_str().unwrap(), kept["id"].as_str().unwrap());

    // Its agent is in the open channel too, where it is said something
    // by name: it is not asked, and the factory says why to everybody
    // there, naming no channel.
    assert_eq!(bring(&rig, "scribe", "builtin", closest, &mira.token).await.0, StatusCode::OK);
    assert_eq!(bring(&rig, "scribe", "builtin", "general", &rig.token).await.0, StatusCode::OK);
    let before = rig.mark();
    say(&rig, &nell.token, "general", json!({ "body": "@scribe what is on the board?" })).await;
    let note =
        rig.event("channel.message", |event| event.seq > before && event.data["author_kind"] == "bot").await;
    let said = note.data["body"].as_str().unwrap();
    assert!(said.starts_with("scribe was not asked: ") && said.contains("asked nowhere else"), "{said}");
    reads_none("in what the factory says of an agent that was not asked", said, &[closest, kept, "Board"]);
    let queued =
        rig.said(journal::EventQuery { topics: "agent.turn.queued".into(), ..Default::default() }).await;
    assert!(queued.is_empty(), "it was asked nothing: {queued:?}");

    // And it is brought into no other kept channel, by a refusal that
    // names neither.
    let (status, refused) = bring(&rig, "scribe", "builtin", kept, &mira.token).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    reads_none("in why it is not brought in", &refused.to_string(), &[closest, "Board minutes"]);

    // Asked there, it is, and answers there.
    let before = rig.mark();
    say(&rig, &mira.token, closest, json!({ "body": "@scribe help" })).await;
    rig.event("agent.turn.finished", |event| event.seq > before && event.data["slot"] == "scribe").await;
}

/// A package for a product there is not, whose program adds a line to a
/// file where it works on a card (a folder of the card's own) and says
/// that it did, wherever it is asked; `wall` is what it says its program
/// keeps in the home of whoever runs it. A card's folder is told by its
/// name (`<project>/.lanes/shop-<n>`) and not by `.lanes` alone: a
/// checkout worked in a place of its own is under one too, and so is
/// everything a test there makes.
fn package(dir: &Path, wall: &str) -> String {
    let file = dir.join("standin-agent");
    if !file.exists() {
        let does = "#!/bin/sh\ncat > /dev/null\ncase \"$PWD\" in */.lanes/shop-*) echo changed >> CHANGES.md ;; esac\n\
                    printf '{\"text\":\"I changed a file.\"}\\n'\n";
        program(&file, does);
    }
    format!(
        "extension: standin\ntitle: Stand-in\ndescription: Not a real agent.\nagent:\n  program: {}\n  input: \
         ${{ turn.prompt }}\n  output:\n    - when: ${{ event.text != null }}\n      reply: ${{ event.text }}\n  \
         wall:\n{wall}",
        file.display()
    )
}

#[tokio::test]
async fn a_fleet_whose_worker_is_in_a_kept_channel_goes_on_working_its_cards_and_answers_there() {
    use crate::fleets::{active, add, shop};
    use crate::lanes::{move_card, moved};
    let rig = Rig::with("api-kept-fleet", |config, dir| {
        config.walls = Some(api::Walls::Held(holder(dir)));
        // The home of whoever runs the daemon is the test's own: what a
        // package says its program keeps is looked for there.
        let home = dir.join("home");
        std::fs::create_dir_all(home.join(".standin")).unwrap();
        std::fs::write(home.join(".standin/credentials"), "the person's own sign-in").unwrap();
        config.user_home = Some(home);
        let approves = ["swallow", "say", "It does what the card asks.", "say", "DECISION: approve"];
        config.adapters.push(("critic".into(), crate::standin::argv(&approves)));
    })
    .await;
    let rig = Rig::owned(rig).await;
    // A fleet whose worker runs on a package, signed in as whoever runs
    // the daemon, as an agent is unless something else is said.
    let keeps = "    keeps: [\".standin\"]\n";
    let says_less = package(&rig.dir, keeps);
    let says_its_sign_in = package(&rig.dir, &format!("{keeps}    signin: [\".standin/credentials\"]\n"));
    rig.send("/api/extensions", json!({ "source": says_less })).await;
    let rows = json!([
        { "title": "worker", "role": "worker", "count": 1, "adapter": "standin" },
        { "title": "reviewer", "role": "reviewer", "count": 1, "adapter": "critic", "permission": "read" },
    ]);
    let dir = shop(&rig, json!({ "agents": rows })).await;
    assert_eq!(active(&rig, "worker").await, ["shop-worker-1"]);
    let made = rig.send("/api/channels", json!({ "name": "Shop talk", "kept": true })).await;
    let channel = made["id"].as_str().unwrap().to_string();

    // Its package does not say which of what its program keeps is its
    // sign-in: it is not brought into the kept channel, and whoever
    // asked is told why and the two ways out.
    let (status, refused) = bring(&rig, "shop-worker-1", "standin", &channel, &rig.token).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    let said = refused.to_string();
    for words in [
        "signs in as whoever runs the daemon, and its package `standin` does not say which",
        "is in no channel kept to its members",
        "agent.wall.signin",
        "help wall",
        "signin add <name> --for standin --kind key",
    ] {
        assert!(said.contains(words), "`{words}` in: {said}");
    }
    let (_, there) = rig.call(Method::GET, &format!("/api/channels/{channel}"), Some(&rig.token), None).await;
    assert!(!there["members"].to_string().contains("agent:"), "{there}");

    // The package comes to say it, and the worker is brought in.
    rig.send("/api/extensions", json!({ "source": says_its_sign_in })).await;
    let (status, brought) = bring(&rig, "shop-worker-1", "standin", &channel, &rig.token).await;
    assert_eq!(status, StatusCode::OK, "{brought}");
    assert!(brought["channel"]["members"].to_string().contains("agent:shop-worker-1"), "{brought}");

    // In the kept channel, it is still the fleet's worker: a card of
    // the fleet's board is worked, reviewed and merged.
    let first = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &first, "ready", None).await;
    moved(&rig, &first, "done", 0).await;
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "changed\n");
    // And asked in the channel, it answers there, from the home that is
    // that channel's.
    let before = rig.mark();
    say(&rig, &rig.token, &channel, json!({ "body": "@shop-worker-1 how is the cart?" })).await;
    let asked_there = rig
        .event("agent.turn.finished", |event| event.seq > before && event.data["slot"] == "shop-worker-1")
        .await;
    assert_eq!((&asked_there.data["ok"], &asked_there.data["channel"]), (&json!(true), &json!(channel)));
    let said = rig.read(&format!("/api/channels/{channel}/messages")).await.to_string();
    assert!(said.contains("I changed a file."), "{said}");
    assert!(rig.dir.join("data/slots/shop-worker-1/kept").join(&channel).join("home").is_dir());
    // Then another card, with the conversation behind it.
    let second = add(&rig, "Add a checkout", &[]).await;
    move_card(&rig, &second, "ready", None).await;
    moved(&rig, &second, "done", 0).await;
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "changed\nchanged\n");

    // The package is replaced by one that says less: the worker leaves
    // the kept channel, which is said there, and goes on with the
    // fleet's cards.
    let before = rig.mark();
    rig.send("/api/extensions", json!({ "source": package(&rig.dir, keeps) })).await;
    rig.event("channel.left", |event| event.seq > before && event.data["member"] == "agent:shop-worker-1")
        .await;
    let said = rig.read(&format!("/api/channels/{channel}/messages")).await.to_string();
    assert!(said.contains("does not say which") && said.contains("it has left this one"), "{said}");
    assert!(!rig.dir.join("data/slots/shop-worker-1/kept").join(&channel).exists());
    let third = add(&rig, "Add a receipt", &[]).await;
    move_card(&rig, &third, "ready", None).await;
    moved(&rig, &third, "done", 0).await;
    let query =
        journal::EventQuery { topics: "agent.turn.finished".into(), limit: 100, ..Default::default() };
    let finished = rig.said(query).await;
    assert!(finished.len() >= 7 && finished.iter().all(|event| event.data["ok"] == true), "{finished:?}");
}
