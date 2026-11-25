//! A task: steps that run as one, under one name and one policy.

use conductor::{RunInfo, RunStatus, StepStatus};
use kernel::Source;
use serde_json::json;

use crate::{Rig, statuses, step};

/// The names of a run's steps, as the run recorded them.
fn recorded(run: &RunInfo) -> Vec<&str> {
    let mut names: Vec<&str> = run.steps.iter().map(|step| step.step.as_str()).collect();
    names.sort_unstable();
    names
}

#[tokio::test]
async fn the_steps_of_a_task_run_in_order_and_are_read_through_it_afterwards() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: release
steps:
  fetch: { do: test.echo, with: { rev: abc } }
  build:
    task:
      compile: { do: test.echo, with: { of: ${ steps.fetch.out.rev }, artifact: app.tgz } }
      test: { do: test.echo, with: { tested: ${ steps.compile.out.artifact } } }
  ship:
    do: test.echo
    with: { shipped: ${ steps.build.out.compile.artifact }, tested: ${ steps.build.out.test.tested } }
outputs:
  artifact: ${ steps.build.out.compile.artifact }
",
            json!({}),
        )
        .await;

    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        rig.shared.calls(),
        ["fetch:test.echo", "build.compile:test.echo", "build.test:test.echo", "ship:test.echo"],
        "a step inside a task is `<task>.<step>` to the run"
    );
    assert_eq!(step(&run, "build.compile").output, json!({ "of": "abc", "artifact": "app.tgz" }));
    assert_eq!(step(&run, "build.test").output, json!({ "tested": "app.tgz" }), "it read the step before it");
    assert_eq!(
        step(&run, "build").output,
        json!({ "compile": { "of": "abc", "artifact": "app.tgz" }, "test": { "tested": "app.tgz" } }),
        "a task gives out what each of its steps gave out, by name"
    );
    assert_eq!(step(&run, "ship").output, json!({ "shipped": "app.tgz", "tested": "app.tgz" }));
    assert_eq!(run.outputs, json!({ "artifact": "app.tgz" }));
}

#[tokio::test]
async fn a_task_and_each_step_in_it_say_when_they_start_and_finish_and_are_measured() {
    let rig = Rig::new().await;
    rig.run(
        "\
blueprint: release
steps:
  build:
    measure: build
    task:
      compile: { do: test.echo }
      test: { do: test.echo }
",
        json!({}),
    )
    .await;

    let said: Vec<(String, String)> = rig
        .events("step.*")
        .await
        .into_iter()
        .map(|event| (event.topic, event.source.step.unwrap_or_default()))
        .collect();
    let said: Vec<(&str, &str)> = said.iter().map(|(topic, step)| (topic.as_str(), step.as_str())).collect();
    assert_eq!(
        said,
        [
            ("step.started", "build"),
            ("step.started", "build.compile"),
            ("step.finished", "build.compile"),
            ("step.started", "build.test"),
            ("step.finished", "build.test"),
            ("step.finished", "build"),
        ]
    );
    assert_eq!(rig.event("step.started", |_| true).await.data["verb"], "task");

    let query = |name: &str| journal::MetricQuery {
        name: name.into(),
        since_ms: 0,
        group_by: vec!["step".into(), "action".into()],
        ..Default::default()
    };
    let steps = rig.journal.query_metrics(query("step")).await.unwrap();
    let by_step: Vec<(&str, &str, u64)> = steps
        .iter()
        .map(|series| (series.labels["step"].as_str(), series.labels["action"].as_str(), series.total.count))
        .collect();
    assert_eq!(
        by_step,
        [("build", "task", 1), ("build.compile", "test.echo", 1), ("build.test", "test.echo", 1)]
    );
    let builds = rig.journal.query_metrics(query("build")).await.unwrap();
    assert_eq!(builds[0].total.count, 1, "a task that names itself is reported under that name as a whole");
}

#[tokio::test]
async fn steps_in_a_task_that_do_not_wait_for_each_other_run_side_by_side() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: release
steps:
  build:
    task:
      linux: { do: test.gate, with: { name: linux } }
      mac: { do: test.gate, needs: [], with: { name: mac } }
      both: { do: test.echo, needs: [linux, mac], with: { got: ${ steps.mac.out.passed } } }
