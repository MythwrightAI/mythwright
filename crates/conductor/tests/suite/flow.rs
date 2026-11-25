use conductor::{InstanceStatus, RunStatus, StepStatus};
use serde_json::json;

use crate::{Rig, statuses, step};

#[tokio::test]
async fn steps_run_top_to_bottom_and_pass_results_along() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: greet
params: { name: world }
steps:
  first: { do: test.echo, with: { text: \"hello ${params.name}\" } }
  second: { do: test.echo, with: { text: \"${steps.first.out.text}!\", n: ${ len(steps.first.out.text) } } }
outputs:
  said: ${steps.second.out.text}
  length: ${steps.second.out.n}
",
            json!({ "name": "factory" }),
        )
        .await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(run.outputs, json!({ "said": "hello factory!", "length": 13 }));
    assert_eq!(rig.shared.calls(), ["first:test.echo", "second:test.echo"]);
    assert_eq!(step(&run, "first").output, json!({ "text": "hello factory" }));
    assert!(applied.created);

    let instance = rig.instance(&applied.instance).await;
    assert_eq!(instance.status, InstanceStatus::Done, "nothing keeps it alive once its steps are done");
    assert_eq!(
        rig.topics("instance.*, run.*, step.*").await,
        [
            "instance.started",
            "run.started",
            "step.started",
            "step.finished",
            "step.started",
            "step.finished",
            "run.finished",
            "instance.finished"
        ]
    );
}

#[tokio::test]
async fn independent_branches_run_side_by_side() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: fan
steps:
  start: { do: test.echo }
  left: { do: test.gate, needs: [start], with: { name: left } }
  right: { do: test.gate, needs: [start], with: { name: right } }
  join: { do: test.echo, needs: [left, right], with: { both: \"${steps.left.out.passed}+${steps.right.out.passed}\" } }
",
            json!({}),
        )
        .await;
    // Both are running at once: neither gate has been opened yet.
    rig.reached("left").await;
    rig.reached("right").await;
    assert!(!rig.shared.calls().contains(&"join:test.echo".to_string()));

    rig.open("right");
    rig.open("left");
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "join").output, json!({ "both": "left+right" }));
}

#[tokio::test]
async fn max_parallel_limits_how_many_steps_run_at_once() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: narrow
max_parallel: 1
steps:
  a: { do: test.gate, needs: [], with: { name: a } }
  b: { do: test.gate, needs: [], with: { name: b } }
",
            json!({}),
        )
        .await;
    rig.reached("a").await;
    assert_eq!(rig.shared.calls(), ["a:test.gate"], "b must wait its turn");
    rig.open("a");
    rig.reached("b").await;
    rig.open("b");
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
}

#[tokio::test]
async fn a_guard_skips_a_step_and_what_follows_still_runs() {
    let rig = Rig::new().await;
    let source = "\
blueprint: branch
params: { enough: true }
steps:
  check: { do: test.echo, with: { ok: ${params.enough} } }
  ask: { when: ${ !steps.check.out.ok }, do: test.echo, with: { asked: true } }
  build: { when: ${ steps.check.out.ok }, do: test.echo, with: { built: true } }
  report:
    do: test.echo
    with: { asked: ${ steps.ask.ok }, built: ${ steps.build.ok }, skipped: ${ steps.ask.skipped } }
";
    let run = rig.run(source, json!({})).await;
    assert_eq!(
        statuses(&run, &["check", "ask", "build", "report"]),
        [("check".into(), "ok"), ("ask".into(), "skipped"), ("build".into(), "ok"), ("report".into(), "ok")]
    );
    assert_eq!(step(&run, "report").output, json!({ "asked": false, "built": true, "skipped": true }));

    let run = rig.run(source, json!({ "enough": false })).await;
    assert_eq!(step(&run, "ask").status, StepStatus::Ok);
    assert_eq!(step(&run, "build").status, StepStatus::Skipped);
}

#[tokio::test]
async fn end_stops_the_run_where_it_is() {
    let rig = Rig::new().await;
    let source = "\
blueprint: early
params: { status: ok }
steps:
  a: { do: test.echo }
  stop: { do: end, with: { status: ${params.status}, reason: nothing to do } }
  never: { do: test.echo }
";
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(statuses(&run, &["a", "stop", "never"])[2], ("never".into(), "skipped"));

    let run = rig.run(source, json!({ "status": "failed" })).await;
    assert_eq!((run.status, run.error.as_deref()), (RunStatus::Failed, Some("nothing to do")));
}

