//! What ships, as a running tree reached it.
//!
//! A blueprint names others, and each is read by name when the step that
//! names it runs. What ships changes when the program does: a new program
//! has new texts under the old names. An instance keeps the text it was
//! applied with, so it went on as it was; but what it then *started* by
//! name was read from what was there now, and a text written for one
//! release found itself calling another's of the next. Nobody had asked
//! for that, and nothing held the two to fitting each other.
//!
//! So what an instance reaches is kept with it, as a lock file is kept
//! with a build:
//!
//! - **When somebody applies a blueprint, every text that ships among
//!   those it reaches is pinned for that instance**, as it stood then: by
//!   its name, the revision ([`Texts::pin`]). The texts themselves are
//!   kept once each, by revision, so that one the program no longer has
//!   can still be read.
//! - **A step is given the pinned text** ([`Texts::pinned_above`]): of the
//!   nearest instance above it that has one for the name. A tree goes on
//!   reading what it was applied with, whatever program is running it.
//! - **The pins move when somebody says so, and not otherwise**: when they
//!   apply saying to take the texts as they now are (`take`), or when the
//!   instance itself comes to another text that ships (a text and what it
//!   names are released together, and move together). An apply that
//!   leaves them says which of them have changed since
//!   ([`Reached::left`]).
//!
//! A project's own text is not pinned here: it is run as it was vouched
//! for ([`crate::vouched`]), which is the same holding for another reason.
//! A name nobody could know when the instance was applied (one worked out
//! as the step runs) is pinned the first time it is reached.
//!
//! What is pinned is in the journal's small durable state, by instance,
//! and does not go into a copy of a factory: nothing is running in one.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Mutex;

use runs::InstanceInfo;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Runner, Texts};

/// Where what each instance reaches is kept: the revision of each name.
const NAMESPACE: &str = runs::beside::PINNED;

/// Where the texts themselves are kept, each once, by revision.
const TEXTS: &str = runs::beside::TEXTS;

/// How far up from an instance what is pinned for it is looked for.
const DEEPEST: usize = 64;

/// What one instance reaches, as it was when somebody applied it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pins {
    pub at: i64,
    /// The revision of each text that ships, by its name.
    pub texts: BTreeMap<String, String>,
    /// What it reached before it was last moved, to be put back on
    /// ([`crate::back`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Before>,
}

/// What an instance went on with before somebody moved it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Before {
    pub at: i64,
    pub texts: BTreeMap<String, String>,
    /// The revision of the text the instance itself ran then, where that
    /// is one that ships: it moved with the rest, and goes back with it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own: Option<String>,
}

/// One text that ships, as an instance went on with it and as another
/// one is: by the version each calls itself, and what moved between them
/// in words ([`blueprint::differs`]).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Moved {
    pub name: String,
    /// The version of the one it went on with, where it says one.
    pub was: Option<String>,
    /// The version of the other.
    pub now: Option<String>,
    pub changed: Vec<String>,
}

impl Moved {
    /// A blueprint is said by the version each text calls itself and by
    /// what moved in it; words a blueprint links say no version, and of
    /// them it is said that they changed and which blanks came or went.
    pub fn between(name: &str, was: Option<&str>, now: &str) -> Moved {
        if crate::library::is_linked(name) {
            let changed = was.map(|was| blueprint::parts::differences(was, now)).unwrap_or_default();
            return Moved { name: name.to_string(), was: None, now: None, changed };
        }
        Moved {
            name: name.to_string(),
            was: was.and_then(blueprint::version),
            now: blueprint::version(now),
            changed: was.map(|was| blueprint::differs::differences(was, now)).unwrap_or_default(),
        }
    }
}

/// What became of the texts an instance reaches that are other than they
/// were when it was last applied: said to whoever applies.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Reached {
    /// Taken up as they now are.
    pub taken: Vec<Moved>,
    /// Left as they were: the instance and what it starts go on with them.
    pub left: Vec<Moved>,
}

impl Reached {
    pub fn is_empty(&self) -> bool {
        self.taken.is_empty() && self.left.is_empty()
    }
}

/// What of a request to apply is read here: the name asked for, whether a
/// text was handed with it, the key and the place it is asked under, and
/// whether whoever asks said to take the texts as they now are.
pub struct Asked<'a> {
    pub blueprint: &'a str,
    pub sourced: bool,
    pub key: Option<&'a str>,
    pub parent: Option<&'a str>,
    pub project: Option<&'a str>,
    pub take: bool,
}

