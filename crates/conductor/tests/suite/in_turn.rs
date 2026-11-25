//! What is applied over an instance is the text as it was read where the
//! instance's turn was held.
//!
//! Whoever asks for an instance to be brought to a text reads the text
//! first and waits for the instance's turn at being applied over
//! afterwards. Somebody else may have the turn in between, and give the
//! instance a later text: the first was then applied over that with the
//! text it had read before. A step that asked for the child it has, as
//! somebody applied a text that dropped one of the child's handlers,
//! took the child back to the text from before, and the handler was
//! there again; the look below a parent did the same with the text it
//! had found the child behind.
//!
//! The order is made here and not left to the machine. The database is
//! answered a turn at a time ([`Stepped`]), whoever asks first goes
//! forward by so many turns, and then somebody applies the later text
//! without giving way ([`Stepped::at_once`]), as a busy machine has it,
//! where each answer is there before it is looked for. Every amount of
//! turns is played, from the first asker not yet begun until its asking
//! is over, so one of them is the moment between its reading and its
//! turn whatever it comes to ask of the database before either.

use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

use conductor::{Applied, Apply, InstanceFilter, InstanceInfo};
use kernel::Source;
use serde_json::json;

use crate::handlers::{active, handled};
use crate::stepped::Stepped;
use crate::{Rig, soon, user};

/// The child, as it is started: it hears two things.
const OLD: &str = "\
blueprint: child
params: { id: 0 }
steps:
  claim: { do: test.count, with: { key: claimed } }
on:
  poke: { event: test.poke, do: test.count, with: { key: poked } }
  again: { event: test.nudge, do: test.count, with: { key: nudged } }
";

/// The text the look below finds the child behind: it differs from the
/// one it runs only in a handler.
fn found() -> String {
    OLD.replace("key: poked", "key: prodded")
}

/// The later text, which drops a handler; `by_steps`, one the child
/// comes to only by its steps running again.
fn later(by_steps: bool) -> String {
    let text = OLD.replace("  again: { event: test.nudge, do: test.count, with: { key: nudged } }\n", "");
    if by_steps { text.replace("key: claimed", "key: claimed-again") } else { text }
}

/// Who asks first, and goes forward a turn at a time.
#[derive(Clone, Copy, Debug, PartialEq)]
enum First {
    /// A step of the parent's handler, spawning the child it has.
    Spawn,
    /// The same, calling it.
    Call,
    /// The look below the parent, which somebody applied again: the
    /// child is behind the text there.
    Below,
    /// Somebody, applying the child by its name and key.
    ByName,
}

impl First {
    /// What starts the child for a card, and asks for it again for each
    /// move of the card.
    fn parent(self) -> String {
        let asks = if self == First::Call { "call" } else { "spawn" };
        format!(
            "blueprint: parent\non:\n  ready:\n    event: board.item.moved\n    steps:\n      go: {{ {asks}: \
             child, as: kid, with: {{ id: 1 }} }}\n"
        )
    }
}

async fn moved(rig: &Rig) {
    rig.journal.emit("board.item.moved", Source::default(), json!({})).await.unwrap();
}

/// An asking that the test looks at itself.
type Asking<'a> = Pin<Box<dyn Future<Output = kernel::Result<Applied>> + Send + 'a>>;

/// Somebody asking beside the test, a task of their own: whoever asks
/// first may have the turn while the later text waits for it.
fn asks(rig: &Rig, request: Apply) -> tokio::task::JoinHandle<kernel::Result<Applied>> {
    let engine = rig.engine.clone();
    tokio::spawn(async move { engine.apply(request).await })
}

/// Something applied by somebody who has each answer before looking
/// for it: nothing else is looked at until they wait for
/// what is no answer of the database, or are over.
async fn applied_at_once(stepped: &mut Stepped, mut second: Asking<'_>) {
    let applied = match stepped.at_once(second.as_mut()) {
        Poll::Ready(applied) => applied,
        Poll::Pending => stepped.through(false, second).await.expect("the journal is open"),
    };
    applied.unwrap_or_else(|refused| panic!("what was applied at once was refused: {}", refused.message));
}

/// Goes on a turn at a time until nothing is asked of any database.
async fn rested(stepped: &mut Stepped) {
    let mut still = 0;
    while still < STILL {
        still = if stepped.turn(false).await == Some(0) { still + 1 } else { 0 };
    }
}

/// How many turns in a row nothing is asked of any database before
/// what is running is taken to be at rest.
const STILL: usize = 12;

/// What one playing came to.
struct Came {
    /// The first asker had not begun when the later text was applied.
    before: bool,
    /// Its asking was over by then.
    after: bool,
    /// The child, at rest.
    child: InstanceInfo,
    /// How often the handler the later text drops ran for something
    /// said once everything was at rest.
    heard: u64,
}

