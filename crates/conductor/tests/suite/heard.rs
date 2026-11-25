//! What a handler hears is held to the events there are when a text is
//! applied, and is not held against a text that was kept; and a kept text
//! whose actions are gone says why.

use blueprint::{ActionSpec, EventSpec, ValueType};
use conductor::{ActionResult, Engine, InstanceStatus, Registry};
use kernel::BoxFut;
use serde_json::{Value, json};

use crate::Rig;

const WATCHER: &str = "
blueprint: watcher
on:
  shuffled:
    event: board.item.shuffled
    for: { id: c-1 }
    when: ${ event.data.by == \"ada\" }
    do: noop
";

/// A registry whose events are these and no others.
fn knowing(events: Vec<EventSpec>) -> Registry {
    let mut registry = Registry::new();
    registry.emits(events).emits_no_more();
    registry
}

fn shuffled() -> EventSpec {
    EventSpec::new("board.item.shuffled", "A card was put somewhere else in its column.")
        .field("id", ValueType::String, "the card")
        .maybe("by", ValueType::String, "who did it")
}

#[tokio::test]
async fn a_handler_is_held_to_its_event_when_its_text_is_applied() {
    let rig = Rig::new().await;
    let engine =
        Engine::new(rig.journal.clone(), rig.logs.clone(), knowing(vec![shuffled()]), rig.library.clone())
            .await
            .unwrap();
    engine.apply(rig.request(WATCHER, json!({}))).await.unwrap();
    let refused =
        engine
            .apply(rig.request(
                &WATCHER.replace("for: { id:", "for: { card:").replace("watcher", "other"),
                json!({}),
            ))
            .await
            .unwrap_err();
    assert!(
        refused.message.contains("`for:` names `card`, which `board.item.shuffled` does not carry"),
        "{}",
        refused.message
    );
    // A registry that was not told it knows every event holds a handler to none:
    // one put together for a test knows only some.
    rig.apply(&WATCHER.replace("board.item.shuffled", "board.item.anything"), json!({})).await;
}

#[tokio::test]
async fn a_text_that_was_kept_is_taken_up_again_whatever_has_become_of_what_its_handlers_hear() {
    let rig = Rig::new().await;
    let first =
        Engine::new(rig.journal.clone(), rig.logs.clone(), knowing(vec![shuffled()]), rig.library.clone())
            .await
            .unwrap();
    let applied = first.apply(rig.request(WATCHER, json!({}))).await.unwrap();
    rig.event("instance.active", |_| true).await;
    first.halt();

    // The next conductor emits no such event. The text could not be
    // applied to it now, and is still what the instance runs: a handler
    // that never fires is no reason to stop what is running.
    let second = Engine::new(rig.journal.clone(), rig.logs.clone(), knowing(Vec::new()), rig.library.clone())
        .await
        .unwrap();
    let checked = second.check(WATCHER, None);
    assert!(
        checked
            .errors()
            .any(|error| error.message.starts_with("there is no event called `board.item.shuffled`")),
        "{:?}",
        checked.diagnostics
    );
    assert_eq!(second.resume().await.unwrap(), 1);
    let instance = second.instance(&applied.instance).await.unwrap();
    assert_eq!(instance.status, InstanceStatus::Active, "{:?}", instance.error);
}

const SPEAKER: &str = "
blueprint: speaker
steps:
  say: { do: board.comment, with: { id: c-1 } }
  more: { do: thing.vanished }
on:
  tick:
    event: clock.tick
    do: noop
";

#[tokio::test]
async fn a_kept_text_whose_action_was_taken_away_says_when_and_why() {
    let rig = Rig::new().await;
    let mut registry = Registry::new();
    for name in ["board.comment", "thing.vanished"] {
        registry.add_fn(
            ActionSpec::new(name, "Something there was once.").open(),
            |_, _| -> BoxFut<'static, ActionResult> { Box::pin(async { Ok(Value::Null) }) },
        );
    }
    let first =
        Engine::new(rig.journal.clone(), rig.logs.clone(), registry, rig.library.clone()).await.unwrap();
    let applied = first.apply(rig.request(SPEAKER, json!({}))).await.unwrap();
    rig.event("instance.active", |_| true).await;
    first.halt();

    let second = Engine::new(rig.journal.clone(), rig.logs.clone(), Registry::new(), rig.library.clone())
        .await
        .unwrap();
    assert_eq!(second.resume().await.unwrap(), 0);
    let instance = second.instance(&applied.instance).await.unwrap();
    assert_eq!(instance.status, InstanceStatus::Failed);
    let said = instance.error.unwrap();
    assert!(said.starts_with("cannot resume: `speaker` has 2 problem(s)"), "{said}");
    // What the list kept with the contract says of the one that is on it,
    assert!(
        said.contains(
            "`board.comment` was taken away on 2026-11-12: Saying something on a record is `record.comment`, \
             for a card as for any other record."
        ),
        "{said}"
    );
    // and of the other nothing is said that is not known.
    assert!(!said.contains("`thing.vanished` was taken away"), "{said}");
    assert!(!said.contains("earlier version"), "the reason is known, so none is guessed at: {said}");
}

#[tokio::test]
async fn a_kept_text_that_no_longer_checks_for_a_reason_nobody_wrote_down_may_be_from_an_earlier_version() {
    let rig = Rig::new().await;
    let mut registry = Registry::new();
    registry.add_fn(
        ActionSpec::new("thing.vanished", "Something there was once."),
        |_, _| -> BoxFut<'static, ActionResult> { Box::pin(async { Ok(Value::Null) }) },
    );
    let first =
        Engine::new(rig.journal.clone(), rig.logs.clone(), registry, rig.library.clone()).await.unwrap();
    let source = SPEAKER.replace("  say: { do: board.comment, with: { id: c-1 } }\n", "");
    let applied = first.apply(rig.request(&source, json!({}))).await.unwrap();
    rig.event("instance.active", |_| true).await;
    first.halt();
    let second = Engine::new(rig.journal.clone(), rig.logs.clone(), Registry::new(), rig.library.clone())
        .await
        .unwrap();
    assert_eq!(second.resume().await.unwrap(), 0);
    let said = second.instance(&applied.instance).await.unwrap().error.unwrap();
    assert!(
        said.ends_with(&format!(
            "The text may have been written for an earlier version of this program than this one, which is {}",
            brand::VERSION
        )),
        "{said}"
    );
}

/// A text that starts a child that listens, and asks something there
/// was once.
const PARENT: &str = "
blueprint: parent
steps:
  kid: { spawn: kid, as: kid }
  more: { do: thing.vanished, needs: [kid] }
on:
  tick:
    event: clock.tick
    do: noop
";

const KID: &str = "
blueprint: kid
on:
  tick:
    event: clock.tick
    do: noop
";

#[tokio::test]
async fn a_kept_text_that_no_longer_checks_at_a_start_fails_and_takes_down_what_it_started() {
    let rig = Rig::new().await;
    rig.library.put("kid", KID);
    let mut registry = Registry::new();
    registry.add_fn(
        ActionSpec::new("thing.vanished", "Something there was once."),
        |_, _| -> BoxFut<'static, ActionResult> { Box::pin(async { Ok(Value::Null) }) },
    );
    let first =
        Engine::new(rig.journal.clone(), rig.logs.clone(), registry, rig.library.clone()).await.unwrap();
    let applied = first.apply(rig.request(PARENT, json!({}))).await.unwrap();
    first.wait_run(applied.run.as_deref().unwrap()).await.unwrap();
    let filter = |live_only| conductor::InstanceFilter { live_only, limit: 10, ..Default::default() };
    let alive = first.instances(filter(true)).await.unwrap();
    let kid = alive.iter().find(|one| one.blueprint == "kid").expect("the child is alive").id.clone();
    first.halt();
    first.halted().await;

    // The next has none of what the parent asks; the child asks nothing.
    let second = Engine::new(rig.journal.clone(), rig.logs.clone(), Registry::new(), rig.library.clone())
        .await
        .unwrap();
    assert_eq!(second.resume().await.unwrap(), 0, "nothing is taken up");
    let parent = second.instance(&applied.instance).await.unwrap();
    assert_eq!(parent.status, InstanceStatus::Failed);
    assert!(parent.error.unwrap().starts_with("cannot resume: `parent` has 1 problem(s)"));
    // What it started is taken down with it, and listens no more beside
    // whatever is applied in its place.
    let child = second.instance(&kid).await.unwrap();
    assert_eq!(child.status, InstanceStatus::Stopped, "{child:?}");
    assert!(second.instances(filter(true)).await.unwrap().is_empty(), "nothing is alive");
    // And that the parent is over is said once what was under it is.
    let over = rig
        .event("instance.finished", |event| event.source.instance.as_deref() == Some(&applied.instance))
        .await;
    assert_eq!(over.data["status"], "failed", "{over:?}");
    second.halt();
}
