//! Why something failed, as it is passed up: the whole of it, and the
//! reason at the bottom.
//!
//! A step that fails fails its run, and each run a failure comes up
//! through puts the name of its step in front ([`within`]). The whole of
//! it says where; the reason alone ([`reason`]) is what a person is told,
//! who has no use for the names of steps inside a blueprint they did not
//! write. Both are a matter of the words alone, and whatever says a
//! failure, in whichever crate, says it by these.
//!
//! What a failure comes up through is known too: the children that `call`
//! steps are waiting on ([`Calls`]), so that one that fails says which
//! instance called it and whether that step lets it fail.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// What a run that `step` failed says of itself: where, then why.
pub fn within(step: &str, error: &str) -> String {
    format!("step `{step}` failed: {error}")
}

/// The reason at the bottom of `error`: what the innermost step said,
/// without the steps it came up through.
pub fn reason(error: &str) -> &str {
    let mut rest = error;
    while let Some(inner) =
        rest.strip_prefix("step `").and_then(|named| named.split_once("` failed: ")).map(|(_, inner)| inner)
    {
        rest = inner;
    }
    rest
}

/// The children that `call` steps are waiting on now, each by the
/// instance that called and the name it called it by.
#[derive(Clone, Default)]
pub struct Calls {
    waited: Arc<Mutex<Waited>>,
}

/// Each child that is waited on, by its parent and its name: how many
/// steps wait on it, and whether one of them lets it fail.
type Waited = HashMap<(String, String), (usize, bool)>;

/// A `call` that is waiting; it waits no longer when this is dropped.
pub struct Call {
    calls: Calls,
    child: (String, String),
}

impl Calls {
    /// Said before the child is asked for, so that one that fails at once
    /// is already known to be waited on; `lets` is whether the step that
    /// waits lets it fail.
    pub fn waiting(&self, parent: &str, key: &str, lets: bool) -> Call {
        let child = (parent.to_string(), key.to_string());
        let mut waited = self.waited.lock().expect("calls lock");
        let (count, let_be) = waited.entry(child.clone()).or_default();
        (*count, *let_be) = (*count + 1, *let_be || lets);
        Call { calls: self.clone(), child }
    }

    /// The instance whose step is waiting on the child of `parent` called
    /// `key`, and whether that step lets it fail: none when nothing
    /// called for it.
    pub fn of(&self, parent: Option<&str>, key: &str) -> Option<(String, bool)> {
        let parent = parent?.to_string();
        let waited = self.waited.lock().expect("calls lock");
        let (_, lets) = waited.get(&(parent.clone(), key.to_string()))?;
        Some((parent, *lets))
    }
}

impl Drop for Call {
    fn drop(&mut self) {
        let mut waited = self.calls.waited.lock().expect("calls lock");
        if let Some((count, _)) = waited.get_mut(&self.child) {
            *count -= 1;
            if *count == 0 {
                waited.remove(&self.child);
            }
        }
    }
}
