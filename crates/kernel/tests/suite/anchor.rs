//! What a comment on a selection of a document's text is about: each part
//! checked where it is written, and bounded.

use kernel::anchor::{KEYS, MOST_BESIDE, MOST_HEADINGS, MOST_QUOTED, WITHIN, read};
use serde_json::{Value, json};

fn refused(value: Value) -> String {
    read(Some(&value)).expect_err(&format!("{value} is refused")).message
}

#[test]
fn nothing_said_is_a_comment_on_the_whole_record() {
    assert_eq!(read(None).unwrap(), None);
    assert_eq!(read(Some(&Value::Null)).unwrap(), None);
}

#[test]
fn an_anchor_is_read_with_what_it_leaves_unsaid_empty() {
    let anchor = read(Some(&json!({ "quote": "the second step" }))).unwrap().unwrap();
    assert_eq!(anchor.quote, "the second step");
    assert!(anchor.before.is_empty() && anchor.after.is_empty() && anchor.under.is_empty());
    let whole = json!({ "quote": "the second step", "before": "Then ", "after": " is", "under": ["Install", "On Linux"] });
    let anchor = read(Some(&whole)).unwrap().unwrap();
    assert_eq!(anchor.under, vec!["Install", "On Linux"]);
    // Written as it was read, and nothing that is empty.
    assert_eq!(serde_json::to_value(&anchor).unwrap(), whole);
    assert_eq!(
        serde_json::to_value(read(Some(&json!({ "quote": "x", "before": "" }))).unwrap()).unwrap(),
        json!({ "quote": "x" })
    );
}

#[test]
fn each_part_is_refused_in_a_sentence_where_it_is_not_what_it_may_be() {
    let cases: &[(Value, &str)] = &[
        (json!("the step"), "says which words"),
        (json!({}), "`anchor.quote` is words"),
        (json!({ "quote": "  " }), "is something"),
        (json!({ "quote": "one\ntwo" }), "on one line"),
        (json!({ "quote": "x".repeat(MOST_QUOTED + 1) }), "400 characters at most"),
        (json!({ "quote": "x", "before": "y".repeat(MOST_BESIDE + 1) }), "`anchor.before` is 40 characters"),
        (json!({ "quote": "x", "after": 3 }), "`anchor.after` is words"),
        (json!({ "quote": "x", "under": "Install" }), "as a list"),
        (json!({ "quote": "x", "under": vec!["h"; MOST_HEADINGS + 1] }), "6 headings at most"),
        (json!({ "quote": "x", "under": [""] }), "none of them is nothing"),
        (json!({ "quote": "x", "under": ["a\nb"] }), "on one line"),
        (json!({ "quote": "x", "at": 4 }), "not `at`"),
    ];
    for (value, says) in cases {
        let said = refused(value.clone());
        assert!(said.contains(says), "{value}: `{said}` says `{says}`");
    }
}

#[test]
fn a_quote_is_counted_in_characters_and_not_bytes() {
    let quote = "é".repeat(MOST_QUOTED);
    assert_eq!(read(Some(&json!({ "quote": quote }))).unwrap().unwrap().quote.chars().count(), MOST_QUOTED);
}

#[test]
fn what_is_said_of_what_is_written_inside_is_every_key_read_and_no_other() {
    let said: Vec<&str> = WITHIN.iter().map(|(key, ..)| *key).collect();
    assert_eq!(said, KEYS);
}
