//! Every word of the language, stopped anywhere and started again.
//!
//! Each blueprint here is the least that has its words in it, played by
//! [`crate::anywhere`]: once with nothing in its way, held to what it is
//! meant to come to, and once for every place it passed with the conductor
//! stopped there. No place is left out: one where it does not come to the
//! same is a fault to put right, in the runner or in the play.

use serde_json::{Value, json};

// A step said to be done again from its start after a restart.
mod again;
// What was started side by side, waited for together.
mod gathered;
// What is started under an environment its step names.
mod targeted;
// What fires by the clock.
mod timed;

use crate::anywhere::{Does, Found, Scenario, Seen, anywhere, once, then};
use crate::judged::{in_any_order, kept_nothing, not_twice, rested, taken_up, waited_once, watched};

/// What a run's steps came to with nothing in the way: `(step, status)`.
fn steps(found: &Found, instance: &str, flow: &str) -> Vec<(String, String)> {
    let run = &found.usual["runs"][instance][flow][0];
    let steps = run["steps"]
        .as_object()
        .unwrap_or_else(|| panic!("{instance} has no run of {flow}: {}", found.usual));
    steps.iter().map(|(step, said)| (step.clone(), said["status"].as_str().unwrap().to_string())).collect()
}

fn came(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(step, status)| (step.to_string(), status.to_string())).collect()
}

fn status(found: &Found, instance: &str) -> Value {
    found.usual["instances"][instance]["status"].clone()
}

fn asked(found: &Found, what: &str) -> u64 {
    found.usual["asked"][what].as_u64().unwrap_or(0)
}

fn said(found: &Found, topic: &str) -> u64 {
    found.usual["said"][topic].as_u64().unwrap_or(0)
}

const ORDER: &str = r#"blueprint: order
version: "3"
description: Steps one after another, and two that need the same one.
max_parallel: 2
steps:
  first: { description: The first of them., do: test.mark, with: { key: first } }
  left: { needs: [first], do: test.echo, with: { said: left } }
  right: { needs: [first], do: test.echo, with: { said: right } }
  last: { needs: [left, right], do: test.mark, with: { key: last } }
outputs:
  left: ${ steps.left.out.said }
  marked: ${ steps.last.out.marked }
"#;

#[tokio::test]
async fn steps_in_order_come_to_the_same_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("order", ORDER)).await;
    assert_eq!(status(&found, "order[order]"), "done");
    assert_eq!(
        found.usual["runs"]["order[order]"]["main"][0]["outputs"],
        json!({ "left": "left", "marked": "last" })
    );
    found.held();
}

const PASSED: &str = r#"blueprint: passed
description: A step passed over before, between and after steps that run.
steps:
  before: { when: ${ false }, do: test.mark, with: { key: never } }
  one: { needs: [before], do: test.mark, with: { key: one } }
  between: { needs: [one], when: ${ steps.one.out.marked == "two" }, do: test.mark, with: { key: never } }
  two: { needs: [between], do: test.mark, with: { key: two } }
  after: { needs: [two], when: ${ false }, do: test.mark, with: { key: never } }
"#;

#[tokio::test]
async fn steps_that_are_passed_over_stay_passed_over_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("passed", PASSED)).await;
    let want =
        [("after", "skipped"), ("before", "skipped"), ("between", "skipped"), ("one", "ok"), ("two", "ok")];
    assert_eq!(steps(&found, "passed[passed]", "main"), came(&want));
    found.held();
}

const EACH: &str = r#"blueprint: each
description: A step done for each of none, of one and of several.
params:
  none: { type: list, default: [], description: Nothing to do it for. }
  one: { type: list, default: [a] }
  several: { type: list, default: [a, b, c], required: false }
  which: { type: string, default: all, enum: [all, some] }
steps:
  none: { for_each: ${ params.none }, do: test.mark, with: { key: "none-${ item }" } }
  one: { for_each: ${ params.one }, do: test.mark, with: { key: "one-${ item }" } }
  several: { for_each: ${ params.several }, do: test.echo, with: { v: ${ item } } }
outputs:
  gave: ${ steps.several.out }
"#;

#[tokio::test]
async fn a_step_done_for_each_of_a_list_is_done_once_for_each_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("each", EACH)).await;
    assert_eq!(asked(&found, "none:test.mark"), 0);
    assert_eq!(asked(&found, "several:test.echo"), 3);
    found.held();
}

const LENIENT: &str = r#"blueprint: lenient
description: A step that may fail, and the step after it reads how it went.
steps:
  lint: { do: fail, allow_failure: true, with: { message: three warnings } }
  next: { do: test.echo, with: { failed: ${ steps.lint.failed }, why: ${ steps.lint.error } } }
