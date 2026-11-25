//! A journal an earlier program made: taken up as it is, with no table
//! written out again and no row of what had ended touched; its rows told
//! apart as surely as those written now; what was under way and each
//! handler's place found by project; and the same after it is written
//! out again when its owner asks, and in a copy.

use std::path::Path;
use std::sync::Arc;

use journal::{Db, Journal};
use kernel::{ManualClock, Principal};
use runs::{Place, Store};
use rusqlite::Connection;

use crate::{run, scratch, told_apart};

/// The runner's tables as the program before this one made them.
const BEFORE: &str = include_str!("../fixtures/before.sql");
/// The version that program left them at, and knew.
const KNEW: usize = 12;
/// How many runs of something long over there are, each with a step, in
/// a journal that has not kept much.
const ENDED: usize = 300;

/// What that program kept, written as it wrote it: something of the
/// project `shop` that is alive, with a child whose handler has a place
/// and a run under way; something that is nobody's, the same; and what
/// is over, of `shop` and of `yard`, with every run it ever had.
fn rows(connection: &Connection, ended: usize) -> rusqlite::Result<()> {
    let by = serde_json::to_string(&Principal::system()).unwrap();
    for (id, status, parent, project) in [
        ("shop-fleet", "active", None, Some("shop")),
        ("shop-fleet-stage", "active", Some("shop-fleet"), Some("shop")),
        ("shop-old", "stopped", None, Some("shop")),
        ("none-watch", "active", None, None),
        ("yard-done", "done", None, Some("yard")),
    ] {
        connection.execute(
            "INSERT INTO instances
                 (id, blueprint, revision, source, params, state, status, parent, key, project, principal,
                  created, updated)
             VALUES (?1, 'thing', 'r1', 'name: thing', '{}', '{}', ?2, ?3, ?1, ?4, ?5, 1, 1)",
            rusqlite::params![id, status, parent, project, by],
        )?;
        connection.execute(
            "INSERT INTO instance_versions
                 (instance, version, revision, source, params, actor, actor_name, applied, principal)
             VALUES (?1, 1, 'r1', 'name: thing', '{}', '', '', 1, ?2)",
            rusqlite::params![id, by],
        )?;
    }
    let ran = |id: &str, of: &str, flow: &str, status: &str| {
        connection.execute(
            "INSERT INTO runs (id, instance, flow, revision, status, event, started)
             VALUES (?1, ?2, ?3, 'r1', ?4, 'null', 2)",
            [id, of, flow, status],
        )?;
        let step = if status == "running" { "running" } else { "ok" };
        connection.execute(
            "INSERT INTO steps (run, step, status, attempt, started) VALUES (?1, 'hello', ?2, 1, 3)",
            [id, step],
        )
    };
    ran("shop-fleet.main", "shop-fleet", "main", "ok")?;
    ran("shop-fleet-stage.heard", "shop-fleet-stage", "on.heard", "running")?;
    ran("none-watch.heard", "none-watch", "on.heard", "running")?;
    ran("yard-done.main", "yard-done", "main", "ok")?;
    for n in 0..ended {
        ran(&format!("shop-old.{n}"), "shop-old", "on.heard", "ok")?;
    }
    connection.execute(
        "INSERT INTO step_elements (run, step, position, output, began)
         VALUES ('shop-fleet-stage.heard', 'hello', 0, NULL, 40)",
        [],
    )?;
    for (of, seq) in [("shop-fleet-stage", 41), ("none-watch", 17), ("shop-old", 9)] {
        connection.execute(
            "INSERT INTO trigger_cursors (instance, trigger, seq) VALUES (?1, 'heard', ?2)",
            rusqlite::params![of, seq],
        )?;
    }
    Ok(())
}

/// A journal at `path` as an earlier program left it: made with the
/// pages a database usually has, its tables at the version that program
/// knew, and what it kept in them, with `ended` runs long over.
pub(crate) async fn from_before(path: &Path, ended: usize) -> Journal {
    {
        let connection = Connection::open(path).unwrap();
        connection.execute_batch("PRAGMA page_size = 4096; CREATE TABLE made (at INTEGER);").unwrap();
    }
    let journal = opened(path).await;
    let made = journal.db().call(move |connection| {
        connection.execute_batch(BEFORE)?;
        connection.execute(
            "INSERT INTO schema_versions (namespace, version) VALUES ('conductor', ?1)",
            [KNEW as i64],
        )?;
        connection.execute("DROP TABLE made", [])?;
        rows(connection, ended)
    });
    made.await.unwrap();
    journal
}

