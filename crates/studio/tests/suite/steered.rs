//! A table whose records say what is done with what another's give says
//! that it steers that one (`steers`), and a record of it is then added
//! or changed only by whoever could write each record of that one as it
//! stands: nobody has what holds more than they do act on their say by
//! writing beside it.

use kernel::{ErrorKind, Principal};
use serde_json::{Map, Value, json};
use studio::collections::Spec;

use crate::{Rig, ada, agent};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// A table of keys, each of which gives somebody what its list says and
/// may be loud, which takes running commands.
fn keys() -> Spec {
    let fields = json!({
        "title": { "type": "text", "required": true },
        "may": { "type": "list", "takes": "each" },
        "loud": { "type": "flag", "takes": { "true": "shell" } },
    });
    Spec::read(&fields, &json!(null), None).unwrap()
}

/// A table that says when each key is used, and so steers them.
fn uses(steers: Option<&str>) -> Spec {
    let fields = json!({ "title": { "type": "text", "required": true }, "at": { "type": "text" } });
    Spec::read(&fields, &json!(null), None).unwrap().steering(steers)
}

/// Somebody who may write records and move cards, and holds no more.
fn narrow() -> Principal {
    agent("narrow", &["projects", "records", "board"])
}

const STEERS: &str = "a record of `uses` steers the records of `keys`, and is added or changed only by whoever \
                      could write each of those as it stands";

#[tokio::test]
async fn a_record_that_steers_is_added_and_changed_by_whoever_could_write_what_it_steers_and_taken_away_by_whoever_may_change_records()
 {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let (collections, records) = (&rig.studio.collections, &rig.studio.records);
    let narrow = narrow();
    let made = collections.ensure("shop", "uses", uses(Some("Keys")), None, &ada()).await.unwrap();
    // The table it names is known by its name as any table is, and is
    // said to whoever reads what this one is.
    assert_eq!(made.steers.as_deref(), Some("keys"));
    assert_eq!(serde_json::to_value(&made).unwrap()["steers"], "keys");
    assert_eq!(collections.get("shop", "uses").await.unwrap().steers.as_deref(), Some("keys"));

    // A table that is not there has no records to steer.
    let nightly = records.add("shop", "uses", data(json!({ "title": "nightly" })), &narrow, None).await;
    let nightly = nightly.unwrap().id;
    // Nor has one with none; and one whose records they could all have
    // written is no more in the way.
    collections.ensure("shop", "keys", keys(), None, &ada()).await.unwrap();
    records.add("shop", "uses", data(json!({ "title": "weekly" })), &narrow, None).await.unwrap();
    let own = json!({ "title": "ci", "may": ["records:read"] });
    records.add("shop", "keys", data(own), &narrow, None).await.unwrap();
    records.update(&nightly, data(json!({ "at": "02:00" })), &narrow, None).await.unwrap();

    // A key that gives more than they hold: nothing that steers it is
    // theirs to add, or to change in any cell.
    let wide =
        records.add("shop", "keys", data(json!({ "title": "cd", "may": ["admin"] })), &ada(), None).await;
    let wide = wide.unwrap().id;
    let refused =
        records.add("shop", "uses", data(json!({ "title": "hourly" })), &narrow, None).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert_eq!(
        refused.message,
        format!(
            "{STEERS}. Of {wide}: a record of `keys` whose `may` says `admin` is changed only by whoever holds \
             `admin`, which narrow does not: nobody gives what they do not hold"
        )
    );
    let refused = records.update(&nightly, data(json!({ "at": "03:00" })), &narrow, None).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert!(refused.message.starts_with(STEERS), "{}", refused.message);
    let kept = collections.record(&nightly).await.unwrap().1;
    assert_eq!(kept.data["at"], "02:00");
    assert_eq!(collections.records(&made).await.unwrap().len(), 2);
    // Somebody who holds what it gives writes both.
    let holds = agent("holds", &["projects", "records", "admin"]);
    records.add("shop", "uses", data(json!({ "title": "hourly" })), &holds, None).await.unwrap();
    records.update(&nightly, data(json!({ "at": "03:00" })), &holds, None).await.unwrap();

    // Or by what one of its cells takes, as it stands.
    records.update(&wide, data(json!({ "may": ["records"], "loud": true })), &ada(), None).await.unwrap();
    let refused = records.update(&nightly, data(json!({ "at": "04:00" })), &narrow, None).await.unwrap_err();
    assert_eq!(
        refused.message,
        format!(
            "{STEERS}. Of {wide}: giving `loud` of `keys` the value `true` takes `shell`, which narrow does not \
             hold"
        )
    );

    // What is steered is written by whoever may write it, as it was; and
    // a record that steers is taken away by whoever may change records,
    // since taking one away has nothing done on anybody's say.
    records.update(&wide, data(json!({ "title": "cd, loud" })), &ada(), None).await.unwrap();
    let docs = json!({ "title": "docs", "may": ["records:read"] });
    records.add("shop", "keys", data(docs), &narrow, None).await.unwrap();
    records.remove(&nightly, &narrow, None).await.unwrap();
    assert!(collections.record(&nightly).await.is_err());
}

