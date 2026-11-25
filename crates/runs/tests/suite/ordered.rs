//! A run's steps are read in the order they began (`runs::put`, and the
//! order its steps are read by, beside it).
//!
//! When a step began is kept in milliseconds, and several steps of a run
//! begin in one, so a step's row says which of its run's steps it was to
//! begin: in the write that first keeps it, and never another afterwards.
//! Held here with no engine: what a row is given and by which write,
//! that nothing written later moves it, that steps taken away and
//! written again are given a place no step of the run has, that what a
//! program from before wrote is read as it was, and what all of it costs
//! the database. That a run played by the runner reads so is the
//! runner's suite's (`crates/conductor/tests/suite/ordered.rs`).

use std::sync::Arc;
use std::sync::atomic::Ordering;

use journal::{Db, Journal};
use kernel::ManualClock;
use runs::{Ending, RunStatus, StepInfo, StepStatus, Store};
use serde_json::{Value, json};

use crate::apart::counting;
use crate::{instance, kept, run, scratch, step};

/// A step called `name` of the run `of`, which began at `started`.
fn called(of: &str, name: &str, status: StepStatus, started: i64) -> StepInfo {
    StepInfo { step: name.into(), started, ..step(of, 1, status) }
}

/// Writes a step's record by itself, as a step that begins or ends does.
async fn put(store: &Store, step: StepInfo) {
    store.put_step_saying(step, || (Vec::new(), Vec::new()), Vec::new()).await.unwrap();
}

/// A store with an instance and a run of it called `first`.
async fn running() -> (Journal, Store) {
    let (journal, store) = kept().await;
    store.insert_instance(instance("one"), None, Vec::new()).await.unwrap();
    store.insert_run(run("first", "one")).await.unwrap();
    (journal, store)
}

/// The names of a run's steps, in the order the run is read in.
async fn read(store: &Store, run: &str) -> Vec<String> {
    store.run(run).await.unwrap().unwrap().steps.into_iter().map(|step| step.step).collect()
}

