//! What of what a comparison counts says that a thing was accepted, a
//! case for each way it can come out; what is wrong with a count that
//! says so; and what makes an event one thing that happened to its thing.

use kernel::Event;
use kernel::compared::{ACCEPTED, Counted, GIVEN, accepted};
use serde_json::{Value, json};

fn words(said: &[&str]) -> Vec<String> {
    said.iter().map(|word| word.to_string()).collect()
}

#[test]
fn a_thing_is_accepted_by_what_its_event_came_to_or_the_last_of_several_and_by_nothing_else() {
    // (what the count accepts, what the event came to, whether it says the thing was accepted)
    let cases: &[(&[&str], &str, bool)] = &[
        // A review that passed, counted by the last word of its topic.
        (&["passed"], "passed", true),
        (&["passed"], "failed", false),
        (&["passed"], "undecided", false),
        // A move, counted by where it was and where it came to: the last says.
        (&["done"], "review>done", true),
        (&["done"], "done>review", false),
        (&["done"], "ready>doing", false),
        // Any of several words.
        (&["approved", "done"], "approved", true),
        (&["approved", "done"], "doing>done", true),
        // A count that accepts nothing says nothing of it; an event that came to nothing is not accepted.
        (&[], "passed", false),
        (&["passed"], "", false),
        (&[""], "", false),
        // A word is the whole of a field's value, never a part of it.
        (&["pass"], "passed", false),
        (&["done"], "undone", false),
        (&["Done"], "done", false),
    ];
    for (accepts, came, wanted) in cases {
        assert_eq!(accepted(&words(accepts), came), *wanted, "{accepts:?} of `{came}`");
    }
}

#[test]
fn a_count_cannot_be_called_what_is_always_counted_or_accept_what_no_event_comes_to() {
    let count = |name: &str, accepts: &[&str]| Counted {
        name: name.into(),
        topic: "review.passed".into(),
        thing: "card".into(),
        by: Vec::new(),
        accepts: words(accepts),
        once: Vec::new(),
    };
    assert_eq!(count("verdict", &["passed"]).wrong(), None);
    assert_eq!(count("verdict", &[]).wrong(), None);
    let named = count(ACCEPTED, &["passed"]).wrong().unwrap();
    assert!(named.contains("a count cannot be called `accepted`"), "{named}");
    assert!(count(GIVEN, &[]).wrong().unwrap().contains("a count cannot be called `given`"));
    for wrong in [" ", "review>done"] {
        let said = count("verdict", &[wrong]).wrong().unwrap();
        assert!(said.contains("which is not what an event can come to"), "{wrong}: {said}");
    }
}

#[test]
fn an_event_is_one_by_the_fields_its_count_says_and_every_event_is_one_where_it_says_none() {
    let event = |data: Value| Event {
        seq: 1,
        ts: 0,
        topic: "card.reverted".into(),
        source: Default::default(),
        data,
    };
    let count = |once: &[&str]| Counted {
        name: "after".into(),
        topic: "card.reverted".into(),
        thing: "card".into(),
        once: words(once),
        ..Default::default()
    };
    // (the fields that make one, what the event says, what it is one by)
    let cases: &[(&[&str], Value, Option<&str>)] = &[
        // Nothing said: every event is counted.
        (&[], json!({ "card": "shop-1", "by": "shop-2" }), None),
        // By the card that did it: said again by the same card, it is the same.
        (&["by"], json!({ "card": "shop-1", "by": "shop-2" }), Some("shop-2")),
        // By a dotted path, as a thing is read.
        (&["of.card"], json!({ "card": "shop-1", "of": { "card": "shop-3" } }), Some("shop-3")),
        // By two, each in its place, so that one missing is not taken for the other.
        (&["by", "at"], json!({ "card": "shop-1", "by": "shop-2" }), Some("shop-2\n")),
        (&["by", "at"], json!({ "card": "shop-1", "at": "shop-2" }), Some("\nshop-2")),
        // An event that says none of them is one of its own.
        (&["by"], json!({ "card": "shop-1" }), None),
    ];
    for (once, data, wanted) in cases {
        assert_eq!(count(once).once_by(&event(data.clone())).as_deref(), *wanted, "{once:?} of {data}");
    }
    let wrong = count(&[" "]).wrong().unwrap();
    assert!(wrong.contains("by a field with no name"), "{wrong}");
    assert_eq!(count(&["by"]).wrong(), None);
}

#[test]
fn an_event_is_about_each_thing_its_field_holds_where_that_is_a_list_and_about_none_where_it_says_nothing() {
    let event = |data: Value| Event {
        seq: 1,
        ts: 0,
        topic: "card.changed_again".into(),
        source: Default::default(),
        data,
    };
    let count = Counted {
        name: "after".into(),
        topic: "card.changed_again".into(),
        thing: "cards".into(),
        ..Default::default()
    };
    // (what the event says, the things it is about)
    let cases: &[(Value, &[&str])] = &[
        (json!({ "cards": "shop-1" }), &["shop-1"]),
        (json!({ "cards": ["shop-1", "shop-2"] }), &["shop-1", "shop-2"]),
        // Nothing, nothing said, and empty words in a list are about nothing.
        (json!({ "cards": [] }), &[]),
        (json!({}), &[]),
        (json!({ "cards": ["", "shop-3", "shop-3"] }), &["shop-3"]),
        // Something that is not words, as it is written.
        (json!({ "cards": [4] }), &["4"]),
    ];
    for (data, wanted) in cases {
        assert_eq!(count.things(&event(data.clone())), words(wanted), "{data}");
    }
}
