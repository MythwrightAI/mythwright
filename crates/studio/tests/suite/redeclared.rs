//! A collection declared again with other fields: the records it already
//! has can still be written afterwards, or it is not declared so.

use kernel::ErrorKind;
use serde_json::{Map, Value, json};
use studio::collections::Spec;
use studio::{Changes, Collection, Collections};

use crate::{Rig, ada};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// Declares `tasks` in the project `shop`, with these fields.
async fn tasks(collections: &Collections, fields: Value, store: &str) -> kernel::Result<Collection> {
    let spec = Spec::read(&fields, &json!(null), Some(store)).unwrap();
    collections.ensure("shop", "tasks", spec, None, &ada()).await
}

async fn add(collections: &Collections, to: &Collection, value: Value) -> studio::Record {
    collections.add(to, data(value), 1.0, &ada()).await.unwrap()
}

/// A board as a blueprint declares one, with `more` beyond a card's own fields.
fn board(columns: &[&str], more: Value) -> Spec {
    let mut fields = data(json!({
        "title": { "type": "text", "required": true },
        "body": { "type": "text" },
        "status": { "type": "choice", "of": columns, "required": true },
        "assignee": { "type": "actor" },
        "labels": { "type": "list" },
        "due": { "type": "date" },
    }));
    fields.extend(data(more));
    Spec::read(&Value::Object(fields), &json!({ "board": { "kind": "board", "by": "status" } }), None)
        .unwrap()
}

#[tokio::test]
async fn a_field_no_longer_declared_is_no_longer_read_and_its_records_are_written_as_before() {
    for store in ["local", "files"] {
        let rig = Rig::new().await;
        rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
        let collections = &rig.studio.collections;
        let sized = json!({ "title": { "type": "text", "required": true }, "size": { "type": "number" } });
        let before = tasks(collections, sized.clone(), store).await.unwrap();
        let basket = add(collections, &before, json!({ "title": "Basket", "size": 3 })).await;
        let checkout = add(collections, &before, json!({ "title": "Checkout", "size": 5 })).await;

        let plain = json!({ "title": { "type": "text", "required": true } });
        let after = tasks(collections, plain, store).await.unwrap();
        let read = collections.records(&after).await.unwrap();
        assert_eq!(read.len(), 2, "{store}");
        assert!(read.iter().all(|record| !record.data.contains_key("size")), "{store}: {read:?}");

        // The one thing that used to stop here: a record from before is written.
        let mut renamed = read[0].clone();
        renamed.data.insert("title".into(), json!("Trolley"));
        let kept = collections.put(&after, renamed).await.unwrap();
        assert_eq!(kept.data, data(json!({ "title": "Trolley" })), "{store}");
        // Whoever writes the field by name is still told it is not one.
        let mut stray = kept.clone();
        stray.data.insert("size".into(), json!(8));
        let refused = collections.put(&after, stray).await.unwrap_err();
        assert_eq!(refused.message, "`size` is not a field of `tasks`; its fields are: title", "{store}");

        // What a record held of it went for good when the record was written.
        let again = tasks(collections, sized, store).await.unwrap();
        let read = collections.records(&again).await.unwrap();
        let size_of =
            |id: &str| read.iter().find(|record| record.id == id).unwrap().data.get("size").cloned();
        assert!(size_of(&basket.id).is_none_or(|size| size.is_null()), "{store}");
        if store == "local" {
            assert_eq!(size_of(&checkout.id), Some(json!(5)), "one not written since still has it");
        }
    }
}

#[tokio::test]
async fn a_board_declared_again_without_a_field_its_cards_had_still_moves_them() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let (collections, cards) = (&rig.studio.collections, &rig.studio.board);
    let with = board(&["todo", "done"], json!({ "size": { "type": "number" } }));
    collections.ensure("shop", "board", with, None, &ada()).await.unwrap();
    let card = cards.add("shop", "Basket", "", None, Vec::new(), &ada()).await.unwrap();
    let sized = Changes { more: data(json!({ "size": 3 })), ..Changes::default() };
    cards.update(&card.id, sized, &ada()).await.unwrap();

    collections.ensure("shop", "board", board(&["todo", "done"], json!({})), None, &ada()).await.unwrap();
    let moved = cards.move_to(&card.id, "done", None, &ada()).await.unwrap();
    assert_eq!(moved.status, "done");
    assert!(moved.more.is_empty(), "{:?}", moved.more);
    let renamed = Changes { title: Some("Trolley".into()), ..Changes::default() };
    assert_eq!(cards.update(&card.id, renamed, &ada()).await.unwrap().title, "Trolley");
}

