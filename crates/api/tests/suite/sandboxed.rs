//! A sandbox test never touches the factory in use.
//!
//! The factory here is one somebody uses: a project with a board and
//! cards, a channel with something said in it, a fleet with agents up, a
//! secret that is set and a service that is running. Each test runs
//! blueprints in sandboxes beside it and then holds the factory to
//! being as it was: every row of every table of every one of its
//! databases, everything it said, every file in its folder.
//!
//! What a sandbox is made of, and that it is handed nothing of this
//! factory, is the crate's own to hold (`sandbox`, whose suite reads
//! its source). What is held here is what that comes to.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::FutureExt;
use reqwest::Method;
use serde_json::{Value, json};

use crate::{Rig, STANDIN, soon};

/// What the daemon writes by itself while it is at rest and asked
/// things, which is not held: each with why.
const ITS_OWN: &[(&str, &str)] = &[
    (
        "metrics",
        "what each request took is measured, and the requests that make and read a sandbox are requests: \
         written when the measurements are next due",
    ),
    ("metric_names", "the names of those measurements"),
];

/// A factory in use: a project `shop` with a board and two cards, a
/// channel with something said in it, the fleet that ships with its
/// agents up, a secret, and a service running. What it is known by.
struct Used {
    rig: Rig,
    card: String,
    channel: String,
    fleet: String,
    service: String,
    secret: &'static str,
}

/// The service is stopped when the test is over, however it ended.
impl Drop for Used {
    fn drop(&mut self) {
        for service in self.rig.node.as_ref().map(|node| node.app.hosting.services()).unwrap_or_default() {
            let _ = hosting::group::stop(service.pid, true);
        }
    }
}

async fn used() -> Used {
    // A clock that stands still: what the factory does by itself on a
    // timer (its device sampled and said, measurements written out) is
    // not what a sandbox did, and comes whenever the machine lets the
    // timer run out. On the wall clock, the device was sampled and said
    // in the middle of what the first test listens over, on a loaded
    // machine where that took longer than the sampler's thirty seconds.
    let still = Arc::new(kernel::ManualClock::new());
    let rig = Rig::owned(Rig::with("api-sandboxed", move |config, _| config.clock = Some(still)).await).await;
    // The node samples its device once as it starts, and then not again
    // while the clock stands: that one is said before anything listens
    // here. It is measured in the same turn as it is said, so a
    // measurement there is a sample said.
    soon("the device's first sample", async {
        let sampled = journal::MetricQuery { name: "device.free_mem_mb".into(), ..Default::default() };
        while rig.node().app.journal.query_metrics(sampled.clone()).await.unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let card = rig.send("/api/board/shop/items", json!({ "title": "Keep me as I am" })).await;
    rig.send("/api/board/shop/items", json!({ "title": "And me" })).await;
    let channel = rig.send("/api/channels", json!({ "name": "general" })).await;
    let channel = channel["id"].as_str().unwrap().to_string();
    rig.send(&format!("/api/channels/{channel}/messages"), json!({ "body": "said before any sandbox" }))
        .await;
    let secret = "sk_live_never_in_a_sandbox_8271";
    let set = rig
        .call(Method::PUT, "/api/secrets/stripe", Some(&rig.token), Some(json!({ "value": secret })))
        .await;
    assert_eq!(set.0, 200, "{}", set.1);
    let fleet =
        json!({ "blueprint": "std/fleet", "project": "shop", "params": { "project": "shop" }, "wait": true });
    let fleet = rig.send("/api/apply", fleet).await;
    assert_eq!(fleet["finished"]["status"], "ok", "{fleet}");
    let serves = format!(
        "blueprint: serves\npermissions: [shell, fleet]\nsteps:\n  up: {{ do: service.start, with: {{ name: sleeper, cmd: ['{STANDIN}', idle] }} }}\n"
    );
    let service = rig.send("/api/apply", json!({ "source": serves, "project": "shop", "wait": true })).await;
    assert_eq!(service["finished"]["status"], "ok", "{service}");
    // At rest: what setting it up set off has run its course.
    crate::simulation::rested(&rig).await;
    Used {
        card: card["id"].as_str().unwrap().to_string(),
        channel,
        fleet: fleet["instance"].as_str().unwrap().to_string(),
        service: service["instance"].as_str().unwrap().to_string(),
        secret,
        rig,
    }
}

/// Every row of every table of every database the factory's journals
/// are kept in, each row as text, by `<journal>.<table>`.
pub(crate) async fn rows(rig: &Rig) -> BTreeMap<String, Vec<String>> {
    let mut all = BTreeMap::new();
    for journal in rig.node().app.journal.every() {
        let name = journal.name().to_string();
        let read = journal.db().call(|connection| {
            let mut tables = Vec::new();
            let mut listed =
                connection.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")?;
            let names: Vec<String> = listed.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?;
            for table in names.into_iter().filter(|table| !table.starts_with("sqlite_")) {
                let mut read = connection.prepare(&format!("SELECT * FROM \"{table}\""))?;
                let columns = read.column_count();
                let rows: Vec<String> = read
                    .query_map([], |row| {
                        let cells: Vec<String> =
                            (0..columns).map(|at| format!("{:?}", row.get_ref(at).unwrap())).collect();
                        Ok(cells.join("|"))
                    })?
                    .collect::<Result<_, _>>()?;
                tables.push((table, rows));
            }
            Ok(tables)
        });
        for (table, rows) in read.await.unwrap() {
            all.insert(format!("{name}.{table}"), rows);
        }
    }
    all
}

/// Where two readings of the factory's rows differ, but for what the
/// daemon writes by itself: a line for each table.
pub(crate) fn differing(
    before: &BTreeMap<String, Vec<String>>,
    after: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let its_own = |table: &str| ITS_OWN.iter().any(|(name, _)| table.rsplit('.').next() == Some(name));
    let mut said = Vec::new();
    for (table, rows) in after.iter().filter(|(table, _)| !its_own(table)) {
        match before.get(table) {
            None => said.push(format!("{table} is new, with {} rows", rows.len())),
            Some(were) if were != rows => {
                let new: Vec<&String> = rows.iter().filter(|row| !were.contains(row)).take(3).collect();
                said.push(format!(
                    "{table}: {} rows were {}, and these are new or changed: {new:?}",
                    rows.len(),
                    were.len()
                ));
            }
            Some(_) => {}
        }
    }
    for table in before.keys().filter(|table| !after.contains_key(*table)) {
        said.push(format!("{table} is gone"));
    }
    said
}

/// Every file under `dir`, by where it is: how long it is and when it
/// was last changed.
fn files(dir: &Path) -> BTreeMap<PathBuf, (u64, std::time::SystemTime)> {
    let mut found = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(folder) = pending.pop() {
        for entry in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                pending.push(entry.path());
            }
            let within = entry.path().strip_prefix(dir).unwrap().to_path_buf();
            // A folder is there or is not: when it was changed is when
            // something was last made or taken away in it.
            let stamp = match meta.is_dir() {
                true => (0, std::time::UNIX_EPOCH),
                false => (meta.len(), meta.modified().unwrap()),
            };
            found.insert(within, stamp);
        }
    }
    found
}

