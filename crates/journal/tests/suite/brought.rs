//! The log beside the database, brought back into it off the writer's
//! turn: by another thread as it grows, by the writer itself at a bound,
//! and by nobody while the journal is at rest.

use std::path::Path;
use std::time::Duration;

use journal::Db;
use journal::checkpoint::Kept;

use crate::scratch;

/// Waits for `is`, giving way to whatever else is running meanwhile, and
/// says what it waited for where that never came.
async fn until(what: &str, mut is: impl FnMut() -> bool) {
    let waited = tokio::time::timeout(Duration::from_secs(40), async {
        while !is() {
            tokio::task::yield_now().await;
        }
    });
    waited.await.unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

async fn filled(path: &Path, kept: Kept) -> Db {
    let db = Db::open_keeping(path, kept).unwrap();
    db.call(|connection| {
        connection.execute_batch("CREATE TABLE filled (n INTEGER PRIMARY KEY, said TEXT NOT NULL);")
    })
    .await
    .unwrap();
    db
}

/// One write, of a page or two.
async fn write(db: &Db) {
    db.call(|connection| connection.execute("INSERT INTO filled (said) VALUES (?1)", ["said ".repeat(300)]))
        .await
        .unwrap();
}

/// How many files this process holds open under `dir`, as the system says.
#[cfg(target_os = "linux")]
fn open_under(dir: &Path) -> usize {
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .flatten()
        .filter(|entry| std::fs::read_link(entry.path()).is_ok_and(|to| to.starts_with(dir)))
        .count()
}

#[tokio::test]
async fn a_log_that_has_grown_is_brought_back_by_another_thread_and_a_database_at_rest_is_left_alone() {
    let dir = scratch("brought");
    let db = filled(&dir.join("journal.db"), Kept { ask: 40, bound: 1_000_000 }).await;
    assert_eq!(db.brought_back(), 0);

    // Written to until the log has grown as far as somebody is told of.
    while db.log_pages() < 40 {
        write(&db).await;
    }
    // Nothing is asked of the writer for it: it is another thread's.
    until("the log to be brought back", || db.brought_back() == 1).await;
    // All of it was, with nothing being written meanwhile: so the next
    // write begins the log again, and it is as long as that write.
    write(&db).await;
    assert!(db.log_pages() < 40, "the log began again, and is {} pages", db.log_pages());

    // At rest, nobody is told and nothing is written, by either thread.
    let (brought, writes) = (db.brought_back(), db.writes());
    for _ in 0..200 {
        tokio::task::yield_now().await;
    }
    assert_eq!((db.brought_back(), db.writes()), (brought, writes));

    // Closed, the thread that brought it back has let the file go as well.
    db.close();
    db.closed().await;
    #[cfg(target_os = "linux")]
    assert_eq!(open_under(&dir), 0, "a database that is closed still holds a file");
}

#[tokio::test]
async fn a_log_nobody_else_brings_back_is_brought_back_by_the_writer_at_the_bound() {
    let dir = scratch("bound");
    // Nobody is ever told: as where the other thread could not be had.
    let db = filled(&dir.join("journal.db"), Kept { ask: 1_000_000, bound: 60 }).await;
    while db.log_pages() < 60 {
        write(&db).await;
    }
    // Brought back by the writer itself, before the next thing it does.
    write(&db).await;
    assert_eq!(db.brought_back(), 1);
    assert!(db.log_pages() < 60, "the log began again, and is {} pages", db.log_pages());
    // And at the bound each time, for as long as it is written to.
    for _ in 0..400 {
        write(&db).await;
        assert!(db.log_pages() < 60 + 8, "the log is {} pages", db.log_pages());
    }
    assert!(db.brought_back() > 2);
}

#[tokio::test]
async fn a_reader_that_holds_the_log_holds_no_writer_and_the_log_is_brought_back_when_it_lets_go() {
    let dir = scratch("reader");
    let path = dir.join("journal.db");
    // Nobody but the writer brings the log back here. With the other
    // thread told as well, it could be in the middle of bringing back
    // what it was told of at the start, for as long as the disk took,
    // when the reader let go: the writer then does what it can and looks
    // again later, which is right, and on a busy machine that was later
    // than the writes this test makes afterwards, once in four hundred.
    let kept = Kept { ask: 1_000_000, bound: 60 };
    let db = filled(&path, kept).await;
    write(&db).await;

    // Somebody else reads, and stays where they began reading.
    let reader = rusqlite::Connection::open(&path).unwrap();
    reader.execute_batch("BEGIN").unwrap();
    let seen: i64 = reader.query_row("SELECT count(*) FROM filled", [], |row| row.get(0)).unwrap();
    assert_eq!(seen, 1);

    // What they began before cannot be brought back past, so the log
    // grows beyond the bound: and every write is answered all the same.
    let written = tokio::time::timeout(Duration::from_secs(40), async {
        for _ in 0..300 {
            write(&db).await;
        }
    });
    written.await.expect("a reader held the writer");
    assert!(db.log_pages() > 2 * kept.bound, "the log is {} pages", db.log_pages());

    // They let go, and the writer brings it back the next time it looks.
    reader.execute_batch("COMMIT").unwrap();
    drop(reader);
    for _ in 0..400 {
        write(&db).await;
        if db.log_pages() < kept.bound {
            break;
        }
    }
    assert!(db.log_pages() < kept.bound, "the log is {} pages", db.log_pages());
    // The file it grew to is cut back to the bound with it.
    let log = std::fs::metadata(dir.join("journal.db-wal")).unwrap().len();
    assert!(log <= (kept.bound as u64 + 2) * 2048 + 4096, "the log's file is {log} bytes");
}

#[tokio::test]
async fn a_copy_taken_while_the_log_is_being_brought_back_is_whole() {
    let dir = scratch("copied");
    let (original, copy) = (dir.join("journal.db"), dir.join("copy.db"));
    let db = filled(&original, Kept { ask: 10, bound: 50 }).await;
    for _ in 0..100 {
        write(&db).await;
    }
    // Written to, told and brought back all the while the copy is taken.
    let writing = {
        let db = db.clone();
        tokio::spawn(async move {
            for _ in 0..400 {
                write(&db).await;
            }
        })
    };
    let (from, to) = (original.clone(), copy.clone());
    tokio::task::spawn_blocking(move || journal::copy::take(&from, &to)).await.unwrap().unwrap();
    writing.await.unwrap();
    // Counted by whichever thread brought it back, once it has: the other
    // thread may still be at the first of it, holding the disk, with the
    // writer turned away each time it looked meanwhile.
    until("the log to be brought back", || db.brought_back() > 0).await;

    let taken = journal::copy::open(&copy).unwrap();
    let sound: String = taken.query_row("PRAGMA quick_check", [], |row| row.get(0)).unwrap();
    assert_eq!(sound, "ok");
    let rows: i64 = taken.query_row("SELECT count(*) FROM filled", [], |row| row.get(0)).unwrap();
    assert!((100..=500).contains(&rows), "the copy holds {rows} rows");
    // Every row it holds is whole, and none is missing before its last.
    let last: i64 = taken.query_row("SELECT max(n) FROM filled", [], |row| row.get(0)).unwrap();
    assert_eq!(last, rows);
}
