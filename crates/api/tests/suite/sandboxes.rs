//! A sandbox test by its routes: who may run one, whose it is, how many
//! there are at once, and how one ends.
#![cfg(target_os = "linux")]

use std::sync::Arc;
use std::time::Duration;

use kernel::ManualClock;
use reqwest::Method;
use serde_json::{Value, json};

use crate::{Rig, soon};

const ASKS: &str = "blueprint: asks\npermissions: [agents]\nsteps:\n  ask: { do: agent.task, with: { prompt: \"Which way?\", role: worker } }\n  keep: { do: set, with: { way: \"${steps.ask.out.reply}\" } }\nstate:\n  way: none\n  rung: nobody\non:\n  rung:\n    event: bell.rung\n    steps:\n      note: { do: set, with: { rung: \"${event.data.by}\" } }\n";

/// The names of this process's threads, as the system says them.
fn threads() -> Vec<String> {
    std::fs::read_dir("/proc/self/task")
        .unwrap()
        .flatten()
        .filter_map(|task| std::fs::read_to_string(task.path().join("comm")).ok())
        .map(|name| name.trim().to_string())
        .collect()
}

/// What the thread of the sandbox called `id` is called.
fn thread_of(id: &str) -> String {
    format!("sandbox-{}", &id[id.len() - 7..])
}

