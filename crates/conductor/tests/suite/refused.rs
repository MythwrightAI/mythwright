//! Nothing is written of a step once the journal has shut at its place.
//!
//! A journal that has shut at a place it was told answers nothing: what
//! a step asks of it from then on waits, and is refused when the daemon
//! goes and its database ends. The step is then looked at once more, in
//! the moment before it is ended with everything else, and took the
//! refusal for a failure of its own action: its log said `failed: the
//! database thread has stopped`, before the line that says it was taken
//! up again after the restart. It showed once, in a daemon told to stop
//! at a step, on a machine with much else to do.
//!
//! The order is made here by what is asked: the journal shuts as the
//! step is said to have started, the step's action asks the database to
//! keep what it sets, and the database is then ended with the conductor
//! still there to be looked at. (An event the step says is the
//! journal's to refuse, and waits there for ever; what is written with
//! no event waits at the database, and is refused when that ends.)

use serde_json::json;

use crate::Rig;

const KEEPS: &str = "\
blueprint: keeps
state: { seen: 0 }
steps:
  note: { do: set, with: { seen: 1 } }
";

#[tokio::test]
async fn a_step_refused_by_a_journal_that_has_shut_writes_no_failure_of_its_own_in_its_log() {
    let rig = Rig::new().await;
    rig.journal
        .shut_after(|event| event.topic == "step.started" && event.source.step.as_deref() == Some("note"));
    let applied = rig.engine.apply(rig.request(KEEPS, json!({}))).await.unwrap();
    rig.soon("the journal to shut", rig.journal.shut()).await;
    // The step's action asks for what it sets to be kept, and waits.
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    // The daemon goes: each database ends, and what waited there is
    // refused. The conductor was not halted, so the step is looked at
    // with that, as one is that has not yet been ended.
    let dbs = rig.journal.dbs();
    for db in &dbs {
        db.close();
    }
    for db in &dbs {
        crate::soon("a database to be closed", db.closed()).await;
    }
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    let run = applied.run.expect("its steps were begun");
    let log: Vec<String> =
        rig.logs.read("runs", &format!("{run}.note"), 0, 100).into_iter().map(|line| line.text).collect();
    assert!(
        !log.iter().any(|line| line.contains("failed")),
        "the step's log says it failed, where its journal had shut: {log:?}"
    );
}
