//! What the runner asks of comparisons.
//!
//! One blueprint started several ways, side by side, is said from outside
//! and is not the runner's to know: what a comparison is, which way each
//! thing went and what is counted of it are kept by a crate above this
//! one (`comparisons`). The runner runs, and there are four moments in
//! that where a comparison has a say. It asks at each, through what was
//! installed ([`Engine::set_compared`]):
//!
//! - what a step starts in place of the blueprint it names (`starts`), in
//!   the one place a step starts a child ([`crate::child`]);
//! - what an instance reads in place of a file it links (`reads`), as
//!   `text.put` comes to the file and before it reads through the pin
//!   ([`crate::words`]);
//! - which kept texts are still wanted as a conductor starts (`keeps`),
//!   where the texts nothing holds are put away ([`crate::pinned`]);
//! - and that a conductor is starting (`resumed`), before anything is
//!   taken up, so that what was being counted goes on being counted.
//!
//! With nothing installed (a rig of the runner's own, a program that
//! never compares) each is answered as it is where no comparison was ever
//! started: a step starts what it names, a file is read through the pin,
//! and no text is wanted.

use std::collections::HashSet;

use blueprint::Labels;
use kernel::{BoxFut, Fields, Principal, Source};
use serde_json::json;

use crate::engine::Engine;

/// What a step is starting, as far as a comparison goes by it.
pub struct Start<'a> {
    pub project: Option<&'a str>,
    pub name: &'a str,
    pub args: &'a Fields,
    /// The instance the step runs in, and the child's name within it.
    pub parent: &'a str,
    pub key: &'a str,
    pub whom: &'a Principal,
}

/// What is started in its place.
pub struct Chosen {
    pub blueprint: String,
    pub source: Option<String>,
    pub args: Fields,
}

/// What a comparison says of a start.
pub enum Starts {
    /// What the step names, as it is written.
    Written,
    /// The same, though a comparison says how that name is started, and
    /// why: the step's instance says so (`instance.call_kept`).
    Left(String),
    /// One of a comparison's ways.
    Way(Chosen),
}

/// What a step starts: asked with the labels its child will be under,
/// to which those of a way are added.
pub type AsksStart =
    dyn for<'a> Fn(&'a Engine, &'a Start<'a>, &'a mut Labels) -> BoxFut<'a, Starts> + Send + Sync;

/// What an instance reads where it links a file, by the file's name in
/// the library: other words, where a way gave what started it any.
pub type AsksRead = dyn for<'a> Fn(&'a Engine, &'a str, &'a str) -> BoxFut<'a, Option<String>> + Send + Sync;

/// The revisions of the kept texts still wanted as a conductor starts,
/// told which instances are alive.
pub type AsksKept = dyn for<'a> Fn(&'a Engine, &'a HashSet<&'a str>) -> BoxFut<'a, Vec<String>> + Send + Sync;

/// That a conductor is starting.
pub type Tells = dyn for<'a> Fn(&'a Engine) -> BoxFut<'a, ()> + Send + Sync;

/// What answers for comparisons. Each is handed the engine that asks,
/// and so need hold none.
pub struct Compared {
    pub starts: Box<AsksStart>,
    pub reads: Box<AsksRead>,
    pub keeps: Box<AsksKept>,
    pub resumed: Box<Tells>,
    /// What answers keeps in memory, for an action of whoever installed
    /// it, which is handed an engine and nothing else, to answer from the
    /// same ([`Engine::compared_kept`]).
    pub kept: std::sync::Arc<dyn std::any::Any + Send + Sync>,
}

impl Engine {
    /// Installs what answers for comparisons, once: the first installed
    /// is what is asked.
    pub fn set_compared(&self, compared: Compared) {
        let _ = self.inner.compared.set(compared);
    }

    /// What what answers for comparisons keeps in memory, where something
    /// was installed.
    pub fn compared_kept(&self) -> Option<std::sync::Arc<dyn std::any::Any + Send + Sync>> {
        self.inner.compared.get().map(|compared| compared.kept.clone())
    }

    /// What a step that starts `start.name` starts instead, where a
    /// comparison says, with the labels the child is then under added to
    /// `labels`. Nothing where no comparison is in force for it.
    pub(crate) async fn way_for(
        &self,
        start: &Start<'_>,
        by: &Source,
        labels: &mut Labels,
    ) -> Option<Chosen> {
        let compared = self.inner.compared.get()?;
        match (compared.starts)(self, start, labels).await {
            Starts::Way(chosen) => Some(chosen),
            Starts::Written => None,
            Starts::Left(why) => {
                let said = json!({ "blueprint": start.name, "key": start.key, "error": why, "kept": "" });
                self.emit("instance.call_kept", by.clone(), said).await;
                None
            }
        }
    }

    /// The words `instance` is to read where it links `name`, where a way
    /// of a comparison gave what started it, or anything above that,
    /// other words for that file.
    pub async fn read_instead(&self, instance: &str, name: &str) -> Option<String> {
        (self.inner.compared.get()?.reads)(self, instance, name).await
    }

    /// The revisions of the texts comparisons keep, which are not put
    /// away as a conductor starts with `alive` alive.
    pub(crate) async fn compared_keeps(&self, alive: &HashSet<&str>) -> Vec<String> {
        match self.inner.compared.get() {
            Some(compared) => (compared.keeps)(self, alive).await,
            None => Vec::new(),
        }
    }

    /// A comparison that counts events goes on counting from where it was.
    /// Told by the factory's own conductor as it starts, and by no
    /// project's: comparisons are kept once for the factory, and what
    /// counts for them follows every journal by itself.
    pub(crate) async fn compared_resumed(&self) {
        if let Some(compared) = self.inner.compared.get().filter(|_| self.inner.project.is_none()) {
            (compared.resumed)(self).await;
        }
    }
}
