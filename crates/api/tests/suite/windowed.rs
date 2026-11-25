//! A table read a window at a time (`GET …/rows`): reading the first page
//! of a view, sorted, held to a choice, from the middle and back, costs
//! the database the same steps beside a hundred rows as beside a hundred
//! thousand (a million, told `ROWS=1000000`), by each database's own
//! count; a view held to what a list holds looks at no more than so many
//! rows for one answer; what is counted of a view is answered; and a
//! table kept from somebody is not there for them, by any of it.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use reqwest::StatusCode;
use serde_json::{Value, json};

use crate::Rig;

/// Two tables of one project, the same but for how many rows they come
/// to have: what each column says is kept in order from the first row.
const TABLES: &str = r#"
blueprint: tables
permissions: [projects]
steps:
  few:
    do: collection.ensure
    with:
      name: few
      fields:
        title: { type: text, indexed: true }
        points: { type: number, indexed: true }
        stage: { type: choice, of: [ready, doing, done], indexed: true }
        labels: { type: list }
      views:
        rows: { kind: table }
        ready: { kind: table, only: { stage: [ready] } }
        labelled: { kind: table, only: { labels: [a] } }
  many:
    do: collection.ensure
    with:
      name: many
      fields:
        title: { type: text, indexed: true }
        points: { type: number, indexed: true }
        stage: { type: choice, of: [ready, doing, done], indexed: true }
        labels: { type: list }
      views:
        rows: { kind: table }
        ready: { kind: table, only: { stage: [ready] } }
        labelled: { kind: table, only: { labels: [a] } }
"#;

/// How many rows the larger table comes to: a hundred thousand in the
/// gate, a million by hand.
fn rows() -> i64 {
    std::env::var("ROWS").ok().and_then(|rows| rows.parse().ok()).unwrap_or(100_000)
}

/// Writes rows `from..=to` into `table` by one statement, as a factory
/// that had added them for a long while would hold them: the first
/// hundred alike in both tables, each a choice of two and a few of them
/// with a label; every one after them coming after those whichever way
/// they are read first, and choosing what nothing below is held to.
async fn planted(rig: &Rig, table: &str, from: i64, to: i64) {
    let journal = rig.node().app.journal.records_of(Some("shop"));
    let table = table.to_string();
    let planting = journal.db().call(move |connection| {
        connection.execute(
            "WITH RECURSIVE n(i) AS (SELECT ?2 UNION ALL SELECT i + 1 FROM n WHERE i < ?3)
             INSERT INTO records (id, project, collection, number, data, position, created_by, created, updated)
             SELECT 'shop-' || ?1 || '-' || i, 'shop', ?1, i,
                    CASE WHEN i <= 100 THEN json_object(
                        'title', 'Row ' || i, 'points', i,
                        'stage', CASE i % 2 WHEN 0 THEN 'ready' ELSE 'doing' END,
                        'labels', CASE WHEN i <= 10 THEN json_array('a') ELSE json_array() END)
                    ELSE json_object('title', 'Zz other ' || i, 'points', 1000 + i, 'stage', 'done') END,
                    i, 'user:ada', 1, 1
             FROM n",
            rusqlite::params![table, from, to],
        )
    });
    planting.await.unwrap();
}

/// Has the database that keeps the shop's tables count its own steps
/// (`progress_handler`) from here on.
async fn counting(rig: &Rig) -> Arc<AtomicU64> {
    let steps = Arc::new(AtomicU64::new(0));
    let journal = rig.node().app.journal.records_of(Some("shop"));
    let counted = steps.clone();
    let told = journal.db().call(move |connection| {
        connection.progress_handler(
            1,
            Some(move || {
                counted.fetch_add(1, Ordering::Relaxed);
                false
            }),
        )
    });
    told.await.unwrap();
    steps
}

/// A window of `table`, read as `asked` says.
async fn window(rig: &Rig, table: &str, asked: &str) -> Value {
    rig.read(&format!("/api/collections/shop/{table}/rows?{asked}")).await
}

/// The ids of a window's rows.
fn ids(window: &Value) -> Vec<String> {
    window["rows"].as_array().unwrap().iter().map(|row| row["id"].as_str().unwrap().to_string()).collect()
}

/// The same read of both tables: what the larger costs, and what each read.
async fn both(rig: &Rig, steps: &AtomicU64, asked: &str) -> ((u64, u64), (Value, Value)) {
    let mut cost = Vec::new();
    let mut read = Vec::new();
    for table in ["few", "many"] {
        // Read once to have every statement's tables known, then counted.
        window(rig, table, asked).await;
        let before = steps.load(Ordering::SeqCst);
        read.push(window(rig, table, asked).await);
        cost.push(steps.load(Ordering::SeqCst) - before);
    }
    let many = read.pop().unwrap();
    ((cost[0], cost[1]), (read.pop().unwrap(), many))
}

