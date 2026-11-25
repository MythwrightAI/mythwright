use blueprint::{Catalog, StepKind, TriggerScope, TriggerSource, WatchCond};
use serde_json::json;

use crate::{check, error_codes, first_error};

const KV_CACHE: &str = include_str!("../fixtures/kv-cache.bp");
const COUNTED: &str = include_str!("../fixtures/counted.bp");

#[test]
fn a_loop_over_a_number_and_what_a_step_says_it_gave_check_clean() {
    let checked = check(COUNTED);
    assert_eq!(
        checked.diagnostics,
        [],
        "{}",
        blueprint::render_diagnostics("counted.bp", &checked.diagnostics)
    );
    let blueprint = checked.into_result().unwrap();
    // The loop is over a number as it is written, and nothing is called to count it out.
    let each = blueprint.flow.step("each").unwrap();
    assert_eq!(each.for_each.as_ref().unwrap().source, "params.copies");
    // And `count` is one of the things a step has, beside `out`.
    assert!(blueprint::vocabulary::STEP_VIEW.iter().any(|choice| choice.name == "count"));
    assert_eq!(blueprint::plain::unplain(COUNTED), Ok(vec![]));
}

#[test]
fn the_kv_cache_example_checks_clean() {
    let checked = check(KV_CACHE);
    assert_eq!(
        checked.diagnostics,
        [],
        "{}",
        blueprint::render_diagnostics("kv-cache.bp", &checked.diagnostics)
    );
    let blueprint = checked.into_result().unwrap();

    assert_eq!(blueprint.name, "kv-cache");
    let ids: Vec<&str> = blueprint.flow.steps.iter().map(|step| step.id.as_str()).collect();
    assert_eq!(ids, ["check", "low_resources", "give_up", "build", "host"]);

    let build = blueprint.flow.step("build").unwrap();
    assert_eq!(build.kind, StepKind::Do("shell".into()));
    assert_eq!(build.needs, ["check", "give_up"]);
    assert!(!build.needs_implicit);
    assert_eq!(build.timeout.as_ref().unwrap().fixed, Some(30 * 60_000));
    assert_eq!(build.retry.as_ref().unwrap().max, 1);
    assert_eq!(build.retry.as_ref().unwrap().backoff_ms, 10_000);
    assert_eq!(build.measure.as_deref(), Some("build"));
    assert_eq!(build.watch[0].timing().and_then(|timing| timing.fixed), Some(5 * 60_000));
    assert_eq!(build.watch[0].hook.action, "emit");
    assert_eq!(build.watch[0].hook.with, json!({ "topic": "build.slow", "msg": "build slow", "code": 5 }));
    assert_eq!(build.watch[1].cond, WatchCond::Log(r"^error(\[E\d+\])?:".into()));
    assert_eq!(build.on_fail[0].action, "agent.consult");

    let host = blueprint.flow.step("host").unwrap();
    assert_eq!(host.kind, StepKind::Call("std/slot".into()));
    assert_eq!(host.key.as_deref(), Some("kv"));
    assert_eq!(host.needs, ["build"], "a step with no `needs` follows the one above it");
    assert!(host.needs_implicit);

    let handler = &blueprint.triggers[0];
    assert!(matches!(&handler.source, TriggerSource::Event(topic) if topic == "service.unhealthy"));
    assert_eq!(handler.scope, TriggerScope::Own);
    assert_eq!(handler.flow.steps[0].kind, StepKind::Do("emit".into()));
}

#[test]
fn params_resolve_with_defaults_types_and_choices() {
    let blueprint = check(KV_CACHE).into_result().unwrap();
    let given = |value: serde_json::Value| value.as_object().cloned().unwrap();

    let resolved = blueprint.resolve_params(&given(json!({ "port": 9000 }))).unwrap();
    assert_eq!(json!(resolved), json!({ "port": 9000, "profile": "release", "helper": "" }));

    let err = blueprint.resolve_params(&given(json!({ "port": "nine" }))).unwrap_err();
    assert!(err.contains("must be int"), "{err}");
    let err = blueprint.resolve_params(&given(json!({ "profile": "fast" }))).unwrap_err();
    assert!(err.contains("must be one of"), "{err}");
    let err = blueprint.resolve_params(&given(json!({ "prot": 1 }))).unwrap_err();
    assert!(err.contains("no param `prot`") && err.contains("port"), "{err}");
}

