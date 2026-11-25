//! Which text an instance runs, whether it is the one now there, and how
//! it comes to run that one.
//!
//! An instance keeps the text it was applied with. The text under its
//! blueprint's name may since have become another: a file was edited, a
//! package was updated. Nothing tells the instance, and it goes on as it
//! was, which is right: it did not ask. This module is what is said of
//! that, and what is done about it.
//!
//! **Said.** Whoever is shown an instance is shown the revision of the
//! text it runs, what that text calls its own version, and whether it is
//! *behind* the text now there, with how it would come to run it
//! ([`Takes`]): worked out from the two texts when it is asked for
//! (`texts::behind`).
//!
//! **Done, when somebody applies what is above it.** Applying an instance
//! again runs its steps again where they have changed, and the children
//! those steps name are applied by them in their turn. What one of its
//! *handlers* started is not named by any step that runs again: a lane,
//! spawned when a card became ready. So, once an apply that somebody asked
//! for has left the instance as asked, every live instance below it is
//! looked at, and one that is behind is brought to the text now there by
//! the rule every apply goes by ([`crate::update`]), as whoever asked,
//! with one difference: the look below asks for nothing to be run again,
//! but where the tree above it is held to a text that ships (below).
//! A child whose text says how it is taken (`on_update`) is carried over
//! ([`crate::carry`]). One whose text begins as its own does
//! ([`blueprint::outset`]: the same params and steps, differing only in
//! its handlers, in what it does when stopped, in what it lists under
//! `permissions` and in what it says of itself) takes it *where it is*:
//! nothing is run again and its state is kept; a run under way finishes on
//! the text it began with, and each handler goes over to the new text when
//! it is next between runs ([`crate::trigger`]). A child that only its
//! steps running again would bring to the text (a lane's first steps claim
//! its card) is left as it is and stays behind, as is one whose text does
//! not check here or that whoever asked may not apply; but for a text that
//! ships, which is what the tree above it is held to ([`crate::pinned`]):
//! somebody who took what ships moved the tree to it, and nothing else
//! would bring the child there. So a child whose steps have settled has
//! them run again for that, and what is under it is looked at once they
//! settle, and not before, so that what they call has its new text before
//! they run. One whose steps are under way finishes them on the text it
//! began with.
//!
//! **Done, by hand, or by a `spawn`.** One instance can be applied again
//! as it is, with the params, the key and the place it has
//! ([`Engine::apply_again`]), and a step that spawns a child that is there
//! applies it too. Both are applies like any other and go by the rule: the
//! instance is carried over, takes the text where it is, or has its steps
//! run again, as the two texts say. A text taken because a handler's step
//! asked is taken as of the event that handler's run is for: what the text
//! adds hears that event.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use kernel::{Error, Principal, Result};
use serde_json::json;
pub(crate) use texts::behind::begins_alike;
pub use texts::behind::{Behind, Shown, Takes};
use tokio::sync::OwnedMutexGuard;

use blueprint::Blueprint;
use kernel::{BoxFut, Source};
use runs::{Below, Definition, InstanceFilter, InstanceInfo, InstanceStatus, Say, Taken, to_say};

use crate::engine::{Applied, Apply, Engine, Live};
use crate::trigger;
use crate::update::{Asker, How};
use crate::whole::Whole;

/// What looks at an instance to bring it to the text now there for it,
/// which says which text that is.
#[derive(Clone, Copy)]
pub(crate) enum Look {
    /// The look below a parent somebody applied.
    Below,
    /// What follows texts as they change ([`crate::taking`]).
    Followed,
}

impl Live {
    /// Notes that what is under it is to be looked at once the steps of
    /// this generation have settled, because somebody asked
    /// ([`crate::current`]). Kept as the generation and one more: an
    /// instance in hand begins at none, which is no note.
    pub(crate) fn owes_look(&self, generation: u64) {
        self.look_below.store(generation + 1, Ordering::SeqCst);
    }

    /// Takes that note, where it is this generation's, once: with how
    /// many texts it had taken owing a look by then.
    pub(crate) fn takes_look(&self) -> Option<u64> {
        let owed = self.generation.load(Ordering::SeqCst) + 1;
        let taken = self.look_below.compare_exchange(owed, 0, Ordering::SeqCst, Ordering::SeqCst);
        taken.ok().map(|_| self.looks.0.load(Ordering::SeqCst))
    }
}

