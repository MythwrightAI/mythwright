//! The store program, and the blueprint that makes a database and hosts it.
//!
//! Both are run for real: the program as a process of its own, and the
//! blueprint by a daemon that is the real binary, since what the blueprint
//! hosts is that binary.
//!
//! Unix's alone: the store and the daemon here are the real program as
//! processes of their own (`daemon::Daemon`), stopped by a signal, and a
//! file's mode is read.
#![cfg(unix)]

use std::path::Path;
use std::process::Stdio;

use reqwest::StatusCode;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;

use crate::daemon::{Daemon, alive, signal};
use crate::{BINARY, run_in, scratch, soon};

/// The store program, serving a file.
struct Program {
    child: Child,
    url: String,
}

impl Program {
    /// Starts it on a port of its choosing and reads where it says it is.
    async fn serve(file: &Path) -> Program {
        let mut child = tokio::process::Command::new(BINARY)
            .args(["store", "serve", "--db", file.to_str().unwrap(), "--listen", "127.0.0.1:0"])
            .env_clear()
            .envs(crate::kept_on_windows())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let said = soon("the store to say where it is", lines.next_line()).await.unwrap().unwrap();
        let (kept, url) = said.split_once(" at ").unwrap_or_else(|| panic!("it said: {said}"));
        assert_eq!(kept, format!("keeping the records in {}", file.display()));
        Program { child, url: url.to_string() }
    }

    /// Asks it to stop, as whatever hosts it does, and waits for it to.
    async fn stop(mut self) {
        signal(self.child.id().unwrap(), "-TERM");
        assert!(soon("the store to exit", self.child.wait()).await.unwrap().success());
    }
}

async fn ask(method: reqwest::Method, url: &str, body: Option<&str>) -> (StatusCode, Value) {
    let mut request = reqwest::Client::new().request(method, url);
    if let Some(body) = body {
        request = request.body(body.to_string());
    }
    let answer = soon(url, request.send()).await.unwrap();
    let status = answer.status();
    (status, answer.json().await.unwrap_or(Value::Null))
}

