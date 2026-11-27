//! The conductors of a factory: its own, and one for each project whose
//! journal keeps what is run for it.
//!
//! A project with a great deal running is not to hold up another, so
//! what is run for a project is kept in the project's own journal
//! (`journal::set`) by a conductor of the project's own: its instances
//! in its own hand, its runs its own tasks, and its own turn at making
//! an instance, at whoever waits for a run and at handing events out.
//! The factory's own conductor runs what is nobody's, and every project
//! whose journal does not keep what is run for it: all of them, in a
//! factory a program from before made.
//!
//! **Which projects have one is exactly which journals say they keep
//! what is run** (`Journal::keepers`), and is never worked out: not from
//! a project's id, and not from where an instance was found.
//!
//! The factory's own is the front: it is what a node is handed, and
//! whatever is asked of it is asked of the conductor that keeps the
//! thing ([`Engine::of`] for a project, and for an instance or a run the
//! one that has it, [`Engine::elsewhere`]). Any of them answers so, so a
//! step's own conductor (`ActionCx::engine`) can be asked of anything.
//! What all of them share is held once behind something each is handed
//! (the registry, the library, the hooks, how texts are taken, what
//! answers for comparisons). What each keeps beside its rows is in its
//! own journal, with them ([`crate::beside`]).

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock, Weak};

use kernel::{Error, Result};
use runs::InstanceInfo;

use crate::engine::{Engine, Inner};

/// How many ids are remembered with the conductor that keeps each.
const KNOWN: usize = 4096;

/// How far a conductor has come. What another conductor, or anybody
/// outside, asks of one that is taking up what it keeps waits until it
/// has, so that nothing is answered from a hand that is half filled or
/// done to an instance that is not in hand; and what is asked of one
/// that was stopped alone waits for the one started in its place.
///
/// **The order, which is what keeps a start from waiting for ever.**
/// The conductors are started one after another, the factory's own
/// first and then each project's by name ([`Engine::resume`]), and a
/// start that waited on one that starts after it would wait on what
/// cannot begin until it is over. So what a start does itself never
/// waits ([`WITHIN_A_START`]): it is answered from what is written,
/// which is all a start asks of another conductor. What a start has
/// taken up is a task of its own and does wait. Something new a start
/// does itself must not wait for a task that asks another conductor:
/// that is the one way left to hold a start up, and a play holds that
/// it is not taken (`alone.rs` in the suite).
pub(crate) mod standing {
    /// Made, and never started: it answers from what is written, as a
    /// conductor that nobody starts always did.
    pub const MADE: u8 = 0;
    pub const STARTING: u8 = 1;
    pub const UP: u8 = 2;
    /// Stopped alone ([`super::Engine::halt_one`]).
    pub const DOWN: u8 = 3;
}

tokio::task_local! {
    /// Set for what a start of the factory's conductors does itself.
    static WITHIN_A_START: ();
}

/// The conductors of one factory, as each of them holds them.
#[derive(Default)]
pub(crate) struct Conductors {
    /// The factory's own, which holds this and so is not held by it.
    front: OnceLock<Weak<Inner>>,
    /// Each project's own, by the project.
    beside: RwLock<BTreeMap<String, Engine>>,
    /// One is made at a time.
    making: tokio::sync::Mutex<()>,
    /// Whether the factory's conductor has been started: one made for a
    /// journal made since is started as it is made.
    started: AtomicBool,
    /// Which conductor keeps an instance or a run that was asked for
    /// and is in nobody's hand: by its project, none for the factory's.
    known: Mutex<HashMap<String, Option<String>>>,
    /// Those that were let go of as the factory's was halted, until
    /// nothing holds them.
    let_go: Mutex<Vec<Weak<Inner>>>,
    /// Told each time a conductor's standing changes, or one is halted.
    pub told: tokio::sync::Notify,
    /// The factory is draining to stop ([`Engine::hold_new_work`]):
    /// nothing new is begun by any conductor of it.
    held: AtomicBool,
    /// How many instances the conductors have taken up since they were
    /// made ([`Engine::how_far`]).
    pub taken: AtomicUsize,
}

