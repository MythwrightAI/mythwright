//! Whose an event is: the project its data names, or that of the instance
//! that said it, and otherwise nobody's, the factory's own
//! (`conductor::whose`). What says no `scope` hears its own project's and
//! what is nobody's, so an event of a project's that did not say so would
//! be heard in every project.
//!
//! Each journey here is played in a factory with one project and then
//! everything that was said is asked whose it is: all of it is the
//! project's but what is listed as the factory's own.

use std::collections::{BTreeMap, BTreeSet};

use kernel::Source;
use serde_json::json;

use crate::Rig;
use crate::fleets::{add, add_row, rig, says, shop};
use crate::lanes::{move_card, moved};

/// What is said in a factory with one project and is no project's: what
/// is of the factory itself, of who is in it, of the one index
/// everything is searched by, of the workspace it has from when its
/// owner registered, which is above its projects, and of somebody's
/// inbox, which is theirs whatever a line of it is about.
const THE_FACTORYS_OWN: &[&str] = &["system.", "identity.", "search.", "workspace.", "inbox."];

/// Whose everything said so far is, by topic: the projects, and `nobody`.
/// What was passed on and not kept is among it: it is heard all the same.
async fn whose(rig: &Rig) -> BTreeMap<String, BTreeSet<String>> {
    let app = &rig.node().app;
    let query = journal::EventQuery { limit: 100_000, ..Default::default() };
    let mut all: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for event in rig.said(query).await {
        let whose = app.engine.whose(&event).await.unwrap_or_else(|| "nobody".to_string());
        all.entry(event.topic.clone()).or_default().insert(whose);
    }
    all
}

/// Holds everything said so far to being the shop's or the factory's
/// own, and hands back the topics that were the shop's.
async fn the_shops(rig: &Rig) -> BTreeSet<String> {
    let all = whose(rig).await;
    let astray: Vec<String> = all
        .iter()
        .filter(|(topic, whose)| {
            let own = THE_FACTORYS_OWN.iter().any(|family| topic.starts_with(family));
            // A channel that is nobody's (`general`) is said to be made
            // and joined as the first person comes.
            let general = topic.starts_with("channel.") && whose.contains("nobody");
            !own && !general && **whose != BTreeSet::from(["shop".to_string()])
        })
        .map(|(topic, whose)| format!("{topic}: {whose:?}"))
        .collect();
    assert!(
        astray.is_empty(),
        "said in a factory whose one project is the shop, and not the shop's:\n{}",
        astray.join("\n")
    );
    all.into_iter().filter(|(_, whose)| whose.contains("shop")).map(|(topic, _)| topic).collect()
}

/// Fails unless each of these was said, and was the shop's.
fn said(shops: &BTreeSet<String>, topics: &[&str]) {
    let missing: Vec<&&str> = topics.iter().filter(|topic| !shops.contains(**topic)).collect();
    assert!(missing.is_empty(), "never said as the shop's: {missing:?}\nwhat was: {shops:?}");
}

#[tokio::test]
async fn everything_said_and_kept_of_a_card_from_added_to_done_is_its_projects() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "done", 0).await;
    // A log attached to it, kept beside its records.
    let attach = format!("{}/api/records/{id}/attachments?name=tests.log", rig.url);
    let sent = rig.client.post(attach).bearer_auth(&rig.token).body("12 passed").send().await.unwrap();
    assert_eq!(sent.status(), reqwest::StatusCode::OK);
    // And one added and deleted, which is kept in the Trash.
    let gone = add(&rig, "Not this one", &[]).await;
    let path = format!("/api/board/items/{gone}");
    let (status, _) = rig.call(reqwest::Method::DELETE, &path, Some(&rig.token), None).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    // What is kept: every row of every file is where its table's line
    // says, and where the shop has a journal of its own that holds its
    // tables, its cards, what was said of them and its agents' turns.
    let held = crate::apart::rows_are_kept_where_their_lines_say(&rig).await;
    for (table, _) in studio::apart::KEPT_APART.iter().chain(agents::apart::KEPT_APART) {
        // Nothing in this journey asks for a change somebody may not make
        // (`told::asks`), whose rows are held where their line says all the same.
        if *table == "asked" {
            continue;
        }
        let rows = held.iter().find(|(project, kept, _)| project == "shop" && kept == table);
        let own = journal::apart() != Some(false);
        assert!(!own || rows.is_some_and(|(_, _, rows)| *rows > 0), "`{table}` of the shop: {held:?}");
    }
    let shops = the_shops(&rig).await;
    said(
        &shops,
        &[
            "board.item.created",
            "board.item.moved",
            "record.created",
            "board.item.updated",
            "card.worked",
            "review.due",
            "review.passed",
            "pool.granted",
            "fleet.arranged",
            "agent.slot.created",
            "agent.turn.queued",
            "agent.turn.started",
            "agent.turn.finished",
            "channel.created",
            "channel.joined",
            "channel.message",
            "instance.active",
            "run.finished",
            "trash.put",
            "record.attached",
        ],
    );
}

