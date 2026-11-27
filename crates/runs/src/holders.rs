//! Which instance was last made with each text, by project: the one a
//! new instance of the same text names for its text, where it keeps that
//! text still ([`crate::store::record_version`]).
//!
//! Only a guess, checked where it is used: the write that makes an
//! instance looks at the version the one named runs, and names it only
//! where it keeps the very text being written, in the same project.
//! Anything else (one moved apart, one given another text since, one of
//! a factory started again, which knows none) has the text written in
//! full, as every one was before.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// How many texts are remembered at most, before all are let go: more
/// than a factory's projects run.
const KEPT: usize = 4096;

#[derive(Clone, Default)]
pub(crate) struct Holders(Arc<Mutex<HashMap<(String, String), String>>>);

impl Holders {
    /// The instance last made with the text `revision` in `project`.
    pub(crate) fn of(&self, project: Option<&str>, revision: &str) -> Option<String> {
        let key = (project.unwrap_or_default().to_string(), revision.to_string());
        self.0.lock().expect("holders lock").get(&key).cloned()
    }

    /// Says that `instance` was made with the text `revision` in `project`.
    pub(crate) fn made(&self, project: Option<&str>, revision: &str, instance: &str) {
        let mut holders = self.0.lock().expect("holders lock");
        if holders.len() >= KEPT {
            holders.clear();
        }
        holders.insert((project.unwrap_or_default().to_string(), revision.to_string()), instance.to_string());
    }
}
