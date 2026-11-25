//! An instance that takes one event at a time (`one_at_a_time: true`): its
//! handlers take turns, so that one which looks at its state and then
//! changes it is not overtaken by another doing the same. Without the
//! word, two handlers of one instance run side by side, as they always
//! did.
//!
//! And a handler that takes turns by a key (`turn:`): what comes to one
//! key is handled in order and one at a time, and what comes to another
//! beside it. That is for an instance that looks after many things of a
//! kind, where one of them must hold up none of the others.

use kernel::Source;
use serde_json::json;

use crate::Rig;
use crate::handlers::{active, handled};

/// Two handlers, each of which takes the one place there is if it is
/// free: it looks, does something that takes a moment, and then takes it.
fn pool(word: &str) -> String {
    let handler = |name: &str| {
        format!(
            "  {name}:\n    event: pool.{name}\n    steps:\n      free: {{ when: ${{ state.taken < 1 }}, do: noop }}\n      \
             think: {{ do: test.gate, with: {{ name: {name} }} }}\n      take: {{ when: ${{ steps.free.ok }}, do: set, \
             with: {{ taken: ${{ state.taken + 1 }} }} }}\n"
        )
    };
    format!("blueprint: pool\n{word}state: {{ taken: 0 }}\non:\n{}{}", handler("one"), handler("two"))
}

async fn reached(rig: &Rig, gate: &str) {
    rig.event("test.reached", |event| event.data["gate"] == gate).await;
}

#[tokio::test]
async fn handlers_of_an_instance_that_takes_one_event_at_a_time_take_turns() {
    let rig = Rig::new().await;
    let applied = rig.apply(&pool("one_at_a_time: true\n"), json!({})).await;
    active(&rig, &applied.instance).await;
    rig.journal.emit("pool.one", Source::default(), json!({})).await.unwrap();
    reached(&rig, "one").await;
    rig.journal.emit("pool.two", Source::default(), json!({})).await.unwrap();

    // The second has not begun: the first is in the middle of its turn.
    // Let go, the first takes the place, and only then does the second
    // look, and finds it taken.
    rig.open("one");
    handled(&rig, "on.one", 1).await;
    reached(&rig, "two").await;
    let begun: Vec<String> = rig
        .events("run.*")
        .await
        .iter()
        .filter(|event| event.data["flow"] != "main")
        .map(|event| format!("{} {}", event.topic, event.data["flow"].as_str().unwrap_or_default()))
        .collect();
    assert_eq!(begun, ["run.started on.one", "run.finished on.one", "run.started on.two"]);
    rig.open("two");
    handled(&rig, "on.two", 1).await;
    assert_eq!(rig.instance(&applied.instance).await.state["taken"], 1, "the one place was taken once");
}

#[tokio::test]
async fn handlers_of_an_instance_that_does_not_say_so_run_side_by_side() {
    let rig = Rig::new().await;
    let applied = rig.apply(&pool(""), json!({})).await;
    active(&rig, &applied.instance).await;
    rig.journal.emit("pool.one", Source::default(), json!({})).await.unwrap();
    rig.journal.emit("pool.two", Source::default(), json!({})).await.unwrap();
    // Both have looked, and both found the place free.
    reached(&rig, "one").await;
    reached(&rig, "two").await;
    rig.open("one");
    rig.open("two");
    handled(&rig, "on.one", 1).await;
    handled(&rig, "on.two", 1).await;
    assert_eq!(rig.instance(&applied.instance).await.state["taken"], 2, "which is what the word is for");
}

