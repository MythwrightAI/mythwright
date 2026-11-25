//! A journal in a file of its own, whose numbers come from the factory's
//! giver: the mark above a run is on the disk before any number of the
//! run is an event's; the next run is asked for ahead of need, so that
//! its thread does not wait for the factory's in the middle of a write;
//! what is said though it has shut waits for no database that is held;
//! a journal that joins holding numbers of its own takes them from
//! whatever anybody had in hand; and nothing is given twice wherever the
//! two are stopped.

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use journal::giver::{DRAWN, RUN};
use journal::{Db, EventQuery, Journal, MetricQuery};
use kernel::{Clock, ManualClock, Source};
use serde_json::json;

use crate::scratch;

/// As long as anything here is waited for: what does not come by then
/// never would.
const PATIENCE: Duration = Duration::from_secs(40);

fn clock() -> Arc<dyn Clock> {
    Arc::new(ManualClock::new())
}

/// The factory's journal in `dir`, and a project's beside it that draws
/// from its giver.
async fn two(dir: &Path) -> (Journal, Journal) {
    let factory = Journal::open(Db::open(&dir.join("journal.db")).unwrap(), clock(), "dev").await.unwrap();
    let project = joined(dir, &factory).await;
    (factory, project)
}

/// The project's journal in `dir`, drawing from `factory`'s giver.
async fn joined(dir: &Path, factory: &Journal) -> Journal {
    let db = Db::open(&dir.join("project.db")).unwrap();
    Journal::open_drawing(db, clock(), "dev", journal::of_project("shop"), &factory.giver()).await.unwrap()
}

async fn say(journal: &Journal) -> u64 {
    journal.emit("said", Source::default(), json!({})).await.unwrap().seq
}

async fn say_many(journal: &Journal, many: u64) -> Vec<u64> {
    let wrote = journal.write(move |writing| {
        (0..many).map(|_| writing.say("said", Source::default(), json!({}))).collect::<Result<Vec<u64>, _>>()
    });
    wrote.await.unwrap().0
}

async fn mark(journal: &Journal) -> Option<u64> {
    journal.kv_get(DRAWN.0, DRAWN.1).await.unwrap().and_then(|kept| kept.as_u64())
}

async fn numbers(journal: &Journal) -> Vec<u64> {
    let all = EventQuery { limit: usize::MAX, ..Default::default() };
    journal.read(all).await.unwrap().iter().map(|event| event.seq).collect()
}

async fn closed(journal: Journal) {
    journal.db().close();
    journal.db().closed().await;
}

/// Completes once the database has done everything asked of it before
/// this: what a journal asked of it and did not wait for, among it.
async fn settled(db: &Db) {
    db.call(|_| Ok(())).await.unwrap();
}

/// Holds a database's thread until it is let go, as work that takes long
/// would: everything asked of it meanwhile waits. Gives what lets it go.
async fn hold(db: &Db) -> mpsc::Sender<()> {
    let (go, gone) = mpsc::channel::<()>();
    let (holding, held) = tokio::sync::oneshot::channel();
    let work = db.call_apart(move |_| {
        let _ = holding.send(());
        let _ = gone.recv();
        Ok(())
    });
    tokio::spawn(work);
    held.await.unwrap();
    go
}

/// What a daemon killed at this moment leaves of both journals, copied
/// to `to` while they are open.
fn left_by_a_stop(dir: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for file in ["journal.db", "journal.db-wal", "project.db", "project.db-wal"] {
        if dir.join(file).exists() {
            std::fs::copy(dir.join(file), to.join(file)).unwrap();
        }
    }
}

#[tokio::test]
async fn the_mark_above_a_run_drawn_from_another_file_is_on_the_disk_before_the_run_is_used() {
    let dir = scratch("to-disk");
    let (factory, project) = two(&dir).await;
    assert_eq!(factory.db().kept_to_the_disk(), 0);

    // The first thing a project's journal says draws its first run, and
    // the factory's database has held the mark to the disk by then.
    assert_eq!(say(&project).await, 1);
    assert_eq!(factory.db().kept_to_the_disk(), 1);
    assert_eq!(mark(&factory).await, Some(RUN));
    assert_eq!(mark(&project).await, None, "the mark is the factory's to keep");
    // What the factory's own journal draws is kept with its own events,
    // in one file and one log, and no commit is held to the disk for it.
    assert_eq!(say(&factory).await, RUN + 1);
    assert_eq!(factory.db().kept_to_the_disk(), 1);
    assert_eq!(project.db().kept_to_the_disk(), 0, "nothing of the project's own is");

    // A commit that is held to the disk is made with the database told
    // to hold its log to the disk, and the next is made as every other.
    let held = |db: &Db| db.call_to_the_disk(|connection| synchronous(connection));
    let usual = |db: &Db| db.call(|connection| synchronous(connection));
    assert_eq!((usual(factory.db()).await.unwrap(), held(factory.db()).await.unwrap()), (1, 2));
    assert_eq!(usual(factory.db()).await.unwrap(), 1);
    assert_eq!(factory.db().kept_to_the_disk(), 2);
}

