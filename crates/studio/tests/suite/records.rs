//! Records of a collection that is not a board: a project's sprints,
//! made, changed and taken away by name, each of which is an event.

use kernel::{ErrorKind, Principal, Source};
use serde_json::{Map, Value, json};
use studio::board::Through;
use studio::collections::Spec;

use crate::{Rig, ada, agent};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// What `planned` declares the sprints with, kept where `store` says.
fn sprints(store: &str) -> Spec {
    let fields = json!({
        "title": { "type": "text", "required": true },
        "size": { "type": "number" },
        "after": { "type": "record", "of": "sprints" },
    });
    Spec::read(&fields, &json!(null), Some(store)).unwrap()
}

/// A project `shop` whose sprints have a title, a size and a sprint they
/// come after, kept where `store` says.
async fn planned(store: &str) -> Rig {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.collections.ensure("shop", "sprints", sprints(store), None, &ada()).await.unwrap();
    rig
}

#[tokio::test]
async fn a_record_is_added_changed_and_taken_away_by_name_and_each_is_said_as_it_happens() {
    for store in ["local", "files"] {
        let rig = planned(store).await;
        let records = &rig.studio.records;
        let november =
            records.add("shop", "Sprints", data(json!({ "title": "November" })), &ada(), None).await.unwrap();
        assert_eq!((november.id.as_str(), november.collection.as_str()), ("shop-sprints-1", "sprints"));
        assert_eq!(november.created_by, "user:ada", "{store}");
        let december = json!({ "title": "December", "size": 8, "after": november.id });
        let december = records.add("shop", "sprints", data(december), &ada(), None).await.unwrap();
        assert_eq!(december.id, "shop-sprints-2");
        assert!(december.position > november.position, "{store}: it comes after the ones there were");
        assert_eq!(
            records.list("shop", "sprints", &ada()).await.unwrap(),
            [november.clone(), december.clone()]
        );
        assert_eq!(
            records.get("shop-sprints-2", &ada()).await.unwrap().data["after"],
            json!("shop-sprints-1")
        );

        // Only the fields named are changed; saying what is so is not a change.
        let grown = records.update(&december.id, data(json!({ "size": 13 })), &ada(), None).await.unwrap();
        assert_eq!(grown.data, data(json!({ "title": "December", "size": 13, "after": "shop-sprints-1" })));
        records
            .update(&december.id, data(json!({ "size": 13, "title": "December" })), &ada(), None)
            .await
            .unwrap();
        let cleared = json!({ "size": null, "after": "", "title": "Winter" });
        let cleared = records.update(&december.id, data(cleared), &ada(), None).await.unwrap();
        assert_eq!(cleared.data["title"], json!("Winter"));
        assert!(cleared.data["size"].is_null() && cleared.data["after"].is_null(), "{store}: {cleared:?}");

        records.remove(&november.id, &ada(), None).await.unwrap();
        assert_eq!(records.list("shop", "sprints", &ada()).await.unwrap().len(), 1, "{store}");
        assert_eq!(records.get(&november.id, &ada()).await.unwrap_err().kind, ErrorKind::NotFound);

        let said: Vec<(String, Value)> =
            rig.events("record.*").await.into_iter().map(|event| (event.topic, event.data)).collect();
        let about = |id: &str| json!({ "id": id, "project": "shop", "collection": "sprints" });
        let changed = |fields: Value| {
            let mut data = about("shop-sprints-2");
            data["changed"] = fields;
            data
        };
        assert_eq!(
            said,
            [
                ("record.created".to_string(), about("shop-sprints-1")),
                ("record.created".to_string(), about("shop-sprints-2")),
                ("record.updated".to_string(), changed(json!(["size"]))),
                ("record.updated".to_string(), changed(json!(["after", "size", "title"]))),
                ("record.removed".to_string(), about("shop-sprints-1")),
            ],
            "{store}"
        );
    }
}

