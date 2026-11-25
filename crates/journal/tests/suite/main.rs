//! One test binary for the crate; add modules here.

mod beside;
mod between;
mod brought;
mod copy;
mod disk;
mod drawn;
mod every;
mod gathered;
mod given;
mod kept;
mod labelled;
mod letgo;
mod made;
mod moment;
mod passed;
mod steps;
mod together;
mod told;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use journal::{Db, EventQuery, Journal, Logs, MetricQuery, Turn};
use kernel::{Clock, ManualClock, Source};
use serde_json::json;

/// Whether this program was told that a project is given no journal of
/// its own (`PROJECT_JOURNALS=one`, `journal::apart`), which to the
/// journal itself means one thing: [`Journal::own`] makes none, and
/// gives the factory's own (`beside.rs` says so, where it is told). This
/// suite makes each journal it is of and adds no project through
/// anything, so it is played as it is whatever it was told; but a test
/// of a project's own journal holds nothing where none is made, and is
/// not played there: it asks first, and says so.
fn one_journal() -> bool {
    let one = journal::apart() == Some(false);
    if one {
        println!("not played: it is of a project's own journal, and the suite is played with one journal");
    }
    one
}

async fn journal() -> (Journal, ManualClock) {
    let clock = ManualClock::new();
    (Journal::memory(Arc::new(clock.clone())).await.unwrap(), clock)
}

/// A folder of a test's own, under the build's target directory, and gone
/// when the test has done with it: a suite that left one behind each time
/// it ran had left thousands.
struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = std::path::Path;

    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

