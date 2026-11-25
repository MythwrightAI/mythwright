//! What a blueprint reaches, and the text of one that a running instance
//! goes on with.
//!
//! A blueprint is a text, and it names others: a `call`, a `spawn`, a
//! `uses`. Each of those is another text, read where it is when the step
//! runs, and it may have been written for another build of this program
//! than the one reading it: an action it uses is not here, or takes other
//! arguments. Two things follow from that, and this module is both.
//!
//! **Asked for, everything is checked.** When someone applies a blueprint,
//! every blueprint it names is read and checked the same way, and those
//! they name, each text once (the walk is `texts::reach`). What is wrong
//! in one of them refuses the apply. The alternative is what there was:
//! an apply that is accepted, and a step that fails in the middle of the
//! work, a long time afterwards.
//!
//! **Running, a project's own text is the one that was vouched for.** What
//! a step is given for a name is settled in [`crate::vouched`] before it
//! is checked here: a file of the project's is run only as somebody who
//! holds what the instance runs with applied or saved it, and nothing
//! below stands in for one.
//!
//! **Running, the last text that checked is kept**, of those that ship. An
//! instance that is running did not ask for the texts it calls to change,
//! and is not there to be told that one no longer checks. So each time a text is read for a
//! `call` or a `spawn` and checks, it is remembered, for its name as that
//! project sees it; and when the text now there does not check (or is not
//! there), the step goes on with the one remembered, and says so: an
//! event, `instance.call_kept`, of the calling instance, which also says
//! it of itself beside its status until a call of that name finds a text
//! that checks. Nothing remembered, the step fails with what is wrong.
//! Only a text that does not check is stood in for: one that checks and
//! then fails is a failure like any other. Such a step checks the one text
//! it calls and not what that reaches in turn; each of those is stood in
//! for at its own call.
//!
//! What is remembered is in the journal's small durable state, written
//! when it differs from what is there, and does not go into a copy of a
//! factory: nothing is running in one.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

use blueprint::{Blueprint, Checked};
use kernel::{Error, Result, Source};
use runs::InstanceInfo;
use serde_json::json;
pub(crate) use texts::reach::{PASSED_OVER, unfit};

use crate::engine::{Apply, Engine};
use crate::known::Known;
use crate::vouched::Held;

/// The instances that say of themselves that they went on with an
/// earlier text, and of which blueprint: what [`standing`] names.
#[derive(Default)]
pub(crate) struct Kept {
    standing: Mutex<HashSet<String>>,
}

/// How an instance begins what it says of a blueprint it went on with an
/// earlier text of. It is what the saying is known by when it is taken back.
fn begins(name: &str) -> String {
    format!("`{name}` cannot be applied here as it is now")
}

fn standing(instance: &str, name: &str) -> String {
    format!("{instance}\n{name}")
}

impl Kept {
    /// What `instance` says of itself about a text it went on with an
    /// earlier one of, if it says so: its steps settling is not what
    /// takes that back.
    pub(crate) async fn said_by(&self, engine: &Engine, instance: &str) -> Option<String> {
        let begins = format!("{instance}\n");
        if !self.standing.lock().expect("kept lock").iter().any(|one| one.starts_with(&begins)) {
            return None;
        }
        engine.inner.store.instance(instance).await.ok().flatten().and_then(|info| info.error)
    }

    /// Notes what an instance brought back after a restart still says of
    /// itself, so that it is taken back when the text checks again.
    pub(crate) fn resumed(&self, info: &InstanceInfo) {
        let Some(said) = info.error.as_deref() else { return };
        if let Some((name, _)) = said.strip_prefix('`').and_then(|said| said.split_once('`'))
            && said.starts_with(&begins(name))
        {
            self.standing.lock().expect("kept lock").insert(standing(&info.id, name));
        }
    }
}

/// A text read to be applied, and what is worked out from it alone: the
/// blueprint it is, every text it reaches that checks, and the params
/// asked as it takes them.
pub(crate) struct Read {
    pub source: String,
    pub blueprint: Arc<Blueprint>,
    pub reaches: Vec<(String, String)>,
    pub params: kernel::Fields,
}