#[tokio::test]
async fn what_is_written_into_a_record_is_checked_as_its_collection_declares_it() {
    for store in ["local", "files"] {
        let rig = planned(store).await;
        let (records, ada) = (&rig.studio.records, ada());
        let add = |fields: Value| records.add("shop", "sprints", data(fields), &ada, None);
        assert_eq!(add(json!({ "size": 3 })).await.unwrap_err().message, "`title` is needed", "{store}");
        assert_eq!(
            add(json!({ "title": "November", "goal": "ship" })).await.unwrap_err().message,
            "`goal` is not a field of `sprints`; its fields are: after, size, title"
        );
        assert_eq!(
            add(json!({ "title": "November", "size": "big" })).await.unwrap_err().message,
            "`size` must be a number"
        );
        let nowhere = add(json!({ "title": "November", "after": "shop-sprints-7" })).await.unwrap_err();
        assert!(nowhere.message.starts_with("`shop-sprints-7` is not a record of `sprints`"), "{nowhere}");
        assert_eq!(records.list("shop", "sprints", &ada).await.unwrap(), [], "{store}: nothing was added");

        let november = add(json!({ "title": "November" })).await.unwrap();
        let change = |fields: Value| records.update(&november.id, data(fields), &ada, None);
        // A field that is not one is told so even when nothing is given for it.
        for stray in [json!({ "goal": "ship" }), json!({ "goal": null })] {
            let refused = change(stray).await.unwrap_err();
            assert!(refused.message.starts_with("`goal` is not a field of `sprints`"), "{store}: {refused}");
        }
        assert_eq!(change(json!({ "title": "" })).await.unwrap_err().message, "`title` is needed");
        assert_eq!(change(json!({ "size": true })).await.unwrap_err().message, "`size` must be a number");
        assert!(rig.events("record.updated").await.is_empty(), "{store}: nothing was changed");

        let none = records.add("shop", "releases", data(json!({ "title": "1.0" })), &ada, None).await;
        assert_eq!(none.unwrap_err().message, "the project `shop` has no collection called `releases`");
        let missing = records.update("shop-sprints-9", data(json!({ "size": 1 })), &ada, None).await;
        assert_eq!(missing.unwrap_err().message, "there is no record `shop-sprints-9`");
    }
}

#[tokio::test]
async fn a_boards_cards_are_read_as_records_and_written_only_through_the_board() {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let card = rig.studio.board.add("shop", "Basket", "", None, Vec::new(), &ada()).await.unwrap();
    let records = &rig.studio.records;
    let read = records.list("shop", "board", &ada()).await.unwrap();
    assert_eq!((read.len(), &read[0].data["title"]), (1, &json!("Basket")));
    assert_eq!(records.get(&card.id, &ada()).await.unwrap().data["status"], json!("draft"));

    let through = format!(
        "Use `board.add`, `board.update` and `board.move`, which `{} board` and the board's page do",
        brand::CLI
    );
    let added =
        records.add("shop", "board", data(json!({ "title": "Till" })), &ada(), None).await.unwrap_err();
    assert_eq!(added.kind, ErrorKind::Invalid);
    assert_eq!(
        added.message,
        format!(
            "a board's cards are not added as records: moving one is an event of its own, which the rest of the factory hangs on. {through}"
        )
    );
    let moved = records.update(&card.id, data(json!({ "status": "done" })), &ada(), None).await.unwrap_err();
    assert!(
        moved.message.starts_with("a board's cards are not changed as records")
            && moved.message.ends_with(&through)
    );
    let gone = records.remove(&card.id, &ada(), None).await.unwrap_err();
    assert!(gone.message.starts_with("a board's cards are not taken away as records"), "{gone}");
    assert_eq!(rig.studio.board.get(&card.id).await.unwrap(), card, "the card is as it was");
    assert!(rig.events("record.*").await.is_empty());
}

