//! Which of a project's branches moved, as its repository says: what a
//! pipeline triggered by a branch hears (`project.branch.moved`).
//!
//! A branch is a file of the repository's own (`refs/heads/<name>`), or a
//! line of `packed-refs` where git writes them together, and every way a
//! branch moves (a commit, a merge, a fetch into it, a reset, a push to
//! the repository from somewhere else) writes one of those. So the
//! folders they are in are followed as the system can say (`hosting::
//! Watch`), and nothing looks on a timer; what moved is read from git as
//! it is then, set against what was said last ([`moved`]).
//!
//! A repository is followed so only where something asked for it
//! ([`Followed`], `branches.follow`): agents commit on branches of their
//! own all day, and nothing is to be said of each commit where nothing
//! hears. Whoever asks asks again as the daemon starts (`std/pipeline`
//! does, on `system.started`): what is asked is kept in memory only.
//!
//! Where it was said last is kept by whoever says it, in the write that
//! says it, so that a branch that moved while the daemon was away, or in
//! the moment it went, is said once it is back, and none twice. Only
//! branches are said, never a remote's or a tag.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use blueprint::ActionSpec;
use conductor::{ActionError, Registry};
use journal::Journal;
use kernel::Source;
use serde_json::json;

/// Where each branch of a project was said to be last, by the project, in
/// the journal that keeps what is said of it.
pub const NAMESPACE: &str = "branches";

/// How long git is given to list the branches.
const ASKING: Duration = Duration::from_secs(20);

/// How many folders under `refs/heads` are followed: one for each part of
/// a branch's name that has a `/` (`lane/…`, `feature/…`).
const MOST_FOLDERS: usize = 200;

/// What a repository's branches point at, by each one's short name.
pub type Heads = BTreeMap<String, String>;

/// A branch that moved: from where to where, either empty where it was
/// made or taken away.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moved {
    pub branch: String,
    pub from: String,
    pub to: String,
}

/// What moved between what was said last and what is so now, in the
/// order of the branches' names.
pub fn moved(was: &Heads, now: &Heads) -> Vec<Moved> {
    let names: BTreeSet<&String> = was.keys().chain(now.keys()).collect();
    names
        .into_iter()
        .filter_map(|branch| {
            let (from, to) =
                (was.get(branch).cloned().unwrap_or_default(), now.get(branch).cloned().unwrap_or_default());
            (from != to).then(|| Moved { branch: branch.clone(), from, to })
        })
        .collect()
}

/// The branches git listed with `--format=%(refname:short)%09%(objectname)`.
pub fn heads_of(listed: &str) -> Heads {
    listed
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(name, commit)| !name.is_empty() && !commit.is_empty())
        .map(|(name, commit)| (name.trim().to_string(), commit.trim().to_string()))
        .collect()
}

/// The branches of the repository in `dir` as they are; nothing where it
/// is no repository or git could not say.
pub async fn heads(dir: &Path) -> Result<Heads, String> {
    let mut command = tokio::process::Command::from(kernel::git::git(dir));
    command
        .args(["for-each-ref", "--format=%(refname:short)%09%(objectname)", "refs/heads"])
        .kill_on_drop(true);
    match tokio::time::timeout(ASKING, command.output()).await {
        Ok(Ok(output)) if output.status.success() => Ok(heads_of(&String::from_utf8_lossy(&output.stdout))),
        Ok(Ok(output)) => Err(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Ok(Err(err)) => Err(format!("git could not be started: {err}. Is git installed?")),
        Err(_) => Err(format!("git did not list the branches in {} seconds", ASKING.as_secs())),
    }
}

/// The folders a repository's branches are written in: its own folder,
/// for `packed-refs`, and `refs/heads` with each folder under it. None
/// where `.git` is no folder (a worktree of another repository).
pub fn folders(dir: &Path) -> Vec<PathBuf> {
    let own = dir.join(".git");
    if !own.is_dir() {
        return Vec::new();
    }
    let mut found = vec![own.clone()];
    let mut under = vec![own.join("refs").join("heads")];
    while let Some(folder) = under.pop() {
        if !folder.is_dir() || found.len() > MOST_FOLDERS {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&folder) {
            under.extend(entries.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()));
        }
        found.push(folder);
    }
    found
}