"#;

#[tokio::test]
async fn a_step_let_fail_fails_once_and_the_run_goes_on_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("lenient", LENIENT)).await;
    assert_eq!(steps(&found, "lenient[lenient]", "main"), came(&[("lint", "failed"), ("next", "ok")]));
    found.held();
}

const HOOKED: &str = r#"blueprint: hooked
description: What is done as a step starts, goes well, fails and ends, and as the steps do.
steps:
  work:
    do: test.mark
    with: { key: work }
    on_start:
      - do: test.mark
        with: { key: work-started }
    on_success:
      - emit: { topic: work.done }
    on_finish:
      - do: test.mark
        with: { key: work-ended }
  bad:
    do: test.fails
    with: { message: no }
    allow_failure: true
    on_fail:
      - do: test.mark
        when: ${ self.error == "no" }
        with: { key: bad-failed }
      - emit: { topic: bad.failed }
on_success:
  - do: test.sleep
    with: { for: 1m }
  - do: test.mark
    with: { key: went-well }
  - do: test.mark
    when: ${ run.status == "failed" }
    with: { key: went-well-and-badly }
on_fail:
  - do: test.mark
    with: { key: went-badly }
on_finish:
  - emit: { topic: hooked.over, how: "${ run.status }" }
"#;

#[tokio::test]
async fn what_is_done_as_steps_start_and_end_is_done_once_wherever_the_conductor_is_stopped() {
    let hooked = Scenario::of("hooked", HOOKED).over("2m", "1m");
    let found = anywhere(hooked).await;
    assert_eq!((said(&found, "work.done"), said(&found, "hooked.over")), (1, 1), "{}", found.usual);
    found.held();
}

const ENDED_WELL: &str = r#"blueprint: ended-well
description: A run that a step ends as having gone well, before what is done when it ends.
steps:
  check: { do: test.mark, with: { key: check } }
  stop: { do: end, with: { status: ok, reason: nothing to do } }
  ship: { do: test.mark, with: { key: never } }
on_success:
  - do: test.mark
    with: { key: went-well }
on_fail:
  - do: test.mark
    with: { key: went-badly }
on_finish:
  - do: test.sleep
    with: { for: 1m }
  - emit: { topic: ended.over, how: "${ run.status }" }
"#;

const ENDED_BADLY: &str = r#"blueprint: ended-badly
description: A run that a step ends as failed, before what is done when it ends.
steps:
  check: { do: test.mark, with: { key: check } }
  stop: { do: end, with: { status: failed, reason: it would not do } }
  ship: { do: test.mark, with: { key: never } }
on_success:
  - do: test.mark
    with: { key: went-well }
on_fail:
  - do: test.sleep
    with: { for: 1m }
  - do: test.mark
    with: { key: went-badly }
on_finish:
  - emit: { topic: ended.over, how: "${ run.status }" }
"#;

#[tokio::test]
async fn a_run_a_step_ended_stays_ended_as_it_was_wherever_the_conductor_is_stopped() {
    let well = Scenario::of("ended-well", ENDED_WELL).over("2m", "1m");
    let well = anywhere(well).await;
    assert_eq!(asked(&well, "ship:test.mark"), 0);
    assert_eq!(well.usual["runs"]["ended-well[ended-well]"]["main"][0]["status"], "ok");
    well.held();
    let badly = Scenario::of("ended-badly", ENDED_BADLY).over("2m", "1m");
    let badly = anywhere(badly).await;
    assert_eq!(badly.usual["runs"]["ended-badly[ended-badly]"]["main"][0]["status"], "failed");
    badly.held();
}

const TASKED: &str = r#"blueprint: tasked
description: Steps done as one, and a run ended from inside such a one.
steps:
  build:
    measure: tasked.build
    labels: { sort: plain }
    task:
      compile: { do: test.echo, with: { artifact: app } }
      test: { do: test.echo, with: { tested: ${ steps.compile.out.artifact } } }
  early:
    task:
      check: { do: test.mark, with: { key: check } }
      stop: { do: end, with: { status: ok, reason: nothing to ship } }
      never: { do: test.mark, with: { key: never } }
  ship: { do: test.mark, with: { key: never } }
on_finish:
  - do: test.sleep
    with: { for: 1m }
  - do: test.mark
    with: { key: finished }
"#;

#[tokio::test]
async fn steps_done_as_one_and_a_run_ended_inside_them_hold_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("tasked", TASKED).over("2m", "1m")).await;
    assert_eq!(asked(&found, "ship:test.mark"), 0, "{}", found.usual);
    found.held();
}