fn synchronous(connection: &rusqlite::Connection) -> rusqlite::Result<i64> {
    connection.query_row("PRAGMA synchronous", [], |row| row.get(0))
}

#[tokio::test]
async fn a_journal_in_a_file_of_its_own_draws_ahead_and_does_not_wait_for_the_factorys_thread() {
    let dir = scratch("ahead");
    let (factory, project) = two(&dir).await;
    // Short of half its first run, nothing more is asked of the factory.
    let mut given = say_many(&project, RUN / 2 - 1).await;
    settled(factory.db()).await;
    assert_eq!((factory.db().kept_to_the_disk(), mark(&factory).await), (1, Some(RUN)));
    // With half of it given the next is asked for, and nobody waits.
    given.push(say(&project).await);
    settled(factory.db()).await;
    assert_eq!((factory.db().kept_to_the_disk(), mark(&factory).await), (2, Some(2 * RUN)));

    // The factory's thread is busy with something long: the project's
    // journal goes over the end of its run and on into the next.
    let go = hold(factory.db()).await;
    let over = tokio::time::timeout(PATIENCE, async {
        let mut given = say_many(&project, RUN / 2 - 3).await;
        for _ in 0..6 {
            given.push(say(&project).await);
        }
        given
    });
    given.extend(over.await.expect("the project's journal waited for the factory's thread"));
    assert_eq!(given, (1..=RUN + 3).collect::<Vec<_>>(), "its runs are end to end where nobody drew between");
    go.send(()).unwrap();
    assert_eq!(numbers(&project).await, given);
    // How long its thread stood still for a run is measured: once for
    // the first, which it waited for, and for the next not at all.
    let drew =
        MetricQuery { name: "db.draw".into(), since_ms: project.now_ms() - 60_000, ..Default::default() };
    assert_eq!(project.query_metrics(drew).await.unwrap()[0].total.count, 2);
}

#[tokio::test]
async fn nothing_is_given_twice_wherever_two_files_are_stopped_about_a_run_drawn_ahead() {
    let dir = scratch("two-stopped");
    let (factory, project) = two(&dir).await;
    // To a few short of where the next run is asked for, then one at a
    // time over that; then to a few short of the end of the run, and
    // one at a time over that, the factory's own saying something in
    // between. Before each, what a daemon killed there would leave.
    let mut stops = Vec::new();
    let (mut of_project, mut of_factory) = (Vec::new(), Vec::new());
    for up_to in [RUN / 2 - 2, RUN - 2] {
        let many = up_to - of_project.len() as u64;
        of_project.extend(say_many(&project, many).await);
        for stop in 0..5 {
            settled(factory.db()).await;
            let left = dir.join(format!("left-{up_to}-{stop}"));
            left_by_a_stop(&dir, &left);
            stops.push((left, of_project.clone(), of_factory.clone()));
            of_project.push(say(&project).await);
            if stop % 2 == 1 {
                of_factory.push(say(&factory).await);
            }
        }
    }
    assert_eq!(stops.len(), 10);

    for (left, of_project, of_factory) in stops {
        let (factory, project) = two(&left).await;
        assert_eq!(numbers(&project).await, of_project, "what the project's was answered is there");
        assert_eq!(numbers(&factory).await, of_factory, "and what the factory's was");
        // How far the giver had given was kept above everything either holds.
        let highest = of_project.iter().chain(&of_factory).copied().max().unwrap_or(0);
        let kept = mark(&factory).await.unwrap();
        assert!(kept >= highest, "the mark kept is {kept}, and {highest} was given");
        // And each goes on above all of it, the project's first or the factory's.
        let next = [say(&project).await, say(&factory).await, say(&project).await];
        assert!(next.iter().all(|number| *number > kept), "{next:?} after a mark of {kept}");
        assert!(next[0] != next[1] && next[2] == next[0] + 1);
        closed(project).await;
        closed(factory).await;
    }
}

