//! Whose each row is: every table kept here has a line that says how its
//! rows are told apart by project, every row is what its line says, and a
//! record is its project's from the write that makes it and never
//! another's.

use std::path::Path;
use std::sync::Arc;

use journal::apart::Told;
use journal::{Db, Journal, Places};
use kernel::{ErrorKind, ManualClock};
use serde_json::{Map, Value, json};
use studio::Studio;
use studio::collections::Spec;

use crate::{Rig, ada};

/// Every row of every table kept here, by what it is named after, with
/// the project its table's line says it is of.
async fn told_apart(rig: &Rig) -> Vec<(String, Vec<(String, Option<String>)>)> {
    // What a row of each is named after here: something that begins with
    // the project it was made for.
    let named = |table: &str| match table {
        "project_files" | "collections" | "workspace_projects" => "kept.project",
        "channels" => "kept.name",
        // Whoever makes a channel is in it under their own name: a row
        // of who is in one is named after its channel.
        "channel_members" => "(SELECT name FROM channels WHERE id = kept.channel)",
        // So is what a channel keeps to hand.
        "channel_docs" => "(SELECT name FROM channels WHERE id = kept.channel)",
        // So are its discussions, and what it offers of them.
        "discussing" | "discussions" => "(SELECT name FROM channels WHERE id = kept.channel)",
        "messages" => "kept.body",
        "attachments" => "kept.name",
        "record_history" => "kept.record",
        "sessions" => "kept.token",
        "invites" => "kept.code",
        // A choice of theme is the account's that made it.
        "themes_chosen" => "kept.account",
        // So is how somebody arranged the side panel.
        "arranged" => "kept.account",
        "clocked" | "appearance" => "kept.account",
        "group_members" => "kept.actor",
        // What somebody follows, and what changed in it, by who.
        "follows" | "follow_changes" => "kept.who",
        "workspace_channels" => "kept.channel",
        // What is in the Trash by what it was: a record of the project.
        "trash" => "kept.thing",
        _ => "kept.id",
    };
    let mut found = Vec::new();
    for (table, told) in studio::apart::TABLES {
        let read = format!("SELECT {}, {} FROM {table} kept", named(table), told.reads());
        let rows = rig
            .journal
            .db()
            .call(move |connection| {
                connection
                    .prepare(&read)?
                    .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
            .unwrap();
        found.push((table.to_string(), rows));
    }
    found
}

/// A table of `project` whose records keep what is done to them.
async fn sprints(rig: &Rig, project: &str) {
    let fields = json!({ "title": { "type": "text", "required": true } });
    let spec = Spec::read(&fields, &json!(null), None)
        .unwrap()
        .with_history(&json!({ "record.commented": "{actor} said: {body}", "record.*": "" }))
        .unwrap();
    rig.studio.collections.ensure(project, "sprints", spec, None, &ada()).await.unwrap();
}

/// Waits until `record` has `lines` of history.
async fn kept_of(rig: &Rig, record: &str, lines: usize) {
    let mut written = rig.journal.follow(0, studio::history::TOPIC);
    while rig.studio.collections.history(record).await.unwrap().len() < lines {
        written.next().await.unwrap();
    }
}

fn titled(title: &str) -> Map<String, Value> {
    json!({ "title": title }).as_object().unwrap().clone()
}

#[tokio::test]
async fn every_table_kept_here_says_how_its_rows_are_told_apart() {
    let rig = Rig::new().await;
    let made = rig.journal.db().call(|connection| {
        let mut statement = connection
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")?;
        statement.query_map([], |row| row.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()
    });
    let mut made: Vec<String> = made
        .await
        .unwrap()
        .into_iter()
        .filter(|table| !journal::copy::TABLES.contains(&table.as_str()))
        .collect();
    made.sort();
    let mut told: Vec<&str> = studio::apart::TABLES.iter().map(|(table, _)| *table).collect();
    told.sort_unstable();
    assert_eq!(
        made, told,
        "a table kept here has a line of `studio::apart::TABLES` that says how its rows are told apart \
         by project: a project's rows are to be kept in its own journal, and found there by what the \
         line says"
    );
    // And each goes into a copy, or is spoken for there, by being listed.
    let mut copied = studio::copy::TABLES.to_vec();
    copied.sort_unstable();
    assert_eq!(copied, told);
}

#[tokio::test]
async fn a_row_of_every_table_is_of_the_project_its_line_says_or_of_none() {
    let rig = Rig::new().await;
    let studio = &rig.studio;
    for project in ["shop", "yard"] {
        rig.board(project).await;
        sprints(&rig, project).await;
        let card = studio.board.add(project, "Count it", "", None, vec![], &ada()).await.unwrap();
        let sprint = studio.records.add(project, "sprints", titled("November"), &ada(), None).await.unwrap();
        studio.records.comment(&sprint.id, "Begun.", false, &ada(), None).await.unwrap();
        kept_of(&rig, &sprint.id, 2).await;
        // A record that is gone leaves its history, which is then told
        // apart by what tables there are.
        let gone = studio.records.add(project, "sprints", titled("October"), &ada(), None).await.unwrap();
        kept_of(&rig, &gone.id, 1).await;
        studio.records.remove(&gone.id, &ada(), None).await.unwrap();
        kept_of(&rig, &gone.id, 2).await;
        kept_of(&rig, &card.id, 1).await;

        let channel =
            studio.channels.create(&format!("{project} talk"), Some(project), "", &ada()).await.unwrap();
        studio.channels.join(&crate::own(), &channel.id, "agent:worker", "Worker").await.unwrap();
        studio
            .channels
            .post(&channel.id, &ada(), &format!("{project}: {} is added", card.id), None)
            .await
            .unwrap();
        let name = format!("{project}-notes.txt");
        studio
            .channels
            .attachments
            .add(&crate::own(), &channel.id, &ada(), &name, None, b"notes".to_vec())
            .await
            .unwrap();
    }
    // A channel of no project, with what is said and attached there:
    // the factory's, as a project's channel is. And a workspace with a
    // project in it.
    let general = studio.channels.create("none of them", None, "", &ada()).await.unwrap();
    studio.channels.join(&crate::own(), &general.id, "agent:worker", "Worker").await.unwrap();
    studio.channels.post(&general.id, &ada(), "none: hello", None).await.unwrap();
    studio
        .channels
        .attachments
        .add(&crate::own(), &general.id, &ada(), "none-notes.txt", None, b"notes".to_vec())
        .await
        .unwrap();
    let both = studio.workspaces.create("Both", &ada()).await.unwrap();
    studio.workspaces.place_project("shop", Some(&both.id), &ada()).await.unwrap();

    let found = told_apart(&rig).await;
    for (table, rows) in &found {
        let told = journal::apart::told(studio::apart::TABLES, table).unwrap();
        for (name, whose) in rows {
            let meant = match told {
                Told::Factory => None,
                _ => name.split(['-', ':', ' ']).next().filter(|project| *project != "none"),
            };
            assert_eq!(whose.as_deref(), meant, "`{name}` of `{table}` is told apart as another's");
        }
    }
    // Nothing of that was held for want of a row to hold.
    let rows =
        |table: &str| found.iter().find(|(name, _)| name == table).map(|(_, rows)| rows.len()).unwrap();
    for table in ["collections", "records", "record_history", "channels", "channel_members", "messages"] {
        assert!(rows(table) >= 2, "{table} has {} rows", rows(table));
    }
    assert_eq!(rows("attachments"), 3);
    assert_eq!(
        rows("record_history"),
        10,
        "for each project: a card added, two lines of the sprint that stays, two of the one that went"
    );
    assert_eq!(rows("workspace_projects"), 1);
}

#[tokio::test]
async fn a_line_of_history_of_a_record_that_is_gone_is_told_apart_by_the_tables_there_are() {
    // `shop-2-1` begins as a record of `shop` would: only the tables
    // there are say that it is the first card of `shop-2`.
    let rig = Rig::new().await;
    rig.board("Shop").await;
    rig.board("Shop 2").await;
    sprints(&rig, "shop").await;
    let card = rig.studio.board.add("shop-2", "Count it", "", None, vec![], &ada()).await.unwrap();
    assert_eq!(card.id, "shop-2-1");
    kept_of(&rig, &card.id, 1).await;
    rig.studio.board.remove(&card.id, &ada()).await.unwrap();
    let sprint = rig.studio.records.add("shop", "sprints", titled("May"), &ada(), None).await.unwrap();
    kept_of(&rig, &sprint.id, 1).await;
    rig.studio.records.remove(&sprint.id, &ada(), None).await.unwrap();

    let found = told_apart(&rig).await;
    let lines = &found.iter().find(|(table, _)| table == "record_history").unwrap().1;
    let whose = |record: &str| {
        let of: Vec<_> =
            lines.iter().filter(|(name, _)| name == record).map(|(_, whose)| whose.clone()).collect();
        assert!(!of.is_empty(), "{record} has no history");
        of
    };
    assert!(whose("shop-2-1").iter().all(|whose| whose.as_deref() == Some("shop-2")));
    assert!(whose("shop-sprints-1").iter().all(|whose| whose.as_deref() == Some("shop")));
}

#[tokio::test]
async fn a_record_is_its_projects_from_the_write_that_makes_it_and_never_anothers() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    rig.board("Yard").await;
    let card = rig.studio.board.add("shop", "Count it", "", None, vec![], &ada()).await.unwrap();
    let write = |statement: &'static str| {
        let journal = rig.journal.clone();
        async move { journal.db().call(move |connection| connection.execute(statement, [])).await }
    };
    let row = "(id, project, collection, number, data, position, created_by, created, updated)";

    // Written whole again as it is, a record is as it was written.
    let again = format!(
        "INSERT OR REPLACE INTO records {row} VALUES ('shop-1', 'shop', 'board', 1, '{{}}', 1.0, 'ada', 1, 2)"
    );
    write(Box::leak(again.into_boxed_str())).await.unwrap();

    // As another project's, it is refused, and is what it was.
    let astray = format!(
        "INSERT OR REPLACE INTO records {row} VALUES ('shop-1', 'yard', 'board', 1, '{{}}', 1.0, 'ada', 1, 2)"
    );
    let refused = write(Box::leak(astray.into_boxed_str())).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("of no other afterwards"), "{}", refused.message);
    let refused = write("UPDATE records SET project = 'yard' WHERE id = 'shop-1'").await.unwrap_err();
    assert!(refused.message.contains("of no other afterwards"), "{}", refused.message);
    let refused = write("UPDATE records SET id = 'yard-9' WHERE id = 'shop-1'").await.unwrap_err();
    assert!(refused.message.contains("of no other afterwards"), "{}", refused.message);

    // A new one called after another project is refused as it is made.
    let named = format!(
        "INSERT OR REPLACE INTO records {row} VALUES ('shop-7', 'yard', 'board', 7, '{{}}', 1.0, 'ada', 1, 2)"
    );
    let refused = write(Box::leak(named.into_boxed_str())).await.unwrap_err();
    assert!(refused.message.contains("begins with the project it is of"), "{}", refused.message);

    let (_, kept) = rig.studio.collections.record(&card.id).await.unwrap();
    assert_eq!(kept.id, "shop-1");
    assert!(rig.studio.collections.record("shop-7").await.is_err());
    assert_eq!(rig.studio.board.list("yard").await.unwrap().len(), 0);
}

