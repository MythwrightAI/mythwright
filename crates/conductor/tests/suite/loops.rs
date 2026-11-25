//! A `for_each` that was under way when the conductor stopped goes on from
//! the element it was at. The elements before it are not done again, the
//! one it was at is picked up as any step is, and the list is the one the
//! loop began with, whatever it would come to now.

use conductor::{RunStatus, StepStatus};
use serde_json::json;

use crate::{Rig, step};

/// Waits until the gate has been reached `times` times.
async fn reached(rig: &Rig, gate: &str, times: usize) {
    let mut follow = rig.follow("test.reached");
    let mut seen = 0;
    rig.soon(&format!("`{gate}` to be reached {times} time(s)"), async {
        while seen < times {
            if follow.next().await.unwrap().data["gate"] == gate {
                seen += 1;
            }
        }
    })
    .await;
}

#[tokio::test]
async fn after_a_restart_a_loop_goes_on_from_the_element_it_was_at() {
    let mut rig = Rig::new().await;
    let source = "\
blueprint: looped
params:
  names: { type: list, default: [one, two, three] }
steps:
  each:
    for_each: ${ params.names }
    do: test.durable
    with: { name: ${ item } }
  after: { do: test.count, with: { key: after } }
outputs:
  gave: ${ steps.each.out }
";
    // The first two are let through; the third waits.
    rig.open("one");
    rig.open("two");
    let applied = rig.apply(source, json!({})).await;
    reached(&rig, "three", 1).await;
    assert_eq!(rig.shared.count("durable:run"), 3);

    rig.restart().await;
    rig.open("three");
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        (rig.shared.count("durable:run"), rig.shared.count("durable:resume")),
        (3, 1),
        "the two that were done were not done again, and the third was picked up where it was"
    );
    let gave =
        json!([{ "resumed_from": null }, { "resumed_from": null }, { "resumed_from": { "pid": 4242 } }]);
    assert_eq!(run.outputs, json!({ "gave": gave }), "and what each gave is all there, in order");
    assert_eq!(rig.shared.count("after"), 1);
}

#[tokio::test]
async fn after_a_restart_an_element_that_cannot_be_picked_up_fails_the_loop_where_it_was() {
    let mut rig = Rig::new().await;
    let source = "\
blueprint: fragile
params:
  names: { type: list, default: [one, two] }
steps:
  each:
    for_each: ${ params.names }
    do: test.gate
    with: { name: ${ item } }
";
    rig.open("one");
    let applied = rig.apply(source, json!({})).await;
    reached(&rig, "two", 1).await;

    rig.restart().await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Failed);
    let each = step(&run, "each");
    assert_eq!(each.status, StepStatus::Failed);
    assert_eq!(
        each.error.as_deref(),
        Some("was interrupted by a restart, and this action is not safe to repeat automatically")
    );
    assert_eq!(
        rig.shared.calls(),
        ["each:test.gate", "each:test.gate"],
        "the one that was done was not done again"
    );
}

#[tokio::test]
async fn after_a_restart_a_loop_is_over_the_list_it_began_with_and_a_task_goes_on_from_its_step() {
    let mut rig = Rig::new().await;
    // Each element is a task, whose first step empties the list the loop
    // was worked out from.
    let source = "\
blueprint: queue
state: { waiting: [one, two] }
steps:
  each:
    for_each: ${ state.waiting }
    task:
      take: { do: set, with: { waiting: [] } }
      mark: { do: test.count, with: { key: \"marked-${ item }\" } }
      work: { do: test.durable, with: { name: ${ item } } }
      done: { do: test.count, with: { key: \"done-${ item }\" } }
";
    rig.open("one");
    let applied = rig.apply(source, json!({})).await;
    reached(&rig, "two", 1).await;

    rig.restart().await;
    rig.open("two");
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let counts = ["marked-one", "done-one", "marked-two", "done-two"].map(|key| rig.shared.count(key));
    assert_eq!(counts, [1, 1, 1, 1], "each element's steps were done once, the second's across the restart");
    assert_eq!((rig.shared.count("durable:run"), rig.shared.count("durable:resume")), (2, 1));
    assert_eq!(step(&run, "each").output.as_array().map(Vec::len), Some(2));
}

#[tokio::test]
async fn after_a_restart_a_loop_of_calls_does_not_call_again_the_children_that_had_finished() {
    let mut rig = Rig::new().await;
    rig.library.put(
        "stage",
        "\
blueprint: stage
params:
  name: { type: string }
steps:
  begin: { do: test.count, with: { key: \"begun-${ params.name }\" } }
  build: { do: test.durable, with: { name: ${ params.name } } }
outputs:
  name: ${ params.name }
",
    );
    let source = "\
blueprint: rollout
params:
  names: { type: list, default: [one, two, three] }
steps:
  shift:
    for_each: ${ params.names }
    call: stage
    with: { name: ${ item } }
outputs:
  shifted: ${ steps.shift.out }
";
    rig.open("one");
    rig.open("two");
    let applied = rig.apply(source, json!({})).await;
    reached(&rig, "three", 1).await;

    rig.restart().await;
    rig.open("three");
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let begun = ["begun-one", "begun-two", "begun-three"].map(|key| rig.shared.count(key));
    assert_eq!(begun, [1, 1, 1], "a stage that had finished was not run again");
    let names: Vec<&str> = run.outputs["shifted"]
        .as_array()
        .unwrap()
        .iter()
        .map(|gave| gave["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["one", "two", "three"]);
}

#[tokio::test]
async fn a_loop_that_is_tried_again_starts_from_its_first_element() {
    let rig = Rig::new().await;
    // The second element fails the first time it is come to.
    let source = "\
blueprint: retried
params:
  times: { type: list, default: [0, 2] }
steps:
  each:
    for_each: ${ params.times }
    do: test.flaky
    with: { times: ${ item } }
    retry: { max: 1, backoff: 10s }
";
    let since = rig.clock.sleep_count();
    let applied = rig.apply(source, json!({})).await;
    rig.advance_after(since, 1, "10s").await;
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        step(&run, "each").output,
        json!([{ "calls": 3 }, { "calls": 4 }]),
        "what the first element gave is from the second time round, not kept from the first"
    );
}

#[tokio::test]
async fn after_a_restart_an_element_the_loop_had_not_begun_begins_and_is_not_taken_for_interrupted() {
    let mut rig = Rig::new().await;
    let source = "\
blueprint: paced
params:
  names: { type: list, default: [one, two] }
steps:
  each:
    for_each: ${ params.names }
    do: test.gate
    with: { name: ${ item } }
";
    rig.open("one");
    let applied = rig.apply(source, json!({})).await;
    reached(&rig, "two", 1).await;
    rig.engine.halt();
    // As it would be had the conductor stopped a moment sooner, between
    // the first element and the second: nothing says the second has begun.
    rig.journal
        .db()
        .call(|connection| connection.execute("DELETE FROM step_elements WHERE position = 1", []))
        .await
        .unwrap();

    rig.restart().await;
    reached(&rig, "two", 2).await;
    rig.open("two");
    let run = rig.finished(&applied).await;

    // An action that is not safe to repeat is refused when it was
    // interrupted. This one was never begun, so it is simply done.
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(step(&run, "each").output, json!([{ "passed": "one" }, { "passed": "two" }]));
}