/// The files the daemon writes by itself while it is asked things, which
/// are not held by when they were changed: what each request took is
/// measured, and what was measured is written to the factory's own
/// journal when it is next due. What is in that journal is held row by
/// row instead ([`rows`]).
const ITS_OWN_FILES: &[&str] = &["data/journal.db", "data/journal.db-wal", "data/journal.db-shm"];

/// The files that are not the same in two readings of a folder, each by
/// where it is.
fn changed(
    before: &BTreeMap<PathBuf, (u64, std::time::SystemTime)>,
    after: &BTreeMap<PathBuf, (u64, std::time::SystemTime)>,
) -> Vec<PathBuf> {
    let mut all: Vec<&PathBuf> = before.keys().chain(after.keys()).collect();
    all.sort();
    all.dedup();
    let its_own = |file: &PathBuf| ITS_OWN_FILES.iter().any(|own| file == Path::new(own));
    all.into_iter().filter(|file| before.get(*file) != after.get(*file) && !its_own(file)).cloned().collect()
}

/// A text that asks one of every action there is here that is no word
/// of the language, each with what it says it needs, and goes on
/// whatever becomes of each.
async fn asks_everything(rig: &Rig) -> (String, Vec<String>) {
    let offered = rig.read("/api/actions").await;
    let language: Vec<String> = blueprint::intrinsics().into_iter().map(|spec| spec.name).collect();
    let mut text = "blueprint: asks-everything\nsteps:\n".to_string();
    let mut asked = Vec::new();
    for action in offered["actions"].as_array().unwrap() {
        let name = action["name"].as_str().unwrap();
        if language.iter().any(|word| word == name) {
            continue;
        }
        let mut with = serde_json::Map::new();
        for arg in action["args"].as_array().unwrap().iter().filter(|arg| arg["required"] == true) {
            let given = match arg["type"].as_str().unwrap() {
                "string" | "any" => json!("x"),
                "int" | "number" => json!(1),
                "bool" => json!(true),
                "list" => json!([]),
                "object" => json!({}),
                "duration" => json!("1s"),
                other => panic!("a kind of argument this does not know: {other}"),
            };
            with.insert(arg["name"].as_str().unwrap().to_string(), given);
        }
        text.push_str(&format!(
            "  s{}: {{ do: {name}, with: {}, allow_failure: true }}\n",
            asked.len(),
            Value::Object(with)
        ));
        asked.push(name.to_string());
    }
    (text, asked)
}

