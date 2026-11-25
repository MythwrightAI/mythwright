//! What a blueprint reaches, held to this device; and the last text of
//! each name that checked.
//!
//! A blueprint is a text, and it names others: a `call`, a `spawn`, a
//! `uses`, a file a step links. Each of those is another text, read where
//! it is when the step runs, and it may have been written for another
//! build of this program than the one reading it.
//!
//! **Asked for, everything is checked** ([`Texts::check_reaching`]). When
//! someone applies a blueprint, every blueprint it names is read and
//! checked the same way, and those they name, each text once. What is
//! wrong in one of them is said against the step that reaches it, with
//! the way there. A name that is only known when the step runs
//! (`call: "${ ... }"`) is left to that moment, and one that is not found
//! is the warning it always was: what brings it may not be here yet.
//!
//! **The last text that checked is kept**, of those that ship
//! ([`Texts::remember`], [`Texts::last_checked`]), for its name as its
//! project sees it, in the journal's small durable state, written when it
//! differs from what is there. What a step does with it when the text now
//! there does not check is the runner's (`conductor`'s `reach`).

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use blueprint::{Blueprint, Checked, Diagnostic, Flow, Span, StepKind, template};
use kernel::{Error, Result};
use serde_json::{Map, Value, json};

use crate::{Runner, Texts};

/// Where the last text of each blueprint that checked is kept: of those
/// that ship. A project's own is stood in for only by what was vouched
/// for ([`crate::vouched`]), so nothing kept here before that was so is
/// read: the name is another.
pub(crate) const NAMESPACE: &str = runs::beside::FITS;

/// What a check says of a text of a project's own that is not the one used.
pub const PASSED_OVER: &str = "own-not-used";

/// What a check says of a name that has more than one file
/// ([`crate::library::named`]): against the step that reaches it, where
/// nothing can be started by that name, and of the text in hand, which is
/// one of them.
const SEVERAL: &str = "several-files";

/// What a text is found with when it uses something this build has not
/// got: the likeliest reason is that it was written for a later one.
const NEWER: &[&str] = &["unknown-action", "unknown-argument"];

/// The revision last remembered for each name, so that a call whose text
/// has not changed asks the database nothing.
#[derive(Default)]
pub(crate) struct Known {
    known: Mutex<HashMap<String, String>>,
}

/// What a place names: a blueprint, which is called, spawned or
/// attached; or a file a step links, which is words and is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Blueprint,
    Linked,
}

/// One place where a blueprint names another text of the library.
struct Place<'a> {
    /// The step that does it; `None` for one attached with `uses`.
    step: Option<&'a str>,
    verb: &'static str,
    kind: Kind,
    /// What the text is called in the library. For a blueprint, as it is
    /// written; for a file a step links, the file's name among the
    /// `prompts/` beside the blueprint that links it.
    name: Cow<'a, str>,
    /// What it is handed: a blueprint's params, or what fills a file's
    /// blanks.
    with: &'a Value,
    span: Span,
}

impl Place<'_> {
    /// How it is said of the text being checked.
    fn first(&self) -> String {
        match self.step {
            Some(_) => format!("this step {} `{}`", self.verb, self.name),
            None => format!("this {} `{}`", self.verb, self.name),
        }
    }

    /// How it is said of a text on the way to the one at fault.
    fn then(&self) -> String {
        match self.step {
            Some(step) => format!(", whose step `{step}` {} `{}`", self.verb, self.name),
            None => format!(", which {} `{}`", self.verb, self.name),
        }
    }
}

fn places(blueprint: &Blueprint) -> Vec<Place<'_>> {
    let mut found: Vec<Place<'_>> = blueprint
        .uses
        .iter()
        .map(|attached| Place {
            step: None,
            verb: "attaches",
            kind: Kind::Blueprint,
            name: Cow::Borrowed(&attached.blueprint),
            with: &attached.with,
            span: attached.span,
        })
        .collect();
    for (_, flow) in blueprint.flows() {
        named(flow, &mut found);
    }
    // What its steps link: each a text of the library too, by the name it
    // has there, so that whatever asks what this text reaches is told of
    // its words with the texts it calls.
    for link in blueprint::parts::linked(blueprint) {
        found.push(Place {
            step: Some(link.step),
            verb: "links",
            kind: Kind::Linked,
            name: Cow::Owned(blueprint::parts::linked_as(&blueprint.name, link.from)),
            with: link.with,
            span: link.span,
        });
    }
    found
}

