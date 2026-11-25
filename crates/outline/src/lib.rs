//! How each project's things are arranged in the side panel: its outline
//! (`kernel::outline`), kept by the factory for everybody who reads the
//! project, and changed by whoever may write its files.
//!
//! **Kept** in the factory's small durable state (`kernel::outline::NAMESPACE`,
//! by the project's id), as where a project keeps its own blueprints is:
//! it is how the project is shown, the factory's whatever journal keeps
//! the project's records. So it goes into a copy of the factory
//! (`studio::copy::KEPT`), stays where it is when a project is moved to a
//! journal of its own, is kept with a project in the Trash, and goes with
//! one deleted for good, as one made anew under its id begins with none
//! (`studio`'s `placed.rs`, `trash.rs`). A project nobody arranged has no
//! outline, and is drawn as it always was.
//!
//! **It acts on nothing.** Nothing reads a folder to decide who may read
//! what, and placing a thing in a folder gives nobody anything: a table
//! is the table it was, wherever it is shown.
//!
//! **Read as whoever reads it may see** ([`Outlines::read`]): every thing
//! of a table, library or mockups they may not read is taken out of what
//! they are handed, and every folder is left in, so that nobody learns
//! the name of something kept from them. So it is **changed by operations
//! and never written back whole** ([`Outlines::change`]): somebody who
//! wrote back what they were shown would take away what they were not.
//! What an operation names, and what the page saw, is held to what they
//! may see the same way: a thing they may not read is no thing they may
//! place or name, said in the same words as one that is not there. What a
//! thing is shown as goes with the thing: nobody who may not read it is
//! handed it.
//!
//! **One write with what is said**: the outline as it is is read inside
//! the write that changes it, the change is worked out from that, and
//! `project.arranged` is said in it ([`events`]), saying which project and
//! how many times it was written and never the outline, which may name
//! what some who hear it may not read. A change that comes to what is
//! kept writes nothing and says nothing.

pub mod actions;
pub mod events;

use journal::Journal;
use kernel::outline::{Applied, NAMESPACE, Op, Outline, apply, checked, is_folder, placed_in};
use kernel::{Error, Principal, Result, Source};
use serde_json::{Value, json};
use studio::Studio;

/// What reading an outline takes.
pub const READ: &str = "projects:read";
/// What changing one takes: what writing the project's files takes.
pub const WRITE: &str = "projects:write";

/// What this crate says, among what the factory describes, and what a
/// blueprint asks of it (`outline.put`).
pub fn register(registry: &mut conductor::Registry, outlines: Outlines) {
    actions::register(registry, outlines);
    registry.emits(events::events());
}

/// What a change came to, as whoever asked may see it.
#[derive(Clone, Debug, PartialEq)]
pub struct Changed {
    /// The outline as it is now, as they may see it: nothing where there is none.
    pub outline: Option<Outline>,
    pub changed: bool,
    /// How many times it has been written.
    pub nth: i64,
    /// The folder made, or what was removed.
    pub done: Value,
}

/// The keeper of every project's outline.
#[derive(Clone)]
pub struct Outlines {
    journal: Journal,
    studio: Studio,
}

/// What `who` may see of a project, by the key of each of its things.
struct Seen {
    project: String,
    /// The tables, libraries and mockups of the project that `who` may read, by name.
    readable: Vec<String>,
}

impl Outlines {
    pub fn new(journal: Journal, studio: Studio) -> Outlines {
        Outlines { journal, studio }
    }

    async fn seen_by(&self, project: &str, who: &Principal) -> Result<Seen> {
        let readable = self
            .studio
            .collections
            .list(Some(project))
            .await?
            .into_iter()
            .filter(|collection| collection.may_read(who).is_ok())
            .map(|collection| collection.name)
            .collect();
        Ok(Seen { project: project.to_string(), readable })
    }

    /// Whether `who` may see the thing `key` names: a table, a library
    /// or mockups only where it is one they may read, an entry of the
    /// project's folder only where it is not kept from them, and the
    /// project's own lines always.
    fn may_see(&self, seen: &Seen, key: &str, who: &Principal) -> bool {
        if key == "board" {
            return seen.readable.iter().any(|name| name == "board");
        }
        for begins in ["table:", "library:", "mockups:"] {
            if let Some(name) = key.strip_prefix(begins) {
                return seen.readable.iter().any(|one| one == name);
            }
        }
        if let Some(path) = key.strip_prefix("entry:") {
            return !self.studio.projects.is_kept_from(&seen.project, path, who);
        }
        true
    }

    /// The outline as it is kept, whoever asks: nothing where there is none.
    pub async fn kept(&self, project: &str) -> Result<Option<Outline>> {
        match self.journal.kv_get(NAMESPACE, project).await? {
            None => Ok(None),
            Some(kept) => checked(&kept).map(Some).map_err(Error::invalid),
        }
    }

    /// `project`'s outline as `who` may see it: nothing where nobody arranged it.
    pub async fn read(&self, project: &str, who: &Principal) -> Result<Option<Outline>> {
        who.require(READ)?;
        let project = self.studio.projects.get(project).await?.id;
        let seen = self.seen_by(&project, who).await?;
        Ok(self.kept(&project).await?.map(|kept| kept.kept_to(&|key| self.may_see(&seen, key, who))))
    }

