//! One thing done several ways, side by side.
//!
//! Whoever wants to know which of two ways of doing something is better
//! has to do some of the work one way and some the other, keep each piece
//! of work to the way it began, and be able to tell afterwards which
//! measurement was of which. None of that is the work's own business, so
//! none of it is written in a blueprint: a comparison is said from
//! outside, of a blueprint by its name, and is done where that blueprint
//! is started by a step: the runner asks there what to start
//! (`conductor::Compared`), and is answered here.
//!
//! - **A comparison is of one blueprint** (`of`), in one project or in
//!   none, and has **ways**. A way may say another blueprint to start in
//!   its place, what to hand it over what the step hands, both, or
//!   neither: a way that says neither is the work as it is written.
//! - **Each thing goes one way and keeps to it.** What a start is *for*
//!   is said by one of the params it is handed (`per`), or else is its
//!   place in the tree. The first start for a thing is given the way that
//!   has had least of its share so far; every later start for it is given
//!   the same one, across restarts. Nothing is drawn at random: the same
//!   things in the same order go the same ways.
//! - **Everything started a way is measured with it**: the labels
//!   `compared` (the comparison) and `way` are added to those the child
//!   is under, and carried as labels are (the runner's `labels`). Other
//!   blueprints started for the same thing may be named to carry them too
//!   (`also`), started as they are written. What is counted is asked of
//!   the measurements by those labels; nothing here says which is better.
//! - **A way's own text is the one that was there when the comparison was
//!   started**, kept by its revision with what a running tree reaches
//!   (the runner's `pinned`): a way that changed while it was being counted
//!   would be two ways counted as one.
//! - **It ends when somebody says**, and they may say which way
//!   everything goes from then on (`goes`). That stands, unmeasured,
//!   until it is ended again saying another, or none.
//!
//! A step's instance acts for whoever applied it, and a way changes what
//! it runs and what that is handed. So a comparison is in force for a
//! step only where whoever started it held everything the step's instance
//! runs with; elsewhere the step goes as it is written, and says so.

use std::collections::{BTreeMap, BTreeSet};

use blueprint::Labels;
use conductor::{Chosen, Start, Starts};
pub use kernel::compared::{COMPARED, Compare, Comparison, Ran, TEXT, WAY, Way};
use kernel::compared::{next, word};
use kernel::{Error, Principal, Result, Source};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::Comparisons;

/// Where comparisons are kept, by name.
pub(crate) const NAMESPACE: &str = "compared";

/// Where the way each thing went is kept: `<comparison>\n<thing>`.
pub(crate) const WENT: &str = "compared.went";

/// The comparisons there are, read once and kept beside what is written.
#[derive(Default)]
pub(crate) struct Kept {
    all: std::sync::Mutex<Option<Vec<Comparison>>>,
    /// Held from finding a thing's way to writing it down.
    pub(crate) choosing: tokio::sync::Mutex<()>,
    /// Held from looking for a comparison of a name to keeping it: two
    /// asked for at once under one name are one, asked again.
    starting: tokio::sync::Mutex<()>,
    /// What each thing started under a way reads in place of a file
    /// that is linked ([`crate::instead`]), read once and kept beside
    /// what is written.
    pub(crate) reads: std::sync::Mutex<Option<std::collections::HashMap<String, crate::instead::Kept>>>,
    /// Whether events are being counted ([`crate::counted`]), and what
    /// tells whoever counts that what is counted has changed.
    pub(crate) counting: tokio::sync::Mutex<bool>,
    pub(crate) changed: tokio::sync::Notify,
    /// What has the topics counted kept in the journal where the runner
    /// would only pass them on (`conductor::Keeping`): they are read
    /// again from the place kept.
    pub(crate) keeping: std::sync::Mutex<Option<conductor::Keeping>>,
}

/// The way a thing went, when it was given it, and the text it last ran.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Went {
    pub way: String,
    #[serde(default)]
    pub at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// When it was first counted as accepted ([`kernel::compared::ACCEPTED`]):
    /// a thing is accepted once, however often that is said of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted: Option<i64>,
}

/// Whether holding `held` is holding everything in `wanted`, as a
/// credential is asked everywhere.
pub(crate) fn covers(held: &[String], wanted: &[String]) -> bool {
    wanted.iter().all(|scope| kernel::principal::covers(held, scope))
}