impl AsRef<std::path::Path> for Scratch {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A fresh directory under the build's own target directory. Never the system
/// temp directory: on the machines this runs on that is RAM.
fn scratch(name: &str) -> Scratch {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("journal-{name}-{}", kernel::id::secret()));
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

#[tokio::test]
async fn events_are_numbered_stamped_and_attributed() {
    let (journal, clock) = journal().await;
    let first = journal.emit("build.started", Source::actor("user:ada"), json!({ "n": 1 })).await.unwrap();
    clock.advance(Duration::from_secs(5));
    let second = journal.emit("build.finished", Source::default(), json!({ "ok": true })).await.unwrap();

    assert_eq!((first.seq, second.seq), (1, 2));
    assert_eq!(second.ts - first.ts, 5_000);
    assert_eq!(first.source.device.as_deref(), Some("test-device"), "the device is filled in");
    assert_eq!(first.source.actor.as_deref(), Some("user:ada"));
    assert_eq!(journal.last_seq(), 2);

    let stored = journal.read(EventQuery::default()).await.unwrap();
    assert_eq!(stored, [(*first).clone(), (*second).clone()]);
}

#[tokio::test]
async fn reading_filters_by_topic_source_and_position() {
    let (journal, _) = journal().await;
    for (topic, run) in
        [("step.started", "r1"), ("step.finished", "r1"), ("board.item.moved", "r2"), ("step.started", "r2")]
    {
        journal.emit(topic, Source { run: Some(run.into()), ..Source::default() }, json!({})).await.unwrap();
    }
    assert_eq!(seqs(&journal, EventQuery { topics: "step.*".into(), ..Default::default() }).await, [1, 2, 4]);
    assert_eq!(
        seqs(&journal, EventQuery { topics: "board.**, step.finished".into(), ..Default::default() }).await,
        [2, 3]
    );
    assert_eq!(
        seqs(&journal, EventQuery { topics: "step.started".into(), ..Default::default() }).await,
        [1, 4]
    );
    assert_eq!(
        seqs(&journal, EventQuery { topics: " step.started ".into(), after: 1, ..Default::default() }).await,
        [4]
    );
    assert_eq!(seqs(&journal, EventQuery { run: Some("r2".into()), ..Default::default() }).await, [3, 4]);
    assert_eq!(seqs(&journal, EventQuery { after: 2, ..Default::default() }).await, [3, 4]);
    assert_eq!(
        seqs(&journal, EventQuery { newest_first: true, limit: 2, ..Default::default() }).await,
        [4, 3]
    );
}

async fn seqs(journal: &Journal, query: EventQuery) -> Vec<u64> {
    journal.read(query).await.unwrap().into_iter().map(|event| event.seq).collect()
}

#[tokio::test]
async fn following_replays_the_backlog_then_goes_live_without_gap_or_duplicate() {
    let (journal, _) = journal().await;
    for n in 1..=3 {
        journal.emit("tick", Source::default(), json!(n)).await.unwrap();
    }
    let mut follow = journal.follow(1, "");
    // Emitted after the follower exists but before it has read anything:
    // present in both the backlog and the live buffer.
    journal.emit("tick", Source::default(), json!(4)).await.unwrap();

    let mut seen = Vec::new();
    for _ in 0..3 {
        seen.push(follow.next().await.unwrap().seq);
    }
    assert_eq!(seen, [2, 3, 4]);

    journal.emit("tick", Source::default(), json!(5)).await.unwrap();
    assert_eq!(follow.next().await.unwrap().seq, 5);
    assert_eq!(follow.cursor(), 5);
}

#[tokio::test]
async fn a_follower_that_is_catching_up_resumes_from_what_it_has_handed_over_not_from_what_it_has_read() {
    let (journal, _) = journal().await;
    for topic in ["tick", "noise", "tick", "noise"] {
        journal.emit(topic, Source::default(), json!({})).await.unwrap();
    }
    let mut follow = journal.follow(0, "tick");
    assert_eq!(follow.next().await.unwrap().seq, 1);
    assert_eq!(
        follow.cursor(),
        2,
        "the second tick is read and not handed over; what is before it was not wanted"
    );
    assert_eq!(follow.next().await.unwrap().seq, 3);
    assert_eq!(follow.cursor(), 4, "nothing is waiting, so everything read is behind it");
}

#[tokio::test]
async fn following_filters_by_topic_and_sees_ephemeral_events() {
    let (journal, _) = journal().await;
    let mut follow = journal.follow(0, "log.*, done");
    journal.emit("noise", Source::default(), json!({})).await.unwrap();
    journal.emit_ephemeral("log.line", Source::default(), json!({ "text": "hello" }));
    journal.emit("done", Source::default(), json!({})).await.unwrap();

    let line = follow.next().await.unwrap();
    assert_eq!((line.topic.as_str(), line.is_durable()), ("log.line", false));
    assert_eq!(follow.next().await.unwrap().topic, "done");
    assert_eq!(
        journal.read(EventQuery::default()).await.unwrap().len(),
        2,
        "ephemeral events are not stored"
    );
}

#[tokio::test]
async fn a_follower_that_falls_behind_catches_up_from_the_log() {
    let (journal, _) = journal().await;
    let mut follow = journal.follow(0, "");
    assert_eq!(journal.emit("first", Source::default(), json!({})).await.unwrap().seq, 1);
    assert_eq!(follow.next().await.unwrap().seq, 1);
    // More than the live buffer holds, while the follower is not reading.
    for _ in 0..5_000 {
        journal.emit("flood", Source::default(), json!({})).await.unwrap();
    }
    let mut last = 1;
    for _ in 0..5_000 {
        let event = follow.next().await.unwrap();
        assert_eq!(event.seq, last + 1, "no event may be skipped or repeated");
        last = event.seq;
    }
    assert_eq!(last, 5_001);
}

#[tokio::test]
async fn pruning_keeps_recent_events_and_a_floor() {
    let (journal, clock) = journal().await;
    for _ in 0..10 {
        journal.emit("old", Source::default(), json!({})).await.unwrap();
    }
    clock.advance(Duration::from_secs(3600));
    journal.emit("new", Source::default(), json!({})).await.unwrap();

    let cutoff = journal.now_ms() - 60_000;
    assert_eq!(
        journal.prune_events(cutoff, 5).await.unwrap(),
        6,
        "ten old, but the newest five overall are kept"
    );
    let left: Vec<u64> = journal.read(EventQuery::default()).await.unwrap().iter().map(|e| e.seq).collect();
    assert_eq!(left, [7, 8, 9, 10, 11]);
}

/// The first and the last event the journal holds, and how many.
async fn span(journal: &Journal) -> (i64, i64, i64) {
    journal
        .db()
        .call(|connection| {
            connection.query_row("SELECT min(seq), max(seq), count(*) FROM events", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn pruning_finds_where_the_old_events_end_with_one_event_in_many_in_the_index_of_when() {
    let (journal, clock) = journal().await;
    for _ in 0..1300 {
        journal.emit("old", Source::default(), json!({})).await.unwrap();
    }
    clock.advance(Duration::from_secs(3600));
    for _ in 0..700 {
        journal.emit("new", Source::default(), json!({})).await.unwrap();
    }
    // An event is not written into an index of when it was said: one in
    // 512 is, which is what a write for each event is spared.
    let indexed: i64 = journal
        .db()
        .call(|connection| {
            connection.query_row(
                "SELECT count(*) FROM events INDEXED BY events_when WHERE seq % 512 = 0",
                [],
                |row| row.get(0),
            )
        })
        .await
        .unwrap();
    assert_eq!(indexed, 3, "of two thousand");

    let cutoff = journal.now_ms() - 60_000;
    assert_eq!(journal.prune_events(cutoff, 100).await.unwrap(), 1300, "every old one, and no new one");
    assert_eq!(span(&journal).await, (1301, 2000, 700));
    assert_eq!(journal.prune_events(cutoff, 100).await.unwrap(), 0, "and there is nothing more to take");

    // The newest are kept however old they are.
    clock.advance(Duration::from_secs(3600));
    assert_eq!(journal.prune_events(journal.now_ms() - 60_000, 100).await.unwrap(), 600);
    assert_eq!(span(&journal).await, (1901, 2000, 100));
}

#[tokio::test]
async fn an_event_stamped_by_a_clock_that_was_ahead_is_kept_and_the_old_ones_after_it_still_go() {
    let (journal, clock) = journal().await;
    for _ in 0..1200 {
        journal.emit("old", Source::default(), json!({})).await.unwrap();
    }
    // Ten of them, early on, were said while the clock was a year ahead.
    let ahead = journal.now_ms() + 365 * 24 * 3600 * 1000;
    journal
        .db()
        .call(move |connection| {
            connection.execute("UPDATE events SET ts = ?1 WHERE seq BETWEEN 21 AND 30", [ahead])
        })
        .await
        .unwrap();
    clock.advance(Duration::from_secs(3600));
    journal.emit("new", Source::default(), json!({})).await.unwrap();

    let cutoff = journal.now_ms() - 60_000;
    // What is old up to the last of the indexed few that is old goes, on
    // either side of the ten; the rest of the old go with a later one.
    assert_eq!(journal.prune_events(cutoff, 1).await.unwrap(), 1190);
    let left: Vec<u64> = journal.read(EventQuery::default()).await.unwrap().iter().map(|e| e.seq).collect();
    assert_eq!(left, [21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 1201]);
}

#[tokio::test]
async fn state_survives_reopening_and_numbers_go_on_above_all_that_was_drawn() {
    let dir = scratch("reopen");
    let path = dir.join("journal.db");
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new());
    {
        let journal = Journal::open(Db::open(&path).unwrap(), clock.clone(), "dev").await.unwrap();
        journal.emit("a", Source::default(), json!({})).await.unwrap();
        journal.kv_set("cursor", "trigger-1", &json!({ "seq": 1 })).await.unwrap();
    }
    let journal = Journal::open(Db::open(&path).unwrap(), clock, "dev").await.unwrap();
    assert_eq!(journal.last_seq(), 1);
    // The journal that was closed had drawn a run of numbers and given
    // one: the rest are never given, and the next is above them all.
    assert_eq!(journal.emit("b", Source::default(), json!({})).await.unwrap().seq, journal::giver::RUN + 1);
    assert_eq!(journal.kv_get("cursor", "trigger-1").await.unwrap(), Some(json!({ "seq": 1 })));
    assert_eq!(journal.kv_get("cursor", "missing").await.unwrap(), None);

    journal.kv_set("cursor", "trigger-1", &json!(null)).await.unwrap();
    assert_eq!(journal.kv_list("cursor").await.unwrap(), []);
}

#[tokio::test]
async fn a_transaction_takes_its_turn_to_write_when_it_begins_and_is_not_refused_part_way() {
    // One that read first and asked to write afterwards is refused at once
    // if anything else wrote to the file in between, however long it was
    // told to wait: a node started again on data whose last holder was
    // still putting its work away was told the database was locked, four
    // times in a thousand. Holding the turn from the start, it waits.
    let dir = scratch("turn");
    let db = Db::open(&dir.join("journal.db")).unwrap();
    //
    // The turn is taken by the thread as it begins a piece of work, before
    // the work has read or written anything; and a whole the work opens
    // for itself is inside that, with the turn already had.
    let turn = |connection: &mut rusqlite::Connection| {
        let begun = connection.transaction_state(Some(rusqlite::MAIN_DB))?;
        let whole = journal::whole(connection)?;
        Ok((begun, whole.transaction_state(Some(rusqlite::MAIN_DB))?))
    };
    let writes = rusqlite::TransactionState::Write;
    assert_eq!(db.call(turn).await.unwrap(), (writes, writes));
    // Work that is done apart from the rest takes its turn the same way.
    assert_eq!(db.call_apart(turn).await.unwrap(), (writes, writes));
}

#[tokio::test]
async fn work_that_begins_a_transaction_of_its_own_is_refused_in_a_sentence_that_says_what_to_write() {
    // The thread has a transaction open for whatever it is asked, which
    // whoever writes a piece of work did not begin and may not know of.
    // A second cannot be begun inside it; the database says only that
    // there is one, so the thread says what to write instead.
    let dir = scratch("second");
    let db = Db::open(&dir.join("journal.db")).unwrap();
    db.call(|connection| connection.execute_batch("CREATE TABLE things (name TEXT PRIMARY KEY)"))
        .await
        .unwrap();
    type Tries = fn(&mut rusqlite::Connection) -> rusqlite::Result<()>;
    let ways: [(&str, Tries); 3] = [
        ("transaction", |connection| connection.transaction().map(|_| ())),
        ("unchecked_transaction", |connection| connection.unchecked_transaction().map(|_| ())),
        ("BEGIN", |connection| connection.execute_batch("BEGIN")),
    ];
    for (way, tries) in ways {
        for apart in [false, true] {
            let work = move |connection: &mut rusqlite::Connection| {
                connection.execute("INSERT INTO things VALUES ('before it tried')", [])?;
                tries(connection)
            };
            let refused = match apart {
                false => db.call(work).await.unwrap_err(),
                true => db.call_apart(work).await.unwrap_err(),
            };
            assert!(refused.message.contains("`journal::whole(connection)`"), "{way}: {refused:?}");
            assert!(
                !refused.message.contains("database:"),
                "{way}, in the database's own words: {refused:?}"
            );
            // What it wrote before it tried was written with no whole, and
            // is kept as any piece of work's is that fails part way.
            db.call(|connection| connection.execute("DELETE FROM things", [])).await.unwrap();
        }
    }
    // Work that can fail for its own reasons is told the same.
    let refused = db
        .try_call(|connection| connection.transaction().map(|_| ()).map_err(journal::db_error))
        .await
        .unwrap_err();
    assert!(refused.message.contains("`journal::whole(connection)`"), "{refused:?}");
    // And what cannot be done inside a transaction at all is said to be
    // for a connection of its own.
    let outside = db.call(|connection| connection.execute_batch("VACUUM")).await.unwrap_err();
    assert!(outside.message.contains("a connection of its own"), "{outside:?}");
    // A whole is taken, and the thread goes on as it was.
    db.call(|connection| {
        let whole = journal::whole(connection)?;
        whole.execute("INSERT INTO things VALUES ('in a whole')", [])?;
        whole.commit()
    })
    .await
    .unwrap();
    // The same words from a connection that is nobody's thread are the
    // database's own: there the transaction is whoever's opened it.
    let own = rusqlite::Connection::open_in_memory().unwrap();
    own.execute_batch("BEGIN").unwrap();
    let said = journal::db_error(own.execute_batch("BEGIN").unwrap_err());
    assert!(
        said.message.contains("within a transaction") && !said.message.contains("journal::whole"),
        "{said:?}"
    );
}

#[tokio::test]
async fn a_damaged_database_is_refused_not_replaced() {
    let dir = scratch("damaged");
    let path = dir.join("journal.db");
    std::fs::write(&path, "this is not a database, it is a text file with enough bytes to look like one")
        .unwrap();
    let err = Db::open(&path).err().expect("a damaged file must not open");
    assert!(err.message.contains("journal.db"), "{}", err.message);
    assert!(
        std::fs::read_to_string(&path).unwrap().starts_with("this is not"),
        "the file must be left as it was"
    );
}

#[tokio::test]
async fn the_database_is_built_keeping_no_count_of_the_memory_it_has_in_use() {
    // The count is one lock for the whole program, taken at every
    // allocation by every database's thread: a factory with a journal for
    // each project, or a test program with a node for each test, had them
    // all wait on it (`.cargo/config.toml`, `docs/build-times.md`).
    let db = Db::memory().unwrap();
    let off: bool = db
        .call(|connection| {
            connection
                .query_row("SELECT sqlite_compileoption_used('DEFAULT_MEMSTATUS=0')", [], |row| row.get(0))
        })
        .await
        .unwrap();
    assert!(
        off,
        "the database library was built counting the memory it has in use: `LIBSQLITE3_FLAGS` in \
         `.cargo/config.toml` says not to, and is read only where cargo is run from inside this repository"
    );
}

#[tokio::test]
async fn a_newer_schema_is_refused() {
    let db = Db::memory().unwrap();
    db.migrate("thing", &["CREATE TABLE a (x)", "CREATE TABLE b (x)"]).await.unwrap();
    db.migrate("thing", &["CREATE TABLE a (x)", "CREATE TABLE b (x)"]).await.unwrap();
    let err = db.migrate("thing", &["CREATE TABLE a (x)"]).await.unwrap_err();
    assert!(err.message.contains("newer than this build understands"), "{}", err.message);
}

#[tokio::test]
async fn measurements_roll_up_and_answer_over_a_window() {
    let (journal, clock) = journal().await;
    let labels = [("action", "shell"), ("blueprint", "kv-cache")];
    for (ms, ok) in [(100.0, true), (200.0, true), (400.0, false), (90_000.0, true)] {
        journal.measure("step", &labels, ms, ok);
    }
    journal.measure("step", &[("action", "wait"), ("blueprint", "kv-cache")], 5.0, true);
    journal.flush_metrics().await.unwrap();
    clock.advance(Duration::from_secs(120));
    // Not flushed yet: a query must still see it.
    journal.measure("step", &labels, 300.0, true);

    let since_ms = journal.now_ms() - 3_600_000;
    let all = journal
        .query_metrics(MetricQuery { name: "step".into(), since_ms, ..Default::default() })
        .await
        .unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].total.count, all[0].total.errors), (6, 1));

    let by_action = journal
        .query_metrics(MetricQuery {
            name: "step".into(),
            since_ms,
            group_by: vec!["action".into()],
            step_minutes: 1,
            ..Default::default()
        })
        .await
        .unwrap();
    let shell = by_action.iter().find(|series| series.labels["action"] == "shell").unwrap();
    assert_eq!(shell.total.count, 5);
    assert_eq!(shell.total.error_rate, 0.2);
    assert_eq!((shell.total.min, shell.total.max), (100.0, 90_000.0));
    assert!(
        (256.0..=512.0).contains(&shell.total.p50),
        "median of 100,200,300,400,90000: {}",
        shell.total.p50
    );
    assert_eq!(shell.total.p95, 90_000.0);
    assert_eq!(shell.points.len(), 2, "two minutes had samples; the minute between has no point");
    assert_eq!(shell.points[1].0 - shell.points[0].0, 120_000);

    let only_wait = journal
        .query_metrics(MetricQuery {
            name: "st*".into(),
            since_ms,
            labels: vec![("action".into(), "wait".into())],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(only_wait[0].total.count, 1);

    let recent = journal
        .query_metrics(MetricQuery {
            name: "step".into(),
            since_ms: journal.now_ms() - 30_000,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(recent[0].total.count, 1, "the window excludes the older minute");
}

#[tokio::test]
async fn a_measure_is_split_by_one_label_and_narrowed_by_another_and_the_labels_there_are_are_listed() {
    let (journal, clock) = journal().await;
    let turn = |slot: &'static str, experiment: &'static str, variant: &'static str| {
        [("slot", slot), ("kind", "task"), ("experiment", experiment), ("variant", variant)]
    };
    journal.measure("agent.turn", &turn("ada", "cart", "a"), 1_000.0, true);
    journal.measure("agent.turn", &turn("ada", "cart", "a"), 3_000.0, true);
    journal.measure("agent.turn", &turn("grace", "cart", "b"), 8_000.0, false);
    journal.measure("agent.turn", &turn("grace", "search", "b"), 500.0, true);
    journal.measure("agent.turn", &[("slot", "ada"), ("kind", "message")], 100.0, true);
    journal.flush_metrics().await.unwrap();
    // Not flushed: what is listed and counted includes it all the same.
    journal.measure("step", &[("step", "work"), ("experiment", "cart"), ("variant", "c")], 10.0, true);
    let since_ms = journal.now_ms() - 3_600_000;

    let by = |group: &[&str], only: &[(&str, &str)]| MetricQuery {
        name: "agent.turn".into(),
        since_ms,
        group_by: group.iter().map(|key| key.to_string()).collect(),
        labels: only.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
        ..Default::default()
    };
    let cart = journal.query_metrics(by(&["variant"], &[("experiment", "cart")])).await.unwrap();
    let found: Vec<(&str, u64, u64, f64)> = cart
        .iter()
        .map(|series| {
            (series.labels["variant"].as_str(), series.total.count, series.total.errors, series.total.avg)
        })
        .collect();
    assert_eq!(found, [("a", 2, 0, 2_000.0), ("b", 1, 1, 8_000.0)]);

    // What has no such label is one row with no value for it, as it always was.
    let all = journal.query_metrics(by(&["variant"], &[])).await.unwrap();
    let counts: Vec<(Option<&str>, u64)> = all
        .iter()
        .map(|series| (series.labels.get("variant").map(String::as_str), series.total.count))
        .collect();
    assert_eq!(counts, [(None, 1), (Some("a"), 2), (Some("b"), 2)]);
    // And a question that names no label is answered as before there were any.
    let by_slot = journal.query_metrics(by(&["slot"], &[])).await.unwrap();
    let counts: Vec<(&str, u64)> =
        by_slot.iter().map(|series| (series.labels["slot"].as_str(), series.total.count)).collect();
    assert_eq!(counts, [("ada", 3), ("grace", 2)]);
    assert_eq!(journal.query_metrics(by(&[], &[])).await.unwrap()[0].total.count, 5);

    let names = |listed: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>| {
        listed
            .into_iter()
            .map(|(name, values)| format!("{name}: {}", values.into_iter().collect::<Vec<_>>().join(" ")))
            .collect::<Vec<_>>()
    };
    // The journal's own measurements of its database are in there too, under names of their own.
    let own = ["slot", "kind", "step", "db", "caller"];
    let everything = MetricQuery { name: "*".into(), since_ms, ..Default::default() };
    assert_eq!(
        names(journal.metric_labels(everything.clone(), &own).await.unwrap()),
        ["experiment: cart search", "variant: a b c"],
        "what a measurement says of itself is not a label"
    );
    let search = MetricQuery { labels: vec![("experiment".into(), "search".into())], ..everything.clone() };
    assert_eq!(
        names(journal.metric_labels(search, &own).await.unwrap()),
        ["experiment: search", "variant: b"],
        "narrowed, only what is there is listed"
    );
    clock.advance(Duration::from_secs(2 * 3600));
    let later = MetricQuery { since_ms: journal.now_ms() - 3_600_000, ..everything };
    assert!(journal.metric_labels(later, &own).await.unwrap().is_empty(), "nothing in this window");
}

/// A journal, its clock, and the database it writes to.
async fn kept() -> (Journal, ManualClock, Db) {
    let clock = ManualClock::new();
    let db = Db::memory().unwrap();
    let journal = Journal::open(db.clone(), Arc::new(clock.clone()), "dev").await.unwrap();
    (journal, clock, db)
}

/// The measurements that have been written, by name.
async fn written(db: &Db) -> Vec<String> {
    db.call(|connection| {
        let mut names = connection.prepare("SELECT DISTINCT name FROM metrics ORDER BY name")?;
        let names = names.query_map([], |row| row.get::<_, String>(0))?;
        names.collect()
    })
    .await
    .unwrap()
}

/// How many times something was measured, written or not.
async fn counted(journal: &Journal, name: &str) -> u64 {
    let query =
        MetricQuery { name: name.into(), since_ms: journal.now_ms() - 3_600_000, ..Default::default() };
    journal.query_metrics(query).await.unwrap().iter().map(|series| series.total.count).sum()
}

#[tokio::test]
async fn what_is_measured_quietly_is_read_at_once_and_written_with_the_next_thing_worth_a_write() {
    let (journal, _clock, db) = kept().await;
    journal.measure_quietly("device.load_per_cpu", &[("device", "dev")], 0.5, true);
    journal.flush_metrics_when_due().await.unwrap();
    assert_eq!(written(&db).await, Vec::<String>::new(), "nothing was worth a write");
    // It is there for whoever asks, as what is written is.
    assert_eq!(counted(&journal, "device.load_per_cpu").await, 1);

    // Something that is worth one takes what was waiting with it.
    journal.measure("step", &[("blueprint", "shop")], 12.0, true);
    journal.flush_metrics_when_due().await.unwrap();
    let names = written(&db).await;
    assert!(
        names.contains(&"step".to_string()) && names.contains(&"device.load_per_cpu".to_string()),
        "{names:?}"
    );
    assert_eq!(counted(&journal, "device.load_per_cpu").await, 1, "and it is counted once, not twice");
}

#[tokio::test]
async fn a_journal_at_rest_is_not_written_to_for_its_own_measurements() {
    // Writing measurements is a piece of the database's work, which is
    // measured. Taken as worth a write, each write was the reason for the
    // next, and a factory nobody was using wrote to its disk every ten
    // seconds for ever.
    let (journal, clock, db) = kept().await;
    journal.emit("build.started", Source::default(), json!({})).await.unwrap();
    journal.measure("step", &[("blueprint", "shop")], 12.0, true);
    journal.flush_metrics_when_due().await.unwrap();
    let rows = |db: &Db| {
        db.call(|connection| {
            connection.query_row("SELECT count(*) FROM metrics", [], |row| row.get::<_, i64>(0))
        })
    };
    let after_the_work = rows(&db).await.unwrap();
    assert!(after_the_work > 0);

    // A minute at rest: the timer comes round, and the device is sampled.
    for _ in 0..6 {
        clock.advance(Duration::from_secs(10));
        journal.measure_quietly("device.free_mem_mb", &[("device", "dev")], 4096.0, true);
        journal.flush_metrics_when_due().await.unwrap();
    }
    assert_eq!(rows(&db).await.unwrap(), after_the_work, "nothing was written while nothing happened");
    // What the database did meanwhile is still known.
    assert!(counted(&journal, "db.work").await > 0);
    assert_eq!(counted(&journal, "device.free_mem_mb").await, 6);
}

#[tokio::test]
async fn what_was_measured_quietly_is_written_by_itself_once_it_has_waited_an_hour() {
    let (journal, clock, db) = kept().await;
    journal.measure_quietly("device.free_mem_mb", &[("device", "dev")], 4096.0, true);
    clock.advance(Duration::from_secs(59 * 60));
    journal.flush_metrics_when_due().await.unwrap();
    assert!(!written(&db).await.contains(&"device.free_mem_mb".to_string()));
    clock.advance(Duration::from_secs(2 * 60));
    journal.flush_metrics_when_due().await.unwrap();
    assert!(written(&db).await.contains(&"device.free_mem_mb".to_string()));
}

#[tokio::test]
async fn what_was_measured_quietly_is_written_when_it_is_asked_for_outright_as_a_daemon_that_stops_asks() {
    let (journal, _clock, db) = kept().await;
    journal.measure_quietly("device.free_mem_mb", &[("device", "dev")], 4096.0, true);
    journal.flush_metrics().await.unwrap();
    assert!(written(&db).await.contains(&"device.free_mem_mb".to_string()));
}

#[tokio::test]
async fn a_piece_of_database_work_that_failed_is_worth_a_write() {
    let (journal, _clock, db) = kept().await;
    assert!(db.call(|connection| connection.execute("NOT A STATEMENT", [])).await.is_err());
    journal.flush_metrics_when_due().await.unwrap();
    assert!(written(&db).await.contains(&"db.work".to_string()));
}

#[tokio::test]
async fn flushing_twice_in_a_minute_adds_up() {
    let (journal, _) = journal().await;
    journal.measure("http", &[("route", "/api/events")], 10.0, true);
    journal.flush_metrics().await.unwrap();
    journal.measure("http", &[("route", "/api/events")], 30.0, false);
    journal.flush_metrics().await.unwrap();
    let series = journal
        .query_metrics(MetricQuery {
            name: "http".into(),
            since_ms: journal.now_ms() - 60_000,
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!((series[0].total.count, series[0].total.errors, series[0].total.avg), (2, 1, 20.0));
    assert!(
        journal
            .query_metrics(MetricQuery { name: "nothing".into(), ..Default::default() })
            .await
            .unwrap()
            .is_empty()
    );
}

#[test]
fn logs_append_read_and_tail() {
    let dir = scratch("logs");
    let logs = Logs::new(dir.to_path_buf());
    for n in 1..=250 {
        logs.append(
            "runs",
            "run_1/build",
            1_000 + n,
            if n % 2 == 0 { 'e' } else { 'o' },
            &format!("line {n}\n"),
        );
    }
    let first = logs.read("runs", "run_1/build", 0, 3);
    assert_eq!(first.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["line 1", "line 2", "line 3"]);
    assert_eq!((first[1].line, first[1].ts, first[1].stream), (2, 1_002, 'e'));

    let next = logs.read("runs", "run_1/build", 248, 100);
    assert_eq!(next.iter().map(|l| l.line).collect::<Vec<_>>(), [249, 250]);

    let tail = logs.tail("runs", "run_1/build", 2);
    assert_eq!(
        tail.iter().map(|l| (l.line, l.text.as_str())).collect::<Vec<_>>(),
        [(249, "line 249"), (250, "line 250")]
    );

    assert!(logs.read("runs", "nothing", 0, 10).is_empty());
    logs.remove("runs", "run_1/build");
    assert!(logs.tail("runs", "run_1/build", 10).is_empty());
}

#[test]
fn a_log_line_is_one_line_and_bounded() {
    let dir = scratch("bounded");
    let logs = Logs::new(dir.to_path_buf());
    logs.append("runs", "r", 1, 'o', "two\nlines");
    logs.append("runs", "r", 2, 'o', &"x".repeat(100_000));
    let lines = logs.read("runs", "r", 0, 10);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].text, "two\\nlines");
    assert_eq!(lines[1].text.len(), 16 * 1024);
}

#[test]
fn log_tail_numbers_lines_correctly_in_a_large_file() {
    let dir = scratch("large");
    let logs = Logs::new(dir.to_path_buf());
    let padding = "p".repeat(200);
    for n in 1..=2_000 {
        logs.append("svc", "kv", n, 'o', &format!("{n} {padding}"));
    }
    let tail = logs.tail("svc", "kv", 3);
    assert_eq!(tail.iter().map(|l| l.line).collect::<Vec<_>>(), [1_998, 1_999, 2_000]);
    assert!(tail[2].text.starts_with("2000 "));
}

/// A database that tells this test of every piece of work it does.
fn told() -> (Db, std::sync::mpsc::Receiver<Turn>) {
    let db = Db::memory().unwrap();
    let (tell, turns) = std::sync::mpsc::channel();
    db.measured(move |turn| {
        let _ = tell.send(turn);
    });
    (db, turns)
}

#[tokio::test]
async fn work_for_the_database_is_told_by_whose_it_was_and_whether_it_wrote() {
    let (db, turns) = told();
    db.call(|connection| connection.execute_batch("CREATE TABLE things (name TEXT)")).await.unwrap();
    db.call(|connection| connection.execute("INSERT INTO things VALUES ('one')", [])).await.unwrap();
    db.call(|connection| connection.query_row("SELECT count(*) FROM things", [], |row| row.get::<_, i64>(0)))
        .await
        .unwrap();

    let told: Vec<Turn> = turns.try_iter().collect();
    assert_eq!(told.len(), 3);
    // This file is kept under `crates/journal`, so what it asks for is the journal's.
    assert!(told.iter().all(|turn| turn.by == "journal" && turn.ok), "{told:?}");
    assert_eq!(
        told.iter().map(|turn| turn.wrote).collect::<Vec<_>>(),
        [false, true, false],
        "making a table changes no row"
    );
}

#[tokio::test]
async fn work_that_waits_behind_other_work_is_told_as_having_waited() {
    let (db, turns) = told();
    let (release, held) = std::sync::mpsc::channel::<()>();
    let (began, begun) = tokio::sync::oneshot::channel::<()>();
    // The first holds the thread until it is let go, and has begun
    // before the second is asked for: let go before the thread had come
    // to it, it held nothing, and which of two pieces of work that take
    // no time took longer was the machine's to say, once in 120 crowded.
    let first = db.call(move |_| {
        let _ = began.send(());
        let _ = held.recv();
        Ok(())
    });
    begun.await.unwrap();
    let asked = std::time::Instant::now();
    let second = db.call(|_| Ok(()));
    let held_for = asked.elapsed().as_secs_f64() * 1000.0;
    release.send(()).unwrap();
    first.await.unwrap();
    second.await.unwrap();

    let told: Vec<Turn> = turns.try_iter().collect();
    assert_eq!(told.len(), 2);
    // The second waited for at least as long as the first was held
    // after it was asked for, and the first took at least that long too.
    assert!(told[1].waited_ms > 0.0, "the second waited for the first: {told:?}");
    assert!(told[0].took_ms >= held_for, "the first is what took the time: {told:?}, held for {held_for}");
}

#[tokio::test]
async fn looking_for_what_is_not_there_is_not_a_failure_and_a_statement_that_does_not_read_is() {
    let (db, turns) = told();
    let missing =
        db.call(|connection| connection.query_row("SELECT 1 WHERE 0", [], |row| row.get::<_, i64>(0))).await;
    assert!(missing.is_err());
    let wrong = db.call(|connection| connection.execute("NOT A STATEMENT", [])).await;
    assert!(wrong.is_err());
    let told: Vec<Turn> = turns.try_iter().collect();
    assert_eq!(told.iter().map(|turn| turn.ok).collect::<Vec<_>>(), [true, false]);
}

#[tokio::test]
async fn the_journal_keeps_how_long_its_database_made_work_wait_and_how_long_the_work_took() {
    let (journal, _clock) = journal().await;
    journal.emit("build.started", Source::default(), json!({})).await.unwrap();
    journal.read(EventQuery::default()).await.unwrap();

    let since_ms = journal.now_ms() - 60_000;
    for name in ["db.wait", "db.work"] {
        let by = journal
            .query_metrics(MetricQuery {
                name: name.into(),
                since_ms,
                group_by: vec!["kind".into()],
                ..Default::default()
            })
            .await
            .unwrap();
        let did: Vec<&str> = by.iter().map(|series| series.labels["kind"].as_str()).collect();
        assert_eq!(did, ["read", "write"], "{name} is kept for what read and for what wrote");
        let all = journal
            .query_metrics(MetricQuery {
                name: name.into(),
                labels: vec![("db".into(), "journal".into()), ("caller".into(), "journal".into())],
                since_ms,
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(all[0].total.count >= 2, "{name}: {all:?}");
        assert_eq!(all[0].total.errors, 0);
    }
}

#[tokio::test]
async fn a_journal_that_is_held_to_something_keeps_what_was_found_wrong_and_emits_all_the_same() {
    let (journal, _) = journal().await;
    // Nothing is held to anything until it is said.
    journal.emit("thing.odd", Source::actor("system"), json!({})).await.unwrap();
    assert!(journal.faults().is_empty());

    journal.hold(Arc::new(|topic: &str, data: &serde_json::Value| {
        (topic == "thing.odd").then(|| format!("`{topic}` is odd, with {data}"))
    }));
    journal.emit("thing.fine", Source::actor("system"), json!({})).await.unwrap();
    let odd = journal.emit("thing.odd", Source::actor("system"), json!({ "n": 1 })).await.unwrap();
    journal.emit("thing.odd", Source::actor("system"), json!({ "n": 1 })).await.unwrap();
    journal.emit_ephemeral("thing.odd", Source::actor("system"), json!({ "n": 2 }));
    // What happened is kept whatever is thought of it, and each thing found is said once.
    assert_eq!(odd.topic, "thing.odd");
    assert_eq!(
        journal.faults(),
        vec![
            "`thing.odd` is odd, with {\"n\":1}".to_string(),
            "`thing.odd` is odd, with {\"n\":2}".to_string()
        ]
    );
    // What it is held to is said once; a second is not taken.
    journal.hold(Arc::new(|_: &str, _: &serde_json::Value| Some("everything".to_string())));
    journal.emit("thing.fine", Source::actor("system"), json!({})).await.unwrap();
    assert_eq!(journal.faults().len(), 2);
}

#[tokio::test]
async fn what_was_said_in_something_taken_away_is_taken_out_of_the_journal_and_nothing_else_is() {
    let (journal, _) = journal().await;
    let said = |room: &str, words: &str| json!({ "room": room, "words": words });
    journal.emit("room.said", Source::actor("ada"), said("r_1", "first")).await.unwrap();
    journal.emit("room.said", Source::actor("ada"), said("r_2", "elsewhere")).await.unwrap();
    // Words that quote what is looked for are not the field itself.
    journal
        .emit("room.said", Source::actor("ada"), said("r_2", r#"they wrote "room":"r_1" here"#))
        .await
        .unwrap();
    journal.emit("room.said", Source::actor("ada"), said("r_1", "second")).await.unwrap();
    // Another topic that names the same thing says something else of it.
    journal.emit("room.opened", Source::actor("ada"), json!({ "room": "r_1" })).await.unwrap();

    assert_eq!(journal.forget_events("room.said", "room", "r_1").await.unwrap(), 2);
    let left = journal.read(EventQuery { limit: 100, ..EventQuery::default() }).await.unwrap();
    let left: Vec<(&str, &str)> = left
        .iter()
        .map(|event| (event.topic.as_str(), event.data["words"].as_str().unwrap_or_default()))
        .collect();
    assert_eq!(
        left,
        [("room.said", "elsewhere"), ("room.said", r#"they wrote "room":"r_1" here"#), ("room.opened", "")]
    );
    // What is said next is numbered on from where the journal was.
    let next = journal.emit("room.said", Source::actor("ada"), said("r_2", "after")).await.unwrap();
    assert_eq!(next.seq, 6);
}

#[tokio::test]
async fn a_database_that_is_closed_does_what_was_asked_before_and_refuses_what_is_asked_after() {
    let db = Db::memory().unwrap();
    db.call(|connection| connection.execute_batch("CREATE TABLE kept (word TEXT)")).await.unwrap();
    // Asked before it is closed, and not yet answered.
    let before = db.call(|connection| connection.execute("INSERT INTO kept VALUES ('asked before')", []));
    db.close();
    let after = db
        .call(|connection| connection.query_row("SELECT count(*) FROM kept", [], |row| row.get::<_, i64>(0)));

    assert_eq!(before.await.unwrap(), 1);
    let refused = after.await.unwrap_err();
    assert!(refused.message.contains("the database thread has stopped"), "{refused:?}");
    // Its thread has ended: one that asks later still is told the same at once.
    let later = db.call(|connection| connection.execute_batch("SELECT 1")).await.unwrap_err();
    assert!(later.message.contains("the database thread has stopped"), "{later:?}");
}