#[test]
fn a_required_param_is_one_with_no_default() {
    let blueprint =
        check("blueprint: b\nparams:\n  name:\n  size: { type: int }\nsteps:\n  a: { do: noop }\n")
            .into_result()
            .unwrap();
    assert!(blueprint.params.iter().all(|param| param.required));
    assert!(blueprint.resolve_params(&Default::default()).unwrap_err().contains("needs param `name`"));
}

#[test]
fn unknown_fields_are_refused_with_a_suggestion() {
    let message = first_error("blueprint: b\nsteps:\n  a:\n    do: noop\n    on_failure: [retry]\n");
    assert!(message.contains("unknown field `on_failure`"), "{message}");
    assert!(message.contains("did you mean `on_fail`?"), "{message}");

    let message = first_error("blueprint: b\nstep:\n  a: { do: noop }\n");
    assert!(message.contains("did you mean `steps`?"), "{message}");
}

#[test]
fn a_step_does_exactly_one_thing() {
    assert_eq!(error_codes("blueprint: b\nsteps:\n  a: { with: { x: 1 } }\n"), ["missing-field"]);
    assert_eq!(error_codes("blueprint: b\nsteps:\n  a: { do: noop, call: other }\n"), ["conflict"]);
}

#[test]
fn a_text_calls_itself_only_where_the_step_says_when_and_never_begins_itself() {
    let step = |says: &str| {
        format!("blueprint: b\nparams:\n  n: {{ type: int, default: 0 }}\nsteps:\n  a: {says}\n")
    };
    // It ends where what the step says no longer holds.
    assert_eq!(error_codes(&step("{ when: \"${ params.n < 3 }\", call: b }")), [] as [&str; 0]);
    // With nothing to end it, it would never end; begun and not waited
    // for, nothing would.
    assert_eq!(error_codes(&step("{ call: b }")), ["recursion"]);
    assert!(first_error(&step("{ call: b }")).contains("says `when`, which ends it"));
    assert_eq!(error_codes(&step("{ when: \"${ params.n < 3 }\", spawn: b }")), ["recursion"]);
}

#[test]
fn unknown_actions_and_arguments_are_caught_before_anything_runs() {
    let message = first_error("blueprint: b\nsteps:\n  a: { do: shel, with: { run: ls } }\n");
    assert!(
        message.contains("no action called `shel`") && message.contains("did you mean `shell`?"),
        "{message}"
    );

    let message = first_error("blueprint: b\nsteps:\n  a: { do: shell, with: { runn: ls } }\n");
    assert!(message.contains("no argument `runn`") && message.contains("did you mean `run`?"), "{message}");

    let codes = error_codes("blueprint: b\nsteps:\n  a: { do: shell }\n");
    assert_eq!(codes, ["missing-argument"]);

    let message =
        first_error("blueprint: b\nsteps:\n  a: { do: system.check, with: { min_free_mem_mb: lots } }\n");
    assert!(message.contains("must be int"), "{message}");
}

