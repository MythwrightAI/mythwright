//! Getting an event to the handlers it is for.
//!
//! Handlers do not follow the journal. One task here does for each
//! journal, for the whole conductor ([`Deliveries`]: the factory's own
//! and each project's are followed apart, and a handler that hears more
//! than one waits in each, with a place in each: [`Inboxes`]), and
//! every handler that is waiting is kept by what it waits
//! for: its topic, and under that the field and value of its `for:`. An
//! event is looked up there and handed to the handlers found, so an event
//! addressed to one handler wakes that one, however many wait on its topic.
//! A handler with no `for:` is handed everything on its topic, as it always
//! was; what it no longer does is wake for every other topic to pass it by.
//!
//! What a handler is owed outlives the conductor. Each handler has a place
//! in the journal, moved in the transaction that starts a run. When it
//! starts to listen it is put on the index first and only then reads the
//! journal from its place, so nothing falls between the two: what was
//! published before it was on the index is in the journal by the time it
//! reads, and what was published after is handed to it. What reaches it
//! both ways is told apart by sequence number and taken once.
//!
//! A handler that nothing is addressed to is never woken, so it never moves
//! its own place, and after a restart would read its topic again from
//! wherever it began. So its place is moved for it: every [`SAVE_EVERY`]
//! events on a topic, the handlers waiting there with nothing in hand are
//! brought up to the event just handed out, in one write.
//!
//! Whoever wrote something may wait until the handlers it is for have
//! done with it ([`Delivery::done_with`]): each handler says, as it
//! comes back for more, the last event it has done with (its run for it
//! has ended, or begun where it takes turns by a key; or it passed it
//! over). Nothing of that is kept: after a restart nobody is waiting.
//!
//! It came out of the runner, which offers it under the names it always
//! gave (`conductor::PLACES_KEPT`, and `crate::delivery` there), and hands
//! it what begins a task and where the rows are ([`Hands`]).

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use blueprint::address::{self, Address};
use journal::{EventQuery, Follow, Journal};
use kernel::event::topic_matches;
use kernel::{Event, Result};
use tokio::sync::Notify;

use crate::Hands;

/// Events on a topic between writes of where its idle handlers have got to,
/// so that none of them reads more than this many again after a restart.
const SAVE_EVERY: u64 = 256;

/// What each write is measured as that keeps a handler's place for no
/// event of its own, by why (`kind`): the handlers idle on a topic brought
/// forward here (`idle`), or a handler that has passed over as much of
/// what it was handed (`passed`, in `trigger`). Which moment such a write
/// falls in goes by everything said before it, and by how a handler's
/// reading fell, so what one piece of work cost the factory is read
/// without them (`costed.rs` in the API suite).
pub const PLACES_KEPT: &str = "handler.places_kept";

/// Events read from the journal at a time while catching up.
const PAGE: usize = 256;

/// The most events that wait for one handler in memory. A handler further
/// behind than this reads the journal instead, which holds them anyway.
const TRAY_HOLDS: usize = 1024;

/// What a handler waits for.
pub struct Wanted {
    pub instance: String,
    pub handler: String,
    /// What its `event:` says: a topic, or a pattern over topics.
    pub pattern: String,
    pub address: Address,
    /// Its place in the journal. It is owed what came after.
    pub after: u64,
    /// What its measurements are labelled with.
    pub labels: Vec<(String, String)>,
}

/// What hands out the events of one journal.
#[derive(Clone)]
pub struct Delivery {
    index: Arc<Mutex<Index>>,
    journal: Journal,
}

/// What hands out the events of each journal, by what it is called.
#[derive(Clone, Default)]
pub struct Deliveries {
    each: Arc<Mutex<HashMap<String, Delivery>>>,
}

impl Deliveries {
    /// What hands out the events of `journal`.
    pub fn of(&self, journal: &Journal) -> Delivery {
        let mut each = self.each.lock().expect("delivery lock");
        let made = || Delivery { index: Arc::default(), journal: journal.clone() };
        each.entry(journal.name().to_string()).or_insert_with(made).clone()
    }

    /// What hands out the events of the journal called `journal`, where
    /// anything here has listened to it.
    pub fn found(&self, journal: &str) -> Option<Delivery> {
        self.each.lock().expect("delivery lock").get(journal).cloned()
    }
}

