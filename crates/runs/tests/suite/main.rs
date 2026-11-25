//! One test binary for the crate; add modules here.
//!
//! What the runner does with what is kept here is held by its own suite
//! (`crates/conductor/tests/suite`), which plays blueprints and stops them
//! anywhere. These hold what this crate promises by itself, with no engine
//! in it: that a record and what is said of it are kept together or not at
//! all, what a write leaves as it is, and what of the rows goes into a copy;
//! whose each row is and what asking by project reads (`apart`), a
//! handler's places journal by journal (`places`), the order a run's steps
//! are read in (`ordered`), where the text an instance runs is kept
//! (`texts`), and what becomes of a journal an earlier program made
//! (`before`).

mod apart;
mod before;
mod moved;
mod ordered;
mod places;
mod texts;

use std::path::PathBuf;
use std::sync::Arc;

use journal::{Db, EventQuery, Journal};
use kernel::{Cancel, Fields, ManualClock, Principal, Source};
use runs::{
    Becomes, Ending, InstanceInfo, InstanceStatus, RunInfo, RunStatus, StepInfo, StepStatus, Store, to_say,
};
use serde_json::{Value, json};

async fn kept() -> (Journal, Store) {
    let journal = Journal::memory(Arc::new(ManualClock::new())).await.unwrap();
    let store = Store::open(&journal).await.unwrap();
    (journal, store)
}

/// An instance that is nobody's.
fn instance(id: &str) -> InstanceInfo {
    InstanceInfo {
        id: id.into(),
        blueprint: "greet".into(),
        revision: "r1".into(),
        source: "name: greet\n".into(),
        params: Fields::new(),
        state: Fields::new(),
        status: InstanceStatus::Starting,
        parent: None,
        key: id.into(),
        target: None,
        project: None,
        labels: Default::default(),
        principal: Principal::system(),
        holds: Vec::new(),
        error: None,
        created: 1,
        updated: 1,
        previous: None,
        heard: None,
        environment: String::new(),
    }
}

/// An instance of `project`, started under `parent` where there is one.
fn of_project(id: &str, project: Option<&str>, parent: Option<&str>) -> InstanceInfo {
    InstanceInfo { project: project.map(str::to_string), parent: parent.map(str::to_string), ..instance(id) }
}

fn run(id: &str, of: &str) -> RunInfo {
    RunInfo {
        id: id.into(),
        instance: of.into(),
        flow: "main".into(),
        revision: "r1".into(),
        status: RunStatus::Running,
        event: Value::Null,
        outputs: Value::Null,
        error: None,
        started: 2,
        finished: None,
        steps: Vec::new(),
    }
}

fn step(of: &str, attempt: u32, status: StepStatus) -> StepInfo {
    StepInfo {
        run: of.into(),
        step: "hello".into(),
        status,
        attempt,
        tolerated: false,
        output: Value::Null,
        error: None,
        saved: Value::Null,
        started: 3,
        finished: None,
    }
}

/// A folder of a test's own, under the build's target directory, and gone
/// when the test has done with it, however it ended.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A fresh directory under the build's own target directory. Never the system
/// temp directory: on the machines this runs on that is RAM.
fn scratch(name: &str) -> Scratch {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("runs-{name}-{}", kernel::id::secret()));
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

/// What the database keeps of the runner's, each table read whole and
/// every row held to what [`runs::apart::TABLES`] says tells it apart:
/// for each table, how many rows it has. The rows of these tests say
/// whose they are in what they are called: an instance `shop-...` is of
/// the project `shop`, and one `none-...` is nobody's, and so with the
/// run, the step and the place that are named after it. And what is
/// kept of what is alive and under way is held to the rows it is kept
/// from: every instance that says it is alive and no other, every run
/// that says it is under way and no other.
fn told_apart(connection: &rusqlite::Connection) -> rusqlite::Result<Vec<(String, usize)>> {
    let mut found = Vec::new();
    for (table, told) in runs::apart::TABLES {
        // What a row is named after: itself, or what it is reached through.
        let named = match told {
            runs::apart::Told::Says => "id",
            runs::apart::Told::Instance(column)
            | runs::apart::Told::SaysOrInstance(column)
            | runs::apart::Told::Run(column) => column,
        };
        let reads = told.reads();
        let mut statement = connection.prepare(&format!("SELECT kept.{named}, {reads} FROM {table} kept"))?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (name, whose) in &rows {
            let meant = name.split('-').next().filter(|project| *project != "none");
            assert_eq!(whose.as_deref(), meant, "`{name}` of `{table}` is told apart as another's");
        }
        found.push((table.to_string(), rows.len()));
    }
    for (kept, of, key, are) in [
        ("alive", "instances", "instance", "status IN ('starting', 'active')"),
        ("under_way", "runs", "run", "status = 'running'"),
    ] {
        let astray: i64 = connection.query_row(
            &format!(
                "SELECT (SELECT count(*) FROM {of} WHERE {are} AND id NOT IN (SELECT {key} FROM {kept}))
                      + (SELECT count(*) FROM {kept} WHERE {key} NOT IN (SELECT id FROM {of} WHERE {are}))"
            ),
            [],
            |row| row.get(0),
        )?;
        assert_eq!(astray, 0, "`{kept}` is not what the rows of `{of}` say");
    }
    Ok(found)
}