fn named<'a>(flow: &'a Flow, found: &mut Vec<Place<'a>>) {
    for step in &flow.steps {
        let verb = match &step.kind {
            StepKind::Call(_) => "calls",
            StepKind::Spawn(_) => "spawns",
            StepKind::Task => {
                named(&step.inner, found);
                continue;
            }
            StepKind::Do(_) => continue,
        };
        found.push(Place {
            step: Some(&step.id),
            verb,
            kind: Kind::Blueprint,
            name: Cow::Borrowed(step.kind.target()),
            with: &step.with,
            span: step.span,
        });
    }
}

/// What is wrong with something reached: the way from the first text
/// reached to the one at fault, and what is wrong with that one.
#[derive(Clone)]
struct Fault {
    through: String,
    what: String,
}

/// What was found of a text the first time it was reached.
#[derive(Clone, Default)]
struct Seen {
    fault: Option<Fault>,
    /// The text as a blueprint, when it checks: what it is given is held
    /// to its `params`.
    blueprint: Option<Arc<Blueprint>>,
}

struct Walk<'a> {
    texts: &'a Texts,
    runner: &'a dyn Runner,
    project: Option<&'a str>,
    seen: HashMap<String, Seen>,
    /// Every text reached that checks, by name.
    fits: Vec<(String, String)>,
    /// Texts read in place of the library's: what the instance being
    /// applied over reached, where it is to go on with it.
    over: &'a BTreeMap<String, String>,
}

impl Walk<'_> {
    fn fault(&mut self, place: &Place<'_>) -> Option<Fault> {
        if template::has_holes(&place.name) {
            return None;
        }
        if place.kind == Kind::Linked {
            return self.linked(place);
        }
        let seen = self.look(&place.name);
        if seen.fault.is_some() {
            return seen.fault;
        }
        let callee = seen.blueprint?;
        let what = given(&callee, place.with)?;
        Some(Fault { through: String::new(), what: format!(", which {what}") })
    }

    /// What is wrong with a file a step links, where anything is: it is
    /// not there, or it leaves a blank the step hands nothing for, or one
    /// that works something out. One that fits is among what was reached.
    fn linked(&mut self, place: &Place<'_>) -> Option<Fault> {
        let name: &str = &place.name;
        let handed: Vec<&str> =
            place.with.as_object().map(|with| with.keys().map(String::as_str).collect()).unwrap_or_default();
        let there = || self.texts.library.load(name, self.project);
        // Words the instance applied over is held to are gone on with
        // where this still fills their blanks. Where it does not, the
        // ones there now are tried: that is what a text that can no
        // longer be run comes to for words.
        let kept = self.over.get(name).filter(|kept| blueprint::parts::unfit(kept, &handed).is_none());
        let Some(text) = kept.cloned().or_else(there) else {
            // What to do about it is said with it: whoever copies a
            // blueprint that ships to change it meets this first.
            let folder = blueprint::parts::FOLDER;
            let ships = name
                .split_once(&format!("{folder}/"))
                .map(|(_, from)| format!("std/{folder}/{from}"))
                .filter(|ships| ships != name && self.texts.library.load(ships, None).is_some());
            let kept_in = crate::library::folder_of(self.texts.library.as_ref(), self.project);
            let to_do = match (ships, kept_in.path()) {
                (Some(ships), Some(path)) => format!(
                    "A file of that name ships (`{ships}`): keep this blueprint under `{path}/std/` and it is \
                     told what ships, or copy that file to `{path}/{name}` in the project"
                ),
                _ => crate::library::folders::write_as(&kept_in, name, self.project),
            };
            let what = format!(
                ", which is not there: it is a file of `{folder}/` beside the blueprint that links it. {to_do}"
            );
            return Some(Fault { through: String::new(), what });
        };
        if let Some(why) = blueprint::parts::unfit(&text, &handed) {
            return Some(Fault { through: String::new(), what: format!(": {why}") });
        }
        if !self.fits.iter().any(|(known, _)| known == name) {
            self.fits.push((name.to_string(), text));
        }
        None
    }

    fn look(&mut self, name: &str) -> Seen {
        if let Some(seen) = self.seen.get(name) {
            return seen.clone();
        }
        // Reached again from below, it has been looked at: once is enough,
        // and a blueprint may reach itself.
        self.seen.insert(name.to_string(), Seen::default());
        let there = || self.texts.library.load(name, self.project);
        let Some(text) = self.over.get(name).cloned().or_else(there) else {
            return Seen::default();
        };
        let checked = self.runner.check_own(&text, self.project);
        let seen = match checked.blueprint {
            Some(blueprint) if !checked.diagnostics.iter().any(Diagnostic::is_error) => {
                let fault = places(&blueprint).iter().find_map(|place| {
                    let below = self.fault(place)?;
                    Some(Fault { through: format!("{}{}", place.then(), below.through), what: below.what })
                });
                self.fits.push((name.to_string(), text));
                Seen { fault, blueprint: Some(Arc::new(blueprint)) }
            }
            _ => {
                let errors: Vec<&Diagnostic> = checked.errors().collect();
                let what = format!(
                    ", which has {} problem(s): {}{}",
                    errors.len(),
                    said(&self.texts.file_of(name, self.project), &errors).join("; "),
                    newer(name, &errors, ". ")
                );
                Seen { fault: Some(Fault { through: String::new(), what }), blueprint: None }
            }
        };
        self.seen.insert(name.to_string(), seen.clone());
        seen
    }
}

