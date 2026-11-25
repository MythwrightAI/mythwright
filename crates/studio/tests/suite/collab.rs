use kernel::ErrorKind;
use serde_json::json;
use studio::Changes;

use crate::{Rig, ada, agent};

#[tokio::test]
async fn cards_are_numbered_per_project_and_moving_one_is_an_event() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    rig.board("Blog").await;
    let board = &rig.studio.board;
    let first = board
        .add("shop", "Add a cart", "Customers need one.", None, vec!["backend".into()], &ada())
        .await
        .unwrap();
    let second = board.add("shop", "Take payment", "", None, vec![], &ada()).await.unwrap();
    let other = board.add("blog", "Write a post", "", None, vec![], &ada()).await.unwrap();
    assert_eq!((first.id.as_str(), second.id.as_str(), other.id.as_str()), ("shop-1", "shop-2", "blog-1"));
    assert_eq!(first.status, "draft", "a new card starts in the first column");

    let moved = board.move_to("shop-1", "ready", None, &ada()).await.unwrap();
    assert_eq!(moved.status, "ready");
    let event = &rig.events("board.item.moved").await[0];
    assert_eq!(
        event.data,
        json!({ "id": "shop-1", "project": "shop", "title": "Add a cart", "from": "draft", "to": "ready", "by": "user:ada" })
    );

    let err = board.move_to("shop-1", "shipped", None, &ada()).await.unwrap_err();
    assert!(err.message.contains("its columns are: draft, ready, doing, review, done"), "{}", err.message);
    assert_eq!(board.move_to("shop-9", "ready", None, &ada()).await.unwrap_err().kind, ErrorKind::NotFound);
    let reader = agent("reviewer", &["board:read"]);
    assert_eq!(board.move_to("shop-1", "doing", None, &reader).await.unwrap_err().kind, ErrorKind::Denied);
}

#[tokio::test]
async fn cards_keep_their_order_within_a_column() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let board = &rig.studio.board;
    for title in ["a", "b", "c", "d"] {
        board.add("shop", title, "", None, vec![], &ada()).await.unwrap();
    }
    let order = || async {
        board
            .list("shop")
            .await
            .unwrap()
            .into_iter()
            .map(|item| format!("{}:{}", item.status, item.title))
            .collect::<Vec<_>>()
    };
    assert_eq!(order().await, ["draft:a", "draft:b", "draft:c", "draft:d"]);

    // d above b; then a to the bottom of ready; then c above a in ready.
    board.move_to("shop-4", "draft", Some("shop-2"), &ada()).await.unwrap();
    assert_eq!(order().await, ["draft:a", "draft:d", "draft:b", "draft:c"]);
    board.move_to("shop-1", "ready", None, &ada()).await.unwrap();
    board.move_to("shop-3", "ready", Some("shop-1"), &ada()).await.unwrap();
    assert_eq!(order().await, ["draft:d", "draft:b", "ready:c", "ready:a"]);
    assert_eq!(rig.events("board.item.moved").await.len(), 2, "reordering within a column is not a move");
}

/// Where a card goes in a column, and the cards of one column, are asked
/// of that column alone: each comes to what reading the board whole came
/// to, wherever the board is kept.
#[tokio::test]
async fn a_column_is_read_by_itself_and_comes_to_what_the_board_lists_there() {
    for store in [studio::Store::Local, studio::Store::Files] {
        let rig = Rig::new().await;
        rig.board_in("Shop", store).await;
        let board = &rig.studio.board;
        for title in ["a", "b", "c", "d", "e", "f"] {
            board.add("shop", title, "", None, vec![], &ada()).await.unwrap();
        }
        let order = || async {
            let listed = board.list("shop").await.unwrap();
            listed.into_iter().map(|item| format!("{}:{}", item.status, item.title)).collect::<Vec<_>>()
        };
        // b to ready, d above it; f above itself and e above a card of
        // another column, each of which is the bottom; c to the top of
        // draft.
        board.move_to("shop-2", "ready", None, &ada()).await.unwrap();
        board.move_to("shop-4", "ready", Some("shop-2"), &ada()).await.unwrap();
        board.move_to("shop-6", "ready", Some("shop-6"), &ada()).await.unwrap();
        board.move_to("shop-5", "ready", Some("shop-1"), &ada()).await.unwrap();
        board.move_to("shop-3", "draft", Some("shop-1"), &ada()).await.unwrap();
        assert_eq!(
            order().await,
            ["draft:c", "draft:a", "ready:d", "ready:b", "ready:f", "ready:e"],
            "{store:?}"
        );
        // Added to a column, and moved to the bottom of one, is after the last there.
        board.add("shop", "g", "", Some("ready"), vec![], &ada()).await.unwrap();
        board.move_to("shop-4", "draft", None, &ada()).await.unwrap();
        let listed = order().await;
        assert_eq!(
            listed,
            ["draft:c", "draft:a", "draft:d", "ready:b", "ready:f", "ready:e", "ready:g"],
            "{store:?}"
        );

        for column in studio::board::DEFAULT_COLUMNS {
            let alone: Vec<String> = board
                .column("shop", column)
                .await
                .unwrap()
                .into_iter()
                .map(|item| format!("{}:{}", item.status, item.title))
                .collect();
            let there: Vec<&String> =
                listed.iter().filter(|card| card.starts_with(&format!("{column}:"))).collect();
            assert_eq!(alone.iter().collect::<Vec<_>>(), there, "{store:?}, {column}");
        }
    }
}