const ASKS: &str = r#"blueprint: asks
description: A step that says something and waits for what answers it.
steps:
  ask:
    do: wait
    with:
      event: answer.given
      where: event.data.to == "colour"
      say: { topic: question.asked, of: colour }
  use: { do: test.echo, with: { got: ${ steps.ask.out.event.data.colour } } }
outputs:
  colour: ${ steps.use.out.got }
"#;

#[tokio::test]
async fn a_wait_asks_once_and_takes_its_answer_wherever_the_conductor_is_stopped() {
    let answer = json!({ "act": "answer", "to": "colour", "colour": "red" });
    let asking = Scenario::of("asks", ASKS)
        .and(once(|seen: &Seen| seen.said("question.asked") > 0, Does::Say("answer.given", answer)));
    let found = anywhere(asking).await;
    assert_eq!(said(&found, "question.asked"), 1);
    assert_eq!(found.usual["runs"]["asks[asks]"]["main"][0]["outputs"], json!({ "colour": "red" }));
    found.held();
}

const GIVES_UP: &str = r#"blueprint: gives-up
description: A wait that nothing answers, which gives up when it said it would.
steps:
  ask: { do: wait, with: { event: never.said, timeout: 2m } }
  after: { do: test.echo, with: { gave_up: ${ steps.ask.out.timed_out } } }
"#;

const NAPS: &str = r#"blueprint: naps
description: A wait for a while.
steps:
  nap: { do: wait, with: { for: 2m } }
  after: { do: test.mark, with: { key: after-nap } }
"#;

#[tokio::test]
async fn a_wait_for_a_while_and_one_that_gives_up_end_when_they_would_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("gives-up", GIVES_UP).over("4m", "1m")).await;
    let after = &found.usual["runs"]["gives-up[gives-up]"]["main"][0]["steps"]["after"]["gave"];
    assert_eq!(after, &json!({ "gave_up": true }));
    found.held();
    let found = anywhere(Scenario::of("naps", NAPS).over("4m", "1m")).await;
    assert_eq!(asked(&found, "after:test.mark"), 1);
    found.held();
}

const QUICK: &str = r#"blueprint: quick
params: { n: { type: int, default: 1 } }
steps:
  double: { do: test.echo, with: { v: ${ params.n * 2 } } }
outputs:
  doubled: ${ steps.double.out.v }
"#;

const SLOW: &str = r#"blueprint: slow
steps:
  nap: { do: test.sleep, with: { for: 1m } }
  done: { do: test.mark, with: { key: slow-done } }
"#;

const FAILING: &str = r#"blueprint: failing
steps:
  bad: { do: test.fails, with: { message: it could not } }
"#;

const BRIEF: &str = r#"blueprint: brief
description: It keeps nothing as it goes.
durable: false
steps:
  one: { do: test.echo, with: { v: 1 } }
  nap: { do: test.sleep, with: { for: 1m } }
  two: { do: test.echo, with: { v: 2 } }
outputs:
  v: ${ steps.two.out.v }
"#;

const SPAWNS: &str = r#"blueprint: spawns
description: It starts something whose steps are over at once, and goes on.
steps:
  kept: { spawn: instant, as: kept }
  after: { needs: [kept], do: test.mark, with: { key: after-spawn } }
"#;

const INSTANT: &str = r#"blueprint: instant
steps:
  one: { do: test.echo, with: { v: 1 } }
"#;

#[tokio::test]
async fn what_a_step_starts_is_started_once_though_its_steps_were_over_before_the_conductor_stopped() {
    let found = anywhere(Scenario::of("spawns", SPAWNS).with(&[INSTANT]).judged(rested)).await;
    assert_eq!(steps(&found, "spawns[spawns]", "main"), came(&[("after", "ok"), ("kept", "ok")]));
    assert_eq!(found.usual["ended"]["spawns[spawns] > instant[kept]"].as_array().map(Vec::len), Some(1));
    // Active for what it started alone, it is done once that has ended.
    assert_eq!(found.usual["instances"]["spawns[spawns]"]["status"], json!("done"), "{}", found.usual);
    found.held();
}

const FAMILY: &str = r#"blueprint: family
description: Others called and started: a quick one, a slow one, one that fails, one that keeps nothing.
steps:
  q: { call: quick, as: the-quick, labels: { way: plain }, with: { n: 21 } }
  s: { call: slow }
  f: { call: failing, allow_failure: true }
  b: { call: brief }
  kept: { spawn: slow, as: spawned }
  many: { call: quick, for_each: ${ [1, 2] }, with: { n: ${ item } } }
  use: { do: test.echo, with: { got: ${ steps.q.out.doubled }, failed: ${ steps.f.failed }, brief: ${ steps.b.out.v } } }