/// Waits until the thread of the sandbox called `id` has ended: a
/// sandbox says it has ended as the last thing its thread does.
async fn thread_gone(id: &str) {
    soon("a sandbox's thread to have ended", async {
        while threads().contains(&thread_of(id)) {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

async fn made(rig: &Rig, body: Value) -> String {
    rig.send("/api/sandboxes", body).await["sandbox"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_sandbox_is_made_acted_on_read_and_ended_and_every_answer_says_what_it_was() {
    let rig = Rig::new().await;
    // A factory nobody runs a sandbox test in has nothing of them.
    assert!(!rig.dir.join("data").join("sandboxes").exists());
    assert_eq!(rig.node().app.sandboxes.count(), 0);

    let applied = rig.send("/api/sandboxes", json!({ "source": ASKS })).await;

    let id = applied["sandbox"].as_str().unwrap().to_string();
    assert_eq!(applied["ok"], true);
    assert!(applied["said"].as_str().unwrap().starts_with("This was a sandbox test"), "{applied}");
    assert_eq!(applied["project"], "sandbox");
    // The agent is not asked: a stand-in waits to be answered in its place.
    assert_eq!(applied["waiting"][0]["action"], "agent.task", "{applied}");
    assert_eq!(applied["waiting"][0]["asked"]["prompt"], "Which way?");
    let acts = format!("/api/sandboxes/{id}/acts");
    let answer = json!({ "answer": { "asked": 1, "gives": { "reply": "left", "available": true } } });
    let answered = rig.send(&acts, answer).await;
    assert_eq!(answered["waiting"], json!([]));
    assert!(answered["said"].as_str().unwrap().starts_with("This was a sandbox test"));
    rig.send(&acts, json!({ "say": { "topic": "bell.rung", "data": { "by": "ada" } } })).await;
    let moved = rig.send(&acts, json!({ "advance": { "by": "5m" } })).await;
    assert_eq!(moved["did"]["advanced_ms"], 300_000);

    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;

    assert!(shown["said"].as_str().unwrap().contains("passing here is not passing for real"), "{shown}");
    assert_eq!(shown["instances"][0]["state"], json!({ "way": "left", "rung": "ada" }), "{shown}");
    // A run is handed as the daemon hands a real one.
    let run = &shown["runs"][0];
    for key in
        ["id", "instance", "flow", "revision", "status", "event", "outputs", "started", "finished", "steps"]
    {
        assert!(run.get(key).is_some(), "a sandbox's run has no `{key}`: {run}");
    }
    for key in ["run", "step", "status", "attempt", "tolerated", "output", "started", "finished"] {
        assert!(run["steps"][0].get(key).is_some(), "a sandbox's step has no `{key}`: {run}");
    }
    assert_eq!(shown["asked"][0]["by"], "answered");
    // No agent of the factory's was asked anything, and none was made.
    assert_eq!(rig.read("/api/agents").await["agents"], json!([]));
    // What is not something done to a sandbox is said, with what is.
    let (status, other) = rig.post(&acts, json!({ "restart": {} })).await;
    assert_eq!(status, 400, "{other}");
    assert!(other["error"]["message"].as_str().unwrap().contains("What is: `apply`"), "{other}");

    let ended = rig.call(Method::DELETE, &format!("/api/sandboxes/{id}"), Some(&rig.token), None).await;

    assert_eq!(ended.0, 200, "{}", ended.1);
    assert!(ended.1["said"].as_str().unwrap().starts_with("This was a sandbox test"));
    let (status, gone) = rig.get(&format!("/api/sandboxes/{id}")).await;
    assert_eq!(status, 404, "{gone}");
    assert!(gone["error"]["message"].as_str().unwrap().contains("there is no sandbox test"), "{gone}");
    assert!(!rig.dir.join("data").join("sandboxes").exists(), "nothing of it is left on the disk");
    thread_gone(&id).await;
}

#[tokio::test]
async fn a_text_that_does_not_check_is_answered_with_what_is_wrong_and_no_sandbox_is_made() {
    let rig = Rig::new().await;

    let wrong = rig
        .send(
            "/api/sandboxes",
            json!({ "source": "blueprint: wrong\nsteps:\n  a: { do: no.such.action }\n" }),
        )
        .await;

    assert_eq!(wrong["ok"], false);
    assert!(wrong["diagnostics"].to_string().contains("no.such.action"), "{wrong}");
    assert!(wrong.get("sandbox").is_none(), "{wrong}");
    assert_eq!(rig.node().app.sandboxes.count(), 0);
    // One that needs to be told something is refused as an apply is, and leaves none.
    let (status, needs) = rig.post("/api/sandboxes", json!({ "name": "std/pool" })).await;
    assert_eq!(status, 400, "{needs}");
    assert_eq!(rig.node().app.sandboxes.count(), 0);
    assert!(!rig.dir.join("data").join("sandboxes").exists());
    // A name nothing has, and a project there is not.
    let (status, _) = rig.post("/api/sandboxes", json!({ "name": "no/such" })).await;
    assert_eq!(status, 404);
    let (status, _) = rig.post("/api/sandboxes", json!({ "name": "std/board", "project": "nowhere" })).await;
    assert_eq!(status, 404);
    // And a text that is told a project other than the sandbox's own.
    let (status, other) =
        rig.post("/api/sandboxes", json!({ "name": "std/fleet", "params": { "project": "shop" } })).await;
    assert_eq!(status, 400, "{other}");
    assert!(other["error"]["message"].as_str().unwrap().contains("a sandbox has one project"), "{other}");
}

#[tokio::test]
async fn whoever_may_read_blueprints_may_run_one_in_a_sandbox_and_a_sandbox_is_its_maker_s_alone() {
    let rig = Rig::new().await;
    // A key that reads blueprints and holds nothing else: not `shell`,
    // which the text lists, and nothing that applying takes.
    let reader = crate::held::key(&rig, &["blueprints:read"]).await;
    let text =
        "blueprint: builds\npermissions: [shell]\nsteps:\n  build: { do: shell, with: { cmd: [make] } }\n";

    let (status, theirs) = rig
        .call(
            Method::POST,
            "/api/sandboxes",
            Some(&reader),
            Some(json!({ "source": text, "answers": "default" })),
        )
        .await;

    assert_eq!(status, 200, "nothing a sandbox does takes any authority of its maker's: {theirs}");
    let id = theirs["sandbox"].as_str().unwrap();
    let path = format!("/api/sandboxes/{id}");
    assert_eq!(rig.call(Method::GET, &path, Some(&reader), None).await.0, 200);
    // The owner, who holds everything, is answered as for one that is not there, in every way.
    let (status, hidden) = rig.get(&path).await;
    let (_, absent) = rig.get("/api/sandboxes/sandbox_0000000000000000").await;
    assert_eq!(status, 404);
    assert_eq!(
        hidden["error"]["message"].as_str().unwrap().replace(id, "X"),
        absent["error"]["message"].as_str().unwrap().replace("sandbox_0000000000000000", "X")
    );
    assert_eq!(rig.post(&format!("{path}/acts"), json!({ "advance": { "by": "1s" } })).await.0, 404);
    assert_eq!(rig.call(Method::DELETE, &path, Some(&rig.token), None).await.0, 404);
    assert_eq!(
        rig.call(Method::GET, &path, Some(&reader), None).await.0,
        200,
        "and it is still there for its maker"
    );
    // Somebody who may not read blueprints may run none, and is told so before anything is read.
    let other = crate::held::key(&rig, &["board:read"]).await;
    for (method, at) in [
        (Method::POST, "/api/sandboxes".to_string()),
        (Method::GET, path.clone()),
        (Method::DELETE, path.clone()),
    ] {
        let (status, refused) = rig.call(method, &at, Some(&other), Some(json!({ "source": text }))).await;
        assert_eq!(status, 403, "{refused}");
    }
    // A project's own blueprints are files of it: read for whoever may read them.
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let (status, refused) = rig
        .call(
            Method::POST,
            "/api/sandboxes",
            Some(&reader),
            Some(json!({ "source": text, "project": "shop" })),
        )
        .await;
    assert_eq!(status, 403, "{refused}");
}

#[tokio::test]
async fn a_project_s_own_blueprints_are_read_as_they_are_when_a_sandbox_is_made_and_one_may_be_run_by_its_name()
 {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let write = |path: &str, content: &str| {
        rig.call(
            Method::PUT,
            "/api/projects/shop/file",
            Some(&rig.token),
            Some(json!({ "path": path, "content": content })),
        )
    };
    let helper = "blueprint: tools/helper\nsteps:\n  ready: { do: noop }\noutputs:\n  word: first\n";
    assert_eq!(write("@blueprints/tools/helper.bp", helper).await.0, 200);
    let caller = "blueprint: caller\nsteps:\n  ask: { call: tools/helper }\n  keep: { do: set, with: { heard: \"${steps.ask.out.word}\" } }\nstate:\n  heard: nothing\n";
    assert_eq!(write("@blueprints/caller.bp", caller).await.0, 200);

    let id = made(&rig, json!({ "name": "caller", "project": "shop" })).await;
    // Changed in the project afterwards, the sandbox goes on with what it read.
    assert_eq!(write("@blueprints/tools/helper.bp", &helper.replace("first", "second")).await.0, 200);
    let again = rig.send(&format!("/api/sandboxes/{id}/acts"), json!({ "apply": { "key": "again" } })).await;
    assert_eq!(again["over"], Value::Null);

    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;
    let heard: Vec<&Value> = shown["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|one| one["blueprint"] == "caller")
        .map(|one| &one["state"]["heard"])
        .collect();
    assert_eq!(heard, [&json!("first"), &json!("first")], "{shown}");
    // It ran in the sandbox's own project, and nothing was applied in the factory's.
    assert!(shown["instances"].as_array().unwrap().iter().all(|one| one["project"] == "sandbox"));
    assert_eq!(rig.read("/api/instances").await["instances"], json!([]));
}

#[tokio::test]
async fn there_are_eight_sandboxes_at_once_and_a_ninth_is_refused_in_a_sentence_until_one_has_ended() {
    let rig = Rig::new().await;
    let text = "blueprint: quiet\nsteps:\n  ready: { do: noop }\n";
    let mut ids = Vec::new();
    for _ in 0..8 {
        ids.push(made(&rig, json!({ "source": text })).await);
    }

    let (status, ninth) = rig.post("/api/sandboxes", json!({ "source": text })).await;

    assert_eq!(status, 503, "{ninth}");
    let said = ninth["error"]["message"].as_str().unwrap();
    assert!(
        said.contains("8 sandbox tests are under way here") && said.contains("End one, or wait"),
        "{said}"
    );
    assert_eq!(rig.node().app.sandboxes.count(), 8);
    rig.call(Method::DELETE, &format!("/api/sandboxes/{}", ids[3]), Some(&rig.token), None).await;
    assert_eq!(rig.post("/api/sandboxes", json!({ "source": text })).await.0, 200);
}

#[tokio::test]
async fn a_sandbox_nobody_asks_anything_for_a_quarter_of_an_hour_is_ended_and_one_that_is_asked_is_kept() {
    let clock = ManualClock::new();
    let theirs = clock.clone();
    let rig =
        Rig::owned(Rig::with("api-sandboxes", move |config, _| config.clock = Some(Arc::new(theirs))).await)
            .await;
    let id = made(&rig, json!({ "source": ASKS })).await;
    let path = format!("/api/sandboxes/{id}");
    let folder = rig.dir.join("data").join("sandboxes").join(&id);
    assert!(folder.is_dir());

    // Asked within the while, it begins again: twenty-eight minutes in
    // all, and it is there.
    clock.advance(Duration::from_secs(14 * 60));
    assert_eq!(rig.get(&path).await.0, 200);
    // Whoever keeps it has heard that it was asked before the clock goes on.
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    clock.advance(Duration::from_secs(14 * 60));
    assert_eq!(rig.get(&path).await.0, 200);
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }

    clock.advance(Duration::from_secs(15 * 60));

    // Ended by its keeper, which takes the folder sandboxes are in away once
    // the sandbox has said it ended: a moment after its own folder is gone
    // and its thread with it, and nothing waits on the keeper to have done
    // so. So what is waited for is that folder too, as the factory with no
    // sandbox has none.
    let sandboxes = rig.dir.join("data").join("sandboxes");
    soon("the sandbox to be ended for having been asked nothing, and nothing of it on the disk", async {
        while rig.node().app.sandboxes.count() > 0 || folder.exists() || sandboxes.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let (status, gone) = rig.get(&path).await;
    assert_eq!(status, 404, "{gone}");
    assert!(gone["error"]["message"].as_str().unwrap().contains("asked nothing for 15 minutes"), "{gone}");
    thread_gone(&id).await;
}

#[tokio::test]
async fn a_daemon_that_stops_ends_every_sandbox_and_one_that_starts_takes_away_what_a_killed_one_left() {
    let mut rig = Rig::new().await;
    let first = made(&rig, json!({ "source": ASKS })).await;
    let second = made(&rig, json!({ "name": "std/board" })).await;
    let folder = rig.dir.join("data").join("sandboxes");
    assert!(folder.join(&first).is_dir() && folder.join(&second).is_dir());

    rig.stop().await;

    // Stopped is stopped: their folders are gone and their threads ended.
    assert!(
        !folder.exists(),
        "a stopped daemon left {:?}",
        std::fs::read_dir(&folder).map(|read| read.count())
    );
    thread_gone(&first).await;
    thread_gone(&second).await;
    // A daemon that was killed in the middle of one leaves its folder,
    // and the next to start takes it away.
    let left = folder.join("sandbox_left_behind").join("projects").join("sandbox");
    std::fs::create_dir_all(&left).unwrap();
    std::fs::write(left.join("SANDBOX.md"), "left by a daemon that was killed").unwrap();

    rig.start().await;

    assert!(!folder.exists());
    // Nothing of a sandbox is found by a later daemon.
    assert_eq!(rig.get(&format!("/api/sandboxes/{first}")).await.0, 404);
    assert_eq!(rig.node().app.sandboxes.count(), 0);
}

/// A package that brings a blueprint, the words it links and an action
/// that runs its program, which would write a file if it were run.
fn package(would: &std::path::Path) -> String {
    format!(
        r#"
extension: reach
title: Reach
description: Not a real product.
program: {program}
actions:
  out:
    description: Reach out.
    args: ["write", "{would}", "ran"]
    out:
      said: ${{ trim(answer) }}
blueprints:
  ask: |
    blueprint: reach/ask
    version: 1
    permissions: [extensions:reach]
    steps:
      words: {{ do: text.put, with: {{ parts: [{{ from: ask.md, with: {{ item: shop-12 }} }}] }} }}
      out: {{ do: reach.out }}
    outputs:
      text: ${{ steps.words.out.text }}
prompts:
  ask.md: |
    Look at ${{item}}.
"#,
        program = crate::STANDIN,
        would = would.display(),
    )
}

#[tokio::test]
async fn a_package_s_blueprint_is_run_in_a_sandbox_with_the_words_it_links_and_a_stand_in_for_its_action() {
    let rig = Rig::new().await;
    let would = rig.dir.join("reached");
    rig.send("/api/extensions", json!({ "source": package(&would) })).await;

    let id = made(&rig, json!({ "name": "reach/ask", "answers": "default" })).await;

    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;
    assert_eq!(shown["runs"][0]["status"], "ok", "{shown}");
    // The words it links were read with it, as the package has them.
    assert_eq!(shown["runs"][0]["outputs"]["text"], "Look at shop-12.", "{shown}");
    // Its action is a stand-in under what the package says of it, and its program was not run.
    assert_eq!(shown["actions"]["reach.out"], "stood");
    assert_eq!(shown["asked"][0]["action"], "reach.out");
    assert_eq!(shown["asked"][0]["summary"], "Reach out.");
    assert!(!would.exists(), "the package's program was run");
    // The package taken away, the sandbox goes on with what it read:
    // nothing of it asks the factory's packages anything.
    let removed = rig.call(Method::DELETE, "/api/extensions/reach", Some(&rig.token), None).await;
    assert_eq!(removed.0, 200, "{}", removed.1);
    let again = rig.send(&format!("/api/sandboxes/{id}/acts"), json!({ "apply": { "key": "again" } })).await;
    assert_eq!(again["over"], Value::Null, "{again}");
    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;
    assert_eq!(shown["runs"][1]["outputs"]["text"], "Look at shop-12.", "{shown}");
    assert!(!would.exists());
}

#[tokio::test]
async fn a_fleet_run_in_a_sandbox_tells_the_made_up_person_of_a_card_left_and_nobody_in_the_factory_in_use() {
    let rig = Rig::new().await;

    // The fleet as it ships, with what it attaches to tell somebody of
    // what waits for a person, over a board with a card made ready.
    let id = made(&rig, json!({ "name": "std/fleet", "first": ["std/board"], "answers": "default" })).await;
    let acts = format!("/api/sandboxes/{id}/acts");
    let card = json!({ "do": { "action": "board.add", "with": { "title": "Try it", "status": "ready" } } });
    let added = rig.send(&acts, card).await;
    assert!(added["over"].is_null() && added["waiting"] == json!([]), "{added}");

    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;

    // Nobody is there to take it (an agent is stood in), so it is left,
    // and whoever applied the fleet is told: in a sandbox, the made-up
    // person, whose inbox is handed back with the tables and channels.
    assert_eq!(shown["actions"]["inbox.tell"], "kept", "{}", shown["actions"]);
    assert_eq!(shown["actions"]["tells.whom"], "own");
    let told = shown["inbox"].as_array().unwrap();
    assert_eq!(told.len(), 1, "{}", shown["inbox"]);
    assert_eq!(told[0]["why"], "left");
    assert_eq!(told[0]["owner"], shown["somebody"]);
    assert!(told[0]["says"].as_str().unwrap().contains("left for a person"), "{}", told[0]);
    let failed: Vec<&Value> =
        shown["instances"].as_array().unwrap().iter().filter(|one| one["status"] == "failed").collect();
    assert_eq!(failed, Vec::<&Value>::new());
    // The made-up person is nobody here, and nobody here was told.
    let me = rig.node().app.studio.identity.authenticate(&rig.token).await.unwrap();
    assert_ne!(json!(me.actor()), shown["somebody"]);
    assert_eq!(rig.read("/api/inbox?all=true").await["lines"], json!([]));
    rig.call(Method::DELETE, &format!("/api/sandboxes/{id}"), Some(&rig.token), None).await;
    thread_gone(&id).await;
}