impl Conductors {
    /// Whether the factory's own conductor is the only one there is.
    pub fn alone(&self) -> bool {
        self.beside.read().expect("conductors lock").is_empty()
    }

    fn of(&self, project: &str) -> Option<Engine> {
        self.beside.read().expect("conductors lock").get(project).cloned()
    }

    fn beside(&self) -> Vec<Engine> {
        self.beside.read().expect("conductors lock").values().cloned().collect()
    }

    fn remember(&self, id: &str, keeper: &Engine) {
        let mut known = self.known.lock().expect("conductors lock");
        if known.len() >= KNOWN {
            known.clear();
        }
        known.insert(id.to_string(), keeper.inner.project.clone());
    }
}

impl Engine {
    /// Begins nothing new from here on, in any conductor, for a daemon
    /// draining to stop: nothing somebody applies, and no handler's run
    /// (it waits with its place unmoved, for the next daemon). What is
    /// under way goes on to its end. Never taken back: the daemon stops.
    pub fn hold_new_work(&self) {
        self.inner.set.held.store(true, Ordering::SeqCst);
    }

    /// Whether nothing new is begun ([`Engine::hold_new_work`]).
    pub fn holds_new_work(&self) -> bool {
        self.inner.set.held.load(Ordering::SeqCst)
    }

    /// Refuses what somebody applies while nothing new is begun.
    pub(crate) fn not_while_held(&self, blueprint: &str) -> Result<()> {
        if !self.holds_new_work() {
            return Ok(());
        }
        Err(Error::conflict(format!(
            "`{blueprint}` was not applied: the daemon is draining to stop, and begins nothing new. Apply it again \
             once it is started again (`{} up`)",
            brand::CLI
        )))
    }

    /// Has this conductor, the factory's own, be the front of the set
    /// it holds, and makes one beside it for each project whose journal
    /// keeps what is run for it.
    pub(crate) async fn with_each(self) -> Result<Engine> {
        let _ = self.inner.set.front.set(Arc::downgrade(&self.inner));
        for (project, own) in self.inner.factory.keepers() {
            if !self.gives_over(&project).await {
                continue;
            }
            let made = self.beside(&project, own).await?;
            self.inner.set.beside.write().expect("conductors lock").insert(project, made);
        }
        Ok(self)
    }

    /// The factory's own conductor.
    pub(crate) fn front(&self) -> Engine {
        if self.inner.project.is_none() {
            return self.clone();
        }
        let front = self.inner.set.front.get().and_then(Weak::upgrade);
        // Gone only where the factory's was let go of, and then nothing
        // more is run: this one answers as far as it can.
        front.map_or_else(|| self.clone(), |inner| Engine { inner })
    }

    /// Every conductor there is: the factory's own first, then each
    /// project's in the order of the projects' names.
    pub(crate) fn every(&self) -> Vec<Engine> {
        let mut every = vec![self.front()];
        every.extend(self.inner.set.beside());
        every
    }

    /// Every conductor but this one, the factory's own first.
    pub(crate) fn others(&self) -> Vec<Engine> {
        if self.inner.set.alone() {
            return Vec::new();
        }
        self.every().into_iter().filter(|other| !Arc::ptr_eq(&other.inner, &self.inner)).collect()
    }

