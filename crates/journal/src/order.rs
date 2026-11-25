//! The order things were kept in, across every journal of a factory:
//! for a program that is run to be tested.
//!
//! An event's number says where it is in its own journal and nothing of
//! any other ([`crate::giver`]): of two events of different journals the
//! later may have the lesser number. A daemon needs no more, and nothing
//! in it goes by more. A test does: it does something, and waits for
//! what comes of it, which may be said in another journal than what it
//! did was said in; "after" has to mean after, whichever journal keeps
//! each.
//!
//! So a factory that is tested may be told to write the order down
//! ([`crate::Journal::orders_by`]): each event, as it is written by the
//! thread of the journal that keeps it, is given the next of one count
//! that all its journals share. Of two events of which one was written
//! once the other was kept, the later has the greater; two written side
//! by side in two journals come in the order their threads came to them.
//! Nothing is kept of it on the disk, and nothing in the program reads
//! it: whoever tests asks ([`Order::of`]).
//!
//! A program that is run to be used keeps none, and pays for one look at
//! an empty cell for each event.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Where the count begins: far above any number an event has, so that
/// one taken for the other is seen at once.
const BEGINS: u64 = 1 << 40;

/// The order events were written in, whichever journal keeps each.
pub struct Order {
    last: AtomicU64,
    /// What was passed on with a number and not kept, in the order it
    /// was said: what a test reads of it cannot be read back from the
    /// journal ([`crate::Writing::pass_on`]).
    passed: Mutex<Vec<(String, std::sync::Arc<kernel::Event>)>>,
    /// Their numbers.
    numbers: Mutex<std::collections::HashSet<u64>>,
    /// Each event's place in the order, by its number, and its number by
    /// its place.
    places: Mutex<(HashMap<u64, u64>, HashMap<u64, u64>)>,
}

impl Default for Order {
    fn default() -> Order {
        Order {
            last: AtomicU64::new(BEGINS),
            passed: Mutex::default(),
            numbers: Mutex::default(),
            places: Mutex::default(),
        }
    }
}

impl Order {
    /// The event numbered `seq` is written: on the thread that writes it.
    pub(crate) fn written(&self, seq: u64) {
        let mut places = self.places.lock().expect("order lock");
        // Counted under the lock, so that the count and what is kept of
        // it are in one order.
        let at = self.last.fetch_add(1, Ordering::SeqCst) + 1;
        places.0.insert(seq, at);
        places.1.insert(at, seq);
    }

    /// An event was passed on with a number and not kept: on the thread
    /// that passed it on, once the write it was said in is kept.
    pub(crate) fn passed(&self, journal: &str, event: &std::sync::Arc<kernel::Event>) {
        self.passed.lock().expect("order lock").push((journal.to_string(), event.clone()));
        self.numbers.lock().expect("order lock").insert(event.seq);
    }

    /// Whether the event numbered `seq` was passed on and not kept.
    pub fn was_passed(&self, seq: u64) -> bool {
        self.numbers.lock().expect("order lock").contains(&seq)
    }

    /// Everything that was passed on with a number and not kept, in the
    /// order it was said, each with its number: what reading the journal
    /// does not find.
    pub fn said_passing(&self) -> Vec<std::sync::Arc<kernel::Event>> {
        self.passed.lock().expect("order lock").iter().map(|(_, event)| event.clone()).collect()
    }

    /// The same, each with the journal it was said in.
    fn passing_in(&self) -> Vec<(String, std::sync::Arc<kernel::Event>)> {
        self.passed.lock().expect("order lock").clone()
    }

    /// Where the event numbered `seq` came. Nothing, for one that was
    /// written before the order was kept, or by a program that kept none.
    pub fn of(&self, seq: u64) -> Option<u64> {
        self.places.lock().expect("order lock").0.get(&seq).copied()
    }

    /// The number of the event that came at `at`.
    pub fn number(&self, at: u64) -> Option<u64> {
        self.places.lock().expect("order lock").1.get(&at).copied()
    }

    /// Where the last event written came: everything written from now
    /// on comes after it, and everything written so far at or before.
    pub fn so_far(&self) -> u64 {
        self.last.load(Ordering::SeqCst)
    }
}

/// Whether `query` would have found `event`, had it been kept.
fn asks(query: &crate::EventQuery, event: &kernel::Event) -> bool {
    let topics = query.topics.trim();
    (topics.is_empty() || kernel::event::topic_matches_any(topics, &event.topic))
        && query.instance.as_ref().is_none_or(|instance| event.source.instance.as_ref() == Some(instance))
        && query.run.as_ref().is_none_or(|run| event.source.run.as_ref() == Some(run))
        && event.seq > query.after
        && (query.before == 0 || event.seq < query.before)
}