/// Every handler that is waiting, by what it waits for.
#[derive(Default)]
struct Index {
    /// Whether the task that hands events out is running.
    routing: bool,
    waiters: u64,
    /// Handlers waiting on one topic, by that topic.
    topics: HashMap<String, Waiting>,
    /// Handlers waiting on a pattern over topics. There is no looking these
    /// up: each pattern is tried against each event, and they are few.
    patterns: Vec<(String, Waiting)>,
}

/// The handlers waiting on one topic or pattern.
#[derive(Default)]
struct Waiting {
    /// Those with no `for:`, whose every event on the topic it is.
    unaddressed: Vec<Arc<Waiter>>,
    /// Those with one, by a field of the event's data and then by the
    /// value it must have there.
    addressed: HashMap<String, HashMap<String, Vec<Arc<Waiter>>>>,
    /// Events since the places of the idle ones were last written.
    since_saved: u64,
}

struct Waiter {
    id: u64,
    instance: String,
    handler: String,
    address: Address,
    tray: Mutex<Tray>,
    told: Notify,
    /// The last event it has done with: everything before it that was for
    /// it, it has done with too. All of them, once it has stopped listening.
    done: AtomicU64,
    /// Told each time `done` moves.
    passed: Notify,
}

/// What has been handed to a handler and not yet taken by it.
#[derive(Default)]
struct Tray {
    events: VecDeque<Arc<Event>>,
    /// More came than the tray holds, and what it held was let go: the
    /// handler has to read the journal from where it had got to.
    overflowed: bool,
    /// The handler has dealt with everything it was handed and is waiting
    /// for more. Its place may then be moved for it.
    idle: bool,
}

/// Handlers whose places are due to be brought forward, each an instance
/// and a handler's name.
type Idle = Vec<(String, String)>;

fn is_pattern(topic: &str) -> bool {
    topic.contains('*')
}

impl Delivery {
    /// Puts a handler among those waiting, and gives back what it reads its
    /// events from. Dropping that takes the handler off again.
    pub fn listen<H: Hands>(&self, hands: &H, wanted: Wanted) -> Inbox<H> {
        let mut index = self.index.lock().expect("delivery lock");
        self.route(hands, &mut index);
        index.waiters += 1;
        let waiter = Arc::new(Waiter {
            id: index.waiters,
            instance: wanted.instance,
            handler: wanted.handler,
            address: wanted.address,
            tray: Mutex::default(),
            told: Notify::new(),
            // What is at or before its place is done with, or not its.
            done: AtomicU64::new(wanted.after),
            passed: Notify::new(),
        });
        index.waiting(&wanted.pattern).add(waiter.clone());
        Inbox {
            delivery: self.clone(),
            hands: hands.clone(),
            pattern: wanted.pattern,
            labels: wanted.labels,
            waiter,
            read_to: wanted.after,
            backlog: VecDeque::new(),
            catching_up: Some(0),
            held: false,
            given: 0,
        }
    }

    /// Starts the task that hands events out, unless it is running. Called
    /// with the index locked, so that a handler put on the index is on it
    /// before the first event that task sees.
    fn route<H: Hands>(&self, hands: &H, index: &mut Index) {
        if index.routing {
            return;
        }
        index.routing = true;
        let journal = &self.journal;
        // From now: what came before, each handler reads for itself.
        let follow = journal.follow(journal.last_seq(), "");
        let (delivery, hands_too) = (self.clone(), hands.clone());
        hands.spawn(async move { delivery.hand_out(hands_too, follow).await });
    }