    /// The conductor that runs what is applied for `project`: its own
    /// where the project's journal keeps what is run for it, and
    /// otherwise the factory's, which also runs what is nobody's. One
    /// is made, and started, for a journal made since the factory's
    /// conductor was: a project that was added while it ran.
    pub(crate) async fn of(&self, project: Option<&str>) -> Result<Engine> {
        let Some(project) = project.filter(|project| !project.is_empty()) else {
            return Ok(self.up(self.front()).await);
        };
        if self.inner.project.as_deref() == Some(project) {
            return Ok(self.up(self.clone()).await);
        }
        let set = &self.inner.set;
        if let Some(found) = set.of(project) {
            return Ok(self.up(found).await);
        }
        let Some(own) = self.inner.factory.rows_of(project) else { return Ok(self.up(self.front()).await) };
        let _making = set.making.lock().await;
        if let Some(found) = set.of(project) {
            return Ok(self.up(found).await);
        }
        let front = self.front();
        if !front.gives_over(project).await {
            return Ok(self.up(front).await);
        }
        let made = front.beside(project, own).await?;
        // Nothing is running in a journal that was just made; started
        // all the same, so that it is as one that was there at the start.
        if set.started.load(Ordering::SeqCst) {
            Box::pin(made.resume_own()).await?;
        }
        set.beside.write().expect("conductors lock").insert(project.to_string(), made.clone());
        Ok(made)
    }

    /// Whether the factory's own conductor, asked, has nothing alive
    /// that was applied for `project`: only then is the project run by
    /// a conductor of its own. A project of a factory from before that
    /// was taken away and added again has a journal that keeps what is
    /// run for it, and may still have something running that the
    /// factory's conductor keeps; run by two, what is applied for it
    /// again would be started a second time beside what is there. It
    /// stays with the factory's until a start finds nothing of it alive
    /// there.
    pub(crate) async fn gives_over(&self, project: &str) -> bool {
        self.inner.store.alive_of(Some(project)).await.is_ok_and(|alive| alive.is_empty())
    }

    /// Says that the factory's conductor has been started, so that one
    /// made from here on is started as it is made.
    pub(crate) fn started(&self) {
        self.inner.set.started.store(true, Ordering::SeqCst);
    }

    /// Whether `id` is an instance in some conductor's hand: for what
    /// asks whether something is still there (`pool.turn`), whichever
    /// conductor it is asked of. A conductor that is taking up what it
    /// keeps, or was stopped alone, has not all of it in hand: what it
    /// keeps is there where it is written as alive.
    pub async fn is_there(&self, id: &str) -> bool {
        if self.live(id).is_some() {
            return true;
        }
        for one in self.every() {
            if one.live(id).is_some() {
                return true;
            }
            if matches!(one.stands(), standing::STARTING | standing::DOWN)
                && one.inner.store.instance(id).await.ok().flatten().is_some_and(|kept| kept.status.is_live())
            {
                return true;
            }
        }
        false
    }

    /// The blueprint an instance runs, by the name it was made with,
    /// where some conductor has it in hand: asks nothing of a database,
    /// so that what reads it as each event is heard (whose step said
    /// something of a record) costs nothing for what is under way.
    pub fn running_in_hand(&self, id: &str) -> Option<String> {
        let live = self.every().iter().find_map(|one| one.live(id))?;
        Some(live.meta.lock().expect("live lock").blueprint.clone())
    }

    /// Another conductor that keeps the instance `id`, where this one
    /// does not: asked once this one's own rows have been found not to
    /// hold it, or where nothing of it is in this one's hand. In hand
    /// first, which asks nothing; then what was found before; then each
    /// conductor's rows.
    pub(crate) async fn elsewhere(&self, id: &str) -> Option<Engine> {
        // Found by what is written, and asked once it can be.
        let found = self.found_elsewhere(id).await?;
        Some(self.up(found).await)
    }

    async fn found_elsewhere(&self, id: &str) -> Option<Engine> {
        let others = self.others();
        if others.is_empty() {
            return None;
        }
        if let Some(found) = others.iter().find(|other| other.live(id).is_some()) {
            return Some(found.clone());
        }
        if let Some(found) = self.known(id, &others) {
            return found;
        }
        for other in &others {
            if other.inner.store.project_of(id).await.ok().flatten().is_some() {
                self.inner.set.remember(id, other);
                return Some(other.clone());
            }
        }
        None
    }