#[tokio::test]
async fn everything_said_of_a_feature_planned_and_accepted_is_its_projects() {
    let (rig, at) = rig().await;
    shop(&rig, json!({})).await;
    let architect =
        json!({ "title": "architect", "role": "architect", "adapter": "critic", "permission": "read" });
    add_row(&rig, "agents", architect).await;
    let asked = json!({ "blueprint": "std/features", "project": "shop", "params": { "project": "shop" }, "wait": true });
    assert_eq!(rig.send("/api/apply", asked).await["finished"]["status"], "ok");
    let plan = "First what a cart holds.\n\n```tickets\n[\n  { \"title\": \"The cart's model\" },\n  { \"title\": \"The cart's page\", \"after\": [1] }\n]\n```";
    says(&at, "shop-architect-1", plan, "propose");
    let fields = json!({ "fields": { "title": "A shopping cart", "body": "More than one thing.", "status": "backlog" } });
    let feature =
        rig.send("/api/collections/shop/features/records", fields).await["id"].as_str().unwrap().to_string();
    let of = feature.clone();
    rig.event("feature.planned", move |event| event.data["record"] == of.as_str()).await;
    let accepted = Some(json!({ "fields": { "status": "building" } }));
    let (status, answer) = rig
        .call(reqwest::Method::PATCH, &format!("/api/records/{feature}"), Some(&rig.token), accepted)
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    let of = feature.clone();
    rig.event("feature.accepted", move |event| event.data["record"] == of.as_str()).await;
    let shops = the_shops(&rig).await;
    said(
        &shops,
        &[
            "feature.planned",
            "feature.accepted",
            "record.created",
            "agent.turn.finished",
            "board.item.created",
        ],
    );
}

#[tokio::test]
async fn what_is_said_in_a_projects_channel_and_an_agents_answer_there_are_its_projects() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    // Brought into the project's channel, an agent is the project's.
    let helper = json!({ "name": "helper", "adapter": "writer" });
    let brought = rig.send("/api/channels/shop/agents", helper).await;
    assert_eq!(brought["agent"]["project"], "shop", "{brought}");
    rig.send("/api/channels/shop/messages", json!({ "body": "@helper are you there?" })).await;
    let answer = rig.event("channel.message", |event| event.data["author"] == "agent:helper").await;
    assert_eq!(answer.data["project"], "shop", "{}", answer.data);
    rig.event("agent.turn.finished", |event| event.data["slot"] == "helper").await;
    // Each said of the agent that it was asked, began and ended, and each
    // of those names the project: none names an instance to go by.
    let app = &rig.node().app;
    for topic in ["agent.turn.queued", "agent.turn.started", "agent.turn.finished"] {
        let query = journal::EventQuery { topics: topic.to_string(), limit: 100, ..Default::default() };
        let turns = app.journal.read_every(query).await.unwrap();
        let turn = turns.iter().find(|event| event.data["slot"] == "helper").unwrap();
        assert_eq!(
            (turn.data["project"].as_str(), turn.source.instance.as_deref()),
            (Some("shop"), None),
            "{turn:?}"
        );
    }
    let shops = the_shops(&rig).await;
    said(
        &shops,
        &[
            "channel.message",
            "channel.joined",
            "agent.turn.queued",
            "agent.turn.started",
            "agent.turn.finished",
        ],
    );
}

#[tokio::test]
async fn a_comparison_of_a_projects_work_and_everything_said_while_it_counts_are_its_projects() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    let asked = json!({
        "name": "ways", "project": "shop", "of": "std/lane-round", "per": "item", "also": ["std/review"],
        "counts": "cards",
        "ways": [{ "name": "one" }, { "name": "other" }],
    });
    rig.send("/api/comparisons", asked).await;
    for title in ["Add a cart", "Add a till"] {
        let id = add(&rig, title, &[]).await;
        move_card(&rig, &id, "ready", None).await;
        moved(&rig, &id, "done", 0).await;
    }
    let shown = rig.read("/api/comparisons/ways").await;
    let counted =
        shown["measured"].as_array().unwrap().iter().any(|one| one["name"] == "system.comparison.came");
    assert!(counted, "{shown}");
    rig.send("/api/comparisons/ways/end", json!({})).await;
    // A comparison is said to start and to end with its project, so those
    // two are the project's though they are the system's to say.
    let all = whose(&rig).await;
    for topic in ["system.comparison.started", "system.comparison.ended"] {
        assert_eq!(all.get(topic), Some(&BTreeSet::from(["shop".to_string()])), "{topic}: {all:?}");
    }
    let shops = the_shops(&rig).await;
    said(&shops, &["board.item.moved", "card.worked", "review.passed", "agent.turn.finished"]);
}

