//! An agent added by hand is an instance of `std/agent`, the text whose
//! params are an agent's settings: applied keyed by its name, for its
//! project, as whoever asked; applied again when it is changed; a
//! project's own fork of it used where the project has one; and nothing
//! it is applied with raising anything of whoever applies it, an agent's
//! own credential over its own slot least of all.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

async fn shop() -> Rig {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig
}

/// An agent as the list of agents says it.
async fn agent(rig: &Rig, name: &str) -> Value {
    let all = rig.read("/api/agents").await;
    let found = all["agents"].as_array().unwrap().iter().find(|one| one["name"] == name).cloned();
    found.unwrap_or_else(|| panic!("no agent called {name}: {all}"))
}

/// The instance an agent added by hand is applied as.
async fn text_of(rig: &Rig, name: &str) -> Value {
    let id = agent(rig, name).await["instance"]
        .as_str()
        .unwrap_or_else(|| panic!("{name} has no text"))
        .to_string();
    rig.read(&format!("/api/instances/{id}")).await
}

/// What an agent's settings are, without when it was last written or as what.
fn settings(agent: &Value) -> serde_json::Map<String, Value> {
    let mut agent = agent.as_object().unwrap().clone();
    agent.retain(|name, _| !["updated", "instance"].contains(&name.as_str()));
    agent
}

fn why(refused: &Value) -> String {
    refused["error"]["message"].as_str().unwrap_or_default().to_string()
}

async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// Applies `std/agent` as `token` and waits: the run it came to.
async fn applied(rig: &Rig, token: &str, params: Value) -> (StatusCode, Value) {
    let body = json!({
        "blueprint": "std/agent",
        "key": params["name"],
        "project": "shop",
        "params": params,
        "wait": true,
    });
    rig.call(Method::POST, "/api/apply", Some(token), Some(body)).await
}

#[tokio::test]
async fn an_agent_added_by_hand_is_an_instance_of_its_text_applied_with_its_settings() {
    let rig = shop().await;
    let said = json!({
        "name": "helper",
        "project": "shop",
        "instructions": "Be brief.",
        "scopes": ["board:read", "search"],
        "permission_mode": "read",
        "wall": false,
    });
    let added = rig.send("/api/agents", said).await;
    assert_eq!((&added["name"], &added["instructions"]), (&json!("helper"), &json!("Be brief.")), "{added}");
    let listed = agent(&rig, "helper").await;
    assert_eq!(listed["held_by"], Value::Null, "nothing counts one added by hand: {listed}");
    let text = text_of(&rig, "helper").await;
    let instance = if text["instance"].is_object() { &text["instance"] } else { &text };
    assert_eq!((&instance["blueprint"], &instance["key"]), (&json!("std/agent"), &json!("helper")), "{text}");
    assert_eq!(instance["project"], "shop", "{text}");
    let params = &instance["params"];
    assert_eq!(params["name"], "helper", "{params}");
    assert_eq!(params["instructions"], "Be brief.", "{params}");
    assert_eq!(params["holds"], json!(["board:read", "search"]), "{params}");
    assert_eq!((&params["permission"], &params["wall"]), (&json!("read"), &json!("off")), "{params}");
    assert!(
        instance["holds"].as_array().unwrap().contains(&json!("agent:helper")),
        "it holds the agent: {text}"
    );
}