/// Which of its run's steps each row of a run says it was to begin, by
/// the step's name: what the rows keep, which is handed to nobody.
async fn places(journal: &Journal, run: &'static str) -> Vec<(String, Option<i64>)> {
    let asked = journal.db().call(move |connection| {
        let mut rows = connection.prepare("SELECT step, nth FROM steps WHERE run = ?1 ORDER BY step")?;
        rows.query_map([run], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()
    });
    asked.await.unwrap()
}

fn place(of: &[(String, Option<i64>)], step: &str) -> Option<i64> {
    of.iter().find(|(name, _)| name == step).unwrap_or_else(|| panic!("no step `{step}`")).1
}

#[tokio::test]
async fn a_step_says_which_of_its_runs_steps_it_was_to_begin_in_its_first_write_and_in_no_write_after() {
    let (journal, store) = running().await;
    // Three in one millisecond, none of them called in the order they began.
    for name in ["zulu", "alpha", "mid"] {
        put(&store, called("first", name, StepStatus::Running, 3)).await;
    }
    assert_eq!(read(&store, "first").await, ["zulu", "alpha", "mid"]);
    let first = places(&journal, "first").await;
    assert_eq!(
        (place(&first, "zulu"), place(&first, "alpha"), place(&first, "mid")),
        (Some(1), Some(2), Some(3))
    );

    // Each written again: as ended; as tried again, later; and with a
    // clock that was put back saying it began before the others had.
    put(&store, called("first", "zulu", StepStatus::Ok, 3)).await;
    let again = StepInfo { attempt: 2, ..called("first", "alpha", StepStatus::Running, 50) };
    put(&store, again).await;
    put(&store, called("first", "mid", StepStatus::Ok, 1)).await;
    // And what is kept beside a row and in it, by a write of its own.
    store.save_hooks("first", Some("zulu"), json!({ "start": 1 })).await.unwrap();
    store.save_checkpoint("first", "alpha", json!({ "pid": 7 })).await.unwrap();
    assert_eq!(read(&store, "first").await, ["zulu", "alpha", "mid"], "each is where it first began");
    assert_eq!(places(&journal, "first").await, first);
    let kept = store.run("first").await.unwrap().unwrap();
    assert_eq!(kept.steps[1].started, 50, "when it began is what was last written of it");

    // Another run's steps are counted by themselves.
    store.insert_run(run("second", "one")).await.unwrap();
    put(&store, called("second", "only", StepStatus::Running, 3)).await;
    assert_eq!(places(&journal, "second").await, [("only".to_string(), Some(1))]);
    assert_eq!(read(&store, "first").await, ["zulu", "alpha", "mid"]);
}

#[tokio::test]
async fn steps_taken_away_and_written_again_are_given_a_place_that_no_step_of_their_run_has() {
    let (journal, store) = running().await;
    // Steps done as one, and a step beside them that began after theirs.
    for name in ["build", "build.yard", "build.bell", "side"] {
        put(&store, called("first", name, StepStatus::Running, 3)).await;
    }
    // Done again: what the steps inside did is taken away, and written anew.
    store.forget_steps("first", "build.").await.unwrap();
    assert_eq!(read(&store, "first").await, ["build", "side"]);
    for name in ["build.yard", "build.bell"] {
        put(&store, called("first", name, StepStatus::Running, 3)).await;
    }
    // Counted by how many rows the run has, the first of them would have
    // been given the place the step beside has.
    assert_eq!(read(&store, "first").await, ["build", "side", "build.yard", "build.bell"]);
    let given = places(&journal, "first").await;
    assert_eq!((place(&given, "build"), place(&given, "side")), (Some(1), Some(4)));
    assert_eq!((place(&given, "build.yard"), place(&given, "build.bell")), (Some(5), Some(6)));

    // And taken away as an element of a list ends, for the next element.
    store.put_element("first", "build", 0, &Value::Null, Some("build.".into())).await.unwrap();
    put(&store, called("first", "build.yard", StepStatus::Running, 3)).await;
    put(&store, called("first", "after", StepStatus::Running, 3)).await;
    assert_eq!(read(&store, "first").await, ["build", "side", "build.yard", "after"]);
    let mut given: Vec<i64> =
        places(&journal, "first").await.into_iter().filter_map(|(_, nth)| nth).collect();
    given.sort_unstable();
    assert_eq!(given, [1, 4, 5, 6], "no two of them have one place");
}

#[tokio::test]
async fn steps_written_with_another_and_with_their_runs_ending_are_kept_in_the_order_they_are_handed() {
    let (_journal, store) = running().await;
    // Two that were passed over, written with the step that begins next
    // and before it.
    let passed = |name: &str| called("first", name, StepStatus::Skipped, 3);
    let before = vec![passed("mike"), passed("alpha")];
    let door = called("first", "door", StepStatus::Running, 3);
    store.put_step_saying(door, move || (before, Vec::new()), Vec::new()).await.unwrap();
    // One written by itself, before something that takes time.
    let held = vec![passed("zulu")];
    store.put_held_saying(move || (held, Vec::new())).await.unwrap();
    // And those written with the run's ending, one of them written before.
    let ending = Ending {
        run: "first".into(),
        status: RunStatus::Ok,
        outputs: Value::Null,
        error: None,
        now: 9,
        steps: vec![passed("yard"), called("first", "door", StepStatus::Ok, 3), passed("able")],
        says: Vec::new(),
        becomes: None,
    };
    store.end_run(ending).await.unwrap();
    assert_eq!(read(&store, "first").await, ["mike", "alpha", "door", "zulu", "yard", "able"]);
}

#[tokio::test]
async fn steps_a_program_from_before_wrote_are_read_as_they_were_and_before_those_that_say_their_place() {
    let (journal, store) = running().await;
    // As that program wrote them: when each began, and nothing more.
    let wrote = journal.db().call(|connection| {
        for (name, started) in [("zulu", 3), ("alpha", 3), ("mid", 2)] {
            connection.execute(
                "INSERT INTO steps (run, step, status, attempt, started) VALUES ('first', ?1, 'running', 1, ?2)",
                rusqlite::params![name, started],
            )?;
        }
        Ok(())
    });
    wrote.await.unwrap();
    let as_it_was = ["mid", "alpha", "zulu"];
    assert_eq!(read(&store, "first").await, as_it_was, "by when each began, and then by its name");

    // The run goes on under this program: a step from before ends, and
    // two begin, the first of them by a clock that was put back.
    put(&store, called("first", "zulu", StepStatus::Ok, 3)).await;
    put(&store, called("first", "baker", StepStatus::Running, 1)).await;
    put(&store, called("first", "able", StepStatus::Running, 4)).await;
    assert_eq!(read(&store, "first").await, ["mid", "alpha", "zulu", "baker", "able"]);
    let given = places(&journal, "first").await;
    assert_eq!(place(&given, "zulu"), None, "a row from before is given no place by being written again");
    assert_eq!((place(&given, "baker"), place(&given, "able")), (Some(1), Some(2)));
}

#[tokio::test]
async fn a_step_written_again_costs_the_database_the_same_however_many_steps_its_run_has() {
    let (journal, store) = running().await;
    store.insert_run(run("long", "one")).await.unwrap();
    for name in ["zulu", "alpha", "mid"] {
        put(&store, called("first", name, StepStatus::Running, 3)).await;
    }
    for n in 0..300 {
        put(&store, called("long", &format!("step-{n}"), StepStatus::Running, 3)).await;
    }
    let done = counting(&journal).await;
    let mut cost = Vec::new();
    for (run, written, new) in [("first", "alpha", "new"), ("long", "step-150", "new")] {
        // Ended, which is the write every step has after its first.
        let before = done.load(Ordering::SeqCst);
        put(&store, called(run, written, StepStatus::Ok, 3)).await;
        let again = done.load(Ordering::SeqCst) - before;
        // And one that begins, which looks through its run's rows once.
        let before = done.load(Ordering::SeqCst);
        put(&store, called(run, new, StepStatus::Running, 3)).await;
        cost.push((again, done.load(Ordering::SeqCst) - before));
    }
    let ((again, begins), (again_in_long, begins_in_long)) = (cost[0], cost[1]);
    assert!(again > 0, "nothing was counted");
    assert_eq!(again, again_in_long, "a step written again looked through its run's rows");
    assert!(begins_in_long > begins + 300, "{begins} and {begins_in_long}: is the database's work counted?");
    assert_eq!(read(&store, "first").await, ["zulu", "alpha", "mid", "new"]);
}

#[tokio::test]
async fn a_copy_reads_a_runs_steps_in_the_order_they_began() {
    let dir = scratch("ordered-copy");
    let (original, copy) = (dir.0.join("journal.db"), dir.0.join("copy.db"));
    let open = |path: &std::path::Path| {
        Journal::open(Db::open_unwaited(path).unwrap(), Arc::new(ManualClock::new()), "here")
    };
    let journal = open(&original).await.unwrap();
    let store = Store::open(&journal).await.unwrap();
    store.insert_instance(instance("one"), None, Vec::new()).await.unwrap();
    store.insert_run(run("first", "one")).await.unwrap();
    for name in ["zulu", "alpha", "mid"] {
        put(&store, called("first", name, StepStatus::Running, 3)).await;
    }
    let (from, to) = (original.clone(), copy.clone());
    tokio::task::spawn_blocking(move || journal::copy::take(&from, &to)).await.unwrap().unwrap();
    // What was under way is ended in a copy, which writes each step.
    let ended = runs::copy::leave_out(&journal::copy::open(&copy).unwrap(), 50, None).unwrap();
    assert_eq!(ended.steps, 3);
    let there = Store::open(&open(&copy).await.unwrap()).await.unwrap();
    assert_eq!(read(&there, "first").await, ["zulu", "alpha", "mid"]);
}
