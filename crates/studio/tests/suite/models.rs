//! Models: what the daemon itself reads of a record is said once, the
//! struct it reads through agrees with that, and a collection that is a
//! model's is held to it when it is declared.

use conductor::Library;
use kernel::ErrorKind;
use serde_json::{Map, Value, json};
use studio::collections::Spec;
use studio::models::{CARD, Card, MODELS};
use studio::{Changes, Collection, Collections};

use crate::{Rig, ada};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// A card's fields as a blueprint writes them, as the model has them.
fn card_fields() -> Value {
    json!({
        "title": { "type": "text", "required": true },
        "body": { "type": "text" },
        "status": { "type": "choice", "of": ["todo", "done"], "required": true },
        "assignee": { "type": "actor" },
        "labels": { "type": "list" },
        "due": { "type": "date" },
        "moved": { "type": "list" },
        "pressed": { "type": "list" },
        "sent": { "type": "text" },
        "given": { "type": "number" },
    })
}

/// The same with some of them said otherwise, left out or added to.
fn fields_but(change: impl FnOnce(&mut Map<String, Value>)) -> Value {
    let mut fields = data(card_fields());
    change(&mut fields);
    Value::Object(fields)
}

async fn declare(
    collections: &Collections,
    name: &str,
    fields: Value,
    model: Option<&str>,
) -> kernel::Result<Collection> {
    let spec = Spec::read(&fields, &json!(null), None).unwrap().modelled(model);
    collections.ensure("shop", name, spec, None, &ada()).await
}

/// Something a field of this kind holds.
fn sample(kind: &str) -> Value {
    match kind {
        "text" => json!("Some words"),
        "choice" => json!("todo"),
        "actor" => json!("agent:builder"),
        "list" => json!(["one", "two"]),
        "date" => json!("2026-11-30"),
        "number" => json!(3),
        other => panic!("a card has a `{other}`, and this test does not know what one holds"),
    }
}

#[test]
fn a_card_is_read_through_a_struct_that_has_each_field_the_model_says_and_no_other() {
    let said: Vec<&str> = CARD.fields.iter().map(|has| has.name).collect();
    // What the struct writes of a card with nothing beyond its own.
    let written = Card::default().data();
    let mut held: Vec<&str> = written.keys().map(String::as_str).collect();
    let mut wanted = said.clone();
    held.sort();
    wanted.sort();
    assert_eq!(held, wanted, "`Card` and `CARD` name the same fields");

    // Each is carried as the kind the model says: what a field of that
    // kind holds goes in and comes out as it was. A field the struct did
    // not have, or had as something else, would come out empty.
    let mut full = Map::new();
    for has in CARD.fields {
        full.insert(has.name.to_string(), sample(has.kind));
    }
    let card = Card::read(full.clone());
    assert_eq!(card.data(), full);
    assert_eq!(
        (card.title.as_str(), card.status.as_str(), card.assignee.as_deref(), card.due.as_deref()),
        ("Some words", "todo", Some("agent:builder"), Some("2026-11-30"))
    );
    assert_eq!(card.labels, ["one", "two"]);
    assert!(card.more.is_empty());

    // What its board keeps beyond a card's own is carried, untouched and unread.
    let mut more = full.clone();
    more.insert("sprint".into(), json!("shop-sprints-1"));
    more.insert("size".into(), json!(3));
    let card = Card::read(more.clone());
    assert_eq!(Value::Object(card.more.clone()), json!({ "sprint": "shop-sprints-1", "size": 3 }));
    assert_eq!(card.data(), more);

    // A card with one odd value is still a card: that field is read as nothing.
    let odd = Card::read(data(json!({ "title": "Basket", "status": "todo", "labels": "urgent", "due": 7 })));
    assert_eq!((odd.title.as_str(), odd.labels.len(), odd.due.as_deref()), ("Basket", 0, None));

    // The board that is made when no blueprint says what one is, is the model's.
    let made = studio::board::spec(&["todo".to_string()], studio::Store::Local);
    assert_eq!(made.model.as_deref(), Some("card"));
    assert_eq!(CARD.hold("board", made.fields.clone()).unwrap(), made.fields);
    let column = made.fields.iter().find(|field| field.name == studio::board::COLUMN).unwrap();
    assert_eq!((column.kind.as_str(), column.of.as_slice()), ("choice", &["todo".to_string()][..]));
}

