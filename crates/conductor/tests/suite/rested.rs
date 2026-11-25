//! An instance that has no handlers and holds nothing is active only for
//! what it started, and is done when the last of that has ended: so it
//! ends the same whichever ended first, its steps or what they started.
//! Each order is made, by what the play says when it is at rest, and
//! played wherever the conductor is stopped.

use serde_json::json;

use crate::anywhere::{Does, Scenario, Seen, anywhere, once};

const ENDS_FIRST: &str = r#"blueprint: ends-first
description: What it starts is over before its own steps are.
steps:
  kept: { spawn: brief-child, as: kept }
  on: { needs: [kept], do: wait, with: { event: go.on } }
"#;

const BRIEF_CHILD: &str = r#"blueprint: brief-child
steps:
  one: { do: test.echo, with: { v: 1 } }
"#;

const ENDS_AFTER: &str = r#"blueprint: ends-after
description: What it starts is over only after its own steps are.
steps:
  kept: { spawn: waiting-child, as: kept }
"#;

const WAITING_CHILD: &str = r#"blueprint: waiting-child
steps:
  wait: { do: wait, with: { event: go.on } }
"#;

/// How the instance at `path` stood and what its endings said.
fn ended(found: &crate::anywhere::Found, path: &str) -> (serde_json::Value, serde_json::Value) {
    (found.usual["instances"][path]["status"].clone(), found.usual["ended"][path].clone())
}

#[tokio::test]
async fn a_parent_whose_child_ended_before_its_own_steps_is_done_wherever_the_conductor_is_stopped() {
    // Told to go on only once what it started has said it is over.
    let child_over = |seen: &Seen| seen.said("instance.finished") > 0;
    let scenario = Scenario::of("ends-first", ENDS_FIRST)
        .with(&[BRIEF_CHILD])
        .and(once(child_over, Does::Say("go.on", json!({ "act": "go" }))));
    let found = anywhere(scenario).await;
    let (status, endings) = ended(&found, "ends-first[ends-first]");
    assert_eq!(status, json!("done"), "{}", found.usual);
    assert_eq!(endings.as_array().map(Vec::len), Some(1), "{}", found.usual);
    assert_eq!(endings[0]["status"], json!("done"));
    // It was never active: nothing it started was alive as its steps ended.
    assert_eq!(found.usual["said"]["instance.active"], serde_json::Value::Null, "{}", found.usual);
    found.held();
}

#[tokio::test]
async fn a_parent_whose_child_ends_after_its_own_steps_comes_to_rest_wherever_the_conductor_is_stopped() {
    // Told to end only once the parent is active for it.
    let parent_active = |seen: &Seen| seen.said("instance.active") > 0;
    let scenario = Scenario::of("ends-after", ENDS_AFTER)
        .with(&[WAITING_CHILD])
        .and(once(parent_active, Does::Say("go.on", json!({ "act": "go" }))));
    let found = anywhere(scenario).await;
    let (status, endings) = ended(&found, "ends-after[ends-after]");
    assert_eq!(status, json!("done"), "{}", found.usual);
    assert_eq!(endings.as_array().map(Vec::len), Some(1), "{}", found.usual);
    assert_eq!(endings[0]["status"], json!("done"));
    assert_eq!(found.usual["said"]["instance.active"], json!(1), "{}", found.usual);
    assert_eq!(
        found.usual["instances"]["ends-after[ends-after] > waiting-child[kept]"]["status"],
        json!("done")
    );
    found.held();
}