#[tokio::test]
async fn records_are_read_by_whoever_may_read_them_and_written_by_whoever_may_write_them() {
    let rig = planned("local").await;
    let records = &rig.studio.records;
    let november =
        records.add("shop", "sprints", data(json!({ "title": "November" })), &ada(), None).await.unwrap();

    let reader = agent("reader", &["records:read", "board"]);
    assert_eq!(records.list("shop", "sprints", &reader).await.unwrap().len(), 1);
    assert_eq!(records.get(&november.id, &reader).await.unwrap(), november);
    let fields = || data(json!({ "title": "December" }));
    for refused in [
        records.add("shop", "sprints", fields(), &reader, None).await.err(),
        records.update(&november.id, fields(), &reader, None).await.err(),
        records.remove(&november.id, &reader, None).await.err(),
    ] {
        let refused = refused.unwrap();
        assert_eq!(refused.kind, ErrorKind::Denied);
        assert_eq!(refused.message, "reader is not allowed to do this (needs `records:write`)");
    }
    // Leave to work a board is not leave to read what else a project keeps.
    let boards = agent("boards", &["board", "projects"]);
    let denied = records.list("shop", "sprints", &boards).await.unwrap_err();
    assert_eq!(denied.message, "boards is not allowed to do this (needs `records:read`)");
    assert!(records.get(&november.id, &boards).await.is_err());

    // A person and an agent are given both unless they are given something else.
    for scopes in [kernel::principal::MEMBER_SCOPES, kernel::principal::AGENT_SCOPES] {
        let usual = agent("usual", scopes);
        assert!(usual.can("records:read") && usual.can("records:write"), "{scopes:?}");
    }
    assert!(kernel::principal::SCOPES.iter().any(|(scope, _)| *scope == "records:write"));
}

#[tokio::test]
async fn what_a_blueprints_step_does_to_a_record_is_said_of_the_blueprint() {
    let rig = planned("local").await;
    let records = &rig.studio.records;
    let step = Through {
        blueprint: "plan".into(),
        source: Source::instance("inst_1"),
        wrote: None,
        decided: None,
        had: None,
        doing: None,
    };
    let november = records
        .add("shop", "sprints", data(json!({ "title": "November" })), &ada(), Some(&step))
        .await
        .unwrap();
    records.update(&november.id, data(json!({ "size": 5 })), &ada(), Some(&step)).await.unwrap();
    records.remove(&november.id, &ada(), Some(&step)).await.unwrap();
    let said = rig.events("record.*").await;
    assert_eq!(said.len(), 3);
    for event in said {
        assert_eq!(event.data["through"], json!("plan"), "{}", event.topic);
        assert_eq!(event.source.instance.as_deref(), Some("inst_1"), "{}", event.topic);
        assert_eq!(event.data["id"], json!("shop-sprints-1"));
    }
}

/// A project `shop` with a board, and sprints that keep what is said and
/// done to them as their history.
async fn spoken() -> Rig {
    let rig = Rig::new().await;
    rig.board("Shop").await;
    let fields = json!({ "title": { "type": "text", "required": true }, "size": { "type": "number" } });
    let spec = Spec::read(&fields, &json!(null), None)
        .unwrap()
        .with_history(&json!({ "record.commented": "{actor} said: {body}", "record.*": "" }))
        .unwrap();
    rig.studio.collections.ensure("shop", "sprints", spec, None, &ada()).await.unwrap();
    rig
}

/// The lines of a record's history once there are `lines` of them: waited
/// for by the news of each being written.
async fn history_of(rig: &Rig, record: &str, lines: usize) -> Vec<String> {
    let mut written = rig.journal.follow(0, studio::history::TOPIC);
    loop {
        let kept = rig.studio.collections.history(record).await.unwrap();
        if kept.len() >= lines {
            return kept.into_iter().map(|line| line.said).collect();
        }
        written.next().await.unwrap();
    }
}

