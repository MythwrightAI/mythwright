//! A table read a window at a time by the daemon (`studio::window`): what
//! the statements read is what the rules read in memory say
//! (`kernel::window`), every kind, both ways, held to what a view is held
//! to, by an index and sorted in the statement; each statement reads by
//! the index it names, and never sorts what it walks; a column is indexed
//! when a large table is first sorted by it, and said; and what is counted
//! of a view is what counting every row in memory comes to.

use std::collections::BTreeMap;

use kernel::window::found::Find;
use kernel::window::read::{From, window as in_hand};
use kernel::window::statements::{Shape, figure, holding, next_value};
use kernel::window::{Column, Order, Sort, held, shows_sql};
use serde_json::{Map, Value, json};
use studio::collections::Spec;
use studio::window::{Asked, Reading};

use crate::{Rig, ada};

/// The fields of the tables read here: words, a number, a choice, a tick,
/// a list and somebody; `indexed` says each is kept in order by an index.
fn fields(indexed: bool) -> Value {
    let mut fields = json!({
        "title": { "type": "text" },
        "points": { "type": "number" },
        "stage": { "type": "choice", "of": ["draft", "ready", "done"] },
        "done": { "type": "flag" },
        "labels": { "type": "list" },
        "owner": { "type": "actor" },
    });
    if indexed {
        for name in ["title", "points", "stage", "done", "owner"] {
            fields[name]["indexed"] = json!(true);
        }
    }
    fields
}

fn views() -> Value {
    json!({
        "rows": { "kind": "table" },
        "ready": { "kind": "table", "only": { "stage": ["ready", "draft"] } },
        "labelled": { "kind": "table", "only": { "labels": ["a"] }, "group": "stage" },
        "gathered": { "kind": "table", "group": "stage" },
    })
}

async fn declared(rig: &Rig, name: &str, fields: Value) -> studio::Collection {
    let spec = Spec::read(&fields, &views(), Some("local")).unwrap();
    rig.studio.collections.ensure("shop", name, spec, None, &ada()).await.unwrap()
}

/// Writes `rows` rows into `table` with one statement, each holding
/// something else by its number: words in both cases, accents, nothing
/// and empty words; numbers below nought, halves and words; every option
/// of a choice, one it no longer offers and none; ticked, not and
/// nothing; lists of two, one and none; somebody or nobody.
async fn planted(rig: &Rig, table: &str, rows: i64) {
    let table = table.to_string();
    let shop = rig.journal.records_of(Some("shop"));
    let db = shop.db();
    let planting = db.call(move |connection| {
        connection.execute(
            "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < ?2)
             INSERT INTO records (id, project, collection, number, data, position, created_by, created, updated)
             SELECT 'shop-' || ?1 || '-' || i, 'shop', ?1, i, json_object(
                 'title', CASE i % 7 WHEN 0 THEN '' WHEN 1 THEN 'Apple ' || (i % 5) WHEN 2 THEN 'apple ' || (i % 5)
                          WHEN 3 THEN 'Émile' WHEN 4 THEN 'zulu' WHEN 5 THEN NULL ELSE 'b' || (i % 11) END,
                 'points', CASE i % 6 WHEN 0 THEN NULL WHEN 1 THEN '' WHEN 2 THEN (i % 13) * 0.5 ELSE (i % 13) - 4 END,
                 'stage', CASE i % 5 WHEN 0 THEN 'draft' WHEN 1 THEN 'ready' WHEN 2 THEN 'done' WHEN 3 THEN 'gone' ELSE '' END,
                 'done', CASE i % 3 WHEN 0 THEN json('true') WHEN 1 THEN json('false') ELSE NULL END,
                 'labels', CASE i % 4 WHEN 0 THEN json_array('a', 'b') WHEN 1 THEN json_array('b') WHEN 2 THEN json_array() ELSE NULL END,
                 'owner', CASE WHEN i % 3 = 0 THEN 'user:' || (i % 4) ELSE NULL END),
               i, 'user:ada', 1, 1
             FROM n",
            rusqlite::params![table, rows],
        )
    });
    planting.await.unwrap();
}

