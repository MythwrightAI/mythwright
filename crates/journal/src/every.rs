//! What every journal of a factory says, read and followed as one.
//!
//! A factory keeps a journal for each project that has one of its own,
//! beside the factory's ([`crate::set`]). What reads the events for
//! people, or follows them for the whole factory (the stream a client
//! follows, search, a record's history, what an agent is asked in a
//! channel), is handed what every one of them says.
//!
//! **What is promised of the order is narrow.** Of two events of one
//! journal the later has the greater number and is handed over later.
//! Of two events of different journals nothing is promised: their
//! numbers say nothing of which was said first, and a follower is handed
//! each journal's as it comes. Where events of several journals are
//! read to be shown ([`Journal::read_every`]) they are put in the order
//! of when each was said, to the millisecond, and two said in the same
//! millisecond in the order of their numbers, which for two journals is
//! no order at all: it is only the same each time.
//!
//! **A place among them is a place in each** ([`Places`]): the number of
//! the last event dealt with, journal by journal, and one number for
//! every journal it names no place in. That number is nought for a
//! place taken now ([`Journal::now`]): a journal made afterwards is read
//! from its beginning, since everything in it came after. And a number
//! alone is a place among them all ([`Places::above`]): after every
//! event numbered that or below, whichever journal keeps it. That is
//! what a place kept or handed out while there was one journal means,
//! and what an event's own number means as a place: numbers are given
//! by one giver that only goes up, so whatever a journal made since
//! then holds is numbered above it.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use kernel::{Event, Result};
use serde_json::Value;

use crate::{EventQuery, Follow, Journal};

/// A place among the events of every journal of a factory: for each
/// journal it names, by what the journal is called, the number of the
/// last of its events that was dealt with; and for every other journal
/// one number, which is nought unless it was made from a number alone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Places {
    /// The place in a journal that is not named.
    floor: u64,
    each: BTreeMap<String, u64>,
}

impl Places {
    /// The place after every event numbered `seq` or below, in whichever
    /// journal: what a single number means.
    pub fn above(seq: u64) -> Places {
        Places { floor: seq, each: BTreeMap::new() }
    }

    /// The place in the journal called `journal`.
    pub fn of(&self, journal: &str) -> u64 {
        self.each.get(journal).copied().unwrap_or(self.floor)
    }

    /// The place in the journal called `journal`, where one is said of
    /// it by name.
    pub fn named(&self, journal: &str) -> Option<u64> {
        self.each.get(journal).copied()
    }

    /// Says the place in the journal called `journal` by name, whatever
    /// the number for the rest is: for a place that was worked out from
    /// another journal's ([`Journal::place_in`]), which is worked out
    /// once and kept, since the place it was worked out from moves on.
    pub fn name(&mut self, journal: &str, seq: u64) {
        self.each.insert(journal.to_string(), seq);
    }

    /// Says the place in the journal called `journal`.
    pub fn put(&mut self, journal: &str, seq: u64) {
        if seq == self.floor {
            self.each.remove(journal);
        } else {
            self.each.insert(journal.to_string(), seq);
        }
    }

    /// As it is kept in the small durable state: a number for each
    /// journal named, by the journal's name, and the number for the
    /// rest under `*` where it is not nought.
    pub fn kept(&self) -> Value {
        let mut kept = serde_json::json!(self.each);
        if self.floor > 0 {
            kept[REST] = serde_json::json!(self.floor);
        }
        kept
    }

    /// Reads what [`Places::kept`] wrote; and a number alone, which a
    /// program from before kept, as the place above it. Nothing, for
    /// what is neither.
    pub fn from_kept(kept: &Value) -> Option<Places> {
        match kept {
            Value::Number(seq) => seq.as_u64().map(Places::above),
            Value::Object(each) => {
                let mut places = Places::above(each.get(REST).map_or(Some(0), Value::as_u64)?);
                for (journal, seq) in each.iter().filter(|(journal, _)| journal.as_str() != REST) {
                    places.name(journal, seq.as_u64()?);
                }
                Some(places)
            }
            _ => None,
        }
    }

    /// As text, for whoever is handed a place to hand back unread: the
    /// number for every journal it does not name, and then each journal
    /// it names and its number. A number alone is written as that. It
    /// is written with nothing an address would have to write otherwise.
    pub fn text(&self) -> String {
        let mut text = self.floor.to_string();
        for (journal, seq) in &self.each {
            text.push_str(&format!("{PART}{}{PART}{seq}", written(journal)));
        }
        text
    }

