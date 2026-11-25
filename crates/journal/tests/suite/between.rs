//! What was measured, read while it is being written down: it is in
//! memory or in the database, and is counted once.
//!
//! A flush took what it was to write out of memory before its write was
//! on the database's thread, and a reading read what is stored and then
//! what is in memory: a reading whose look at the database came before
//! the write, and whose look at memory came after the taking, found what
//! was being written in neither. A comparison once answered three turns
//! measured where six were meant. The order is made here, and not left
//! to which thread is quicker: the database does its work in the order
//! it was asked, and is kept at a first piece until the test lets it go,
//! with the reading and the flush asked behind it.

use journal::{Journal, MetricQuery};

fn turns() -> MetricQuery {
    MetricQuery { name: "turn".into(), ..Default::default() }
}

async fn counted(journal: &Journal) -> u64 {
    journal.query_metrics(turns()).await.unwrap().first().map_or(0, |series| series.total.count)
}

/// Has `asked` go as far as it goes without the database, which is to
/// put its piece of work in the database's queue: it is not answered,
/// since the database is kept at something else.
async fn queued<F: std::future::Future>(asked: std::pin::Pin<&mut F>) {
    tokio::select! {
        biased;
        _ = asked => panic!("it was answered with the database kept at something else"),
        () = std::future::ready(()) => {}
    }
}

/// Six turns measured, three of them written down already and three not.
async fn six_turns() -> Journal {
    let (journal, _clock) = crate::journal().await;
    for _ in 0..3 {
        journal.measure("turn", &[], 1.0, true);
    }
    journal.flush_metrics().await.unwrap();
    for _ in 0..3 {
        journal.measure("turn", &[], 1.0, true);
    }
    assert_eq!(counted(&journal).await, 6);
    journal
}

/// Keeps the database at a piece of work until what is handed back is
/// let go, once it is at it.
async fn kept(journal: &Journal) -> (std::sync::mpsc::Sender<()>, impl std::future::Future<Output = ()>) {
    let (go, kept) = std::sync::mpsc::channel::<()>();
    let (at, at_it) = tokio::sync::oneshot::channel::<()>();
    let held = journal.db().call_apart(move |_| {
        let _ = at.send(());
        let _ = kept.recv();
        Ok(())
    });
    at_it.await.expect("the database is at the piece that keeps it");
    (go, async move { held.await.unwrap() })
}

#[tokio::test]
async fn what_is_being_written_down_is_counted_by_a_reading_that_looked_at_the_database_before_the_write() {
    let journal = six_turns().await;
    let (go, held) = kept(&journal).await;
    assert_eq!(journal.db().waiting(), 0);

    // The reading is asked first, and its look at the database is behind
    // the piece that keeps it; the flush is asked behind the reading.
    let reading = counted(&journal);
    tokio::pin!(reading);
    queued(reading.as_mut()).await;
    let flushing = journal.flush_metrics();
    tokio::pin!(flushing);
    queued(flushing.as_mut()).await;
    assert_eq!(journal.db().waiting(), 2, "the reading and the flush are each waiting, in that order");

    go.send(()).unwrap();
    held.await;
    let (read, flushed) = tokio::join!(reading, flushing);
    flushed.unwrap();
    assert_eq!(read, 6, "what the flush was to write is counted by the reading before it");
    // And once it is written down, it is counted once.
    assert_eq!(counted(&journal).await, 6, "what was written is counted once");
}

#[tokio::test]
async fn what_was_written_down_is_counted_once_by_a_reading_behind_the_write() {
    let journal = six_turns().await;
    let (go, held) = kept(&journal).await;

    let flushing = journal.flush_metrics();
    tokio::pin!(flushing);
    queued(flushing.as_mut()).await;
    let reading = counted(&journal);
    tokio::pin!(reading);
    queued(reading.as_mut()).await;

    go.send(()).unwrap();
    held.await;
    let (flushed, read) = tokio::join!(flushing, reading);
    flushed.unwrap();
    assert_eq!(read, 6, "what was written is read from the database, and is no longer in memory");
    assert_eq!(counted(&journal).await, 6);
}

#[tokio::test]
async fn what_a_write_could_not_keep_is_counted_from_memory_and_written_by_the_next() {
    let journal = six_turns().await;
    // The database refuses what the flush writes.
    journal
        .db()
        .call(|connection| {
            connection.execute_batch(
                "CREATE TRIGGER refused BEFORE INSERT ON metrics BEGIN SELECT RAISE(ABORT, 'refused'); END",
            )
        })
        .await
        .unwrap();
    assert!(journal.flush_metrics().await.is_err());
    assert_eq!(counted(&journal).await, 6, "what was not kept is in memory again");

    journal.db().call(|connection| connection.execute_batch("DROP TRIGGER refused")).await.unwrap();
    journal.flush_metrics().await.unwrap();
    assert_eq!(counted(&journal).await, 6, "and once it is kept, it is counted once");
    let stored: i64 = journal
        .db()
        .call(|connection| {
            connection.query_row("SELECT sum(count) FROM metrics WHERE name = 'turn'", [], |row| row.get(0))
        })
        .await
        .unwrap();
    assert_eq!(stored, 6, "all of it is written down");
}
