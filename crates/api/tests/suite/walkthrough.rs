//! The product's first-run story, end to end over HTTP: sign up, open a
//! channel, bring in an agent, have it set up a project and its fleet, open
//! the fleet blueprint as text and as a graph, start the fleet, and move a
//! card across the board.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::{Rig, soon};

fn fleet_file() -> String {
    format!("@blueprints/{}", brand::blueprint_file("fleet"))
}

/// What an agent or a person said in a channel, as `author: body`.
async fn transcript(rig: &Rig, channel: &str) -> Vec<String> {
    let messages = rig.read(&format!("/api/channels/{channel}/messages")).await;
    messages["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|message| {
            format!("{}: {}", message["author_name"].as_str().unwrap(), message["body"].as_str().unwrap())
        })
        .collect()
}

#[tokio::test]
async fn from_signing_up_to_a_running_fleet() {
    let rig = Rig::new().await;

    // "User starts their first channel and brings in an agent."
    rig.send("/api/channels", json!({ "name": "general" })).await;
    let brought = rig.send("/api/channels/general/agents", json!({ "name": "helper" })).await;
    assert_eq!(
        (brought["agent"]["name"].as_str(), brought["agent"]["adapter"].as_str()),
        (Some("helper"), Some("builtin"))
    );
    assert_eq!(brought["channel"]["members"].as_array().unwrap().len(), 2);

    // "User asks the agent to set up a new project, Riverside Shop."
    rig.send(
        "/api/channels/general/messages",
        json!({ "body": "Please set up a new project Riverside Shop" }),
    )
    .await;
    rig.count("agent.turn.finished", 1).await;
    let projects = rig.read("/api/projects").await;
    assert_eq!(projects["projects"][0]["id"], "riverside-shop");

    // "The agent scaffolds the files in the resources tab."
    let paths = |tree: &Value| -> Vec<String> {
        tree["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["path"].as_str().unwrap().to_string())
            .collect()
    };
    let tree = rig.read("/api/projects/riverside-shop/tree").await;
    assert!(paths(&tree).contains(&"docs/overview.md".to_string()), "{:?}", paths(&tree));
    let at = format!("/api/projects/riverside-shop/file?path={}", fleet_file());
    assert_eq!(rig.call(Method::GET, &at, Some(&rig.token), None).await.0, StatusCode::NOT_FOUND);

    // "You can then ask the agent to set up your fleet, which scaffolds fleet.bp under the project."
    rig.send("/api/channels/general/messages", json!({ "body": "Now set up your fleet for Riverside Shop" }))
        .await;
    rig.count("agent.turn.finished", 2).await;
    assert_eq!(rig.call(Method::GET, &at, Some(&rig.token), None).await.0, StatusCode::OK);
    let said = transcript(&rig, "general").await;
    assert!(said[1].starts_with("helper: Created the project **Riverside Shop**"), "{said:?}");
    assert!(said[3].contains("is kept for the project now"), "{said:?}");

    // "User clicks on fleet.bp: a doc that looks like a yml file..."
    let file = rig.read(&at).await;
    let source = file["content"].as_str().unwrap();
    assert!(source.contains("blueprint: fleet") && source.contains("params:") && source.contains("steps:"));

    // "...and in the node view we see a DAG of the same file."
    let checked =
        rig.send("/api/blueprints/check", json!({ "source": source, "project": "riverside-shop" })).await;
    assert_eq!(checked["ok"], true, "{}", checked["diagnostics"]);
    assert_eq!(
        checked["diagnostics"],
        json!([]),
        "the scaffolded fleet checks clean against everything on this device"
    );
    let nodes: Vec<&str> = checked["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap())
        .collect();
    // The file says where the fleet is, and is one step: the fleet that
    // ships, called for the project. What that does is `std/fleet`'s own graph.
    assert_eq!(nodes, ["main/fleet"]);
    let first = &checked["graph"]["nodes"][0];
    assert_eq!((first["target"].as_str(), first["kind"].as_str()), (Some("std/fleet"), Some("call")));
    let whole = rig.read("/api/blueprints/source?name=std/fleet").await;
    let whole = rig
        .send("/api/blueprints/check", json!({ "source": whole["source"], "project": "riverside-shop" }))
        .await;
    let steps: Vec<&str> =
        whole["graph"]["nodes"].as_array().unwrap().iter().map(|node| node["id"].as_str().unwrap()).collect();
    for expected in
        ["main/check", "main/agents", "main/stages", "main/arranged", "on.changed/again", "on.trouble/run"]
    {
        assert!(steps.contains(&expected), "the fleet's graph has no `{expected}`: {steps:?}");
    }
    assert!(first["line"].as_u64().unwrap() > 1, "each node knows where it is in the text");

    // Start the fleet.
    let applied = rig
        .send("/api/apply", json!({ "project": "riverside-shop", "path": fleet_file(), "wait": true }))
        .await;
    assert_eq!(applied["finished"]["status"], "ok", "{}", applied["finished"]);
    let instance = rig.read(&format!("/api/instances/{}", applied["instance"].as_str().unwrap())).await;
    assert_eq!(
        (instance["instance"]["status"].as_str(), instance["instance"]["project"].as_str()),
        (Some("active"), Some("riverside-shop"))
    );
    assert_eq!(instance["graph"]["name"], "fleet");
    let said = transcript(&rig, "riverside-shop").await;
    assert_eq!(
        said,
        [
            "conductor: The fleet for riverside-shop is up. riverside-shop-architect-1 is here to talk a change \
             over with, and one agent works cards.\n\nSay what you want changed in this channel and \
             riverside-shop-architect-1 answers; asked for the plan of a change, it writes one whose tickets \
             become cards. Or add a card to the board yourself and move it to ready: it is worked when its \
             turn comes.\n\nWho is in the fleet and what they do with a card are two tables of the project, \
             `agents` and `stages`: change a row of either and the fleet follows.\n\nWrite `@conductor help` \
             here for what I take as a command."
        ]
    );
    // It names the architect and asks it nothing: nobody was given a turn by being told of.
    let asked = rig.read("/api/agents/riverside-shop-architect-1/turns").await;
    assert_eq!(asked["turns"].as_array().unwrap().len(), 0, "{asked}");
    let agents = rig.read("/api/agents").await;
    let names: Vec<&str> =
        agents["agents"].as_array().unwrap().iter().map(|agent| agent["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        ["helper", "riverside-shop-architect-1", "riverside-shop-worker-1"],
        "the fleet brought in its own agents"
    );

    // Give the fleet's worker something that can do work (in the tests, a
    // stand-in program; on a real device, an agent CLI).
    rig.send("/api/agents", json!({ "name": "riverside-shop-worker-1", "adapter": "worker" })).await;

    // A card that becomes ready is worked, and taken to review.
    rig.board("riverside-shop").await;
    let card = rig
        .send(
            "/api/board/riverside-shop/items",
            json!({ "title": "Add a cart", "body": "Customers need one." }),
        )
        .await;
    let id = card["id"].as_str().unwrap();
    let (status, moved) = rig
        .call(
            reqwest::Method::PATCH,
            &format!("/api/board/items/{id}"),
            Some(&rig.token),
            Some(json!({ "status": "ready" })),
        )
        .await;
    assert_eq!((status, moved["status"].as_str()), (StatusCode::OK, Some("ready")));
    rig.event("board.item.moved", |event| event.data["to"] == "review").await;
    rig.event("run.finished", |event| {
        event.data["blueprint"] == "std/working" && event.data["flow"] == "on.go"
    })
    .await;

    // The fleet is there, and with nobody to review, nothing is running
    // for the card: it is a row in review.
    let board = rig.read("/api/board/riverside-shop").await;
    assert_eq!(board["items"][0]["status"], "review");
    let lanes = rig.read("/api/instances?live=true&project=riverside-shop").await;
    let blueprints: Vec<&str> =
        lanes["instances"].as_array().unwrap().iter().map(|i| i["blueprint"].as_str().unwrap()).collect();
    assert!(blueprints.contains(&"fleet"), "{blueprints:?}");
    let for_the_card: Vec<&Value> = lanes["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|one| one["key"].as_str().is_some_and(|key| key.ends_with(id)))
        .collect();
    assert_eq!(for_the_card, [] as [&Value; 0], "nothing is kept running for a card nobody is working");
    let said = transcript(&rig, "riverside-shop").await;
    assert_eq!(
        said[1], "riverside-shop-worker-1: I did the work.",
        "the agent said what it did, in the channel: {said:?}"
    );
    let messages = rig.read("/api/channels/riverside-shop/messages").await;
    assert_eq!(
        messages["messages"][1]["author"], "agent:riverside-shop-worker-1",
        "in its own name, not the lane's"
    );

    // Finished, the card says so, and the stage that worked it tidies
    // its place away.
    rig.call(
        reqwest::Method::PATCH,
        &format!("/api/board/items/{id}"),
        Some(&rig.token),
        Some(json!({ "status": "done" })),
    )
    .await;
    rig.event("card.finished", |event| event.data["stage"] == "done").await;
    rig.event("run.finished", |event| {
        event.data["blueprint"] == "std/working" && event.data["flow"] == "on.tidied"
    })
    .await;

    // All of it can be found, and all of it was measured.
    rig.node().app.studio.search.catch_up().await.unwrap();
    let found = rig.read("/api/search?q=cart").await;
    let kinds: Vec<&str> =
        found["hits"].as_array().unwrap().iter().map(|hit| hit["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"card"), "{kinds:?}");
    let found = rig.read("/api/search?q=did the work&kind=message").await;
    assert_eq!(found["hits"][0]["kind"], "message", "what was said in channels is searchable too: {found}");
    let steps = rig.read("/api/metrics?name=step&by=action").await;
    let actions: Vec<&str> =
        steps["series"].as_array().unwrap().iter().map(|s| s["labels"]["action"].as_str().unwrap()).collect();
    for expected in
        ["system.check", "channel.ensure", "agent.ensure", "agent.task", "board.move", "std/lane-round"]
    {
        assert!(actions.contains(&expected), "no measurement for `{expected}`: {actions:?}");
    }
    let http = rig.read("/api/metrics?name=http&by=route,caller").await;
    assert!(
        http["series"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["labels"]["route"] == "/api/apply" && s["labels"]["caller"] == "cli")
    );
    let turns = rig.read("/api/metrics?name=agent.turn&by=slot").await;
    assert_eq!(turns["series"].as_array().unwrap().len(), 2, "both agents' turns: {turns}");
    // All of that is told apart by what each measurement says of itself.
    // None of it is a label, which only a blueprint gives: a name that
    // turns up here is missing from the names labels may not have.
    assert_eq!(rig.read("/api/metrics/labels").await, json!({ "labels": {} }));

    // A project can be asked about by itself: what ran for it, and how it went.
    let count = |answer: &Value| answer["series"][0]["total"]["count"].as_u64().unwrap_or(0);
    let all = count(&rig.read("/api/metrics?name=run").await);
    let ours = count(&rig.read("/api/metrics?name=run&labels=project:riverside-shop").await);
    assert!(ours > 0 && ours <= all, "{ours} of {all} runs were for the project");
    assert_eq!(count(&rig.read("/api/metrics?name=run&labels=project:nonesuch").await), 0);
    let runs = rig.read("/api/runs?project=riverside-shop").await;
    let instances = rig.read("/api/instances?project=riverside-shop").await;
    let known: Vec<&str> =
        instances["instances"].as_array().unwrap().iter().map(|one| one["id"].as_str().unwrap()).collect();
    let listed = runs["runs"].as_array().unwrap();
    assert!(!listed.is_empty(), "the project's runs are listed: {runs}");
    assert!(listed.iter().all(|run| known.contains(&run["instance"].as_str().unwrap())), "{runs}");
    assert_eq!(rig.read("/api/runs?project=nonesuch").await["runs"], json!([]));
}

#[tokio::test]
async fn a_channel_takes_people_agents_and_bots_and_a_bot_answers_when_it_is_named() {
    let rig = Rig::new().await;
    let channel = rig.send("/api/channels", json!({ "name": "general" })).await;
    let id = channel["id"].as_str().unwrap().to_string();
    let actors = |channel: &Value| -> Vec<String> {
        channel["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["actor"].as_str().unwrap().to_string())
            .collect()
    };
    let me = actors(&channel).remove(0);

    // People: those with an account here can be added; nobody else can.
    let people = rig.read("/api/people").await;
    assert_eq!(people["people"], json!([{ "actor": me, "name": "Ada" }]), "names, and no addresses");
    let again = rig.send(&format!("/api/channels/{id}/members"), json!({ "actor": me })).await;
    assert_eq!(actors(&again), std::slice::from_ref(&me), "adding someone who is there changes nothing");
    let stranger = rig
        .call(
            reqwest::Method::POST,
            &format!("/api/channels/{id}/members"),
            Some(&rig.token),
            Some(json!({ "actor": "user:nobody" })),
        )
        .await;
    assert_eq!(stranger.0, 404, "{}", stranger.1);

    // A bot is a blueprint applied to the channel. It joins under its name.
    // Its command names a program that reads what was said from where it is handed: two variables.
    let says = crate::standin::line(&["put", "heard ", "var", "MESSAGE", "put", " from ", "var", "AUTHOR"]);
    let params = json!({ "channel": "general", "name": "Parrot", "run": says });
    let applied = rig
        .send(
            "/api/apply",
            json!({ "blueprint": "std/bot", "key": "bot-parrot", "params": params, "wait": true }),
        )
        .await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let with_bot = rig.read(&format!("/api/channels/{id}")).await;
    assert_eq!(actors(&with_bot), [me.clone(), "bot:parrot".to_string()]);

    // Said to nobody, it stays quiet; named, it answers as itself, to that message.
    rig.send(&format!("/api/channels/{id}/messages"), json!({ "body": "morning all" })).await;
    let asked =
        rig.send(&format!("/api/channels/{id}/messages"), json!({ "body": "@parrot; rm -rf $HOME" })).await;
    let answer = rig.event("channel.message", |event| event.data["author"] == "bot:parrot").await;
    assert_eq!(answer.data["author_name"], "Parrot");
    assert_eq!(
        answer.data["body"], "heard @parrot; rm -rf $HOME from Ada",
        "what was said is given, never run"
    );
    assert_eq!(answer.data["reply_to"], asked["id"]);
    let said = rig.read(&format!("/api/channels/{id}/messages")).await;
    assert_eq!(said["messages"].as_array().unwrap().len(), 3, "one answer, and none to itself: {said}");

    // Asked under a message, it answers under it; and a thread is read by itself.
    let under = rig
        .send(
            &format!("/api/channels/{id}/messages"),
            json!({ "body": "@parrot and here?", "thread": asked["id"] }),
        )
        .await;
    assert_eq!(under["thread"], asked["id"]);
    let answer = rig
        .event("channel.message", |event| {
            event.data["author"] == "bot:parrot" && event.data["reply_to"] == under["id"]
        })
        .await;
    assert_eq!(answer.data["thread"], asked["id"], "{}", answer.data);
    let thread =
        rig.read(&format!("/api/channels/{id}/messages?thread={}", under["id"].as_str().unwrap())).await;
    let bodies: Vec<&str> =
        thread["messages"].as_array().unwrap().iter().map(|m| m["body"].as_str().unwrap()).collect();
    assert_eq!(bodies, ["@parrot; rm -rf $HOME", "@parrot and here?", "heard @parrot and here? from Ada"]);
    let said = rig.read(&format!("/api/channels/{id}/messages")).await;
    let shown = said["messages"].as_array().unwrap();
    assert_eq!(shown.len(), 3, "the channel itself is as it was: {said}");
    assert_eq!(shown[1]["replies"], 2, "and says how much is under the message");

    // Stopped, it is no longer in the channel.
    let instance = applied["instance"].as_str().unwrap();
    rig.send(&format!("/api/instances/{instance}/stop"), json!({})).await;
    rig.event("channel.left", |event| event.data["member"] == "bot:parrot").await;
    assert_eq!(actors(&rig.read(&format!("/api/channels/{id}")).await), [me]);
}

#[tokio::test]
async fn a_bot_given_no_command_says_back_what_was_said_and_runs_nothing() {
    let rig = Rig::new().await;
    let channel = rig.send("/api/channels", json!({ "name": "general" })).await;
    let id = channel["id"].as_str().unwrap().to_string();
    let applied = rig
        .send(
            "/api/apply",
            json!({ "blueprint": "std/bot", "key": "bot-echo", "params": { "channel": "general", "name": "Echo" }, "wait": true }),
        )
        .await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");

    rig.send(&format!("/api/channels/{id}/messages"), json!({ "body": "@echo is $HOME anywhere; `id`?" }))
        .await;
    let answer = rig.event("channel.message", |event| event.data["author"] == "bot:echo").await;
    assert_eq!(answer.data["body"], "You said: @echo is $HOME anywhere; `id`?", "as it was written");
    // The answer is the blueprint's own: no command was started for it.
    // (What the runner says of steps is passed on and not kept, and is
    // read here as what was said.)
    let started = journal::EventQuery { topics: "step.started".into(), limit: 1000, ..Default::default() };
    let started = rig.said(started).await;
    let steps: Vec<&str> = started.iter().filter_map(|event| event.source.step.as_deref()).collect();
    assert!(steps.contains(&"say") && !steps.contains(&"answer"), "{steps:?}");
}

#[tokio::test]
async fn every_blueprint_that_ships_checks_clean_against_everything_on_the_device() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/projects/shop/fleet", json!({})).await;

    let mut sources: Vec<(String, String)> = Vec::new();
    for name in rig.read("/api/blueprints").await["names"].as_array().unwrap() {
        let name = name.as_str().unwrap();
        let source = rig.read(&format!("/api/blueprints/source?name={name}")).await["source"]
            .as_str()
            .unwrap()
            .to_string();
        sources.push((name.to_string(), source));
    }
    assert!(
        sources.iter().any(|(name, _)| name == "std/slot") && sources.len() >= 3,
        "{:?}",
        sources.iter().map(|s| &s.0).collect::<Vec<_>>()
    );
    let fleet = rig.read(&format!("/api/projects/shop/file?path={}", fleet_file())).await;
    sources.push(("the fleet template".into(), fleet["content"].as_str().unwrap().to_string()));
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/blueprints");
    for entry in std::fs::read_dir(examples).unwrap().flatten() {
        sources.push((
            entry.file_name().to_string_lossy().into_owned(),
            std::fs::read_to_string(entry.path()).unwrap(),
        ));
    }

    for (name, source) in sources {
        let checked = rig.send("/api/blueprints/check", json!({ "source": source, "project": "shop" })).await;
        assert_eq!(checked["diagnostics"], json!([]), "`{name}` has problems");
        // A rule is asked and never run: it has no steps to draw. There are two.
        assert!(
            name == agents::rule::RULE
                || name == agents::dispatch::RULE
                || checked["graph"]["nodes"].as_array().is_some_and(|nodes| !nodes.is_empty()),
            "`{name}` draws nothing"
        );
    }

    // And the catalog they were checked against describes itself.
    let catalog = rig.read("/api/actions").await;
    let names: Vec<&str> =
        catalog["actions"].as_array().unwrap().iter().map(|a| a["name"].as_str().unwrap()).collect();
    for expected in [
        "wait",
        "emit",
        "shell",
        "service.start",
        "router.route",
        "system.check",
        "channel.post",
        "board.move",
        "fs.write",
        "agent.consult",
    ] {
        assert!(names.contains(&expected), "the catalog is missing `{expected}`");
    }
    let shell = catalog["actions"].as_array().unwrap().iter().find(|a| a["name"] == "shell").unwrap();
    assert_eq!(shell["capability"], "shell");
    assert!(shell["args"].as_array().unwrap().iter().any(|arg| arg["name"] == "run"));
}

#[tokio::test]
async fn an_editor_is_told_the_whole_language_in_one_read() {
    let rig = Rig::new().await;
    let language = rig.read("/api/blueprints/language").await;

    // Every mapping there is, each field with what goes after it and why.
    let shape = |name: &str| {
        language["shapes"].as_array().unwrap().iter().find(|shape| shape["name"] == name).cloned().unwrap()
    };
    let field = |shape: &Value, name: &str| {
        shape["fields"].as_array().unwrap().iter().find(|field| field["name"] == name).cloned().unwrap()
    };
    let step = shape("step");
    assert_eq!(field(&step, "do")["value"], json!({ "kind": "action" }));
    assert_eq!(field(&step, "retry")["value"], json!({ "kind": "shape", "of": "retry" }));
    assert_eq!(field(&step, "on_start")["value"], json!({ "kind": "list_of", "of": "hook" }));
    assert_eq!(field(&step, "on_fail")["value"], json!({ "kind": "list_of", "of": "failure_hook" }));
    assert_eq!(shape("failure_hook")["short"], json!({ "kind": "decision" }), "which may be a decision");
    assert_eq!(field(&step, "with")["value"], json!({ "kind": "arguments", "of": null }));
    assert!(field(&step, "timeout")["help"].as_str().unwrap().contains("runs longer"));
    assert_eq!(field(&shape("blueprint"), "steps")["value"], json!({ "kind": "map_of", "of": "step" }));
    assert_eq!(shape("hook")["short"], json!({ "kind": "action" }), "a hook may be an action's bare name");
    let scopes = field(&shape("handler"), "scope")["value"]["of"].clone();
    // What a handler hears where it says nothing is said first.
    let names: Vec<&str> =
        scopes.as_array().unwrap().iter().filter_map(|scope| scope["name"].as_str()).collect();
    assert_eq!(names, ["project", "fleet", "own", "target"]);

    // What this device can do, and what an expression can say.
    let shell = language["actions"].as_array().unwrap().iter().find(|a| a["name"] == "shell").unwrap();
    assert!(
        shell["args"].as_array().unwrap().iter().any(|arg| arg["name"] == "run" && arg["type"] == "string")
    );
    assert!(shell["outputs"].as_array().unwrap().iter().any(|output| output[0] == "code"));
    let event = language["names"].as_array().unwrap().iter().find(|name| name["name"] == "event").unwrap();
    assert!(event["members"].as_array().unwrap().iter().any(|member| member["name"] == "data"));
    assert!(language["step"].as_array().unwrap().iter().any(|member| member["name"] == "out"));
    assert!(language["functions"].as_array().unwrap().iter().any(|function| function["name"] == "slug"));
    assert_eq!(language["decisions"], json!(["retry", "continue", "fail"]));
}

/// The client's own tests of what it colours and offers are run against a
/// copy of the language, kept beside them. This holds the copy to what the
/// daemon really describes, so those tests cannot pass against a language
/// that no longer exists.
#[tokio::test]
async fn the_copy_of_the_language_the_client_is_tested_against_is_the_language() {
    let rig = Rig::new().await;
    let mut language = rig.read("/api/blueprints/language").await;
    // Which actions and events there are depends on the device; the client's tests bring their own.
    language.as_object_mut().unwrap().remove("actions");
    language.as_object_mut().unwrap().remove("events");
    let described = format!("{}\n", serde_json::to_string_pretty(&language).unwrap());

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/blueprint/language.json");
    if std::env::var_os("WRITE_LANGUAGE").is_some() {
        std::fs::write(path, &described).unwrap();
    }
    let kept = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        kept == described,
        "ui/src/blueprint/language.json is not the language as the daemon describes it. \
         Write it again with `WRITE_LANGUAGE=1 cargo test -p api the_copy_of_the_language`, \
         then run the client's tests"
    );
}

#[tokio::test]
async fn the_blueprints_there_are_are_listed_with_what_each_is_started_with() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let put = async |name: &str, content: &str| {
        let path = format!("@blueprints/{}", brand::blueprint_file(name));
        let body = json!({ "path": path, "content": content });
        let (status, answer) =
            rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
    };
    put(
        "deploy",
        "blueprint: deploy\ndescription: Put a version out.\nparams:\n  version:\n  port: 8080\nsteps:\n  go: { do: noop }\noutputs:\n  address: \"127.0.0.1:${params.port}\"\n",
    )
    .await;
    put("half", "blueprint: half\nsteps: [").await;

    let listed = rig.read("/api/blueprints?project=shop").await;
    let one = |name: &str| {
        listed["blueprints"].as_array().unwrap().iter().find(|one| one["name"] == name).cloned().unwrap()
    };
    let deploy = one("deploy");
    assert_eq!(deploy["description"], "Put a version out.");
    assert_eq!(deploy["params"][0]["name"], "version");
    assert_eq!(deploy["params"][0]["required"], true);
    assert_eq!(
        deploy["params"][1],
        json!({ "name": "port", "type": "int", "default": 8080, "required": false, "description": null, "choices": [] })
    );
    assert_eq!(deploy["outputs"], json!(["address"]));
    // The project's own are files in it; the ones that ship are not.
    assert_eq!(deploy["file"], format!("@blueprints/{}", brand::blueprint_file("deploy")));
    assert_eq!(one("std/board")["file"], Value::Null);
    // The ones that ship are there too, and one that does not read is
    // listed by its name alone.
    assert!(one("std/board")["params"].as_array().unwrap().iter().any(|param| param["name"] == "columns"));
    assert_eq!(one("half")["params"], json!([]));
}

#[tokio::test]
async fn a_blueprint_with_mistakes_is_refused_with_where_and_why() {
    let rig = Rig::new().await;
    let source = "blueprint: broken\nsteps:\n  build: { do: shel, with: { run: make } }\n  tell: { do: channel.post, with: { channel: general } }\n";
    let checked = rig.send("/api/blueprints/check", json!({ "source": source })).await;
    assert_eq!(checked["ok"], false);
    let found: Vec<(u64, &str)> = checked["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["severity"] == "error")
        .map(|d| (d["span"]["start"]["line"].as_u64().unwrap(), d["code"].as_str().unwrap()))
        .collect();
    assert_eq!(found, [(3, "unknown-action"), (4, "missing-argument")]);
    assert!(
        checked["graph"]["nodes"].as_array().unwrap().len() == 2,
        "a half-right blueprint is still drawn"
    );

    let (status, error) = rig.post("/api/apply", json!({ "source": source })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let message = error["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("broken.bp:3:3: error: there is no action called `shel`; did you mean `shell`?"),
        "{message}"
    );
    assert_eq!(rig.read("/api/instances").await["instances"], json!([]), "nothing was started");
}

#[tokio::test]
async fn the_event_stream_delivers_what_happens_and_resumes_where_it_left_off() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let client = reqwest::Client::new();
    let open = |last: Option<String>| {
        let mut request =
            client.get(format!("{}/api/events?topics=board.**", rig.url)).bearer_auth(&rig.token);
        if let Some(last) = last {
            request = request.header("last-event-id", last);
        }
        request.send()
    };
    /// Reads the stream until it has `count` events; returns their ids and topics.
    async fn take(response: &mut reqwest::Response, count: usize) -> Vec<(String, String)> {
        let mut text = String::new();
        loop {
            let frames: Vec<(String, String)> = text
                .split("\n\n")
                .filter(|frame| frame.contains("data:"))
                .filter_map(|frame| {
                    let line = |prefix: &str| {
                        frame.lines().find_map(|line| line.strip_prefix(prefix)).map(str::trim)
                    };
                    let data: Value = serde_json::from_str(line("data:")?).ok()?;
                    Some((line("id:")?.to_string(), data["topic"].as_str()?.to_string()))
                })
                .collect();
            if frames.len() >= count && text.ends_with("\n\n") {
                return frames;
            }
            let chunk =
                soon("more of the event stream", response.chunk()).await.unwrap().expect("the stream ended");
            text.push_str(&String::from_utf8_lossy(&chunk));
        }
    }

    rig.board("shop").await;
    let mut stream = open(None).await.unwrap();
    assert_eq!(stream.headers()["content-type"], "text/event-stream");
    rig.send("/api/board/shop/items", json!({ "title": "First" })).await;
    rig.send("/api/channels", json!({ "name": "noise" })).await;
    rig.send("/api/board/shop/items", json!({ "title": "Second" })).await;
    let seen = take(&mut stream, 2).await;
    assert_eq!(
        seen.iter().map(|(_, topic)| topic.as_str()).collect::<Vec<_>>(),
        ["board.item.created", "board.item.created"]
    );
    drop(stream);

    // While nobody is listening...
    rig.send("/api/board/shop/items", json!({ "title": "Third" })).await;
    // ...and a client that reconnects with the last id it saw gets exactly what it missed.
    let mut stream = open(Some(seen[0].0.clone())).await.unwrap();
    let missed = take(&mut stream, 2).await;
    assert_eq!(missed[0].0, seen[1].0, "it resumes after the event it named");
    // And then what was said meanwhile, which is another place again. An
    // id is a place to hand back and no number: nothing is worked out of it.
    assert!(missed[1].0 != seen[0].0 && missed[1].0 != seen[1].0, "{missed:?}");
    assert!(seen.iter().chain(&missed).all(|(id, _)| id.parse::<f64>().is_err()), "{seen:?} {missed:?}");
}

#[tokio::test]
async fn a_card_that_cannot_be_done_is_reported_and_not_sent_round_again() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/projects/shop/fleet", json!({})).await;
    let applied =
        rig.send("/api/apply", json!({ "project": "shop", "path": fleet_file(), "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok");

    // The fleet's worker is the built-in assistant, which cannot do real work.
    rig.board("shop").await;
    let card = rig.send("/api/board/shop/items", json!({ "title": "Add a cart" })).await;
    let id = card["id"].as_str().unwrap();
    rig.call(
        reqwest::Method::PATCH,
        &format!("/api/board/items/{id}"),
        Some(&rig.token),
        Some(json!({ "status": "ready" })),
    )
    .await;
    rig.event("channel.message", |event| {
        event.data["body"].as_str().is_some_and(|body| body.contains("failed"))
    })
    .await;

    let said = transcript(&rig, "shop").await;
    assert!(
        said[1].starts_with("conductor: Something in the fleet failed: shop-worker-1 could not do it"),
        "{said:?}"
    );
    assert!(
        said[1].contains(&format!("`{} record set shop-agents-2 adapter=<package>`", brand::CLI)),
        "and it says what to do about it, by the row that agent is of: {said:?}"
    );
    let board = rig.read("/api/board/shop").await;
    assert_eq!(board["items"].as_array().unwrap().len(), 1, "no stray cards");
    assert_eq!(board["items"][0]["status"], "doing", "left where it stopped, for a person to see");
    // The round that failed is what was reported. The stage that asked
    // for it then went on without failing in its turn, so nothing was said
    // twice, and nothing was asked for again.
    rig.event("run.finished", |event| {
        event.data["blueprint"] == "std/working" && event.data["flow"] == "on.go"
    })
    .await;
    let rounds = rig.read("/api/instances?project=shop").await;
    let round_count =
        rounds["instances"].as_array().unwrap().iter().filter(|i| i["blueprint"] == "std/lane-round").count();
    assert_eq!(round_count, 1, "one attempt, not a loop");
    let said = transcript(&rig, "shop").await;
    let reports = said.iter().filter(|line| line.contains("Something in the fleet failed")).count();
    assert_eq!(reports, 1, "{said:?}");
    let fleet = rig.read(&format!("/api/instances/{}", applied["instance"].as_str().unwrap())).await;
    assert_eq!(fleet["instance"]["status"], "active", "one lane failing does not take the fleet down");
}

#[tokio::test]
async fn a_projects_board_is_what_comes_of_applying_the_blueprint_that_ships_for_it() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;

    // A project has no board until a blueprint makes one, and says so.
    let (status, none) = rig.get("/api/board/shop").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(none["error"]["message"].as_str().unwrap().contains("apply `std/board`"), "{none}");
    assert_eq!(rig.read("/api/collections?project=shop").await["collections"], json!([]));
    let apply = |params: Value| {
        let body = json!({ "blueprint": "std/board", "project": "shop", "params": params, "wait": true });
        let rig = &rig;
        async move { rig.send("/api/apply", body).await }
    };

    // As it comes: the usual columns, and nothing on it.
    let applied = apply(json!({})).await;
    assert_eq!(applied["finished"]["status"], "ok", "{}", applied["finished"]);
    let board = rig.read("/api/board/shop").await;
    assert_eq!(board["columns"], json!(["backlog", "ready", "doing", "review", "done"]));
    assert_eq!(board["items"], json!([]));
    // The blueprint says where the cards may be kept, for whoever applies it to choose from.
    let listed = rig.read("/api/blueprints?project=shop").await;
    let params = listed["blueprints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|one| one["name"] == "std/board")
        .unwrap()["params"]
        .clone();
    let names: Vec<&str> =
        params.as_array().unwrap().iter().map(|param| param["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["columns", "store", "database"]);
    assert_eq!(params[1]["choices"], json!(["local", "files", "service"]));
    let card = rig.send("/api/board/shop/items", json!({ "title": "Sell mugs" })).await;
    assert_eq!(card["status"], "backlog", "a card starts in the first column");

    // A card that is added is a row of the board's table, and nothing is
    // started for it. What the cards of a board can do beyond being moved
    // is one blueprint for all of them, which the board started; what it
    // says of a card is in the card's history with the rest.
    let id = card["id"].as_str().unwrap().to_string();
    rig.event("collection.history", |event| event.data["record"] == id.as_str()).await;
    let running = rig.read("/api/instances").await;
    let mut running: Vec<&str> = running["instances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| one["blueprint"].as_str().unwrap())
        .collect();
    running.sort_unstable();
    assert_eq!(
        running,
        ["std/board", "std/cards"],
        "the board and what looks after its cards, and nothing for the card"
    );
    let said = |history: &Value| -> Vec<String> {
        history["history"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line["said"].as_str().unwrap().to_string())
            .collect()
    };
    let history = rig.read(&format!("/api/records/{id}/history")).await;
    assert_eq!(said(&history), ["Ada added it to backlog"], "{history}");

    // Moved to where cards are finished, that is said of it; the history has both.
    let path = format!("/api/board/items/{id}");
    rig.call(reqwest::Method::PATCH, &path, Some(&rig.token), Some(json!({ "status": "done" }))).await;
    rig.event("card.finished", |event| event.data["card"] == id.as_str()).await;
    rig.event("collection.history", |event| {
        event.data["record"] == id.as_str() && event.data["topic"] == "card.finished"
    })
    .await;
    let history = rig.read(&format!("/api/records/{id}/history")).await;
    let lines = said(&history);
    assert!(lines.iter().any(|line| line.ends_with("moved it from backlog to done")), "{lines:?}");
    assert!(lines.contains(&"it reached done".to_string()), "{lines:?}");
    rig.call(reqwest::Method::PATCH, &path, Some(&rig.token), Some(json!({ "status": "backlog" }))).await;

    // It is a collection the blueprint declared, held by the instance that
    // came of applying it, and kept where the blueprint said.
    let declared = rig.read("/api/collections?project=shop").await;
    let made = &declared["collections"][0];
    assert_eq!(
        (&made["name"], &made["store"], &made["owner"]),
        (&json!("board"), &json!("local"), &applied["instance"])
    );
    let fields: Vec<&str> =
        made["fields"].as_array().unwrap().iter().map(|f| f["name"].as_str().unwrap()).collect();
    let declared = "after again asker assignee at body due fixes given labels moved part_of plan pressed \
                    priority question rounds rung sent status taken title waiting";
    assert_eq!(fields.join(" "), declared);
    // What is asked on a card is the board's own to keep, beyond a card's
    // six, and a change to it alone is no line of the card's history.
    let question = made["fields"].as_array().unwrap().iter().find(|f| f["name"] == "question").unwrap();
    assert_eq!(
        question,
        &json!({ "name": "question", "kind": "text", "of": [], "required": false, "quiet": true })
    );
    assert_eq!(
        made["views"],
        json!([
            { "name": "board", "kind": "board", "by": "status", "saved": false },
            { "name": "order", "kind": "order", "by": "after", "saved": false, "group": "part_of", "done": "status" },
        ]),
        "a board starts as a board, and as its cards by what comes first; the other ways of laying it out are saved by whoever looks at it"
    );
    // The blueprint also says what is kept of a card's journey: some events
    // in their own words, and the rest of what is said of a card as a group.
    let kept: Vec<(&str, bool)> = made["history"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| (rule["topic"].as_str().unwrap(), rule["says"].is_string()))
        .collect();
    assert!(
        kept.contains(&("board.item.moved", true)) && kept.contains(&("board.item.*", false)),
        "{kept:?}"
    );
    assert!(kept.contains(&("card.finished", true)) && kept.contains(&("card.*", false)), "{kept:?}");
    // And what a lane says of the card it took: which agent worked on it,
    // and that its branch was sent; and what is said by whatever proposes
    // that branch for merging.
    assert!(kept.contains(&("card.worked", true)), "{kept:?}");
    assert!(kept.contains(&("card.pushed", true)) && kept.contains(&("card.proposed", true)), "{kept:?}");
    // And what is said on the card itself, as on any record; and that
    // something was asked on it, and answered.
    assert!(
        kept.contains(&("record.commented", true))
            && kept.contains(&("card.asked", true))
            && kept.contains(&("card.answered", true)),
        "{kept:?}"
    );
    // And that it had to wait its turn, and that its turn came.
    assert!(kept.contains(&("card.waiting", true)) && kept.contains(&("card.taken", true)), "{kept:?}");
    // And that the cards it waited on are done.
    assert!(kept.contains(&("card.freed", true)), "{kept:?}");
    // And what an agent that looked the work over said of it.
    assert!(kept.contains(&("card.approved", true)), "{kept:?}");
    // And that somebody said how it is to be done, where a fleet plans.
    assert!(kept.contains(&("card.planned", true)), "{kept:?}");
    // And that its agent went on with it after the daemon was stopped and started again.
    assert!(kept.contains(&("card.went_on", true)), "{kept:?}");
    // And that what was changed could not be committed.
    assert!(kept.contains(&("card.unkept", true)), "{kept:?}");
    // And that a card passed by hand had its branch merged for that, or had not.
    assert!(kept.contains(&("card.merged", true)) && kept.contains(&("card.unmerged", true)), "{kept:?}");
    // And that it was left for a person, and that it waits for one where nobody is there (`waited.rs`).
    assert!(kept.contains(&("card.left", true)) && kept.contains(&("card.waits", true)), "{kept:?}");
    // And that it was given back from the Trash, said as that and not as added; and what is attached, and taken away.
    let said =
        ["board.item.created.restored", "record.attached", "record.attached.restored", "record.detached"];
    assert!(said.iter().all(|topic| kept.contains(&(*topic, true))), "{kept:?}");
    // Pressed, run long, an A/B test, asked for and declined, a number come to a word, and a later card
    // that reverted it or says it fixes it (`pressed.rs`, `slowed.rs`, `contested.rs`, `asked.rs`,
    // `prioritised.rs`, `aftermath.rs`).
    assert_eq!(kept.len(), 37);
    let instance = rig.read(&format!("/api/instances/{}", applied["instance"].as_str().unwrap())).await;
    assert_eq!(instance["instance"]["holds"], json!(["collection:board"]));

    // The same again changes nothing, and the card is where it was.
    apply(json!({})).await;
    assert_eq!(rig.read("/api/board/shop").await["items"].as_array().unwrap().len(), 1);

    // Other columns, and the board follows the blueprint; but a column
    // with a card still in it is not taken away from under the card.
    let other = json!({ "columns": ["to do", "doing", "done"] });
    let refused = apply(other.clone()).await;
    assert_eq!(refused["finished"]["status"], "failed", "{}", refused["finished"]);
    assert!(
        refused["finished"]["error"]
            .as_str()
            .unwrap()
            .contains("`backlog` is still the `status` of 1 record(s)"),
        "{}",
        refused["finished"]
    );
    let id = card["id"].as_str().unwrap();
    rig.call(
        Method::PATCH,
        &format!("/api/board/items/{id}"),
        Some(&rig.token),
        Some(json!({ "status": "doing" })),
    )
    .await;
    let applied = apply(other).await;
    assert_eq!(applied["finished"]["status"], "ok", "{}", applied["finished"]);
    let board = rig.read("/api/board/shop").await;
    assert_eq!(board["columns"], json!(["to-do", "doing", "done"]));
    assert_eq!(board["items"][0]["status"], "doing", "the card is where it was put");

    // Where the cards are kept is one of the places the blueprint names.
    let nowhere =
        json!({ "blueprint": "std/board", "project": "shop", "params": { "store": "moon" }, "wait": true });
    let (status, refused) = rig.post("/api/apply", nowhere).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let why = refused["error"]["message"].as_str().unwrap();
    assert!(why.contains("store") && why.contains("local") && why.contains("files"), "{why}");

    // Every definition the instance was given is kept, the latest first:
    // what it was applied with, by whom, and whether it held. The same
    // again was no change and is not there; the one that was refused is,
    // and says it was undone.
    let id = applied["instance"].as_str().unwrap();
    let history = rig.read(&format!("/api/instances/{id}/versions")).await;
    let versions = history["versions"].as_array().unwrap();
    let told: Vec<(u64, &str, Value)> = versions
        .iter()
        .map(|one| {
            (
                one["version"].as_u64().unwrap(),
                one["outcome"].as_str().unwrap(),
                one["params"]["columns"].clone(),
            )
        })
        .collect();
    assert_eq!(
        told,
        [
            (3, "applied", json!(["to do", "doing", "done"])),
            (2, "reverted", json!(["to do", "doing", "done"])),
            (1, "applied", json!(["backlog", "ready", "doing", "review", "done"])),
        ]
    );
    assert_eq!(
        (&versions[0]["by_name"], versions[0]["by"].as_str().unwrap().starts_with("user:")),
        (&json!("Ada"), true)
    );
    assert!(versions[0]["source"].as_str().unwrap().contains("blueprint: std/board"));
    assert!(versions[0]["applied"].as_i64().unwrap() >= versions[2]["applied"].as_i64().unwrap());
    let (status, _) = rig.get("/api/instances/in_nothing/versions").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Kept as files instead: one file a card, in the project.
    rig.send("/api/projects", json!({ "name": "Blog" })).await;
    let body =
        json!({ "blueprint": "std/board", "project": "blog", "params": { "store": "files" }, "wait": true });
    let applied = rig.send("/api/apply", body).await;
    assert_eq!(applied["finished"]["status"], "ok", "{}", applied["finished"]);
    rig.send("/api/board/blog/items", json!({ "title": "Write the first post" })).await;
    let card = rig.read("/api/projects/blog/file?path=board/1.md").await;
    assert!(card["content"].as_str().unwrap().ends_with("# Write the first post\n"), "{card}");
    assert_eq!(rig.read("/api/board/blog").await["items"][0]["title"], "Write the first post");
}

#[tokio::test]
async fn a_handler_under_the_fleet_that_fails_is_said_in_the_fleets_channel() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/projects/shop/fleet", json!({})).await;
    let applied =
        rig.send("/api/apply", json!({ "project": "shop", "path": fleet_file(), "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok");
    // The fleet itself is what the project's file calls: its handlers hear
    // what happens under it.
    let fleet = rig.event("instance.active", |event| event.data["outputs"].is_object()).await;
    let all = rig.read("/api/instances?live=true&project=shop").await;
    let fleet = all["instances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|one| one["blueprint"] == "std/fleet")
        .map(|one| one["id"].as_str().unwrap().to_string())
        .unwrap_or_else(|| panic!("no fleet under the project's own: {all}, after {fleet:?}"));
    let fleet = fleet.as_str();

    // A run of a handler somewhere under it fails. Its instance goes on
    // running and says nothing of it, so the fleet does: the reason, with
    // which handler of what it was kept on the message.
    let journal = &rig.node().app.journal;
    let failed = |flow: &str| json!({ "flow": flow, "blueprint": "std/working", "error": "step `claim` failed: no", "reason": "no" });
    let of = |flow: &'static str| {
        move |event: &kernel::Event| {
            event.data["said_by"]["detail"].as_str().is_some_and(|detail| detail.starts_with(flow))
        }
    };
    journal.emit("run.failed", kernel::Source::instance(fleet), failed("on.first")).await.unwrap();
    rig.event("channel.message", of("on.first")).await;
    // What fails as a whole is said as an instance failing, once, and not
    // here as well.
    journal.emit("run.failed", kernel::Source::instance(fleet), failed("main")).await.unwrap();
    journal.emit("run.failed", kernel::Source::instance(fleet), failed("on.back")).await.unwrap();
    rig.event("channel.message", of("on.back")).await;

    let said = rig.read("/api/channels/shop/messages").await;
    let handlers: Vec<(&str, &str)> = said["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["body"].as_str().unwrap().contains("failed"))
        .map(|message| (message["body"].as_str().unwrap(), message["said_by"]["detail"].as_str().unwrap()))
        .collect();
    assert_eq!(
        handlers,
        [
            ("Something in the fleet failed: no", "on.first of std/working: step `claim` failed: no"),
            ("Something in the fleet failed: no", "on.back of std/working: step `claim` failed: no"),
        ],
        "{said}"
    );
}