pub(crate) async fn opened(path: &Path) -> Journal {
    Journal::open(Db::open(path).unwrap(), Arc::new(ManualClock::new()), "here").await.unwrap()
}

/// Closes a journal's database and waits for its file to be let go.
pub(crate) async fn closed(journal: Journal) {
    journal.db().close();
    journal.db().closed().await;
}

/// Where each table from before, and each index of one, is kept in the
/// file: one that was written out again would be somewhere else.
fn kept_at(connection: &Connection) -> rusqlite::Result<Vec<(String, i64)>> {
    let mut statement = connection.prepare(
        "SELECT name, rootpage FROM sqlite_master
         WHERE tbl_name IN ('instances', 'instance_versions', 'runs', 'steps', 'step_elements', 'trigger_cursors')
           AND rootpage > 0 AND name NOT IN ('alive_project')
         ORDER BY name",
    )?;
    statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect()
}

/// What is left in the database from before that nothing reads any more.
async fn left(journal: &Journal) -> Vec<String> {
    let found = journal.db().call(|connection| {
        let mut statement = connection.prepare(
            "SELECT name FROM sqlite_master
             WHERE name IN ('instances_status', 'runs_status', 'trigger_cursors') ORDER BY name",
        )?;
        statement.query_map([], |row| row.get(0))?.collect()
    });
    found.await.unwrap()
}

/// What is found by project in a journal from before that was taken up:
/// the same whether it is as it was taken up, written out again, or
/// opened another time.
async fn found_by_project(journal: &Journal, store: &Store, ended: usize) {
    let here = journal.name().to_string();
    let ids = |found: Vec<runs::InstanceInfo>| found.into_iter().map(|one| one.id).collect::<Vec<_>>();
    assert_eq!(ids(store.alive_of(Some("shop")).await.unwrap()), ["shop-fleet", "shop-fleet-stage"]);
    assert_eq!(ids(store.alive_of(None).await.unwrap()), ["none-watch"]);
    assert!(store.alive_of(Some("yard")).await.unwrap().is_empty());

    let under_way = |found: Vec<runs::RunInfo>| found.into_iter().map(|one| one.id).collect::<Vec<_>>();
    assert_eq!(under_way(store.under_way_of(Some("shop")).await.unwrap()), ["shop-fleet-stage.heard"]);
    assert_eq!(under_way(store.under_way_of(None).await.unwrap()), ["none-watch.heard"]);
    assert_eq!(store.unfinished_runs().await.unwrap().len(), 2);

    // A place there was is a place in the one journal there was, and
    // one of something long over was nobody's to go on from.
    let at = |instance: &str, seq: u64| Place {
        instance: instance.into(),
        handler: "heard".into(),
        journal: here.clone(),
        seq,
    };
    assert_eq!(store.places_of(Some("shop")).await.unwrap(), [at("shop-fleet-stage", 41)]);
    assert_eq!(store.places_of(None).await.unwrap(), [at("none-watch", 17)]);
    assert_eq!(store.place("shop-fleet-stage", "heard", &here).await.unwrap(), Some(41));
    assert_eq!(store.place("shop-old", "heard", &here).await.unwrap(), None);

    // Every row is told apart, those that say nothing through what they
    // are of.
    let told = journal.db().call(|connection| told_apart(connection)).await.unwrap();
    let rows = |table: &str| told.iter().find(|(name, _)| name == table).map(|(_, rows)| *rows);
    assert_eq!(rows("runs"), Some(ended + 4));
    assert_eq!(rows("steps"), Some(ended + 4));
    assert_eq!(rows("instances"), Some(5));
    assert_eq!(store.project_of_run("shop-old.7").await.unwrap(), Some(Some("shop".to_string())));
    assert_eq!(store.project_of_run("none-watch.heard").await.unwrap(), Some(None));
}

