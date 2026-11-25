//! A list of things done one after another for one item, where which
//! comes next is told as the list is gone through: nothing in the
//! language goes through such a list, so it is run as a chain. A handler
//! that takes turns by the item is told the name of what to do, calls it
//! by that name with what it was told to hand it, and ends by saying what
//! it came to; a handler of the same instance hears that and says what
//! begins the next.
//!
//! What a chain rests on is held here, with no table and no item's row
//! in it: a name and a mapping that a step worked out are what is called
//! and what is handed; what is called keeps nothing, and is called again
//! under the same name where the conductor stopped while it was there;
//! and what a handler says as it ends is heard once by its own instance.
//!
//! What it cannot rest on is held here too: a handler under way while its
//! instance is given other params reads them as they now are in each step
//! it comes to. So what is to be as it was when a thing was begun is
//! taken from what a step answered before the thing was called, which is
//! kept with the step, and never from the params afterwards.

use serde_json::json;

use crate::Rig;
use crate::anywhere::{Does, Found, Scenario, anywhere, then};
use crate::handlers::{active, handled};
use crate::judged::kept_nothing;

const CHAIN: &str = r#"blueprint: chain
description: A list gone through as a chain, each of its things called by a name a step was told.
on:
  due:
    event: thing.due
    outside: true
    turn: "${ event.data.item }"
    steps:
      which: { do: test.echo, with: { name: "${ event.data.does }", with: { word: "${ event.data.does }", times: 2 } } }
      does:
        call: "${ steps.which.out.name }"
        as: "does-${ event.data.item }"
        with:
          item: "${ event.data.item }"
          ...: ${ steps.which.out.with }
      came:
        do: emit
        with:
          topic: thing.came
          item: "${ event.data.item }"
          at: "${ event.data.does }"
          came: "${ steps.does.out.came }"
  next:
    event: thing.came
    scope: own
    for: { at: first }
    turn: "${ event.data.item }"
    do: emit
    with: { topic: thing.due, item: "${ event.data.item }", does: second }
"#;

const FIRST: &str = r#"blueprint: first
description: One thing done with an item, which keeps nothing as it goes.
durable: false
params:
  item: { type: string }
  word: { type: string }
  times: { type: int, default: 1 }
steps:
  slow: { do: test.sleep, with: { for: 1m } }
  mark: { do: test.mark, with: { key: "first-${ params.item }" } }
outputs:
  came: "${ params.word } x${ params.times }"
"#;

const SECOND: &str = r#"blueprint: second
description: The thing done after it, which is kept.
params:
  item: { type: string }
  word: { type: string }
  times: { type: int, default: 1 }
steps:
  slow: { do: test.sleep, with: { for: 1m } }
  mark: { do: test.mark, with: { key: "second-${ params.item }" } }
outputs:
  came: "${ params.word } x${ params.times }"
"#;

fn said(found: &Found, topic: &str) -> u64 {
    found.usual["said"][topic].as_u64().unwrap_or(0)
}

