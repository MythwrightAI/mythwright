//! A studio that reaches nothing but what it was handed.
//!
//! A studio is tables in a journal, and four things besides: a new
//! project is made a repository, which is a program started; a table may
//! be kept by a store service, which is an address requests are sent
//! to, or as files of its project (one file a record, a library's
//! documents, a project's mockups); and what is in a project's
//! repository is asked of git. A studio opened here does none of the
//! four. It is what a sandbox test of a blueprint is given: a journal of
//! the sandbox's own and one folder, and what it keeps is all it can
//! touch. Every table is kept in the journal, so nothing it does with
//! one is work on files; the folder is where its one project is said to
//! be, and what is attached to a message would go.

use std::path::Path;

use conductor::Registry;
use journal::{Db, Journal};
use kernel::{Error, Result};

use crate::{Projects, Store, Studio};

impl Studio {
    /// A studio over `journal` that keeps its files under `folder`,
    /// starts no program and asks no service.
    pub async fn open_within(journal: Journal, search_db: Db, folder: &Path) -> Result<Studio> {
        let studio =
            Studio::open(journal, search_db, folder.join("projects"), folder.join("attached")).await?;
        studio.projects.reach_nothing();
        Ok(studio)
    }

    /// Adds the actions that keep to the studio's tables, and the ones
    /// that only work something out, to `registry`: every action of
    /// [`Studio::register`] but those that reach a project's files. What
    /// this crate emits is not said: whoever asks says what is emitted.
    pub fn register_kept(&self, registry: &mut Registry) {
        crate::actions::kept(registry, self);
    }
}

/// Refuses a table kept anywhere but in the journal, where `projects`
/// reach nothing but what they were handed: by a store service, which is
/// an address requests are sent to, or as files of a project.
pub(crate) fn keeps(projects: &Projects, store: Store) -> Result<()> {
    if !projects.is_within() || store == Store::Local {
        return Ok(());
    }
    let (kept, instead) = match store {
        Store::Service => {
            ("by a store service", "Run with `store: local`, what is done with its records is the same")
        }
        Store::Files => ("as files", "Run with `store: local`, what is done with its records is the same"),
        Store::Documents | Store::Mockups | Store::Local => (
            "as the files of a folder",
            "A library's documents and a project's mockups are files of a project, and a sandbox has none of a \
             project's files: what is done with them is not shown there",
        ),
    };
    Err(Error::invalid(format!(
        "a table kept {kept} is not made here: this is a sandbox, which keeps its tables in a journal of \
         its own and reaches nothing outside it. {instead}"
    )))
}
