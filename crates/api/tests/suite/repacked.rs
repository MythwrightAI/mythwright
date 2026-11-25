//! A factory made by an earlier program goes on with the database it has,
//! exactly as it was made, and is given the one a factory is made with now
//! when its owner asks (`api::repack`): while it is stopped, with its
//! folder held as a daemon holds it, and said by the daemon that next
//! starts.

use std::path::{Path, PathBuf};

use journal::EventQuery;

use crate::Rig;

/// The journal's part of a database as the first programs made it: pages
/// of the size a database usually has, and events the database numbers,
/// of which five were said and the last two taken out.
const AS_IT_WAS: &str = "
    PRAGMA page_size = 4096;
    PRAGMA journal_mode = WAL;
    CREATE TABLE schema_versions (namespace TEXT PRIMARY KEY, version INTEGER NOT NULL);
    CREATE TABLE events (
        seq INTEGER PRIMARY KEY AUTOINCREMENT, ts INTEGER NOT NULL, topic TEXT NOT NULL, device TEXT,
        instance TEXT, run TEXT, step TEXT, actor TEXT, data TEXT NOT NULL
    );
    CREATE INDEX events_topic ON events (topic, seq);
    CREATE INDEX events_instance ON events (instance, seq) WHERE instance IS NOT NULL;
    CREATE INDEX events_run ON events (run, seq) WHERE run IS NOT NULL;
    CREATE INDEX events_ts ON events (ts);
    CREATE TABLE kv (
        namespace TEXT NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY (namespace, key)
    ) WITHOUT ROWID;
    CREATE TABLE metrics (
        minute INTEGER NOT NULL, name TEXT NOT NULL, labels TEXT NOT NULL, count INTEGER NOT NULL,
        errors INTEGER NOT NULL, sum REAL NOT NULL, min REAL NOT NULL, max REAL NOT NULL, hist TEXT NOT NULL,
        PRIMARY KEY (minute, name, labels)
    ) WITHOUT ROWID;
    CREATE INDEX metrics_name ON metrics (name, minute);
    ALTER TABLE events ADD COLUMN takes TEXT;
    INSERT INTO schema_versions (namespace, version) VALUES ('journal', 2);
    INSERT INTO events (ts, topic, device, data) VALUES
        (10, 'before.said', 'dev', '{\"n\":1}'), (11, 'before.said', 'dev', '{\"n\":2}'),
        (12, 'before.said', 'dev', '{\"n\":3}'), (13, 'before.gone', 'dev', '{}'), (14, 'before.gone', 'dev', '{}');
    DELETE FROM events WHERE seq > 3;
";

/// How the node's database is made: the size of its pages, and whether
/// the database numbers its events.
async fn made(rig: &Rig) -> (i64, bool) {
    rig.node()
        .app
        .journal
        .db()
        .call(|connection| {
            let page = connection.query_row("PRAGMA page_size", [], |row| row.get(0))?;
            let numbered = journal::numbered::by_the_database(connection, "main")?;
            Ok((page, numbered))
        })
        .await
        .unwrap()
}

/// What was said before, as the journal now reads it, and the number of
/// the newest thing said.
async fn said_before(rig: &Rig) -> (Vec<(u64, i64)>, u64) {
    let journal = &rig.node().app.journal;
    let query = EventQuery { topics: "before.said".into(), ..Default::default() };
    let before = journal.read(query).await.unwrap();
    (before.iter().map(|event| (event.seq, event.data["n"].as_i64().unwrap())).collect(), journal.last_seq())
}