#[tokio::test]
async fn what_is_said_on_a_record_is_kept_in_its_history_whatever_it_is_a_record_of_and_changes_nothing_of_it()
 {
    let rig = spoken().await;
    let records = &rig.studio.records;
    let card =
        rig.studio.board.add("shop", "Ship it", "", None, vec!["urgent".into()], &ada()).await.unwrap();
    let sprint =
        records.add("shop", "sprints", data(json!({ "title": "November" })), &ada(), None).await.unwrap();
    // A step of a blueprint, acting with Ada's authority.
    let round = Through {
        blueprint: "std/lane-round".into(),
        source: Source { instance: Some("in_1".into()), ..Source::actor("user:ada") },
        wrote: None,
        decided: None,
        had: None,
        doing: None,
    };

    let on_card =
        records.comment(&card.id, "  Customers keep asking for this.  ", false, &ada(), None).await.unwrap();
    assert_eq!(on_card.data, records.get(&card.id, &ada()).await.unwrap().data, "the card is as it was");
    assert_eq!(on_card.data["labels"], json!(["urgent"]));
    records.comment(&card.id, "Waiting on an answer.", false, &ada(), Some(&round)).await.unwrap();
    records
        .comment(&sprint.id, "Two weeks is too short.", false, &agent("builder", &["records:write"]), None)
        .await
        .unwrap();

    assert_eq!(
        history_of(&rig, &card.id, 3).await,
        [
            "ada added it to draft",
            "ada said: Customers keep asking for this.",
            // What a blueprint's step said is the factory's bot's saying.
            "conductor said: Waiting on an answer.",
        ]
    );
    assert_eq!(
        history_of(&rig, &sprint.id, 2).await,
        ["ada: created (collection sprints)", "builder said: Two weeks is too short."]
    );

    // The event says which record, what was said, and who said it, in the
    // form anything that hears it can tell one speaker from another by.
    let said: Vec<Value> = rig.events("record.commented").await.into_iter().map(|event| event.data).collect();
    assert_eq!(
        said,
        [
            json!({ "id": card.id, "project": "shop", "collection": "board", "by": "user:ada",
                    "body": "Customers keep asking for this." }),
            json!({ "id": card.id, "project": "shop", "collection": "board", "by": "user:ada",
                    "body": "Waiting on an answer.", "through": "std/lane-round" }),
            json!({ "id": sprint.id, "project": "shop", "collection": "sprints", "by": "agent:builder",
                    "body": "Two weeks is too short." }),
        ]
    );
    assert!(
        rig.events("record.updated").await.is_empty() && rig.events("board.item.updated").await.is_empty()
    );

    // Nothing is not something to say, and neither is a book.
    let nothing = records.comment(&card.id, "   ", false, &ada(), None).await.unwrap_err().message;
    assert_eq!(nothing, "there is nothing here to say on it: it needs some words");
    let book =
        records.comment(&card.id, &"word ".repeat(4000), false, &ada(), None).await.unwrap_err().message;
    assert!(
        book.starts_with("that is too much to say on a record at once (19999 bytes; the most is 16384)"),
        "{book}"
    );
    assert_eq!(
        records.comment("shop-9", "Hello?", false, &ada(), None).await.unwrap_err().message,
        "there is no record `shop-9`"
    );
}

#[tokio::test]
async fn what_is_said_is_private_unless_it_is_marked_public_and_said_with_a_key_it_is_the_key_that_said_it() {
    let rig = spoken().await;
    let records = &rig.studio.records;
    let sprint =
        records.add("shop", "sprints", data(json!({ "title": "November" })), &ada(), None).await.unwrap();
    // A table whose own words for something said name the mark themselves.
    let fields = json!({ "title": { "type": "text", "required": true } });
    let spec = Spec::read(&fields, &json!(null), None)
        .unwrap()
        .with_history(&json!({ "record.commented": "{actor}[ told everybody ({public})]: {body}" }))
        .unwrap();
    rig.studio.collections.ensure("shop", "plans", spec, None, &ada()).await.unwrap();
    let plan = records.add("shop", "plans", data(json!({ "title": "Spring" })), &ada(), None).await.unwrap();
    // A key of Ada's, and a step of a blueprint applied with it.
    let key = Principal { key: Some("key_1".into()), ..ada() };
    let step = Through {
        blueprint: "season".into(),
        source: Source { instance: Some("in_1".into()), ..Source::actor("user:ada") },
        wrote: None,
        decided: None,
        had: None,
        doing: None,
    };

    records.comment(&sprint.id, "Between us.", false, &ada(), None).await.unwrap();
    records.comment(&sprint.id, "For everybody.", true, &ada(), None).await.unwrap();
    records.comment(&sprint.id, "From a script.", true, &key, None).await.unwrap();
    records.comment(&sprint.id, "From a step.", true, &key, Some(&step)).await.unwrap();
    records.comment(&plan.id, "Quietly.", false, &ada(), None).await.unwrap();
    records.comment(&plan.id, "Aloud.", true, &ada(), None).await.unwrap();

    // The event says the mark where there is one and nothing where there
    // is none; and who said it is the key, where it was said with one and
    // by no step of a blueprint.
    let query = journal::EventQuery { topics: "record.commented".into(), limit: 100, ..Default::default() };
    let mut said = rig.journal.read(query).await.unwrap();
    said.sort_by_key(|event| event.seq);
    let said: Vec<_> = said
        .iter()
        .map(|event| {
            let (data, actor) = (&event.data, event.source.actor.clone().unwrap_or_default());
            (
                data["body"].as_str().unwrap().to_string(),
                data.get("public").cloned(),
                data["by"].clone(),
                actor,
            )
        })
        .collect();
    let line = |body: &str, public: bool, by: &str, actor: &str| {
        (body.to_string(), public.then_some(Value::Bool(true)), json!(by), actor.to_string())
    };
    assert_eq!(
        said,
        [
            line("Between us.", false, "user:ada", "user:ada"),
            line("For everybody.", true, "user:ada", "user:ada"),
            line("From a script.", true, "key:key_1", "key:key_1"),
            line("From a step.", true, "user:ada", "user:ada"),
            line("Quietly.", false, "user:ada", "user:ada"),
            line("Aloud.", true, "user:ada", "user:ada"),
        ]
    );

    // The line of its history says the mark after the table's own words
    // for something said, and leaves them as they are where they say it.
    let lines = history_of(&rig, &sprint.id, 5).await;
    assert_eq!(lines[1..3], ["ada said: Between us.", "ada said: For everybody. (public)"], "{lines:?}");
    assert!(lines[3].ends_with("said: From a script. (public)"), "{lines:?}");
    assert_eq!(lines[4], "conductor said: From a step. (public)", "a step's saying is the factory's bot's");
    assert_eq!(history_of(&rig, &plan.id, 2).await, ["ada: Quietly.", "ada told everybody (true): Aloud."]);
    // And each line keeps what its event said, the mark with it.
    let kept = rig.studio.collections.history(&sprint.id).await.unwrap();
    let marks: Vec<Option<bool>> = kept[1..].iter().map(|line| line.data["public"].as_bool()).collect();
    assert_eq!(marks, [None, Some(true), Some(true), Some(true)]);
}

