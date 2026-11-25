//! A table declared again asks its rows nothing where nothing of a field
//! changed, and what is asked otherwise is a statement over the rows and
//! never a reading of all of them (`studio::held`); how many records a
//! table has is kept by the daemon (`studio::counted`).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Map, Value, json};
use studio::collections::Spec;

use crate::{Rig, ada};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// A table's fields: words, a number, a choice that says what nothing
/// stands for, and a field each record has to say.
fn fields() -> Value {
    json!({
        "title": { "type": "text", "required": true },
        "size": { "type": "number" },
        "stage": { "type": "choice", "of": ["draft", "sent", "done"], "unsaid": "draft" },
    })
}

async fn declared(rig: &Rig, name: &str, fields: Value) -> kernel::Result<studio::Collection> {
    let spec = Spec::read(&fields, &json!({ "rows": { "kind": "table" } }), Some("local")).unwrap();
    rig.studio.collections.ensure("shop", name, spec, None, &ada()).await
}

/// Writes `rows` records into `table` with one statement, as a factory
/// that had been adding them for a long while would hold them, and not
/// through the daemon, which has nothing to do with how they came.
async fn planted(rig: &Rig, table: &str, rows: i64) {
    let table = table.to_string();
    let shop = rig.journal.records_of(Some("shop"));
    let db = shop.db();
    let planting = db.call(move |connection| {
        connection.execute(
            "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < ?2)
             INSERT INTO records (id, project, collection, number, data, position, created_by, created, updated)
             SELECT 'shop-' || ?1 || '-' || i, 'shop', ?1, i,
                    json_object('title', 'Row ' || i, 'size', i, 'stage', CASE i % 3 WHEN 0 THEN 'sent' ELSE 'done' END),
                    i, 'user:ada', 1, 1
             FROM n",
            rusqlite::params![table, rows],
        )
    });
    planting.await.unwrap();
}

/// Counts the steps every database of the factory takes from here on.
async fn counting(rig: &Rig) -> Arc<AtomicU64> {
    let done = Arc::new(AtomicU64::new(0));
    for db in rig.journal.dbs() {
        let counting = done.clone();
        let told = db.call(move |connection| {
            connection.progress_handler(
                1,
                Some(move || {
                    counting.fetch_add(1, Ordering::SeqCst);
                    false
                }),
            )
        });
        told.await.unwrap();
    }
    done
}

#[tokio::test]
async fn a_table_of_a_hundred_thousand_rows_declared_again_asks_the_database_as_much_as_one_of_three() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;
    for (table, rows) in [("few", 3), ("many", 100_000)] {
        declared(&rig, table, fields()).await.unwrap();
        planted(&rig, table, rows).await;
        // The first count after a start reads the index; after it, it is kept.
        let made = declared(&rig, table, fields()).await.unwrap();
        assert_eq!(collections.count(&made).await.unwrap(), rows as usize, "{table}");
    }
    // What keeps the histories of records hears of each declaration and
    // reads the tables again in its own time, which falls inside one
    // count or the next: it is stopped, and given a declaration to be
    // done with, before anything is counted.
    rig.studio.halt();
    declared(&rig, "few", fields()).await.unwrap();
    let done = counting(&rig).await;
    let mut steps = Vec::new();
    for (table, rows) in [("few", 3), ("many", 100_000)] {
        let before = done.load(Ordering::SeqCst);
        let made = declared(&rig, table, fields()).await.unwrap();
        assert_eq!(collections.count(&made).await.unwrap(), rows as usize, "{table}");
        steps.push(done.load(Ordering::SeqCst) - before);
    }
    // Read whole, as every declaration did, a hundred thousand rows are
    // millions of steps more.
    assert_eq!(
        steps[0], steps[1],
        "declared again, a table of three rows took {} steps of the database and one of a hundred thousand {}",
        steps[0], steps[1]
    );

    // What changes a field is asked of the rows, and answered rightly.
    let mut fewer = fields();
    fewer["stage"]["of"] = json!(["draft", "done"]);
    let refused = declared(&rig, "many", fewer).await.unwrap_err();
    assert_eq!(
        refused.message,
        "`sent` is still the `stage` of 33333 record(s) of `many`; change them before taking it away"
    );
    let mut needed = fields();
    needed["size"]["required"] = json!(true);
    declared(&rig, "many", needed).await.unwrap();
}