#[tokio::test]
async fn a_handlers_own_when_is_asked_again_when_its_turn_comes() {
    let rig = Rig::new().await;
    // The second handler runs only while the place is free, and says so
    // of itself, beside its event.
    let source = "\
blueprint: pool
one_at_a_time: true
state: { taken: 0 }
on:
  one:
    event: pool.one
    steps:
      think: { do: test.gate, with: { name: one } }
      take: { do: set, with: { taken: ${ state.taken + 1 } } }
  two:
    event: pool.two
    when: ${ state.taken < 1 }
    do: set
    with: { taken: ${ state.taken + 1 } }
";
    let applied = rig.apply(source, json!({})).await;
    active(&rig, &applied.instance).await;
    rig.journal.emit("pool.one", Source::default(), json!({})).await.unwrap();
    reached(&rig, "one").await;
    // The place was free when this was said, and is not when its turn comes.
    rig.journal.emit("pool.two", Source::default(), json!({})).await.unwrap();
    rig.open("one");
    handled(&rig, "on.one", 1).await;
    // Something said after it, which has its turn after it: once that has
    // begun, the second handler has had its own.
    rig.journal.emit("pool.one", Source::default(), json!({})).await.unwrap();
    let mut reached = rig.follow("test.reached");
    rig.soon("the first handler to begin again", async {
        for _ in 0..2 {
            reached.next().await.unwrap();
        }
    })
    .await;
    assert_eq!(rig.instance(&applied.instance).await.state["taken"], 1);
    let twos = rig.events("run.started").await.iter().filter(|event| event.data["flow"] == "on.two").count();
    assert_eq!(twos, 0, "it did not run at all");
}