impl Comparisons {
    async fn compared(&self) -> Vec<Comparison> {
        if let Some(all) = self.kept.all.lock().expect("compared lock").clone() {
            return all;
        }
        let kept = self.engine.journal().kv_list(NAMESPACE).await.unwrap_or_default();
        let all: Vec<Comparison> =
            kept.into_iter().filter_map(|(_, one)| serde_json::from_value(one).ok()).collect();
        *self.kept.all.lock().expect("compared lock") = Some(all.clone());
        all
    }

    pub(crate) async fn keep_compared(&self, one: &Comparison) -> Result<()> {
        self.engine.journal().kv_set(NAMESPACE, &one.name, &json!(one)).await?;
        self.remember(one);
        Ok(())
    }

    /// Holds `one` in memory as it is now kept.
    pub(crate) fn remember(&self, one: &Comparison) {
        let mut all = self.kept.all.lock().expect("compared lock");
        if let Some(all) = all.as_mut() {
            all.retain(|kept| kept.name != one.name);
            all.push(one.clone());
            all.sort_by_key(|one| one.started);
        }
    }

    /// The revisions of the texts comparisons keep, which are not put
    /// away while a comparison says to start them.
    pub(crate) async fn compared_texts(&self) -> BTreeSet<String> {
        let all = self.compared().await;
        let ways = all.iter().flat_map(|one| one.ways.iter());
        // Its own text, and the words it reads in place of others: as they
        // were when it began, and each it ran since.
        ways.flat_map(|way| {
            let reads = way.prompts.values().filter_map(|reads| reads.revision.clone());
            let ran = way.texts.iter().flat_map(|ran| ran.prompts.values().cloned());
            way.revision.clone().into_iter().chain(reads).chain(ran)
        })
        .collect()
    }

    /// Every comparison, the oldest first; of one project where one is given.
    pub async fn comparisons(&self, project: Option<&str>) -> Vec<Comparison> {
        let mut all = self.compared().await;
        all.retain(|one| project.is_none() || one.project.as_deref() == project);
        all.sort_by_key(|one| one.started);
        all
    }

    pub async fn comparison(&self, name: &str) -> Result<Comparison> {
        let all = self.compared().await;
        all.into_iter()
            .find(|one| one.name == name)
            .ok_or_else(|| Error::not_found(format!("there is no comparison called `{name}`")))
    }

    /// Starts a comparison, as `by`: from now, what a step starts under
    /// the name `of` is started one of `ways`.
    pub async fn compare(&self, asked: Compare, by: &Principal) -> Result<Comparison> {
        // An asked comparison asked for again is answered as it is, with
        // the ways that are new added ([`Comparisons::ask_again`]).
        let _one = self.kept.starting.lock().await;
        if let Some(kept) = self.compared().await.into_iter().find(|one| one.name == asked.name) {
            return self.ask_again(kept, asked, by).await;
        }
        let one = self.begun(asked, by).await?;
        self.keep_compared(&one).await?;
        self.count_from_here().await;
        self.engine.emit("system.comparison.started", Source::default(), json!(one)).await;
        Ok(one)
    }

