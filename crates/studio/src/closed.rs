//! Which folder of its project a table keeps its records in as files,
//! for what a table keeps private (`reads`): whoever may read a project's
//! files could read them there, so the folder is kept with the table, by
//! the projects (`projects::Projects::keep`), which are told it as a table
//! is declared and read.

use crate::Projects;
use crate::collections::{Collection, Store};

impl Collection {
    /// The folder of its project its records are kept in as files, from
    /// the top of the project (`""` for the project itself); nothing, for
    /// one kept in a database or by a store service.
    pub fn folder(&self) -> Option<String> {
        match self.store {
            Store::Documents | Store::Mockups => Some(crate::documents::folder(self).to_string()),
            Store::Files => Some(self.name.clone()),
            Store::Local | Store::Service => None,
        }
    }

    /// Tells `projects` what reading the folder it keeps its records in
    /// takes, as it is declared or read.
    pub(crate) fn say_kept(&self, projects: &Projects) {
        projects.keep(&self.project, &self.name, self.folder(), self.reads.as_deref());
    }
}
