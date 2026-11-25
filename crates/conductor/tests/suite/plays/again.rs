//! A step whose action is not safe to do twice, said to be done again
//! from its start (`again: true`), stopped anywhere and started again.

use super::{asked, came, steps};
use crate::anywhere::{Scenario, anywhere};

const DONE_AGAIN: &str = r#"blueprint: done-again
description: A step whose action is not safe to do twice, said to be done again from its start.
steps:
  before: { do: test.mark, with: { key: before } }
  door: { do: test.gate, again: true, with: { name: door } }
  after: { do: test.mark, with: { key: after } }
"#;

// Where the step was under way when the conductor stopped, its action is
// asked again from its start, as a command a writer says may simply be
// run again: the run comes to what it comes to with nothing in its way,
// that action asked once more for the one time it was under way.
#[tokio::test]
async fn a_step_said_to_be_done_again_is_done_from_its_start_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("done-again", DONE_AGAIN).there(asked_again_from_its_start)).await;
    let want = [("after", "ok"), ("before", "ok"), ("door", "ok")];
    assert_eq!(steps(&found, "done-again[done-again]", "main"), came(&want), "{}", found.usual);
    assert_eq!(asked(&found, "after:test.mark"), 1);
    found.held();
    assert!(
        found.noted > 0,
        "done-again: in no play was the step taken up and its action asked again from its start: \
         the conductor was never stopped with it under way, and the play has held nothing of `again`"
    );
}

/// Whether the step `door` was taken up after a stop and its action
/// begun again there: it says it reached its gate after it was taken up.
fn asked_again_from_its_start(kept: &[kernel::Event]) -> bool {
    let of_door = |event: &kernel::Event| event.source.step.as_deref() == Some("door");
    let Some(taken_up) = kept
        .iter()
        .position(|event| event.topic == "step.started" && of_door(event) && event.data["resumed"] == true)
    else {
        return false;
    };
    kept[taken_up..].iter().any(|event| event.topic == "test.reached" && of_door(event))
}
