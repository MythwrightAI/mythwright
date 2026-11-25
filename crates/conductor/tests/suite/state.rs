//! An instance's state is changed by one `set` at a time, and each works
//! out what it sets from the state as it is at that moment: two that each
//! add one to a count leave it two higher, whichever handlers or steps
//! they are in and however they fall.

use kernel::Source;
use serde_json::json;

use crate::Rig;
use crate::handlers::{active, handled};

/// Two handlers, each of which adds one to the same count.
const COUNTER: &str = "\
blueprint: counter
state: { n: 0, last: none }
on:
  a: { event: tick.a, do: set, with: { n: ${ state.n + 1 }, last: a } }
  b: { event: tick.b, do: set, with: { n: ${ state.n + 1 }, last: b } }
";

#[tokio::test]
async fn two_handlers_that_each_add_to_a_count_lose_nothing_of_it() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(COUNTER, json!({})).await;
    active(&rig, &applied.instance).await;
    for _ in 0..60 {
        rig.journal.emit("tick.a", Source::default(), json!({})).await.unwrap();
        rig.journal.emit("tick.b", Source::default(), json!({})).await.unwrap();
    }
    handled(&rig, "on.a", 60).await;
    handled(&rig, "on.b", 60).await;
    assert_eq!(rig.instance(&applied.instance).await.state["n"], 120, "every one of them counted");

    // What is written down is what it has: a conductor started again reads
    // the same count, and goes on from it.
    rig.restart().await;
    assert_eq!(rig.instance(&applied.instance).await.state["n"], 120);
    rig.journal.emit("tick.a", Source::default(), json!({})).await.unwrap();
    handled(&rig, "on.a", 61).await;
    assert_eq!(rig.instance(&applied.instance).await.state["n"], 121);
}

#[tokio::test]
async fn steps_that_run_side_by_side_and_each_add_to_a_count_lose_nothing_of_it() {
    let rig = Rig::new().await;
    let source = "\
blueprint: sides
state: { n: 0 }
steps:
  one: { needs: [], do: set, with: { n: ${ state.n + 1 } } }
  two: { needs: [], do: set, with: { n: ${ state.n + 1 } } }
  three: { needs: [], do: set, with: { n: ${ state.n + 1 } } }
  four: { needs: [], do: set, with: { n: ${ state.n + 1 } } }
outputs:
  n: ${ state.n }
";
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.outputs, json!({ "n": 4 }));
    // Each was told what it set, which is what the count was after it.
    let mut set: Vec<i64> = ["one", "two", "three", "four"]
        .iter()
        .map(|id| crate::step(&run, id).output["n"].as_i64().unwrap())
        .collect();
    set.sort_unstable();
    assert_eq!(set, [1, 2, 3, 4]);
}

#[tokio::test]
async fn what_is_said_of_each_change_of_state_is_in_the_order_the_changes_were_made() {
    let rig = Rig::new().await;
    let applied = rig.apply(COUNTER, json!({})).await;
    active(&rig, &applied.instance).await;
    for _ in 0..20 {
        rig.journal.emit("tick.a", Source::default(), json!({})).await.unwrap();
        rig.journal.emit("tick.b", Source::default(), json!({})).await.unwrap();
    }
    handled(&rig, "on.a", 20).await;
    handled(&rig, "on.b", 20).await;
    let said: Vec<i64> = rig
        .soon("the events", rig.events("instance.state"))
        .await
        .iter()
        .map(|event| event.data["n"].as_i64().unwrap())
        .collect();
    assert_eq!(said, (1..=40).collect::<Vec<i64>>(), "one event for each change, counting up");
}

/// A list of what is under way, each with what it is called, and a list
/// of plain words; kept in and dropped from by handlers that run side by
/// side.
const KEPT: &str = "\
blueprint: kept
state:
  under_way: [early, { item: older, called: r0 }]
  seen: []
