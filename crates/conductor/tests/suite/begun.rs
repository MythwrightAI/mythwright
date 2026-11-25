//! An instance written down and not yet begun, stopped anywhere and
//! started again.
//!
//! Between an instance being written down and its first run there is a
//! moment, and a conductor stopped in it starts with an instance that has
//! no run. Two things then come to it: the start itself, which goes over
//! everything alive, and the step that asked for it, taken up as its own
//! run goes on. Each play here has such a child, played by
//! [`crate::anywhere`] as the plays of `plays.rs` are: it is begun once,
//! by whichever comes first, and one that cannot be begun fails once.
//!
//! Which of the two comes first is not left to chance, nor to the
//! machine. A start comes to instances in the order they were made, and
//! the step's run is taken up when the start comes to its instance: so
//! the step is first wherever the start has more to ask of the database
//! for what was made between the two than the step has before it comes
//! to the child. That is what `waits` is for: something at rest that
//! listens for a great deal, each handler a place in the journal the
//! start reads. With nothing between them, or only things still at their
//! steps, the start is always first, and a runner that lets both begin
//! the child holds every play all the same.
//!
//! That is so on any machine because a conductor that is started again
//! here has its database answered a turn at a time ([`crate::stepped`]):
//! each thing it has running goes forward by one answer a turn, and
//! which comes first is how many answers each needs. Left to the
//! machine, the step came first where there were processors to spare
//! and the start did where there was one, since there an answer is back
//! before whoever asked has given way, and a start runs on to its end:
//! the plays that are there for the step coming first failed on a slow
//! machine with nothing wrong in what they hold. A play still says
//! whether it was there: who began each child is measured
//! (`instance.begun`), and a play in which the step never came first
//! fails, saying so ([`crate::anywhere::Found::came_by_the_step`]);
//! which now says that the play was written wrongly, or that a start has
//! come to ask less.
//!
//! And one moment no stop is at: the one in which the steps the first
//! began end, as the second comes. It is come to by the amount a start
//! has to do, each amount played (the last test here).

use serde_json::{Value, json};

use crate::anywhere::{Found, Scenario, anywhere};
use crate::judged::kept_nothing;

fn status(found: &Found, instance: &str) -> Value {
    found.usual["instances"][instance]["status"].clone()
}

fn asked(found: &Found, what: &str) -> u64 {
    found.usual["asked"][what].as_u64().unwrap_or(0)
}

fn said(found: &Found, topic: &str) -> u64 {
    found.usual["said"][topic].as_u64().unwrap_or(0)
}

pub const WAITS: &str = r#"blueprint: waits
description: Something that stays, and listens for a great deal: a start has much to do for each of these before it comes to what was written after them.
steps:
  begin: { do: test.mark, with: { key: waits-began } }
on:
  h01: { event: waits.h01, outside: true, do: noop }
  h02: { event: waits.h02, outside: true, do: noop }
  h03: { event: waits.h03, outside: true, do: noop }
  h04: { event: waits.h04, outside: true, do: noop }
  h05: { event: waits.h05, outside: true, do: noop }
  h06: { event: waits.h06, outside: true, do: noop }
  h07: { event: waits.h07, outside: true, do: noop }
  h08: { event: waits.h08, outside: true, do: noop }
  h09: { event: waits.h09, outside: true, do: noop }
  h10: { event: waits.h10, outside: true, do: noop }
  h11: { event: waits.h11, outside: true, do: noop }
  h12: { event: waits.h12, outside: true, do: noop }
  h13: { event: waits.h13, outside: true, do: noop }
  h14: { event: waits.h14, outside: true, do: noop }
  h15: { event: waits.h15, outside: true, do: noop }
  h16: { event: waits.h16, outside: true, do: noop }
  h17: { event: waits.h17, outside: true, do: noop }
  h18: { event: waits.h18, outside: true, do: noop }
  h19: { event: waits.h19, outside: true, do: noop }
  h20: { event: waits.h20, outside: true, do: noop }
  h21: { event: waits.h21, outside: true, do: noop }
  h22: { event: waits.h22, outside: true, do: noop }
  h23: { event: waits.h23, outside: true, do: noop }
  h24: { event: waits.h24, outside: true, do: noop }
