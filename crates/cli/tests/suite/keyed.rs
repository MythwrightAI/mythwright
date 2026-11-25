//! A branch asked for and sent as a key the factory keeps, for real.
//!
//! A fleet told `key` reaches its repository's remote as that key, a
//! bot's, and not as whoever runs the daemon. What says so is git, which
//! is told to ask this program who the remote is reached as. So these are
//! played by a daemon that is the real binary, with the real git, against
//! a remote that asks who is there: git's own program for serving a
//! repository over http, behind a door that lets through only who offers
//! the key it takes, and writes down everything it was offered.
//!
//! Whoever runs the daemon is signed in to that remote too, as git knows
//! it: the repository has a helper of its own that offers their password.
//! It is what git would offer if it were asked, and it is never offered.
//!
//! Unix's alone: the daemon here is the real program as a process of its
//! own, and git's helper and the agent are files of a Unix shell.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Child;

use crate::{BINARY, Ran, command_in, run_in, scratch, soon};

/// A project's own fleet, written from the fleet that ships: one who works,
/// run by the stand-in, and a person to review. What it may send where is
/// its own to say.
const FLEET: &str = r#"blueprint: fleet
description: Runs the Shop factory.
permissions: [fleet, channels, board, agents, blueprints, shell, records, projects]

params:
  push: { type: bool, default: false }
  fetch: { type: bool, default: false }
  key: { type: string, default: "" }

steps:
  fleet:
    call: std/fleet
    as: fleet
    with:
      project: shop
      channel: shop
      push: ${params.push}
      fetch: ${params.fetch}
      key: "${params.key}"
      agents:
        - { title: worker, role: worker, count: 1, adapter: writer }

      stages:
        - { title: work, column: ready, does: work, role: worker, pass: review }
        - { title: review, column: review, does: review, role: reviewer, pass: done, fail: doing }
"#;
/// The key the factory keeps, as `shop-bot`.
const KEY: &str = "tok_that_is_the_bots_alone";
/// What whoever runs the daemon is signed in to the remote with.
const OWNERS: &str = "the-password-of-whoever-runs-the-daemon";

/// One thing the remote was asked, and who by, as far as was said.
#[derive(Clone, Debug)]
struct Asked {
    what: String,
    who: Option<String>,
}

/// A remote reached over http that asks who is there.
struct Hosted {
    url: String,
    asked: Arc<Mutex<Vec<Asked>>>,
}

impl Hosted {
    /// Serves the repositories under `root` to whoever offers `takes`.
    /// `None` where git's program for that is not on this system.
    async fn serve(root: &Path, takes: &'static str) -> Option<Hosted> {
        Hosted::serve_as(root, takes, false).await
    }

    /// A remote that takes nobody and says back what it was offered, as
    /// one that names the token it refuses does.
    async fn telling(root: &Path) -> Option<Hosted> {
        Hosted::serve_as(root, "", true).await
    }

    async fn serve_as(root: &Path, takes: &'static str, says_back: bool) -> Option<Hosted> {
        let said = asking(Path::new(".")).arg("--exec-path").output().ok()?;
        let backend = PathBuf::from(String::from_utf8_lossy(&said.stdout).trim()).join("git-http-backend");
        if !backend.is_file() {
            return None;
        }
        std::fs::create_dir_all(root).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let asked = Arc::new(Mutex::new(Vec::new()));
        let (root, heard) = (root.to_path_buf(), asked.clone());
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(answer(stream, backend.clone(), root.clone(), takes, says_back, heard.clone()));
            }
        });
        Some(Hosted { url, asked })
    }

    fn asked(&self) -> Vec<Asked> {
        self.asked.lock().unwrap().clone()
    }

    /// Everybody who offered something, each once.
    fn offered(&self) -> Vec<String> {
        let mut who: Vec<String> = self.asked().into_iter().filter_map(|asked| asked.who).collect();
        who.sort();
        who.dedup();
        who
    }
}