#[tokio::test]
async fn a_collection_that_says_it_is_a_card_and_declares_a_field_as_something_else_is_refused_with_the_field()
 {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;

    let as_text = fields_but(|fields| fields["status"] = json!({ "type": "text", "required": true }));
    let refused = declare(collections, "board", as_text, Some("card")).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert_eq!(
        refused.message,
        "`board` is a `card`, whose `status` is a `choice` that is needed; it is declared here as a `text` that is needed. Write it as the model has it, or leave it out"
    );

    let loose = fields_but(|fields| fields["title"] = json!({ "type": "text" }));
    let refused = declare(collections, "board", loose, Some("card")).await.unwrap_err();
    assert_eq!(
        refused.message,
        "`board` is a `card`, whose `title` is a `text` that is needed; it is declared here as a `text` that may be left empty. Write it as the model has it, or leave it out"
    );

    // Nothing was made of either.
    assert!(collections.list(Some("shop")).await.unwrap().is_empty());

    let made = declare(collections, "board", card_fields(), Some("card")).await.unwrap();
    assert_eq!(made.model.as_deref(), Some("card"));
    let names: Vec<&str> = made.fields.iter().map(|field| field.name.as_str()).collect();
    assert_eq!(names.len(), CARD.fields.len());
    assert!(CARD.fields.iter().all(|has| names.contains(&has.name)));
}

#[tokio::test]
async fn a_field_of_the_model_left_out_is_there_all_the_same_unless_a_card_cannot_do_without_it() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;

    // An older copy of a board's blueprint: no day a card is wanted by,
    // nobody it is given to, and a field of its own.
    let older = fields_but(|fields| {
        fields.remove("due");
        fields.remove("assignee");
        fields.insert("size".into(), json!({ "type": "number" }));
    });
    let made = declare(collections, "board", older, Some("card")).await.unwrap();
    let kind_of = |name: &str| made.field(name).map(|field| (field.kind.as_str(), field.required));
    assert_eq!(kind_of("due"), Some(("date", false)));
    assert_eq!(kind_of("assignee"), Some(("actor", false)));
    assert_eq!(kind_of("size"), Some(("number", false)), "what is beyond the model is the blueprint's own");

    // And a card of it has them, as any card does.
    let board = &rig.studio.board;
    let card = board.add("shop", "Basket", "", None, Vec::new(), &ada()).await.unwrap();
    let changes = Changes {
        due: Some(Some("2026-11-30".into())),
        assignee: Some(Some("agent:builder".into())),
        more: data(json!({ "size": 3 })),
        ..Changes::default()
    };
    let changed = board.update(&card.id, changes, &ada()).await.unwrap();
    assert_eq!(
        (changed.due.as_deref(), changed.assignee.as_deref(), &changed.more),
        (Some("2026-11-30"), Some("agent:builder"), &data(json!({ "size": 3 })))
    );
    let moved = board.move_to(&card.id, "done", None, &ada()).await.unwrap();
    assert_eq!((moved.status.as_str(), moved.due.as_deref()), ("done", Some("2026-11-30")));

    // What a card cannot do without must be declared: nothing can say for
    // the blueprint what a card's columns are, and a title is needed.
    for (gone, said) in [("status", "a `choice` that is needed"), ("title", "a `text` that is needed")] {
        let without = fields_but(|fields| {
            fields.remove(gone);
        });
        let refused = declare(collections, "cards", without, Some("card")).await.unwrap_err();
        assert_eq!(
            refused.message,
            format!("`cards` is a `card`, which has `{gone}`: {said}. Declare it among the fields")
        );
    }
}

