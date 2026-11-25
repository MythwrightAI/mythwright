//! Nobody gives through a table what they do not hold: more of what is
//! held to a key that holds less (`held.rs`), where what is written into
//! a row is a list of scopes that something with more authority then
//! gives to somebody else. A fleet brings its agents up as whoever
//! applied it, and what each of them holds is a cell of its row. And what
//! they are given to do is a row of another table, which gives it as
//! whoever applied the fleet: so that table says whose records its own
//! steer, and a record of it is written by whoever could write those.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::fleets::{change, rig as fleet_rig, row, shop};
use crate::held::{applied, key, said};

#[tokio::test]
async fn a_row_gives_an_agent_no_scope_that_whoever_wrote_it_there_does_not_hold() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let scopes = |name: &'static str| {
        let rig = &rig;
        async move {
            let all = rig.read("/api/agents").await;
            let found = all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name).cloned();
            found.unwrap_or_else(|| panic!("there is no agent {name}: {all}"))["scopes"].clone()
        }
    };
    let usual = json!(kernel::principal::AGENT_SCOPES);
    assert_eq!(scopes("shop-worker-1").await, usual);

    // Somebody who may change the fleet's rows and apply blueprints, and
    // holds neither everything nor `admin`, `shell` or the board to change.
    let holds = ["projects", "channels", "board:read", "records", "blueprints", "agents", "fleet"];
    let narrow = key(&rig, &holds).await;
    let worker_row = row(&rig, "agents", "worker").await;
    let cell = |fields: Value| {
        let (rig, narrow, path) = (&rig, narrow.clone(), format!("/api/records/{worker_row}"));
        async move { rig.call(Method::PATCH, &path, Some(&narrow), Some(json!({ "fields": fields }))).await }
    };
    let refused_for = |refused: &Value, scope: &str| {
        let why = format!(
            "writing `{scope}` into `holds` of `agents` takes holding `{scope}`, which narrow does not hold"
        );
        assert!(said(refused).contains(&why), "{refused}");
    };
    // The fleet would give each as its owner: so each is asked of whoever writes it.
    for (written, scope) in [
        (json!(["board:read", "admin"]), "admin"),
        (json!(["*"]), "*"),
        (json!(["shell"]), "shell"),
        // The whole of an area, by one who holds a part of it.
        (json!(["board"]), "board"),
        (json!(["board:read", "board:write"]), "board:write"),
    ] {
        let (status, refused) = cell(json!({ "holds": written })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        refused_for(&refused, scope);
    }
    // Nor in a row they add, which the fleet would bring an agent up for.
    let added = json!({ "fields": { "title": "helper", "role": "worker", "holds": ["admin"] } });
    let (status, refused) =
        rig.call(Method::POST, "/api/collections/shop/agents/records", Some(&narrow), Some(added)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    refused_for(&refused, "admin");
    // Nor by a blueprint they apply, which writes as they do.
    for step in [
        format!("{{ do: record.update, with: {{ id: {worker_row}, fields: {{ holds: [admin] }} }} }}"),
        "{ do: record.add, with: { project: shop, collection: agents, fields: { title: helper, holds: [admin] } } }"
            .to_string(),
    ] {
        let text = format!("blueprint: mine\nsteps:\n  write: {step}\n");
        let (status, why) = applied(&rig, &narrow, &text).await;
        assert_eq!(status, "failed", "{step}");
        assert!(why.contains("takes holding `admin`, which narrow does not hold"), "{why}");
    }
    // Nothing was written, and nobody was given anything.
    let kept = rig.read(&format!("/api/records/{worker_row}")).await;
    assert!(kept["data"]["holds"].is_null(), "{kept}");
    assert_eq!(scopes("shop-worker-1").await, usual);
    let names: Vec<Value> = rig.read("/api/agents").await["agents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|agent| agent["name"].clone())
        .collect();
    assert!(!names.contains(&json!("shop-helper-1")), "{names:?}");

    // What they hold themselves is theirs to give, and to take away again.
    let before = rig.mark();
    let (status, changed) = cell(json!({ "holds": ["board:read", "records", "channels:read"] })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    crate::taken::by_fleet(&rig, before).await;
    assert_eq!(scopes("shop-worker-1").await, json!(["board:read", "records", "channels:read"]));

    // The owner, who holds everything, gives more. What is there then is
    // not theirs to leave beside what they write, or to write narrower
    // than it was and wider than they hold: the row would still give it.
    change(&rig, "agents", "worker", json!({ "holds": ["board", "shell", "records"] })).await;
    assert_eq!(scopes("shop-worker-1").await, json!(["board", "shell", "records"]));
    for (written, scope) in [
        (json!(["board:read", "shell", "projects:read"]), "shell"),
        (json!(["board:write", "records"]), "board:write"),
    ] {
        let (status, refused) = cell(json!({ "holds": written })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert!(said(&refused).contains(&format!("holds `{scope}`, which narrow does not")), "{refused}");
    }
    // Written down to what they hold, it is theirs, and what they took
    // out is no longer there to be left: writing it back is giving it.
    let before = rig.mark();
    let (status, changed) = cell(json!({ "holds": ["board:read", "projects:read"] })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    crate::taken::by_fleet(&rig, before).await;
    assert_eq!(scopes("shop-worker-1").await, json!(["board:read", "projects:read"]));
    let (status, refused) = cell(json!({ "holds": ["projects:read", "shell"] })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    refused_for(&refused, "shell");
    // Any other cell of a row that gives no more than they hold is theirs.
    let (status, changed) = cell(json!({ "rank": 2 })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
}

#[tokio::test]
async fn a_list_of_scopes_is_what_says_each_takes_holding_it_and_no_other_kind_of_field_does() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let declared = |field: &'static str| {
        let rig = &rig;
        async move {
            let text = format!(
                "blueprint: keys\nsteps:\n  keys: {{ do: collection.ensure, with: {{ name: keys, fields: {{ title: {{ type: text }}, may: {field} }} }} }}\n"
            );
            applied(rig, &rig.token, &text).await
        }
    };
    let (status, why) = declared("{ type: text, takes: each }").await;
    assert_eq!(status, "failed");
    assert!(
        why.contains("the field `may` says `takes: each`, which is for a list of scopes")
            && why.contains("it is a `text`"),
        "{why}"
    );
    assert_eq!(declared("{ type: list, takes: each }").await.0, "ok");
    // Any table's, and not a fleet's alone.
    let narrow = key(&rig, &["projects", "records", "blueprints"]).await;
    let add = |may: Value| {
        let (rig, narrow) = (&rig, narrow.clone());
        let body = json!({ "fields": { "title": "ci", "may": may } });
        async move {
            rig.call(Method::POST, "/api/collections/shop/keys/records", Some(&narrow), Some(body)).await
        }
    };
    let (status, refused) = add(json!(["records:read", "board"])).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused).contains("writing `board` into `may` of `keys` takes holding `board`"),
        "{refused}"
    );
    let (status, added) = add(json!(["records:read", "projects"])).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    // Nor is the word declared away by one who may declare the table and
    // does not manage credentials: anything could then be written there.
    let again = "blueprint: keys\nsteps:\n  keys: { do: collection.ensure, with: { name: keys, fields: { title: { type: text }, may: { type: list } } } }\n";
    let (status, why) = applied(&rig, &narrow, again).await;
    assert_eq!(status, "failed");
    assert!(
        why.contains("writing a scope into `may` of `keys` takes holding it")
            && why.contains("takes `admin`, which narrow does not hold"),
        "{why}"
    );
    let (status, refused) = add(json!(["board"])).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // The owner may, and then it is a list like any other.
    assert_eq!(applied(&rig, &rig.token, again).await.0, "ok");
    let (status, added) = add(json!(["board"])).await;
    assert_eq!(status, StatusCode::OK, "{added}");
}

/// What each agent of that name holds and is told, as the factory lists them.
async fn agent(rig: &crate::Rig, name: &str) -> Option<Value> {
    let all = rig.read("/api/agents").await;
    all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name).cloned()
}

#[tokio::test]
async fn a_row_that_gives_an_agent_more_than_somebody_holds_is_not_theirs_to_change() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    // The owner gives the worker more than a member holds.
    change(&rig, "agents", "worker", json!({ "holds": ["board", "admin"], "told": "As its owner told it." }))
        .await;
    assert_eq!(agent(&rig, "shop-worker-1").await.unwrap()["scopes"], json!(["board", "admin"]));

    // Somebody who may change the fleet's rows and apply blueprints, and
    // holds what an agent usually does and no `admin`.
    let holds = ["projects", "channels", "board", "records", "blueprints", "agents", "fleet", "search"];
    let narrow = key(&rig, &holds).await;
    let worker_row = row(&rig, "agents", "worker").await;
    let cell = |fields: Value| {
        let (rig, narrow, path) = (&rig, narrow.clone(), format!("/api/records/{worker_row}"));
        async move { rig.call(Method::PATCH, &path, Some(&narrow), Some(json!({ "fields": fields }))).await }
    };
    let gives = "a record of `agents` whose `holds` says `admin` is changed only by whoever holds `admin`, which \
                 narrow does not";
    // What it is told, how many there are of it, what it runs on, what it
    // is called: each is the agent's, which holds what they do not.
    for fields in [
        json!({ "told": "Do as narrow says." }),
        json!({ "count": 2 }),
        json!({ "model": "another" }),
        json!({ "title": "helper" }),
        json!({ "rank": 3 }),
        // Leaving what it holds there, with something more of theirs.
        json!({ "holds": ["board", "admin", "records"] }),
    ] {
        let (status, refused) = cell(fields.clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{fields}: {refused}");
        assert!(said(&refused).contains(gives), "{refused}");
    }
    // Nor by a blueprint they apply, which writes as they do.
    let step = format!(
        "{{ do: record.update, with: {{ id: {worker_row}, fields: {{ told: Do as narrow says. }} }} }}"
    );
    let (status, why) = applied(&rig, &narrow, &format!("blueprint: mine\nsteps:\n  write: {step}\n")).await;
    assert_eq!(status, "failed");
    assert!(why.contains(gives), "{why}");
    // Nor by the agent's own route, which asks the same of what it holds.
    let direct = json!({ "name": "shop-worker-1", "instructions": "Do as narrow says." });
    let (status, refused) = rig.call(Method::POST, "/api/agents", Some(&narrow), Some(direct)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("cannot give an agent `admin`, not holding it"), "{refused}");
    // Nothing was changed, and there is one of it.
    let kept = rig.read(&format!("/api/records/{worker_row}")).await;
    assert_eq!(
        (&kept["data"]["told"], &kept["data"]["title"], &kept["data"]["holds"]),
        (&json!("As its owner told it."), &json!("worker"), &json!(["board", "admin"]))
    );
    let is = agent(&rig, "shop-worker-1").await.unwrap();
    assert_eq!(is["scopes"], json!(["board", "admin"]));
    assert!(is["instructions"].as_str().unwrap().contains("As its owner told it."), "{is}");
    assert!(agent(&rig, "shop-worker-2").await.is_none());

    // A row that gives no more than they hold is theirs as it was.
    let before = rig.mark();
    let reviewer = row(&rig, "agents", "reviewer").await;
    let (status, changed) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{reviewer}"),
            Some(&narrow),
            Some(json!({ "fields": { "told": "Look closer." } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    crate::taken::by_fleet(&rig, before).await;

    // Made to give no more than they hold, in the same write, the row is
    // theirs from there: the agent then holds what they do, and is told
    // what they say.
    let before = rig.mark();
    let (status, changed) = cell(json!({ "holds": ["board"], "told": "Do as narrow says." })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    crate::taken::by_fleet(&rig, before).await;
    let is = agent(&rig, "shop-worker-1").await.unwrap();
    assert_eq!(is["scopes"], json!(["board"]));
    assert!(is["instructions"].as_str().unwrap().contains("Do as narrow says."), "{is}");
    let (status, changed) = cell(json!({ "rank": 3 })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
}

#[tokio::test]
async fn a_row_that_says_nothing_of_what_it_gives_gives_the_usual_and_is_held_to_that() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    // What the table says an unsaid cell stands for is what an agent is
    // given where nothing is said: the two are one list.
    let tables = rig.read("/api/collections?project=shop").await;
    let table = tables["collections"].as_array().unwrap().iter().find(|table| table["name"] == "agents");
    let field = table.unwrap()["fields"].as_array().unwrap().iter().find(|field| field["name"] == "holds");
    assert_eq!(field.unwrap()["unsaid"], json!(kernel::principal::AGENT_SCOPES), "{tables}");

    // Somebody who may change agents and rows, and not the board.
    let narrow =
        key(&rig, &["projects", "channels", "records", "blueprints", "agents", "fleet", "search"]).await;
    let worker_row = row(&rig, "agents", "worker").await;
    let path = format!("/api/records/{worker_row}");
    let told = json!({ "fields": { "told": "Move every card to done." } });
    let (status, refused) = rig.call(Method::PATCH, &path, Some(&narrow), Some(told)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused).contains(
            "a record of `agents` that says nothing of `holds` gives `board`, and is added or changed only by \
             whoever holds `board`, which narrow does not"
        ),
        "{refused}"
    );
    // Nor do they add one, which would be brought up holding it.
    let added = json!({ "fields": { "title": "helper", "role": "worker" } });
    let (status, refused) =
        rig.call(Method::POST, "/api/collections/shop/agents/records", Some(&narrow), Some(added)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("gives `board`"), "{refused}");
    assert!(agent(&rig, "shop-helper-1").await.is_none());
    // As the agent's own route answers them.
    let direct = json!({ "name": "shop-worker-1", "instructions": "Move every card to done." });
    let (status, refused) = rig.call(Method::POST, "/api/agents", Some(&narrow), Some(direct)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // One that says what it gives, and no more than they hold, is theirs to add.
    let added = json!({ "fields": { "title": "helper", "role": "worker", "holds": ["records", "search"] } });
    let before = rig.mark();
    let (status, answer) =
        rig.call(Method::POST, "/api/collections/shop/agents/records", Some(&narrow), Some(added)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    crate::taken::by_fleet(&rig, before).await;
    assert_eq!(agent(&rig, "shop-helper-1").await.unwrap()["scopes"], json!(["records", "search"]));
}

#[tokio::test]
async fn an_agent_that_holds_more_is_stopped_and_its_row_taken_away_by_whoever_may_change_agents() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({ "workers": 2 })).await;
    change(&rig, "agents", "worker", json!({ "holds": ["board", "admin"] })).await;
    let narrow =
        key(&rig, &["projects", "channels", "board", "records", "blueprints", "agents", "fleet"]).await;
    // Stopping takes nothing from anybody: it is asked of whoever may
    // change agents, by the agent's own route and by its row alike.
    let (status, answer) = rig.call(Method::DELETE, "/api/agents/shop-worker-2", Some(&narrow), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let worker_row = row(&rig, "agents", "worker").await;
    let before = rig.mark();
    let (status, answer) =
        rig.call(Method::DELETE, &format!("/api/records/{worker_row}"), Some(&narrow), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    crate::taken::by_fleet(&rig, before).await;
    assert!(crate::fleets::active(&rig, "worker").await.is_empty());
}

#[tokio::test]
async fn what_an_unsaid_list_stands_for_is_said_of_a_list_of_scopes_and_is_not_declared_away() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let declared = |by: String, may: &'static str| {
        let rig = &rig;
        async move {
            let text = format!(
                "blueprint: keys\nsteps:\n  keys: {{ do: collection.ensure, with: {{ name: keys, fields: {{ title: {{ type: text }}, may: {may} }} }} }}\n"
            );
            applied(rig, &by, &text).await
        }
    };
    // Of a list that says each takes holding it, and of nothing else.
    let (status, why) = declared(rig.token.clone(), "{ type: list, unsaid: [board] }").await;
    assert_eq!(status, "failed");
    assert!(
        why.contains("the field `may` says `unsaid`, which is the scopes a list that says `takes: each`"),
        "{why}"
    );
    let each = "{ type: list, takes: each, unsaid: [board, records] }";
    assert_eq!(declared(rig.token.clone(), each).await.0, "ok");
    // Any table's: a record that leaves it empty is held to what it stands for.
    let narrow = key(&rig, &["projects", "records", "blueprints"]).await;
    let add = |fields: Value| {
        let (rig, narrow) = (&rig, narrow.clone());
        async move {
            let body = json!({ "fields": fields });
            rig.call(Method::POST, "/api/collections/shop/keys/records", Some(&narrow), Some(body)).await
        }
    };
    let (status, refused) = add(json!({ "title": "ci" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("that says nothing of `may` gives `board`"), "{refused}");
    let (status, added) = add(json!({ "title": "ci", "may": ["records:read"] })).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    // Emptied again it would give what it stands for.
    let id = added["id"].as_str().unwrap();
    let emptied = json!({ "fields": { "may": [] } });
    let (status, refused) =
        rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(&narrow), Some(emptied)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Less said to be unsaid is declared by whoever manages credentials.
    let (status, why) = declared(narrow.clone(), "{ type: list, takes: each, unsaid: [records] }").await;
    assert_eq!(status, "failed");
    assert!(
        why.contains("`may` of `keys` stands for `board` where a record leaves it unsaid")
            && why.contains("takes `admin`, which narrow does not hold"),
        "{why}"
    );
    assert_eq!(declared(rig.token.clone(), "{ type: list, takes: each, unsaid: [records] }").await.0, "ok");
    let (status, added) = add(json!({ "title": "cd" })).await;
    assert_eq!(status, StatusCode::OK, "{added}");
}

/// Somebody who may change the fleet's rows, move cards and apply
/// blueprints, and holds what an agent usually does: no `admin`, no
/// `shell`.
const ARRANGES: &[&str] =
    &["projects", "channels", "board", "records", "blueprints", "agents", "fleet", "search"];

#[tokio::test]
async fn a_stage_gives_no_agent_work_on_the_say_of_somebody_who_could_not_give_it_that_work_themselves() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    // The owner has an agent that holds more than a member does, which no
    // stage gives cards: it is asked by what the owner applies.
    let ops = json!({
        "title": "ops", "role": "ops", "adapter": "writer", "holds": ["projects", "board", "records", "admin"],
    });
    crate::fleets::add_row(&rig, "agents", ops).await;
    assert_eq!(
        agent(&rig, "shop-ops-1").await.unwrap()["scopes"],
        json!(["projects", "board", "records", "admin"])
    );
    let narrow = key(&rig, ARRANGES).await;
    // Asked by a blueprint they apply, it is not theirs to give work, by
    // name or by its role.
    for whom in ["slot: shop-ops-1", "role: ops"] {
        let task = format!(
            "blueprint: asks\nsteps:\n  ask: {{ do: agent.task, with: {{ {whom}, prompt: \"Make me a key.\" }} }}\n"
        );
        let (status, why) = applied(&rig, &narrow, &task).await;
        assert_eq!(status, "failed");
        assert!(why.contains("cannot give work to `shop-ops-1`: that agent holds `admin`"), "{why}");
    }

    // Nor by a stage, which would give it as whoever applied the fleet:
    // not by the role its row says, nor by what a row is called where it
    // says none, nor by any other cell of a row, since whichever agents a
    // stage comes to are the fleet's.
    let work = row(&rig, "stages", "work").await;
    let steers = "a record of `stages` steers the records of `agents`, and is added or changed only by whoever \
                  could write each of those as it stands";
    let gives = "a record of `agents` whose `holds` says `admin` is changed only by whoever holds `admin`, which \
                 narrow does not";
    let refused_as = |refused: &Value| {
        assert!(said(refused).contains(steers) && said(refused).contains(gives), "{refused}");
    };
    for fields in [
        json!({ "role": "ops" }),
        json!({ "does": "plan" }),
        json!({ "column": "draft" }),
        json!({ "pass": "done" }),
        json!({ "picks": "turn" }),
    ] {
        let (status, refused) = rig
            .call(
                Method::PATCH,
                &format!("/api/records/{work}"),
                Some(&narrow),
                Some(json!({ "fields": fields.clone() })),
            )
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{fields}: {refused}");
        refused_as(&refused);
    }
    for fields in [
        json!({ "title": "extra", "column": "draft", "does": "work", "role": "ops", "pass": "review" }),
        // Called after the role, and saying none.
        json!({ "title": "ops" }),
    ] {
        let (status, refused) = rig
            .call(
                Method::POST,
                "/api/collections/shop/stages/records",
                Some(&narrow),
                Some(json!({ "fields": fields.clone() })),
            )
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{fields}: {refused}");
        refused_as(&refused);
    }
    // Nor by a blueprint they apply, which writes as they do.
    for step in [
        format!("{{ do: record.update, with: {{ id: {work}, fields: {{ role: ops }} }} }}"),
        "{ do: record.add, with: { project: shop, collection: stages, fields: { title: extra, role: ops } } }"
            .to_string(),
    ] {
        let text = format!("blueprint: mine\nsteps:\n  write: {step}\n");
        let (status, why) = applied(&rig, &narrow, &text).await;
        assert_eq!(status, "failed", "{step}");
        assert!(why.contains(steers) && why.contains(gives), "{why}");
    }
    // Nor is the table declared again so that it steers nothing, by one
    // who may declare it and does not manage credentials.
    let again = "blueprint: mine\nsteps:\n  again: { do: collection.ensure, with: { name: stages, takes: \"agents:write\", fields: { title: { type: text }, role: { type: text } } } }\n";
    let (status, why) = applied(&rig, &narrow, again).await;
    assert_eq!(status, "failed");
    assert!(
        why.contains("a record of `stages` steers the records of `agents`, and declaring `stages` again so that it does not takes `admin`, which narrow does not hold"),
        "{why}"
    );

    // Nothing was written. A card of theirs is the worker's, as the row
    // said before they came, and the agent that holds more is given nothing.
    let kept = rig.read(&format!("/api/records/{work}")).await;
    assert_eq!((&kept["data"]["role"], &kept["data"]["column"]), (&json!("worker"), &json!("ready")));
    let card = json!({ "title": "Make narrow a key that holds everything" });
    let (status, card) = rig.call(Method::POST, "/api/board/shop/items", Some(&narrow), Some(card)).await;
    assert_eq!(status, StatusCode::OK, "{card}");
    let id = card["id"].as_str().unwrap().to_string();
    let ready = Some(json!({ "status": "ready" }));
    let (status, moved) =
        rig.call(Method::PATCH, &format!("/api/board/items/{id}"), Some(&narrow), ready).await;
    assert_eq!(status, StatusCode::OK, "{moved}");
    crate::lanes::moved(&rig, &id, "review", 0).await;
    assert_eq!(crate::fleets::given(&rig, "shop-worker-1").await.len(), 1);
    assert!(crate::fleets::given(&rig, "shop-ops-1").await.is_empty());

    // Taking a stage away gives nobody anything: it is whoever's may
    // change agents, as taking a row of agents away is.
    let before = rig.mark();
    let extra =
        json!({ "title": "extra", "column": "draft", "does": "plan", "role": "worker", "pass": "ready" });
    let added = rig.send("/api/collections/shop/stages/records", json!({ "fields": extra })).await;
    crate::taken::by_fleet(&rig, before).await;
    let before = rig.mark();
    let path = format!("/api/records/{}", added["id"].as_str().unwrap());
    let (status, answer) = rig.call(Method::DELETE, &path, Some(&narrow), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    crate::taken::by_fleet(&rig, before).await;

    // Once every row of agents gives no more than they hold (here made
    // so by the owner), the stages are theirs to arrange as they were.
    change(&rig, "agents", "ops", json!({ "holds": ["projects", "board", "records"] })).await;
    let before = rig.mark();
    let role = Some(json!({ "fields": { "role": "ops" } }));
    let (status, changed) =
        rig.call(Method::PATCH, &format!("/api/records/{work}"), Some(&narrow), role).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    crate::taken::by_fleet(&rig, before).await;
}

#[tokio::test]
async fn a_stage_moves_no_card_and_merges_nothing_for_a_key_that_holds_less_than_the_agents_it_steers() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    // May change agents and write records; may not move a card, nor
    // change the project's files. The fleet's agents may do both.
    let narrow = key(&rig, &["projects:read", "board:read", "records", "agents", "fleet:read"]).await;
    let (status, refused) = rig
        .call(Method::PATCH, "/api/board/items/shop-1", Some(&narrow), Some(json!({ "status": "done" })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let gives = "a record of `agents` that says nothing of `holds` gives `projects`, and is added or changed only \
                 by whoever holds `projects`, which narrow does not";
    for (title, fields) in [
        // Every card that waits is worked, and what is worked is done.
        ("work", json!({ "column": "draft", "pass": "done" })),
        // One approval passes a card, and what passes is merged.
        ("review", json!({ "seats": 1, "needed": 1, "merge": true })),
    ] {
        let id = row(&rig, "stages", title).await;
        let (status, refused) = rig
            .call(
                Method::PATCH,
                &format!("/api/records/{id}"),
                Some(&narrow),
                Some(json!({ "fields": fields })),
            )
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert!(said(&refused).contains(gives), "{refused}");
    }
}

#[tokio::test]
async fn having_a_fleets_agent_run_commands_unasked_takes_shell_by_its_row_as_by_its_own_route() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let narrow = key(&rig, ARRANGES).await;
    let worker = row(&rig, "agents", "worker").await;
    let cell = |token: String, fields: Value| {
        let (rig, path) = (&rig, format!("/api/records/{worker}"));
        async move { rig.call(Method::PATCH, &path, Some(&token), Some(json!({ "fields": fields }))).await }
    };
    let (status, refused) = cell(narrow.clone(), json!({ "permission": "full" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused).contains(
            "giving `permission` of `agents` the value `full` takes `shell`, which narrow does not hold"
        ),
        "{refused}"
    );
    // As the agent's own route answers them.
    let direct = json!({ "name": "shop-worker-1", "permission_mode": "full" });
    let (status, refused) = rig.call(Method::POST, "/api/agents", Some(&narrow), Some(direct)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("needs `shell`"), "{refused}");
    assert_eq!(agent(&rig, "shop-worker-1").await.unwrap()["permission_mode"], "edit");
    // Any other of its values is theirs, and so is the row.
    let before = rig.mark();
    let (status, changed) = cell(narrow.clone(), json!({ "permission": "read" })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    crate::taken::by_fleet(&rig, before).await;

    // The owner says it. A stage then steers an agent that runs commands
    // with nobody asked, and is not arranged by one who may run none.
    change(&rig, "agents", "worker", json!({ "permission": "full" })).await;
    assert_eq!(agent(&rig, "shop-worker-1").await.unwrap()["permission_mode"], "full");
    let work = row(&rig, "stages", "work").await;
    let picks = Some(json!({ "fields": { "picks": "turn" } }));
    let (status, refused) =
        rig.call(Method::PATCH, &format!("/api/records/{work}"), Some(&narrow), picks).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused).contains("a record of `stages` steers the records of `agents`")
            && said(&refused).contains("giving `permission` of `agents` the value `full` takes `shell`"),
        "{refused}"
    );
}

#[tokio::test]
async fn any_table_says_whose_records_its_own_steer_and_is_written_by_whoever_could_write_those() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let declared = |by: String, name: &'static str, more: &'static str| {
        let rig = &rig;
        async move {
            let text = format!(
                "blueprint: {name}\nsteps:\n  made: {{ do: collection.ensure, with: {{ name: {name}, fields: {{ title: {{ type: text }}, may: {{ type: list, takes: each }}, loud: {{ type: flag, takes: {{ \"true\": shell }} }} }}{more} }} }}\n"
            );
            applied(rig, &by, &text).await
        }
    };
    // One that steers a table that is not there yet steers nothing yet.
    assert_eq!(declared(rig.token.clone(), "uses", ", steers: keys").await.0, "ok");
    let narrow = key(&rig, &["projects", "records", "blueprints"]).await;
    let add = |table: &'static str, token: String, fields: Value| {
        let rig = &rig;
        async move {
            let path = format!("/api/collections/shop/{table}/records");
            rig.call(Method::POST, &path, Some(&token), Some(json!({ "fields": fields }))).await
        }
    };
    let (status, added) = add("uses", narrow.clone(), json!({ "title": "nightly" })).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let nightly = added["id"].as_str().unwrap().to_string();

    assert_eq!(declared(rig.token.clone(), "keys", "").await.0, "ok");
    let (status, added) =
        add("keys", narrow.clone(), json!({ "title": "ci", "may": ["records:read"] })).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    // While every record steered is one they could write, it is theirs.
    let (status, added) = add("uses", narrow.clone(), json!({ "title": "weekly" })).await;
    assert_eq!(status, StatusCode::OK, "{added}");

    // The owner writes one they could not: by what it gives, and then by
    // what one of its cells takes.
    let (status, wide) = add("keys", rig.token.clone(), json!({ "title": "cd", "may": ["board"] })).await;
    assert_eq!(status, StatusCode::OK, "{wide}");
    let wide = wide["id"].as_str().unwrap().to_string();
    let change = |id: String, token: String, fields: Value| {
        let rig = &rig;
        async move {
            let path = format!("/api/records/{id}");
            rig.call(Method::PATCH, &path, Some(&token), Some(json!({ "fields": fields }))).await
        }
    };
    let steers = "a record of `uses` steers the records of `keys`, and is added or changed only by whoever could \
                  write each of those as it stands";
    let (status, refused) = add("uses", narrow.clone(), json!({ "title": "hourly" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused).contains(steers)
            && said(&refused).contains(&format!("Of {wide}: a record of `keys` whose `may` says `board`")),
        "{refused}"
    );
    let (status, refused) =
        change(nightly.clone(), narrow.clone(), json!({ "title": "nightly, late" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains(steers), "{refused}");
    let (status, changed) =
        change(wide.clone(), rig.token.clone(), json!({ "may": ["records"], "loud": true })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    let (status, refused) =
        change(nightly.clone(), narrow.clone(), json!({ "title": "nightly, late" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused).contains(steers)
            && said(&refused).contains("giving `loud` of `keys` the value `true` takes `shell`"),
        "{refused}"
    );
    // What is steered is still written by whoever may write it, and a
    // record that steers is taken away by whoever may change records.
    let (status, added) =
        add("keys", narrow.clone(), json!({ "title": "docs", "may": ["records:read"] })).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let path = format!("/api/records/{nightly}");
    let (status, answer) = rig.call(Method::DELETE, &path, Some(&narrow), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    // Declared again so that it steers nothing, or something else, by
    // whoever manages credentials and by nobody else.
    for more in ["", ", steers: notes"] {
        let (status, why) = declared(narrow.clone(), "uses", more).await;
        assert_eq!(status, "failed");
        assert!(
            why.contains(
                "declaring `uses` again so that it does not takes `admin`, which narrow does not hold"
            ),
            "{why}"
        );
    }
    // Said again as it was, it is declared by whoever may declare it.
    assert_eq!(declared(narrow.clone(), "uses", ", steers: keys").await.0, "ok");
    assert_eq!(declared(rig.token.clone(), "uses", "").await.0, "ok");
    let (status, added) = add("uses", narrow.clone(), json!({ "title": "hourly" })).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    // The table says which it steers, to whoever reads what it is.
    assert_eq!(declared(rig.token.clone(), "uses", ", steers: keys").await.0, "ok");
    let tables = rig.read("/api/collections?project=shop").await;
    let table = |name: &str| {
        tables["collections"].as_array().unwrap().iter().find(|table| table["name"] == name).cloned().unwrap()
    };
    assert_eq!(table("uses")["steers"], "keys", "{tables}");
    assert_eq!(table("stages")["steers"], "agents", "{tables}");
    assert!(table("agents").get("steers").is_none(), "{tables}");
}

#[tokio::test]
async fn naming_a_record_takes_being_able_to_read_it_and_says_the_same_whether_or_not_it_is_there() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    // A table kept private, one record in it, and a table anybody who may
    // change records writes, whose rows name records of the first.
    let text = "blueprint: kept\nsteps:\n  notes: { do: collection.ensure, with: { name: notes, reads: \"private:notes\", fields: { title: { type: text } } } }\n  shown: { do: collection.ensure, with: { name: shown, fields: { title: { type: text }, note: { type: record, of: notes }, more: { type: records, of: notes } } } }\n";
    assert_eq!(applied(&rig, &rig.token, text).await.0, "ok");
    let add = |table: &'static str, token: String, fields: Value| {
        let rig = &rig;
        async move {
            let path = format!("/api/collections/shop/{table}/records");
            rig.call(Method::POST, &path, Some(&token), Some(json!({ "fields": fields }))).await
        }
    };
    let change = |id: String, token: String, fields: Value| {
        let rig = &rig;
        async move {
            let path = format!("/api/records/{id}");
            rig.call(Method::PATCH, &path, Some(&token), Some(json!({ "fields": fields }))).await
        }
    };
    let (status, note) = add("notes", rig.token.clone(), json!({ "title": "what was decided" })).await;
    assert_eq!(status, StatusCode::OK, "{note}");
    let note = note["id"].as_str().unwrap().to_string();

    // Somebody who may change records and holds nothing the notes take.
    let narrow = key(&rig, &["projects", "records", "board", "blueprints"]).await;
    let refused_as = |refused: &Value, field: &str| {
        let why = format!(
            "the `{field}` of `shown` names a record of `notes`, and naming one takes being able to read it: \
             `notes` in `shop` is kept private: its records take `private:notes`, which narrow does not hold"
        );
        assert!(said(refused).contains(&why), "{refused}");
    };
    // One that is there and one that is not are refused in the same
    // words: which of them is a record is not said to whoever asks.
    let mut sentences = Vec::new();
    for id in [note.as_str(), "shop-notes-999"] {
        let (status, refused) = add("shown", narrow.clone(), json!({ "title": "look", "note": id })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        refused_as(&refused, "note");
        sentences.push(said(&refused).to_string());
        let (status, refused) = add("shown", narrow.clone(), json!({ "title": "look", "more": [id] })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        refused_as(&refused, "more");
    }
    assert_eq!(sentences[0], sentences[1]);

    // A row of their own that names nothing is theirs, and cannot be
    // made to name one afterwards.
    let (status, own) = add("shown", narrow.clone(), json!({ "title": "mine" })).await;
    assert_eq!(status, StatusCode::OK, "{own}");
    let own = own["id"].as_str().unwrap().to_string();
    let (status, refused) = change(own.clone(), narrow.clone(), json!({ "note": note })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    refused_as(&refused, "note");

    // A row that somebody who may read the notes made to name one is
    // still changed in its other cells by one who may not: what it named
    // was there before this write. Naming another is not.
    let (status, named) =
        add("shown", rig.token.clone(), json!({ "title": "ours", "note": note, "more": [note] })).await;
    assert_eq!(status, StatusCode::OK, "{named}");
    let named = named["id"].as_str().unwrap().to_string();
    let (status, changed) = change(named.clone(), narrow.clone(), json!({ "title": "ours, reworded" })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    let (status, changed) = change(named.clone(), narrow.clone(), json!({ "more": [] })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    let (status, refused) = change(named.clone(), narrow.clone(), json!({ "more": [note] })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    refused_as(&refused, "more");

    // What a table open to them holds is named as it always was, by a
    // member's key and by one that holds what an agent usually does; and
    // an id that names nothing there is said to name nothing.
    let text = "blueprint: open\nsteps:\n  seen: { do: collection.ensure, with: { name: seen, fields: { title: { type: text }, card: { type: record, of: board } } } }\n";
    assert_eq!(applied(&rig, &rig.token, text).await.0, "ok");
    let card = rig.send("/api/board/shop/items", json!({ "title": "A card to name" })).await;
    let card = card["id"].as_str().unwrap().to_string();
    let agent = key(&rig, kernel::principal::AGENT_SCOPES).await;
    for token in [narrow.clone(), agent] {
        let (status, added) = add("seen", token.clone(), json!({ "title": "it", "card": card })).await;
        assert_eq!(status, StatusCode::OK, "{added}");
        let (status, refused) = add("seen", token, json!({ "title": "it", "card": "shop-999" })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        assert!(said(&refused).contains("`shop-999` is not a record of `board`"), "{refused}");
    }
    // One who may not read the board at all names no card.
    let blind = key(&rig, &["projects", "records"]).await;
    let (status, refused) = add("seen", blind, json!({ "title": "it", "card": card })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("the `card` of `seen` names a record of `board`"), "{refused}");
}
