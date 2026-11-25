//! Applying over an instance that is running: how it comes to the text,
//! and whose authority it has from then on.
//!
//! **How.** An instance that is there is brought to what is asked by one
//! rule, whoever asks ([`Engine::reapply`]; it is written out in
//! `docs/architecture.md`, *Instances and runs*, and nowhere else):
//!
//! 1. the same text and the same params: nothing happens, and steps of
//!    it that are still going go on;
//! 2. asked with force: its steps run again;
//! 3. the text says `on_update` and the instance is active: it is carried
//!    over ([`crate::carry`]);
//! 4. the text begins as the one it runs does and the params are the same:
//!    it takes the text where it is, and nothing is run again
//!    ([`crate::current`]);
//! 5. otherwise: its steps run again.
//!
//! **With which text.** Whoever asks reads the text first and waits for
//! the instance's turn afterwards, and somebody else may have had the
//! turn in between. A text read before that and applied after it took
//! the instance back: a step that asked for its child, with somebody
//! applying a later text over the child in that moment, left the child
//! on the earlier one, with a handler the later text had dropped. So
//! what is applied over an instance is what was read where its turn is
//! held: whoever asks notes when it began to read ([`mark`]), takes the
//! turn ([`Engine::turn_of`]), and reads again there where the instance
//! was given anything to run since ([`Turn::given_since`]). The rule is
//! then asked with that text, and whoever asked is told what they would
//! have been told had they asked a moment later. An instance applied
//! again as it is ([`Engine::apply_again`]) is asked with the params it
//! has, which are read from it before its turn the same way: given
//! others meanwhile, it is asked with those.
//!
//! Who asks ([`Asker`]) changes two things only. The look below a parent
//! asks for nothing to be run, so where the rule comes to 5 it leaves the
//! instance as it is, and behind, unless the text is one that ships, which
//! the tree above it is held to (`How::held`): then its steps run, once
//! those it is in the middle of have settled. A `call` asks for exactly that, the
//! steps to be run and what they give, so it is never answered by 3 or 4.
//!
//! **As whom.** An instance acts for whoever applied the text it is
//! running: everything it does is checked against what that bearer held
//! when they applied it. So an apply over a running instance is an apply
//! like the first. It is the asker's text, and from the moment it takes it
//! runs as the asker, with no more than the asker holds, whoever started
//! the instance and whatever they held. Asking for what is already running
//! is not an update: nothing happens, and it goes on as whoever applied it.
//!
//! The text and the authority are never apart:
//!
//! - they are written to the instance's row in one statement, and written
//!   down together as its next version, so a restart finds the two as they
//!   were;
//! - in memory they are replaced together ([`Live::redefine`]) and read
//!   together ([`Live::acting`]) by whatever starts work, and a run keeps
//!   the pair it began with. What is left of a run of the earlier text is
//!   cancelled when its steps are to run again, and anything of it that
//!   still happens (a hook as a step ends) happens as whoever applied that
//!   text;
//! - what to go back to if the update fails is kept with the authority it
//!   was applied with and the state the instance had, so a failed update
//!   gives nothing of the asker's to the version that is kept running,
//!   takes nothing from it, and leaves it holding what it held: what the
//!   update's own steps started is handed back ([`crate::settle`]).
//!
//! Children follow by the same rule and by no other mechanism: a child the
//! new text starts, or names again with other text or params, is applied
//! by its parent's run, which acts as the asker, so it is checked and
//! updated here in its turn. One the new text leaves as it is keeps the
//! text, the params and the authority it was given by whoever applied them.
//! What a handler started is not named by the steps that run again, and is
//! reached another way, when somebody asked for the apply: see
//! [`crate::current`].

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use blueprint::{Blueprint, Labels};
use kernel::{BoxFut, Error, Principal, Result, Source};
use runs::{Definition, InstanceInfo, InstanceStatus, RunStatus, Started};
use serde_json::json;
use tokio::sync::OwnedMutexGuard;

