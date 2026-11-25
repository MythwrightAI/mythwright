//! What the studio hands whoever moves a project of a factory from
//! before to a journal of its own (`studio::moved`): a project's tables,
//! their records, each record's history and what a scan last saw of its
//! files, each row by what its table's line says of whose it is, and a
//! line of history by its record and the tables there are, taken out
//! before either.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use journal::moved::Row;
use kernel::ManualClock;
use rusqlite::Connection;

/// A folder of this test's own, taken away when the test is over.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A journal's file called `name` with the tables of what is a
/// project's in it, as they are made in such a file, closed.
async fn made(file: &Path, name: &str) {
    let journal = journal::moved::made(file, name, Arc::new(ManualClock::new())).await.unwrap();
    studio::apart::make(journal.db()).await.unwrap();
    journal.db().close();
    journal.db().closed().await;
}

/// Every row of each table that is a project's, in a file: the table,
/// and what the row is known by.
fn held(file: &Path) -> Vec<(&'static str, String)> {
    let connection = Connection::open(file).unwrap();
    let known = [
        ("collections", "project || '/' || name"),
        ("records", "id"),
        ("record_history", "record || '@' || seq"),
        ("project_files", "project || '/' || path"),
    ];
    let mut all = Vec::new();
    for (table, by) in known {
        let mut statement = connection.prepare(&format!("SELECT {by} FROM {table} ORDER BY 1")).unwrap();
        let rows = statement.query_map([], |row| row.get::<_, String>(0)).unwrap();
        all.extend(rows.map(|row| (table, row.unwrap())));
    }
    all
}

#[tokio::test]
async fn a_projects_tables_records_history_and_files_are_moved_by_their_lines_and_no_other_projects_are() {
    let dir = Scratch(
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("moved-{}", &kernel::id::secret()[..12])),
    );
    std::fs::create_dir_all(&dir.0).unwrap();
    let (factory, file) = (dir.0.join("journal.db"), dir.0.join("shop.db"));
    made(&factory, "factory").await;
    {
        let connection = Connection::open(&factory).unwrap();
        // Three projects, one called as a card of another would be.
        for (project, name) in [("shop", "board"), ("shop", "notes"), ("shop-2", "board"), ("yard", "board")]
        {
            let wrote = connection.execute(
                "INSERT INTO collections (project, name, fields, views, store, owner, created, updated, at)
                 VALUES (?1, ?2, '[]', '[]', 'local', 'in_declared', 1, 1, 'http://127.0.0.1:1')",
                [project, name],
            );
            wrote.unwrap();
        }
        let records = [
            ("shop-1", "shop", "board"),
            ("shop-notes-1", "shop", "notes"),
            ("shop-2-5", "shop-2", "board"),
            ("yard-1", "yard", "board"),
        ];
        for (id, project, collection) in records {
            let wrote = connection.execute(
                "INSERT INTO records (id, project, collection, number, data, position, created_by, created, updated)
                 VALUES (?1, ?2, ?3, 1, '{}', 1.0, 'ada', 1, 1)",
                [id, project, collection],
            );
            wrote.unwrap();
        }
        // Lines of records that are there, of two that are gone, and of
        // one no table was ever of.
        let lines = [
            ("shop-1", 10),
            ("shop-1", 11),
            ("shop-9", 12),
            ("shop-notes-1", 13),
            ("shop-2-5", 14),
            ("shop-2-7", 15),
            ("yard-1", 16),
            ("nothing-3", 17),
        ];
        for (record, seq) in lines {
            let wrote = connection.execute(
                "INSERT INTO record_history (record, seq, ts, topic, actor, who, said, data)
                 VALUES (?1, ?2, 1, 'board.item.moved', 'user:ada', 'Ada', 'Ada moved it', '{}')",
                rusqlite::params![record, seq],
            );
            wrote.unwrap();
        }
        for (project, path) in [("shop", "a.txt"), ("shop", "b.txt"), ("shop-2", "d.txt"), ("yard", "c.txt")]
        {
            let wrote = connection.execute(
                "INSERT INTO project_files (project, path, mtime, size) VALUES (?1, ?2, 1, 1)",
                [project, path],
            );
            wrote.unwrap();
        }
    }
    {
        // What of each was deleted: a card of each project, and a project,
        // which is the factory's and says none.
        let connection = Connection::open(&factory).unwrap();
        for (id, thing, project) in
            [("trash_a", "shop-3", "shop"), ("trash_b", "yard-3", "yard"), ("trash_c", "old", "")]
        {
            let wrote = connection.execute(
                "INSERT INTO trash (id, kind, thing, project, of_table, called, kept, reads, by, at)
                 VALUES (?1, 'record', ?2, ?3, 'board', 'Gone', '{}', NULL, 'user:ada', 1)",
                [id, thing, project],
            );
            wrote.unwrap();
        }
    }
    made(&file, "project:shop").await;
    let lines = studio::moved::tables();
    assert!(lines.iter().all(|(level, _)| *level == journal::keeps::RECORDS));
    let tables: Vec<_> = lines.into_iter().map(|(_, kept)| kept).collect();
    let before = held(&factory);

    let carried = journal::moved::carry(&factory, &file, "shop", &tables, &[]).unwrap();
    let went: Vec<(&str, u64)> =
        carried.iter().map(|counted| (counted.table.as_str(), counted.moves)).collect();
    assert_eq!(
        went,
        [
            ("record_history", 4),
            ("records", 2),
            ("collections", 2),
            ("project_files", 2),
            ("trash", 1),
            ("asked", 0),
            ("record_attachments", 0)
        ]
    );
    assert_eq!(held(&factory), before, "written, and nothing decided");
    let row = Row {
        name: "project:shop".into(),
        project: "shop".into(),
        file: "shop.db".into(),
        keeps: journal::keeps::RECORDS,
        made: 5,
    };
    let note = serde_json::json!({ "moved": "shop" });
    let kept = ("factory", "moved_apart");
    journal::moved::decide(&factory, &row, &tables, &[], &carried, kept, &note, std::time::Instant::now())
        .unwrap();

    // The shop's are in its file: its two tables, their records, the
    // lines of each and of its card that is gone, and its files. A
    // card of the project called `shop-2` is not the shop's, nor is the
    // line of one that is gone, by the tables there are.
    let the = |table: &'static str, rows: &[&str]| -> Vec<(&'static str, String)> {
        rows.iter().map(|row| (table, row.to_string())).collect()
    };
    let shops = [
        the("collections", &["shop/board", "shop/notes"]),
        the("records", &["shop-1", "shop-notes-1"]),
        the("record_history", &["shop-1@10", "shop-1@11", "shop-9@12", "shop-notes-1@13"]),
        the("project_files", &["shop/a.txt", "shop/b.txt"]),
    ]
    .concat();
    assert_eq!(held(&file), shops);
    // Every other is where it was: the other projects', and the line
    // nothing says the project of.
    let left: Vec<_> = before.into_iter().filter(|row| !shops.contains(row)).collect();
    assert_eq!(held(&factory), left);
    assert!(left.contains(&("record_history", "nothing-3@17".to_string())));
    assert!(left.contains(&("record_history", "shop-2-7@15".to_string())));
    // A row went as it was, every column: where a table's store is
    // reached and what declared it among them.
    let connection = Connection::open(&file).unwrap();
    let table: (String, String) = connection
        .query_row("SELECT owner, at FROM collections WHERE name = 'notes'", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(table, ("in_declared".to_string(), "http://127.0.0.1:1".to_string()));
}