/// Runs `body` in a sandbox with stand-ins answering at once, and gives
/// what the sandbox is called.
async fn sandbox(rig: &Rig, mut body: Value) -> Value {
    body["answers"] = json!("default");
    rig.send("/api/sandboxes", body).await
}

async fn end(rig: &Rig, id: &str) {
    let ended = rig.call(Method::DELETE, &format!("/api/sandboxes/{id}"), Some(&rig.token), None).await;
    assert_eq!(ended.0, 200, "{}", ended.1);
}

#[tokio::test]
async fn sandbox_tests_of_every_blueprint_that_ships_and_of_every_action_there_is_write_nothing_in_the_factory_s_journals_and_say_nothing_there()
 {
    let used = used().await;
    let rig = &used.rig;
    let app = &rig.node().app;
    let before = rows(rig).await;
    let said_before = rig.mark();
    let mut heard = app.journal.follow_every(app.journal.now(), "");

    // Every blueprint that ships, and one that asks every action there is.
    let listed = rig.read("/api/blueprints").await;
    let mut ran = 0;
    for one in listed["blueprints"].as_array().unwrap() {
        let name = one["name"].as_str().unwrap();
        let (status, made) = rig.post("/api/sandboxes", json!({ "name": name, "answers": "default" })).await;
        // One that needs to be told something is refused as it is anywhere, and leaves none.
        let Some(id) = made["sandbox"].as_str() else {
            assert_eq!(status, 400, "{name}: {made}");
            assert!(made["error"]["message"].as_str().unwrap().contains("needs param"), "{name}: {made}");
            continue;
        };
        rig.read(&format!("/api/sandboxes/{id}")).await;
        end(rig, id).await;
        ran += 1;
    }
    assert!(ran >= 10, "only {ran} of the blueprints that ship were run in a sandbox");
    // The fleet that ships with a board under it and a card to work,
    // which is most of what there is: every table it has, its agents
    // asked, its place made, its channel said in.
    let fleet = sandbox(rig, json!({ "name": "std/fleet", "first": ["std/board"] })).await;
    let id = fleet["sandbox"].as_str().unwrap();
    let card = json!({ "do": { "action": "board.add", "with": { "title": "Work me", "status": "ready" } } });
    let worked = rig.send(&format!("/api/sandboxes/{id}/acts"), card).await;
    assert!(worked["stood_in"].as_u64().unwrap() >= 5, "{worked}");
    end(rig, id).await;
    let (text, asked) = asks_everything(rig).await;
    let everything = sandbox(rig, json!({ "source": text })).await;
    let id = everything["sandbox"].as_str().unwrap();
    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;
    assert_eq!(shown["runs"][0]["steps"].as_array().unwrap().len(), asked.len(), "every action was asked");
    end(rig, id).await;

    // Nothing was said in the factory while they ran, kept or only passed on.
    let came = heard.next().now_or_never();
    assert!(came.is_none(), "the factory in use said something while sandboxes ran: {came:?}");
    assert_eq!(rig.mark(), said_before, "an event was written in the factory in use");
    // And every row of every table of every one of its journals is as it was.
    let after = rows(rig).await;
    // Every one of its journals was read: its own, and its project's
    // where that has one (played with one journal, it has none).
    assert!(before.len() > 30, "{:?}", before.keys());
    let apart = before.keys().any(|table| table.starts_with("project:"));
    assert_eq!(apart, journal::apart() != Some(false), "{:?}", before.keys());
    let differs = differing(&before, &after);
    assert!(
        differs.is_empty(),
        "the factory in use was written to by a sandbox test:\n{}",
        differs.join("\n")
    );
    // What it has running is what it had: its instances, its agents, its service.
    assert_eq!(app.hosting.services().len(), 1);
    assert_eq!(rig.read(&format!("/api/instances/{}", used.fleet)).await["instance"]["status"], "active");
    assert_eq!(rig.read(&format!("/api/instances/{}", used.service)).await["instance"]["status"], "active");
    assert_eq!(app.sandboxes.count(), 0);
}