/// How many runs say whose they are in their own row.
async fn say_theirs(journal: &Journal) -> i64 {
    journal
        .db()
        .call(|connection| {
            connection.query_row("SELECT count(*) FROM runs WHERE project IS NOT NULL", [], |row| row.get(0))
        })
        .await
        .unwrap()
}

/// What the database holds of the tables from before, and how many rows
/// it has written to since it was opened.
async fn stood(journal: &Journal) -> (Vec<(String, i64)>, u64) {
    journal.db().call(|connection| Ok((kept_at(connection)?, connection.total_changes()))).await.unwrap()
}

#[tokio::test]
async fn a_journal_from_before_is_taken_up_as_it_is_and_its_rows_are_told_apart_all_the_same() {
    let dir = scratch("before");
    let journal = from_before(&dir.0.join("journal.db"), ENDED).await;
    let (mut at, changed) = stood(&journal).await;

    let store = Store::open(&journal).await.unwrap();

    // No table was written out again and no index made over one. Of all
    // the rows there were, only what was under way was written to: the
    // two runs told whose they are. Beside them, three instances noted as
    // alive and two runs as under way, two places brought over, the
    // version, and one note: it had something alive, so what is kept
    // beside its rows may still be in the factory's journal
    // (`runs::beside::OWED`). What had ended says nothing, and is reached.
    let (now_at, now_changed) = stood(&journal).await;
    assert_eq!(now_changed - changed, 11, "rows written to as it was taken up");
    assert_eq!(say_theirs(&journal).await, 2);
    found_by_project(&journal, &store, ENDED).await;
    // It has kept little, so what is left in it that nothing reads was
    // taken away with it: that reads no more than a few pages.
    assert_eq!(left(&journal).await, [""; 0]);
    at.retain(|(name, _)| !["instances_status", "runs_status", "trigger_cursors"].contains(&name.as_str()));
    at.retain(|(name, _)| !name.contains("autoindex_trigger_cursors"));
    assert_eq!(now_at, at);

    // Its tables are at the version this program knows, and the program
    // that made it will not have it back.
    let version = journal.db().call(|connection| {
        connection.query_row("SELECT version FROM schema_versions WHERE namespace = 'conductor'", [], |row| {
            row.get::<_, i64>(0)
        })
    });
    assert_eq!(runs::copy::KNOWS, [("conductor", version.await.unwrap() as usize)]);
    assert!(runs::copy::KNOWS[0].1 > KNEW);
    const AS_IT_KNEW: &[&str] = &["SELECT 1"; KNEW];
    let refused = journal.db().migrate("conductor", AS_IT_KNEW).await.expect_err("an earlier program");
    assert!(refused.message.contains("newer than this build understands"), "{}", refused.message);

    // What is written from here on says whose it is, beside what does not.
    store.insert_run(run("shop-fleet.again", "shop-fleet")).await.unwrap();
    store.insert_run(run("none-watch.again", "none-watch")).await.unwrap();
    assert_eq!(say_theirs(&journal).await, 4);
    let under_way = store.under_way_of(Some("shop")).await.unwrap();
    assert_eq!(under_way.len(), 2, "one from before and one made now, found together");
    journal.db().call(|connection| told_apart(connection)).await.unwrap();

    // A run that had ended and says nothing may be told whose it is,
    // once and rightly or not at all; one under way is kept by project
    // from what it said as it began, and is told nothing else.
    let tell = |sql: &'static str| journal.db().call(move |connection| connection.execute(sql, []));
    tell("UPDATE runs SET project = 'shop' WHERE id = 'shop-old.1'").await.unwrap();
    for refused in [
        "UPDATE runs SET project = 'yard' WHERE id = 'shop-old.1'",
        "UPDATE runs SET project = 'yard' WHERE id = 'shop-fleet-stage.heard'",
        "UPDATE runs SET project = NULL WHERE id = 'shop-fleet-stage.heard'",
    ] {
        let why = tell(refused).await.expect_err(refused);
        assert!(why.message.contains("a run is of the instance"), "{refused}: {}", why.message);
    }
    journal.db().call(|connection| told_apart(connection)).await.unwrap();

    // Opened again there is nothing to do.
    let (_, changed) = stood(&journal).await;
    Store::open(&journal).await.unwrap();
    let (_, again) = stood(&journal).await;
    assert!(again - changed <= 1, "{} rows were written to by opening it again", again - changed);
}