/// What has been read of what is kept, so that a step that starts
/// something reads the database for none of it: a revision's text never
/// changes, and what is pinned for an instance is written only here.
#[derive(Default)]
pub(crate) struct KeptTexts {
    texts: Mutex<HashMap<String, String>>,
    pins: Mutex<HashMap<String, Option<Pins>>>,
}

/// How many instances' pins are remembered before they are read again.
const REMEMBERED: usize = 10_000;

impl Texts {
    pub async fn pins(&self, instance: &str) -> Option<Pins> {
        if let Some(known) = self.pinned.pins.lock().expect("pins lock").get(instance) {
            return known.clone();
        }
        let kept = self.own.kv_get(NAMESPACE, instance).await.ok().flatten();
        let pins: Option<Pins> = kept.and_then(|kept| serde_json::from_value(kept).ok());
        let mut known = self.pinned.pins.lock().expect("pins lock");
        if known.len() >= REMEMBERED {
            known.clear();
        }
        known.insert(instance.to_string(), pins.clone());
        pins
    }

    pub async fn put_pins(&self, instance: &str, pins: Option<Pins>) {
        let kept = pins.as_ref().map_or(serde_json::Value::Null, |pins| json!(pins));
        let _ = self.own.kv_set(NAMESPACE, instance, &kept).await;
        self.pinned.pins.lock().expect("pins lock").insert(instance.to_string(), pins);
    }

    /// Keeps a text by its revision, once, with what this conductor
    /// keeps: where the trees it is kept for are.
    pub async fn keep_text(&self, revision: &str, source: &str) {
        if self.pinned.texts.lock().expect("texts lock").contains_key(revision) {
            return;
        }
        let journal = &self.own;
        if journal.kv_get(TEXTS, revision).await.ok().flatten().is_none() {
            let _ = journal.kv_set(TEXTS, revision, &json!({ "source": source })).await;
        }
        self.pinned.texts.lock().expect("texts lock").insert(revision.to_string(), source.to_string());
    }

    pub async fn text_of(&self, revision: &str) -> Option<String> {
        if let Some(known) = self.pinned.texts.lock().expect("texts lock").get(revision) {
            return Some(known.clone());
        }
        // Its own first; then the factory's, where a comparison keeps
        // the text of a way, and a program from before kept every text.
        let mut kept = self.own.kv_get(TEXTS, revision).await.ok().flatten();
        if kept.is_none() && self.project.is_some() {
            kept = self.factory.kv_get(TEXTS, revision).await.ok().flatten();
        }
        let source = kept?["source"].as_str()?.to_string();
        self.pinned.texts.lock().expect("texts lock").insert(revision.to_string(), source.clone());
        Some(source)
    }

    /// Of what any tree this keeps for is held to or can be put back on:
    /// each text of `name` not yet `looked` at, into `found` where it calls
    /// itself `version`, and its version into `others` where it says
    /// another. Whoever asks goes over every conductor's.
    pub async fn at_version(
        &self,
        name: &str,
        version: &str,
        looked: &mut BTreeSet<String>,
        found: &mut Vec<String>,
        others: &mut BTreeSet<String>,
    ) {
        for (_, kept) in self.own.kv_list(NAMESPACE).await.unwrap_or_default() {
            let Ok(pins) = serde_json::from_value::<Pins>(kept) else { continue };
            let before = pins.before.as_ref().and_then(|before| before.texts.get(name));
            for revision in pins.texts.get(name).into_iter().chain(before) {
                if !looked.insert(revision.clone()) {
                    continue;
                }
                let Some(text) = self.text_of(revision).await else { continue };
                match blueprint::version(&text) {
                    Some(said) if said == version => found.push(text),
                    Some(said) => drop(others.insert(said)),
                    None => {}
                }
            }
        }
    }