/// What is wrong with what a step gives the blueprint it names, where that
/// can be told without running it: the same as the blueprint will say when
/// it is applied with them.
fn given(callee: &Blueprint, with: &Value) -> Option<String> {
    let none = Map::new();
    let given = match with {
        Value::Object(given) => given,
        Value::Null => &none,
        // `with: ${ ... }` is filled in when the step runs.
        _ => return None,
    };
    // What `...` hands on is known when the step runs, and the blueprint
    // says then what it has no param for: what is written beside it is
    // held to the params now, and what is needed may be among the rest.
    let handed_on = given.contains_key(template::HANDED_ON);
    let written = || given.keys().filter(|key| *key != template::HANDED_ON);
    if let Some(unknown) = written().find(|key| !callee.params.iter().any(|param| &param.name == *key)) {
        let known: Vec<&str> = callee.params.iter().map(|param| param.name.as_str()).collect();
        let takes = if known.is_empty() {
            "it takes no params".to_string()
        } else {
            format!("it takes: {}", known.join(", "))
        };
        return Some(format!("has no param `{unknown}`; {takes}"));
    }
    callee
        .params
        .iter()
        .filter(|_| !handed_on)
        .find(|param| param.required && param.default.is_none() && !given.contains_key(&param.name))
        .map(|param| format!("needs param `{}`, which is not given here", param.name))
}

/// Each error as it is said of the file it is in.
fn said(file: &str, errors: &[&Diagnostic]) -> Vec<String> {
    errors.iter().map(|error| error.render(file)).collect()
}

/// What is added when a text uses something this build has not got.
fn newer(name: &str, errors: &[&Diagnostic], before: &str) -> String {
    if !errors.iter().any(|error| NEWER.contains(&error.code)) {
        return String::new();
    }
    format!(
        "{before}`{name}` may have been written for a newer version than this one, which is {}",
        brand::VERSION
    )
}

/// What is wrong with the text of `name`, each thing said of `file`, which
/// is what its file is called ([`Texts::file_of`]).
pub fn unfit(name: &str, file: &str, diagnostics: &[Diagnostic], hint: bool) -> Error {
    let errors: Vec<&Diagnostic> = diagnostics.iter().filter(|diagnostic| diagnostic.is_error()).collect();
    let hint = if hint { newer(name, &errors, "\n") } else { String::new() };
    Error::invalid(format!(
        "`{name}` has {} problem(s):\n{}{hint}",
        errors.len(),
        said(file, &errors).join("\n")
    ))
}

fn key(name: &str, project: Option<&str>) -> String {
    // A line break is in neither a project's name nor a blueprint's.
    format!("{}\n{name}", project.unwrap_or_default())
}

impl Texts {
    /// What the file of the blueprint `name` is called, as far as anything
    /// knows: the file the library reads it from, and otherwise what such
    /// a file is usually called. For saying where in it something is.
    pub fn file_of(&self, name: &str, project: Option<&str>) -> String {
        self.library.called(name, project).unwrap_or_else(|| brand::blueprint_file(name))
    }