#[test]
fn what_is_kept_in_or_dropped_from_is_a_list_the_text_declares_and_is_refused_where_it_is_written_otherwise()
{
    let none = first_error("blueprint: b\nsteps:\n  a: { do: keep, with: { in: seen, what: one } }\n");
    assert_eq!(
        none,
        "`keep` names the list `seen`, and there is no state key `seen`; declare it under `state` as a list, \
         `seen: []`"
    );
    let count = first_error(
        "blueprint: b\nstate: { seen: 0 }\nsteps:\n  a: { do: drop, with: { from: seen, is: one } }\n",
    );
    assert_eq!(
        count,
        "`drop` names `seen`, which begins as 0 and is no list; declare it under `state` as a list, `seen: []`"
    );
    // Where it is written: the step, by its line.
    let checked = check(
        "blueprint: b\nstate: { seen: 0 }\nsteps:\n  a:\n    do: drop\n    with: { from: seen, is: one }\n",
    );
    let at = checked.errors().next().unwrap();
    assert_eq!((at.code, at.span.start.line), ("not-a-list", 4));

    // What each needs, and what `by` is.
    let kept = "blueprint: b\nstate: { seen: [] }\nsteps:\n";
    assert_eq!(
        error_codes(&format!("{kept}  a: {{ do: keep, with: {{ in: seen }} }}\n")),
        ["missing-argument"]
    );
    assert_eq!(
        error_codes(&format!("{kept}  a: {{ do: keep, with: {{ what: one }} }}\n")),
        ["missing-argument"]
    );
    assert_eq!(
        error_codes(&format!("{kept}  a: {{ do: drop, with: {{ from: seen }} }}\n")),
        ["missing-argument"]
    );
    assert_eq!(
        error_codes(&format!("{kept}  a: {{ do: drop, with: {{ is: one }} }}\n")),
        ["missing-argument"]
    );
    let by = first_error(&format!("{kept}  a: {{ do: keep, with: {{ in: seen, what: one, by: [id] }} }}\n"));
    assert!(by.contains("`keep`: `by` must be"), "{by}");
    let other =
        first_error(&format!("{kept}  a: {{ do: drop, with: {{ from: seen, is: one, where: id }} }}\n"));
    assert!(other.contains("no argument `where`"), "{other}");

    // Written as they are meant, nothing is said of either.
    let right = check(&format!(
        "{kept}  a: {{ do: keep, with: {{ in: seen, by: id, what: {{ id: one, n: 1 }} }} }}\n  \
         b: {{ do: drop, with: {{ from: seen, by: id, is: \"${{params.which}}\" }} }}\n  \
         c: {{ do: keep, with: {{ in: \"${{params.which}}\", what: one }} }}\nparams:\n  which: {{ type: string }}\n"
    ));
    assert_eq!(right.diagnostics, [], "{:?}", right.diagnostics);
}

#[test]
fn what_is_written_beside_what_is_handed_on_is_still_held_to_what_the_action_takes() {
    // What `...` hands on is known when the step runs: what is needed may
    // be among it, so nothing is called missing.
    let handed = "blueprint: b\nparams:\n  all: { type: any }\nsteps:\n  a:\n    do: shell\n    with:\n      ...: ${params.all}\n";
    assert_eq!(error_codes(handed), [] as [&str; 0]);
    // And an argument written beside it that the action has not got is refused as ever.
    let beside = format!("{handed}      runn: ls\n");
    let message = first_error(&beside);
    assert!(message.contains("no argument `runn`"), "{message}");
    assert_eq!(error_codes(&beside), ["unknown-argument"]);
}

#[test]
fn an_argument_with_a_hole_is_checked_when_it_is_filled() {
    let source = "blueprint: b\nparams: { floor: 512 }\nsteps:\n  a: { do: system.check, with: { min_free_mem_mb: ${params.floor} } }\n";
    assert_eq!(error_codes(source), [] as [&str; 0]);
}

#[test]
fn a_step_may_only_read_steps_it_depends_on() {
    let source = "\
blueprint: b
steps:
  a: { do: noop }
  b: { do: noop, needs: [] }
  c:
    needs: [a]
    do: log
    with: { message: ${ steps.b.out } }
";
    let message = first_error(source);
    assert!(message.contains("`steps.b` may not have finished"), "{message}");
    assert!(message.contains("add `b` to this step's `needs`"), "{message}");

    // Transitive dependencies count, and so does the step itself in its hooks.
    let source = "\
blueprint: b
steps:
  a: { do: noop }
  b: { do: noop }
  c:
    do: log
    with: { message: ${ steps.a.out } }
    on_finish:
      - do: log
        with: { message: ${ steps.c.duration_ms } and ${ self.out } }
";
    assert_eq!(error_codes(source), [] as [&str; 0]);
}

