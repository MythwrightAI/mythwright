//! What is written with a project's row as it is made and as it is
//! forgotten, by what keeps where things belong.
//!
//! Every project is in a workspace from the moment it is there, and its
//! channels are somewhere from the moment it is gone, so each is the one
//! write with the project's row. Which workspace that is, is kept by
//! `workspaces`, which knows of the projects only their table: whoever
//! opens both (`studio::Studio::open`) says here how a project is placed
//! and what is done as one is forgotten, and `Projects` asks in its own
//! write.
//!
//! And where a project keeps its own blueprints (`texts::library::folders`),
//! and how its things are arranged in the side panel (`kernel::outline`):
//! one made anew begins with nothing said of either, and one deleted for
//! good takes both with it (also `crate::trashed::take_out`), in the write of
//! its row.

use std::sync::{Arc, OnceLock};

use journal::Writing;
use kernel::Source;
use texts::library::folders::{Folder, Folders, NAMESPACE};

/// What gives a project its place in the write that makes its row. It is
/// handed the write, the project's id, the workspace whoever adds it
/// asked for and who that is, and says in that write where it put the
/// project.
pub type Placing =
    dyn Fn(&mut Writing<'_>, &str, Option<&str>, &Source) -> rusqlite::Result<()> + Send + Sync;

/// What is done, in the write that forgets a project, with what was
/// placed by it: the project is taken out of its workspace, and its
/// channels are said to be somewhere. It is handed the write, the
/// project and who forgets it.
pub type Forgetting = dyn Fn(&mut Writing<'_>, &str, &Source) -> rusqlite::Result<()> + Send + Sync;

/// The two, each said once by whatever keeps where things belong; a
/// second saying is not taken. With neither said a project is made and
/// forgotten alone.
#[derive(Clone, Default)]
pub struct Placed {
    making: Arc<OnceLock<Arc<Placing>>>,
    forgetting: Arc<OnceLock<Arc<Forgetting>>>,
    folders: Folders,
}

impl crate::Projects {
    /// Where each project keeps its own blueprints, as held in memory: read
    /// by whoever serves, and told whenever that changes.
    pub fn folders(&self) -> &Folders {
        &self.placed.folders
    }

    /// What is written with a project's row as it is made and as it is
    /// forgotten, for whatever keeps where things belong to say.
    pub fn placed(&self) -> &Placed {
        &self.placed
    }
}

impl Placed {
    pub fn makes_with(&self, placing: Arc<Placing>) {
        let _ = self.making.set(placing);
    }

    pub fn forgets_with(&self, forgetting: Arc<Forgetting>) {
        let _ = self.forgetting.set(forgetting);
    }

    pub(crate) fn making(&self) -> Option<Arc<Placing>> {
        self.making.get().cloned()
    }

    pub(crate) fn forgetting(&self) -> Option<Arc<Forgetting>> {
        self.forgetting.get().cloned()
    }

    /// What is written with the row of a project made anew, and not given
    /// back from the Trash: nothing said of where it keeps its own
    /// blueprints, and its place.
    pub(crate) fn anew(
        &self,
    ) -> impl Fn(&mut Writing<'_>, &str, Option<&str>, &Source) -> rusqlite::Result<()> + Send + 'static {
        let placing = self.making();
        move |writing, project, asked, by| {
            writing.forget(NAMESPACE, project)?;
            writing.forget(kernel::outline::NAMESPACE, project)?;
            placing.as_ref().map_or(Ok(()), |placing| placing(writing, project, asked, by))
        }
    }

    /// What is written with the row of a project that is forgotten, and
    /// where it is deleted for good and not kept in the Trash, that it
    /// said nothing of where it keeps its own blueprints.
    pub(crate) fn forgotten(
        &self,
    ) -> impl Fn(&mut Writing<'_>, &str, &Source, bool) -> rusqlite::Result<()> + Send + 'static {
        let forgetting = self.forgetting();
        move |writing, project, by, for_good| {
            if for_good {
                writing.forget(NAMESPACE, project)?;
                writing.forget(kernel::outline::NAMESPACE, project)?;
            }
            forgetting.as_ref().map_or(Ok(()), |forgetting| forgetting(writing, project, by))
        }
    }

    /// `project` was made, or deleted for good, and that is kept.
    pub(crate) fn said_nothing(&self, project: &str) {
        self.folders.put(project, Folder::Usual);
    }
}
