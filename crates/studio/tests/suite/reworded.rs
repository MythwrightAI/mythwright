//! A choice's numbers written as its words, stopped half way: the mark
//! that it is under way is kept on the field in the table's own row, and
//! whatever finds it next (a declaration, a start) finishes it, each row
//! said once (`studio::reworded`).

use serde_json::{Map, Value, json};
use studio::collections::Spec;

use crate::{Rig, ada};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// A table of tasks whose `priority` is a word that a number is counted to.
fn spec() -> Spec {
    let fields = json!({
        "title": { "type": "text", "required": true },
        "priority": { "type": "choice", "of": ["urgent", "high", "normal", "low"], "unsaid": "normal" },
    });
    Spec::read(&fields, &json!(null), Some("local")).unwrap()
}

/// What a daemon stopped half way through rewording `tasks` leaves: its
/// declaration kept with the mark on `priority`, the first task's number
/// written as its word and said, the other two still numbers.
async fn half_reworded(rig: &Rig) -> Vec<String> {
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.collections.ensure("shop", "tasks", spec(), None, &ada()).await.unwrap();
    let mut ids = Vec::new();
    for title in ["Cart", "Till", "Shelf"] {
        let added =
            rig.studio.records.add("shop", "tasks", data(json!({ "title": title })), &ada(), None).await;
        ids.push(added.unwrap().id);
    }
    let planted = (ids.clone(), "shop".to_string());
    let shop = rig.journal.records_of(Some("shop"));
    let planting = shop.db().call(move |connection| {
        let (ids, project) = planted;
        connection.execute(
            "UPDATE records SET data = json_set(data, '$.priority', 'urgent') WHERE id = ?1",
            [&ids[0]],
        )?;
        connection
            .execute("UPDATE records SET data = json_set(data, '$.priority', 2) WHERE id = ?1", [&ids[1]])?;
        connection
            .execute("UPDATE records SET data = json_set(data, '$.priority', -1) WHERE id = ?1", [&ids[2]])?;
        let fields: String = connection.query_row(
            "SELECT fields FROM collections WHERE project = ?1 AND name = 'tasks'",
            [&project],
            |row| row.get(0),
        )?;
        let mut fields: Vec<Value> = serde_json::from_str(&fields).unwrap();
        let priority = fields.iter_mut().find(|field| field["name"] == "priority").unwrap();
        priority["rewording"] = json!(true);
        connection.execute(
            "UPDATE collections SET fields = ?2 WHERE project = ?1 AND name = 'tasks'",
            [&project, &serde_json::to_string(&fields).unwrap()],
        )
    });
    planting.await.unwrap();
    ids
}

/// What each task holds of its priority, and what was said of each being reworded.
async fn held(rig: &Rig, collections: &studio::Collections, ids: &[String]) -> (Vec<Value>, Vec<Value>) {
    let table = collections.get("shop", "tasks").await.unwrap();
    assert!(!table.field("priority").unwrap().rewording, "the mark is taken off once every row is a word");
    let records = collections.records(&table).await.unwrap();
    let priority =
        |id: &String| records.iter().find(|record| &record.id == id).unwrap().data["priority"].clone();
    let said =
        rig.events("record.reworded").await.into_iter().map(|event| event.data["id"].clone()).collect();
    (ids.iter().map(priority).collect(), said)
}

#[tokio::test]
async fn a_rewording_stopped_half_way_is_finished_by_the_next_declaration_each_row_said_once() {
    let rig = Rig::new().await;
    let ids = half_reworded(&rig).await;
    // Declared again as it was: nothing of the field is another, and the mark is what asks.
    rig.studio.collections.ensure("shop", "tasks", spec(), None, &ada()).await.unwrap();
    let (words, said) = held(&rig, &rig.studio.collections, &ids).await;
    assert_eq!(words, [json!("urgent"), json!("urgent"), json!("low")]);
    assert_eq!(said, [json!(ids[1]), json!(ids[2])], "the two left, each once");

    rig.studio.collections.ensure("shop", "tasks", spec(), None, &ada()).await.unwrap();
    assert_eq!(held(&rig, &rig.studio.collections, &ids).await.1.len(), 2, "and nothing more");
}

#[tokio::test]
async fn a_rewording_stopped_half_way_is_finished_as_the_daemon_starts_again_each_row_said_once() {
    let rig = Rig::new().await;
    let ids = half_reworded(&rig).await;
    let open = || {
        studio::Studio::open(
            rig.journal.clone(),
            journal::Db::memory().unwrap(),
            rig.dir.join("projects"),
            rig.dir.join("attachments"),
        )
    };
    let again = open().await.unwrap();
    let (words, said) = held(&rig, &again.collections, &ids).await;
    assert_eq!(words, [json!("urgent"), json!("urgent"), json!("low")]);
    assert_eq!(said, [json!(ids[1]), json!(ids[2])], "the two left, each once");

    let once_more = open().await.unwrap();
    assert_eq!(held(&rig, &once_more.collections, &ids).await.1.len(), 2, "and nothing more");
}

#[tokio::test]
async fn a_rewording_that_ends_takes_its_mark_off_and_a_declaration_after_it_asks_the_rows_nothing() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let numbered = json!({ "title": { "type": "text", "required": true }, "priority": { "type": "number" } });
    let before = Spec::read(&numbered, &json!(null), Some("local")).unwrap();
    let collections = &rig.studio.collections;
    collections.ensure("shop", "tasks", before, None, &ada()).await.unwrap();
    let cart = data(json!({ "title": "Cart", "priority": 2 }));
    let cart = rig.studio.records.add("shop", "tasks", cart, &ada(), None).await.unwrap();

    collections.ensure("shop", "tasks", spec(), None, &ada()).await.unwrap();
    let (words, said) = held(&rig, collections, std::slice::from_ref(&cart.id)).await;
    assert_eq!((words, said), (vec![json!("urgent")], vec![json!(cart.id)]));
    // Declared again, nothing is left under way and nothing more is said.
    collections.ensure("shop", "tasks", spec(), None, &ada()).await.unwrap();
    assert_eq!(held(&rig, collections, std::slice::from_ref(&cart.id)).await.1.len(), 1);
}
