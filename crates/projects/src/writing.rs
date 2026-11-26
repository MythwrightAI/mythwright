//! The files being written through the factory, which a look over a
//! project's folder leaves to whoever writes them.
//!
//! A file written through here is put on the disk first and written down
//! as the project's after ([`crate::Projects::write`]). A look in between
//! (one a run that ended asked for, or a library's folder said had
//! changed, which the write itself makes it say) found it on the disk and
//! not written down, and said it had been added, or changed, by nobody:
//! the system. The write then said it again, as whoever wrote it, but what
//! hears such a thing had done what the file asks in nobody's name (a
//! plan's cards said nothing of the agent that wrote the plan). So a file
//! is held here from before it reaches the disk until it is written down,
//! and a look says nothing of it meanwhile, nor writes it down: the write
//! does both, as who it is.
//!
//! A look asks only once it is on the database's thread, and after it has
//! looked at the file as it is then: a file written through here that it
//! saw is held from before it was on the disk, and stays held until its
//! own write is done on that thread, which waits behind the look. Asked
//! any earlier, a write could begin, and be looked at, after the asking.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

/// The files of each project being written through the factory: how many
/// writes of each are under way.
#[derive(Clone, Default)]
pub(crate) struct Writing(Arc<Mutex<HashMap<(String, String), u32>>>);

impl Writing {
    /// Holds `path` of `project` until what is handed back is dropped.
    pub(crate) fn hold(&self, project: &str, path: &str) -> Held {
        let key = (project.to_string(), path.to_string());
        *self.0.lock().unwrap_or_else(PoisonError::into_inner).entry(key.clone()).or_default() += 1;
        Held { writing: self.clone(), key }
    }

    /// Whether `path` of `project` is being written through the factory.
    pub(crate) fn holds(&self, project: &str, path: &str) -> bool {
        let key = (project.to_string(), path.to_string());
        self.0.lock().unwrap_or_else(PoisonError::into_inner).contains_key(&key)
    }
}

/// A write of a file under way: over when it is dropped, however the
/// write ended.
pub(crate) struct Held {
    writing: Writing,
    key: (String, String),
}

impl Drop for Held {
    fn drop(&mut self) {
        let mut held = self.writing.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = held.get_mut(&self.key) {
            *count -= 1;
            if *count == 0 {
                held.remove(&self.key);
            }
        }
    }
}