    /// Asks again for the asked comparison `kept`: as it is where nothing
    /// is new, and with the ways that are new added, each checked as a
    /// way is as a comparison begins, the counts that are new, and what
    /// else is started for its things that is new (`also`, `also_per`,
    /// which one that kept none takes). Adding
    /// to it is changing what it has starts run, so it is for whoever
    /// holds everything the one who started it held.
    async fn ask_again(&self, kept: Comparison, asked: Compare, by: &Principal) -> Result<Comparison> {
        if let Some(wrong) = kernel::compared::again(&kept, &asked) {
            return Err(Error::conflict(wrong));
        }
        let new: Vec<Way> = asked.ways.into_iter().filter(|way| kept.way(&way.name).is_none()).collect();
        let counts: Vec<_> = asked.counts.into_iter().filter(|count| !kept.counts.contains(count)).collect();
        let also: Vec<String> = asked.also.into_iter().filter(|also| !kept.also.contains(also)).collect();
        let also_per = asked.also_per.filter(|per| !per.is_empty() && kept.also_per.is_none());
        if new.is_empty() && counts.is_empty() && also.is_empty() && also_per.is_none() {
            return Ok(kept);
        }
        let name = &kept.name;
        if kept.ended.is_some() {
            return Err(Error::conflict(format!(
                "the comparison `{name}` has ended, and nothing is added to it; what was measured under that \
                 name is still kept, so a comparison asked anew is called something else"
            )));
        }
        if !covers(&by.scopes, &kept.held) {
            return Err(Error::denied(format!(
                "adding to `{name}` changes what its starts run, and {} started it holding more than you do; \
                 somebody who holds that adds to it",
                kept.by
            )));
        }
        // The new ways, checked as they are as a comparison begins.
        self.may_count(&counts, by)?;
        let added = match new.is_empty() {
            true => Vec::new(),
            false => {
                let ask = Compare {
                    name: name.clone(),
                    project: kept.project.clone(),
                    of: kept.of.clone(),
                    per: kept.per.clone(),
                    also: kept.also.clone(),
                    also_per: kept.also_per.clone(),
                    ways: new,
                    counts: Vec::new(),
                    asked: true,
                    rule: None,
                };
                self.begun(ask, by).await?.ways
            }
        };
        let _one = self.kept.choosing.lock().await;
        let mut one = self.comparison(&kept.name).await?;
        let names: BTreeSet<String> = one.ways.iter().map(|way| way.name.clone()).collect();
        one.ways.extend(added.into_iter().filter(|way| !names.contains(&way.name)));
        let counts: Vec<_> = counts.into_iter().filter(|count| !one.counts.contains(count)).collect();
        one.counts.extend(counts);
        for also in also {
            if !one.also.contains(&also) {
                one.also.push(also);
            }
        }
        one.also_per = one.also_per.take().or(also_per);
        self.keep_compared(&one).await?;
        self.count_from_here().await;
        self.engine.emit("system.comparison.added", Source::default(), json!(one)).await;
        Ok(one)
    }