impl Engine {
    /// The text to apply for `request` ([`Engine::text_for`]), with the
    /// params it was asked with.
    pub(crate) async fn read(
        &self,
        request: &Apply,
        caller: Option<Source>,
        within: Option<&str>,
    ) -> Result<Read> {
        let (source, blueprint, reaches) = self.text_for(request, caller, within).await?;
        let params = blueprint.resolve_params(&request.params).map_err(Error::invalid)?;
        Ok(Read { source, blueprint, reaches, params })
    }

    /// What the file of the blueprint `name` is called, as far as anything
    /// knows: the file the library reads it from, and otherwise what such
    /// a file is usually called. For saying where in it something is.
    pub(crate) fn file_of(&self, name: &str, project: Option<&str>) -> String {
        self.inner.texts.file_of(name, project)
    }

    /// Checks blueprint text against this device's actions and library, and
    /// with it everything the text reaches: what is wrong in a blueprint it
    /// calls, spawns or attaches is said against the step that does.
    pub fn check(&self, source: &str, project: Option<&str>) -> Checked {
        self.check_reaching(source, project, &BTreeMap::new()).0
    }

    /// The text alone, as it is checked whoever names it.
    pub(crate) fn check_own(&self, source: &str, project: Option<&str>) -> Checked {
        self.checked_by(project, |known| blueprint::check(source, known))
    }

    /// Whether what a compile asked of the library for another project
    /// is answered the same for `project`.
    fn answers(&self, project: Option<&str>, asked: &[crate::compiled::Asked]) -> bool {
        self.checked_by(project, |known| known.answers(asked))
    }

    /// What a text is checked against for `project`.
    fn checked_by<T>(&self, project: Option<&str>, asked: impl FnOnce(&Known) -> T) -> T {
        let brought = self.inner.registry.brought();
        let brought_events = self.inner.registry.brought_events();
        let known = Known {
            registry: &self.inner.registry,
            absent: &|name| self.inner.registry.absent(name, false),
            brought: &brought,
            library: self.inner.library.as_ref(),
            project,
            brought_events: &brought_events,
            said: std::sync::OnceLock::new(),
        };
        asked(&known)
    }

    /// The text and what it reaches, and every text reached that checks.
    /// `over` is read in place of the library where it has a name.
    pub fn check_reaching(
        &self,
        source: &str,
        project: Option<&str>,
        over: &BTreeMap<String, String>,
    ) -> (Checked, Vec<(String, String)>) {
        self.inner.texts.check_reaching(self, source, project, over)
    }

    /// What applying `source` to `project` passes over: each text of the
    /// project's own, for the blueprint or for one it reaches, that is not
    /// the one used, as the sentence a check says of it.
    pub fn passed_over(&self, source: &str, project: Option<&str>) -> Vec<String> {
        let mut said: Vec<String> = self
            .check(source, project)
            .diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.code == PASSED_OVER)
            .map(|diagnostic| diagnostic.message)
            .collect();
        said.dedup();
        said
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
    pub fn compile(&self, name: &str, source: &str, project: Option<&str>) -> Result<Arc<Blueprint>> {
        // Once for a start, whatever is alive that runs it.
        let compile = || self.inner.texts.compile(self, name, source, project);
        crate::compiled::once(false, name, source, project, compile, |asked| self.answers(project, asked))
    }

    /// A text an instance is to be given: checked alone and in full, what
    /// its handlers hear included.
    pub fn compile_new(&self, name: &str, source: &str, project: Option<&str>) -> Result<Arc<Blueprint>> {
        let compile = || self.inner.texts.compile_new(self, name, source, project);
        crate::compiled::once(true, name, source, project, compile, |asked| self.answers(project, asked))
    }