",
            json!({}),
        )
        .await;
    // Neither gate is open yet, so both being reached is both running.
    rig.reached("linux").await;
    rig.reached("mac").await;
    rig.open("mac");
    rig.open("linux");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(step(&run, "build.both").output, json!({ "got": "mac" }));
}

#[tokio::test]
async fn a_step_that_fails_in_a_task_fails_the_task_and_the_rest_of_it_does_not_run() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: release
steps:
  build:
    task:
      compile: { do: test.echo, with: { artifact: app.tgz } }
      test: { do: test.flaky, with: { message: 2 tests failed } }
      package: { do: test.echo }
  ship: { do: test.echo }
",
            json!({}),
        )
        .await;

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.error.as_deref(), Some("step `build` failed: step `test` failed: 2 tests failed"));
    assert_eq!(
        statuses(&run, &["build", "build.compile", "build.test", "build.package", "ship"]),
        [
            ("build".into(), "failed"),
            ("build.compile".into(), "ok"),
            ("build.test".into(), "failed"),
            ("build.package".into(), "cancelled"),
            ("ship".into(), "cancelled"),
        ]
    );
    assert_eq!(rig.shared.calls(), ["build.compile:test.echo", "build.test:test.flaky"]);
    assert_eq!(
        step(&run, "build").output,
        json!({ "compile": { "artifact": "app.tgz" }, "test": { "call": 1 } }),
        "what its steps gave out before it failed is kept"
    );
}

#[tokio::test]
async fn a_task_whose_guard_is_false_is_skipped_whole_and_what_follows_still_runs() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: release
params: { build: false }
steps:
  build:
    when: ${ params.build }
    task:
      compile: { do: test.echo }
      test: { do: test.echo }
  ship: { do: test.echo, with: { built: ${ !steps.build.skipped } } }
",
            json!({}),
        )
        .await;

    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(step(&run, "build").status, StepStatus::Skipped);
    assert_eq!(recorded(&run), ["build", "ship"], "nothing inside it began");
    assert_eq!(rig.shared.calls(), ["ship:test.echo"]);
    assert_eq!(step(&run, "ship").output, json!({ "built": false }));
}

#[tokio::test]
async fn a_task_that_is_tried_again_runs_all_its_steps_again_from_the_first() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: release
steps:
  build:
    retry: { max: 2, backoff: 10s }
    task:
      compile: { do: test.count, with: { key: compiles } }
      test: { do: test.flaky, with: { times: 1 } }
  ship: { do: test.echo }
",
            json!({}),
        )
        .await;
    rig.advance_after(since, 1, "10s").await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        rig.shared.calls(),
        [
            "build.compile:test.count",
            "build.test:test.flaky",
            "build.compile:test.count",
            "build.test:test.flaky",
            "ship:test.echo"
        ]
    );
    assert_eq!(step(&run, "build").attempt, 2);
    assert_eq!(step(&run, "build").output, json!({ "compile": { "count": 2 }, "test": { "calls": 2 } }));
    let retrying = rig.event("step.retrying", |_| true).await;
    assert_eq!(retrying.source.step.as_deref(), Some("build"), "it is the task that is tried again");
    assert_eq!(retrying.data["error"], "step `test` failed: boom");
}

#[tokio::test]
async fn what_a_task_does_when_it_fails_is_done_once_when_it_has_failed_for_good() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: release
steps:
  build:
    retry: { max: 1, backoff: 1s }
    task:
      compile:
        do: test.flaky
        with: { message: no compiler }
        on_fail:
          - { do: test.count, with: { key: compile_failed } }
    on_fail:
      - { do: test.count, with: { key: build_failed } }
      - emit: { topic: build.broken, why: ${ self.error }, tries: ${ self.attempts } }
",
            json!({}),
        )
        .await;
    rig.advance_after(since, 1, "1s").await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(rig.shared.count("flaky:build.compile"), 2, "the task was tried twice");
    assert_eq!(rig.shared.count("compile_failed"), 2, "a step in a task keeps its own policy, each time");
    assert_eq!(rig.shared.count("build_failed"), 1);
    let broken = rig.events("build.broken").await;
    assert_eq!(broken.len(), 1);
    assert_eq!(broken[0].data, json!({ "why": "step `compile` failed: no compiler", "tries": 2 }));
}