/// Every process there is whose command line names `marked`.
fn naming(marked: &str) -> Vec<String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let said = std::fs::read(entry.path().join("cmdline")).unwrap_or_default();
        let said = String::from_utf8_lossy(&said).replace('\0', " ");
        if said.contains(marked) {
            found.push(said);
        }
    }
    found
}

#[tokio::test]
async fn a_command_said_any_way_there_is_is_not_run_in_a_sandbox_and_no_program_is_started() {
    let rig = Rig::new().await;
    // Each would write a file of its own if it were run, and each names
    // a folder nothing else does, which is how a program started for it
    // would be found.
    let marked = rig.dir.join("would-have-run");
    std::fs::create_dir_all(&marked).unwrap();
    let file = |name: &str| marked.join(name).display().to_string();
    let line = |name: &str| format!("\\\"{STANDIN}\\\" write \\\"{}\\\" ran", file(name));
    let argv = |name: &str| format!("['{STANDIN}', write, '{}', ran]", file(name));
    let text = format!(
        "blueprint: commands\npermissions: [shell, fleet]\nparams:\n  own: {{ type: string, default: \"{run}\" }}\nsteps:\n  run: {{ do: shell, with: {{ run: \"${{params.own}}\" }} }}\n  cmd: {{ do: shell, with: {{ cmd: {cmd} }} }}\n  cli: {{ do: shell, with: {{ cli: [project, create, never-made] }} }}\n  service:\n    do: service.start\n    with:\n      name: never-started\n      cmd: {service}\n      health: {{ cmd: \"{health}\" }}\n  waited: {{ do: service.wait_healthy, with: {{ service: never-started }} }}\n",
        run = line("run"),
        cmd = argv("cmd"),
        service = argv("service"),
        health = line("health"),
    );
    let made = sandbox(&rig, json!({ "source": text })).await;
    let id = made["sandbox"].as_str().unwrap();
    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;

    // Every step went through, each answered by a stand-in that wrote
    // down what it was asked.
    let steps: Vec<(&str, &str)> = shown["runs"][0]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| (step["step"].as_str().unwrap(), step["status"].as_str().unwrap()))
        .collect();
    assert_eq!(
        steps,
        [("run", "ok"), ("cmd", "ok"), ("cli", "ok"), ("service", "ok"), ("waited", "ok")],
        "{shown}"
    );
    let asked: Vec<&str> =
        shown["asked"].as_array().unwrap().iter().map(|one| one["action"].as_str().unwrap()).collect();
    assert_eq!(asked, ["shell", "shell", "shell", "service.start", "service.wait_healthy"]);
    assert_eq!(shown["asked"][2]["asked"]["cli"], json!(["project", "create", "never-made"]));
    assert_eq!(shown["asked"][0]["answered"]["code"], 0, "a command stood in ended with 0: {shown}");
    // None of them was run: no file was written, no program names the
    // folder, the factory started nothing and has no such service, and
    // this program's own command line made no project.
    let written: Vec<_> =
        std::fs::read_dir(&marked).unwrap().flatten().map(|entry| entry.file_name()).collect();
    assert!(written.is_empty(), "a command was run: it wrote {written:?}");
    let programs = naming(&marked.display().to_string());
    assert!(programs.is_empty(), "a program was started for a sandbox: {programs:?}");
    assert_eq!(rig.node().app.hosting.services().len(), 0);
    let projects = rig.read("/api/projects").await;
    assert_eq!(projects["projects"], json!([]), "{projects}");
    end(&rig, id).await;
    // Whatever a factory in this process starts says how it ended in
    // its folder, and is ended by that: there is nothing to end.
    assert_eq!(hosting::end_under(&rig.dir), Vec::<u32>::new(), "a sandbox left something running");
}