    /// The text to apply for `request` and the blueprint it is: the one
    /// asked for, or, for a step of a running instance, the last that
    /// checked when the one now there does not. What an instance does
    /// (`within`) is given a project's own text only as it was vouched for
    /// ([`crate::vouched`]).
    ///
    /// With them, for somebody asking, every text it reaches that checks,
    /// as the texts now are: what is pinned for the instance
    /// ([`crate::pinned`]). A step is given a text that ships as the tree
    /// it is of reached it, and reaches nothing anew.
    async fn text_for(
        &self,
        request: &Apply,
        caller: Option<Source>,
        within: Option<&str>,
    ) -> Result<(String, Arc<Blueprint>, Vec<(String, String)>)> {
        let (name, project) = (request.blueprint.as_str(), request.project.as_deref());
        let library = &self.inner.library;
        let mut there = request.source.clone().or_else(|| library.load(name, project));
        // A name with two files is not a name nobody has: which they are is said.
        let missing = || match library.refused(name, project) {
            Some(why) => Error::conflict(why),
            None => Error::not_found(format!("there is no blueprint called `{name}`")),
        };
        // Whether the text in hand is a file of the project's: such a one
        // is never stood in for by anything but what was vouched for.
        let mut own = request.source.is_none() && there.is_some() && library.is_own(name, project);
        let mut instead = None;
        // A step that calls or spawns is its instance doing something.
        let within = within.or(caller.as_ref().and_then(|step| step.instance.as_deref()));
        if let (None, Some(instance)) = (&request.source, within) {
            let held = self.held_to(instance, &request.principal, name, project, there.as_deref()).await;
            if let Held::Instead { source, why } = held {
                let Some(source) = source else {
                    return Err(Error::conflict(why.unwrap_or_else(|| missing().message)));
                };
                (there, own, instead) = (Some(source), true, why);
            }
        }
        // What an instance does on its own behalf with a text that ships
        // is done with the one its tree reached, as a step's is.
        let ships = request.source.is_none() && !own && instead.is_none();
        let mut pinned = false;
        if let (true, Some(instance)) = (ships, within)
            && let Some((revision, source)) = self.pinned_for(instance, name).await
        {
            let other = there.as_deref().is_none_or(|there| {
                blueprint::revision(there) != revision && !blueprint::means_the_same(&source, there)
            });
            // One this program can no longer run is not gone on with: the
            // one there now is tried, and stood in for as it always was.
            if other && self.compile(name, &source, project).is_ok() {
                (there, pinned) = (Some(source), true);
            }
        }
        let Some(caller) = caller else {
            let source = there.ok_or_else(missing)?;
            // Applied over an instance that goes on with what it reached,
            // it is held to those texts and not to the ones now there.
            let over = self.kept_reach(request, &source).await;
            let (checked, fits) = self.check_reaching(&source, project, &over);
            let blueprint = checked
                .into_result()
                .map_err(|diagnostics| unfit(name, &self.file_of(name, project), &diagnostics, true))?;
            for (reached, text) in &fits {
                if !library.is_own(reached, project) {
                    self.remember(reached, project, text).await;
                }
            }
            if let (Some(why), Some(instance)) = (instead, within) {
                let said = json!({ "blueprint": name, "key": request.key, "error": why, "kept": blueprint::revision(&source) });
                self.emit("instance.call_kept", Source::instance(instance), said).await;
            }
            return Ok((source, Arc::new(blueprint), fits));
        };
        // The last text that checked is kept with what this conductor
        // runs, for the project it runs it for.
        let journal = &self.inner.own;
        let began = journal.now_ms();
        let found = there.ok_or_else(missing).and_then(|source| {
            // What was pinned was checked when it was reached, and is held
            // now only to being runnable here.
            if pinned {
                return Ok((source.clone(), self.compile(name, &source, project)?));
            }
            let checked = self.check_own(&source, project);
            let blueprint = checked
                .into_result()
                .map_err(|diagnostics| unfit(name, &self.file_of(name, project), &diagnostics, true))?;
            Ok((source, Arc::new(blueprint)))
        });
        journal.measure(
            "blueprint.fit",
            &[("blueprint", name)],
            (journal.now_ms() - began) as f64,
            found.is_ok(),
        );
        let (source, blueprint, wrong, how) = match (found, instead) {
            (Ok((source, blueprint)), None) => {
                // A text handed over as it is, is nobody's last of that name.
                if !own && !pinned && request.source.is_none() {
                    self.remember(name, project, &source).await;
                }
                // Reached for the first time under a name nothing above
                // had pinned: the tree goes on with this one from now.
                if let (true, false, Some(instance)) = (ships, pinned, caller.instance.as_deref())
                    && self.holds(name, project)
                    && self.pinned_for(instance, name).await.is_none()
                {
                    self.pin_later(instance, name, &source).await;
                }
                self.mended(&caller, name).await;
                return Ok((source, blueprint, Vec::new()));
            }
            // Another than the one there now, as it was vouched for.
            (Ok((source, blueprint)), Some(why)) => (source, blueprint, why, "was applied"),
            // A project's own is stood in for by nothing nobody vouched for.
            (Err(wrong), _) if own => return Err(wrong),
            (Err(wrong), _) => {
                let Some(source) = self.inner.texts.last_checked(name, project).await else {
                    return Err(wrong);
                };
                // What was remembered checked on the build that remembered
                // it, which may not be this one.
                let Ok(blueprint) = self.compile(name, &source, project) else {
                    return Err(wrong);
                };
                (source, blueprint, wrong.message, "last checked")
            }
        };
        let said = json!({ "blueprint": name, "key": request.key, "error": wrong, "kept": blueprint::revision(&source) });
        if let Some(instance) = &caller.instance {
            let step = caller.step.as_deref().unwrap_or_default();
            let note = format!("{}, so `{step}` went on with it as it {how}.\n{wrong}", begins(name));
            let now = journal.now_ms();
            if self.inner.store.set_instance_note(instance, Some(note), begins(name), now).await.is_ok() {
                self.inner.kept.standing.lock().expect("kept lock").insert(standing(instance, name));
            }
        }
        self.emit("instance.call_kept", caller, said).await;
        Ok((source, blueprint, Vec::new()))
    }