#[tokio::test]
async fn a_card_can_be_changed_and_only_real_changes_are_announced() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let board = &rig.studio.board;
    board.add("shop", "Add a cart", "", None, vec![], &ada()).await.unwrap();

    let changes = Changes {
        assignee: Some(Some("agent:builder".into())),
        body: Some("With tests.".into()),
        ..Changes::default()
    };
    let item = board.update("shop-1", changes.clone(), &ada()).await.unwrap();
    assert_eq!((item.assignee.as_deref(), item.body.as_str()), (Some("agent:builder"), "With tests."));
    board.update("shop-1", changes, &ada()).await.unwrap();
    let updates = rig.events("board.item.updated").await;
    assert_eq!(updates.len(), 1, "saying the same thing again is not a change");
    assert_eq!(updates[0].data["changed"], json!(["body", "assignee"]));

    let cleared =
        board.update("shop-1", Changes { assignee: Some(None), ..Changes::default() }, &ada()).await.unwrap();
    assert_eq!(cleared.assignee, None);
    board.remove("shop-1", &ada()).await.unwrap();
    assert!(board.list("shop").await.unwrap().is_empty());
}

#[tokio::test]
async fn two_changes_to_one_card_at_the_same_moment_are_both_kept() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let board = &rig.studio.board;
    board.add("shop", "Add a cart", "", None, vec![], &ada()).await.unwrap();
    // A card is moved by one thing while another writes on it: its lane's
    // round takes it as its own blueprint takes a note off it. Each reads
    // the card, changes it and writes the whole of it back; side by side,
    // whichever wrote last would write the card as it was before the other.
    let columns = ["ready", "doing", "review", "draft"];
    for (round, column) in columns.iter().cycle().take(40).enumerate() {
        let body = format!("round {round}");
        let written = Changes { body: Some(body.clone()), ..Changes::default() };
        let who = ada();
        let (moved, changed) =
            tokio::join!(board.move_to("shop-1", column, None, &who), board.update("shop-1", written, &who));
        moved.unwrap();
        changed.unwrap();
        let card = board.get("shop-1").await.unwrap();
        assert_eq!((card.status.as_str(), card.body.as_str()), (*column, body.as_str()), "round {round}");
    }
}

#[tokio::test]
async fn cards_added_in_the_same_moment_are_each_kept_under_a_number_of_their_own() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let board = &rig.studio.board;
    let who = ada();
    // Four at once, ten times over: as an agent that writes out the cards
    // for a piece of work does, or four people at four screens. Each was
    // numbered after the last card there was, and all four found the same
    // last: one number between them, and three cards written over.
    for round in 0..10 {
        let titles: Vec<String> = (1..=4).map(|n| format!("Card {round}.{n}")).collect();
        let add = |n: usize| board.add("shop", &titles[n], "", None, vec![], &who);
        let (a, b, c, d) = tokio::join!(add(0), add(1), add(2), add(3));
        let mut ids = vec![a.unwrap().id, b.unwrap().id, c.unwrap().id, d.unwrap().id];
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 4, "round {round}: each has an id of its own: {ids:?}");
    }
    let cards = board.list("shop").await.unwrap();
    assert_eq!(cards.len(), 40, "every card that was added is there");
    let mut titles: Vec<&str> = cards.iter().map(|card| card.title.as_str()).collect();
    titles.sort();
    titles.dedup();
    assert_eq!(titles.len(), 40, "and none is another's");
    // And one is found by its id among them, whichever it is.
    assert_eq!(board.get("shop-40").await.unwrap().id, "shop-40");
    assert_eq!(board.get("shop-41").await.unwrap_err().kind, kernel::ErrorKind::NotFound);
}

