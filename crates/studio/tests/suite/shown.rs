//! A table may say it sits beside another of its project (`beside`),
//! and which of its fields says what of that a row is about (`about`),
//! and that its rows are not written by hand (`by_hand: false`).
//! That is how it is shown and nothing else: it is kept with the table
//! and said to whoever reads what the table is, and nobody reads or
//! writes anything for it that they did not, or is kept from anything
//! they were not.

use kernel::{ErrorKind, Principal};
use serde_json::{Map, Value, json};
use studio::collections::Spec;
use studio::shown::{Group, Shown};

use crate::{Rig, ada, agent};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// What is done with the cards of a column, a row for each.
fn stages() -> Spec {
    let fields = json!({ "title": { "type": "text", "required": true }, "column": { "type": "text" } });
    Spec::read(&fields, &json!(null), None).unwrap()
}

fn beside(table: &str, about: Option<&str>) -> Shown {
    Shown {
        beside: Some(table.into()),
        about: about.map(str::to_string),
        groups: Vec::new(),
        by_hand: None,
        ..Shown::default()
    }
}

#[tokio::test]
async fn a_table_says_what_it_sits_beside_and_what_a_row_is_about_which_is_kept_and_said_with_it() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let collections = &rig.studio.collections;
    let spec = stages().sitting(Some("board"), Some("column")).unwrap();
    let made = collections.ensure("shop", "stages", spec, None, &ada()).await.unwrap();
    assert_eq!(made.shown, beside("board", Some("column")));
    // It is said to whoever reads what the table is, in the two words it
    // was declared in, and read back as it was kept.
    let said = serde_json::to_value(&made).unwrap();
    assert_eq!((&said["beside"], &said["about"]), (&json!("board"), &json!("column")), "{said}");
    assert_eq!(collections.get("shop", "stages").await.unwrap().shown, beside("board", Some("column")));
    let listed = collections.list(Some("shop")).await.unwrap();
    let listed = listed.iter().find(|table| table.name == "stages").unwrap();
    assert_eq!(listed.shown, beside("board", Some("column")));

    // Beside a table and about nothing of it; and beside one that is not
    // there, which is whoever draws it to make nothing of.
    for other in ["board", "plans"] {
        let spec = stages().sitting(Some(other), None).unwrap();
        let made = collections.ensure("shop", "agents", spec, None, &ada()).await.unwrap();
        assert_eq!(made.shown, beside(other, None));
        let said = serde_json::to_value(&made).unwrap();
        assert_eq!(said["beside"], other);
        assert!(said.get("about").is_none(), "{said}");
    }

    // The table it sits beside says nothing of it, and neither does one
    // that says nothing: nor a word left empty.
    let board = serde_json::to_value(collections.get("shop", "board").await.unwrap()).unwrap();
    assert!(board.get("beside").is_none() && board.get("about").is_none(), "{board}");
    for nothing in [stages(), stages().sitting(Some("  "), Some("")).unwrap()] {
        let plain = collections.ensure("shop", "notes", nothing, None, &ada()).await.unwrap();
        assert_eq!(plain.shown, Shown::default());
        let said = serde_json::to_value(&plain).unwrap();
        assert!(said.get("beside").is_none() && said.get("about").is_none(), "{said}");
    }

    // Declared again saying nothing, it sits beside nothing: it is the
    // declaration's own, and takes no more to say or to leave out than
    // declaring the table does.
    let narrow = agent("narrow", &["projects"]);
    let again = collections.ensure("shop", "stages", stages(), None, &narrow).await.unwrap();
    assert_eq!(again.shown, Shown::default());
    let spec = stages().sitting(Some("board"), Some("column")).unwrap();
    let again = collections.ensure("shop", "stages", spec, None, &narrow).await.unwrap();
    assert_eq!(again.shown, beside("board", Some("column")));
}

/// A board as a blueprint declares one, with a size beside a card's own.
fn cards() -> Spec {
    let card = json!({
        "title": { "type": "text", "required": true },
        "body": { "type": "text" },
        "status": { "type": "choice", "of": ["todo", "done"], "required": true },
        "assignee": { "type": "actor" },
        "labels": { "type": "list" },
        "size": { "type": "number" },
    });
    Spec::read(&card, &json!({ "board": { "kind": "board", "by": "status" } }), None).unwrap()
}

