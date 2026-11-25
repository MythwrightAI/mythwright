//! How a journal's database is made, and what becomes of one made by an
//! earlier program: events the journal numbers itself and small pages for
//! one made now; one from before carried on exactly as it is, never giving
//! a number twice, but for the indexes of its events by instance and by
//! run, which go where it holds few; and the same written out again as
//! one made now, when its owner asks and only then.

use std::sync::Arc;

use journal::giver::RUN;
use journal::{Db, EventQuery, Journal};
use kernel::{Clock, ManualClock, Source};
use serde_json::json;

use crate::scratch;

#[tokio::test]
async fn a_new_database_has_small_pages_and_one_from_before_keeps_the_pages_it_has() {
    let size = |db: Db| async move {
        db.call(|connection| connection.query_row("PRAGMA page_size", [], |row| row.get::<_, i64>(0)))
            .await
            .unwrap()
    };
    let dir = scratch("pages");
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new());
    let path = dir.join("journal.db");
    let journal = Journal::open(Db::open(&path).unwrap(), clock.clone(), "dev").await.unwrap();
    assert_eq!(size(journal.db().clone()).await, 2048);

    // One made with the pages a database usually has is opened as it is.
    let before = dir.join("before.db");
    {
        let connection = journal::copy::Connection::open(&before).unwrap();
        connection.execute_batch("PRAGMA page_size = 4096; CREATE TABLE made (at INTEGER);").unwrap();
    }
    let journal = Journal::open(Db::open(&before).unwrap(), clock, "dev").await.unwrap();
    journal.emit("said", Source::default(), json!({})).await.unwrap();
    assert_eq!(size(journal.db().clone()).await, 4096);
}

#[tokio::test]
async fn an_events_number_is_never_given_twice_though_the_last_were_taken_out_and_the_journal_opened_again() {
    let dir = scratch("numbered");
    let path = dir.join("journal.db");
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new());
    {
        let journal = Journal::open(Db::open(&path).unwrap(), clock.clone(), "dev").await.unwrap();
        journal.emit("kept", Source::default(), json!({})).await.unwrap();
        for _ in 0..3 {
            journal.emit("room.said", Source::default(), json!({ "room": "r_1" })).await.unwrap();
        }
        // Somebody kept their place at the last of them.
        assert_eq!(journal.last_seq(), 4);
        assert_eq!(journal.forget_events("room.said", "room", "r_1").await.unwrap(), 3);
        assert_eq!(journal.emit("after", Source::default(), json!({})).await.unwrap().seq, 5);
        assert_eq!(journal.forget_events("after", "", "").await.unwrap(), 0);
    }
    {
        // Nothing was said after the last were taken out, and it is opened
        // again: what it goes on with is above all it had drawn, the rest
        // of which is never given.
        let journal = Journal::open(Db::open(&path).unwrap(), clock.clone(), "dev").await.unwrap();
        assert_eq!(journal.last_seq(), 5);
        journal
            .db()
            .call(|connection| connection.execute("DELETE FROM events WHERE seq = 5", []))
            .await
            .unwrap();
        let said = journal.emit("room.said", Source::default(), json!({ "room": "r_2" })).await.unwrap();
        assert_eq!(said.seq, RUN + 1);
        assert_eq!(journal.forget_events("room.said", "room", "r_2").await.unwrap(), 1);
    }
    let journal = Journal::open(Db::open(&path).unwrap(), clock, "dev").await.unwrap();
    assert_eq!(journal.last_seq(), RUN + 1, "the highest there ever was, though the events end at 1");
    assert_eq!(journal.held_when_opened(), 1, "and what it held when it was opened is its last event");
    assert_eq!(journal.emit("next", Source::default(), json!({})).await.unwrap().seq, 2 * RUN + 1);
}

