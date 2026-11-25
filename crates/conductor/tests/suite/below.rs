//! The look below a parent somebody applied, stopped anywhere and started
//! again.
//!
//! Somebody who applies over a parent is answered when its steps have
//! begun, and what is under it is looked at when they have settled:
//! what a handler of the parent started, which no step of it brings to
//! the text now there. A conductor stopped in between has nobody left to
//! ask again, so that the look is owed is kept with the text, and it is
//! done when the steps taken up settle, or at once where they had.

use serde_json::json;

use crate::anywhere::{Does, Found, Scenario, anywhere, once, then};

const UNDER: &str = r#"blueprint: under
description: Something a handler starts, which stays.
version: 1
steps:
  begin: { do: test.mark, with: { key: under-began } }
on:
  poke: { event: under.poke, outside: true, do: test.mark, with: { key: poked-one } }
"#;

/// Its next text, which begins as the first does: taken where it is.
const UNDER_NEXT: &str = r#"blueprint: under
description: Something a handler starts, which stays, and marks otherwise.
version: 2
steps:
  begin: { do: test.mark, with: { key: under-began } }
on:
  poke: { event: under.poke, outside: true, do: test.mark, with: { key: poked-two } }
"#;

const MINDER: &str = r#"blueprint: minder
description: Its steps take a while, and a handler of it starts something for what it hears of.
version: 1
steps:
  settle: { do: test.sleep, with: { for: 1m } }
  mark: { do: test.mark, with: { key: minded } }
on:
  came:
    event: minder.came
    outside: true
    steps:
      start: { spawn: under, as: "under-${ event.data.id }" }
"#;

/// Applied over it by somebody: its steps are others, and run again.
const MINDER_NEXT: &str = r#"blueprint: minder
description: Its steps take a while longer, and a handler of it starts something for what it hears of.
version: 2
steps:
  settle: { do: test.sleep, with: { for: 2m } }
  mark: { do: test.mark, with: { key: minded-again } }
on:
  came:
    event: minder.came
    outside: true
    steps:
      start: { spawn: under, as: "under-${ event.data.id }" }
"#;

#[tokio::test]
async fn what_a_handler_started_is_brought_to_the_text_now_there_when_its_parent_is_applied_over_wherever_the_conductor_is_stopped()
 {
    let minder = Scenario::of("minder", MINDER)
        .with(&[UNDER])
        .and(once(
            |seen| seen.said("instance.active") >= 1,
            Does::Say("minder.came", json!({ "act": 1, "id": "a" })),
        ))
        .and(once(|seen| seen.said("instance.active") >= 2, Does::Put(UNDER_NEXT)))
        .and(then(Does::Apply(MINDER_NEXT, json!({}))))
        .over("10m", "1m");
    let found = anywhere(minder).await;
    // With nothing in its way, what the handler started runs the text
    // now there once the parent's steps have settled.
    let under = &found.usual["instances"]["minder[minder] > under[under-a]"];
    assert_eq!(under["text"], blueprint::revision(UNDER_NEXT), "{}", found.usual);
    assert_eq!(under["status"], "active", "{}", found.usual);
    found.held();
}

/// Starts the minder, and hands it another round the first time something
/// under it is brought to another text: a step that gives the minder
/// another text in the middle of the look below it, once.
const ABOVE: &str = r#"blueprint: above
description: Starts the minder, and hands it another round as what is under it is first brought on.
version: 1
one_at_a_time: true
state: { handed: false }
steps:
  start: { spawn: lingers, as: minded }
on:
  brought:
    event: instance.updated
    scope: own
    when: ${ event.data.blueprint == 'under' && !state.handed }
    steps:
      again: { spawn: lingers, as: minded, with: { round: 2 } }
      handed: { do: set, with: { handed: true } }
"#;

/// A minder that says how one that is running takes another text: it is
/// carried over, whoever asks.
const LINGERS: &str = r#"blueprint: lingers
description: A handler of it starts something for what it hears of.
version: 1
params: { round: 1 }
steps:
  mark: { do: test.mark, with: { key: minded } }
on:
  came:
    event: minder.came
    outside: true
    steps:
      start: { spawn: under, as: "under-${ event.data.id }" }
on_update:
  - do: test.mark
    with: { key: carried }
"#;

/// Its next text, which somebody applies over it.
const LINGERS_NEXT: &str = r#"blueprint: lingers
description: A handler of it starts something for what it hears of, and it marks otherwise.
version: 2
params: { round: 1 }
steps:
  mark: { do: test.mark, with: { key: minded-again } }
on:
  came:
    event: minder.came
    outside: true
    steps:
      start: { spawn: under, as: "under-${ event.data.id }" }
on_update:
  - do: test.mark
    with: { key: carried-again }
"#;

/// What the minder starts something for, one at a time.
const UNDERS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

