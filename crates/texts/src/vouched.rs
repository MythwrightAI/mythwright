//! Which of a project's own texts an instance may run.
//!
//! A blueprint runs with the authority of whoever applied it, and that is
//! said of a text: the one they applied. The texts it calls, spawns and
//! attaches are read by name when the step runs, and a project's own are
//! files in the project, which anybody who may change the project's files
//! can change: an agent among them, holding far less than the person whose
//! fleet calls the file. Read as it is at that moment, such a file would
//! be run with an authority nobody gave it: whoever may write a file would
//! hold whatever anybody who ever applied something in that project does.
//!
//! So a project's own text is run for an instance only as somebody who
//! holds what the instance runs with has answered for it:
//!
//! - **Whoever asks for an instance to be applied vouches for the project's
//!   own texts as they stand then** ([`Texts::vouch`]), all of them, for
//!   that instance and whatever is started below it. Applying is the one
//!   thing a person does to say "run this", and what is in the project at
//!   that moment is what they said it of.
//! - **Whoever saves a text through the daemon has vouched for it** where
//!   they hold at least what was vouched with ([`Texts::vouch_saved`]):
//!   they could have applied it, and chose to write it.
//! - **A step is held to that** ([`Texts::held_to`]). It is given the
//!   project's own text for a name only as the nearest instance above it
//!   has it vouched for by somebody who holds everything the step's
//!   instance runs with. A file changed since, by anything else (a merge,
//!   an agent, an editor outside the daemon), is run as it was vouched
//!   for; one taken away, the same; one that was not there is not taken
//!   up, and what ships under its name is run if anything does. Each is
//!   said (`instance.call_kept`, and beside the instance's status) with
//!   what to do: apply again.
//!
//! What ships is not a project's file. It changes when the program does,
//! which is whoever installed it, and is read where it is.
//!
//! What is vouched for is kept by instance in the journal's small durable
//! state, each with what its voucher held. A weaker asker's applying over
//! an instance adds to what is kept and takes nothing away: what a
//! stronger one vouched for goes on standing for whatever below still
//! runs with more than the weaker holds.

use std::collections::BTreeMap;

use kernel::{Principal, PrincipalKind};
use runs::InstanceInfo;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Runner, Texts};

/// Where what was vouched for is kept, by instance.
const NAMESPACE: &str = runs::beside::VOUCHED;

/// How many askers of different authority one instance keeps the word of.
/// Each is one nothing kept covers; there are few of those in a factory.
const MOST: usize = 8;

/// How far up from an instance what stands for it is looked for.
const DEEPEST: usize = 64;

/// A text as it was vouched for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Text {
    revision: String,
    source: String,
}

/// What one asker vouched for: every text of the project's own, as it stood.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Vouch {
    /// What whoever vouched held then.
    by: Vec<String>,
    who: String,
    at: i64,
    project: String,
    texts: BTreeMap<String, Text>,
}

/// What an apply did about the project's own blueprints, for whoever asked.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Vouched {
    /// Those taken up as they now are, that were otherwise before.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub taken: Vec<String>,
    /// Those that have changed and were left as they were vouched for:
    /// nobody said to take them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub left: Vec<String>,
}

impl Vouched {
    pub fn is_empty(&self) -> bool {
        self.taken.is_empty() && self.left.is_empty()
    }
}

/// What a step is given for a name.
pub enum Held {
    /// The text there now: it ships, or it is the project's own as it was
    /// vouched for.
    There,
    /// Another in its place, or none; and why, where there is something to
    /// say to whoever reads the instance.
    Instead { source: Option<String>, why: Option<String> },
}

/// Whether holding `held` is holding everything in `wanted`.
fn covers(held: &[String], wanted: &[String]) -> bool {
    let holder = Principal {
        kind: PrincipalKind::System,
        id: String::new(),
        name: String::new(),
        scopes: held.to_vec(),
        key: None,
    };
    wanted.iter().all(|scope| holder.can(scope))
}

