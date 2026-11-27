//! A field that says what giving it a value takes may say that this
//! stands (`stands`): while a record says that value, adding the record
//! or changing any cell of it takes it, checked on the record as it is to
//! be written. What the record lets something with more authority do is
//! then not had by changing the cells beside it.

use kernel::{ErrorKind, Principal};
use serde_json::{Map, Value, json};
use studio::collections::Spec;

use crate::{Rig, ada, agent};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// A table of programs, each run behind a wall unless it says `off`, and
/// shown what else it `reads`: what either takes stands, or not.
fn programs(stands: bool) -> Spec {
    let fields = json!({
        "title": { "type": "text", "required": true },
        "told": { "type": "text" },
        "wall": { "type": "choice", "of": ["on", "off"], "takes": { "off": "shell" }, "stands": stands },
        "reads": { "type": "list", "takes": "shell", "stands": stands },
    });
    Spec::read(&fields, &json!(null), None).unwrap()
}

/// Somebody who may write records and declare tables, and runs no command.
fn narrow() -> Principal {
    agent("narrow", &["projects", "records", "blueprints"])
}

fn stands(cell: &str, says: &str) -> String {
    format!(
        "a record of `programs` whose `{cell}` says {says} is changed only by whoever holds `shell`, which narrow \
         does not: what that cell takes stands while it says so"
    )
}

#[tokio::test]
async fn what_a_cell_took_is_asked_of_whoever_changes_any_cell_of_its_record_while_it_says_so() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let (collections, records) = (&rig.studio.collections, &rig.studio.records);
    let made = collections.ensure("shop", "programs", programs(true), None, &ada()).await.unwrap();
    assert!(
        made.fields.iter().filter(|field| field.name == "wall" || field.name == "reads").all(|f| f.stands)
    );
    // Said to whoever reads what the table is, and left unsaid where it does not stand.
    let said = serde_json::to_value(&made).unwrap();
    let field = |name: &str| said["fields"].as_array().unwrap().iter().find(|f| f["name"] == name).cloned();
    assert_eq!(
        (field("wall").unwrap()["stands"].clone(), field("told").unwrap().get("stands").cloned()),
        (json!(true), None)
    );
    let narrow = narrow();

    // One that says nothing that takes `shell` is anybody's who may write records.
    let walled = records.add("shop", "programs", data(json!({ "title": "walled" })), &narrow, None).await;
    let walled = walled.unwrap().id;
    records.update(&walled, data(json!({ "told": "Build it." })), &narrow, None).await.unwrap();

    // Left with no wall by whoever runs commands, any cell of it takes that.
    let open = json!({ "title": "open", "wall": "off", "told": "As Ada says." });
    let open = records.add("shop", "programs", data(open), &ada(), None).await.unwrap().id;
    for cells in [json!({ "told": "As narrow says." }), json!({ "title": "renamed" })] {
        let refused = records.update(&open, data(cells), &narrow, None).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Denied);
        assert_eq!(refused.message, stands("wall", "`off`"));
    }
    // Shown more of the device, the same, whatever it is shown.
    let shown = json!({ "title": "shown", "reads": ["~/.cargo"] });
    let shown = records.add("shop", "programs", data(shown), &ada(), None).await.unwrap().id;
    let refused = records.update(&shown, data(json!({ "told": "Look." })), &narrow, None).await.unwrap_err();
    assert_eq!(refused.message, stands("reads", "anything"));
    // Nor is one added saying so beside what takes nothing.
    let refused = records
        .add("shop", "programs", data(json!({ "title": "more", "wall": "off" })), &narrow, None)
        .await
        .unwrap_err();
    assert_eq!(
        refused.message,
        "giving `wall` of `programs` the value `off` takes `shell`, which narrow does not hold"
    );
    let kept = collections.record(&open).await.unwrap().1;
    assert_eq!(kept.data["told"], "As Ada says.");

    // Set back to what takes nothing in the same write, it is theirs.
    let back = json!({ "wall": "on", "told": "As narrow says." });
    let changed = records.update(&open, data(back), &narrow, None).await.unwrap();
    assert_eq!((&changed.data["wall"], &changed.data["told"]), (&json!("on"), &json!("As narrow says.")));
    // Emptied by whoever may, it says nothing, and the rest is theirs.
    records.update(&shown, data(json!({ "reads": [] })), &ada(), None).await.unwrap();
    records.update(&shown, data(json!({ "told": "Look." })), &narrow, None).await.unwrap();

    // Whoever holds what it took changes any of it; and taking one away
    // has nothing done on anybody's say, and is whoever's may change records.
    let wide = json!({ "title": "wide", "wall": "off", "reads": ["~"] });
    let wide = records.add("shop", "programs", data(wide), &ada(), None).await.unwrap().id;
    let runs = agent("runs", &["projects", "records", "shell"]);
    records.update(&wide, data(json!({ "told": "Go on." })), &runs, None).await.unwrap();
    records.remove(&wide, &narrow, None).await.unwrap();
    assert!(collections.record(&wide).await.is_err());
}

#[tokio::test]
async fn a_cell_that_took_nothing_cannot_stand_and_one_that_stood_is_declared_otherwise_by_whoever_holds_what_it_took()
 {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let collections = &rig.studio.collections;

    // It stands with what it takes, and there is nothing to stand without that.
    let bare = json!({ "title": { "type": "text", "stands": true } });
    let refused = Spec::read(&bare, &json!(null), None).unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid);
    assert!(
        refused.message.starts_with(
            "the field `title` says `stands`, which is for a field that says what giving it a value takes"
        ),
        "{}",
        refused.message
    );
    // A list that gives what it says stands by itself, and says nothing more by it.
    let each = json!({ "may": { "type": "list", "takes": "each", "stands": true } });
    let spec = Spec::read(&each, &json!(null), None).unwrap();
    assert!(spec.fields.iter().all(|field| field.takes_each && !field.stands));

    // Declared again so that it does not stand, by one who does not hold
    // what it took, it is refused, and stands as it did.
    collections.ensure("shop", "programs", programs(true), None, &ada()).await.unwrap();
    let refused = collections.ensure("shop", "programs", programs(false), None, &narrow()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert_eq!(
        refused.message,
        "what `reads` of `programs` takes stands while a record says it, and declaring `programs` again so that \
         it does not takes holding `shell`, which narrow does not hold"
    );
    let kept = collections.get("shop", "programs").await.unwrap();
    assert!(kept.fields.iter().find(|field| field.name == "wall").unwrap().stands);
    // Said again as it is, it is theirs to declare; and made to stand, as
    // asking more is.
    collections.ensure("shop", "programs", programs(true), None, &narrow()).await.unwrap();
    collections.ensure("shop", "loose", programs(false), None, &narrow()).await.unwrap();
    collections.ensure("shop", "loose", programs(true), None, &narrow()).await.unwrap();
    // Whoever holds what it took declares it so that it does not.
    let runs = agent("runs", &["projects", "records", "blueprints", "shell"]);
    let again = collections.ensure("shop", "programs", programs(false), None, &runs).await.unwrap();
    assert!(again.fields.iter().all(|field| !field.stands));
}