"#;

const DOUBLES: &str = r#"blueprint: doubles
params: { n: { type: int, default: 1 } }
steps:
  double: { do: test.echo, with: { v: ${ params.n * 2 } } }
  mark: { do: test.mark, with: { key: doubled } }
outputs:
  doubled: ${ steps.double.out.v }
"#;

const NAPS: &str = r#"blueprint: naps
description: Something started and left to go on, which takes a while.
steps:
  nap: { do: test.sleep, with: { for: 1m } }
  done: { do: test.mark, with: { key: napped } }
"#;

const CROWD: &str = r#"blueprint: crowd
description: It starts two that stay and lets them come to rest, and then calls one and starts another, so that a start comes to those two after it has done all it does for the rest.
steps:
  many: { spawn: waits, for_each: ${ [1, 2] } }
  rest: { do: test.sleep, with: { for: 1m } }
  q: { call: doubles, as: the-called, with: { n: 21 } }
  s: { spawn: naps, as: the-started }
  use: { do: test.echo, with: { got: ${ steps.q.out.doubled } } }
"#;

#[tokio::test]
async fn a_child_with_no_run_yet_is_begun_once_wherever_the_conductor_is_stopped() {
    let crowd = Scenario::of("crowd", CROWD).with(&[WAITS, DOUBLES, NAPS]).over("3m", "1m");
    let found = anywhere(crowd).await;
    let used = &found.usual["runs"]["crowd[crowd]"]["main"][0]["steps"]["use"]["gave"];
    assert_eq!(used, &json!({ "got": 42 }), "{}", found.usual);
    assert_eq!(asked(&found, "mark:test.mark"), 1, "what was called ran its steps once: {}", found.usual);
    assert_eq!(asked(&found, "done:test.mark"), 1, "and so did what was started: {}", found.usual);
    assert_eq!(asked(&found, "begin:test.mark"), 2);
    found.held();
    // And it was held where it is there to be: with the step first.
    found.came_by_the_step("doubles", "first");
    found.came_by_the_step("naps", "first");
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

const ASKS: &str = r#"blueprint: asks
description: It calls something that cannot be begun, and starts another such, lets each be and goes on.
steps:
  before: { do: test.mark, with: { key: before } }
  called: { call: geared, as: the-called, allow_failure: true }
  between: { do: test.mark, with: { key: between } }
  left: { spawn: geared, as: the-started, allow_failure: true }
  after: { do: test.echo, with: { called: ${ steps.called.failed }, left: ${ steps.left.failed } } }
"#;

#[tokio::test]
async fn a_child_that_cannot_be_begun_fails_once_wherever_the_conductor_is_stopped() {
    let asks = Scenario::of("asks", ASKS).with(&[GEARED, GEAR]).judged(kept_nothing);
    let found = anywhere(asks).await;
    let after = &found.usual["runs"]["asks[asks]"]["main"][0]["steps"]["after"]["gave"];
    assert_eq!(after, &json!({ "called": true, "left": true }), "{}", found.usual);
    assert_eq!(said(&found, "instance.failed"), 2, "each fails, and says so once: {}", found.usual);
    assert_eq!(status(&found, "asks[asks]"), "done", "{}", found.usual);
    found.held();
}

const ASKS_BEHIND: &str = r#"blueprint: asks-behind
description: The same, behind two that stay and have come to rest, so that the step that asked comes to each before a start does.
steps:
  many: { spawn: waits, for_each: ${ [1, 2] } }
  rest: { do: test.sleep, with: { for: 1m } }
  called: { call: geared, as: the-called, allow_failure: true }
  between: { do: test.mark, with: { key: between } }
  left: { spawn: geared, as: the-started, allow_failure: true }
  after: { do: test.echo, with: { called: ${ steps.called.failed }, left: ${ steps.left.failed } } }
"#;

#[tokio::test]
async fn a_child_that_cannot_be_begun_fails_once_where_the_step_that_asked_comes_to_it_first() {
    let asks = Scenario::of("asks-behind", ASKS_BEHIND)
        .with(&[WAITS, GEARED, GEAR])
        .over("3m", "1m")
        .judged(kept_nothing);
    let found = anywhere(asks).await;
    let after = &found.usual["runs"]["asks-behind[asks-behind]"]["main"][0]["steps"]["after"]["gave"];
    assert_eq!(after, &json!({ "called": true, "left": true }), "{}", found.usual);
    assert_eq!(said(&found, "instance.failed"), 2, "each fails, and says so once: {}", found.usual);
    assert_eq!(status(&found, "asks-behind[asks-behind]"), "active", "{}", found.usual);
    found.held();
    // In `asks` the start is always first; here the step is.
    found.came_by_the_step("geared", "first");
}

const TWICE: &str = r#"blueprint: twice
description: Doubles what it is handed, and stays.
params: { n: { type: int, default: 1 } }
steps:
  double: { do: test.echo, with: { v: ${ params.n * 2 } } }
  mark: { do: test.mark, with: { key: doubled } }
on:
  poke: { event: twice.poke, outside: true, do: noop }
outputs:
  doubled: ${ steps.double.out.v }
"#;

const RECALLS: &str = r#"blueprint: recalls
description: Behind two that stay and have come to rest, it calls something that stays, and calls it again handing it something else, which is an update of what the first call made.
steps:
  many: { spawn: waits, for_each: ${ [1, 2] } }
  rest: { do: test.sleep, with: { for: 1m } }
  first: { call: twice, as: the-child, with: { n: 1 } }
  second: { call: twice, as: the-child, with: { n: 21 } }
  use: { do: test.echo, with: { was: ${ steps.first.out.doubled }, got: ${ steps.second.out.doubled } } }
"#;

/// An update is two writes as well: the text and what it is handed are
/// written as taken, and then the run of its steps. A conductor stopped
/// between them starts with an instance that is owed those steps, and
/// the `call` that asked, where it came first, was told that nothing had
/// changed and handed what the steps gave the time before.
#[tokio::test]
async fn a_child_that_took_what_a_call_handed_it_and_has_no_run_yet_runs_its_steps_once_for_the_call() {
    let recalls = Scenario::of("recalls", RECALLS).with(&[WAITS, TWICE]).over("3m", "1m");
    let found = anywhere(recalls).await;
    let used = &found.usual["runs"]["recalls[recalls]"]["main"][0]["steps"]["use"]["gave"];
    assert_eq!(used, &json!({ "was": 2, "got": 42 }), "{}", found.usual);
    assert_eq!(asked(&found, "mark:test.mark"), 2, "its steps ran once for each call: {}", found.usual);
    found.held();
    found.came_by_the_step("twice", "again");
}

/// Something that stays and listens for so many things: what a start
/// has to go through, one at a time, before it comes to what was written
/// after.
fn listens(many: usize) -> &'static str {
    let mut text = String::from(
        "blueprint: waits\n\
         description: Something that stays, and listens for so many things.\n\
         steps:\n  begin: { do: test.mark, with: { key: waits-began } }\n\
         on:\n",
    );
    for n in 1..=many {
        text.push_str(&format!("  h{n:03}: {{ event: waits.h{n:03}, outside: true, do: noop }}\n"));
    }
    Box::leak(text.into_boxed_str())
}

