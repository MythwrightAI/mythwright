//! What happened, kept.
//!
//! One append-only log of [`Event`]s is the source of truth for the whole
//! system. Writing an event and telling the people who care about it are the
//! same operation, in the same order, so nothing ever has to poll to find out
//! what changed: a reader either follows the live stream or asks for
//! "everything after sequence N".
//!
//! Alongside the log this crate keeps the things derived from it or needed
//! beside it: metrics rolled up by the minute, small durable key/value state
//! (cursors, settings), and the text logs of runs and services.

pub mod apart;
pub mod checkpoint;
pub mod copy;
pub mod db;
pub mod disk;
mod events;
mod every;
mod gather;
pub mod giver;
pub mod letgo;
pub mod logs;
pub mod metrics;
pub mod moment;
pub mod moved;
pub mod numbered;
mod order;
pub mod rewrite;
mod set;
mod shut;
pub mod steps;
mod together;
mod told;

use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use kernel::{Clock, Event, Result};
use tokio::sync::broadcast;

pub use db::{Db, Turn, db_error, optional};
pub use disk::Disk;
pub use events::{EventQuery, Follow, HERE, Holding, READ_BACK, stays_here};
pub use every::{FollowEvery, Places, joined};
pub use gather::{gathering_forced, whole};
pub use giver::Giver;
pub use logs::{LogLine, Logs};
pub use metrics::{MetricQuery, MetricSeries, MetricStats};
pub use order::{Following, Order};
pub use set::{Held, Listed, apart, keeps, listed};
pub use together::Writing;
pub use told::Members;

/// How many live events a slow subscriber may fall behind before it is told
/// to catch up from the log instead.
const LIVE_BUFFER: usize = 4096;

#[derive(Clone)]
pub struct Journal {
    inner: Arc<Inner>,
}

/// A journal held without holding it: for whoever asks whether
/// everything that held it has let it go.
#[derive(Clone)]
pub struct Downgraded(std::sync::Weak<Inner>);

impl Journal {
    pub fn downgrade(&self) -> Downgraded {
        Downgraded(Arc::downgrade(&self.inner))
    }
}

impl Downgraded {
    /// Whether nothing holds the journal any more.
    pub fn is_given_back(&self) -> bool {
        self.0.strong_count() == 0
    }
}

struct Inner {
    db: Db,
    clock: Arc<dyn Clock>,
    device: String,
    /// What it is called ([`Journal::name`]).
    name: String,
    live: broadcast::Sender<Arc<Event>>,
    /// The number of the last event that is kept, or passed on with a
    /// number ([`Writing::pass_on`]).
    last_seq: AtomicU64,
    /// Whether it has written down that its events have holes
    /// (`numbered::HOLES`): once something is passed on with a number.
    holed: std::sync::atomic::AtomicBool,
    /// The number of the last event this journal passed on and did not
    /// keep, or, for what came before it was opened, the last there was
    /// then ([`Journal::passed_after`]).
    passed: AtomicU64,
    /// The highest number given to an event: the same, but while events
    /// are written and the commit that keeps them is not yet made
    /// ([`gather`]). What the next is numbered above, by more than one
    /// where a run of numbers ended in between ([`giver`]).
    given: AtomicU64,
    /// Where its events' numbers come from.
    numbers: Numbers,
    /// The number of the last event it held when it was opened
    /// ([`Journal::held_when_opened`]).
    found: u64,
    metrics: metrics::Aggregator,
    /// Where what is measured through it is kept, for a journal that is
    /// kept beside a factory's own: in the factory's, which is where
    /// measurements are read. Not kept alive by it.
    measured_in: Option<std::sync::Weak<Inner>>,
    /// What its events are held to, where it is run to be tested: the
    /// same for every journal of a factory.
    held: Arc<events::Held>,
    /// Where it is to take nothing more, if it was told a place
    /// ([`Journal::shut_after`]): the same for every journal of a
    /// factory, which shut together.
    shut: Arc<shut::Shut>,
    /// What answers who an event kept to the members of something may
    /// be handed to ([`told`]): the same for every journal of a factory.
    told: Arc<told::Told>,
    /// The journals of projects kept beside this one, where it is a
    /// factory's own and was told where they are ([`set`]).
    set: std::sync::OnceLock<set::Set>,
    /// Where each recent event that is part of a chain is in it
    /// ([`Journal::chain_of`]).
    chains: std::sync::Mutex<std::collections::VecDeque<(u64, kernel::Chain)>>,
}

