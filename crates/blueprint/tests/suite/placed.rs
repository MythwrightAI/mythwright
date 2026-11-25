//! A refusal says where: a name that is not there is pointed at where it
//! is written, and not at the start of the step that reads it; and what
//! was probably meant is offered where a name is close to one there is.

use crate::check;

/// Each error as `line:col message`, in the order of the file.
fn said(source: &str) -> Vec<String> {
    check(source)
        .errors()
        .map(|found| format!("{}:{} {}", found.span.start.line, found.span.start.col, found.message))
        .collect()
}

const MISSPELT: &str = "\
blueprint: misspelt
params:
  name: { type: string, default: you }
state: { seen: 0 }
steps:
  first:
    do: system.check
  second:
    description: Reads four things that are not there.
    do: shell
    # params.nme in a comment is not where it is written
    with:
      run: \"echo ${params.nme} ${ state.sene } ${steps.frist.out.ok}\"
      cwd: \"${event.data.dir}\"
";

#[test]
fn a_name_that_is_not_there_is_pointed_at_where_it_is_written_with_what_was_probably_meant() {
    assert_eq!(
        said(MISSPELT),
        [
            "13:20 there is no param `nme`; did you mean `name`?",
            "13:35 there is no state key `sene`; did you mean `seen`?",
            "13:50 there is no step `frist`; did you mean `first`?",
            "14:15 `event` is only available inside an `on` handler",
        ]
    );
    // And the mark an editor draws is the name, and not the step.
    let checked = check(MISSPELT);
    let first = checked.errors().next().unwrap();
    assert_eq!((first.span.start.col, first.span.end.col), (20, 30), "`params.nme` and no more");
    assert_eq!(first.span.start.line, first.span.end.line);
}

#[test]
fn a_name_like_nothing_there_says_how_one_is_declared_and_no_guess() {
    let source = "\
blueprint: far
state: { seen: 0 }
steps:
  only:
    do: shell
    with: { run: \"${state.totals} ${steps.zzz.out.code}\" }
";
    assert_eq!(
        said(source),
        [
            "6:21 there is no state key `totals`; declare it under `state` with an initial value",
            "6:37 there is no step `zzz`",
        ]
    );
}

#[test]
fn a_name_written_as_part_of_a_longer_one_is_not_taken_for_it() {
    // `params.names` is another name, and is there; `my_params.nme` is no
    // read of `params` at all. The one that is wrong is the last.
    let source = "\
blueprint: longer
params:
  names: { type: list, default: [] }
steps:
  only:
    do: shell
    with:
      run: \"echo ${params.names} my_params.nme\"
      cwd: \"${params.nme}\"
";
    assert_eq!(said(source), ["9:15 there is no param `nme`; declare it under `params`"]);
}

#[test]
fn a_name_in_what_a_handler_or_a_hook_reads_is_pointed_at_too() {
    let source = "\
blueprint: heard
steps:
  only:
    do: shell
    with: { run: echo }
    on_fail:
      - do: agent.consult
        with: { prompt: \"${steps.onyl.error}\" }
on:
  moved:
    event: thing.moved
    outside: true
    when: ${ params.loud }
    do: system.check
";
    assert_eq!(
        said(source),
        [
            "8:28 there is no step `onyl`; did you mean `only`?",
            "13:14 there is no param `loud`; declare it under `params`",
        ]
    );
}

#[test]
fn a_function_that_is_not_one_says_which_was_probably_meant_or_which_there_are() {
    let close = blueprint::expr::parse("uper(params.name)").unwrap_err().message;
    assert_eq!(close, "unknown function `uper`; did you mean `upper`?");
    let far = blueprint::expr::parse("frobnicate(1)").unwrap_err().message;
    assert!(far.starts_with("unknown function `frobnicate`; there are: len, contains, "), "{far}");
}

#[test]
fn a_line_that_cannot_be_read_says_what_to_do_about_it() {
    let said = |source: &str| check(source).errors().next().map(|found| found.message.clone()).unwrap();
    assert_eq!(
        said("blueprint: a\nsteps:\n  say:\n    do: shell\n     with: { run: echo }\n"),
        "unexpected indentation: this line is further in than the line above it, which has its value already. \
         Line the two up; or, if this belongs under that one, write nothing after its colon"
    );
    assert_eq!(
        said("blueprint: a\nsteps:\n  say:\n    do shell\n"),
        "expected `key: value`: this line has no colon after a key (a colon is followed by a space, or ends the \
         line)"
    );
    assert_eq!(
        said(
            "blueprint: a\nsteps:\n  say:\n    do: shell\n    on_fail:\n      - do: retry\n        - do: fail\n"
        ),
        "unexpected indentation: this line is further in than the `- ` items it is among; line it up with them"
    );
}