#[tokio::test]
async fn saying_something_on_a_record_needs_what_changing_that_kind_of_record_needs() {
    let rig = spoken().await;
    let records = &rig.studio.records;
    let card = rig.studio.board.add("shop", "Ship it", "", None, Vec::new(), &ada()).await.unwrap();
    let sprint =
        records.add("shop", "sprints", data(json!({ "title": "November" })), &ada(), None).await.unwrap();

    // An agent that may write the board, as every agent made before there
    // were other records may, speaks on its card and on nothing else.
    let builder = agent("builder", &["board:read", "board:write"]);
    records.comment(&card.id, "Done by Friday.", false, &builder, None).await.unwrap();
    assert_eq!(
        records.comment(&sprint.id, "Done by Friday.", false, &builder, None).await.unwrap_err().kind,
        ErrorKind::Denied
    );
    // One that may write records and not the board, the other way about.
    let planner = agent("planner", &["records:read", "records:write"]);
    records.comment(&sprint.id, "Too short.", false, &planner, None).await.unwrap();
    assert_eq!(
        records.comment(&card.id, "Too short.", false, &planner, None).await.unwrap_err().kind,
        ErrorKind::Denied
    );
    // And one that may only read says nothing anywhere.
    let reader = agent("reader", &["board:read", "records:read"]);
    for id in [&card.id, &sprint.id] {
        assert_eq!(
            records.comment(id, "May I?", false, &reader, None).await.unwrap_err().kind,
            ErrorKind::Denied
        );
    }
    assert_eq!(rig.events("record.commented").await.len(), 2);
}