#[tokio::test]
async fn a_sandbox_writes_no_file_but_in_its_own_folder_which_is_gone_when_it_has_ended() {
    let used = used().await;
    let rig = &used.rig;
    let before = files(&rig.dir);
    let rows_before = rows(rig).await;

    // A text that would write a project's files every way there is: by
    // an action that writes one, by a library and by a table kept as
    // files, and by naming a folder that is not in its project at all.
    let outside = rig.dir.join("named-outside");
    let text = format!(
        "blueprint: writes\nsteps:\n  file: {{ do: fs.write, with: {{ path: notes/made.md, content: written }} }}\n  read: {{ do: fs.read, with: {{ path: README.md }} }}\n  site: {{ do: library.export, with: {{ collection: docs }} }}\n  library: {{ call: std/docs, allow_failure: true }}\n  notes:\n    do: collection.ensure\n    with: {{ name: notes, store: files, fields: {{ title: {{ type: text }} }} }}\n    allow_failure: true\n  up:\n    do: collection.ensure\n    with: {{ name: up, store: documents, at: ../../../../named-up, fields: {{ title: {{ type: text }} }} }}\n    allow_failure: true\n  rooted:\n    do: collection.ensure\n    with: {{ name: rooted, store: documents, at: '{outside}', fields: {{ title: {{ type: text }} }} }}\n    allow_failure: true\n  kept: {{ do: collection.ensure, with: {{ name: kept, fields: {{ title: {{ type: text }} }} }} }}\n  row: {{ do: record.add, with: {{ collection: kept, fields: {{ title: Kept in the journal }} }} }}\n  said: {{ do: log, with: {{ message: a line of a step's log }} }}\n",
        outside = outside.display()
    );
    let made = sandbox(rig, json!({ "source": text, "project": "shop" })).await;
    let id = made["sandbox"].as_str().unwrap();
    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;
    let run = shown["runs"].as_array().unwrap().iter().find(|run| run["steps"][0]["step"] == "file").unwrap();
    assert_eq!(run["status"], "ok", "{shown}");
    // What writes a file was stood in, and a table kept as files is
    // refused: a sandbox keeps its tables in its journal.
    let step =
        |name: &str| run["steps"].as_array().unwrap().iter().find(|step| step["step"] == name).unwrap();
    for stood in ["file", "read", "site"] {
        assert_eq!(step(stood)["status"], "ok", "{}", step(stood));
    }
    for refused in ["library", "notes", "up", "rooted"] {
        assert_eq!(step(refused)["status"], "failed", "`{refused}` was let keep files: {}", step(refused));
    }
    assert_eq!(step("row")["status"], "ok", "{}", step("row"));
    assert!(!outside.exists());

    // While it is there, only its own folder differs, and it is in the
    // data folder under `sandboxes`: what says whose its project's
    // folder is, and the lines of its steps.
    let during = files(&rig.dir);
    let own = Path::new("data").join("sandboxes");
    let outside: Vec<PathBuf> =
        changed(&before, &during).into_iter().filter(|file| !file.starts_with(&own)).collect();
    assert!(outside.is_empty(), "a sandbox wrote outside its own folder: {outside:?}");
    let within: Vec<&PathBuf> = during.keys().filter(|file| file.starts_with(own.join(id))).collect();
    let project = own.join(id).join("projects").join("sandbox");
    let of_project: Vec<&&PathBuf> =
        within.iter().filter(|file| file.starts_with(&project) && **file != &project).collect();
    assert_eq!(of_project, [&&project.join("SANDBOX.md")], "a sandbox's project has a file of its own");
    assert!(within.iter().any(|file| file.to_string_lossy().contains("logs")), "{within:?}");
    assert!(!during.keys().any(|file| file.ends_with("notes/made.md")), "a project's file was written");

    end(rig, id).await;

    // Ended, nothing differs at all.
    let left = changed(&before, &files(&rig.dir));
    assert!(left.is_empty(), "a sandbox that ended left something changed: {left:?}");
    assert!(!rig.dir.join(&own).join(id).exists());
    assert!(differing(&rows_before, &rows(rig).await).is_empty());
}

#[tokio::test]
async fn a_secret_s_value_is_nowhere_in_what_a_sandbox_hands_back_for_a_text_that_names_it() {
    let used = used().await;
    let rig = &used.rig;
    let text = format!(
        "blueprint: pays\npermissions: [shell, secrets]\nenv:\n  PAY_KEY: {{ secret: stripe }}\nsteps:\n  pay: {{ do: shell, with: {{ cmd: ['{STANDIN}', var, PAY_KEY] }} }}\n  said: {{ do: log, with: {{ message: \"paid with ${{steps.pay.out.stdout}}\" }} }}\n"
    );

    let made = sandbox(rig, json!({ "source": text, "project": "shop" })).await;
    let id = made["sandbox"].as_str().unwrap();
    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;

    // The step that would have been handed it was stood in, and says
    // which secret by its name.
    assert_eq!(shown["asked"][0]["action"], "shell");
    assert_eq!(shown["asked"][0]["secrets"], json!(["stripe"]), "{shown}");
    // Its value is in nothing the sandbox says, and in no file of its own.
    assert!(!made.to_string().contains(used.secret));
    assert!(!shown.to_string().contains(used.secret), "a secret's value is in what a sandbox hands back");
    let folder = rig.dir.join("data").join("sandboxes").join(id);
    for (file, _) in files(&folder) {
        let held = std::fs::read(folder.join(&file)).unwrap_or_default();
        let found = held.windows(used.secret.len()).any(|part| part == used.secret.as_bytes());
        assert!(!found, "a secret's value is in {}", file.display());
    }
    end(rig, id).await;
}