"#;

#[tokio::test]
async fn others_called_and_started_come_to_the_same_wherever_the_conductor_is_stopped() {
    let family = Scenario::of("family", FAMILY)
        .with(&[QUICK, SLOW, FAILING, BRIEF])
        .over("10m", "1m")
        .judged(kept_nothing);
    let found = anywhere(family).await;
    let used = &found.usual["runs"]["family[family]"]["main"][0]["steps"]["use"]["gave"];
    assert_eq!(used, &json!({ "got": 42, "failed": true, "brief": 2 }), "{}", found.usual);
    found.held();
}

const HEARS: &str = r#"blueprint: hears
description: Handlers of events: any, those addressed to it, those of its own.
one_at_a_time: true
state: { heard: 0 }
on:
  plain:
    event: thing.happened
    outside: true
    steps:
      plain: { do: test.mark, with: { key: plain } }
  mine:
    event: thing.happened
    for: { id: a }
    when: ${ event.data.n > 0 }
    steps:
      note: { do: set, with: { heard: ${ state.heard + 1 } } }
      slow: { do: test.sleep, with: { for: 1m } }
      tell: { do: emit, with: { topic: thing.noted, id: "${ event.data.id }" } }
  own:
    event: thing.noted
    scope: own
    call: quick
    with: { n: 2 }
"#;

#[tokio::test]
async fn handlers_hear_what_is_theirs_once_wherever_the_conductor_is_stopped() {
    let happened = |act: u64, id: &str, n: u64| {
        then(Does::Say("thing.happened", json!({ "act": act, "id": id, "n": n })))
    };
    let hearing = Scenario::of("hears", HEARS)
        .with(&[QUICK])
        .and(happened(1, "a", 1))
        .and(happened(2, "b", 1))
        .and(happened(3, "a", 0))
        .over("5m", "1m");
    let found = anywhere(hearing).await;
    assert_eq!(asked(&found, "plain:test.mark"), 3, "{}", found.usual);
    assert_eq!(found.usual["instances"]["hears[hears]"]["state"], json!({ "heard": 1 }));
    assert_eq!(said(&found, "thing.noted"), 1);
    found.held();
}

const KEEPER: &str = r#"blueprint: keeper
description: A handler whose events take turns by what they are about.
on:
  changed:
    event: row.changed
    outside: true
    turn: "${ event.data.row }"
    steps:
      slow: { do: test.sleep, with: { for: 1m } }
      mark: { do: test.echo, with: { row: "${ event.data.row }", n: ${ event.data.n } } }
"#;

#[tokio::test]
async fn what_takes_turns_by_a_key_is_handled_once_each_wherever_the_conductor_is_stopped() {
    let changed =
        |act: u64, row: &str| then(Does::Say("row.changed", json!({ "act": act, "row": row, "n": act })));
    let keeping = Scenario::of("keeper", KEEPER)
        .and(changed(1, "a"))
        .and(changed(2, "a"))
        .and(changed(3, "b"))
        .over("5m", "1m");
    let found = anywhere(keeping).await;
    assert_eq!(asked(&found, "mark:test.echo"), 3, "{}", found.usual);
    found.held();
}

const CARRIED: &str = r#"blueprint: carried
description: Something that stays, as it was first applied.
state: { rounds: 0 }
steps:
  claim: { do: test.mark, with: { key: claimed } }
on:
  poke:
    event: carried.poke
    outside: true
    steps:
      slow: { do: test.sleep, with: { for: 1m } }
      mark: { do: set, with: { rounds: 1 } }
"#;

const CARRIED_ON: &str = r#"blueprint: carried
description: The same with another text, which says how one that is running takes it.
state: { rounds: 0, came: none }
steps:
  claim: { do: test.mark, with: { key: claimed-again } }
on:
  poke:
    event: carried.poke
    outside: true
    steps:
      slow: { do: test.sleep, with: { for: 1m } }
      mark: { do: set, with: { rounds: 2 } }
on_update:
  - do: set
    with: { came: "from ${ was.state.rounds }" }
"#;

#[tokio::test]
async fn an_instance_carried_over_in_the_middle_of_a_handler_is_carried_once_wherever_the_conductor_is_stopped()
 {
    let carrying = Scenario::of("carried", CARRIED)
        .and(then(Does::Say("carried.poke", json!({ "act": 1 }))))
        .and(once(|seen: &Seen| seen.of_step("step.started", "slow") > 0, Does::Apply(CARRIED_ON, json!({}))))
        .over("3m", "1m");
    let found = anywhere(carrying).await;
    assert_eq!(asked(&found, "claim:test.mark"), 1, "its steps were not run again: {}", found.usual);
    assert_eq!(
        found.usual["instances"]["carried[carried]"]["state"],
        json!({ "rounds": 1, "came": "from 1" })
    );
    found.held();
}