    /// Like [`Engine::elsewhere`], for a run.
    pub(crate) async fn elsewhere_run(&self, id: &str) -> Option<Engine> {
        let found = self.found_elsewhere_run(id).await?;
        Some(self.up(found).await)
    }

    async fn found_elsewhere_run(&self, id: &str) -> Option<Engine> {
        let others = self.others();
        if others.is_empty() {
            return None;
        }
        if let Some(found) = self.known(id, &others) {
            return found;
        }
        for other in &others {
            if other.inner.store.project_of_run(id).await.ok().flatten().is_some() {
                self.inner.set.remember(id, other);
                return Some(other.clone());
            }
        }
        None
    }

    /// What was found before of who keeps `id`: another, or (the inner
    /// none) this one.
    fn known(&self, id: &str, others: &[Engine]) -> Option<Option<Engine>> {
        let known = self.inner.set.known.lock().expect("conductors lock").get(id).cloned()?;
        if known == self.inner.project {
            return Some(None);
        }
        others.iter().find(|other| other.inner.project == known).map(|found| Some(found.clone()))
    }

    /// Keeps which conductor has `id`, an instance or a run that was
    /// just made in it or found in it.
    pub(crate) fn keeps(&self, id: &str) {
        if !self.inner.set.alone() {
            self.inner.set.remember(id, self);
        }
    }

    /// The conductor that keeps the instance `id` where this one's own
    /// hand and rows do not: itself where it does, or where nobody does.
    pub(crate) async fn keeper(&self, id: &str) -> Engine {
        if self.inner.set.alone() || self.live(id).is_some() {
            return self.clone();
        }
        if let Some(found) = self.others().into_iter().find(|other| other.live(id).is_some()) {
            return self.up(found).await;
        }
        if let Some(known) = self.known(id, &self.others()) {
            return match known {
                Some(another) => self.up(another).await,
                None => self.clone(),
            };
        }
        if self.inner.store.project_of(id).await.ok().flatten().is_some() {
            self.inner.set.remember(id, self);
            return self.clone();
        }
        self.elsewhere(id).await.unwrap_or_else(|| self.clone())
    }

    /// Another conductor that keeps the instance `id`, where this one
    /// does not: for what is asked of any conductor about an instance
    /// and is answered from what is kept with it. Nothing where this one
    /// keeps it or nobody does, and nothing asked where it is in this
    /// one's hand or there is one conductor.
    pub(crate) async fn kept_by_another(&self, id: &str) -> Option<Engine> {
        if self.inner.set.alone() || self.live(id).is_some() {
            return None;
        }
        let keeper = self.keeper(id).await;
        (!Arc::ptr_eq(&keeper.inner, &self.inner)).then_some(keeper)
    }

    /// Every instance that is alive, in whichever conductor's rows: for
    /// what is tidied once for the factory as it starts, which must not
    /// take what another conductor keeps for something that is gone.
    pub(crate) async fn alive_everywhere(&self) -> Vec<InstanceInfo> {
        let mut alive = self.live_instances().await;
        for other in self.others() {
            alive.extend(other.live_instances().await);
        }
        alive
    }

    /// Lets go of every conductor beside the factory's own, each
    /// halted: asked of the factory's as it is halted. They hold the
    /// set and the set holds them, so nothing is given back until this.
    pub(crate) fn let_go(&self) {
        let beside = std::mem::take(&mut *self.inner.set.beside.write().expect("conductors lock"));
        let mut let_go = self.inner.set.let_go.lock().expect("conductors lock");
        for (_, other) in beside {
            other.halt_own();
            let_go.push(Arc::downgrade(&other.inner));
        }
    }

    /// Whether anything still holds a conductor that was let go of.
    pub(crate) fn still_held(&self) -> bool {
        let mut let_go = self.inner.set.let_go.lock().expect("conductors lock");
        let_go.retain(|one| one.strong_count() > 0);
        !let_go.is_empty()
    }
}

impl Engine {
    pub(crate) fn stands(&self) -> u8 {
        self.inner.standing.load(Ordering::SeqCst)
    }