#[tokio::test]
async fn a_board_can_have_its_own_columns() {
    let rig = Rig::new().await;
    rig.board("Research").await;
    rig.board("Other").await;
    let board = &rig.studio.board;
    let columns = board
        .set_columns("research", vec!["Question".into(), "Reading".into(), "Answered".into()], &ada())
        .await
        .unwrap();
    assert_eq!(columns, ["question", "reading", "answered"]);
    let item = board.add("research", "Which queue?", "", None, vec![], &ada()).await.unwrap();
    assert_eq!(item.status, "question");
    assert!(
        board.move_to(&item.id, "ready", None, &ada()).await.is_err(),
        "the default columns no longer apply"
    );
    assert_eq!(board.columns("other").await.unwrap(), ["draft", "ready", "doing", "review", "done"]);

    // A column that still has cards in it is not taken away from under them.
    let kept =
        board.set_columns("research", vec!["Reading".into(), "Answered".into()], &ada()).await.unwrap_err();
    assert_eq!(kept.kind, ErrorKind::Conflict);
    assert!(kept.message.contains("`question` is still the `status` of 1 record(s)"), "{}", kept.message);
}

#[tokio::test]
async fn a_project_has_no_board_until_one_is_made_for_it() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let board = &rig.studio.board;
    assert!(!board.exists("shop").await);
    for refused in [
        board.add("shop", "Add a cart", "", None, vec![], &ada()).await.map(|_| ()),
        board.list("shop").await.map(|_| ()),
        board.columns("shop").await.map(|_| ()),
    ] {
        let err = refused.unwrap_err();
        assert_eq!(err.kind, ErrorKind::NotFound);
        assert!(
            err.message.contains("has no board yet") && err.message.contains("apply `std/board`"),
            "{}",
            err.message
        );
    }
    // And a board is of a project: there is none for one that does not exist.
    let columns = vec!["todo".to_string()];
    assert!(board.make("nowhere", columns, studio::Store::Local, None, &ada()).await.is_err());

    rig.board("Shop").await;
    assert!(board.exists("shop").await);
    assert_eq!(board.list("shop").await.unwrap(), []);
    let made = rig.events("collection.ensured").await;
    assert_eq!(
        (&made[0].data["name"], &made[0].data["store"], &made[0].data["made"]),
        (&json!("board"), &json!("local"), &json!(true))
    );
}

#[tokio::test]
async fn a_board_kept_as_files_is_one_file_a_card_that_anyone_can_read_and_change() {
    let rig = Rig::new().await;
    rig.board_in("Shop", studio::Store::Files).await;
    let board = &rig.studio.board;
    let dir = rig.studio.projects.dir_of("shop").unwrap().join("board");

    let cart = board
        .add("shop", "Add a cart", "Customers need one.", None, vec!["backend".into()], &ada())
        .await
        .unwrap();
    board.add("shop", "Take payment", "", None, vec![], &ada()).await.unwrap();
    assert_eq!((cart.id.as_str(), cart.number), ("shop-1", 1));
    let file = std::fs::read_to_string(dir.join("1.md")).unwrap();
    assert!(file.starts_with("---\nstatus: \"draft\"\n"), "{file}");
    assert!(file.ends_with("---\n# Add a cart\n\nCustomers need one.\n"), "{file}");
    assert!(file.contains("labels: [\"backend\"]\n"), "{file}");

    // It behaves as any board does.
    board.move_to("shop-2", "ready", None, &ada()).await.unwrap();
    board
        .update("shop-1", Changes { title: Some("Add a basket".into()), ..Changes::default() }, &ada())
        .await
        .unwrap();
    let cards: Vec<(String, String)> = board
        .list("shop")
        .await
        .unwrap()
        .into_iter()
        .map(|card| (card.card.title, card.card.status))
        .collect();
    assert_eq!(cards, [("Add a basket".into(), "draft".into()), ("Take payment".into(), "ready".into())]);
    assert_eq!(rig.topics("board.item.moved").await.len(), 1);

    // Changed by hand, in plain words, it is read as it now is.
    let by_hand = "---\nstatus: doing\nassignee: agent:builder\n---\n# Add a basket\n\nWith a count on it.\n";
    std::fs::write(dir.join("1.md"), by_hand).unwrap();
    let card = board.get("shop-1").await.unwrap();
    assert_eq!(
        (card.status.as_str(), card.assignee.as_deref(), card.body.as_str()),
        ("doing", Some("agent:builder"), "With a count on it.")
    );
    // A file somebody added is a card too; a card whose file is gone is gone.
    std::fs::write(dir.join("7.md"), "---\nstatus: review\n---\n# Written in an editor\n").unwrap();
    std::fs::remove_file(dir.join("2.md")).unwrap();
    let titles: Vec<String> =
        board.list("shop").await.unwrap().into_iter().map(|card| card.card.title).collect();
    assert_eq!(titles, ["Add a basket", "Written in an editor"]);
    assert_eq!(
        board.add("shop", "Next", "", None, vec![], &ada()).await.unwrap().id,
        "shop-8",
        "numbers carry on from what is there"
    );

    // Where a board is kept is settled when it is made.
    let moved =
        board.make("shop", vec!["todo".into()], studio::Store::Local, None, &ada()).await.unwrap_err();
    assert_eq!(moved.kind, ErrorKind::Conflict);
    assert!(moved.message.contains("is kept in `files`"), "{}", moved.message);
}