    /// A comparison as `asked` by `by`, checked and with each way's own
    /// text kept, and not yet kept itself.
    async fn begun(&self, asked: Compare, by: &Principal) -> Result<Comparison> {
        let Compare { name, project, of, per, also, also_per, mut ways, counts, asked, rule } = asked;
        self.may_count(&counts, by)?;
        let project = project.filter(|project| !project.is_empty());
        let seen = project.as_deref();
        if !word(&name) {
            return Err(Error::invalid(format!(
                "`{name}` cannot be what a comparison is called: everything measured under it is labelled \
                 with its name, so it is a word of lowercase letters, digits, `-` and `_`, such as `models`"
            )));
        }
        let all = self.compared().await;
        // One whose ways the start names is not the one in force for its
        // blueprint, and any number of those may be.
        let in_force =
            |one: &&Comparison| !one.asked && one.in_force() && one.of == of && one.project == project;
        if let Some(other) = all.iter().filter(|one| one.name != name).find(in_force).filter(|_| !asked) {
            return Err(Error::conflict(format!(
                "`{}` already says how `{of}` is started here; end it first (`compare end {}`)",
                other.name, other.name
            )));
        }
        let library = self.engine.library();
        let Some(written) = library.load(&of, seen) else {
            return Err(Error::not_found(format!("there is no blueprint called `{of}` to compare ways of")));
        };
        if ways.is_empty() {
            return Err(Error::invalid(
                "a comparison has at least one way; a way that says nothing is the work as it is written",
            ));
        }
        // Every text the comparison's blueprints reach, for the ways that
        // read other words where one of them links a file.
        let mut reached = Vec::new();
        if ways.iter().any(|way| !way.prompts.is_empty()) {
            let started = also.iter().filter_map(|name| Some((name.clone(), library.load(name, seen)?)));
            for (name, text) in std::iter::once((of.clone(), written.clone())).chain(started) {
                reached.extend(self.engine.check_reaching(&text, seen, &BTreeMap::new()).1);
                reached.push((name, text));
            }
        }
        let mut names = BTreeSet::new();
        for way in &mut ways {
            if !word(&way.name) || !names.insert(way.name.clone()) {
                return Err(Error::invalid(format!(
                    "`{}` cannot be a way here: each way is called a word of its own, such as `a` or `mine`",
                    way.name
                )));
            }
            if way.share == 0 {
                return Err(Error::invalid(format!(
                    "the way `{}` has a share of 0, and would be given nothing; leave it out instead",
                    way.name
                )));
            }
            (way.went, way.revision) = (0, None);
            way.texts.clear();
            // What it starts, and that what it hands is something that takes.
            // One held to a version starts the text that calls itself
            // that, of the blueprint it names or of the one compared.
            if (way.version.is_some() || way.source.is_some()) && way.blueprint.is_none() {
                way.blueprint = Some(of.clone());
            }
            // One whose text was sent starts that text under the name, and
            // is held to it as one held to a version is.
            let sent = way.source.take();
            way.sent = sent.is_some();
            if way.sent && way.version.is_some() {
                return Err(Error::invalid(format!(
                    "the way `{}` both sends its text and is held to a version: say one",
                    way.name
                )));
            }
            let (started, text) = match (&way.blueprint, &way.version) {
                (Some(other), None) if sent.is_some() => (other.as_str(), sent.unwrap_or_default()),
                (Some(other), Some(version)) => {
                    let text = self.engine.text_at(other, version, seen).await;
                    (other.as_str(), text.map_err(|why| why.context(format!("the way `{}`", way.name)))?)
                }
                (Some(other), None) => {
                    let text = library.load(other, seen).ok_or_else(|| {
                        Error::not_found(format!(
                            "the way `{}` starts `{other}`, and there is none",
                            way.name
                        ))
                    })?;
                    (other.as_str(), text)
                }
                (None, _) => (of.as_str(), written.clone()),
            };
            let blueprint = self.engine.compile_new(started, &text, seen)?;
            if let Some(unknown) =
                way.with.keys().find(|key| !blueprint.params.iter().any(|p| &p.name == *key))
            {
                return Err(Error::invalid(format!(
                    "the way `{}` hands `{unknown}`, which `{started}` does not take",
                    way.name
                )));
            }
            if way.blueprint.is_some() {
                let revision = blueprint::revision(&text);
                self.engine.keep_text(&revision, &text).await;
                way.revision = Some(revision);
            }
            if !way.prompts.is_empty() {
                // What its own blueprint reaches links files too.
                let mut reached = reached.clone();
                if way.blueprint.is_some() {
                    reached.extend(self.engine.check_reaching(&text, seen, &BTreeMap::new()).1);
                    reached.push((started.to_string(), text.clone()));
                }
                self.reads_checked(way, &reached, seen).await?;
            }
        }
        let started = self.engine.journal().now_ms();
        let one = Comparison {
            name,
            project,
            of,
            per: per.filter(|per| !per.is_empty()),
            also,
            also_per: also_per.filter(|per| !per.is_empty()),
            ways,
            counts,
            by: by.actor(),
            held: by.scopes.clone(),
            started,
            ended: None,
            goes: None,
            asked,
            until: None,
            rule: None,
            rolling: None,
            shared: Vec::new(),
        };
        // A rule its shares change by is checked against what it is now.
        match rule {
            None => Ok(one),
            Some(rule) => crate::rolled::ruled(one, rule, started),
        }
    }

    /// Ends a comparison, or says again of one that is ended, which way
    /// everything goes from now on: `goes`, or as it is written.
    pub async fn end_comparison(&self, name: &str, goes: Option<String>) -> Result<Comparison> {
        let _one = self.kept.choosing.lock().await;
        let mut one = self.comparison(name).await?;
        let goes = goes.filter(|goes| !goes.is_empty());
        if goes.is_some() && one.asked {
            return Err(Error::invalid(format!(
                "each start that names `{name}` names its way, so nothing goes one way after it; end it \
                 without saying a way"
            )));
        }
        if let Some(goes) = &goes
            && one.way(goes).is_none()
        {
            let ways: Vec<&str> = one.ways.iter().map(|way| way.name.as_str()).collect();
            return Err(Error::invalid(format!(
                "`{name}` has no way called `{goes}`; its ways are {}",
                ways.join(", ")
            )));
        }
        if goes.is_some()
            && let Some(other) = self.compared().await.iter().find(|other| {
                other.name != one.name
                    && !other.asked
                    && other.in_force()
                    && other.of == one.of
                    && other.project == one.project
            })
        {
            return Err(Error::conflict(format!(
                "`{}` now says how `{}` is started here; end it first (`compare end {}`)",
                other.name, one.of, other.name
            )));
        }
        // What was said up to the moment it ends is still counted under it
        // ([`crate::counted`]): whoever counts may be behind.
        if one.ended.is_none() && !one.counts.is_empty() {
            one.until = Some(self.engine.journal().now().kept());
        }
        one.ended = one.ended.or(Some(self.engine.journal().now_ms()));
        one.goes = goes;
        self.keep_compared(&one).await?;
        // What it counted is counted no longer.
        self.count_from_here().await;
        self.engine.emit("system.comparison.ended", Source::default(), json!(one)).await;
        Ok(one)
    }