#[tokio::test]
async fn a_table_whose_records_would_be_called_what_another_projects_cards_are_takes_none_of_them() {
    // `shop-2-1` is the first card of `shop-2`, and would be the first
    // record of a table of `shop`'s called `2`.
    let rig = Rig::new().await;
    rig.board("Shop").await;
    rig.board("Shop 2").await;
    let fields = json!({ "title": { "type": "text", "required": true } });
    let spec = Spec::read(&fields, &json!(null), None).unwrap();
    rig.studio.collections.ensure("shop", "2", spec, None, &ada()).await.unwrap();
    let card = rig.studio.board.add("shop-2", "Count it", "", None, vec![], &ada()).await.unwrap();
    assert_eq!(card.id, "shop-2-1");

    let refused = rig.studio.records.add("shop", "2", titled("Not a card"), &ada(), None).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict, "{}", refused.message);
    assert!(refused.message.contains("of no other afterwards"), "{}", refused.message);
    let (table, kept) = rig.studio.collections.record("shop-2-1").await.unwrap();
    assert_eq!((table.project.as_str(), table.name.as_str()), ("shop-2", "board"));
    assert_eq!(kept.data["title"], "Count it");
}

/// A factory kept in files under `dir`, where a project that is added
/// has a journal of its own: its journal and its studio.
async fn on_files(dir: &Path) -> (Journal, Studio) {
    let db = Db::open(&dir.join("journal.db")).unwrap();
    let journal = Journal::open(db, Arc::new(ManualClock::new()), "test-device").await.unwrap();
    journal.keeps_projects_in(dir.join("journals")).await.unwrap();
    let searched = Db::memory().unwrap();
    let studio =
        Studio::open(journal.clone(), searched, dir.join("projects"), dir.join("attachments")).await.unwrap();
    (journal, studio)
}