async fn listed(store: &str, project: &str) -> Vec<Value> {
    let (status, answer) =
        ask(reqwest::Method::GET, &format!("{store}/records?project={project}&collection=board"), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    answer["records"].as_array().unwrap().clone()
}

fn card(number: i64, title: &str) -> Value {
    json!({
        "id": format!("shop-{number}"), "project": "shop", "collection": "board", "number": number,
        "data": { "title": title, "status": "backlog", "labels": ["a", "b"], "assignee": null },
        "position": number as f64 + 0.5, "created_by": "user:ada", "created": 1_000, "updated": 2_000,
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn the_store_program_keeps_what_it_was_given_across_its_own_restart() {
    let dir = scratch("store");
    let file = dir.join("kept").join("cards.db");
    let store = Program::serve(&file).await;
    assert!(file.is_file(), "the database is made where it was said to be, folder and all");
    let url = store.url.clone();
    assert_eq!(ask(reqwest::Method::GET, &format!("{url}/health"), None).await.0, StatusCode::OK);
    assert_eq!(listed(&url, "shop").await, Vec::<Value>::new());

    // Put, put again changed, and removed.
    let put = |record: Value| {
        let url = format!("{url}/records/{}", record["id"].as_str().unwrap());
        async move { ask(reqwest::Method::PUT, &url, Some(&record.to_string())).await }
    };
    for record in [card(2, "Pack boxes"), card(1, "Sell mugs"), card(3, "Print flyers")] {
        assert_eq!(put(record).await.0, StatusCode::OK);
    }
    let mut changed = card(2, "Pack the boxes");
    changed["data"]["status"] = json!("doing");
    assert_eq!(put(changed.clone()).await.0, StatusCode::OK);
    let remove = |id: &str| {
        let url = format!("{url}/records/{id}");
        async move { ask(reqwest::Method::DELETE, &url, None).await }
    };
    assert_eq!(remove("shop-3").await.0, StatusCode::OK);
    assert_eq!(remove("shop-3").await.0, StatusCode::OK, "removing what is not there is not an error");
    // A record is given back exactly as it was handed over, and only to the table it is of.
    assert_eq!(listed(&url, "shop").await, [card(1, "Sell mugs"), changed.clone()]);
    assert_eq!(listed(&url, "blog").await, Vec::<Value>::new());

    // What it will not do, it says in a sentence.
    let said = |(status, answer): (StatusCode, Value)| {
        (status, answer["error"].as_str().unwrap_or_else(|| panic!("no sentence: {answer}")).to_string())
    };
    let (status, why) =
        said(ask(reqwest::Method::PUT, &format!("{url}/records/shop-9"), Some("not a record")).await);
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(why.starts_with("what was sent for `shop-9` is not a record"), "{why}");
    let (status, why) = said(
        ask(reqwest::Method::PUT, &format!("{url}/records/shop-9"), Some(&card(1, "x").to_string())).await,
    );
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(why.contains("says its id is `shop-1`"), "{why}");
    let (status, why) = said(ask(reqwest::Method::GET, &format!("{url}/records?project=shop"), None).await);
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(why.starts_with("say which records: `project` and `collection`"), "{why}");
    let (status, _) = said(ask(reqwest::Method::GET, &format!("{url}/tables"), None).await);
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = said(ask(reqwest::Method::POST, &format!("{url}/records"), Some("{}")).await);
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);

    // Stopped and started again, it has what it had.
    store.stop().await;
    let again = Program::serve(&file).await;
    assert_eq!(listed(&again.url, "shop").await, [card(1, "Sell mugs"), changed]);
    again.stop().await;
    // And making a database that is there leaves it as it is.
    let made =
        run_in(&dir, "http://127.0.0.1:1", &[], &["store", "init", "--db", file.to_str().unwrap()]).await;
    assert_eq!(made.ok().trim(), format!("{} is ready, with the 2 record(s) it had", file.display()));
    let unsaid = run_in(&dir, "http://127.0.0.1:1", &[], &["store", "serve"]).await;
    assert_eq!(unsaid.code, 2);
    assert!(unsaid.err.starts_with("error: which database file?"), "{}", unsaid.err);
    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread")]
async fn the_store_program_asked_to_stop_as_it_says_where_it_is_stops_as_one_that_was_asked() {
    use tokio::io::AsyncReadExt;
    let dir = scratch("store-early");
    let file = dir.join("cards.db");
    // Where it is cannot be read from what it says, which is held: it is
    // told a port that is this test's until the test is over (`ports`).
    let own = crate::ports::free();
    let port = own.number();
    // What it says goes to a pipe that is full: it is held where it says
    // where it is, serving by then and with nothing said yet.
    let (full, mut said) = tokio::net::unix::pipe::pipe().unwrap();
    full.writable().await.unwrap();
    let mut filled = 0;
    while full.try_write(b".").is_ok() {
        filled += 1;
    }
    assert!(filled >= 4096, "the pipe took {filled} and is full");
    let mut child = tokio::process::Command::new(BINARY)
        .args(["store", "serve", "--db", file.to_str().unwrap(), "--listen", &format!("127.0.0.1:{port}")])
        .env_clear()
        .envs(crate::kept_on_windows())
        .stdin(Stdio::null())
        .stdout(full.into_blocking_fd().unwrap())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let health = format!("http://127.0.0.1:{port}/health");
    let serving = async {
        while reqwest::get(&health).await.is_err() {
            tokio::task::yield_now().await;
        }
    };
    soon("the store to be serving", serving).await;

    // Whatever hosts it asks it to stop as soon as it answers.
    signal(child.id().unwrap(), "-TERM");
    let mut all = Vec::new();
    soon("what it said", said.read_to_end(&mut all)).await.unwrap();
    assert!(String::from_utf8_lossy(&all).contains("keeping the records in"), "it said where it was");
    let ended = soon("the store to exit", child.wait()).await.unwrap();
    assert!(ended.success(), "it stopped as one that was asked, and was not cut off: {ended:?}");
    let _ = std::fs::remove_dir_all(dir);
}

/// What a daemon in a test is hosting. A test that fails part-way must not
/// leave it running: the daemon leaves services alone when it goes, on
/// purpose, so they are stopped here, each by the process it is.
#[derive(Default)]
struct Hosted(Vec<u32>);

impl Drop for Hosted {
    fn drop(&mut self) {
        for pid in &self.0 {
            // A service leads its own process group.
            let _ = std::process::Command::new("kill")
                .args(["-KILL", "--", &format!("-{pid}")])
                .stderr(Stdio::null())
                .status();
        }
    }
}

/// A command's answer exactly as the daemon gave it, whether or not what
/// was asked for went well: a run that failed is an answer too, and is
/// looked at after noting what it left running.
async fn answer(daemon: &Daemon, args: &[&str]) -> Value {
    let mut args = args.to_vec();
    args.push("--json");
    let ran = daemon.run(&args).await;
    serde_json::from_str(&ran.out).unwrap_or_else(|_| panic!("not JSON: {}\n{}", ran.out, ran.err))
}

/// The services the daemon is hosting, each by name with its status,
/// noting their processes in `hosted`.
async fn services(daemon: &Daemon, hosted: &mut Hosted) -> Vec<(String, String)> {
    let fleet = answer(daemon, &["services"]).await;
    let services = fleet["services"].as_array().unwrap();
    for service in services.iter().filter(|service| service["status"] != "stopped") {
        let pid = service["pid"].as_u64().unwrap() as u32;
        if !hosted.0.contains(&pid) {
            hosted.0.push(pid);
        }
    }
    services
        .iter()
        .map(|one| (one["name"].as_str().unwrap().to_string(), one["status"].as_str().unwrap().to_string()))
        .filter(|(_, status)| status != "stopped")
        .collect()
}

/// A daemon with an owner signed in and a project called `shop`.
async fn daemon_with_a_project() -> Daemon {
    let daemon = Daemon::start("serve").await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    daemon.run(&["project", "create", "--bare", "Shop"]).await.ok();
    daemon
}

/// Stops the daemon, having seen that it hosts nothing any more.
async fn finish(mut daemon: Daemon, mut hosted: Hosted) {
    assert_eq!(services(&daemon, &mut hosted).await, [], "what the test started was stopped");
    assert!(hosted.0.iter().all(|pid| !alive(*pid)), "and its processes are gone");
    hosted.0.clear();
    signal(daemon.child.id().unwrap(), "-TERM");
    assert!(soon("the daemon to exit", daemon.child.wait()).await.unwrap().success());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_store_blueprint_makes_a_database_hosts_it_and_gives_out_where_it_answers() {
    let daemon = daemon_with_a_project().await;
    let mut hosted = Hosted::default();
    let file = daemon.dir.join("data/projects/shop/.data/store.db");

    let applied = answer(&daemon, &["apply", "std/store", "--project", "shop"]).await;
    let running = services(&daemon, &mut hosted).await;
    assert_eq!(applied["finished"]["status"], "ok", "{}", applied["finished"]);
    let outputs = &applied["finished"]["outputs"];
    assert_eq!((&outputs["file"], &outputs["service"]), (&json!(".data/store.db"), &json!("shop-store-g1")));
    assert!(file.is_file(), "the database was made, in the project");
    assert_eq!(running, [("shop-store-g1".to_string(), "healthy".to_string())]);
    let url = outputs["url"].as_str().unwrap().to_string();
    assert_eq!(url, format!("http://127.0.0.1:{}", outputs["port"]));
    assert_eq!(ask(reqwest::Method::GET, &format!("{url}/health"), None).await.0, StatusCode::OK);

    // A board is kept there by being given the address.
    let database = format!("database={url}");
    let board = ["apply", "std/board", "--project", "shop", "--param", "store=service", "--param", &database];
    assert_eq!(answer(&daemon, &board).await["finished"]["status"], "ok");
    daemon.run(&["board", "add", "shop", "Sell", "mugs"]).await.ok();
    let kept = listed(&url, "shop").await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!((&kept[0]["id"], &kept[0]["data"]["title"]), (&json!("shop-1"), &json!("Sell mugs")));

    // Applied again with something changed, the database is found and a
    // new store takes the place of the old one, at the address it had.
    let changed = ["apply", "std/store", "--project", "shop", "--param", "ready_timeout=30s"];
    let again = answer(&daemon, &changed).await;
    let running = services(&daemon, &mut hosted).await;
    assert_eq!(again["finished"]["status"], "ok", "{}", again["finished"]);
    assert_eq!(again["finished"]["outputs"]["url"], url.as_str());
    assert_eq!(running, [("shop-store-g2".to_string(), "healthy".to_string())]);
    assert!(daemon.run(&["board", "shop"]).await.ok().contains("Sell mugs"));

    // Stopped, the store answers no more, the board says what to do about
    // it, and the database is still there.
    daemon.run(&["stop", applied["instance"].as_str().unwrap()]).await.ok();
    let silent = daemon.run(&["board", "shop"]).await;
    assert_eq!(silent.code, 1);
    assert!(
        silent.err.contains(&format!("is kept by a store service at {url}"))
            && silent.err.contains("Start the blueprint that hosts the store"),
        "{}",
        silent.err
    );
    assert!(file.is_file());
    finish(daemon, hosted).await;
}

/// The blueprints written out in full on the page about stores, as they
/// are written there.
fn the_documented_blueprints() -> Vec<String> {
    let page = include_str!("../../../../docs/stores.md");
    page.split("```yaml\n")
        .skip(1)
        .map(|after| after.split_once("```").expect("a block ends").0.to_string())
        .filter(|block| block.starts_with("blueprint: "))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_projects_blueprint_makes_a_store_and_a_board_kept_in_it_as_the_documentation_says() {
    let daemon = daemon_with_a_project().await;
    let mut hosted = Hosted::default();
    let documented = the_documented_blueprints();
    let source = &documented[0];
    assert!(source.starts_with("blueprint: tracker\n") && source.contains("call: std/store"), "{source}");
    let blueprint = daemon.file("tracker.bp", source);
    // The one for another database is not run, there being no such
    // database here, but it is a blueprint this daemon would take.
    let other = daemon.file("pg-store.bp", &documented[1]);
    assert_eq!(daemon.run(&["check", &blueprint, &other]).await.ok().matches(": ok").count(), 2);

    let applied = answer(&daemon, &["apply", &blueprint, "--project", "shop"]).await;
    let running = services(&daemon, &mut hosted).await;
    assert_eq!(applied["finished"]["status"], "ok", "{}", applied["finished"]);
    assert_eq!(running, [("shop-cards-g1".to_string(), "healthy".to_string())]);
    let url = applied["finished"]["outputs"]["database"].as_str().unwrap().to_string();
    assert!(daemon.dir.join("data/projects/shop/.data/cards.db").is_file());

    daemon.run(&["board", "add", "shop", "Sell", "mugs"]).await.ok();
    daemon.run(&["board", "move", "shop-1", "doing"]).await.ok();
    let kept = listed(&url, "shop").await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0]["data"]["status"], "doing");

    // Applied again as it is, nothing is done and nothing is lost.
    let same = answer(&daemon, &["apply", &blueprint, "--project", "shop"]).await;
    assert_eq!(same["changed"], false, "{same}");
    let board = daemon.run(&["board", "shop"]).await.ok();
    assert!(board.contains("Sell mugs"), "{board}");

    // Stopping it stops what it made: the store and the service behind it.
    daemon.run(&["stop", applied["instance"].as_str().unwrap()]).await.ok();
    finish(daemon, hosted).await;
}
