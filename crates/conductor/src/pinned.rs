//! What ships, as a running tree reached it: the runner's side.
//!
//! What a tree is held to is kept by `texts` (`texts::pinned`, which says
//! what is pinned, when the pins move and where they are kept), one
//! keeper for each conductor. What is here is what the runner does around
//! that: which conductor keeps an instance is asked first, what is in
//! hand is what answers who is above whom, and what asks every conductor
//! or reads through what was vouched for is put together here.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use kernel::{Error, Result};
use runs::InstanceInfo;
use texts::pinned::Asked;
pub(crate) use texts::pinned::{Before, Pins};
pub use texts::pinned::{Moved, Reached};

use crate::engine::Engine;

/// The words an instance is given for a file it links.
pub struct Linked {
    pub text: String,
    /// Where `text` is what the instance's tree is held to and the file
    /// is another now: the one there now.
    pub there: Option<String>,
}

/// What the keeping of texts reads of a request to apply.
fn asked(request: &crate::Apply) -> Asked<'_> {
    Asked {
        blueprint: &request.blueprint,
        sourced: request.source.is_some(),
        key: request.key.as_deref(),
        parent: request.parent.as_deref(),
        project: request.project.as_deref(),
        take: request.take,
    }
}

impl Engine {
    pub(crate) async fn pins(&self, instance: &str) -> Option<Pins> {
        self.inner.texts.pins(instance).await
    }

    pub(crate) async fn put_pins(&self, instance: &str, pins: Option<Pins>) {
        self.inner.texts.put_pins(instance, pins).await
    }

    /// Keeps a text by its revision, once, with what this conductor
    /// keeps: where the trees it is kept for are.
    pub async fn keep_text(&self, revision: &str, source: &str) {
        self.inner.texts.keep_text(revision, source).await
    }

    pub async fn text_of(&self, revision: &str) -> Option<String> {
        self.inner.texts.text_of(revision).await
    }

    /// The text of `name` that calls itself `version`, of those the
    /// factory has: the one there now, or one a running tree reached or
    /// was moved from, which is kept by its revision for as long as that
    /// tree may be put back on it. Nothing else is kept, and nothing of a
    /// project's own file is: only what ships is pinned. Where it is not
    /// kept that is said, with what there is and what to do.
    pub async fn text_at(&self, name: &str, version: &str, project: Option<&str>) -> Result<String> {
        let library = &self.inner.library;
        let Some(now) = library.load(name, project) else {
            return Err(Error::not_found(format!("there is no blueprint called `{name}`")));
        };
        let there = blueprint::version(&now);
        if there.as_deref() == Some(version) {
            return Ok(now);
        }
        let mut looked = BTreeSet::new();
        let mut found: Vec<String> = Vec::new();
        let mut others = BTreeSet::new();
        // What any tree of the factory is held to, in whichever
        // conductor's keeping.
        for keeper in self.every() {
            keeper.inner.texts.at_version(name, version, &mut looked, &mut found, &mut others).await;
        }
        // Laid out again or with a comment written in it, a text is
        // another revision of the same version: any of those is it. Two
        // that mean different things under one version are not.
        if let Some(first) = found.first() {
            if found.iter().all(|other| blueprint::means_the_same(first, other)) {
                return Ok(first.clone());
            }
            return Err(Error::conflict(format!(
                "is `{name}` at version {version}, and the factory has kept {} texts that call themselves that \
                 and do not mean the same. Put the one you mean in the project under a name of its own and name \
                 that as the way",
                found.len()
            )));
        }
        let has = match there {
            Some(there) => format!("What is there now calls itself version {there}"),
            None => "What is there now says no version".to_string(),
        };
        let kept = match others.is_empty() {
            true => "nothing else of it is kept".to_string(),
            false => format!(
                "what is kept besides is version {}",
                others.into_iter().collect::<Vec<_>>().join(", ")
            ),
        };
        let whose = match library.is_own(name, project) {
            true => " It is the project's own file here, and no earlier text of a project's file is kept.",
            false => "",
        };
        let put = match crate::library::folder_of(library.as_ref(), project).path() {
            Some(path) => format!("the project's `{path}/`"),
            None => format!(
                "a folder of the project's own blueprints, which it keeps none of now (`{} project blueprints {} \
                 --usual`),",
                brand::CLI,
                project.unwrap_or("<project>")
            ),
        };
        Err(Error::not_found(format!(
            "is `{name}` at version {version}, and the factory has not kept that version. {has}, and {kept}: a \
             text that ships is kept only while a running tree was started with it or can be put back on \
             it.{whose} To compare with a text that is not kept, put it in {put} under a name of its own and \
             name that as the way"
        )))
    }