#[tokio::test]
async fn what_is_said_though_shut_waits_for_no_database_that_is_doing_nothing_more() {
    let dir = scratch("last");
    let (factory, project) = two(&dir).await;
    // The factory's journal shuts, and its database does nothing more:
    // the project's has no run in hand, and what says that the daemon
    // stopped draws one all the same.
    factory.shut_now();
    let last = |journal: &Journal| {
        let journal = journal.clone();
        async move {
            let said = journal.emit_last("system.stopped", Source::default(), json!({}));
            tokio::time::timeout(PATIENCE, said)
                .await
                .expect("it waited for a database that is held")
                .unwrap()
                .seq
        }
    };
    assert_eq!(last(&project).await, 1);
    // What is said in the usual way meanwhile draws ahead at half a
    // run, which the factory's database never answers...
    let given = say_many(&project, RUN - 2).await;
    assert_eq!(given.last(), Some(&(RUN - 1)));
    // ...and the last thing said, at the end of the run and over it,
    // waits for that no more than for anything.
    assert_eq!([last(&project).await, last(&project).await], [RUN, 2 * RUN + 1]);
    assert_eq!(last(&factory).await, 3 * RUN + 1);
    closed(project).await;
    closed(factory).await;
    // The marks were kept though the database was held: started again,
    // neither gives a number that was given.
    let (factory, project) = two(&dir).await;
    assert!(mark(&factory).await.unwrap() >= 4 * RUN);
    assert!(say(&project).await > 3 * RUN + 1 && say(&factory).await > 3 * RUN + 1);
}

#[tokio::test]
async fn a_journal_that_joins_holding_numbers_takes_them_from_a_run_another_has_in_hand() {
    let dir = scratch("joins");
    // A journal that numbered its own, and holds more than the factory
    // has yet given.
    let held = {
        let journal =
            Journal::open(Db::open(&dir.join("project.db")).unwrap(), clock(), "dev").await.unwrap();
        let held = say_many(&journal, 500).await;
        closed(journal).await;
        held
    };
    let factory = Journal::open(Db::open(&dir.join("journal.db")).unwrap(), clock(), "dev").await.unwrap();
    // The factory's has a run in hand, the next of which the other holds.
    assert_eq!(say_many(&factory, 5).await, [1, 2, 3, 4, 5]);
    let project = joined(&dir, &factory).await;
    // It lets go of that run and draws another: nothing the joiner
    // holds is given, by either.
    assert_eq!(say(&factory).await, RUN + 1);
    assert_eq!(say(&project).await, 2 * RUN + 1);
    assert_eq!(say(&factory).await, RUN + 2);
    assert_eq!(numbers(&project).await, [held, vec![2 * RUN + 1]].concat());
    // What the factory's passed over is written down as a gap, for what
    // counts events by number: of seven events the newest three are
    // kept, and the fourth newest goes.
    let old = factory.now_ms() + 1;
    assert_eq!(factory.prune_events(old, 3).await.unwrap(), 4);
    assert_eq!(numbers(&factory).await, [5, RUN + 1, RUN + 2]);

    // One that joins holding nothing above what was drawn takes nothing
    // from anybody.
    let (other, giver) = (Db::open(&dir.join("other.db")).unwrap(), factory.giver());
    let other = Journal::open_drawing(other, clock(), "dev", journal::of_project("other"), &giver);
    let other = other.await.unwrap();
    assert_eq!(say(&factory).await, RUN + 3);
    assert_eq!(say(&other).await, 3 * RUN + 1);
}