    /// What a step that starts `start.name` starts: one of a comparison's
    /// ways where a comparison says, with the labels the child is then
    /// under added to `labels`; what it names, as it is written, where no
    /// comparison is in force for it, with why where one would have been.
    pub(crate) async fn way_for(&self, start: &Start<'_>, labels: &mut Labels) -> Starts {
        let mut left = None;
        match self.way(start, labels, &mut left).await {
            Some(chosen) => Starts::Way(chosen),
            None => left.map_or(Starts::Written, Starts::Left),
        }
    }

    /// Whether `start` is made from inside a way's own text of `one`: by
    /// the instance that was started as that text, or by anything under
    /// it. A way's own text that calls what it stands in for (to do
    /// something before or after it) means the thing itself. Given its
    /// way again there, it was started under itself, and that under
    /// itself, for as long as there was room: two hundred instances in a
    /// few seconds, none of which ever began.
    async fn within_its_way(&self, one: &Comparison, start: &Start<'_>, labels: &Labels) -> bool {
        if labels.get(COMPARED) != Some(&one.name) {
            return false;
        }
        let own = labels.get(WAY).and_then(|way| one.way(way)).and_then(|way| way.blueprint.as_deref());
        let Some(own) = own else { return false };
        // No tree is as deep as this, and nothing is followed for ever.
        let mut at = Some(start.parent.to_string());
        for _ in 0..64 {
            let Some(id) = at else { return false };
            let Ok(above) = self.engine.instance(&id).await else { return false };
            if above.blueprint == own {
                return true;
            }
            at = above.parent;
        }
        false
    }

    /// The way a start goes, where it goes one; `left` is why it is left
    /// as written though a comparison says how its name is started.
    async fn way(&self, start: &Start<'_>, labels: &mut Labels, left: &mut Option<String>) -> Option<Chosen> {
        // A start that names an asked comparison and its way.
        let named = kernel::compared::named(start.args);
        // Nothing is read for a name no comparison speaks of.
        let all = {
            let kept = self.kept.all.lock().expect("compared lock");
            match kept.as_ref() {
                Some(all) if all.is_empty() && named.is_none() => return None,
                Some(all) => Some(all.clone()),
                None => None,
            }
        };
        let all = match all {
            Some(all) => all,
            None => self.compared().await,
        };
        if let Some((name, way)) = named {
            return self.named_way(&all, start, labels, left, &name, &way).await;
        }
        let here = |one: &&Comparison| one.project.as_deref() == start.project;
        let in_force = |one: &&Comparison| !one.asked && one.in_force() && one.of == start.name;
        let Some(one) = all.iter().filter(here).find(in_force) else {
            // Started as written, and counted with the thing's way.
            for one in all.iter().filter(here).filter(|one| one.ended.is_none()) {
                if one.also.iter().any(|also| also == start.name)
                    && let Some(thing) = also_thing(one, start)
                    && let Some(went) = self.went(&one.name, &thing).await
                {
                    labels.insert(COMPARED.into(), one.name.clone());
                    labels.insert(WAY.into(), went.way.clone());
                    // And with the text the thing last ran, so that it is
                    // counted beside the thing's own work and not apart;
                    // what it reads is the words that text was run with.
                    if let Some(text) = went.text {
                        let way = one.way(&went.way);
                        let ran = way.and_then(|way| way.texts.iter().find(|ran| ran.text == text));
                        if let Some(ran) = ran.filter(|ran| !ran.prompts.is_empty()) {
                            self.keep_reads(start.parent, start.key, ran.prompts.clone()).await;
                        }
                        labels.insert(TEXT.into(), text);
                    }
                }
            }
            return None;
        };
        if self.within_its_way(one, start, labels).await {
            // Started as it is written, and counted with the way it is
            // under, whose labels it has from what started it.
            return None;
        }
        if !covers(&one.held, &start.whom.scopes) {
            *left = Some(format!(
                "the comparison `{}` says how `{}` is started, and whoever started it ({}) does not hold \
                 everything this instance runs with, so it was started as it is written. Somebody who holds \
                 that starts the comparison, or this goes on as written",
                one.name, one.of, one.by
            ));
            return None;
        }
        let (way, counted) = match (&one.goes, one.ended) {
            (Some(goes), _) => (one.way(goes)?.clone(), false),
            (None, Some(_)) => return None,
            (None, None) => (self.choose(&one.name, &thing(one, start, true)?).await?, true),
        };
        self.give(one, way, counted, start, labels).await
    }