#[tokio::test]
async fn a_board_from_before_boards_were_made_by_blueprints_keeps_its_cards() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    // As an installation from then has it: cards in one table for every
    // project, and a project's columns kept beside them.
    rig.journal.kv_set("board.columns", "shop", &json!(["todo", "doing"])).await.unwrap();
    rig.journal
        .db()
        .call(|connection| {
            let card = "INSERT INTO board_items
                (id, project, number, title, body, status, assignee, labels, position, created_by, created, updated)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'user:ada', 100, 200)";
            connection.execute(card, rusqlite::params!["shop-1", "shop", 1, "Add a cart", "Customers need one.", "doing", "agent:builder", "[\"backend\"]", 2.0])?;
            connection.execute(card, rusqlite::params!["shop-4", "shop", 4, "Old idea", "", "someday", Option::<String>::None, "[]", 1.0])?;
            connection.execute(card, rusqlite::params!["gone-1", "gone", 1, "Of a project that is gone", "", "todo", Option::<String>::None, "[]", 1.0])
        })
        .await
        .unwrap();

    // The daemon starts again, as the new one.
    let studio = studio::Studio::open(
        rig.journal.clone(),
        journal::Db::memory().unwrap(),
        rig.dir.join("projects"),
        rig.dir.join("attachments"),
    )
    .await
    .unwrap();
    assert_eq!(
        studio.board.columns("shop").await.unwrap(),
        ["todo", "doing", "someday"],
        "a column a card is still in is kept"
    );
    let cards = studio.board.list("shop").await.unwrap();
    let told: Vec<(&str, &str, &str, f64)> = cards
        .iter()
        .map(|card| (card.id.as_str(), card.title.as_str(), card.status.as_str(), card.position))
        .collect();
    assert_eq!(told, [("shop-1", "Add a cart", "doing", 2.0), ("shop-4", "Old idea", "someday", 1.0)]);
    assert_eq!(
        (cards[0].assignee.as_deref(), cards[0].labels.as_slice(), cards[0].created, cards[0].updated),
        (Some("agent:builder"), ["backend".to_string()].as_slice(), 100, 200)
    );
    // New cards carry on from the numbers there were.
    assert_eq!(studio.board.add("shop", "Next", "", None, vec![], &ada()).await.unwrap().id, "shop-5");

    // Starting once more moves nothing twice.
    let again = studio::Studio::open(
        rig.journal.clone(),
        journal::Db::memory().unwrap(),
        rig.dir.join("projects"),
        rig.dir.join("attachments"),
    )
    .await
    .unwrap();
    assert_eq!(again.board.list("shop").await.unwrap().len(), 3);
}

#[test]
fn a_collection_is_laid_out_in_the_ways_there_are_and_another_is_refused_saying_which() {
    let fields =
        json!({ "title": { "type": "text" }, "status": { "type": "choice", "of": ["now", "later"] } });
    let ways =
        json!({ "cards": { "kind": "gallery" }, "lines": { "kind": "list" }, "rows": { "kind": "table" } });
    let spec = studio::collections::Spec::read(&fields, &ways, None).unwrap();
    let kinds: Vec<&str> = spec.views.iter().map(|view| view.kind.as_str()).collect();
    assert_eq!(kinds, ["gallery", "list", "table"]);

    let error = studio::collections::Spec::read(&fields, &json!({ "when": { "kind": "timeline" } }), None)
        .unwrap_err()
        .to_string();
    assert!(error.contains("`when` is a `timeline`"), "{error}");
    assert!(error.contains("there are: table, board, list, gallery, calendar, order"), "{error}");
}