#[tokio::test]
async fn nothing_of_the_factory_in_use_is_there_to_be_named_in_a_sandbox_and_what_is_named_is_as_it_was() {
    let used = used().await;
    let rig = &used.rig;
    let before = rows(rig).await;
    let ada = rig.node().app.studio.identity.authenticate(&rig.token).await.unwrap().actor();
    let run = rig.read(&format!("/api/instances/{}", used.fleet)).await["runs"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let text = format!(
        "blueprint: reaches\nsteps:\n  halt: {{ do: stop, with: {{ instance: {fleet} }}, allow_failure: true }}\n  card: {{ do: board.move, with: {{ id: {card}, to: done }}, allow_failure: true }}\n  change: {{ do: record.update, with: {{ id: {card}, fields: {{ title: Taken }} }}, allow_failure: true }}\n  read: {{ do: board.get, with: {{ id: {card} }}, allow_failure: true }}\n  rows: {{ do: record.list, with: {{ project: shop, collection: board }}, allow_failure: true }}\n  say: {{ do: channel.post, with: {{ channel: {channel}, body: from a sandbox }}, allow_failure: true }}\n  named: {{ do: channel.post, with: {{ channel: general, body: from a sandbox }}, allow_failure: true }}\n  comment: {{ do: record.comment, with: {{ id: {card}, body: from a sandbox }}, allow_failure: true }}\n  agents: {{ do: rows.read, with: {{ project: shop, collection: agents }}, allow_failure: true }}\n  who: {{ do: actor.holds, with: {{ actor: \"{ada}\", scope: admin }} }}\n  waits: {{ do: wait, with: {{ event: run.finished, for: 1s }}, allow_failure: true }}\n",
        fleet = used.fleet,
        card = used.card,
        channel = used.channel,
    );

    let made = sandbox(rig, json!({ "source": text, "project": "shop" })).await;
    let id = made["sandbox"].as_str().unwrap();
    rig.send(&format!("/api/sandboxes/{id}/acts"), json!({ "advance": { "by": "2s" } })).await;
    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;

    // Each found nothing by that name, and said so as it would anywhere.
    let steps = shown["runs"][0]["steps"].as_array().unwrap();
    let step = |name: &str| steps.iter().find(|step| step["step"] == name).unwrap();
    for (name, says) in [
        ("card", "no"),
        ("change", "no"),
        ("read", "no"),
        ("rows", "no"),
        ("say", "no"),
        ("named", "no"),
        ("comment", "no"),
        ("agents", "no"),
    ] {
        assert_eq!(
            step(name)["status"],
            "failed",
            "`{name}` found something of the factory in use: {}",
            step(name)
        );
        assert!(step(name)["error"].as_str().unwrap().contains(says), "{}", step(name));
    }
    assert_ne!(step("halt")["status"], "ok", "{}", step("halt"));
    // The person who owns the factory is nobody there.
    assert_eq!(step("who")["output"]["known"], false, "{}", step("who"));
    // And nothing it can read names anything of the factory in use: not
    // an id, not what a card or a message says.
    let whole = shown.to_string();
    for live in
        [used.fleet.as_str(), run.as_str(), "Keep me as I am", "said before any sandbox", "shop-worker"]
    {
        let named_by_the_text = text.matches(live).count();
        let found = whole.matches(live).count();
        assert!(
            found <= named_by_the_text * 4,
            "`{live}` of the factory in use is in a sandbox: {found} times"
        );
    }
    assert!(!whole.contains("Keep me as I am") && !whole.contains("said before any sandbox"));
    end(rig, id).await;

    // What was named is as it was, with everything else.
    let differs = differing(&before, &rows(rig).await);
    assert!(differs.is_empty(), "{}", differs.join("\n"));
    assert_eq!(rig.read(&format!("/api/instances/{}", used.fleet)).await["instance"]["status"], "active");
    let board = rig.read("/api/board/shop").await;
    assert!(board.to_string().contains("Keep me as I am") && !board.to_string().contains("Taken"), "{board}");
}

/// The actions a sandbox has as they are and no stand-in for, in the
/// families they come in: each with why nothing of it reaches outside
/// the sandbox. An action that is not here and is in a sandbox as the
/// real one fails the test below: it is added here, with its reason, by
/// whoever has read that it only works something out or only keeps
/// rows; one that starts a program, opens a file of a project, asks an
/// agent or goes to another device is never added, and is a stand-in
/// by being registered where its crate registers what reaches outside.
const AS_THEY_ARE: &[(&str, &[&str], &str)] = &[
    (
        "own",
        &["wait", "emit", "set", "keep", "drop", "measure", "end", "fail", "noop", "log", "stop"],
        "the language's own words, done by the sandbox's own conductor to its own instances",
    ),
    (
        "own",
        &["pool.turn", "text.put", "profile.set", "environment.ensure", "flag.set"],
        "the runner's blocks, over the sandbox's own conductor, whose project's environments are its own",
    ),
    (
        "own",
        &[
            "review.tally",
            "agent.counted",
            "place.unkept",
            "agent.round",
            "agent.news",
            "work.turn",
            "work.due",
            "work.ended",
            "work.held",
            "work.away",
            "work.left",
            "work.owed",
            "work.started",
            "work.worked",
            "work.came",
            "review.owed",
            "review.verdict",
            "review.decided",
            "review.over",
            "column.changed",
            "column.paced",
            "column.wrong",
            "segments.next",
            "segments.ran",
            "triggers.fired",
            "triggers.refused",
            "triggers.timed",
            "segments.wrong",
            "segments.where",
            "segments.recorded",
            "place.tidied",
            "tells.whom",
            "turn.came",
            "discussion.next",
            "discussion.ending",
            "chain.read",
            "chain.next",
            "chain.decided",
            "timed.fields",
            "contest.pending",
            "contest.ways",
            "contest.own",
            "contest.tally",
            "contest.scores",
            "contest.talked",
            "contest.outcome",
            "contest.shown",
            "items.passed",
            "tickets.read",
            "tickets.filed",
            "tickets.named",
            "tickets.fit",
            "tickets.placed",
            "sides.cross",
            "sides.owed",
            "flags.due",
        ],
        "what a fleet's texts ask so as to work nothing out: each a function of what it is handed",
    ),
    (
        "own",
        &["rollout.next", "rollout.watched"],
        "the two blocks of a rollout: functions of what they are handed",
    ),
    (
        "kept",
        &["channel.answer"],
        "a block the studio adds with its tables' actions: a function of what it is handed",
    ),
    (
        "kept",
        &["timed.read"],
        "reads a card's history, which the sandbox's own journal keeps, and asks a block of it",
    ),
    (
        "kept",
        &[
            "collection.ensure",
            "rows.read",
            "rows.start",
            "record.add",
            "record.keep",
            "record.update",
            "record.change",
            "record.comment",
            "record.ask",
            "record.get",
            "record.said",
            "record.list",
            "record.remove",
        ],
        "tables and their rows, in the sandbox's own journal; a table kept as files, as a library's documents \
         or by a store service is refused there",
    ),
    (
        "kept",
        &[
            "board.add",
            "board.move",
            "board.settle",
            "board.update",
            "board.list",
            "board.get",
            "board.came",
            "board.said",
            "board.waits",
            "board.after",
            "board.held",
        ],
        "a board and its cards, which are rows",
    ),
    (
        "kept",
        &["channel.ensure", "channel.post", "channel.join", "channel.leave"],
        "channels and what is said in them, which are rows",
    ),
    ("kept", &["actor.holds"], "what an actor of the sandbox's own accounts holds: a row read"),
    (
        "kept",
        &["board.press", "board.hold"],
        "what is pressed on a card and that it waits for it, kept on the card's row with what is said of it",
    ),
    (
        "kept",
        &["inbox.tell"],
        "a line in an inbox, which is a row of the sandbox's own journal: whom a handle names is read from \
         the sandbox's own accounts, of which there is the one made-up person",
    ),
    (
        "kept",
        &["discussion.offer", "discussion.withdraw", "discussion.turned", "discussion.end"],
        "what a channel offers of discussions, and where each stands: rows of the sandbox's own journal",
    ),
];

#[tokio::test]
async fn every_action_of_a_sandbox_is_the_real_one_for_a_reason_that_is_written_down_or_a_stand_in_that_answers_for_the_one_described()
 {
    let rig = Rig::new().await;
    let live = rig.read("/api/actions").await;
    let (text, asked) = asks_everything(&rig).await;
    let made = sandbox(&rig, json!({ "source": text })).await;
    let id = made["sandbox"].as_str().unwrap();
    let who = rig.node().app.studio.identity.authenticate(&rig.token).await.unwrap();
    let held = rig.node().app.sandboxes.find(id, &who).unwrap();
    let offers = held.sandbox.offers().await.unwrap();
    let offers: BTreeMap<&str, &Value> =
        offers.as_array().unwrap().iter().map(|one| (one["action"]["name"].as_str().unwrap(), one)).collect();

    // Every action there is here is in the sandbox, under the same name
    // and saying the same of itself, so a text that checks here checks
    // there; and the sandbox has none besides.
    let live = live["actions"].as_array().unwrap();
    assert!(live.len() > 100, "{}", live.len());
    for action in live {
        let name = action["name"].as_str().unwrap();
        let there = offers.get(name).unwrap_or_else(|| panic!("`{name}` is not in a sandbox"));
        assert_eq!(&there["action"], action, "`{name}` says something else of itself in a sandbox");
    }
    assert_eq!(offers.len(), live.len(), "a sandbox has an action the device has not");

    // Each is one of the three kinds, and the real ones are exactly
    // those written down above with why.
    let written: BTreeMap<&str, &str> = AS_THEY_ARE
        .iter()
        .flat_map(|(kind, names, _)| names.iter().map(move |name| (*name, *kind)))
        .collect();
    let mut stood = Vec::new();
    for (name, one) in &offers {
        match (one["kind"].as_str().unwrap(), written.get(name)) {
            ("stood", None) => stood.push(*name),
            ("stood", Some(kind)) => {
                panic!("`{name}` is written down as {kind} and is a stand-in in a sandbox")
            }
            (kind @ ("own" | "kept"), Some(said)) => assert_eq!(kind, *said, "`{name}`"),
            (kind @ ("own" | "kept"), None) => panic!(
                "`{name}` is in a sandbox as the real action ({kind}), and nothing says why it reaches nothing \
                 outside one. If it starts a program, reads or writes a file of a project, asks an agent, a \
                 service or another device, or reads a secret, it is registered where its crate registers what \
                 reaches outside (`outside` in the studio's `actions.rs`; any crate above `deciding`), and a \
                 sandbox then has a stand-in for it. If it only works something out, or only keeps rows, add it \
                 to `AS_THEY_ARE` in {} with that reason",
                file!()
            ),
            (other, _) => panic!("`{name}` is of a kind there is not: {other}"),
        }
    }
    for name in written.keys() {
        assert!(
            offers.contains_key(name),
            "`{name}` is written down as real in a sandbox and there is no such action"
        );
    }
    // What starts programs, asks agents, keeps places, reaches files,
    // devices and copies is among the stand-ins, all of it.
    for outside in [
        "shell",
        "service.start",
        "service.stop",
        "service.status",
        "service.wait_healthy",
        "router.ensure",
        "router.route",
        "router.observe",
        "router.remove",
        "system.check",
        "agent.ensure",
        "agent.task",
        "agent.consult",
        "agent.stop",
        "agent.let_go",
        "agent.present",
        "place.make",
        "place.keep",
        "place.send",
        "place.merge",
        "place.clear",
        "place.changed",
        "place.take",
        "place.follows",
        "fs.read",
        "fs.write",
        "project.create",
        "project.scan",
        "library.export",
        "device.list",
        "device.pick",
        "blueprint.apply",
        "blueprint.stop",
        "factory.snapshot",
        "system.answer_as",
    ] {
        assert!(stood.contains(&outside), "`{outside}` is no stand-in in a sandbox");
    }

    // And a stand-in is what answered for each of those: asked, every
    // one of them wrote down what it was asked, and no real one did.
    let shown = rig.read(&format!("/api/sandboxes/{id}")).await;
    let answered: Vec<&str> =
        shown["asked"].as_array().unwrap().iter().map(|one| one["action"].as_str().unwrap()).collect();
    for name in &asked {
        let kind = offers[name.as_str()]["kind"].as_str().unwrap();
        assert_eq!(
            answered.contains(&name.as_str()),
            kind == "stood",
            "`{name}` ({kind}) and what answered for it"
        );
    }
    assert_eq!(shown["stands_in_for"].as_array().unwrap().len(), stood.len());
    end(&rig, id).await;
}
