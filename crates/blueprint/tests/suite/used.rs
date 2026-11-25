//! Where texts use each word, each name an expression starts with and each
//! action, as the reader reads them: the shortest texts first, each at the
//! first place it does, a handful at most and the rest counted.

use blueprint::used::{MOST, Place, used};

fn one(source: &str) -> blueprint::used::Used {
    used(&[("one".to_string(), source.to_string())])
}

fn at(line: u32, place: &str) -> Place {
    Place { blueprint: "one".into(), line, at: place.into() }
}

const TEXT: &str = "\
blueprint: one
# retry: is only a comment here
params:
  who: { type: string, default: ada }
steps:
  retry:
    do: shell
    with: { cmd: [echo, \"${params.who}\"] }
    retry: { max: 2 }
    on_fail:
      - when: ${ self.attempts < 2 }
        do: retry
      - emit: { topic: one.failed }
  build:
    task:
      make: { do: log, with: { message: made } }
on:
  moved:
    event: board.item.moved
    when: ${ event.data.to == \"done\" }
    steps:
      tell: { do: channel.post, with: { channel: ops, body: \"${event.data.id}\" } }
  tick:
    every: 1m
    do: noop
";

#[test]
fn a_word_is_where_it_is_written_as_the_word_and_not_where_its_name_is_a_comment_or_a_step() {
    let used = one(TEXT);
    assert_eq!(used.words["step/retry"].places, [at(9, "retry")]);
    assert_eq!(used.words["retry"].places, [at(9, "retry")]);
    assert_eq!(used.words["retry/max"].places, [at(9, "retry")]);
    assert_eq!(used.words["failure_hook/when"].places, [at(11, "retry")]);
    assert_eq!(used.words["param/type"].places, [at(4, "params.who")]);
    assert_eq!(used.words["blueprint"].places, [at(1, "")]);
    assert!(!used.words.contains_key("handler/poll"));
}

#[test]
fn a_place_is_said_by_its_step_or_its_handler_as_the_graph_names_them() {
    let used = one(TEXT);
    assert_eq!(used.words["step/task"].places, [at(15, "build")]);
    assert_eq!(used.actions["log"].places[0], at(16, "build/make"));
    assert_eq!(used.words["handler/event"].places, [at(19, "on.moved")]);
    assert_eq!(used.actions["channel.post"].places, [at(22, "on.moved/tell")]);
}

#[test]
fn an_action_is_where_a_step_a_hook_or_a_handler_does_it_and_a_decision_is_no_action() {
    let used = one(TEXT);
    assert_eq!(used.actions["shell"].places, [at(7, "retry")]);
    assert_eq!(used.actions["emit"].places, [at(13, "retry")]);
    assert_eq!(used.actions["noop"].places, [at(25, "on.tick")]);
    assert!(!used.actions.contains_key("retry"), "a decision is no action: {:?}", used.actions.keys());
    // `fail` is the action where a step does it, and a decision where a failure hook says it.
    let failing = one("blueprint: two\nsteps:\n  a:\n    do: fail\n    on_fail: [fail]\n");
    assert_eq!(failing.actions["fail"].places, [Place { blueprint: "one".into(), line: 4, at: "a".into() }]);
    // A hook that is an action's name alone is that action.
    let bare = one("blueprint: two\nsteps:\n  a:\n    do: noop\n    on_success: [log]\n");
    assert_eq!(bare.actions["log"].places[0].line, 5);
}

#[test]
fn a_name_is_where_an_expression_starts_with_it_in_a_hole_or_written_bare() {
    let used = one(TEXT);
    assert_eq!(used.names["params"].places, [at(8, "retry")]);
    assert_eq!(used.names["self"].places, [at(11, "retry")]);
    assert_eq!(used.names["event"].places, [at(20, "on.moved")]);
    assert!(!used.names.contains_key("state"));
}

#[test]
fn the_shortest_texts_are_said_first_a_handful_at_most_and_the_rest_counted() {
    let texts: Vec<(String, String)> = (0..MOST + 2)
        .map(|n| {
            let padding = "#\n".repeat(MOST + 2 - n);
            (format!("t{n}"), format!("blueprint: t{n}\n{padding}steps:\n  a:\n    do: noop\n"))
        })
        .collect();
    let used = used(&texts);
    let said: Vec<&str> = used.actions["noop"].places.iter().map(|place| place.blueprint.as_str()).collect();
    assert_eq!(said, ["t6", "t5", "t4", "t3", "t2"]);
    assert_eq!(used.actions["noop"].more, 2);
}

#[test]
fn a_text_that_does_not_read_uses_nothing() {
    let used = used(&[("broken".into(), "blueprint: [\n".into())]);
    assert!(used.words.is_empty() && used.names.is_empty() && used.actions.is_empty());
}