#[test]
fn names_in_expressions_must_exist() {
    let cases: &[(&str, &str)] = &[
        ("${ params.nope }", "no param `nope`"),
        ("${ state.nope }", "no state key `nope`"),
        ("${ steps.nope.ok }", "no step `nope`"),
        ("${ parms.x }", "did you mean `params`?"),
        ("${ event.data }", "only available inside an `on` handler"),
        ("${ item }", "only available in a step with `for_each`"),
        ("${ self.out }", "only available in a step's hooks"),
    ];
    for (expr, expected) in cases {
        let source = format!(
            "blueprint: b\nparams: {{ x: 1 }}\nsteps:\n  a: {{ do: log, with: {{ message: {expr} }} }}\n"
        );
        let message = first_error(&source);
        assert!(message.contains(expected), "{expr} gave {message:?}");
    }
}

#[test]
fn cycles_are_named() {
    let source = "blueprint: b\nsteps:\n  a: { do: noop, needs: [c] }\n  b: { do: noop, needs: [a] }\n  c: { do: noop, needs: [b] }\n";
    let message = first_error(source);
    assert!(message.contains("a -> c -> b -> a"), "{message}");
    assert!(message.contains("use `retry` or `for_each`"), "{message}");
}

#[test]
fn needs_must_name_real_steps() {
    let message =
        first_error("blueprint: b\nsteps:\n  build: { do: noop }\n  b: { do: noop, needs: [biuld] }\n");
    assert!(message.contains("did you mean `build`?"), "{message}");
}

#[test]
fn permissions_are_checked_against_what_the_actions_need() {
    let source = "blueprint: b\npermissions: [system]\nsteps:\n  a: { do: shell, with: { run: ls } }\n";
    let message = first_error(source);
    assert!(message.contains("uses `shell`, which is not listed under `permissions`"), "{message}");

    // Saying nothing is allowed, but it is pointed out.
    let checked = check("blueprint: b\nsteps:\n  a: { do: shell, with: { run: ls } }\n");
    assert!(!checked.has_errors());
    assert_eq!(checked.diagnostics[0].code, "no-permissions");
    assert!(checked.diagnostics[0].message.contains("[shell]"));
}

#[test]
fn decisions_belong_in_failure_hooks() {
    let ok = "blueprint: b\nsteps:\n  a:\n    do: noop\n    on_fail: [retry]\n    on_timeout: continue\n";
    assert_eq!(error_codes(ok), [] as [&str; 0]);
    let bad = "blueprint: b\nsteps:\n  a:\n    do: noop\n    on_finish: [retry]\n";
    assert_eq!(error_codes(bad), ["misplaced-decision"]);
    let bad = "blueprint: b\nsteps:\n  a: { do: retry }\n";
    assert_eq!(error_codes(bad), ["misplaced-decision"]);
}

#[test]
fn emitted_topics_are_checked() {
    let emit =
        |topic: &str| format!("blueprint: b\nsteps:\n  a: {{ do: emit, with: {{ topic: {topic} }} }}\n");
    assert_eq!(error_codes(&emit("build.slow")), [] as [&str; 0]);
    assert_eq!(error_codes(&emit("Build Slow")), ["bad-topic"]);
    assert_eq!(error_codes(&emit("step.finished")), ["reserved-topic"]);
    // What happens to a record is said by whatever did it to the record.
    assert_eq!(error_codes(&emit("record.created")), ["reserved-topic"]);
}

#[test]
fn a_child_named_with_as_in_a_for_each_must_be_named_by_what_tells_the_elements_apart() {
    let step = |named: &str| {
        format!(
            "blueprint: b\nparams: {{ cards: [a, b] }}\nsteps:\n  each: {{ spawn: other, for_each: \
             ${{ params.cards }}{named} }}\n"
        )
    };
    // Nothing said: each is named for its position. What `as` says is the
    // name, so it has to differ from one element to the next.
    assert_eq!(error_codes(&step("")), [] as [&str; 0]);
    assert_eq!(error_codes(&step(", as: \"card-${ item }\"")), [] as [&str; 0]);
    assert_eq!(error_codes(&step(", as: \"card-${ index }\"")), [] as [&str; 0]);
    assert_eq!(error_codes(&step(", as: the-card")), ["same-name"]);
    assert_eq!(error_codes(&step(", as: \"card-${ params.cards }\"")), ["same-name"]);
    assert_eq!(
        first_error(&step(", as: the-card")),
        "`as: the-card` is the same for every element of `for_each`, so each would be applied over the one \
         before; put what tells them apart in it (`${ item }`, `${ index }`), or leave `as` out and each is \
         named `each#<position>`"
    );
    // Outside a `for_each` a fixed name is what a name is.
    assert_eq!(error_codes("blueprint: b\nsteps:\n  one: { spawn: other, as: the-card }\n"), [] as [&str; 0]);
}