/// Answers one request: nothing for who offers no credential or the wrong
/// one, and otherwise what git's own program answers.
async fn answer(
    stream: TcpStream,
    backend: PathBuf,
    root: PathBuf,
    takes: &'static str,
    says_back: bool,
    heard: Arc<Mutex<Vec<Asked>>>,
) {
    let mut stream = BufReader::new(stream);
    let mut line = String::new();
    if stream.read_line(&mut line).await.unwrap_or(0) == 0 {
        return;
    }
    let mut words = line.split_whitespace();
    let (method, target) = (words.next().unwrap().to_string(), words.next().unwrap().to_string());
    let mut headers = BTreeMap::new();
    loop {
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        let Some((name, value)) = line.trim_end().split_once(':') else { break };
        headers.insert(name.to_ascii_lowercase(), value.trim().to_string());
    }
    let length = headers.get("content-length").and_then(|length| length.parse().ok()).unwrap_or(0);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).await.unwrap();

    let who = headers
        .get("authorization")
        .and_then(|said| said.strip_prefix("Basic "))
        .and_then(|coded| STANDARD.decode(coded).ok())
        .map(|plain| String::from_utf8_lossy(&plain).into_owned());
    heard.lock().unwrap().push(Asked { what: format!("{method} {target}"), who: who.clone() });
    // Refused in the remote's own words, with what was offered in them:
    // git prints what a remote says, whatever it is.
    if let (true, Some(offered)) = (says_back, &who) {
        let service =
            if target.contains("git-receive-pack") { "git-receive-pack" } else { "git-upload-pack" };
        let packet = |text: String| format!("{:04x}{text}", text.len() + 4);
        let body = format!(
            "{}0000{}",
            packet(format!("# service={service}\n")),
            packet(format!("ERR {offered} is not let in here\n"))
        );
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-{service}-advertisement\r\n\
             Cache-Control: no-cache\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(reply.as_bytes()).await;
        return;
    }
    let user = who
        .as_deref()
        .and_then(|who| who.split_once(':'))
        .filter(|(_, password)| *password == takes)
        .map(|(user, _)| user.to_string());
    let Some(user) = user else {
        let refusal = "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"shop\"\r\n\
                       Content-Length: 0\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(refusal.as_bytes()).await;
        return;
    };

    let (path, query) = target.split_once('?').unwrap_or((&target, ""));
    let header = |name: &str| headers.get(name).cloned().unwrap_or_default();
    let mut served = tokio::process::Command::new(backend)
        .env_clear()
        .envs(crate::kept_on_windows())
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("GIT_PROJECT_ROOT", root)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("REQUEST_METHOD", &method)
        .env("PATH_INFO", path)
        .env("QUERY_STRING", query)
        .env("CONTENT_TYPE", header("content-type"))
        .env("CONTENT_LENGTH", length.to_string())
        .env("HTTP_CONTENT_ENCODING", header("content-encoding"))
        .env("GIT_PROTOCOL", header("git-protocol"))
        .env("REMOTE_USER", user)
        .env("REMOTE_ADDR", "127.0.0.1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut given = served.stdin.take().unwrap();
    // It may answer before it has read everything; that is not a failure.
    let _ = given.write_all(&body).await;
    drop(given);
    let said = served.wait_with_output().await.unwrap().stdout;
    let end = said.windows(4).position(|window| window == b"\r\n\r\n").unwrap_or(said.len());
    let (head, rest) =
        (String::from_utf8_lossy(&said[..end]).into_owned(), &said[(end + 4).min(said.len())..]);
    let status = head.lines().find_map(|line| line.strip_prefix("Status: ")).unwrap_or("200 OK");
    let kept: Vec<&str> = head.lines().filter(|line| !line.starts_with("Status: ")).collect();
    let reply = format!(
        "HTTP/1.1 {status}\r\n{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        kept.join("\r\n"),
        rest.len()
    );
    let _ = stream.write_all(reply.as_bytes()).await;
    let _ = stream.write_all(rest).await;
}

