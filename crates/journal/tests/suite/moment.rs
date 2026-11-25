//! Every journal of a factory read as of one moment: what each file
//! written from the moment holds, that a project's thread waiting on the
//! factory's for its first numbers does not keep the moment from coming,
//! and that a thread which never comes between two pieces of work is not
//! waited for past what was said.

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use journal::copy::Connection;
use journal::giver::{DRAWN, RUN};
use journal::moment::Moment;
use journal::{Db, Journal};
use kernel::{Clock, ErrorKind, ManualClock, Source};
use serde_json::json;

use crate::scratch;

/// As long as anything here is waited for: what does not come by then
/// never would.
const PATIENCE: Duration = Duration::from_secs(40);

/// The factory's journal in `dir`, keeping its projects' in a folder there.
async fn factory(dir: &Path) -> Journal {
    let clock: Arc<dyn Clock> = Arc::new(ManualClock::new());
    let opened = Journal::open(Db::open(&dir.join("journal.db")).unwrap(), clock, "dev").await.unwrap();
    opened.keeps_projects_in(dir.join("journals")).await.unwrap();
    opened
}

/// Says one thing in `journal`, in one piece of its database's work.
async fn say(journal: &Journal, topic: &'static str) -> u64 {
    let wrote = journal.write(move |writing| writing.say(topic, Source::default(), json!({})));
    wrote.await.unwrap().0
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

/// Completes once `so` is so, giving way to whatever is to make it so.
async fn until(what: &str, so: impl Fn() -> bool) {
    let waited = tokio::time::timeout(PATIENCE, async {
        while !so() {
            tokio::task::yield_now().await;
        }
    });
    waited.await.unwrap_or_else(|_| panic!("never came about: {what}"));
}

/// The factory read as of one moment, off the thread the test runs on.
async fn moment(of_factory: &Journal) -> kernel::Result<Moment> {
    let of_factory = of_factory.clone();
    let asked = tokio::task::spawn_blocking(move || of_factory.moment());
    tokio::time::timeout(PATIENCE, asked).await.expect("the factory was never read").unwrap()
}

/// What a file written from a moment says was said in it, and the mark
/// it keeps where it is the factory's: a database that is whole.
fn written(file: &Path) -> (Vec<String>, Vec<i64>, Option<i64>) {
    let copy = journal::copy::open(file).unwrap();
    let sound: String = copy.query_row("PRAGMA quick_check", [], |row| row.get(0)).unwrap();
    assert_eq!(sound, "ok", "{}", file.display());
    let column = |sql: &str| -> Vec<String> {
        let mut statement = copy.prepare(sql).unwrap();
        statement.query_map([], |row| row.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
    };
    let numbers = column("SELECT CAST(seq AS TEXT) FROM events ORDER BY seq");
    let mark =
        column(&format!("SELECT value FROM kv WHERE namespace = '{}' AND key = '{}'", DRAWN.0, DRAWN.1));
    (
        column("SELECT topic FROM events ORDER BY seq"),
        numbers.iter().map(|seq| seq.parse().unwrap()).collect(),
        mark.first().map(|mark| mark.parse().unwrap()),
    )
}

async fn closed(factory: Journal) {
    for db in factory.dbs() {
        db.close();
        db.closed().await;
    }
}

#[tokio::test]
async fn every_file_written_from_a_moment_holds_what_was_said_by_then_and_nothing_said_since() {
    if crate::one_journal() {
        return;
    }
    let dir = scratch("moment");
    let of_factory = factory(&dir).await;
    let shop = of_factory.own("shop").await.unwrap();
    let yard = of_factory.own("yard").await.unwrap();
    for journal in [&of_factory, &shop, &yard] {
        say(journal, "said.before").await;
    }

    let read = moment(&of_factory).await.unwrap();
    let listed: Vec<&str> = read.beside.iter().map(|(listed, _)| listed.file.as_str()).collect();
    assert_eq!(listed, ["shop.db", "yard.db"], "each journal the factory lists, by its file");
    // Every thread is let go by now: each says something more, and a
    // project is given a journal, while the files are still to be written.
    for journal in [&of_factory, &shop, &yard] {
        say(journal, "said.since").await;
    }
    of_factory.own("den").await.unwrap();

    let to = dir.join("copy");
    std::fs::create_dir_all(&to).unwrap();
    let Moment { factory: own, beside, .. } = read;
    let wrote = tokio::task::spawn_blocking({
        let to = to.clone();
        move || -> kernel::Result<()> {
            own.write(&to.join("database.db"))?;
            for (listed, read) in beside {
                read.write(&to.join(listed.file))?;
            }
            Ok(())
        }
    });
    wrote.await.unwrap().unwrap();

    let (said, numbers, mark) = written(&to.join("database.db"));
    assert_eq!(said, ["said.before"]);
    let mark = mark.expect("the giver's mark is in the factory's file");
    assert!(numbers.iter().all(|seq| *seq <= mark));
    for file in ["shop.db", "yard.db"] {
        let (said, numbers, _) = written(&to.join(file));
        assert_eq!(said, ["said.before"], "{file}");
        assert!(numbers.iter().all(|seq| *seq <= mark), "{file} holds {numbers:?}, and the mark is {mark}");
    }
    // What was listed since is not in what was read: the copy lists two.
    let copy = Connection::open(to.join("database.db")).unwrap();
    assert_eq!(journal::listed(&copy).unwrap().len(), 2);
    // Each is one file with nothing beside it, once it has been opened
    // as a copy is and closed.
    let mut files: Vec<String> = std::fs::read_dir(&to)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    assert_eq!(files, ["database.db", "shop.db", "yard.db"]);

    // A file is never written over something that is there.
    let again = moment(&of_factory).await.unwrap();
    let over = to.join("database.db");
    let refused = tokio::task::spawn_blocking(move || again.factory.write(&over)).await.unwrap();
    assert!(refused.is_err());
    // And a project's journal is not what a factory is read from.
    assert_eq!(shop.moment().err().map(|err| err.kind), Some(ErrorKind::Invalid));
    closed(of_factory).await;
}

#[tokio::test]
async fn a_factory_is_read_at_one_moment_while_a_projects_thread_waits_on_the_factorys_for_its_first_numbers()
{
    if crate::one_journal() {
        return;
    }
    let dir = scratch("moment-drawing");
    let of_factory = factory(&dir).await;
    let shop = of_factory.own("shop").await.unwrap();
    say(&of_factory, "said.before").await;

    // The project's thread is at something long, and behind that is the
    // first thing its journal says: which draws its first run of numbers
    // in the middle of the write, and waits there for the factory's
    // thread to have kept the mark.
    let go = hold(shop.db()).await;
    let first = tokio::spawn({
        let shop = shop.clone();
        async move { say(&shop, "said.first").await }
    });
    until("the project's first event is asked of its thread", || shop.db().waiting() == 1).await;
    // The factory is read now. The project's thread is asked to stand
    // still behind that write, and the factory's is not asked until it
    // does: parked first, the factory's would never keep the mark, and
    // neither thread would ever move.
    let asked = tokio::task::spawn_blocking({
        let of_factory = of_factory.clone();
        move || of_factory.moment()
    });
    until("the project's thread is asked to stand still", || shop.db().waiting() == 2).await;
    go.send(()).unwrap();
    let read = tokio::time::timeout(PATIENCE, asked)
        .await
        .expect("the factory's thread was parked under a project's thread that was waiting for it")
        .unwrap()
        .unwrap();
    let said = tokio::time::timeout(PATIENCE, first).await.expect("the project never said it").unwrap();
    assert_eq!(said, RUN + 1, "its first run, drawn above the factory's own");

    // What it said then was said before the moment, with numbers the
    // factory's file has the mark above.
    let to = dir.join("copy");
    std::fs::create_dir_all(&to).unwrap();
    let Moment { factory: own, mut beside, .. } = read;
    let (_, of_shop) = beside.pop().unwrap();
    let wrote = tokio::task::spawn_blocking({
        let to = to.clone();
        move || own.write(&to.join("database.db")).and_then(|()| of_shop.write(&to.join("shop.db")))
    });
    wrote.await.unwrap().unwrap();
    let (_, _, mark) = written(&to.join("database.db"));
    let (said, numbers, _) = written(&to.join("shop.db"));
    assert_eq!((said, numbers), (vec!["said.first".to_string()], vec![RUN as i64 + 1]));
    assert_eq!(mark, Some(2 * RUN as i64));
    closed(of_factory).await;
}

#[tokio::test]
async fn a_thread_that_does_not_come_between_two_pieces_of_work_is_not_waited_for_past_what_was_said() {
    if crate::one_journal() {
        return;
    }
    let dir = scratch("moment-bound");
    let of_factory = factory(&dir).await;
    let shop = of_factory.own("shop").await.unwrap();
    say(&shop, "said.before").await;

    // The project's thread is at something long, and the factory is read
    // by somebody who waits for nothing.
    let go = hold(shop.db()).await;
    let asked = tokio::task::spawn_blocking({
        let of_factory = of_factory.clone();
        move || of_factory.moment_within(Duration::ZERO)
    });
    let refused =
        asked.await.unwrap().err().expect("a thread that was held was read between two pieces of work");
    assert_eq!(refused.kind, ErrorKind::Unavailable, "asking again may do: {refused}");
    assert!(
        refused.message.contains("the journal `project:shop`")
            && refused.message.contains("did not come between two pieces of its work")
            && refused.message.contains("ask again"),
        "{refused}"
    );
    // Nothing is left standing still for it: the factory's own thread
    // was never asked, and the project's, once it has done what held it,
    // goes on with what it is asked next.
    tokio::time::timeout(PATIENCE, say(&of_factory, "said.since"))
        .await
        .expect("the factory's thread is held");
    go.send(()).unwrap();
    tokio::time::timeout(PATIENCE, say(&shop, "said.since")).await.expect("the project's thread is held");
    // And asked again, the factory is read.
    assert_eq!(moment(&of_factory).await.unwrap().beside.len(), 1);
    closed(of_factory).await;
}
