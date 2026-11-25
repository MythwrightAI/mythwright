//! Whether an instance is behind the text now there, and how it would
//! come to run that one.
//!
//! An instance keeps the text it was applied with. The text under its
//! blueprint's name may since have become another: a file was edited, a
//! package was updated. Nothing tells the instance, and it goes on as it
//! was, which is right: it did not ask.
//!
//! Whoever is shown an instance is shown the revision of the text it
//! runs, what that text calls its own version, and whether it is
//! *behind*: the text now there for its name, as its project sees it, is
//! not the one it runs. With that goes how it would come to run it
//! ([`Takes`]). All of it is worked out when it is asked for, from the two
//! texts, each name read once for a listing. Nothing is kept and nothing
//! looks on a timer. What is done about one that is behind is the
//! runner's (`conductor`'s `current`).

use std::collections::HashMap;

use runs::InstanceInfo;
use serde::Serialize;

use crate::reach::unfit;
use crate::{Runner, Texts};

/// An instance as it is shown.
#[derive(Clone, Debug, Serialize)]
pub struct Shown {
    #[serde(flatten)]
    pub instance: InstanceInfo,
    /// What the text it runs calls its own version, where it says.
    pub version: Option<String>,
    /// Said of a live instance when the text now there for its
    /// blueprint's name is not the one it runs.
    pub behind: Option<Behind>,
}

/// The text an instance is behind, and how it would come to run it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Behind {
    /// The revision of the text now there.
    pub revision: String,
    /// What that text calls its own version, where it says.
    pub version: Option<String>,
    pub takes: Takes,
    /// The same as a sentence, with what to do.
    pub why: String,
    /// What moved between the text it runs and the one now there, a
    /// sentence for each thing ([`blueprint::differs`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<String>,
}

/// When an instance that is behind is brought to the text now there
/// without being asked itself. `held` is whether the text is one that
/// ships: what started it then goes on with the texts it reached until
/// somebody says to take them as they now are ([`crate::pinned`]).
fn when(instance: &InstanceInfo, held: bool) -> &'static str {
    match (instance.parent.is_some(), held) {
        (true, true) => {
            "what started it is applied again taking the texts as they now are (`--take`), or it is \
             itself applied again"
        }
        (true, false) => "what started it, or it itself, is applied again",
        (false, _) => "it is applied again",
    }
}

/// How an instance that is behind comes to run the text now there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Takes {
    /// Where it is, with nothing run again: when what started it is
    /// applied again, or when it is.
    InPlace,
    /// Where it is, with what the text says under `on_update` done in
    /// place of its steps: when what started it is applied again, or when
    /// it is.
    Carried,
    /// Only by being applied again itself, which runs its steps again.
    Again,
    /// Not at all as the text stands: it does not check here.
    Unfit,
}

impl Texts {
    /// The instances as they are shown: each with the version its text
    /// says and whether it is behind the text now there.
    pub fn shown(&self, runner: &dyn Runner, instances: Vec<InstanceInfo>) -> Vec<Shown> {
        let mut there: HashMap<(Option<String>, String), Option<String>> = HashMap::new();
        let mut found: HashMap<(Option<String>, String, String), Option<Behind>> = HashMap::new();
        let mut versions: HashMap<String, Option<String>> = HashMap::new();
        instances
            .into_iter()
            .map(|instance| {
                let version = versions
                    .entry(instance.revision.clone())
                    .or_insert_with(|| blueprint::version(&instance.source))
                    .clone();
                let behind = if instance.status.is_live() {
                    let place = (instance.project.clone(), instance.blueprint.clone());
                    let text = there
                        .entry(place.clone())
                        .or_insert_with(|| self.library.load(&place.1, place.0.as_deref()));
                    match text {
                        Some(text) => found
                            .entry((place.0, place.1, instance.revision.clone()))
                            .or_insert_with(|| self.behind(runner, &instance, text))
                            .clone(),
                        // Applied from a text that is in no library, or
                        // one that has gone: there is nothing to be behind.
                        None => None,
                    }
                } else {
                    None
                };
                Shown { instance, version, behind }
            })
            .collect()
    }

    /// What is said of `instance` when `there` is the text now under its
    /// blueprint's name; nothing when that is the text it runs.
    pub fn behind(&self, runner: &dyn Runner, instance: &InstanceInfo, there: &str) -> Option<Behind> {
        let revision = blueprint::revision(there);
        // Laid out another way, or with other comments, it is the text
        // this runs: laying a blueprint out is never an update.
        if revision == instance.revision || blueprint::means_the_same(&instance.source, there) {
            return None;
        }
        let name = &instance.blueprint;
        let held = self.holds(name, instance.project.as_deref());
        let (takes, why) = match runner.check_own(there, instance.project.as_deref()).into_result() {
            Err(diagnostics) => (
                Takes::Unfit,
                format!(
                    "The text of `{name}` now there cannot be applied here, so this goes on with the one \
                     it has.\n{}",
                    unfit(name, &self.file_of(name, instance.project.as_deref()), &diagnostics, true).message
                ),
            ),
            Ok(blueprint) if blueprint.on_update.is_some() => (
                Takes::Carried,
                format!(
                    "The text of `{name}` now there says how one that is running takes it (`on_update`). \
                     It is carried over to that text where it is, with that done in place of its steps, \
                     which are not run again, when {}.",
                    when(instance, held)
                ),
            ),
            Ok(_) if begins_alike(&instance.source, there) => (
                Takes::InPlace,
                format!(
                    "The text of `{name}` now there differs from the one this runs only in what it does \
                     later: its handlers, what it does when it is stopped, what it may use. It takes that \
                     text where it is, with none of its steps run again, when {}.",
                    when(instance, held)
                ),
            ),
            Ok(_) => (
                Takes::Again,
                format!(
                    "The text of `{name}` now there begins otherwise than the one this runs: its steps, \
                     its params, or something else it says at the top. It runs that text only when it is \
                     itself applied again, which runs its steps again."
                ),
            ),
        };
        let changed = blueprint::differs::differences(&instance.source, there);
        Some(Behind { revision, version: blueprint::version(there), takes, why, changed })
    }
}

/// Whether two texts do and settle the same when they are applied.
pub fn begins_alike(one: &str, other: &str) -> bool {
    match (blueprint::outset(one), blueprint::outset(other)) {
        (Some(one), Some(other)) => one == other,
        _ => false,
    }
}
