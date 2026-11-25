//! A fleet whose agents work behind walls the system really holds: a card
//! goes through it as it does with none, and what an agent's program
//! could reach of the factory with none is not there.
//!
//! The other suites hold no walls, so that their stand-ins see the device
//! on every system. Here the system is asked for real, and where it holds
//! none the test says so and holds nothing to it.
//!
//! Unix's alone: a wall is held by a program of that system's, and
//! nothing holds one on Windows; the stand-ins here are lines of its
//! shell.
#![cfg(unix)]

use serde_json::json;

use crate::Rig;
use crate::fleets::{active, add, shop};
use crate::lanes::{git, move_card, moved};

#[tokio::test]
async fn a_card_goes_through_a_fleet_whose_agents_work_behind_walls_and_none_of_them_reads_the_factorys_own()
{
    if let Err(why) = api::Walls::system().can() {
        println!("skipped: this system holds no wall ({why})");
        return;
    }
    let rig = Rig::with("api-walled", |config, dir| {
        // Held by the system's own program, as a factory is when nobody says otherwise.
        config.walls = None;
        // The worker says what it could reach of the factory's own, in the
        // file it changes: the secret, and what another agent signs in to
        // the factory with. And it tries to make the repository run
        // something of its own the next time anybody asks git anything.
        let data = dir.join("data");
        let writes = format!(
            "cat >/dev/null; \
             (cat {secret} >/dev/null 2>&1 && echo read-the-secret || echo no-secret) >> CHANGES.md; \
             (cat {other} >/dev/null 2>&1 && echo read-a-credential || echo no-credential) >> CHANGES.md; \
             (echo '[core] fsmonitor = touch {dir}/ran-outside' >> \"$(git rev-parse --git-common-dir)/config\") 2>/dev/null \
               && echo wrote-the-repository >> CHANGES.md || echo repository-kept >> CHANGES.md; \
             echo I changed a file.",
            secret = data.join("installation.secret").display(),
            other = data.join("slots/shop-reviewer-1/token").display(),
            dir = dir.display(),
        );
        let looks = "cat >/dev/null; echo It does what the card asks.; echo DECISION: approve";
        config.adapters.push(("writer".into(), vec!["sh".into(), "-c".into(), writes]));
        config.adapters.push(("critic".into(), vec!["sh".into(), "-c".into(), looks.into()]));
    })
    .await;
    let rig = Rig::owned(rig).await;
    let dir = shop(&rig, json!({})).await;
    assert_eq!(active(&rig, "worker").await, ["shop-worker-1"]);
    let agents = rig.read("/api/agents").await;
    assert!(agents["agents"].as_array().unwrap().iter().all(|agent| agent["wall"] == true), "{agents}");

    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "done", 0).await;

    // Worked in a worktree of its own, kept as a commit by the fleet, looked
    // at and merged: what the worker wrote is on the project's own branch.
    let written = std::fs::read_to_string(dir.join("CHANGES.md")).unwrap();
    assert_eq!(written, "no-secret\nno-credential\nrepository-kept\n", "{written}");
    assert!(git(&dir, &["log", "--oneline"]).lines().count() >= 2);
    // And nothing of the worker's ran when the fleet asked git to keep and merge it.
    assert!(!rig.dir.join("ran-outside").exists());
    assert!(!git(&dir, &["config", "--local", "--list"]).contains("fsmonitor"));
    let query =
        journal::EventQuery { topics: "agent.turn.finished".into(), limit: 100, ..Default::default() };
    let finished = rig.said(query).await;
    assert!(finished.len() >= 2 && finished.iter().all(|event| event.data["ok"] == true), "{finished:?}");
}

#[test]
fn git_is_asked_in_one_place_and_by_no_blueprint_that_ships() {
    // A hook runs what is in the worktree, which an agent wrote, and what
    // the daemon asks of git it asks with no wall. That it runs no hook is
    // said once, where git is asked (`kernel::git`, whose own tests hold it
    // to that), so nothing else may ask git a way of its own: not a
    // blueprint that ships, and not another line of Rust.
    let top = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut wrong = Vec::new();
    let mut looked = 0;
    let mut folders = vec![top.join("blueprints"), top.join("extensions"), top.join("crates")];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                // A crate's tests make repositories of their own to try it against.
                if name != "tests" && name != "target" {
                    folders.push(path);
                }
                continue;
            }
            let (blueprint, rust) = (name.ends_with(".bp"), name.ends_with(".rs"));
            if !blueprint && !rust || path.ends_with("kernel/src/git.rs") {
                continue;
            }
            looked += 1;
            let text = std::fs::read_to_string(&path).unwrap();
            for (at, line) in text.lines().enumerate() {
                let words = line.trim();
                let asks = if blueprint {
                    // Written on a line, as a list in a line of its own, or worked out.
                    words.contains("[git,") || words.contains("[\"git\",") || words == "- git"
                } else {
                    words.contains("Command::new(\"git\")")
                };
                if asks {
                    wrong.push(format!("{}:{}: {words}", path.strip_prefix(&top).unwrap().display(), at + 1));
                }
            }
        }
    }
    assert!(looked > 100, "looked at only {looked} files: is this still where they are?");
    assert!(
        wrong.is_empty(),
        "asked of git a way of their own; ask it through `kernel::git`, and from a blueprint with an action that does:\n{}",
        wrong.join("\n")
    );
}

