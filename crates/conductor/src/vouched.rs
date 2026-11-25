//! Which of a project's own texts an instance may run: the runner's side.
//!
//! What was vouched for is kept by `texts` (`texts::vouched`, which says
//! who vouches for what, when, and what a step is held to), one keeper
//! for each conductor. What is here is what only the runner can say: what
//! an instance is called and what started it, from what is in hand; and
//! which conductor keeps what was vouched for an instance or a project.

use kernel::Principal;
use runs::{InstanceFilter, InstanceInfo};
pub(crate) use texts::vouched::Held;
pub use texts::vouched::Vouched;

use crate::engine::Engine;

/// What is kept of texts asks two things back of whatever runs
/// ([`texts::Runner`], which says why these two): this conductor answers.
impl texts::Runner for Engine {
    fn check_own(&self, source: &str, project: Option<&str>) -> blueprint::Checked {
        Engine::check_own(self, source, project)
    }

    fn above<'a>(&'a self, instance: &'a str) -> kernel::BoxFut<'a, Option<(String, Option<String>)>> {
        Box::pin(Engine::above(self, instance))
    }
}

impl Engine {
    /// What an instance is called and what started it.
    pub async fn above(&self, instance: &str) -> Option<(String, Option<String>)> {
        if let Some(live) = self.live(instance) {
            let meta = live.meta();
            return Some((meta.key, meta.parent));
        }
        let info = match self.inner.store.instance(instance).await.ok().flatten() {
            Some(info) => info,
            // Another conductor's: asked of the one that keeps it.
            None => return Box::pin(self.elsewhere(instance).await?.above(instance)).await,
        };
        Some((info.key, info.parent))
    }

    /// What an apply does about the project's own texts once its instance
    /// is known, and what is said of it to whoever asked
    /// ([`texts::Texts::answered`]).
    pub(crate) async fn answered(
        &self,
        instance: &str,
        project: Option<&str>,
        applied: (bool, (&str, bool)),
        by: &Principal,
        asked: (bool, Option<String>),
    ) -> Vouched {
        self.inner.texts.answered(self, instance, project, applied, by, asked).await
    }

    /// `by` wrote, or took away, the file that is `project`'s own `name`,
    /// through the daemon: it is vouched for wherever what was vouched
    /// for in that project was vouched for with no more than `by` holds
    /// ([`texts::Texts::vouch_saved`]).
    pub async fn vouch_saved(&self, project: &str, name: &str, by: &Principal) {
        // What was vouched for is kept with the instances it was vouched
        // for, by the conductor that runs what is applied for the project.
        if let Ok(keeper) = self.of(Some(project)).await
            && !std::sync::Arc::ptr_eq(&keeper.inner, &self.inner)
        {
            return Box::pin(keeper.vouch_saved(project, name, by)).await;
        }
        self.inner.texts.vouch_saved(self, project, name, by).await
    }

    /// What a step of `instance`, acting as `whom`, is given for `name`
    /// when `there` is the text now under that name
    /// ([`texts::Texts::held_to`]).
    pub(crate) async fn held_to(
        &self,
        instance: &str,
        whom: &Principal,
        name: &str,
        project: Option<&str>,
        there: Option<&str>,
    ) -> Held {
        self.inner.texts.held_to(self, instance, whom, name, project, there).await
    }

    /// The text `instance`, acting as `whom`, may be given for `name`: the
    /// one there now, or the project's own as it was vouched for. For
    /// whatever asks a blueprint something on an instance's behalf without
    /// applying it (a rule that is asked and never run).
    pub async fn text_within(
        &self,
        instance: &str,
        whom: &Principal,
        name: &str,
        project: Option<&str>,
    ) -> Option<String> {
        // What was vouched for an instance is kept by its conductor.
        if let Some(keeper) = self.kept_by_another(instance).await {
            return Box::pin(keeper.text_within(instance, whom, name, project)).await;
        }
        let there = self.inner.library.load(name, project);
        match self.held_to(instance, whom, name, project, there.as_deref()).await {
            Held::There => there,
            Held::Instead { source, .. } => source,
        }
    }

    /// What is running in `project`, at the top of its trees, that reaches
    /// `name` and goes on with another text under it than the one there
    /// now: the project's own file under the name, written outside the
    /// factory or changed there since that was applied, is not what it is
    /// given ([`texts::Texts::held_to`]). Worked out from what was vouched
    /// for, as a step would be given the text, and nothing is kept.
    pub async fn goes_on_otherwise(&self, project: &str, name: &str) -> Vec<InstanceInfo> {
        let Some(there) = self.inner.library.load(name, Some(project)) else { return Vec::new() };
        let roots = InstanceFilter {
            live_only: true,
            roots_only: true,
            project: Some(project.to_string()),
            limit: 100_000,
            ..Default::default()
        };
        let mut found = Vec::new();
        for root in crate::routed::instances(self, roots).await.unwrap_or_default() {
            if !self.reaches(&root, name, project) {
                continue;
            }
            let given = self.text_within(&root.id, &root.principal, name, Some(project)).await;
            // Words are the same only where they are the same words; a blueprint laid out another way is the same.
            let words = crate::library::is_linked(name);
            let same =
                |given: &String| *given == there || (!words && blueprint::means_the_same(given, &there));
            if !given.as_ref().is_some_and(same) {
                found.push(root);
            }
        }
        found
    }

    /// Whether the tree `root` is at the top of reaches `name`, by what its
    /// texts as they are now name ([`blueprint::names::of`]) and the files
    /// they link, all the way down.
    fn reaches(&self, root: &InstanceInfo, name: &str, project: &str) -> bool {
        let library = &self.inner.library;
        let actions = self.registry().all_specs();
        let known = |one: &str| library.load(one, Some(project)).is_some();
        let first = match root.source.is_empty() {
            true => library.load(&root.blueprint, Some(project)).unwrap_or_default(),
            false => root.source.clone(),
        };
        let mut seen = std::collections::HashSet::from([root.blueprint.clone()]);
        let mut texts = vec![(root.blueprint.clone(), first)];
        while let Some((of, text)) = texts.pop() {
            let mut named: Vec<String> =
                blueprint::names::of(&text, &known, &actions).into_iter().map(|one| one.name).collect();
            if let Some(read) = blueprint::check(&text, &blueprint::NoCatalog).blueprint {
                let links = blueprint::parts::linked(&read);
                named.extend(links.into_iter().map(|link| blueprint::parts::linked_as(&of, link.from)));
            }
            for one in named {
                if one == name {
                    return true;
                }
                if seen.insert(one.clone())
                    && let Some(text) = library.load(&one, Some(project))
                {
                    texts.push((one, text));
                }
            }
        }
        false
    }

    /// When the conductor starts ([`texts::Texts::vouch_resumed`]).
    pub(crate) async fn vouch_resumed(&self, live: &[InstanceInfo], alive: &[InstanceInfo]) {
        self.inner.texts.vouch_resumed(self, live, alive).await
    }

    /// Every live instance, for [`Engine::vouch_resumed`].
    pub(crate) async fn live_instances(&self) -> Vec<InstanceInfo> {
        let filter = InstanceFilter { live_only: true, limit: 100_000, ..Default::default() };
        self.inner.store.instances(filter).await.unwrap_or_default()
    }
}
