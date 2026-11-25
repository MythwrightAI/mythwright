//! What a project's fork of a blueprint that ships was forked from.
//!
//! A blueprint that ships is changed for one project by being copied
//! into it: the copy is a file of the project's, and is what runs there
//! under the name (`crate::library`, a project's own before what ships).
//! That copy is a **fork** of the one that ships. From the moment it is
//! changed, nothing in its text says what it was a copy of: which
//! version, which text exactly. And the one that ships moves when the
//! program does, so that what it was copied from is soon nowhere.
//!
//! So what a fork was made from is kept when it is made ([`Origin`]):
//! which blueprint, the version that text called itself, **the text
//! itself, to the letter**, where the copy was written, who made it and
//! when. The text is kept whole in the record and not by its revision
//! among the texts trees are held to ([`crate::pinned`]): those are put
//! away when no tree is held to them and are left out of a copy of a
//! factory, and this one has to be there for as long as the fork is,
//! whatever program is running by then, to be set beside the fork and
//! beside what ships now.
//!
//! It is kept by project and name, in the journal of the conductor that
//! runs what is applied for the project (`runs::beside::FORKED`), so
//! that it is copied and moved with the project. Nothing here says an
//! event or writes a file: the fork is made, and taken away, by whoever
//! writes the project's files, and the runner writes this with what is
//! said of it, as one write (`conductor::forked`).
//!
//! **A record is never wrong.** A fork an earlier program made has none,
//! and nothing makes one up: what is said of such a fork is worked out
//! from texts each time it is asked. One that is being taken away says
//! so before its file goes ([`Origin::leaving`]), and is then read as no
//! record: a daemon stopped between the two leaves a file with nothing
//! said of where it came from, never a record that outlived its file to
//! be found by a file somebody wrote there later.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Texts;

/// Where what each fork was made from is kept.
pub const NAMESPACE: &str = runs::beside::FORKED;

/// What a fork was made from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Origin {
    /// The blueprint that ships it is a fork of, by its name: the name
    /// the fork runs under.
    pub blueprint: String,
    /// The version the text it was copied from called itself, where it
    /// said one.
    #[serde(default)]
    pub version: Option<String>,
    /// The revision of that text.
    pub revision: String,
    /// That text, to the letter.
    pub source: String,
    /// Where the copy was written, within the project.
    pub path: String,
    /// Who made it, as an actor, and by name.
    pub by: String,
    pub by_name: String,
    /// When, in milliseconds.
    pub at: i64,
    /// It is being taken away: its file is about to go, and what is kept
    /// here is no longer said of whatever file is there.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub leaving: bool,
}

impl Origin {
    /// What a copy of `source`, which ships as `blueprint`, was made
    /// from, written at `path` by `by` at `at`. Words a blueprint links
    /// say no version, whatever a line of them looks like.
    pub fn of(blueprint: &str, source: &str, path: &str, by: &kernel::Principal, at: i64) -> Origin {
        let words = crate::library::is_linked(blueprint);
        Origin {
            blueprint: blueprint.to_string(),
            version: if words { None } else { blueprint::version(source) },
            revision: blueprint::revision(source),
            source: source.to_string(),
            path: path.to_string(),
            by: by.actor(),
            by_name: by.name.clone(),
            at,
            leaving: false,
        }
    }

    /// As it is kept.
    pub fn kept(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

/// What a fork's record is kept under: its project and the name it runs
/// under, as everything kept by project is (`runs::beside::project_of`).
pub fn key(project: &str, blueprint: &str) -> String {
    format!("{project}\n{blueprint}")
}

impl Texts {
    /// The record of `project`'s fork of `blueprint` as it is kept,
    /// whatever it says: for whoever is taking the fork away.
    pub async fn fork_kept(&self, project: &str, blueprint: &str) -> Option<Origin> {
        let kept = self.own.kv_get(NAMESPACE, &key(project, blueprint)).await.ok().flatten()?;
        serde_json::from_value(kept).ok()
    }

    /// What `project`'s fork of `blueprint` was made from, where that
    /// was kept and still stands: nothing for a fork an earlier program
    /// made, and for one that is being taken away.
    pub async fn forked_from(&self, project: &str, blueprint: &str) -> Option<Origin> {
        self.fork_kept(project, blueprint).await.filter(|origin| !origin.leaving)
    }
}
