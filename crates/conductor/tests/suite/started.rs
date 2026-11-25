//! What is done as a step starts, where the step is done again.
//!
//! A step with things done as it starts (`on_start`) is under way from
//! the first of them, and its own action begins only when the last is
//! done. A conductor stopped among them has a step that is written as
//! under way whose action never began; and one stopped after the action
//! has said what it says has events of the step's in the journal that
//! are not all the action's. Both were read wrongly when the step was
//! taken up: an action that cannot be done twice was refused as done
//! again with nothing done, and an `emit` beside one said as its step
//! starts was said a second time, since it could not tell its own event
//! from the entry's.
//!
//! Each is played here stopped at every place. Nothing in either goes by
//! which of two things comes first: there is one run, and its steps come
//! one after the other.

use serde_json::json;

use crate::anywhere::{Does, Found, Scenario, anywhere, then};
use crate::judged::opened_once;

fn asked(found: &Found, what: &str) -> u64 {
    found.usual["asked"][what].as_u64().unwrap_or(0)
}

fn said(found: &Found, topic: &str) -> u64 {
    found.usual["said"][topic].as_u64().unwrap_or(0)
}

/// A step that cannot be done twice, with things done as it starts that
/// take a while and are counted, two of them saying something: so that
/// the conductor is stopped before the first of them, among them, and
/// after the step's own action has been asked.
const OPENS: &str = r#"blueprint: opens
description: A step whose action cannot be done twice, with things done as it starts that take a while.
steps:
  before: { do: test.mark, with: { key: before } }
  door:
    do: test.gate
    with: { name: door }
    on_start:
      - emit: { topic: opens.starting, at: first }
      - do: test.sleep
        with: { for: 1m }
      - do: test.mark
        with: { key: door-starting }
      - emit: { topic: opens.starting, at: last }
  after: { do: test.mark, with: { key: after } }
"#;

#[tokio::test]
async fn a_step_stopped_in_what_is_done_as_it_starts_is_not_refused_as_done_again() {
    let found = anywhere(Scenario::of("opens", OPENS).over("3m", "30s").judged(opened_once)).await;
    assert_eq!(
        (asked(&found, "door:test.gate"), asked(&found, "after:test.mark"), said(&found, "opens.starting")),
        (1, 1, 2),
        "{}",
        found.usual
    );
    assert_eq!(asked(&found, "door:test.mark (on_start 2)"), 1, "{}", found.usual);
    found.held();
}

/// A step that says an event, with the same event said as it starts and
/// as it has gone well, and a handler that is asked once for each: which
/// of them it heard is in what its step gave. The steps are a handler's,
/// so that what they say is said by something that is listening.
const HERALDS: &str = r#"blueprint: heralds
description: A step that says an event, with the same event said as it starts and when it has gone well.
on:
  go:
    event: heralds.go
    outside: true
    steps:
      tell:
        do: emit
        with: { topic: heralds.told, by: step }
        on_start:
          - emit: { topic: heralds.told, by: start }
          - do: test.mark
            with: { key: tell-starting }
        on_success:
          - emit: { topic: heralds.told, by: end }
      each:
        for_each: ${ ["a", "b"] }
        do: emit
        with: { topic: heralds.each, of: "${ item }" }
        on_start:
          - emit: { topic: heralds.each, of: start }
      after: { do: test.mark, with: { key: after } }
  heard:
    event: heralds.told
    scope: own
    steps:
      which: { do: test.echo, with: { by: "${ event.data.by }" } }
"#;

#[tokio::test]
async fn an_event_a_step_says_beside_one_said_as_it_starts_is_said_once_wherever_the_conductor_is_stopped() {
    let heralding = Scenario::of("heralds", HERALDS).and(then(Does::Say("heralds.go", json!({ "act": 1 }))));
    let found = anywhere(heralding).await;
    assert_eq!((said(&found, "heralds.told"), said(&found, "heralds.each")), (3, 3), "{}", found.usual);
    assert_eq!(asked(&found, "which:test.echo"), 3, "{}", found.usual);
    let mut heard: Vec<String> = found.usual["runs"]["heralds[heralds]"]["on.heard"]
        .as_array()
        .unwrap()
        .iter()
        .map(|run| run["steps"]["which"]["gave"]["by"].as_str().unwrap_or_default().to_string())
        .collect();
    heard.sort();
    assert_eq!(heard, ["end", "start", "step"], "{}", found.usual);
    found.held();
}