#[tokio::test]
async fn a_journal_from_before_that_has_kept_much_keeps_what_taking_away_would_read_all_of() {
    let dir = scratch("much");
    let ended = runs::SMALL as usize + 1;
    let journal = from_before(&dir.0.join("journal.db"), ended).await;
    let (at, changed) = stood(&journal).await;

    let store = Store::open(&journal).await.unwrap();

    // The same eleven rows, however many there are; and every table and
    // index it had is where it was, those that nothing reads any more
    // among them: taking an index away reads all of it.
    let (now_at, now_changed) = stood(&journal).await;
    assert_eq!(now_changed - changed, 11, "rows written to as it was taken up");
    assert_eq!(now_at, at);
    assert_eq!(left(&journal).await, ["instances_status", "runs_status", "trigger_cursors"]);
    assert_eq!(say_theirs(&journal).await, 2);
    // Nothing goes by them: what is found by project is found as in one
    // that has none, and what is written goes on being kept by project.
    found_by_project(&journal, &store, ended).await;
    store
        .insert_triggered_run(run("shop-fleet-stage.next", "shop-fleet-stage"), "heard", journal.name(), 60)
        .await
        .unwrap();
    assert_eq!(store.under_way_of(Some("shop")).await.unwrap().len(), 2);
    let ending =
        runs::Ending { becomes: None, ..crate::ending("shop-fleet-stage.next", "shop-fleet-stage", false) };
    store.end_run(ending).await.unwrap();
    assert_eq!(store.under_way_of(Some("shop")).await.unwrap().len(), 1);
    assert_eq!(store.place("shop-fleet-stage", "heard", journal.name()).await.unwrap(), Some(60));
    journal.db().call(|connection| told_apart(connection)).await.unwrap();
    // Opened again they are still there: it has not kept less.
    Store::open(&journal).await.unwrap();
    assert_eq!(left(&journal).await.len(), 3);
}

#[tokio::test]
async fn a_journal_from_before_written_out_again_is_told_apart_as_it_was() {
    let dir = scratch("repacked");
    let path = dir.0.join("journal.db");
    let journal = from_before(&path, ENDED).await;
    let store = Store::open(&journal).await.unwrap();
    found_by_project(&journal, &store, ENDED).await;
    drop(store);
    closed(journal).await;

    // Its owner asks, with nothing else holding it.
    let wanted = journal::rewrite::look(&path).unwrap();
    assert!(wanted.wanted(), "it was made with pages a database is no longer made with");
    journal::rewrite::rewrite(&path).unwrap();

    let journal = opened(&path).await;
    let store = Store::open(&journal).await.unwrap();
    found_by_project(&journal, &store, ENDED).await;
    assert_eq!(say_theirs(&journal).await, 2, "writing it out again fills nothing in");
    // The rules went with the tables: what says whose a row is stays.
    let refused = journal
        .db()
        .call(|connection| {
            connection.execute("UPDATE instances SET project = 'yard' WHERE id = 'shop-old'", [])
        })
        .await
        .expect_err("another project");
    assert!(refused.message.contains("an instance is of the project"), "{}", refused.message);
    // And it goes on: a handler's place moves where it was brought to.
    store.set_place("shop-fleet-stage", "heard", journal.name(), 50).await.unwrap();
    assert_eq!(store.place("shop-fleet-stage", "heard", journal.name()).await.unwrap(), Some(50));
}