#[tokio::test]
async fn a_table_says_its_columns_in_groups_in_the_order_they_are_written_which_is_kept_and_said_with_it() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let collections = &rig.studio.collections;
    let fields = json!({
        "title": { "type": "text", "required": true },
        "role": { "type": "text" },
        "count": { "type": "number" },
        "told": { "type": "text" },
        "wall": { "type": "flag" },
    });
    let agents = || Spec::read(&fields, &json!(null), None).unwrap();
    // The groups in the order they are written, and the fields of each
    // in the order they are written: neither is the order of their names.
    let written = json!([{ "Who they are": ["role", "count"] }, { "On the device": ["wall"] }]);
    let spec = agents().grouped(&written).unwrap().sitting(Some("board"), None).unwrap();
    let made = collections.ensure("shop", "agents", spec, None, &ada()).await.unwrap();
    let group = |name: &str, fields: &[&str]| Group {
        name: name.into(),
        fields: fields.iter().map(|one| one.to_string()).collect(),
    };
    let groups = vec![group("Who they are", &["role", "count"]), group("On the device", &["wall"])];
    assert_eq!(
        made.shown,
        Shown {
            beside: Some("board".into()),
            about: None,
            groups: groups.clone(),
            by_hand: None,
            ..Shown::default()
        }
    );
    let said = serde_json::to_value(&made).unwrap();
    assert_eq!(
        said["groups"],
        json!([{ "name": "Who they are", "fields": ["role", "count"] }, { "name": "On the device", "fields": ["wall"] }])
    );
    assert_eq!(collections.get("shop", "agents").await.unwrap().shown.groups, groups);
    // Said of a table that sits beside nothing, and by one that says
    // nothing else of how it is shown.
    let alone = agents().grouped(&json!([{ "Who they are": ["count", "role"] }])).unwrap();
    let alone = collections.ensure("shop", "staff", alone, None, &ada()).await.unwrap();
    assert_eq!(
        alone.shown,
        Shown {
            beside: None,
            about: None,
            groups: vec![group("Who they are", &["count", "role"])],
            by_hand: None,
            ..Shown::default()
        }
    );
    // Declared again saying none, it has none; and one that never did
    // says nothing of groups to whoever reads it.
    let again =
        collections.ensure("shop", "staff", agents(), None, &agent("narrow", &["projects"])).await.unwrap();
    assert_eq!(again.shown, Shown::default());
    assert!(serde_json::to_value(&again).unwrap().get("groups").is_none());

    // A board says its groups too, and keeps them when its columns are
    // changed, which is the board declaring itself again as it was.
    let spec = cards().grouped(&json!([{ "How big": ["size"] }])).unwrap();
    collections.ensure("shop", "board", spec, None, &ada()).await.unwrap();
    let columns = ["todo", "doing", "done"].map(String::from).to_vec();
    rig.studio.board.set_columns("shop", columns, &ada()).await.unwrap();
    assert_eq!(
        collections.get("shop", "board").await.unwrap().shown.groups,
        vec![group("How big", &["size"])]
    );

    // What is not such a list, a field the table has not got, and a
    // field named under two headings are each refused in a sentence.
    let list = "`groups` is a list of headings in the order they are drawn, each with the fields under it in order, \
                as in `groups: [{ \"Who they are\": [role, count] }, { \"What they are told\": [told] }]`";
    for wrong in [
        json!({ "Who they are": ["role"] }),
        json!(["role", "count"]),
        json!([{ "Who they are": ["role"], "On the device": ["wall"] }]),
        json!([{ "Who they are": "role" }]),
        json!([{ "  ": ["role"] }]),
    ] {
        let refused = agents().grouped(&wrong).unwrap_err();
        assert_eq!((refused.kind, refused.message.as_str()), (ErrorKind::Invalid, list), "{wrong}");
    }
    let spec = agents().grouped(&json!([{ "Who they are": ["role", "rank"] }])).unwrap();
    let refused = collections.ensure("shop", "agents", spec, None, &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert_eq!(
        refused.message,
        "the group `Who they are` of `agents` names `rank`, and `agents` has no field called that; it has: count, \
         role, title, told, wall"
    );
    let twice = json!([{ "Who they are": ["role", "count"] }, { "What they are told": ["told", "role"] }]);
    let refused = collections.ensure("shop", "agents", agents().grouped(&twice).unwrap(), None, &ada()).await;
    assert_eq!(
        refused.unwrap_err().message,
        "`role` of `agents` is named in the group `Who they are` and again in `What they are told`: a field is \
         drawn in one place, under one heading"
    );
    assert_eq!(collections.get("shop", "agents").await.unwrap().shown.groups, groups, "as it was");
}

