//! Handlers that go by the clock (on a timer, at a time of day, on what
//! an action answers, on the last of a burst), stopped anywhere and
//! started again.

use serde_json::json;

use super::asked;
use crate::anywhere::{Does, Scenario, anywhere, then};
use crate::judged::by_the_clock;

const TIMED: &str = r#"blueprint: timed
description: Handlers that go by the clock: on a timer, at a time of day, on what an action answers, on the last of a burst.
on:
  struck:
    at: 00:02 UTC
    steps:
      struck: { do: test.mark, with: { key: struck } }
  beat:
    every: 1m
    steps:
      beat: { do: test.mark, with: { key: beats } }
  moved:
    poll: { every: 1m, do: test.echo, with: { v: 1 } }
    steps:
      moved: { do: test.mark, with: { key: moved } }
  settled:
    event: file.changed
    outside: true
    debounce: 30s
    steps:
      settled: { do: test.mark, with: { key: settled } }
"#;

#[tokio::test]
async fn handlers_that_go_by_the_clock_go_on_firing_wherever_the_conductor_is_stopped() {
    let changed = |act: u64| then(Does::Say("file.changed", json!({ "act": act })));
    let timing =
        Scenario::of("timed", TIMED).and(changed(1)).and(changed(2)).over("4m", "30s").judged(by_the_clock);
    let found = anywhere(timing).await;
    assert!(asked(&found, "beat:test.mark") >= 3, "{}", found.usual);
    assert_eq!(asked(&found, "settled:test.mark"), 1, "a burst is one run: {}", found.usual);
    assert_eq!(asked(&found, "struck:test.mark"), 1, "a time of day is fired for once: {}", found.usual);
    found.held();
}
