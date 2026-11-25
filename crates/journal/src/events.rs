//! Appending, reading and following events.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use kernel::event::topic_matches_any;
use kernel::{Event, Result, Source};
use rusqlite::types::Value as Sql;
use serde_json::Value;
use tokio::sync::broadcast::{self, error::RecvError};

use crate::Journal;

/// Which stored events to read.
#[derive(Clone, Debug, Default)]
pub struct EventQuery {
    /// Only events after this sequence number.
    pub after: u64,
    /// Only events before this sequence number, when non-zero.
    pub before: u64,
    /// Comma-separated topic patterns; empty means every topic.
    pub topics: String,
    pub instance: Option<String>,
    pub run: Option<String>,
    /// Stop after this many (default 500).
    pub limit: usize,
    /// Newest first, for "what just happened" views.
    pub newest_first: bool,
}

pub(crate) const PAGE: usize = 500;

/// The indexes of the events by instance and by run, which a journal made
/// now does not keep. Each was a page of the log for nearly every commit
/// that says anything (an event's place in each is beside the last
/// event's of its instance and of its run, and every commit wrote that
/// page again): two in nine of what a card had written to the disk.
/// What reads by them is what a run, or something done around a step,
/// looks for of its own when it is taken up after a restart, and each
/// such look says where to begin from (a place the run, the step or the
/// stop kept, [`EventQuery::after`]): it reads along the index of topics,
/// or in order, from there. A journal from before that has held more
/// than [`FEW`] keeps them, since taking an index away reads all of it,
/// and its reads go by them as they always did.
pub(crate) const UNKEPT: [&str; 2] = ["events_instance", "events_run"];

/// How many events a journal may hold for [`UNKEPT`] to be taken away as
/// it is opened: a few hundred pages of each, which is a moment from any
/// disk. One made now holds none.
const FEW: i64 = 20_000;

/// Takes [`UNKEPT`] away where the journal holds few enough events for
/// that to be nothing ([`FEW`]). Asked each time a journal is opened; it
/// reads the names of what there is and, where one of them is there,
/// counts the events along the index of topics as far as [`FEW`].
pub(crate) fn shed(connection: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut there = Vec::new();
    for name in UNKEPT {
        let kept: bool = connection.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1)",
            [name],
            |row| row.get(0),
        )?;
        if kept {
            there.push(name);
        }
    }
    if there.is_empty() {
        return Ok(());
    }
    let held: i64 = connection.query_row(
        "SELECT count(*) FROM (SELECT 1 FROM events INDEXED BY events_topic LIMIT ?1)",
        [FEW + 1],
        |row| row.get(0),
    )?;
    if held <= FEW {
        for name in there {
            connection.execute(&format!("DROP INDEX {name}"), [])?;
        }
    }
    Ok(())
}

/// How many events' places in a chain are remembered. One further back is
/// taken to be in none, which starts a chain afresh: a handler that far
/// behind is not going round with anything.
const CHAINS_KEPT: usize = 65_536;

/// What an event is held to as it is emitted: given its topic and what it
/// carries, says what is wrong with it, if anything is.
pub type Holding = Arc<dyn Fn(&str, &Value) -> Option<String> + Send + Sync>;

/// The check every event is put to, where there is one, and what it found.
///
/// A program that is run to be tested is held to what it says of its own
/// events; one that is run to be used is not, and pays for one look at an
/// empty cell. What is found is kept and not acted on: the event happened
/// whatever is thought of it, and whoever set the check asks what it found.
#[derive(Default)]
pub(crate) struct Held {
    to: std::sync::OnceLock<Holding>,
    faults: std::sync::Mutex<Vec<String>>,
    /// Where the order events are written in is written down, where it
    /// is ([`Journal::orders_by`]).
    order: std::sync::OnceLock<Arc<crate::Order>>,
}

impl Held {
    pub(crate) fn look(&self, topic: &str, data: &Value) {
        let Some(check) = self.to.get() else { return };
        if let Some(fault) = check(topic, data) {
            let mut faults = self.faults.lock().expect("journal lock");
            if !faults.contains(&fault) {
                faults.push(fault);
            }
        }
    }
}