#[test]
fn a_poller_has_a_floor() {
    let source = "blueprint: b\non:\n  tick:\n    poll: { every: 1s, do: system.check }\n    do: noop\n";
    let message = first_error(source);
    assert!(message.contains("may not run more often than every 5s"), "{message}");
    assert!(message.contains("subscribe to an event instead"), "{message}");
}

#[test]
fn handlers_need_one_source_and_one_body() {
    assert_eq!(error_codes("blueprint: b\non:\n  h: { do: noop }\n"), ["missing-field"]);
    assert_eq!(
        error_codes("blueprint: b\non:\n  h: { event: a.b, every: 5m, do: noop }\n"),
        ["missing-field"]
    );
    assert_eq!(error_codes("blueprint: b\non:\n  h: { event: a.b }\n"), ["missing-field"]);
    assert_eq!(error_codes("blueprint: b\non:\n  h: { event: \"a b\", do: noop }\n"), ["bad-topic"]);
    assert_eq!(error_codes("blueprint: b\non:\n  h: { event: \"board.**\", do: noop }\n"), [] as [&str; 0]);
}

#[test]
fn several_problems_are_reported_at_once_in_file_order() {
    let source = "\
blueprint: b
steps:
  a: { do: nope }
  b: { do: shell }
  c: { do: log, with: { message: ${ params.x } } }
";
    let checked = check(source);
    let found: Vec<(u32, &str)> = checked.errors().map(|d| (d.span.start.line, d.code)).collect();
    assert_eq!(found, [(3, "unknown-action"), (4, "missing-argument"), (5, "unknown-param")]);
}

/// A catalog that knows why some actions are not there.
struct Elsewhere(blueprint::ActionList, bool);

impl Catalog for Elsewhere {
    fn action(&self, name: &str) -> Option<&blueprint::ActionSpec> {
        self.0.action(name)
    }

    fn action_names(&self) -> Vec<&str> {
        self.0.action_names()
    }

    fn is_complete(&self) -> bool {
        self.1
    }

    fn absent(&self, name: &str) -> Option<String> {
        name.starts_with("far.").then(|| format!("`{name}` is kept somewhere that is not here; fetch it"))
    }
}

#[test]
fn a_catalog_that_knows_why_an_action_is_missing_is_what_says_so() {
    let source = "blueprint: b\nsteps:\n  a: { do: far.away }\n  b: { do: shel }\n";
    let said = |complete: bool| {
        let checked = blueprint::check(source, &Elsewhere(crate::catalog(), complete));
        checked.errors().map(|d| format!("{}: {}", d.code, d.message)).collect::<Vec<_>>()
    };
    assert_eq!(
        said(true),
        [
            "unknown-action: `far.away` is kept somewhere that is not here; fetch it",
            "unknown-action: there is no action called `shel`; did you mean `shell`?",
        ],
        "what it has to say comes before any guess at what was meant"
    );
    // One that does not list every action still knows what it knows.
    assert_eq!(said(false), ["unknown-action: `far.away` is kept somewhere that is not here; fetch it"]);
}

#[test]
fn a_half_written_blueprint_still_has_a_shape() {
    let checked = check("blueprint: b\nsteps:\n  a: { do: nope }\n  b: { do: noop }\n");
    assert!(checked.has_errors());
    assert_eq!(checked.blueprint.as_ref().unwrap().flow.steps.len(), 2);
    assert!(checked.into_result().is_err());
}