    /// The text and what it reaches, and every text reached that checks.
    /// `over` is read in place of the library where it has a name.
    pub fn check_reaching(
        &self,
        runner: &dyn Runner,
        source: &str,
        project: Option<&str>,
        over: &BTreeMap<String, String>,
    ) -> (Checked, Vec<(String, String)>) {
        let mut checked = runner.check_own(source, project);
        let Some(blueprint) = &checked.blueprint else {
            return (checked, Vec::new());
        };
        // The text in hand is the one being checked, whatever the library
        // has under its name.
        let seen = HashMap::from([(blueprint.name.clone(), Seen::default())]);
        let mut walk = Walk { texts: self, runner, project, seen, fits: Vec::new(), over };
        let mut found = Vec::new();
        for place in places(blueprint) {
            if let Some(fault) = walk.fault(&place) {
                let said = format!("{}{}{}", place.first(), fault.through, fault.what);
                found.push(Diagnostic::error("reaches-unfit", said, place.span));
            }
        }
        // A text of the project's own that is not the one used, for this
        // name or for one it reaches, is said where it would have counted:
        // whoever checks or applies otherwise reads one text and runs
        // another, or runs the one they expected and wonders at the file.
        let library = &self.library;
        if let Some(said) = library.passed_over(&blueprint.name, project) {
            found.push(Diagnostic::warning(PASSED_OVER, said, blueprint.span));
        }
        for place in places(blueprint) {
            if !template::has_holes(&place.name)
                && let Some(said) = library.passed_over(&place.name, project)
            {
                found.push(Diagnostic::warning(PASSED_OVER, said, place.span));
            }
        }
        // A name with more than one file is read from neither. Reached from
        // here, that is what is wrong with the step that reaches it; of the
        // text in hand it is said and no more, since this is one of them
        // and whoever checks it may be about to take the other away.
        if let Some(said) = library.refused(&blueprint.name, project) {
            found.push(Diagnostic::warning(SEVERAL, said, blueprint.span));
        }
        for place in places(blueprint) {
            if !template::has_holes(&place.name)
                && let Some(said) = library.refused(&place.name, project)
            {
                found.push(Diagnostic::error(SEVERAL, said, place.span));
            }
        }
        checked.diagnostics.extend(found);
        checked.diagnostics.sort_by_key(|d| (d.span.start.line, d.span.start.col, !d.is_error()));
        (checked, walk.fits)
    }

    /// A text that is already an instance's: checked alone, as what it was
    /// when it was applied.
    ///
    /// What a handler hears is not held against it. It was checked when
    /// it was applied; if the events have changed under it since, it has
    /// a handler that does not fire, which is no reason to stop what is
    /// running, where a step whose action is gone cannot be run at all.
    /// For that, what is said points at why: what was taken away and when,
    /// where the list kept with the contract says (`broken`).
    pub fn compile(
        &self,
        runner: &dyn Runner,
        name: &str,
        source: &str,
        project: Option<&str>,
    ) -> Result<Arc<Blueprint>> {
        let mut checked = runner.check_own(source, project);
        checked.diagnostics.retain(|diagnostic| !blueprint::HEARD.contains(&diagnostic.code));
        checked.into_result().map(Arc::new).map_err(|diagnostics| {
            let errors: Vec<&Diagnostic> = diagnostics.iter().filter(|one| one.is_error()).collect();
            let why = crate::broken::why(&errors);
            let unfit = unfit(name, &self.file_of(name, project), &diagnostics, false);
            match why.is_empty() {
                true => unfit,
                false => Error::invalid(format!("{}\n{}", unfit.message, why.join("\n"))),
            }
        })
    }

    /// A text an instance is to be given: checked alone and in full, what
    /// its handlers hear included.
    pub fn compile_new(
        &self,
        runner: &dyn Runner,
        name: &str,
        source: &str,
        project: Option<&str>,
    ) -> Result<Arc<Blueprint>> {
        let checked = runner.check_own(source, project);
        checked
            .into_result()
            .map(Arc::new)
            .map_err(|diagnostics| unfit(name, &self.file_of(name, project), &diagnostics, false))
    }

    /// Keeps `text` as the last of `name` that checked, as `project` sees
    /// it. Nothing is written when it is what is kept already. Not being
    /// able to write it down is no reason to stop the call it was read for.
    pub async fn remember(&self, name: &str, project: Option<&str>, text: &str) {
        let (key, revision) = (key(name, project), blueprint::revision(text));
        if self.known.known.lock().expect("kept lock").get(&key) == Some(&revision) {
            return;
        }
        let journal = &self.own;
        let stored = journal.kv_get(NAMESPACE, &key).await.ok().flatten();
        if stored.as_ref().and_then(|stored| stored["revision"].as_str()) != Some(revision.as_str()) {
            let _ = journal.kv_set(NAMESPACE, &key, &json!({ "revision": revision, "source": text })).await;
        }
        self.known.known.lock().expect("kept lock").insert(key, revision);
    }

    /// The last text of `name` that checked, as `project` sees it, where
    /// one is kept: what a step goes on with when the one now there does
    /// not check.
    pub async fn last_checked(&self, name: &str, project: Option<&str>) -> Option<String> {
        let kept = self.own.kv_get(NAMESPACE, &key(name, project)).await.ok().flatten();
        kept.as_ref().and_then(|kept| kept["source"].as_str()).map(str::to_string)
    }
}