#[tokio::test]
async fn a_board_is_a_card_whether_or_not_it_says_so_and_anything_else_is_one_only_when_it_says() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;
    let plain = json!({ "title": { "type": "text", "required": true } });

    // Called something else and saying nothing, its fields are all its own.
    let tasks = declare(collections, "tasks", plain.clone(), None).await.unwrap();
    assert_eq!((tasks.model.as_deref(), tasks.fields.len()), (None, 1));

    // Saying it is a card, it is held to one.
    let short = declare(collections, "cards", plain.clone(), Some("card")).await.unwrap_err();
    assert!(short.message.starts_with("`cards` is a `card`, which has `status`"), "{}", short.message);
    let cards = declare(collections, "cards", card_fields(), Some("card")).await.unwrap();
    assert_eq!(cards.model.as_deref(), Some("card"));

    // The board is one without saying, as it was before a blueprint could say.
    let unsaid = declare(collections, "board", plain, None).await.unwrap_err();
    assert!(unsaid.message.starts_with("`board` is a `card`, which has `status`"), "{}", unsaid.message);
    let board = declare(collections, "board", card_fields(), None).await.unwrap();
    assert_eq!(board.model.as_deref(), Some("card"));
    assert_eq!(collections.get("shop", "board").await.unwrap().model.as_deref(), Some("card"));

    // A model nobody knows is said to be none, with the ones there are.
    let unknown = declare(collections, "sprints", card_fields(), Some("sprint")).await.unwrap_err();
    assert_eq!(unknown.kind, ErrorKind::Invalid);
    let known: Vec<&str> = MODELS.iter().map(|model| model.name).collect();
    assert_eq!(
        unknown.message,
        format!(
            "`sprint` is not a model a collection can be; there is: {}. A collection that is none of them says no `model`, and its fields are all its own",
            known.join(", ")
        )
    );
}

#[tokio::test]
async fn a_board_written_down_before_the_model_had_a_field_has_it_with_nothing_applied_again() {
    let rig = Rig::new().await;
    rig.board("shop").await;
    let board = &rig.studio.board;
    let card = board.add("shop", "Basket", "", None, Vec::new(), &ada()).await.unwrap();

    // As it was written down by a daemon that knew no models, and whose
    // cards had no day they were wanted by.
    let before = json!([
        { "name": "title", "kind": "text", "required": true },
        { "name": "body", "kind": "text" },
        { "name": "status", "kind": "choice", "of": studio::board::DEFAULT_COLUMNS, "required": true },
        { "name": "assignee", "kind": "actor" },
        { "name": "labels", "kind": "list" },
    ])
    .to_string();
    rig.journal
        .db()
        .call(move |connection| {
            connection.execute(
                "UPDATE collections SET fields = ?1, model = NULL WHERE project = 'shop' AND name = 'board'",
                [before],
            )
        })
        .await
        .unwrap();

    let read = rig.studio.collections.get("shop", "board").await.unwrap();
    assert_eq!(read.model.as_deref(), Some("card"));
    assert_eq!(read.field("due").map(|field| field.kind.as_str()), Some("date"));

    // Its cards are moved and changed as before, and are given a day.
    let moved = board.move_to(&card.id, "doing", None, &ada()).await.unwrap();
    assert_eq!(moved.status, "doing");
    let due = Changes { due: Some(Some("2026-12-01".into())), ..Changes::default() };
    assert_eq!(board.update(&card.id, due, &ada()).await.unwrap().due.as_deref(), Some("2026-12-01"));
    let added =
        board.add("shop", "Checkout", "", Some("ready"), vec!["urgent".into()], &ada()).await.unwrap();
    assert_eq!((added.status.as_str(), added.labels.as_slice()), ("ready", &["urgent".to_string()][..]));

    // Its columns are changed, which declares it again, and it is still the model's.
    let columns: Vec<String> =
        ["ready", "doing", "shipped"].iter().map(|column| column.to_string()).collect();
    board.set_columns("shop", columns, &ada()).await.unwrap();
    let again = rig.studio.collections.get("shop", "board").await.unwrap();
    assert_eq!(again.model.as_deref(), Some("card"));
    assert_eq!(board.get(&card.id).await.unwrap().due.as_deref(), Some("2026-12-01"));
}

/// Every `collection.ensure` in these steps that names a model, with the
/// fields it writes.
fn modelled(flow: &blueprint::Flow, found: &mut Vec<(String, Value)>) {
    for step in &flow.steps {
        if step.kind == blueprint::StepKind::Do("collection.ensure".into())
            && let Some(model) = step.with.get("model").and_then(Value::as_str)
        {
            found.push((model.to_string(), step.with.get("fields").cloned().unwrap_or(Value::Null)));
        }
        modelled(&step.inner, found);
    }
}

