//! What a package says its program needs behind a wall, and what a turn
//! behind one is started with.
//!
//! What an agent's wall is made of, and what it is shown there, is the
//! agents' (`agents::walled`, where these are offered under the names
//! they always had); [`hosting::wall`] holds a wall and knows nothing of
//! agents. This is the part of it that is a package's word or a turn's
//! plain values: what a manifest says under `agent.wall` ([`Needs`]),
//! what a turn is started with ([`Behind`]), how a slot's folders and its
//! `wall` are said, and what is added to the sentence of a turn that
//! failed behind one.

use std::path::{Component, Path, PathBuf};

use hosting::repository::Git;
use hosting::{Shown, Walls};
use kernel::{Error, Result};
use serde_json::Value;

use crate::extension::Check;

/// What a package says its program needs behind a wall (`agent.wall`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Needs {
    /// Folders of the program's own, for one that is a file among several:
    /// written from where its file is, links followed (`../..`).
    pub beside: Vec<String>,
    /// What it keeps in the home of whoever runs it, written from there:
    /// its sign-in and its conversations. Shown whole only to an agent
    /// whose sign-in cannot be told from the rest (`signin` says none);
    /// an agent that draws a sign-in the factory holds is shown none of
    /// it.
    pub keeps: Vec<String>,
    /// Of what it keeps, what is its sign-in and nothing it was asked or
    /// answered, written the same way: one of `keeps`, or a folder or a
    /// file within one. In every home of an agent's own (its ordinary
    /// one, and each kept channel's) this is shown where it is, the one
    /// place for every home, and the rest of what it keeps is that home's
    /// own. A package that says none has a program whose sign-in cannot
    /// be told from its conversations, and an agent on it that signs in
    /// as whoever runs the daemon has no home of its own and is in no
    /// kept channel (`agents::kept::Theirs`).
    pub signin: Vec<String>,
    /// The file its program reads as its own standing instructions,
    /// written from its home (`.tool/INSTRUCTIONS.md`): where the factory
    /// writes what the agent is and how it stands before each turn, in a
    /// home that is the program's own (`Behind::instructions`), so that
    /// what it is asked is all a message carries. A package that says
    /// none has agents told all of it ahead of every prompt.
    pub instructions: Option<String>,
}

const FIELDS: &[&str] = &["beside", "keeps", "signin", "instructions"];

impl Needs {
    /// Reads `agent.wall`, saying what is wrong with it.
    pub fn read(check: &mut Check, wall: &Value) -> Needs {
        let Some(map) = wall.as_object() else {
            check.problem("agent.wall", "must be a map of `beside`, `keeps`, `signin` and `instructions`");
            return Needs::default();
        };
        for key in map.keys().filter(|key| !FIELDS.contains(&key.as_str())) {
            check.problem("agent.wall", format!("has no `{key}`; there are: {}", FIELDS.join(", ")));
        }
        let mut list = |name: &str, fits: fn(&Path) -> bool, means: &str| -> Vec<String> {
            let at = format!("agent.wall.{name}");
            let Some(said) = map.get(name) else { return Vec::new() };
            let Some(items) = said.as_array() else {
                check.problem(&at, "must be a list");
                return Vec::new();
            };
            let mut found = Vec::new();
            for item in items {
                match item.as_str().filter(|path| !path.is_empty() && fits(Path::new(path))) {
                    Some(path) => found.push(path.to_string()),
                    None => {
                        check.problem(&at, format!("`{}` is not {means}", blueprint::expr::display(item)))
                    }
                }
            }
            found
        };
        let up_and_down = |path: &Path| {
            path.components().all(|part| matches!(part, Component::Normal(_) | Component::ParentDir))
        };
        let down = |path: &Path| path.components().all(|part| matches!(part, Component::Normal(_)));
        let from_home =
            "a folder or file written from the home of whoever runs the program, such as `.config/tool`";
        let beside =
            list("beside", up_and_down, "a folder written from where the program's file is, such as `../..`");
        let keeps = list("keeps", down, from_home);
        let signin = list("signin", down, from_home);
        let instructions = match map.get("instructions") {
            None | Some(Value::Null) => None,
            Some(said) => match said.as_str().filter(|path| !path.is_empty() && down(Path::new(path))) {
                Some(path) => Some(path.to_string()),
                None => {
                    check.problem(
                        "agent.wall.instructions",
                        format!(
                            "`{}` is not a file written from the home of whoever runs the program, such as \
                             `.tool/INSTRUCTIONS.md`",
                            blueprint::expr::display(said)
                        ),
                    );
                    None
                }
            },
        };
        let needs = Needs { beside, keeps, signin, instructions };
        // A sign-in is of what the program keeps: shown where that is.
        let kept = |path: &str| needs.keeps.iter().any(|keeps| Path::new(path).starts_with(keeps));
        for stray in needs.signin.iter().filter(|path| !kept(path)) {
            check.problem(
                "agent.wall.signin",
                format!("`{stray}` is not one of `keeps`, nor within one: the sign-in is of what the program keeps"),
            );
        }
        // What is written for each turn is no sign-in: one that was would
        // be shown from the home of whoever runs the daemon, where nothing
        // is written for an agent.
        if let Some(file) = &needs.instructions
            && needs.signin.iter().any(|signin| Path::new(file).starts_with(signin))
        {
            check.problem(
                "agent.wall.instructions",
                format!(
                    "`{file}` is of the sign-in (`signin`): what is written for each turn is the home's own"
                ),
            );
        }
        needs
    }
}