    /// The way a start that names the asked comparison `name` and its way
    /// `way` goes: that way, whatever the shares, where the comparison
    /// speaks for the start; else nothing, with why in `left`.
    async fn named_way(
        &self,
        all: &[Comparison],
        start: &Start<'_>,
        labels: &mut Labels,
        left: &mut Option<String>,
        name: &str,
        way: &str,
    ) -> Option<Chosen> {
        let one = all.iter().find(|one| one.name == name);
        if let Err(why) = kernel::compared::asked_way(one, name, start.name, start.project, way) {
            *left = Some(why);
            return None;
        }
        let one = one?;
        if self.within_its_way(one, start, labels).await {
            return None;
        }
        if !covers(&one.held, &start.whom.scopes) {
            *left = Some(format!(
                "this start names the comparison `{name}`, and whoever started it ({}) does not hold everything \
                 this instance runs with, so it was started as it is written",
                one.by
            ));
            return None;
        }
        let thing = thing(one, start, true)?;
        let chosen = match self.choose_named(name, way, &thing).await {
            Ok(chosen) => chosen,
            Err(why) => {
                *left = Some(why);
                return None;
            }
        };
        self.give(one, chosen, true, start, labels).await
    }

    /// What `start` starts going the way `way` of `one`, with the labels
    /// it is then under added to `labels` where it is `counted`.
    async fn give(
        &self,
        one: &Comparison,
        way: Way,
        counted: bool,
        start: &Start<'_>,
        labels: &mut Labels,
    ) -> Option<Chosen> {
        let mut args = start.args.clone();
        args.extend(way.with.clone());
        let project = start.project;
        // A way's own text is the one from when the comparison began where
        // texts are held, and its file as it now is where they are
        // followed ([`conductor::taking`]): the last that checked, if that one
        // does not.
        let (blueprint, source) = match &way.blueprint {
            Some(other) => {
                let kept = match &way.revision {
                    Some(revision) => self.engine.text_of(revision).await,
                    None => None,
                };
                // One held to a version, or whose text was sent, is that
                // text and no other, whatever its file has come to.
                let there = match self.engine.follows() && way.version.is_none() && !way.sent {
                    true => self.engine.text_within(start.parent, start.whom, other, project).await,
                    false => None,
                };
                let there = there.filter(|there| self.engine.compile_new(other, there, project).is_ok());
                (other.clone(), Some(there.or(kept)?))
            }
            None => (start.name.to_string(), None),
        };
        // The words it reads in place of others, as they are for this
        // thing: settled now, and written down for what is started.
        let reads = match way.prompts.is_empty() {
            true => BTreeMap::new(),
            false => self.reads_now(&way, start.parent, start.whom, project).await,
        };
        self.keep_reads(start.parent, start.key, reads.clone()).await;
        if counted {
            // What is run, to say which text each measurement is of.
            let ran = match &source {
                Some(text) => Some(text.clone()),
                None => match self.engine.pinned_for(start.parent, start.name).await {
                    Some((_, pinned)) => Some(pinned),
                    None => self.engine.text_within(start.parent, start.whom, start.name, project).await,
                },
            };
            labels.insert(COMPARED.into(), one.name.clone());
            labels.insert(WAY.into(), way.name.clone());
            if let Some(ran) = ran {
                let text = self.ran(&one.name, &way, &ran, reads).await;
                // Each thing says which text it last ran, and is counted
                // once under each text it was given to.
                if let Some(thing) = thing(one, start, true) {
                    self.ran_by(one, &way.name, &thing, &text).await;
                }
                labels.insert(TEXT.into(), text);
            }
        }
        Some(Chosen { blueprint, source, args })
    }

