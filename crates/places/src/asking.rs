//! Two gits in one repository, and what is kept of what the daemon asks of
//! git for places ([`crate`]) so that they do not meet.
//!
//! - **A daemon that is stopping lets git finish** ([`Places::settle`]):
//!   nothing more is asked, and what is being asked is waited for.
//! - **What the daemon itself writes in one repository is done one at a
//!   time.** Git guards what it writes with locks and does not wait at
//!   them: of two that write at once the second is refused. A repository's
//!   worktrees share most of what is guarded (the branches, the settings,
//!   what is known of each worktree), so an action that writes (a place
//!   made or cleared, what was left there kept, a merge) takes the turn of
//!   the repository, known by the folder its worktrees share, and holds it
//!   for as long as it lasts: it is several things asked of git, each
//!   resting on the answer before. Reading takes no turn, and neither does
//!   what speaks to a remote, which may be slow: nothing of one repository
//!   waits on a remote for another card.
//! - **A lock met all the same is somebody else's**: a person's own git
//!   at work in the project's folder, or an agent's program in its
//!   worktree. It is waited for, and what was refused is asked again
//!   (`Place::patiently`, by `kernel::git::LOCK_PATIENCE`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::Places;

/// What is being asked of git for places just now, whether anything more
/// may be, and whose turn it is in each repository.
#[derive(Default)]
pub(crate) struct Asking {
    in_flight: AtomicUsize,
    stopping: AtomicBool,
    ended: tokio::sync::Notify,
    /// The turn of each repository, by the folder its worktrees share.
    turns: Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>,
}

/// One thing being asked of git, for as long as it is.
pub(crate) struct InFlight(Arc<Asking>);

impl Drop for InFlight {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::SeqCst);
        self.0.ended.notify_waiters();
    }
}

impl Asking {
    /// Counts one more thing asked of git, for as long as what is handed
    /// back is kept; nothing, where the daemon is stopping and git is
    /// asked nothing more.
    pub(crate) fn begin(self: &Arc<Self>) -> Option<InFlight> {
        self.in_flight.fetch_add(1, Ordering::SeqCst);
        let asked = InFlight(self.clone());
        (!self.stopping.load(Ordering::SeqCst)).then_some(asked)
    }

    /// The turn of the repository whose worktrees share `common`.
    pub(crate) fn turn(&self, common: &Path) -> Arc<tokio::sync::Mutex<()>> {
        self.turns.lock().expect("git lock").entry(common.to_path_buf()).or_default().clone()
    }
}

impl Places {
    /// For a daemon that was asked to stop, before it stops what runs:
    /// nothing more is asked of git for a place from now, and what is
    /// being asked is waited for, `most` at the longest. Git stopped in
    /// the middle leaves a worktree half made, a lock on an index or a
    /// merge not committed, which is what this is for; git that takes
    /// longer than that (a remote that does not answer) is stopped with
    /// the daemon after all. Whatever would ask git something more waits
    /// instead, and is taken away with everything else.
    pub async fn settle(&self, most: Duration) {
        let asking = &self.git;
        asking.stopping.store(true, Ordering::SeqCst);
        let over = async {
            loop {
                // Listening before looking, so that an ending in between is heard.
                let ended = asking.ended.notified();
                if asking.in_flight.load(Ordering::SeqCst) == 0 {
                    return;
                }
                ended.await;
            }
        };
        let _ = tokio::time::timeout(most, over).await;
    }
}