/// One instance that looks after rows, whichever they are: what is said of
/// a row waits at a gate named for the row and for what was said.
const KEEPER: &str = "\
blueprint: keeper
on:
  changed:
    event: row.changed
    turn: \"${event.data.row}\"
    steps:
      wait: { do: test.durable, with: { name: \"changed-${event.data.row}-${event.data.n}\" } }
  spoken:
    event: row.spoken
    turn: \"${event.data.row}\"
    steps:
      wait: { do: test.durable, with: { name: \"spoken-${event.data.row}-${event.data.n}\" } }
";

async fn say(rig: &Rig, topic: &str, row: &str, n: u64) {
    rig.journal.emit(topic, Source::default(), json!({ "row": row, "n": n })).await.unwrap();
}

/// The gates that have been reached, in the order they were.
async fn gates(rig: &Rig) -> Vec<String> {
    let reached = rig.events("test.reached").await;
    reached.iter().map(|event| event.data["gate"].as_str().unwrap_or_default().to_string()).collect()
}

#[tokio::test]
async fn what_comes_to_one_key_is_handled_in_order_and_one_at_a_time_and_what_comes_to_another_beside_it() {
    let rig = Rig::new().await;
    let applied = rig.apply(KEEPER, json!({})).await;
    active(&rig, &applied.instance).await;
    say(&rig, "row.changed", "a", 1).await;
    say(&rig, "row.changed", "a", 2).await;
    say(&rig, "row.changed", "b", 1).await;

    // The first of `a` is in hand, and `b` beside it: one handler, and it
    // did not wait for `a` before it took `b`. The second of `a` waits.
    reached(&rig, "changed-a-1").await;
    reached(&rig, "changed-b-1").await;
    assert_eq!(gates(&rig).await.len(), 2, "the second of `a` has not begun");

    // `b` done changes nothing for `a`; the first of `a` done, its second begins.
    rig.open("changed-b-1");
    handled(&rig, "on.changed", 1).await;
    assert_eq!(gates(&rig).await.len(), 2);
    rig.open("changed-a-1");
    reached(&rig, "changed-a-2").await;
    rig.open("changed-a-2");
    handled(&rig, "on.changed", 3).await;
}

#[tokio::test]
async fn two_handlers_that_come_to_the_same_key_take_turns_with_each_other() {
    let rig = Rig::new().await;
    let applied = rig.apply(KEEPER, json!({})).await;
    active(&rig, &applied.instance).await;
    say(&rig, "row.changed", "a", 1).await;
    reached(&rig, "changed-a-1").await;
    say(&rig, "row.spoken", "a", 1).await;
    say(&rig, "row.spoken", "b", 1).await;

    // What was said on `b` is taken up; what was said on `a` waits for
    // what is being done with `a`, though another handler is doing it.
    reached(&rig, "spoken-b-1").await;
    assert_eq!(gates(&rig).await, ["changed-a-1", "spoken-b-1"]);
    rig.open("changed-a-1");
    reached(&rig, "spoken-a-1").await;
    rig.open("spoken-a-1");
    rig.open("spoken-b-1");
    handled(&rig, "on.spoken", 2).await;
}

#[tokio::test]
async fn after_a_restart_what_was_under_way_for_each_key_is_carried_on_beside_the_others_and_in_its_order() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(KEEPER, json!({})).await;
    active(&rig, &applied.instance).await;
    say(&rig, "row.changed", "a", 1).await;
    say(&rig, "row.changed", "a", 2).await;
    say(&rig, "row.changed", "b", 1).await;
    reached(&rig, "changed-a-1").await;
    reached(&rig, "changed-b-1").await;

    rig.restart().await;

    // Both that were in hand are taken up again where they were, neither
    // behind the other: `b` let go is done while both of `a` are still to
    // do. And the one that was waiting its turn still waits for it.
    rig.open("changed-b-1");
    handled(&rig, "on.changed", 1).await;
    assert!(!gates(&rig).await.contains(&"changed-a-2".to_string()));
    rig.open("changed-a-1");
    reached(&rig, "changed-a-2").await;
    rig.open("changed-a-2");
    handled(&rig, "on.changed", 3).await;
    assert_eq!(
        rig.shared.count("durable:resume"),
        2,
        "the two that were in hand, and not the one that waited"
    );
}

#[tokio::test]
async fn after_a_restart_what_was_in_hand_for_a_key_goes_on_before_what_another_handler_had_waiting_behind_it()
 {
    let mut rig = Rig::new().await;
    let applied = rig.apply(KEEPER, json!({})).await;
    active(&rig, &applied.instance).await;
    // Something said on `a` is in hand, and a change to `a` waits behind
    // it. The change to `b` is heard after that one by the same handler,
    // so the change to `a` has its place by the time `b`'s has begun.
    say(&rig, "row.spoken", "a", 1).await;
    reached(&rig, "spoken-a-1").await;
    say(&rig, "row.changed", "a", 1).await;
    say(&rig, "row.changed", "b", 1).await;
    reached(&rig, "changed-b-1").await;

    rig.restart().await;

    // The change to `a` could go at once, were it given its turn: its gate
    // is open. It is not, while what was in hand for `a` is still in hand,
    // though the handler that heard it comes first in the text. `b`, let
    // go, is done meanwhile.
    rig.open("changed-a-1");
    rig.open("changed-b-1");
    handled(&rig, "on.changed", 1).await;
    assert!(!gates(&rig).await.contains(&"changed-a-1".to_string()), "begun before what was in hand went on");
    rig.open("spoken-a-1");
    reached(&rig, "changed-a-1").await;
    handled(&rig, "on.changed", 2).await;
    handled(&rig, "on.spoken", 1).await;
}

// This says what is so, and does not catch its absence. On the one thread
// these tests run on, what was in hand is always given its turn before a
// handler has read what it missed, so it passes without the waiting too
// (`Again::all_given`); tried, five times of five. Where the two are run
// side by side they can come the other way round, and what catches that
// is a play: a card sent back, stopped as its worker's turn begins, on
// four processors (`simulated.rs` in the API suite), which came to the
// card sent back twice.
#[tokio::test]
async fn what_a_handler_hears_after_a_restart_takes_its_turn_behind_what_was_in_hand_for_its_key() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(KEEPER, json!({})).await;
    active(&rig, &applied.instance).await;
    // Something said on `a` is in hand when the conductor stops. Nothing
    // of the handler that hears changes is under way, so it listens as
    // soon as there is a conductor again; and a change to `a` and one to
    // `b` are said while there is none, for it to hear first of all.
    say(&rig, "row.spoken", "a", 1).await;
    reached(&rig, "spoken-a-1").await;
    rig.engine.halt();
    rig.engine.halted().await;
    say(&rig, "row.changed", "a", 1).await;
    say(&rig, "row.changed", "b", 1).await;
    rig.open("changed-a-1");
    rig.open("changed-b-1");

    rig.restart().await;

    // The change to `a` could go at once, were it given its turn: its gate
    // is open. It is not, while what was in hand for `a` when the
    // conductor stopped is still in hand, though the handler that heard it
    // comes first in the text and had nothing to take up. `b`'s is done
    // meanwhile.
    handled(&rig, "on.changed", 1).await;
    assert!(!gates(&rig).await.contains(&"changed-a-1".to_string()), "begun before what was in hand went on");
    rig.open("spoken-a-1");
    reached(&rig, "changed-a-1").await;
    handled(&rig, "on.changed", 2).await;
    handled(&rig, "on.spoken", 1).await;
}
