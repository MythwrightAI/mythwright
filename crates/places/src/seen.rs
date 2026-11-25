//! A project's repository as it stands, for whoever looks at it: the
//! branch its folder is on and how that stands against the one it
//! follows, its branches, the commits of one of them, what is changed in
//! the working tree, and its remotes. And being told that any of that
//! may have changed.
//!
//! It only reads. Git is asked the one way there is (`kernel::git::git`),
//! which takes no lock that reading can do without, so that looking never
//! keeps a person's own git, or an agent's, from writing; and what it
//! answered is read by functions of plain values that are held without a
//! repository (`kernel::git::status`, `branches`, `commits`, `remotes`).
//!
//! **Being told.** Git says nothing as it changes a repository, but what
//! it changes is files in the repository's own folder: the branch the
//! folder is on (`HEAD`), what is staged (`index`), every branch and every
//! remote's (`refs/`, and `packed-refs` where they are written together).
//! Those folders are followed as the system can say ([`followed`], by
//! `hosting::Watch`), so a commit, a branch made or switched to, a file
//! staged and a fetch are heard however they were made, a person's own
//! git and an agent's in a worktree among them. What nothing tells of is
//! a file of the working tree written and not staged: its folder is not
//! git's, and following every folder of a working tree is more than the
//! system gives anybody; it is seen when something else is, or when
//! whoever looks asks again.
//!
//! A repository is followed so only once somebody has looked at it
//! ([`Looked`]): a factory's projects are repositories that agents commit
//! in all day, and nothing is to be said of each commit for a page
//! nobody opened. What is followed of every project's folder that is no
//! repository is that one is made there, which is how a project comes to
//! be listed with a Git page at all.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use kernel::Error;
use kernel::git::{self, Branch, Changed, Commit, Head, Remote};

/// How long git is given for each thing it is asked here.
const ASKING: Duration = Duration::from_secs(20);

/// How many folders under a repository's `refs` are followed: one for
/// each remote and each part of a branch's name that has a `/`.
const MOST_FOLDERS: usize = 200;

/// A repository as it stands.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Seen {
    pub head: Head,
    pub branches: Vec<Branch>,
    /// The branch the commits are of, whole (`refs/heads/main`), or
    /// `HEAD` for what the folder is on.
    pub of: String,
    pub commits: Vec<Commit>,
    pub changes: Vec<Changed>,
    /// How many changed paths there were beyond those listed.
    pub more: usize,
    pub remotes: Vec<Remote>,
}