/// What is said of something that is the factory's for good, by how its
/// topic begins: a channel, who is in it and what is said there are kept
/// in the factory's own file whatever project a channel says, so what is
/// said of them is kept in the factory's own journal ([`Journal::emit_here`],
/// `Writing::say_here`), and stays there when a project is moved to a
/// journal of its own ([`crate::moved::said`]). So is where a project is
/// listed (`project.placed`): which workspace it is in is a row of the
/// factory's own tables and nothing of the project's, so that it was
/// placed is kept with that row, in the one write, whichever journal
/// keeps everything else said of the project; and so, the same way, is
/// where a project keeps its own blueprints (`project.blueprints`), a
/// setting kept in the factory's own small durable state, and how its
/// things are arranged in the side panel (`project.arranged`), kept there
/// beside it; and what a project is called (`project.renamed`), said with
/// its row. And so is what is said of
/// somebody's inbox (`inbox.`): a line of one is a row of the factory's
/// own, written with what it is of (something said in a channel), and
/// names no project. And what is said of somebody's themes (`theme.`):
/// a theme is a row of the factory's own, kept with an account. And what
/// is said of how somebody arranged the side panel (`arranged.`), and of
/// how a time is written for them (`clock.`), and of how what they read
/// looks for them (`appearance.`), kept the same way. One
/// list, so that the two
/// cannot part: an event said here whose topic is not on it is refused
/// where the program is built to be tested.
pub const HERE: &[&str] = &[
    "channel.",
    "project.placed",
    "project.blueprints",
    "project.arranged",
    "project.renamed",
    "project.setup.",
    "inbox.",
    "theme.",
    "arranged.",
    "clock.",
    "appearance.",
    "topic.",
];

/// Whether an event on `topic` is the factory's for good ([`HERE`]).
pub fn stays_here(topic: &str) -> bool {
    HERE.iter().any(|begins| topic.starts_with(begins))
}

impl Journal {
    /// Holds every event emitted from now on to `check`. Set once; a
    /// second is not taken.
    pub fn hold(&self, check: Holding) {
        let _ = self.inner.held.to.set(check);
    }

    /// Has every journal of this factory write down, in `order`, the
    /// order in which events are written from now on, whichever of them
    /// keeps each ([`crate::Order`]): for a program that is run to be
    /// tested. Set once; a second is not taken.
    pub fn orders_by(&self, order: Arc<crate::Order>) {
        let _ = self.inner.held.order.set(order);
    }

    /// What the check set by [`Journal::hold`] has found wrong so far.
    pub fn faults(&self) -> Vec<String> {
        self.inner.held.faults.lock().expect("journal lock").clone()
    }

    /// Records an event and tells everyone following.
    ///
    /// For an event that says something nobody wrote down beside it.
    /// Where something is written that the event then says, the two are
    /// one write ([`Journal::write`]): said after it with this, a daemon
    /// stopped between the two has written what nobody was told.
    ///
    /// The event is durable once this returns. It reaches live subscribers in
    /// sequence order, because the broadcast happens on the database thread
    /// right after the insert, never from the callers racing each other.
    pub async fn emit(&self, topic: &str, source: Source, data: Value) -> Result<Arc<Event>> {
        // What is said of a project whose journal keeps what is run for
        // it is kept there, with what is run: so that everything said of
        // the project is in one journal, in the order it was said.
        if let Some(own) = self.said_in(topic, &data) {
            return own.emit_as(topic, source, data, false).await;
        }
        self.emit_as(topic, source, data, false).await
    }

    /// Records an event in this journal whatever project it names: for
    /// what is said of something this journal keeps for good (a channel
    /// and what is said in it are the factory's, whichever project the
    /// channel says). It is heard by project as any event is.
    pub async fn emit_here(&self, topic: &str, source: Source, data: Value) -> Result<Arc<Event>> {
        debug_assert!(
            stays_here(topic),
            "`{topic}` is said as the factory's for good and is not in `journal::HERE`"
        );
        self.emit_as(topic, source, data, false).await
    }

    /// The journal that keeps an event on `topic` carrying `data`, where
    /// that is another than this one: the journal of the project the
    /// event names, where this is a factory's own and that project's
    /// journal keeps what is run for it ([`Journal::rows_of`]). An event
    /// that names no project, or one kept in the factory's own, is this
    /// journal's; so is whatever is said in a project's journal; and so
    /// is what is the factory's for good ([`HERE`]), whatever it names
    /// and whichever way it is said: kept, with a row or by itself, or
    /// only passed on. The list is asked here so that it is asked for
    /// every one of them, and not only by whoever remembered to say so.
    pub(crate) fn said_in(&self, topic: &str, data: &Value) -> Option<Journal> {
        if stays_here(topic) {
            return None;
        }
        self.kept_for(data.get("project").and_then(Value::as_str))
    }