#[test]
fn a_syntax_error_is_a_diagnostic_too() {
    let checked = check("blueprint: b\nsteps: [\n");
    assert!(checked.blueprint.is_none());
    assert_eq!(checked.diagnostics[0].code, "syntax");
    assert_eq!(
        checked.diagnostics[0].render("fleet.bp"),
        "fleet.bp:3:1: error: this `[` or `{` is never closed"
    );
}

#[test]
fn the_revision_tracks_the_text() {
    assert_eq!(blueprint::revision("a"), blueprint::revision("a"));
    assert_ne!(blueprint::revision("a"), blueprint::revision("a "));
    assert_eq!(blueprint::revision("a").len(), 16);
}

#[test]
fn the_builtin_actions_are_always_known() {
    let catalog = crate::catalog();
    assert!(catalog.action("wait").is_none(), "builtins are not in the runtime's list");
    for name in ["wait", "emit", "set", "end", "fail", "noop", "log", "stop"] {
        assert!(blueprint::intrinsic(name).is_some(), "{name}");
    }
    let source = "blueprint: b\nstate: { n: 0 }\nsteps:\n  w: { do: wait, with: { for: 5m } }\n  s: { do: set, with: { n: 1 } }\n";
    assert_eq!(error_codes(source), [] as [&str; 0]);
}

#[test]
fn asking_an_agent_when_a_step_fails_is_said_on_the_step_and_is_a_hook_like_any_other() {
    // Named as it is spoken to in a channel, or by its bare name.
    for agent in ["\"@builder\"", "builder"] {
        let source = format!(
            "blueprint: b\npermissions: [shell, agents]\nsteps:\n  build:\n    do: shell\n    with: {{ run: make }}\n    if_fail_ask_agent: {agent}\n    else: build-broken\n"
        );
        let checked = check(&source);
        assert_eq!(checked.errors().count(), 0, "{:?}", checked.diagnostics);
        let blueprint = checked.blueprint.unwrap();
        let build = &blueprint.flow.steps[0];
        assert_eq!(
            (build.asks.as_deref(), build.fail_code.as_deref()),
            (Some("builder"), Some("build-broken"))
        );
        // The asking is the last thing tried when it fails.
        let ask = build.on_fail.last().unwrap();
        assert_eq!(ask.action, "agent.consult");
        assert_eq!(
            (&ask.with["slot"], &ask.with["options"]),
            (&serde_json::json!("builder"), &serde_json::json!(["retry", "fail"]))
        );
        let prompt = ask.with["prompt"].as_str().unwrap();
        assert!(
            prompt.starts_with("The step `build` failed while running `shell`:\n${self.error}"),
            "{prompt}"
        );
    }

    // It comes after whatever the step already does when it fails.
    let both = "blueprint: b\npermissions: [shell, agents]\nsteps:\n  build:\n    do: shell\n    with: { run: make }\n    on_fail:\n      - emit: { topic: build.broke }\n    if_fail_ask_agent: builder\n";
    let blueprint = check(both).blueprint.unwrap();
    let hooks: Vec<&str> = blueprint.flow.steps[0].on_fail.iter().map(|hook| hook.action.as_str()).collect();
    assert_eq!(hooks, ["emit", "agent.consult"]);
}

#[test]
fn what_a_step_fails_with_goes_with_the_agent_that_was_asked() {
    let message = first_error(
        "blueprint: b\npermissions: [shell]\nsteps:\n  build:\n    do: shell\n    with: { run: make }\n    else: build-broken\n",
    );
    assert!(message.contains("it goes with `if_fail_ask_agent`"), "{message}");
    let message = first_error(
        "blueprint: b\npermissions: [shell]\nsteps:\n  build:\n    do: shell\n    with: { run: make }\n    if_fail_ask_agent: \"@\"\n",
    );
    assert!(message.contains("names an agent, such as `@builder`"), "{message}");
}