#[tokio::test]
async fn a_write_in_a_file_of_its_own_that_keeps_nothing_gives_its_run_back_once() {
    let dir = scratch("back");
    let (factory, project) = two(&dir).await;
    // A write that fails having drawn the first run: its numbers are
    // the next write's, and the mark that was kept for them stands.
    let failed = project.write(|writing| {
        writing.say("said", Source::default(), json!({}))?;
        writing.transaction().execute("INSERT INTO nothing_there VALUES (1)", [])?;
        Ok(())
    });
    assert!(failed.await.is_err());
    assert_eq!((mark(&factory).await, project.last_seq()), (Some(RUN), 0));
    assert_eq!(say(&project).await, 1);

    // A commit that cannot be made, over the end of a run and into the
    // one that was drawn ahead.
    let made = "CREATE TABLE wholes (name TEXT PRIMARY KEY);
                CREATE TABLE parts (of TEXT NOT NULL REFERENCES wholes (name) DEFERRABLE INITIALLY DEFERRED);";
    project.db().call(move |connection| connection.execute_batch(made)).await.unwrap();
    assert_eq!(say_many(&project, RUN - 3).await.last(), Some(&(RUN - 2)));
    settled(factory.db()).await;
    let lost = project.write(|writing| {
        for _ in 0..4 {
            writing.say("said", Source::default(), json!({}))?;
        }
        writing.transaction().execute("INSERT INTO parts VALUES ('no whole')", []).map(|_| ())
    });
    assert!(lost.await.unwrap_err().message.contains("could not be kept"));
    // The numbers it gave were nobody's, and are given in order; the
    // factory's own, drawing meanwhile, is given none of them.
    let next = say_many(&project, 4).await;
    assert_eq!(next, [RUN - 1, RUN, RUN + 1, RUN + 2]);
    let of_factory = say(&factory).await;
    assert!(of_factory > 2 * RUN, "{of_factory}");
    let mut all = [numbers(&project).await, numbers(&factory).await].concat();
    let many = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), many, "a number was given twice");
}

#[tokio::test]
async fn a_run_given_back_by_work_that_failed_is_not_given_back_again_by_its_commit_from_under_who_drew_it_since()
 {
    // The order is made, and not waited for: the factory's thread is
    // held in the middle of one commit, between the work that drew a run
    // and failed and the end of a commit that cannot be made, and the
    // project's journal draws in that moment. A commit takes in what is
    // waiting only for so long, so on a machine with much else to do the
    // commit may be made before the moment comes: it is arranged again
    // until it has come about, and has to have.
    let mut came_about = false;
    for _ in 0..40 {
        let dir = scratch("back-once");
        let (factory, project) = two(&dir).await;
        let made = "CREATE TABLE wholes (name TEXT PRIMARY KEY);
                    CREATE TABLE parts (of TEXT NOT NULL REFERENCES wholes (name) DEFERRABLE INITIALLY DEFERRED);";
        factory.db().call(move |connection| connection.execute_batch(made)).await.unwrap();

        // Asked while the thread is held, so that all of it waits and is
        // kept by one commit: what makes the commit fail, the write that
        // draws the factory's first run and fails, and what holds the
        // thread there until it is let go.
        let go = hold(factory.db()).await;
        let lost = factory.db().call(|c| c.execute("INSERT INTO parts VALUES ('no whole')", []).map(|_| ()));
        let failed = factory.write(|writing| {
            writing.say("said", Source::default(), json!({}))?;
            writing.transaction().execute("INSERT INTO nothing_there VALUES (1)", []).map(|_| ())
        });
        let (there, is_there) = mpsc::channel::<()>();
        let (on, goes_on) = mpsc::channel::<()>();
        let between = factory.db().call(move |_| {
            let _ = there.send(());
            let _ = goes_on.recv();
            Ok(())
        });
        let rest: Vec<_> = (0..16).map(|_| factory.db().call(|_| Ok(()))).collect();
        go.send(()).unwrap();
        tokio::task::spawn_blocking(move || is_there.recv()).await.unwrap().unwrap();
        // The write has failed and given its run back: the project's
        // journal draws, and is handed those numbers.
        assert_eq!(factory.giver().drawn(), 0);
        let of_project = tokio::spawn({
            let project = project.clone();
            async move { say(&project).await }
        });
        tokio::time::timeout(PATIENCE, async {
            while factory.giver().drawn() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the project's journal never drew");
        on.send(()).unwrap();

        assert!(failed.await.is_err());
        let not_made = lost.await.is_err() && between.await.is_err();
        for one in rest {
            let _ = one.await;
        }
        let of_project = tokio::time::timeout(PATIENCE, of_project).await.unwrap().unwrap();
        let of_factory = say(&factory).await;
        // Whether or not the commit was the one that failed, no number
        // is given to both.
        assert_ne!(of_project, of_factory, "the factory's journal was given a number the project's has");
        assert_eq!((numbers(&project).await, numbers(&factory).await), (vec![of_project], vec![of_factory]));
        closed(project).await;
        closed(factory).await;
        // A program told to keep each piece of work by a commit of its
        // own (`JOURNAL_GATHERS=1`) makes no commit that holds them all:
        // what is above is held of it once, and the order it cannot come
        // to is not arranged again.
        if not_made || !crate::gathered::gathers() {
            came_about = true;
            break;
        }
    }
    assert!(came_about, "the commit that could not be made never held the work that drew");
}