#[tokio::test]
async fn the_first_page_of_a_view_and_a_page_from_the_middle_cost_the_same_beside_a_hundred_rows_as_beside_many()
 {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // Planted before the tables are declared, as a factory from before
    // would hold them: how many rows a table has is counted as the daemon
    // first comes to it, and moved by its own writes from then on.
    planted(&rig, "few", 1, 100).await;
    planted(&rig, "many", 1, rows()).await;
    let applied = rig.send("/api/apply", json!({ "source": TABLES, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    // Each table's rows are counted once after a start, and are kept so from then on.
    for table in ["few", "many"] {
        window(&rig, table, "limit=1").await;
    }
    let counted = rig.read("/api/collections/shop/many/rows/counted").await;
    assert_eq!(counted["of"], rows(), "{counted}");
    let steps = counting(&rig).await;
    let numbered = window(&rig, "few", "limit=50").await;
    let middle = numbered["after"].as_str().unwrap().to_string();
    let reads: Vec<(&str, String)> = vec![
        ("by number", "limit=50".into()),
        ("by a number, up", "sort=points&limit=50".into()),
        ("by a number, down", "sort=points&down=1&limit=50".into()),
        ("by words", "sort=title&limit=50".into()),
        ("held to a choice, by a number", "view=ready&sort=points&limit=50".into()),
        ("by a choice", "sort=stage&limit=50".into()),
        ("from the middle", format!("limit=50&after={middle}")),
        ("about a row", "around=30&limit=50".into()),
        ("about a row, by a number", "around=30&sort=points&limit=50".into()),
    ];
    for (what, asked) in &reads {
        let ((few, many), (read_few, read_many)) = both(&rig, &steps, asked).await;
        assert!(!read_many["rows"].as_array().unwrap().is_empty(), "{what}");
        if *what != "by a number, down" {
            // The same rows, by number, but for the table they are of.
            let numbers = |read: &Value| -> Vec<i64> {
                read["rows"].as_array().unwrap().iter().map(|row| row["number"].as_i64().unwrap()).collect()
            };
            assert_eq!(numbers(&read_few), numbers(&read_many), "{what}");
        }
        assert_eq!(
            few,
            many,
            "{what}: a window beside 100 rows took {few} steps of the database, and beside {} {many}",
            rows()
        );
    }
    // Back from the middle, by a number: what came before it.
    let up = window(&rig, "many", "sort=points&limit=50").await;
    let after = up["after"].as_str().unwrap();
    let on = window(&rig, "many", &format!("sort=points&limit=50&after={after}")).await;
    let back =
        window(&rig, "many", &format!("sort=points&limit=50&before={}", on["before"].as_str().unwrap()))
            .await;
    assert_eq!(ids(&back), ids(&up));
    assert_eq!(on["rows"][0]["data"]["points"], 51);
    assert_eq!(on["rows"][49]["data"]["points"], 100);

    // Held to what a list holds, every row is looked at for what it holds,
    // and one answer looks at no more than so many: it says so, and where it got to.
    let labelled = window(&rig, "many", "view=labelled&limit=50").await;
    assert_eq!(ids(&labelled).len(), 10);
    assert_eq!(labelled["short"], true);
    assert_eq!(labelled["looked"], kernel::window::LOOK);
    let on = window(
        &rig,
        "many",
        &format!("view=labelled&limit=50&after={}", labelled["after"].as_str().unwrap()),
    )
    .await;
    assert!(ids(&on).is_empty() && on["looked"].as_u64().unwrap() <= kernel::window::LOOK);
    let few = window(&rig, "few", "view=labelled&limit=50").await;
    assert_eq!((ids(&few).len(), &few["short"], &few["after"]), (10, &json!(false), &Value::Null));

    // Searched for, the same: the first nineteen rows that say both words, and no more looked at.
    let found = window(&rig, "many", "find=row%205&limit=50").await;
    assert_eq!(ids(&found).len(), 19, "{found}");
    assert_eq!((&found["short"], &found["looked"]), (&json!(true), &json!(kernel::window::LOOK)));
    let few = window(&rig, "few", "find=row%205&limit=50").await;
    let same: Vec<String> = ids(&found).iter().map(|id| id.replace("many", "few")).collect();
    assert_eq!((ids(&few), &few["short"]), (same, &json!(false)));
    let counted = rig.read("/api/collections/shop/many/rows/counted?find=row%205").await;
    assert_eq!(counted["count"], 19, "{counted}");
    assert!(counted["looked"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn what_a_view_shows_is_counted_rows_are_answered_by_id_and_a_place_of_another_reading_is_refused() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    planted(&rig, "many", 1, 300).await;
    let applied = rig.send("/api/apply", json!({ "source": TABLES, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let counted = rig
        .read("/api/collections/shop/many/rows/counted?view=ready&figures=points:sum,points:greatest")
        .await;
    assert_eq!(counted["count"], 50);
    assert_eq!(counted["of"], 300);
    assert_eq!(counted["whole"], true);
    assert_eq!(counted["figures"], json!({ "points": 100.0 }), "one figure a column: the last said");
    let counted = rig.read("/api/collections/shop/many/rows/counted?figures=points:greatest").await;
    assert_eq!((&counted["count"], &counted["figures"]["points"]), (&json!(300), &json!(1300.0)));

    // Rows by their ids: whether the view shows each, where it stands, and the row.
    let found = rig
        .read("/api/collections/shop/many/rows?view=ready&sort=points&ids=shop-many-4,shop-many-5,shop-few-4")
        .await;
    let rows = found["rows"].as_array().unwrap();
    assert_eq!(
        (rows[0]["in"].clone(), rows[1]["in"].clone(), rows[2]["in"].clone()),
        (json!(true), json!(false), json!(false))
    );
    assert_eq!(rows[0]["row"]["data"]["points"], 4);
    assert!(rows[2]["row"].is_null(), "a row of another table is no row of this one");
    let from = rows[0]["place"].as_str().unwrap();
    let after = window(&rig, "many", &format!("view=ready&sort=points&limit=2&after={from}")).await;
    assert_eq!(ids(&after), ["shop-many-6", "shop-many-8"]);

    // About a row: where it stands among those the view shows, and the rows about it; or what keeps it out.
    let about = window(&rig, "many", "view=ready&sort=points&around=shop-many-8&limit=4").await;
    assert_eq!(
        (&about["around"]["in"], &about["around"]["rank"], &about["rank"]),
        (&json!(true), &json!(3), &json!(1))
    );
    assert_eq!(ids(&about), ["shop-many-4", "shop-many-6", "shop-many-8", "shop-many-10"]);
    let out = window(&rig, "many", "view=ready&around=7").await;
    assert_eq!((&out["around"]["in"], &out["around"]["kept_out"]), (&json!(false), &json!(["stage"])));
    let (status, refused) = rig.get("/api/collections/shop/many/rows?around=shop-many-9999").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert_eq!(
        refused["error"]["message"],
        "`many` has no row `shop-many-9999`: go to a row by its id or its number"
    );

    // A place of one reading is no place in another.
    let (status, refused) =
        rig.get(&format!("/api/collections/shop/many/rows?view=ready&sort=points&down=1&after={from}")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert_eq!(
        refused["error"]["message"],
        "the view was changed, or is sorted another way, since this was read: read it from the start"
    );
    let (status, _) = rig.get("/api/collections/shop/many/rows?after=x&before=y").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_table_kept_from_somebody_is_not_there_for_them_by_any_window_or_count() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let notes = r#"
blueprint: notes
permissions: [projects]
steps:
  moods:
    do: collection.ensure
    with:
      name: moods
      reads: private:journal
      fields: { title: { type: text, indexed: true } }
"#;
    let applied = rig.send("/api/apply", json!({ "source": notes, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    rig.send("/api/collections/shop/moods/records", json!({ "fields": { "title": "Saturday" } })).await;
    assert_eq!(ids(&window(&rig, "moods", "sort=title").await), ["shop-moods-1"]);
    let made = rig
        .send("/api/credentials", json!({ "name": "helper", "scopes": ["projects:read", "records:read"] }))
        .await;
    let token = made["token"].as_str().unwrap();
    let missing =
        rig.call(reqwest::Method::GET, "/api/collections/shop/nothing/rows", Some(token), None).await;
    for path in ["rows", "rows?sort=title", "rows?ids=shop-moods-1", "rows/counted"] {
        let (status, body) = rig
            .call(reqwest::Method::GET, &format!("/api/collections/shop/moods/{path}"), Some(token), None)
            .await;
        assert_eq!(status, missing.0, "{path}: {body}");
        assert_eq!(
            body["error"]["message"].as_str().unwrap().replace("moods", "nothing"),
            missing.1["error"]["message"].as_str().unwrap(),
            "{path}"
        );
    }
}