/// What a turn behind a wall is started with: how walls are held here,
/// what the program is shown, and what its package says it needs.
#[derive(Clone)]
pub struct Behind {
    pub walls: Walls,
    pub shown: Shown,
    /// Folders of the program's own, from where its file is.
    pub beside: Vec<String>,
    /// Variables of the daemon's own environment the program keeps besides
    /// those every program behind a wall does: the ones its package says it
    /// would sign in by, for one that signs in as whoever runs the daemon.
    pub inherits: Vec<String>,
    /// Where its home is, as it is told: where the home of whoever runs
    /// the daemon is.
    pub home: Option<PathBuf>,
    /// What it may do with its project's repository ([`hosting::repository`]).
    pub git: Git,
    /// Variables it is started with besides: who its commits are by,
    /// where it may make any.
    pub sets: Vec<(String, String)>,
    /// Where the file its program reads as its own standing instructions
    /// is, in the home of its own it is shown, for a turn where that file
    /// is the home's own: what it is and how it stands is written there
    /// before the turn (`Agents::briefed`). `None` where its package names
    /// no such file, or where what is there is not the factory's to write
    /// (the home of whoever runs the daemon is shown there, or a folder
    /// its program is pointed at in place of its home).
    pub instructions: Option<PathBuf>,
    /// Whether what its program keeps of what it is asked is kept in the
    /// home of its own it is shown, and not in what is shown there from
    /// the home of whoever runs the daemon: what it remembers of a
    /// conversation is carried on only in a home of the same kind.
    pub own_home: bool,
}

/// Where something a slot was told of is: written from the top, or from
/// the home of whoever runs the daemon (`~`).
pub fn place(written: &str, home: Option<&Path>) -> Option<PathBuf> {
    match written.strip_prefix('~') {
        Some(rest) => Some(home?.join(rest.trim_start_matches('/'))),
        None => Some(PathBuf::from(written)),
    }
}

/// What a slot is told it may read or change, as it is kept: each written
/// from the top or from `~`, and none that climbs.
pub fn folders(said: &[String], what: &str) -> Result<Vec<String>> {
    let mut kept: Vec<String> = Vec::new();
    for written in said.iter().map(|written| written.trim()).filter(|written| !written.is_empty()) {
        let from_home = written == "~" || written.starts_with("~/");
        let climbs = Path::new(written).components().any(|part| matches!(part, Component::ParentDir));
        if !(from_home || written.starts_with('/')) || climbs {
            return Err(Error::invalid(format!(
                "`{written}` cannot be among what an agent {what} behind its wall: a folder or a file is written from the top (`/opt/tools`) or from the home of whoever runs the daemon (`~/.cargo`), with no `..` in it"
            )));
        }
        if !kept.iter().any(|known| known == written) {
            kept.push(written.to_string());
        }
    }
    Ok(kept)
}

/// Whether a wall was asked for, as it is said in a blueprint, a row or a
/// request: `on` or `off`, true or false. Nothing, or the empty text, says
/// nothing of it.
pub fn said(wall: Option<&Value>) -> Result<Option<bool>> {
    match wall {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(wall)) => Ok(Some(*wall)),
        Some(Value::String(wall)) if wall.is_empty() => Ok(None),
        Some(Value::String(wall)) if wall == "on" => Ok(Some(true)),
        Some(Value::String(wall)) if wall == "off" => Ok(Some(false)),
        Some(other) => Err(Error::invalid(format!(
            "an agent's `wall` is `on` or `off`, and not `{}`",
            blueprint::expr::display(other)
        ))),
    }
}

/// What is added to a failed turn's sentence when its program ran behind
/// a wall and ended by itself: the wall may be why, and whoever reads the
/// failure cannot see that from what the program said.
pub fn hint(message: String, walled: bool, code: Option<i32>) -> String {
    if !walled || code.is_none() {
        return message;
    }
    format!(
        "{message} (Its program works behind a wall: it is shown its work, a home of its own, and what its \
         agent was told it may read and change. If something it needs was not there, `{} help wall` says how \
         to show it.)",
        brand::CLI
    )
}