    async fn hand_out(self, hands: impl Hands, mut follow: Follow) {
        let journal = self.journal.clone();
        while let Ok(event) = follow.next().await {
            // Nothing that is not kept is ever a handler's: it could not be
            // handed over again after a restart.
            if !event.is_durable() {
                continue;
            }
            let idle = {
                let mut index = self.index.lock().expect("delivery lock");
                if index.topics.is_empty() && index.patterns.is_empty() {
                    // Nobody is waiting for anything. The next handler to
                    // listen starts this again, from where the journal is
                    // by then.
                    index.routing = false;
                    return;
                }
                index.hand(&event, &journal)
            };
            if !idle.is_empty() {
                // Nothing is lost if this fails: those handlers read a
                // little more of the journal after the next restart.
                let began = std::time::Instant::now();
                let moved = hands.store().advance_places(idle, journal.name(), event.seq).await;
                if moved.as_ref().is_ok_and(|moved| *moved > 0) {
                    let took = began.elapsed().as_secs_f64() * 1000.0;
                    journal.measure(PLACES_KEPT, &[("kind", "idle")], took, true);
                }
            }
        }
        // The journal could not be read. Every handler is sent back to it,
        // where it finds out for itself, and the first that can read it
        // starts this again; staying on the index and hearing nothing would
        // be the one outcome that looks like all being well.
        let mut index = self.index.lock().expect("delivery lock");
        index.routing = false;
        for waiting in index.topics.values().chain(index.patterns.iter().map(|(_, waiting)| waiting)) {
            for waiter in waiting.waiters() {
                waiter.tray.lock().expect("tray lock").overflow();
                waiter.told.notify_one();
            }
        }
    }

    /// Waits until each handler listening now that `event` is for has done
    /// with it: its run for it has ended (been begun, for one that takes
    /// turns by a key), or it passed it over. One that stops listening
    /// first is not waited for; one that takes its place is. Never waits
    /// for one whose place was already past it.
    pub async fn done_with(&self, event: &Event) {
        loop {
            let owed = self.index.lock().expect("delivery lock").owing(event);
            let Some(waiter) = owed.first() else { return };
            waiter.past(event.seq).await;
        }
    }

    /// Which handlers have not yet done with `event`, each an instance and
    /// a handler's name.
    pub fn owing(&self, event: &Event) -> Vec<(String, String)> {
        let owed = self.index.lock().expect("delivery lock").owing(event);
        owed.iter().map(|waiter| (waiter.instance.clone(), waiter.handler.clone())).collect()
    }

    fn forget(&self, pattern: &str, waiter: &Waiter) {
        let mut index = self.index.lock().expect("delivery lock");
        if is_pattern(pattern) {
            if let Some(at) = index.patterns.iter().position(|(known, _)| known == pattern) {
                index.patterns[at].1.remove(waiter);
                if index.patterns[at].1.is_empty() {
                    index.patterns.remove(at);
                }
            }
        } else if let Some(waiting) = index.topics.get_mut(pattern) {
            waiting.remove(waiter);
            if waiting.is_empty() {
                index.topics.remove(pattern);
            }
        }
    }
}

impl Index {
    /// The handlers `event` is for, of those listening, that have not yet
    /// done with it: the same it is handed to ([`Index::hand`]).
    fn owing(&self, event: &Event) -> Vec<Arc<Waiter>> {
        let topic = self.topics.get(&event.topic).into_iter();
        let patterns = self.patterns.iter().filter(|(pattern, _)| topic_matches(pattern, &event.topic));
        let waiting = topic.chain(patterns.map(|(_, waiting)| waiting));
        let for_it = waiting.flat_map(|waiting| waiting.for_event(event));
        for_it.filter(|waiter| waiter.done.load(Ordering::SeqCst) < event.seq).collect()
    }

    fn waiting(&mut self, pattern: &str) -> &mut Waiting {
        if !is_pattern(pattern) {
            return self.topics.entry(pattern.to_string()).or_default();
        }
        let at = match self.patterns.iter().position(|(known, _)| known == pattern) {
            Some(at) => at,
            None => {
                self.patterns.push((pattern.to_string(), Waiting::default()));
                self.patterns.len() - 1
            }
        };
        &mut self.patterns[at].1
    }

    /// Hands an event to the handlers it is for, and says whose places are
    /// due to be brought up to it.
    ///
    /// How many handlers it went to is measured for each topic or pattern
    /// it was looked up under, and labelled with that and not with the
    /// event's own topic: what handlers wait on is a few things said in
    /// blueprints, and what is published can be a topic for every card.
    fn hand(&mut self, event: &Arc<Event>, journal: &Journal) -> Idle {
        let mut idle = Idle::new();
        if let Some(waiting) = self.topics.get_mut(&event.topic) {
            let to = waiting.hand(event);
            journal.measure("handler.delivery", &[("topic", &event.topic)], to as f64, true);
            waiting.passed(&mut idle);
        }
        for (pattern, waiting) in &mut self.patterns {
            if topic_matches(pattern, &event.topic) {
                let to = waiting.hand(event);
                journal.measure("handler.delivery", &[("topic", pattern)], to as f64, true);
            }
            // A pattern cannot be looked up in the journal, so catching up
            // on one reads everything: every event counts towards it.
            waiting.passed(&mut idle);
        }
        idle
    }
}