    /// Does `op` to `project`'s outline as `who`, with `seen` (what the
    /// page drew at the top that the outline does not name, in the order
    /// drawn) named first: kept and said in one write, or refused in a
    /// sentence with nothing written.
    pub async fn change(&self, project: &str, op: Op, seen: Vec<String>, who: &Principal) -> Result<Changed> {
        who.require(WRITE)?;
        let project = self.studio.projects.get(project).await?.id;
        let sees = self.seen_by(&project, who).await?;
        let mut named: Vec<&str> = Vec::new();
        match &op {
            Op::Make { at, .. } => named.extend(at.before.as_deref()),
            Op::Put { nodes, at } => {
                named.extend(nodes.iter().map(String::as_str));
                named.extend(at.before.as_deref());
            }
            Op::Restore { removed } => {
                named.extend(removed.items.iter().map(String::as_str));
                named.extend(removed.before.as_deref());
            }
            Op::Name { thing, .. } => named.push(thing),
            _ => {}
        }
        if let Some(hidden) = named.into_iter().find(|key| !is_folder(key) && !self.may_see(&sees, key, who))
        {
            return Err(self.unseen(&project, hidden, who));
        }
        // What the page saw is what it may see: anything else it says is passed over.
        let seen: Vec<String> = seen.into_iter().filter(|key| self.may_see(&sees, key, who)).collect();
        self.written(&project, &sees, who, move |kept| apply(kept, &op, &seen)).await
    }

    /// Puts `things` in the folder at `path` of `project`'s outline (names
    /// from the top, whatever their capitals; the top where it is empty),
    /// as `who`, making each folder of the path that is not there: the
    /// same done twice, so that a blueprint that declares a table can
    /// show it in a folder each time it is applied (`outline.put`).
    pub async fn put_in(
        &self,
        project: &str,
        path: Vec<String>,
        things: Vec<String>,
        who: &Principal,
    ) -> Result<Changed> {
        who.require(WRITE)?;
        let project = self.studio.projects.get(project).await?.id;
        let sees = self.seen_by(&project, who).await?;
        if let Some(hidden) = things.iter().find(|key| !self.may_see(&sees, key, who)) {
            return Err(self.unseen(&project, hidden, who));
        }
        self.written(&project, &sees, who, move |kept| placed_in(kept, &path, &things)).await
    }

    fn unseen(&self, project: &str, key: &str, who: &Principal) -> Error {
        Error::not_found(format!(
            "`{key}` is nothing of `{project}`'s that {} may read, and nothing is put where it is not: say a \
             thing the project has, as `{} project outline {project}` lists",
            who.name,
            brand::CLI
        ))
    }

    /// What `work` makes of the outline as it is kept, read inside the
    /// write that keeps it, kept and said there; or refused in a sentence
    /// with nothing written. Handed back as `who` may see it.
    async fn written(
        &self,
        project: &str,
        sees: &Seen,
        who: &Principal,
        work: impl FnOnce(Option<&Outline>) -> std::result::Result<Applied, String> + Send + 'static,
    ) -> Result<Changed> {
        let (key, by) = (project.to_string(), who.actor());
        let (came, _) = self
            .journal
            .write(move |writing| {
                let kept = match writing.kept(NAMESPACE, &key)? {
                    None => None,
                    Some(kept) => match checked(&kept) {
                        Ok(kept) => Some(kept),
                        Err(wrong) => return Ok(Err(wrong)),
                    },
                };
                let nth = kept.as_ref().map_or(0, |kept| kept.nth);
                let applied = match work(kept.as_ref()) {
                    Ok(applied) => applied,
                    Err(refused) => return Ok(Err(refused)),
                };
                if !applied.changed {
                    return Ok(Ok((applied, nth)));
                }
                let nth = nth + 1;
                let Applied { outline, changed, done } = applied;
                let outline = outline.map(|outline| Outline { nth, ..outline });
                match &outline {
                    Some(outline) => writing.keep(NAMESPACE, &key, &outline.to_value())?,
                    None => writing.forget(NAMESPACE, &key)?,
                }
                writing.say_here(
                    "project.arranged",
                    Source::actor(by.clone()),
                    json!({ "project": key, "nth": nth, "by": by, "forgot": outline.is_none() }),
                )?;
                Ok(Ok((Applied { outline, changed, done }, nth)))
            })
            .await?;
        let (applied, nth) = came.map_err(Error::invalid)?;
        let mut done = applied.done;
        // What was in a folder removed is said as they may see it.
        if let Some(items) = done.pointer_mut("/removed/items").and_then(Value::as_array_mut) {
            items.retain(|item| {
                item.as_str().is_some_and(|key| is_folder(key) || self.may_see(sees, key, who))
            });
        }
        Ok(Changed {
            outline: applied.outline.map(|kept| kept.kept_to(&|key| self.may_see(sees, key, who))),
            changed: applied.changed,
            nth,
            done,
        })
    }
}