    /// Reads what [`Places::text`] wrote, and a number alone. Nothing,
    /// for a text that is neither.
    pub fn parse(text: &str) -> Option<Places> {
        let mut parts = text.trim().split(PART);
        let mut places = Places::above(parts.next()?.parse().ok()?);
        while let Some(journal) = parts.next() {
            places.name(&read(journal)?, parts.next()?.parse().ok()?);
        }
        Some(places)
    }
}

/// The place in a journal whose project was moved apart, for whoever
/// has no place of its own there: `rest` is its place in a journal it
/// says nothing of, `there` its place in the journal the project was
/// moved from, and `at` where the giver of numbers stood at the move.
/// Everything the project said before the move is numbered at or below
/// `at`, and was read in the journal it was moved from as far as
/// `there`; so in the project's own it has been read as far as the
/// lesser of the two, wherever those events are kept now. And a place
/// that is a number alone (`rest`, above `at`) is after everything
/// numbered so in whichever journal, the project's own among them.
pub fn joined(rest: u64, there: u64, at: u64) -> u64 {
    rest.max(there.min(at))
}

/// What parts a place written as text.
const PART: char = '~';

/// Under what the number for every journal a place does not name is
/// kept: no journal is called so.
const REST: &str = "*";

/// A journal's name as it is written in a place handed out as text:
/// letters, digits and the few marks a name is made with as they are,
/// and anything else written out.
fn written(journal: &str) -> String {
    let mut text = String::new();
    for byte in journal.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b':' | b'-' | b'_' | b'.' => text.push(byte as char),
            other => text.push_str(&format!("%{other:02x}")),
        }
    }
    text
}

fn read(written: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let mut rest = written.bytes();
    while let Some(byte) = rest.next() {
        match byte {
            b'%' => {
                let pair = [rest.next()?, rest.next()?];
                bytes.push(u8::from_str_radix(std::str::from_utf8(&pair).ok()?, 16).ok()?);
            }
            other => bytes.push(other),
        }
    }
    String::from_utf8(bytes).ok()
}

/// The reading of a journal's next event: it has the follower until it
/// has one, and gives it back with what it read.
type Reading = Pin<Box<dyn Future<Output = (Box<Follow>, Result<Option<Arc<Event>>>)> + Send>>;

/// What says that a journal was made, once one is.
type Told = tokio::sync::watch::Receiver<u64>;

/// Waiting to be told that a journal was made: gives back what says so,
/// or nothing once nothing can.
type Waiting = Pin<Box<dyn Future<Output = Option<Told>> + Send>>;

/// One journal's part of [`FollowEvery`]: its follower, or the reading
/// of its next event, which has the follower until it has one.
enum Part {
    Ready(Box<Follow>),
    Reading(Reading),
    /// Taken while it is looked at.
    Taken,
}

/// A position among the events of every journal that can be read
/// forward for ever: each journal's in its order, and the journals' as
/// they come.
pub struct FollowEvery {
    journal: Journal,
    topics: String,
    /// Where it is in each journal: what was handed over, or passed by.
    places: Places,
    parts: Vec<(String, Part)>,
    /// Which part is looked at first next time, so that one journal
    /// with a great deal to say does not keep another from being heard.
    turn: usize,
    /// Which part the event last handed over came from.
    last: Option<usize>,
    /// Completes when a journal is made, and gives back what says so:
    /// kept across the times this is asked, since what waits to be told
    /// is told only while it is waiting. None once nothing can say so.
    made: Option<Waiting>,
}

/// Waits to be told that a journal was made.
fn told(mut made: Told) -> Waiting {
    Box::pin(async move { made.changed().await.ok().map(|()| made) })
}

impl Journal {
    /// The place that is now, in every journal: everything said so far
    /// is before it.
    pub fn now(&self) -> Places {
        let mut places = Places::default();
        for journal in self.every() {
            places.put(journal.name(), journal.last_seq());
        }
        places
    }

    /// The place `places` comes to in `journal`, one of this factory's:
    /// the one it says of it by name; else, for the journal of a project
    /// that was moved apart, what its place in the journal the project
    /// was moved from comes to there ([`joined`]); else its number for
    /// the rest. This is the one place a place among all the journals
    /// becomes a place in one, so that whatever kept a place before a
    /// project was moved (a follower, a page, a count) goes on in the
    /// project's journal having missed nothing and hearing nothing
    /// twice. Asked of the factory's own journal.
    pub fn place_in(&self, places: &Places, journal: &Journal) -> u64 {
        if let Some(named) = places.named(journal.name()) {
            return named;
        }
        match self.moved_of(journal.name()) {
            Some((from, at)) => joined(places.floor, places.of(&from), at),
            None => places.floor,
        }
    }