#[tokio::test]
async fn for_each_runs_the_step_once_per_element() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: each
params: { weights: [10, 50, 100] }
steps:
  shift:
    for_each: ${ params.weights }
    do: test.echo
    with: { weight: ${item}, position: ${index} }
  after: { do: test.echo, with: { last: ${ steps.shift.out[-1].weight }, count: ${ len(steps.shift.out) } } }
",
            json!({}),
        )
        .await;
    assert_eq!(
        step(&run, "shift").output,
        json!([{ "weight": 10, "position": 0 }, { "weight": 50, "position": 1 }, { "weight": 100, "position": 2 }])
    );
    assert_eq!(step(&run, "after").output, json!({ "last": 100, "count": 3 }));
}

#[tokio::test]
async fn for_each_over_a_number_runs_the_step_so_many_times_counting_from_one() {
    let rig = Rig::new().await;
    let source = "\
blueprint: counted-out
params: { seats: { type: any, default: 3 } }
steps:
  seat: { for_each: ${params.seats}, do: test.echo, with: { seat: ${item}, position: ${index} } }
outputs:
  seats: ${steps.seat.out}
";
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        run.outputs["seats"],
        json!([{ "seat": 1, "position": 0 }, { "seat": 2, "position": 1 }, { "seat": 3, "position": 2 }])
    );
    // None, and less than none, is no times; as a list of none is.
    for none in [json!(0), json!(-2)] {
        let run = rig.run(source, json!({ "seats": none })).await;
        assert_eq!((run.status, &run.outputs["seats"]), (RunStatus::Ok, &json!([])), "{:?}", run.error);
    }
    // What cannot be counted up to is refused as `range()` refuses it.
    for (seats, why) in [
        (json!(2.5), "`for_each` over a number takes a whole number"),
        (json!(1001), "`for_each` over a number goes to 1000 at most, and was asked for 1001"),
        (json!("three"), "`for_each` must produce a list, or a number to count up to; got \"three\""),
    ] {
        let run = rig.run(source, json!({ "seats": seats })).await;
        assert_eq!(run.status, RunStatus::Failed);
        assert!(run.error.as_deref().unwrap_or_default().contains(why), "{:?}", run.error);
    }
}

#[tokio::test]
async fn a_step_says_how_many_things_it_gave_where_it_gave_a_list_and_nothing_where_it_did_not() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: counted
params: { weights: [10, 50, 100], none: [] }
steps:
  shift: { for_each: ${ params.weights }, do: test.echo, with: { weight: ${item} } }
  nothing: { for_each: ${ params.none }, do: test.echo, with: { weight: ${item} } }
  one: { do: test.echo, with: { weight: 1 } }
  passed: { when: ${ params.none == 1 }, for_each: ${ params.weights }, do: test.echo }
outputs:
  shift: ${steps.shift.count}
  nothing: ${steps.nothing.count}
  one: ${steps.one.count}
  passed: ${steps.passed.count}
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    // One for each element; none for a list of none, which is a number;
    // and nothing for what is not a list, or was never done.
    assert_eq!(run.outputs, json!({ "shift": 3, "nothing": 0, "one": null, "passed": null }));
}

#[tokio::test]
async fn state_is_set_by_steps_and_seen_by_later_ones() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: counter
params: { start: 5 }
state: { n: ${params.start}, note: none }
steps:
  bump: { do: set, with: { n: ${ state.n + 1 }, note: bumped } }
  read: { do: test.echo, with: { n: ${state.n}, note: ${state.note} } }
",
            json!({}),
        )
        .await;
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "read").output, json!({ "n": 6, "note": "bumped" }));
    assert_eq!(
        rig.instance(&applied.instance).await.state,
        kernel::fields(json!({ "n": 6, "note": "bumped" }))
    );
    let change = rig.event("instance.state", |_| true).await;
    assert_eq!(change.data, json!({ "n": 6, "note": "bumped" }));
}

#[tokio::test]
async fn emit_publishes_an_event_attributed_to_its_step() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "blueprint: shout\nparams: { code: 5 }\nsteps:\n  say: { do: emit, with: { topic: build.slow, msg: build slow, code: ${params.code} } }\n",
            json!({}),
        )
        .await;
    rig.finished(&applied).await;
    let event = rig.event("build.slow", |_| true).await;
    assert_eq!(event.data, json!({ "msg": "build slow", "code": 5 }));
    assert_eq!(event.source.instance.as_deref(), Some(applied.instance.as_str()));
    assert_eq!(event.source.step.as_deref(), Some("say"));
    assert_eq!(event.source.actor.as_deref(), Some("user:ada"));
}