const TAKEN: &str = r#"blueprint: taken
description: Something that stays, as it was first applied.
state: { n: 0 }
steps:
  claim: { do: test.mark, with: { key: taken-claimed } }
on:
  poke: { event: taken.poke, outside: true, do: noop }
"#;

const TAKEN_ON: &str = r#"blueprint: taken
description: The same with another text, which says and does several things as it is taken, and hears the first.
state: { n: 0, came: none, heard: 0 }
steps:
  claim: { do: test.mark, with: { key: taken-claimed-again } }
on:
  poke: { event: taken.poke, outside: true, do: noop }
  first:
    event: taken.first
    scope: own
    steps:
      note: { do: set, with: { heard: ${ state.heard + 1 } } }
      mark: { do: test.mark, with: { key: taken-heard } }
on_update:
  - do: emit
    with: { topic: taken.first, n: ${ state.n } }
  - do: set
    with: { n: ${ state.n + 1 } }
  - do: emit
    when: ${ state.n > 5 }
    with: { topic: taken.never }
  - do: test.told
    with: { key: taken-told }
  - do: set
    with: { came: "from ${ was.state.n } to ${ state.n }" }
  - do: emit
    with: { topic: taken.last, n: ${ state.n } }
"#;

#[tokio::test]
async fn what_a_text_says_and_does_as_it_is_taken_is_said_and_done_once_wherever_the_conductor_is_stopped() {
    let taking = Scenario::of("taken", TAKEN).and(then(Does::Apply(TAKEN_ON, json!({}))));
    let found = anywhere(taking).await;
    assert_eq!(
        found.usual["instances"]["taken[taken]"]["state"],
        json!({ "n": 1, "came": "from 0 to 1", "heard": 1 }),
        "{}",
        found.usual
    );
    assert_eq!(
        (said(&found, "taken.first"), said(&found, "taken.last"), said(&found, "taken.never")),
        (1, 1, 0)
    );
    assert_eq!(asked(&found, "mark:test.mark"), 1, "what it said first was heard by the handler it brought");
    found.held();
}

const LEAVES: &str = r#"blueprint: leaves
description: Something that stays, and says and does several things as it is stopped.
state: { n: 0 }
steps:
  begin: { do: test.mark, with: { key: leaves-began } }
on:
  poke: { event: leaves.poke, outside: true, do: noop }
on_stop:
  - do: emit
    with: { topic: leaves.going, n: ${ state.n } }
  - do: emit
    when: ${ state.n > 5 }
    with: { topic: leaves.never }
  - do: test.told
    with: { key: leaves-first }
  - do: set
    with: { n: ${ state.n + 1 } }
  - do: emit
    with: { topic: leaves.gone, n: ${ state.n } }
  - do: test.told
    with: { key: leaves-last }
"#;

const ENDER: &str = r#"blueprint: ender
description: It starts something that stays and stops it, and is stopped itself, each doing several things as it stops.
steps:
  kid: { spawn: leaves, as: the-kid }
  nap: { do: test.sleep, with: { for: 1m } }
  enough: { do: stop, with: { instance: the-kid } }
  after: { do: test.mark, with: { key: after-stop } }
on:
  poke: { event: ender.poke, outside: true, do: noop }
on_stop:
  - do: emit
    with: { topic: ender.going }
  - do: test.told
    with: { key: ender-first }
  - do: emit
    with: { topic: ender.gone }
"#;

#[tokio::test]
async fn what_is_said_and_done_as_something_stops_is_said_and_done_once_wherever_the_conductor_is_stopped() {
    let ending = Scenario::of("ender", ENDER)
        .with(&[LEAVES])
        .over("2m", "1m")
        .and(once(|seen: &Seen| seen.of_step("step.finished", "after") > 0, Does::Stop("ender")));
    let found = anywhere(ending).await;
    assert_eq!(status(&found, "ender[ender] > leaves[the-kid]"), "stopped", "{}", found.usual);
    assert_eq!(status(&found, "ender[ender]"), "stopped");
    assert_eq!(
        (said(&found, "leaves.going"), said(&found, "leaves.gone"), said(&found, "leaves.never")),
        (1, 1, 0)
    );
    assert_eq!((said(&found, "ender.going"), said(&found, "ender.gone")), (1, 1));
    found.held();
}