impl Waiting {
    fn waiters(&self) -> impl Iterator<Item = &Arc<Waiter>> {
        self.unaddressed.iter().chain(self.addressed.values().flat_map(|values| values.values().flatten()))
    }

    fn is_empty(&self) -> bool {
        self.unaddressed.is_empty() && self.addressed.is_empty()
    }

    fn add(&mut self, waiter: Arc<Waiter>) {
        match waiter.address.first() {
            None => self.unaddressed.push(waiter),
            Some((field, value)) => {
                let values = self.addressed.entry(field.to_string()).or_default();
                values.entry(value.to_string()).or_default().push(waiter.clone());
            }
        }
    }

    fn remove(&mut self, waiter: &Waiter) {
        let Some((field, value)) = waiter.address.first() else {
            self.unaddressed.retain(|other| other.id != waiter.id);
            return;
        };
        let Some(values) = self.addressed.get_mut(field) else { return };
        if let Some(waiters) = values.get_mut(value) {
            waiters.retain(|other| other.id != waiter.id);
            if waiters.is_empty() {
                values.remove(value);
            }
        }
        if values.is_empty() {
            self.addressed.remove(field);
        }
    }

    /// Hands an event on this topic to those of the handlers here that it
    /// is for, and says how many that was.
    fn hand(&self, event: &Arc<Event>) -> usize {
        let to = self.for_event(event);
        to.iter().for_each(|waiter| waiter.hand(event));
        to.len()
    }

    /// The handlers here that an event on this topic is for.
    fn for_event(&self, event: &Event) -> Vec<Arc<Waiter>> {
        let mut to = self.unaddressed.clone();
        // One look per field that anyone here is addressed by, which is one
        // or two, and not one per handler.
        for (field, values) in &self.addressed {
            let found = address::at(&event.data, field).and_then(|value| values.get(&address::key(value)));
            // The field it was found under is one of those it asked for;
            // the others have to hold too.
            let carried = found.into_iter().flatten().filter(|waiter| waiter.address.carried_by(&event.data));
            to.extend(carried.cloned());
        }
        to
    }

    /// Counts an event that the handlers here would read again after a
    /// restart, and says whose places to move when enough have passed.
    fn passed(&mut self, idle: &mut Idle) {
        self.since_saved += 1;
        if self.since_saved < SAVE_EVERY {
            return;
        }
        self.since_saved = 0;
        let waiting = self.waiters().filter(|waiter| waiter.tray.lock().expect("tray lock").is_idle());
        idle.extend(waiting.map(|waiter| (waiter.instance.clone(), waiter.handler.clone())));
    }
}

impl Waiter {
    /// Says it has done with everything up to `seq` that was for it.
    fn done_with(&self, seq: u64) {
        self.done.fetch_max(seq, Ordering::SeqCst);
        self.passed.notify_waiters();
    }

    /// Waits until it has done with everything up to `seq` that was for it.
    async fn past(&self, seq: u64) {
        loop {
            let mut passed = std::pin::pin!(self.passed.notified());
            passed.as_mut().enable();
            if self.done.load(Ordering::SeqCst) >= seq {
                return;
            }
            passed.await;
        }
    }

    fn hand(&self, event: &Arc<Event>) {
        let mut tray = self.tray.lock().expect("tray lock");
        if tray.events.len() >= TRAY_HOLDS {
            tray.overflow();
        }
        // Until the handler has gone back to the journal, what comes is
        // there for it to read; keeping it here as well would only be
        // dropped as already seen.
        if !tray.overflowed {
            tray.events.push_back(event.clone());
        }
        drop(tray);
        self.told.notify_one();
    }
}

impl Tray {
    fn overflow(&mut self) {
        self.events.clear();
        self.overflowed = true;
        self.idle = false;
    }

