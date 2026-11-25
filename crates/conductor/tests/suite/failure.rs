use conductor::{Apply, InstanceStatus, RunStatus, StepStatus};
use kernel::{ErrorKind, Principal, PrincipalKind};
use serde_json::json;

use crate::{Rig, statuses, step, user};

#[tokio::test]
async fn a_failed_step_stops_the_run_and_what_never_ran_says_so() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: broken
steps:
  fine: { do: test.echo }
  bad: { do: test.flaky, with: { message: out of disk } }
  never: { do: test.echo }
",
            json!({}),
        )
        .await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.error.as_deref(), Some("step `bad` failed: out of disk"));
    assert_eq!(
        statuses(&run, &["fine", "bad", "never"]),
        [("fine".into(), "ok"), ("bad".into(), "failed"), ("never".into(), "cancelled")]
    );
    assert_eq!(step(&run, "bad").output, json!({ "call": 1 }), "what the action reported is kept");

    let failed = rig.event("instance.failed", |_| true).await;
    assert_eq!(failed.data["error"], "step `bad` failed: out of disk");
    assert_eq!(rig.instance(&applied.instance).await.status, InstanceStatus::Failed);
}

#[tokio::test]
async fn a_failure_cancels_steps_still_running_beside_it() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: race
steps:
  slow: { do: test.gate, needs: [], with: { name: never } }
  bad: { do: test.gate, needs: [], with: { name: go } }
  boom: { do: test.flaky, needs: [bad] }
",
            json!({}),
        )
        .await;
    rig.reached("never").await;
    rig.open("go");
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "slow").status, StepStatus::Cancelled);
    assert_eq!(step(&run, "boom").status, StepStatus::Failed);
}

#[tokio::test]
async fn a_tolerated_failure_lets_the_run_carry_on() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: lenient
steps:
  lint: { do: test.flaky, allow_failure: true, with: { message: 3 warnings } }
  next: { do: test.echo, with: { lint_failed: ${ steps.lint.failed }, why: ${ steps.lint.error } } }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok);
    assert!(step(&run, "lint").tolerated);
    assert_eq!(step(&run, "next").output, json!({ "lint_failed": true, "why": "3 warnings" }));
}

#[tokio::test]
async fn retries_back_off_and_then_succeed() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "blueprint: flaky\nsteps:\n  work: { do: test.flaky, with: { times: 2 }, retry: { max: 3, backoff: 10s, factor: 2 } }\n",
            json!({}),
        )
        .await;
    rig.advance_after(since, 1, "10s").await;
    rig.advance_after(since, 2, "20s").await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(step(&run, "work").attempt, 3);
    assert_eq!(step(&run, "work").output, json!({ "calls": 3 }));
    let waits: Vec<i64> =
        rig.events("step.retrying").await.iter().map(|event| event.data["in_ms"].as_i64().unwrap()).collect();
    assert_eq!(waits, [10_000, 20_000]);
}

#[tokio::test]
async fn retries_run_out() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "blueprint: hopeless\nsteps:\n  work: { do: test.flaky, retry: { max: 2, backoff: 1s } }\n",
            json!({}),
        )
        .await;
    rig.advance_after(since, 1, "1s").await;
    rig.advance_after(since, 2, "2s").await;
    let run = rig.finished(&applied).await;
    assert_eq!((run.status, step(&run, "work").attempt), (RunStatus::Failed, 3));
    assert_eq!(rig.shared.count("flaky:work"), 3);
}

#[tokio::test]
async fn hooks_run_around_a_step_and_a_broken_hook_does_not_break_it() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: hooked
steps:
  work:
    do: test.echo
    with: { v: 1 }
    on_start:
      - do: test.count
        with: { key: started }
    on_finish:
      - do: test.flaky
      - emit: { topic: work.done, v: ${ self.out.v }, ok: ${ steps.work.ok } }
    on_fail:
      - emit: { topic: work.failed }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok, "a failing hook is reported, not fatal");
    assert_eq!(rig.shared.count("started"), 1);
    assert_eq!(rig.event("work.done", |_| true).await.data, json!({ "v": 1, "ok": true }));
    let broken = rig.event("hook.failed", |_| true).await;
    assert_eq!(broken.data, json!({ "action": "test.flaky", "error": "boom" }));
    assert!(rig.events("work.failed").await.is_empty());
}