const KEEPS: &str = r#"blueprint: keeps
description: What an instance keeps, says and measures.
permissions: [shell]
env: { REGION: eu }
state: { n: 0, word: none, seen: [] }
steps:
  one: { do: set, with: { n: 1 } }
  two: { do: set, with: { word: set } }
  three: { do: keep, with: { in: seen, by: id, what: { id: a, n: 1 } } }
  four: { do: keep, with: { in: seen, what: b } }
  five: { do: keep, with: { in: seen, by: id, what: { id: a, n: 2 } } }
  six: { do: drop, with: { from: seen, is: b } }
  tell: { do: emit, with: { topic: keeps.said, n: ${ state.n } } }
  timed: { do: test.shell, measure: keeps.shell, env: { ZONE: a }, device: test-device }
  counted: { do: measure, with: { name: keeps.counted, value: 3, labels: { sort: test } } }
  noted: { do: log, with: { message: all kept } }
on:
  stays: { event: keeps.poke, outside: true, do: noop }
outputs:
  n: ${ state.n }
  word: "${ state.word }"
"#;

#[tokio::test]
async fn what_an_instance_keeps_and_says_is_kept_and_said_once_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("keeps", KEEPS)).await;
    assert_eq!(
        found.usual["instances"]["keeps[keeps]"]["state"],
        json!({ "n": 1, "word": "set", "seen": [{ "id": "a", "n": 2 }] })
    );
    assert_eq!(said(&found, "keeps.said"), 1);
    assert_eq!(status(&found, "keeps[keeps]"), "active");
    found.held();
}

const STAYS: &str = r#"blueprint: stays
description: Something that stays until it is stopped, and says so then.
steps:
  begin: { do: test.mark, with: { key: stays-began } }
on:
  poke: { event: stays.poke, outside: true, do: noop }
on_stop:
  - do: test.mark
    with: { key: stays-stopped }
"#;

const STOPPER: &str = r#"blueprint: stopper
description: It starts two things that stay, and stops them, one saying that nothing there would be no fault.
steps:
  kid: { spawn: stays, as: the-kid }
  other: { spawn: stays, as: the-other }
  nap: { do: test.sleep, with: { for: 1m } }
  enough: { do: stop, with: { instance: the-kid } }
  also: { do: stop, with: { instance: the-other, missing_ok: true } }
  after: { do: test.mark, with: { key: after-stop } }
on:
  poke: { event: stopper.poke, outside: true, do: noop }
"#;

#[tokio::test]
async fn something_stopped_by_a_step_and_from_outside_is_stopped_once_wherever_the_conductor_is_stopped() {
    let stopping = Scenario::of("stopper", STOPPER)
        .with(&[STAYS])
        .over("2m", "1m")
        .and(once(|seen: &Seen| seen.of_step("step.finished", "after") > 0, Does::Stop("stopper")));
    let found = anywhere(stopping).await;
    assert_eq!(status(&found, "stopper[stopper] > stays[the-kid]"), "stopped", "{}", found.usual);
    assert_eq!(status(&found, "stopper[stopper]"), "stopped");
    assert_eq!(status(&found, "stopper[stopper] > stays[the-other]"), "stopped", "{}", found.usual);
    assert_eq!(said(&found, "instance.stopped"), 3);
    found.held();
}

const LASTS: &str = r#"blueprint: lasts
description: Something that stays, with work under way, and says so when it is stopped.
steps:
  begin: { do: test.mark, with: { key: lasts-began } }
on:
  go:
    event: lasts.go
    outside: true
    steps:
      long: { do: test.sleep, with: { for: 10m } }
      after: { do: test.mark, with: { key: lasts-went-on } }
on_stop:
  - do: test.mark
    with: { key: lasts-stopped }
"#;

const FALLS: &str = r#"blueprint: falls
description: It starts something that stays, and fails over it while that is at work.
steps:
  kid: { spawn: lasts, as: the-kid }
  nap: { do: test.sleep, with: { for: 1m } }
  bad: { do: test.fails, with: { message: it could not } }
"#;