/// The topics of everything the journal kept, in order.
async fn said(journal: &Journal) -> Vec<String> {
    journal.read(EventQuery::default()).await.unwrap().into_iter().map(|event| event.topic).collect()
}

fn says(topic: &'static str, of: &str) -> Vec<runs::Say> {
    vec![to_say(topic, Source::instance(of), json!({}))]
}

#[tokio::test]
async fn the_tables_are_kept_under_the_name_a_journal_from_before_knows_them_by() {
    let (journal, _) = kept().await;
    let version = |journal: &Journal| {
        journal.db().call(|connection| {
            connection.query_row(
                "SELECT version FROM schema_versions WHERE namespace = 'conductor'",
                [],
                |row| row.get::<_, i64>(0),
            )
        })
    };
    let made = version(&journal).await.unwrap();
    assert_eq!(runs::copy::KNOWS, [("conductor", made as usize)]);
    // Opened again there is nothing to make.
    Store::open(&journal).await.unwrap();
    assert_eq!(version(&journal).await.unwrap(), made);
}

#[tokio::test]
async fn an_instance_is_written_and_said_to_have_started_in_one_write_or_not_at_all() {
    let (journal, store) = kept().await;
    // What it would be started under was cancelled first: nothing of it.
    let under = Cancel::new();
    under.cancel();
    let wrote = store.insert_instance(instance("late"), Some(under), says("instance.started", "late")).await;
    assert!(!wrote.unwrap());
    assert!(store.instance("late").await.unwrap().is_none());
    assert!(said(&journal).await.is_empty());

    let wrote = store.insert_instance(instance("one"), Some(Cancel::new()), says("instance.started", "one"));
    assert!(wrote.await.unwrap());
    assert_eq!(store.instance("one").await.unwrap().unwrap().status, InstanceStatus::Starting);
    assert_eq!(said(&journal).await, ["instance.started"]);
    // What it starts with is its first version.
    let versions = store.versions("one", 10).await.unwrap();
    assert_eq!(versions.iter().map(|version| version.version).collect::<Vec<_>>(), [1]);
}

fn ending(run: &str, of: &str, still: bool) -> Ending {
    Ending {
        run: run.into(),
        status: RunStatus::Ok,
        outputs: json!({ "said": "hello" }),
        error: None,
        now: 9,
        steps: Vec::new(),
        says: vec![to_say("run.finished", Source::instance(of), json!({}))],
        becomes: Some(Becomes {
            instance: of.into(),
            status: Some(InstanceStatus::Active),
            note: None,
            update: None,
            owes: false,
            says: says("instance.active", of),
            still: Box::new(move || still),
            then: None,
        }),
    }
}

#[tokio::test]
async fn a_run_that_no_longer_speaks_for_its_instance_ends_and_says_nothing_of_it() {
    let (journal, store) = kept().await;
    store.insert_instance(instance("one"), None, Vec::new()).await.unwrap();
    store.insert_run(run("first", "one")).await.unwrap();
    // Somebody took the instance in hand since: the run ends, and says so.
    assert!(!store.end_run(ending("first", "one", false)).await.unwrap());
    assert_eq!(store.run("first").await.unwrap().unwrap().status, RunStatus::Ok);
    assert_eq!(store.instance("one").await.unwrap().unwrap().status, InstanceStatus::Starting);
    assert_eq!(said(&journal).await, ["run.finished"]);

    // One that still does settles it, in the write that ends the run.
    store.insert_run(run("second", "one")).await.unwrap();
    assert!(store.end_run(ending("second", "one", true)).await.unwrap());
    assert_eq!(store.instance("one").await.unwrap().unwrap().status, InstanceStatus::Active);
    assert_eq!(said(&journal).await, ["run.finished", "run.finished", "instance.active"]);

    // And an instance that is over is nobody's to settle again.
    store.stop_instance("one", 10, says("instance.stopped", "one")).await.unwrap();
    store.insert_run(run("third", "one")).await.unwrap();
    assert!(!store.end_run(ending("third", "one", true)).await.unwrap());
    assert_eq!(store.instance("one").await.unwrap().unwrap().status, InstanceStatus::Stopped);
    assert_eq!(said(&journal).await.last().map(String::as_str), Some("run.finished"));
}