#[tokio::test]
async fn a_view_somebody_saved_is_kept_when_the_blueprint_is_applied_again() {
    let rig = Rig::new().await;
    let project = "shop".to_string();
    rig.board(&project).await;
    let collections = &rig.studio.collections;
    let view = |name: &str, kind: &str, by: Option<&str>| studio::View {
        name: name.into(),
        kind: kind.into(),
        by: by.map(str::to_string),
        saved: false,
        hides: None,
        group: None,
        done: None,
        only: None,
        shown: Default::default(),
    };

    let kept = collections
        .save_view(&project, "board", view("When", "calendar", Some("due")), &ada())
        .await
        .unwrap();
    let names = |collection: &studio::Collection| -> Vec<String> {
        collection.views.iter().map(|view| format!("{}:{}:{}", view.name, view.kind, view.saved)).collect()
    };
    assert_eq!(names(&kept), ["board:board:false", "When:calendar:true"]);

    // A calendar goes by a date of its records, and says so of anything else.
    let error = collections
        .save_view(&project, "board", view("Wrong", "calendar", Some("title")), &ada())
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("a calendar by `title`, which is not a date"), "{error}");

    // The blueprint applied again: its own view as it declares it, and theirs after.
    rig.board(&project).await;
    assert_eq!(
        names(&collections.get(&project, "board").await.unwrap()),
        ["board:board:false", "When:calendar:true"]
    );

    // A view is only a way of looking: taking one away leaves the cards, and the last one stays.
    let card = rig.studio.board.add(&project, "Ship it", "", None, Vec::new(), &ada()).await.unwrap();
    let left = collections.remove_view(&project, "board", "When", &ada()).await.unwrap();
    assert_eq!(names(&left), ["board:board:false"]);
    assert_eq!(rig.studio.board.get(&card.id).await.unwrap().title, "Ship it");
    let error = collections.remove_view(&project, "board", "board", &ada()).await.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Conflict);
}

#[tokio::test]
async fn a_text_says_the_words_it_offers_and_is_held_to_none_of_them() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let project = "shop".to_string();
    let collections = &rig.studio.collections;
    let declared =
        |fields: serde_json::Value| studio::collections::Spec::read(&fields, &serde_json::Value::Null, None);

    // The words are kept as they were written, once each, and said to whoever reads the table.
    let fields = json!({
        "title": { "type": "text", "required": true },
        "does": { "type": "text", "suggests": ["plan", " work ", "plan", "", "team/audit"] },
        "notes": { "type": "text" },
    });
    let made = collections.ensure(&project, "steps", declared(fields).unwrap(), None, &ada()).await.unwrap();
    assert_eq!(made.field("does").unwrap().suggests, ["plan", "work", "team/audit"]);
    let read = serde_json::to_value(&made).unwrap();
    let said =
        |name: &str| read["fields"].as_array().unwrap().iter().find(|field| field["name"] == name).cloned();
    assert_eq!(said("does").unwrap()["suggests"], json!(["plan", "work", "team/audit"]));
    assert!(said("notes").unwrap().get("suggests").is_none(), "one that offers nothing says nothing: {read}");

    // They are offered and nothing more: a record holds any other word as it always did.
    let row = json!({ "title": "one", "does": "something-else" }).as_object().unwrap().clone();
    let row = collections.add(&made, row, 1.0, &ada()).await.unwrap();
    assert_eq!(row.data["does"], "something-else");

    // Declared again with other words, or none, the records are as they were: nothing was held to them.
    let fields = json!({
        "title": { "type": "text", "required": true },
        "does": { "type": "text", "suggests": ["review"] },
        "notes": { "type": "text" },
    });
    let again = collections.ensure(&project, "steps", declared(fields).unwrap(), None, &ada()).await.unwrap();
    assert_eq!(again.field("does").unwrap().suggests, ["review"]);
    let kept = collections.records(&again).await.unwrap();
    assert_eq!(kept[0].data["does"], "something-else");

    // What a choice can be is said with `of`; the words a field offers are a list.
    let error = declared(json!({ "kind": { "type": "choice", "of": ["a"], "suggests": ["a"] } }))
        .unwrap_err()
        .to_string();
    assert!(error.contains("`suggests`, which is for a field that takes any word"), "{error}");
    let error = declared(json!({ "does": { "type": "text", "suggests": "plan" } })).unwrap_err().to_string();
    assert!(error.contains("`suggests`, which is a list of the words it offers"), "{error}");
}

