//! What is done around a step and as a run ends, for something that was
//! taken up after a restart: an entry is told it is being done again only
//! where it had begun, and a run is taken up whether or not it has steps.
//! And what is done around a step keeps no place in the step's own record:
//! that is where its action's place is kept, and what is found there when
//! the step is taken up is taken for the action's.

use conductor::RunStatus;
use serde_json::json;

use crate::Rig;

const LATE: &str = "\
blueprint: late
steps:
  build: { do: test.durable, with: { name: build } }
on_success:
  - { do: test.told, with: { key: ended } }
";

#[tokio::test]
async fn what_a_run_does_as_it_ends_is_not_told_it_is_done_again_because_the_run_was_once_taken_up() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(LATE, json!({})).await;
    // Stopped in its step, long before anything is done at its ending.
    rig.reached("build").await;
    rig.restart().await;
    rig.open("build");
    rig.finished(&applied).await;
    assert_eq!(rig.shared.count("ended"), 1);
    assert_eq!(rig.shared.count("again:ended"), 0, "it had not begun when the conductor stopped");
}

const BARE: &str = "\
blueprint: bare
on:
  poke: { event: bare.poke, do: noop }
on_success:
  - { do: emit, with: { topic: bare.said } }
  - { do: test.told, with: { key: second } }
  - { do: test.gate, with: { name: hold } }
";

#[tokio::test]
async fn a_run_with_no_steps_stopped_among_what_it_does_as_it_ends_goes_on_from_where_it_was() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(BARE, json!({})).await;
    // The first two are done and kept; it waits in the third.
    rig.reached("hold").await;
    rig.restart().await;
    rig.open("hold");
    rig.event("run.finished", |event| event.source.run.as_deref() == Some(applied.run.as_deref().unwrap()))
        .await;
    assert_eq!(rig.events("bare.said").await.len(), 1, "what it had said it did not say again");
    assert_eq!(rig.shared.count("second"), 1, "and what it had done it did not do again");
    let began = rig.events("run.started").await;
    let began = began.iter().filter(|event| event.source.run == applied.run).count();
    assert_eq!(began, 1, "a run that is taken up is not said to have started a second time");
}

const BEATS: &str = "\
blueprint: beats
state:
  beats: 0
steps:
  long:
    do: test.durable
    with: { name: long }
    watch:
      - every: 1m
        do: set
        with: { beats: \"${ state.beats + 1 }\" }
  after: { do: test.count, with: { key: after-long }, needs: [long] }
";

#[tokio::test]
async fn something_set_by_a_rule_that_watches_a_step_does_not_say_the_step_has_acted() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(BEATS, json!({})).await;
    rig.reached("long").await;
    // Its rule fires once, and sets something.
    rig.advance_after(0, 1, "1m").await;
    rig.event("instance.state", |event| event.data["beats"] == 1).await;
    rig.restart().await;
    // Taken up, the step is still to be done: it waits where it was.
    rig.open("long");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(rig.shared.count("durable:resume"), 1, "its action was taken up, and not passed over");
    let gave = &crate::step(&run, "long").output;
    assert!(
        gave.get("resumed_from").is_some(),
        "it gave what its action gives, and not what the rule set: {gave}"
    );
}

#[tokio::test]
async fn an_action_is_told_whether_it_is_a_steps_own_or_something_done_around_it() {
    let rig = Rig::new().await;
    let source = "\
blueprint: both
steps:
  work:
    do: test.told
    with: { key: own }
    on_success:
      - { do: test.told, with: { key: after } }
";
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Ok);
    // Both are done under the step's name: an action that leaves something
    // behind, and looks for it when done again, tells the two apart by this.
    assert_eq!((rig.shared.count("own"), rig.shared.count("around:own")), (1, 0));
    assert_eq!((rig.shared.count("after"), rig.shared.count("around:after")), (1, 1));
}

/// Several things are done under one step's name, and one that leaves
/// something behind and looks for it when it is done again goes by which
/// of them it is. Each of these pairs was one and the same to it: a post
/// done again found the other's message and said nothing.
#[tokio::test]
async fn each_thing_done_under_a_steps_name_is_told_apart_by_its_try_its_list_and_its_element() {
    let rig = Rig::new().await;
    let apart = |key: &str| {
        let noted = rig.shared.noted(key);
        let mut once = noted.clone();
        once.sort();
        once.dedup();
        assert!(noted.len() >= 2 && once.len() == noted.len(), "`{key}` was done as {noted:?}");
    };
    // What is done as a step starts, at its first try and at its second.
    let tried = "\
blueprint: tried
steps:
  work:
    do: test.until
    with: { key: tried, times: 2 }
    retry: { max: 3, backoff: 0s }
    on_start:
      - { do: test.which, with: { key: starting } }
";
    assert_eq!(rig.run(tried, json!({})).await.status, RunStatus::Ok);
    apart("starting");
    // The first thing done when it fails, and the first done when it then
    // goes well: each is the first of its list.
    let mended = "\
blueprint: mended
steps:
  work:
    do: test.until
    with: { key: mended, times: 2 }
    on_fail:
      - { do: test.which, with: { key: ending } }
      - { do: retry }
    on_success:
      - { do: test.which, with: { key: ending } }
";
    assert_eq!(rig.run(mended, json!({})).await.status, RunStatus::Ok);
    apart("ending");
    // A step inside steps done as one for each of a list: its own action,
    // and what is done as it ends, for the first of the list and the next.
    let looped = "\
blueprint: looped
steps:
  all:
    for_each: ${ [1, 2] }
    task:
      one:
        do: test.which
        with: { key: inside }
        on_success:
          - { do: test.which, with: { key: inside-ending } }
";
    assert_eq!(rig.run(looped, json!({})).await.status, RunStatus::Ok);
    apart("inside");
    apart("inside-ending");
    // And steps done as one that are tried again: the step inside is at
    // its first try both times.
    let again = "\
blueprint: again
steps:
  all:
    retry: { max: 2, backoff: 0s }
    task:
      one: { do: test.which, with: { key: tried-inside } }
      two: { do: test.until, with: { key: again, times: 2 } }
";
    assert_eq!(rig.run(again, json!({})).await.status, RunStatus::Ok);
    apart("tried-inside");
}

