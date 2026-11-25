//! Projects that reach nothing but what they were handed: a new project
//! is made a repository by no program started, and nothing of its
//! repository is asked of git. It is what a sandbox test of a blueprint is
//! given (`studio::Studio::open_within`), where a table kept anywhere but
//! in the journal is refused too, which is the studio's to say.

use std::sync::atomic::Ordering;

use crate::Projects;

impl Projects {
    /// From here on, nothing is reached but what this was handed.
    pub fn reach_nothing(&self) {
        self.within.store(true, Ordering::SeqCst);
    }

    /// Whether this reaches nothing but what it was handed.
    pub fn is_within(&self) -> bool {
        self.within.load(Ordering::SeqCst)
    }
}