    /// Notes that `way` ran the text `ran`, where it had not before, and
    /// gives what measurements of it carry as `text`.
    async fn ran(&self, comparison: &str, way: &Way, ran: &str, prompts: BTreeMap<String, String>) -> String {
        let revision = blueprint::revision(ran);
        // Other words read with the same text are counted apart from it.
        let told = match prompts.is_empty() {
            true => revision.clone(),
            false => blueprint::revision(&format!("{revision}\n{prompts:?}")),
        };
        let text: String = told.chars().take(8).collect();
        let same = |last: &Ran| last.revision == revision && last.prompts == prompts;
        if way.texts.last().is_some_and(same) {
            return text;
        }
        let _one = self.kept.choosing.lock().await;
        let Ok(mut one) = self.comparison(comparison).await else { return text };
        let Some(kept) = one.ways.iter_mut().find(|kept| kept.name == way.name) else { return text };
        if kept.texts.last().is_some_and(same) {
            return text;
        }
        // Run again after another: it is the one that is running now.
        kept.texts.retain(|before| !same(before));
        // The words that fitted are what it goes on with where their file stops fitting.
        for (linked, revision) in &prompts {
            if let Some(reads) = kept.prompts.get_mut(linked) {
                reads.revision = Some(revision.clone());
            }
        }
        kept.texts.push(Ran {
            text: text.clone(),
            revision: revision.clone(),
            version: blueprint::version(ran),
            since: self.engine.journal().now_ms(),
            prompts,
        });
        if kept.blueprint.is_some() {
            // The last of its own that ran is what it goes on with where its file stops checking.
            self.engine.keep_text(&revision, ran).await;
            kept.revision = Some(revision);
        }
        let _ = self.keep_compared(&one).await;
        text
    }

    /// The way `thing` went under `comparison`, with when it was given it
    /// and the text it last ran, where it went one. One written down
    /// before those were kept says the way alone.
    pub(crate) async fn went(&self, comparison: &str, thing: &str) -> Option<Went> {
        let kept =
            self.engine.journal().kv_get(WENT, &format!("{comparison}\n{thing}")).await.ok().flatten()?;
        match kept.as_str() {
            Some(way) => Some(Went { way: way.to_string(), ..Default::default() }),
            None => serde_json::from_value(kept).ok(),
        }
    }

    /// Keeps which text `thing` last ran, where it is another than is
    /// kept, and counts the thing as given to that text of its way
    /// ([`crate::counted::GIVEN`]): how many things a way's text was given
    /// is what anything added up under it is divided by.
    async fn ran_by(&self, one: &Comparison, way: &str, thing: &str, text: &str) {
        // In turn with whatever else writes down what a thing came to.
        let _one = self.kept.choosing.lock().await;
        let Some(went) = self.went(&one.name, thing).await else { return };
        if went.text.as_deref() == Some(text) {
            return;
        }
        let now = Went { text: Some(text.to_string()), ..went };
        let _ = self.engine.journal().kv_set(WENT, &format!("{}\n{thing}", one.name), &json!(now)).await;
        let since = (self.engine.journal().now_ms() - now.at.unwrap_or(i64::MAX)).max(0) as f64;
        let labels =
            [(COMPARED, one.name.as_str()), (WAY, way), (TEXT, text), ("counted", crate::counted::GIVEN)];
        self.engine.journal().measure(crate::counted::CAME, &labels, since, true);
    }

    /// The way `thing` goes where a start names it: `way`, written down
    /// before it is given where the thing went none, and refused where it
    /// went another ([`kernel::compared::kept_to`]).
    async fn choose_named(
        &self,
        comparison: &str,
        way: &str,
        thing: &str,
    ) -> std::result::Result<Way, String> {
        let _one = self.kept.choosing.lock().await;
        let gone = || format!("the comparison `{comparison}` is no longer there to name");
        let mut one = self.comparison(comparison).await.map_err(|_| gone())?;
        let at = kernel::compared::asked_way(
            Some(&one),
            comparison,
            &one.of.clone(),
            one.project.clone().as_deref(),
            way,
        )?;
        let before = self.went(comparison, thing).await;
        if let Some(wrong) =
            kernel::compared::kept_to(comparison, thing, way, before.as_ref().map(|went| went.way.as_str()))
        {
            return Err(wrong);
        }
        if before.is_some() {
            return Ok(one.ways[at].clone());
        }
        one.ways[at].went += 1;
        let went =
            Went { way: way.to_string(), at: Some(self.engine.journal().now_ms()), ..Default::default() };
        let key = format!("{comparison}\n{thing}");
        self.engine.journal().kv_set(WENT, &key, &json!(went)).await.map_err(|why| why.message)?;
        self.keep_compared(&one).await.map_err(|why| why.message)?;
        Ok(one.ways[at].clone())
    }

