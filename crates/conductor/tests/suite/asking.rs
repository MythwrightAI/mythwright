//! The run a step's call began is found by what the event that set the
//! step's run off named its asking (`kernel::asking`,
//! `Engine::begun_for`), whichever came to a child that was written down
//! and not yet begun when the conductor stopped: its start, or the step
//! that asked for it, taken up (`crates/conductor/src/child.rs`, `had`).
//! Here the start comes first, as it does where nothing is made between
//! the two (`begun.rs`): the run it begins is kept as asked for by the
//! step once that is taken up. Where the step comes first, it begins the
//! run as asked for, which the API suite's `pipelines.rs` holds through
//! the daemon (`the_run_an_asking_began_is_found_where_the_daemon_stopped…`).

use kernel::Source;
use serde_json::json;

use crate::Rig;

const DOUBLES: &str = r#"blueprint: doubles
params: { n: { type: int, default: 1 } }
steps:
  double: { do: test.echo, with: { v: ${ params.n * 2 } } }
"#;

/// Calls what doubles as it is asked, under a name of its own.
const ASKER: &str = r#"blueprint: asker
steps:
  ready: { do: test.mark, with: { key: ready } }
on:
  go:
    event: asking.go
    outside: true
    steps:
      ran: { call: doubles, as: the-called, with: { n: 21 } }
"#;

/// Asks `source` for its child under the name `one`, stops the conductor
/// once the child is written down and before it is begun, starts it
/// again, and answers the run the asking began with who began the child.
async fn stopped_between(source: &str) -> (Option<String>, String, Vec<String>) {
    let mut rig = Rig::on_a_file().await;
    rig.library.put("doubles", DOUBLES);
    let applied = rig.apply(source, json!({})).await;
    rig.finished(&applied).await;
    rig.journal.shut_after(|event| event.topic == "instance.started" && event.data["blueprint"] == "doubles");
    rig.journal.emit("asking.go", Source::default(), json!({ "asked": "one" })).await.unwrap();
    rig.soon("the journal to shut once the child is written down", rig.journal.shut()).await;
    rig.stopped_there().await;
    let ended = rig.event("run.finished", |event| event.data["blueprint"] == "doubles").await;
    let ran = ended.source.run.clone().expect("a run");
    // And the step that asked, taken up, has what it asked for.
    rig.event("run.finished", |event| event.data["flow"] == "on.go").await;
    let begun = rig.engine.begun_for(&applied.instance, "one").await.unwrap();
    let query = journal::MetricQuery {
        name: "instance.begun".into(),
        group_by: vec!["blueprint".into(), "by".into()],
        ..Default::default()
    };
    let by: Vec<String> = rig
        .journal
        .query_metrics(query)
        .await
        .unwrap_or_default()
        .iter()
        .filter(|series| series.labels.get("blueprint").map(String::as_str) == Some("doubles"))
        .filter_map(|series| series.labels.get("by").cloned())
        .collect();
    (begun, ran, by)
}

#[tokio::test]
async fn the_run_an_asking_began_is_found_where_the_start_came_to_the_child_first() {
    let (begun, ran, by) = stopped_between(ASKER).await;
    assert_eq!(by, ["start"], "the order this is there for: the start first");
    assert_eq!(begun, Some(ran));
}
