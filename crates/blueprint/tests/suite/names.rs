//! What a blueprint names, and what follows from what each names: which
//! are applied by somebody, and which are parts of which.

use std::collections::BTreeMap;
use std::path::Path;

use blueprint::names::{self, How, Named, Runs};
use blueprint::{ActionSpec, ValueType};
use serde::Deserialize;

/// The blueprints there are, for these cases.
const KNOWN: &[&str] =
    &["shop/round", "shop/attached", "shop/called", "shop/spawned", "shop/usual", "shop/rule", "shop/listed"];

fn known(name: &str) -> bool {
    KNOWN.contains(&name)
}

/// An action that reaches a blueprint by itself, and one that reaches none.
fn actions() -> Vec<ActionSpec> {
    vec![
        ActionSpec::new("work.give", "Give somebody a piece of work.")
            .arg("to", ValueType::String, "whom")
            .reaching("shop/rule"),
        ActionSpec::new("work.count", "Count the work.").arg("of", ValueType::String, "whose"),
    ]
}

fn named(source: &str) -> Vec<(String, String)> {
    names::of(source, &known, &actions()).into_iter().map(|one| (one.name, one.how.said())).collect()
}

fn said(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(name, how)| ((*name).to_string(), (*how).to_string())).collect()
}

#[test]
fn a_text_names_what_it_attaches_calls_and_spawns_what_a_param_is_unless_said_what_its_actions_reach_and_what_it_writes_whole()
 {
    let text = "\
blueprint: shop/round
params:
  review: { type: string, default: shop/usual }
  limit: { type: int, default: 2 }
uses:
  logs: shop/attached
steps:
  first:
    call: shop/called
  second:
    spawn: shop/spawned
    as: second
  third:
    do: work.give
    with: { to: somebody }
  fourth:
    do: work.count
    with:
      of:
        - { order: 10, does: shop/listed }
        - { order: 20, does: shop/called }
";
    assert_eq!(
        named(text),
        said(&[
            ("shop/attached", "attaches it"),
            ("shop/called", "calls it"),
            ("shop/spawned", "spawns it"),
            ("shop/usual", "the default of `review`"),
            ("shop/rule", "asks `work.give`, which reaches it"),
            // One that a step calls is not also said to be written.
            ("shop/listed", "writes it as a value"),
        ])
    );
}

#[test]
fn what_names_nothing_names_nothing() {
    let cases: &[(&str, &str)] = &[
        ("a comment", "blueprint: shop/round\n# It is like shop/called.\nsteps:\n  one: { do: noop }\n"),
        (
            "a sentence",
            "blueprint: shop/round\ndescription: Does what shop/called does, by `shop/spawned`.\nsteps:\n  one: { do: noop }\n",
        ),
        (
            "a key",
            "blueprint: shop/round\nsteps:\n  one:\n    do: work.count\n    with: { of: { shop/called: true } }\n",
        ),
        (
            "an expression",
            "blueprint: shop/round\nparams:\n  begins: { type: string, default: \"\" }\nsteps:\n  one:\n    when: ${ params.begins == \"shop/called\" }\n    do: noop\n",
        ),
        (
            "a name with a blank in it",
            "blueprint: shop/round\nparams:\n  which: { type: string, default: called }\nsteps:\n  one:\n    call: \"shop/${params.which}\"\n",
        ),
        (
            "itself",
            "blueprint: shop/round\nsteps:\n  one:\n    do: work.count\n    with: { of: shop/round }\n",
        ),
        (
            "a list a param is unless said, which is a setting somebody chooses among",
            "blueprint: shop/round\nparams:\n  rows:\n    type: list\n    default:\n      - { name: one, blueprint: shop/called }\nsteps:\n  one: { do: work.count }\n",
        ),
        (
            "a mapping a param is unless said",
            "blueprint: shop/round\nparams:\n  rows: { type: object, default: { one: shop/spawned } }\nsteps:\n  one: { do: work.count }\n",
        ),
        ("a blueprint there is not", "blueprint: shop/round\nsteps:\n  one:\n    call: shop/gone\n"),
        ("an action that reaches none", "blueprint: shop/round\nsteps:\n  one: { do: work.count }\n"),
        ("an action nobody described", "blueprint: shop/round\nsteps:\n  one: { do: work.unknown }\n"),
        ("a text that does not read", "blueprint: shop/round\nsteps: [\n"),
        ("a text that is no blueprint", "- shop/called\n- shop/spawned\n"),
    ];
    for (what, text) in cases {
        assert_eq!(named(text), said(&[]), "{what}");
    }
}

#[test]
fn a_name_is_found_wherever_a_step_or_a_value_is_written() {
    // In a task's steps, a handler's, what is done around a step and as
    // the text ends; and as a value in its state and in what it gives out.
    let text = "\
blueprint: shop/round
state:
  usual: shop/usual
steps:
  all:
    task:
      inner:
        call: shop/called
        on_fail:
          - { do: work.give, with: { to: somebody } }
on:
  heard:
    event: shop.said
    steps:
      one:
        spawn: shop/spawned
        as: one
on_stop:
  - { do: work.count, with: { of: shop/listed } }
outputs:
  what: shop/attached
";
    let found: Vec<String> = named(text).into_iter().map(|(name, _)| name).collect();
    assert_eq!(
        found,
        ["shop/called", "shop/spawned", "shop/rule", "shop/usual", "shop/listed", "shop/attached"]
    );
}

#[test]
fn a_way_is_handed_out_as_a_word_and_what_it_is_of() {
    let named = Named { name: "shop/rule".into(), how: How::Asks("work.give".into()) };
    assert_eq!(
        serde_json::to_value(&named).unwrap(),
        serde_json::json!({ "name": "shop/rule", "how": { "asks": "work.give" } })
    );
    assert_eq!(serde_json::to_value(How::Calls).unwrap(), serde_json::json!("calls"));
}

/// One case of the table the client's list is held to as well
/// (`ui/src/blueprintparts.json`).
#[derive(Deserialize)]
struct Case {
    says: String,
    runs: Runs,
    applied: Vec<String>,
    tops: Vec<String>,
    parts: BTreeMap<String, Vec<String>>,
    part_of: BTreeMap<String, Vec<String>>,
    unreached: Vec<String>,
}

#[test]
fn what_is_applied_and_what_is_a_part_of_what_come_to_what_the_clients_list_comes_to() {
    let table = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/blueprintparts.json");
    let cases: Vec<Case> =
        serde_json::from_str(&std::fs::read_to_string(table).unwrap()).expect("a list of cases");
    assert!(cases.len() >= 10, "the table was found");
    for case in cases {
        let says = &case.says;
        assert_eq!(names::tops(&case.runs, &case.applied), case.tops, "{says}: what is applied");
        for (name, parts) in &case.parts {
            assert_eq!(&names::parts(name, &case.runs), parts, "{says}: the parts of {name}");
        }
        for (name, of) in &case.part_of {
            assert_eq!(
                &names::part_of(name, &case.runs, &case.applied),
                of,
                "{says}: what {name} is a part of"
            );
        }
        assert_eq!(names::unreached(&case.runs, &case.applied), case.unreached, "{says}: what is in no list");
    }
}