impl Texts {
    async fn vouches(&self, instance: &str) -> Vec<Vouch> {
        let kept = self.own.kv_get(NAMESPACE, instance).await.ok().flatten();
        kept.and_then(|kept| serde_json::from_value(kept).ok()).unwrap_or_default()
    }

    async fn keep(&self, instance: &str, vouches: &[Vouch]) {
        let value = if vouches.is_empty() { serde_json::Value::Null } else { json!(vouches) };
        let _ = self.own.kv_set(NAMESPACE, instance, &value).await;
    }

    /// What stands for `instance` when it acts as `whom`: the nearest of
    /// itself and what is above it that somebody holding all of that
    /// vouched for, and the key of the instance it is kept with. With
    /// nothing found, the key of the one at the top, which is what to
    /// apply again.
    async fn standing(
        &self,
        runner: &dyn Runner,
        instance: &str,
        whom: &Principal,
    ) -> (Option<Vouch>, String) {
        let (mut at, mut top) = (instance.to_string(), String::new());
        for _ in 0..DEEPEST {
            let Some((key, parent)) = runner.above(&at).await else { break };
            let found = self.vouches(&at).await.into_iter().find(|vouch| covers(&vouch.by, &whom.scopes));
            if found.is_some() {
                return (found, key);
            }
            top = key;
            match parent {
                Some(parent) => at = parent,
                None => break,
            }
        }
        (None, top)
    }

    /// The project's own texts as they stand, by name.
    fn own_now(&self, project: &str) -> BTreeMap<String, Text> {
        let own = self.library.own(Some(project)).into_iter();
        own.map(|(name, source)| (name, Text { revision: blueprint::revision(&source), source })).collect()
    }

