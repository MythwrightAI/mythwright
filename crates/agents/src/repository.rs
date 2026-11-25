//! What an agent may do with its project's repository, behind its wall:
//! the wall's (`hosting::repository`, offered here under the name it
//! always had), and whether its project keeps something from it that the
//! repository holds, which is asked of the studio.

pub use hosting::repository::{Git, show};

use crate::{Agents, Slot};

impl Agents {
    /// Whether `slot`'s project keeps something from it that its
    /// repository holds: then the repository is not shown to it at all.
    /// Asked of git, which takes a moment, and only where something is
    /// kept from it.
    pub(crate) async fn kept_is_held(&self, slot: &Slot) -> bool {
        let Some(project) = &slot.project else { return false };
        if !slot.wall || slot.git == Git::Off.word() {
            return false;
        }
        let projects = &self.inner.studio.projects;
        let kept = projects.kept_from(project, &slot.principal());
        !kept.is_empty() && projects.history_holds(project, &kept).await
    }
}