    /// The same for whoever keeps its places a journal at a time and has
    /// none in `journal`: `kept_in` is its place in the journal called
    /// so, where it has one. From the beginning, for a journal whose
    /// project was not moved, or where nothing is kept in the journal it
    /// was moved from.
    pub fn joins(&self, journal: &Journal, kept_in: impl Fn(&str) -> Option<u64>) -> u64 {
        self.moved_of(journal.name())
            .and_then(|(from, at)| kept_in(&from).map(|there| joined(0, there, at)))
            .unwrap_or(0)
    }

    /// Follows the events of every journal from just after `after`:
    /// each journal's as [`Journal::follow`] gives them, with no gap and
    /// no duplicate, and a journal made meanwhile from its beginning.
    /// Asked of the factory's own journal.
    pub fn follow_every(&self, after: Places, topics: impl Into<String>) -> FollowEvery {
        let mut follow = FollowEvery {
            journal: self.clone(),
            topics: topics.into(),
            places: after,
            parts: Vec::new(),
            turn: 0,
            last: None,
            made: Some(told(self.made())),
        };
        follow.join();
        follow
    }

    /// Reads stored events of every journal, as [`Journal::read`] reads
    /// one's: those `query` asks for of each, put together in the order
    /// of when each was said (newest first where that is asked), and no
    /// more of them than `query` allows. `after` and `before` are numbers
    /// and mean in each journal what they mean in one: an event's number
    /// is its own in the factory, and says where it is in its journal
    /// and nothing of the others.
    pub async fn read_every(&self, query: EventQuery) -> Result<Vec<Event>> {
        let every = self.every();
        if every.len() == 1 {
            return self.read(query).await;
        }
        let limit = if query.limit == 0 { crate::events::PAGE } else { query.limit };
        let mut all = Vec::new();
        for journal in every {
            all.extend(journal.read(query.clone()).await?);
        }
        match query.newest_first {
            true => all.sort_by_key(|event| std::cmp::Reverse((event.ts, event.seq))),
            false => all.sort_by_key(|event| (event.ts, event.seq)),
        }
        all.truncate(limit);
        Ok(all)
    }

    /// Reads stored events of every journal from just after `after`, a
    /// place in each: at most `limit` of each journal's that `topics`
    /// asks for, in the order of when each was said, with the place just
    /// after all that was read.
    pub async fn read_after(
        &self,
        after: &Places,
        topics: &str,
        limit: usize,
    ) -> Result<(Vec<Event>, Places)> {
        let (mut all, mut places) = (Vec::new(), after.clone());
        for journal in self.every() {
            let from = self.place_in(after, &journal);
            let query = EventQuery { after: from, topics: topics.to_string(), limit, ..Default::default() };
            let read = journal.read(query).await?;
            match read.last() {
                Some(last) => places.put(journal.name(), last.seq),
                // Worked out from another journal's place: said by name,
                // so that it is not worked out again from where that one
                // has got to by then.
                None if from != after.of(journal.name()) => places.name(journal.name(), from),
                None => {}
            }
            all.extend(read);
        }
        all.sort_by_key(|event| (event.ts, event.seq));
        Ok((all, places))
    }

    /// Reads stored events of every journal between two places, after
    /// `after` and up to `upto` in each: those `query` asks for besides
    /// (its own `after` and `before` are not read), in the order of when
    /// each was said, newest first where that is asked, and no more of
    /// them than it allows. For whoever takes a place first and reads
    /// up to it, so that what is said meanwhile is neither read twice
    /// nor missed by what goes on from the place.
    pub async fn read_within(&self, after: &Places, upto: &Places, query: EventQuery) -> Result<Vec<Event>> {
        let limit = if query.limit == 0 { crate::events::PAGE } else { query.limit };
        let mut all = Vec::new();
        for journal in self.every() {
            let (after, upto) = (self.place_in(after, &journal), self.place_in(upto, &journal));
            let within = EventQuery { after, before: upto.saturating_add(1), ..query.clone() };
            all.extend(journal.read(within).await?);
        }
        match query.newest_first {
            true => all.sort_by_key(|event| std::cmp::Reverse((event.ts, event.seq))),
            false => all.sort_by_key(|event| (event.ts, event.seq)),
        }
        all.truncate(limit);
        Ok(all)
    }

    /// The journal that keeps the event numbered `seq`, where one does:
    /// its own number says nothing of which.
    pub async fn keeping(&self, seq: u64) -> Result<Option<Journal>> {
        for journal in self.every() {
            let query =
                EventQuery { after: seq.saturating_sub(1), before: seq + 1, limit: 1, ..Default::default() };
            if seq > 0 && !journal.read(query).await?.is_empty() {
                return Ok(Some(journal));
            }
        }
        Ok(None)
    }