#[tokio::test]
async fn a_steps_output_is_logged_and_readable() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "blueprint: noisy\nsteps:\n  talk: { do: test.gate, with: { name: g, lines: [compiling kv-cache, finished] } }\n  note: { do: log, with: { message: all done } }\n",
            json!({}),
        )
        .await;
    rig.open("g");
    let run = rig.finished(&applied).await;
    let lines: Vec<String> =
        rig.engine.step_log(&run.id, "talk", 0, 10).into_iter().map(|line| line.text).collect();
    assert_eq!(lines, ["compiling kv-cache", "finished"]);
    assert_eq!(rig.engine.step_log(&run.id, "note", 0, 10)[0].text, "all done");
}

#[tokio::test]
async fn what_is_applied_for_a_project_is_measured_and_listed_as_that_projects() {
    let rig = Rig::new().await;
    let source = "blueprint: timed\nsteps:\n  ship: { do: test.echo, measure: build }\n";
    let for_shop = conductor::Apply {
        project: Some("shop".into()),
        key: Some("for-shop".into()),
        ..rig.request(source, json!({}))
    };
    let for_shop = rig.engine.apply(for_shop).await.unwrap();
    let for_nobody = rig.apply(source, json!({})).await;
    rig.finished(&for_shop).await;
    rig.finished(&for_nobody).await;

    let count = async |name: &str, project: Option<&str>| {
        let labels = project.iter().map(|project| ("project".to_string(), project.to_string())).collect();
        let query = journal::MetricQuery { name: name.into(), since_ms: 0, labels, ..Default::default() };
        rig.journal.query_metrics(query).await.unwrap().first().map_or(0, |series| series.total.count)
    };
    for name in ["run", "step", "build"] {
        assert_eq!(count(name, None).await, 2, "`{name}` across everything");
        assert_eq!(count(name, Some("shop")).await, 1, "`{name}` for the project");
        assert_eq!(count(name, Some("other")).await, 0, "`{name}` for a project with nothing applied");
    }

    let runs = rig.engine.recent_runs(10, Some("shop".into())).await.unwrap();
    assert_eq!(
        runs.iter().map(|run| run.instance.as_str()).collect::<Vec<_>>(),
        [for_shop.instance.as_str()]
    );
    assert_eq!(rig.engine.recent_runs(10, None).await.unwrap().len(), 2);
}

#[tokio::test]
async fn every_step_is_measured_and_named_measures_are_reported_separately() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "blueprint: timed\nsteps:\n  compile: { do: test.sleep, measure: build, with: { for: 90s } }\n  ship: { do: test.echo }\n",
            json!({}),
        )
        .await;
    rig.advance_after(0, 1, "90s").await;
    rig.finished(&applied).await;

    let query = |name: &str, group: &str| journal::MetricQuery {
        name: name.into(),
        since_ms: 0,
        group_by: vec![group.into()],
        ..Default::default()
    };
    let steps = rig.journal.query_metrics(query("step", "step")).await.unwrap();
    let by_step: Vec<(&str, u64, f64)> = steps
        .iter()
        .map(|series| (series.labels["step"].as_str(), series.total.count, series.total.max))
        .collect();
    assert_eq!(by_step, [("compile", 1, 90_000.0), ("ship", 1, 0.0)]);

    let builds = rig.journal.query_metrics(query("build", "blueprint")).await.unwrap();
    assert_eq!((builds[0].labels["blueprint"].as_str(), builds[0].total.avg), ("timed", 90_000.0));
    let runs = rig.journal.query_metrics(query("run", "flow")).await.unwrap();
    assert_eq!((runs[0].total.count, runs[0].total.errors), (1, 0));
}

/// The names of what an expression found, in order, to set beside what the
/// language says is there.
fn found(keys: &serde_json::Value) -> Vec<String> {
    let mut names: Vec<String> =
        keys.as_array().unwrap().iter().map(|key| key.as_str().unwrap().to_string()).collect();
    names.sort();
    names
}

fn described(choices: &[blueprint::vocabulary::Choice]) -> Vec<String> {
    let mut names: Vec<String> = choices.iter().map(|choice| choice.name.to_string()).collect();
    names.sort();
    names
}

#[tokio::test]
async fn what_an_editor_is_told_follows_a_step_and_an_instance_is_what_an_expression_finds_there() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: looks
steps:
  first: { do: test.echo }
outputs:
  step: ${ keys(steps.first) }
  instance: ${ keys(instance) }
",
            json!({}),
        )
        .await;
    let run = rig.finished(&applied).await;

    assert_eq!(found(&run.outputs["step"]), described(blueprint::vocabulary::STEP_VIEW));
    assert_eq!(found(&run.outputs["instance"]), described(blueprint::vocabulary::INSTANCE_VIEW));
}