#[tokio::test]
async fn a_table_kept_from_before_one_could_say_so_sits_beside_nothing() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let collections = &rig.studio.collections;
    let spec = stages().sitting(Some("board"), Some("column")).unwrap();
    collections.ensure("shop", "stages", spec, None, &ada()).await.unwrap();
    // A row as a program from before left it: the column is there, since
    // this program added it, and says nothing.
    let db = rig.journal.records_of(Some("shop")).db().clone();
    db.call(|connection| {
        connection.execute("UPDATE collections SET beside = NULL WHERE name = 'stages'", [])
    })
    .await
    .unwrap();
    let kept = collections.get("shop", "stages").await.unwrap();
    assert_eq!(kept.shown, Shown::default());
    let said = serde_json::to_value(&kept).unwrap();
    assert!(said.get("beside").is_none() && said.get("about").is_none(), "{said}");
    // And what a row keeps that is no such thing is read as nothing, and
    // fails nobody who reads the table.
    db.call(|connection| {
        connection.execute("UPDATE collections SET beside = 'board' WHERE name = 'stages'", [])
    })
    .await
    .unwrap();
    assert_eq!(collections.get("shop", "stages").await.unwrap().shown, Shown::default());
}

#[tokio::test]
async fn what_a_table_cannot_sit_beside_or_be_about_is_refused_in_a_sentence() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let collections = &rig.studio.collections;
    // Itself.
    let spec = stages().sitting(Some("stages"), None).unwrap();
    let refused = collections.ensure("shop", "Stages", spec, None, &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert_eq!(
        refused.message,
        "`stages` says it sits beside `stages`, which is itself; `beside` names another table of the same \
         project, as in `beside: board`"
    );
    // A field it has not got.
    let spec = stages().sitting(Some("board"), Some("colum")).unwrap();
    let refused = collections.ensure("shop", "stages", spec, None, &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert_eq!(
        refused.message,
        "`stages` says what a row of it is about is in its field `colum` (`about`), and it has no field called \
         that; it has: column, title"
    );
    assert!(collections.get("shop", "stages").await.is_err(), "nothing was declared");
    // A table of another project's, or anything else that is not a
    // table's name as one is written.
    for written in ["depot/board", "depot:board", "Board"] {
        let refused = stages().sitting(Some(written), None).unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid);
        assert_eq!(
            refused.message,
            format!(
                "`beside` names another table of the same project, as in `beside: board`; `{written}` is not a \
                 table's name, and a table sits beside one of its own project and of no other"
            )
        );
    }
    // What has a page of its own kind under its project: a board, a
    // library and mockups. Each would be reached two ways, or be drawn
    // in a drawer as what it is not.
    let own = "each have a page of their own under their project: only a table of plain records sits beside another";
    let spec = cards().sitting(Some("stages"), None).unwrap();
    let refused = collections.ensure("shop", "board", spec, None, &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert_eq!(
        refused.message,
        format!("`board` says it sits beside `stages`, and a board, a library and mockups {own}")
    );
    for (store, name) in [("documents", "docs"), ("mockups", "mockups")] {
        let fields = json!({ "title": { "type": "text" } });
        let spec =
            Spec::read(&fields, &json!(null), Some(store)).unwrap().sitting(Some("board"), None).unwrap();
        let refused = collections.ensure("shop", name, spec, None, &ada()).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{store}");
        assert_eq!(
            refused.message,
            format!("`{name}` says it sits beside `board`, and a board, a library and mockups {own}")
        );
        assert!(collections.get("shop", name).await.is_err(), "{store}: nothing was declared");
    }
    // And what a row is about, of nothing.
    let refused = stages().sitting(None, Some("column")).unwrap_err();
    assert_eq!(
        refused.message,
        "`about: column` says which field says what a row is about of the table this one sits beside, and no \
         `beside` says which table that is"
    );
}

/// What each of them may do to a table and to the board, as words: done,
/// or why it was refused.
async fn tried(rig: &Rig, table: &str, by: &Principal) -> Vec<String> {
    let (records, board) = (&rig.studio.records, &rig.studio.board);
    let came = |done: kernel::Result<()>| done.map_or_else(|refused| refused.message, |()| "done".into());
    let mut all = Vec::new();
    all.push(came(records.list("shop", table, by).await.map(|rows| assert_eq!(rows.len(), 1))));
    let added =
        records.add("shop", table, data(json!({ "title": "work", "column": "ready" })), by, None).await;
    let row = records.list("shop", table, &ada()).await.unwrap()[0].id.clone();
    all.push(came(records.update(&row, data(json!({ "column": "doing" })), by, None).await.map(drop)));
    all.push(came(records.get(&row, by).await.map(drop)));
    if let Ok(added) = &added {
        all.push(came(records.remove(&added.id, by, None).await.map(|_| ())));
    }
    all.push(came(added.map(drop)));
    all.push(came(board.add("shop", "A card", "", None, Vec::new(), by).await.map(drop)));
    all.into_iter().map(|said| said.replace(table, "the table")).collect()
}

#[tokio::test]
async fn sitting_beside_a_table_lets_nobody_in_and_keeps_nobody_out_of_either() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let collections = &rig.studio.collections;
    let spec = stages().sitting(Some("board"), Some("column")).unwrap();
    collections.ensure("shop", "stages", spec, None, &ada()).await.unwrap();
    collections.ensure("shop", "plain", stages(), None, &ada()).await.unwrap();
    for table in ["stages", "plain"] {
        let first = data(json!({ "title": "review", "column": "review" }));
        rig.studio.records.add("shop", table, first, &ada(), None).await.unwrap();
    }
    // Whoever may write records and cards, whoever may only read them,
    // whoever may do one and not the other, and whoever holds nothing:
    // each is answered alike of the table that sits beside the board and
    // of one that sits beside nothing, and of the board whichever is asked.
    let keys: [&[&str]; 6] = [
        &["projects", "records", "board"],
        &["projects:read", "records:read", "board:read"],
        &["projects:read", "records"],
        &["projects:read", "board"],
        &["projects:read"],
        &[],
    ];
    for scopes in keys {
        let by = agent("narrow", scopes);
        let beside = tried(&rig, "stages", &by).await;
        let plain = tried(&rig, "plain", &by).await;
        assert_eq!(beside, plain, "{scopes:?}");
    }
    // And the first of those did write, so the two were compared doing
    // something.
    let wrote = tried(&rig, "stages", &agent("narrow", keys[0])).await;
    assert!(wrote.iter().all(|said| said == "done"), "{wrote:?}");
    let read = tried(&rig, "stages", &agent("narrow", keys[1])).await;
    assert_eq!(read.iter().filter(|said| *said == "done").count(), 2, "{read:?}");
}

