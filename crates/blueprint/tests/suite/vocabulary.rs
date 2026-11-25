//! The language's description of itself, which an editor is handed, says
//! what the reader really accepts.

use blueprint::ValueType;
use blueprint::vocabulary::{FAILURE_HOOK, HOOK, Kind, SCOPES, SHAPES, STEP_VIEW, TYPES, members};

use crate::{check, first_error};

const PAGE: &str = include_str!("../../../../docs/blueprints.md");

#[test]
fn every_shape_a_field_points_at_is_one_there_is() {
    for shape in SHAPES {
        for field in shape.fields {
            if let Kind::Shape(name) | Kind::MapOf(name) | Kind::ListOf(name) = field.value {
                assert!(
                    SHAPES.iter().any(|known| known.name == name),
                    "`{}` in {} is said to hold `{name}`, which is not a shape",
                    field.name,
                    shape.what
                );
            }
        }
    }
}

#[test]
fn every_field_says_what_it_is_for_in_a_sentence() {
    for shape in SHAPES {
        for field in shape.fields {
            assert!(
                field.help.len() > 10 && field.help.ends_with('.'),
                "`{}` in {} needs a sentence, and has {:?}",
                field.name,
                shape.what,
                field.help
            );
        }
    }
}

/// What goes after a field is said in words where the language is said,
/// for whoever shows it: the kind by itself, what each thing in a mapping
/// of names or a list is, and what a bare value in a mapping's place is.
#[test]
fn every_field_says_in_words_what_goes_after_it_and_is_handed_out_with_them() {
    use blueprint::vocabulary::after;
    for shape in SHAPES {
        for field in shape.fields {
            assert!(
                !after(field.value).is_empty(),
                "`{}` in {} says what goes after it",
                field.name,
                shape.what
            );
        }
    }
    let of = |shape: &str, name: &str| {
        let shape = SHAPES.iter().find(|one| one.name == shape).unwrap();
        *shape.fields.iter().find(|field| field.name == name).unwrap()
    };
    assert_eq!(after(of("step", "timeout").value), "a length of time, such as 30s or 5m");
    assert_eq!(after(of("blueprint", "steps").value), "names, each a step");
    assert_eq!(after(of("step", "on_fail").value), "a list, each a hook");
    assert_eq!(after(of("step", "retry").value), "a number, or a mapping");
    assert_eq!(after(of("handler", "scope").value), "one of a few words");
    assert_eq!(after(of("step", "with").value), "arguments");
    let handed = serde_json::to_value(of("step", "timeout")).unwrap();
    assert_eq!(
        handed,
        serde_json::json!({
            "name": "timeout", "value": { "kind": "duration" },
            "help": "Fail the step if it runs longer, so that one that hangs does not hold the run up.",
            "after": "a length of time, such as 30s or 5m",
            "example": blueprint::vocabulary::STEP.fields.iter().find(|field| field.name == "timeout").unwrap().example,
        })
    );
}

#[test]
fn no_shape_names_a_field_twice() {
    for shape in SHAPES {
        let mut names: Vec<&str> = shape.names().collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "{} names a field twice", shape.what);
    }
}

#[test]
fn the_page_that_is_the_whole_language_names_every_field() {
    for shape in SHAPES {
        for field in shape.fields {
            let quoted = format!("`{}", field.name);
            let written = format!("{}:", field.name);
            assert!(
                PAGE.contains(&quoted) || PAGE.contains(&written),
                "docs/blueprints.md says it is the whole language and does not mention `{}`, a field of {}",
                field.name,
                shape.what
            );
        }
    }
}

#[test]
fn a_field_that_is_not_in_the_vocabulary_is_refused_and_the_nearest_one_offered() {
    let message = first_error("blueprint: x\nsteps:\n  a:\n    do: noop\n    timout: 5m\n");
    assert!(message.contains("unknown field `timout`"), "{message}");
    assert!(message.contains("did you mean `timeout`?"), "{message}");
}

#[test]
fn every_field_of_a_step_is_read_without_being_called_unknown() {
    // What goes after each is not right for all of them; only that none is
    // refused for its name.
    let step = SHAPES.iter().find(|shape| shape.name == "step").unwrap();
    for field in step.fields {
        let source = format!("blueprint: x\nsteps:\n  a:\n    do: noop\n    {}: 1\n", field.name);
        let unknown = check(&source).diagnostics.iter().any(|problem| problem.code == "unknown-field");
        assert!(!unknown, "`{}` is in the vocabulary and the reader does not know it", field.name);
    }
}

#[test]
fn the_types_a_param_may_have_are_the_types_there_are() {
    let offered: Vec<&str> = TYPES.iter().map(|choice| choice.name).collect();
    let real: Vec<&str> = ValueType::ALL.iter().map(|ty| ty.name()).collect();
    assert_eq!(offered, real);
}

#[test]
fn every_scope_offered_is_one_a_handler_may_have() {
    for scope in SCOPES {
        let source = format!(
            "blueprint: x\npermissions: []\non:\n  tick:\n    every: 1m\n    scope: {}\n    do: noop\n",
            scope.name
        );
        let checked = check(&source);
        assert_eq!(checked.diagnostics, [], "scope `{}`", scope.name);
    }
    let message = first_error("blueprint: x\non:\n  tick:\n    every: 1m\n    scope: mine\n    do: noop\n");
    assert!(message.contains("unknown scope `mine`"), "{message}");
}