#[tokio::test]
async fn a_view_says_which_fields_it_leaves_out_and_a_field_that_it_is_left_out_to_begin_with() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let project = "shop".to_string();
    let collections = &rig.studio.collections;
    let fields = json!({
        "title": { "type": "text", "required": true },
        "due": { "type": "date" },
        "kept": { "type": "text", "quiet": true, "hidden": true },
    });
    let declared = |ways: serde_json::Value| studio::collections::Spec::read(&fields, &ways, None);

    // A field says it is left out of a view to begin with, and that is kept with it and said to whoever reads the table.
    let ways = json!({ "all": { "kind": "table" }, "short": { "kind": "table", "hides": ["due", "kept"] } });
    let made =
        collections.ensure(&project, "notes", declared(ways.clone()).unwrap(), None, &ada()).await.unwrap();
    assert!(made.field("kept").unwrap().hidden && !made.field("due").unwrap().hidden);
    let hides = |collection: &studio::Collection, name: &str| -> Option<Vec<String>> {
        collection.views.iter().find(|view| view.name == name).unwrap().hides.clone()
    };
    // A view of the blueprint's says which it leaves out, or says nothing.
    assert_eq!(hides(&made, "short"), Some(vec!["due".to_string(), "kept".to_string()]));
    assert_eq!(hides(&made, "all"), None);
    let read = serde_json::to_value(&made).unwrap();
    assert_eq!(read["fields"].as_array().unwrap().iter().filter(|field| field["hidden"] == true).count(), 1);
    assert!(read["views"][0].get("hides").is_none(), "one that says nothing says nothing: {read}");

    // What a blueprint's view leaves out is a field there is, or it is a slip, and said to be.
    let slip = declared(json!({ "short": { "kind": "table", "hides": ["dew"] } })).unwrap();
    let error = collections.ensure(&project, "notes", slip, None, &ada()).await.unwrap_err().to_string();
    assert!(
        error.contains("the view `short` hides `dew`, which is not a field") && error.contains("due"),
        "{error}"
    );
    let error = declared(json!({ "short": { "kind": "table", "hides": "due" } })).unwrap_err().to_string();
    assert!(error.contains("`hides`, which is a list of the fields it leaves out"), "{error}");

    // Somebody shows a column in a view: the view is theirs from then, with what it now leaves out, said once each.
    let theirs = studio::View {
        name: "short".into(),
        kind: "table".into(),
        by: None,
        saved: false,
        hides: Some(vec!["kept".into(), " kept ".into(), String::new()]),
        group: None,
        done: None,
        only: None,
        shown: Default::default(),
    };
    let kept = collections.save_view(&project, "notes", theirs, &ada()).await.unwrap();
    assert_eq!(hides(&kept, "short"), Some(vec!["kept".to_string()]));
    // One saved saying nothing is kept as leaving nothing out: it is not taken for one from before.
    let plain = studio::View {
        name: "Mine".into(),
        kind: "list".into(),
        by: None,
        saved: false,
        hides: None,
        group: None,
        done: None,
        only: None,
        shown: Default::default(),
    };
    let kept = collections.save_view(&project, "notes", plain, &ada()).await.unwrap();
    assert_eq!(hides(&kept, "Mine"), Some(Vec::new()));

    // The blueprint applied again, with the field somebody's view leaves out gone from the table:
    // their views are theirs still, with what they said, and the blueprint's is as it declares it.
    let fewer = json!({ "title": { "type": "text", "required": true }, "due": { "type": "date" } });
    let again = studio::collections::Spec::read(
        &fewer,
        &json!({ "all": { "kind": "table" }, "short": { "kind": "table" } }),
        None,
    );
    let after = collections.ensure(&project, "notes", again.unwrap(), None, &ada()).await.unwrap();
    assert_eq!(hides(&after, "short"), Some(vec!["kept".to_string()]));
    assert_eq!(hides(&after, "Mine"), Some(Vec::new()));
    assert_eq!(hides(&after, "all"), None);
}

#[tokio::test]
async fn a_card_is_wanted_by_a_day_and_only_by_something_that_is_one() {
    let rig = Rig::new().await;
    let project = "shop".to_string();
    rig.board(&project).await;
    let card = rig.studio.board.add(&project, "Ship it", "", None, Vec::new(), &ada()).await.unwrap();
    assert_eq!(card.due, None);

    let due = |day: Option<&str>| Changes { due: Some(day.map(str::to_string)), ..Changes::default() };
    let dated = rig.studio.board.update(&card.id, due(Some("2026-11-30")), &ada()).await.unwrap();
    assert_eq!(dated.due.as_deref(), Some("2026-11-30"));
    let error =
        rig.studio.board.update(&card.id, due(Some("next week")), &ada()).await.unwrap_err().to_string();
    assert!(error.contains("`due` must be a day written year-month-day"), "{error}");
    assert_eq!(rig.studio.board.update(&card.id, due(None), &ada()).await.unwrap().due, None);
}