/// How many rows `table` has in `journal`'s database: none, where it
/// has no such table.
async fn rows_in(journal: &Journal, table: &'static str) -> i64 {
    let counted = journal.db().call(move |connection| {
        Ok(connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)).unwrap_or(0))
    });
    counted.await.unwrap()
}

#[tokio::test]
async fn a_card_of_a_project_added_now_and_each_line_of_its_history_are_kept_by_its_own_journal_alone() {
    if crate::one_journal() {
        return;
    }
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("studio-apart-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    let (journal, studio) = on_files(&dir).await;
    studio.projects.create("Shop", None, &ada()).await.unwrap();
    let columns = studio::board::DEFAULT_COLUMNS.iter().map(|column| column.to_string()).collect();
    studio.board.make("shop", columns, studio::Store::Local, None, &ada()).await.unwrap();
    let shop = journal.of_project("shop").unwrap();
    assert!(journal.records_of(Some("shop")).db().is(shop.db()));

    // Waits until the card has so many lines of history, wherever they
    // are said, and gives the last.
    async fn lines(journal: &Journal, studio: &Studio, card: &str, lines: usize) -> studio::history::Line {
        let mut written = journal.follow_every(Places::default(), studio::history::TOPIC);
        loop {
            let kept = studio.collections.history(card).await.unwrap();
            if kept.len() >= lines {
                assert_eq!(kept.len(), lines, "{kept:?}");
                return kept.last().unwrap().clone();
            }
            written.next().await.unwrap();
        }
    }
    let card = studio.board.add("shop", "Count it", "", None, vec![], &ada()).await.unwrap();
    lines(&journal, &studio, &card.id, 1).await;

    // A card moved, the saying of it and the line of its history: the
    // factory's own thread writes nothing for any of them.
    let before = journal.db().writes();
    studio.board.move_to(&card.id, "doing", None, &ada()).await.unwrap();
    let last = lines(&journal, &studio, &card.id, 2).await;
    assert_eq!(
        journal.db().writes(),
        before,
        "something of a card's move was written by the factory's thread"
    );
    for table in ["collections", "records", "record_history", "project_files"] {
        assert!(rows_in(&shop, table).await > 0, "the shop's own journal holds no `{table}`");
        assert_eq!(rows_in(&journal, table).await, 0, "the factory's own holds rows of `{table}`");
    }
    // Where the keeper of histories has read to in the shop's journal is
    // kept there, with the line: the event the last line is of.
    assert_eq!(shop.kv_get("history", "seq").await.unwrap(), Some(json!(last.seq)));

    // Started again, it goes on from there: what is moved next has its
    // line, and each line is there once.
    for db in journal.dbs() {
        db.close();
        db.closed().await;
    }
    let (journal, studio) = on_files(&dir).await;
    assert!(journal.records_of(Some("shop")).db().is(journal.of_project("shop").unwrap().db()));
    studio.board.move_to(&card.id, "review", None, &ada()).await.unwrap();
    let last = lines(&journal, &studio, &card.id, 3).await;
    assert_eq!(last.data["to"], "review");
    for db in journal.dbs() {
        db.close();
        db.closed().await;
    }
    let _ = std::fs::remove_dir_all(&dir);
}
