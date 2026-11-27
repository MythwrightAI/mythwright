//! What a credential lacks is not got another way.
//!
//! Much of the factory is an instance that somebody with authority
//! applied, acting on what others write: a fleet on its tables, a handler
//! on an event, an agent on what it is asked. Each test here holds a key
//! with less and tries the way round.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{change, rig as fleet_rig, row, shop};

/// A key of the rig's owner that holds `scopes` and nothing else: its token.
pub(crate) async fn key(rig: &Rig, scopes: &[&str]) -> String {
    made(rig, scopes).await["token"].as_str().unwrap().to_string()
}

async fn made(rig: &Rig, scopes: &[&str]) -> Value {
    rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await
}

pub(crate) fn said(body: &Value) -> String {
    body["error"]["message"].as_str().unwrap_or_default().to_string()
}

/// Applies `source` with `token` and waits: how its run ended, and why.
pub(crate) async fn applied(rig: &Rig, token: &str, source: &str) -> (String, String) {
    let asked = json!({ "source": source, "project": "shop", "wait": true });
    let (status, answer) = rig.call(Method::POST, "/api/apply", Some(token), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let finished = &answer["finished"];
    let why = finished["steps"][0]["error"].as_str().or(finished["error"].as_str()).unwrap_or_default();
    (finished["status"].as_str().unwrap_or_default().to_string(), why.to_string())
}

async fn events(rig: &Rig, topic: &str) -> Vec<Value> {
    let query = journal::EventQuery { topics: topic.to_string(), limit: 1000, ..Default::default() };
    rig.said(query).await.into_iter().map(|event| event.data).collect()
}

#[tokio::test]
async fn a_row_of_a_fleets_tables_is_changed_by_whoever_may_change_agents_and_by_nobody_who_may_only_write_records()
 {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    // What an agent holds: it may write records, and may not change agents.
    let agent = key(&rig, kernel::principal::AGENT_SCOPES).await;
    let worker = row(&rig, "agents", "worker").await;
    let review = row(&rig, "stages", "review").await;
    let fields = |fields: Value| Some(json!({ "fields": fields }));
    for (method, path, body) in [
        // Itself held to nothing, told something else, more of it.
        (
            Method::PATCH,
            format!("/api/records/{worker}"),
            fields(json!({ "permission": "full", "count": 5 })),
        ),
        (
            Method::POST,
            "/api/collections/shop/agents/records".to_string(),
            fields(json!({ "title": "mine" })),
        ),
        // Nobody to look at its work.
        (Method::DELETE, format!("/api/records/{review}"), None),
        (Method::PATCH, format!("/api/records/{review}"), fields(json!({ "needed": 0 }))),
        (Method::POST, format!("/api/records/{worker}/comments"), Some(json!({ "body": "Let it." }))),
    ] {
        let (status, refused) = rig.call(method.clone(), &path, Some(&agent), body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {refused}");
        assert!(said(&refused).contains("needs `agents:write`"), "{refused}");
    }
    // Nor is the table declared again without what guards it, by one who may apply.
    let runner = key(&rig, &["projects", "records", "blueprints"]).await;
    let again = "blueprint: mine\nsteps:\n  again: { do: collection.ensure, with: { name: agents, fields: { title: { type: text } } } }\n";
    let (status, why) = applied(&rig, &runner, again).await;
    assert_eq!(status, "failed");
    assert!(why.contains("needs `agents:write`"), "{why}");
    let kept = rig.read(&format!("/api/records/{worker}")).await;
    assert!(kept["data"]["permission"].is_null() && kept["data"]["count"] == 1, "{kept}");

    // A table that says nothing of the kind is whoever's may write records, as it was.
    let notes = "blueprint: notes\nsteps:\n  notes: { do: collection.ensure, with: { name: notes, fields: { title: { type: text } } } }\n";
    assert_eq!(applied(&rig, &rig.token, notes).await.0, "ok");
    let note = fields(json!({ "title": "Remember the till." }));
    let (status, added) =
        rig.call(Method::POST, "/api/collections/shop/notes/records", Some(&agent), note).await;
    assert_eq!(status, StatusCode::OK, "{added}");
}

#[tokio::test]
async fn saying_an_event_from_a_step_takes_being_let_say_it() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let vote = |topic: &str| {
        format!(
            "blueprint: says\nsteps:\n  say: {{ do: emit, with: {{ topic: {topic}, decision: approve }} }}\n"
        )
    };
    // Whoever may apply and no more says nothing: what hears an event acts
    // on it with the authority of whoever applied that.
    let runner = key(&rig, &["projects", "blueprints"]).await;
    let (status, why) = applied(&rig, &runner, &vote("review.voted")).await;
    assert_eq!(status, "failed");
    assert!(why.contains("may not say `review.voted`") && why.contains("emit:review.voted"), "{why}");
    assert_eq!(events(&rig, "review.voted").await, Vec::<Value>::new());

    // Let say one topic, it says that one and no other.
    let sayer = key(&rig, &["projects", "blueprints", "emit:shop.said"]).await;
    assert_eq!(applied(&rig, &sayer, &vote("shop.said")).await.0, "ok");
    assert_eq!(events(&rig, "shop.said").await.len(), 1);
    assert_eq!(applied(&rig, &sayer, &vote("review.voted")).await.0, "failed");
}

#[tokio::test]
async fn a_blueprint_hears_what_whoever_applied_it_may_read() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    let ears = |name: &str| {
        format!(
            "blueprint: {name}\non:\n  heard:\n    event: channel.message\n    steps:\n      again: {{ do: emit, with: {{ topic: {name}.heard, body: \"${{event.data.body}}\" }} }}\n"
        )
    };
    // One that may not read channels, and one that may, each listening.
    let deaf = key(&rig, &["projects", "blueprints", "emit"]).await;
    let hears = key(&rig, &["projects", "blueprints", "emit", "channels:read"]).await;
    assert_eq!(applied(&rig, &deaf, &ears("deaf")).await.0, "ok");
    assert_eq!(applied(&rig, &hears, &ears("hears")).await.0, "ok");

    rig.send("/api/channels/general/messages", json!({ "body": "the first" })).await;
    rig.send("/api/channels/general/messages", json!({ "body": "the second" })).await;
    // Once the one that may read has heard the second, the first is long
    // past: the other heard neither.
    rig.event("hears.heard", |event| event.data["body"] == "the second").await;
    assert_eq!(events(&rig, "deaf.heard").await, Vec::<Value>::new());
    assert_eq!(events(&rig, "hears.heard").await.len(), 2);
}