    /// Pins `instance` to `reached` ([`texts::Texts::pin`]).
    pub(crate) async fn pin(
        &self,
        instance: &str,
        project: Option<&str>,
        reached: &[(String, String)],
        own: Option<&str>,
    ) {
        self.inner.texts.pin(instance, project, reached, own).await
    }

    /// What an apply by somebody is checked against in place of the texts
    /// now there ([`texts::Texts::kept_reach`]).
    pub(crate) async fn kept_reach(&self, request: &crate::Apply, source: &str) -> BTreeMap<String, String> {
        self.inner.texts.kept_reach(self, &asked(request), source).await
    }

    /// Whether `request` brings `existing` to another text that ships
    /// under the name it was asked for by.
    pub(crate) fn comes_to_another(
        &self,
        request: &crate::Apply,
        existing: &InstanceInfo,
        source: &str,
    ) -> bool {
        self.inner.texts.comes_to_another(&asked(request), existing, source)
    }

    /// What somebody's apply of `instance` does about what it reaches, and
    /// what is said of it ([`texts::Texts::repin`]).
    pub(crate) async fn repin(
        &self,
        instance: &str,
        project: Option<&str>,
        reached: &[(String, String)],
        moved: Option<Option<&str>>,
    ) -> Reached {
        self.inner.texts.repin(instance, project, reached, moved).await
    }

    /// Takes away what is pinned for each live instance under `instance`,
    /// as somebody takes what ships at it: what is under it goes by what
    /// it is pinned to from now, and a part somebody once took by itself
    /// does not hold what is under it to what it reached then. Each with
    /// what it had, to be put back where what moved it does not go through.
    pub(crate) async fn unpin_below(&self, instance: &str) -> Vec<(String, Option<Pins>)> {
        let mut was = Vec::new();
        let mut parents = vec![instance.to_string()];
        while let Some(parent) = parents.pop() {
            let filter = runs::InstanceFilter {
                parent: Some(parent),
                live_only: true,
                limit: 100_000,
                ..Default::default()
            };
            for child in self.inner.store.instances(filter).await.unwrap_or_default() {
                if let Some(pins) = self.pins(&child.id).await {
                    self.put_pins(&child.id, None).await;
                    was.push((child.id.clone(), Some(pins)));
                }
                parents.push(child.id);
            }
        }
        was
    }

    /// What a part below the top of its tree, taken by itself, is held to
    /// beside what it `reached`: each other name that ships which what is
    /// above it is held to, as it is now. A name only known as a step runs
    /// is held to at the top of the tree ([`Engine::pin_later`]), and what
    /// a part under this one starts by such a name moves with the rest.
    pub(crate) async fn held_above(
        &self,
        instance: &InstanceInfo,
        reached: &[(String, String)],
    ) -> Vec<(String, String)> {
        let project = instance.project.as_deref();
        let mut more: Vec<(String, String)> = Vec::new();
        let mut at = instance.parent.clone();
        while let Some(above) = at {
            for name in self.pins(&above).await.map(|pins| pins.texts.into_keys()).unwrap_or_default() {
                let known = reached.iter().chain(&more).any(|(one, _)| *one == name);
                if !known
                    && self.holds(&name, project)
                    && let Some(there) = self.inner.library.load(&name, project)
                {
                    more.push((name, there));
                }
            }
            at = self.above(&above).await.and_then(|(_, parent)| parent);
        }
        more
    }

