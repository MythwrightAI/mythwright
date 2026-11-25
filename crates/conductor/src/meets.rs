//! An apply that comes to an instance while it is being stopped.
//!
//! A stop takes a while: what the instance's text says is done as it
//! stops is done, what is under it is stopped, what it held is given
//! back, and only then is it written as stopped. Until that write it
//! reads as alive, and an apply that came to it was taken by it and cut
//! with it: the steps it asked for were cancelled as they began, whoever
//! applied was told `cancelled`, and nothing was left under the key.
//!
//! So a stop says that it is under way, from before it does anything
//! until it is written ([`Stops`]), and an apply that comes to such an
//! instance takes nothing and changes nothing: it waits until the stop
//! is over, and is then an apply where nothing is running under the key
//! (`Engine::apply_from` asks once more, as it does for anything that
//! ended as it was applied over). Both happen, in the order they were
//! asked. A stop takes no turn and waits for no apply: one that was
//! already at work when the stop began was first, and is stopped as
//! anything is.
//!
//! Where waiting would be the stop waiting for itself, the apply is
//! refused, saying so, and nothing waits:
//!
//! - asked by what the stop does: something done as the instance stops,
//!   or as something under it does, and whatever that began and may be
//!   waiting for (`WITHIN`, which goes with the work a task starts);
//! - asked by a step of the instance or of anything under it: that
//!   step's instance is stopped with it, and the step is cut as it is.
//!
//! The look below a parent ([`crate::current`]) asks nothing of one that
//! is being stopped, and says nothing.
//!
//! **What is under it.** Everything in hand under what is stopped is
//! called off first, in one pass, before anything is taken down
//! ([`Engine::call_off`]): what is stopped later under it takes in
//! nothing the stopping of what is beside it did. An apply that comes to
//! one of those waits for the stop it is part of.

use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;

use kernel::Error;
use runs::InstanceInfo;

use crate::engine::{Engine, Live};

/// The stops under way are of what the runner keeps ([`runs::stops`]):
/// here under the names they always had.
pub(crate) use runs::stops::Stops;

tokio::task_local! {
    /// The instances whose stop what is done here is part of.
    static WITHIN: Vec<String>;
}

/// A stop an instance asked of itself in one of its own steps, which is
/// begun beside the step, since it cuts the run the step is in: it is
/// said to be under way from the moment the step asks, and until it is
/// over, held by whatever begins it. Begun only when it was looked at,
/// the run's steps could be over first, and the instance, which nothing
/// kept, was written as done by itself and never said to be stopped:
/// which of the two it came to was the machine's to say
/// ([`Engine::settling`] leaves an instance alone that a stop is under
/// way for).
pub(crate) struct Ahead {
    engine: Engine,
    id: String,
}

/// Says a stop of `id` is under way from now ([`Ahead`]).
pub(crate) fn ahead(engine: &Engine, id: &str) -> Ahead {
    engine.inner.stops.ahead(id);
    Ahead { engine: engine.clone(), id: id.to_string() }
}

impl Drop for Ahead {
    fn drop(&mut self) {
        self.engine.inner.stops.end(&self.id);
    }
}

/// Does `stop`, the stopping of `id`: what it does itself, and what
/// that begins, is part of it.
pub(crate) async fn within<T>(id: &str, stop: impl Future<Output = T>) -> T {
    let mut ids = WITHIN.try_with(Clone::clone).unwrap_or_default();
    ids.push(id.to_string());
    WITHIN.scope(ids, stop).await
}

/// `work`, as part of whatever stops the task that begins it is part of.
pub(crate) fn carried<T>(work: impl Future<Output = T> + Send + 'static) -> impl Future<Output = T> + Send {
    let ids = WITHIN.try_with(Clone::clone).ok();
    async move {
        match ids {
            Some(ids) => WITHIN.scope(ids, work).await,
            None => work.await,
        }
    }
}

/// How the sentence ends that says an apply came to something being
/// stopped, between where that is found and where it is acted on
/// ([`Engine::met`]): nobody who asked is told it.
const MET: &str = "is being stopped";

/// Whether `refused` says that ([`Engine::meets`]).
pub(crate) fn is(refused: &Error) -> bool {
    refused.message.ends_with(MET)
}

