//! A task: steps grouped under one name, with one policy for all of them.

use blueprint::{StepKind, graph};

use crate::{check, error_codes, first_error};

const RELEASE: &str = "\
blueprint: release
permissions: [shell]
params:
  build: true
steps:
  fetch: { do: shell, with: { run: git pull } }
  build:
    description: Everything it takes to have something to ship.
    task:
      compile: { do: shell, with: { run: make } }
      test:
        do: shell
        when: ${ steps.compile.out.code == 0 && steps.fetch.ok }
        with: { run: make test }
    when: ${ params.build }
    retry: 2
    timeout: 10m
    on_fail:
      - emit: { topic: build.broken, at: \"${self.out.compile.code}\" }
  ship:
    do: shell
    with: { run: \"ship ${steps.build.out.compile.code} ${steps.build.out.test.code}\" }
";

#[test]
fn a_task_is_a_step_whose_steps_are_written_under_it() {
    let checked = check(RELEASE);
    assert_eq!(
        checked.diagnostics,
        [],
        "{}",
        blueprint::render_diagnostics("release.bp", &checked.diagnostics)
    );
    let blueprint = checked.into_result().unwrap();

    let ids: Vec<&str> = blueprint.flow.steps.iter().map(|step| step.id.as_str()).collect();
    assert_eq!(ids, ["fetch", "build", "ship"], "the steps beside a task do not see inside it");
    let build = blueprint.flow.step("build").unwrap();
    assert_eq!(build.kind, StepKind::Task);
    assert_eq!((build.kind.verb(), build.needs.as_slice()), ("task", ["fetch".to_string()].as_slice()));
    assert_eq!(build.retry.as_ref().unwrap().max, 2, "what is said beside `task` is the task's");
    assert_eq!(blueprint.flow.step("ship").unwrap().needs, ["build"], "what follows a task waits for it");

    // Inside, steps are steps: the first starts when the task does, and
    // each one after it follows the one above.
    let inner: Vec<(&str, &[String])> =
        build.inner.steps.iter().map(|step| (step.id.as_str(), step.needs.as_slice())).collect();
    assert_eq!(inner, [("compile", [].as_slice()), ("test", ["compile".to_string()].as_slice())]);
    assert_eq!(build.inner.steps[0].kind, StepKind::Do("shell".into()));
}

#[test]
fn the_task_written_out_on_the_page_that_is_the_language_checks_clean() {
    let page = include_str!("../../../../docs/blueprints.md");
    let section = page.split("\n## Tasks\n").nth(1).expect("the page has a section on tasks");
    let example = section.split("```yaml\n").nth(1).and_then(|rest| rest.split("```").next()).unwrap();
    let source = format!("blueprint: release\npermissions: [shell]\nparams: {{ build: true }}\n{example}");
    let checked = check(&source);
    assert_eq!(
        checked.diagnostics,
        [],
        "{}",
        blueprint::render_diagnostics("the example", &checked.diagnostics)
    );
    assert_eq!(checked.blueprint.unwrap().flow.step("build").unwrap().inner.steps.len(), 3);
}

#[test]
fn outside_a_task_its_steps_are_read_through_it_and_the_checker_says_how() {
    let source = "\
blueprint: b
steps:
  build:
    task:
      compile: { do: noop }
  ship: { do: log, with: { message: \"${steps.compile.out}\" } }
";
    assert_eq!(error_codes(source), ["unordered-read"]);
    assert_eq!(
        first_error(source),
        "`compile` is a step inside the task `build`; outside the task, what it gave out is \
         `steps.build.out.compile`"
    );
}

#[test]
fn a_step_in_a_task_reads_only_what_has_settled_before_it() {
    // A step lower down in the same task.
    let message = first_error(
        "blueprint: b\nsteps:\n  build:\n    task:\n      compile: { do: log, with: { message: \"${steps.test.ok}\" } }\n      test: { do: noop }\n",
    );
    assert_eq!(
        message,
        "`steps.test` may not have finished when this runs; add `test` to this step's `needs`"
    );

    // A step outside that the task did not wait for: it is the task that has to.
    let unordered = "\
blueprint: b
steps:
  fetch: { do: noop }
  build:
    needs: []
    task:
      compile: { do: log, with: { message: \"${steps.fetch.ok}\" } }
";
    assert_eq!(
        first_error(unordered),
        "`steps.fetch` may not have finished when the task `build` starts; add `fetch` to the `needs` of `build`"
    );
    assert_eq!(error_codes(&unordered.replace("needs: []", "needs: [fetch]")), [] as [&str; 0]);

    // The task itself, which has not finished while its steps run.
    let message = first_error(
        "blueprint: b\nsteps:\n  build:\n    task:\n      compile: { do: noop }\n      test: { do: log, with: { message: \"${steps.build.out.compile}\" } }\n",
    );
    assert!(message.starts_with("`steps.build` is the task this is inside"), "{message}");
    assert!(message.ends_with("read each other by their own names, as `steps.<name>`"), "{message}");
}

#[test]
fn a_step_in_a_task_may_not_take_the_name_of_a_step_outside_it() {
    let clash = "\
blueprint: b
steps:
  fetch: { do: noop }
  build:
    task:
      fetch: { do: noop }
";
    assert_eq!(error_codes(clash), ["conflict"]);
    let message = first_error(clash);
    assert!(
        message.contains("step `fetch` in the task `build` has the name of a step outside it"),
        "{message}"
    );
    assert!(message.ends_with("give it another name"), "{message}");

    // Two tasks are outside each other, so each may have a step of the same name.
    let apart = "\
blueprint: b
steps:
  build:
    task:
      check: { do: noop }
  ship:
    task:
      check: { do: noop }
";
    assert_eq!(error_codes(apart), [] as [&str; 0]);
}

#[test]
fn what_only_an_action_can_be_told_is_refused_on_a_task() {
    let task =
        |extra: &str| format!("blueprint: b\nsteps:\n  build:\n    task:\n      a: {{ do: noop }}\n{extra}");
    let refused = [
        ("    with: { run: make }\n", "a task takes no arguments; give `with` to the steps inside it"),
        (
            "    device: builder-1\n",
            "a task runs on no device of its own; say `device` on the steps inside it",
        ),
        ("    as: lane\n", "`as` names a child blueprint; it only applies to `call` and `spawn`"),
        ("    again: true\n", "`again` is said of a step that does an action"),
        ("    watch:\n      - { log: \"^error\", fail: true }\n", "a task prints nothing itself"),
        ("    do: noop\n", "step `build` has more than one of `do`, `call`, `spawn` and `task`"),
    ];
    for (extra, expected) in refused {
        let message = first_error(&task(extra));
        assert!(message.starts_with(expected), "{extra:?} gave: {message}");
    }
    // A rule about how long it has run is about the task, and is kept.
    assert_eq!(error_codes(&task("    watch:\n      - { after: 5m, fail: true }\n")), [] as [&str; 0]);

    assert_eq!(
        first_error("blueprint: b\nsteps:\n  build:\n    task: {}\n"),
        "task `build` has no steps; write the steps it groups under `task:`"
    );
    assert_eq!(
        first_error("blueprint: b\nsteps:\n  build:\n    task: [a, b]\n"),
        "`task` must be a mapping of step name to step"
    );
    let message = first_error("blueprint: b\nsteps:\n  build:\n    timeout: 5m\n");
    assert!(message.ends_with("or `task:` with steps under it"), "{message}");
}

#[test]
fn again_is_refused_on_a_step_that_starts_a_child_which_is_found_again_as_it_is() {
    for verb in ["call", "spawn"] {
        let message =
            first_error(&format!("blueprint: b\nsteps:\n  kid:\n    {verb}: other\n    again: true\n"));
        assert!(message.contains("a `call` or `spawn` finds the child it made"), "{verb}: {message}");
    }
    assert_eq!(error_codes("blueprint: b\nsteps:\n  build: { do: noop, again: true }\n"), [] as [&str; 0]);
}

#[test]
fn a_task_may_be_in_a_handler_and_inside_another_task() {
    let source = "\
blueprint: b
permissions: [shell]
steps:
  release:
    task:
      build:
        retry: 1
        task:
          compile: { do: shell, with: { run: make } }
      ship: { do: shell, with: { run: \"ship ${steps.build.out.compile.code}\" } }
on:
  pushed:
    event: repo.pushed
    steps:
      build:
        task:
          compile: { do: shell, with: { run: \"make ${event.data.branch}\" } }
      tell: { do: log, with: { message: \"${steps.build.out.compile.code}\" } }
outputs:
  code: ${ steps.release.out.build.compile.code }
";
    let checked = check(source);
    assert_eq!(checked.errors().count(), 0, "{:?}", checked.diagnostics);
    let blueprint = checked.blueprint.unwrap();
    let nested = &blueprint.flow.steps[0].inner.steps[0];
    assert_eq!((nested.id.as_str(), &nested.kind, nested.inner.steps.len()), ("build", &StepKind::Task, 1));
    assert_eq!(blueprint.triggers[0].flow.steps[0].kind, StepKind::Task);

    // What is wrong inside a task is found there, however deep.
    let broken = source.replace("run: make }", "run: make, shel: sh }");
    assert_eq!(error_codes(&broken), ["unknown-argument"]);
}

#[test]
fn a_task_that_runs_for_each_of_a_list_gives_its_steps_the_item() {
    let source = "\
blueprint: b
params:
  targets: [a, b]
steps:
  build:
    for_each: ${ params.targets }
    task:
      compile: { do: log, with: { message: \"${index}: ${item}\" } }
";
    assert_eq!(error_codes(source), [] as [&str; 0]);
    assert_eq!(
        error_codes(&source.replace("    for_each: ${ params.targets }\n", "")),
        ["misplaced-name"; 2]
    );
}

#[test]
fn a_task_hands_its_environment_to_the_steps_inside_it() {
    let source = "\
blueprint: b
permissions: [shell]
steps:
  build:
    env: { PROFILE: release, JOBS: 8 }
    task:
      compile: { do: shell, with: { run: make } }
      test: { do: shell, with: { run: make test }, env: { JOBS: 1 } }
";
    let blueprint = check(source).into_result().unwrap();
    let build = &blueprint.flow.steps[0];
    let env = |index: usize| serde_json::Value::Object(build.inner.steps[index].env.clone());
    assert_eq!(env(0), serde_json::json!({ "PROFILE": "release", "JOBS": 8 }));
    assert_eq!(env(1), serde_json::json!({ "PROFILE": "release", "JOBS": 1 }), "a step's own word stands");
}

#[test]
fn an_agent_asked_about_a_task_is_told_it_is_one() {
    let source = "\
blueprint: b
permissions: [shell, agents]
steps:
  build:
    task:
      compile: { do: shell, with: { run: make } }
    if_fail_ask_agent: builder
    else: build-broken
";
    let checked = check(source);
    assert_eq!(checked.errors().count(), 0, "{:?}", checked.diagnostics);
    let blueprint = checked.blueprint.unwrap();
    let ask = blueprint.flow.steps[0].on_fail.last().unwrap();
    let prompt = ask.with["prompt"].as_str().unwrap();
    assert!(prompt.starts_with("The task `build` failed:\n${self.error}"), "{prompt}");
    assert!(prompt.contains("choose `retry` to have its steps run again from the first"), "{prompt}");
}

#[test]
fn the_graph_shows_a_task_and_after_it_the_steps_inside_it() {
    let graph = graph(&check(RELEASE).into_result().unwrap());
    let nodes: Vec<(&str, &str, &str, u32)> = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.kind, node.target.as_str(), node.layer))
        .collect();
    assert_eq!(
        nodes,
        [
            ("main/fetch", "action", "shell", 0),
            ("main/build", "task", "compile, test", 1),
            ("main/build.compile", "action", "shell", 2),
            ("main/build.test", "action", "shell", 3),
            ("main/ship", "action", "shell", 4),
        ],
        "a step is drawn where it happens: what follows a task is to the right of all that is in it"
    );
    // A step inside is named as it is in a run, which is how a run is drawn on the graph.
    let names: Vec<&str> = graph.nodes.iter().map(|node| node.name.as_str()).collect();
    assert_eq!(names, ["fetch", "build", "build.compile", "build.test", "ship"]);
    assert!(graph.nodes.iter().all(|node| node.group == "main"));

    let edges: Vec<(&str, &str, &str)> =
        graph.edges.iter().map(|edge| (edge.from.as_str(), edge.to.as_str(), edge.kind)).collect();
    assert_eq!(
        edges,
        [
            ("main/fetch", "main/build", "then"),
            ("main/build", "main/build.compile", "starts"),
            ("main/build.compile", "main/build.test", "then"),
            ("main/build", "main/ship", "then"),
        ]
    );

    let build = &graph.nodes[1];
    assert_eq!(build.description.as_deref(), Some("Everything it takes to have something to ship."));
    assert_eq!((build.when.as_deref(), build.timeout.as_deref()), (Some("params.build"), Some("10m")));
    assert_eq!(build.hooks[0].point, "on_fail");
    assert_eq!((build.line, build.end_line), (7, 19), "a task's lines take in the steps inside it");
}

#[test]
fn a_task_inside_a_task_is_drawn_under_both_names() {
    let source = "\
blueprint: b
steps:
  release:
    task:
      build:
        task:
          compile: { do: noop }
      ship: { do: noop }
  tell: { do: noop }
";
    let graph = graph(&check(source).into_result().unwrap());
    let nodes: Vec<(&str, u32)> = graph.nodes.iter().map(|node| (node.id.as_str(), node.layer)).collect();
    assert_eq!(
        nodes,
        [
            ("main/release", 0),
            ("main/release.build", 1),
            ("main/release.build.compile", 2),
            ("main/release.ship", 3),
            ("main/tell", 4),
        ]
    );
}

#[test]
fn a_step_inside_a_task_is_named_one_way_wherever_it_is_named() {
    use blueprint::within;
    let build = within::task("build");
    assert_eq!(within::name(&build, "compile"), "build.compile");
    assert_eq!(within::name("", "ship"), "ship", "a step of the blueprint itself is its own name");
    // A task inside another: the inner one is named in full before its steps are.
    let inner = within::task(&within::name(&within::task("release"), "build"));
    assert_eq!(within::name(&inner, "compile"), "release.build.compile");

    // Taking one apart gives the step directly under, and no other.
    assert_eq!(within::own(&build, "build.compile"), Some("compile"));
    assert_eq!(within::own(&build, "build.pack.sign"), None, "one further in is the inner task's");
    assert_eq!(within::own(&build, "ship"), None);
    assert_eq!(within::own("", "ship"), Some("ship"));
    assert_eq!(within::own("", "build.compile"), None);
}
