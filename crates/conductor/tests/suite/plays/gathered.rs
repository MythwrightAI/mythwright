//! What was started side by side, waited for together: a `wait` for
//! what an instance started under several names to have ended, stopped
//! anywhere and started again.

use serde_json::json;

use crate::anywhere::{Scenario, anywhere};
use crate::judged::rested;

const GATHERS: &str = r#"blueprint: gathers
description: Three started side by side and waited for together, one of them over before the wait begins.
steps:
  one: { spawn: slow, as: one }
  two: { spawn: slower, as: two }
  three: { spawn: instant, as: three }
  all: { needs: [one, two, three], do: wait, with: { ended: [one, two, three, never] } }
  after: { needs: [all], do: test.mark, with: { key: after-all } }
outputs:
  ended: ${ steps.all.out.ended }
  gave: ${ steps.all.out.gave }
"#;

const SLOW: &str = r#"blueprint: slow
steps:
  nap: { do: test.sleep, with: { for: 1m } }
  done: { do: test.mark, with: { key: slow-done } }
"#;

const SLOWER: &str = r#"blueprint: slower
steps:
  nap: { do: test.sleep, with: { for: 3m } }
  done: { do: test.mark, with: { key: slower-done } }
"#;

const INSTANT: &str = r#"blueprint: instant
steps:
  one: { do: test.echo, with: { v: 1 } }
outputs:
  v: ${ steps.one.out.v }
"#;

#[tokio::test]
async fn what_was_started_side_by_side_is_waited_for_together_once_wherever_the_conductor_is_stopped() {
    let gathers =
        Scenario::of("gathers", GATHERS).with(&[SLOW, SLOWER, INSTANT]).over("10m", "1m").judged(rested);
    let found = anywhere(gathers).await;
    let run = &found.usual["runs"]["gathers[gathers]"]["main"][0];
    // Each as it ended, and a name nothing was started under as nothing: none of them is alive.
    assert_eq!(
        run["outputs"]["ended"],
        json!({ "one": "done", "two": "done", "three": "done", "never": null }),
        "{}",
        found.usual
    );
    // What each gave out, in the order they were named: the one that says nothing gave nothing.
    assert_eq!(run["outputs"]["gave"], json!([{}, {}, { "v": 1 }, null]), "{}", found.usual);
    // What came after the wait was done once, after the slowest had ended.
    assert_eq!(found.usual["asked"]["after:test.mark"].as_u64(), Some(1), "{}", found.usual);
    found.held();
}
