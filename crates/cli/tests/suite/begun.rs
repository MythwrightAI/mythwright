//! What a new project's repository begins with: the files the factory
//! started it with, as its first commit, so that the first card worked
//! there has a branch to be cut from and a branch to be merged into.
//!
//! Played by the real program as the daemon, as a person runs it, with a
//! home of its own: who git commits as is what that home's `.gitconfig`
//! says, and nothing of whoever runs the suite. Seen in a real run first:
//! a project with no commit had its first card worked in its own folder,
//! with nothing kept of it; and once its owner made a first commit of
//! their own, the starting files were still untracked there, and the first
//! approved branch that changed one of them could not be merged.
//!
//! Unix's alone: the daemon here is the real program as a process of its
//! own (`daemon::Daemon`), which the rig stops by a signal.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use crate::Ran;
use crate::daemon::Daemon;
use crate::keyed::asking;
use crate::standin::argv;

/// What the factory starts a project with that has a library.
const STARTED: [&str; 3] = ["README.md", "docs/architecture.md", "docs/overview.md"];

/// Says in the daemon's home who git commits as: a name and an address,
/// or nobody. Told nobody, git would make somebody up from the machine's
/// names where it can, which a first commit is never made as.
fn commits_as(daemon: &Daemon, who: Option<(&str, &str)>) {
    let said = match who {
        Some((name, email)) => format!("[user]\n\tname = {name}\n\temail = {email}\n"),
        None => String::new(),
    };
    std::fs::write(daemon.dir.join(".gitconfig"), said).unwrap();
}

/// Git in `dir`, as the person whose home is the daemon's: what it
/// printed, and whether it did what was asked.
fn git_as(daemon: &Daemon, dir: &Path, args: &[&str]) -> (bool, String) {
    let mut asked = asking(dir);
    asked.env("HOME", &daemon.dir).env("XDG_CONFIG_HOME", daemon.dir.join("config")).args(args);
    let ran = asked.output().unwrap();
    let said = format!("{}{}", String::from_utf8_lossy(&ran.stdout), String::from_utf8_lossy(&ran.stderr));
    (ran.status.success(), said.trim().to_string())
}

/// The same, where it must have done it.
fn git(daemon: &Daemon, dir: &Path, args: &[&str]) -> String {
    let (done, said) = git_as(daemon, dir, args);
    assert!(done, "git {args:?}: {said}");
    said
}

/// A daemon whose agents run on `writer`, which adds a line to the
/// project's README where it is run, and `critic`, which approves; with
/// an owner signed in.
async fn factory() -> Daemon {
    let writes = argv(&["swallow", "append", "README.md", "A-cart.", "say", "done"]);
    // Each word of a program said with `--command` is apart from the next by a space.
    let looks = argv(&["swallow", "say", "Right.", "say", "DECISION:approve"]);
    let (writes, looks) = (format!("writer={}", writes.join(" ")), format!("critic={}", looks.join(" ")));
    let told = ["--walls", "off", "--adapter", "writer", "--command", &writes, "--command", &looks];
    let daemon = Daemon::start_told("serve", None, None, &told).await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    daemon
}

/// Adds `Shop` with what a project starts with, and gives where it is
/// and what the command said.
async fn shop(daemon: &Daemon) -> (PathBuf, String) {
    let said = daemon.run(&["project", "create", "Shop"]).await.ok();
    let listed = daemon.run(&["project", "list", "--json"]).await.ok();
    let listed: serde_json::Value = serde_json::from_str(&listed).unwrap();
    let shop = listed["projects"].as_array().unwrap().iter().find(|one| one["id"] == "shop").unwrap().clone();
    (PathBuf::from(shop["path"].as_str().unwrap()), said)
}

/// Asks `args` until what it prints says `wanted`, and fails with what
/// the project's channel and the card say where it never does.
async fn until(daemon: &Daemon, args: &[&str], wanted: &str) -> String {
    let asked = async {
        loop {
            let said = daemon.run(args).await.out;
            if said.contains(wanted) {
                return said;
            }
            tokio::task::yield_now().await;
        }
    };
    match tokio::time::timeout(std::time::Duration::from_secs(40), asked).await {
        Ok(said) => said,
        Err(_) => {
            let read = daemon.run(&["read", "shop"]).await.out;
            let history = daemon.run(&["board", "history", "shop-1"]).await.out;
            panic!("`{}` never said `{wanted}`.\nThe channel:\n{read}\nThe card:\n{history}", args.join(" "))
        }
    }
}