#[test]
fn history_is_declared_as_groups_of_events_or_as_each_with_its_own_words() {
    use studio::history::{Rule, rule_for, rules};
    let rule = |topic: &str, says: Option<&str>| Rule { topic: topic.into(), says: says.map(str::to_string) };

    // Groups by themselves, in the usual words.
    assert_eq!(
        rules(&json!(["board.item.*", "lane.*"])).unwrap(),
        [rule("board.item.*", None), rule("lane.*", None)]
    );
    assert_eq!(rules(&json!("board.item.*")).unwrap(), [rule("board.item.*", None)]);
    // Each with its own words; and the two written together.
    let written = json!({ "board.item.moved": "{actor} moved it from {from} to {to}", "board.item.*": "" });
    let both = rules(&written).unwrap();
    assert!(both.contains(&rule("board.item.moved", Some("{actor} moved it from {from} to {to}"))));
    assert!(both.contains(&rule("board.item.*", None)));
    assert_eq!(
        rules(&json!(["lane.*", { "board.item.removed": "{actor} took it away" }])).unwrap(),
        [rule("lane.*", None), rule("board.item.removed", Some("{actor} took it away"))]
    );

    // The more exact of two that fit an event speaks for it, whichever was written first.
    assert_eq!(rule_for(&both, "board.item.moved").unwrap().topic, "board.item.moved");
    assert_eq!(rule_for(&both, "board.item.archived").unwrap().topic, "board.item.*");
    assert!(rule_for(&both, "channel.message").is_none());

    let error = rules(&json!({ "board.item.moved": 3 })).unwrap_err().to_string();
    assert!(error.contains("must be a sentence"), "{error}");
}

#[tokio::test]
async fn a_cards_journey_across_its_board_is_kept_and_outlasts_the_card() {
    let rig = Rig::new().await;
    let project = "shop".to_string();
    rig.board(&project).await;
    let board = &rig.studio.board;
    // A line is said to have been written when it has been: that is what is waited for.
    let mut written = rig.journal.follow(0, studio::history::TOPIC);

    let card = board.add(&project, "Ship it", "", None, Vec::new(), &ada()).await.unwrap();
    written.next().await.unwrap();
    board.move_to(&card.id, "doing", None, &agent("builder", &["board"])).await.unwrap();
    written.next().await.unwrap();
    let changes = Changes { title: Some("Ship it today".into()), ..Changes::default() };
    board.update(&card.id, changes, &ada()).await.unwrap();
    written.next().await.unwrap();

    let said = |lines: &[studio::history::Line]| -> Vec<String> {
        lines.iter().map(|line| line.said.clone()).collect()
    };
    let history = rig.studio.collections.history(&card.id).await.unwrap();
    assert_eq!(
        said(&history),
        ["ada added it to draft", "builder moved it from draft to doing", "ada changed its title"]
    );
    assert!(history.windows(2).all(|pair| pair[0].seq < pair[1].seq), "in the order it happened");
    assert_eq!(history[1].actor.as_deref(), Some("agent:builder"));

    // Putting a card somewhere else in its column is not something that happened to it.
    board.move_to(&card.id, "doing", None, &ada()).await.unwrap();
    // Taking the card away is the last line of its history, and the history stays.
    board.remove(&card.id, &ada()).await.unwrap();
    written.next().await.unwrap();
    let history = rig.studio.collections.history(&card.id).await.unwrap();
    assert_eq!(said(&history).last().map(String::as_str), Some("ada took it away"));
    assert_eq!(history.len(), 4);
    assert!(board.get(&card.id).await.is_err(), "the card is gone");
}