/// The parent with its child, `first` asking for the child and going
/// forward `turns` turns, and then the later text applied over the
/// child by somebody who does not give way.
async fn between(first: First, by_steps: bool, turns: usize) -> Came {
    let rig = Rig::new().await;
    let parent = first.parent();
    rig.library.put("parent", &parent);
    rig.library.put("child", OLD);
    let top = rig.apply(&parent, json!({})).await.instance;
    active(&rig, &top).await;
    moved(&rig).await;
    handled(&rig, "on.ready", 1).await;
    let live = rig.engine.instances(InstanceFilter { live_only: true, ..Default::default() }).await.unwrap();
    let kid = live.into_iter().find(|one| one.key == "kid").expect("the child").id;
    active(&rig, &kid).await;
    // The move that started it has come to all it comes to.
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    if first == First::Below {
        rig.library.put("child", &found());
    }

    let mut stepped = Stepped::holding(&rig.journal);
    let began = rig.mark();
    let asking = match first {
        First::Spawn | First::Call => {
            stepped.through(false, moved(&rig)).await;
            None
        }
        First::Below => Some(asks(&rig, rig.request(&parent, json!({})))),
        First::ByName => Some(asks(
            &rig,
            Apply {
                key: Some("kid".into()),
                parent: Some(top.clone()),
                params: kernel::fields(json!({ "id": 1 })),
                ..Apply::new("child", user())
            },
        )),
    };
    for _ in 0..turns {
        stepped.turn(false).await;
    }

    let (came, answered) = (rig.mark(), asking.as_ref().is_some_and(|asked| asked.is_finished()));
    let text = later(by_steps);
    rig.library.put("child", &text);
    applied_at_once(&mut stepped, Box::pin(rig.engine.apply_again(&kid, user(), false))).await;
    let told = match asking {
        Some(asked) => Some(stepped.through(false, asked).await.expect("the journal is open").unwrap()),
        None => None,
    };
    rested(&mut stepped).await;
    drop(stepped);

    // Where the first asker was when the later text came: a step, by
    // what its run had said; somebody, by whether they were answered.
    let (before, after) = match first {
        First::Spawn | First::Call => {
            let asked: Vec<_> = rig
                .events("run.started,run.finished")
                .await
                .into_iter()
                .filter(|event| event.data["flow"] == "on.ready" && event.seq > began)
                .collect();
            assert_eq!(asked.len(), 2, "{first:?}, {turns} turns: the step's run began and ended");
            assert_eq!(asked[1].data["status"], "ok", "{first:?}, {turns} turns: and the step was answered");
            (asked[0].seq > came, asked[1].seq <= came)
        }
        First::Below | First::ByName => {
            let told = told.expect("whoever asked first was answered");
            told.unwrap_or_else(|refused| panic!("{first:?}, {turns} turns: {}", refused.message));
            (turns == 0, answered)
        }
    };
    rig.journal.emit("test.nudge", Source::default(), json!({})).await.unwrap();
    rig.journal.emit("test.poke", Source::default(), json!({})).await.unwrap();
    handled(&rig, "on.poke", 1).await;
    Came { before, after, child: rig.instance(&kid).await, heard: rig.shared.count("nudged") }
}

/// The most turns whoever asks first is given before its asking is
/// taken never to end.
const MOST: usize = 200;

/// Plays every amount of turns for `first`, from none until its asking
/// is over when the later text comes, and holds each to the child being
/// left on the later text, with the handler it drops gone.
async fn every_amount(first: First, by_steps: bool) {
    let text = blueprint::revision(&later(by_steps));
    for turns in 0..=MOST {
        let case = format!("{first:?}, by steps: {by_steps}, {turns} turns");
        let came = soon(&case, between(first, by_steps, turns)).await;
        assert_eq!(came.child.revision, text, "{case}: the child is on the text applied last");
        assert_eq!(came.heard, 0, "{case}: and the handler that text drops was not given back");
        assert!(turns > 0 || came.before, "{case}: whoever asks first had begun before it was given a turn");
        if came.after {
            return;
        }
    }
    panic!("{first:?}: its asking was not over after {MOST} turns: the amounts no longer reach past it");
}

#[tokio::test]
async fn a_step_that_spawns_its_child_does_not_take_it_back_from_a_text_applied_as_it_asked() {
    every_amount(First::Spawn, false).await;
    every_amount(First::Spawn, true).await;
}

#[tokio::test]
async fn a_step_that_calls_its_child_does_not_take_it_back_from_a_text_applied_as_it_asked() {
    every_amount(First::Call, false).await;
    every_amount(First::Call, true).await;
}