/// Git as a test asks it, in `dir`: the one place this suite starts it.
/// Who commits is what the repository there says and never what the
/// environment the suite is run from names (`kernel::git::WHO`).
pub(crate) fn asking(dir: &Path) -> std::process::Command {
    let mut git = std::process::Command::new("git");
    git.current_dir(dir);
    for name in kernel::git::WHO {
        git.env_remove(name);
    }
    git
}

/// Runs `git` in `dir` and gives what it printed; a failure is the test's.
fn git(dir: &Path, args: &[&str]) -> String {
    let ran = asking(dir).args(args).output().unwrap();
    assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
    String::from_utf8_lossy(&ran.stdout).trim().to_string()
}

/// A daemon of its own with an owner, and a project `shop` that is a
/// repository with a remote beside it.
struct Factory {
    _child: Child,
    url: String,
    dir: PathBuf,
    /// The project's folder.
    shop: PathBuf,
    /// The remote, as a folder: what `origin` is until a test says otherwise.
    remote: PathBuf,
}

impl Drop for Factory {
    fn drop(&mut self) {
        // The daemon, and what it started and left.
        let _ = self._child.start_kill();
        crate::ended_under(&self.dir);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl Factory {
    async fn start() -> Factory {
        let dir = scratch("keyed");
        let mut child = tokio::process::Command::new(BINARY)
            .args(["serve", "--data", dir.join("data").to_str().unwrap(), "--listen", "127.0.0.1:0"])
            .args(["--adapter", "builtin"])
            .env_clear()
            .envs(crate::kept_on_windows())
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", &dir)
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_DATA_HOME", dir.join("share"))
            // The stand-in for an agent is a script of the test's own; a
            // system that can hold no wall would start none behind one.
            .env(brand::env_var("WALLS"), "off")
            // The daemon is started where an author and a committer are
            // named, as one started from a shell that names its owner is:
            // what it commits is still by whoever the project says.
            .env("GIT_AUTHOR_NAME", "Somebody Else")
            .env("GIT_AUTHOR_EMAIL", "somebody@elsewhere.invalid")
            .env("GIT_COMMITTER_NAME", "Somebody Else")
            .env("GIT_COMMITTER_EMAIL", "somebody@elsewhere.invalid")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            // A test that fails part-way must not leave its daemon running.
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let url = soon("the daemon to say where it is", async {
            loop {
                let line = lines.next_line().await.unwrap().expect("the daemon ended before it was running");
                if line.contains("is running at") {
                    return line.rsplit(' ').next().unwrap().to_string();
                }
            }
        })
        .await;
        let (shop, remote) = (PathBuf::new(), dir.join("hosted").join("remote.git"));
        let mut factory = Factory { _child: child, url, dir, shop, remote };
        factory
            .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
            .await
            .ok();
        factory.writer().await;

        let made: Value =
            serde_json::from_str(&factory.run(&["project", "create", "--bare", "Shop", "--json"]).await.ok())
                .unwrap();
        factory.shop = PathBuf::from(made["path"].as_str().unwrap());
        let (shop, remote) = (&factory.shop, &factory.remote);
        std::fs::create_dir_all(remote.parent().unwrap()).unwrap();
        git(&factory.dir, &["init", "--quiet", "--bare", remote.to_str().unwrap()]);
        git(shop, &["init", "--quiet", "."]);
        git(shop, &["config", "user.name", "Ada"]);
        git(shop, &["config", "user.email", "ada@example.com"]);
        git(shop, &["add", "-A"]);
        git(shop, &["commit", "--quiet", "-m", "first"]);
        git(shop, &["remote", "add", "origin", remote.to_str().unwrap()]);
        let branch = git(shop, &["rev-parse", "--abbrev-ref", "HEAD"]);
        git(shop, &["push", "--quiet", "origin", &branch]);
        // Whoever runs the daemon is signed in to the remote, as git knows it.
        let theirs = format!("!f() {{ echo username=ada; echo password={OWNERS}; }}; f");
        git(shop, &["config", "credential.helper", &theirs]);
        factory.run(&["apply", "std/board", "--project", "shop"]).await.ok();
        factory
    }

    /// A package for a stand-in agent, which adds a line to a file where
    /// it is run and says so.
    async fn writer(&self) {
        let package = self.dir.join("writer");
        std::fs::create_dir_all(&package).unwrap();
        let program = package.join("program");
        let script = "#!/bin/sh\ncat > /dev/null\necho changed >> CHANGES.md\n\
                      printf '{\"kind\":\"end\",\"text\":\"I changed a file.\"}\\n'\n";
        std::fs::write(&program, script).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let manifest = "extension: writer\ntitle: Writer\ndescription: Not a real agent.\nagent:\n  \
                        program: PROGRAM\n  input: ${ turn.prompt }\n  output:\n    \
                        - when: ${ event.kind == \"end\" }\n      reply: ${ event.text }\n";
        std::fs::write(
            package.join("extension.yaml"),
            manifest.replace("PROGRAM", &program.to_string_lossy()),
        )
        .unwrap();
        self.run(&["extension", "add", package.to_str().unwrap()]).await.ok();
    }

    async fn run(&self, args: &[&str]) -> Ran {
        run_in(&self.dir, &self.url, &[], args).await
    }

    /// Keeps `value` as the secret `shop-bot`, from a file that is then gone.
    async fn keeps(&self, value: &str) {
        let file = self.dir.join("key.txt");
        std::fs::write(&file, value).unwrap();
        self.run(&["secret", "set", "shop-bot", "--file", file.to_str().unwrap()]).await.ok();
        std::fs::remove_file(file).unwrap();
    }

    /// Starts the fleet with `params`, adds a card that is ready, and
    /// waits until it has been worked and taken to review: what was then
    /// said in the project's channel.
    async fn works(&self, params: &[&str]) -> String {
        let fleet = self.dir.join("fleet.bp");
        std::fs::write(&fleet, FLEET).unwrap();
        let mut args = vec!["apply", fleet.to_str().unwrap(), "--project", "shop"];
        for param in params {
            args.extend(["--param", param]);
        }
        self.run(&args).await.ok();

        let mut follow = command_in(
            &self.dir,
            &self.url,
            &["events", "--json", "--follow", "--topics", "board.item.moved", "--limit", "200"],
        );
        let mut follow = follow.stdout(Stdio::piped()).kill_on_drop(true).spawn().unwrap();
        let mut lines = BufReader::new(follow.stdout.take().unwrap()).lines();
        self.run(&["board", "add", "shop", "Add a cart", "--status", "ready"]).await.ok();
        soon("the card to reach review", async {
            loop {
                let line = lines.next_line().await.unwrap().expect("the events ended");
                let event: Value = serde_json::from_str(&line).unwrap();
                if event["data"]["id"] == "shop-1" && event["data"]["to"] == "review" {
                    return;
                }
            }
        })
        .await;
        self.run(&["read", "shop", "--limit", "50"]).await.ok()
    }

    /// The branches the remote has.
    fn branches(&self) -> String {
        git(&self.remote, &["branch", "--list", "--format=%(refname:short)"])
    }

    /// Whether any file the factory keeps, or the project has, holds `text`.
    fn anywhere(&self, text: &str) -> Option<PathBuf> {
        let mut folders = vec![self.dir.clone()];
        while let Some(folder) = folders.pop() {
            for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
                let path = entry.path();
                match entry.file_type() {
                    Ok(kind) if kind.is_dir() => folders.push(path),
                    Ok(kind) if kind.is_file() => {
                        let held = std::fs::read(&path).unwrap_or_default();
                        if held.windows(text.len()).any(|window| window == text.as_bytes()) {
                            return Some(path);
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

/// Said where no branch could be sent over http on this system.
fn no_program() {
    println!("git's program for serving a repository over http is not on this system: no branch was sent");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fleet_told_a_key_asks_and_sends_as_the_key_and_never_as_whoever_runs_the_daemon() {
    let factory = Factory::start().await;
    let Some(hosted) = Hosted::serve(&factory.dir.join("hosted"), KEY).await else { return no_program() };
    git(&factory.shop, &["remote", "set-url", "origin", &format!("{}/remote.git", hosted.url)]);
    factory.keeps(KEY).await;

    let told = factory.works(&["fetch=true", "push=true", "key=shop-bot"]).await;
    assert!(
        told.contains("shop-1: its branch lane/shop-1 was sent to origin with the key shop-bot"),
        "{told}"
    );
    // The remote has the work, and took it from the bot.
    assert_eq!(git(&factory.remote, &["log", "--format=%s", "lane/shop-1"]), "shop-1: Add a cart\nfirst");
    let asked = hosted.asked();
    let as_the_key = format!("token:{KEY}");
    for service in ["git-upload-pack", "git-receive-pack"] {
        assert!(
            asked.iter().any(|one| one.what.contains(service) && one.who.as_deref() == Some(&as_the_key)),
            "{service} was asked as the key: {asked:?}"
        );
    }
    // Nobody else was offered: not whoever runs the daemon, whom git
    // would have offered had it been left to ask as it does.
    assert_eq!(hosted.offered(), vec![as_the_key]);

    // The key is in the file it is kept sealed in and nowhere else: no
    // event, no log of a step, nothing the repository holds.
    assert_eq!(factory.anywhere(KEY), None);
    let events = factory.run(&["events", "--json", "--limit", "500"]).await.ok();
    assert!(!events.contains(KEY) && !told.contains(KEY));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_the_remote_will_not_take_sends_nothing_and_nobody_else_is_tried() {
    let factory = Factory::start().await;
    let Some(hosted) = Hosted::serve(&factory.dir.join("hosted"), OWNERS).await else { return no_program() };
    git(&factory.shop, &["remote", "set-url", "origin", &format!("{}/remote.git", hosted.url)]);
    factory.keeps(KEY).await;

    // The remote would take whoever runs the daemon, and not the key.
    let told = factory.works(&["push=true", "key=shop-bot"]).await;
    assert!(told.contains("shop-1: its branch was not sent to origin: "), "{told}");
    assert_eq!(hosted.offered(), vec![format!("token:{KEY}")], "the key was offered, and nobody after it");
    assert!(!factory.branches().contains("lane/"), "nothing reached the remote");
}

#[tokio::test(flavor = "multi_thread")]
async fn what_a_remote_says_back_of_the_key_it_was_offered_is_written_down_by_the_keys_name() {
    let factory = Factory::start().await;
    let Some(hosted) = Hosted::telling(&factory.dir.join("hosted")).await else { return no_program() };
    git(&factory.shop, &["remote", "set-url", "origin", &format!("{}/remote.git", hosted.url)]);
    factory.keeps(KEY).await;

    // The remote names the token it will not take, and git prints what a
    // remote says: so what git printed has the key in it.
    let told = factory.works(&["push=true", "key=shop-bot"]).await;
    assert_eq!(hosted.offered(), vec![format!("token:{KEY}")], "the key was offered, so the remote said it");
    assert!(told.contains("shop-1: its branch was not sent to origin: "), "{told}");
    assert!(told.contains("token:[the secret `shop-bot`] is not let in here"), "{told}");
    // It is in nothing that was said of it or kept.
    assert!(!told.contains(KEY), "{told}");
    assert_eq!(factory.anywhere(KEY), None);
    let events = factory.run(&["events", "--json", "--limit", "500"]).await.ok();
    assert!(!events.contains(KEY), "{events}");
    assert!(!factory.branches().contains("lane/"), "nothing reached the remote");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_the_factory_does_not_keep_sends_nothing_and_says_which_is_missing() {
    let factory = Factory::start().await;
    let Some(hosted) = Hosted::serve(&factory.dir.join("hosted"), OWNERS).await else { return no_program() };
    git(&factory.shop, &["remote", "set-url", "origin", &format!("{}/remote.git", hosted.url)]);

    let told = factory.works(&["push=true", "key=shop-bot"]).await;
    assert!(
        told.contains("its branch was not sent to origin: ")
            && told.contains("there is no secret called `shop-bot`"),
        "{told}"
    );
    // The remote was not so much as asked: not as whoever runs the daemon either.
    assert!(hosted.asked().is_empty(), "{:?}", hosted.asked());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_is_given_only_to_a_remote_reached_over_https_and_the_branch_is_not_sent_as_somebody_else() {
    let factory = Factory::start().await;
    factory.keeps(KEY).await;

    // The remote is a folder beside the project, which asks nobody who they are.
    let told = factory.works(&["push=true", "key=shop-bot"]).await;
    assert!(
        told.contains(
            "origin is not reached over https, and the key shop-bot is only given to a remote that is"
        ),
        "{told}"
    );
    assert!(!factory.branches().contains("lane/"), "nothing reached the remote");
    // With no key said, the same remote is sent to as it always was.
    let other = Factory::start().await;
    let told = other.works(&["push=true"]).await;
    assert!(told.contains("shop-1: its branch lane/shop-1 was sent to origin"), "{told}");
    assert!(!told.contains("with the key"), "{told}");
    // And what was sent is the agent's work, committed by whoever git is
    // set up as in the project: not by whoever the environment this
    // daemon was started in names (`Factory::start`).
    assert_eq!(
        git(&other.remote, &["log", "-1", "--format=%an <%ae> / %cn <%ce>", "lane/shop-1"]),
        "shop-worker-1 <shop-worker-1@agents.invalid> / Ada <ada@example.com>"
    );
}

/// What this program answers git, asked for a credential with `key` handed
/// for `remote`: what it printed, and what it said beside.
async fn answered(asked: &str, key: &str, remote: &str) -> Ran {
    let dir = scratch("credential");
    let mut command = command_in(&dir, "http://127.0.0.1:1", &["credential", "get"]);
    command.env("FACTORY_REMOTE_KEY", key).env("FACTORY_REMOTE", remote);
    let mut child =
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(asked.as_bytes()).await.unwrap();
    let output = soon("the answer", child.wait_with_output()).await.unwrap();
    let _ = std::fs::remove_dir_all(dir);
    Ran {
        code: output.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

#[tokio::test]
async fn git_is_answered_with_the_key_for_the_remote_it_was_handed_for_and_for_no_other() {
    let remote = "https://git.example/acme/shop.git";
    let there = "protocol=https\nhost=git.example\n\n";
    assert_eq!(answered(there, KEY, remote).await.ok(), format!("username=token\npassword={KEY}\n"));
    // Who it is given as is the remote's to say, in its address or as git asks.
    let named = answered(there, KEY, "https://shop-bot@git.example/acme/shop.git").await.ok();
    assert_eq!(named, format!("username=shop-bot\npassword={KEY}\n"));
    let asked = answered("protocol=https\nhost=git.example\nusername=deploy\n\n", KEY, remote).await.ok();
    assert_eq!(asked, format!("username=deploy\npassword={KEY}\n"));

    // On the way to anywhere else it is not given, and git is told to ask
    // nobody else and prompt nobody.
    for (elsewhere, why) in [
        ("protocol=https\nhost=elsewhere.example\n\n", "another host"),
        ("protocol=http\nhost=git.example\n\n", "the same host, in the clear"),
        ("protocol=https\nhost=git.example:8443\n\n", "another port"),
    ] {
        let refused = answered(elsewhere, KEY, remote).await;
        assert_eq!(refused.out, "quit=true\n", "{why}");
        assert!(
            refused.err.contains("It was not given") && !refused.err.contains(KEY),
            "{why}: {}",
            refused.err
        );
    }
    // A remote that is not reached that way has no address a key is for.
    let unsaid = answered(there, KEY, "git@git.example:acme/shop.git").await;
    assert_eq!(unsaid.out, "quit=true\n");
    // Handed no key, it has nothing to say, and git goes on as it would.
    assert_eq!(answered(there, "", remote).await.ok(), "");
}