/// Every id of a view, read on a window of `limit` at a time; and read
/// back from its end the same way.
async fn read_through(rig: &Rig, table: &studio::Collection, mut asked: Asked) -> (Vec<String>, Vec<String>) {
    let collections = &rig.studio.collections;
    let mut on = Vec::new();
    let mut last_before = None;
    loop {
        let answer = collections.window(table, &asked, &ada()).await.unwrap();
        assert!(!answer.short, "a small table is never short");
        on.extend(answer.rows.iter().map(|row| row["id"].as_str().unwrap().to_string()));
        if !answer.rows.is_empty() {
            last_before = Some(answer.rows.last().unwrap()["id"].as_str().unwrap().to_string());
        }
        match answer.after {
            Some(after) if !answer.rows.is_empty() => asked.reading = Reading::After(after),
            _ => break,
        }
    }
    // Back from the end: from the last row, and every row before it.
    let mut back = Vec::new();
    if let Some(last) = last_before {
        back.push(last);
        let found = collections.window_ids(table, &asked, &back).await.unwrap();
        asked.reading = Reading::Before(found[0]["place"].as_str().unwrap().to_string());
        loop {
            let answer = collections.window(table, &asked, &ada()).await.unwrap();
            let mut ids: Vec<String> =
                answer.rows.iter().map(|row| row["id"].as_str().unwrap().to_string()).collect();
            ids.extend(back);
            back = ids;
            match answer.before {
                Some(before) if !answer.rows.is_empty() => asked.reading = Reading::Before(before),
                _ => break,
            }
        }
    }
    (on, back)
}

/// Every id of a view in order, as the rules in memory read it.
fn as_in_memory(
    table: &studio::Collection,
    records: &[studio::collections::Record],
    asked: &Asked,
) -> Vec<String> {
    let columns: Vec<Column> = table
        .fields
        .iter()
        .map(|field| Column { name: field.name.clone(), kind: field.kind.clone(), of: field.of.clone() })
        .collect();
    let view = asked.view.as_ref().and_then(|name| table.views.iter().find(|view| &view.name == name));
    let only = view.and_then(|view| view.only.clone()).unwrap_or_default();
    let band = asked.band.as_deref().map(|band| (view.unwrap().group.as_deref().unwrap(), band));
    let held = held(&columns, &only, band);
    let order = Order::of(&columns, asked.sort.as_ref()).unwrap();
    let find = asked.find.as_deref().and_then(|typed| Find::of(&columns, &[], typed));
    let found: Vec<&studio::collections::Record> =
        records.iter().filter(|record| find.as_ref().is_none_or(|find| find.finds(&record.data))).collect();
    let rows: Vec<(i64, &Map<String, Value>)> =
        found.iter().map(|record| (record.number, &record.data)).collect();
    let read = in_hand(&order, &held, &rows, &From::Start, usize::MAX);
    read.rows.iter().map(|at| found[*at].id.clone()).collect()
}