#[test]
fn a_blueprint_with_outputs_alone_is_a_rule_and_one_with_nothing_at_all_is_refused() {
    let rule = "blueprint: pick\npermissions: []\nparams:\n  among: { type: list, default: [] }\noutputs:\n  take: ${ sort(params.among)[0] }\n";
    let checked = check(rule);
    assert_eq!(error_codes(rule), Vec::<&str>::new(), "{:?}", checked.diagnostics);
    assert!(checked.into_result().unwrap().flow.steps.is_empty(), "it runs nothing");
    let nothing = "blueprint: idle\nparams:\n  among: { type: list, default: [] }\n";
    assert!(
        first_error(nothing).contains("needs `steps`, `on`, `uses` or `outputs`"),
        "{}",
        first_error(nothing)
    );
}

/// Every diagnostic with this code, as its message.
fn said(source: &str, code: &str) -> Vec<String> {
    crate::check(source)
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

#[test]
fn a_name_misspelt_after_event_instance_self_or_a_step_is_refused_and_not_read_as_nothing() {
    // Each of these read as nothing, and the text was said to be right.
    let source = "\
blueprint: typos
permissions: [shell]
steps:
  hello:
    do: shell
    with: { run: echo hello }
    on_finish:
      - do: log
        with: { message: \"${ self.duraton_ms }\" }
  after: { do: log, with: { message: \"${ steps.hello.output.code } in ${ instance.projct }\" } }
on:
  heard:
    event: shop.said
    do: log
    with: { message: \"${ event.topik } ${ event.dat.title } ${ event.source.instanse }\" }
";
    let found = said(source, "unknown-member");
    assert_eq!(
        found,
        [
            "`self.duraton_ms` is not something a step has; did you mean `duration_ms`? A step has `ok`, \
             `failed`, `skipped`, `status`, `out`, `count`, `error`, `reason`, `attempts`, `duration_ms`",
            "`steps.hello.output` is not something a step has. A step has `ok`, `failed`, `skipped`, `status`, \
             `out`, `count`, `error`, `reason`, `attempts`, `duration_ms`",
            "`instance.projct` is not something an instance has; did you mean `project`? An instance has \
             `id`, `name`, `key`, `parent`, `project`, `device`",
            "`event.topik` is not something an event has; did you mean `topic`? An event has `topic`, `data`, \
             `source`, `ts`, `seq`",
            "`event.dat` is not something an event has; did you mean `data`? An event has `topic`, `data`, \
             `source`, `ts`, `seq`",
            "`event.source.instanse` is not something an event's source has; did you mean `instance`? An \
             event's source has `instance`, `run`, `step`, `device`, `actor`",
        ]
    );
    assert_eq!(crate::error_codes(source), ["unknown-member"; 6]);

    // Spelt as they are, nothing is said.
    let right = source
        .replace("duraton_ms", "duration_ms")
        .replace("hello.output", "hello.out")
        .replace("projct", "project")
        .replace("topik", "topic")
        .replace("event.dat.", "event.data.")
        .replace("instanse", "instance");
    assert_eq!(crate::error_codes(&right), [] as [&str; 0]);
}

#[test]
fn reading_of_a_step_what_its_action_does_not_say_it_gives_is_warned_of() {
    let source = "\
blueprint: outs
permissions: [shell, agents]
steps:
  build: { do: shell, with: { run: make } }
  ask: { do: agent.consult, with: { prompt: \"exit ${ steps.build.out.cod }?\" } }
  say: { do: log, with: { message: \"${ steps.ask.out.decison } after ${ steps.build.out.code }\" } }
";
    assert_eq!(
        said(source, "unknown-output"),
        [
            "`steps.build.out.cod`: `shell` does not say it gives `cod`; did you mean `code`? It gives `code`, \
             `stdout`, `stderr`, `duration_ms`",
            "`steps.ask.out.decison`: `agent.consult` does not say it gives `decison`; did you mean \
             `decision`? It gives `decision`",
        ]
    );
    // A warning: the text is applied all the same, since an action may
    // give more than it says.
    assert_eq!(crate::error_codes(source), [] as [&str; 0]);

    // Where a step is done once for each of a list, what it gives is a
    // list, and where an action says nothing of what it gives, nothing is
    // known: neither is spoken of.
    let unknown = "\
blueprint: outs
permissions: [shell]
params:
  names: { type: list, default: [] }
steps:
  each: { for_each: ${ params.names }, do: shell, with: { run: \"echo ${ item }\" } }
  wait: { do: wait, with: { for: 1s } }
  say: { do: log, with: { message: \"${ steps.each.out.anything } ${ steps.wait.out.timed_out }\" } }
";
    assert_eq!(said(unknown, "unknown-output"), [] as [&str; 0]);
}

#[test]
fn reading_of_a_task_a_step_it_has_not_got_is_refused() {
    let source = "\
blueprint: tasks
permissions: [shell]
steps:
  build:
    task:
      compile: { do: shell, with: { run: make } }
      test: { do: shell, with: { run: make test } }
  say: { do: log, with: { message: \"${ steps.build.out.compile.code } ${ steps.build.out.tset.code }\" } }
";
    assert_eq!(
        said(source, "unknown-output"),
        [
            "`steps.build.out.tset`: the task `build` has no step `tset`; did you mean `test`? It has `compile`, `test`"
        ]
    );
    assert_eq!(crate::error_codes(source), ["unknown-output"]);
}

#[test]
fn what_a_poll_read_the_time_before_is_read_in_its_handler_and_nowhere_else() {
    let polled = "\
blueprint: upstream
permissions: [system]
on:
  moved:
    poll: { every: 30s, do: system.check }
    do: log
    with: { message: \"${ event.data.ok } was ${ event.previous.ok }\" }
";
    assert_eq!(crate::error_codes(polled), [] as [&str; 0]);
    let heard = "\
blueprint: upstream
on:
  moved:
    event: shop.moved
    do: log
    with: { message: \"${ event.previous }\" }
";
    assert_eq!(
        said(heard, "unknown-member"),
        [
            "`event.previous` is not something an event has. An event has `topic`, `data`, `source`, `ts`, `seq`"
        ]
    );
}

#[test]
fn the_name_of_a_secret_is_text_like_any_other_and_what_it_reads_has_to_be_there() {
    let naming = |name: &str| {
        format!(
            "blueprint: told\npermissions: [secrets]\nparams: {{ key: {{ type: string, default: \"\" }} }}\n\
             steps:\n  a: {{ do: log, env: {{ TOKEN: {{ secret: {name} }} }}, with: {{ message: hi }} }}\n"
        )
    };
    // Written, said by a param, or worked out: each is a name.
    for name in ["deploy-key", "\"${params.key}\"", "\"mail-${ environment.name }\""] {
        assert_eq!(error_codes(&naming(name)), [""; 0], "{name}");
    }
    // A param that is not there names nothing, and that is said before anything runs.
    assert_eq!(error_codes(&naming("\"${params.kye}\"")), ["unknown-param"]);
}

#[test]
fn a_text_that_sets_the_profile_and_reads_the_environment_is_told_it_was_probably_written_before_the_rename()
{
    let codes = |source: &str| check(source).diagnostics.iter().map(|said| said.code).collect::<Vec<_>>();
    let sets = |reads: &str| {
        format!(
            "blueprint: mine\npermissions: [admin]\nsteps:\n  set:\n    do: profile.set\n    with: {{ vars: {{}} }}\n  \
             say:\n    do: log\n    with: {{ message: \"${{ {reads} }}\" }}\n"
        )
    };
    // Setting the profile's variables and reading the environment's is
    // what a text written before 0.1.0 did, when both were one thing.
    let before = sets("environment.vars.api");
    assert!(codes(&before).contains(&"environment-renamed"), "{:?}", check(&before).diagnostics);
    let told = check(&before);
    let said = told.diagnostics.iter().find(|said| said.code == "environment-renamed").unwrap();
    assert!(said.message.contains("`profile.vars.<name>`"), "{}", said.message);
    assert_eq!(said.span.start.line, 9, "it is said where `environment` is first read");
    // Written the way it is now, or reading the environment without
    // setting the profile, it is told nothing.
    assert!(!codes(&sets("profile.vars.api")).contains(&"environment-renamed"));
    let reads_alone = "blueprint: mine\npermissions: []\nsteps:\n  say:\n    do: log\n    \
                       with: { message: \"${ environment.vars.api }\" }\n";
    assert!(!codes(reads_alone).contains(&"environment-renamed"));
}