/// The journal's tables as the first programs made them: pages of the size
/// a database usually has, and events numbered by the database, of which
/// five were said and the last two taken out. With them, a table of
/// another crate's, as every crate keeps its own in the one database.
const AS_IT_WAS: &str = "
    PRAGMA page_size = 4096;
    PRAGMA journal_mode = WAL;
    CREATE TABLE schema_versions (namespace TEXT PRIMARY KEY, version INTEGER NOT NULL);
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
    ALTER TABLE events ADD COLUMN takes TEXT;
    INSERT INTO schema_versions (namespace, version) VALUES ('journal', 2), ('another', 1);
    INSERT INTO events (ts, topic, device, instance, run, step, actor, data, takes) VALUES
        (10, 'run.started', 'dev', 'i_1', 'r_1', NULL, 'user:ada', '{\"flow\":\"main\"}', NULL),
        (11, 'step.started', 'dev', 'i_1', 'r_1', 'build', NULL, '{}', 'private'),
        (12, 'run.finished', 'dev', 'i_1', 'r_1', NULL, NULL, '{}', NULL),
        (13, 'room.said', 'dev', NULL, NULL, NULL, NULL, '{}', NULL),
        (14, 'room.said', 'dev', NULL, NULL, NULL, NULL, '{}', NULL);
    DELETE FROM events WHERE seq > 3;
    INSERT INTO kv (namespace, key, value) VALUES ('cursor', 'trigger-1', '{\"seq\":3}');
    CREATE TABLE another (id TEXT PRIMARY KEY, held INTEGER NOT NULL, said TEXT) WITHOUT ROWID;
    CREATE INDEX another_held ON another (held);
    INSERT INTO another (id, held, said) VALUES ('a', 1, 'one'), ('b', 2, NULL), ('c', 2, 'three');
";

/// A database as the first programs made it, at `path`.
fn from_before(path: &std::path::Path) {
    let connection = journal::copy::Connection::open(path).unwrap();
    connection.execute_batch(AS_IT_WAS).unwrap();
}

/// How the database at the journal's hand is made: the size of its pages,
/// and everything it says of the events table and what is found by.
async fn made(journal: &Journal) -> (i64, String) {
    journal
        .db()
        .call(|connection| {
            let page = connection.query_row("PRAGMA page_size", [], |row| row.get(0))?;
            let mut statement = connection.prepare(
                "SELECT name || ': ' || sql FROM sqlite_master WHERE tbl_name = 'events' ORDER BY name",
            )?;
            let said: Vec<String> =
                statement.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
            Ok((page, said.join("\n")))
        })
        .await
        .unwrap()
}

/// That the events are found by topic and by when, and by nothing else
/// (`journal::events`): what [`made`] says of a journal made now.
fn kept_by_topic_and_when(said: &str) {
    for index in ["events_topic", "events_when"] {
        assert!(said.contains(index), "{index} is there:\n{said}");
    }
    for index in ["events_instance", "events_run"] {
        assert!(!said.contains(index), "{index} is not there:\n{said}");
    }
}

/// What a journal from before held, read as a journal reads it.
async fn held_as_before(journal: &Journal) {
    let before = journal.read(EventQuery { before: 4, ..Default::default() }).await.unwrap();
    assert_eq!(
        before.iter().map(|event| event.seq).collect::<Vec<_>>(),
        [1, 2, 3],
        "its events are as they were"
    );
    assert_eq!(before[0].data, json!({ "flow": "main" }));
    assert_eq!(before[0].source.actor.as_deref(), Some("user:ada"));
    assert_eq!((before[1].ts, before[1].source.step.as_deref()), (11, Some("build")));
    assert_eq!(before[1].source.takes.as_deref(), Some("private"));
    let of_run = EventQuery { run: Some("r_1".into()), ..Default::default() };
    assert_eq!(journal.read(of_run).await.unwrap().len(), 3, "and found as they were");
    let by_topic = EventQuery { topics: "run.*".into(), after: 1, ..Default::default() };
    let found: Vec<u64> = journal.read(by_topic).await.unwrap().iter().map(|event| event.seq).collect();
    assert_eq!(found, [3], "by what they were about, and from a place");
    let newest =
        EventQuery { instance: Some("i_1".into()), newest_first: true, limit: 1, ..Default::default() };
    assert_eq!(journal.read(newest).await.unwrap()[0].topic, "run.finished", "and by whose they were");
    assert_eq!(journal.kv_get("cursor", "trigger-1").await.unwrap(), Some(json!({ "seq": 3 })));
    let another: Vec<(String, i64, Option<String>)> = journal
        .db()
        .call(|connection| {
            let mut statement =
                connection.prepare("SELECT id, held, said FROM another WHERE held = 2 ORDER BY id")?;
            statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?.collect()
        })
        .await
        .unwrap();
    assert_eq!(another, [("b".to_string(), 2, None), ("c".to_string(), 2, Some("three".to_string()))]);
}