#[tokio::test]
async fn every_window_read_by_the_database_is_what_reading_every_row_in_memory_says_every_kind_both_ways() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    // One table sorted in the statement, and one walked by its indexes.
    for (name, indexed) in [("plain", false), ("kept", true)] {
        declared(&rig, name, fields(indexed)).await;
        planted(&rig, name, 211).await;
    }
    let sorts: Vec<Option<Sort>> = std::iter::once(None)
        .chain(
            ["title", "points", "stage", "done", "labels", "owner"]
                .into_iter()
                .flat_map(|field| [false, true].map(|down| Some(Sort { field: field.to_string(), down }))),
        )
        .collect();
    // Each searched for nothing, and some of them for words in several columns.
    let readings: [(Option<&str>, Option<&str>, Option<&str>); 9] = [
        (None, None, None),
        (Some("ready"), None, None),
        (Some("labelled"), None, None),
        (Some("gathered"), Some("ready"), None),
        (Some("gathered"), Some("gone"), None),
        (Some("gathered"), Some(""), None),
        (None, None, Some("APPLE 3")),
        (Some("ready"), None, Some("b")),
        (Some("gathered"), Some("ready"), Some("user:")),
    ];
    for name in ["plain", "kept"] {
        let table = rig.studio.collections.get("shop", name).await.unwrap();
        let records = rig.studio.collections.records(&table).await.unwrap();
        for sort in &sorts {
            // The kept table's list is no index's: it sorts a small table in the statement.
            for (view, band, find) in readings {
                let asked = Asked {
                    view: view.map(str::to_string),
                    sort: sort.clone(),
                    band: band.map(str::to_string),
                    find: find.map(str::to_string),
                    reading: Reading::Start,
                    limit: 13,
                    fields: None,
                };
                let meant = as_in_memory(&table, &records, &asked);
                let (on, back) = read_through(&rig, &table, asked.clone()).await;
                assert_eq!(on, meant, "{name} read on, sorted {sort:?}, {view:?} {band:?}");
                assert_eq!(back, meant, "{name} read back, sorted {sort:?}, {view:?} {band:?}");
                // A window from a rank begins with the row at that rank.
                for rank in [0, 5, meant.len().saturating_sub(1)] {
                    let at = Asked { reading: Reading::At(rank as u64), limit: 3, ..asked.clone() };
                    let answer = rig.studio.collections.window(&table, &at, &ada()).await.unwrap();
                    let first = answer.rows.first().map(|row| row["id"].as_str().unwrap().to_string());
                    assert_eq!(
                        first,
                        meant.get(rank).cloned(),
                        "{name} at {rank}, {sort:?} {view:?} {band:?}"
                    );
                    // About a row: where it stands, and the rows before it and from it on.
                    let Some(id) = meant.get(rank) else { continue };
                    let about = Asked { reading: Reading::Around(id.clone()), limit: 5, ..asked.clone() };
                    let answer = rig.studio.collections.window(&table, &about, &ada()).await.unwrap();
                    assert_eq!(
                        answer.around.as_ref().unwrap()["rank"],
                        rank,
                        "{name} about {id}, {sort:?} {view:?} {band:?} {find:?}"
                    );
                    let first = answer.rank.unwrap() as usize;
                    let ids: Vec<String> =
                        answer.rows.iter().map(|row| row["id"].as_str().unwrap().to_string()).collect();
                    assert_eq!(
                        ids,
                        meant[first..(first + 5).min(meant.len())],
                        "{name} about {id}, {sort:?} {view:?} {band:?} {find:?}"
                    );
                }
                // A row the view does not show is said to be kept out, and by what.
                if let Some(out) = records.iter().find(|record| !meant.contains(&record.id)) {
                    let about = Asked { reading: Reading::Around(out.id.clone()), ..asked.clone() };
                    let answer = rig.studio.collections.window(&table, &about, &ada()).await.unwrap();
                    let around = answer.around.unwrap();
                    assert_eq!(around["in"], false, "{name} {view:?} {band:?} {find:?}");
                    assert!(!around["kept_out"].as_array().unwrap().is_empty());
                    assert!(answer.rows.is_empty());
                }
            }
        }
    }
}