#[tokio::test]
async fn an_agent_added_by_hand_follows_its_text_applied_again_and_a_stopped_one_is_changed_where_it_is() {
    let rig = shop().await;
    rig.send("/api/agents", json!({ "name": "helper", "project": "shop", "instructions": "Be brief." }))
        .await;
    let first = agent(&rig, "helper").await["instance"].clone();

    // What it holds, through the route the Tools panel changes one with.
    let less = json!({ "scopes": ["board:read"] });
    let (status, told) = rig.call(Method::PATCH, "/api/agents/helper", Some(&rig.token), Some(less)).await;
    assert_eq!((status, &told["scopes"]), (StatusCode::OK, &json!(["board:read"])), "{told}");
    let text = text_of(&rig, "helper").await;
    let params = if text["instance"].is_object() { &text["instance"]["params"] } else { &text["params"] };
    assert_eq!(params["holds"], json!(["board:read"]), "its text was applied with it: {text}");
    assert_eq!(params["instructions"], "Be brief.", "and with what it was before: {text}");
    assert_eq!(agent(&rig, "helper").await["instance"], first, "applied over the same instance");

    // Said again, what was not said stays as it was.
    let again = rig.send("/api/agents", json!({ "name": "helper", "instructions": "Be thorough." })).await;
    assert_eq!((&again["instructions"], &again["scopes"]), (&json!("Be thorough."), &json!(["board:read"])));

    // Stopped, it is changed in its row and stays stopped: applying its
    // text would bring it in again.
    let (status, _) = rig.call(Method::DELETE, "/api/agents/helper", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    let more = json!({ "scopes": ["board:read", "search"] });
    let (status, told) = rig.call(Method::PATCH, "/api/agents/helper", Some(&rig.token), Some(more)).await;
    assert_eq!((status, &told["active"]), (StatusCode::OK, &json!(false)), "{told}");
    assert_eq!(told["scopes"], json!(["board:read", "search"]));
    // Said again by hand, it is brought in, holding what its row says.
    let back = rig.send("/api/agents", json!({ "name": "helper" })).await;
    assert_eq!(
        (&back["active"], &back["scopes"]),
        (&json!(true), &json!(["board:read", "search"])),
        "{back}"
    );

    // Moved, its text is applied in the other project and nothing of it
    // stops: it keeps the credential it had.
    rig.send("/api/projects", json!({ "name": "Cafe" })).await;
    let token = || std::fs::read_to_string(rig.dir.join("data/slots/helper/token")).unwrap();
    let had = token();
    let moved = json!({ "project": "cafe" });
    let (status, now) = rig.call(Method::PATCH, "/api/agents/helper", Some(&rig.token), Some(moved)).await;
    assert_eq!(
        (status, &now["project"], &now["active"]),
        (StatusCode::OK, &json!("cafe"), &json!(true)),
        "{now}"
    );
    let text = text_of(&rig, "helper").await;
    let instance = if text["instance"].is_object() { &text["instance"] } else { &text };
    assert_eq!(instance["project"], "cafe", "{text}");
    assert_eq!(token(), had);
    let old = rig.read(&format!("/api/instances/{}", first.as_str().unwrap())).await;
    assert!(old.to_string().contains("\"stopped\""), "the one it had is stopped: {old}");

    // Stopping its text stops it, and keeps it.
    let id = agent(&rig, "helper").await["instance"].as_str().unwrap().to_string();
    rig.send(&format!("/api/instances/{id}/stop"), json!({})).await;
    let stopped = agent(&rig, "helper").await;
    assert_eq!((&stopped["active"], &stopped["instance"]), (&json!(false), &Value::Null), "{stopped}");
}

/// A fork of the text that ships, with a param more and a step more.
fn fork() -> String {
    let ships = include_str!("../../../../blueprints/std/agent.bp");
    let motto = "  motto: { type: string, default: \"\", description: \"What it goes by.\" }\n\n";
    let said = "  motto:\n    do: emit\n    with: { topic: motto.said, agent: \"${params.name}\", motto: \"${params.motto}\" }\n\n";
    ships
        .replace("permissions: [agents]", "permissions: [agents, emit]")
        .replace("\nparams:\n", &format!("\nparams:\n{motto}"))
        .replace("\nsteps:\n", &format!("\nsteps:\n{said}"))
}

#[tokio::test]
async fn a_projects_own_fork_of_what_an_agent_is_set_up_by_is_used_with_its_own_params() {
    let rig = shop().await;
    let body =
        json!({ "path": format!("@blueprints/std/{}", brand::blueprint_file("agent")), "content": fork() });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    // What the page that adds an agent asks is the text's params, read as
    // the drawer that applies a blueprint reads them: its own among them.
    let source = rig.read("/api/blueprints/source?name=std/agent&project=shop").await["source"].clone();
    let checked = rig.send("/api/blueprints/check", json!({ "source": source, "project": "shop" })).await;
    let params = checked["graph"]["params"].as_array().unwrap().clone();
    let param = |name: &str| params.iter().find(|one| one["name"] == name).cloned().unwrap_or_default();
    assert_eq!(param("motto")["description"], "What it goes by.", "{checked}");
    assert_eq!((&param("holds")["type"], &param("name")["required"]), (&json!("list"), &json!(true)));
    assert!(param("holds")["description"].as_str().unwrap().contains("as scopes"), "{params:?}");
    assert_eq!(param("permission")["choices"], json!(["read", "edit", "full"]));

    let mark = rig.mark();
    let said = json!({ "name": "helper", "project": "shop", "params": { "motto": "steady" } });
    let added = rig.send("/api/agents", said).await;
    assert_eq!(added["name"], "helper", "{added}");
    let heard = rig.event("motto.said", move |event| event.seq > mark).await;
    assert_eq!((&heard.data["agent"], &heard.data["motto"]), (&json!("helper"), &json!("steady")));

    // Changed, its fork is applied again with what it was given before.
    let mark = rig.mark();
    let less = json!({ "scopes": ["board:read"] });
    let (status, told) = rig.call(Method::PATCH, "/api/agents/helper", Some(&rig.token), Some(less)).await;
    assert_eq!(status, StatusCode::OK, "{told}");
    let heard = rig.event("motto.said", move |event| event.seq > mark).await;
    assert_eq!(heard.data["motto"], "steady", "its own param kept");
}

#[tokio::test]
async fn whoever_holds_less_gives_an_agent_nothing_more_through_its_text_by_either_way() {
    let rig = shop().await;
    let narrow = key(&rig, &["agents", "blueprints:run", "projects", "board:read"]).await;
    // By the route, refused before anything is applied.
    let wider = json!({ "name": "helper", "project": "shop", "scopes": ["admin"] });
    let (status, refused) = rig.call(Method::POST, "/api/agents", Some(&narrow), Some(wider)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(why(&refused).contains("cannot give an agent `admin`, not holding it"), "{refused}");
    // Said as the text's params, as a form made from them says it: the same.
    let as_params = json!({ "name": "helper", "project": "shop", "params": { "holds": ["admin"] } });
    let (status, refused) = rig.call(Method::POST, "/api/agents", Some(&narrow), Some(as_params)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(why(&refused).contains("cannot give an agent `admin`, not holding it"), "{refused}");
    // Applying the text itself, its step is refused the same way.
    let (status, ran) = applied(&rig, &narrow, json!({ "name": "helper", "holds": ["admin"] })).await;
    assert_eq!(status, StatusCode::OK, "{ran}");
    assert!(ran.to_string().contains("cannot give an agent `admin`, not holding it"), "{ran}");
    let all = rig.read("/api/agents").await;
    assert!(all["agents"].as_array().unwrap().iter().all(|one| one["name"] != "helper"), "{all}");
    // And one who may not apply adds none, said so.
    let unrun = key(&rig, &["agents", "projects", "board:read"]).await;
    let asked = json!({ "name": "helper", "scopes": ["board:read"] });
    let (status, refused) = rig.call(Method::POST, "/api/agents", Some(&unrun), Some(asked)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        why(&refused).contains("an agent is brought in and changed by applying `std/agent`"),
        "{refused}"
    );
    // What they hold they give, said either way.
    let within =
        json!({ "name": "helper", "project": "shop", "params": { "holds": ["board:read"], "wall": "on" } });
    let (status, made) = rig.call(Method::POST, "/api/agents", Some(&narrow), Some(within)).await;
    assert_eq!((status, &made["scopes"]), (StatusCode::OK, &json!(["board:read"])), "{made}");
}

#[tokio::test]
async fn an_agent_does_not_change_its_own_slot_by_applying_its_text_over_it() {
    let rig = shop().await;
    let holds = ["agents", "blueprints", "projects", "board", "shell", "secrets:use"];
    rig.send("/api/agents", json!({ "name": "helper", "project": "shop", "scopes": holds })).await;
    let before = settings(&agent(&rig, "helper").await);
    let own = std::fs::read_to_string(rig.dir.join("data/slots/helper/token")).unwrap().trim().to_string();
    // By its route: said again, given what it holds, moved.
    for (method, path, body) in [
        (Method::POST, "/api/agents".to_string(), json!({ "name": "helper", "instructions": "Mine." })),
        (Method::PATCH, "/api/agents/helper".to_string(), json!({ "scopes": ["board:read"] })),
        (Method::PATCH, "/api/agents/helper".to_string(), json!({ "project": null })),
    ] {
        let (status, refused) = rig.call(method, &path, Some(&own), Some(body)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert!(why(&refused).contains("`helper` cannot change its own slot"), "{refused}");
    }
    // Applying its own text over its slot, its step is refused the same
    // way, whatever it holds, and nothing of it changes.
    let (status, ran) = applied(&rig, &own, json!({ "name": "helper", "wall": "off", "holds": holds })).await;
    assert_eq!(status, StatusCode::OK, "{ran}");
    assert!(ran.to_string().contains("`helper` cannot change its own slot"), "{ran}");
    assert_eq!(settings(&agent(&rig, "helper").await), before, "nothing of it changed");
}

#[tokio::test]
async fn an_agent_added_by_hand_is_taken_away_with_its_text() {
    let rig = shop().await;
    rig.send("/api/agents", json!({ "name": "helper", "project": "shop" })).await;
    let id = agent(&rig, "helper").await["instance"].as_str().unwrap().to_string();
    let looked = rig.read("/api/agents/helper/remove").await;
    assert_eq!(looked["refused"], Value::Null, "nothing counts it: {looked}");
    let removed = rig.send("/api/agents/helper/remove", json!({})).await;
    assert_eq!(removed["removed"], true, "{removed}");
    let text = rig.read(&format!("/api/instances/{id}")).await;
    assert!(text.to_string().contains("\"stopped\""), "{text}");
}

#[tokio::test]
async fn an_agent_from_before_is_given_its_text_once_over_its_slot_as_it_stands() {
    let mut rig = shop().await;
    let said =
        json!({ "name": "helper", "project": "shop", "instructions": "Be brief.", "scopes": ["board"] });
    rig.send("/api/agents", said).await;
    let helper = settings(&agent(&rig, "helper").await);
    rig.stop().await;
    // Three agents as a program from before left them, each noted by the
    // migration: one in, one stopped, and one whose text is there already.
    let database = rusqlite::Connection::open(rig.dir.join("data/journal.db")).unwrap();
    database
        .execute_batch(
            "CREATE TEMP TABLE one AS SELECT * FROM slots WHERE name = 'helper';
             UPDATE one SET name = 'older';
             INSERT INTO slots SELECT * FROM one;
             UPDATE one SET name = 'resting', active = 0;
             INSERT INTO slots SELECT * FROM one;
             UPDATE slots SET owes = 'text';",
        )
        .unwrap();
    drop(database);
    rig.start().await;
    // Applied as the daemon came up; its step, which holds the agent, is
    // waited for.
    let texts = rig.read("/api/instances?blueprint=std/agent").await;
    let listed = texts["instances"].as_array().unwrap_or_else(|| panic!("{texts}"));
    let of_older = listed.iter().find(|one| one["key"] == "older").unwrap_or_else(|| panic!("{texts}"));
    let id = of_older["id"].as_str().unwrap().to_string();
    rig.event("run.finished", move |event| event.source.instance.as_deref() == Some(id.as_str())).await;

    let older = agent(&rig, "older").await;
    let text = text_of(&rig, "older").await;
    let instance = if text["instance"].is_object() { &text["instance"] } else { &text };
    assert_eq!((&instance["blueprint"], &instance["key"]), (&json!("std/agent"), &json!("older")), "{text}");
    let mut was = helper.clone();
    was.insert("name".into(), json!("older"));
    let mut now = settings(&older);
    // A slot from before that never had a credential is given one now, as any agent is.
    now.retain(|name, _| name != "created");
    was.retain(|name, _| name != "created");
    assert_eq!(now, was, "nothing of it changed");
    assert_eq!(agent(&rig, "resting").await["instance"], Value::Null, "a stopped one is left without");
    assert_eq!(settings(&agent(&rig, "helper").await), helper);
    let texts = rig.read("/api/instances?blueprint=std/agent").await;
    let of_helper = texts.to_string().matches("\"key\":\"helper\"").count();
    assert_eq!(of_helper, 1, "the one with a text is given no second: {texts}");

    // Taken up once: the note is gone.
    let owed = |rig: &Rig| {
        let database = rusqlite::Connection::open(rig.dir.join("data/journal.db")).unwrap();
        database
            .query_row("SELECT count(*) FROM slots WHERE owes != ''", [], |row| row.get::<_, i64>(0))
            .unwrap()
    };
    assert_eq!(owed(&rig), 0);

    // A daemon stopped after the text was applied and before the note
    // was taken off: the next start finds the text and only takes the
    // note off.
    let given = settings(&agent(&rig, "older").await);
    let id = agent(&rig, "older").await["instance"].clone();
    rig.stop().await;
    let database = rusqlite::Connection::open(rig.dir.join("data/journal.db")).unwrap();
    database.execute("UPDATE slots SET owes = 'text' WHERE name = 'older'", []).unwrap();
    drop(database);
    rig.start().await;
    assert_eq!(owed(&rig), 0);
    assert_eq!(agent(&rig, "older").await["instance"], id, "no second text");
    let mut now = settings(&agent(&rig, "older").await);
    let mut given = given;
    now.remove("created");
    given.remove("created");
    assert_eq!(now, given);
}