#[tokio::test]
async fn a_collection_that_would_keep_nothing_of_what_is_said_refuses_it_and_says_what_to_do() {
    // Sprints whose blueprint counts nothing as their history.
    let rig = planned("local").await;
    let records = &rig.studio.records;
    let sprint =
        records.add("shop", "sprints", data(json!({ "title": "November" })), &ada(), None).await.unwrap();
    let refused = records.comment(&sprint.id, "Too short.", false, &ada(), None).await.unwrap_err().message;
    assert_eq!(
        refused,
        "what is said on a record of `sprints` would be kept nowhere: `sprints` in `shop` keeps no history \
         of it. The blueprint that declares `sprints` must count `record.commented` in its `history` \
         (`record.commented: \"{actor} said: {body}\"`), and be applied again"
    );

    // A board made before cards could be spoken on counts the board's own
    // events and nothing of this one: it says to apply its blueprint again.
    let old = json!(["board.item.*", "card.*"]);
    let columns = vec!["draft".to_string(), "done".to_string()];
    let spec = studio::board::spec(&columns, studio::Store::Local).with_history(&old).unwrap();
    rig.studio.collections.ensure("shop", "board", spec, None, &ada()).await.unwrap();
    let card = rig.studio.board.add("shop", "Ship it", "", None, Vec::new(), &ada()).await.unwrap();
    let refused = records.comment(&card.id, "When?", false, &ada(), None).await.unwrap_err().message;
    assert_eq!(
        refused,
        format!(
            "what is said on a record of `board` would be kept nowhere: `board` in `shop` keeps no history \
             of it. This board was made before cards could be spoken on: apply its blueprint again \
             (`{} apply std/board --project shop`, or the project's own)",
            brand::CLI
        )
    );
    assert!(rig.events("record.commented").await.is_empty(), "nothing was said to nobody");
}

#[tokio::test]
async fn a_change_to_a_field_its_collection_calls_quiet_is_not_a_line_of_the_records_history() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let fields = json!({
        "title": { "type": "text", "required": true },
        "size": { "type": "number" },
        "question": { "type": "text", "quiet": true },
    });
    let spec = Spec::read(&fields, &json!(null), None).unwrap().with_history(&json!(["record.*"])).unwrap();
    let made = rig.studio.collections.ensure("shop", "sprints", spec, None, &ada()).await.unwrap();
    assert!(made.field("question").unwrap().quiet && !made.field("size").unwrap().quiet);
    let records = &rig.studio.records;
    let sprint =
        records.add("shop", "sprints", data(json!({ "title": "November" })), &ada(), None).await.unwrap();

    records.update(&sprint.id, data(json!({ "question": "How long?" })), &ada(), None).await.unwrap();
    records.update(&sprint.id, data(json!({ "size": 5 })), &ada(), None).await.unwrap();
    // Changed with something that is news, it is said with it.
    records.update(&sprint.id, data(json!({ "question": "", "size": 8 })), &ada(), None).await.unwrap();

    assert_eq!(
        history_of(&rig, &sprint.id, 3).await,
        [
            "ada: created (collection sprints)",
            "ada: updated (changed size, collection sprints)",
            "ada: updated (changed question, size, collection sprints)",
        ]
    );
    assert_eq!(rig.events("record.updated").await.len(), 3, "each change is still an event");
}

#[tokio::test]
async fn a_record_is_found_by_its_id_whichever_table_owns_it_however_many_words_the_table_and_the_project_are_called()
 {
    for store in ["local", "files"] {
        let rig = Rig::new().await;
        let (collections, records) = (&rig.studio.collections, &rig.studio.records);
        // A project called in one word and one called in two, each with
        // a table called in two: nothing in an id says where its
        // project's name ends and its table's begins.
        for (project, table) in [("Shop", "Open questions"), ("Shop Open", "notes"), ("Shop Open", "to-do")] {
            if rig.studio.projects.dir_of(&blueprint::expr::slug(project)).is_none() {
                rig.studio.projects.create(project, None, &ada()).await.unwrap();
            }
            let fields =
                json!({ "title": { "type": "text", "required": true }, "size": { "type": "number" } });
            let spec = Spec::read(&fields, &json!(null), Some(store)).unwrap();
            let project = blueprint::expr::slug(project);
            collections.ensure(&project, table, spec, None, &ada()).await.unwrap();
        }
        for (project, table, id) in [
            ("shop", "open-questions", "shop-open-questions-1"),
            ("shop-open", "notes", "shop-open-notes-1"),
            ("shop-open", "to-do", "shop-open-to-do-1"),
        ] {
            let added = records.add(project, table, data(json!({ "title": "Why?" })), &ada(), None).await;
            assert_eq!(added.unwrap().id, id, "{store}");
            // It is found with the table it is in, read, changed and
            // taken away, each by its id and nothing else.
            let (of, found) = collections.record(id).await.unwrap_or_else(|err| panic!("{store}: {err:?}"));
            assert_eq!((of.project.as_str(), of.name.as_str()), (project, table), "{store}");
            assert_eq!(found.data["title"], "Why?", "{store}");
            assert_eq!(records.get(id, &ada()).await.unwrap().id, id, "{store}");
            let changed = records.update(id, data(json!({ "size": 3 })), &ada(), None).await.unwrap();
            assert_eq!(
                (&changed.data["title"], &changed.data["size"]),
                (&json!("Why?"), &json!(3)),
                "{store}"
            );
            assert_eq!(collections.record(id).await.unwrap().1.data["size"], 3, "{store}");
            records.remove(id, &ada(), None).await.unwrap();
            let gone = collections.record(id).await.unwrap_err();
            assert_eq!(
                (gone.kind, gone.message),
                (ErrorKind::NotFound, format!("there is no record `{id}`"))
            );
        }
        // What no table owns is not there, in the words it always was: a
        // number of a table that is, a table that is not, and no number.
        for id in ["shop-open-questions-7", "shop-open-answers-1", "shop-open-questions", "shop"] {
            let gone = collections.record(id).await.unwrap_err();
            assert_eq!(
                (gone.kind, gone.message),
                (ErrorKind::NotFound, format!("there is no record `{id}`"))
            );
        }
    }
}

