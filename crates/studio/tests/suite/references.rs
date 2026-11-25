//! A field that names a record of another collection: a card's sprint.

use kernel::ErrorKind;
use serde_json::{Map, Value, json};
use studio::collections::Spec;
use studio::{Changes, Collection, Collections};

use crate::{Rig, ada};

fn spec(fields: Value, store: &str) -> Spec {
    Spec::read(&fields, &json!(null), Some(store)).unwrap()
}

fn sprints(store: &str) -> Spec {
    spec(json!({ "title": { "type": "text", "required": true } }), store)
}

/// Tasks, each of which may be in a sprint and may come after another task.
fn tasks(store: &str) -> Spec {
    spec(
        json!({
            "title": { "type": "text", "required": true },
            "sprint": { "type": "record", "of": "sprints" },
            "after": { "type": "record", "of": "tasks" },
        }),
        store,
    )
}

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// A project with sprints and tasks, kept where `store` says.
async fn planned(rig: &Rig, project: &str, store: &str) -> (Collection, Collection) {
    rig.studio.projects.create(project, None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;
    let project = blueprint::expr::slug(project);
    let sprints = collections.ensure(&project, "sprints", sprints(store), None, &ada()).await.unwrap();
    let tasks = collections.ensure(&project, "tasks", tasks(store), None, &ada()).await.unwrap();
    (sprints, tasks)
}

async fn add(collections: &Collections, to: &Collection, value: Value) -> kernel::Result<studio::Record> {
    collections.add(to, data(value), 1.0, &ada()).await
}

#[tokio::test]
async fn a_field_that_names_a_record_says_the_one_collection_it_names_which_is_there_already_or_is_itself() {
    let unsaid = Spec::read(&json!({ "sprint": { "type": "record" } }), &json!(null), None).unwrap_err();
    assert_eq!(
        unsaid.message,
        "the field `sprint` names records and needs `of`: the one collection whose records it can name, as in `of: sprints`"
    );
    let two = json!({ "sprint": { "type": "record", "of": ["sprints", "releases"] } });
    assert!(Spec::read(&two, &json!(null), None).unwrap_err().message.contains("the one collection"));

    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;
    let early = collections.ensure("shop", "tasks", tasks("local"), None, &ada()).await.unwrap_err();
    assert_eq!(early.kind, ErrorKind::Invalid);
    assert_eq!(
        early.message,
        "the field `sprint` of `tasks` names records of `sprints`, and the project `shop` has no collection called `sprints`; declare `sprints` first, in a step before this one"
    );
    assert_eq!(collections.list(Some("shop")).await.unwrap(), [], "nothing was declared by half");

    collections.ensure("shop", "sprints", sprints("local"), None, &ada()).await.unwrap();
    // `after` names a task, and tasks are only now being declared.
    let made = collections.ensure("shop", "tasks", tasks("local"), None, &ada()).await.unwrap();
    assert_eq!(made.field("sprint").unwrap().points_at(), Some("sprints"));
    assert_eq!(made.field("after").unwrap().points_at(), Some("tasks"));
    assert_eq!(made.field("title").unwrap().points_at(), None);
}

#[tokio::test]
async fn a_record_names_only_a_record_that_is_there_in_the_collection_its_field_says() {
    let rig = Rig::new().await;
    let collections = &rig.studio.collections;
    let (sprints, tasks) = planned(&rig, "Shop", "local").await;
    let (elsewhere, _) = planned(&rig, "Farm", "local").await;
    let sprint = add(collections, &sprints, json!({ "title": "November" })).await.unwrap();
    let theirs = add(collections, &elsewhere, json!({ "title": "Harvest" })).await.unwrap();
    assert_eq!((sprint.id.as_str(), theirs.id.as_str()), ("shop-sprints-1", "farm-sprints-1"));

    let first =
        add(collections, &tasks, json!({ "title": "Basket", "sprint": " shop-sprints-1 " })).await.unwrap();
    assert_eq!(first.data["sprint"], json!("shop-sprints-1"), "it is kept as the id alone");
    assert_eq!(first.data["after"], json!(null));
    let second = add(collections, &tasks, json!({ "title": "Checkout", "after": first.id, "sprint": "" }))
        .await
        .unwrap();
    assert_eq!((&second.data["after"], &second.data["sprint"]), (&json!("shop-tasks-1"), &json!(null)));

    // One that is not there; a task where a sprint is asked for; another project's sprint.
    for wrong in ["shop-sprints-9", "shop-tasks-1", "farm-sprints-1", "November"] {
        let refused =
            add(collections, &tasks, json!({ "title": "Returns", "sprint": wrong })).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{wrong}");
        assert_eq!(
            refused.message,
            format!(
                "`{wrong}` is not a record of `sprints`, which is what the `sprint` of `tasks` names. Give the id of one that is there, which are written `shop-sprints-1`, `shop-sprints-2` and so on, or leave it empty"
            )
        );
    }
    let number = add(collections, &tasks, json!({ "title": "Returns", "sprint": 1 })).await.unwrap_err();
    assert_eq!(
        number.message,
        format!(
            "`sprint` must be the id of a record of `sprints`, as `{} record list shop sprints` shows them",
            brand::CLI
        )
    );
    assert_eq!(collections.records(&tasks).await.unwrap().len(), 2, "what was refused was not added");

    // Written back, it is asked again of what it now names.
    let mut changed = first.clone();
    changed.data.insert("sprint".into(), json!("shop-sprints-2"));
    assert!(
        collections.put(&tasks, changed).await.unwrap_err().message.starts_with("`shop-sprints-2` is not")
    );
}

#[tokio::test]
async fn a_record_that_names_one_since_taken_away_is_read_and_changed_as_before_and_nothing_names_it_anew() {
    for store in ["local", "files"] {
        let rig = Rig::new().await;
        let collections = &rig.studio.collections;
        let (sprints, tasks) = planned(&rig, "Shop", store).await;
        let sprint = add(collections, &sprints, json!({ "title": "November" })).await.unwrap();
        let task = add(collections, &tasks, json!({ "title": "Basket", "sprint": sprint.id })).await.unwrap();
        if store == "files" {
            let dir = rig.studio.projects.dir_of("shop").unwrap();
            let kept = std::fs::read_to_string(dir.join("tasks/1.md")).unwrap();
            assert!(kept.contains("sprint: \"shop-sprints-1\"\n"), "{kept}");
            // Nobody is asked before a file is deleted.
            std::fs::remove_file(dir.join("sprints/1.md")).unwrap();
        } else {
            collections.remove(&sprints, &sprint).await.unwrap();
        }

        let (_, read) = collections.record(&task.id).await.unwrap();
        assert_eq!(read.data["sprint"], json!("shop-sprints-1"), "{store}: it says what it said");
        let mut renamed = read.clone();
        renamed.data.insert("title".into(), json!("A basket"));
        let kept = collections.put(&tasks, renamed).await.unwrap();
        assert_eq!(
            (&kept.data["title"], &kept.data["sprint"]),
            (&json!("A basket"), &json!("shop-sprints-1")),
            "{store}"
        );

        let anew = add(collections, &tasks, json!({ "title": "Checkout", "sprint": "shop-sprints-1" }))
            .await
            .unwrap_err();
        assert!(anew.message.starts_with("`shop-sprints-1` is not a record of `sprints`"), "{store}: {anew}");
        // Taken off, it is not put back.
        let mut cleared = kept.clone();
        cleared.data.insert("sprint".into(), json!(null));
        collections.put(&tasks, cleared).await.unwrap();
        assert!(collections.put(&tasks, kept).await.is_err(), "{store}");
    }
}

#[tokio::test]
async fn a_card_names_a_record_among_what_else_its_board_keeps_of_it() {
    let rig = Rig::new().await;
    let (sprints, _) = planned(&rig, "Shop", "local").await;
    let collections = &rig.studio.collections;
    // A board as a blueprint declares one, with a field beyond a card's own.
    let board = Spec::read(
        &json!({
            "title": { "type": "text", "required": true },
            "body": { "type": "text" },
            "status": { "type": "choice", "of": ["todo", "done"], "required": true },
            "labels": { "type": "list" },
            "sprint": { "type": "record", "of": "sprints" },
        }),
        &json!({ "board": { "kind": "board", "by": "status" } }),
        None,
    )
    .unwrap();
    collections.ensure("shop", "board", board, None, &ada()).await.unwrap();
    let sprint = add(collections, &sprints, json!({ "title": "November" })).await.unwrap();
    let board = &rig.studio.board;
    let card = board.add("shop", "Basket", "", None, Vec::new(), &ada()).await.unwrap();
    assert!(card.more.is_empty());
    assert_eq!(
        serde_json::to_value(&card).unwrap().get("more"),
        None,
        "a card with nothing more says nothing of it"
    );

    let more = |value: Value| Changes { more: data(value), ..Changes::default() };
    let placed = board.update(&card.id, more(json!({ "sprint": sprint.id })), &ada()).await.unwrap();
    assert_eq!(placed.more, data(json!({ "sprint": "shop-sprints-1" })));
    board.update(&card.id, more(json!({ "sprint": sprint.id })), &ada()).await.unwrap();
    let updates = rig.events("board.item.updated").await;
    assert_eq!(updates.len(), 1, "saying the same thing again is not a change");
    assert_eq!(updates[0].data["changed"], json!(["sprint"]));

    let missing =
        board.update(&card.id, more(json!({ "sprint": "shop-sprints-2" })), &ada()).await.unwrap_err();
    assert!(missing.message.starts_with("`shop-sprints-2` is not a record of `sprints`"), "{missing}");
    let stray = board.update(&card.id, more(json!({ "epic": "shop-1" })), &ada()).await.unwrap_err();
    assert!(stray.message.starts_with("`epic` is not a field of `board`"), "{stray}");
    let own = board.update(&card.id, more(json!({ "status": "done" })), &ada()).await.unwrap_err();
    assert_eq!(
        own.message,
        "`status` is one of a card's own fields and is not changed among the others; move the card to change it"
    );

    // The sprint goes, and the card that was in it is moved all the same.
    collections.remove(&sprints, &sprint).await.unwrap();
    let moved = board.move_to(&card.id, "done", None, &ada()).await.unwrap();
    assert_eq!((moved.status.as_str(), &moved.more["sprint"]), ("done", &json!("shop-sprints-1")));
    let cleared = board.update(&card.id, more(json!({ "sprint": "" })), &ada()).await.unwrap();
    assert!(cleared.more.is_empty());
}

#[tokio::test]
async fn a_field_may_name_several_records_of_one_collection_each_of_which_is_there_when_it_is_named() {
    let unsaid = Spec::read(&json!({ "in": { "type": "records" } }), &json!(null), None).unwrap_err();
    assert!(unsaid.message.contains("names records and needs `of`"), "{}", unsaid.message);

    for store in ["local", "files"] {
        let rig = Rig::new().await;
        let (sprints, _) = planned(&rig, "Shop", store).await;
        let collections = &rig.studio.collections;
        // A goal is for one sprint or for several.
        let goals = spec(
            json!({ "title": { "type": "text", "required": true }, "in": { "type": "records", "of": "sprints" } }),
            store,
        );
        let goals = collections.ensure("shop", "goals", goals, None, &ada()).await.unwrap();
        assert_eq!(goals.field("in").unwrap().points_at(), Some("sprints"));
        let mut made = Vec::new();
        for title in ["November", "December", "January"] {
            made.push(add(collections, &sprints, json!({ "title": title })).await.unwrap().id);
        }

        // As a list, or as they are written on a command line; each once, in the order given.
        let ship =
            add(collections, &goals, json!({ "title": "Ship it", "in": [made[0], made[1], made[0]] })).await;
        let ship = ship.unwrap();
        assert_eq!(ship.data["in"], json!([made[0], made[1]]), "{store}");
        let line = format!(" {}, {} ", made[2], made[1]);
        let tidy = add(collections, &goals, json!({ "title": "Tidy up", "in": line })).await.unwrap();
        assert_eq!(tidy.data["in"], json!([made[2], made[1]]), "{store}");
        // None is nothing, said either way.
        for none in [json!([]), json!(""), json!(null)] {
            let idle = add(collections, &goals, json!({ "title": "Rest", "in": none })).await.unwrap();
            assert_eq!(idle.data.get("in").cloned().unwrap_or(Value::Null), Value::Null, "{store}");
        }
        assert_eq!(goals.field("in").unwrap().named(&ship.data["in"]), [made[0].as_str(), made[1].as_str()]);

        // One of them that is not there is refused, and says which.
        let wrong =
            add(collections, &goals, json!({ "title": "Dream", "in": [made[0], "shop-sprints-99"] })).await;
        let wrong = wrong.unwrap_err();
        assert_eq!(wrong.kind, ErrorKind::Invalid, "{store}");
        assert!(
            wrong.message.starts_with(
                "`shop-sprints-99` is not a record of `sprints`, which is what the `in` of `goals` names"
            ),
            "{}",
            wrong.message
        );
        let wrong = add(collections, &goals, json!({ "title": "Dream", "in": [1, 2] })).await.unwrap_err();
        assert!(
            wrong.message.contains("`in` must be a list of the ids of records of `sprints`"),
            "{}",
            wrong.message
        );

        // One it named is taken away: it is read as it was, and written back
        // as it was; what it did not name before must still be there.
        collections.remove(&sprints, &collections.record(&made[1]).await.unwrap().1).await.unwrap();
        let (_, mut kept) = collections.record(&ship.id).await.unwrap();
        assert_eq!(kept.data["in"], json!([made[0], made[1]]), "{store}");
        kept.data.insert("title".into(), json!("Ship it soon"));
        let kept = collections.put(&goals, kept).await.unwrap();
        assert_eq!(kept.data["in"], json!([made[0], made[1]]), "{store}");
        let (_, mut anew) = collections.record(&tidy.id).await.unwrap();
        anew.data.insert("in".into(), json!([made[2], "shop-sprints-98"]));
        assert_eq!(collections.put(&goals, anew).await.unwrap_err().kind, ErrorKind::Invalid, "{store}");
    }
}