    /// The journal that keeps what is said of `project`, where that is
    /// another than this one.
    fn kept_for(&self, project: Option<&str>) -> Option<Journal> {
        self.inner.set.get()?;
        self.rows_of(project.filter(|project| !project.is_empty())?)
    }

    /// Records an event; `last`, though the journal has shut.
    pub(crate) async fn emit_as(
        &self,
        topic: &str,
        source: Source,
        data: Value,
        last: bool,
    ) -> Result<Arc<Event>> {
        self.inner.held.look(topic, &data);
        let inner = self.inner.clone();
        if inner.shut.is() && !last {
            // Said after the place the journal shut at: it is not part of
            // what happened, and whoever says it goes no further.
            return std::future::pending().await;
        }
        // Said while an event was being handled, it is one deeper in that
        // event's chain. Read here, where whoever says it is.
        let chain = kernel::chain::current();
        let mut event = Event {
            seq: 0,
            ts: self.now_ms(),
            topic: topic.to_string(),
            source: Source { device: source.device.or_else(|| Some(inner.device.clone())), ..source },
            data,
        };
        let keep = move |connection: &mut rusqlite::Connection| {
            // One that was on its way here when the journal shut.
            if inner.shut.is() && !last {
                return Ok(None);
            }
            // The next of the numbers it has drawn, given here: on the
            // database's thread, where events are written one at a time
            // and nowhere else.
            let (before, mut drew) = (inner.run(), Vec::new());
            let mut run = before;
            event.seq = inner.number(connection, &mut run, &mut drew, last)?;
            if let Err(err) = insert(connection, &event) {
                inner.undraw(&drew);
                return Err(err);
            }
            inner.put_back(before, run, drew);
            Ok(Some(inner.written(event, chain, true)))
        };
        let kept = match last {
            true => self.inner.db.call_though_held(keep).await?,
            false => self.inner.db.call(keep).await?,
        };
        match kept {
            Some(event) => Ok(event),
            None => std::future::pending().await,
        }
    }

    /// Where the event at `seq` is in a chain of events each said while
    /// the one before was being handled ([`kernel::chain`]). None for one
    /// that is in no chain: it came from outside, from a person, from a
    /// timer. Kept in memory for the events of late and no others, so a
    /// chain does not outlast the program.
    pub fn chain_of(&self, seq: u64) -> Option<kernel::Chain> {
        let chains = self.inner.chains.lock().expect("journal lock");
        let at = chains.binary_search_by_key(&seq, |(seq, _)| *seq).ok()?;
        chains.get(at).map(|(_, chain)| *chain)
    }

    /// The journal that keeps what is said of `project`, as [`Journal::emit`]
    /// would keep an event that names it: asked by whoever says an event
    /// of something that is a project's and carries no `project` (what a
    /// service says of itself, which the instance that owns it says).
    /// Said there, it is with everything else said of the project, in
    /// the one order that journal keeps; said in the factory's own it is
    /// still the project's to whatever hears it, and comes in no order
    /// with the rest. It is the one question `emit` asks, so that where
    /// an event is kept is decided in one place; whoever says an event
    /// there says none that is the factory's for good ([`HERE`]).
    pub fn said_of(&self, project: Option<&str>) -> Journal {
        self.kept_for(project).unwrap_or_else(|| self.clone())
    }

    /// Broadcasts an event without keeping it. For high-volume, low-value
    /// signals (log lines, progress) that only matter to someone watching.
    pub fn emit_ephemeral(&self, topic: &str, source: Source, data: Value) -> Arc<Event> {
        // Passed on where what is kept of the same thing is kept, as an
        // event that is kept is ([`Journal::emit`]): whoever follows
        // every journal is handed each one's in its order and no order
        // between two, and what an agent is writing would otherwise
        // come after the turn it was written in was said to be over.
        let kept_in = self.said_in(topic, &data);
        let inner = &kept_in.as_ref().unwrap_or(self).inner;
        inner.held.look(topic, &data);
        let event = Arc::new(Event {
            seq: 0,
            ts: self.now_ms(),
            topic: topic.to_string(),
            source: Source { device: source.device.or_else(|| Some(inner.device.clone())), ..source },
            data,
        });
        // After the place the journal shut at, nobody is told anything.
        // And what is only passed on may be the place itself: a step of an
        // instance that keeps nothing begins without anything being kept.
        if !inner.shut.is() {
            let _ = inner.live.send(event.clone());
            inner.shut.look(&event);
        }
        event
    }