#[tokio::test]
async fn what_is_counted_of_a_view_off_its_indexes_and_by_its_rows_is_what_counting_every_row_comes_to() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    declared(&rig, "kept", fields(true)).await;
    planted(&rig, "kept", 400).await;
    let table = rig.studio.collections.get("shop", "kept").await.unwrap();
    let records = rig.studio.collections.records(&table).await.unwrap();
    for (view, figures) in [
        (
            "gathered",
            vec![
                ("points", "filled"),
                ("points", "least"),
                ("points", "greatest"),
                ("done", "ticked"),
                ("title", "empty"),
            ],
        ),
        (
            "gathered",
            vec![("points", "sum"), ("points", "mean"), ("title", "different"), ("labels", "filled")],
        ),
        ("labelled", vec![("points", "sum"), ("done", "ticked")]),
    ] {
        let asked = Asked { view: Some(view.to_string()), limit: 100, ..Asked::default() };
        let figures: BTreeMap<String, String> =
            figures.iter().map(|(field, which)| (field.to_string(), which.to_string())).collect();
        let counted =
            rig.studio.collections.counted_rows(&table, &asked, Some(figures.clone())).await.unwrap();
        assert_eq!(counted["whole"], true, "{view}");
        assert_eq!(counted["of"], 400);
        // Every row in memory, by the rules.
        let columns: Vec<Column> = table
            .fields
            .iter()
            .map(|field| Column { name: field.name.clone(), kind: field.kind.clone(), of: field.of.clone() })
            .collect();
        let only = table.views.iter().find(|one| one.name == view).unwrap().only.clone().unwrap_or_default();
        let held = held(&columns, &only, None);
        let shown: Vec<Map<String, Value>> = records
            .iter()
            .filter(|record| kernel::window::shows(&held, &record.data))
            .map(|record| record.data.clone())
            .collect();
        assert_eq!(counted["count"], shown.len(), "{view}");
        for (field, which) in &figures {
            let column = columns.iter().find(|column| &column.name == field).unwrap();
            let meant = kernel::window::figures::figure(&shown, column, which);
            assert_eq!(counted["figures"][field].as_f64(), meant, "{view} {field} {which}");
        }
        let mut bands = BTreeMap::new();
        let stage = columns.iter().find(|column| column.name == "stage").unwrap();
        for row in &shown {
            let things = kernel::window::holds(stage, row.get("stage"));
            for thing in if things.is_empty() { vec![String::new()] } else { things } {
                *bands.entry(thing).or_insert(0u64) += 1;
            }
        }
        let (meant, _) = kernel::window::figures::ordered_bands(stage, &bands);
        let meant: Vec<Value> =
            meant.iter().map(|(key, count)| json!({ "key": key, "count": count })).collect();
        if view == "gathered" {
            assert_eq!(counted["bands"], json!(meant), "{view}");
        }
    }
}

/// What the database says it does with `statement`.
async fn planned(rig: &Rig, statement: String) -> String {
    let shop = rig.journal.records_of(Some("shop"));
    let db = shop.db();
    let asked = db.call(move |connection| {
        let mut prepared = connection.prepare(&format!("EXPLAIN QUERY PLAN {statement}"))?;
        let lines: Vec<String> =
            prepared.query_map([], |row| row.get::<_, String>(3))?.collect::<rusqlite::Result<_>>()?;
        Ok(lines.join("\n"))
    });
    asked.await.unwrap()
}