    fn is_idle(&self) -> bool {
        self.idle && self.events.is_empty() && !self.overflowed
    }
}

/// Where one handler reads its events from, in the order they happened:
/// first what the journal has since its place, then what is handed to it.
pub struct Inbox<H: Hands> {
    delivery: Delivery,
    hands: H,
    pattern: String,
    labels: Vec<(String, String)>,
    waiter: Arc<Waiter>,
    /// Everything up to here has been read from the journal or taken from
    /// the tray; what is still in `backlog` is at or before it.
    read_to: u64,
    /// Read from the journal and not yet handed over.
    backlog: VecDeque<Arc<Event>>,
    /// While it is reading the journal: how many events it has read so far.
    catching_up: Option<u64>,
    /// The handler has been given an event and has not said it is done
    /// with it ([`Inbox::rest`]).
    held: bool,
    /// The event it was last given.
    given: u64,
}

impl<H: Hands> Inbox<H> {
    /// The next event for this handler that is there to be had without
    /// waiting: read from the journal already, or handed to it. Nothing
    /// where there is none, or where it has first to read the journal
    /// ([`Inbox::reads`]).
    fn take(&mut self) -> Option<Arc<Event>> {
        if let Some(event) = self.backlog.pop_front() {
            (self.held, self.given) = (true, event.seq);
            return Some(event);
        }
        if self.catching_up.is_some() {
            return None;
        }
        let mut tray = self.waiter.tray.lock().expect("tray lock");
        if tray.overflowed {
            tray.overflowed = false;
            tray.events.clear();
            self.catching_up = Some(0);
            return None;
        }
        while let Some(event) = tray.events.pop_front() {
            // Anything at or before this was read from the journal.
            if event.seq > self.read_to {
                tray.idle = false;
                self.read_to = event.seq;
                (self.held, self.given) = (true, event.seq);
                return Some(event);
            }
        }
        tray.idle = !self.held;
        None
    }

    /// Whether it has the journal to read before it can say what is next.
    fn reads(&self) -> bool {
        self.backlog.is_empty() && self.catching_up.is_some()
    }

    /// The journal it is handed the events of.
    pub fn journal(&self) -> &Journal {
        &self.delivery.journal
    }

    /// Says that whatever [`Inbox::next`] last gave has been dealt with: a
    /// run was recorded for it, or it was passed over. Until then the
    /// handler's place is its own to move.
    pub fn rest(&mut self) {
        if self.held {
            self.waiter.done_with(self.given);
        }
        self.held = false;
    }

    /// The place to start from after a restart: everything up to it has
    /// been handed over by [`Inbox::next`], or was not this handler's.
    pub fn settled(&self) -> u64 {
        match self.backlog.front() {
            Some(waiting) => waiting.seq - 1,
            None => self.read_to,
        }
    }

    /// Reads the next of what the journal has for this handler since its
    /// place, and notices when that was the last of it.
    async fn read_journal(&mut self) -> Result<()> {
        // If what hands events out has stopped, it is started before the
        // journal is read, for the same reason a handler is put on the
        // index before it reads: so that nothing falls between.
        {
            let mut index = self.delivery.index.lock().expect("delivery lock");
            self.delivery.route(&self.hands, &mut index);
        }
        let last = self.journal().last_seq();
        // One topic is read through the journal's index, a page of that
        // topic's events at a time. A pattern has no index to go by, so it
        // is read a stretch of the journal at a time instead: asking for a
        // page of matches could read the whole of it in one go.
        let before = match is_pattern(&self.pattern) {
            true => (self.read_to + PAGE as u64).min(last) + 1,
            false => 0,
        };
        let query = EventQuery {
            after: self.read_to,
            before,
            topics: self.pattern.clone(),
            limit: PAGE,
            ..EventQuery::default()
        };
        let page = self.journal().read(query).await?;
        let (read, whole) = (self.catching_up.unwrap_or(0) + page.len() as u64, page.len() < PAGE);
        for event in page {
            self.read_to = event.seq;
            if self.waiter.address.carried_by(&event.data) {
                self.backlog.push_back(Arc::new(event));
            }
        }
        let done = match before {
            0 => whole,
            _ => {
                self.read_to = self.read_to.max(before - 1);
                self.read_to >= last
            }
        };
        self.catching_up = match done {
            true => {
                let labels: Vec<(&str, &str)> =
                    self.labels.iter().map(|(name, value)| (name.as_str(), value.as_str())).collect();
                self.journal().measure("handler.catch_up", &labels, read as f64, true);
                None
            }
            false => Some(read),
        };
        Ok(())
    }
}