#[tokio::test]
async fn an_agent_is_made_and_given_work_by_whoever_holds_what_it_does() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    // May change agents and apply; may not run commands, nor speak in a channel.
    let maker = key(&rig, &["projects", "agents", "blueprints"]).await;
    let ask = |body: Value| rig.call(Method::POST, "/api/agents", Some(&maker), Some(body));

    // An agent held to nothing runs commands as whoever runs the daemon.
    let (status, refused) =
        ask(json!({ "name": "free", "permission_mode": "full", "scopes": ["projects"] })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("needs `shell`"), "{refused}");
    // Nor is an agent a way into a channel its maker may not speak in.
    let (status, refused) =
        ask(json!({ "name": "voice", "channel": "general", "scopes": ["projects"] })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("needs `channels:write`"), "{refused}");
    // One of its own, holding what it holds: made.
    let (status, own) = ask(json!({ "name": "own", "scopes": ["projects"] })).await;
    assert_eq!(status, StatusCode::OK, "{own}");

    // An agent the owner made holds what an agent does, which is more than
    // this key: it is not given work by a blueprint the key applies.
    rig.send("/api/agents", json!({ "name": "wide" })).await;
    let task = |slot: &str| {
        format!(
            "blueprint: asks\nsteps:\n  ask: {{ do: agent.task, with: {{ slot: {slot}, prompt: \"Say hello.\" }} }}\n"
        )
    };
    let (status, why) = applied(&rig, &maker, &task("wide")).await;
    assert_eq!(status, "failed");
    assert!(why.contains("cannot give work to `wide`"), "{why}");
    // One it holds as much as is asked, and answers as it can: here the
    // stand-in that runs it, which does no such work and says so.
    let (_, why) = applied(&rig, &maker, &task("own")).await;
    assert!(why.starts_with("own could not do it") && !why.contains("cannot give work"), "{why}");

    // What an agent was asked and answered is the channels' to show.
    let watcher = key(&rig, &["agents:read"]).await;
    let (status, refused) = rig.call(Method::GET, "/api/agents/own/turns", Some(&watcher), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("needs `channels:read`"), "{refused}");
}

