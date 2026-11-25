//! A project's fork of a blueprint that ships: the runner's side.
//!
//! What a fork was made from is kept by `texts` (`texts::forked`, which
//! says what is kept, where and why the text is kept whole). What is
//! here is what the runner does around that: which conductor keeps what
//! is a project's is asked first, and what is kept is written with what
//! is said of it, as one write in that conductor's own journal
//! (`project.forked`, `project.unforked`).
//!
//! The fork itself is a file of the project's, written and taken away by
//! whoever writes the project's files; the journal's write does not
//! reach it. So the file is written and then this is kept and said, and
//! a fork is taken away in three moves: its record is marked as leaving
//! ([`Engine::fork_leaving`]), its file is taken away, and the record is
//! forgotten in the write that says so ([`Engine::fork_gone`]). A daemon
//! stopped anywhere among them leaves a file with no record that stands,
//! or a record with no file, which says nothing and is written over by
//! the next fork made there.
//!
//! Nothing here changes which text runs under a name, what a tree is
//! held to or whether an instance is behind: a fork is a project's own
//! file to the runner, as it always was.

use kernel::{Result, Source};
use serde_json::json;
pub use texts::forked::{NAMESPACE, Origin, key};

use crate::engine::Engine;

impl Engine {
    /// The conductor that keeps what is `project`'s, or this one where
    /// the project has none of its own.
    pub(crate) async fn keeper_of(&self, project: &str) -> Engine {
        self.of(Some(project)).await.unwrap_or_else(|_| self.clone())
    }

    /// What `project`'s fork of `blueprint` was made from, where that
    /// was kept and still stands ([`texts::Texts::forked_from`]).
    pub async fn forked_from(&self, project: &str, blueprint: &str) -> Option<Origin> {
        self.keeper_of(project).await.inner.texts.forked_from(project, blueprint).await
    }

    /// Keeps what a fork of `project`'s was made from, and says that it
    /// was made, as one write. `source` says who made it and what
    /// hearing of it takes. A record there already for the name (of a
    /// fork taken away under a daemon that stopped) is written over.
    pub async fn fork_made(&self, project: &str, origin: Origin, source: Source) -> Result<()> {
        let (at, kept) = (key(project, &origin.blueprint), origin.kept());
        let said = json!({
            "project": project,
            "blueprint": origin.blueprint,
            "version": origin.version,
            "revision": origin.revision,
            "path": origin.path,
            "by": origin.by,
            "by_name": origin.by_name,
        });
        let journal = self.keeper_of(project).await.inner.own.clone();
        let write = journal.write(move |writing| {
            writing.keep(NAMESPACE, &at, &kept)?;
            writing.say("project.forked", source, said)
        });
        write.await.map(|_| ())
    }

    /// Marks what is kept of `project`'s fork of `blueprint` as leaving,
    /// before its file is taken away, or takes the mark off where the
    /// file could not be. Nothing is said: nothing has happened yet.
    /// Gives what was kept, whatever it said.
    pub async fn fork_leaving(&self, project: &str, blueprint: &str, leaving: bool) -> Option<Origin> {
        let keeper = self.keeper_of(project).await;
        let was = keeper.inner.texts.fork_kept(project, blueprint).await?;
        if was.leaving != leaving {
            let now = Origin { leaving, ..was.clone() };
            let _ = keeper.inner.own.kv_set(NAMESPACE, &key(project, blueprint), &now.kept()).await;
        }
        Some(was)
    }

    /// Says that `project`'s fork of `blueprint` is written at `path` now,
    /// where what is kept of it says it was at `was`: its file was moved
    /// (into the factory's place for the project's own blueprints), and
    /// what it is a fork of is as it was. Nothing is said: the fork is
    /// the fork it was. Gives whether there was a record to move.
    pub async fn fork_moved(
        &self,
        project: &str,
        blueprint: &str,
        (was, path): (&str, &str),
    ) -> Result<bool> {
        let keeper = self.keeper_of(project).await;
        let Some(kept) =
            keeper.inner.texts.fork_kept(project, blueprint).await.filter(|kept| kept.path == was)
        else {
            return Ok(false);
        };
        let now = Origin { path: path.to_string(), ..kept };
        keeper.inner.own.kv_set(NAMESPACE, &key(project, blueprint), &now.kept()).await?;
        Ok(true)
    }

    /// Forgets what `project`'s fork of `blueprint` was made from, and
    /// says that the fork was taken away, as one write: its file, at
    /// `path`, is gone. `was` is what was kept of it, where anything was;
    /// `lost` whether its text said something else than what ships.
    pub async fn fork_gone(
        &self,
        project: &str,
        blueprint: &str,
        (path, lost): (&str, bool),
        was: Option<&Origin>,
        (by, source): (&kernel::Principal, Source),
    ) -> Result<()> {
        let at = key(project, blueprint);
        let said = json!({
            "project": project,
            "blueprint": blueprint,
            "path": path,
            "version": was.and_then(|origin| origin.version.clone()),
            "lost": lost,
            "by": by.actor(),
            "by_name": by.name,
        });
        let journal = self.keeper_of(project).await.inner.own.clone();
        let write = journal.write(move |writing| {
            writing.forget(NAMESPACE, &at)?;
            writing.say("project.unforked", source, said)
        });
        write.await.map(|_| ())
    }
}