#[tokio::test]
async fn an_event_of_a_projects_agent_or_channel_that_does_not_say_the_project_is_a_fault() {
    let (mut rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    let journal = rig.node().app.journal.clone();
    // As it is said, it is no fault.
    assert_eq!(rig.node().undeclared(), Vec::<String>::new());
    // Said with no project it would be nobody's, and heard by every
    // project's handlers that say no scope.
    journal.emit("agent.slot.stopped", Source::default(), json!({ "slot": "shop-worker-1" })).await.unwrap();
    let turn = json!({ "slot": "shop-worker-1", "turn": "t-1", "kind": "task", "project": "blog" });
    journal.emit("agent.turn.started", Source::default(), turn).await.unwrap();
    // Something said in the shop's channel, as it was said, but for whose it is.
    let query = journal::EventQuery { topics: "channel.message".to_string(), limit: 1, ..Default::default() };
    let mut message = journal.read_every(query).await.unwrap().remove(0).data;
    assert_eq!(message["project"], "shop", "{message}");
    message.as_object_mut().unwrap().remove("project");
    journal.emit("channel.message", Source::default(), message).await.unwrap();
    let faults = rig.node().undeclared();
    let about = |words: &str| faults.iter().any(|fault| fault.contains(words));
    assert!(
        about(
            "`agent.slot.stopped` is said of the agent `shop-worker-1`, which is in the project `shop`, and \
             says no `project`"
        ),
        "{faults:?}"
    );
    assert!(about("says `project: blog`"), "{faults:?}");
    assert!(about("`channel.message` is said of the channel `"), "{faults:?}");
    assert_eq!(faults.len(), 3, "{faults:?}");
    // The node is put away here and not by the rig, which would fail the
    // test for what it was made to say.
    rig.node.take().unwrap().stop().await;
}

/// A text with one handler that listens for a card being moved.
fn listens(name: &str, more: &str) -> String {
    format!(
        "blueprint: {name}\non:\n  moved:\n    event: board.item.moved\n{more}    steps:\n      said: {{ do: \
         log, with: {{ message: moved }} }}\n"
    )
}

/// What an apply of `source` for `project` warns of as never heard.
async fn warned(rig: &Rig, source: &str, project: Option<&str>) -> Vec<String> {
    let mut asked = json!({ "source": source });
    if let Some(project) = project {
        asked["project"] = json!(project);
    }
    // A warning and not a refusal: the apply went ahead.
    let applied = rig.send("/api/apply", asked).await;
    assert!(applied["instance"].is_string(), "{applied}");
    let unheard = |warnings: &serde_json::Value| -> Vec<String> {
        warnings
            .as_array()
            .into_iter()
            .flatten()
            .filter(|warning| warning["code"] == "unheard")
            .map(|warning| {
                assert_eq!(warning["severity"], "warning", "{warning}");
                assert!(warning["span"]["start"]["line"].as_u64().is_some_and(|line| line > 1), "{warning}");
                warning["message"].as_str().unwrap().to_string()
            })
            .collect()
    };
    let at_apply = unheard(&applied["warnings"]);
    // The check says the same where it is told which project, and
    // nothing of it where it is not: it does not know where the text
    // will be applied.
    let mut checking = json!({ "source": source });
    if let Some(project) = project {
        checking["project"] = json!(project);
    }
    let checked = rig.send("/api/blueprints/check", checking).await;
    let at_check = unheard(&checked["diagnostics"]);
    match project {
        Some(_) => assert_eq!(at_check, at_apply, "{checked}"),
        None => assert_eq!(at_check, Vec::<String>::new(), "{checked}"),
    }
    at_apply
}

#[tokio::test]
async fn an_apply_says_which_handlers_will_hear_nothing_and_goes_ahead() {
    let (rig, _at) = rig().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let nothing = Vec::<String>::new();
    // Applied for its project it hears the project's cards: nothing to say.
    assert_eq!(warned(&rig, &listens("in-the-shop", ""), Some("shop")).await, nothing);
    // Applied for no project it hears what is nobody's, and a card is
    // always some project's.
    let said = warned(&rig, &listens("in-no-project", ""), None).await;
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].starts_with(
            "this is applied for no project, and `board.item.moved`, which `moved` listens for, is always some \
             project's. A handler that says no `scope` hears its own project's events and what is no \
             project's, so this one will hear nothing."
        ) && said[0].contains("`--project`")
            && said[0].contains("`scope: fleet`"),
        "{said:?}"
    );
    // Saying that it hears every project's is what makes it so.
    assert_eq!(warned(&rig, &listens("everywhere", "    scope: fleet\n"), None).await, nothing);
    // A handler that names another project hears it only by saying so,
    // and the sentence says that.
    let other = "    for: { project: blog }\n";
    let said = warned(&rig, &listens("of-another", other), Some("shop")).await;
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].starts_with("`moved` names the project `blog` in `for:`, and this is applied for `shop`.")
            && said[0].ends_with("a handler that names another project in `for:` hears it only by saying so"),
        "{said:?}"
    );
    let fleet = "    scope: fleet\n    for: { project: blog }\n";
    assert_eq!(warned(&rig, &listens("of-another-said", fleet), Some("shop")).await, nothing);
    // And what was said is so: a card moved in the shop is heard by the
    // one applied for the shop and the one that hears every project's,
    // and by neither of the two that were warned.
    let id = rig.send("/api/board/shop/items", json!({ "title": "Add a cart" })).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    move_card(&rig, &id, "ready", None).await;
    rig.event("run.finished", |event| {
        event.data["blueprint"] == "everywhere" && event.data["flow"] == "on.moved"
    })
    .await;
    rig.event("run.finished", |event| {
        event.data["blueprint"] == "in-the-shop" && event.data["flow"] == "on.moved"
    })
    .await;
    let query =
        journal::EventQuery { topics: "run.started".to_string(), limit: 10_000, ..Default::default() };
    let runs = rig.said(query).await;
    let heard = |name: &str| {
        runs.iter().filter(|run| run.data["blueprint"] == name && run.data["flow"] == "on.moved").count()
    };
    assert_eq!(
        (heard("in-the-shop"), heard("everywhere"), heard("in-no-project"), heard("of-another")),
        (1, 1, 0, 0)
    );
}