    /// The live stream, from now. Prefer [`Journal::follow`], which also
    /// handles catching up and falling behind.
    pub fn live(&self) -> broadcast::Receiver<Arc<Event>> {
        self.inner.live.subscribe()
    }

    /// Sequence number of the newest stored event.
    pub fn last_seq(&self) -> u64 {
        self.inner.last_seq.load(Ordering::SeqCst)
    }

    /// Whether this journal may have passed on, and not kept, anything
    /// after the place `after`: whoever goes on from that place reads
    /// back only what was kept, and is to read what it came to another
    /// way ([`crate::Writing::pass_on`]). A place from before the journal
    /// was opened is taken to have missed something, since what a program
    /// before this one passed on is gone with it.
    pub fn passed_after(&self, after: u64) -> bool {
        self.inner.passed.load(Ordering::SeqCst) > after
    }

    /// Reads stored events.
    pub async fn read(&self, query: EventQuery) -> Result<Vec<Event>> {
        let limit = if query.limit == 0 { PAGE } else { query.limit };
        self.inner
            .db
            .call(move |connection| {
                let mut sql = String::from(
                    "SELECT seq, ts, topic, device, instance, run, step, actor, data, takes FROM events WHERE seq > ?1",
                );
                // A place past any the database can number is the last
                // there could be, and not one below nought.
                let place = |seq: u64| Sql::Integer(i64::try_from(seq).unwrap_or(i64::MAX));
                let mut params: Vec<Sql> = vec![place(query.after)];
                if query.before > 0 {
                    params.push(place(query.before));
                    sql.push_str(&format!(" AND seq < ?{}", params.len()));
                }
                if let Some(instance) = &query.instance {
                    params.push(Sql::Text(instance.clone()));
                    sql.push_str(&format!(" AND instance = ?{}", params.len()));
                }
                if let Some(run) = &query.run {
                    params.push(Sql::Text(run.clone()));
                    sql.push_str(&format!(" AND run = ?{}", params.len()));
                }
                // One whole topic is found through the index on topics, so
                // reading what happened on it since some point costs what
                // happened on it, not everything that happened.
                if is_one_topic(&query.topics) {
                    params.push(Sql::Text(query.topics.trim().to_string()));
                    sql.push_str(&format!(" AND topic = ?{}", params.len()));
                }
                sql.push_str(if query.newest_first { " ORDER BY seq DESC" } else { " ORDER BY seq ASC" });
                let mut statement = connection.prepare(&sql)?;
                let mut rows = statement.query(rusqlite::params_from_iter(params))?;
                let mut events = Vec::new();
                // Topic patterns are matched here rather than in SQL: they are
                // globs over dotted words, and the scan is bounded by `limit`.
                while let Some(row) = rows.next()? {
                    let topic: String = row.get(2)?;
                    if !query.topics.is_empty() && !topic_matches_any(&query.topics, &topic) {
                        continue;
                    }
                    let data: String = row.get(8)?;
                    events.push(Event {
                        seq: row.get::<_, i64>(0)? as u64,
                        ts: row.get(1)?,
                        topic,
                        source: Source {
                            device: row.get(3)?,
                            instance: row.get(4)?,
                            run: row.get(5)?,
                            step: row.get(6)?,
                            actor: row.get(7)?,
                            takes: row.get(9)?,
                        },
                        data: serde_json::from_str(&data).unwrap_or(Value::Null),
                    });
                    if events.len() >= limit {
                        break;
                    }
                }
                Ok(events)
            })
            .await
    }

