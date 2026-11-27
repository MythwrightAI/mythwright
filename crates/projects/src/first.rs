//! A new project's first commit: the files the factory started it with.
//!
//! A project the factory makes is a repository from the start, so that a
//! card's work can be on a branch of its own. A repository with no commit
//! has nothing to cut a branch from, and its first card was worked in the
//! project's own folder with nothing kept of it; and where somebody then
//! made a first commit of their own, the starting files were still
//! untracked there, and the first branch that changed one of them could
//! not be merged over them. So the files it was started with are its first
//! commit, made as it is made.
//!
//! It is committed the way the fleet commits a card's work (`place.keep`):
//! whose work it is is said as the author (the factory, at an address
//! nobody has), and whoever git says commits in that repository commits
//! it. Where git does not know who that is, nothing is committed, and the
//! person is told what the fleet tells them, in the same words
//! (`kernel::git::TELL_WHO`). Only what the factory wrote is committed:
//! a folder that was there already is never touched here.

use std::path::Path;
use std::sync::atomic::Ordering;

use serde::Serialize;

use crate::Projects;

impl Projects {
    /// From here on, a project the factory makes a repository of has the
    /// files it was started with as its first commit. A program that runs
    /// a factory says so as it opens; a test's own node does not, and its
    /// projects are repositories with no commit.
    pub fn commit_first(&self) {
        self.commits_first.store(true, Ordering::SeqCst);
    }

    pub(crate) fn commits_first(&self) -> bool {
        self.commits_first.load(Ordering::SeqCst)
    }
}

/// What came of committing the files a new project was started with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FirstCommit {
    /// The commit, short; empty where none was made.
    pub commit: String,
    /// The files it holds, or was to hold, from the project's top.
    pub files: Vec<String>,
    /// Whether git knew who to commit as.
    pub named: bool,
    /// What came of it, as a sentence for whoever added the project.
    pub says: String,
}

/// Commits `files` of the repository in `dir`, which has no commit yet:
/// the files a project called `name` was started with.
pub(crate) fn commit(dir: &Path, name: &str, files: Vec<String>) -> FirstCommit {
    let author = format!("--author={} <{}@factory.invalid>", brand::DISPLAY, brand::CLI);
    let message = format!("{name}: the files it was started with");
    let added =
        ask(dir, &[&["add", "--"][..], &files.iter().map(String::as_str).collect::<Vec<_>>()].concat());
    let made =
        added.and_then(|()| ask(dir, &[&TOLD[..], &["commit", "--quiet", &author, "-m", &message]].concat()));
    let short = made.and_then(|()| said(dir, &["rev-parse", "--short", "HEAD"]));
    match short {
        Ok(commit) => FirstCommit {
            says: format!(
                "The files it was started with are its first commit ({commit}), so each card's work is on a \
                 branch of its own."
            ),
            commit,
            files,
            named: true,
        },
        Err(why) => {
            let named = said(dir, &[&TOLD[..], &["var", "GIT_COMMITTER_IDENT"]].concat()).is_ok();
            let says = match named {
                false => format!(
                    "Git does not know who to commit as, so the files it was started with are not committed, and a \
                     card's work is done in its folder and not on a branch of its own: {}, then commit them there.",
                    kernel::git::TELL_WHO
                ),
                true => format!(
                    "The files it was started with could not be committed ({why}), and a card's work is done in its \
                     folder and not on a branch of its own until they are: commit them there."
                ),
            };
            FirstCommit { commit: String::new(), files, named, says }
        }
    }
}

/// Git is told to commit as nobody it made up. Left to itself, git that
/// was never told who commits makes somebody up from the machine's name
/// and the account's, and says so: a first commit made unasked in
/// somebody's repository is made only as whoever git was told commits.
/// The fleet's own commits come after, in a repository its owner has had
/// in their hands (`place.keep`).
const TOLD: [&str; 2] = ["-c", "user.useConfigOnly=true"];

/// Asks git `args` in `dir`, and gives what it printed, or what it said
/// was wrong.
fn said(dir: &Path, args: &[&str]) -> Result<String, String> {
    let ran =
        kernel::git::git(dir).args(args).output().map_err(|err| format!("git could not be run: {err}"))?;
    match ran.status.success() {
        true => Ok(String::from_utf8_lossy(&ran.stdout).trim().to_string()),
        false => {
            let words = String::from_utf8_lossy(&ran.stderr);
            let words: Vec<&str> = words.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
            Err(match words.is_empty() {
                true => format!("git exited with {}", ran.status),
                false => words.join(" "),
            })
        }
    }
}

fn ask(dir: &Path, args: &[&str]) -> Result<(), String> {
    said(dir, args).map(|_| ())
}
