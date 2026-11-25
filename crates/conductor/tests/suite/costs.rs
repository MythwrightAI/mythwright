//! What a handler's event costs the journal, by the journal's own count
//! of its work: how many pieces of work wrote, how many were done at all,
//! and how many pages went into the log with nothing brought back. And
//! what a step whose action only decides costs, which is no write of its
//! own, and one whose action is the same done twice, whose ending is
//! written with what comes after it.

use std::sync::Arc;

use conductor::Engine;
use journal::checkpoint::Kept;
use journal::{Db, Journal};
use kernel::Source;
use serde_json::json;

use crate::handlers::active;
use crate::{Rig, registry};

const HEARS: &str = "\
blueprint: hears
on:
  said:
    event: thing.said
    when: ${ event.data.wanted }
    steps:
      note: { do: emit, with: { topic: thing.heard, n: \"${ event.data.n }\" } }
";

/// What a handler that hears a thing works out of it before it says it
/// heard, where `of` is how it is worked out, handed what was heard as
/// `given`: an action that only decides (`test.decides`), one that is the
/// same done twice (`test.echo`), or one that says neither
/// (`test.decide`).
fn working(of: &str, given: &str) -> String {
    format!(
        "\
blueprint: works
on:
  said:
    event: thing.said
    steps:
      what: {{ do: {of}, with: {{ {given}: \"n ${{ event.data.n }}\" }} }}
      note: {{ do: emit, with: {{ topic: thing.heard, n: \"${{ steps.what.out.n }}\" }} }}
"
    )
}

/// A rig whose journal is a file with a log that is never brought back,
/// so that how long the log is says how many pages were written. With
/// `from_before`, the journal is one that has kept much from before: it
/// still has the index of every instance's status and of every run's,
/// which nothing reads and each write of a status goes on writing.
async fn counting(from_before: bool) -> Rig {
    let mut rig = Rig::new().await;
    std::fs::create_dir_all(&rig.dir).unwrap();
    let db =
        Db::open_keeping(&rig.dir.join("journal.db"), Kept { ask: 1_000_000, bound: 1_000_000 }).unwrap();
    rig.journal = Journal::open(db, Arc::new(rig.clock.clone()), "test-device").await.unwrap();
    rig.journal.orders_by(rig.order.clone());
    if from_before {
        runs::Store::open(&rig.journal).await.unwrap();
        let kept = rig.journal.db().call(|connection| {
            connection.execute_batch(
                "CREATE INDEX instances_status ON instances (status);
                 CREATE INDEX runs_status ON runs (status);",
            )?;
            let mut ran = connection.prepare(
                "INSERT INTO runs (id, instance, flow, revision, status, event, started, project)
                 VALUES (?1, 'long-over', 'main', 'r1', 'ok', 'null', 1, '')",
            )?;
            for n in 0..=runs::SMALL {
                ran.execute([format!("run-{n}")])?;
            }
            Ok(())
        });
        kept.await.unwrap();
    }
    rig.engine =
        Engine::new(rig.journal.clone(), rig.logs.clone(), registry(&rig.shared), rig.library.clone())
            .await
            .unwrap();
    rig
}

/// What the journal has counted so far: work that wrote, work done, and
/// pages in the log. Read once the database has answered something
/// asked now: it counts a piece of work a moment after whoever follows
/// the journal has heard what that work said, and this is asked on
/// hearing. What is asked for that is itself a piece of work done, which
/// whoever takes one count from another takes off ([`cost`]).
async fn counted(rig: &Rig) -> (u64, u64, i64) {
    let db = rig.journal.db();
    db.call(|_| Ok(())).await.unwrap();
    (db.writes(), db.done(), db.log_pages())
}

/// What `events` things said one at a time cost, each waited for until
/// what it led to is over: for one event, the pieces of work that wrote,
/// the pieces of work done, and the pages written.
async fn cost(rig: &Rig, events: u64, wanted: bool) -> (f64, f64, f64) {
    let before = counted(rig).await;
    let mut over = rig.journal.follow_every(rig.journal.now(), "run.finished");
    for n in 0..events {
        let data = json!({ "n": n, "wanted": wanted });
        rig.journal.emit("thing.said", Source::default(), data).await.unwrap();
        if wanted {
            rig.soon("the run it started", over.next()).await.unwrap();
        }
    }
    let after = counted(rig).await;
    let each = |all: u64| all as f64 / events as f64;
    // But for what was asked of the database to read the second count.
    let done = after.1 - before.1 - 1;
    (each(after.0 - before.0), each(done), each((after.2 - before.2) as u64))
}

/// What an event that starts a handler's run, and one that is passed
/// over, each cost a journal.
async fn costs(from_before: bool) -> ((f64, f64, f64), (f64, f64, f64)) {
    costs_of(HEARS, from_before).await
}

/// The same, of the handler `text`.
async fn costs_of(text: &str, from_before: bool) -> ((f64, f64, f64), (f64, f64, f64)) {
    let rig = counting(from_before).await;
    let applied = rig.apply(text, json!({})).await;
    active(&rig, &applied.instance).await;
    // Something first, so that every table and index has its first page.
    cost(&rig, 64, true).await;
    (cost(&rig, 512, true).await, cost(&rig, 512, false).await)
}

#[tokio::test]
async fn a_handlers_event_costs_the_journal_no_more_for_its_place_naming_a_journal_and_its_run_a_project() {
    let (ran, passed) = costs(false).await;
    println!("an event that starts a run: {:.4} writes, {:.4} done, {:.4} pages", ran.0, ran.1, ran.2);
    println!("an event passed over: {:.4} writes, {:.4} done, {:.4} pages", passed.0, passed.1, passed.2);
    // The event itself, its run with its place, the run said to have
    // started with its one step, what the step says, and the run's
    // ending with the step's, which is held until then since saying an
    // event is the same done twice: six pieces of work, each of which
    // wrote, and nothing read. Counted over 2,048 events before a place
    // named its journal and a run its project it was seven (the step's
    // ending a write of its own), and 39.9 to 40.1 pages; after, 39.8 to
    // 40.0. Over the 512 here it comes to a little less, the tables
    // having grown less. A journal made now keeps no index of its events
    // by instance or by run, each a page of nearly every write that says
    // anything: 25.8 to 25.9 here, where it was 39.8 to 40.0; and 24.9
    // to 25.0 with the step's ending held. And what the runner says of
    // its own steps and runs, which no text here hears, is passed on and
    // not kept: four events fewer in the table of events and in the
    // index of their topics, 16.4 pages with the step's ending held as
    // well. The run said to have started is still a write of its own, its
    // row written as begun.
    assert_eq!((ran.0, ran.1), (6.0, 6.0));
    assert!(ran.2 < 17.0, "{} pages for an event that starts a run", ran.2);
    // Passed over, it is its own write, and one more in 256 for the
    // place of the handler that did not want it (the last of which the
    // handler may not have come to as this is read): 2.3125 pages, as
    // it was.
    assert_eq!(passed.0, passed.1, "nothing is read for an event passed over");
    assert!((1.0..=1.0 + 1.0 / 256.0).contains(&passed.0), "{} writes for an event passed over", passed.0);
    assert!(passed.2 < 2.35, "{} pages for an event passed over", passed.2);
}

#[tokio::test]
async fn in_a_journal_that_has_kept_much_from_before_a_handlers_event_costs_a_page_more_as_its_run_begins_and_ends()
 {
    let (ran, passed) = costs(true).await;
    println!("from before, an event that starts a run: {:.4} writes, {:.4} pages", ran.0, ran.2);
    println!("from before, an event passed over: {:.4} writes, {:.4} pages", passed.0, passed.2);
    // The index of every run's status is still there, and is written as
    // the run begins and again as it ends: the same six pieces of work,
    // and about two pages more. It is taken away where the journal
    // has kept little, and not where taking it away would read all of it.
    // (This journal has said little, so it keeps no index of its events
    // by instance or by run, as one made now: 28.2 to 28.4 pages, where
    // it was 40.5 to 43; 27.6 with the step's ending held; and 18.9
    // since what the runner says of its own steps and runs is passed on
    // where nothing hears it.)
    assert_eq!((ran.0, ran.1), (6.0, 6.0));
    assert!(ran.2 > 17.0 && ran.2 < 19.5, "{} pages for an event that starts a run", ran.2);
    assert!((1.0..=1.0 + 1.0 / 256.0).contains(&passed.0), "{} writes for an event passed over", passed.0);
    assert!(passed.2 < 2.35, "{} pages for an event passed over", passed.2);
}

#[tokio::test]
async fn a_step_whose_action_only_decides_is_written_with_the_next_thing_its_run_writes_and_has_no_write_of_its_own()
 {
    let ((plain, _), (again, _), (decided, _)) = (
        costs_of(&working("test.decide", "decision"), false).await,
        costs_of(&working("test.echo", "n"), false).await,
        costs_of(&working("test.decides", "n"), false).await,
    );
    println!(
        "worked out by an action that says nothing of itself: {:.4} writes, {:.4} pages",
        plain.0, plain.2
    );
    println!("worked out by one that is the same done twice: {:.4} writes, {:.4} pages", again.0, again.2);
    println!("worked out by one that only decides: {:.4} writes, {:.4} pages", decided.0, decided.2);
    // The event, its run with its place, the run said to have started
    // with the step that works it out, that step's ending, the step that
    // says it heard as it begins, what it says, and the run's ending
    // with that step's (saying is the same done twice): seven, and one
    // more for the place of the handler. Where what works it out is the
    // same done twice, its ending is written with the step after it:
    // seven. Where it only decides, its beginning is too: six, and the
    // same events said. 31.4 pages, 29.6 and 27.9.
    assert_eq!(plain.0, 8.0);
    assert_eq!(again.0, plain.0 - 1.0);
    assert_eq!(decided.0, plain.0 - 2.0);
    assert!(
        decided.2 < again.2 && again.2 < plain.2,
        "{} and {} pages where it was {}",
        decided.2,
        again.2,
        plain.2
    );
}

#[tokio::test]
async fn every_write_that_keeps_a_handlers_place_for_events_it_passed_over_is_measured_and_no_other() {
    let rig = counting(false).await;
    let applied = rig.apply(HEARS, json!({})).await;
    active(&rig, &applied.instance).await;
    cost(&rig, 64, true).await;
    // Asked of the measurements before the first count and after the
    // last: kept, they are what the database writes next.
    let kept = async || {
        rig.journal.flush_metrics().await.unwrap();
        let query = journal::MetricQuery { name: conductor::PLACES_KEPT.into(), ..Default::default() };
        let series = rig.journal.query_metrics(query).await.unwrap();
        series.iter().map(|series| series.total.count).sum::<u64>()
    };
    let was = kept().await;
    let before = counted(&rig).await;
    let mut over = rig.journal.follow_every(rig.journal.now(), "run.finished");
    for n in 0..600 {
        let data = json!({ "n": n, "wanted": false });
        rig.journal.emit("thing.said", Source::default(), data).await.unwrap();
    }
    // One it wants, and its run waited for: the handler takes what it is
    // handed in order, so it has passed over all six hundred by then,
    // and written its place for them where it was due.
    let data = json!({ "n": 600, "wanted": true });
    rig.journal.emit("thing.said", Source::default(), data).await.unwrap();
    rig.soon("the run it started", over.next()).await.unwrap();
    let after = counted(&rig).await;
    let kept = kept().await - was;
    // Six hundred passed over is twice its 256 of its own reading at the
    // least, and the handlers idle on a topic may be brought forward as
    // well, as the handler is found with nothing in hand: how many there
    // are goes by how the handler's reading fell, and each is measured.
    assert!(kept >= 2, "{kept} places kept for six hundred events passed over");
    // Each event passed over is its own write, the one wanted is six (as
    // above), and whatever else wrote kept a place and was measured so.
    assert_eq!(after.0 - before.0, 600 + 6 + kept, "{kept} measured as keeping a place");
}