#[tokio::test]
async fn each_statement_reads_by_the_index_it_names_and_a_walk_sorts_nothing_it_reads() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let table = declared(&rig, "kept", fields(true)).await;
    planted(&rig, "kept", 50).await;
    let columns: Vec<Column> = table
        .fields
        .iter()
        .map(|field| Column { name: field.name.clone(), kind: field.kind.clone(), of: field.of.clone() })
        .collect();
    let record = "id, project, collection, number, data, position, created_by, created, updated".to_string();
    let place = kernel::window::Place { stretch: 0, key: kernel::window::Key::Text("b".into()), number: 7 };
    for (sort, index) in [
        (None, "records_numbered"),
        (Some("title"), "records_by shop/kept/title"),
        (Some("points"), "records_by shop/kept/points"),
        (Some("stage"), "records_by shop/kept/stage"),
        (Some("done"), "records_by shop/kept/done"),
    ] {
        for down in [false, true] {
            let sort = sort.map(|field| Sort { field: field.to_string(), down });
            let order = Order::of(&columns, sort.as_ref()).unwrap();
            for only in [json!({}), json!({ "labels": ["a"] })] {
                let only: BTreeMap<String, Vec<String>> = serde_json::from_value(only).unwrap();
                let shape = Shape {
                    record: record.clone(),
                    project: "shop".into(),
                    table: "kept".into(),
                    order: order.clone(),
                    shows: shows_sql(&held(&columns, &only, None)),
                };
                for stretch in 0..order.stretches.len() {
                    for from in [None, Some(false), Some(true)] {
                        let at = kernel::window::Place { stretch, ..place.clone() };
                        for ascending in [true, false] {
                            let statement =
                                shape.walk(stretch, from.map(|inclusive| (&at, inclusive)), ascending);
                            let plan = planned(&rig, statement).await;
                            let named = plan.contains(&format!("INDEX {index} "))
                                || plan.contains(&format!("INDEX \"{index}\""));
                            assert!(named, "{sort:?} {stretch}: {plan}");
                            assert!(
                                !plan.contains("TEMP B-TREE"),
                                "a walk sorts nothing it reads: {sort:?} {stretch}: {plan}"
                            );
                        }
                        // How many come before a place is read off the index alone where nothing is checked of a row.
                        let plan = planned(&rig, shape.before(stretch, from.map(|_| &at))).await;
                        assert!(plan.contains(index), "{sort:?} {stretch}: {plan}");
                        if shape.shows == "1" {
                            assert!(plan.contains("COVERING INDEX"), "{sort:?} {stretch}: {plan}");
                        }
                    }
                }
            }
        }
    }
    // What is counted off an index reads no row.
    let ix = |name: &str| columns.iter().find(|column| column.name == name).unwrap().clone();
    let off = [
        figure("shop", "kept", &ix("points"), "filled").unwrap(),
        figure("shop", "kept", &ix("points"), "least").unwrap(),
        figure("shop", "kept", &ix("points"), "greatest").unwrap(),
        figure("shop", "kept", &ix("done"), "ticked").unwrap(),
        next_value("shop", "kept", &ix("stage"), &kernel::window::Key::Null),
        next_value("shop", "kept", &ix("stage"), &kernel::window::Key::Text("draft".into())),
        holding("shop", "kept", &ix("stage"), &kernel::window::Key::Text("ready".into())),
        holding("shop", "kept", &ix("stage"), &kernel::window::Key::Null),
    ];
    for statement in off {
        let plan = planned(&rig, statement.clone()).await;
        assert!(
            plan.contains("COVERING INDEX \"records_by shop/kept/")
                || plan.contains("COVERING INDEX records_by"),
            "{statement}: {plan}"
        );
    }
    assert!(figure("shop", "kept", &ix("points"), "sum").is_none(), "what adds up reads the rows");
    // A gathering is counted off its index too.
    let gather =
        kernel::window::Gather { column: ix("stage"), keys: vec![kernel::window::Key::Text("ready".into())] };
    let shape = Shape {
        record,
        project: "shop".into(),
        table: "kept".into(),
        order: Order::of(&columns, Some(&Sort { field: "points".into(), down: false })).unwrap(),
        shows: "1".into(),
    };
    let plan = planned(&rig, shape.gathered(&gather)).await;
    assert!(plan.contains("COVERING INDEX"), "{plan}");
    let plan = planned(&rig, shape.sorted(Some(&gather), None, true, 10, 0)).await;
    assert!(plan.contains("records_by shop/kept/stage"), "{plan}");
}

/// The indexes the file that keeps the shop's tables holds, by name.
async fn indexes(rig: &Rig) -> Vec<String> {
    let shop = rig.journal.records_of(Some("shop"));
    let db = shop.db();
    let asked = db.call(|connection| {
        let mut prepared = connection.prepare(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND name LIKE 'records_by %' ORDER BY name",
        )?;
        let names: Vec<String> =
            prepared.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
        Ok(names)
    });
    asked.await.unwrap()
}