impl Engine {
    /// The instances as they are shown: each with the version its text
    /// says and whether it is behind the text now there
    /// ([`texts::Texts::shown`]).
    pub fn shown(&self, instances: Vec<InstanceInfo>) -> Vec<Shown> {
        self.inner.texts.shown(self, instances)
    }

    /// One instance as it is shown.
    pub fn shown_one(&self, instance: InstanceInfo) -> Shown {
        self.shown(vec![instance]).remove(0)
    }

    /// What is said of `instance` when `there` is the text now under its
    /// blueprint's name; nothing when that is the text it runs.
    /// What is said of `instance` being behind the text `there`, for what
    /// follows texts ([`crate::taking`]).
    pub(crate) fn shown_behind(&self, instance: &InstanceInfo, there: &str) -> Option<Behind> {
        self.inner.texts.behind(self, instance, there)
    }

    /// Looks at every live instance below `root` and brings each that is
    /// behind to the text now there, when that can be done with nothing
    /// run again. `by` is whoever asked for `root` to be applied.
    pub(crate) async fn bring_below(&self, root: &str, by: &Principal) {
        let mut parents = vec![root.to_string()];
        while let Some(parent) = parents.pop() {
            let filter = InstanceFilter {
                parent: Some(parent),
                live_only: true,
                limit: 100_000,
                ..Default::default()
            };
            for child in self.inner.store.instances(filter).await.unwrap_or_default() {
                let id = child.id.clone();
                // One whose steps run again has what is under it looked at
                // once they settle, and not now: what they call and start
                // is brought by them first.
                if !self.bring(child, by).await {
                    parents.push(id);
                }
            }
        }
    }

    /// After the steps of an apply that somebody asked for have settled:
    /// looks below, as whoever that was.
    ///
    /// That it is owed is kept with the text, from the write that took
    /// it (`owes`, `below`), and the note is taken off when the look is
    /// done: a conductor stopped before then looks when it is started,
    /// as the steps it takes up settle or, where they had, at once
    /// (`Engine::resume`). The look is the same done twice. A text taken
    /// before it is over, whoever gives it, keeps it owed (`take_owing`), and
    /// it is done again once that text's steps have settled: so this one
    /// does not take the note off.
    pub(crate) async fn look_below(&self, live: &Arc<Live>) {
        if let Some(asked) = live.takes_look() {
            self.bring_below(&live.id, &live.meta().principal).await;
            let _turn = live.looks.1.lock().await;
            if live.looks.0.load(Ordering::SeqCst) == asked {
                let _ = self.inner.store.looked_below(&live.id).await;
            }
        }
    }

    /// Writes the text `live` takes, which says whether what is under it
    /// is owed a look from there: nothing where it was not written. In
    /// the turn a look takes its note off in, and counted where a look is
    /// owed, so that one under way does not take off what this kept.
    pub(crate) async fn take_owing(
        &self,
        live: &Live,
        taken: Taken,
        says: Vec<Say>,
    ) -> Result<Option<Below>> {
        let _turn = live.looks.1.lock().await;
        let now = self.inner.own.now_ms();
        let written = self.inner.store.update_instance_definition(&live.id, taken, now, says).await?;
        if written.is_some_and(Below::owed) {
            live.looks.0.fetch_add(1, Ordering::SeqCst);
        }
        Ok(written)
    }

    /// For the look below a parent: applies over `instance` the text now
    /// there for its blueprint, with the params it has, asking for nothing
    /// to be run again. Anything in the way leaves it exactly as it was.
    /// Boxed, and so of a type that is written down: an apply looks below,
    /// and the look below comes here.
    /// Whether its steps were run again.
    fn bring<'a>(&'a self, instance: InstanceInfo, by: &'a Principal) -> BoxFut<'a, bool> {
        Box::pin(self.bring_to(instance, Look::Below, by))
    }

    /// The text `look` brings `instance` to, as the texts now are, and
    /// whether it is one the tree above it is held to; nothing where there
    /// is none for it.
    async fn there(&self, instance: &InstanceInfo, look: Look) -> Option<(String, bool)> {
        let (name, project) = (instance.blueprint.as_str(), instance.project.as_deref());
        let there = self.inner.library.load(name, project);
        match look {
            // What ships is taken as the tree above it reached it: a look
            // below brings a child to what its parent now goes by, and not
            // to what a new program brought that nobody has taken. Of
            // itself first: one somebody moved on by itself to what ships
            // goes by that ([`crate::pinned`]), and nothing above it moves
            // it back.
            Look::Below => match (&instance.parent, self.holds(name, project)) {
                (Some(_), true) => match self.pinned_for(&instance.id, name).await {
                    Some((_, pinned)) => Some((pinned, true)),
                    None => there.map(|there| (there, true)),
                },
                _ => there.map(|there| (there, false)),
            },
            // The one this instance may be given: a project's own is
            // given as it was vouched for.
            Look::Followed => {
                let there = there?;
                let within = self.text_within(&instance.id, &instance.principal, name, project).await;
                Some((within.unwrap_or(there), false))
            }
        }
    }