#[tokio::test]
async fn what_is_asked_of_a_field_declared_otherwise_is_what_reading_every_row_found() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let (collections, records) = (&rig.studio.collections, &rig.studio.records);
    let worded = json!({ "title": { "type": "text", "required": true }, "note": { "type": "text" } });
    let table = declared(&rig, "tasks", worded.clone()).await.unwrap();
    for note in [json!("12"), json!("\t\u{3000} "), json!(""), json!("big")] {
        let data = data(json!({ "title": "A task", "note": note }));
        records.add("shop", "tasks", data, &ada(), None).await.unwrap();
    }
    records.add("shop", "tasks", data(json!({ "title": "Without" })), &ada(), None).await.unwrap();
    assert_eq!(collections.count(&table).await.unwrap(), 5);

    // Words with nothing but what `trim` takes off are nothing, as nothing is.
    let mut needed = worded.clone();
    needed["note"]["required"] = json!(true);
    let refused = declared(&rig, "tasks", needed).await.unwrap_err();
    assert!(refused.message.contains("and 3 record(s) of `tasks` have none"), "{refused}");
    // Every row with something there that a number is not.
    let numbered = json!({ "title": { "type": "text", "required": true }, "note": { "type": "number" } });
    let refused = declared(&rig, "tasks", numbered).await.unwrap_err();
    assert!(refused.message.contains("which is not what 2 record(s) of `tasks` hold of it"), "{refused}");
    // A field said for the first time is had by no record, whatever a row
    // may still say under its name of a field that went.
    let mut owned = worded;
    owned["owner"] = json!({ "type": "text", "required": true });
    let refused = declared(&rig, "tasks", owned).await.unwrap_err();
    assert!(refused.message.contains("and 5 record(s) of `tasks` have none"), "{refused}");
}

#[tokio::test]
async fn how_many_records_a_table_has_follows_each_row_added_written_over_deleted_and_given_back_and_a_start()
{
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let (collections, records) = (&rig.studio.collections, &rig.studio.records);
    let table = declared(&rig, "tasks", fields()).await.unwrap();
    assert_eq!(collections.count(&table).await.unwrap(), 0);
    let mut added = Vec::new();
    for title in ["One", "Two", "Three"] {
        let data = data(json!({ "title": title }));
        added.push(records.add("shop", "tasks", data, &ada(), None).await.unwrap());
    }
    assert_eq!(collections.count(&table).await.unwrap(), 3);
    records.update(&added[0].id, data(json!({ "size": 4 })), &ada(), None).await.unwrap();
    assert_eq!(collections.count(&table).await.unwrap(), 3, "written over, it is the one row");

    records.remove(&added[1].id, &ada(), None).await.unwrap();
    assert_eq!(collections.count(&table).await.unwrap(), 2, "in the Trash");
    let shop = rig.journal.records_of(Some("shop"));
    let listed = studio::trash::rows(&shop, None).await.unwrap();
    let kept = studio::trash::rows(&shop, Some(listed[0].id.clone())).await.unwrap();
    collections.restore_record(&shop, &kept[0], &ada()).await.unwrap();
    assert_eq!(collections.count(&table).await.unwrap(), 3, "given back");
    studio::trash::keep_for(&rig.journal, Some(0), &ada()).await.unwrap();
    records.remove(&added[2].id, &ada(), None).await.unwrap();
    assert_eq!(collections.count(&table).await.unwrap(), 2, "deleted at once");

    // A daemon started again counts it again, from the rows.
    let again = studio::Studio::open(
        rig.journal.clone(),
        journal::Db::memory().unwrap(),
        rig.dir.join("projects"),
        rig.dir.join("attachments"),
    )
    .await
    .unwrap();
    assert_eq!(again.collections.count(&table).await.unwrap(), 2);
}
