//! What is under something that is stopped stops reacting before
//! anything under it is taken down.
//!
//! What is under an instance is stopped a thing at a time, the newest
//! first, and each was called off only as its turn came. So what was
//! under one that is stopped later went on reacting to what the stopping
//! of an earlier one did: a run that waited for something another says
//! as it is stopped heard it, and went on to do what it does then, under
//! a parent that was being stopped.
//!
//! The order is made here: what says something as it is stopped waits at
//! a gate once it has said it, and the play opens the gate only when it
//! is at rest, so whatever would take that in has taken it in by then,
//! on any machine.
//!
//! And being called off has to be enough by itself, whenever what was
//! called off is next looked at. Whoever stops the tree may go on from
//! calling off what is under it to what the first of those says as it
//! stops without once giving way: on a busy machine each answer of the
//! database is there before it is looked for. What waits for that event
//! is then looked at with both at hand, that it was called off and the
//! event, and which it saw first was chance: half the time the run that
//! waited took the event and ended well, or the handler began a run. It
//! showed once in six gates held to four processors, and never alone.
//! That order is made too: the stop asked by somebody who does not give
//! way ([`Does::StopsAtOnce`]), in every play of the scenario.

use serde_json::json;

use crate::anywhere::{Does, Found, Scenario, Seen, anywhere, once};

const SAYS: &str = r#"blueprint: says
description: Something that stays, says that it is going as it is stopped, and takes a while over the rest.
on:
  poke: { event: says.poke, outside: true, do: noop }
on_stop:
  - do: emit
    with: { topic: tree.going }
  - do: test.lingers
    with: { name: going }
  - do: test.mark
    with: { key: says-gone }
"#;

const HEARS: &str = r#"blueprint: hears
description: Something that stays; told to, it waits for something to say that it is going and writes that it heard, and it writes whenever it hears that.
on:
  listen:
    event: tree.listen
    outside: true
    steps:
      heard: { do: wait, with: { event: tree.going } }
      wrote: { needs: [heard], do: test.mark, with: { key: heard-by-a-run } }
  gone:
    event: tree.going
    steps:
      wrote: { do: test.mark, with: { key: heard-by-a-handler } }
"#;

/// The one that hears is the older: it is stopped after the one that
/// says, which is what was not held.
const OLDER_HEARS: &str = r#"blueprint: tree
description: It starts what hears, and then what says.
steps:
  first: { spawn: hears, as: hears }
  second: { needs: [first], spawn: says, as: says }
on:
  poke: { event: tree.poke, outside: true, do: noop }
"#;

const NEWER_HEARS: &str = r#"blueprint: tree
description: It starts what says, and then what hears.
steps:
  first: { spawn: says, as: says }
  second: { needs: [first], spawn: hears, as: hears }
on:
  poke: { event: tree.poke, outside: true, do: noop }
"#;

/// The tree, with a run of what hears waiting when somebody stops it:
/// `stops` is how.
fn stopped(name: &'static str, tree: &'static str, stops: Does) -> Scenario {
    let waits = |seen: &Seen| seen.of_step("step.started", "heard") >= 1;
    Scenario::of(name, tree)
        .with(&[SAYS, HEARS])
        .and(once(|seen| seen.said("instance.active") >= 3, Does::Say("tree.listen", json!({ "act": 1 }))))
        .and(once(waits, stops))
}

/// With nothing in its way: everything was stopped, what says something
/// as it is stopped said it and did the rest, and nothing under the tree
/// took that in.
fn nothing_took_it_in(found: &Found) {
    let usual = &found.usual;
    for one in ["tree[tree]", "tree[tree] > says[says]", "tree[tree] > hears[hears]"] {
        assert_eq!(usual["instances"][one]["status"], "stopped", "{one}: {usual}");
    }
    assert_eq!(usual["said"]["tree.going"], 1, "{usual}");
    assert_eq!(
        usual["asked"][":test.mark (on_stop 2)"], 1,
        "what it does as it stops was done whole: {usual}"
    );
    let heard = |by: &str| usual["asked"][by].as_u64().unwrap_or(0);
    assert_eq!(
        (heard("wrote:test.mark"), usual["runs"]["tree[tree] > hears[hears]"]["on.gone"].clone()),
        (0, json!(null)),
        "what was under the tree was called off before any of it was taken down: {usual}"
    );
    let waited = &usual["runs"]["tree[tree] > hears[hears]"]["on.listen"][0];
    assert_eq!(waited["status"], "cancelled", "the run that waited was cut, and did not fail: {waited}");
    assert_eq!(usual["said"]["run.failed"], json!(null), "{usual}");
    assert_eq!(usual["amiss"], json!([]));
}

#[tokio::test]
async fn what_is_stopped_later_under_something_takes_in_nothing_the_stopping_of_what_is_beside_it_did() {
    let found = anywhere(stopped("older-hears", OLDER_HEARS, Does::Stops("tree"))).await;
    nothing_took_it_in(&found);
    found.held();
}

#[tokio::test]
async fn what_is_stopped_sooner_under_something_takes_in_nothing_either() {
    let found = anywhere(stopped("newer-hears", NEWER_HEARS, Does::Stops("tree"))).await;
    nothing_took_it_in(&found);
    found.held();
}

#[tokio::test]
async fn what_was_called_off_takes_in_nothing_though_whoever_stops_it_never_gives_way() {
    let found = anywhere(stopped("older-hears-at-once", OLDER_HEARS, Does::StopsAtOnce("tree"))).await;
    nothing_took_it_in(&found);
    found.held();
}

#[tokio::test]
async fn nor_does_what_is_stopped_sooner_where_whoever_stops_it_never_gives_way() {
    let found = anywhere(stopped("newer-hears-at-once", NEWER_HEARS, Does::StopsAtOnce("tree"))).await;
    nothing_took_it_in(&found);
    found.held();
}