use crate::asked::Apply;
use crate::engine::{Applied, Engine, Live, initial_state};
use crate::reach::Read;

/// Who asks for an apply over an instance that is there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Asker {
    /// Somebody, or something on their behalf: an apply by name and key,
    /// one instance applied again as it is, what a text attaches. What is
    /// below is looked at once the instance is as asked.
    Person,
    /// The look below a parent somebody applied ([`crate::current`]). It
    /// asks for nothing to be run again: an instance that only its steps
    /// would bring to the text is left exactly as it is, but for a text
    /// the tree above it is held to ([`How::held`]).
    Below,
    /// A `spawn` step of a running instance: the child should be there.
    /// `during` is the place in the journal of the event the step's run is
    /// handling, when it is an event's.
    Spawn { during: Option<u64> },
    /// A `call` step of a running instance: it asks for the steps to be
    /// run, and waits for what they give.
    Call,
}

/// How an apply over a running instance was come by.
#[derive(Clone)]
pub(crate) struct How {
    /// The steps are to run again whatever else is so.
    pub force: bool,
    pub asker: Asker,
    /// The step that asked, for a `spawn` ([`crate::carry::asked_by`]).
    pub step: Option<String>,
    /// For the look below: the text is the one the tree above it is held
    /// to ([`crate::pinned`]), which somebody moved it to by taking what
    /// ships. Where it begins otherwise and the instance's steps have
    /// settled, they are run again: the tree goes by that text, and
    /// nothing else would bring it there.
    pub held: bool,
}

/// How an instance came to what was applied over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Took {
    /// Its steps ran again.
    Again,
    /// It was carried over: what the text says under `on_update` was done
    /// in place of its steps.
    Carried,
    /// It took the text where it was, and nothing was run again.
    InPlace,
}

pub(crate) use texts::ran::may_apply_over;

/// How the sentence ends that says an instance ended as it was applied
/// over ([`ended_meanwhile`]), which is what it is known by.
const ENDED: &str = "ended as this was being applied over it, and nothing was changed; apply it again";

/// What whoever applied over an instance is told where it ended in the
/// moment they asked: its steps came to their end, or failed, or it was
/// stopped, and that was written first.
pub(crate) fn ended_meanwhile(key: &str) -> Error {
    Error::conflict(format!("`{key}` {ENDED}"))
}

/// How many times anything in hand has been given something to run.
static GIVEN: AtomicU64 = AtomicU64::new(0);

/// What an update goes back to, saying that a look owed from before it
/// is owed still there, as the write that took its text said.
pub(crate) fn keeping(mut previous: Definition, below: runs::Below) -> Definition {
    if let (Some(stood), runs::Below::Kept) = (previous.before.as_mut(), below) {
        stood.below = true;
    }
    previous
}

/// When whoever is about to read a text for an instance began: before
/// the first thing it reads ([`Turn::given_since`]).
pub(crate) fn mark() -> u64 {
    GIVEN.load(Ordering::SeqCst)
}

impl Live {
    /// Notes that it is given something to run now: it came into hand,
    /// a text or params were put in force, or it is to be carried over.
    pub(crate) fn given(&self) {
        self.given.store(GIVEN.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
    }
}

/// An instance's turn at being applied over, with the instance in hand.
pub(crate) struct Turn {
    live: Arc<Live>,
    /// It was not in hand, and was loaded for this asking.
    loaded: bool,
    /// The turn itself; none only while a text is taken where the
    /// instance is, which keeps it until that is done ([`crate::whole`]).
    applying: Option<OwnedMutexGuard<()>>,
}

impl Turn {
    /// Whether the instance was given anything to run since `read`
    /// ([`mark`]): what was read for it since then is read again, here,
    /// where nobody else is applying over it. One loaded for this asking
    /// was in nobody's hand meanwhile.
    pub(crate) fn given_since(&self, read: u64) -> bool {
        !self.loaded && self.live.given.load(Ordering::SeqCst) > read
    }
}

impl Engine {
    /// Takes the turn of `existing` at being applied over, for `by`, who
    /// asks for `blueprint`: waited for where somebody else has it.
    pub(crate) async fn turn_of(
        &self,
        existing: &InstanceInfo,
        blueprint: &Arc<Blueprint>,
        by: &Principal,
    ) -> Turn {
        let (live, loaded) = match self.live(&existing.id) {
            Some(live) => (live, false),
            // Loaded as what is being asked for, text and authority both: a
            // text is never held with the authority of the one before it.
            None => {
                let asked = InstanceInfo { principal: by.clone(), ..existing.clone() };
                (self.register(&asked, blueprint.clone()), true)
            }
        };
        let applying = Some(live.apply_lock.clone().lock_owned().await);
        Turn { live, loaded, applying }
    }

