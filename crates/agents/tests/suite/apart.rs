//! Whose each row the agents keep is: every table says how its rows are
//! told apart by project; a turn says the project of its slot as it is
//! queued, and no other afterwards; and one kept before a turn said so is
//! reached through its slot.

use agents::apart::TABLES;
use agents::{SlotSpec, TurnRequest};
use journal::Db;
use rusqlite::params;

use crate::{Rig, ada};

/// An agent called `name`, of `project` or of none.
async fn agent(rig: &Rig, name: &str, project: Option<&str>) {
    let spec = SlotSpec {
        name: name.into(),
        adapter: Some("fake".into()),
        project: project.map(str::to_string),
        ..SlotSpec::default()
    };
    rig.agents.ensure(spec, &ada()).await.unwrap();
}

/// Work asked of `slot`, and its turn.
async fn asked(rig: &Rig, slot: &str, words: &str) -> String {
    let request = TurnRequest {
        slot: slot.into(),
        kind: "task".into(),
        thread: format!("about:{words}"),
        prompt: words.into(),
        ..TurnRequest::default()
    };
    rig.agents.enqueue(request).await.unwrap()
}

/// Every row of every table the agents keep, in every file that keeps
/// turns, read against its line: what each turn is named after (its
/// slot, called for the project it began in), and whose the line says
/// it is. How many rows each table has, in all of them together.
async fn told_apart(rig: &Rig) -> Vec<(String, usize)> {
    let read = |connection: &mut rusqlite::Connection| {
        let mut found = Vec::new();
        for (table, told) in TABLES {
            // What a row is named after: a turn after its slot, which is
            // called for the project it began in.
            let named = if told.of_a_project() { "slot" } else { "rowid" };
            let reads = told.reads();
            let mut statement = connection
                .prepare(&format!("SELECT CAST(kept.{named} AS TEXT), {reads} FROM {table} kept"))?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            found.push((table.to_string(), *told, rows));
        }
        Ok(found)
    };
    let mut counted: Vec<(String, usize)> = TABLES.iter().map(|(table, _)| (table.to_string(), 0)).collect();
    for (table, told, rows) in rig.of_turns(read).await.into_iter().flatten() {
        for (name, whose) in &rows {
            let meant = match told.of_a_project() {
                false => None,
                true => name.split('-').next().filter(|project| *project != "none"),
            };
            assert_eq!(whose.as_deref(), meant, "`{name}` of `{table}` is told apart as another's");
        }
        counted.iter_mut().find(|(name, _)| *name == table).unwrap().1 += rows.len();
    }
    counted
}

/// The ids of the turns `journal`'s file holds, in the order of its rows.
async fn turns_in(journal: &journal::Journal) -> Vec<String> {
    let read = journal.db().call(|connection| {
        let mut statement = connection.prepare("SELECT id FROM turns ORDER BY rowid")?;
        statement.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<Vec<String>>>()
    });
    read.await.unwrap()
}

/// What a turn's own row says of whose it is.
async fn says(rig: &Rig, turn: &str) -> Option<String> {
    let turn = turn.to_string();
    let said = move |connection: &mut rusqlite::Connection| {
        journal::optional(connection.query_row("SELECT project FROM turns WHERE id = ?1", [&turn], |row| {
            row.get::<_, Option<String>>(0)
        }))
    };
    rig.of_turns(said).await.into_iter().flatten().next().expect("there is such a turn")
}