#[tokio::test]
async fn a_table_is_declared_again_so_that_it_steers_nothing_or_something_else_by_whoever_holds_admin() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;
    collections.ensure("shop", "keys", keys(), None, &ada()).await.unwrap();
    collections.ensure("shop", "uses", uses(Some("keys")), None, &ada()).await.unwrap();
    let narrow = narrow();
    for steers in [None, Some("notes"), Some("  ")] {
        let refused = collections.ensure("shop", "uses", uses(steers), None, &narrow).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Denied);
        assert_eq!(
            refused.message,
            "a record of `uses` steers the records of `keys`, and declaring `uses` again so that it does not \
             takes `admin`, which narrow does not hold"
        );
    }
    assert_eq!(collections.get("shop", "uses").await.unwrap().steers.as_deref(), Some("keys"));
    // Said again as it is, it is declared by whoever may declare it; and
    // one that steered nothing is said to steer by them too.
    collections.ensure("shop", "uses", uses(Some("keys")), None, &narrow).await.unwrap();
    collections.ensure("shop", "notes", uses(None), None, &narrow).await.unwrap();
    let notes = collections.ensure("shop", "notes", uses(Some("keys")), None, &narrow).await.unwrap();
    assert_eq!(notes.steers.as_deref(), Some("keys"));
    // Whoever manages credentials declares it away.
    let again = collections.ensure("shop", "uses", uses(None), None, &ada()).await.unwrap();
    assert_eq!(again.steers, None);
    assert!(serde_json::to_value(&again).unwrap().get("steers").is_none());
}

#[tokio::test]
async fn a_card_of_a_board_that_steers_is_added_changed_and_moved_by_whoever_could_write_what_it_steers() {
    let rig = Rig::new().await;
    rig.board("shop").await;
    let (collections, board) = (&rig.studio.collections, &rig.studio.board);
    let card = board.add("shop", "Add a cart", "", None, vec![], &ada()).await.unwrap();
    // A board that ships steers nothing, and is moved on by whoever may
    // move a card.
    let narrow = narrow();
    assert_eq!(collections.get("shop", "board").await.unwrap().steers, None);
    board.move_to(&card.id, "ready", None, &narrow).await.unwrap();

    // The same board, said to steer a table one of whose records gives
    // more than they hold.
    collections.ensure("shop", "keys", keys(), None, &ada()).await.unwrap();
    let wide = json!({ "title": "cd", "may": ["admin"] });
    let wide = rig.studio.records.add("shop", "keys", data(wide), &ada(), None).await.unwrap().id;
    let fields = json!({
        "title": { "type": "text", "required": true },
        "status": { "type": "choice", "of": studio::board::DEFAULT_COLUMNS, "required": true },
    });
    let views = json!({ "board": { "kind": "board", "by": "status" } });
    let spec = Spec::read(&fields, &views, None).unwrap().steering(Some("keys"));
    collections.ensure("shop", "board", spec, None, &ada()).await.unwrap();
    let before = rig.events("board.item.*").await.len();
    let why = format!(
        "a record of `board` steers the records of `keys`, and is added or changed only by whoever could write \
         each of those as it stands. Of {wide}: a record of `keys` whose `may` says `admin` is changed only by \
         whoever holds `admin`, which narrow does not: nobody gives what they do not hold"
    );
    // Moved to another column, moved within its own, and moved as a key.
    for (column, said) in [("doing", None), ("ready", None), ("done", Some("key:k1"))] {
        let refused = match said {
            Some(said) => board.move_said(&card.id, column, (None, Some("Done.")), &narrow, said).await,
            None => board.move_to(&card.id, column, None, &narrow).await,
        }
        .unwrap_err();
        assert_eq!((refused.kind, &refused.message), (ErrorKind::Denied, &why), "{column}");
    }
    // Nor is one added, or changed in what it says.
    let refused = board.add("shop", "Add a till", "", None, vec![], &narrow).await.unwrap_err();
    assert_eq!(refused.message, why);
    let reworded = studio::Changes { title: Some("Add two carts".into()), ..Default::default() };
    let refused = board.update(&card.id, reworded, &narrow).await.unwrap_err();
    assert_eq!(refused.message, why);
    // Nothing was written or said: it is where it was, called what it was.
    let kept = board.get(&card.id).await.unwrap();
    assert_eq!((kept.status.as_str(), kept.title.as_str()), ("ready", "Add a cart"));
    assert_eq!(rig.events("board.item.*").await.len(), before);
    assert_eq!(board.list("shop").await.unwrap().len(), 1);

    // Somebody who holds what it gives moves it, and it is moved.
    let holds = agent("holds", &["board", "admin"]);
    let moved = board.move_to(&card.id, "doing", None, &holds).await.unwrap();
    assert_eq!(moved.status, "doing");
    let said = rig.events("board.item.moved").await;
    assert_eq!(said.last().unwrap().data["to"], "doing");
    // And taking a card off is not asked it.
    board.remove(&card.id, &narrow).await.unwrap();
}