#[tokio::test]
async fn a_field_declared_as_another_kind_is_refused_while_records_hold_what_would_not_be_one() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;
    let fields = |size: Value| json!({ "title": { "type": "text", "required": true }, "size": size });
    let worded = tasks(collections, fields(json!({ "type": "text" })), "local").await.unwrap();
    let small = add(collections, &worded, json!({ "title": "Basket", "size": "small" })).await;
    let big = add(collections, &worded, json!({ "title": "Checkout", "size": "big" })).await;
    add(collections, &worded, json!({ "title": "Returns" })).await;

    let refused = tasks(collections, fields(json!({ "type": "number" })), "local").await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert_eq!(
        refused.message,
        "`size` was a `text` and is now declared a `number`, which is not what 2 record(s) of `tasks` hold of it; change them before declaring it so"
    );
    let chosen = json!({ "type": "choice", "of": ["small", "medium"] });
    let refused = tasks(collections, fields(chosen.clone()), "local").await.unwrap_err();
    assert!(refused.message.contains("is not what 1 record(s) of `tasks` hold of it"), "{refused}");
    assert_eq!(
        collections.get("shop", "tasks").await.unwrap().field("size").unwrap().kind,
        "text",
        "it is what it was"
    );

    // Every value there is fits, and one that has none fits anything.
    let mut big = big;
    big.data.insert("size".into(), json!("medium"));
    collections.put(&worded, big).await.unwrap();
    let chosen = tasks(collections, fields(chosen), "local").await.unwrap();
    assert_eq!(chosen.field("size").unwrap().kind, "choice");
    let mut small = small;
    small.data.insert("title".into(), json!("Trolley"));
    assert_eq!(collections.put(&chosen, small).await.unwrap().data["size"], json!("small"));
}

#[tokio::test]
async fn a_field_newly_needed_is_refused_while_records_have_none_of_it() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;
    let fields = |owner: Value| json!({ "title": { "type": "text", "required": true }, "owner": owner });
    let loose =
        tasks(collections, json!({ "title": { "type": "text", "required": true } }), "local").await.unwrap();
    let basket = add(collections, &loose, json!({ "title": "Basket" })).await;

    let needed = json!({ "type": "text", "required": true });
    for was in ["not there", "there and not needed"] {
        let refused = tasks(collections, fields(needed.clone()), "local").await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Conflict, "{was}");
        assert_eq!(
            refused.message,
            "`owner` is now declared as needed, and 1 record(s) of `tasks` have none; declare it without `required`, give each of them one, and then say it is needed",
            "{was}"
        );
        tasks(collections, fields(json!({ "type": "text" })), "local").await.unwrap();
    }
    let loose = collections.get("shop", "tasks").await.unwrap();
    let mut owned = basket;
    owned.data.insert("owner".into(), json!("Ada"));
    collections.put(&loose, owned).await.unwrap();
    let strict = tasks(collections, fields(needed), "local").await.unwrap();
    assert!(strict.field("owner").unwrap().required);
}

#[tokio::test]
async fn a_board_whose_columns_are_changed_keeps_what_else_its_blueprint_declared_of_a_card() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let (collections, cards) = (&rig.studio.collections, &rig.studio.board);
    let mut declared = board(&["todo", "done"], json!({ "size": { "type": "number" } }));
    declared = declared.with_history(&json!({ "board.item.moved": "{actor} carried it to {to}" })).unwrap();
    collections.ensure("shop", "board", declared, None, &ada()).await.unwrap();
    let card = cards.add("shop", "Basket", "", None, Vec::new(), &ada()).await.unwrap();
    let sized = Changes { more: data(json!({ "size": 3 })), ..Changes::default() };
    cards.update(&card.id, sized, &ada()).await.unwrap();

    let columns = vec!["todo".to_string(), "doing".to_string(), "done".to_string()];
    assert_eq!(cards.set_columns("shop", columns.clone(), &ada()).await.unwrap(), columns);
    let kept = collections.get("shop", "board").await.unwrap();
    assert_eq!(kept.field("size").map(|field| field.kind.as_str()), Some("number"));
    assert_eq!(kept.field("status").unwrap().of, columns);
    assert_eq!(kept.history.len(), 1, "and what is kept as a card's history: {:?}", kept.history);
    let moved = cards.move_to(&card.id, "doing", None, &ada()).await.unwrap();
    assert_eq!(moved.more, data(json!({ "size": 3 })));
}

#[tokio::test]
async fn a_card_is_added_with_what_else_its_board_needs_of_it() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let (collections, cards) = (&rig.studio.collections, &rig.studio.board);
    let needs = board(&["todo", "done"], json!({ "size": { "type": "number", "required": true } }));
    collections.ensure("shop", "board", needs, None, &ada()).await.unwrap();

    let bare = cards.add("shop", "Basket", "", None, Vec::new(), &ada()).await.unwrap_err();
    assert_eq!(bare.message, "`size` is needed");
    let card = |more: Value| studio::board::New {
        title: "Basket".into(),
        more: data(more),
        ..studio::board::New::default()
    };
    let added = cards.add_new("shop", card(json!({ "size": 3 })), &ada()).await.unwrap();
    assert_eq!((added.status.as_str(), &added.more), ("todo", &data(json!({ "size": 3 }))));

    let own = cards.add_new("shop", card(json!({ "size": 3, "status": "done" })), &ada()).await.unwrap_err();
    assert_eq!(
        own.message,
        "`status` is one of a card's own fields and is not written among the others; give it by its name"
    );
    let stray =
        cards.add_new("shop", card(json!({ "size": 3, "epic": "shop-1" })), &ada()).await.unwrap_err();
    assert!(stray.message.starts_with("`epic` is not a field of `board`"), "{stray}");
    assert_eq!(cards.list("shop").await.unwrap().len(), 1, "what was refused was not added");
}