#[tokio::test]
async fn every_table_the_agents_keep_says_how_its_rows_are_told_apart() {
    let db = Db::memory().unwrap();
    agents::apart::make(&db).await.unwrap();
    let made = db.call(|connection| {
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
    let mut told: Vec<&str> = TABLES.iter().map(|(table, _)| *table).collect();
    told.sort_unstable();
    assert_eq!(
        made, told,
        "a table the agents keep has a line of `agents::apart::TABLES` that says how its rows are told \
         apart by project: a project's rows are to be moved to its own journal, and not worked out"
    );
    // And each goes into a copy, or is emptied there, by being said.
    let mut copied = agents::copy::TABLES.to_vec();
    copied.sort_unstable();
    assert_eq!(copied, told);
}

#[tokio::test]
async fn a_turn_says_the_project_of_its_slot_as_it_is_queued_and_no_other_afterwards() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.projects.create("Yard", None, &ada()).await.unwrap();
    agent(&rig, "shop-worker", Some("shop")).await;
    agent(&rig, "yard-worker", Some("yard")).await;
    agent(&rig, "none-helper", None).await;

    let shops = asked(&rig, "shop-worker", "a shelf").await;
    let yards = asked(&rig, "yard-worker", "a gate").await;
    let nobodys = asked(&rig, "none-helper", "a question").await;
    rig.turns_finished(3).await;

    // Each says it in its own row: the empty text for nobody's, so that
    // it is told from one that says nothing.
    assert_eq!(says(&rig, &shops).await.as_deref(), Some("shop"));
    assert_eq!(says(&rig, &yards).await.as_deref(), Some("yard"));
    assert_eq!(says(&rig, &nobodys).await.as_deref(), Some(""));
    let rows = told_apart(&rig).await;
    assert_eq!(rows, [("slots".to_string(), 3), ("turns".to_string(), 3), ("signins".to_string(), 0)]);
    // And each is kept in its project's own journal, where a project has
    // one, with what is said of it; nobody's in the factory's.
    let apart = journal::apart() != Some(false);
    let (of_shop, of_yard) = (rig.turns_of("shop"), rig.turns_of("yard"));
    assert_eq!(of_shop.db().is(rig.journal.db()), !apart);
    if apart {
        assert_eq!(turns_in(&of_shop).await, std::slice::from_ref(&shops));
        assert_eq!(turns_in(&of_yard).await, std::slice::from_ref(&yards));
        assert_eq!(turns_in(&rig.journal).await, std::slice::from_ref(&nobodys));
        let said = of_shop.read(journal::EventQuery { topics: "agent.turn.*".into(), ..Default::default() });
        let said: Vec<String> = said.await.unwrap().into_iter().map(|event| event.topic).collect();
        assert_eq!(said, ["agent.turn.queued", "agent.turn.started", "agent.turn.finished"]);
    }

    // The agent is given to another project: what it did before is the
    // first one's still, begun, worked and ended as it was, and what it
    // is asked from now on is the other's.
    agent(&rig, "shop-worker", Some("yard")).await;
    assert_eq!(rig.agents.slot("shop-worker").await.unwrap().project.as_deref(), Some("yard"));
    assert_eq!(says(&rig, &shops).await.as_deref(), Some("shop"));
    assert_eq!(rig.agents.turn(&shops).await.unwrap().status, "done");
    let later = asked(&rig, "shop-worker", "a fence").await;
    rig.turns_finished(4).await;
    assert_eq!(says(&rig, &later).await.as_deref(), Some("yard"));
    // So its turns are in two files, and are still its turns, the
    // newest first.
    if apart {
        assert_eq!(turns_in(&of_shop).await, std::slice::from_ref(&shops));
        assert_eq!(turns_in(&of_yard).await, [yards.clone(), later.clone()]);
    }
    let its: Vec<String> =
        rig.agents.turns("shop-worker", 10).await.unwrap().into_iter().map(|turn| turn.id).collect();
    assert_eq!(its, [later.clone(), shops.clone()]);

    // And nothing makes a turn another's, or another agent's: the
    // database refuses the write, whoever asks it.
    for change in ["project = 'yard'", "project = NULL", "slot = 'yard-worker'"] {
        let (turn, change) = (shops.clone(), change.to_string());
        let refused = of_shop.db().call(move |connection| {
            connection.execute(&format!("UPDATE turns SET {change} WHERE id = ?1"), [&turn])
        });
        let refused = refused.await.expect_err("a turn made another's");
        assert!(refused.message.contains("of no other afterwards"), "{}", refused.message);
    }
    assert_eq!(says(&rig, &shops).await.as_deref(), Some("shop"));
}

#[tokio::test]
async fn a_turn_from_before_says_nothing_and_is_reached_through_its_slot() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    agent(&rig, "shop-worker", Some("shop")).await;
    agent(&rig, "none-helper", None).await;
    // As a program from before left them: the column is added to such a
    // row and nothing is written in it.
    for (id, slot) in [("turn_before", "shop-worker"), ("turn_nobodys", "none-helper")] {
        let wrote = rig.journal.db().call(move |connection| {
            connection.execute(
                "INSERT INTO turns (id, slot, kind, thread, prompt, options, status, created, finished)
                 VALUES (?1, ?2, 'task', 'about:before', 'a shelf', '[]', 'done', 1, 2)",
                params![id, slot],
            )
        });
        wrote.await.unwrap();
    }
    assert_eq!(says(&rig, "turn_before").await, None);
    // Read against its line, each is its slot's project's, or nobody's.
    let rows = told_apart(&rig).await;
    assert!(rows.contains(&("turns".to_string(), 2)), "{rows:?}");
    // It is still a turn like any other: read, and listed with its agent's.
    assert_eq!(rig.agents.turn("turn_before").await.unwrap().prompt, "a shelf");
    assert_eq!(rig.agents.turns("shop-worker", 10).await.unwrap().len(), 1);
}