/// Where a journal's events come by their numbers: the giver it draws
/// from, and what it has drawn and not yet given.
struct Numbers {
    giver: Giver,
    /// Whether the giver's mark is kept in this journal's own database,
    /// and so raised in the write that first uses a run.
    here: bool,
    /// The run in hand. Read and put back on the database's thread, by
    /// whoever writes events there, one piece of work at a time.
    run: std::sync::Mutex<giver::Run>,
    /// The run asked for ahead of need, by a journal whose giver is
    /// kept elsewhere, until it is the one in hand.
    ahead: std::sync::Mutex<Option<giver::Ahead>>,
}

/// What a journal is opened beside.
enum Beside {
    /// Nothing: it is a factory's own, with a giver of its own.
    Nothing,
    /// Another journal's giver, and nothing else of that journal.
    Giver(Giver),
    /// A factory's own journal, as the journal of one of its projects
    /// ([`set`]): it draws from the factory's giver, is measured there,
    /// and shuts and is held to what its events are with it.
    Factory(Arc<Inner>),
}

impl Beside {
    fn giver(&self) -> Option<Giver> {
        match self {
            Beside::Nothing => None,
            Beside::Giver(giver) => Some(giver.clone()),
            Beside::Factory(factory) => Some(factory.numbers.giver.clone()),
        }
    }

    fn factory(&self) -> Option<&Arc<Inner>> {
        match self {
            Beside::Factory(factory) => Some(factory),
            _ => None,
        }
    }
}