#[test]
fn a_blueprint_that_ships_and_names_a_model_writes_every_field_of_it_out_as_the_model_has_it() {
    let shipped = conductor::standard();
    let mut naming = Vec::new();
    for name in shipped.names(None) {
        let source = shipped.load(&name, None).unwrap();
        let read = blueprint::check(&source, &blueprint::NoCatalog).blueprint;
        let blueprint = read.unwrap_or_else(|| panic!("`{name}` cannot be read"));
        let mut found = Vec::new();
        for (_, flow) in blueprint.flows() {
            modelled(flow, &mut found);
        }
        for (model, fields) in found {
            let known = studio::models::of("", Some(&model)).unwrap_or_else(|err| {
                panic!("`{name}` declares a collection as a model there is not: {}", err.message)
            });
            // Fix this in the blueprint: write the field out under `fields`
            // as the sentence says the model has it. A standard blueprint
            // says the whole of what it makes, so that it can be read alone.
            if let Err(err) = known.unwrap().written(&fields) {
                panic!("`{name}` does not say what it makes in full: {}", err.message);
            }
            naming.push(name.clone());
        }
    }
    assert!(
        naming.contains(&"std/board".to_string()),
        "the board's blueprint says its cards are the model's"
    );
    assert!(naming.contains(&"std/fleet".to_string()), "the fleet's says its `actions` are the model's");

    // What that says of a blueprint that leaves a field out, or says it otherwise.
    let short = fields_but(|fields| {
        fields.remove("due");
    });
    assert_eq!(
        CARD.written(&short).unwrap_err().message,
        "the model `card` has `due` (a `date` that may be left empty), and it is not written among the fields"
    );
    let other = fields_but(|fields| fields["labels"] = json!({ "type": "text" }));
    assert_eq!(
        CARD.written(&other).unwrap_err().message,
        "the model `card` has `labels` as a `list` that may be left empty, and it is written as a `text` that may be left empty"
    );
    assert!(CARD.written(&card_fields()).is_ok());
}

#[test]
fn a_row_of_what_can_be_pressed_is_read_through_the_cells_the_model_says_and_no_other() {
    use studio::models::ACTION;
    // A row with every cell of the model filled, as its table holds it.
    let mut cells = Map::new();
    for has in ACTION.fields {
        let value = match has.kind {
            "flag" => json!(true),
            "choice" => json!("person"),
            _ => json!(format!("the {}", has.name)),
        };
        cells.insert(has.name.to_string(), value);
    }
    let row = json!({ "id": "shop-actions-1", "data": cells });
    let action = kernel::pressed::Action::read(&row).unwrap();
    assert_eq!(action.name, "the-name");
    assert_eq!(
        (action.title.as_str(), action.column.as_str(), action.scope.as_str(), action.says.as_str()),
        ("the title", "the column", "the scope", "the says")
    );
    assert!(action.holds && action.again && action.who == kernel::pressed::Who::Person);
    // A cell the model does not have is no part of what is read.
    let mut more = cells.clone();
    more.insert("colour".into(), json!("red"));
    assert_eq!(
        kernel::pressed::Action::read(&json!({ "id": "shop-actions-1", "data": more })).unwrap(),
        action
    );
}

#[test]
fn a_topic_and_topic_name_the_same_fields_and_the_keeper_reads_it_whole() {
    use studio::models::{TOPIC, Topic};
    let said: Vec<&str> = TOPIC.fields.iter().map(|has| has.name).collect();
    let written = serde_json::to_value(Topic::default()).unwrap();
    let held: Vec<&str> = written.as_object().unwrap().keys().map(String::as_str).collect();
    let (mut said, mut held) = (said, held);
    said.sort_unstable();
    held.sort_unstable();
    assert_eq!(held, said, "`Topic` and `TOPIC` name the same fields");
    let topic = Topic::read(&data(json!({
        "title": "Why", "body": "# Why", "status": "answered", "workspace": "ada", "followers": ["user:a"],
        "answer": "41", "colour": "red", "project": 7
    })));
    assert_eq!(
        (
            topic.title.as_str(),
            topic.status.as_deref(),
            topic.workspace.as_deref(),
            topic.project,
            topic.followers
        ),
        ("Why", Some("answered"), Some("ada"), None, vec!["user:a".to_string()]),
        "what does not fit is taken as nothing"
    );
}