on:
  begun:
    event: work.begun
    do: keep
    with: { in: under_way, by: item, what: { item: \"${event.data.item}\", called: \"${event.data.called}\" } }
  over: { event: work.over, do: drop, with: { from: under_way, by: called, is: \"${event.data.called}\" } }
  gone: { event: work.gone, do: drop, with: { from: under_way, by: item, is: \"${event.data.item}\" } }
  saw: { event: work.seen, do: keep, with: { in: seen, what: \"${event.data.item}\" } }
";

#[tokio::test]
async fn what_is_kept_in_a_list_is_there_once_and_what_is_dropped_is_gone_however_often_either_is_done() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(KEPT, json!({})).await;
    active(&rig, &applied.instance).await;
    let say = |topic: &'static str, data: serde_json::Value| {
        let journal = rig.journal.clone();
        async move { journal.emit(topic, Source::default(), data).await.unwrap() }
    };

    // Kept twice it is there once, with what it now says; an entry is the
    // same by the field its step names.
    say("work.begun", json!({ "item": "a", "called": "r1" })).await;
    say("work.begun", json!({ "item": "b", "called": "r2" })).await;
    say("work.begun", json!({ "item": "a", "called": "r3" })).await;
    handled(&rig, "on.begun", 3).await;
    assert_eq!(
        rig.instance(&applied.instance).await.state["under_way"],
        json!(["early", { "item": "older", "called": "r0" }, { "item": "b", "called": "r2" }, { "item": "a", "called": "r3" }])
    );

    // Dropped by another field of it; dropped again, there is nothing to
    // drop and nothing fails. An entry that is no mapping is looked at
    // whole: a list that held the things themselves goes on being read.
    say("work.over", json!({ "called": "r2" })).await;
    say("work.over", json!({ "called": "r2" })).await;
    say("work.gone", json!({ "item": "early" })).await;
    say("work.gone", json!({ "item": "nobody" })).await;
    handled(&rig, "on.over", 2).await;
    handled(&rig, "on.gone", 2).await;
    let state = rig.instance(&applied.instance).await.state;
    assert_eq!(
        state["under_way"],
        json!([{ "item": "older", "called": "r0" }, { "item": "a", "called": "r3" }])
    );

    // A plain word is the same by all of it. It is what is written down:
    // a conductor started again has the same lists.
    say("work.seen", json!({ "item": "a" })).await;
    say("work.seen", json!({ "item": "a" })).await;
    handled(&rig, "on.saw", 2).await;
    rig.restart().await;
    let state = rig.instance(&applied.instance).await.state;
    assert_eq!(state["seen"], json!(["a"]));
    assert_eq!(
        state["under_way"],
        json!([{ "item": "older", "called": "r0" }, { "item": "a", "called": "r3" }])
    );
}

#[tokio::test]
async fn handlers_that_each_keep_an_entry_of_their_own_side_by_side_lose_none_of_them() {
    let rig = Rig::new().await;
    let applied = rig.apply(KEPT, json!({})).await;
    active(&rig, &applied.instance).await;
    for n in 0..40 {
        let begun = json!({ "item": format!("item-{n}"), "called": format!("r{n}") });
        rig.journal.emit("work.begun", Source::default(), begun).await.unwrap();
        rig.journal
            .emit("work.seen", Source::default(), json!({ "item": format!("item-{n}") }))
            .await
            .unwrap();
        rig.journal.emit("work.gone", Source::default(), json!({ "item": "early" })).await.unwrap();
    }
    handled(&rig, "on.begun", 40).await;
    handled(&rig, "on.saw", 40).await;
    handled(&rig, "on.gone", 40).await;
    let state = rig.instance(&applied.instance).await.state;
    assert_eq!(state["under_way"].as_array().unwrap().len(), 41, "the one from before, and every one kept");
    assert_eq!(state["seen"].as_array().unwrap().len(), 40);
}

#[tokio::test]
async fn what_cannot_be_kept_or_dropped_as_it_is_written_fails_its_step_in_a_sentence() {
    let rig = Rig::new().await;
    let failed = |source: &'static str| {
        let rig = &rig;
        async move { rig.run(source, json!({})).await.error.unwrap_or_default() }
    };
    // An entry that is to be the same by a field has to have it.
    let lacking = failed(
        "blueprint: lacking\nstate: { seen: [] }\nsteps:\n  a: { do: keep, with: { in: seen, by: id, what: { name: one } } }\n",
    )
    .await;
    assert!(
        lacking.contains("`what` has no `id`, which is what an entry of `seen` is the same by"),
        "{lacking}"
    );
    let plain = failed(
        "blueprint: plain\nstate: { seen: [] }\nsteps:\n  a: { do: keep, with: { in: seen, by: id, what: one } }\n",
    )
    .await;
    assert!(
        plain.contains("`by: id` is said of entries that are mappings, and `what` is \"one\""),
        "{plain}"
    );
    // A list named by something worked out is looked for when the step runs.
    let unknown = failed(
        "blueprint: unknown\nstate: { seen: [] }\nparams:\n  which: { type: string, default: sene }\nsteps:\n  a: { do: drop, with: { from: \"${params.which}\", is: one } }\n",
    )
    .await;
    assert!(unknown.contains("there is no state key `sene`"), "{unknown}");
}