#[tokio::test]
async fn a_factory_from_before_goes_on_as_it_is_and_is_given_the_database_made_now_when_its_owner_asks() {
    let mut data = PathBuf::new();
    let mut rig = Rig::with("repacked", |config, _| {
        std::fs::create_dir_all(&config.data_dir).unwrap();
        let connection = journal::copy::Connection::open(config.data_dir.join("journal.db")).unwrap();
        connection.execute_batch(AS_IT_WAS).unwrap();
        data = config.data_dir.clone();
    })
    .await;

    // Started by this program, it is as the one before made it, with what
    // it held, and nothing it says is given a number that was given before.
    assert_eq!(made(&rig).await, (4096, true));
    let (before, newest) = said_before(&rig).await;
    assert_eq!(before, [(1, 1), (2, 2), (3, 3)]);
    assert!(newest > 5, "five were said before, two of them since taken out: {newest}");
    let started = rig.event("system.started", |_| true).await;
    assert!(started.seq > 5, "{}", started.seq);
    // The one thing done to it by being started on: its index of when was
    // made anew, which the daemon says once, with how long it took.
    let reindexed = rig.event("system.reindexed", |_| true).await;
    assert!(reindexed.data["took_ms"].is_u64(), "{}", reindexed.data);

    // While it runs it is not touched, and is not even looked at. (Asked
    // off the test's own thread: it waits a moment for the folder, as a
    // node that is starting does, before it says so.)
    let running = data.clone();
    let refused =
        tokio::task::spawn_blocking(move || api::repack::run(&running, |_| panic!("nothing is looked at")))
            .await
            .unwrap()
            .unwrap_err();
    assert!(
        refused.message.contains("is running") && refused.message.contains("down"),
        "{}",
        refused.message
    );
    assert_eq!(made(&rig).await, (4096, true));

    rig.stop().await;
    let asked = api::repack::look(&data).unwrap();
    assert!(asked.plan.wanted() && asked.plan.numbered && asked.plan.page == 4096);
    assert_eq!(asked.refused(), None, "there is the room for a database of a few pages");
    // (How big it is with its log is not said twice the same: looking
    // at it folds the log back.)
    let mut shown = None;
    let done = api::repack::run(&data, |asked| shown = Some((asked.plan.page, asked.plan.numbered)))
        .unwrap()
        .expect("it was done");
    assert_eq!(shown, Some((4096, true)), "what it would take was said before it began");
    assert!(done.events >= 4, "what was said before, and what this program said: {}", done.events);
    assert!(leftovers(&data).is_empty(), "nothing is left beside it: {:?}", leftovers(&data));
    // Asked again there is nothing to do, and nothing is done.
    assert!(!api::repack::look(&data).unwrap().plan.wanted());
    assert_eq!(api::repack::run(&data, |_| {}).unwrap(), None);

    // The daemon that next starts says it was done, once, and goes on
    // with everything as it was, in a database as one is made now.
    rig.start().await;
    let said = rig.event("system.repacked", |_| true).await;
    assert_eq!(said.data["events"], done.events);
    assert_eq!((&said.data["before"], &said.data["after"]), (&done.before.into(), &done.after.into()));
    assert_eq!(made(&rig).await, (2048, false));
    let (after, later) = said_before(&rig).await;
    assert_eq!(after, before);
    assert!(later > newest, "it goes on numbering above all it ever said: {later} after {newest}");
    rig.restart().await;
    assert_eq!(rig.count_of("system.repacked").await, 1);
    assert_eq!(rig.count_of("system.reindexed").await, 1);
    assert_eq!(made(&rig).await, (2048, false));
}

/// What is in a factory's folder that is the database's and is not the
/// database: a log, or something written beside it.
fn leftovers(data: &Path) -> Vec<String> {
    std::fs::read_dir(data)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .filter(|name| name.starts_with("journal.db") && name != "journal.db")
        .collect()
}

#[tokio::test]
async fn a_factory_made_now_has_nothing_to_be_written_out_again() {
    let mut data = PathBuf::new();
    let mut rig = Rig::with("repacked-new", |config, _| data = config.data_dir.clone()).await;
    assert_eq!(made(&rig).await, (2048, false));
    rig.stop().await;
    let asked = api::repack::look(&data).unwrap();
    assert!(!asked.plan.wanted());
    assert_eq!(api::repack::run(&data, |_| {}).unwrap(), None);
    // And a folder that is no factory's is not made one by being asked about.
    let elsewhere = rig.dir.join("elsewhere");
    assert!(api::repack::look(&elsewhere).unwrap_err().message.contains("there is no factory"));
    assert!(!elsewhere.exists());
    // Nothing was done to its journal by being started on, and nothing is said.
    rig.start().await;
    assert_eq!((rig.count_of("system.reindexed").await, rig.count_of("system.repacked").await), (0, 0));
}

#[tokio::test]
async fn the_search_index_is_made_again_when_its_file_is_taken_away_while_the_factory_is_stopped() {
    // `repack` leaves the search index's file as it is, and says that it
    // can be taken away: what it holds is read again from what there is.
    let mut data = PathBuf::new();
    let rig = Rig::with("repacked-search", |config, _| data = config.data_dir.clone()).await;
    let mut rig = Rig::owned(rig).await;
    rig.send("/api/projects", serde_json::json!({ "name": "Shop" })).await;
    let search = |rig: &Rig| rig.node().app.studio.search.clone();
    search(&rig).catch_up().await.unwrap();
    let before = search(&rig).status().await.docs;
    assert!(before > 0, "a project's own notes can be searched");
    let mark = rig.mark();

    rig.stop().await;
    for ending in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(data.join(format!("search.db{ending}")));
    }
    rig.start().await;
    rig.event("search.indexed", move |event| event.seq > mark).await;
    search(&rig).catch_up().await.unwrap();
    assert_eq!(search(&rig).status().await.docs, before, "everything that could be searched can be again");
    let page: i64 = journal::copy::Connection::open(data.join("search.db"))
        .unwrap()
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .unwrap();
    assert_eq!(page, 2048, "in a file made as one is made now");
}

