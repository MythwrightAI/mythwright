//! What an agent may do with its project's repository, behind its wall:
//! the word its slot says and what of the repository's own folder that
//! shows. It is the wall's, and came out of `agents`, which still offers
//! it as `agents::repository` and asks whether anything is kept from the
//! agent there.
//!
//! A repository's own folder is two things at once. It is the history of
//! the project, which an agent may well be meant to read, add to and
//! rearrange. And it is where git is told what to run: its settings and
//! its hooks, which whoever next asks git for something runs, and the
//! daemon asks with no wall around it. So how much of it an agent's
//! program may change is said of the agent, a word of four, and whatever
//! the word, what git would run stays as it is:
//!
//! - `off`: the repository is not there.
//! - `read`: it is there to read. What the agent changes is committed for
//!   it by whatever gave it the work (`std/lane-round`).
//! - `commit`: it may commit its own work. In a card's worktree that is
//!   the worktree's own state, new objects, and the branch it is on with
//!   those named beside it; the project's own branch is not its to move.
//!   In the project itself it is the branches, and nothing else.
//! - `write`: all of it, but what git would run.
//!
//! An agent that part of its project is kept from (`studio::closed`) has
//! no repository whatever the word, where the repository holds anything
//! of that part: its history would show what the wall hides.

use std::path::{Path, PathBuf};

use kernel::{Error, Result, whole};

use crate::Shown;

/// How much of its project's repository an agent's program may change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Git {
    Off,
    Read,
    Commit,
    Write,
}

const WORDS: &[(&str, Git)] =
    &[("off", Git::Off), ("read", Git::Read), ("commit", Git::Commit), ("write", Git::Write)];

impl Git {
    pub fn word(self) -> &'static str {
        WORDS.iter().find(|(_, git)| *git == self).map_or("read", |(word, _)| word)
    }

    pub fn parse(word: &str) -> Option<Git> {
        WORDS.iter().find(|(known, _)| *known == word).map(|(_, git)| *git)
    }

    /// What a slot is to have: as said; unsaid, as it had (`had`, the word
    /// of the slot that is there); and a new one reads.
    pub fn said(said: Option<&str>, had: Option<&str>) -> Result<Git> {
        match said.map(str::trim).filter(|word| !word.is_empty()) {
            Some(word) => Git::parse(word).ok_or_else(|| {
                let known: Vec<&str> = WORDS.iter().map(|(word, _)| *word).collect();
                Error::invalid(format!(
                    "`{word}` is not something an agent may do with a repository; there are: {}",
                    known.join(", ")
                ))
            }),
            None => Ok(had.and_then(Git::parse).unwrap_or(Git::Read)),
        }
    }

    /// What a slot may do in a turn: what it was told (`said`, its word),
    /// held to reading where it is held to reading everything (`reads`),
    /// and to nothing where the repository `holds` something that is kept
    /// from it.
    pub fn of(said: &str, reads: bool, holds: bool) -> Git {
        let said = Git::parse(said).unwrap_or(Git::Read);
        match (holds, reads) {
            (true, _) => Git::Off,
            (false, true) => said.min(Git::Read),
            (false, false) => said,
        }
    }

    /// What an agent is told of it, ahead of what it is asked.
    pub fn told(self) -> &'static str {
        match self {
            Git::Off => {
                "The project's repository is not shown to you: git has nothing to work with here. What you \
                 change is committed for you when you have finished."
            }
            Git::Read => {
                "A repository's own folder (`.git`) is there to read and not to change, so do not commit: what \
                 you change is committed for you when you have finished."
            }
            Git::Commit => {
                "You may commit your own work with git. The repository's settings and hooks, and branches that \
                 are not yours, are not yours to change; what you leave uncommitted is committed for you when \
                 you have finished."
            }
            Git::Write => {
                "You may use git on the repository as you need to: commit, branch, merge, tag. Its settings and \
                 hooks are not yours to change."
            }
        }
    }
}

/// The worktree's own part of the repository, from what its `.git` file
/// says, where that is one of this repository's.
fn own_part(lane: &Path, repository: &Path) -> Option<PathBuf> {
    let said = std::fs::read_to_string(lane.join(".git")).ok()?;
    let at = said.lines().find_map(|line| line.strip_prefix("gitdir:"))?.trim();
    let own = whole::whole(&lane.join(at)).ok()?;
    let among = whole::whole(repository).ok()?.join("worktrees");
    whole::same(own.parent()?, &among).then_some(own)
}

/// What git would run or go by, in a repository's own folder: shown as it
/// is, whatever else there may be changed.
fn as_it_is(shown: &mut Shown, repository: &Path) {
    for run in ["config", "hooks", "modules"] {
        shown.read(&repository.join(run));
    }
    shown.shut(&repository.join("config.worktree"));
}

/// The same, of one worktree's part: where the rest of the repository is,
/// and its own settings.
fn part_as_it_is(shown: &mut Shown, own: &Path) {
    for says in ["commondir", "gitdir"] {
        shown.read(&own.join(says));
    }
    shown.shut(&own.join("config.worktree"));
}

/// Shows `project`'s repository to a program that may do `git` with it,
/// working in the project or in `lane`, a card's worktree of it. Where
/// there is no repository nothing is shown, and whoever asks says whether
/// one may be made.
pub fn show(shown: &mut Shown, git: Git, project: &Path, lane: Option<&Path>) {
    let repository = project.join(".git");
    // What says where a worktree's repository is cannot be put another way.
    if let Some(lane) = lane {
        shown.read(&lane.join(".git"));
    }
    // One that is itself a part of another's is left to be read.
    if !repository.is_dir() {
        shown.read(&repository);
        return;
    }
    let own = lane.and_then(|lane| own_part(lane, &repository));
    match (git, lane.is_some(), own) {
        (Git::Off, _, _) => shown.hide(&repository),
        (Git::Read, _, _) | (Git::Commit, true, None) => shown.read(&repository),
        (Git::Commit, _, Some(own)) => {
            shown.read(&repository);
            shown.change(&repository.join("objects"));
            shown.change(&own);
            part_as_it_is(shown, &own);
            // The branch it is on, with those named beside it: a ref is
            // written by a file beside it being given its name.
            let head = std::fs::read_to_string(own.join("HEAD")).unwrap_or_default();
            let beside = head.trim().strip_prefix("ref: ").and_then(|branch| Path::new(branch).parent());
            if let Some(beside) = beside.filter(|beside| beside.starts_with("refs/heads")) {
                shown.change(&repository.join(beside));
                shown.change(&repository.join("logs").join(beside));
            }
        }
        (Git::Commit, false, None) => {
            shown.change(&repository);
            as_it_is(shown, &repository);
            for kept in ["worktrees", "refs/tags", "refs/remotes", "packed-refs", "info"] {
                shown.read(&repository.join(kept));
            }
        }
        (Git::Write, _, _) => {
            shown.change(&repository);
            as_it_is(shown, &repository);
            let parts = std::fs::read_dir(repository.join("worktrees")).into_iter().flatten().flatten();
            for part in parts {
                part_as_it_is(shown, &part.path());
            }
        }
    }
}