    /// Follows events from just after `after`: first everything stored since
    /// then, then live ones as they happen, with no gap and no duplicate
    /// between the two.
    pub fn follow(&self, after: u64, topics: impl Into<String>) -> Follow {
        // Subscribe before looking at the log, so nothing emitted in between
        // is missed; the cursor drops what arrives twice. What comes live
        // is numbered from about where the journal was as it subscribed.
        let since = self.last_seq();
        let live = self.live();
        Follow {
            live,
            journal: self.clone(),
            topics: topics.into(),
            cursor: after,
            from: after,
            since,
            read: HashSet::new(),
            backlog: VecDeque::new(),
            // A follower that starts at the live edge has nothing to replay,
            // and going straight to the stream keeps ephemeral and stored
            // events in the order they happened.
            caught_up: after >= self.last_seq(),
        }
    }

    /// Whether the journal keeps an event on `topic` whose data says `key`
    /// is `value`, a text: for whoever wrote something down and may have
    /// stopped before saying so, to know which side of the saying it
    /// stopped on.
    pub async fn said(&self, topic: &str, key: &str, value: &str) -> Result<bool> {
        // As it is written in the data, as `forget_events` finds it.
        let holds = format!("{}:{}", serde_json::json!(key), serde_json::json!(value));
        let topic = topic.to_string();
        self.inner
            .db
            .call(move |connection| {
                connection.query_row(
                    "SELECT EXISTS (SELECT 1 FROM events WHERE topic = ?1 AND instr(data, ?2) > 0)",
                    [&topic, &holds],
                    |row| row.get(0),
                )
            })
            .await
    }

    /// A place in this journal before everything it kept at `ts` or
    /// later: for a look at what was said since a moment, which then
    /// reads on from there. Found, as where old events end is, from the
    /// few whose time is in an index (one in 512, `events_when`), so it is
    /// up to some hundreds of events early and never late; nought where
    /// none of those is older.
    pub async fn place_before(&self, ts: i64) -> Result<u64> {
        self.inner
            .db
            .call(move |connection| {
                let sampled: i64 = connection.query_row(
                    "SELECT coalesce(max(seq), 0) FROM events INDEXED BY events_when WHERE seq % 512 = 0 AND ts < ?1",
                    [ts],
                    |row| row.get(0),
                )?;
                Ok(u64::try_from(sampled).unwrap_or(0))
            })
            .await
    }

    /// Deletes stored events older than `before_ts`, keeping the newest
    /// `keep` regardless. Returns how many were removed.
    ///
    /// The newest `keep` are counted, and not taken to be those within
    /// `keep` of the last number: numbers have gaps, and the journal has
    /// written down where its own are ([`crate::numbered::floor`]).
    ///
    /// Events are kept in the order they were said, so the old ones are
    /// the first, and where they end is found without reading the rest:
    /// from the last of the few whose time is in an index (one in 512,
    /// `events_when`) that is old, and then event by event until one is
    /// not. What it reads is what it takes away. An event said under a
    /// clock that was ahead is not old and stays where it is; old ones
    /// after it go once one of the few after it is old too.
    pub async fn prune_events(&self, before_ts: i64, keep: u64) -> Result<usize> {
        let last = self.last_seq();
        let inner = self.inner.clone();
        self.inner
            .db
            .call(move |connection| {
                let transaction = crate::gather::whole(connection)?;
                let floor = match crate::numbered::holed(&transaction, keep)? {
                    Some(floor) => floor,
                    None => crate::numbered::floor(last, keep, &crate::numbered::gaps(&transaction)?),
                } as i64;
                let sampled: i64 = transaction.query_row(
                    "SELECT coalesce(max(seq), 0) FROM events INDEXED BY events_when
                     WHERE seq % 512 = 0 AND ts < ?1 AND seq <= ?2",
                    [before_ts, floor],
                    |row| row.get(0),
                )?;
                let newer: Option<i64> = transaction
                    .query_row(
                        "SELECT seq FROM events WHERE seq > ?1 AND seq <= ?2 AND ts >= ?3 ORDER BY seq LIMIT 1",
                        [sampled, floor, before_ts],
                        |row| row.get(0),
                    )
                    .map(Some)
                    .or_else(|err| match err {
                        rusqlite::Error::QueryReturnedNoRows => Ok(None),
                        other => Err(other),
                    })?;
                let last = newer.map_or(floor, |seq| seq - 1);
                let gone = transaction.execute("DELETE FROM events WHERE seq <= ?1 AND ts < ?2", [last, before_ts])?;
                if gone > 0 {
                    crate::numbered::keep_said(&transaction, inner.given.load(Ordering::SeqCst) as i64)?;
                }
                transaction.commit()?;
                Ok(gone)
            })
            .await
    }
}

