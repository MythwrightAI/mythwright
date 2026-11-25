//! `labels:` on a step that starts something: what everything measured
//! under it carries.

use blueprint::labels::{self, Labels, MAX_VALUE, TAKEN};
use serde_json::{Value, json};

use crate::{check, error_codes, first_error};

/// A blueprint whose one step calls another, with `labels` as its `labels:`.
fn calling(labels: &str) -> String {
    format!(
        "blueprint: trial\npermissions: [blueprints]\nparams:\n  item:\nsteps:\n  a:\n    call: lane\n    \
         labels: {labels}\n"
    )
}

fn carried(pairs: &[(&str, &str)]) -> Labels {
    pairs.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
}

#[test]
fn labels_are_read_on_a_call_a_spawn_and_a_task_as_names_and_the_value_each_has() {
    let checked = check(&calling("{ experiment: \"cart-${params.item}\", variant: a, round: 2 }"));
    assert_eq!(checked.diagnostics, []);
    let blueprint = checked.into_result().unwrap();
    assert_eq!(
        Value::Object(blueprint.flow.steps[0].labels.clone()),
        json!({ "experiment": "cart-${params.item}", "variant": "a", "round": 2 })
    );

    let others = "\
blueprint: trial
permissions: [blueprints, shell]
steps:
  started: { spawn: lane, labels: { variant: b } }
  built:
    labels: { variant: c }
    task:
      compile: { do: shell, with: { run: make } }
  plain: { do: shell, with: { run: make } }
";
    let checked = check(others);
    assert_eq!(checked.diagnostics, []);
    let steps = checked.into_result().unwrap().flow.steps;
    assert_eq!(Value::Object(steps[0].labels.clone()), json!({ "variant": "b" }));
    assert_eq!(Value::Object(steps[1].labels.clone()), json!({ "variant": "c" }));
    assert!(steps[2].labels.is_empty(), "a step that says nothing carries nothing of its own");
}

#[test]
fn a_name_a_measurement_uses_for_itself_is_refused_with_what_is_taken() {
    for taken in TAKEN {
        let source = calling(&format!("{{ {taken}: a }}"));
        assert_eq!(error_codes(&source), ["bad-name"], "{taken}");
        let message = first_error(&source);
        assert!(message.contains(&format!("`{taken}` cannot be a label")), "{message}");
        assert!(message.contains("measurements already say that of themselves"), "{message}");
        assert!(message.contains(&format!("Taken are: {}", TAKEN.join(", "))), "{message}");
        assert!(message.contains("such as `variant` or `experiment`"), "{message}");
    }
    // What the product measures by today, each of which would be taken for a label.
    for name in ["action", "step", "blueprint", "project", "flow", "handler", "slot", "kind", "route"] {
        assert!(TAKEN.contains(&name), "`{name}` is what a measurement says of itself");
    }
}

#[test]
fn a_labels_name_is_a_lowercase_word() {
    for name in ["variant", "model_2", "a"] {
        assert_eq!(error_codes(&calling(&format!("{{ {name}: x }}"))), [] as [&str; 0], "{name}");
    }
    for name in ["Variant", "the-variant", "2nd", "_hidden", "which.one"] {
        let source = calling(&format!("{{ \"{name}\": x }}"));
        assert_eq!(error_codes(&source), ["bad-name"], "{name}");
        let message = first_error(&source);
        assert!(message.contains(&format!("`{name}` cannot be a label's name")), "{message}");
        assert!(message.contains("a lowercase word, of letters, digits and `_`"), "{message}");
    }
}

#[test]
fn a_value_that_is_not_text_or_a_number_is_refused() {
    for (value, what) in
        [("[a, b]", "a list"), ("{ of: a }", "a mapping"), ("true", "true or false"), ("null", "nothing")]
    {
        let source = calling(&format!("{{ variant: {value} }}"));
        assert_eq!(error_codes(&source), ["wrong-type"], "{value}");
        let message = first_error(&source);
        assert!(
            message
                .contains(&format!("the label `variant` must be text or a number, and came out as {what}")),
            "{message}"
        );
    }
    for fine in ["a", "2", "1.5", "\"${params.item}\"", "\"card ${params.item}\""] {
        assert_eq!(error_codes(&calling(&format!("{{ variant: {fine} }}"))), [] as [&str; 0], "{fine}");
    }

    let long = "x".repeat(MAX_VALUE + 1);
    let message = first_error(&calling(&format!("{{ variant: {long} }}")));
    assert!(message.contains("at most 100 characters"), "{message}");

    let message = first_error(&calling("variant-a"));
    assert!(message.contains("`labels` must be a mapping from a label's name to its value"), "{message}");
    assert!(message.contains("found a string"), "{message}");
}

#[test]
fn labels_on_a_step_that_is_one_action_are_refused_and_say_where_they_go() {
    let source = "blueprint: trial\npermissions: [shell]\nsteps:\n  build:\n    do: shell\n    \
                  labels: { variant: a }\n    with: { run: make }\n";
    assert_eq!(error_codes(source), ["conflict"]);
    let message = first_error(source);
    assert!(message.contains("a step that is one `do` starts nothing under it"), "{message}");
    assert!(message.contains("put `labels` on the `call`, `spawn` or `task` around it"), "{message}");
}