#[tokio::test]
async fn a_journal_made_now_numbers_its_own_events_in_a_table_the_database_does_not() {
    let dir = scratch("made-now");
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new());
    let journal = Journal::open(Db::open(&dir.join("journal.db")).unwrap(), clock, "dev").await.unwrap();
    let (_, said) = made(&journal).await;
    assert!(!said.contains("AUTOINCREMENT"), "{said}");
    kept_by_topic_and_when(&said);
    assert_eq!(journal.emit("first", Source::default(), json!({})).await.unwrap().seq, 1);
    assert_eq!(journal.emit("second", Source::default(), json!({})).await.unwrap().seq, 2);
    // Nothing was done to it that anyone need be told of.
    assert_eq!(journal.kv_get(journal::REINDEXED.0, journal::REINDEXED.1).await.unwrap(), None);
}

#[tokio::test]
async fn a_journal_from_before_is_carried_on_as_it_is_and_never_gives_a_number_twice() {
    let dir = scratch("before");
    let path = dir.join("journal.db");
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new());
    from_before(&path);
    {
        let journal = Journal::open(Db::open(&path).unwrap(), clock.clone(), "dev").await.unwrap();
        held_as_before(&journal).await;
        // Nothing of it was written out again by being opened: its pages
        // and its table are as the program before made them.
        let (page, said) = made(&journal).await;
        assert_eq!(page, 4096);
        assert!(said.contains("AUTOINCREMENT"), "{said}");
        // The two things done to it by being opened: its index of when is
        // the one a journal has now, which is noted with how long it took,
        // for whoever opened it to say once; and holding few events, it
        // keeps no index of them by instance or by run, as one made now.
        assert!(said.contains("events_when") && !said.contains("events_ts"), "{said}");
        kept_by_topic_and_when(&said);
        let noted = journal.kv_get(journal::REINDEXED.0, journal::REINDEXED.1).await.unwrap().expect("noted");
        assert!(noted["took_ms"].is_u64(), "{noted}");
        // Five were said and two taken out: the next is the sixth. It kept
        // no mark of how far its numbers were given, and none was written
        // by opening it: the first is kept with the first thing said.
        let mark = journal::giver::DRAWN;
        assert_eq!(journal.last_seq(), 5);
        assert_eq!(journal.kv_get(mark.0, mark.1).await.unwrap(), None);
        assert_eq!(journal.emit("next", Source::default(), json!({})).await.unwrap().seq, 6);
        assert_eq!(journal.kv_get(mark.0, mark.1).await.unwrap(), Some(json!(5 + RUN)));
        assert_eq!(
            journal.emit("room.said", Source::default(), json!({ "room": "r_9" })).await.unwrap().seq,
            7
        );
        assert_eq!(journal.forget_events("room.said", "room", "r_9").await.unwrap(), 1);
    }
    let journal = Journal::open(Db::open(&path).unwrap(), clock, "dev").await.unwrap();
    assert_eq!(journal.last_seq(), 7, "the last of them was taken out, and its number is not given again");
    assert_eq!(
        journal.held_when_opened(),
        6,
        "what it held when it was opened is the last event that is in it"
    );
    // It had drawn a run of numbers from the sixth on, as a journal made
    // now does, and goes on above the last of that run.
    assert_eq!(journal.emit("after", Source::default(), json!({})).await.unwrap().seq, 5 + RUN + 1);
    // Opened again it reads the same, nothing more is done to it, and
    // what was noted is as it was until it is said.
    held_as_before(&journal).await;
    assert!(journal.kv_get(journal::REINDEXED.0, journal::REINDEXED.1).await.unwrap().is_some());
    // What is old in it is found by the index it now has.
    assert_eq!(journal.prune_events(13, 2).await.unwrap(), 3);
    let left: Vec<u64> =
        journal.read(EventQuery::default()).await.unwrap().iter().map(|event| event.seq).collect();
    assert_eq!(left, [6, 5 + RUN + 1]);
}

