//! What an agent may do with its project's repository behind its wall:
//! nothing, read it, commit its own work, or all of it; and that what git
//! would run is never its to change.
//!
//! As the tests of walls are: a stand-in for the agent's program, a
//! stand-in for what holds the wall where what is asked of it is what is
//! held to, and the system's own where the system will hold one.
//!
//! Unix's alone: played behind a wall, or the stand-in for one
//! (`walls.rs`), which only Unix has.
#![cfg(unix)]

use std::path::{Path, PathBuf};

use agents::SlotSpec;
use hosting::Walls;
use kernel::ErrorKind;

use crate::ada;
use crate::walls::{Walled, shows, stand_in, words};

/// Runs git in `dir` as the tests do: with no hooks, and as somebody,
/// whoever the environment the suite is run from names (`kernel::git::WHO`).
pub(crate) fn git(dir: &Path, args: &[&str]) -> String {
    let mut git = std::process::Command::new("git");
    for name in kernel::git::WHO {
        git.env_remove(name);
    }
    let ran = git
        .current_dir(dir)
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "user.name=Ada",
            "-c",
            "user.email=ada@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
    String::from_utf8_lossy(&ran.stdout).trim().to_string()
}

/// The project as a repository with a commit, and a worktree of it for a
/// card on a branch of its own, as a lane has: the worktree, and what the
/// project's own branch is called.
fn with_a_lane(project: &Path) -> (PathBuf, String) {
    if !project.join(".git").exists() {
        git(project, &["init", "--quiet", "."]);
    }
    std::fs::write(project.join("README.md"), "# Shop\n").unwrap();
    git(project, &["add", "README.md"]);
    git(project, &["commit", "--quiet", "-m", "first"]);
    std::fs::create_dir_all(project.join(".lanes")).unwrap();
    std::fs::write(project.join(".lanes/.gitignore"), "*\n").unwrap();
    git(project, &["worktree", "add", "--quiet", "-b", "lane/shop-1", ".lanes/shop-1"]);
    (project.join(".lanes/shop-1"), git(project, &["rev-parse", "--abbrev-ref", "HEAD"]))
}

async fn may(walled: &Walled, git: &str) {
    let spec = SlotSpec { name: "builder".into(), git: Some(git.into()), ..SlotSpec::default() };
    walled.rig.agents.ensure(spec, &ada()).await.unwrap();
}