const WAITED: &str = "\
blueprint: waited
steps:
  door:
    do: test.gate
    with: { name: door }
    on_start:
      - { do: wait, with: { for: 30s } }
  after: { do: test.count, with: { key: after-door }, needs: [door] }
";

/// A `wait` done as a step starts kept when it would wake in the step's
/// own record. A conductor stopped while the step's action was under way
/// then found a place kept there, took it for the action's, and had the
/// action go on from it: one that is not safe to do twice was done twice,
/// and told nothing.
#[tokio::test]
async fn a_wait_done_as_a_step_starts_keeps_no_place_in_the_step_for_its_action_to_be_taken_up_from() {
    let mut rig = Rig::new().await;
    let slept = rig.clock.sleep_count();
    let applied = rig.apply(WAITED, json!({})).await;
    // What is done as it starts waits and is over, and its action is under way.
    rig.advance_after(slept, 1, "30s").await;
    rig.reached("door").await;
    let run = rig.engine.run(applied.run.as_deref().unwrap()).await.unwrap();
    let kept = &crate::step(&run, "door").saved;
    assert!(kept.is_null(), "what was done as the step started kept {kept} in the step's own record");
    rig.restart().await;
    // Taken up with nothing of its own kept, it is a step whose action
    // cannot be done twice, and fails saying so, as it does with nothing
    // done around it.
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Failed);
    let why = crate::step(&run, "door").error.clone().unwrap_or_default();
    assert!(why.contains("is not safe to repeat automatically"), "{why}");
    assert_eq!(rig.events("test.reached").await.len(), 1, "its action was done a second time");
    assert_eq!(rig.shared.count("after-door"), 0);
}

const WAITED_TO_TRY: &str = "\
blueprint: waited-to-try
steps:
  work:
    do: test.until
    with: { key: waited-to-try, times: 2 }
    on_fail:
      - { do: wait, with: { for: 30s } }
      - { do: retry }
";

/// The same of a `wait` done as a step fails, before it is tried again:
/// the next try's record was written with what the wait had kept, and
/// that try's action was handed it as a place of its own to go on from.
#[tokio::test]
async fn a_wait_done_as_a_step_fails_keeps_nothing_that_its_next_try_is_handed() {
    let rig = Rig::new().await;
    let slept = rig.clock.sleep_count();
    let applied = rig.apply(WAITED_TO_TRY, json!({})).await;
    let id = applied.run.as_deref().unwrap();
    // It has failed once, and what is done then is waiting.
    rig.soon("what is done as it fails to wait", rig.clock.sleeps_started(slept + 1)).await;
    let kept = crate::step(&rig.engine.run(id).await.unwrap(), "work").saved.clone();
    assert!(kept.is_null(), "what was done as the step failed kept {kept} in the step's own record");
    rig.clock.advance(std::time::Duration::from_secs(30));
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok);
    let work = crate::step(&run, "work");
    assert_eq!(work.attempt, 2);
    assert!(work.saved.is_null(), "its second try was handed {}", work.saved);
}

const BUILT_FIRST: &str = "\
blueprint: built-first
steps:
  door:
    do: test.gate
    with: { name: door }
    on_start:
      - { do: test.durable, with: { name: first } }
";

/// And of any action done around a step that keeps where it is as it
/// goes: nothing takes such a thing up where it was, so what it keeps is
/// kept nowhere, and never in the record of the step it is done around.
#[tokio::test]
async fn an_action_done_as_a_step_starts_that_keeps_where_it_is_keeps_nothing_in_the_steps_record() {
    let rig = Rig::new().await;
    let applied = rig.apply(BUILT_FIRST, json!({})).await;
    // What is done as it starts has kept its place and waits; then it is
    // over, and the step's own action is under way.
    rig.reached("first").await;
    rig.open("first");
    rig.reached("door").await;
    let run = rig.engine.run(applied.run.as_deref().unwrap()).await.unwrap();
    let kept = &crate::step(&run, "door").saved;
    assert!(kept.is_null(), "what was done as the step started kept {kept} in the step's own record");
    rig.open("door");
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
}