#[tokio::test]
async fn a_large_table_has_a_column_indexed_as_it_is_first_sorted_by_it_said_and_let_go_with_its_column() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;
    // Nine numbers, each a column a reader may sort by.
    let nine: Map<String, Value> = (1..=9).map(|n| (format!("n{n}"), json!({ "type": "number" }))).collect();
    let spec =
        Spec::read(&Value::Object(nine), &json!({ "rows": { "kind": "table" } }), Some("local")).unwrap();
    collections.ensure("shop", "big", spec, None, &ada()).await.unwrap();
    let shop = rig.journal.records_of(Some("shop"));
    let db = shop.db();
    let planting = db.call(|connection| {
        connection.execute(
            "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 10000)
             INSERT INTO records (id, project, collection, number, data, position, created_by, created, updated)
             SELECT 'shop-big-' || i, 'shop', 'big', i,
                    json_object('n1', i % 97, 'n2', i % 89, 'n3', i % 83, 'n4', i % 79, 'n5', i % 73,
                                'n6', i % 71, 'n7', i % 67, 'n8', i % 61, 'n9', 10000 - i),
                    i, 'user:ada', 1, 1
             FROM n",
            [],
        )
    });
    planting.await.unwrap();
    let mut table = collections.get("shop", "big").await.unwrap();
    // A small table is sorted in the statement, and has nothing made.
    let sorted = |field: &str| Asked {
        sort: Some(Sort { field: field.into(), down: false }),
        limit: 3,
        ..Asked::default()
    };
    // Ten thousand rows is a large table: the first sort by a column makes its index, once.
    for n in 1..=8 {
        let answer = collections.window(&table, &sorted(&format!("n{n}")), &ada()).await.unwrap();
        assert_eq!(answer.rows.len(), 3);
    }
    let first = collections.window(&table, &sorted("n9"), &ada()).await.unwrap();
    assert_eq!(first.rows[0]["id"], "shop-big-10000", "sorted by its index, the least first");
    let made = indexes(&rig).await;
    assert_eq!(made.len(), 8, "at most eight of a table's columns are kept in order: {made:?}");
    assert!(
        !made.contains(&"records_by shop/big/n1".to_string()),
        "the one asked for least lately was let go: {made:?}"
    );
    let said = rig.events("collection.indexed").await;
    assert_eq!(said.len(), 9, "each one made was said once");
    assert_eq!(said[8].data, json!({ "project": "shop", "name": "big", "field": "n9" }));
    collections.window(&table, &sorted("n9"), &ada()).await.unwrap();
    assert_eq!(rig.events("collection.indexed").await.len(), 9, "and nothing made twice");

    // A column somebody added, taken away, takes its index with it.
    let new =
        studio::added::New { label: "Size".into(), kind: "number".into(), ..studio::added::New::default() };
    let (_, size) = collections.add_column("shop", "big", new, &ada()).await.unwrap();
    table = collections.get("shop", "big").await.unwrap();
    collections.window(&table, &sorted(&size), &ada()).await.unwrap();
    assert!(indexes(&rig).await.contains(&format!("records_by shop/big/{size}")));
    collections.remove_column("shop", "big", &size, &ada()).await.unwrap();
    assert!(!indexes(&rig).await.contains(&format!("records_by shop/big/{size}")));

    // A declaration asks for no more than a table has.
    let many: Map<String, Value> =
        (1..=9).map(|n| (format!("n{n}"), json!({ "type": "number", "indexed": true }))).collect();
    let refused = Spec::read(&Value::Object(many), &json!({}), Some("local")).unwrap_err();
    assert_eq!(
        refused.message,
        "9 fields say `indexed`, and a table has 8 kept in order at most: say it of those it is sorted or held by most"
    );
    let listed =
        Spec::read(&json!({ "tags": { "type": "list", "indexed": true } }), &json!({}), Some("local"))
            .unwrap_err();
    assert!(listed.message.contains("is kept in order by no index"), "{listed}");
}