impl Order {
    /// What `query` asks of every journal there is, as
    /// [`crate::Journal::read_every`] answers it, with what was passed on
    /// and not kept that it would have found had it been kept: for a test,
    /// which is held to what was said and not to what is kept of it. In
    /// the order things were written in, the newest first where that is
    /// asked.
    pub async fn read_every(
        &self,
        journal: &crate::Journal,
        query: crate::EventQuery,
    ) -> kernel::Result<Vec<kernel::Event>> {
        let limit = if query.limit == 0 { crate::events::PAGE } else { query.limit };
        let mut events = journal.read_every(query.clone()).await?;
        events.extend(
            self.said_passing().iter().filter(|event| asks(&query, event)).map(|event| (**event).clone()),
        );
        events.sort_by_key(|event| (self.of(event.seq).unwrap_or(0), event.seq));
        if query.newest_first {
            events.reverse();
        }
        events.truncate(limit);
        Ok(events)
    }

    /// Follows every journal on `topics` from the beginning, as
    /// [`crate::Journal::follow_every`] does, with what was passed on and
    /// not kept: what was passed on before the follower began is handed
    /// over first, and nothing twice.
    pub fn follow_every(self: &std::sync::Arc<Self>, journal: &crate::Journal, topics: &str) -> Following {
        self.follow_from(journal, crate::Places::default(), topics)
    }

    /// The same, from `after`, a place among every journal.
    pub fn follow_from(
        self: &std::sync::Arc<Self>,
        journal: &crate::Journal,
        after: crate::Places,
        topics: &str,
    ) -> Following {
        Following {
            follow: journal.follow_every(after.clone(), topics.to_string()),
            journal: journal.clone(),
            after,
            order: self.clone(),
            topics: topics.to_string(),
            looked: 0,
            handed: std::collections::HashSet::new(),
        }
    }

    /// What [`crate::Journal::read_after`] reads of every journal from
    /// `after`, with what was passed on and not kept after it, and the
    /// place that comes to.
    pub async fn read_after(
        &self,
        journal: &crate::Journal,
        after: &crate::Places,
        topics: &str,
        limit: usize,
    ) -> kernel::Result<(Vec<kernel::Event>, crate::Places)> {
        let (mut all, mut places) = journal.read_after(after, topics, limit).await?;
        let query = crate::EventQuery { topics: topics.to_string(), ..Default::default() };
        for (name, event) in self.passing_in() {
            if asks(&query, &event) && event.seq > place_in(journal, after, &name) {
                if event.seq > places.of(&name) {
                    places.put(&name, event.seq);
                }
                all.push((*event).clone());
            }
        }
        all.sort_by_key(|event| (event.ts, self.of(event.seq).unwrap_or(0), event.seq));
        Ok((all, places))
    }
}

/// Where `after` is in the journal called `name`, of `journal`'s factory.
fn place_in(journal: &crate::Journal, after: &crate::Places, name: &str) -> u64 {
    match journal.every().into_iter().find(|one| one.name() == name) {
        Some(one) => journal.place_in(after, &one),
        None => after.of(name),
    }
}

/// What every journal says, followed, with what was passed on and not
/// kept ([`Order::follow_every`]).
pub struct Following {
    follow: crate::FollowEvery,
    journal: crate::Journal,
    /// Where it follows from.
    after: crate::Places,
    order: std::sync::Arc<Order>,
    topics: String,
    /// How much of what was passed on has been looked at.
    looked: usize,
    /// The numbers of what was handed over that was passed on.
    handed: std::collections::HashSet<u64>,
}

impl Following {
    /// The next event, kept or passed on. Waits if there is none yet.
    pub async fn next(&mut self) -> kernel::Result<std::sync::Arc<kernel::Event>> {
        let query = crate::EventQuery { topics: self.topics.clone(), ..Default::default() };
        loop {
            let passing = self.order.passing_in();
            while let Some((name, event)) = passing.get(self.looked) {
                self.looked += 1;
                let after = place_in(&self.journal, &self.after, name);
                if asks(&query, event) && event.seq > after && self.handed.insert(event.seq) {
                    return Ok(event.clone());
                }
            }
            let event = self.follow.next().await?;
            if !event.is_durable() || !self.order.was_passed(event.seq) || self.handed.insert(event.seq) {
                return Ok(event);
            }
        }
    }
}

impl Following {
    /// Where it has got to in each journal, by what was kept and came to
    /// it in its order: what was passed on and handed over from the order
    /// (said before it began, or in a journal it had not come to yet)
    /// moves nothing, since what was kept before it may still be to come.
    pub fn places(&self) -> crate::Places {
        self.follow.places().clone()
    }
}