    /// What is pinned for `instance` as it stands, to be put back if what
    /// moved it does not go through.
    pub(crate) async fn pinned_now(&self, instance: &str) -> Option<Pins> {
        self.pins(instance).await
    }

    /// Puts back what [`Engine::pinned_now`] gave.
    pub(crate) async fn pin_back(&self, instance: &str, was: Option<Pins>) {
        self.put_pins(instance, was).await;
    }

    /// The text that ships under `name` as `instance` reached it: of the
    /// nearest of itself and what is above it that has one pinned. Its
    /// revision and the text; nothing where none of them has, or the text
    /// was not kept.
    pub async fn pinned_for(&self, instance: &str, name: &str) -> Option<(String, String)> {
        // Followed, a text is read where it is whatever was reached before.
        if self.follows() {
            return None;
        }
        // What a tree reached is kept by the conductor that runs it.
        if let Some(keeper) = self.kept_by_another(instance).await {
            return Box::pin(keeper.pinned_for(instance, name)).await;
        }
        self.inner.texts.pinned_above(self, instance, name).await
    }

    /// The words `instance`, acting as `whom`, is given for the file it
    /// links as `name`. One that ships is the one its tree reached, as a
    /// blueprint it starts by name is, for as long as texts are held; one
    /// its tree never reached (linked by a name worked out as the step
    /// ran, or by a tree from before words were held) is held from now.
    /// A project's own is given as it was vouched for, and where texts
    /// are followed every one is read as it is.
    pub async fn linked_for(
        &self,
        instance: &str,
        whom: &kernel::Principal,
        name: &str,
        project: Option<&str>,
    ) -> Option<Linked> {
        let there = self.text_within(instance, whom, name, project).await;
        if !self.holds(name, project) {
            return there.map(|text| Linked { text, there: None });
        }
        match self.pinned_for(instance, name).await {
            Some((_, text)) => {
                let there = there.filter(|there| *there != text);
                Some(Linked { text, there })
            }
            None => {
                let text = there?;
                self.pin_later(instance, name, &text).await;
                Some(Linked { text, there: None })
            }
        }
    }

    /// The text under `name` that what is running in `project` goes on
    /// with, where a tree there is held to one: what somebody who wants
    /// that text as their own is handed, and not a later one the tree has
    /// not taken.
    pub async fn reached_in(&self, project: &str, name: &str) -> Option<String> {
        let roots = runs::InstanceFilter {
            live_only: true,
            roots_only: true,
            project: Some(project.to_string()),
            limit: 100_000,
            ..Default::default()
        };
        for root in crate::routed::instances(self, roots).await.unwrap_or_default() {
            if let Some((_, text)) = self.pinned_for(&root.id, name).await {
                return Some(text);
            }
        }
        None
    }

    /// A text that ships, reached by a step of `instance` under a name
    /// nothing above it had pinned: pinned from now, at the top of its
    /// tree ([`texts::Texts::pin_later`]).
    pub(crate) async fn pin_later(&self, instance: &str, name: &str, source: &str) {
        self.inner.texts.pin_later(self, instance, name, source).await
    }

    /// When the conductor starts ([`texts::Texts::pin_resumed`]): what
    /// comparisons still want of the kept texts is asked first.
    pub(crate) async fn pin_resumed(&self, live: &[InstanceInfo], alive: &[InstanceInfo]) {
        let alive: HashSet<&str> = alive.iter().map(|info| info.id.as_str()).collect();
        // A comparison's ways are texts kept the same way, once for the
        // factory, in the factory's journal.
        let wanted: HashSet<String> = match self.inner.project {
            None => self.compared_keeps(&alive).await.into_iter().collect(),
            Some(_) => HashSet::new(),
        };
        self.inner.texts.pin_resumed(live, &alive, wanted).await
    }
}