#[tokio::test]
async fn a_failure_hook_can_send_the_step_round_again() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: recover
steps:
  build:
    do: test.flaky
    with: { times: 1, message: missing semicolon }
    on_fail:
      - emit: { topic: build.broken, why: ${ self.error } }
      - do: test.decide
        with: { decision: retry }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(step(&run, "build").attempt, 2);
    assert_eq!(rig.event("build.broken", |_| true).await.data["why"], "missing semicolon");
    let retry = rig.event("step.retrying", |_| true).await;
    assert_eq!(retry.data["by"], "hook");
}

#[tokio::test]
async fn a_failure_hook_cannot_retry_forever() {
    let rig = Rig::new().await;
    let run = rig
        .run("blueprint: stubborn\nsteps:\n  build: { do: test.flaky, on_fail: [retry] }\n", json!({}))
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(step(&run, "build").attempt, 3, "the first try and two recoveries");
}

#[tokio::test]
async fn a_failure_hook_can_wave_the_failure_through() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: shrug
steps:
  optional:
    do: test.flaky
    on_fail:
      - do: test.decide
        with: { decision: continue }
  next: { do: test.echo }
",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Ok);
    assert!(step(&run, "optional").tolerated);
    assert_eq!(step(&run, "next").status, StepStatus::Ok);
}

#[tokio::test]
async fn an_argument_that_turns_out_wrong_fails_the_step_clearly() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "blueprint: typed\nparams: { times: { type: any, default: lots } }\nsteps:\n  work: { do: test.flaky, with: { times: ${params.times} } }\n",
            json!({}),
        )
        .await;
    assert_eq!(step(&run, "work").error.as_deref(), Some("`test.flaky`: `times` must be int, got \"lots\""));
}

#[tokio::test]
async fn a_crashing_action_fails_the_run_instead_of_hanging_it() {
    let rig = Rig::new().await;
    let run = rig
        .run("blueprint: buggy\nsteps:\n  crash: { do: test.panic }\n  after: { do: test.echo }\n", json!({}))
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    assert!(run.error.as_deref().unwrap().starts_with("a step crashed"), "{:?}", run.error);
}

#[tokio::test]
async fn a_blueprint_may_only_use_what_it_declares_and_what_its_caller_holds() {
    let rig = Rig::new().await;
    let source = "blueprint: sh\npermissions: [shell]\nsteps:\n  a: { do: test.shell }\n";

    let run = rig.run(source, json!({})).await;
    assert_eq!(step(&run, "a").output, json!({ "ran_as": "Ada" }));

    // Declared, but the person applying it does not hold it.
    let limited = Principal {
        kind: PrincipalKind::Agent,
        id: "builder".into(),
        name: "builder".into(),
        scopes: vec!["blueprints".into()],
        key: None,
    };
    let applied = rig
        .engine
        .apply(Apply { principal: limited, key: Some("limited".into()), ..rig.request(source, json!({})) })
        .await
        .unwrap();
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "a").error.as_deref(), Some("builder may not use `shell`"));

    // Not declared: refused before anything runs.
    let err = rig
        .engine
        .apply(rig.request("blueprint: sh\npermissions: []\nsteps:\n  a: { do: test.shell }\n", json!({})))
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Invalid);
    assert!(
        err.message.contains("sh.bp:4:3: error: this uses `shell`, which is not listed under `permissions`"),
        "{}",
        err.message
    );
}

#[tokio::test]
async fn applying_refuses_what_it_cannot_run() {
    let rig = Rig::new().await;

    let err = rig.engine.apply(Apply::new("nowhere", user())).await.unwrap_err();
    assert_eq!(
        (err.kind, err.message.as_str()),
        (ErrorKind::NotFound, "there is no blueprint called `nowhere`")
    );

    let err = rig
        .engine
        .apply(rig.request("blueprint: bad\nsteps:\n  a: { do: nope }\n", json!({})))
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Invalid);
    assert!(err.message.contains("`bad` has 1 problem(s)"), "{}", err.message);
    assert!(err.message.contains("bad.bp:3:3: error: there is no action called `nope`"), "{}", err.message);

    let ok = "blueprint: p\nparams: { n: 1 }\nsteps:\n  a: { do: noop }\n";
    let err = rig.engine.apply(rig.request(ok, json!({ "n": "one" }))).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Invalid);
    assert!(err.message.contains("must be int"), "{}", err.message);

    let nobody = Principal {
        kind: PrincipalKind::User,
        id: "eve".into(),
        name: "Eve".into(),
        scopes: vec![],
        key: None,
    };
    let err = rig.engine.apply(Apply { principal: nobody, ..rig.request(ok, json!({})) }).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Denied);
    assert!(rig.engine.instances(Default::default()).await.unwrap().is_empty(), "nothing was started");
}