#[tokio::test]
async fn a_table_says_its_rows_are_not_written_by_hand_which_is_kept_and_said_with_it_until_it_is_declared_again_saying_nothing()
 {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let collections = &rig.studio.collections;
    let runs = || stages().by_hand(&json!(false)).unwrap();
    let made = collections.ensure("shop", "test-runs", runs(), None, &ada()).await.unwrap();
    assert_eq!(made.shown, Shown { by_hand: Some(false), ..Shown::default() });
    // Said to whoever reads what the table is in the word it was declared
    // in, and read back as it was kept, beside and in groups too.
    let said = serde_json::to_value(&made).unwrap();
    assert_eq!(said["by_hand"], json!(false), "{said}");
    assert_eq!(collections.get("shop", "test-runs").await.unwrap().shown.by_hand, Some(false));
    let listed = collections.list(Some("shop")).await.unwrap();
    assert_eq!(listed.iter().find(|table| table.name == "test-runs").unwrap().shown.by_hand, Some(false));
    let all = stages()
        .by_hand(&json!(false))
        .unwrap()
        .sitting(Some("board"), Some("column"))
        .unwrap()
        .grouped(&json!([{ "Where": ["column"] }]))
        .unwrap();
    let both = collections.ensure("shop", "stages", all, None, &ada()).await.unwrap();
    assert_eq!((both.shown.beside.as_deref(), both.shown.by_hand), (Some("board"), Some(false)));
    assert_eq!(both.shown.groups.len(), 1);

    // `true` is what saying nothing says, and is kept as nothing said: a
    // table that says it is shown exactly as one that does not.
    for usual in [json!(true), json!(null)] {
        let spec = stages().by_hand(&usual).unwrap();
        let plain = collections.ensure("shop", "notes", spec, None, &ada()).await.unwrap();
        assert_eq!(plain.shown, Shown::default(), "{usual}");
        assert!(serde_json::to_value(&plain).unwrap().get("by_hand").is_none(), "{usual}");
    }

    // Declared again saying nothing, its rows are written by hand again;
    // saying it or leaving it out takes no more than declaring the table.
    let narrow = agent("narrow", &["projects"]);
    let again = collections.ensure("shop", "test-runs", stages(), None, &narrow).await.unwrap();
    assert_eq!(again.shown, Shown::default());
    let again = collections.ensure("shop", "test-runs", runs(), None, &narrow).await.unwrap();
    assert_eq!(again.shown.by_hand, Some(false));
}

