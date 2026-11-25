//! Events as they are described: what one carries, what is wrong with one
//! that was emitted otherwise, and what a blueprint's own `emit` says.

use blueprint::events::{self, emitted};
use blueprint::{EventList, EventSpec, NoCatalog, ValueType};
use serde_json::json;

fn moved() -> EventList {
    EventList::new(vec![
        EventSpec::new("board.item.moved", "A card was moved.")
            .field("id", ValueType::String, "the card")
            .field("to", ValueType::String, "where to")
            .maybe("note", ValueType::String, "what was said"),
        EventSpec::new("instance.state", "State changed.").open(),
    ])
}

#[test]
fn an_event_emitted_as_it_is_described_has_nothing_wrong_with_it() {
    let list = moved();
    assert_eq!(list.fault("board.item.moved", &json!({ "id": "c-1", "to": "done" })), None);
    assert_eq!(list.fault("board.item.moved", &json!({ "id": "c-1", "to": "done", "note": "ok" })), None);
    // A field that is there some of the time may be nothing.
    assert_eq!(list.fault("board.item.moved", &json!({ "id": "c-1", "to": "done", "note": null })), None);
    // One that carries names of its emitter's own is held to nothing more.
    assert_eq!(list.fault("instance.state", &json!({ "rounds": 3 })), None);
    // And what a blueprint says is not this program's to describe.
    assert_eq!(list.fault("shop.restocked", &json!({ "anything": 1 })), None);
}

#[test]
fn what_is_wrong_with_an_event_emitted_otherwise_is_said() {
    let list = moved();
    let fault = |topic: &str, data| list.fault(topic, &data).unwrap();
    assert!(
        fault("board.item.shuffled", json!({}))
            .starts_with("the event `board.item.shuffled` was emitted and is not described")
    );
    assert_eq!(
        fault("board.item.moved", json!({ "id": "c-1", "to": "done", "by": "ada" })),
        "the event `board.item.moved` was emitted with `by`, which its description does not list"
    );
    assert_eq!(
        fault("board.item.moved", json!({ "id": "c-1" })),
        "the event `board.item.moved` was emitted without `to`, which its description says is always there"
    );
    assert_eq!(
        fault("board.item.moved", json!({ "id": "c-1", "to": null })),
        "the event `board.item.moved` was emitted without `to`, which its description says is always there"
    );
    assert_eq!(
        fault("board.item.moved", json!({ "id": 7, "to": "done" })),
        "the event `board.item.moved` was emitted with `id` as 7, and its description says string"
    );
}

#[test]
fn a_group_of_topics_takes_one_word_for_a_star_and_any_number_for_two() {
    assert!(events::matches("board.item.moved", "board.item.moved"));
    assert!(events::matches("board.item.*", "board.item.moved"));
    assert!(!events::matches("board.*", "board.item.moved"));
    assert!(events::matches("board.**", "board.item.moved"));
    assert!(events::matches("**", "board.item.moved"));
    assert!(!events::matches("board.item.*", "board.columns"));
}

#[test]
fn what_a_blueprint_says_with_emit_is_the_fields_its_step_writes() {
    let source = "
blueprint: says
steps:
  tell: { do: emit, with: { topic: shop.restocked, sku: b-12, left: 40 } }
  work:
    task:
      inner: { do: emit, with: { topic: shop.counted, total: 3 } }
  slow:
    do: noop
    on_fail:
      - emit: { topic: shop.failed, reason: \"${self.error}\" }
  loose: { do: emit, with: { topic: \"shop.${params.what}\", x: 1 } }
params:
  what: thing
on:
  heard:
    event: shop.restocked
    do: emit
    with: { topic: shop.heard }
";
    let blueprint = blueprint::check(source, &NoCatalog).blueprint.unwrap();
    let said: Vec<(String, Vec<String>)> = emitted(&blueprint)
        .into_iter()
        .map(|(topic, said)| (topic, said.fields.into_iter().collect()))
        .collect();
    let has = |topic: &str, fields: &[&str]| {
        let fields: Vec<String> = fields.iter().map(|field| field.to_string()).collect();
        assert!(said.contains(&(topic.to_string(), fields)), "{topic} in {said:?}");
    };
    has("shop.restocked", &["left", "sku"]);
    has("shop.counted", &["total"]);
    has("shop.failed", &["reason"]);
    has("shop.heard", &[]);
    // A topic that is filled in when it runs says nothing that can be read here.
    assert_eq!(said.len(), 4, "{said:?}");
}

#[test]
fn a_handler_that_says_the_very_event_it_listens_for_is_told_it_will_not_hear_it() {
    let warnings = |source: &str| -> Vec<(&'static str, String)> {
        let checked = blueprint::check(source, &NoCatalog);
        assert_eq!(checked.errors().count(), 0, "{:?}", checked.diagnostics);
        checked
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "own-event")
            .map(|diagnostic| (diagnostic.code, diagnostic.message.clone()))
            .collect()
    };
    // Trying to run itself again by saying what it listens for.
    let looks = "\
blueprint: pool
on:
  look:
    event: pool.look
    steps:
      work: { do: noop }
      again: { do: emit, with: { topic: pool.look } }
";
    let found = warnings(looks);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].1,
        "`look` says `pool.look`, which is what it listens for, and a handler does not hear what its own runs \
         say: saying it does not run `look` again. Have another handler listen for it, or run this one again \
         with `every:`."
    );
    // Said by another handler, it is heard: nothing to say.
    let two = "\
blueprint: pool
on:
  look: { event: pool.look, do: noop }
  done: { event: pool.done, do: emit, with: { topic: pool.look } }
";
    assert_eq!(warnings(two), []);
    // One that answers within the group it listens to relies on not
    // hearing itself, and is not told what it knows.
    let parrot = "\
blueprint: parrot
on:
  echo: { event: \"chatter.*\", do: emit, with: { topic: chatter.reply } }
";
    assert_eq!(warnings(parrot), []);
}