#[test]
fn a_suite_starts_git_in_one_place_which_is_not_told_who_commits_by_the_environment() {
    // Git goes by an author and a committer named in its environment
    // before what a repository says. Where the suites are run with those
    // named (a hosted runner, a shell that names its owner), what a test
    // committed was by somebody else than the repository it had made
    // says, and three tests of who committed said so. The daemon's own
    // git takes them out where it is asked (`kernel::git::WHO`, held by
    // the kernel's suite); a suite's own is started in one place, which
    // does the same, so that the suites say one thing wherever they run.
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let (mut wrong, mut places) = (Vec::new(), 0);
    for member in std::fs::read_dir(&crates).unwrap().flatten() {
        let mut starts = Vec::new();
        let mut folders = vec![member.path().join("tests")];
        while let Some(folder) = folders.pop() {
            for entry in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    folders.push(path);
                } else if path.extension().is_some_and(|ending| ending == "rs") {
                    let text = std::fs::read_to_string(&path).unwrap();
                    let lines: Vec<&str> = text.lines().collect();
                    for (at, line) in lines.iter().enumerate() {
                        if line.contains("Command::new(\"git\")") {
                            // Taken out in the lines that follow, before anything is asked of it.
                            let takes_out = lines[at..(at + 8).min(lines.len())]
                                .iter()
                                .any(|line| line.contains("kernel::git::WHO"));
                            starts.push((format!("{}:{}", path.display(), at + 1), takes_out));
                        }
                    }
                }
            }
        }
        places += starts.len();
        if starts.len() > 1 {
            let all: Vec<&str> = starts.iter().map(|(at, _)| at.as_str()).collect();
            wrong.push(format!("git is started in more than one place of a suite: {}", all.join(", ")));
        }
        for (at, takes_out) in starts {
            if !takes_out {
                wrong.push(format!(
                    "{at} starts git without taking `kernel::git::WHO` out of what it is handed"
                ));
            }
        }
    }
    assert!(places >= 3, "found git started in only {places} suites: is this still where they are?");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[tokio::test]
async fn hooks_an_agent_left_in_its_work_are_not_run_when_the_fleet_keeps_and_merges_it() {
    // With no wall at all, as on a system that holds none: it is the
    // daemon's own git this is about, wherever the agent's program ran.
    let rig = Rig::with("api-hooks", |config, dir| {
        // The worker writes a hook for everything git does, in the folder
        // the repository is set to find its hooks in.
        let hooks = "pre-commit post-commit commit-msg post-merge pre-merge-commit post-checkout reference-transaction post-index-change";
        let writes = format!(
            "cat >/dev/null; mkdir -p .githooks; for hook in {hooks}; do \
             printf '#!/bin/sh\\ntouch {dir}/a-hook-ran\\n' > .githooks/$hook; chmod +x .githooks/$hook; done; \
             echo changed >> CHANGES.md; echo I changed a file.",
            dir = dir.display(),
        );
        let looks = "cat >/dev/null; echo It does what the card asks.; echo DECISION: approve";
        config.adapters.push(("writer".into(), vec!["sh".into(), "-c".into(), writes]));
        config.adapters.push(("critic".into(), vec!["sh".into(), "-c".into(), looks.into()]));
    })
    .await;
    let rig = Rig::owned(rig).await;
    let dir = shop(&rig, json!({})).await;
    // As a project that keeps its hooks among its files has it.
    git(&dir, &["config", "core.hooksPath", ".githooks"]);

    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "done", 0).await;

    // Kept as a commit, merged, its worktree taken away: and none of it ran a hook.
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "changed\n");
    assert!(dir.join(".githooks/post-merge").exists(), "the hooks were merged with the rest of the work");
    assert!(!rig.dir.join("a-hook-ran").exists(), "a hook the agent wrote was run by the daemon's own git");
}