/// One who reviews, on `critic`, and a card worked and passed by the fleet.
async fn worked_and_passed(daemon: &Daemon) {
    daemon.run(&["record", "set", "shop-agents-3", "count=1", "adapter=critic"]).await.ok();
    until(daemon, &["agent", "list"], "shop-reviewer-1").await;
    daemon.run(&["board", "add", "shop", "Add a cart"]).await.ok();
    daemon.run(&["board", "move", "shop-1", "ready"]).await.ok();
    until(daemon, &["board", "shop"], "done (1)").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_projects_starting_files_are_its_first_commit_made_as_whoever_git_says_commits() {
    let daemon = factory().await;
    commits_as(&daemon, Some(("Ada", "ada@example.com")));
    let (dir, said) = shop(&daemon).await;

    // One commit, on main, holding what it was started with and nothing
    // else: written by the factory, committed by whoever git says.
    let log = git(&daemon, &dir, &["log", "--format=%an <%ae>|%cn <%ce>|%s", "main"]);
    let factory = format!("{} <{}@factory.invalid>", brand::DISPLAY, brand::CLI);
    assert_eq!(log, format!("{factory}|Ada <ada@example.com>|Shop: the files it was started with"));
    let files = git(&daemon, &dir, &["ls-files"]);
    assert_eq!(files.lines().collect::<Vec<_>>(), STARTED);
    assert_eq!(git(&daemon, &dir, &["status", "--porcelain"]), "", "nothing is left untracked");
    // And the command says so.
    let commit = git(&daemon, &dir, &["rev-parse", "--short", "main"]);
    assert!(
        said.contains(&format!(
            "The files it was started with are its first commit ({commit}), so each card's work is on a branch of its own"
        )),
        "{said}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_first_card_of_a_new_project_is_worked_on_a_branch_of_its_own_and_merged_when_it_passes() {
    let daemon = factory().await;
    commits_as(&daemon, Some(("Ada", "ada@example.com")));
    let (dir, _) = shop(&daemon).await;
    worked_and_passed(&daemon).await;

    // Its work was kept on its branch and merged, and nothing of it lies
    // about the project's folder untracked.
    let merges = git(&daemon, &dir, &["log", "--merges", "--format=%s", "main"]);
    assert!(merges.contains("shop-1"), "{merges}");
    let kept = git(&daemon, &dir, &["log", "--no-merges", "--format=%an|%s", "main"]);
    assert!(kept.lines().any(|line| line == "shop-worker-1|shop-1: Add a cart"), "{kept}");
    assert_eq!(git(&daemon, &dir, &["status", "--porcelain"]), "");
    let readme = std::fs::read_to_string(dir.join("README.md")).unwrap();
    assert!(readme.starts_with("# Shop\n") && readme.ends_with("A-cart.\n"), "{readme}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_branch_that_changes_a_starting_file_is_merged_where_the_owner_made_a_first_commit_of_their_own() {
    let daemon = factory().await;
    commits_as(&daemon, Some(("Ada", "ada@example.com")));
    let (dir, _) = shop(&daemon).await;
    // As the person in the real run did, finding no commit there.
    git(&daemon, &dir, &["commit", "--quiet", "--allow-empty", "-m", "A start of my own"]);
    worked_and_passed(&daemon).await;

    let readme = git(&daemon, &dir, &["show", "main:README.md"]);
    assert!(readme.starts_with("# Shop") && readme.ends_with("A-cart."), "{readme}");
    assert_eq!(git(&daemon, &dir, &["status", "--porcelain"]), "");
}

#[tokio::test(flavor = "multi_thread")]
async fn where_git_does_not_know_who_to_commit_as_a_new_project_is_made_and_says_what_is_not_committed() {
    let daemon = factory().await;
    commits_as(&daemon, None);
    let (dir, said) = shop(&daemon).await;

    // Made, with its files, and nothing committed for it.
    assert!(said.contains("project `shop` created at"), "{said}");
    for file in STARTED {
        assert!(dir.join(file).is_file(), "{file}");
    }
    let (made, _) = git_as(&daemon, &dir, &["rev-parse", "--verify", "--quiet", "HEAD"]);
    assert!(!made, "no commit was made");
    // Said as the fleet says it, with what to do.
    assert!(
        said.contains(
            "Git does not know who to commit as, so the files it was started with are not committed, and a card's \
             work is done in its folder and not on a branch of its own: tell it in the project (git config \
             user.name, and git config user.email), then commit them there"
        ),
        "{said}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_that_was_there_already_is_taken_up_with_nothing_of_it_committed() {
    let daemon = factory().await;
    commits_as(&daemon, Some(("Ada", "ada@example.com")));
    // A repository with no commit yet, and a file of its owner's own.
    let theirs = daemon.dir.join("theirs");
    std::fs::create_dir_all(&theirs).unwrap();
    git(&daemon, &theirs, &["init", "--quiet", "."]);
    std::fs::write(theirs.join("notes.md"), "mine\n").unwrap();
    // And one that has its own history.
    let kept = daemon.dir.join("kept");
    std::fs::create_dir_all(&kept).unwrap();
    git(&daemon, &kept, &["init", "--quiet", "."]);
    std::fs::write(kept.join("notes.md"), "mine\n").unwrap();
    git(&daemon, &kept, &["add", "-A"]);
    git(&daemon, &kept, &["commit", "--quiet", "-m", "Mine"]);
    std::fs::write(kept.join("draft.md"), "not yet\n").unwrap();

    for (name, folder) in [("Theirs", &theirs), ("Kept", &kept)] {
        let ran: Ran = daemon.run(&["project", "create", name, "--path", folder.to_str().unwrap()]).await;
        let said = ran.ok();
        assert!(said.contains("adopted as it is"), "{said}");
        assert!(!said.contains("first commit"), "{said}");
    }
    let (made, _) = git_as(&daemon, &theirs, &["rev-parse", "--verify", "--quiet", "HEAD"]);
    assert!(!made, "nothing of theirs was committed for them");
    assert_eq!(git(&daemon, &theirs, &["status", "--porcelain"]), "?? notes.md");
    assert_eq!(git(&daemon, &kept, &["log", "--format=%s"]), "Mine");
    assert_eq!(git(&daemon, &kept, &["status", "--porcelain"]), "?? draft.md");
}