const RECALLS_BEHIND_ONE: &str = r#"blueprint: recalls
description: The same, behind one that stays, so that each thing more it listens for brings a start to what was written after it one answer of the database later.
steps:
  many: { spawn: waits }
  rest: { do: test.sleep, with: { for: 1m } }
  first: { call: twice, as: the-child, with: { n: 1 } }
  second: { call: twice, as: the-child, with: { n: 21 } }
  use: { do: test.echo, with: { was: ${ steps.first.out.doubled }, got: ${ steps.second.out.doubled } } }
"#;

/// How many amounts of what a start has to do are played, from one
/// thing listened for.
const AMOUNTS: usize = 32;

/// How many amounts past the one at which the step first comes before
/// the start there have to be, for the start to have come to the child
/// at every answer of the database while the steps the step began were
/// under way, their last included. The steps of `twice` take some dozen
/// answers; a child whose steps came to take more than this would be
/// one this holds nothing of, and it says so.
const WHILE_UNDER_WAY: usize = 20;

/// Whichever of the two comes second finds the steps the first began
/// under way, or over, and there is a moment between: the one in which
/// they end. Whoever came then read an instance that was starting, and
/// then a run that was over, and took it to be owed its steps again
/// (`Store::standing` reads the two as one now). No stop is at that
/// moment, which is one answer of the database wide; what comes to it
/// is the right amount for a start to have to do. So the same thing is
/// played for every amount from one thing listened for, the database
/// answered a turn at a time ([`crate::stepped`]), each amount bringing
/// the start to the child one answer later: stopped where the child is
/// written down and not begun, and where it has taken what the second
/// call handed it. Among the amounts the start comes first, and then the
/// step does, and then the start comes at each answer while the steps
/// are under way and after.
#[tokio::test]
async fn whoever_comes_to_it_second_as_its_steps_end_does_not_have_them_run_again() {
    // The first amount at which the step came before the start, for the
    // child's first steps and for those of what it took.
    let (mut first, mut again) = (None, None);
    let (mut start_first, mut start_again) = (false, false);
    for many in 1..=AMOUNTS {
        let library: &'static [&'static str] = Box::leak(vec![listens(many), TWICE].into_boxed_slice());
        let recalls =
            Scenario::of("recalls", RECALLS_BEHIND_ONE).with(library).over("3m", "1m").only(|place| {
                place.blueprint == "twice"
                    && matches!(place.topic.as_str(), "instance.started" | "instance.updated")
            });
        let found = anywhere(recalls).await;
        let used = &found.usual["runs"]["recalls[recalls]"]["main"][0]["steps"]["use"]["gave"];
        assert_eq!(used, &json!({ "was": 2, "got": 42 }), "listening for {many}: {}", found.usual);
        eprintln!("listening for {many}:");
        found.held();
        let by = |who: &str, owed: &str| {
            found.begun.get(&("twice".to_string(), who.to_string(), owed.to_string())).copied().unwrap_or(0)
        };
        start_first |= by("start", "first") > 0;
        start_again |= by("start", "again") > 0;
        if by("step", "first") > 0 {
            first.get_or_insert(many);
        }
        if by("step", "again") > 0 {
            again.get_or_insert(many);
        }
    }
    // Both orders were played, and the amounts went on long enough past
    // where the step first came first: or this has held nothing of the
    // moment it is there for, however it came out.
    for (owed, start, step) in [("first", start_first, first), ("again", start_again, again)] {
        assert!(start, "its steps ({owed}): in no amount did the conductor's start come to `twice` first");
        let Some(step) = step else {
            panic!(
                "its steps ({owed}): in none of {AMOUNTS} amounts did the step that asked for `twice` come to it \
                 before the conductor's start did: play more amounts"
            );
        };
        assert!(
            AMOUNTS - step >= WHILE_UNDER_WAY,
            "its steps ({owed}): the step first came before the start with {step} things listened for, and \
             only {} amounts were played past that, where {WHILE_UNDER_WAY} are to be: the start has not come \
             to `twice` at every answer while its steps were under way. Play more amounts.",
            AMOUNTS - step
        );
    }
}