#[tokio::test]
async fn a_repository_is_shown_as_far_as_an_agent_may_do_git_with_it_and_what_git_would_run_is_never_its_to_change()
 {
    let (walls, told) = stand_in();
    let walled = Walled::holding(walls).await;
    let project = walled.project();
    let (lane, _) = with_a_lane(&project);
    let repository = project.join(".git");
    let own = repository.join("worktrees/shop-1");
    let shut = |said: &[String], file: &Path| {
        let file = file.to_string_lossy();
        said.windows(3).any(|three| three[0] == "--ro-bind" && three[1] == "/dev/null" && three[2] == file)
    };
    let in_lane = Some(".lanes/shop-1");

    // Unless it is said, it reads: the repository is there, and nothing of it is changed.
    assert_eq!(walled.rig.agents.slot("builder").await.unwrap().git, "read");
    walled.does("builder", "echo done", in_lane).await;
    let said = words(&told);
    assert!(
        shows(&said, "--ro-bind", &repository) && shows(&said, "--ro-bind", &lane.join(".git")),
        "{said:#?}"
    );
    assert!(!said.windows(2).any(|two| two[0] == "--bind" && two[1].contains("/.git")), "{said:#?}");
    assert!(!walled.env().contains("GIT_AUTHOR_NAME"));

    // Committing its own work, in a card's worktree: the worktree's own
    // part, new objects, and the branch it is on with those beside it.
    may(&walled, "commit").await;
    walled.does("builder", "echo done", in_lane).await;
    let said = words(&told);
    assert!(shows(&said, "--ro-bind", &repository), "{said:#?}");
    for its in [repository.join("objects"), own.clone(), repository.join("refs/heads/lane")] {
        assert!(shows(&said, "--bind", &its), "{} is its to change: {said:#?}", its.display());
    }
    // Not the repository whole, nor the branches at large: the project's own is not its to move.
    assert!(!shows(&said, "--bind", &repository) && !shows(&said, "--bind", &repository.join("refs/heads")));
    // And what says where its repository is, and how git behaves there, stays as it is.
    assert!(
        shows(&said, "--ro-bind", &own.join("commondir")) && shows(&said, "--ro-bind", &own.join("gitdir"))
    );
    assert!(shut(&said, &own.join("config.worktree")), "{said:#?}");
    // Its commits are its own, at an address that is nobody's.
    let env = walled.env();
    for line in
        ["GIT_AUTHOR_NAME=builder", "GIT_COMMITTER_NAME=builder", "GIT_AUTHOR_EMAIL=builder@agents.invalid"]
    {
        assert!(env.lines().any(|one| one == line), "{line}: {env}");
    }
    let about = std::fs::read_to_string(walled.here.join("args")).unwrap();
    assert!(about.contains("You may commit your own work with git"), "{about}");

    // The same in the project itself: the repository, less what git would run and what is not branches.
    walled.does("builder", "echo done", None).await;
    let said = words(&told);
    assert!(shows(&said, "--bind", &repository), "{said:#?}");
    for kept in ["config", "hooks", "worktrees", "info"] {
        assert!(shows(&said, "--ro-bind", &repository.join(kept)), "`{kept}` stays as it is: {said:#?}");
    }
    assert!(shut(&said, &repository.join("config.worktree")));

    // All of it: still not what git would run, nor where a worktree's repository is.
    may(&walled, "write").await;
    walled.does("builder", "echo done", in_lane).await;
    let said = words(&told);
    assert!(shows(&said, "--bind", &repository), "{said:#?}");
    assert!(
        shows(&said, "--ro-bind", &repository.join("config"))
            && shows(&said, "--ro-bind", &repository.join("hooks"))
    );
    assert!(shows(&said, "--ro-bind", &own.join("commondir")) && shut(&said, &own.join("config.worktree")));
    assert!(!shows(&said, "--ro-bind", &repository.join("worktrees")));

    // None of it.
    may(&walled, "off").await;
    walled.does("builder", "echo done", in_lane).await;
    let said = words(&told);
    let folder = repository.to_string_lossy();
    assert!(said.windows(2).any(|two| two[0] == "--tmpfs" && two[1] == folder), "{said:#?}");
    assert!(!shows(&said, "--ro-bind", &repository) && !shows(&said, "--bind", &repository));

    // One held to reading everything reads the repository too, whatever it was told.
    let reads = SlotSpec {
        name: "builder".into(),
        git: Some("write".into()),
        permission_mode: Some("read".into()),
        ..SlotSpec::default()
    };
    walled.rig.agents.ensure(reads, &ada()).await.unwrap();
    walled.does("builder", "echo done", in_lane).await;
    let said = words(&told);
    assert!(shows(&said, "--ro-bind", &repository) && !shows(&said, "--bind", &repository), "{said:#?}");

    // And a word that is none of the four is refused where it is said.
    let wrong = SlotSpec { name: "builder".into(), git: Some("push".into()), ..SlotSpec::default() };
    let refused = walled.rig.agents.ensure(wrong, &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert!(refused.message.contains("there are: off, read, commit, write"), "{}", refused.message);
}

#[tokio::test]
async fn behind_a_real_wall_an_agent_that_may_commit_commits_its_own_work_and_moves_nothing_that_is_not_its()
{
    let walls = Walls::system();
    if let Err(why) = walls.can() {
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let walled = Walled::holding(walls).await;
    let project = walled.project();
    let (lane, own_branch) = with_a_lane(&project);
    let first = git(&project, &["rev-parse", "HEAD"]);
    let in_lane = Some(".lanes/shop-1");
    let commits = "echo mine > mine.txt; git add -A 2>&1 && git commit --quiet -m mine 2>&1 && echo committed || echo not-committed";

    // Reading, it cannot.
    let turn = walled.does("builder", commits, in_lane).await;
    assert!(turn.reply.lines().any(|line| line == "not-committed"), "{}", turn.reply);
    assert_eq!(git(&lane, &["rev-parse", "HEAD"]), first);

    // Told it may commit, it does: on its own branch, as itself.
    may(&walled, "commit").await;
    let tries = [
        commits.to_string(),
        "git log -1 --format='%an <%ae>'".to_string(),
        format!("git update-ref refs/heads/{own_branch} HEAD 2>/dev/null && echo moved-the-projects-branch || echo branch-kept"),
        "git tag v1 2>/dev/null && echo tagged || echo no-tag".to_string(),
        "common=$(git rev-parse --git-common-dir); own=$(git rev-parse --git-dir)".to_string(),
        "(echo '[core] fsmonitor = false' >> $common/config) 2>/dev/null && echo wrote-settings || echo settings-kept".to_string(),
        "(echo x > $common/hooks/pre-commit) 2>/dev/null && echo wrote-a-hook || echo hooks-kept".to_string(),
        "(echo x > $own/config.worktree) 2>/dev/null && echo wrote-its-settings || echo its-settings-kept".to_string(),
        "(echo /elsewhere > $own/commondir) 2>/dev/null && echo moved-its-repository || echo its-repository-kept".to_string(),
    ]
    .join("; ");
    let turn = walled.does("builder", &tries, in_lane).await;
    assert_eq!(turn.status, "done", "{:?}", turn.error);
    for expected in [
        "committed",
        "builder <builder@agents.invalid>",
        "branch-kept",
        "no-tag",
        "settings-kept",
        "hooks-kept",
        "its-settings-kept",
        "its-repository-kept",
    ] {
        assert!(
            turn.reply.lines().any(|line| line.trim() == expected),
            "expected `{expected}` in:\n{}",
            turn.reply
        );
    }
    // Seen from outside: its branch has its commit, and the project's own is where it was.
    assert_eq!(git(&project, &["log", "-1", "--format=%s %an", "lane/shop-1"]), "mine builder");
    assert_eq!(git(&project, &["rev-parse", &own_branch]), first);
    assert!(!git(&project, &["config", "--local", "--list"]).contains("fsmonitor"));

    // Told it may write, it moves what it likes, and still not what git would run.
    may(&walled, "write").await;
    let tries = [
        "git tag v1 2>/dev/null && echo tagged || echo no-tag",
        "common=$(git rev-parse --git-common-dir)",
        "(echo '[core] fsmonitor = false' >> $common/config) 2>/dev/null && echo wrote-settings || echo settings-kept",
        "(echo x > $common/hooks/pre-commit) 2>/dev/null && echo wrote-a-hook || echo hooks-kept",
    ]
    .join("; ");
    let turn = walled.does("builder", &tries, in_lane).await;
    for expected in ["tagged", "settings-kept", "hooks-kept"] {
        assert!(
            turn.reply.lines().any(|line| line.trim() == expected),
            "expected `{expected}` in:\n{}",
            turn.reply
        );
    }
    assert_eq!(git(&project, &["tag", "--list"]), "v1");
}