const SCHEMA: &[&str] = &[
    "
    CREATE TABLE events (
        seq INTEGER PRIMARY KEY AUTOINCREMENT,
        ts INTEGER NOT NULL,
        topic TEXT NOT NULL,
        device TEXT,
        instance TEXT,
        run TEXT,
        step TEXT,
        actor TEXT,
        data TEXT NOT NULL
    );
    CREATE INDEX events_topic ON events (topic, seq);
    CREATE INDEX events_instance ON events (instance, seq) WHERE instance IS NOT NULL;
    CREATE INDEX events_run ON events (run, seq) WHERE run IS NOT NULL;
    CREATE INDEX events_ts ON events (ts);

    CREATE TABLE kv (
        namespace TEXT NOT NULL,
        key TEXT NOT NULL,
        value TEXT NOT NULL,
        PRIMARY KEY (namespace, key)
    ) WITHOUT ROWID;

    CREATE TABLE metrics (
        minute INTEGER NOT NULL,
        name TEXT NOT NULL,
        labels TEXT NOT NULL,
        count INTEGER NOT NULL,
        errors INTEGER NOT NULL,
        sum REAL NOT NULL,
        min REAL NOT NULL,
        max REAL NOT NULL,
        hist TEXT NOT NULL,
        PRIMARY KEY (minute, name, labels)
    ) WITHOUT ROWID;
    CREATE INDEX metrics_name ON metrics (name, minute);
",
    "
    -- What hearing an event takes besides what its topic does, where
    -- whoever said it said so: a scope.
    ALTER TABLE events ADD COLUMN takes TEXT;
",
    "
    -- When an event was said is asked of the journal once an hour, to
    -- find where the old ones end, and was written into an index for
    -- every event: a page of the database for each thing said. One event
    -- in 512 is in it now, which is as much as finding that place takes
    -- (`Journal::prune_events`).
    DROP INDEX events_ts;
    CREATE INDEX events_when ON events (ts) WHERE seq % 512 = 0;
    -- A journal that has events is one from before, and this read every
    -- one of them: noted with the change itself, so that the daemon says
    -- once that it was done (`REINDEXED`), however it is stopped meanwhile.
    INSERT INTO kv (namespace, key, value)
        SELECT 'journal', 'reindexed', '{}' WHERE EXISTS (SELECT 1 FROM events);
",
    "
    -- The journals kept beside this one, where it is a factory's own:
    -- one for each project that has a journal of its own, by what the
    -- journal is called, with the file it is in (`set`). A project that
    -- has no row here has no journal of its own, and what is said and
    -- kept of it is in this one. A program from before this knows no
    -- such table and reads no such file: it refuses a journal whose
    -- tables are at this version, as it must.
    CREATE TABLE journals (
        name TEXT PRIMARY KEY,
        project TEXT NOT NULL UNIQUE,
        file TEXT NOT NULL UNIQUE,
        made INTEGER NOT NULL
    ) WITHOUT ROWID;
",
    "
    -- How much of its project a journal kept beside this one keeps: a
    -- level, read as at least that (`set::keeps`). 0 is nothing: what is
    -- run for the project and what is said of it stay in this one, with
    -- the factory's own conductor, until its owner asks for it to be
    -- moved apart. 1 is what is run for it (the runner's rows, which a
    -- conductor of the project's own writes there) and what is said of
    -- it. Written when a journal is made, as the level the program that
    -- makes it keeps a project at, and never worked out: a journal
    -- listed before this says 0. A program from before this would find
    -- no rows of a project kept at 1 here and take it for one with
    -- nothing running: it refuses these tables, as it must.
    ALTER TABLE journals ADD COLUMN keeps INTEGER NOT NULL DEFAULT 0;
    -- Where what a journal keeps was moved from, and where the giver of
    -- numbers stood when it was: for a project of a factory from before
    -- that its owner had moved apart, the journal its rows were in (the
    -- factory's own) and the number everything of the project said
    -- before the move is at or below. Nothing in either for a project
    -- that was made apart, which is every one this program makes:
    -- whatever moves a project writes them, with the level, in the one
    -- write that decides the move. They are here so that no later step
    -- has to change this table again.
    ALTER TABLE journals ADD COLUMN moved_from TEXT;
    ALTER TABLE journals ADD COLUMN moved_at INTEGER;
",
];

/// Which of the journal's steps makes the index of when anew.
const REINDEXES: usize = 2;

/// Where a journal from before notes that its index of when was made anew
/// as this program first opened it, and how long that took where it is
/// known (`took_ms`), until whoever opened it has said so. It is the one
/// thing an upgrade does to a journal that takes as long as the journal is
/// big: measured, 1.2 s for one of 296 MB and 840,000 events read from the
/// disk, and 0.1 s with it in memory.
pub const REINDEXED: (&str, &str) = ("journal", "reindexed");

/// What the factory's own journal is called: the one that keeps what is
/// nobody's, the giver's mark, and what is said and kept of a project
/// that has no journal of its own ([`Journal::name`]).
pub const FACTORY: &str = "factory";

/// Where a journal that is kept beside a factory's own says what it is
/// called, of its own small durable state: written as its file is made,
/// and held to each time it is opened, so that a file put in another's
/// place is refused and not believed. The factory's own says nothing,
/// as it never did, and is refused only where it says it is another's.
pub const NAME: (&str, &str) = ("journal", "name");

/// What the journal of the project `id` is called. A journal is called
/// for whose events it keeps, by whoever opens it and knows: never for
/// where its file is, which is the program's own to change, and so the
/// same after a restart and in a copy. No project's is called as the
/// factory's is, whatever the project is called.
pub fn of_project(id: &str) -> String {
    format!("project:{id}")
}

/// The project whose events the journal called `name` keeps
/// ([`of_project`]); none, for the factory's own.
pub fn project_of(name: &str) -> Option<&str> {
    name.strip_prefix("project:")
}

impl Journal {
    /// Opens the journal in `db`. `device` names this machine in every event
    /// it records. Its events are numbered by a giver of its own, whose
    /// mark is kept in `db` ([`Journal::giver`]): the factory's journal,
    /// and called that ([`FACTORY`]).
    pub async fn open(db: Db, clock: Arc<dyn Clock>, device: impl Into<String>) -> Result<Journal> {
        Self::open_with(db, clock, device.into(), FACTORY.to_string(), Beside::Nothing).await
    }

    /// Opens the journal in `db`, its events numbered by `giver`, which
    /// another journal keeps: no event of either has a number the other
    /// has, and each gives its own in order. What it held before (it may
    /// be a journal from before, that numbered its own) the giver is told,
    /// and gives nothing at or below; so it is opened before anything
    /// else that draws there says anything.
    ///
    /// It is another journal than the one that keeps the giver, and says
    /// what it is called (`name`, as [`of_project`] gives it): whoever
    /// keeps a place in both keeps each by its journal's name.
    pub async fn open_drawing(
        db: Db,
        clock: Arc<dyn Clock>,
        device: impl Into<String>,
        name: impl Into<String>,
        giver: &Giver,
    ) -> Result<Journal> {
        let name = name.into();
        if name.is_empty() || name == FACTORY {
            return Err(kernel::Error::invalid(format!(
                "a journal that draws its numbers from another's giver is called something of its own,                  and `{name}` is not: `{FACTORY}` is what the journal that keeps the giver is called"
            )));
        }
        Self::open_with(db, clock, device.into(), name, Beside::Giver(giver.clone())).await
    }

    /// What this journal is called: [`FACTORY`] for the factory's own,
    /// and for a project's what [`of_project`] gives. A place among the
    /// events is a place in one journal, and is kept with this; a number
    /// of one journal is never held against a number of another.
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// The giver this journal's numbers come from, for another journal to
    /// draw from as well ([`Journal::open_drawing`]).
    pub fn giver(&self) -> Giver {
        self.inner.numbers.giver.clone()
    }

    async fn open_with(
        db: Db,
        clock: Arc<dyn Clock>,
        device: String,
        name: String,
        beside: Beside,
    ) -> Result<Journal> {
        // What the journal's tables were at, to know whether this is the
        // opening that makes the index of when anew, and how long it took.
        let before: i64 = db
            .call(|connection| {
                optional(connection.query_row(
                    "SELECT version FROM schema_versions WHERE namespace = 'journal'",
                    [],
                    |row| row.get(0),
                ))
            })
            .await?
            .unwrap_or(0);
        let began = std::time::Instant::now();
        db.migrate("journal", SCHEMA).await?;
        if before > 0 && before as usize <= REINDEXES {
            let took = serde_json::json!({ "took_ms": began.elapsed().as_millis() as u64 }).to_string();
            db.call(move |connection| {
                connection.execute(
                    "UPDATE kv SET value = ?3 WHERE namespace = ?1 AND key = ?2 AND value = '{}'",
                    [REINDEXED.0, REINDEXED.1, &took],
                )
            })
            .await?;
        }
        // The last event it holds, and the last number an event was given
        // by a journal that numbers its own: the same, unless the last
        // events were taken out. And one in which nothing was ever said is
        // given the table a journal is made with now ([`numbered`]).
        let (held, last_seq) = db.call(numbered::settle).await?;
        db.call(|connection| events::shed(connection)).await?;
        set::called(&db, &name).await?;
        // Whoever gives its numbers gives none at or below what it ever
        // held: its own giver begins above that, and another's is told.
        let giver = match beside.giver() {
            Some(giver) => {
                giver.above(last_seq as u64).await?;
                giver
            }
            None => Giver::kept_in(&db, last_seq as u64).await?,
        };
        let factory = beside.factory();
        let here = giver.is_kept_in(&db);
        let numbers = Numbers { here, giver, run: Default::default(), ahead: Default::default() };
        let (live, _) = broadcast::channel(LIVE_BUFFER);
        let journal = Journal {
            inner: Arc::new(Inner {
                db: db.clone(),
                clock,
                device,
                name,
                live,
                last_seq: AtomicU64::new(last_seq as u64),
                holed: Default::default(),
                // What a program before this one passed on is gone with it:
                // a place from before it was opened may have missed some.
                passed: AtomicU64::new(last_seq as u64),
                given: AtomicU64::new(last_seq as u64),
                numbers,
                found: held as u64,
                metrics: metrics::Aggregator::default(),
                measured_in: factory.map(Arc::downgrade),
                held: factory.map(|factory| factory.held.clone()).unwrap_or_default(),
                shut: factory.map(|factory| factory.shut.clone()).unwrap_or_default(),
                told: factory.map(|factory| factory.told.clone()).unwrap_or_default(),
                set: Default::default(),
                chains: Default::default(),
            }),
        };
        journal.inner.shut.holds(&db);
        // Its own database's work is measured as the journal's; that of
        // one kept beside a factory's, in the factory's, by its name.
        match factory {
            Some(factory) => Journal { inner: factory.clone() }.measures(&db, journal.name()),
            None => journal.measures(&db, "journal"),
        }
        Ok(journal)
    }

    /// Keeps, as measurements here, how long each piece of work `db` does
    /// waited its turn and how long it took: `db.wait` and `db.work`, by the
    /// database (`db`), by whose work it was (`caller`) and by whether it
    /// wrote or only read (`kind`). One thread does all of a database's
    /// work, so this is what says who is waiting behind whom. And of work
    /// that shared its commit with other work, which is done only where
    /// the thread had fallen behind ([`gather`]): how many pieces the
    /// commit kept (`db.shared_count`) and how long this one was done
    /// before it was answered (`db.held`).
    pub fn measures(&self, db: &Db, name: &str) {
        // Not kept alive by the database it is itself kept alive by.
        let (inner, name) = (Arc::downgrade(&self.inner), name.to_string());
        db.measured(move |turn| {
            let Some(inner) = inner.upgrade() else {
                return;
            };
            let journal = Journal { inner };
            let labels = [
                ("db", name.as_str()),
                ("caller", turn.by),
                ("kind", if turn.wrote { "write" } else { "read" }),
            ];
            // Quietly: writing measurements is itself a piece of the
            // database's work, and measured loudly it would be the reason
            // for the next write, and that for the one after. Work that
            // failed is worth a write.
            journal.measure_quietly("db.wait", &labels, turn.waited_ms, true);
            if turn.shared > 1 {
                journal.measure_quietly("db.shared_count", &labels, turn.shared as f64, true);
                journal.measure_quietly("db.held", &labels, turn.held_ms, true);
            }
            if turn.ok {
                journal.measure_quietly("db.work", &labels, turn.took_ms, true);
            } else {
                journal.measure("db.work", &labels, turn.took_ms, false);
            }
        });
    }

    /// An in-memory journal on the given clock, for tests.
    pub async fn memory(clock: Arc<dyn Clock>) -> Result<Journal> {
        Journal::open(Db::memory()?, clock, "test-device").await
    }

    pub fn db(&self) -> &Db {
        &self.inner.db
    }

    /// The number of the last event the journal held when it was opened:
    /// the last thing kept before this start of the daemon, whatever has
    /// been said since. Nought where it held nothing.
    pub fn held_when_opened(&self) -> u64 {
        self.inner.found
    }

    pub fn clock(&self) -> &Arc<dyn Clock> {
        &self.inner.clock
    }

    pub fn now_ms(&self) -> i64 {
        self.inner.clock.now_ms()
    }

    /// This machine's name.
    pub fn device(&self) -> &str {
        &self.inner.device
    }

    /// A new id of the given kind, e.g. `run_...`.
    pub fn new_id(&self, prefix: &str) -> String {
        kernel::id::new(prefix, self.now_ms())
    }

    /// Reads a small durable value.
    pub async fn kv_get(&self, namespace: &str, key: &str) -> Result<Option<serde_json::Value>> {
        let (namespace, key) = (namespace.to_string(), key.to_string());
        let text = self
            .inner
            .db
            .call(move |connection| {
                optional(connection.query_row(
                    "SELECT value FROM kv WHERE namespace = ?1 AND key = ?2",
                    [&namespace, &key],
                    |row| row.get::<_, String>(0),
                ))
            })
            .await?;
        Ok(text.and_then(|text| serde_json::from_str(&text).ok()))
    }

    /// Writes a small durable value. `null` removes it.
    pub async fn kv_set(&self, namespace: &str, key: &str, value: &serde_json::Value) -> Result<()> {
        let (namespace, key) = (namespace.to_string(), key.to_string());
        let text = (!value.is_null()).then(|| value.to_string());
        self.inner
            .db
            .call(move |connection| match text {
                Some(text) => connection.execute(
                    "INSERT INTO kv (namespace, key, value) VALUES (?1, ?2, ?3)
                     ON CONFLICT (namespace, key) DO UPDATE SET value = excluded.value",
                    [&namespace, &key, &text],
                ),
                None => {
                    connection.execute("DELETE FROM kv WHERE namespace = ?1 AND key = ?2", [&namespace, &key])
                }
            })
            .await
            .map(|_| ())
    }

    /// Every key in a namespace, with its value.
    pub async fn kv_list(&self, namespace: &str) -> Result<Vec<(String, serde_json::Value)>> {
        let namespace = namespace.to_string();
        let rows = self
            .inner
            .db
            .call(move |connection| {
                let mut statement =
                    connection.prepare("SELECT key, value FROM kv WHERE namespace = ?1 ORDER BY key")?;
                let rows = statement.query_map([&namespace], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?;
        Ok(rows
            .into_iter()
            .filter_map(|(key, text)| Some((key, serde_json::from_str(&text).ok()?)))
            .collect())
    }
}