#[tokio::test]
async fn a_task_whose_failure_is_tolerated_lets_the_run_carry_on() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: release
steps:
  build:
    allow_failure: true
    task:
      compile: { do: test.echo, with: { artifact: app.tgz } }
      test: { do: test.flaky, with: { message: red } }
  report:
    do: test.echo
    with: { failed: ${ steps.build.failed }, why: ${ steps.build.error }, had: ${ steps.build.out.compile.artifact } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok);
    assert!(step(&run, "build").tolerated);
    assert_eq!(
        step(&run, "report").output,
        json!({ "failed": true, "why": "step `test` failed: red", "had": "app.tgz" })
    );
}

#[tokio::test]
async fn a_task_that_overruns_its_timeout_stops_the_step_it_was_on_before_it_fails() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: release
steps:
  build:
    timeout: 1m
    task:
      compile: { do: test.sleep, with: { for: 10m } }
      test: { do: test.echo }
",
            json!({}),
        )
        .await;
    // Two sleeps: the step's own, and the task's timeout watching all of it.
    rig.advance_after(since, 2, "1m").await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(step(&run, "build").error.as_deref(), Some("timed out after 1m"));
    assert_eq!(
        statuses(&run, &["build", "build.compile", "build.test"]),
        [
            ("build".into(), "failed"),
            ("build.compile".into(), "cancelled"),
            ("build.test".into(), "cancelled")
        ]
    );
    let ended: Vec<String> = rig
        .events("step.*")
        .await
        .into_iter()
        .filter(|event| event.topic != "step.started")
        .map(|event| format!("{} {}", event.source.step.unwrap_or_default(), event.topic))
        .collect();
    assert_eq!(
        ended,
        [
            "build.compile step.cancelled",
            "build.compile step.finished",
            "build.test step.cancelled",
            "build step.failed",
            "build step.finished"
        ],
        "by the time the task says it failed, its steps have said how they ended: the one that was under way \
         that it was cancelled and then that it finished, the one that never began only that it was cancelled"
    );
}

#[tokio::test]
async fn stopping_an_instance_stops_the_step_its_task_was_on() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: release
steps:
  build:
    task:
      compile: { do: test.gate, with: { name: never } }
      test: { do: test.echo }
",
            json!({}),
        )
        .await;
    rig.reached("never").await;
    rig.engine.stop(&applied.instance).await.unwrap();

    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Cancelled);
    assert_eq!(
        statuses(&run, &["build", "build.compile", "build.test"]),
        [
            ("build".into(), "cancelled"),
            ("build.compile".into(), "cancelled"),
            ("build.test".into(), "cancelled")
        ]
    );
}

#[tokio::test]
async fn a_task_in_a_handler_runs_each_time_the_handler_fires() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: on-push
on:
  pushed:
    event: repo.pushed
    steps:
      build:
        task:
          compile: { do: test.echo, with: { branch: ${ event.data.branch } } }
          test: { do: test.echo, with: { of: ${ steps.compile.out.branch } } }
      tell: { do: emit, with: { topic: build.done, branch: ${ steps.build.out.test.of } } }
",
            json!({}),
        )
        .await;
    rig.event("instance.active", |_| true).await;
    for branch in ["main", "fix"] {
        rig.journal.emit("repo.pushed", Source::default(), json!({ "branch": branch })).await.unwrap();
    }

    let done = rig.event("build.done", |event| event.data["branch"] == "fix").await;
    let branches: Vec<String> =
        rig.events("build.done").await.iter().map(|event| event.data["branch"].to_string()).collect();
    assert_eq!(branches, ["\"main\"", "\"fix\""]);

    let run =
        rig.soon("the handler's run", rig.engine.wait_run(done.source.run.as_ref().unwrap())).await.unwrap();
    assert_eq!((run.instance.as_str(), run.flow.as_str()), (applied.instance.as_str(), "on.pushed"));
    assert_eq!(recorded(&run), ["build", "build.compile", "build.test", "tell"]);
}