#[tokio::test]
async fn what_cannot_say_its_rows_are_not_written_by_hand_is_refused_in_a_sentence() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let collections = &rig.studio.collections;
    for wrong in [json!("false"), json!(0), json!(["no"])] {
        let refused = stages().by_hand(&wrong).unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{wrong}");
        assert!(
            refused.message.starts_with(
                "`by_hand` is `false` for a table whose rows are written only by the commands and blueprints \
                 that keep it, and `true` (the usual) for one people write in too; `"
            ),
            "{}",
            refused.message
        );
    }
    // What has a page of its own kind under its project.
    let own = "and a board, a library and mockups each have a page of their own under their project: only a table \
               of plain records says so";
    let spec = cards().by_hand(&json!(false)).unwrap();
    let refused = collections.ensure("shop", "board", spec, None, &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert_eq!(
        refused.message,
        format!("`board` says its rows are not written by hand (`by_hand: false`), {own}")
    );
    for (store, name) in [("documents", "docs"), ("mockups", "mockups")] {
        let fields = json!({ "title": { "type": "text" } });
        let spec = Spec::read(&fields, &json!(null), Some(store)).unwrap().by_hand(&json!(false)).unwrap();
        let refused = collections.ensure("shop", name, spec, None, &ada()).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{store}");
        assert_eq!(
            refused.message,
            format!("`{name}` says its rows are not written by hand (`by_hand: false`), {own}")
        );
        assert!(collections.get("shop", name).await.is_err(), "{store}: nothing was declared");
    }
    // A board that says `true` says nothing, and is declared as it was.
    collections.ensure("shop", "board", cards().by_hand(&json!(true)).unwrap(), None, &ada()).await.unwrap();
}

#[tokio::test]
async fn rows_not_written_by_hand_let_nobody_in_and_keep_nobody_out_and_every_door_writes_them_as_before() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let collections = &rig.studio.collections;
    collections.ensure("shop", "kept", stages().by_hand(&json!(false)).unwrap(), None, &ada()).await.unwrap();
    collections.ensure("shop", "plain", stages(), None, &ada()).await.unwrap();
    for table in ["kept", "plain"] {
        let first = data(json!({ "title": "run", "column": "passed" }));
        rig.studio.records.add("shop", table, first, &ada(), None).await.unwrap();
    }
    // Whoever may write records, whoever may only read them, and whoever
    // holds nothing: each is answered alike of either, adding, changing,
    // reading and taking a row away.
    let keys: [&[&str]; 4] = [
        &["projects", "records", "board"],
        &["projects:read", "records:read", "board:read"],
        &["projects:read"],
        &[],
    ];
    for scopes in keys {
        let by = agent("narrow", scopes);
        assert_eq!(tried(&rig, "kept", &by).await, tried(&rig, "plain", &by).await, "{scopes:?}");
    }
    let wrote = tried(&rig, "kept", &agent("narrow", keys[0])).await;
    assert!(wrote.iter().all(|said| said == "done"), "{wrote:?}");
}