#[tokio::test]
async fn a_journal_from_before_is_written_out_again_as_one_made_now_when_its_owner_asks() {
    let dir = scratch("rewritten");
    let path = dir.join("journal.db");
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new());
    from_before(&path);
    // What it would take is said first, and asking changes nothing.
    let plan = journal::rewrite::look(&path).unwrap();
    assert!(plan.wanted());
    assert_eq!((plan.page, plan.numbered), (4096, true));
    assert_eq!(
        plan.bytes,
        std::fs::metadata(&path).unwrap().len(),
        "its log was folded back when it was made"
    );
    assert!(plan.needs() > plan.bytes && plan.seconds() >= 1);
    assert_eq!(plan.refused(plan.needs()), None);
    let refused = plan.refused(1024).unwrap();
    assert!(
        refused.starts_with("there is not the room") && refused.ends_with("Nothing was changed"),
        "{refused}"
    );

    // Who may read it is its owner alone, say, and stays so.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let done = journal::rewrite::rewrite(&path).unwrap();
    assert_eq!(done.events, 3);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "what took its place is as closed as it was");
    }
    assert_eq!(done.after, std::fs::metadata(&path).unwrap().len());
    let beside: Vec<String> = std::fs::read_dir(&*dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(beside, ["journal.db"], "one file, and nothing left beside it");
    assert!(
        !journal::rewrite::look(&path).unwrap().wanted(),
        "it is as one made now, and there is nothing to do"
    );

    let journal = Journal::open(Db::open(&path).unwrap(), clock, "dev").await.unwrap();
    held_as_before(&journal).await;
    let (page, said) = made(&journal).await;
    assert_eq!(page, 2048);
    assert!(!said.contains("AUTOINCREMENT"), "{said}");
    kept_by_topic_and_when(&said);
    // The database's own count of them did not go, and what it was for did.
    assert_eq!(journal.last_seq(), 5);
    assert_eq!(journal.emit("next", Source::default(), json!({})).await.unwrap().seq, 6);
}

#[tokio::test]
async fn a_rewrite_that_was_cut_off_or_refused_leaves_the_database_as_it_was() {
    let dir = scratch("cut-off");
    let path = dir.join("journal.db");
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new());
    from_before(&path);
    let as_it_was = std::fs::read(&path).unwrap();
    // Cut off: what was being written beside it is there, half made.
    let beside = dir.join("journal.db.rewriting");
    std::fs::write(&beside, &as_it_was[..as_it_was.len() / 3]).unwrap();
    {
        let journal = Journal::open(Db::open(&path).unwrap(), clock.clone(), "dev").await.unwrap();
        held_as_before(&journal).await;
        assert_eq!(made(&journal).await.0, 4096);
        journal.db().close();
    }
    // Asked again, it is done, and what was left is not in the way.
    assert_eq!(journal::rewrite::rewrite(&path).unwrap().events, 3);
    assert!(!beside.exists());

    // What is not a factory's database is refused, and is not touched.
    let other = dir.join("other.db");
    {
        let connection = journal::copy::Connection::open(&other).unwrap();
        connection
            .execute_batch("CREATE TABLE notes (said TEXT); INSERT INTO notes VALUES ('kept');")
            .unwrap();
    }
    let before = std::fs::read(&other).unwrap();
    let refused = journal::rewrite::rewrite(&other).unwrap_err();
    assert!(refused.message.contains("is not a factory's database"), "{}", refused.message);
    assert_eq!(std::fs::read(&other).unwrap(), before);
    assert!(!dir.join("other.db.rewriting").exists());
    assert!(
        journal::rewrite::look(&dir.join("nothing.db")).unwrap_err().message.contains("there is no database")
    );
    let journal = Journal::open(Db::open(&path).unwrap(), clock, "dev").await.unwrap();
    held_as_before(&journal).await;
}

