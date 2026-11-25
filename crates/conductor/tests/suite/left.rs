//! A step whose action is the same done twice has its ending held, and
//! written with the next thing its run writes; its beginning is written as
//! every step's is ([`conductor`]'s `RunCx::hush`). Such an action may
//! leave something behind (a message, a row, an event), and is the same
//! done twice only because, told that it is being done again, it looks
//! for what it left. Stopped while its ending is held, its record says it
//! was under way, so it is told: stopped anywhere, a run of such steps
//! comes to the same as one never stopped, and nothing is left twice.

use std::collections::BTreeMap;
use std::sync::Arc;

use conductor::Registry;
use journal::EventQuery;
use serde_json::{Value, json};

use crate::anywhere::{Does, Place, Scenario, anywhere, differs, then};
use crate::{Shared, action};

/// What `test.leaves` says it left: an event the plays count as they
/// count every event a blueprint has said, so one left twice is a play
/// that does not come to the same.
pub const LEFT: &str = "thing.left";

/// `test.leaves`: leaves an event that says what it is and which doing of
/// its step left it, once. Told it is being done again, it looks for what
/// it left first, as `channel.post` looks for its message; told nothing,
/// it leaves another. So it is the same done twice only where whoever
/// does it again says so.
pub fn register(registry: &mut Registry, shared: &Arc<Shared>) {
    action(
        registry,
        blueprint::ActionSpec::new(
            "test.leaves",
            "Leave something behind, once: told it is being done again, look for what was left first.",
        )
        .required("what", blueprint::ValueType::String, "what is left")
        .idempotent(),
        shared,
        |cx, args, _| async move {
            let what = args["what"].clone();
            let by = json!({ "run": cx.run(), "step": cx.step(), "doing": cx.doing() });
            if cx.again() {
                let query = EventQuery { topics: LEFT.into(), limit: 100_000, ..Default::default() };
                let said = cx.engine().journal().read(query).await?;
                if said.iter().any(|event| event.data["by"] == by) {
                    return Ok(json!({ "left": what }));
                }
            }
            // Kept in the factory's journal, which names no project, and
            // looked for there: one place whichever conductor runs it.
            cx.engine().journal().emit(LEFT, cx.source(), json!({ "what": what, "by": by })).await?;
            Ok(json!({ "left": what }))
        },
    );
}

/// What a play comes to, held as every play is (what was said, the
/// things left among it, to the one), but for how often an action that
/// only decides was asked: stopped while a step of it is held, nothing of
/// it is written and it is done again from its start, once for each such
/// stop (`decides.rs`).
fn left(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    let only = |of: &Value| {
        let mut of = of.clone();
        if let Some(asked) = of["asked"].as_object_mut() {
            asked.retain(|key, _| !key.ends_with(":test.decides"));
        }
        of
    };
    differs(&only(usual), &only(came), again)
}

const LEAVES: &str = r#"blueprint: leaves
description: Things left behind among steps that decide and one passed over.
steps:
  what: { do: test.decides, with: { said: hello } }
  says: { needs: [what], do: test.leaves, with: { what: "${ steps.what.out.said }" } }
  never: { needs: [says], when: ${ steps.says.out.left == "twice" }, do: test.leaves, with: { what: never } }
  more: { needs: [never], do: test.leaves, with: { what: more } }
  last: { needs: [more], do: test.decides, with: { left: "${ steps.more.out.left }" } }
outputs:
  left: ${ steps.last.out.left }
"#;

#[tokio::test]
async fn what_a_step_that_is_the_same_done_twice_leaves_behind_is_left_once_wherever_the_conductor_is_stopped()
 {
    let found = anywhere(Scenario::of("leaves", LEAVES).judged(left)).await;
    let run = &found.usual["runs"]["leaves[leaves]"]["main"][0];
    assert_eq!(run["outputs"], json!({ "left": "more" }));
    let said: Vec<_> = ["what", "says", "never", "more", "last"]
        .iter()
        .map(|step| run["steps"][*step]["status"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(said, ["ok", "ok", "skipped", "ok", "ok"]);
    assert_eq!(found.usual["said"][LEFT], 2, "{}", found.usual);
    found.held();
}

const BESIDE: &str = r#"blueprint: left-beside
description: A step that leaves something beside one that acts, and one after both.
max_parallel: 2
steps:
  first: { do: test.leaves, with: { what: first } }
  leaves: { needs: [first], do: test.leaves, with: { what: beside } }
  acts: { needs: [first], do: test.mark, with: { key: acts } }
  last: { needs: [leaves, acts], do: test.leaves, with: { what: "${ steps.leaves.out.left }" } }
"#;

#[tokio::test]
async fn a_step_that_leaves_something_beside_one_that_acts_leaves_it_once_wherever_the_conductor_is_stopped()
{
    let found = anywhere(Scenario::of("left-beside", BESIDE).judged(left)).await;
    let run = &found.usual["runs"]["left-beside[left-beside]"]["main"][0];
    assert_eq!(run["steps"]["last"]["status"], "ok");
    assert_eq!(found.usual["said"][LEFT], 3, "{}", found.usual);
    found.held();
}

const HEARD: &str = r#"blueprint: left-heard
description: A handler that leaves something for each thing it hears, and decides between.
on:
  heard:
    event: thing.happened
    outside: true
    steps:
      what: { do: test.decides, with: { said: "heard ${ event.data.n }" } }
      says: { needs: [what], do: test.leaves, with: { what: "${ steps.what.out.said }" } }
      then: { needs: [says], do: test.leaves, with: { what: then } }
"#;

#[tokio::test]
async fn a_handler_that_leaves_something_for_each_thing_it_hears_leaves_it_once_wherever_the_conductor_is_stopped()
 {
    let happened = |act: u64| then(Does::Say("thing.happened", json!({ "act": act, "n": act })));
    let found =
        anywhere(Scenario::of("left-heard", HEARD).judged(left).and(happened(1)).and(happened(2))).await;
    assert_eq!(found.usual["said"][LEFT], 4, "{}", found.usual);
    found.held();
}