#[tokio::test]
async fn a_table_says_what_it_is_called_for_people_and_what_somebody_calls_it_stands_when_it_is_declared_again()
 {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let collections = &rig.studio.collections;
    let declared = || stages().titled(Some("  What each column does ")).unwrap();
    let made = collections.ensure("shop", "stages", declared(), None, &ada()).await.unwrap();
    assert_eq!(made.shown.title.as_deref(), Some("What each column does"));
    let chosen = collections.title("shop", "stages", Some("Stages"), &ada()).await.unwrap();
    assert_eq!((chosen.name.as_str(), chosen.shown.title_chosen.as_deref()), ("stages", Some("Stages")));
    let again = collections.ensure("shop", "stages", declared(), None, &ada()).await.unwrap();
    assert_eq!(again.shown.title_chosen.as_deref(), Some("Stages"), "what somebody called it stands");
    // Called what it is declared to be, or nothing, the choice is taken away.
    let back = collections.title("shop", "stages", Some("What each column does"), &ada()).await.unwrap();
    assert_eq!(back.shown.title_chosen, None);
    // A board keeps its words, and somebody who may not write the project's files calls nothing anything.
    assert_eq!(
        collections.title("shop", "board", Some("Cards"), &ada()).await.unwrap_err().kind,
        ErrorKind::Invalid
    );
    let narrow = agent("narrow", &["projects:read", "records"]);
    assert_eq!(
        collections.title("shop", "stages", Some("Mine"), &narrow).await.unwrap_err().kind,
        ErrorKind::Denied
    );
}

#[tokio::test]
async fn a_table_says_what_it_is_for_and_which_fields_a_record_is_called_by_which_are_kept_and_said_with_it()
{
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let collections = &rig.studio.collections;
    // What each contender of a card's A/B test did: its title is what the test finds it by, and it is called by its
    // card and its contender.
    let fields = json!({
        "title": { "type": "text", "required": true },
        "card": { "type": "text", "calls": 1 },
        "way": { "type": "text", "calls": 2 },
    });
    let attempts = || Spec::read(&fields, &json!(null), None).unwrap();
    let said = "What each contender of a test did with its card, a row each.";
    let made = collections
        .ensure("shop", "entries", attempts().described(Some(said)).unwrap(), None, &ada())
        .await
        .unwrap();
    assert_eq!(made.shown.description.as_deref(), Some(said));
    let read = collections.get("shop", "entries").await.unwrap();
    let calls = |name: &str| read.fields.iter().find(|field| field.name == name).unwrap().calls;
    assert_eq!((calls("title"), calls("card"), calls("way")), (None, Some(1), Some(2)));
    let handed = serde_json::to_value(&read).unwrap();
    assert_eq!(handed["description"], json!(said), "{handed}");
    // Declared again saying nothing, it says nothing.
    let again = collections.ensure("shop", "entries", attempts(), None, &ada()).await.unwrap();
    assert_eq!(again.shown.description, None);
    // What is no place among them, and what is no sentence, are refused in a sentence.
    let nowhere = Spec::read(&json!({ "card": { "type": "text", "calls": 0 } }), &json!(null), None);
    assert!(nowhere.is_err_and(|err| err.to_string().contains("a number from 1 to 9")));
    let long = "x".repeat(studio::shown::LONGEST_DESCRIPTION + 1);
    assert!(attempts().described(Some(&long)).is_err_and(|err| err.to_string().contains("one line")));
    assert!(attempts().described(Some("two\nlines")).is_err());
}
