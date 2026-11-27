//! A conductor that starts takes up what was alive when the last one
//! stopped.
//!
//! There is one recovery path, and it is taken every time a conductor
//! starts: whatever stopped the one before, cleanly or not, what it left
//! is what is written down, and this goes on from that.

use std::sync::atomic::Ordering;
use std::time::Instant;

use kernel::{Result, Source};
use runs::{InstanceFilter, InstanceStatus, RunInfo, to_say};
use serde_json::json;

use crate::compiled;
use crate::engine::Engine;
use crate::set::standing;
use crate::trigger;

/// Says that a conductor's start is over, when it is.
struct Up<'a>(&'a Engine);

impl Drop for Up<'_> {
    fn drop(&mut self) {
        self.0.stand(standing::UP);
    }
}

/// How far the start of a factory's conductors has come
/// ([`Engine::how_far`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HowFar {
    /// The conductors there are: the factory's own, and one for each
    /// project whose journal keeps what is run for it.
    pub conductors: usize,
    /// How many of them have taken up what they keep.
    pub up: usize,
    /// How many instances have been taken up so far, of all of them.
    pub instances: usize,
}

impl Engine {
    /// How far the start of the factory's conductors has come
    /// ([`Engine::resume`]): for a daemon that answers while it takes up
    /// what it was doing, to say so. Asks nothing of a database.
    pub fn how_far(&self) -> HowFar {
        let every = self.every();
        HowFar {
            conductors: every.len(),
            up: every.iter().filter(|one| one.stands() == standing::UP).count(),
            instances: self.inner.set.taken.load(Ordering::SeqCst),
        }
    }

    /// Fails an instance whose kept text no longer checks against this
    /// device's actions and events, as a conductor starts: the actions
    /// changed under a stored blueprint, and it is said so and not
    /// guessed at. Written as owing its taking down, as an instance that
    /// fails is, so that what it started is stopped
    /// ([`Engine::take_down_owed`], asked once all of them are written)
    /// and is never taken up beside it.
    async fn unreadable_at_start(&self, info: &runs::InstanceInfo, why: &str) -> Result<()> {
        let now = self.inner.own.now_ms();
        let message = format!("cannot resume: {why}");
        let source = Source::instance(&info.id);
        let failed = json!({ "error": message, "reason": message });
        // That it is over is said once what was under it is taken down.
        let says = vec![to_say("instance.failed", source, failed)];
        self.inner.store.fail_instance(&info.id, message, true, now, says).await?;
        Ok(())
    }

    /// Picks up every instance and run that was in progress when the
    /// conductor last stopped. Returns how many instances were brought back.
    pub async fn resume(&self) -> Result<usize> {
        // The factory's own first and then each project's in the order
        // of their names, one after another: handlers read from their
        // places, so nothing rests on the order, and a start is the
        // same every time.
        // What is asked of any of them from now waits until it has
        // taken up what it keeps; what a start does itself does not
        // (`crate::set::standing`).
        let every = self.every();
        for conductor in &every {
            conductor.stand(standing::STARTING);
        }
        let began = Instant::now();
        let (resumed, compiled) = Engine::within_a_start(compiled::for_a_start(async {
            let mut resumed = 0;
            for conductor in &every {
                resumed += conductor.resume_own().await?;
            }
            Ok(resumed)
        }))
        .await;
        // What a start took, by how much it took up and how many texts it
        // compiled for that: it is paid by every start of a daemon.
        let journal = self.journal();
        let ok = resumed.is_ok();
        journal.measure("system.resumed", &[], began.elapsed().as_secs_f64() * 1000.0, ok);
        journal.measure("system.resumed_count", &[], *resumed.as_ref().unwrap_or(&0) as f64, ok);
        journal.measure("system.resumed_texts_count", &[], compiled.count() as f64, ok);
        // One a start that failed never came to is as it was made.
        for conductor in every.iter().filter(|one| one.stands() == standing::STARTING) {
            conductor.stand(standing::MADE);
        }
        self.started();
        resumed
    }

    /// Takes up what this conductor keeps. It can be asked from when
    /// this is over, whichever way it went.
    pub(crate) async fn resume_own(&self) -> Result<usize> {
        // A conductor started by itself compiles once for its own start.
        compiled::for_a_start(self.take_up()).await.0
    }

