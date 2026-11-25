//! What a blueprint's step says of a record for itself (an `emit`, which
//! no action that changes a record says), in that record's history: the
//! factory's doing, with the blueprint after it, found by the instance
//! that said it (`studio::through`). Here no engine is attached to the
//! studio, as in a rig that runs blueprints and never says so: the
//! instance is found by its row, and nothing waits for an engine.

use std::sync::Arc;

use conductor::{Apply, Engine, MemoryLibrary, Registry};
use journal::Logs;
use serde_json::{Value, json};
use studio::collections::Spec;

use crate::{Rig, ada};

const NOTES: &str = "\
blueprint: notes
steps:
  note:
    do: emit
    with: { topic: sprint.noted, record: shop-sprints-1, project: shop }
";

#[tokio::test]
async fn what_a_step_says_of_a_record_is_kept_in_its_history_as_the_bots_with_no_engine_attached() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let fields = json!({ "title": { "type": "text", "required": true } });
    let spec = Spec::read(&fields, &json!(null), None)
        .unwrap()
        .with_history(&json!({ "sprint.noted": "it was noted", "record.*": "" }))
        .unwrap();
    rig.studio.collections.ensure("shop", "sprints", spec, None, &ada()).await.unwrap();
    let mut said = serde_json::Map::new();
    said.insert("title".into(), json!("November"));
    rig.studio.records.add("shop", "sprints", said, &ada(), None).await.unwrap();

    let logs = Logs::new(rig.dir.join("logs"));
    let library = Arc::new(MemoryLibrary::new());
    let engine = Engine::new(rig.journal.clone(), logs, Registry::new(), library).await.unwrap();
    let apply =
        Apply { source: Some(NOTES.into()), project: Some("shop".into()), ..Apply::new("notes", ada()) };
    let applied = engine.apply(apply).await.unwrap();
    engine.wait_run(applied.run.as_ref().unwrap()).await.unwrap();

    let mut written = rig.journal.follow(0, studio::history::TOPIC);
    let line = loop {
        let lines = rig.studio.collections.history("shop-sprints-1").await.unwrap();
        if let Some(line) = lines.into_iter().find(|line| line.topic == "sprint.noted") {
            break line;
        }
        written.next().await.unwrap();
    };
    assert_eq!(line.data["through"], Value::from("notes"), "{line:?}");
    assert_eq!(line.who, talk::speaker::DEFAULT, "said as the factory's bot, and not as Ada: {line:?}");
    assert_eq!(line.said, "it was noted");
    // Read for want of it in hand, which goes by whether the instance had
    // been let go of as it was asked, it is measured as such a reading is
    // (`conductor::LOOKED_UP`), and what one card cost counts it apart
    // (`costed.rs` in the API suite).
    rig.journal.flush_metrics().await.unwrap();
    let query = journal::MetricQuery {
        name: conductor::LOOKED_UP.into(),
        labels: vec![("kind".into(), "blueprint".into())],
        ..Default::default()
    };
    let read: u64 =
        rig.journal.query_metrics(query).await.unwrap().iter().map(|series| series.total.count).sum();
    assert!(read >= 1, "the reading of the instance's row was not measured");
    engine.halt();
}
