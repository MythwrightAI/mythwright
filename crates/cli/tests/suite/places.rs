//! Where a program reads its blueprints, packages and client from: the
//! checkout it was built in while it is run from there, and never from a
//! copy of it anywhere else.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;

use crate::{BINARY, run_in, scratch, soon};

/// The checkout these tests were built in.
fn checkout() -> PathBuf {
    // As the product says a whole path: plainly, on every system.
    kernel::whole::whole(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// A daemon started from `program`, with a home of its own.
pub(crate) struct Started {
    _child: Child,
    /// What it printed as it started, down to which program it is.
    said: Vec<String>,
    url: String,
    home: PathBuf,
}

impl Drop for Started {
    fn drop(&mut self) {
        // The daemon, and what it started and left.
        crate::ended_under(&self.home);
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

impl Started {
    pub(crate) async fn from(program: &Path) -> Started {
        Started::told(program, &["--adapter", "builtin"]).await
    }

    /// The same, told what its agents run on.
    pub(crate) async fn told(program: &Path, agents: &[&str]) -> Started {
        let home = scratch("place");
        let mut child = tokio::process::Command::new(program)
            .args(["serve", "--data", home.join("data").to_str().unwrap()])
            .args(["--listen", "127.0.0.1:0"])
            .args(agents)
            .env_clear()
            .envs(crate::kept_on_windows())
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", &home)
            // Where the command line keeps its session, as `run_in` says it: the
            // owner its first start makes is signed in there.
            .env("XDG_CONFIG_HOME", home.join("config"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut said = Vec::new();
        soon("the daemon to say which program it is", async {
            loop {
                let line = lines.next_line().await.unwrap().expect("the daemon's output ended");
                let last = line.starts_with("  program  ");
                said.push(line);
                if last {
                    return;
                }
            }
        })
        .await;
        let url = said[0].rsplit(' ').next().unwrap().to_string();
        Started { _child: child, said, url, home }
    }

    fn line(&self, start: &str) -> Option<&str> {
        self.said.iter().map(String::as_str).find(|line| line.starts_with(start))
    }

    pub(crate) async fn run(&self, args: &[&str]) -> String {
        run_in(&self.home, &self.url, &[], args).await.ok()
    }

    /// The same, for a command that may be refused: how it ended, with what it said.
    pub(crate) async fn ran(&self, args: &[&str]) -> crate::Ran {
        run_in(&self.home, &self.url, &[], args).await
    }

    async fn get(&self, path: &str) -> String {
        reqwest::get(format!("{}{path}", self.url)).await.unwrap().text().await.unwrap()
    }

    pub(crate) async fn runs_from(&self) -> String {
        let meta: Value = serde_json::from_str(&self.get("/api/meta").await).unwrap();
        meta["runs_from"].as_str().unwrap_or_default().to_string()
    }

    pub(crate) async fn register(&self) {
        self.run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
            .await;
    }
}

/// A folder made for a test, taken away with it.
pub(crate) struct Folder(pub(crate) PathBuf);

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The program put in `dir` under the name it is installed by. It is the
/// same bytes under another name in another folder, which is all a copy
/// is to the program: where it is run from is where that name is.
/// What the program's file is called on this system.
pub(crate) const PROGRAM: &str = if cfg!(windows) { "factory.exe" } else { "factory" };

fn put_program(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let to = dir.join(PROGRAM);
    if std::fs::hard_link(BINARY, &to).is_err() {
        std::fs::copy(BINARY, &to).unwrap();
    }
}

/// A copy of the program in a folder of its own, as an install leaves it.
pub(crate) fn copied(name: &str) -> (Folder, PathBuf) {
    let made = Folder(scratch(name));
    let dir = made.0.join("app");
    put_program(&dir);
    (made, dir)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copy_of_the_program_in_a_folder_of_its_own_reads_nothing_from_the_checkout_it_was_built_in() {
    // What there is to read, or this would say nothing.
    let checkout = checkout();
    assert!(checkout.join("blueprints/std").is_dir() && checkout.join("extensions/reserved").is_file());

    let (_made, app) = copied("alone");
    let copy = Started::from(&app.join(PROGRAM)).await;
    assert_eq!(copy.line("  blueprints  "), None, "{:?}", copy.said);
    let which = copy.line("  program  ").unwrap();
    assert!(which.contains(&format!("a copy on its own in {}", app.display())), "{which}");
    assert!(
        which.ends_with(": it changes only when it is replaced") && !which.contains("install"),
        "{which}"
    );
    assert_eq!(copy.runs_from().await, "installed");

    // No client of the checkout's, built or not: it says what it is and
    // what would give it one.
    let page = copy.get("/").await;
    assert!(page.contains("a copy of the program on its own"), "{page}");

    copy.register().await;
    let packages = copy.run(&["extension", "list"]).await;
    assert!(!packages.contains("came with this installation"), "{packages}");
    assert!(copy.run(&["status"]).await.contains("A copy on its own: it changes only when it is replaced."));

    // The standard blueprints are the ones built into it.
    copy.run(&["project", "create", "--bare", "Shop"]).await;
    let made = copy.run(&["apply", "std/board", "--project", "shop"]).await;
    assert!(made.contains(" ok in "), "{made}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copy_serves_the_client_and_offers_the_packages_that_are_beside_it() {
    let (_made, app) = copied("beside");
    std::fs::create_dir_all(app.join("ui")).unwrap();
    std::fs::write(
        app.join("ui/index.html"),
        "<!doctype html><meta name=\"build\" content=\"beside-1\"><title>The client beside it</title>",
    )
    .unwrap();
    // One of the packages there are, and only that one, beside it.
    let mut packages: Vec<PathBuf> = std::fs::read_dir(checkout().join("extensions"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("extension.yaml").is_file())
        .collect();
    packages.sort();
    let (one, others) = packages.split_first().expect("the checkout has packages");
    let name = |path: &Path| path.file_name().unwrap().to_string_lossy().into_owned();
    std::fs::create_dir_all(app.join("extensions").join(name(one))).unwrap();
    std::fs::copy(one.join("extension.yaml"), app.join("extensions").join(name(one)).join("extension.yaml"))
        .unwrap();

    let copy = Started::from(&app.join(PROGRAM)).await;
    assert!(copy.get("/").await.contains("The client beside it"));
    let meta: Value = serde_json::from_str(&copy.get("/api/meta").await).unwrap();
    assert_eq!(meta["client_build"], "beside-1");

    copy.register().await;
    let offered = copy.run(&["extension", "list"]).await;
    let came = format!("came with this installation, not installed: {}  (", name(one));
    assert!(offered.contains(&came), "{offered}");
    assert!(!others.is_empty(), "the checkout has more, which a copy does not offer");
}

#[tokio::test(flavor = "multi_thread")]
async fn run_from_where_it_was_built_the_program_reads_its_checkout() {
    let checkout = checkout();
    let built = Started::from(Path::new(BINARY)).await;
    let blueprints = built.line("  blueprints  ").expect("it says where the blueprints are read from");
    assert!(blueprints.contains(checkout.join("blueprints").to_str().unwrap()), "{blueprints}");
    let which = built.line("  program  ").unwrap();
    assert!(which.contains(&format!("run from its checkout at {}", checkout.display())), "{which}");
    assert_eq!(built.runs_from().await, "checkout");

    built.register().await;
    let packages = built.run(&["extension", "list"]).await;
    assert!(packages.contains("came with this installation, not installed: "), "{packages}");
    assert!(built.run(&["status"]).await.contains("Run from its checkout"));

    // The mark is cargo's, not the path: the same program copied into a
    // folder that cargo did not build into is a copy, wherever that is,
    // and one in a folder with cargo's mark is where it was built.
    let made = Folder(scratch("marked"));
    let marked = made.0.join("debug");
    std::fs::create_dir_all(marked.join(".fingerprint")).unwrap();
    put_program(&marked);
    let there = Started::from(&marked.join(PROGRAM)).await;
    assert_eq!(there.runs_from().await, "checkout");
}