    /// The highest number any event has that is kept, in whichever
    /// journal. It is no place among the events: [`Journal::now`] is.
    pub fn highest(&self) -> u64 {
        self.every().iter().map(Journal::last_seq).max().unwrap_or(0)
    }

    /// Whether any journal keeps an event on `topic` whose data says
    /// `key` is `value` ([`Journal::said`]).
    pub async fn said_in_any(&self, topic: &str, key: &str, value: &str) -> Result<bool> {
        for journal in self.every() {
            if journal.said(topic, key, value).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Takes out of every journal the events on `topic` whose data says
    /// `key` is `value` ([`Journal::forget_events`]), and says how many
    /// went.
    pub async fn forget_in_every(&self, topic: &str, key: &str, value: &str) -> Result<usize> {
        let mut gone = 0;
        for journal in self.every() {
            gone += journal.forget_events(topic, key, value).await?;
        }
        Ok(gone)
    }
}

impl FollowEvery {
    /// Takes up each journal there is that it does not follow yet, from
    /// its place: its beginning, for one the place says nothing of.
    fn join(&mut self) {
        for journal in self.journal.every() {
            if self.parts.iter().all(|(name, _)| name != journal.name()) {
                let from = self.journal.place_in(&self.places, &journal);
                if from != self.places.of(journal.name()) {
                    self.places.name(journal.name(), from);
                }
                let follow = journal.follow(from, self.topics.clone());
                self.parts.push((journal.name().to_string(), Part::Ready(Box::new(follow))));
            }
        }
    }

    /// The next matching event of any journal. Waits if there is none
    /// yet. Safe to drop while it waits: nothing is taken until it is
    /// returned, and what a journal was in the middle of reading is read
    /// on when this is asked again.
    pub async fn next(&mut self) -> Result<Arc<Event>> {
        loop {
            if let Some(event) = self.next_or_behind().await? {
                return Ok(event);
            }
        }
    }

    /// The next matching event of any journal, or nothing where following
    /// one of them fell too far behind what is said as it is said
    /// ([`crate::Follow::next_or_behind`]): what was passed on meanwhile
    /// and not kept is gone, and is read another way by whoever needs it.
    pub async fn next_or_behind(&mut self) -> Result<Option<Arc<Event>>> {
        std::future::poll_fn(|cx| {
            // A journal made meanwhile is followed from its beginning.
            while let Some(waiting) = self.made.as_mut() {
                match waiting.as_mut().poll(cx) {
                    Poll::Pending => break,
                    Poll::Ready(again) => {
                        self.made = again.map(told);
                        self.join();
                    }
                }
            }
            let many = self.parts.len();
            for step in 0..many {
                let at = (self.turn + step) % many;
                let (name, part) = &mut self.parts[at];
                let mut reading = match std::mem::replace(part, Part::Taken) {
                    Part::Ready(mut follow) => Box::pin(async move {
                        let next = follow.next_or_behind().await;
                        (follow, next)
                    }),
                    Part::Reading(reading) => reading,
                    Part::Taken => continue,
                };
                match reading.as_mut().poll(cx) {
                    Poll::Pending => *part = Part::Reading(reading),
                    Poll::Ready((follow, next)) => {
                        // Where it has got to in that journal: what was
                        // handed over just now, and what it passed by.
                        self.places.put(name, follow.cursor());
                        *part = Part::Ready(follow);
                        self.turn = (at + 1) % many;
                        if matches!(next, Ok(Some(_))) {
                            self.last = Some(at);
                        }
                        return Poll::Ready(next);
                    }
                }
            }
            Poll::Pending
        })
        .await
    }

    /// The place to go on from after a disconnect: in each journal,
    /// everything up to it has been handed over by [`FollowEvery::next`]
    /// or was not wanted.
    pub fn places(&self) -> &Places {
        &self.places
    }

    /// The place just after `event`, which was the last handed over: in
    /// its own journal just after it, and in every other where this has
    /// got to. What it passed by since in its own journal is read again
    /// from there and passed by again, so that the place of an event is
    /// the same whoever was handed it, where there is one journal.
    pub fn after(&self, event: &Event) -> Places {
        let mut places = self.places.clone();
        if let Some(journal) = self.heard_in().filter(|_| event.is_durable()) {
            places.put(journal, event.seq);
        }
        places
    }

    /// What the journal the event last handed over came from is called.
    pub fn heard_in(&self) -> Option<&str> {
        self.last.and_then(|at| self.parts.get(at)).map(|(name, _)| name.as_str())
    }
}
