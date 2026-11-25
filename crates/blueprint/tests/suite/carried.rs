//! `on_update`: what a running instance does when it takes a text, in
//! place of running the steps again; and `was`, the name that is read
//! there and nowhere else.

use crate::{check, error_codes};

const LANE: &str = "\
blueprint: lane
version: 3
permissions: [system]
state:
  rounds: 0
steps:
  claim: { do: system.check }
on:
  poke:
    event: board.item.moved
    do: log
    with: { message: poked }
on_update:
  - do: set
    when: ${ was.version == \"2\" }
    with: { rounds: ${ was.state.builds } }
  - emit: { topic: lane.carried, from: \"${ was.revision }\", size: \"${ was.params.size }\" }
";

fn messages(source: &str) -> Vec<String> {
    check(source).errors().map(|problem| problem.message.clone()).collect()
}

#[test]
fn on_update_is_read_as_a_list_of_things_to_do_which_may_read_what_the_instance_was() {
    let checked = check(LANE);
    assert_eq!(checked.diagnostics, [], "{}", blueprint::render_diagnostics("lane.bp", &checked.diagnostics));
    let lane = checked.into_result().unwrap();
    let says = lane.on_update.expect("it says how it is taken");
    let actions: Vec<&str> = says.iter().map(|hook| hook.action.as_str()).collect();
    assert_eq!(actions, ["set", "emit"]);
    assert!(says[0].when.is_some(), "an entry may be done only for some of what it comes from");
}

#[test]
fn a_text_that_says_nothing_of_it_is_told_apart_from_one_that_says_there_is_nothing_to_do() {
    let silent = LANE.split("on_update:").next().unwrap();
    assert_eq!(check(silent).into_result().unwrap().on_update.map(|says| says.len()), None);
    for nothing in ["on_update: []\n", "on_update:\n"] {
        let written = format!("{silent}{nothing}");
        let checked = check(&written);
        assert_eq!(checked.diagnostics, [], "{nothing:?}");
        assert_eq!(checked.into_result().unwrap().on_update.map(|says| says.len()), Some(0), "{nothing:?}");
    }
}

#[test]
fn was_is_refused_anywhere_but_in_on_update_and_says_why() {
    let why = "`was` is only available in `on_update`: it is what a running instance was before it took \
               this text, and there is no such thing anywhere else";
    let silent = LANE.split("on_update:").next().unwrap();
    let places = [
        // A step, a handler, what is given back, what is done when it stops,
        // and what its state starts at.
        silent.replace(
            "claim: { do: system.check }",
            "claim: { do: log, with: { message: \"${ was.version }\" } }",
        ),
        silent.replace("{ message: poked }", "{ message: \"${ was.state.builds }\" }"),
        format!("{silent}outputs:\n  from: ${{ was.revision }}\n"),
        format!("{silent}on_stop:\n  - do: log\n    with: {{ message: \"${{ was.version }}\" }}\n"),
        silent.replace("rounds: 0", "rounds: ${ was.state.builds }"),
    ];
    for place in places {
        assert_eq!(messages(&place), [why], "{place}");
        assert_eq!(error_codes(&place), ["misplaced-name"]);
    }
}

#[test]
fn on_update_reads_no_step_and_only_what_was_has_and_decides_nothing() {
    let with = |entry: &str| format!("{}{entry}", LANE.split("on_update:").next().unwrap());
    // The steps are not run, so there is nothing of theirs to read.
    let steps = with("on_update:\n  - do: log\n    with: { message: \"${ steps.claim.out.cpu }\" }\n");
    assert_eq!(messages(&steps), ["step results are not available here"]);
    // What follows `was` is one of four things.
    let member = with("on_update:\n  - do: log\n    with: { message: \"${ was.status }\" }\n");
    assert_eq!(
        messages(&member),
        [
            "`was.status` is not something `was` has; there are `was.version`, `was.revision`, `was.state`, `was.params`"
        ]
    );
    // What follows `was.state` is the earlier text's, and is not checked;
    // what follows `state` is this text's, and is.
    let state =
        with("on_update:\n  - do: set\n    with: { rounds: ${ was.state.anything + state.builds } }\n");
    assert_eq!(
        messages(&state),
        ["there is no state key `builds`; declare it under `state` with an initial value"]
    );
    // It is not a failed step: there is nothing to try again or let pass.
    let decides = with("on_update: [retry]\n");
    assert_eq!(error_codes(&decides), ["misplaced-decision"]);
    // And an action it names is held to what that action takes, as anywhere.
    let unknown = with("on_update:\n  - do: system.check\n    with: { loudly: true }\n");
    assert_eq!(error_codes(&unknown), ["unknown-argument"]);
}

#[test]
fn what_a_text_says_of_how_it_is_taken_is_not_part_of_how_it_begins() {
    let silent = LANE.split("on_update:").next().unwrap();
    assert_eq!(blueprint::outset(LANE), blueprint::outset(silent));
}