#[tokio::test]
async fn what_is_shown_is_what_its_asker_may_read() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/apply", json!({ "blueprint": "std/board", "project": "shop", "wait": true })).await;
    rig.send("/api/agents", json!({ "name": "helper" })).await;

    // The fleet as it is read, for one who may see the fleet and no more: neither
    // who works here nor what is running.
    let fleet = key(&rig, &["fleet:read"]).await;
    let (status, seen) = rig.call(Method::GET, "/api/fleet", Some(&fleet), None).await;
    assert_eq!(status, StatusCode::OK, "{seen}");
    assert_eq!((&seen["agents"], &seen["instances"]), (&json!([]), &json!([])), "{seen}");
    let owner = rig.read("/api/fleet").await;
    assert!(
        !owner["agents"].as_array().unwrap().is_empty() && !owner["instances"].as_array().unwrap().is_empty()
    );

    // Which other factories are on the device is the fleet's to tell.
    let reader = key(&rig, &["projects:read"]).await;
    let (status, refused) = rig.call(Method::GET, "/api/profiles", Some(&reader), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // A card's history is the board's: one who may read records and not
    // the board is not shown it, nor its card as a record.
    let card = rig.send("/api/board/shop/items", json!({ "title": "Add a cart" })).await;
    let id = card["id"].as_str().unwrap();
    let records = key(&rig, &["records:read"]).await;
    for path in [format!("/api/records/{id}/history"), format!("/api/records/{id}")] {
        let (status, refused) = rig.call(Method::GET, &path, Some(&records), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {refused}");
        assert!(said(&refused).contains("needs `board:read`"), "{refused}");
    }

    // Applying is refused before anything is read for it.
    let (status, refused) = rig
        .call(
            Method::POST,
            "/api/apply",
            Some(&reader),
            Some(json!({ "project": "shop", "path": "nothing.bp" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("needs `blueprints:run`"), "{refused}");
}

/// Whether `token` is anybody's: how asking who it is is answered.
async fn me(rig: &Rig, token: &str) -> StatusCode {
    rig.call(Method::GET, "/api/me", Some(token), None).await.0
}

#[tokio::test]
async fn a_key_is_withdrawn_by_the_person_who_made_it_and_is_then_nobodys() {
    let rig = Rig::new().await;
    let script = made(&rig, &["board:read"]).await;
    let (token, id) = (script["token"].as_str().unwrap(), script["id"].as_str().unwrap());
    let other = made(&rig, &["board:read", "projects:read"]).await;
    assert_eq!(me(&rig, token).await, StatusCode::OK);

    // A key given to a script is not a way to take its maker's others away.
    let path = format!("/api/credentials/{}", other["id"].as_str().unwrap());
    let (status, refused) = rig.call(Method::DELETE, &path, Some(token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // The person who made it withdraws it: said, and from then it is nobody's.
    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/credentials/{id}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!(me(&rig, token).await, StatusCode::UNAUTHORIZED);
    assert_eq!(me(&rig, other["token"].as_str().unwrap()).await, StatusCode::OK);
    let (made, withdrawn) =
        (events(&rig, "identity.key.made").await, events(&rig, "identity.key.revoked").await);
    assert_eq!((made.len(), withdrawn.len()), (2, 1));
    assert_eq!(withdrawn[0]["key"], id);
    // Withdrawn twice, there is no such key in use.
    let (status, _) =
        rig.call(Method::DELETE, &format!("/api/credentials/{id}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_row_says_whether_its_agents_work_behind_a_wall_and_a_key_that_runs_no_command_takes_no_wall_away()
{
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let agent = |name: &'static str| {
        let rig = &rig;
        async move {
            let all = rig.read("/api/agents").await;
            let found = all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name).cloned();
            found.unwrap_or_else(|| panic!("there is no agent {name}: {all}"))
        }
    };
    // A row that says nothing of it: a wall, and nothing more of the device.
    let worker = agent("shop-worker-1").await;
    assert_eq!(
        (&worker["wall"], &worker["reads"], &worker["changes"]),
        (&json!(true), &json!([]), &json!([]))
    );

    // Said in its row, by the owner, whose fleet it is and who may run commands.
    let cells = json!({ "wall": "off", "reads": ["~/.toolchain"], "changes": ["/srv/cache"] });
    change(&rig, "agents", "worker", cells).await;
    let worker = agent("shop-worker-1").await;
    assert_eq!(worker["wall"], false);
    assert_eq!((&worker["reads"], &worker["changes"]), (&json!(["~/.toolchain"]), &json!(["/srv/cache"])));
    // The other row's agents are as they were.
    assert_eq!(agent("shop-reviewer-1").await["wall"], true);
    // What its agents may do with the repository is a cell of the row too:
    // reading it, where nothing is said.
    assert_eq!(agent("shop-worker-1").await["git"], "read");
    change(&rig, "agents", "worker", json!({ "git": "commit" })).await;
    assert_eq!(agent("shop-worker-1").await["git"], "commit");
    change(&rig, "agents", "worker", json!({ "git": null })).await;
    assert_eq!(agent("shop-worker-1").await["git"], "read");
    // And the keys they are handed, each by the name the factory keeps it under.
    change(&rig, "agents", "worker", json!({ "secrets": ["REMOTE_TOKEN=remote-bot"] })).await;
    assert_eq!(agent("shop-worker-1").await["secrets"], json!({ "REMOTE_TOKEN": "remote-bot" }));
    change(&rig, "agents", "worker", json!({ "secrets": [] })).await;
    assert_eq!(agent("shop-worker-1").await["secrets"], json!({}));

    // A cell that was emptied says nothing, and saying nothing is a wall.
    change(&rig, "agents", "worker", json!({ "wall": null, "reads": [], "changes": [] })).await;
    let worker = agent("shop-worker-1").await;
    assert_eq!(
        (&worker["wall"], &worker["reads"], &worker["changes"]),
        (&json!(true), &json!([]), &json!([]))
    );

    // By hand, with a key that may make agents and may run no command: one
    // with a wall is theirs to make, and that is all.
    let holds: Vec<&str> =
        kernel::principal::AGENT_SCOPES.iter().copied().chain(["agents", "blueprints:run"]).collect();
    assert!(!holds.contains(&"shell"));
    let narrow = key(&rig, &holds).await;
    let ask = |body: Value| {
        let (rig, narrow) = (&rig, narrow.clone());
        async move { rig.call(Method::POST, "/api/agents", Some(&narrow), Some(body)).await }
    };
    let (status, made) = ask(json!({ "name": "walled", "adapter": "builtin" })).await;
    assert_eq!((status, &made["wall"]), (StatusCode::OK, &json!(true)), "{made}");
    let (status, refused) = ask(json!({ "name": "open", "adapter": "builtin", "wall": false })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("takes `shell`, which narrow does not hold"), "{refused}");
    let (status, refused) = ask(json!({ "name": "shown", "adapter": "builtin", "reads": ["~/.ssh"] })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("more of this device than its work (~/.ssh)"), "{refused}");
    // Nor through the fleet, which would do it with its owner's authority:
    // the cells that say so take what doing it by hand takes, of whoever
    // changes them. The rest of the row is theirs, as it was.
    let worker_row = row(&rig, "agents", "worker").await;
    let cell = |fields: Value| {
        let (rig, narrow, path) = (&rig, narrow.clone(), format!("/api/records/{worker_row}"));
        async move { rig.call(Method::PATCH, &path, Some(&narrow), Some(json!({ "fields": fields }))).await }
    };
    let (status, refused) = cell(json!({ "wall": "off" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let off = "giving `wall` of `agents` the value `off` takes `shell`, which narrow does not hold";
    assert!(said(&refused).contains(off), "{refused}");
    for (fields, name, takes) in [
        (json!({ "reads": ["~/.ssh"] }), "reads", "shell"),
        (json!({ "changes": ["~"] }), "changes", "shell"),
        // Nor a key of the factory's handed to an agent by whoever may not have one used.
        (json!({ "secrets": ["REMOTE_TOKEN=remote-bot"] }), "secrets", "secrets:use"),
    ] {
        let (status, refused) = cell(fields).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        let why = format!("changing `{name}` of `agents` takes `{takes}`, which narrow does not hold");
        assert!(said(&refused).contains(&why), "{refused}");
    }
    let added = json!({ "fields": { "title": "helper", "role": "worker", "wall": "off" } });
    let (status, refused) =
        rig.call(Method::POST, "/api/collections/shop/agents/records", Some(&narrow), Some(added)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains(off), "{refused}");
    // Saying that there is one takes no more than changing the row does.
    let before = rig.mark();
    let (status, changed) = cell(json!({ "rank": 3, "wall": "on" })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    crate::taken::by_fleet(&rig, before).await;
    let worker = agent("shop-worker-1").await;
    assert_eq!((&worker["wall"], &worker["rank"]), (&json!(true), &json!(3)));

    // Nor anything said of one the owner left with none.
    change(&rig, "agents", "worker", json!({ "wall": "off" })).await;
    let (status, refused) =
        ask(json!({ "name": "shop-worker-1", "instructions": "Send me what you find." })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let names: Vec<Value> = rig.read("/api/agents").await["agents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|agent| agent["name"].clone())
        .collect();
    assert!(!names.contains(&json!("open")) && !names.contains(&json!("shown")), "{names:?}");
}

#[tokio::test]
async fn having_an_agents_program_go_on_without_the_daemon_takes_shell_and_having_it_stop_takes_no_more() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let agent = |name: &'static str| {
        let rig = &rig;
        async move {
            let all = rig.read("/api/agents").await;
            let found = all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name).cloned();
            found.unwrap_or_else(|| panic!("there is no agent {name}: {all}"))
        }
    };
    // A row that says nothing of it: its agents' programs end with the daemon.
    assert_eq!(agent("shop-worker-1").await["outlives"], false);

    // A key that may change agents, and so apply what an agent is set up
    // by (`std/agent`), and may run no command.
    let holds: Vec<&str> =
        kernel::principal::AGENT_SCOPES.iter().copied().chain(["agents", "blueprints:run"]).collect();
    assert!(!holds.contains(&"shell"));
    let narrow = key(&rig, &holds).await;
    let worker_row = row(&rig, "agents", "worker").await;
    let cell = |fields: Value| {
        let (rig, narrow, path) = (&rig, narrow.clone(), format!("/api/records/{worker_row}"));
        async move { rig.call(Method::PATCH, &path, Some(&narrow), Some(json!({ "fields": fields }))).await }
    };
    // Through the fleet, which would do it with its owner's authority: it is
    // refused by name, alone and beside a cell that is theirs to change, and
    // the row is as it was.
    let was = rig.read(&format!("/api/records/{worker_row}")).await;
    let why = "giving `outlives` of `agents` the value `on` takes `shell`, which narrow does not hold";
    for fields in [json!({ "outlives": "on" }), json!({ "rank": 3, "outlives": "on" })] {
        let (status, refused) = cell(fields).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert!(said(&refused).contains(why), "{refused}");
    }
    assert_eq!(rig.read(&format!("/api/records/{worker_row}")).await["data"], was["data"]);
    assert_eq!(agent("shop-worker-1").await["outlives"], false);
    // Nor in a row of their own making.
    let added = json!({ "fields": { "title": "helper", "role": "worker", "outlives": "on" } });
    let (status, refused) =
        rig.call(Method::POST, "/api/collections/shop/agents/records", Some(&narrow), Some(added)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains(why), "{refused}");
    // Nor by hand: an agent whose program ends with the daemon is theirs to
    // make, and that is all.
    let ask = |body: Value| {
        let (rig, narrow) = (&rig, narrow.clone());
        async move { rig.call(Method::POST, "/api/agents", Some(&narrow), Some(body)).await }
    };
    let (status, made) = ask(json!({ "name": "tied", "adapter": "builtin" })).await;
    assert_eq!((status, &made["outlives"]), (StatusCode::OK, &json!(false)), "{made}");
    for body in [
        json!({ "name": "kept", "adapter": "builtin", "outlives": true }),
        // One that is there is not made to go on either.
        json!({ "name": "tied", "outlives": "on" }),
    ] {
        let (status, refused) = ask(body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert!(
            said(&refused).contains("turning that on takes `shell`, which narrow does not hold"),
            "{refused}"
        );
    }
    let all = rig.read("/api/agents").await;
    let all = all["agents"].as_array().unwrap();
    assert!(!all.iter().any(|agent| agent["name"] == "kept" || agent["outlives"] == true), "{all:?}");

    // Said by the owner, whose fleet it is and who may run commands.
    change(&rig, "agents", "worker", json!({ "outlives": "on" })).await;
    assert_eq!(agent("shop-worker-1").await["outlives"], true);
    assert_eq!(agent("shop-reviewer-1").await["outlives"], false);
    // The rest of such a row is still whoever's may change agents, and its
    // agents go on as the owner had them.
    let before = rig.mark();
    let (status, changed) = cell(json!({ "rank": 3 })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    crate::taken::by_fleet(&rig, before).await;
    let worker = agent("shop-worker-1").await;
    assert_eq!((&worker["outlives"], &worker["rank"]), (&json!(true), &json!(3)));

    // Having it stop takes no more than changing the row does: said `off`,
    // and said by emptying the cell, which is off as well.
    for off in [json!("off"), Value::Null] {
        let before = rig.mark();
        let (status, changed) = cell(json!({ "outlives": off })).await;
        assert_eq!(status, StatusCode::OK, "{changed}");
        crate::taken::by_fleet(&rig, before).await;
        assert_eq!(agent("shop-worker-1").await["outlives"], false);
        change(&rig, "agents", "worker", json!({ "outlives": "on" })).await;
        assert_eq!(agent("shop-worker-1").await["outlives"], true);
    }
    // By hand as well, of one the owner had go on.
    let owner = |body: Value| {
        let rig = &rig;
        async move { rig.call(Method::POST, "/api/agents", Some(&rig.token), Some(body)).await }
    };
    let (status, made) = owner(json!({ "name": "tied", "outlives": true })).await;
    assert_eq!((status, &made["outlives"]), (StatusCode::OK, &json!(true)), "{made}");
    // Saying something else of it is not turning anything on.
    let (status, made) = ask(json!({ "name": "tied", "instructions": "Mind the till." })).await;
    assert_eq!((status, &made["outlives"]), (StatusCode::OK, &json!(true)), "{made}");
    let (status, made) = ask(json!({ "name": "tied", "outlives": false })).await;
    assert_eq!((status, &made["outlives"]), (StatusCode::OK, &json!(false)), "{made}");
}

#[tokio::test]
async fn letting_an_agents_program_use_the_web_unasked_takes_shell_and_turning_it_off_takes_no_more() {
    let (rig, _at) = fleet_rig().await;
    // A key that may change agents, and so apply what an agent is set up
    // by (`std/agent`), and may run no command.
    let holds: Vec<&str> =
        kernel::principal::AGENT_SCOPES.iter().copied().chain(["agents", "blueprints:run"]).collect();
    assert!(!holds.contains(&"shell"));
    let narrow = key(&rig, &holds).await;
    let ask = |body: Value| {
        let (rig, narrow) = (&rig, narrow.clone());
        async move { rig.call(Method::POST, "/api/agents", Some(&narrow), Some(body)).await }
    };
    // An agent that asks leave before it looks anything up is theirs to make.
    let (status, made) = ask(json!({ "name": "plain", "adapter": "builtin" })).await;
    assert_eq!((status, &made["web"]), (StatusCode::OK, &json!(false)), "{made}");
    // One that need not ask is not, new or there already.
    for body in [
        json!({ "name": "looker", "adapter": "builtin", "web": true }),
        json!({ "name": "plain", "web": "on" }),
    ] {
        let (status, refused) = ask(body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert!(
            said(&refused).contains("turning that on takes `shell`, which narrow does not hold"),
            "{refused}"
        );
    }
    let all = rig.read("/api/agents").await;
    let all = all["agents"].as_array().unwrap();
    assert!(!all.iter().any(|agent| agent["name"] == "looker" || agent["web"] == true), "{all:?}");

    // Said by the owner, who may run commands here.
    let owner = |body: Value| {
        let rig = &rig;
        async move { rig.call(Method::POST, "/api/agents", Some(&rig.token), Some(body)).await }
    };
    let (status, made) = owner(json!({ "name": "plain", "web": "on" })).await;
    assert_eq!((status, &made["web"]), (StatusCode::OK, &json!(true)), "{made}");
    // Saying something else of it is not turning anything on, and turning
    // it off takes no more than changing the agent.
    let (status, made) = ask(json!({ "name": "plain", "instructions": "Mind the till." })).await;
    assert_eq!((status, &made["web"]), (StatusCode::OK, &json!(true)), "{made}");
    let (status, made) = ask(json!({ "name": "plain", "web": false })).await;
    assert_eq!((status, &made["web"]), (StatusCode::OK, &json!(false)), "{made}");
    // And it is on or off, and nothing else.
    let (status, refused) = owner(json!({ "name": "plain", "web": "sometimes" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("is `on` or `off`, and not `sometimes`"), "{refused}");
}

#[tokio::test]
async fn whether_a_fleets_agents_may_use_the_web_unasked_is_a_cell_of_their_row_that_takes_shell_to_turn_on()
{
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let agent = |name: &'static str| {
        let rig = &rig;
        async move {
            let all = rig.read("/api/agents").await;
            let found = all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name).cloned();
            found.unwrap_or_else(|| panic!("there is no agent {name}: {all}"))
        }
    };
    // A row that says nothing of it: its agents ask leave, and are refused.
    assert_eq!(agent("shop-worker-1").await["web"], false);

    // A key that may change agents, and may run no command.
    let holds: Vec<&str> = kernel::principal::AGENT_SCOPES.iter().copied().chain(["agents"]).collect();
    assert!(!holds.contains(&"shell"));
    let narrow = key(&rig, &holds).await;
    let worker_row = row(&rig, "agents", "worker").await;
    let cell = |fields: Value| {
        let (rig, narrow, path) = (&rig, narrow.clone(), format!("/api/records/{worker_row}"));
        async move { rig.call(Method::PATCH, &path, Some(&narrow), Some(json!({ "fields": fields }))).await }
    };
    // Through the fleet, which would do it with its owner's authority: it
    // is refused by name, and the row and its agents are as they were.
    let was = rig.read(&format!("/api/records/{worker_row}")).await;
    let why = "giving `web` of `agents` the value `on` takes `shell`, which narrow does not hold";
    let (status, refused) = cell(json!({ "web": "on" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains(why), "{refused}");
    let added = json!({ "fields": { "title": "helper", "role": "worker", "web": "on" } });
    let (status, refused) =
        rig.call(Method::POST, "/api/collections/shop/agents/records", Some(&narrow), Some(added)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains(why), "{refused}");
    assert_eq!(rig.read(&format!("/api/records/{worker_row}")).await["data"], was["data"]);
    assert_eq!(agent("shop-worker-1").await["web"], false);

    // Said by the owner, whose fleet it is and who may run commands: the
    // row's agents may, and no other row's.
    change(&rig, "agents", "worker", json!({ "web": "on" })).await;
    assert_eq!(agent("shop-worker-1").await["web"], true);
    assert_eq!(agent("shop-reviewer-1").await["web"], false);

    // Turning it off takes no more than changing the row does: said `off`,
    // and said by emptying the cell, which is off as well.
    for off in [json!("off"), Value::Null] {
        let before = rig.mark();
        let (status, changed) = cell(json!({ "web": off })).await;
        assert_eq!(status, StatusCode::OK, "{changed}");
        crate::taken::by_fleet(&rig, before).await;
        assert_eq!(agent("shop-worker-1").await["web"], false);
        change(&rig, "agents", "worker", json!({ "web": "on" })).await;
        assert_eq!(agent("shop-worker-1").await["web"], true);
    }
}

#[tokio::test]
async fn what_one_value_of_a_cell_takes_is_asked_of_whoever_gives_it_that_value_and_is_not_declared_away() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let table = |cells: &str| {
        format!(
            "blueprint: doors\nsteps:\n  doors: {{ do: collection.ensure, with: {{ name: doors, fields: {{ title: {{ type: text }}, {cells} }} }} }}\n"
        )
    };
    let guarded = "lock: { type: choice, of: [open, shut], takes: { open: shell } }, \
                   loud: { type: flag, takes: { true: shell } }, bell: { type: text, takes: shell }";
    assert_eq!(applied(&rig, &rig.token, &table(guarded)).await, ("ok".to_string(), String::new()));

    // Whoever may write records and run no command: every value but the one.
    let writer = key(&rig, &["projects", "records", "blueprints"]).await;
    let add = |token: String, fields: Value| {
        let rig = &rig;
        async move {
            let body = Some(json!({ "fields": fields }));
            rig.call(Method::POST, "/api/collections/shop/doors/records", Some(&token), body).await
        }
    };
    let (status, door) = add(writer.clone(), json!({ "title": "Back", "lock": "shut", "loud": false })).await;
    assert_eq!(status, StatusCode::OK, "{door}");
    let id = door["id"].as_str().unwrap().to_string();
    for (fields, why) in [
        (
            json!({ "title": "Front", "lock": "open" }),
            "giving `lock` of `doors` the value `open` takes `shell`",
        ),
        (json!({ "title": "Side", "loud": true }), "giving `loud` of `doors` the value `true` takes `shell`"),
    ] {
        let (status, refused) = add(writer.clone(), fields).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert!(
            said(&refused).contains(why) && said(&refused).contains("which narrow does not hold"),
            "{refused}"
        );
    }
    let change = |token: String, fields: Value| {
        let (rig, path) = (&rig, format!("/api/records/{id}"));
        async move { rig.call(Method::PATCH, &path, Some(&token), Some(json!({ "fields": fields }))).await }
    };
    let (status, refused) = change(writer.clone(), json!({ "lock": "open" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Given by the owner, it is taken back by whoever may change the record.
    let (status, opened) = change(rig.token.clone(), json!({ "lock": "open", "loud": true })).await;
    assert_eq!((status, &opened["data"]["lock"]), (StatusCode::OK, &json!("open")), "{opened}");
    let (status, shut) = change(writer.clone(), json!({ "lock": "shut", "loud": false })).await;
    assert_eq!(
        (status, &shut["data"]["lock"], &shut["data"]["loud"]),
        (StatusCode::OK, &json!("shut"), &json!(false))
    );
    let (status, emptied) = change(writer.clone(), json!({ "lock": null })).await;
    assert_eq!((status, &emptied["data"]["lock"]), (StatusCode::OK, &Value::Null), "{emptied}");

    // Nor is the table declared again with the word left off a cell, or said
    // of fewer of its values, by one who does not hold what the word asked.
    for (cells, why) in [
        (
            "lock: { type: choice, of: [open, shut] }, loud: { type: flag, takes: { true: shell } }, bell: { type: text, takes: shell }",
            "giving `lock` of `doors` the value `open` takes `shell`, and declaring `doors` again so that it does not takes holding `shell`",
        ),
        (
            "lock: { type: choice, of: [open, shut], takes: { open: shell } }, loud: { type: flag, takes: { true: shell } }, bell: { type: text }",
            "giving `bell` of `doors` a value takes `shell`, and declaring `doors` again so that it does not takes holding `shell`",
        ),
    ] {
        let (status, refused) = applied(&rig, &writer, &table(cells)).await;
        assert_eq!(status, "failed");
        assert!(refused.contains(why), "{refused}");
    }
    let (status, refused) = change(writer.clone(), json!({ "lock": "open" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Said again as it was, or of more, it is whoever's may declare it; and
    // the owner may take the word off.
    assert_eq!(applied(&rig, &writer, &table(guarded)).await.0, "ok");
    let more = "lock: { type: choice, of: [open, shut], takes: shell }, \
                loud: { type: flag, takes: { true: shell } }, bell: { type: text, takes: shell }";
    assert_eq!(applied(&rig, &writer, &table(more)).await.0, "ok");
    let (status, refused) = change(writer.clone(), json!({ "lock": "shut" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("changing `lock` of `doors` takes `shell`"), "{refused}");
    let bare = "lock: { type: choice, of: [open, shut] }, loud: { type: flag }, bell: { type: text }";
    assert_eq!(applied(&rig, &rig.token, &table(bare)).await.0, "ok");
    let (status, opened) = change(writer.clone(), json!({ "lock": "open" })).await;
    assert_eq!(status, StatusCode::OK, "{opened}");

    // What one value takes is said of a value the cell can have, by a cell
    // whose values can be written one way.
    for (cells, why) in [
        (
            "lock: { type: choice, of: [open, shut], takes: { ajar: shell } }",
            "`ajar` is not one of: open, shut",
        ),
        (
            "bell: { type: text, takes: { rung: shell } }",
            "which a choice or a flag can say and a `text` cannot",
        ),
        ("lock: { type: choice, of: [open, shut], takes: { open: everything } }", "the field `lock` takes"),
    ] {
        let (status, refused) = applied(&rig, &rig.token, &table(cells)).await;
        assert_eq!(status, "failed");
        assert!(refused.contains(why), "{refused}");
    }
}

/// Asks, as whoever holds `token`, whether `actor` holds `scope`: how the
/// run ended, why if it failed, and what was answered.
async fn asked_whether(rig: &Rig, token: &str, actor: &str, scope: &str) -> (String, String, Value) {
    let source = format!(
        "blueprint: asks\nsteps:\n  ask:\n    do: actor.holds\n    with: {{ actor: \"{actor}\", scope: \"{scope}\" }}\noutputs:\n  holds: ${{steps.ask.out.holds}}\n  known: ${{steps.ask.out.known}}\n"
    );
    let asked = json!({ "source": source, "project": "shop", "wait": true });
    let (status, answer) = rig.call(Method::POST, "/api/apply", Some(token), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let finished = &answer["finished"];
    let why = finished["steps"][0]["error"].as_str().or(finished["error"].as_str()).unwrap_or_default();
    (
        finished["status"].as_str().unwrap_or_default().to_string(),
        why.to_string(),
        finished["outputs"].clone(),
    )
}

#[tokio::test]
async fn whether_an_actor_holds_a_scope_is_said_of_who_they_are_now_and_only_to_one_who_holds_it_themselves()
{
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let owner = rig.read("/api/me").await["actor"].as_str().unwrap().to_string();
    let narrow = made(&rig, &["board"]).await;
    let key = format!("key:{}", narrow["id"].as_str().unwrap());
    let held = |holds: bool, known: bool| json!({ "holds": holds, "known": known });

    // The owner holds it. A key of the owner's holds what it was given and
    // nothing of its maker's besides.
    let ask = |actor: &str, scope: &str| {
        let (rig, token, actor, scope) = (&rig, rig.token.clone(), actor.to_string(), scope.to_string());
        async move { asked_whether(rig, &token, &actor, &scope).await.2 }
    };
    assert_eq!(ask(&owner, "projects:write").await, held(true, true));
    assert_eq!(ask(&key, "projects:write").await, held(false, true));
    assert_eq!(ask(&key, "board:write").await, held(true, true));
    // Nobody holds anything: an account that is not there, what is no
    // person or key or agent, and nothing at all.
    for nobody in ["user:acct_nobody", "caller:deploys", "system", ""] {
        assert_eq!(ask(nobody, "projects:write").await, held(false, false), "{nobody}");
    }
    // Withdrawn, a key holds nothing from then.
    let path = format!("/api/credentials/{}", narrow["id"].as_str().unwrap());
    let (status, gone) = rig.call(Method::DELETE, &path, Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!(ask(&key, "board:write").await, held(false, false));

    // And nobody learns by asking what they could not do themselves: a key
    // that may run blueprints and holds no `projects` is told nothing of
    // who does.
    let curious = key_of(&rig, &["blueprints", "board"]).await;
    let (status, why, answered) = asked_whether(&rig, &curious, &owner, "projects:write").await;
    assert_eq!(status, "failed", "{answered}");
    assert!(why.contains("needs `projects:write`"), "{why}");
}

/// A key of the rig's owner that holds `scopes` and nothing else: its token.
async fn key_of(rig: &Rig, scopes: &[&str]) -> String {
    key(rig, scopes).await
}

#[tokio::test]
async fn a_card_passed_by_hand_is_merged_where_its_stage_says_so_for_one_who_holds_what_merging_takes_and_not_for_a_key_that_holds_less()
 {
    use crate::lanes::{git, history, move_card, moved};
    let (rig, _at) = fleet_rig().await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    let worked = |title: &'static str| {
        let rig = &rig;
        async move {
            let id = crate::fleets::add(rig, title, &[]).await;
            move_card(rig, &id, "ready", None).await;
            moved(rig, &id, "review", 0).await;
            id
        }
    };
    let said = |words: &'static str, of: &str| {
        let (rig, of) = (&rig, of.to_string());
        async move {
            let told = rig
                .event("channel.message", move |event| {
                    let body = event.data["body"].as_str().unwrap_or_default();
                    body.contains(words) && body.contains(of.as_str())
                })
                .await;
            told.data["body"].as_str().unwrap().to_string()
        }
    };
    let tidied = |after: u64| {
        let rig = &rig;
        async move {
            rig.event("run.finished", move |event| {
                event.seq > after
                    && event.data["blueprint"] == "std/working"
                    && event.data["flow"] == "on.tidied"
            })
            .await;
        }
    };
    let on_the_projects_branch =
        |id: &str| git(&dir, &["log", "--format=%s"]).lines().any(|line| line.starts_with(&format!("{id}:")));

    // Where the row says `leave`: a card a person passes is done, and its
    // work is left on its branch.
    change(&rig, "stages", "review", json!({ "by_hand": "leave" })).await;
    let first = worked("Add a cart").await;
    let now = rig.mark();
    move_card(&rig, &first, "done", None).await;
    tidied(now).await;
    assert!(!on_the_projects_branch(&first));
    assert_eq!(git(&dir, &["branch", "--list", &format!("lane/{first}")]).trim(), format!("lane/{first}"));

    // The row of the stage that reviews says a card passed by hand is
    // merged, as it is where it merges and says nothing of this.
    change(&rig, "stages", "review", json!({ "by_hand": "" })).await;

    // Passed with a key that may move cards and holds nothing of the
    // project's files: the card is where the key put it, nothing is
    // merged, and the channel says why and what to do.
    let second = worked("Add a till").await;
    let narrow = key(&rig, &["board"]).await;
    let now = rig.mark();
    let (status, answer) = rig
        .call(
            Method::PATCH,
            &format!("/api/board/items/{second}"),
            Some(&narrow),
            Some(json!({ "status": "done" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let told = said("was not merged", &second).await;
    assert!(
        told.contains("Whoever passed it does not hold `projects:write`, and merging into the project's branch takes that here.")
            && told.contains("Someone who holds `projects:write` can pass it again"),
        "{told}"
    );
    tidied(now).await;
    assert_eq!(crate::fleets::card(&rig, &second).await["status"], "done");
    assert!(!on_the_projects_branch(&second));
    assert_eq!(git(&dir, &["branch", "--list", &format!("lane/{second}")]).trim(), format!("lane/{second}"));
    let lines = history(
        &rig,
        &second,
        &format!("it was passed by hand, and its branch lane/{second} was not merged"),
    )
    .await;
    assert!(
        lines.iter().any(|line| line.contains("(with the key narrow) moved it from review to done")),
        "{lines:?}"
    );

    // Passed by the owner, who holds it: merged as an agents' approval is,
    // and then tidied away.
    let third = worked("Add a shelf").await;
    let now = rig.mark();
    move_card(&rig, &third, "done", None).await;
    let merged = third.clone();
    rig.event("card.merged", move |event| event.data["card"] == merged.as_str()).await;
    tidied(now).await;
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), format!("{third}: Add a shelf"));
    assert!(!dir.join(".lanes").join(&third).exists());
    assert_eq!(git(&dir, &["branch", "--list", &format!("lane/{third}")]), "");
    history(&rig, &third, &format!("its branch lane/{third} was merged into the project's as Ada passed it"))
        .await;
}
