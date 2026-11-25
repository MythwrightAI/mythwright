//! What each backend of a router is sent, from what was written: a table
//! of what is handed and what it comes to.

use hosting::Backend;
use hosting::shares::shares;
use serde_json::{Value, json};

fn service(name: &str, weight: u32) -> Backend {
    Backend { service: Some(name.to_string()), address: None, weight }
}

#[test]
fn a_weight_is_a_number_and_the_rest_is_what_the_others_leave_of_a_hundred() {
    let cases: Vec<(Value, Vec<Backend>)> = vec![
        // Numbers are taken as they are, whatever they come to together.
        (
            json!([{ "service": "a", "weight": 3 }, { "service": "b", "weight": 1 }]),
            vec![service("a", 3), service("b", 1)],
        ),
        (json!([]), vec![]),
        // The rest of a hundred, wherever in the list it is said.
        (
            json!([{ "service": "stable", "weight": "rest" }, { "service": "next", "weight": 10 }]),
            vec![service("stable", 90), service("next", 10)],
        ),
        (
            json!([{ "service": "next", "weight": 50 }, { "service": "stable", "weight": "rest" }]),
            vec![service("next", 50), service("stable", 50)],
        ),
        // Everything to the other leaves nothing, which is a weight: it keeps
        // what it has and is sent nothing new.
        (
            json!([{ "service": "stable", "weight": "rest" }, { "service": "next", "weight": 100 }]),
            vec![service("stable", 0), service("next", 100)],
        ),
        // Alone, the rest is the whole.
        (json!([{ "service": "only", "weight": "rest" }]), vec![service("only", 100)]),
        // Of several others, what they leave together.
        (
            json!([
                { "service": "a", "weight": 20 },
                { "address": "127.0.0.1:9", "weight": 30 },
                { "service": "c", "weight": "rest" }
            ]),
            vec![
                service("a", 20),
                Backend { service: None, address: Some("127.0.0.1:9".into()), weight: 30 },
                service("c", 50),
            ],
        ),
        // One that names nothing is left out: no previous version, say. What
        // it would have been sent still counts against the rest, as written.
        (
            json!([{ "service": null, "weight": "rest" }, { "service": "next", "weight": 10 }]),
            vec![service("next", 10)],
        ),
        (
            json!([{ "service": "stable", "weight": "rest" }, { "service": null, "weight": 10 }]),
            vec![service("stable", 90)],
        ),
    ];
    for (written, comes_to) in cases {
        assert_eq!(shares(&written), Ok(comes_to), "{written}");
    }
}

#[test]
fn what_makes_no_sense_of_weights_is_refused_in_a_sentence() {
    let cases: Vec<(Value, &str)> = vec![
        (json!("stable"), "it is a list, of `{ service: <name>, weight: <n> }`"),
        (json!(null), "it is a list, of"),
        (
            json!([{ "service": "a", "weight": "rest" }, { "service": "b", "weight": "rest" }]),
            "2 of them say `weight: rest`, and what the others leave of a hundred can be given to one",
        ),
        (
            json!([{ "service": "a", "weight": "rest" }, { "service": "b", "weight": 120 }]),
            "the others' weights come to 120, which leaves nothing of a hundred for the one that says `weight: rest`",
        ),
        (
            json!([{ "service": "a", "weight": "most" }, { "service": "b", "weight": 1 }]),
            "one says its weight is `most`; a weight is a number, or `rest` for what the others leave of a hundred",
        ),
        // What was always refused still is, in the reader's own words.
        (json!([{ "service": "a" }]), "missing field `weight`"),
        (json!([{ "service": "a", "weight": -1 }]), "invalid value: integer `-1`"),
    ];
    for (written, why) in cases {
        let refused = shares(&written).unwrap_err();
        assert!(refused.contains(why), "{written}: {refused}");
    }
}