/// The projects whose branches something asked to follow since the
/// daemon started, and the telling that one more was.
#[derive(Default)]
pub struct Followed {
    asked: Mutex<BTreeSet<String>>,
    told: tokio::sync::Notify,
}

impl Followed {
    /// Says that `project`'s branches are to be followed; told to whoever
    /// follows where they were not.
    pub fn ask(&self, project: &str) {
        if self.asked.lock().expect("followed lock").insert(project.to_string()) {
            self.told.notify_one();
        }
    }

    /// The projects asked for.
    pub fn projects(&self) -> Vec<String> {
        self.asked.lock().expect("followed lock").iter().cloned().collect()
    }

    /// Waits until a project nobody had asked for is asked for.
    pub async fn more(&self) {
        self.told.notified().await;
    }
}

/// What `branches.follow` is shown working with.
const FOLLOWING: &str = "\
blueprint: built-on-main
description: Build the project each time its main branch moves.
permissions: [projects:read, shell]

steps:
  followed:
    do: branches.follow

on:
  moved:
    event: project.branch.moved
    when: ${ event.data.branch == \"main\" && event.data.to != \"\" }
    do: shell
    with: { cmd: [make, build] }
";

/// Adds `branches.follow`.
pub fn register(registry: &mut Registry, followed: std::sync::Arc<Followed>) {
    let spec = ActionSpec::new(
        "branches.follow",
        "Have the project's branches said as they move (`project.branch.moved`), until the daemon stops. Asked again it changes nothing.",
    )
    .example(FOLLOWING)
    .capability("projects:read")
    .output("project", "the project whose branches are followed")
    .idempotent();
    registry.add_fn(spec, move |cx, _args| {
        let followed = followed.clone();
        Box::pin(async move {
            let Some(project) = cx.project().map(str::to_string) else {
                return Err(ActionError::new(
                    "branches are followed for a project, and this was applied for none: apply it for the project",
                ));
            };
            // Where they are as it is asked is kept, where nothing was, so
            // that one that moves before the follower has begun is said.
            let journal = cx.engine().journal().of(Some(&project));
            said(&journal, &project, &cx.workdir(), true).await;
            followed.ask(&project);
            Ok(json!({ "project": project }))
        })
    });
}

/// Says each branch of the repository in `dir` that moved since it was
/// last said, with where each is now kept in the same write; where none
/// was ever kept, keeps where they are and says nothing. What was said
/// last is read in that write, so that nothing says one move twice.
/// Asked `first`, it only keeps where they are where nothing was kept:
/// it reads them beside whoever says moves, and may read them before
/// what that one has just said.
pub async fn said(journal: &Journal, project: &str, dir: &Path, first: bool) {
    let Ok(now) = heads(dir).await else { return };
    let project = project.to_string();
    let _ = journal
        .write(move |writing| {
            let kept: Option<String> = writing
                .transaction()
                .query_row(
                    "SELECT value FROM kv WHERE namespace = ?1 AND key = ?2",
                    [NAMESPACE, &project],
                    |row| row.get(0),
                )
                .ok();
            let kept: Option<Heads> = kept.and_then(|kept| serde_json::from_str(&kept).ok());
            let moved = kept.as_ref().map(|was| moved(was, &now)).unwrap_or_default();
            if kept.is_some() && (first || moved.is_empty()) {
                return Ok(());
            }
            for one in &moved {
                let data =
                    json!({ "project": project, "branch": one.branch, "from": one.from, "to": one.to });
                writing.say("project.branch.moved", Source::actor("system"), data)?;
            }
            writing.keep(NAMESPACE, &project, &json!(now))
        })
        .await;
}