    /// Those of `reached` that a tree is held to, as `project` sees them
    /// ([`Texts::holds`]): a project's own are held another way
    /// ([`crate::vouched`]), and unless somebody said otherwise a file
    /// somebody writes where the factory runs is read where it is.
    fn shipping<'a>(
        &self,
        project: Option<&str>,
        reached: &'a [(String, String)],
    ) -> Vec<&'a (String, String)> {
        reached.iter().filter(|(name, _)| self.holds(name, project)).collect()
    }

    /// Pins `instance` to `reached`: each text that ships among them, as it
    /// is in hand. What was pinned for it before is replaced, and kept
    /// beside as what it can be put back on; `own` is the text the
    /// instance itself ran until now, where it moves with them.
    pub async fn pin(
        &self,
        instance: &str,
        project: Option<&str>,
        reached: &[(String, String)],
        own: Option<&str>,
    ) {
        let mut pins = Pins { at: self.own.now_ms(), ..Default::default() };
        for (name, source) in self.shipping(project, reached) {
            let revision = blueprint::revision(source);
            self.keep_text(&revision, source).await;
            pins.texts.insert(name.clone(), revision);
        }
        if let Some(was) = self.pins(instance).await {
            pins.before = match was.texts == pins.texts {
                true => was.before,
                false => {
                    let own = match own {
                        Some(own) => {
                            let revision = blueprint::revision(own);
                            self.keep_text(&revision, own).await;
                            Some(revision)
                        }
                        None => None,
                    };
                    Some(Before { at: was.at, texts: was.texts, own })
                }
            };
        }
        self.put_pins(instance, Some(pins)).await;
    }

    /// What an apply by somebody is checked against in place of the texts
    /// now there: everything pinned for the instance it is applied over,
    /// its own and what is above it, where that instance goes on with
    /// them. Nothing where it is new, where they said to take the texts
    /// as they now are, or where it comes itself to another text that
    /// ships (what that names comes with it).
    pub async fn kept_reach(
        &self,
        runner: &dyn Runner,
        request: &Asked<'_>,
        source: &str,
    ) -> BTreeMap<String, String> {
        let mut kept = BTreeMap::new();
        // Followed, nothing is held: an apply is checked against the texts as they are.
        if request.take || self.follows() {
            return kept;
        }
        let project = request.project;
        let key = request.key.map(str::to_string).or_else(|| {
            blueprint::check(source, &blueprint::NoCatalog).blueprint.map(|blueprint| blueprint.name)
        });
        let Some(key) = key else { return kept };
        let store = &self.store;
        let Ok(Some(existing)) = store.live_instance_by_key(request.parent, &key, project).await else {
            return kept;
        };
        if self.comes_to_another(request, &existing, source) {
            return kept;
        }
        let mut at = existing.id;
        for _ in 0..DEEPEST {
            for (name, revision) in self.pins(&at).await.map(|pins| pins.texts).unwrap_or_default() {
                if !kept.contains_key(&name)
                    && let Some(text) = self.text_of(&revision).await
                {
                    kept.insert(name, text);
                }
            }
            match runner.above(&at).await {
                Some((_, Some(parent))) => at = parent,
                _ => break,
            }
        }
        // One this program can no longer run is not gone on with. Words a
        // blueprint links are not run: whether they can still be filled
        // is asked where they are linked, of what links them
        // (`Walk::linked`).
        kept.retain(|name, text| {
            crate::library::is_linked(name) || self.compile(runner, name, text, project).is_ok()
        });
        kept
    }

    /// Whether `request` brings `existing` to another text that ships
    /// under the name it was asked for by.
    pub fn comes_to_another(&self, request: &Asked<'_>, existing: &InstanceInfo, source: &str) -> bool {
        !request.sourced
            && self.holds(request.blueprint, request.project)
            && blueprint::revision(source) != existing.revision
            && !blueprint::means_the_same(&existing.source, source)
    }

    /// What somebody's apply of `instance` does about what it reaches, and
    /// what is said of it. `reached` is what it reaches as the texts now
    /// are; `moved` is whether the pins are to move to that (they said to
    /// take them, or the instance itself comes to another text that
    /// ships), and with it the text the instance itself ran until now
    /// where that moves too. What is given back says of each text that is
    /// other than what the instance went on with: taken up, or left.
    pub async fn repin(
        &self,
        instance: &str,
        project: Option<&str>,
        reached: &[(String, String)],
        moved: Option<Option<&str>>,
    ) -> Reached {
        // Each text pinned for it that is another now, where it still ships.
        let library = &self.library;
        let mut differ = Vec::new();
        for (name, revision) in self.pins(instance).await.map(|pins| pins.texts).unwrap_or_default() {
            if let (true, Some(there)) = (self.holds(&name, project), library.load(&name, project))
                && blueprint::revision(&there) != revision
            {
                let was = self.text_of(&revision).await;
                if !was.as_deref().is_some_and(|was| crate::library::same(&name, was, &there)) {
                    differ.push(Moved::between(&name, was.as_deref(), &there));
                }
            }
        }
        match moved {
            Some(own) => {
                self.pin(instance, project, reached, own).await;
                Reached { taken: differ, left: Vec::new() }
            }
            None => Reached { taken: Vec::new(), left: differ },
        }
    }

    /// The text that ships under `name` as `instance` reached it: of the
    /// nearest of itself and what is above it that has one pinned. Its
    /// revision and the text; nothing where none of them has, or the text
    /// was not kept.
    ///
    /// The walk alone: whether texts are followed here, and which
    /// conductor keeps the instance, whoever asks has asked first.
    pub async fn pinned_above(
        &self,
        runner: &dyn Runner,
        instance: &str,
        name: &str,
    ) -> Option<(String, String)> {
        let mut at = instance.to_string();
        for _ in 0..DEEPEST {
            if let Some(revision) = self.pins(&at).await.and_then(|pins| pins.texts.get(name).cloned()) {
                let source = self.text_of(&revision).await?;
                return Some((revision, source));
            }
            match runner.above(&at).await {
                Some((_, Some(parent))) => at = parent,
                _ => return None,
            }
        }
        None
    }

    /// A text that ships, reached by a step of `instance` under a name
    /// nothing above it had pinned (one only known as the step ran): it is
    /// pinned from now, with the instance at the top of the tree, so that
    /// everything in the tree goes on with the same one.
    pub async fn pin_later(&self, runner: &dyn Runner, instance: &str, name: &str, source: &str) {
        let mut top = instance.to_string();
        for _ in 0..DEEPEST {
            match runner.above(&top).await {
                Some((_, Some(parent))) => top = parent,
                _ => break,
            }
        }
        let _one = self.vouching.lock().await;
        let mut pins = self.pins(&top).await.unwrap_or_default();
        if pins.texts.contains_key(name) {
            return;
        }
        let revision = blueprint::revision(source);
        self.keep_text(&revision, source).await;
        pins.texts.insert(name.to_string(), revision);
        self.put_pins(&top, Some(pins)).await;
    }

    /// When the conductor starts: an instance at the top of its tree that
    /// was applied before anything was pinned is pinned to the texts its
    /// project last ran (the last of each that checked, which an earlier
    /// program remembered: [`crate::reach`]), so that a tree from before
    /// goes on with what it had and not with what this program brought.
    /// What is pinned for instances that are gone is put away, and with it
    /// the texts nothing pins any more.
    ///
    /// `live` is what this conductor keeps that is alive, and `alive`
    /// everything that may still have something kept in its journal:
    /// the same, but for the factory's own, in whose journal a program
    /// from before kept what was pinned for every project's instances,
    /// until each project's conductor has brought its own over. `wanted`
    /// is what else is still wanted of the kept texts: a comparison's
    /// ways are texts kept the same way.
    pub async fn pin_resumed(
        &self,
        live: &[InstanceInfo],
        alive: &HashSet<&str>,
        mut wanted: HashSet<String>,
    ) {
        let journal = &self.own;
        for (instance, kept) in journal.kv_list(NAMESPACE).await.unwrap_or_default() {
            if !alive.contains(instance.as_str()) {
                self.put_pins(&instance, None).await;
                continue;
            }
            if let Ok(pins) = serde_json::from_value::<Pins>(kept) {
                wanted.extend(pins.texts.into_values());
                // What it can be put back on is kept as long as it can be.
                if let Some(before) = pins.before {
                    wanted.extend(before.texts.into_values());
                    wanted.extend(before.own);
                }
            }
        }
        let last = journal.kv_list(crate::reach::NAMESPACE).await.unwrap_or_default();
        for info in live.iter().filter(|info| info.parent.is_none()) {
            if self.pins(&info.id).await.is_some() {
                continue;
            }
            // As its project saw each name; a project sees its own and no other's.
            let seen = format!("{}\n", info.project.as_deref().unwrap_or_default());
            let mut pins = Pins { at: journal.now_ms(), ..Default::default() };
            for (key, kept) in &last {
                let (Some(name), Some(revision), Some(source)) =
                    (key.strip_prefix(&seen), kept["revision"].as_str(), kept["source"].as_str())
                else {
                    continue;
                };
                if !self.holds(name, info.project.as_deref()) {
                    continue;
                }
                self.keep_text(revision, source).await;
                wanted.insert(revision.to_string());
                pins.texts.insert(name.to_string(), revision.to_string());
            }
            self.put_pins(&info.id, Some(pins)).await;
        }
        for (revision, _) in journal.kv_list(TEXTS).await.unwrap_or_default() {
            if !wanted.contains(&revision) {
                let _ = journal.kv_set(TEXTS, &revision, &serde_json::Value::Null).await;
                self.pinned.texts.lock().expect("texts lock").remove(&revision);
            }
        }
    }
}