#[test]
fn a_value_reads_what_the_steps_arguments_may_read() {
    let each = "\
blueprint: trial
permissions: [blueprints, shell]
params:
  agents: { type: list, default: [] }
steps:
  card: { do: shell, with: { run: echo } }
  lanes:
    for_each: ${ params.agents }
    call: lane
    labels: { experiment: \"cart-${steps.card.out.code}\", variant: \"${item}\" }
";
    assert_eq!(error_codes(each), [] as [&str; 0]);

    assert_eq!(error_codes(&calling("{ variant: \"${params.agent}\" }")), ["unknown-param"]);
    assert_eq!(error_codes(&calling("{ variant: \"${item}\" }")), ["misplaced-name"]);
    assert_eq!(error_codes(&calling("{ variant: \"${params.item +}\" }")), ["bad-expression"]);
}

#[test]
fn labels_are_worked_out_when_the_step_starts_and_added_to_those_around_it() {
    let scope = json!({ "params": { "item": 12, "agent": "", "list": [1] } }).as_object().cloned().unwrap();
    let written = |labels: Value| labels.as_object().cloned().unwrap();
    let around = carried(&[("experiment", "outer"), ("team", "blue")]);

    let inner = written(json!({ "experiment": "cart-${params.item}", "variant": "a", "round": 2 }));
    assert_eq!(
        labels::under(&around, &inner, &scope).unwrap(),
        carried(&[("experiment", "cart-12"), ("round", "2"), ("team", "blue"), ("variant", "a")]),
        "what is said further in is added, and wins on the same name"
    );

    // One that comes out empty says nothing: what was there stays.
    let empty = written(json!({ "experiment": "${params.agent}", "variant": "${params.agent}" }));
    assert_eq!(labels::under(&around, &empty, &scope).unwrap(), around);

    // One hole keeps its type, so a number stays a number and a list is seen to be one.
    let number = written(json!({ "variant": "${params.item}" }));
    assert_eq!(labels::under(&Labels::new(), &number, &scope).unwrap(), carried(&[("variant", "12")]));
    let list = written(json!({ "variant": "${params.list}" }));
    let why = labels::under(&Labels::new(), &list, &scope).unwrap_err();
    assert_eq!(why, "the label `variant` must be text or a number, and came out as a list");
    let missing = written(json!({ "variant": "${params.nobody}" }));
    assert!(labels::under(&Labels::new(), &missing, &scope).unwrap_err().contains("came out as nothing"));
}

#[test]
fn labels_given_from_outside_are_held_to_what_labels_in_a_blueprint_are() {
    let given = |labels: Value| labels::given(labels.as_object().unwrap());
    assert_eq!(
        given(json!({ "experiment": "cart", "round": 3, "none": "" })).unwrap(),
        carried(&[("experiment", "cart"), ("round", "3")])
    );
    assert!(given(json!({ "step": "a" })).unwrap_err().contains("`step` cannot be a label"));
    assert!(given(json!({ "Which": "a" })).unwrap_err().contains("cannot be a label's name"));
    assert!(given(json!({ "variant": ["a"] })).unwrap_err().contains("must be text or a number"));
}

#[test]
fn what_the_page_that_is_the_language_says_of_labels_is_so() {
    let page = include_str!("../../../../docs/blueprints.md");
    let section = page.split("\n## Labels\n").nth(1).expect("the page has a section on labels");
    let section = section.split("\n## ").next().unwrap();
    let mut examples = section.split("```yaml\n").skip(1).map(|rest| rest.split("```").next().unwrap());
    let clean = |source: String| {
        let checked = blueprint::check(&source, &blueprint::NoCatalog);
        assert_eq!(
            checked.diagnostics,
            [],
            "{}",
            blueprint::render_diagnostics("the example", &checked.diagnostics)
        );
        checked.into_result().unwrap()
    };

    let head =
        "blueprint: trial\npermissions: [blueprints]\nparams:\n  title:\n  item:\n  agent_a:\n  agent_b:\n";
    let trial = clean(format!("{head}{}", examples.next().expect("the two lanes")));
    assert_eq!(
        Value::Object(trial.flow.step("b").unwrap().labels.clone()),
        json!({ "experiment": "cart-${params.item}", "variant": "b" })
    );
    let nested = clean(format!("blueprint: trial\npermissions: [blueprints]\n{}", examples.next().unwrap()));
    let inside = &nested.flow.step("trial").unwrap().inner;
    assert_eq!(Value::Object(inside.step("new").unwrap().labels.clone()), json!({ "variant": "v2" }));

    // The names it says are taken are the names that are.
    let taken = section.split("`action`").nth(1).expect("the page lists the names that are taken");
    let taken = format!("`action`{}", taken.split(" are taken").next().unwrap());
    let listed: Vec<&str> = taken.split('`').skip(1).step_by(2).collect();
    assert_eq!(listed, TAKEN);
}

#[test]
fn a_measurement_says_what_it_is_of_by_how_its_name_ends_and_is_a_time_where_it_says_nothing() {
    use blueprint::labels::unit;
    for (name, of) in [
        ("agent.cost_usd", Some("usd")),
        ("agent.input_tokens", Some("tokens")),
        ("device.free_mem_mb", Some("mb")),
        ("shop.order_usd", Some("usd")),
        ("shop.tokens", Some("tokens")),
        // Nothing said: a length of time.
        ("agent.turn", None),
        ("system.comparison.came", None),
        ("shop.checkout_done", None),
        // A unit is the end of the name's last word, not a word somewhere in it.
        ("usd.spent", None),
        ("shop.usd_spent", None),
    ] {
        assert_eq!(unit(name), of, "{name}");
    }
}
