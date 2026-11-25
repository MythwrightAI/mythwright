//! Letting go whoever waits at a hold (`holds.rs`, beside this), once
//! somebody does: for the suites that say so, each of which names this
//! file (`#[path]`) beside that one.

use std::path::Path;

use super::holds::{WITHIN, made};

/// Waits until somebody is waiting at the hold at `path`, and lets go
/// everybody who is. `what` is what a failure says was waited for.
pub async fn let_go(what: &str, path: &Path) {
    let hold = made(path);
    let waited = async {
        loop {
            let came = hold.came.notified();
            tokio::pin!(came);
            // Listening before looking, so that one who comes between
            // the two is heard of.
            came.as_mut().enable();
            let waiting = std::mem::take(hold.state.lock().unwrap().there());
            if !waiting.is_empty() {
                return;
            }
            came.await;
        }
    };
    if tokio::time::timeout(WITHIN, waited).await.is_err() {
        panic!("timed out waiting for: {what}");
    }
}
