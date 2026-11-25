//! What an instance keeps, and that a step set it, written as one thing.
//!
//! Setting is one at a time for an instance, and all of it: reading what
//! is kept, working out what to set from it, writing it down and saying
//! so. Two handlers, or two steps side by side, that each add one to a
//! count each read the count the other left, and what is written down and
//! said of each change is in the order the changes were made.
//!
//! What is set is written in one transaction with the record that the
//! step which set it has acted, and what it gave ([`Done`]): the step's
//! own record, the element's where the step is done for each of a list,
//! the count of entries done where it is one of those done around a step,
//! as a run ends, as a text is taken or as an instance stops. After any stop either both are there, and the step
//! is not acted again; or neither is, and doing it again is doing it
//! once. So `${ state.n + 1 }` adds once whoever else set the same
//! instance in between, and nothing is kept of who set what. Anything
//! else that changes what an instance keeps by what is there (one thing
//! added to a list, one taken from it) goes the same way, through
//! [`Engine::set_state`].
//!
//! An instance that keeps nothing as it goes has its state in hand and
//! nowhere else: stopped, it is begun again whole, from what it began
//! with, so there is nothing of it to set twice.

use kernel::{Error, Fields, Source};
use runs::{Done, to_say};
use serde_json::Value;

use crate::engine::{Engine, Live};

impl Engine {
    /// Changes keys of an instance's durable state to what `work_out`
    /// comes to, given the state as it is, and hands back what was set.
    /// `done` says, of what was set, which record is written with it.
    pub(crate) async fn set_state<E: From<Error>>(
        &self,
        live: &Live,
        work_out: impl FnOnce(&Fields) -> std::result::Result<Fields, E>,
        done: impl FnOnce(&Fields) -> Option<Done>,
    ) -> std::result::Result<Fields, E> {
        let _setting = live.setting.lock().await;
        let changes = work_out(&live.meta().state)?;
        let state = {
            let mut meta = live.meta.lock().expect("live lock");
            for (key, value) in &changes {
                meta.state.insert(key.clone(), value.clone());
            }
            meta.state.clone()
        };
        // An instance that keeps nothing has its state in hand and nowhere
        // else, and what is said of it changing is passed on and not kept.
        if !live.blueprint().durable {
            let said = Value::Object(changes.clone());
            self.inner.own.emit_ephemeral("instance.state", Source::instance(&live.id), said);
            return Ok(changes);
        }
        let (store, now) = (&self.inner.store, self.inner.own.now_ms());
        // What was set, the record that its step has acted and what is
        // said of it are one write.
        let says = vec![to_say("instance.state", Source::instance(&live.id), Value::Object(changes.clone()))];
        store.set_instance_state_with(&live.id, state, done(&changes), now, says).await?;
        Ok(changes)
    }
}
