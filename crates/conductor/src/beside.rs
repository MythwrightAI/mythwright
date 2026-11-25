//! What a conductor keeps beside its rows is kept where its rows are.
//!
//! Which text of each name that ships a tree reached, what was vouched
//! for an instance, what a `poll:` last read and the last text of a
//! blueprint that checked are small values in a journal's small durable
//! state, each under a name that says whose it is (`runs::beside`). A
//! conductor keeps them in its own journal, with the rows of what it
//! runs: a project's in the project's file, so that they are copied,
//! moved and taken away with the project, and so that keeping them is
//! no work for the thread every project shares. What is the factory's
//! own (how texts are taken, the variables of the profile, the
//! comparisons) is the factory's journal's, whichever conductor asks.
//!
//! **A journal from before.** A program before this kept all of it in
//! the factory's journal, for every conductor. A journal that had
//! something alive when it was first opened by this program says so
//! (`runs::beside::OWED`, written by the step of the tables), and the
//! conductor that starts on it brings over what is its own, once,
//! before anything is taken up or tidied: each thing is written in its
//! own journal and then taken out of the factory's, so a conductor
//! stopped between the two finds it in both and brings the same over
//! again. The factory's own conductor has nothing to bring. Until a
//! project's conductor has done so, the factory's leaves what is kept
//! for anything alive anywhere where it is (`Engine::pin_resumed`).

use journal::Journal;
use runs::beside::{FITS, OWED, PINNED, POLL, TEXTS, VOUCHED, instance_of, project_of};
use serde_json::Value;

use crate::engine::Engine;
use crate::pinned::Pins;

impl Engine {
    /// The journal that keeps what is kept beside the rows of
    /// `instance`: its conductor's own. For what keeps something of an
    /// instance's from outside the runner (what a way of a comparison
    /// has it read), which is then where the instance is.
    pub async fn kept_beside(&self, instance: &str) -> Journal {
        match self.kept_by_another(instance).await {
            Some(keeper) => keeper.inner.own.clone(),
            None => self.inner.own.clone(),
        }
    }

    /// Every journal a conductor keeps what it runs in: the factory's
    /// own first, then each project's that has one.
    pub fn own_journals(&self) -> Vec<Journal> {
        self.every().into_iter().map(|conductor| conductor.inner.own.clone()).collect()
    }

    /// Brings over, once, what a program from before kept in the
    /// factory's journal for what this conductor keeps, where its own
    /// journal says it may be owed that.
    pub(crate) async fn bring_over(&self) {
        let own = &self.inner.own;
        if own.kv_get(OWED.0, OWED.1).await.ok().flatten().is_none() {
            return;
        }
        if let Some(project) = self.inner.project.clone() {
            let factory = &self.inner.factory;
            for name in [PINNED, VOUCHED, POLL] {
                for (key, kept) in factory.kv_list(name).await.unwrap_or_default() {
                    if !matches!(self.inner.store.project_of(instance_of(&key)).await, Ok(Some(_))) {
                        continue;
                    }
                    if name == PINNED {
                        self.bring_texts(&kept).await;
                    }
                    self.moved_here(name, &key, &kept).await;
                }
            }
            for (key, kept) in factory.kv_list(FITS).await.unwrap_or_default() {
                if project_of(&key) == Some(project.as_str()) {
                    self.moved_here(FITS, &key, &kept).await;
                }
            }
        }
        let _ = own.kv_set(OWED.0, OWED.1, &Value::Null).await;
    }

    /// One thing, written here and then taken out of the factory's.
    async fn moved_here(&self, name: &str, key: &str, kept: &Value) {
        if self.inner.own.kv_set(name, key, kept).await.is_ok() {
            let _ = self.inner.factory.kv_set(name, key, &Value::Null).await;
        }
    }

    /// The texts a tree is held to, and can be put back on, are kept
    /// here with it. They stay in the factory's too, for whatever else
    /// is held to them there, until its conductor finds nothing is.
    async fn bring_texts(&self, pins: &Value) {
        let Ok(pins) = serde_json::from_value::<Pins>(pins.clone()) else { return };
        let mut revisions: Vec<String> = pins.texts.into_values().collect();
        if let Some(before) = pins.before {
            revisions.extend(before.texts.into_values());
            revisions.extend(before.own);
        }
        for revision in revisions {
            if self.inner.own.kv_get(TEXTS, &revision).await.ok().flatten().is_some() {
                continue;
            }
            if let Some(text) = self.inner.factory.kv_get(TEXTS, &revision).await.ok().flatten() {
                let _ = self.inner.own.kv_set(TEXTS, &revision, &text).await;
            }
        }
    }
}