    /// What `request` comes to where `existing` is running under the key
    /// it names: the text `read` for it since `at`, or read again where
    /// the turn is held, applied over it by the one rule.
    ///
    /// Everything that is worked out from the text is worked out in the
    /// turn, and what moves with it moves there: what was vouched for,
    /// and what the instance reaches, which is put back there too where
    /// the apply is refused.
    pub(crate) async fn apply_over(
        &self,
        mut existing: InstanceInfo,
        mut request: Apply,
        (at, as_it_is, mut read): (u64, bool, Read),
        (caller, asker): (Option<Source>, Asker),
        within: Option<String>,
    ) -> Result<Applied> {
        let (id, key) = (existing.id.clone(), existing.key.clone());
        // Whoever asks vouches for the project's own texts as they
        // stand; what an instance does is held to what was vouched for.
        let asked = (caller.is_none(), within.clone());
        // Somebody asking, and no instance on its own behalf: what is
        // reached is pinned for the instance, or said to have changed.
        let person = caller.is_none() && within.is_none();
        let asks = caller.as_ref().and_then(|step| step.instance.clone()).or_else(|| within.clone());
        let spawned = caller.as_ref().filter(|_| matches!(asker, Asker::Spawn { .. }));
        let step = spawned
            .and_then(|step| Some(crate::carry::asked_by(step.run.as_deref()?, step.step.as_deref()?)));
        let mut turn = self.turn_of(&existing, &read.blueprint, &request.principal).await;
        if turn.given_since(at) {
            if let Some(now) = self.inner.store.instance(&id).await? {
                existing = now;
            }
            // Applied again as it is, it is asked with the params it
            // has now, and not those whoever asked read from it.
            if as_it_is {
                request.params = existing.params.clone();
            }
            read = self.read(&request, caller, within.as_deref()).await?;
            // Named by what its text is called, which is now another:
            // it is not what is asked for any more.
            if request.key.is_none() && read.blueprint.name != key {
                return Err(ended_meanwhile(&key));
            }
        }
        // Moved on by somebody by itself to a text that ships, it goes on
        // with that whatever the tree above it is held to: what is above
        // it, applied again, does not move it back. Only taking what ships
        // above it moves it with the rest ([`Engine::unpin_below`]).
        if !person && request.source.is_none() && read.blueprint.name == existing.blueprint {
            read = self.its_own(&existing, &request, read).await?;
        }
        let project = request.project.as_deref();
        let Read { source, blueprint, mut reaches, params } = read;
        let top = (request.parent.is_none(), (blueprint.name.as_str(), request.take));
        let vouched = self.answered(&id, project, top, &request.principal, asked).await;
        // What it reaches moves with it when whoever asks says to
        // take the texts as they now are, or when it comes itself
        // to another text that ships: a text and what it names
        // are released together. Otherwise it goes on with what
        // it reached before, and is told what has changed since.
        let another = self.comes_to_another(&request, &existing, &source);
        let was = self.pinned_now(&id).await;
        let mut unpinned = Vec::new();
        let reached = match person {
            true => {
                // Its own text goes back with the rest only where it moved with them.
                let moved = (request.take || another).then(|| another.then_some(existing.source.as_str()));
                if moved.is_some() {
                    // What is under it moves with it, whatever it was held to.
                    unpinned = self.unpin_below(&id).await;
                    // Below something else, it is held to its own text
                    // from now, as what is above it is not, and to what
                    // ships now of what that is held to.
                    if existing.parent.is_some() {
                        reaches.push((existing.blueprint.clone(), source.clone()));
                        let above = self.held_above(&existing, &reaches).await;
                        reaches.extend(above);
                    }
                }
                self.repin(&id, project, &reaches, moved).await
            }
            false => Default::default(),
        };
        let revision = blueprint::revision(&source);
        let (by, heard) = (Some(request.principal.clone()), crate::whose::taken());
        let definition =
            Definition { revision, source, params, principal: by, state: None, before: None, heard };
        let how = How { force: request.force, asker, step, held: false };
        match self.reapply(&mut turn, existing, blueprint, definition, request.labels.clone(), how).await {
            Ok(applied) => Ok(Applied { vouched, reached, ..applied }),
            Err(refused) => {
                // Refused, the instance is as it was, and so is what it reaches.
                if !reached.taken.is_empty() || !unpinned.is_empty() {
                    self.pin_back(&id, was).await;
                }
                for (below, pins) in unpinned {
                    self.pin_back(&below, pins).await;
                }
                drop(turn);
                // Met as it was being stopped, it is waited for.
                Err(self.met(refused, &id, &key, asks.as_deref()).await)
            }
        }
    }