#[tokio::test]
async fn a_change_says_what_each_changed_choice_and_each_changed_flag_now_says_and_nothing_of_any_other_cell()
{
    for store in ["local", "files"] {
        let rig = Rig::new().await;
        rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
        let fields = json!({
            "title": { "type": "text", "required": true },
            "stage": { "type": "choice", "of": ["draft", "final"] },
            "public": { "type": "flag" },
            "size": { "type": "number" },
        });
        let spec = Spec::read(&fields, &json!(null), Some(store)).unwrap();
        rig.studio.collections.ensure("shop", "notes", spec, None, &ada()).await.unwrap();
        let records = &rig.studio.records;
        let note =
            records.add("shop", "notes", data(json!({ "title": "Plan" })), &ada(), None).await.unwrap();

        // A flag ticked: the event says what it now is, and names no choice.
        records.update(&note.id, data(json!({ "public": true })), &ada(), None).await.unwrap();
        // A flag, a choice and a number in one change: each of the first two, and not the number.
        let three = json!({ "public": false, "stage": "final", "size": 3 });
        records.update(&note.id, data(three), &ada(), None).await.unwrap();
        // Neither a choice nor a flag: neither word is said.
        records
            .update(&note.id, data(json!({ "size": 4, "title": "The plan" })), &ada(), None)
            .await
            .unwrap();
        // Emptied: each is said to say nothing now, which is not the same as not having changed.
        records.update(&note.id, data(json!({ "public": null, "stage": null })), &ada(), None).await.unwrap();
        // Said again as it is: nothing changed, and nothing is said.
        records.update(&note.id, data(json!({ "size": 4 })), &ada(), None).await.unwrap();

        let said: Vec<Value> =
            rig.events("record.updated").await.into_iter().map(|event| event.data).collect();
        assert_eq!(said.len(), 4, "{store}: {said:?}");
        assert_eq!(said[0]["flags"], json!({ "public": true }), "{store}");
        assert_eq!(said[0]["changed"], json!(["public"]), "{store}");
        assert!(said[0].get("chose").is_none(), "{store}: {}", said[0]);
        assert_eq!(said[1]["flags"], json!({ "public": false }), "{store}");
        assert_eq!(said[1]["chose"], json!({ "stage": "final" }), "{store}");
        assert_eq!(said[1]["changed"].as_array().unwrap().len(), 3, "{store}: {}", said[1]);
        assert!(said[2].get("flags").is_none() && said[2].get("chose").is_none(), "{store}: {}", said[2]);
        assert_eq!(said[3]["flags"], json!({ "public": null }), "{store}");
        assert_eq!(said[3]["chose"], json!({ "stage": null }), "{store}");
    }
}

