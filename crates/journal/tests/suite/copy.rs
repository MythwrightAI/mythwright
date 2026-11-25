//! A copy of the database, taken while the database is in use, and what
//! is then taken out of the copy.

use std::sync::Arc;

use journal::{Db, EventQuery, Journal, MetricQuery};
use kernel::{ManualClock, Source};
use serde_json::json;

use crate::scratch;

fn holds(bytes: &[u8], text: &str) -> bool {
    bytes.windows(text.len()).any(|window| window == text.as_bytes())
}

#[tokio::test]
async fn a_copy_is_taken_without_the_database_thread_and_what_is_removed_from_it_is_gone_from_the_file() {
    let dir = scratch("copy");
    let (original, copy) = (dir.join("journal.db"), dir.join("copy.db"));
    let clock = Arc::new(ManualClock::new());
    let journal = Journal::open(Db::open(&original).unwrap(), clock.clone(), "here").await.unwrap();
    journal.emit("build.finished", Source::default(), json!({ "said": "kept-in-the-copy" })).await.unwrap();
    journal.kv_set("cursor", "place", &json!("a-place-that-does-not-travel")).await.unwrap();
    journal.kv_set("forgotten", "by-name", &json!("a-thing-somebody-forgets")).await.unwrap();
    journal.kv_set("content", "columns", &json!(["state-that-travels"])).await.unwrap();
    // What this journal has yet to say of itself, which is not so of a copy.
    let (of_the_journal, noted) = journal::REINDEXED;
    journal.kv_set(of_the_journal, noted, &json!({ "note": "of-this-journal-alone" })).await.unwrap();
    // A table that nothing has said goes into a copy.
    let made = "CREATE TABLE secrets (said TEXT); INSERT INTO secrets VALUES ('a-row-nobody-spoke-for');";
    journal.db().call(move |connection| connection.execute_batch(made)).await.unwrap();
    journal.measure("build", &[("who", "a-label-that-does-not-travel")], 5.0, true);
    journal.flush_metrics().await.unwrap();

    // The thread that does the database's work is busy, and stays busy:
    // the copy is not its work, and does not wait for it.
    let (release, held) = std::sync::mpsc::channel::<()>();
    let busy = journal.db().call(move |_| {
        let _ = held.recv();
        Ok(())
    });
    let (from, to) = (original.clone(), copy.clone());
    tokio::task::spawn_blocking(move || journal::copy::take(&from, &to)).await.unwrap().unwrap();
    release.send(()).unwrap();
    busy.await.unwrap();
    // A copy is not written over something that is there.
    let (from, to) = (original.clone(), copy.clone());
    assert!(tokio::task::spawn_blocking(move || journal::copy::take(&from, &to)).await.unwrap().is_err());

    // What does not travel is taken out of the copy: the measurements,
    // what a crate forgets by name, and whatever nobody said travels.
    let connection = journal::copy::open(&copy).unwrap();
    // (Four: what the journal was first made of, what hearing an event
    // takes, one event in many in the index of when, and which projects
    // have a journal of their own.)
    assert_eq!(journal::copy::versions(&connection).unwrap(), [("journal".to_string(), 5)]);
    // (The database's own work is measured too, beside what was built.)
    assert!(journal::copy::leave_out(&connection).unwrap() >= 1);
    assert_eq!(journal::copy::forget(&connection, "forgotten").unwrap(), 1);
    assert_eq!(journal::copy::keep_state(&connection, &["content"]).unwrap(), 1);
    assert_eq!(journal::copy::keep_tables(&connection, journal::copy::TABLES).unwrap(), ["secrets"]);
    // Done again, there is nothing more to take.
    assert_eq!(journal::copy::leave_out(&connection).unwrap(), 0);
    assert_eq!(journal::copy::keep_state(&connection, &["content"]).unwrap(), 0);
    journal::copy::note(&connection, "content", "from", &json!("noted")).unwrap();
    journal::copy::seal(connection).unwrap();

    // It is one file with nothing beside it, and what was removed is not
    // in its bytes anywhere, used or not.
    let mut beside: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("copy.db"))
        .collect();
    beside.sort();
    assert_eq!(beside, ["copy.db"]);
    let bytes = std::fs::read(&copy).unwrap();
    assert!(holds(&bytes, "kept-in-the-copy") && holds(&bytes, "state-that-travels"));
    for removed in [
        "a-place-that-does-not-travel",
        "a-thing-somebody-forgets",
        "a-label-that-does-not-travel",
        "a-row-nobody-spoke-for",
        "of-this-journal-alone",
    ] {
        assert!(!holds(&bytes, removed), "`{removed}` is still in the file");
    }

    // Opened as a journal it is one: what happened, numbered as it was.
    let copied = Journal::open(Db::open(&copy).unwrap(), clock, "there").await.unwrap();
    assert_eq!(copied.last_seq(), 1);
    assert_eq!(copied.read(EventQuery::default()).await.unwrap()[0].data["said"], "kept-in-the-copy");
    assert_eq!(copied.kv_get("content", "from").await.unwrap(), Some(json!("noted")));
    assert_eq!(copied.kv_get("cursor", "place").await.unwrap(), None);
    assert_eq!(copied.kv_get(of_the_journal, noted).await.unwrap(), None);
    let query = || MetricQuery { name: "build".into(), ..MetricQuery::default() };
    assert!(copied.query_metrics(query()).await.unwrap().is_empty());

    // And the database it was taken from is as it was.
    assert_eq!(journal.kv_get("cursor", "place").await.unwrap(), Some(json!("a-place-that-does-not-travel")));
    assert_eq!(journal.query_metrics(query()).await.unwrap()[0].total.count, 1);
    let _ = std::fs::remove_dir_all(dir);
}