    /// An apply, for whoever asks: somebody, an instance on its own
    /// behalf (`within`), or a `call` or `spawn` of a running instance
    /// (`caller`: the step, and which of the two).
    ///
    /// What is found running under the key may end in the moment between
    /// being found and being written over: its steps came to their end,
    /// or it was stopped, and that was written first. Nothing was
    /// changed, and what was asked is then what it would have been a
    /// moment later, something applied where nothing is running. So it
    /// is asked once more, here, and not handed back to whoever asked: a
    /// step has nobody to ask again for it. Once: one that ends again in
    /// that moment is said.
    ///
    /// `again`, for an instance applied again as it is: when what is
    /// asked was read from it ([`mark`]).
    pub(crate) fn apply_from(
        &self,
        request: Apply,
        caller: Option<(Source, Asker)>,
        within: Option<String>,
        again: Option<u64>,
    ) -> BoxFut<'_, Result<Applied>> {
        Box::pin(async move {
            match self.apply_once(request.clone(), caller.clone(), within.clone(), again).await {
                Err(refused) if refused.message.ends_with(ENDED) => {
                    self.apply_once(request, caller, within, again).await
                }
                answered => answered,
            }
        })
    }

    /// Brings the instance `existing` to `definition`, for whoever the
    /// definition says applied it, by the one rule (the module doc), in
    /// its `turn`, which whoever asks keeps. When a person asked, what is
    /// below it is looked at as well, once it is itself as asked
    /// ([`crate::current`]).
    pub(crate) async fn reapply(
        &self,
        turn: &mut Turn,
        existing: InstanceInfo,
        blueprint: Arc<Blueprint>,
        mut definition: Definition,
        labels: Labels,
        How { force, asker, step, held }: How,
    ) -> Result<Applied> {
        let asked = asker == Asker::Person;
        let by = definition.principal.clone().unwrap_or_else(|| existing.principal.clone());
        let (live, loaded) = (turn.live.clone(), turn.loaded);
        // What it is running now and as whom, read where nobody else is
        // applying over it: the one row holds both.
        // With the last run of its steps, as of one moment: its steps end
        // in no turn, and a run read as over beside a row read before it
        // had become what that run made it is an instance owed nothing
        // that reads as one owed its steps (`Store::standing`).
        let (mut existing, last) = match self.inner.store.standing(&existing.id).await? {
            Some((now, last)) => (now, last),
            None => (existing, None),
        };
        // Being stopped, it takes nothing and is asked nothing: whoever
        // asks waits until that is over ([`crate::meets`]).
        if let Some(met) = self.meets(&existing, loaded) {
            return Err(met);
        }
        let nothing = Applied {
            instance: existing.id.clone(),
            run: None,
            created: false,
            changed: false,
            took: None,
            vouched: Default::default(),
            reached: Default::default(),
        };
        // The look below asks with the params the instance has, which
        // are those it has now: whoever looked read them before it had
        // the turn, and may have read them long before.
        if asker == Asker::Below {
            match blueprint.resolve_params(&existing.params) {
                Ok(params) => definition.params = params,
                Err(_) => {
                    if loaded {
                        self.forget(&existing.id);
                    }
                    return Ok(nothing);
                }
            }
        }
        let settled_ok = last.as_ref().is_some_and(|run| run.status == RunStatus::Ok);
        // Its steps are going now. Only said of an instance that was known
        // here, since that is what has a run in hand: one loaded for this
        // asking with a run still written as running has nobody running it.
        let under_way = last.as_ref().filter(|run| !loaded && run.status == RunStatus::Running);
        // 1. Whoever asks: what is already running goes on as whoever
        // applied it, and nobody's authority is given to it or taken from
        // it by asking for what is there. Only an update that takes changes
        // hands.
        // A text that differs from the one it runs only in how it is laid
        // out and what its comments say is the text it runs.
        let same_text = existing.revision == definition.revision
            || blueprint::means_the_same(&existing.source, &definition.source);
        let same = same_text && existing.params == definition.params;
        // Asked for what it is in the middle of: its steps go on as they
        // are, and whoever asked is given the run to wait on. Starting
        // them over would do twice whatever they have done so far, for no
        // more reason than that somebody asked twice. What is below is
        // being started by those steps, and is not looked at for this.
        if let (true, false, Some(going)) = (same, force, under_way) {
            self.relabel(&mut existing, labels).await?;
            return Ok(Applied { run: Some(going.id.clone()), ..nothing });
        }
        // Written down and never begun: a conductor was stopped between
        // the two, and this is the step that asked for it, taken up. What
        // is asked for is what is written, so nothing is taken: it is
        // begun, the one way that is done, unless the conductor's start
        // came to it first (`Engine::begin_owed`). Left to be taken as
        // an update, it was given a second run of its steps beside the
        // one the start gave it, and said to have been updated.
        // And so with one that was taking a text: the text was written,
        // and the conductor stopped before the run of its steps was.
        // Asked for what is written, it is owed those steps, and a
        // `call` that came to it before the start did was told that
        // nothing had changed and handed what its steps gave the time
        // before. (One that is being carried over to a text is taking
        // one too, and its steps are not what it is owed.)
        let no_steps = !last.as_ref().is_some_and(|run| run.status == RunStatus::Running);
        let owed = existing.status == InstanceStatus::Starting && no_steps && same && !force;
        if owed && (last.is_none() || !self.inner.store.carrying(&existing.id, crate::carry::FLOW).await?) {
            if loaded {
                live.meta.lock().expect("live lock").principal = existing.principal.clone();
            }
            self.relabel(&mut existing, labels).await?;
            let by = if asked { "apply" } else { "step" };
            let run = self.begin_owed(&live, by).await?;
            return Ok(Applied { run, ..nothing });
        }
        // And one that could not be begun, as whoever asks waited for its
        // turn: it has failed and said so, and they are answered what it
        // failed with. Taken as an update, a text would be given to
        // something that is over.
        if existing.status == InstanceStatus::Failed && last.is_none() && same && !force {
            if loaded {
                self.forget(&existing.id);
            }
            let why = crate::unbegun::why(existing.error.as_deref().unwrap_or_default());
            return Err(Error::invalid(why.to_string()));
        }
        let unchanged = same && settled_ok;
        if unchanged && !force {
            if loaded {
                // Loaded for an asker whose asking changed nothing: it is
                // what the row says it is, as whoever the row says.
                live.meta.lock().expect("live lock").principal = existing.principal.clone();
            }
            self.relabel(&mut existing, labels).await?;
            if asked {
                self.bring_below(&existing.id, &by).await;
            }
            return Ok(nothing);
        }

        // Where a taken text lands by its steps being run again, another
        // text is taken that way whoever asks ([`crate::taking`]).
        let again = !same_text && self.lands_again();
        // 2 and what a `call` asks: the steps, whatever else is so.
        let gently = !force && !again && asker != Asker::Call;
        // 3. A text that says how a running instance takes it is taken that
        // way by one whose steps have settled ([`crate::carry`]).
        let carried = gently && blueprint.on_update.is_some() && existing.status == InstanceStatus::Active;
        // 4. Another text that begins as its own does, with the params it
        // has: there is nothing in its steps to run again. Not while an
        // update of it may yet be undone, which is not added to.
        let in_place = gently
            && !carried
            && !same_text
            && existing.params == definition.params
            && existing.status.is_live()
            && live.previous.lock().expect("live lock").is_none()
            && crate::current::begins_alike(&existing.source, &definition.source);
        // 5. Its steps run again; which the look below a parent asks only
        // of a text that ships, as the tree above it is held to it, of an
        // instance whose steps have settled. One whose steps are under way
        // finishes them on the text it began with; what its steps start,
        // and what is under it, is looked at once they settle, as for
        // somebody's apply: so what a step of it calls has the text the
        // tree is held to before the step runs, and is not brought there
        // twice.
        let renewed = asker == Asker::Below
            && held
            && !carried
            && !in_place
            && existing.status == InstanceStatus::Active
            && under_way.is_none();
        if asker == Asker::Below && !carried && !in_place && !again && !renewed {
            if loaded {
                self.forget(&existing.id);
            }
            return Ok(nothing);
        }

        // Everything that can refuse the update comes before anything is
        // changed, so that a refusal leaves the instance exactly as it was.
        let state = may_apply_over(&existing, &blueprint, &by).and_then(|()| {
            initial_state(self, &blueprint, (&definition.params, self.seen_by(&live)), &live.meta().state)
        });
        let state = match state {
            Ok(state) => state,
            Err(refused) => {
                if loaded {
                    self.forget(&existing.id);
                }
                return Err(refused);
            }
        };
        self.relabel(&mut existing, labels).await?;
        let during = match asker {
            Asker::Spawn { during } => during,
            _ => None,
        };
        if carried {
            // Asked of it from here, whenever it is carried over.
            live.given();
            return self.carry(&live, existing.id, blueprint, definition, (asked, during), step).await;
        }
        if in_place {
            let id = existing.id.clone();
            let applying = turn.applying.take().expect("the turn is held by whoever applies");
            let (taken, applying) =
                self.take_in_place(&live, existing, blueprint, definition, during, applying).await;
            turn.applying = Some(applying);
            taken?;
            if asked {
                self.bring_below(&id, &by).await;
            }
            return Ok(Applied {
                instance: id,
                run: None,
                created: false,
                changed: true,
                took: Some(Took::InPlace),
                vouched: Default::default(),
                reached: Default::default(),
            });
        }

        let stood = self.stood(&live).await?;
        let generation = live.next_generation();
        // A stop that began since this looked has it in hand, and nothing
        // is written over it: asked with nothing waited for between the
        // generation and here, so that a stop which begins after this is
        // one of an instance that had taken the text.
        if let Some(met) = self.meets(&existing, loaded) {
            return Err(met);
        }
        let fallback = self.fallback(&live, &existing, settled_ok, stood);
        let falls_back = fallback.is_some();
        // The text is written as taken and said to be, as one write.
        let said = json!({ "blueprint": existing.blueprint, "revision": definition.revision, "from": existing.revision });
        let says = vec![runs::to_say("instance.updated", self.source_taking(&live, &definition), said)];
        // And with it, where somebody asked, that what is under it is to
        // be looked at once its steps have settled; which a look owed and
        // not over yet is as well.
        let taken = runs::Taken {
            definition: definition.clone(),
            state: state.clone(),
            previous: fallback,
            below: asked || renewed,
            places: Vec::new(),
        };
        let Some(below) = self.take_owing(&live, taken, says).await? else {
            // It ended in the moment this was asked, and has said how.
            if loaded {
                self.forget(&existing.id);
            }
            return Err(ended_meanwhile(&existing.key));
        };
        if falls_back && let Some(previous) = live.previous.lock().expect("live lock").as_mut() {
            *previous = keeping(previous.clone(), below);
        }
        live.redefine(blueprint.clone(), definition, Some(state));
        // Where it cannot be begun on the text it was given, it goes back
        // to the one it had and stays up, and whoever applied this is told:
        // left as it was written, it was taking a text for ever, and the
        // next conductor to start took it down for that.
        let begun = async {
            self.start_attachments(&live, &blueprint).await?;
            if below.owed() {
                live.owes_look(generation);
            }
            self.start_main(&live, generation, None).await
        };
        let run = match begun.await {
            Ok(run) => run,
            Err(err) => {
                self.could_not_begin(&live, &err).await;
                return Err(err);
            }
        };
        Ok(Applied {
            instance: existing.id,
            run: Some(run),
            created: false,
            changed: true,
            took: Some(Took::Again),
            vouched: Default::default(),
            reached: Default::default(),
        })
    }