/// Whether a project's folder is a repository of its own: one inside
/// another's is not, since git asked there would answer for the
/// repository around it.
pub fn is_repository(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// What git said, or why it could not be asked.
async fn asked(dir: &Path, what: &[&str], more: &[&str]) -> Result<(bool, String, String), Error> {
    let mut command = tokio::process::Command::from(git::git(dir));
    command.args(what).args(more).kill_on_drop(true);
    match tokio::time::timeout(ASKING, command.output()).await {
        Ok(Ok(output)) => Ok((
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )),
        Ok(Err(err)) => {
            Err(Error::unavailable(format!("git could not be started: {err}. Is git installed?")))
        }
        Err(_) => Err(Error::unavailable(format!(
            "git did not answer `git {}` in {} seconds",
            what.first().copied().unwrap_or_default(),
            ASKING.as_secs()
        ))),
    }
}

/// What git said, where it said it without failing; why not, in its last words.
async fn answered(dir: &Path, what: &[&str], more: &[&str]) -> Result<String, Error> {
    let (ok, out, err) = asked(dir, what, more).await?;
    if ok {
        return Ok(out);
    }
    let why = err.lines().rev().find(|line| !line.trim().is_empty()).unwrap_or("it ended without saying why");
    Err(Error::unavailable(format!("git could not say what was asked (`git {}`): {}", what[0], why.trim())))
}

/// The repository in `dir` as it stands, with the commits of the branch
/// `of` names (whole, as a branch is listed) or of what the folder is on.
/// A branch that is not one of those listed is refused, so that nothing
/// handed in is ever taken by git for anything but a branch.
pub async fn seen(dir: &Path, of: Option<&str>) -> Result<Seen, Error> {
    let (status, branches, remotes) = tokio::try_join!(
        answered(dir, git::STATUS, &[]),
        answered(dir, git::BRANCHES, &[]),
        answered(dir, git::REMOTES, &[]),
    )?;
    let (status, branches, remotes) =
        (git::status(&status), git::branches(&branches), git::remotes(&remotes));
    let of = match of {
        None | Some("") | Some("HEAD") => "HEAD".to_string(),
        Some(named) => match branches.iter().find(|branch| branch.refname == named) {
            Some(branch) => branch.refname.clone(),
            None => {
                return Err(Error::not_found(format!(
                    "this repository has no branch `{named}`: a branch is asked for as it is listed, whole \
                     (`refs/heads/main`)"
                )));
            }
        },
    };
    // A repository with nothing committed has no commit to list, and git
    // says so as a failure.
    let commits = match (of.as_str(), &status.head.commit) {
        ("HEAD", None) => Vec::new(),
        _ => git::commits(&answered(dir, git::COMMITS, &[&of]).await?),
    };
    Ok(Seen { head: status.head, branches, of, commits, changes: status.changes, more: status.more, remotes })
}

/// The folders of a repository's own that change as it does: its folder
/// (`HEAD`, `index`, `packed-refs`), `refs`, and each folder under that, as
/// many as [`MOST_FOLDERS`]. Nothing where `.git` is no folder (a worktree
/// of another, which says where its repository is in a file).
pub fn folders(dir: &Path) -> Vec<PathBuf> {
    let own = dir.join(".git");
    if !own.is_dir() {
        return Vec::new();
    }
    let mut found = vec![own.clone()];
    let mut under = vec![own.join("refs")];
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

/// The projects whose repository somebody has looked at since the daemon
/// started, which are followed whole from then on; and the telling that
/// one more was.
#[derive(Default)]
pub struct Looked {
    asked: Mutex<HashSet<String>>,
    told: tokio::sync::Notify,
}

impl Looked {
    /// Says that `project`'s repository was looked at; told to whoever
    /// follows where it had not been.
    pub fn ask(&self, project: &str) {
        if self.asked.lock().expect("looked lock").insert(project.to_string()) {
            self.told.notify_one();
        }
    }

    pub fn has(&self, project: &str) -> bool {
        self.asked.lock().expect("looked lock").contains(project)
    }

    /// Waits until a repository nobody had looked at is looked at.
    pub async fn more(&self) {
        self.told.notified().await;
    }
}

/// What a repository is followed for: its Git page (`page`: everything
/// git keeps of it, each change said), and its branches (`branches`: each
/// that moved said, `crate::branches`). One follower does both, so that a
/// repository is followed once whatever is asked of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct For {
    pub page: bool,
    pub branches: bool,
}

/// What a follower heard of a repository, once it is following it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heard {
    /// It began to follow one that was a repository already: what changed
    /// before it began, while whoever followed it before was let go of.
    Began,
    /// The folder came to be a repository.
    Made,
    /// Something of it changed.
    Changed,
}

/// Follows the repository in `dir` for as long as this is awaited, and
/// tells `heard` what it heard, each time once it follows it again, so
/// that nothing that changes while `heard` is at work goes unheard. Where
/// it is followed for nothing (`wants` says neither), that it is made one
/// is all that is told, once; a folder that is no repository is followed
/// for one to be made there (`git init`), whatever it is followed for.
pub async fn followed<F: Future<Output = ()>>(
    watch: hosting::Watch,
    dir: PathBuf,
    wants: For,
    heard: impl Fn(Heard) -> F,
) {
    let whole = wants.page || wants.branches;
    let mut owed: Option<Heard> = None;
    let mut first = true;
    loop {
        let named = {
            let dir = dir.clone();
            let of_page = wants.page;
            let named = move || if of_page { folders(&dir) } else { crate::branches::folders(&dir) };
            tokio::task::spawn_blocking(named).await.unwrap_or_default()
        };
        let made = dir.join(".git");
        let is = !named.is_empty();
        // Made while its folders were being named: named again.
        if !is && made.is_dir() {
            continue;
        }
        if std::mem::take(&mut first) && is {
            owed = Some(Heard::Began);
        }
        let changes = match is {
            // A worktree of another repository, whose `.git` is a file that says where that is.
            false if made.exists() || !dir.is_dir() => None,
            false => Some(watch.files(&[made.as_path()])),
            true if !whole => None,
            true => Some(watch.within(&named.iter().map(PathBuf::as_path).collect::<Vec<_>>())),
        };
        if let Some(said) = owed.take() {
            heard(said).await;
        }
        let Some(mut changes) = changes else { return };
        changes.next().await;
        owed = Some(if is { Heard::Changed } else { Heard::Made });
    }
}