    /// Says how far this conductor has come, to whoever waits on it.
    pub(crate) fn stand(&self, standing: u8) {
        self.inner.standing.store(standing, Ordering::SeqCst);
        self.inner.set.told.notify_waiters();
    }

    /// Does `start`, what a start of conductors does itself: nothing
    /// in it waits on another conductor ([`standing`]).
    pub(crate) async fn within_a_start<T>(start: impl Future<Output = T>) -> T {
        WITHIN_A_START.scope((), start).await
    }

    pub(crate) fn is_within_a_start() -> bool {
        WITHIN_A_START.try_with(|_| ()).is_ok()
    }

    /// The conductor that now runs what this one ran, once it can be
    /// asked: itself, or the one started in its place where it was
    /// stopped alone, when that has taken up what this one left. At
    /// once where the factory is being halted: nothing more is run.
    pub(crate) async fn when_up(&self) -> Engine {
        let mut one = self.clone();
        loop {
            // Listening before looking, so that a change in between is heard.
            let told = self.inner.set.told.notified();
            tokio::pin!(told);
            told.as_mut().enable();
            let set = &self.inner.set;
            if let Some(now) = one.inner.project.as_deref().and_then(|project| set.of(project)) {
                one = now;
            }
            let waits = matches!(one.stands(), standing::STARTING | standing::DOWN);
            if !waits || self.front().is_halting() {
                return one;
            }
            told.await;
        }
    }

    /// `other`, once it can be asked ([`Engine::when_up`]): at once
    /// where what asks is a start, and where it is this conductor
    /// itself, which does not wait for its own start.
    pub(crate) async fn up(&self, other: Engine) -> Engine {
        let itself = Arc::ptr_eq(&other.inner, &self.inner) && other.stands() != standing::DOWN;
        if itself || Engine::is_within_a_start() {
            return other;
        }
        other.when_up().await
    }

    /// Every conductor there is, each once it can be asked.
    pub(crate) async fn every_up(&self) -> Vec<Engine> {
        let mut every = Vec::new();
        for one in self.every() {
            every.push(self.up(one).await);
        }
        every
    }

    /// Stops the conductor of `project` and no other, as
    /// [`Engine::halt`] stops them all, until it is started again
    /// ([`Engine::start_one`]): the others go on, and what they ask of
    /// this one waits. Whether the project had one that was running.
    pub fn halt_one(&self, project: &str) -> bool {
        let Some(one) = self.inner.set.of(project).filter(|one| one.stands() != standing::DOWN) else {
            return false;
        };
        one.stand(standing::DOWN);
        one.halt_own();
        true
    }

    /// Waits until nothing the conductor of `project` started before it
    /// was stopped alone is still about, as [`Engine::halted`] does for
    /// them all: a task is stopped when it next comes to be run.
    pub async fn halted_one(&self, project: &str) {
        let Some(one) = self.inner.set.of(project) else { return };
        for _ in 0..100_000 {
            if one.tasks_gone() {
                return;
            }
            tokio::task::yield_now().await;
        }
    }

    /// Starts a conductor for `project` in place of the one that was
    /// stopped alone, on the same journal, and has it take up what was
    /// left: how many instances it brought back. What waited on the
    /// project's conductor is answered by this one, once it has.
    pub async fn start_one(&self, project: &str) -> Result<usize> {
        let set = &self.inner.set;
        let _making = set.making.lock().await;
        let Some(was) = set.of(project).filter(|one| one.stands() == standing::DOWN) else {
            return Err(Error::conflict(format!(
                "the conductor of the project `{project}` was not started: only one that was stopped by \
                 itself is started by itself, and this one is running, or the project is run by the factory's own"
            )));
        };
        let made = self.front().beside(project, was.inner.own.clone()).await?;
        made.stand(standing::STARTING);
        set.beside.write().expect("conductors lock").insert(project.to_string(), made.clone());
        made.resume_own().await
    }
}