/// Every blueprint's text under `dir`, by its file's path from the top
/// of the repository.
fn texts_under(repo: &std::path::Path, dir: &std::path::Path, found: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let at = entry.path();
        if at.is_dir() {
            texts_under(repo, &at, found);
        } else if brand::is_blueprint_path(&at.to_string_lossy()) {
            // Named the same on every system, whatever it writes between folders.
            let name = at.strip_prefix(repo).unwrap().to_string_lossy().replace('\\', "/");
            found.push((name, std::fs::read_to_string(&at).unwrap()));
        }
    }
}

#[tokio::test]
async fn nothing_that_ships_warns_where_it_is_applied_and_what_would_for_no_project_is_only_ever_a_projects()
{
    let rig = Rig::new().await;
    let engine = &rig.node().app.engine;
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let mut texts = Vec::new();
    // What a package brings is in its own file, and is held where its
    // cases are played: one fails where applying a blueprint of its
    // package warns (the command line's `packages.rs`).
    for dir in ["blueprints/std", "examples"] {
        texts_under(&repo, &repo.join(dir), &mut texts);
    }
    assert!(texts.len() >= 40, "{}", texts.len());
    let (mut for_none, mut for_one) = (Vec::new(), Vec::new());
    for (name, text) in &texts {
        let Some(blueprint) = engine.check(text, Some("shop")).blueprint else {
            panic!("{name} does not read")
        };
        let described = engine.registry().own_events();
        // Applied for a project, as a fleet and all it starts are, nothing
        // that ships has a handler that will hear nothing.
        if !conductor::whose::unheard(&blueprint, Some("shop"), described).is_empty() {
            for_one.push(name.clone());
        }
        for one in conductor::whose::unheard(&blueprint, None, described) {
            for_none.push(format!("{name}: {}", one.name));
        }
    }
    assert_eq!(for_one, Vec::<String>::new());
    for_none.sort();
    // Applied for no project, these would hear nothing: each is something
    // of a project's (its board, its cards, its features, its fleet, a
    // stage of one kind or another), which is never applied for none. What is applied for the
    // factory itself (its bot, something watched, a copy, a service
    // offered on a network) is not among them.
    let only_a_projects = [
        ("ab", &["accepted", "changed"][..]),
        ("cards", &["moved", "planned_again", "said", "taken_away"]),
        ("chain", &["changed", "held", "moved"]),
        ("feature", &["built", "changed", "removed", "said"]),
        ("features", &["added"]),
        ("flags", &["asked", "written"]),
        ("fleet", &["changed", "planned"]),
        ("pipeline", &["came", "changed", "moved"]),
        ("planning", &["added", "arrived", "pressed", "taken_off", "went"]),
        ("proposal", &["changed", "moved", "removed", "said"]),
        ("review", &["moved", "removed"]),
        ("reviewing", &["came", "dropped", "handed", "pressed"]),
        ("site", &["changed", "many"]),
        ("timed", &["came", "columns", "declared"]),
        ("told", &["held"]),
        ("working", &["added", "arrived", "away", "back", "done", "pressed", "unboarded", "unmerged"]),
    ];
    let expected: Vec<String> = only_a_projects
        .iter()
        .flat_map(|(text, handlers)| {
            handlers
                .iter()
                .map(move |handler| format!("blueprints/std/{}: {handler}", brand::blueprint_file(text)))
        })
        .collect();
    assert_eq!(for_none, expected);
}

