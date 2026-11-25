//! What an environment last ran, as it is said wherever a project's
//! environments are listed (the page's *Last run*, `project environment`
//! in a terminal): one sentence, with when beside it where there is a
//! when, each door writing the time as it writes times.
//!
//! In this order: what was last loaded through Run (its deploy blueprint,
//! under `deploy-<env>`); else the latest run of anything applied under
//! the environment, said as what was applied; else, where something runs
//! under it with no run of it read, that; else that nothing ran, with the
//! command that names the blueprint it is loaded by where it names none.
//! Nothing is ever said to have not run beside something that runs.
//!
//! The runs that acted under it are listed the same way ([`runs`]), the
//! newest first, and the last run said of what was applied is the first
//! of them.
//!
//! What is read for it is bounded, as a list of runs is: the project's
//! latest [`RUNS`] runs and the instances that are alive and the latest
//! [`THINGS`] made, whose environment says which runs are under which.
//! So "never run" is said only where what was read is all there is
//! (`whole`), and otherwise what it is among.

use serde::Serialize;

use crate::project::Ran;

/// How many of a project's latest runs are read to find each
/// environment's latest, and how many of its latest instances.
pub const RUNS: usize = 200;
pub const THINGS: usize = 500;

/// A run of the project's, as much of it as is said.
#[derive(Clone, Debug, PartialEq)]
pub struct Once {
    pub id: String,
    pub instance: String,
    pub started: i64,
    /// `running`, `ok`, `failed`, `cancelled`.
    pub status: String,
}

/// An instance of the project's, as much of it as is said.
#[derive(Clone, Debug, PartialEq)]
pub struct Thing {
    pub id: String,
    pub parent: Option<String>,
    /// The environment it runs under, the empty text for none.
    pub environment: String,
    pub blueprint: String,
    pub key: String,
    /// Who applied it, by name.
    pub by_name: String,
    pub live: bool,
}

/// What was read of a project to say its environments' last runs from:
/// its latest runs, the newest first; its instances; and whether those
/// are all it has.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Read {
    pub runs: Vec<Once>,
    pub things: Vec<Thing>,
    pub whole: bool,
}

/// What is said of an environment's last run.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Last {
    /// `loaded` (through Run), `applied` (something applied under it),
    /// `running` (something runs under it with no run of it read),
    /// `unsaid` (nothing ran, and it names no blueprint it is loaded by),
    /// `never` (nothing ran).
    pub kind: &'static str,
    /// The sentence, without when.
    pub said: String,
    /// When, in milliseconds, where it is a run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

/// How a run went, in words.
pub fn how(status: Option<&str>) -> &'static str {
    match status {
        Some("running") => "running",
        Some("ok") => "went well",
        Some("failed") => "failed",
        Some("cancelled") => "was cancelled",
        _ => "not known",
    }
}

/// What `name` of `project` last ran. `loaded` is what it was last loaded
/// with through Run and how that run went, where it was; `deploy` the
/// blueprint it says it is loaded by; `read` what was read of the project,
/// nothing where whoever asks may not be told what runs.
pub fn last(
    project: &str,
    name: &str,
    deploy: Option<&str>,
    loaded: Option<(&Ran, Option<&str>)>,
    read: Option<&Read>,
) -> Last {
    if let Some((ran, status)) = loaded {
        let commit: String = ran.commit.chars().take(7).collect();
        let at = if commit.is_empty() { String::new() } else { format!(" at {commit}") };
        return Last {
            kind: "loaded",
            said: format!("{}{at}, {}, by {}", ran.branch, how(status), ran.by_name),
            at: Some(ran.at),
            run: ran.run.clone(),
            instance: Some(ran.instance.clone()),
        };
    }
    if let Some(acted) = read.and_then(|read| runs(name, read).into_iter().next()) {
        return Last {
            kind: "applied",
            said: format!("{}, {}, by {}", acted.what, how(Some(&acted.status)), acted.by_name),
            at: Some(acted.at),
            run: Some(acted.run),
            instance: Some(acted.top),
        };
    }
    let whole = read.is_some_and(|read| read.whole);
    let running =
        read.map_or(0, |read| read.things.iter().filter(|one| one.live && one.environment == name).count());
    let plain = |kind, said: String| Last { kind, said, at: None, run: None, instance: None };
    if running > 0 {
        let said = match whole {
            true => format!("{running} running under it, with no run yet"),
            false => {
                format!("{running} running under it, its last run older than the project's latest {RUNS}")
            }
        };
        return plain("running", said);
    }
    let nothing = match (read, whole) {
        (None, _) => None,
        (Some(_), true) => Some("nothing has run under it".to_string()),
        (Some(_), false) => Some(format!("nothing among the project's latest {RUNS} runs ran under it")),
    };
    match deploy {
        None => {
            let command = format!("`{} project environment {project} {name} deploy <blueprint>`", brand::CLI);
            let said = match nothing {
                Some(nothing) => {
                    format!("{nothing}, and no blueprint loads it from a branch: {command} names one")
                }
                None => format!("No blueprint loads it from a branch: {command} names one"),
            };
            plain("unsaid", said)
        }
        Some(_) => plain(
            "never",
            match (nothing, whole) {
                (Some(_), true) => "never run".to_string(),
                (Some(nothing), false) => nothing,
                (None, _) => "never loaded from a branch".to_string(),
            },
        ),
    }
}

/// How many of the runs that acted under an environment its page lists.
pub const LISTED: usize = 20;

/// A run that acted under an environment, as its page lists it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Acted {
    pub run: String,
    /// How it went: `running`, `ok`, `failed`, `cancelled`.
    pub status: String,
    /// When it began, in milliseconds.
    pub at: i64,
    /// What it is a run of, said as what was applied under the
    /// environment: the instance at the top of what is read of it that is
    /// still under it, and who applied that, by name.
    pub what: String,
    pub top: String,
    pub by_name: String,
}

/// The runs `read` holds that acted under `name`, the newest first, at
/// most [`LISTED`]: each a run of an instance under it. What applied it
/// is climbed to only as far as it is under it too: a pipeline under none
/// whose step started its child under the environment (`environment:`)
/// is said as that child, which is what acted there.
pub fn runs(name: &str, read: &Read) -> Vec<Acted> {
    let thing = |id: &str| read.things.iter().find(|thing| thing.id == id);
    let under = |id: &str| thing(id).filter(|thing| thing.environment == name);
    let mut listed = Vec::new();
    for once in &read.runs {
        let Some(mut top) = under(&once.instance) else { continue };
        while let Some(parent) = top.parent.as_deref().and_then(under) {
            top = parent;
        }
        let what = match top.key.is_empty() || top.key == top.blueprint {
            true => top.blueprint.clone(),
            false => format!("{} as {}", top.blueprint, top.key),
        };
        listed.push(Acted {
            run: once.id.clone(),
            status: once.status.clone(),
            at: once.started,
            what,
            top: top.id.clone(),
            by_name: top.by_name.clone(),
        });
        if listed.len() == LISTED {
            break;
        }
    }
    listed
}