    /// The text `existing` goes on with where it was moved on by itself
    /// to one that ships ([`Engine::apply_over`]): its own, with what was
    /// asked, in place of `read`; `read` where nothing of it is pinned.
    async fn its_own(&self, existing: &InstanceInfo, request: &Apply, read: Read) -> Result<Read> {
        let pinned =
            self.pins(&existing.id).await.and_then(|pins| pins.texts.get(&existing.blueprint).cloned());
        let other = blueprint::revision(&read.source) != existing.revision
            && !blueprint::means_the_same(&existing.source, &read.source);
        if pinned.as_deref() != Some(existing.revision.as_str()) || !other {
            return Ok(read);
        }
        let project = request.project.as_deref();
        let blueprint = self.compile(&existing.blueprint, &existing.source, project)?;
        let params = blueprint.resolve_params(&request.params).map_err(|wrong| {
            Error::invalid(format!(
                "`{}` was moved on by itself to another text of `{}` than the one what asks for it is held \
                 to, and what it is asked with does not fit that: {wrong}. Nothing was changed. Apply what \
                 is above it taking what ships now (`--take`), which moves the two together",
                existing.key, existing.blueprint
            ))
        })?;
        Ok(Read { source: existing.source.clone(), blueprint, reaches: Vec::new(), params })
    }

    /// What to return to if the update now starting fails: what the
    /// instance is running, as whoever applied that, holding what it
    /// holds. Unless an earlier update is already waiting to be undone:
    /// then the one before that is still the last that worked, and stays
    /// what is returned to.
    fn fallback(
        &self,
        live: &Live,
        existing: &InstanceInfo,
        settled_ok: bool,
        stood: Started,
    ) -> Option<Definition> {
        let mut previous = live.previous.lock().expect("live lock");
        if previous.is_some() || !settled_ok {
            return None;
        }
        *previous = Some(Definition {
            revision: existing.revision.clone(),
            source: existing.source.clone(),
            params: existing.params.clone(),
            principal: Some(existing.principal.clone()),
            state: Some(live.meta().state),
            before: Some(stood),
            heard: existing.heard.clone(),
        });
        previous.clone()
    }

    /// Where the instance stands now, for an update about to begin: what
    /// it has started since is the update's, and is handed back if the
    /// update fails ([`Engine::give_back`]).
    pub(crate) async fn stood(&self, live: &Live) -> Result<Started> {
        Ok(Started {
            holds: live.meta().holds,
            newest: self.inner.store.newest_instance().await?,
            below: false,
        })
    }
}