/// Whose each event on `topic` so far is, in the order they were said.
async fn each(rig: &Rig, topic: &str) -> Vec<String> {
    let app = &rig.node().app;
    let query = journal::EventQuery { topics: topic.to_string(), limit: 10_000, ..Default::default() };
    let mut all = Vec::new();
    for event in rig.said(query).await {
        all.push(app.engine.whose(&event).await.unwrap_or_else(|| "nobody".to_string()));
    }
    all
}

/// Puts `body` at `path`, as the owner.
async fn put(rig: &Rig, path: &str, body: serde_json::Value) {
    let (status, answer) = rig.call(reqwest::Method::PUT, path, Some(&rig.token), Some(body)).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{path}: {answer}");
}

#[tokio::test]
async fn a_workspaces_events_are_nobodys_and_what_is_placed_says_whose_it_is_from_then_on() {
    let rig = Rig::new().await;
    let say = |words: &'static str| rig.send("/api/channels/floor/messages", json!({ "body": words }));
    // A workspace is above projects: what is said of one is no project's.
    rig.send("/api/workspaces", json!({ "name": "Front" })).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // A project put in one is said as the project's own news.
    put(&rig, "/api/projects/shop/workspace", json!({ "workspace": "front" })).await;
    // A channel made for no project, which is the usual workspace's own,
    // then the shop's, then another workspace's: what is said in it is
    // nobody's, the shop's, nobody's.
    rig.send("/api/channels", json!({ "name": "floor" })).await;
    say("to nobody in particular").await;
    put(&rig, "/api/channels/floor/belongs", json!({ "project": "shop" })).await;
    say("to the shop").await;
    put(&rig, "/api/channels/floor/belongs", json!({ "workspace": "front" })).await;
    say("to the whole front").await;
    let (status, renamed) = rig
        .call(
            reqwest::Method::PATCH,
            "/api/workspaces/front",
            Some(&rig.token),
            Some(json!({ "name": "Shops" })),
        )
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{renamed}");
    // Taken away, what was in it goes to the workspace the factory
    // always has: the project's move is the project's news, the
    // channel's nobody's.
    let to = Some(json!({ "to": "ada-s-workspace" }));
    let (status, removed) =
        rig.call(reqwest::Method::DELETE, "/api/workspaces/front", Some(&rig.token), to).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{removed}");

    assert_eq!(each(&rig, "workspace.created").await, ["nobody", "nobody"]);
    for topic in ["workspace.renamed", "workspace.removed"] {
        assert_eq!(each(&rig, topic).await, ["nobody"], "{topic}");
    }
    // As it was made, as it was put in another, and as that one went.
    assert_eq!(each(&rig, "project.placed").await, ["shop", "shop", "shop"]);
    // Made a workspace's own it is nobody's news; placed in the shop,
    // the shop's; placed in the workspace, nobody's, so that the shop
    // hears that it went; and so is its move with the workspace.
    assert_eq!(each(&rig, "channel.placed").await, ["nobody", "shop", "nobody", "nobody"]);
    let said = each(&rig, "channel.message").await;
    assert_eq!(said[said.len() - 3..], ["nobody", "shop", "nobody"], "{said:?}");
    // And the node is held, as every node is, to what is said in a
    // channel saying the project the channel was last placed in.
    assert_eq!(rig.node().undeclared(), Vec::<String>::new());
}