#[tokio::test]
async fn what_a_failed_instance_started_is_stopped_as_anything_is_wherever_the_conductor_is_stopped() {
    let falling = Scenario::of("falls", FALLS)
        .with(&[LASTS])
        .and(then(Does::Say("lasts.go", json!({ "act": 1 }))))
        .over("2m", "1m");
    let found = anywhere(falling).await;
    // With nothing in its way: what it started says that it is stopped,
    // does what it does when it is, and the work it had under way is
    // ended and says so, with none of its steps left as under way.
    assert_eq!(status(&found, "falls[falls]"), "failed", "{}", found.usual);
    assert_eq!(status(&found, "falls[falls] > lasts[the-kid]"), "stopped", "{}", found.usual);
    assert_eq!(
        asked(&found, ":test.mark (on_stop 0)"),
        1,
        "what it does when it is stopped: {}",
        found.usual
    );
    let under_way = &found.usual["runs"]["falls[falls] > lasts[the-kid]"]["on.go"][0];
    assert_eq!(under_way["status"], "cancelled", "{}", found.usual);
    assert_eq!(
        steps(&found, "falls[falls] > lasts[the-kid]", "on.go"),
        came(&[("after", "cancelled"), ("long", "cancelled")])
    );
    assert_eq!(said(&found, "instance.stopped"), 1);
    found.held();
}

const TRIES: &str = r#"blueprint: tries
description: A step tried again, and one that runs out of time.
steps:
  work:
    do: test.until
    with: { key: tries, times: 3 }
    retry: { max: 3, backoff: 10s, factor: 2, max_backoff: 1m }
  slow:
    do: test.sleep
    with: { for: 10m }
    timeout: 1m
    allow_failure: true
    on_timeout:
      - do: test.mark
        with: { key: timed-out }
  after: { do: test.echo, with: { slow_failed: ${ steps.slow.failed } } }
"#;

#[tokio::test]
async fn a_step_tried_again_and_one_out_of_time_end_the_same_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("tries", TRIES).over("5m", "10s")).await;
    let want = [("after", "ok"), ("slow", "failed"), ("work", "ok")];
    assert_eq!(steps(&found, "tries[tries]", "main"), came(&want), "{}", found.usual);
    assert_eq!(asked(&found, "slow:test.mark (on_timeout 0)"), 1, "{}", found.usual);
    found.held();
}

const WATCHED: &str = r#"blueprint: watched
description: Rules looked at while a step runs, which ends between two firings of the one that fires every minute.
# The step takes two and a half minutes, and not three: a rule that is due
# in the instant its step ends has no order with that ending, here or
# anywhere, and only a test's clock makes the two fall together. With a
# round number the rule fired a third time in some plays and not in others.
steps:
  work:
    do: test.speak
    with: { lines: [starting, "error: disk"], for: 150s }
    watch:
      - after: 1m
        emit: { topic: watched.slow }
      - every: 1m
        do: test.mark
        with: { key: beats }
      - log: "error:"
        emit: { topic: watched.error }
      - after: 10m
        fail: true
        do: test.mark
        with: { key: too-long }
      - after: 1m
        when: ${ params.loud }
        do: test.mark
        with: { key: loud }
params:
  loud: { type: bool, default: false }
"#;

#[tokio::test]
async fn a_step_that_is_watched_ends_the_same_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("watched", WATCHED).over("5m", "30s").judged(watched)).await;
    assert_eq!((said(&found, "watched.slow"), said(&found, "watched.error")), (1, 1), "{}", found.usual);
    found.held();
}

const WATCHER: &str = r#"blueprint: watcher
params:
  word: { type: string, required: true }
on:
  seen:
    event: thing.done
    scope: target
    steps:
      seen: { do: test.mark, with: { key: "${ params.word }" } }
"#;

const HOST: &str = r#"blueprint: host
description: Something with another attached to it, which hears what it says.
uses:
  w: { blueprint: watcher, with: { word: seen } }
on:
  go: { event: host.go, outside: true, do: emit, with: { topic: thing.done } }
"#;

#[tokio::test]
async fn what_is_attached_hears_its_host_once_wherever_the_conductor_is_stopped() {
    let hosting =
        Scenario::of("host", HOST).with(&[WATCHER]).and(then(Does::Say("host.go", json!({ "act": 1 }))));
    let found = anywhere(hosting).await;
    assert_eq!(asked(&found, "seen:test.mark"), 1, "{}", found.usual);
    found.held();
}

const ONCE_ONLY: &str = r#"blueprint: once-only
description: A step whose action is not safe to do twice.
steps:
  before: { do: test.mark, with: { key: before } }
  door: { do: test.gate, with: { name: door } }
  after: { do: test.mark, with: { key: after } }
"#;

#[tokio::test]
async fn a_step_that_cannot_be_done_twice_fails_saying_so_when_the_conductor_is_stopped_in_it() {
    let found = anywhere(Scenario::of("once-only", ONCE_ONLY).judged(not_twice)).await;
    assert_eq!(asked(&found, "after:test.mark"), 1);
    found.held();
}