#[tokio::test]
async fn a_step_written_again_keeps_what_is_kept_beside_it_until_its_next_try() {
    let (journal, store) = kept().await;
    store.insert_instance(instance("one"), None, Vec::new()).await.unwrap();
    store.insert_run(run("first", "one")).await.unwrap();
    let nothing = || (Vec::new(), Vec::new());
    let started = says("step.started", "one");
    store.put_step_saying(step("first", 1, StepStatus::Running), nothing, started).await.unwrap();
    store.save_hooks("first", Some("hello"), json!({ "start": 1 })).await.unwrap();

    // Written as ended, in the try it was kept in: what is beside it stays.
    let finished = says("step.finished", "one");
    store.put_step_saying(step("first", 1, StepStatus::Ok), nothing, finished).await.unwrap();
    assert_eq!(store.kept_hooks("first", Some("hello")).await.unwrap(), json!({ "start": 1 }));
    // Another try begins with none of it.
    store.put_step_saying(step("first", 2, StepStatus::Running), nothing, Vec::new()).await.unwrap();
    assert_eq!(store.kept_hooks("first", Some("hello")).await.unwrap(), Value::Null);
    assert_eq!(said(&journal).await, ["step.started", "step.finished"]);
}

#[tokio::test]
async fn what_the_rows_of_a_copy_say_was_under_way_is_ended_there_and_ended_once() {
    let dir = scratch("copy");
    let (original, copy) = (dir.0.join("journal.db"), dir.0.join("copy.db"));
    let journal =
        Journal::open(Db::open(&original).unwrap(), Arc::new(ManualClock::new()), "here").await.unwrap();
    let store = Store::open(&journal).await.unwrap();
    for id in ["one", "making"] {
        store.insert_instance(instance(id), None, Vec::new()).await.unwrap();
        store.insert_run(run(&format!("run-of-{id}"), id)).await.unwrap();
        let begun = step(&format!("run-of-{id}"), 1, StepStatus::Running);
        store.put_step_saying(begun, || (Vec::new(), Vec::new()), Vec::new()).await.unwrap();
    }
    store.set_place("one", "heard", journal.name(), 4).await.unwrap();

    let (from, to) = (original.clone(), copy.clone());
    tokio::task::spawn_blocking(move || journal::copy::take(&from, &to)).await.unwrap().unwrap();
    let connection = journal::copy::open(&copy).unwrap();
    // What was making the copy was under way too, and is not counted.
    let ended = runs::copy::leave_out(&connection, 50, Some(("making", "run-of-making"))).unwrap();
    assert_eq!(ended, runs::copy::Ended { instances: 1, runs: 1, steps: 1 });
    let why = |table: &str, key: &str, id: &str| -> (String, String) {
        connection
            .query_row(&format!("SELECT status, error FROM {table} WHERE {key} = ?1"), [id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap()
    };
    assert_eq!(why("instances", "id", "one"), ("stopped".into(), runs::copy::NOT_STARTED.into()));
    assert_eq!(why("runs", "id", "run-of-one"), ("cancelled".into(), runs::copy::NOT_FINISHED.into()));
    assert_eq!(why("steps", "run", "run-of-one"), ("cancelled".into(), runs::copy::NOT_FINISHED.into()));
    assert_eq!(why("instances", "id", "making"), ("stopped".into(), runs::copy::MAKING_THE_COPY.into()));
    let places: i64 = connection.query_row("SELECT count(*) FROM places", [], |row| row.get(0)).unwrap();
    assert_eq!(places, 0);
    // Done again, nothing is under way there.
    assert_eq!(runs::copy::leave_out(&connection, 60, None).unwrap(), runs::copy::Ended::default());
    drop(connection);
    // The factory copied goes on as it was.
    assert_eq!(store.instance("one").await.unwrap().unwrap().status, InstanceStatus::Starting);
    assert_eq!(store.place("one", "heard", journal.name()).await.unwrap(), Some(4));
}
