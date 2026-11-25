//! A step whose action only decides has no write of its own: its records
//! are held as it begins and as it ends, and written with the next thing
//! its run writes. Stopped anywhere, a run of such steps comes to the
//! same as one never stopped, and what acts on what one gave is done once.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::anywhere::{Place, Scenario, anywhere, differs};

/// What a play comes to, held as every play is, but for how often an
/// action that only decides was asked, which is by design: stopped while
/// a step of it is held, nothing of the step is written, and it is done
/// again from its start, once for each such stop. That says nothing and
/// does nothing, and is no part of what a run came to; it is never asked
/// less than with nothing in the way.
fn decided(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    let mut found = Vec::new();
    let only = |of: &Value| {
        let mut of = of.clone();
        if let Some(asked) = of["asked"].as_object_mut() {
            asked.retain(|key, _| !key.ends_with(":test.decides"));
        }
        of
    };
    for (key, was) in usual["asked"].as_object().into_iter().flatten() {
        let is = came["asked"][key].as_u64().unwrap_or(0);
        if key.ends_with(":test.decides") && is < was.as_u64().unwrap_or(0) {
            found.push(format!("`{key}` was asked {is} times, and {was} with nothing in its way"));
        }
    }
    found.extend(differs(&only(usual), &only(came), again));
    found
}

const DECIDED: &str = r#"blueprint: decided
description: What is worked out, acted on, passed over and worked out again.
steps:
  what: { do: test.decides, with: { said: once } }
  acts: { needs: [what], do: test.mark, with: { key: "${ steps.what.out.said }" } }
  never: { needs: [acts], when: ${ steps.what.out.said == "twice" }, do: test.decides, with: { said: never } }
  last: { needs: [never], do: test.decides, with: { marked: "${ steps.acts.out.marked }" } }
outputs:
  marked: ${ steps.last.out.marked }
"#;

#[tokio::test]
async fn steps_that_only_decide_come_to_the_same_and_what_acts_on_them_is_done_once_wherever_the_conductor_is_stopped()
 {
    let found = anywhere(Scenario::of("decided", DECIDED).judged(decided)).await;
    let run = &found.usual["runs"]["decided[decided]"]["main"][0];
    assert_eq!(run["outputs"], json!({ "marked": "once" }));
    let said: Vec<_> = ["what", "acts", "never", "last"]
        .iter()
        .map(|step| run["steps"][*step]["status"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(said, ["ok", "ok", "skipped", "ok"]);
    found.held();
}

const BESIDE: &str = r#"blueprint: beside
description: A step that only decides beside one that acts.
max_parallel: 2
steps:
  first: { do: test.mark, with: { key: first } }
  decides: { needs: [first], do: test.decides, with: { said: beside } }
  acts: { needs: [first], do: test.mark, with: { key: acts } }
  last: { needs: [decides, acts], do: test.mark, with: { key: "${ steps.decides.out.said }" } }
"#;

#[tokio::test]
async fn a_step_that_only_decides_beside_one_that_acts_comes_to_the_same_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("beside", BESIDE).judged(decided)).await;
    let run = &found.usual["runs"]["beside[beside]"]["main"][0];
    assert_eq!(run["steps"]["last"]["status"], "ok");
    found.held();
}
