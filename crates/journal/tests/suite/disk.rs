//! A database made for a test does not wait for the disk, and one that
//! says nothing waits as it always did: which is every test here but
//! these, and so every test of the disk itself (`drawn.rs`, `given.rs`,
//! `brought.rs`).

use std::sync::Arc;

use journal::{Db, Disk, Journal};
use kernel::{Clock, ManualClock, Source};
use serde_json::json;

use crate::beside::closed;
use crate::scratch;

fn clock() -> Arc<dyn Clock> {
    Arc::new(ManualClock::new())
}

fn synchronous(connection: &mut rusqlite::Connection) -> rusqlite::Result<i64> {
    connection.query_row("PRAGMA synchronous", [], |row| row.get(0))
}

#[tokio::test]
async fn a_database_opened_for_a_test_waits_for_the_disk_nowhere_and_keeps_what_it_is_given() {
    let dir = scratch("unwaited");
    let db = Db::open_unwaited(&dir.join("journal.db")).unwrap();
    assert_eq!(db.disk(), Disk::Unwaited);
    let factory = Journal::open(db, clock(), "dev").await.unwrap();
    factory.keeps_projects_in(dir.join("journals")).await.unwrap();
    // A project's journal is opened as the factory's own was.
    let project = factory.own("shop").await.unwrap();
    assert_eq!(project.db().disk(), Disk::Unwaited);

    // Neither is told to hold anything to the disk: not as every other
    // commit is made, and not for one that asks to be on the disk before
    // it is answered, which is counted as held nowhere.
    for db in [factory.db(), project.db()] {
        assert_eq!(db.call(synchronous).await.unwrap(), 0);
        assert_eq!(db.call_to_the_disk(synchronous).await.unwrap(), 0);
        assert_eq!(db.call(synchronous).await.unwrap(), 0);
    }
    // What a project's journal says draws its numbers from the
    // factory's, whose mark would be held to the disk first.
    let said = project.emit("said", Source::default(), json!({ "project": "shop" })).await.unwrap().seq;
    assert_eq!(factory.db().kept_to_the_disk(), 0);

    // What was written is there for whoever opens the files next,
    // waiting for the disk or not: a file says nothing of how it was held.
    closed(factory).await;
    let db = Db::open(&dir.join("journal.db")).unwrap();
    assert_eq!(db.disk(), Disk::Waited);
    let factory = Journal::open(db, clock(), "dev").await.unwrap();
    factory.keeps_projects_in(dir.join("journals")).await.unwrap();
    let project = factory.of(Some("shop"));
    assert_eq!(project.db().disk(), Disk::Waited);
    assert_eq!(crate::beside::numbers(&project).await, vec![said]);
    assert_eq!(project.db().call(synchronous).await.unwrap(), 1);
    closed(factory).await;
}

#[tokio::test]
async fn a_database_that_says_nothing_waits_for_the_disk_as_a_factory_that_is_run_does() {
    let dir = scratch("waited");
    let db = Db::open(&dir.join("journal.db")).unwrap();
    assert_eq!(db.disk(), Disk::Waited);
    assert_eq!(db.call(synchronous).await.unwrap(), 1);
    assert_eq!(db.call_to_the_disk(synchronous).await.unwrap(), 2);
    assert_eq!(db.kept_to_the_disk(), 1);
    assert_eq!(Db::memory().unwrap().disk(), Disk::Waited);
}