#[tokio::test]
async fn a_copy_of_a_journal_from_before_keeps_whose_each_row_is_and_no_place() {
    let dir = scratch("copied");
    let (original, copy) = (dir.0.join("journal.db"), dir.0.join("copy.db"));
    let journal = from_before(&original, ENDED).await;
    let store = Store::open(&journal).await.unwrap();
    store.insert_run(run("shop-fleet.again", "shop-fleet")).await.unwrap();

    let (from, to) = (original.clone(), copy.clone());
    tokio::task::spawn_blocking(move || journal::copy::take(&from, &to)).await.unwrap().unwrap();
    let connection = journal::copy::open(&copy).unwrap();
    let emptied =
        journal::copy::keep_tables(&connection, &[journal::copy::TABLES, runs::copy::TABLES].concat());
    assert_eq!(emptied.unwrap(), [""; 0], "every table kept for what runs is spoken for in a copy");
    let ended = runs::copy::leave_out(&connection, 50, None).unwrap();
    assert_eq!(ended, runs::copy::Ended { instances: 3, runs: 3, steps: 2 });

    // Nothing is alive or under way there and no handler has a place;
    // what was is history, and is told apart as it was.
    let count = |sql: &str| connection.query_row(sql, [], |row| row.get::<_, i64>(0)).unwrap();
    assert_eq!(count("SELECT count(*) FROM instances WHERE status IN ('starting', 'active')"), 0);
    assert_eq!(count("SELECT count(*) FROM runs WHERE status = 'running'"), 0);
    for kept in ["places", "alive", "under_way"] {
        assert_eq!(count(&format!("SELECT count(*) FROM {kept}")), 0, "{kept}");
    }
    let told = told_apart(&connection).unwrap();
    assert!(told.contains(&("runs".to_string(), ENDED + 5)), "{told:?}");
    assert_eq!(count("SELECT count(*) FROM runs WHERE project = 'shop'"), 2);
    let refused = connection.execute("UPDATE runs SET instance = 'yard-done' WHERE id = 'shop-old.1'", []);
    assert!(refused.expect_err("another instance").to_string().contains("a run is of the instance"));
    journal::copy::seal(connection).unwrap();

    // A factory started from the copy takes it up as its own.
    let journal = opened(&copy).await;
    let store = Store::open(&journal).await.unwrap();
    assert!(store.alive_of(Some("shop")).await.unwrap().is_empty());
    assert!(store.under_way_of(Some("shop")).await.unwrap().is_empty());
    assert_eq!(store.project_of("shop-fleet-stage").await.unwrap(), Some(Some("shop".to_string())));
}

#[tokio::test]
async fn a_copy_an_earlier_program_made_is_ended_as_it_is_and_taken_up_by_a_factory_started_from_it() {
    let dir = scratch("copied-before");
    let (original, copy) = (dir.0.join("journal.db"), dir.0.join("copy.db"));
    // The tables as that program left them, never opened by this one.
    let journal = from_before(&original, ENDED).await;
    let (from, to) = (original.clone(), copy.clone());
    tokio::task::spawn_blocking(move || journal::copy::take(&from, &to)).await.unwrap().unwrap();
    closed(journal).await;

    // What was under way there is ended in the copy, whichever table a
    // handler's place was kept in when it was made: done when the copy
    // is made, and again by the program a factory is started from it with.
    let connection = journal::copy::open(&copy).unwrap();
    let ended = runs::copy::leave_out(&connection, 50, None).unwrap();
    assert_eq!(ended, runs::copy::Ended { instances: 3, runs: 2, steps: 2 });
    let places: i64 =
        connection.query_row("SELECT count(*) FROM trigger_cursors", [], |row| row.get(0)).unwrap();
    assert_eq!(places, 0);
    assert_eq!(runs::copy::leave_out(&connection, 60, None).unwrap(), runs::copy::Ended::default());
    journal::copy::seal(connection).unwrap();

    // Started on, its tables are brought up: nothing is alive, under way
    // or listening there, and every row is told apart all the same.
    let journal = opened(&copy).await;
    let store = Store::open(&journal).await.unwrap();
    for project in [Some("shop"), None] {
        assert!(store.alive_of(project).await.unwrap().is_empty());
        assert!(store.under_way_of(project).await.unwrap().is_empty());
        assert!(store.places_of(project).await.unwrap().is_empty());
    }
    let told = journal.db().call(|connection| told_apart(connection)).await.unwrap();
    assert!(told.contains(&("runs".to_string(), ENDED + 4)), "{told:?}");
    assert_eq!(say_theirs(&journal).await, 0, "what had ended is reached through what it is of");
    assert_eq!(store.project_of_run("shop-fleet-stage.heard").await.unwrap(), Some(Some("shop".to_string())));
}