impl Journal {
    /// Takes out of the journal every event on `topic` whose data says
    /// `key` is `value`, a text: for what was said in something that has
    /// been taken away, which the journal would otherwise go on holding,
    /// and a copy of the factory with it. Returns how many went. Whoever
    /// has read an event already has read it; nothing that follows the
    /// journal is told of what is no longer there.
    pub async fn forget_events(&self, topic: &str, key: &str, value: &str) -> Result<usize> {
        // As it is written in the data: a text inside another text has its
        // quotes escaped, so this is only ever the field itself.
        let holds = format!("{}:{}", serde_json::json!(key), serde_json::json!(value));
        let topic = topic.to_string();
        let inner = self.inner.clone();
        self.inner
            .db
            .call(move |connection| {
                let transaction = crate::gather::whole(connection)?;
                let gone = transaction.execute(
                    "DELETE FROM events WHERE topic = ?1 AND instr(data, ?2) > 0",
                    [&topic, &holds],
                )?;
                // The last thing said may be among them, and its number
                // is not given again.
                if gone > 0 {
                    crate::numbered::keep_said(&transaction, inner.given.load(Ordering::SeqCst) as i64)?;
                }
                transaction.commit()?;
                Ok(gone)
            })
            .await
    }
}

/// Writes an event down as it is, its number given: by whoever numbers
/// it, on the database's thread.
pub(crate) fn insert(connection: &rusqlite::Connection, event: &Event) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO events (seq, ts, topic, device, instance, run, step, actor, data, takes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            event.seq as i64,
            event.ts,
            event.topic,
            event.source.device,
            event.source.instance,
            event.source.run,
            event.source.step,
            event.source.actor,
            event.data.to_string(),
            event.source.takes,
        ],
    )?;
    Ok(())
}

impl crate::Inner {
    /// Takes in an event that has just been written, on the database's
    /// thread, by a piece of work that went well: its number is given,
    /// and what follows from its being kept follows the commit that
    /// keeps it ([`crate::gather::then`]), in the order events were
    /// written. Whether it is the place the journal was told is asked
    /// here, once; if it is, the commit ends with this piece of work.
    /// One that is passed on and not `stored` ([`crate::Writing::pass_on`])
    /// is all of that but written: it is in what the order holds of what
    /// was said, where an order is kept, since nothing can read it back.
    pub(crate) fn written(
        self: &Arc<Self>,
        event: Event,
        chain: Option<kernel::Chain>,
        stored: bool,
    ) -> Arc<Event> {
        self.given.store(event.seq, Ordering::SeqCst);
        if let Some(order) = self.held.order.get() {
            order.written(event.seq);
        }
        let event = Arc::new(event);
        let place = self.shut.asks(&event);
        if place {
            crate::gather::cut();
        }
        let (inner, written) = (self.clone(), event.clone());
        crate::gather::then(move |kept| match kept {
            true => {
                // Before whoever follows is told, so that a test reading
                // both finds it once.
                if !stored {
                    inner.passed.fetch_max(written.seq, Ordering::SeqCst);
                    if let Some(order) = inner.held.order.get() {
                        order.passed(&inner.name, &written);
                    }
                }
                inner.kept(&written, chain);
                if place {
                    inner.shut.reach();
                }
            }
            // Nothing of the commit was kept: its numbers were never given.
            false => inner.given.store(inner.last_seq.load(Ordering::SeqCst), Ordering::SeqCst),
        });
        event
    }

    /// An event is kept: it is the last there is, it has its place in a
    /// chain, and everyone following is told. In the order they are
    /// kept, on the database's thread.
    fn kept(&self, event: &Arc<Event>, chain: Option<kernel::Chain>) {
        self.last_seq.store(event.seq, Ordering::SeqCst);
        if let Some(chain) = chain {
            let mut chains = self.chains.lock().expect("journal lock");
            chains.push_back((event.seq, chain));
            if chains.len() > CHAINS_KEPT {
                chains.pop_front();
            }
        }
        // No subscribers is not an error.
        let _ = self.live.send(event.clone());
    }
}

/// Whether `topics` is exactly one topic, with nothing in it to match.
fn is_one_topic(topics: &str) -> bool {
    let topics = topics.trim();
    !topics.is_empty() && !topics.contains([',', '*'])
}

