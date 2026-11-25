//! How deep a tree may go: the chain of parents an instance has, which a
//! step that starts a child counts, and what that is held to, which is a
//! function of plain values and the kernel's (`kernel::deepest`, offered
//! here under the names it always had).

pub use kernel::deepest::{DEEPEST, too_deep};

use crate::engine::Engine;

impl Engine {
    /// The blueprint of the instance and of each above it, the instance's
    /// first: from what is in hand, which every instance of a running
    /// tree is, and from its row where it is not. Never more than one past
    /// the bound, which is all that is asked of it.
    pub(crate) async fn chain_above(&self, instance: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut at = Some(instance.to_string());
        while let Some(id) = at.take() {
            if chain.len() > DEEPEST {
                break;
            }
            let (blueprint, parent) = match self.live(&id) {
                Some(live) => {
                    let meta = live.meta();
                    (meta.blueprint, meta.parent)
                }
                None => match self.inner.store.instance(&id).await.ok().flatten() {
                    Some(info) => (info.blueprint, info.parent),
                    None => break,
                },
            };
            chain.push(blueprint);
            at = parent;
        }
        chain
    }
}