    /// `by` asked for `instance` to be applied: the project's own texts are
    /// vouched for, for it and what is started below it.
    ///
    /// The first time, as they stand: whoever starts something in a
    /// project takes its blueprints as they are. After that an apply takes
    /// up the text that is itself applied (`applied`), which is what was
    /// asked for, and leaves the others as they were vouched for, however
    /// they have changed since: a file an agent wrote, or a merge brought,
    /// is not run as whoever happens to apply next. `take` is that one
    /// saying to take them all as they now are, having been told which
    /// changed. What is given back is which were taken up changed, and
    /// which were left as they were.
    async fn vouch(
        &self,
        runner: &dyn Runner,
        instance: &str,
        project: Option<&str>,
        by: &Principal,
        (applied, take): (&str, bool),
    ) -> Vouched {
        let Some(project) = project else { return Vouched::default() };
        let mut texts = self.own_now(project);
        let before = self.standing(runner, instance, by).await.0;
        let mut said = Vouched::default();
        // A file under the project's folder of blueprints is also what its
        // name is without that folder before it.
        let folder = crate::library::folder_of(self.library.as_ref(), Some(project));
        let within = folder.path().map(|path| format!("{path}/"));
        fn without<'a>(within: Option<&str>, name: &'a str) -> Option<&'a str> {
            within.and_then(|within| name.strip_prefix(within))
        }
        if let Some(before) = &before {
            let asked = |name: &str| name == applied || without(within.as_deref(), name) == Some(applied);
            let names: Vec<String> = texts.keys().chain(before.texts.keys()).cloned().collect();
            for name in names {
                let (now, was) = (texts.get(&name), before.texts.get(&name));
                if now.map(|text| &text.revision) == was.map(|text| &text.revision) {
                    continue;
                }
                if take || asked(&name) {
                    said.taken.push(name);
                    continue;
                }
                match was {
                    Some(was) => texts.insert(name.clone(), was.clone()),
                    None => texts.remove(&name),
                };
                said.left.push(name);
            }
        }
        // One file may have two names; it is said once, by the shorter.
        for list in [&mut said.taken, &mut said.left] {
            list.sort();
            list.dedup();
            let all = list.clone();
            list.retain(|name| {
                let short = without(within.as_deref(), name);
                name != applied
                    && short != Some(applied)
                    && !short.is_some_and(|short| all.iter().any(|one| one == short))
            });
        }
        let vouch = Vouch {
            by: by.scopes.clone(),
            who: by.actor(),
            at: self.own.now_ms(),
            project: project.to_string(),
            texts,
        };
        self.add(instance, vouch).await;
        said
    }

    /// What an apply does about the project's own texts once its instance
    /// is known, and what is said of it to whoever asked. `asked` is
    /// whether nobody's step asked, and the instance something was done
    /// by, if it was no person either (the runner's `apply_within`); `top` is
    /// whether it has nothing above it, and `applied` the blueprint it is
    /// of, with whether the asker said to take the project's files as they
    /// now are.
    pub async fn answered(
        &self,
        runner: &dyn Runner,
        instance: &str,
        project: Option<&str>,
        (top, applied): (bool, (&str, bool)),
        by: &Principal,
        asked: (bool, Option<String>),
    ) -> Vouched {
        match asked {
            (true, None) => self.vouch(runner, instance, project, by, applied).await,
            // Below something, what stands for that stands for it.
            (true, Some(from)) if top => {
                self.vouched_as(runner, instance, &from, by).await;
                Vouched::default()
            }
            _ => Vouched::default(),
        }
    }

    /// Adds what one asker vouched for to what is kept for an instance.
    async fn add(&self, instance: &str, vouch: Vouch) {
        let _one = self.vouching.lock().await;
        let mut kept = self.vouches(instance).await;
        // What somebody holding no more vouched for earlier is replaced.
        kept.retain(|old| !covers(&vouch.by, &old.by));
        // More askers than are kept: the oldest that the newcomer's word
        // could not stand in for anyway goes. Where everything kept covers
        // the newcomer, it is the newcomer's that is not kept: what stands
        // for it is then what somebody who holds more vouched for.
        while kept.len() >= MOST {
            match kept.iter().rposition(|old| !covers(&old.by, &vouch.by)) {
                Some(oldest) => {
                    kept.remove(oldest);
                }
                None => return,
            }
        }
        kept.insert(0, vouch);
        self.keep(instance, &kept).await;
    }

    /// An instance started by something `from` did (an action that
    /// applies, and no person asking): what stands for `from` stands for
    /// it, and nothing is vouched for anew.
    async fn vouched_as(&self, runner: &dyn Runner, instance: &str, from: &str, whom: &Principal) {
        if let (Some(vouch), _) = self.standing(runner, from, whom).await {
            self.add(instance, vouch).await;
        }
    }

    /// `by` wrote, or took away, the file that is `project`'s own `name`,
    /// through the daemon: wherever what was vouched for in that project
    /// was vouched for with no more than `by` holds, the text as it is now
    /// is what is vouched for. One that does not check is not taken up, so
    /// that what is running goes on with the last that did.
    pub async fn vouch_saved(&self, runner: &dyn Runner, project: &str, name: &str, by: &Principal) {
        let library = &self.library;
        let now = match library.load(name, Some(project)) {
            Some(source) if library.is_own(name, Some(project)) => {
                // A file a blueprint links is words: it is held to its
                // blanks where it is linked, when that blueprint is
                // checked and when its step runs.
                let words = crate::library::is_linked(name);
                if !words && runner.check_own(&source, Some(project)).into_result().is_err() {
                    return;
                }
                Some(Text { revision: blueprint::revision(&source), source })
            }
            _ => None,
        };
        let _one = self.vouching.lock().await;
        for (instance, kept) in self.own.kv_list(NAMESPACE).await.unwrap_or_default() {
            let Ok(mut kept) = serde_json::from_value::<Vec<Vouch>>(kept) else { continue };
            let mut changed = false;
            for vouch in
                kept.iter_mut().filter(|vouch| vouch.project == project && covers(&by.scopes, &vouch.by))
            {
                let was = match &now {
                    Some(text) => vouch.texts.insert(name.to_string(), text.clone()),
                    None => vouch.texts.remove(name),
                };
                changed |= was != now;
            }
            if changed {
                self.keep(&instance, &kept).await;
            }
        }
    }

    /// What a step of `instance`, acting as `whom`, is given for `name`
    /// when `there` is the text now under that name.
    pub async fn held_to(
        &self,
        runner: &dyn Runner,
        instance: &str,
        whom: &Principal,
        name: &str,
        project: Option<&str>,
        there: Option<&str>,
    ) -> Held {
        let Some(project) = project else { return Held::There };
        let library = &self.library;
        let own = there.is_some() && library.is_own(name, Some(project));
        let (vouch, key) = self.standing(runner, instance, whom).await;
        let vouched = vouch.as_ref().and_then(|vouch| vouch.texts.get(name));
        let again = format!(
            "Read what is there now, and to have it run apply `{key}` again saying to take the project's \
             files as they are (`--take`): it then runs as whoever applies."
        );
        match (own, there, vouched) {
            (false, _, None) => Held::There,
            (true, Some(there), Some(text)) if text.revision == blueprint::revision(there) => Held::There,
            // Laid out another way, it is the text that was vouched for,
            // and that one is run: nothing is said, since nothing differs.
            (true, Some(there), Some(text)) if blueprint::means_the_same(&text.source, there) => {
                Held::Instead { source: Some(text.source.clone()), why: None }
            }
            (true, _, Some(text)) => Held::Instead {
                source: Some(text.source.clone()),
                why: Some(format!(
                    "`{name}` is a file of `{project}`'s own that was changed after `{key}` was applied, \
                     and is run as it was then: a running instance runs a project's file only as somebody \
                     who holds what it runs with applied or saved it. {again}"
                )),
            },
            (false, _, Some(text)) => Held::Instead {
                source: Some(text.source.clone()),
                why: Some(format!(
                    "`{name}` was a file of `{project}`'s own when `{key}` was applied and is not one \
                     now, and is run as it was then. {again}"
                )),
            },
            (true, _, None) => {
                let ships = library.shipped(name, Some(project));
                let why = match (&ships, vouch.is_some()) {
                    (Some(_), _) => format!(
                        "`{project}` has a file of its own for `{name}` that nobody who holds what `{key}` \
                         runs with has applied or saved, so the one that ships is run. {again}"
                    ),
                    (None, _) => format!(
                        "`{name}` is a file of `{project}`'s own that nobody who holds what `{key}` runs \
                         with has applied or saved, and a running instance does not take up a file on \
                         its own. {again}"
                    ),
                };
                Held::Instead { source: ships, why: Some(why) }
            }
        }
    }

    /// When the conductor starts: an instance at the top of its tree that
    /// was applied before anything was vouched for is taken to have been
    /// applied with its project's files as they stand, as whoever applied
    /// it; and what is kept for instances that are gone, with everything
    /// below them, is put away. `live` is what this conductor keeps that
    /// is alive, and `alive` everything that may still have something
    /// kept in its journal ([`Texts::pin_resumed`]).
    pub async fn vouch_resumed(&self, runner: &dyn Runner, live: &[InstanceInfo], alive: &[InstanceInfo]) {
        let mut wanted: std::collections::HashSet<String> = std::collections::HashSet::new();
        for info in alive {
            let mut at = Some(info.id.clone());
            while let Some(id) = at.take().filter(|id| wanted.insert(id.clone())) {
                at = runner.above(&id).await.and_then(|(_, parent)| parent);
            }
        }
        for (instance, _) in self.own.kv_list(NAMESPACE).await.unwrap_or_default() {
            if !wanted.contains(&instance) {
                self.keep(&instance, &[]).await;
            }
        }
        for info in live.iter().filter(|info| info.parent.is_none() && info.project.is_some()) {
            if self.vouches(&info.id).await.is_empty() {
                self.vouch(runner, &info.id, info.project.as_deref(), &info.principal, ("", true)).await;
            }
        }
    }
}