    /// Writes `thing` down as a thing of the way `way` of the asked
    /// comparison `name` in `project`, as `by`, with nothing started for
    /// it: what is said of it from then is counted under that way. It is
    /// not counted as given (what was started under the way was), so a
    /// way's work that something else goes on with is counted with the
    /// way. Refused where the thing went another way, and the same done
    /// twice: whether it was written now.
    pub(crate) async fn write_down(
        &self,
        name: &str,
        project: Option<&str>,
        thing: &str,
        way: &str,
        by: &Principal,
    ) -> Result<bool> {
        if thing.is_empty() {
            return Err(Error::invalid(format!("nothing was said to write down as a thing of `{name}`")));
        }
        let _one = self.kept.choosing.lock().await;
        let one = self.comparison(name).await?;
        let at = kernel::compared::asked_way(Some(&one), name, &one.of, project, way).map_err(|_| {
            Error::invalid(format!(
                "`{name}` is not a comparison whose ways are named by the start in this project that is still \
                 going and has a way `{way}`, and a thing is written down only as one of those"
            ))
        })?;
        if !covers(&by.scopes, &one.held) {
            return Err(Error::denied(format!(
                "{} started `{name}` holding more than whoever asks does, so what it counts is not theirs to \
                 add to",
                one.by
            )));
        }
        let before = self.went(name, thing).await;
        if let Some(wrong) =
            kernel::compared::kept_to(name, thing, way, before.as_ref().map(|went| went.way.as_str()))
        {
            return Err(Error::conflict(wrong));
        }
        if before.is_some() {
            return Ok(false);
        }
        let text = one.ways[at].texts.last().map(|ran| ran.text.clone());
        let went =
            Went { way: way.to_string(), at: Some(self.engine.journal().now_ms()), text, accepted: None };
        self.engine.journal().kv_set(WENT, &format!("{name}\n{thing}"), &json!(went)).await?;
        Ok(true)
    }

    /// The way `thing` goes: the one it went before, or the one that has
    /// had least of its share, written down before it is given.
    async fn choose(&self, comparison: &str, thing: &str) -> Option<Way> {
        let _one = self.kept.choosing.lock().await;
        let mut one = self.comparison(comparison).await.ok()?;
        if one.ended.is_some() {
            return None;
        }
        if let Some(before) = self.went(comparison, thing).await
            && let Some(way) = one.way(&before.way)
        {
            return Some(way.clone());
        }
        let at = next(&one.ways);
        one.ways[at].went += 1;
        let key = format!("{comparison}\n{thing}");
        let went = Went {
            way: one.ways[at].name.clone(),
            at: Some(self.engine.journal().now_ms()),
            ..Default::default()
        };
        self.engine.journal().kv_set(WENT, &key, &json!(went)).await.ok()?;
        self.keep_compared(&one).await.ok()?;
        Some(one.ways[at].clone())
    }
}

/// What a start of one of `also` is for: what the param `also_per` names
/// was handed, where `one` says one; else as any start ([`thing`]).
fn also_thing(one: &Comparison, start: &Start<'_>) -> Option<String> {
    let Some(per) = &one.also_per else { return thing(one, start, false) };
    match start.args.get(per) {
        Some(Value::String(text)) if !text.is_empty() => Some(text.clone()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    }
}

/// What a start is for, as `one` tells things apart: what the param it
/// names was handed, or (for the blueprint it is of, `placed`) where the
/// child is in the tree.
fn thing(one: &Comparison, start: &Start<'_>, placed: bool) -> Option<String> {
    let said = one.per.as_ref().and_then(|per| start.args.get(per)).and_then(|value| match value {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    });
    said.or_else(|| placed.then(|| format!("{}/{}", start.parent, start.key)))
}