/// When the minder and so many of what it starts are active: the next is
/// asked for then.
const ACTIVE: [fn(&crate::anywhere::Seen) -> bool; 7] = [
    |seen| seen.said("instance.active") >= 2,
    |seen| seen.said("instance.active") >= 3,
    |seen| seen.said("instance.active") >= 4,
    |seen| seen.said("instance.active") >= 5,
    |seen| seen.said("instance.active") >= 6,
    |seen| seen.said("instance.active") >= 7,
    |seen| seen.said("instance.active") >= 8,
];

/// Whether, in what a play kept, a step's carrying the minder over came
/// while the look below it was still bringing what is under it on: the
/// text was taken with the params alone changed (as only the step asks),
/// and something under it was brought on after that.
fn carried_mid_look(kept: &[kernel::Event]) -> bool {
    let carried = kept.iter().position(|event| {
        event.topic == "instance.updated"
            && event.data["blueprint"] == "lingers"
            && event.data["carried"] == true
            && event.data["revision"] == event.data["from"]
    });
    carried.is_some_and(|at| {
        kept[at..].iter().any(|event| event.topic == "instance.updated" && event.data["blueprint"] == "under")
    })
}

/// Plays the minder applied over by somebody and carried over by the
/// step above it, to `next`, while the look below it is under way.
async fn carried_mid_way(name: &'static str, next: &'static str) -> Found {
    let mut lingers = Scenario::of(name, ABOVE).with(&[LINGERS, UNDER]);
    // Several under it, so that the look is still going when the step
    // comes to carry it over.
    for (nth, id) in UNDERS.iter().enumerate() {
        lingers =
            lingers.and(once(ACTIVE[nth], Does::Say("minder.came", json!({ "act": nth + 1, "id": id }))));
    }
    let lingers = lingers
        .and(once(ACTIVE[UNDERS.len()], Does::Put(UNDER_NEXT)))
        .and(then(Does::Put(next)))
        .and(then(Does::Again("minded")))
        .there(carried_mid_look);
    let found = anywhere(lingers).await;
    // With nothing in its way, everything under the minder runs the text
    // now there.
    for id in UNDERS {
        let under = &found.usual["instances"][format!("above[above] > lingers[minded] > under[under-{id}]")];
        assert_eq!(under["text"], blueprint::revision(UNDER_NEXT), "{}", found.usual);
    }
    found.held();
    assert!(
        found.noted > 0,
        "{name}: in no play did the step carry the minder over while the look below it was under way: \
         the order this play is there for was not come to, and it has held nothing of it. Give the \
         minder more under it"
    );
    found
}

// The look below a parent somebody applied over is owed until it is
// over, whoever gives the parent another text meanwhile: here a step of
// what is above it, which hears what the look brings on and carries the
// parent over to another round. The note is kept by the write that takes
// the step's text, and the look is done again once that text is taken.
// The write took the note off, and a conductor stopped before the look
// was over left what it had not come to on the text it had, at eighteen
// of the places.
#[tokio::test]
async fn a_look_below_that_is_owed_is_done_where_a_step_carries_the_parent_over_before_it_is_over() {
    let found = carried_mid_way("above", LINGERS_NEXT).await;
    // Carried over twice: by somebody, and by the step.
    assert_eq!(found.usual["said"]["instance.updated"], UNDERS.len() + 2, "{}", found.usual);
}

/// The same, which cannot be carried over to the round the step hands it.
const LINGERS_REFUSES: &str = r#"blueprint: lingers
description: A handler of it starts something for what it hears of, and it takes no second round.
version: 2
params: { round: 1 }
steps:
  mark: { do: test.mark, with: { key: minded-again } }
on:
  came:
    event: minder.came
    outside: true
    steps:
      start: { spawn: under, as: "under-${ event.data.id }" }
on_update:
  - do: test.mark
    with: { key: carried-again }
  - when: ${ params.round == 2 }
    do: test.fails
"#;

// And where what the step carries it over to fails as it is taken, and
// it goes back: the look somebody asked for is owed still, and is done
// again on the text gone back to. Going back took the note off, at
// twelve of the places. And the step that asked, taken up after a stop
// that fell once its asking was answered, does not ask again: it asked
// again, and the text was carried over and failed a second time, at
// twelve others.
#[tokio::test]
async fn a_look_below_that_is_owed_is_done_where_a_step_carries_the_parent_over_and_it_goes_back() {
    let found = carried_mid_way("above-back", LINGERS_REFUSES).await;
    let minder = &found.usual["instances"]["above[above] > lingers[minded]"];
    assert_eq!(minder["text"], blueprint::revision(LINGERS_REFUSES), "{}", found.usual);
    assert_eq!(found.usual["said"]["instance.update_failed"], 1, "{}", found.usual);
}