#[tokio::test]
async fn what_is_said_as_a_card_is_moved_goes_with_the_move_and_into_its_history() {
    let rig = Rig::new().await;
    let project = "shop".to_string();
    rig.board(&project).await;
    let board = &rig.studio.board;
    let mut written = rig.journal.follow(0, studio::history::TOPIC);
    let mut moves = rig.journal.follow(0, "board.item.moved");

    let card = board.add(&project, "Ship it", "", None, Vec::new(), &ada()).await.unwrap();
    written.next().await.unwrap();
    board.move_to(&card.id, "review", None, &ada()).await.unwrap();
    written.next().await.unwrap();
    assert!(moves.next().await.unwrap().data.get("note").is_none(), "nothing was said");

    // Sent back from review, saying why: the event carries it, and the card does not.
    let said = Some("  the total is wrong  ");
    let moved = board.move_through(&card.id, "doing", None, said, &ada(), None).await.unwrap();
    written.next().await.unwrap();
    let event = moves.next().await.unwrap();
    assert_eq!(event.data["note"], "the total is wrong");
    assert_eq!((event.data["from"].as_str(), event.data["to"].as_str()), (Some("review"), Some("doing")));
    assert_eq!(moved.body, "", "a note is said of the move, and is not written on the card");

    // Saying nothing is saying nothing, however it is spelled.
    board.move_through(&card.id, "review", None, Some("  "), &ada(), None).await.unwrap();
    written.next().await.unwrap();
    assert!(moves.next().await.unwrap().data.get("note").is_none());

    let history = rig.studio.collections.history(&card.id).await.unwrap();
    let lines: Vec<&str> = history.iter().map(|line| line.said.as_str()).collect();
    assert_eq!(
        lines,
        [
            "ada added it to draft",
            "ada moved it from draft to review",
            "ada moved it from review to doing, saying: the total is wrong",
            "ada moved it from doing to review",
        ]
    );
}

#[test]
fn a_part_of_a_sentence_in_square_brackets_is_said_only_when_the_event_has_what_it_names() {
    use studio::history::{Rule, said};
    let rule = |sentence: &str| Rule { topic: "review.asked".into(), says: Some(sentence.into()) };
    let event = |data: serde_json::Value| kernel::Event {
        seq: 1,
        ts: 0,
        topic: "review.asked".into(),
        source: kernel::Source::default(),
        data,
    };
    let sentence = rule("{actor} asked {whom}[ about {what}][, by {day}] and waited");
    assert_eq!(
        said(&sentence, &event(json!({ "whom": "Grace", "what": "the price", "day": "Friday" })), "Ada"),
        "Ada asked Grace about the price, by Friday and waited"
    );
    assert_eq!(
        said(&sentence, &event(json!({ "whom": "Grace", "what": "", "day": null })), "Ada"),
        "Ada asked Grace and waited",
        "empty and nothing are not there"
    );
    // Outside brackets a name the event lacks is said as before: as nothing at all.
    assert_eq!(said(&rule("{actor} asked {whom}"), &event(json!({})), "Ada"), "Ada asked ");
    // A bracket that is never closed is only a bracket.
    assert_eq!(
        said(&rule("{actor} asked [{whom}"), &event(json!({ "whom": "Grace" })), "Ada"),
        "Ada asked [Grace"
    );
}

#[tokio::test]
async fn a_board_that_says_what_its_history_is_only_now_is_given_what_already_happened() {
    let rig = Rig::new().await;
    let project = "shop".to_string();
    rig.board(&project).await;
    let collections = &rig.studio.collections;
    let mut written = rig.journal.follow(0, studio::history::TOPIC);
    let card = rig.studio.board.add(&project, "Ship it", "", None, Vec::new(), &ada()).await.unwrap();
    written.next().await.unwrap();

    // Declared again with one more event in its history: a group, in the usual words.
    let was = collections.get(&project, "board").await.unwrap();
    let spec = studio::collections::Spec::read(
        &json!({
            "title": { "type": "text", "required": true },
            "status": { "type": "choice", "of": studio::board::DEFAULT_COLUMNS, "required": true },
        }),
        &json!({ "board": { "kind": "board", "by": "status" } }),
        None,
    )
    .unwrap()
    .with_history(&json!(["board.item.*", "review.*"]))
    .unwrap();
    assert_ne!(was.history, spec.history);
    rig.journal
        .emit(
            "review.asked",
            kernel::Source::actor("agent:reviewer"),
            json!({ "card": card.id, "of": "builder" }),
        )
        .await
        .unwrap();
    collections.ensure(&project, "board", spec, None, &ada()).await.unwrap();
    written.next().await.unwrap();

    let history = collections.history(&card.id).await.unwrap();
    let said: Vec<&str> = history.iter().map(|line| line.said.as_str()).collect();
    // The first line keeps the words it was written in; the one from before the declaration is in the usual words.
    assert_eq!(said, ["ada added it to draft", "reviewer: asked (of builder)"]);
}