#[tokio::test]
async fn a_task_inside_a_task_is_named_by_both_and_fails_or_succeeds_with_it() {
    let rig = Rig::new().await;
    let source = "\
blueprint: release
params: { red: false }
steps:
  release:
    task:
      build:
        task:
          compile: { do: test.echo, with: { artifact: app.tgz } }
          test: { do: test.flaky, when: ${ params.red }, with: { message: red } }
      ship: { do: test.echo, with: { what: ${ steps.build.out.compile.artifact } } }
outputs:
  shipped: ${ steps.release.out.ship.what }
";
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        recorded(&run),
        ["release", "release.build", "release.build.compile", "release.build.test", "release.ship"]
    );
    assert_eq!(step(&run, "release.build.test").status, StepStatus::Skipped);
    assert_eq!(run.outputs, json!({ "shipped": "app.tgz" }));

    let run = rig.run(source, json!({ "red": true })).await;
    assert_eq!(
        run.error.as_deref(),
        Some("step `release` failed: step `build` failed: step `test` failed: red")
    );
    assert_eq!(step(&run, "release.ship").status, StepStatus::Cancelled);
}

#[tokio::test]
async fn a_task_runs_once_for_each_of_a_list() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: release
params:
  targets: [linux, mac]
steps:
  build:
    for_each: ${ params.targets }
    task:
      compile: { do: test.echo, with: { for: ${ item } } }
      test: { do: test.echo, with: { of: ${ steps.compile.out.for }, at: ${ index } } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        step(&run, "build").output,
        json!([
            { "compile": { "for": "linux" }, "test": { "of": "linux", "at": 0 } },
            { "compile": { "for": "mac" }, "test": { "of": "mac", "at": 1 } },
        ])
    );
}

#[tokio::test]
async fn ending_the_run_from_inside_a_task_ends_it_there() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: release
steps:
  build:
    task:
      enough: { do: end, with: { reason: nothing to build } }
      compile: { do: test.echo }
  ship: { do: test.echo }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(
        statuses(&run, &["build", "build.enough", "build.compile", "ship"]),
        [
            ("build".into(), "ok"),
            ("build.enough".into(), "ok"),
            ("build.compile".into(), "skipped"),
            ("ship".into(), "skipped")
        ]
    );
    assert!(rig.shared.calls().is_empty());
}

#[tokio::test]
async fn after_a_restart_a_task_carries_on_from_the_step_it_was_on() {
    let mut rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: release
steps:
  build:
    task:
      once: { do: test.count, with: { key: once } }
      nap: { do: test.sleep, with: { for: 10m } }
      last: { do: test.count, with: { key: last } }
  ship: { do: test.echo, with: { after: ${ steps.build.out.once.count } } }
",
            json!({}),
        )
        .await;
    rig.soon("the nap to start", rig.clock.sleeps_started(since + 1)).await;

    rig.restart().await;
    let since = rig.clock.sleep_count();
    rig.advance_after(since, 1, "10m").await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(rig.shared.count("once"), 1, "it had finished before the restart");
    assert_eq!(
        rig.shared.calls().iter().filter(|call| *call == "build.nap:test.sleep").count(),
        2,
        "safe to repeat, so repeated"
    );
    assert_eq!(rig.shared.count("last"), 1);
    assert_eq!(
        step(&run, "ship").output,
        json!({ "after": 1 }),
        "what was done before the restart is still read"
    );
}

#[tokio::test]
async fn after_a_restart_a_task_on_its_second_go_does_not_take_its_first_for_it() {
    let mut rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: release
steps:
  build:
    retry: { max: 1, backoff: 1s }
    task:
      compile: { do: test.durable, with: { name: compile } }
      test: { do: test.flaky, with: { times: 1 } }
",
            json!({}),
        )
        .await;
    // The first go fails at `test`. The second is caught while `compile`
    // is under way, with nothing yet said of `test` this time round.
    rig.reached("compile").await;
    rig.open("compile");
    rig.advance_after(since, 1, "1s").await;
    let mut reached = rig.follow("test.reached");
    rig.soon("`compile` to be under way a second time", async {
        reached.next().await.unwrap();
        reached.next().await.unwrap();
    })
    .await;

    rig.restart().await;
    rig.open("compile");
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        (rig.shared.count("durable:run"), rig.shared.count("durable:resume")),
        (2, 1),
        "the step it was on is picked up where it was"
    );
    assert_eq!(rig.shared.count("flaky:build.test"), 2, "how `test` went the first time is not how it went");
    assert_eq!(step(&run, "build.test").output, json!({ "calls": 2 }));
    assert_eq!(step(&run, "build").attempt, 2);
}