#[tokio::test]
async fn the_look_below_does_not_take_a_child_back_from_a_text_applied_as_it_looked() {
    every_amount(First::Below, false).await;
}

#[tokio::test]
async fn somebody_applying_by_name_does_not_take_an_instance_back_from_a_text_applied_as_they_asked() {
    every_amount(First::ByName, false).await;
    every_amount(First::ByName, true).await;
}

/// What starts a child for each card, with the card for its param.
const MANY: &str = "\
blueprint: many
on:
  ready:
    event: board.item.moved
    steps:
      go: { spawn: child, as: \"kid-${ event.data.id }\", with: { id: ${ event.data.id } } }
";

/// A text the children are behind, which says how one that is running
/// takes it: the look below carries a child over to it.
fn says() -> String {
    format!("{}on_update:\n  - do: test.count\n    with: {{ key: carried }}\n", found())
}

/// Who reads a child's params before the child's turn.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Looks {
    /// The look below the parent, which somebody applied again.
    Below,
    /// Somebody, applying the child again as it is.
    AsItIs,
}

/// How many children the look below goes through, one after another.
const CHILDREN: u64 = 5;

/// A parent with a child for each of several cards, all behind the text
/// there; somebody applies the parent again, whose look below goes
/// forward `turns` turns, and then the child of `card` is applied with
/// another param by somebody who does not give way. Or, with the text
/// as it was, somebody applies that child again as it is, and goes
/// forward so. Whether whoever looked was over by then, and the param
/// that child is left with.
async fn looked_at_as_its_params_change(looks: Looks, card: u64, turns: usize) -> (bool, serde_json::Value) {
    let rig = Rig::new().await;
    rig.library.put("many", MANY);
    rig.library.put("child", OLD);
    let top = rig.apply(MANY, json!({})).await.instance;
    active(&rig, &top).await;
    for id in 1..=CHILDREN {
        rig.journal.emit("board.item.moved", Source::default(), json!({ "id": id })).await.unwrap();
    }
    handled(&rig, "on.ready", CHILDREN as usize).await;
    let live = rig.engine.instances(InstanceFilter { live_only: true, ..Default::default() }).await.unwrap();
    for child in live.iter().filter(|one| one.key.starts_with("kid-")) {
        active(&rig, &child.id).await;
    }
    let kid = live.iter().find(|one| one.key == format!("kid-{card}")).expect("the child").id.clone();
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    if looks == Looks::Below {
        rig.library.put("child", &says());
    }

    let mut stepped = Stepped::holding(&rig.journal);
    let looking = match looks {
        Looks::Below => asks(&rig, rig.request(MANY, json!({}))),
        Looks::AsItIs => {
            let (engine, kid) = (rig.engine.clone(), kid.clone());
            tokio::spawn(async move { engine.apply_again(&kid, user(), false).await })
        }
    };
    for _ in 0..turns {
        stepped.turn(false).await;
    }
    let over = looking.is_finished();
    // Asked with force: its steps run again with the param, at once.
    let second = rig.engine.apply(Apply {
        key: Some(format!("kid-{card}")),
        parent: Some(top.clone()),
        params: kernel::fields(json!({ "id": 90 })),
        force: true,
        ..Apply::new("child", user())
    });
    applied_at_once(&mut stepped, Box::pin(second)).await;
    stepped.through(false, looking).await.expect("the journal is open").unwrap().unwrap();
    rested(&mut stepped).await;
    drop(stepped);
    (over, rig.instance(&kid).await.params["id"].clone())
}

/// Plays every amount of turns, from none until whoever looks is over
/// when the child is given another param, and holds each to the child
/// being left with that param.
async fn every_look(looks: Looks, card: u64) {
    for turns in 0..=MOST {
        let case = format!("{looks:?}, card {card}, {turns} turns");
        let (over, id) = soon(&case, looked_at_as_its_params_change(looks, card, turns)).await;
        assert_eq!(id, 90, "{case}: the param it was given as it was looked at");
        if over {
            return;
        }
    }
    panic!("{looks:?}: whoever looked was not over after {MOST} turns: the amounts no longer reach past it");
}

#[tokio::test]
async fn the_look_below_asks_with_the_params_a_child_has_when_its_turn_has_come() {
    // The children are listed once and gone through in turn: whichever
    // the look comes to last was read longest before it has its turn.
    every_look(Looks::Below, 1).await;
    every_look(Looks::Below, CHILDREN).await;
}

#[tokio::test]
async fn an_instance_applied_again_as_it_is_is_asked_with_the_params_it_has_when_its_turn_has_come() {
    every_look(Looks::AsItIs, 1).await;
}