impl<H: Hands> Drop for Inbox<H> {
    fn drop(&mut self) {
        self.delivery.forget(&self.pattern, &self.waiter);
        // Nobody waits for it any more; whatever takes its place is waited for.
        self.waiter.done_with(u64::MAX);
    }
}

/// Where one handler reads its events from, journal by journal: an
/// [`Inbox`] for each journal it hears. Each journal's events come in
/// their order; of two journals' nothing is said of which comes first,
/// and neither is kept waiting behind the other.
pub struct Inboxes<H: Hands> {
    each: Vec<Inbox<H>>,
    /// Which is looked at first next time.
    turn: usize,
    /// Which gave the event last handed over.
    last: usize,
    /// The factory's journal, which says when another is made.
    journals: Journal,
    /// How many journals there were when whoever reads was last told
    /// that one had been made.
    known: usize,
}

impl<H: Hands> Inboxes<H> {
    /// Made before whoever reads looks at which journals there are: one
    /// made between the two is then told of, and looked for once more.
    pub fn new(journals: &Journal, each: Vec<Inbox<H>>) -> Inboxes<H> {
        Inboxes { each, turn: 0, last: 0, journals: journals.clone(), known: journals.every().len() }
    }

    /// Has the handler read from one journal more.
    pub fn add(&mut self, inbox: Inbox<H>) {
        self.each.push(inbox);
    }

    /// Whether the handler reads from the journal called `journal`.
    pub fn has(&self, journal: &str) -> bool {
        self.each.iter().any(|inbox| inbox.journal().name() == journal)
    }

    /// The next event for this handler, of whichever journal. Waits if
    /// there is none yet. Nothing, where a journal has been made since
    /// whoever asks was last told of one: they see whether the handler
    /// is to read from that one too, and ask again. Whenever it was
    /// made: a handler is here only between its runs, and a journal
    /// made while it was in the middle of one was counted among those
    /// there were when it next waited, and never read by it.
    ///
    /// Safe to drop while it waits: nothing is taken until it is
    /// returned. A journal that has to be read first is read before any
    /// is waited on, each in its turn, so that one with a great deal
    /// handed to it does not keep another from being read.
    pub async fn next(&mut self) -> Result<Option<Arc<Event>>> {
        loop {
            let made = self.journals.every().len();
            if made > self.known {
                self.known = made;
                return Ok(None);
            }
            let many = self.each.len();
            for step in 0..many {
                let at = (self.turn + step) % many;
                let inbox = &mut self.each[at];
                if inbox.reads() {
                    inbox.read_journal().await?;
                }
                if let Some(event) = inbox.take() {
                    (self.turn, self.last) = ((at + 1) % many, at);
                    return Ok(Some(event));
                }
            }
            if self.each.iter().any(Inbox::reads) {
                continue;
            }
            // Until one of them is handed something, or a journal is made.
            let waiters: Vec<Arc<Waiter>> = self.each.iter().map(|inbox| inbox.waiter.clone()).collect();
            let mut told: Vec<_> = waiters.iter().map(|waiter| Box::pin(waiter.told.notified())).collect();
            let mut made = Box::pin(self.journals.more_than(self.known));
            std::future::poll_fn(|cx| {
                if told.iter_mut().any(|told| told.as_mut().poll(cx).is_ready()) {
                    return std::task::Poll::Ready(());
                }
                made.as_mut().poll(cx)
            })
            .await;
        }
    }

    /// Says that whatever [`Inboxes::next`] last gave has been dealt with.
    pub fn rest(&mut self) {
        self.each.iter_mut().for_each(Inbox::rest);
    }

    /// What the event last handed over came from.
    pub fn last(&self) -> &Inbox<H> {
        &self.each[self.last]
    }

    /// Each journal it reads from.
    pub fn each(&self) -> &[Inbox<H>] {
        &self.each
    }
}