const WAITED: &str = r#"blueprint: waited
description: A wait done as a step fails and as one starts, around actions of their own.
steps:
  before:
    do: test.until
    with: { key: waited, times: 2 }
    on_fail:
      - do: wait
        with: { for: 30s }
      - do: retry
  door:
    do: test.gate
    with: { name: door }
    on_start:
      - do: wait
        with: { for: 30s }
  after: { do: test.mark, with: { key: after } }
"#;

#[tokio::test]
async fn a_wait_done_around_a_step_keeps_no_place_for_the_steps_action_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("waited", WAITED).over("3m", "10s").judged(waited_once)).await;
    let want = [("after", "ok"), ("before", "ok"), ("door", "ok")];
    assert_eq!(steps(&found, "waited[waited]", "main"), came(&want), "{}", found.usual);
    assert_eq!(
        (asked(&found, "door:test.gate"), asked(&found, "after:test.mark")),
        (1, 1),
        "{}",
        found.usual
    );
    found.held();
}

const PICKED_UP: &str = r#"blueprint: picked-up
description: A step whose action keeps where it is, and is taken up there.
steps:
  build: { do: test.durable, with: { name: build } }
  after: { do: test.echo, with: { v: 1 } }
"#;

#[tokio::test]
async fn a_step_that_keeps_where_it_is_is_taken_up_there_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("picked-up", PICKED_UP).judged(taken_up)).await;
    assert_eq!(asked(&found, "after:test.echo"), 1);
    found.held();
}

const ROOMY: &str = r#"blueprint: roomy
description: Steps side by side, as many at a time as it says.
max_parallel: 2
steps:
  a: { needs: [], do: test.sleep, with: { for: 1m } }
  b: { needs: [], do: test.sleep, with: { for: 1m } }
  c: { needs: [], do: test.sleep, with: { for: 1m } }
  last: { needs: [a, b, c], do: test.mark, with: { key: last } }
"#;

#[tokio::test]
async fn steps_side_by_side_all_end_once_wherever_the_conductor_is_stopped() {
    let found = anywhere(Scenario::of("roomy", ROOMY).over("5m", "1m")).await;
    assert_eq!(asked(&found, "last:test.mark"), 1);
    found.held();
}

const ADDS: &str = r#"blueprint: adds
description: Two handlers of one instance each add one to a number it keeps.
state: { n: 0 }
on:
  one:
    event: adds.go
    outside: true
    steps:
      add: { do: set, with: { n: ${ state.n + 1 } } }
  two:
    event: adds.go
    outside: true
    steps:
      add: { do: set, with: { n: ${ state.n + 1 } } }
"#;

#[tokio::test]
async fn two_handlers_that_each_add_one_add_once_each_wherever_the_conductor_is_stopped() {
    let adding = Scenario::of("adds", ADDS)
        .and(then(Does::Say("adds.go", json!({ "act": 1 }))))
        .and(then(Does::Say("adds.go", json!({ "act": 2 }))))
        .judged(in_any_order);
    let found = anywhere(adding).await;
    assert_eq!(found.usual["instances"]["adds[adds]"]["state"], json!({ "n": 4 }), "{}", found.usual);
    found.held();
}

const GEARED: &str = r#"blueprint: geared
description: Attaches something that cannot be brought up with what it is handed.
params: { size: many }
uses:
  g: { blueprint: gear, with: { teeth: "${ params.size }" } }
on:
  go: { event: geared.go, do: test.mark, with: { key: went } }
"#;

const GEAR: &str = r#"blueprint: gear
description: Takes a number, and is handed something else.
params:
  teeth: { type: int }
on:
  turned: { event: gear.turn, do: test.mark, with: { key: turned } }
"#;

const BESIDE: &str = r#"blueprint: beside
description: Stays, beside one that cannot be begun.
on:
  go: { event: beside.go, do: test.mark, with: { key: beside } }
"#;

#[tokio::test]
async fn an_instance_that_cannot_be_begun_fails_once_and_the_conductor_starts_wherever_it_is_stopped() {
    // A conductor that could not start would fail the play where it is
    // started again: `Rig::stopped_there` holds its start to working.
    let unbegun = Scenario::of("unbegun", BESIDE)
        .with(&[GEAR])
        .and(then(Does::Offer(GEARED, json!({}))))
        .and(once(|seen| seen.said("instance.failed") == 1, Does::Say("beside.go", json!({ "act": 1 }))));
    let found = anywhere(unbegun).await;
    assert_eq!(said(&found, "instance.failed"), 1, "what it attaches cannot be brought up, and it says so");
    assert_eq!(asked(&found, "run:test.mark"), 1, "and what stays beside it goes on");
    found.held();
}