/// What each page a follower reads back from the log is measured as:
/// what the journal had said since the follower's place, or what it fell
/// behind on. How much was said between a follower's place being taken
/// and its first look goes by how the work about it fell, so what one
/// piece of work cost the factory is read without them (`costed.rs` in
/// the API suite).
pub const READ_BACK: &str = "db.read_back";

/// A position in the event stream that can be read forward forever.
pub struct Follow {
    journal: Journal,
    live: broadcast::Receiver<Arc<Event>>,
    topics: String,
    /// Sequence number of the last stored event seen, wanted or not.
    cursor: u64,
    /// Where it follows from: nothing at or before it is handed over.
    from: u64,
    /// Where the journal was as this subscribed: what comes live is
    /// numbered from there.
    since: u64,
    /// The numbers of what was read from the log that may come live as
    /// well: from where the journal was as this subscribed. What is
    /// passed on with a number and not kept ([`crate::Writing::pass_on`])
    /// comes live with a number the log has not, and is not taken for
    /// one read there already.
    read: HashSet<u64>,
    backlog: VecDeque<Arc<Event>>,
    caught_up: bool,
}

impl Follow {
    /// The next matching event. Waits if there is none yet.
    pub async fn next(&mut self) -> Result<Arc<Event>> {
        loop {
            if let Some(event) = self.next_or_behind().await? {
                return Ok(event);
            }
        }
    }

    /// The next matching event, or nothing where this fell too far behind
    /// what is said as it is said and goes back to the log for what was
    /// kept meanwhile: what was passed on meanwhile and not kept
    /// ([`crate::Writing::pass_on`]) is not there, and whoever follows
    /// something that is only passed on reads what it came to another way.
    /// Asked again, it goes on from the log.
    pub async fn next_or_behind(&mut self) -> Result<Option<Arc<Event>>> {
        loop {
            if let Some(event) = self.backlog.pop_front() {
                return Ok(Some(event));
            }
            if !self.caught_up {
                self.read_backlog().await?;
                continue;
            }
            match self.live.recv().await {
                Ok(event) => {
                    if event.is_durable() {
                        if event.seq <= self.cursor {
                            let before = event.seq <= self.from || event.seq < self.since;
                            if self.read.remove(&event.seq) || before {
                                continue;
                            }
                        } else {
                            self.cursor = event.seq;
                        }
                    }
                    if self.wants(&event) {
                        return Ok(Some(event));
                    }
                }
                // Fell too far behind the live stream: go back to the log,
                // which has everything kept. Ephemeral events are lost,
                // which is what ephemeral means, and so is what was only
                // passed on: whoever asked is told.
                Err(RecvError::Lagged(_)) => {
                    self.caught_up = false;
                    return Ok(None);
                }
                Err(RecvError::Closed) => {
                    return Err(kernel::Error::unavailable("the journal has shut down"));
                }
            }
        }
    }

    /// The sequence number to resume from after a disconnect: everything up
    /// to it has been handed over by [`Follow::next`] or was not wanted.
    /// While it is catching up that is short of how far it has read, by
    /// what it has read and not yet handed over.
    pub fn cursor(&self) -> u64 {
        match self.backlog.front() {
            Some(waiting) => waiting.seq - 1,
            None => self.cursor,
        }
    }

    fn wants(&self, event: &Event) -> bool {
        self.topics.is_empty() || topic_matches_any(&self.topics, &event.topic)
    }

    async fn read_backlog(&mut self) -> Result<()> {
        let target = self.journal.last_seq();
        while self.cursor < target {
            let began = std::time::Instant::now();
            let page = self
                .journal
                .read(EventQuery { after: self.cursor, limit: PAGE, ..EventQuery::default() })
                .await?;
            let took = began.elapsed().as_secs_f64() * 1000.0;
            self.journal.measure_quietly(READ_BACK, &[], took, true);
            let Some(last) = page.last() else { break };
            self.cursor = last.seq;
            let since = self.since;
            self.read.extend(page.iter().map(|event| event.seq).filter(|seq| *seq >= since));
            let wanted: Vec<Arc<Event>> =
                page.into_iter().filter(|event| self.wants(event)).map(Arc::new).collect();
            self.backlog.extend(wanted);
            if !self.backlog.is_empty() {
                // Hand over what we have; the next call continues from the cursor.
                return Ok(());
            }
        }
        self.caught_up = true;
        Ok(())
    }
}