#[test]
fn a_step_and_what_a_hook_calls_self_are_described_alike() {
    assert_eq!(members("self"), STEP_VIEW);
    assert!(members("params").is_empty(), "what is in `params` is the writer's own");
    assert!(members("event").iter().any(|member| member.name == "data"));
}

#[test]
fn fields_said_to_stand_in_for_each_other_are_fields_and_two_of_them_are_refused() {
    for shape in SHAPES {
        for group in shape.alternatives {
            for name in *group {
                assert!(shape.has(name), "`{name}` is not a field of {}", shape.what);
            }
        }
    }
    let refused = [
        ("a step", "blueprint: x\nsteps:\n  a: { do: noop, call: std/slot }\n"),
        (
            "a hook",
            "blueprint: x\nsteps:\n  a:\n    do: noop\n    on_start:\n      - { do: noop, emit: { topic: a.b } }\n",
        ),
        (
            "when a watch rule fires",
            "blueprint: x\nsteps:\n  a:\n    do: noop\n    watch:\n      - { after: 1m, every: 1m, do: noop }\n",
        ),
        (
            "what a watch rule does",
            "blueprint: x\nsteps:\n  a:\n    do: noop\n    watch:\n      - { after: 1m, do: noop, emit: { topic: a.b } }\n",
        ),
        ("what fires a handler", "blueprint: x\non:\n  h: { event: a.b, every: 1m, do: noop }\n"),
        ("what a handler does", "blueprint: x\non:\n  h: { every: 1m, do: noop, call: std/slot }\n"),
    ];
    for (what, source) in refused {
        assert!(check(source).has_errors(), "two of {what} were accepted");
    }
}

#[test]
fn a_hook_on_a_failure_has_a_hooks_fields_and_may_be_a_decision() {
    let names = |shape: &blueprint::vocabulary::Shape| shape.names().collect::<Vec<_>>();
    assert_eq!(names(&FAILURE_HOOK), names(&HOOK), "the reader checks both against one list");
    assert_eq!(FAILURE_HOOK.short, Some(Kind::Decision));

    // What the vocabulary says about where a decision may be is what the checker holds to.
    let deciding = |hook: &str| {
        format!("blueprint: x\npermissions: []\nsteps:\n  a:\n    do: noop\n    {hook}: [retry]\n")
    };
    let step = SHAPES.iter().find(|shape| shape.name == "step").unwrap();
    for field in step.fields {
        let Kind::ListOf(of) = field.value else { continue };
        if of == "watch" {
            continue;
        }
        let checked = check(&deciding(field.name));
        let refused = checked.diagnostics.iter().any(|problem| problem.code == "misplaced-decision");
        assert_eq!(refused, of != "failure_hook", "`{}: [retry]`", field.name);
    }
}

#[test]
fn the_page_names_every_name_an_expression_can_start_with() {
    for root in blueprint::ROOTS {
        let name = root.name;
        assert!(
            PAGE.contains(&format!("| `{name}")) || PAGE.contains(&format!(", `{name}`")),
            "docs/blueprints.md lists the names an expression can start with and has no row for `{name}`"
        );
    }
}

#[test]
fn an_expression_reads_what_the_environment_is_called_and_its_variables_and_nothing_it_does_not_have() {
    let reading = |what: &str| {
        format!(
            "blueprint: x\npermissions: []\nsteps:\n  a:\n    do: noop\n    when: ${{ {what} != \"prod\" }}\n"
        )
    };
    assert_eq!(check(&reading("environment.name")).diagnostics, []);
    // The map itself may be handed on whole.
    assert_eq!(check(&reading("environment")).diagnostics, []);
    // Its variables are the owner's to name: whatever follows `vars` is
    // read, and is nothing where it was not set.
    for read in [
        "environment.vars",
        "environment.vars.api",
        "environment.vars.anything_at_all",
        "environment.vars['api']",
        "(environment.vars.api ?? 'http://localhost')",
    ] {
        assert_eq!(check(&reading(read)).diagnostics, [], "{read}");
    }

    let checked = check(&reading("environment.other"));
    assert_eq!(checked.errors().map(|problem| problem.code).collect::<Vec<_>>(), ["unknown-member"]);
    let message = first_error(&reading("environment.other"));
    assert!(message.contains("`environment.other` is not something an environment has yet"), "{message}");
    assert!(message.contains("it has `name`, `vars`"), "{message}");

    assert_eq!(members("environment").iter().map(|member| member.name).collect::<Vec<_>>(), ["name", "vars"]);
    let vars = members("environment").iter().find(|member| member.name == "vars").unwrap();
    assert!(vars.help.contains("Not for secrets"), "what they are not for is said where they are offered");
}

#[test]
fn an_expression_reads_what_the_profile_is_called_and_its_variables_and_nothing_it_does_not_have() {
    let reading = |what: &str| {
        format!(
            "blueprint: x\npermissions: []\nsteps:\n  a:\n    do: noop\n    when: ${{ {what} != \"prod\" }}\n"
        )
    };
    for read in ["profile", "profile.name", "profile.vars", "profile.vars.api", "(profile.vars.api ?? 'x')"] {
        assert_eq!(check(&reading(read)).diagnostics, [], "{read}");
    }
    let message = first_error(&reading("profile.other"));
    assert!(message.contains("`profile.other` is not something a profile has yet"), "{message}");
    assert!(message.contains("it has `name`, `vars`"), "{message}");
    assert_eq!(members("profile").iter().map(|member| member.name).collect::<Vec<_>>(), ["name", "vars"]);
}
