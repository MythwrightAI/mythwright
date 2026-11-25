//! What a key applied, found and stopped as the key is withdrawn.
//!
//! What an instance runs with says which key it was applied with
//! (`Principal::key`), and what an instance starts runs as it does. So
//! what a key applied that is alive is found among what is alive, by
//! that, in every conductor there is; and what is listed of it is each
//! instance the key applied and not what that started in turn, which is
//! stopped with it. Nothing else keeps instances by key: the look is over
//! what is alive (`runs::Store::instances` reads that from the table of
//! what is alive, never every instance there was), once for a key
//! withdrawn.

use kernel::Result;

use crate::Engine;
use crate::{InstanceFilter, InstanceInfo};

impl Engine {
    /// What any of `keys` applied that is alive here: each instance one of
    /// them applied whose parent none of them applied, the newest first.
    /// What a step of one of theirs applied on another device lives in
    /// that device's runner and is not here; what was applied before keys
    /// were kept with what they applied names none.
    pub async fn applied_with(&self, keys: &[String]) -> Result<Vec<InstanceInfo>> {
        let filter = InstanceFilter { live_only: true, limit: 100_000, ..Default::default() };
        let alive = self.instances(filter).await?;
        let theirs = |one: &InstanceInfo| {
            one.principal.key.as_ref().is_some_and(|key| keys.iter().any(|it| it == key))
        };
        let parent_theirs = |one: &InstanceInfo| {
            one.parent.as_ref().is_some_and(|parent| alive.iter().any(|up| up.id == *parent && theirs(up)))
        };
        Ok(alive.iter().filter(|one| theirs(one) && !parent_theirs(one)).cloned().collect())
    }

    /// Stops an instance because the key it was applied with was
    /// withdrawn: as a stop asked for by somebody does, its children and
    /// what it holds with it, said as `withdrawn` (`why` of
    /// `instance.stopped`).
    pub async fn stop_withdrawn(&self, id: &str) -> Result<()> {
        self.stop_as(id, "withdrawn").await
    }
}