#[tokio::test]
async fn a_chain_of_things_called_by_a_name_a_step_was_told_is_gone_through_once_wherever_the_conductor_is_stopped()
 {
    let due = |act: u64, item: &str| {
        then(Does::Say("thing.due", json!({ "act": act, "item": item, "does": "first" })))
    };
    // One item: what two of them come to side by side is in which order
    // their runs are counted, which is the machine's to say.
    let chain = Scenario::of("chain", CHAIN)
        .with(&[FIRST, SECOND])
        .and(due(1, "a"))
        .over("4m", "1m")
        .judged(kept_nothing);
    let found = anywhere(chain).await;
    // It went through both, each was told what a step worked out, and
    // each ending was said once.
    assert_eq!(said(&found, "thing.came"), 2, "{}", found.usual);
    assert_eq!(said(&found, "thing.due"), 2, "{}", found.usual);
    let came: Vec<String> = found.usual["runs"]["chain[chain]"]["on.due"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|run| run["steps"]["does"]["gave"]["came"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(came, ["first x2", "second x2"], "{}", found.usual);
    found.held();
}

const TOLD: &str = "\
blueprint: told
params: { word: one }
on:
  due:
    event: thing.due
    turn: \"${ event.data.item }\"
    steps:
      told: { do: test.echo, with: { word: \"${ params.word }\" } }
      think: { do: test.gate, with: { name: think } }
      say: { do: emit, with: { topic: thing.said, word: \"${ params.word }\", was: \"${ steps.told.out.word }\" } }
on_update:
  - do: test.count
    with: { key: carried }
";

#[tokio::test]
async fn a_handler_under_way_while_its_instance_is_given_other_params_reads_them_as_they_now_are() {
    let rig = Rig::new().await;
    let applied = rig.apply(TOLD, json!({ "word": "one" })).await;
    active(&rig, &applied.instance).await;
    rig.journal.emit("thing.due", kernel::Source::default(), json!({ "item": "a" })).await.unwrap();
    rig.event("test.reached", |event| event.data["gate"] == "think").await;

    // Given other params where it is: a handler that takes turns by a key
    // is not waited for, and the instance is carried over beside it.
    let again = rig.apply(TOLD, json!({ "word": "two" })).await;
    assert_eq!(again.instance, applied.instance);
    // Carried over: what the text does as it is taken is done once the
    // text and its params are in hand, so that is what is waited for. Its
    // row says them a moment sooner, in the write that takes them, and
    // the run let go as soon as the row said them read the params it had
    // in hand before that once, in a gate held to four processors at a
    // load of 33 (`what_a_step_begun_once_the_row_says_its_params_reads_is_them`
    // plays every turn of that moment and finds no order of the
    // database's answers that does it: what is left is which of two tasks
    // the runtime looks at first, which a test cannot hold).
    handled(&rig, "on_update", 1).await;
    assert_eq!(rig.instance(&applied.instance).await.params["word"], "two");

    // The same run goes on (it is let go once, and ends): the step it
    // comes to reads the params as they now are, and what a step answered
    // before the instance was given them is as it was.
    rig.open("think");
    handled(&rig, "on.due", 1).await;
    let said = rig.events("thing.said").await;
    assert_eq!(said.len(), 1);
    assert_eq!(said[0].data["word"], "two", "the params as they now are");
    assert_eq!(said[0].data["was"], "one", "and what a step answered when it began");
    assert_eq!(rig.shared.count("carried"), 1, "it was carried over beside the run, once");
}

/// Whether `asked` has come to its end, looked at once.
fn answered<T>(asked: &mut std::pin::Pin<Box<impl Future<Output = T>>>) -> bool {
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    asked.as_mut().poll(&mut cx).is_ready()
}

/// The run is let go after every amount of turns of the database from
/// the moment other params are applied, so that one of them is the moment
/// the row says them and nothing has looked at the instance since. Where
/// the row said them as the run was let go, the step it comes to reads
/// them; where it did not, either may be read, the instance taking them
/// meanwhile.
#[tokio::test]
async fn what_a_step_begun_once_the_row_says_its_params_reads_is_them() {
    let mut said_two = 0;
    for turns in 0..16 {
        let rig = Rig::new().await;
        let applied = rig.apply(TOLD, json!({ "word": "one" })).await;
        active(&rig, &applied.instance).await;
        rig.journal.emit("thing.due", kernel::Source::default(), json!({ "item": "a" })).await.unwrap();
        rig.event("test.reached", |event| event.data["gate"] == "think").await;
        let mut stepped = crate::stepped::Stepped::holding(&rig.journal);
        let mut asking = Box::pin(rig.engine.apply(rig.request(TOLD, json!({ "word": "two" }))));
        let mut asked = false;
        for _ in 0..turns {
            asked = asked || answered(&mut asking);
            stepped.turn(true).await;
        }
        let row = stepped.through(true, rig.instance(&applied.instance)).await.unwrap();
        rig.open("think");
        let said = stepped
            .through(true, async {
                handled(&rig, "on.due", 1).await;
                rig.events("thing.said").await
            })
            .await
            .unwrap();
        if !asked {
            stepped.through(true, &mut asking).await;
        }
        drop(stepped);
        handled(&rig, "on_update", 1).await;
        assert_eq!(said.len(), 1);
        if row.params["word"] == "two" {
            said_two += 1;
            assert_eq!(
                said[0].data["word"], "two",
                "let go after {turns} turns, the row saying the new params"
            );
        }
    }
    assert!(
        said_two > 0,
        "the row never said the new params as the run was let go: the moment was never played"
    );
}