#[tokio::test]
async fn a_run_that_fails_says_so_and_then_that_it_finished_and_another_handler_can_hear_it() {
    let rig = Rig::new().await;
    // One handler fails every time. Another of the same instance hears
    // that a run of it failed, and nothing else: not a run that went well.
    let source = "\
blueprint: careless
on:
  work:
    event: job.wanted
    steps:
      slip: { when: ${ event.data.bad }, do: test.flaky, with: { message: it slipped } }
      done: { do: test.count, with: { key: done } }
  trouble:
    event: run.failed
    scope: own
    do: test.count
    with: { key: \"trouble with ${ event.data.flow }: ${ event.data.error }\" }
";
    let applied = rig.apply(source, json!({})).await;
    rig.event("instance.active", |_| true).await;
    for bad in [false, true] {
        rig.journal.emit("job.wanted", kernel::Source::default(), json!({ "bad": bad })).await.unwrap();
    }
    let heard = rig.event("run.finished", |event| event.data["flow"] == "on.trouble").await;
    assert_eq!(heard.data["status"], "ok");
    assert_eq!(rig.shared.count("done"), 1);
    assert_eq!(rig.shared.count("trouble with on.work: step `slip` failed: it slipped"), 1);

    // Failed is said first, and finished after it, as it is after any
    // ending: what went well says only that it finished.
    let of_work: Vec<(String, String)> = rig
        .events("run.*")
        .await
        .iter()
        .filter(|event| event.data["flow"] == "on.work" && event.topic != "run.started")
        .map(|event| (event.topic.clone(), event.data["status"].as_str().unwrap_or("-").to_string()))
        .collect();
    assert_eq!(
        of_work,
        [
            ("run.finished".to_string(), "ok".to_string()),
            ("run.failed".to_string(), "-".to_string()),
            ("run.finished".to_string(), "failed".to_string()),
        ]
    );
    // The instance is as it was: one of its handlers failing is not it failing.
    assert_eq!(rig.instance(&applied.instance).await.status, conductor::InstanceStatus::Active);
    assert!(rig.events("instance.failed").await.is_empty());
}

#[tokio::test]
async fn the_steps_of_a_blueprint_failing_say_run_failed_too_and_its_own_on_fail_is_theirs_alone() {
    let rig = Rig::new().await;
    // What it says under `on_fail` is about its steps. A handler of its
    // that fails later does not set it off.
    let source = "\
blueprint: rollout
params: { broken: false }
steps:
  ship: { when: ${ params.broken }, do: test.flaky, with: { message: bad release } }
on_fail:
  - do: test.count
    with: { key: rolled-back }
on:
  poke:
    event: rollout.poke
    steps:
      slip: { do: test.flaky, with: { message: it slipped } }
";
    // A handler of a running one fails: its `on_fail` is not done.
    let good = rig.apply(source, json!({})).await;
    rig.event("instance.active", |_| true).await;
    rig.journal.emit("rollout.poke", kernel::Source::default(), json!({})).await.unwrap();
    let failed = rig.event("run.failed", |_| true).await;
    assert_eq!(
        (failed.data["flow"].as_str(), failed.data["error"].as_str()),
        (Some("on.poke"), Some("step `slip` failed: it slipped"))
    );
    rig.event("run.finished", |event| event.data["flow"] == "on.poke").await;
    assert_eq!(rig.shared.count("rolled-back"), 0);
    rig.engine.stop(&good.instance).await.unwrap();

    // Its steps fail: `run.failed` for `main`, and its `on_fail` is done.
    let bad = rig
        .engine
        .apply(Apply { key: Some("second".into()), ..rig.request(source, json!({ "broken": true })) })
        .await
        .unwrap();
    assert_eq!(rig.finished(&bad).await.status, RunStatus::Failed);
    let failed = rig.event("run.failed", |event| event.data["flow"] == "main").await;
    assert_eq!(failed.data["error"], "step `ship` failed: bad release");
    assert_eq!(rig.shared.count("rolled-back"), 1);
}
