//! A text that something running was to be carried over to, asked for
//! and answered, stopped anywhere and started again.
//!
//! A step that asks for something that is running to take another text,
//! where the text says how one that is running takes it, is answered at
//! once: the run that will carry it over is there, and waits until what
//! is to be carried is between the runs of its handlers. The step ends,
//! and its own run after it. A conductor stopped in between has nobody
//! left to ask again: the step was told yes. So what it is to be carried
//! over to is kept with the run that will do it, in the write that makes
//! the run, and a conductor that starts carries it over, once what was
//! under way in it has ended as it does with nothing in the way.
//!
//! Two things come to such a run when a conductor starts, where the
//! step had not yet kept its answer: the start, which goes over
//! everything alive, and the step, taken up, which asks again. The step
//! is answered with the run there is whichever comes first; and where
//! it comes first, to something whose handlers the start has not yet
//! started, nothing is carried over until it has. Which comes first is
//! what is made between the two, as in [`crate::begun`]: with nothing
//! between them the start does, and with things that listen for a
//! great deal the step does. Who came first is measured, and the play
//! that is there for the step coming first fails where it never did.

use serde_json::json;

use crate::anywhere::{Does, Found, Place, Scenario, Seen, anywhere, once, then};

/// Something that stays, with work that takes a while.
const STAYS: &str = r#"blueprint: stays
description: Something that stays, as it is first started.
version: 1
state: { rounds: 0 }
steps:
  claim: { do: test.mark, with: { key: stays-claimed } }
on:
  poke:
    event: stays.poke
    outside: true
    steps:
      slow: { do: test.sleep, with: { for: 1m } }
      mark: { do: set, with: { rounds: 1 } }
"#;

/// Its next text, which says how one that is running takes it, and
/// brings a handler for the very event that has it brought.
const STAYS_NEXT: &str = r#"blueprint: stays
description: The same with another text, which says how one that is running takes it.
version: 2
state: { rounds: 0, came: none }
steps:
  claim: { do: test.mark, with: { key: stays-claimed-again } }
on:
  poke:
    event: stays.poke
    outside: true
    steps:
      slow: { do: test.sleep, with: { for: 1m } }
      mark: { do: set, with: { rounds: 2 } }
  moved:
    event: sender.moved
    outside: true
    steps:
      heard: { do: test.mark, with: { key: stays-heard } }
on_update:
  - do: set
    with: { came: "from ${ was.state.rounds }" }
  - do: test.mark
    with: { key: stays-carried }
"#;

const SENDER: &str = r#"blueprint: sender
description: It starts something that stays, and a handler of it asks for that again as the text then is.
steps:
  begin: { spawn: stays, as: the-one }
on:
  moved:
    event: sender.moved
    outside: true
    steps:
      again: { spawn: stays, as: the-one }
      after: { needs: [again], do: test.mark, with: { key: sender-went-on } }
"#;

/// The same, with two things made between it and what it starts last
/// that a start has much to do for: the step that asked, taken up,
/// comes to what it asked of before the start does.
const CROWDED: &str = r#"blueprint: sender
description: It starts two that listen for a great deal and then something that stays, and a handler of it asks for that again as the text then is.
steps:
  many: { spawn: waits, for_each: ${ [1, 2] } }
  begin: { needs: [many], spawn: stays, as: the-one }
on:
  moved:
    event: sender.moved
    outside: true
    steps:
      again: { spawn: stays, as: the-one }
      after: { needs: [again], do: test.mark, with: { key: sender-went-on } }
"#;

fn asked(found: &Found, what: &str) -> u64 {
    found.usual["asked"][what].as_u64().unwrap_or(0)
}

/// Where, among the places a play passed, the first is that `is` holds of.
fn first(found: &Found, is: impl Fn(&Place) -> bool) -> usize {
    found.places.iter().position(is).unwrap_or_else(|| panic!("the play never passed it: {:?}", found.places))
}

/// What is done to either: work begun in what stays, its next text put
/// where texts are found, and what it was started by told to ask again.
fn sending(source: &'static str, library: &'static [&'static str], active: fn(&Seen) -> bool) -> Scenario {
    Scenario::of("sender", source)
        .with(library)
        .and(once(active, Does::Say("stays.poke", json!({ "act": 1 }))))
        .and(once(|seen: &Seen| seen.of_step("step.started", "slow") > 0, Does::Put(STAYS_NEXT)))
        .and(then(Does::Say("sender.moved", json!({ "act": 2 }))))
        .over("3m", "1m")
}

/// What either comes to with nothing in its way, and that it passed
/// the places it is there for.
fn carried_once(found: &Found) {
    // With nothing in its way: what was under way in it ended on the
    // text it began with, and then it was carried over, once, its steps
    // not run again; and the handler the text brought heard the event
    // that had the text brought.
    let stays = &found.usual["instances"]["sender[sender] > stays[the-one]"];
    assert_eq!(stays["text"], blueprint::revision(STAYS_NEXT), "{}", found.usual);
    assert_eq!(stays["state"], json!({ "rounds": 1, "came": "from 1" }), "{}", found.usual);
    assert_eq!(asked(found, "claim:test.mark"), 1, "its steps were not run again: {}", found.usual);
    assert_eq!(asked(found, ":test.mark (on_update 1)"), 1, "{}", found.usual);
    assert_eq!(asked(found, "heard:test.mark"), 1, "{}", found.usual);
    assert_eq!(asked(found, "after:test.mark"), 1, "the handler that asked went on: {}", found.usual);
    // And it was stopped where it is there to be: with the step that
    // asked answered and over, its run over too, and the text not yet
    // taken. That is by what is written here and not by how long
    // anything takes: the text is taken only once the work under way
    // has ended, and that ends only when time is let pass, at rest.
    let answered = first(found, |place| place.topic == "step.finished" && place.step == "again");
    let over = first(found, |place| place.topic == "run.finished" && place.flow == "on.moved");
    let slept = first(found, |place| place.topic == "step.finished" && place.step == "slow");
    let taken = first(found, |place| place.topic == "instance.updated");
    assert!(
        answered < over && over < slept && slept < taken,
        "the step that asked was to be over before the text was taken, and what was under way to end \
         between the two: {:?}",
        found.places
    );
}

#[tokio::test]
async fn what_a_step_asked_to_be_carried_over_and_was_answered_is_carried_over_wherever_the_conductor_is_stopped()
 {
    let found = anywhere(sending(SENDER, &[STAYS], |seen| seen.said("instance.active") >= 2)).await;
    carried_once(&found);
    found.held();
}

// The same where the step that asked, taken up, comes to what it asked
// of before the conductor's start has: it is answered with the run
// there is, kept from before, and nothing is carried over until the
// start has come to what stays and started its handlers. Asked for
// with no run kept, the text was taken at once, and what it says is
// done as it is taken was done before the work that was under way.
#[tokio::test]
async fn and_where_the_step_that_asked_comes_to_it_again_before_the_conductors_start_does() {
    let crowded = sending(CROWDED, &[STAYS, crate::begun::WAITS], |seen| seen.said("instance.active") >= 4);
    let found = anywhere(crowded).await;
    carried_once(&found);
    found.held();
    // And it was held where it is there to be: with the step first.
    found.came_by_the_step("stays", "text");
}