#[tokio::test]
async fn a_journal_from_before_whose_last_number_cannot_be_read_is_not_opened() {
    let dir = scratch("unread");
    let path = dir.join("journal.db");
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new());
    from_before(&path);
    {
        // What the database remembers of the numbers it gave is gone
        // from it. Its events end at 3, and it had given 5.
        let connection = journal::copy::Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "PRAGMA writable_schema = ON;
                 DELETE FROM sqlite_master WHERE name = 'sqlite_sequence';
                 PRAGMA writable_schema = OFF;",
            )
            .unwrap();
    }
    // Taken for nothing, the next event would be the fourth again.
    let refused = match Journal::open(Db::open(&path).unwrap(), clock, "dev").await {
        Ok(journal) => panic!("it was opened, and would go on from {}", journal.last_seq()),
        Err(refused) => refused,
    };
    assert!(
        refused.message.contains("the last number the database gave an event could not be read"),
        "{}",
        refused.message
    );
}

/// A place to read after is a number of the journal's, and one past any
/// there could be has nothing after it. Handed to the database as it was,
/// the greatest such number read as one below nought, and everything was
/// after it: whoever meant "nothing counts" by it was told everything did.
#[tokio::test]
async fn nothing_is_read_after_a_place_past_any_the_journal_could_come_to() {
    let (journal, _) = crate::journal().await;
    for topic in ["one", "two", "three"] {
        journal.emit(topic, kernel::Source::default(), json!({})).await.unwrap();
    }
    for after in [u64::MAX, u64::MAX - 1, i64::MAX as u64, i64::MAX as u64 + 1] {
        let read = journal.read(EventQuery { after, ..Default::default() }).await.unwrap();
        assert!(read.is_empty(), "after {after}: {read:?}");
    }
    let read = journal.read(EventQuery { after: 2, before: u64::MAX, ..Default::default() }).await.unwrap();
    assert_eq!(
        read.iter().map(|event| event.seq).collect::<Vec<_>>(),
        [3],
        "and all of it is before such a place"
    );
}

#[tokio::test]
async fn a_journal_from_before_that_holds_many_events_keeps_its_indexes_and_both_read_alike() {
    let dir = scratch("many");
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new());
    // Made as one from before was, with every index it had, and given
    // more events than are let go of the indexes by being opened.
    let many = dir.join("many.db");
    from_before(&many);
    let more = "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 20001)
                INSERT INTO events (ts, topic, device, instance, run, data)
                SELECT 20 + i, 'step.started', 'dev', 'i_' || (i % 7), 'r_' || (i % 50), '{}' FROM n";
    journal::copy::Connection::open(&many).unwrap().execute_batch(more).unwrap();
    let kept = Journal::open(Db::open(&many).unwrap(), clock.clone(), "dev").await.unwrap();
    let (_, said) = made(&kept).await;
    for index in ["events_instance", "events_run"] {
        assert!(said.contains(index), "{index} is there still:\n{said}");
    }
    // And one that keeps none finds the same: what reads by instance or
    // by run reads in order from where it is told to begin.
    let few = dir.join("few.db");
    std::fs::copy(&many, &few).unwrap();
    journal::copy::Connection::open(&few)
        .unwrap()
        .execute_batch("DROP INDEX events_instance; DROP INDEX events_run;")
        .unwrap();
    let unkept = Journal::open(Db::open(&few).unwrap(), clock, "dev").await.unwrap();
    for journal in [&kept, &unkept] {
        let of_run = EventQuery { run: Some("r_7".into()), after: 19_000, limit: 5, ..Default::default() };
        let found: Vec<u64> = journal.read(of_run).await.unwrap().iter().map(|event| event.seq).collect();
        assert_eq!(found, [19_012, 19_062, 19_112, 19_162, 19_212]);
        let of_instance = EventQuery {
            instance: Some("i_3".into()),
            topics: "step.started".into(),
            newest_first: true,
            limit: 2,
            ..Default::default()
        };
        let found: Vec<u64> =
            journal.read(of_instance).await.unwrap().iter().map(|event| event.seq).collect();
        assert_eq!(found, [20_000, 19_993]);
    }
}