    /// Takes back what the calling instance said of `name` when it went on
    /// with an earlier text of it: the one there now checks.
    async fn mended(&self, caller: &Source, name: &str) {
        if let Some(instance) = &caller.instance {
            self.mend(instance, name).await;
        }
    }

    /// Takes back what `instance` said of `name`, where it says anything.
    async fn mend(&self, instance: &str, name: &str) {
        if !self.inner.kept.standing.lock().expect("kept lock").remove(&standing(instance, name)) {
            return;
        }
        let now = self.inner.own.now_ms();
        let _ = self.inner.store.set_instance_note(instance, None, begins(name), now).await;
    }

    /// Takes back what each instance of `project` says of `name` having
    /// gone on with an earlier text of it, there and then and not at its
    /// next call: for a name whose file a project no longer reads, which
    /// its next call would find with nothing to say
    /// (`api::library::settle`).
    pub async fn mend_all(&self, project: &str, name: &str) {
        if let Ok(keeper) = self.of(Some(project)).await
            && !Arc::ptr_eq(&keeper.inner, &self.inner)
        {
            return Box::pin(keeper.mend_all(project, name)).await;
        }
        let ending = format!("\n{name}");
        let noted: Vec<String> = {
            let standing = self.inner.kept.standing.lock().expect("kept lock");
            standing.iter().filter_map(|one| one.strip_suffix(&ending).map(str::to_string)).collect()
        };
        for instance in noted {
            let whose =
                self.inner.store.instance(&instance).await.ok().flatten().and_then(|info| info.project);
            if whose.as_deref() == Some(project) {
                self.mend(&instance, name).await;
            }
        }
    }

    /// Keeps `text` as the last of `name` that checked, as `project` sees
    /// it ([`texts::Texts::remember`]).
    async fn remember(&self, name: &str, project: Option<&str>, text: &str) {
        self.inner.texts.remember(name, project, text).await
    }
}