#[tokio::test]
async fn a_record_added_beside_three_asks_the_database_as_much_as_one_added_beside_three_hundred() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    let rig = planned("local").await;
    let records = &rig.studio.records;
    let add = async |title: String| {
        records.add("shop", "sprints", data(json!({ "title": title })), &ada(), None).await.unwrap()
    };
    for nth in 1..=3 {
        add(format!("Sprint {nth}")).await;
    }
    // What the studio does by itself on the same databases is no part of
    // an add: the keeper of records' histories reads every table as one
    // is declared, and that, come late, was counted with the first add
    // (351 steps beside 316, on a hosted machine). It is stopped, and
    // what it asked before is done by the time the counting begins,
    // which is asked of each database's thread after it.
    rig.studio.halt();
    // How much the databases do, counted by each of them (`progress_handler`),
    // on the one connection each one's thread asks everything on.
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
    let counted = async |title: &str| {
        let before = done.load(Ordering::SeqCst);
        let added = add(title.to_string()).await;
        (done.load(Ordering::SeqCst) - before, added)
    };
    // The table declared again, as applying a fleet does, from another
    // thread while this one does nothing: whatever hears it here is
    // polled only once this one waits on the add, and what it asks of the
    // database is then counted with the add, as it was on that machine.
    // Kept, so that what heard it is seen to be stopped.
    let collections = rig.studio.collections.clone();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            runtime.block_on(collections.ensure("shop", "sprints", sprints("local"), None, &ada())).unwrap();
        });
    });
    let (few, fourth) = counted("Beside three").await;
    assert_eq!(fourth.position, 4.0, "placed after the last");
    for nth in 5..=300 {
        add(format!("Sprint {nth}")).await;
    }
    let (many, last) = counted("Beside three hundred").await;
    assert_eq!(last.position, 301.0, "placed after the last");
    // Each is placed after the last by one row read of an index, and
    // numbered after the highest the same way: read through, three
    // hundred records are thousands of steps more.
    assert_eq!(
        few, many,
        "an add took {few} steps of the database beside three records and {many} beside three hundred"
    );
}

#[tokio::test]
async fn a_step_that_moves_a_card_an_agent_decided_is_the_agents_and_leaves_a_card_moved_since_it_looked_where_it_is()
 {
    let rig = spoken().await;
    let board = &rig.studio.board;
    let card = board.add("shop", "Add a lamp", "", None, vec![], &ada()).await.unwrap();
    // A rung of the chain, acting with Ada's authority as the lead decided,
    // having looked at the card before it was ever moved.
    let rung = Through {
        blueprint: "std/chain-wait".into(),
        source: Source { instance: Some("in_1".into()), ..Source::actor("user:ada") },
        wrote: None,
        decided: Some("agent:shop-lead-1".into()),
        had: Some(0),
        doing: None,
    };
    let moved =
        board.move_through(&card.id, "ready", None, Some("Not yet."), &ada(), Some(&rung)).await.unwrap();
    assert_eq!(moved.status, "ready");
    // Done again, as a step that is kept is after a stop: the card has
    // been moved since that step looked, and is left where it is.
    let again =
        board.move_through(&card.id, "doing", None, Some("Not yet."), &ada(), Some(&rung)).await.unwrap();
    assert_eq!(again.status, "ready", "a card moved since the step looked is not moved again");
    assert_eq!(rig.events("board.item.moved").await.len(), 1, "and nothing is said of a move not made");
    assert_eq!(
        history_of(&rig, &card.id, 2).await,
        ["ada added it to draft", "shop-lead-1 moved it from draft to ready, saying: Not yet."],
        "the line is the agent's, the blueprint kept with it"
    );
    let line = rig.studio.collections.history(&card.id).await.unwrap().pop().unwrap();
    assert_eq!((line.who.as_str(), line.data["through"].as_str()), ("shop-lead-1", Some("std/chain-wait")));
}

#[tokio::test]
async fn the_turns_of_many_records_are_taken_once_each_beside_one_held_whatever_turns_they_share() {
    let rig = Rig::new().await;
    let collections = &rig.studio.collections;
    // There are so many turns and no more, so three hundred ids share
    // them, and one of them the turn held already: taken one by one, the
    // second of two sharing a turn waited for the first for ever.
    let held = collections.changing("the tree").await;
    let ids: Vec<String> =
        (0..300).map(|n| format!("@-topics-{n}")).chain(["the tree".to_string()]).collect();
    let taken = collections.changing_all(&["the tree"], &ids).await;
    assert!(!taken.is_empty() && taken.len() < ids.len(), "{} turns for {} ids", taken.len(), ids.len());
    drop((taken, held));
    let again = collections.changing_all(&[], &ids).await;
    assert!(again.len() > 1);
}
