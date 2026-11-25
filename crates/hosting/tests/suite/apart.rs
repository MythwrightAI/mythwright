//! Whose each row is: every table kept here has a line that says how its
//! rows are told apart by project, and the line reads the table there is.

use crate::Rig;

#[tokio::test]
async fn every_table_kept_here_says_how_its_rows_are_told_apart() {
    let rig = Rig::new().await;
    let made = rig.journal.db().call(|connection| {
        let mut statement = connection
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")?;
        statement.query_map([], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()
    });
    // What the journal keeps for itself and what is kept for what runs
    // are told apart by their own lists.
    let mut made: Vec<String> = made
        .await
        .unwrap()
        .into_iter()
        .filter(|table| !journal::copy::TABLES.contains(&table.as_str()))
        .filter(|table| !conductor::copy::TABLES.contains(&table.as_str()))
        .collect();
    made.sort();
    let mut told: Vec<&str> = hosting::apart::TABLES.iter().map(|(table, _)| *table).collect();
    told.sort_unstable();
    assert_eq!(
        made, told,
        "a table kept here has a line of `hosting::apart::TABLES` that says how its rows are told \
         apart by project"
    );
    let mut copied = hosting::copy::TABLES.to_vec();
    copied.sort_unstable();
    assert_eq!(copied, told);

    // Each line reads the table as it is made: what it names is there.
    for (table, told) in hosting::apart::TABLES {
        let read = format!("SELECT kept.name, {} FROM {table} kept", told.reads());
        let rows = rig.journal.db().call(move |connection| {
            connection
                .prepare(&read)?
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()
        });
        assert!(rows.await.unwrap().is_empty(), "nothing was started, so `{table}` holds nothing");
    }
}