impl Engine {
    /// Calls `live` off as it is stopped or taken down (`ended`: its
    /// generation is ended too, as a stop ends it), and with it, where
    /// nobody had called it off before, everything in hand under it:
    /// each generation ended, its handlers taking no more, its runs cut.
    /// In one pass, with nothing waited for, before anything under it is
    /// taken down and before anything any of them does as it stops.
    ///
    /// What is under an instance is stopped a thing at a time, the
    /// newest first. Called off only as its turn came, what was stopped
    /// later went on reacting to what the stopping of an earlier one
    /// did: a run that waited on something another says or does as it
    /// stops took it in, and went on to do what it does then, under a
    /// parent that was being stopped. Each is still taken down in its
    /// turn, with what it does as it stops done whole; and a run that is
    /// cut is cancelled, as one cut by its own instance's stop is.
    ///
    /// Whether this was the first to call it off: the top of what is
    /// being stopped, and not something stopped in its turn under that.
    pub(crate) fn call_off(&self, live: &Live, ended: bool) -> bool {
        let first = !live.cancel.is_cancelled();
        if ended {
            live.next_generation();
        }
        live.cancel.cancel();
        // Whoever called it off first called off what is under it.
        if !first {
            return false;
        }
        let mut left: Vec<Arc<Live>> =
            self.inner.live.lock().expect("engine lock").values().cloned().collect();
        let mut off = HashSet::from([live.id.clone()]);
        // A parent before what is under it: from there nothing more is
        // started under it (`Store::insert_instance`).
        loop {
            let before = left.len();
            left.retain(|one| {
                let parent = one.meta.lock().expect("live lock").parent.clone();
                let under = parent.is_some_and(|parent| off.contains(&parent));
                if under {
                    one.next_generation();
                    one.cancel.cancel();
                    off.insert(one.id.clone());
                }
                !under
            });
            if left.len() == before {
                return true;
            }
        }
    }

    /// That `existing` is being stopped, where it is, as it is applied
    /// over: nothing is taken or changed, and what was loaded for the
    /// asking is let go of. Being stopped itself, or called off with
    /// what it is under and not yet come to in its turn.
    pub(crate) fn meets(&self, existing: &InstanceInfo, loaded: bool) -> Option<Error> {
        let off = || self.live(&existing.id).is_some_and(|live| live.cancel.is_cancelled());
        if self.inner.stops.over(&existing.id).is_none() && !off() {
            return None;
        }
        if loaded {
            self.forget(&existing.id);
        }
        Some(Error::conflict(format!("`{}` {MET}", existing.key)))
    }

    /// What whoever applied over `id` is told where that was `refused`.
    /// Where it was because the instance is being stopped: once the stop
    /// is over, that it ended as it was applied over, which has it
    /// applied once more where nothing is running; or, asked from inside
    /// that stop or by a step of what is being stopped (`asks`: the
    /// instance that asked, where one did), why it was not applied.
    pub(crate) async fn met(&self, refused: Error, id: &str, key: &str, asks: Option<&str>) -> Error {
        if !is(&refused) {
            return refused;
        }
        let ended = crate::update::ended_meanwhile(key);
        // The stop it is part of: its own, or that of what it is under.
        let (mut id, mut stop) = (id.to_string(), None);
        for _ in 0..64 {
            stop = self.inner.stops.over(&id);
            let above = match (&stop, self.live(&id)) {
                (Some(_), _) => None,
                (None, Some(live)) => live.meta.lock().expect("live lock").parent.clone(),
                (None, None) => self.inner.store.parent_of(&id).await.ok().flatten(),
            };
            let Some(above) = above else { break };
            id = above;
        }
        let Some(over) = stop else { return ended };
        let inside = WITHIN.try_with(|ids| ids.contains(&id)).unwrap_or(false);
        let under = match asks {
            Some(asks) => self.is_within(asks, &id).await,
            None => false,
        };
        if inside || under {
            return Error::conflict(format!(
                "`{key}` was not applied: it is being stopped, and what asked for it is part of that stop (it \
                 is done as something stops, or by a step of what is being stopped), so the stop would wait \
                 for it and it for the stop. Nothing was changed. Apply it from what is not being stopped: \
                 whatever hears that it is over (`instance.finished`), or whoever asked for the stop, once \
                 that is answered"
            ));
        }
        over.cancelled().await;
        ended
    }
}
