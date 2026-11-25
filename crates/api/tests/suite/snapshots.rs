//! A copy of one factory, to start another from: what a snapshot holds,
//! what it leaves out, and where a factory may be started from one.
//!
//! Every test that starts something in a factory stops the instance that
//! owns it before it ends: a daemon leaves what it runs running.

use std::path::{Path, PathBuf};

use api::{Config, Node};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A folder for what the factories of one test have between them: the
/// list they are on, the snapshots, and directories outside any of them.
pub(crate) struct Shared(pub(crate) PathBuf);

impl Shared {
    pub(crate) fn new() -> Shared {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("api-snap-shared-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        Shared(dir)
    }

    pub(crate) fn registry(&self) -> PathBuf {
        self.0.join("environments.json")
    }

    /// Where a snapshot called `name` is written.
    pub(crate) fn snapshot(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// A factory called `name`, with its owner signed in.
    pub(crate) async fn factory(&self, name: &str) -> Rig {
        let (name, registry) = (name.to_string(), self.registry());
        Rig::owned(
            Rig::with(&format!("api-snap-{name}"), move |config, _| {
                config.profile = Some(name);
                config.registry = Some(registry);
            })
            .await,
        )
        .await
    }

    /// A factory called `name`, started from the snapshot at `from`, with
    /// its owner signed in again: the account came with the copy.
    pub(crate) async fn copy(&self, name: &str, from: &Path) -> Rig {
        let (name, registry, from) = (name.to_string(), self.registry(), from.to_path_buf());
        let mut rig = Rig::with(&format!("api-snap-{name}"), move |config, _| {
            config.profile = Some(name);
            config.registry = Some(registry);
            config.from = Some(from);
        })
        .await;
        // It is a factory from here on, and is started again as any is.
        rig.config.from = None;
        rig.token = sign_in(&rig).await;
        rig
    }

    /// What starting a factory called `name` from `from` would be: for one
    /// that is expected to be refused.
    pub(crate) fn config(&self, name: Option<&str>, from: &Path) -> (Config, PathBuf) {
        let dir = self.0.join(format!("new-{}", &kernel::id::secret()[..12]));
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            device: Some("test-device".into()),
            profile: name.map(str::to_string),
            registry: Some(self.registry()),
            from: Some(from.to_path_buf()),
            walls: Some(api::Walls::Open),
            stands_between: true,
            ..Config::new(dir.join("data"))
        };
        (config, dir)
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        // What a factory refused here had started, if anything.
        crate::ended_with(&self.0);
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Signs Ada in to a factory with the password she registered with, and
/// gives back her session there.
pub(crate) async fn sign_in(rig: &Rig) -> String {
    let body = json!({ "email": "ada@example.com", "password": "correct horse" });
    let (status, signed) = rig.call(Method::POST, "/api/auth/login", None, Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{signed}");
    signed["token"].as_str().unwrap().to_string()
}

/// Asks a factory for a snapshot of itself, and gives back how that went.
pub(crate) async fn snapshot(rig: &Rig, to: &Path) -> Value {
    let body = json!({ "blueprint": "std/snapshot", "params": { "to": to }, "wait": true, "force": true });
    rig.send("/api/apply", body).await["finished"].clone()
}

/// A snapshot that was made, as its `snapshot.json` says.
pub(crate) async fn taken(rig: &Rig, to: &Path) -> Value {
    let finished = snapshot(rig, to).await;
    assert_eq!(finished["status"], "ok", "{finished}");
    serde_json::from_str(&std::fs::read_to_string(to.join(api::snapshot::MANIFEST_FILE)).unwrap()).unwrap()
}

/// Why starting a node on `config` was refused.
pub(crate) async fn refused(config: Config) -> String {
    match Node::start(config).await {
        Ok(_) => panic!("the node started"),
        Err(err) => {
            assert_eq!(err.kind, kernel::ErrorKind::Invalid, "asking again would not change it: {err}");
            err.message
        }
    }
}

/// Every file under `dir`, with what is in it. A node that is running
/// puts files away as it goes (what a program's turn was handed, once the
/// turn has ended): one that was there when its folder was listed and is
/// gone when it is read holds nothing, and is left out.
pub(crate) fn files_under(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let gone = |err: &std::io::Error| err.kind() == std::io::ErrorKind::NotFound;
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let listed = match std::fs::read_dir(&dir) {
            Ok(listed) => listed,
            Err(err) if gone(&err) => continue,
            Err(err) => panic!("{}: {err}", dir.display()),
        };
        for entry in listed.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                match std::fs::read(entry.path()) {
                    Ok(held) => found.push((entry.path(), held)),
                    Err(err) if gone(&err) => {}
                    Err(err) => panic!("{}: {err}", entry.path().display()),
                }
            }
        }
    }
    found
}

/// `text` with the command's name where it says `{cli}`.
pub(crate) fn cli(text: &str) -> String {
    text.replace("{cli}", brand::CLI)
}

pub(crate) fn holds(bytes: &[u8], text: &str) -> bool {
    bytes.windows(text.len()).any(|window| window == text.as_bytes())
}

/// What a factory's database says, asked of it directly.
pub(crate) async fn asked(rig: &Rig, sql: &'static str) -> Vec<String> {
    rig.node()
        .app
        .journal
        .db()
        .call(move |database| {
            database
                .prepare(sql)?
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<_>>()
        })
        .await
        .unwrap()
}

pub(crate) async fn counted(rig: &Rig, sql: &'static str) -> i64 {
    rig.node()
        .app
        .journal
        .db()
        .call(move |database| database.query_row(sql, [], |row| row.get(0)))
        .await
        .unwrap()
}

pub(crate) fn bodies(messages: &Value) -> Vec<String> {
    messages["messages"].as_array().unwrap().iter().map(|m| m["body"].as_str().unwrap().to_string()).collect()
}

pub(crate) fn titles(board: &Value) -> Vec<&str> {
    board["items"].as_array().unwrap().iter().map(|item| item["title"].as_str().unwrap()).collect()
}

pub(crate) async fn stop_instance(rig: &Rig, id: &str) {
    rig.send(&format!("/api/instances/{id}/stop"), json!({})).await;
}

/// Something that goes on running until it is stopped: a service, and a
/// run that waits for an event that never comes.
pub(crate) fn keeper() -> String {
    let serves = crate::standin::cmd(&["idle"]);
    format!(
        "blueprint: keeper
permissions: [fleet]
steps:
  serve: {{ do: service.start, with: {{ name: keeper, cmd: {serves} }} }}
  hold: {{ do: wait, needs: [serve], with: {{ event: never.comes }} }}
"
    )
}

#[tokio::test]
async fn a_factory_started_from_a_snapshot_holds_what_the_other_held_with_nobody_signed_in_and_nothing_running()
 {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;

    // What a factory comes to hold: a channel with things said and
    // attached, a project with files, a board with cards,
    let channel = prod.send("/api/channels", json!({ "name": "general" })).await;
    let channel = channel["id"].as_str().unwrap().to_string();
    let attach = format!("{}/api/channels/{channel}/attachments?name=plan.txt", prod.url);
    let attached: Value = prod
        .client
        .post(attach)
        .bearer_auth(&prod.token)
        .body("the plan, as attached")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let said = json!({ "body": "here is the plan", "attachments": [attached["id"]] });
    prod.send(&format!("/api/channels/{channel}/messages"), said).await;
    prod.send(&format!("/api/channels/{channel}/messages"), json!({ "body": "second thoughts" })).await;
    prod.send("/api/projects", json!({ "name": "Web" })).await;
    let (status, _) = prod
        .call(
            Method::PUT,
            "/api/projects/web/file",
            Some(&prod.token),
            Some(json!({ "path": "docs/notes.md", "content": "# Notes\nkept in prod\n" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    prod.board("web").await;
    prod.send("/api/board/web/items", json!({ "title": "Sell mugs" })).await;
    prod.send("/api/board/web/items", json!({ "title": "Pack boxes", "status": "doing" })).await;
    // Something is said on a card and marked public, and is in its history.
    prod.send("/api/records/web-2/comments", json!({ "body": "Ships on Friday.", "public": true })).await;
    prod.event("collection.history", |event| {
        event.data["record"] == "web-2" && event.data["topic"] == "record.commented"
    })
    .await;
    // A picture is attached to it, kept beside the factory's as a channel's is.
    let pictured: Value = prod
        .client
        .post(format!("{}/api/records/web-2/attachments?name=boxes.png", prod.url))
        .bearer_auth(&prod.token)
        .header("content-type", "image/png")
        .body("the boxes, packed")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // an agent, a device, a key for a script, an invitation,
    prod.send("/api/agents", json!({ "name": "builder", "project": "web" })).await;
    prod.send("/api/devices", json!({ "name": "builder-1" })).await;
    let key =
        prod.send("/api/credentials", json!({ "name": "ci" })).await["token"].as_str().unwrap().to_string();
    prod.send("/api/tier", json!({ "tier": "premium" })).await;
    let invite = prod.send("/api/invites", json!({})).await["code"].as_str().unwrap().to_string();
    // and something running.
    let kept = prod.send("/api/apply", json!({ "source": keeper() })).await;
    let keeper = kept["instance"].as_str().unwrap().to_string();
    prod.event("step.waiting", |event| event.source.instance.as_deref() == Some(&keeper)).await;
    let pid = prod.read("/api/fleet").await["services"][0]["pid"].clone();
    assert!(pid.as_i64().is_some_and(|pid| pid > 0), "{pid}");
    prod.node().app.journal.flush_metrics().await.unwrap();
    assert!(counted(&prod, "SELECT count(*) FROM metrics").await > 0);
    // What is kept of a session and a credential, where the copy is taken
    // from; and of the invitation a device was sent, by which it is let in.
    let kept_secrets = [
        asked(&prod, "SELECT token FROM sessions").await,
        asked(&prod, "SELECT token FROM credentials").await,
        asked(&prod, "SELECT code FROM invites").await,
        asked(&prod, "SELECT key FROM kv WHERE namespace = 'gateway.invitations'").await,
    ]
    .concat();
    assert!(
        kept_secrets.len() >= 5,
        "a session, an agent, a key, an invitation and a device's invitation: {kept_secrets:?}"
    );

    let live = prod.read("/api/instances?live=true&limit=1000").await["instances"].as_array().unwrap().len();

    // The owner asks for a snapshot. It says what it is, and what is not in it.
    let to = shared.snapshot("prod-copy");
    let manifest = taken(&prod, &to).await;
    assert_eq!(manifest["what"], "factory-snapshot");
    assert_eq!(manifest["from"], json!({ "profile": "prod", "device": "test-device" }));
    assert!(manifest["made"].as_i64().unwrap() > 0 && manifest["id"].as_str().unwrap().starts_with("snap_"));
    assert_eq!(manifest["holds"]["attachments"], 2, "the message's and the card's");
    assert_eq!(manifest["holds"]["projects"][0]["id"], "web");
    assert!(manifest["schema"]["conductor"].as_i64().unwrap() >= 1, "{}", manifest["schema"]);
    let left = &manifest["left_out"];
    assert_eq!(left["unhosted"], json!({ "services": 1, "routers": 0 }));
    assert_eq!(left["ended"]["instances"], live, "what was live, and not the making of the copy: {left}");
    assert!(live >= 2 && left["ended"]["steps"].as_i64().unwrap() >= 1, "{left}");
    assert_eq!(
        (&left["sessions"], &left["credentials"], &left["invites"]),
        (&json!(1), &json!(2), &json!(1)),
        "an agent's and a key: a device has no credential but the key it proves"
    );
    assert_eq!(left["agents"], json!(["builder"]));
    assert_eq!(
        left["tables"],
        json!([]),
        "a table no crate has spoken for is emptied in a copy: say in its crate's `copy.rs` whether it goes"
    );
    assert!(
        left["never"].as_array().unwrap().iter().any(|never| never.as_str().unwrap().contains("sessions"))
    );
    let said = prod.read("/api/events/recent?topics=system.snapshot").await;
    let said = &said["events"][0]["data"];
    assert_eq!((&said["snapshot"], &said["profile"]), (&manifest["id"], &json!("prod")), "{said}");
    assert!(said["note"].as_str().unwrap().contains(to.to_str().unwrap()), "{said}");
    prod.node().app.journal.flush_metrics().await.unwrap();
    let measured = prod.read("/api/metrics?name=snapshot").await;
    assert_eq!(
        (&measured["series"][0]["total"]["count"], &measured["series"][0]["total"]["errors"]),
        (&json!(1), &json!(0))
    );

    // The folder is a plain one, its owner's alone, with one database
    // file and nothing beside it; and nothing in it, anywhere, is what a
    // session, a credential or an invitation is known by. (Whose a
    // folder is, is a mode where a file has one: Unix.)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&to).unwrap().permissions().mode() & 0o777, 0o700);
    }
    let mut names: Vec<String> = std::fs::read_dir(&to)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    // (The journals of its projects, each a file, are a folder of it,
    // where its projects have journals of their own.)
    let mut holds_these =
        vec!["attachments", "database.db", "extensions", "journals", "projects", "snapshot.json"];
    if journal::apart() == Some(false) {
        holds_these.retain(|name| *name != "journals");
    }
    assert_eq!(names, holds_these);
    let written = files_under(&to);
    assert!(written.iter().any(|(_, bytes)| holds(bytes, "second thoughts")), "what was said is in it");
    for secret in kept_secrets.iter().chain([&prod.token, &key, &invite]) {
        let found: Vec<&PathBuf> =
            written.iter().filter(|(_, bytes)| holds(bytes, secret)).map(|(path, _)| path).collect();
        assert!(found.is_empty(), "`{secret}` is in {found:?}");
    }

    // Nor is any place in this factory's journal, or anything remembered
    // of what it was running: only what is content, and with the events
    // how far their numbers had been given, which is the journal's own
    // and is what keeps the copy from giving one of them again; and that
    // their numbers have holes, where something was passed on with a
    // number and not kept, by which the newest of them are counted.
    let copy = rusqlite::Connection::open(to.join("database.db")).unwrap();
    let state: Vec<String> = copy
        .prepare(
            "SELECT DISTINCT namespace || '.' || key FROM kv
             WHERE namespace NOT IN ('history', 'board.columns') ORDER BY 1",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    // And that what a factory from before vouched for of a folder its projects no longer read was let go,
    // which a factory started from the copy is not to do again.
    let settled =
        format!("{}.{}", conductor::library::folders::NAMESPACE, conductor::library::folders::SETTLED);
    assert_eq!(state, [settled.as_str(), "journal.drawn", "journal.holes"]);
    let (drawn, held): (i64, i64) = copy
        .query_row(
            "SELECT (SELECT CAST(value AS INTEGER) FROM kv WHERE namespace = 'journal' AND key = 'drawn'),
                    (SELECT max(seq) FROM events)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert!(drawn >= held, "the mark that went is {drawn}, and the copy holds an event numbered {held}");
    let mode: String = copy.query_row("PRAGMA journal_mode", [], |row| row.get(0)).unwrap();
    assert_eq!(mode, "delete", "one file, with no log beside it to be left behind");
    drop(copy);

    // A new factory is started from it, under a name of its own.
    let dev = shared.copy("dev2", &to).await;
    assert_ne!(dev.node().addr, prod.node().addr);
    assert_eq!(dev.read("/api/meta").await["profile"], "dev2");
    let arrived = dev.node().arrived.clone().expect("it says what it was started from");
    assert_eq!(
        (arrived.from.profile.as_deref(), arrived.snapshot.as_str()),
        (Some("prod"), manifest["id"].as_str().unwrap())
    );
    let told = arrived.said().join("\n");
    assert!(told.contains("started from a copy of the `prod` factory on `test-device`"), "{told}");
    assert!(
        told.contains(&cli("sign in again with the password you have there (`{cli} --profile dev2 login`)")),
        "{told}"
    );
    assert!(
        told.contains("Nothing is running here") && told.contains("none of its 1 service and "),
        "{told}"
    );
    assert!(told.contains(&cli("`{cli} --profile dev2 apply std/board --project <project>`")), "{told}");
    assert!(told.contains("The agents (`builder`) are here as they were set up"), "{told}");
    assert!(told.contains("under the same user as that factory these agents use the same sign-in"), "{told}");
    let copied = dev.read("/api/events/recent?topics=system.copied").await;
    // What the new factory says is numbered above all the other had drawn
    // when the copy was taken, whatever of that it had given.
    let first = copied["events"][0]["seq"].as_i64().unwrap();
    assert!(first > drawn, "said as {first}, and the copy came with numbers drawn up to {drawn}");
    let copied = &copied["events"].as_array().unwrap()[0]["data"];
    assert_eq!(copied["from"]["profile"], "prod");
    assert_eq!((&copied["made"], &copied["snapshot"]), (&manifest["made"], &manifest["id"]));
    assert_eq!(copied["left_out"]["unhosted"]["services"], 1);
    assert!(copied["note"].as_str().unwrap().starts_with("This factory was started from a copy of"));

    // The same people, with the passwords they have; and nobody signed in:
    // a session, a key or an invitation of the other factory is nothing here.
    assert_eq!(dev.read("/api/me").await["account"]["email"], "ada@example.com");
    for other in [&prod.token, &key] {
        let (status, _) = dev.call(Method::GET, "/api/me", Some(other), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    assert_eq!(dev.read("/api/credentials").await["credentials"], json!([]));
    assert_eq!(counted(&dev, "SELECT count(*) FROM sessions").await, 1, "the one just made here");
    assert_eq!(counted(&dev, "SELECT count(*) FROM invites").await, 0);
    let fleet = dev.read("/api/fleet").await;
    assert_eq!((&fleet["devices"], &fleet["hub"]), (&json!([]), &Value::Null));
    // It is another installation: its secret is its own, made when it first
    // started, so what it proves to a device or a hub is not what the one
    // it was copied from proves. Nobody invited there is waited for here.
    let (ours, theirs) = (fleet["device"]["id"].as_str().unwrap(), prod.read("/api/fleet").await);
    assert_eq!(ours.len(), 24, "{ours}");
    assert_ne!(ours, theirs["device"]["id"].as_str().unwrap());
    assert_eq!(
        theirs["devices"][0]["name"], "builder-1",
        "the one it was copied from still waits for its own"
    );
    assert!(dev.dir.join("data/installation.secret").exists() && !to.join("installation.secret").exists());

    // What was said, and attached; the project and its files, in this
    // factory's own folder; the board and its cards; the agent.
    let messages = dev.read(&format!("/api/channels/{channel}/messages")).await;
    assert_eq!(bodies(&messages), ["here is the plan", "second thoughts"]);
    let fetched = dev
        .client
        .get(format!("{}/api/attachments/{}", dev.url, attached["id"].as_str().unwrap()))
        .bearer_auth(&dev.token)
        .send()
        .await
        .unwrap();
    assert_eq!(fetched.text().await.unwrap(), "the plan, as attached");
    let fetched = dev
        .client
        .get(format!("{}/api/attachments/{}", dev.url, pictured["id"].as_str().unwrap()))
        .bearer_auth(&dev.token)
        .send()
        .await
        .unwrap();
    assert_eq!(fetched.text().await.unwrap(), "the boxes, packed", "what is attached to a card goes too");
    let project = dev.read("/api/projects/web").await;
    assert_eq!(
        std::path::Path::new(project["path"].as_str().unwrap()),
        dev.dir.join("data").join("projects").join("web")
    );
    let notes = dev.read("/api/projects/web/file?path=docs/notes.md").await;
    assert_eq!(notes["content"], "# Notes\nkept in prod\n");
    assert_eq!(
        dev.send("/api/projects/web/scan", json!({})).await["changes"],
        json!([]),
        "as a scan last saw it"
    );
    assert_eq!(titles(&dev.read("/api/board/web").await), ["Sell mugs", "Pack boxes"]);
    let history = dev.read("/api/records/web-2/history").await;
    assert!(history["history"].as_array().is_some_and(|lines| !lines.is_empty()), "{history}");
    // What was said on it is there as it was said, the mark with it: the
    // line was written where it was said, and a copy takes the lines.
    let said = history["history"].as_array().unwrap().iter().find(|line| line["topic"] == "record.commented");
    let said = said.unwrap_or_else(|| panic!("{history}"));
    assert_eq!(
        (said["said"].as_str(), &said["data"]["public"], &said["data"]["body"]),
        (Some("Ada said: Ships on Friday. (public)"), &json!(true), &json!("Ships on Friday.")),
        "{history}"
    );
    assert_eq!(fleet["agents"][0]["name"], "builder");
    assert!(
        dev.read("/api/events/recent?topics=project.created").await["events"][0]["data"]["project"] == "web"
    );

    // Nothing is running, and nothing was started: what was live is here
    // as stopped, in so many words, and the run it was in as cancelled.
    assert_eq!(
        (&fleet["instances"], &fleet["services"], &fleet["routers"]),
        (&json!([]), &json!([]), &json!([]))
    );
    let was = dev.read(&format!("/api/instances/{keeper}")).await;
    assert_eq!(was["instance"]["status"], "stopped");
    assert!(was["instance"]["error"].as_str().unwrap().contains("a copy starts nothing"), "{was}");
    assert_eq!(was["instance"]["holds"], json!([]));
    assert_eq!(was["runs"][0]["status"], "cancelled");
    // (The copy was being made as it was made, and says so of itself.)
    let all = dev.read("/api/instances?limit=1000").await;
    let making =
        all["instances"].as_array().unwrap().iter().find(|i| i["blueprint"] == "std/snapshot").unwrap();
    assert_eq!(making["status"], "stopped");
    assert!(making["error"].as_str().unwrap().contains("it was making the copy"), "{making}");
    let run = dev.read(&format!("/api/runs/{}", was["runs"][0]["id"].as_str().unwrap())).await;
    let hold = run["steps"].as_array().unwrap().iter().find(|step| step["step"] == "hold").unwrap();
    assert_eq!(hold["status"], "cancelled", "{run}");
    assert_eq!(
        counted(&dev, "SELECT count(*) FROM metrics WHERE name IN ('step', 'run', 'snapshot')").await,
        0
    );
    assert_eq!(counted(&dev, "SELECT count(*) FROM places").await, 0);
    // Search is built again, from what is here.
    dev.node().app.studio.search.catch_up().await.unwrap();
    let found = dev.read("/api/search?q=thoughts").await;
    assert!(!found["hits"].as_array().unwrap().is_empty(), "{found}");

    // The board is given its life again by applying its blueprint here.
    dev.board("web").await;
    dev.send("/api/board/web/items", json!({ "title": "Only in dev" })).await;
    assert_eq!(titles(&dev.read("/api/board/web").await).len(), 3);
    for instance in dev.read("/api/instances?live=true&roots=true").await["instances"].as_array().unwrap() {
        stop_instance(&dev, instance["id"].as_str().unwrap()).await;
    }

    // And the factory it was copied from is as it was: signed in, running
    // what it ran, holding what it held and no more.
    assert_eq!(prod.read("/api/me").await["account"]["email"], "ada@example.com");
    let fleet = prod.read("/api/fleet").await;
    assert!(fleet["instances"].as_array().unwrap().iter().any(|instance| instance["id"] == keeper.as_str()));
    let service = &fleet["services"][0];
    assert_eq!(
        (&service["name"], &service["status"], &service["pid"]),
        (&json!("keeper"), &json!("healthy"), &pid)
    );
    assert_eq!(titles(&prod.read("/api/board/web").await), ["Sell mugs", "Pack boxes"]);
    assert_eq!(
        prod.read("/api/projects/web").await["path"],
        prod.dir.join("data").join("projects").join("web").to_str().unwrap()
    );
    assert_eq!(prod.read("/api/credentials").await["credentials"].as_array().unwrap().len(), 2);
    assert_eq!(prod.read("/api/devices").await["devices"][0]["name"], "builder-1", "still invited there");
    assert!(!prod.dir.join("data").join(api::arrival::MARKER).exists());
    for instance in prod.read("/api/instances?live=true&roots=true").await["instances"].as_array().unwrap() {
        stop_instance(&prod, instance["id"].as_str().unwrap()).await;
    }
}

#[tokio::test]
async fn a_copy_is_refused_wherever_it_could_land_on_something_or_be_taken_for_the_factory_it_was_made_from()
{
    let shared = Shared::new();
    let mut prod = shared.factory("prod").await;
    let to = shared.snapshot("prod-copy");
    let manifest = taken(&prod, &to).await;

    // A new factory needs a name of its own,
    let (config, dir) = shared.config(None, &to);
    let said = refused(config).await;
    assert!(said.contains("the new one needs a name of its own"), "{said}");
    assert!(said.contains(&format!("`{} up --profile dev2 --from {}`", brand::CLI, to.display())), "{said}");
    assert!(!dir.exists(), "nothing is made for a factory that is refused");
    // and not the name of the one it is a copy of.
    let (config, dir) = shared.config(Some("prod"), &to);
    let said = refused(config).await;
    assert!(said.contains("is a snapshot of the `prod` factory"), "{said}");
    assert!(said.contains("cannot be called `prod` too") && said.contains("--profile prod-copy"), "{said}");
    assert!(!dir.exists());

    // What it is started from has to be a snapshot.
    let nothing = shared.snapshot("not-one");
    std::fs::create_dir_all(&nothing).unwrap();
    for from in [nothing.clone(), shared.snapshot("not-there"), prod.dir.join("data")] {
        let (config, dir) = shared.config(Some("dev2"), &from);
        let said = refused(config).await;
        assert!(said.contains(&format!("{} is not a snapshot of a factory", from.display())), "{said}");
        assert!(said.contains(&cli("`{cli} apply std/snapshot --param to=<folder>`")), "{said}");
        assert!(!dir.exists());
    }
    std::fs::write(nothing.join("snapshot.json"), "{ \"what\": \"something else\" }").unwrap();
    let said = refused(shared.config(Some("dev2"), &nothing).0).await;
    assert!(said.contains("is not a snapshot of a factory"), "{said}");

    // One made by a version that keeps its tables in a way this one does
    // not know is refused: it could not be told what of it must not travel.
    let newer = shared.snapshot("newer");
    for (name, bytes) in files_under(&to) {
        if let Ok(name) = name.strip_prefix(&to) {
            std::fs::create_dir_all(newer.join(name).parent().unwrap()).unwrap();
            std::fs::write(newer.join(name), bytes).unwrap();
        }
    }
    let mut says = manifest.clone();
    says["schema"]["conductor"] = json!(99);
    std::fs::write(newer.join("snapshot.json"), says.to_string()).unwrap();
    let said = refused(shared.config(Some("dev2"), &newer).0).await;
    assert!(said.contains("was made by a newer version"), "{said}");
    assert!(said.contains("its `conductor` tables are at version 99"), "{said}");
    // And when only its database says so.
    std::fs::write(newer.join("snapshot.json"), manifest.to_string()).unwrap();
    let database = rusqlite::Connection::open(newer.join("database.db")).unwrap();
    database.execute("INSERT INTO schema_versions (namespace, version) VALUES ('teleport', 1)", []).unwrap();
    drop(database);
    let (config, dir) = shared.config(Some("dev2"), &newer);
    let said = refused(config).await;
    assert!(said.contains("it has `teleport` tables, which this one has never heard of"), "{said}");
    assert!(!dir.join("data/journal.db").exists(), "and the folder is still no factory");

    // A copy never lands on a factory that exists: not the one it came
    // from, and not another.
    let other = shared.factory("dev1").await;
    for (name, rig) in [("prod-again", &prod), ("dev1", &other)] {
        let config = Config { from: Some(to.clone()), profile: Some(name.into()), ..rig.config.clone() };
        // (`prod-again` on prod's data is refused for the name before this.)
        let said = refused(config).await;
        assert!(
            said.contains("already holds a factory") || said.contains("is the data of a `prod` factory"),
            "{said}"
        );
    }
    prod.stop().await;
    let said = refused(Config { from: Some(to.clone()), profile: None, ..prod.config.clone() }).await;
    assert!(said.contains("needs a name of its own"), "{said}");
    let mut config = Config { from: Some(to.clone()), ..other.config.clone() };
    config.data_dir = other.dir.join("data");
    let said = refused(config).await;
    assert!(
        said.contains("already holds a factory, and a copy is only ever put where there is none"),
        "{said}"
    );
    assert!(said.contains("leave `--from` out"), "{said}");
    assert_eq!(other.read("/api/me").await["account"]["email"], "ada@example.com", "and it is as it was");

    // Nor in a folder that has anything else in it.
    let (config, dir) = shared.config(Some("dev3"), &to);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/notes.txt"), "somebody's").unwrap();
    let said = refused(config).await;
    assert!(said.contains("is not empty (it has `notes.txt` in it)"), "{said}");
    assert_eq!(std::fs::read_to_string(dir.join("data/notes.txt")).unwrap(), "somebody's");

    // And a snapshot is not a factory's data: nothing is started on one.
    let config = Config { profile: Some("dev4".into()), from: None, ..Config::new(to.clone()) };
    let said = match Node::start(Config { listen: "127.0.0.1:0".parse().unwrap(), ..config }).await {
        Ok(_) => panic!("a factory was started on a snapshot"),
        Err(err) => err.message,
    };
    assert!(said.contains("is a snapshot of a factory, not a factory's data"), "{said}");
    assert!(!to.join("journal.db").exists() && !to.join("lock").exists(), "and it is left as it was");
}

#[tokio::test]
async fn a_snapshot_is_the_owners_to_ask_for_and_is_written_to_a_folder_of_its_own() {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    prod.send("/api/projects", json!({ "name": "Web" })).await;
    let action = prod.read("/api/actions").await;
    let action =
        action["actions"].as_array().unwrap().iter().find(|a| a["name"] == "factory.snapshot").unwrap();
    assert_eq!(action["capability"], "admin", "the strictest there is");
    assert!(
        !kernel::principal::AGENT_SCOPES.contains(&"admin"),
        "which no agent holds unless it is given it"
    );
    assert!(!kernel::principal::MEMBER_SCOPES.contains(&"admin"), "and no member but the owner");

    // A key for a script that was given what a member holds may not.
    let member = json!({ "name": "ci", "scopes": kernel::principal::MEMBER_SCOPES });
    let key = prod.send("/api/credentials", member).await["token"].as_str().unwrap().to_string();
    let to = shared.snapshot("by-a-script");
    let body = json!({ "blueprint": "std/snapshot", "params": { "to": to }, "wait": true });
    let (status, applied) = prod.call(Method::POST, "/api/apply", Some(&key), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["finished"]["status"], "failed");
    assert!(applied["finished"]["error"].as_str().unwrap().contains("may not use `admin`"), "{applied}");
    assert!(!to.exists());
    // Nor may a blueprint that does not say it takes it.
    let quiet = "blueprint: quiet\npermissions: [fleet]\nsteps:\n  copy: { do: factory.snapshot, with: { to: /nowhere } }\n";
    let checked = prod.send("/api/blueprints/check", json!({ "source": quiet })).await;
    assert!(checked["diagnostics"].to_string().contains("not listed under `permissions`"), "{checked}");

    // Where it is written is said by its whole path, is outside the
    // factory, and has nothing in it.
    let data = prod.dir.join("data");
    let full = shared.snapshot("full");
    std::fs::create_dir_all(&full).unwrap();
    std::fs::write(full.join("mine.txt"), "somebody's").unwrap();
    for (to, why) in [
        (PathBuf::from("copies/prod"), "said by its whole path"),
        (data.join("copies"), "is inside this factory's data folder"),
        (data.join("projects/web/copy"), "is inside this factory's data folder"),
        (full.clone(), "already has things in it"),
        (full.join("mine.txt"), "is not a folder"),
    ] {
        let finished = snapshot(&prod, &to).await;
        assert_eq!(finished["status"], "failed", "{}", to.display());
        assert!(finished["error"].as_str().unwrap().contains(why), "{}: {finished}", to.display());
    }
    assert_eq!(std::fs::read_dir(&full).unwrap().count(), 1, "and nothing was put there");
    assert!(!data.join("copies").exists());
    // A project somewhere else is not where a snapshot goes either.
    let site = shared.0.join("site");
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(site.join("README.md"), "# Site\n").unwrap();
    prod.send("/api/projects", json!({ "name": "Site", "path": site })).await;
    let finished = snapshot(&prod, &site.join("copy")).await;
    assert!(finished["error"].as_str().unwrap().contains("is inside the project `site`"), "{finished}");
    // One that failed is measured as that.
    prod.node().app.journal.flush_metrics().await.unwrap();
    let measured = prod.read("/api/metrics?name=snapshot").await;
    assert_eq!(
        measured["series"][0]["total"]["count"], measured["series"][0]["total"]["errors"],
        "{measured}"
    );

    // An empty folder will do, and is made its owner's alone.
    let empty = shared.snapshot("empty");
    std::fs::create_dir_all(&empty).unwrap();
    taken(&prod, &empty).await;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&empty).unwrap().permissions().mode() & 0o777, 0o700);
    }
}

#[tokio::test]
async fn a_factory_started_from_a_snapshot_still_holds_a_table_to_whose_records_it_steers() {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    // A fleet, whose stages steer its agents, and an agent of it that
    // holds more than a member does.
    prod.send("/api/projects", json!({ "name": "Shop" })).await;
    prod.board("shop").await;
    let fleet =
        json!({ "blueprint": "std/fleet", "project": "shop", "params": { "project": "shop" }, "wait": true });
    let applied = prod.send("/api/apply", fleet).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    crate::fleets::change(&prod, "agents", "worker", json!({ "holds": ["board", "admin"] })).await;
    let steers = |tables: &Value| {
        let stages = tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == "stages");
        stages.unwrap()["steers"].clone()
    };
    assert_eq!(steers(&prod.read("/api/collections?project=shop").await), "agents");

    let to = shared.snapshot("prod-copy");
    taken(&prod, &to).await;
    let dev = shared.copy("dev", &to).await;

    // The copy's table says it as the other's did, and is held to it: a
    // key made there that holds less than that agent arranges no stage.
    assert_eq!(steers(&dev.read("/api/collections?project=shop").await), "agents");
    let scopes = ["projects", "channels", "board", "records", "blueprints", "agents", "fleet", "search"];
    let narrow = crate::held::key(&dev, &scopes).await;
    let work = crate::fleets::row(&dev, "stages", "work").await;
    let role = json!({ "fields": { "role": "architect" } });
    let (status, refused) =
        dev.call(Method::PATCH, &format!("/api/records/{work}"), Some(&narrow), Some(role.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let why = crate::held::said(&refused);
    assert!(
        why.contains(
            "a record of `stages` steers the records of `agents`, and is added or changed only by whoever \
             could write each of those as it stands"
        ) && why.contains("whose `holds` says `admin` is changed only by whoever holds `admin`"),
        "{refused}"
    );
    let kept = dev.read(&format!("/api/records/{work}")).await;
    assert_eq!(kept["data"]["role"], "worker", "{kept}");
    // Its owner's it is, there as here.
    let (status, changed) =
        dev.call(Method::PATCH, &format!("/api/records/{work}"), Some(&dev.token), Some(role)).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
}

#[tokio::test]
async fn a_factory_started_from_a_snapshot_has_the_others_workspaces_and_one_that_holds_a_project_in_none_puts_it_in_one()
 {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    prod.send("/api/workspaces", json!({ "name": "Front" })).await;
    prod.send("/api/workspaces", json!({ "name": "Attic" })).await;
    prod.send("/api/projects", json!({ "name": "Shop", "workspace": "front" })).await;
    prod.send("/api/projects", json!({ "name": "Shed" })).await;
    prod.send("/api/channels", json!({ "name": "plans", "workspace": "front" })).await;
    prod.send("/api/channels", json!({ "name": "shop-talk", "project": "shop" })).await;
    let chat = prod.send("/api/channels", json!({ "name": "chat" })).await;
    let before = prod.read("/api/workspaces").await;

    let to = shared.snapshot("prod-copy");
    taken(&prod, &to).await;
    let dev = shared.copy("dev", &to).await;

    // The same workspaces, each with the same in it, one with nothing in
    // it among them; and a channel's project is what it was.
    let after = dev.read("/api/workspaces").await;
    assert_eq!(after, before);
    let front = after["workspaces"].as_array().unwrap().iter().find(|one| one["id"] == "front").unwrap();
    assert_eq!(front["projects"], json!(["shop"]));
    assert_eq!(front["channels"].as_array().unwrap().len(), 1);
    assert_eq!(dev.read("/api/channels/shop-talk").await["project"], "shop");
    assert_eq!(dev.read("/api/channels/plans").await["project"], Value::Null);
    // The owner's workspace is the usual one there as here, and the
    // project nothing was said of is in it.
    assert_eq!(after["usual"], "ada-s-workspace");
    let usual = after["workspaces"].as_array().unwrap().iter().find(|one| one["usual"] == true).unwrap();
    assert_eq!(usual["projects"], json!(["shed"]));

    // A copy that holds a project in no workspace and a channel that
    // belongs to nothing, as one of a factory an earlier program made
    // may: a factory started from it brings that about as any factory
    // does when it starts, and says so once.
    {
        let copy = rusqlite::Connection::open(to.join("database.db")).unwrap();
        assert_eq!(copy.execute("DELETE FROM workspace_projects WHERE project = 'shed'", []).unwrap(), 1);
        let id = chat["id"].as_str().unwrap();
        assert_eq!(copy.execute("DELETE FROM workspace_channels WHERE channel = ?1", [id]).unwrap(), 1);
    }
    let late = shared.copy("late", &to).await;
    assert_eq!(late.read("/api/workspaces").await, before);
    let query = journal::EventQuery { topics: "project.placed".into(), ..Default::default() };
    let by_the_start: Vec<_> = late
        .said(query)
        .await
        .into_iter()
        .filter(|event| event.source.actor.as_deref() == Some("system"))
        .map(|event| (event.data["project"].clone(), event.data["workspace"].clone()))
        .collect();
    assert_eq!(by_the_start, [(json!("shed"), json!("ada-s-workspace"))]);
    let query = journal::EventQuery { topics: "channel.placed".into(), ..Default::default() };
    let by_the_start: Vec<_> = late
        .said(query)
        .await
        .into_iter()
        .filter(|event| event.source.actor.as_deref() == Some("system"))
        .map(|event| (event.data["channel"].clone(), event.data["workspace"].clone()))
        .collect();
    assert_eq!(by_the_start, [(chat["id"].clone(), json!("ada-s-workspace"))]);
}

#[tokio::test]
async fn a_factory_started_from_a_snapshot_has_a_projects_environments_and_the_names_of_their_secrets_and_no_value()
 {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    prod.send("/api/projects", json!({ "name": "Shop" })).await;
    let base = "/api/projects/shop/environments";
    prod.send(base, json!({ "name": "staging" })).await;
    let var = json!({ "value": "https://staging.example", "agents": true });
    let (status, set) =
        prod.call(Method::PUT, &format!("{base}/staging/vars/api"), Some(&prod.token), Some(var)).await;
    assert_eq!(status, StatusCode::OK, "{set}");
    let secret = json!({ "value": "sk_live_left_behind", "agents": "STRIPE_KEY" });
    let path = format!("{base}/staging/secrets/stripe");
    let (status, set) = prod.call(Method::PUT, &path, Some(&prod.token), Some(secret)).await;
    assert_eq!(status, StatusCode::OK, "{set}");

    let to = shared.snapshot("prod-copy");
    taken(&prod, &to).await;
    assert!(
        files_under(&to).iter().all(|(_, bytes)| !holds(bytes, "sk_live_left_behind")),
        "no value of a secret is in a copy"
    );
    let dev = shared.copy("dev", &to).await;
    let copied = dev.read(&format!("{base}/staging")).await;
    assert_eq!(copied["vars"], json!({ "api": { "value": "https://staging.example", "agents": true } }));
    // Its secret is listed, with what an agent is handed it as, and has no
    // value there: a copy carries none, and says how one is set.
    let stripe = &copied["secrets"]["stripe"];
    assert_eq!((&stripe["agents"], &stripe["kept"]), (&json!("STRIPE_KEY"), &json!(false)), "{copied}");
    let missing = dev.node().app.hosting.secrets().unwrap().get_of("shop", "staging", "stripe").unwrap_err();
    assert!(missing.message.contains("a copy of a factory carries none"), "{}", missing.message);
    let secret = json!({ "value": "sk_test_its_own" });
    let (status, set) = dev.call(Method::PUT, &path, Some(&dev.token), Some(secret)).await;
    assert_eq!(status, StatusCode::OK, "{set}");
    assert_eq!(dev.read(&format!("{base}/staging")).await["secrets"]["stripe"]["kept"], true);
}