#[tokio::test]
async fn what_a_repack_that_was_cut_off_left_is_taken_away_as_the_daemon_starts_and_said() {
    let mut data = PathBuf::new();
    let mut rig = Rig::with("repacked-cut", |config, _| data = config.data_dir.clone()).await;
    rig.stop().await;
    // Cut off: a copy of the database as far as it had been written.
    let left = data.join("journal.db.rewriting");
    std::fs::write(&left, vec![7u8; 5000]).unwrap();

    rig.start().await;
    assert!(!left.exists(), "nothing is left beside the database");
    let said = rig.event("system.repack_cleared", |_| true).await;
    assert_eq!(said.data["bytes"], 5000);
    rig.restart().await;
    assert_eq!(rig.count_of("system.repack_cleared").await, 1, "there was nothing more to take away");
}

#[tokio::test]
async fn pruning_that_fails_is_said_and_not_kept_quiet() {
    let clock = kernel::ManualClock::new();
    let ticking = clock.clone();
    let rig =
        Rig::with("repacked-prune", move |config, _| config.clock = Some(std::sync::Arc::new(ticking))).await;
    // A journal whose index of when is not there: what is old in it
    // cannot be found, at this hour or any other.
    rig.node()
        .app
        .journal
        .db()
        .call(|connection| connection.execute_batch("DROP INDEX events_when;"))
        .await
        .unwrap();
    let said = rig.event("system.degraded", |event| {
        event.data["what"].as_str().is_some_and(|what| what.contains("old events"))
    });
    tokio::pin!(said);
    let said = crate::soon("the daemon to say that old events are no longer taken out", async {
        loop {
            tokio::select! {
                event = &mut said => break event,
                // An hour goes by, as often as it takes for the daemon
                // to have been waiting when it did.
                () = tokio::task::yield_now() => clock.advance(std::time::Duration::from_secs(3600)),
            }
        }
    })
    .await;
    assert!(said.data["reason"].as_str().unwrap().contains("events_when"), "{}", said.data);
    assert!(said.data["effect"].as_str().unwrap().contains("goes on growing"), "{}", said.data);
}

/// A card of a factory from before is moved with nobody told as the daemon
/// goes, and the move is said when the daemon is back: by the rows
/// written about when it stopped and the last thing the journal held when
/// it was opened, on the journal as the program before made it or, where
/// `rewritten`, written out again while it was stopped.
async fn a_move_nobody_was_told_of_is_said_on_a_factory_from_before(rewritten: bool) {
    let mut data = PathBuf::new();
    let rig = Rig::with("repacked-unsaid", |config, _| {
        std::fs::create_dir_all(&config.data_dir).unwrap();
        let connection = journal::copy::Connection::open(config.data_dir.join("journal.db")).unwrap();
        connection.execute_batch(AS_IT_WAS).unwrap();
        data = config.data_dir.clone();
    });
    let mut rig = Rig::owned(rig.await).await;
    rig.send("/api/projects", serde_json::json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    let id = crate::fleets::add(&rig, "Add a cart", &[]).await;
    {
        // Written to its row, and the daemon goes before it is said.
        let collections = &rig.node().app.studio.collections;
        let _turn = collections.changing(&id).await;
        let (board, mut record) = collections.record(&id).await.unwrap();
        record.data.insert("status".into(), serde_json::json!("ready"));
        collections.put(&board, record).await.unwrap();
    }
    rig.stop().await;
    if rewritten {
        api::repack::run(&data, |_| {}).unwrap().expect("it was written out again");
    }
    rig.start().await;
    assert_eq!(made(&rig).await, if rewritten { (2048, false) } else { (4096, true) });

    let look = serde_json::json!({
        "blueprint": "std/owed", "project": "shop", "params": { "project": "shop" }, "wait": true,
    });
    let looked = rig.send("/api/apply", look).await;
    assert_eq!(looked["finished"]["status"], "ok", "{looked}");
    let query = EventQuery { topics: "board.item.moved".into(), limit: 1000, ..Default::default() };
    let said = rig.said(query).await;
    let late: Vec<_> = said.iter().filter(|event| event.data["id"] == id.as_str()).collect();
    assert_eq!(late.len(), 1, "the move is said once, late: {late:?}");
    assert_eq!(
        (&late[0].data["to"], &late[0].data["late"]),
        (&serde_json::json!("ready"), &serde_json::json!(true))
    );
}

#[tokio::test]
async fn a_move_nobody_was_told_of_is_said_on_a_journal_from_before_as_it_is() {
    a_move_nobody_was_told_of_is_said_on_a_factory_from_before(false).await;
}

#[tokio::test]
async fn a_move_nobody_was_told_of_is_said_on_a_journal_that_was_written_out_again() {
    a_move_nobody_was_told_of_is_said_on_a_factory_from_before(true).await;
}