    /// Applies over `instance` the text `look` finds for it, with the
    /// params it has, as `by`, asking for nothing to be run again unless a
    /// taken text lands that way here ([`crate::taking`]). Anything in the
    /// way leaves it exactly as it was.
    ///
    /// The text is read before the instance's turn is waited for, and
    /// read again where the turn is held if the instance was given
    /// anything meanwhile ([`crate::update`]). Whether its steps were run
    /// again.
    pub(crate) fn bring_to<'a>(
        &'a self,
        instance: InstanceInfo,
        look: Look,
        by: &'a Principal,
    ) -> BoxFut<'a, bool> {
        Box::pin(async move {
            let project = instance.project.as_deref();
            // The text, where it is another than the instance runs and
            // one it can be given, with the blueprint it is.
            let other = async || {
                let (there, held) = self.there(&instance, look).await?;
                let revision = blueprint::revision(&there);
                if revision == instance.revision || blueprint::means_the_same(&instance.source, &there) {
                    return None;
                }
                let blueprint = self.compile_new(&instance.blueprint, &there, project).ok()?;
                let params = blueprint.resolve_params(&instance.params).ok()?;
                Some((there, revision, blueprint, params, held))
            };
            let at = crate::update::mark();
            let Some(mut read) = other().await else { return false };
            let mut turn = self.turn_of(&instance, &read.2, by).await;
            if turn.given_since(at) {
                let Some(again) = other().await else { return false };
                read = again;
            }
            let (source, revision, blueprint, params, held) = read;
            let definition = Definition {
                revision,
                source,
                params,
                principal: Some(by.clone()),
                state: None,
                before: None,
                heard: Some(crate::whose::PROJECT.into()),
            };
            let how = How { force: false, asker: Asker::Below, step: None, held };
            let labels = instance.labels.clone();
            let brought = self.reapply(&mut turn, instance.clone(), blueprint, definition, labels, how).await;
            drop(turn);
            match brought {
                Ok(applied) => applied.took == Some(crate::update::Took::Again),
                Err(refused) => {
                    if !crate::meets::is(&refused) {
                        let said = json!({ "error": refused.message, "kept": instance.revision });
                        self.emit("instance.update_failed", Source::instance(&instance.id), said).await;
                    }
                    false
                }
            }
        })
    }

    /// Gives `instance` the text `definition` where it is, as whoever the
    /// definition says applied it: nothing is run again and its state is
    /// kept. Called where nobody else is applying over it, by what has
    /// found that the text begins as its own does and that whoever asks
    /// may apply it ([`Engine::reapply`]). `during` is the place of the
    /// event that brought the asking about, if one did: what the text adds
    /// hears from there.
    ///
    /// Whole or not at all, whoever is waiting ([`Whole`]): `turn` is the
    /// instance's turn at being applied over, which goes with the work
    /// and is given back when the text is taken, or was refused.
    pub(crate) fn take_in_place(
        &self,
        live: &Arc<Live>,
        instance: InstanceInfo,
        blueprint: Arc<Blueprint>,
        definition: Definition,
        during: Option<u64>,
        turn: OwnedMutexGuard<()>,
    ) -> Whole<(Result<()>, OwnedMutexGuard<()>), Engine> {
        let (engine, live) = (self.clone(), live.clone());
        Whole::of(self, async move {
            (engine.take_where_it_is(&live, instance, blueprint, definition, during).await, turn)
        })
    }

    async fn take_where_it_is(
        &self,
        live: &Arc<Live>,
        instance: InstanceInfo,
        blueprint: Arc<Blueprint>,
        definition: Definition,
        during: Option<u64>,
    ) -> Result<()> {
        let now = self.inner.own.now_ms();
        // The text is written as taken and said to be, as one write.
        let said = json!({
            "blueprint": instance.blueprint,
            "revision": definition.revision,
            "from": instance.revision,
            "in_place": true,
        });
        let says = vec![to_say("instance.updated", self.source_taking(live, &definition), said)];
        // What is said once that is said is the taken text's to hear. So
        // where a handler the text brings begins is taken before the
        // write and written with it, and the handlers there are are told
        // to take no more before it: what comes to one from here on has
        // no run and waits for the handler that follows it. Told after
        // the write, with other things asked of the database first, one
        // went on to handle in the text from before what whoever heard
        // `instance.updated` said next.
        let from = trigger::begins(self, during).await;
        let places = trigger::places_from(self, live, &blueprint, &from).await;
        trigger::retire(live);
        let taken = self.inner.store.take_text(&instance.id, definition.clone(), places, now, says).await;
        if let Err(refused) = taken {
            // Nothing of it was changed: it listens again as it was.
            trigger::rearm(self, live, None).await;
            return Err(refused);
        }
        // A text it was waiting to be carried over to is called off: this
        // is the later asking, and the one that stands. What it was to be
        // carried over to went in the write, and what waits in hand to
        // carry it is let go of here, before anybody else has the
        // instance's turn: left waiting, the earlier text landed after
        // this one, when its handlers were next between runs.
        if let Some(waiting) = live.carrying.lock().expect("live lock").waiting.take() {
            waiting.cancel.cancel();
        }
        live.redefine(blueprint, definition, None);
        trigger::rearm(self, live, Some(from)).await;
        Ok(())
    }

    /// Applies the live instance `id` again as it is: the text now there
    /// for its blueprint, with the params, the key and the place it has,
    /// for `by`. An apply like any other, by the one rule
    /// ([`crate::update`]): it is carried over, takes the text where it
    /// is, or has its steps run again.
    pub async fn apply_again(&self, id: &str, by: Principal, force: bool) -> Result<Applied> {
        self.apply_again_taking(id, by, force, false).await
    }

    /// The same, saying whether the project's own blueprints are taken as
    /// they now are ([`crate::vouched`]).
    pub async fn apply_again_taking(
        &self,
        id: &str,
        by: Principal,
        force: bool,
        take: bool,
    ) -> Result<Applied> {
        // Before anything is said of the instance: it needs what applying needs.
        by.require("blueprints:run")?;
        // Noted before it is read: what is asked is what it has, and it
        // may be given something else before this has its turn.
        let at = crate::update::mark();
        let instance = self.instance(id).await?;
        // Applied again by the conductor that keeps it.
        let conductor = self.of(instance.project.as_deref()).await?;
        if !Arc::ptr_eq(&conductor.inner, &self.inner) {
            return Box::pin(conductor.apply_again_taking(id, by, force, take)).await;
        }
        let (name, key) = (instance.blueprint.clone(), instance.key.clone());
        if !instance.status.is_live() {
            let was = match instance.status {
                InstanceStatus::Done => "has finished",
                InstanceStatus::Failed => "has failed",
                _ => "was stopped",
            };
            return Err(Error::conflict(format!(
                "`{key}` {was}, so there is nothing running to apply `{name}` over. Apply `{name}` to start it."
            )));
        }
        // Said here, where it can be said what to do about it: it was
        // started with params that only what started it knows how to give.
        if let Some(there) = self.inner.library.load(&name, instance.project.as_deref())
            && let Ok(blueprint) = self.check_own(&there, instance.project.as_deref()).into_result()
            && let Err(wrong) = blueprint.resolve_params(&instance.params)
        {
            return Err(Error::invalid(format!(
                "`{key}` cannot be applied again with the params it was started with: {wrong}. Nothing \
                 was changed. Stop it and let what started it start it again."
            )));
        }
        let request = Apply {
            blueprint: name,
            source: None,
            params: instance.params,
            key: Some(instance.key),
            parent: instance.parent,
            target: instance.target,
            project: instance.project,
            principal: by,
            force,
            take,
            labels: instance.labels,
            environment: (!instance.environment.is_empty()).then_some(instance.environment),
        };
        self.apply_read(request, Some(at)).await
    }
}

impl Engine {
    /// Which blueprints are in use where: each project, with the name of
    /// each blueprint that has an instance alive there, at any depth. Asked
    /// of the instances as they are, so it is as true with ten thousand of
    /// them as with ten.
    pub async fn in_use(&self) -> Result<Vec<(String, String)>> {
        crate::routed::in_use(self).await
    }
}