    async fn take_up(&self) -> Result<usize> {
        let _up = Up(self);
        let _resuming = self.inner.owed.resuming();
        // What runs under a project's environment reads it from what is held.
        self.inner.environs.load(self.inner.own.kv_list(crate::environs::NAMESPACE).await?);
        // What a program from before kept for this one's instances in
        // the factory's journal is its own from here on (`beside`).
        self.bring_over().await;
        // Before anything is taken up: what an instance that failed had
        // started is not, where its taking down was cut short.
        self.take_down_owed().await;
        // Nor is one whose stop had begun: it is finished.
        self.finish_stops().await;
        // Owed one look, by a journal a program from before this wrote.
        let unlooked = self.inner.store.owing("said").await.unwrap_or_default();
        let unlooked: std::collections::HashSet<String> = unlooked.into_iter().map(|one| one.id).collect();
        let mut instances = self
            .inner
            .store
            .instances(InstanceFilter { live_only: true, limit: 100_000, ..Default::default() })
            .await?;
        // An instance that keeps nothing is not picked up: it is over, with
        // what it started and what it held, and says so. Before anything
        // else, since what it started may be among the rest.
        let mut over = false;
        let mut unread = false;
        for info in &instances {
            let compiled = self.compile(&info.blueprint, &info.source, info.project.as_deref());
            // One whose text no longer checks against this device's
            // actions and events has failed, and says why: it is not
            // guessed at. What it had started is taken down with it, as
            // what is under any instance that failed is, before anything
            // is taken up: left alive, what was under it went on beside
            // what its parent applied anew under the same key, two of
            // each where one was meant (a fleet's arrangement, which
            // left every stage it had running beside the new one's).
            if let Err(err) = &compiled {
                self.unreadable_at_start(info, &err.message).await?;
                unread = true;
                continue;
            }
            let plain = compiled.is_ok_and(|blueprint| !blueprint.durable);
            // Stopped, with why and with its runs, as one thing: before
            // anything that is kept goes on, since what called it looks
            // at its run (`Engine::over_at_start`).
            if plain {
                self.over_at_start(info).await?;
                over = true;
            }
        }
        if unread {
            self.take_down_owed().await;
        }
        if over || unread {
            instances = self
                .inner
                .store
                .instances(InstanceFilter { live_only: true, limit: 100_000, ..Default::default() })
                .await?;
        }
        instances.reverse();
        let mut unfinished = self.inner.store.unfinished_runs().await?;
        // Every instance is known here before any of them goes on. A `call`
        // that was under way asks for its child again as soon as its run is
        // picked up; a child not yet known would be loaded for that asking,
        // beside the one this loop was about to bring back.
        let mut known = Vec::with_capacity(instances.len());
        for info in instances {
            let blueprint = match self.compile(&info.blueprint, &info.source, info.project.as_deref()) {
                Ok(blueprint) => blueprint,
                // Looked at above, and the same compiled once for a start:
                // one that does not check was failed there.
                Err(_) => continue,
            };
            let live = self.register(&info, blueprint);
            self.inner.kept.resumed(&info);
            known.push((info, live));
        }
        // What is kept beside the rows is each conductor's own, in its
        // own journal, and each tidies its own over what it has alive.
        // But the factory's: its journal may still keep what a program
        // from before kept there for a project's instances, until that
        // project's conductor, which starts after it, has brought its
        // own over. So it puts away only what nothing alive anywhere
        // is kept for.
        let live: Vec<_> = known.iter().map(|(info, _)| info.clone()).collect();
        let alive = match self.inner.project {
            None => self.alive_everywhere().await,
            Some(_) => live.clone(),
        };
        self.vouch_resumed(&live, &alive).await;
        self.pin_resumed(&live, &alive).await;
        // A comparison that counts events goes on counting from where it was.
        self.compared_resumed().await;
        let _reaching = self.carries_owed(&known, &unfinished).await;
        // Applied over by somebody, and what is under each not yet looked
        // at: it is, when the steps taken up have settled, or below.
        let below = self.inner.store.owing("below").await.unwrap_or_default();
        let below: std::collections::HashSet<String> = below.into_iter().map(|one| one.id).collect();
        let (mut resumed, mut looks) = (0, Vec::new());
        for (info, live) in known {
            let generation = live.generation.load(Ordering::SeqCst);
            let owes_look = below.contains(&info.id);
            if owes_look {
                live.owes_look(generation);
            }
            let (mine, rest): (Vec<RunInfo>, Vec<RunInfo>) =
                unfinished.into_iter().partition(|run| run.instance == info.id);
            unfinished = rest;
            let (mut handler_runs, mut main, mut carrying) = (Vec::new(), None, Vec::new());
            for run in mine {
                let Some(full) = self.inner.store.run(&run.id).await? else { continue };
                match full.flow.as_str() {
                    "main" => main = Some(full),
                    crate::carry::FLOW => carrying.push(full),
                    _ => handler_runs.push(full),
                }
            }
            // Its handlers' runs are theirs when they are started: here,
            // or when its steps or the text it was taking have settled.
            trigger::leave(&live, handler_runs);
            let taking = info.status == InstanceStatus::Starting && main.is_none();
            let carried = self.resume_carry(&live, generation, carrying, taking).await;
            match (info.status, main) {
                _ if carried => {}
                // Neither fails the conductor's start: an instance that
                // cannot be begun has failed, and says so (`unbegun`).
                (_, Some(run)) => self.go_on(&live, generation, run).await,
                // Stopped between being recorded and its first run starting:
                // what it attaches may not have been brought up either.
                (InstanceStatus::Starting, None) => self.begin_unbegun(&live).await,
                _ => {
                    trigger::prepare(self, &live, None).await;
                    trigger::arm(self, &live, Vec::new());
                    // Written down as active and never said to be, by a
                    // program from before this: looked for once.
                    if info.status == InstanceStatus::Active && unlooked.contains(&info.id) {
                        self.say_active_unsaid(&live).await;
                    }
                    // Its steps had settled, and the conductor stopped
                    // before what is under it was looked at.
                    if owes_look {
                        looks.push(live.clone());
                    }
                    // What it started may have ended as the conductor stopped.
                    if info.status == InstanceStatus::Active {
                        self.rests(Some(info.id.clone()));
                    }
                }
            }
            self.reached(&live);
            resumed += 1;
            self.inner.set.taken.fetch_add(1, Ordering::SeqCst);
        }
        // Once everything is in hand: what is looked at is among it.
        for live in looks {
            self.look_below(&live).await;
        }
        // A run whose instance is over cannot go on. It was cancelled as
        // its instance ended, and is ended as one that is (`left`).
        for left in unfinished {
            let now = self.inner.own.now_ms();
            self.end_left(&left, "cancelled", now).await?;
        }
        if !unlooked.is_empty() {
            self.inner.store.looked().await?;
        }
        Ok(resumed)
    }
}
