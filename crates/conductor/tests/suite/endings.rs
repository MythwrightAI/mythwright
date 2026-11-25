//! How things say they have ended. `finished` is what is said after any
//! ending of something that began, with how it went; it is not the
//! opposite of `failed`. What failed says `failed` first, what was
//! cancelled `cancelled`, what was stopped `stopped`. The hooks are named
//! the same way: `on_finish` after any ending, `on_success` when it went
//! well, `on_fail` when it did not.

use conductor::{InstanceStatus, RunStatus};
use kernel::Event;
use serde_json::json;

use crate::Rig;

/// What was said of the step `id`, in order: each topic with the `status`
/// it carried, where it carried one.
fn of_step(events: &[Event], id: &str) -> Vec<String> {
    events.iter().filter(|event| event.source.step.as_deref() == Some(id)).map(said).collect()
}

fn of_instance(events: &[Event], id: &str) -> Vec<String> {
    events
        .iter()
        .filter(|event| event.source.instance.as_deref() == Some(id) && event.source.run.is_none())
        .map(said)
        .collect()
}

fn said(event: &Event) -> String {
    match event.data["status"].as_str() {
        Some(status) => format!("{} {status}", event.topic),
        None => event.topic.clone(),
    }
}

#[tokio::test]
async fn a_step_that_began_says_it_finished_however_it_went_and_one_that_never_began_does_not() {
    let rig = Rig::new().await;
    let source = "\
blueprint: steps
steps:
  good: { do: test.count, with: { key: good } }
  passed_over: { when: ${ false }, do: test.count, with: { key: never } }
  bad: { do: test.flaky, with: { message: it slipped }, allow_failure: true }
  worse: { do: test.flaky, with: { message: it broke } }
  after: { do: test.count, with: { key: never } }
";
    let mut passed = rig.follow("step.skipped");
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Failed);
    let events = rig.events("step.*").await;
    assert_eq!(of_step(&events, "good"), ["step.started", "step.finished ok"]);
    assert_eq!(of_step(&events, "bad"), ["step.started", "step.failed", "step.finished failed"]);
    assert_eq!(of_step(&events, "worse"), ["step.started", "step.failed", "step.finished failed"]);
    // Its `when` was false, or the run ended before its turn: it never
    // began, so it did not finish. One that was passed over is said as it
    // happens and, with nobody listening for it, has no place in the
    // journal (`passed.rs`).
    let said = rig.soon("the step to be said to have been passed over", passed.next()).await.unwrap();
    assert_eq!(said.source.step.as_deref(), Some("passed_over"));
    assert_eq!(of_step(&events, "passed_over"), [] as [&str; 0]);
    assert_eq!(of_step(&events, "after"), ["step.cancelled"]);
}

#[tokio::test]
async fn a_step_that_was_stopped_in_the_middle_says_it_was_cancelled_and_then_that_it_finished() {
    let rig = Rig::new().await;
    let applied = rig
        .apply("blueprint: held\nsteps:\n  wait: { do: test.gate, with: { name: door } }\n", json!({}))
        .await;
    rig.reached("door").await;
    rig.engine.stop(&applied.instance).await.unwrap();
    rig.finished(&applied).await;
    let events = rig.events("step.*").await;
    assert_eq!(of_step(&events, "wait"), ["step.started", "step.cancelled", "step.finished cancelled"]);
}

#[tokio::test]
async fn a_run_that_was_cancelled_is_counted_and_not_among_those_that_failed() {
    let rig = Rig::new().await;
    let source = |name: &str, does: &str| format!("blueprint: {name}\nsteps:\n  go: {{ {does} }}\n");
    let held = rig.apply(&source("held", "do: test.gate, with: { name: door }"), json!({})).await;
    rig.reached("door").await;
    rig.engine.stop(&held.instance).await.unwrap();
    assert_eq!(rig.finished(&held).await.status, RunStatus::Cancelled);
    let broken = rig.apply(&source("broken", "do: test.flaky, with: { message: no }"), json!({})).await;
    assert_eq!(rig.finished(&broken).await.status, RunStatus::Failed);

    let query = journal::MetricQuery {
        name: "run".into(),
        since_ms: 0,
        group_by: vec!["blueprint".into()],
        ..Default::default()
    };
    let runs = rig.journal.query_metrics(query).await.unwrap();
    let counted: Vec<(&str, u64, u64)> = runs
        .iter()
        .map(|series| (series.labels["blueprint"].as_str(), series.total.count, series.total.errors))
        .collect();
    // What failed said `run.failed`; what was cancelled never did.
    assert_eq!(counted, [("broken", 1, 1), ("held", 1, 0)]);
    assert_eq!(rig.events("run.failed").await.len(), 1);
}

#[tokio::test]
async fn a_run_whose_last_step_stops_its_own_instance_ended_well_whichever_came_first() {
    let rig = Rig::new().await;
    // What is said as the stopping step ends is what makes the order:
    // the stop is asked for beside the step, and is there, the run
    // cancelled with it, before the step has said both. Without them the
    // run is over first here, and on a machine at work either comes first.
    let source = "\
blueprint: decides
on:
  told:
    event: decides.told
    steps:
      said: { do: test.count, with: { key: said } }
      close:
        do: stop
        on_success:
          - { do: emit, with: { topic: decides.closing } }
          - { do: emit, with: { topic: decides.closed } }
";
    let kept = rig.apply(source, json!({})).await;
    rig.event("instance.active", |event| event.source.instance == Some(kept.instance.clone())).await;
    rig.engine.emit("decides.told", kernel::Source::default(), json!({})).await;
    let ended = rig.event("run.finished", |event| event.data["flow"] == "on.told").await;
    let run = rig.engine.run(ended.source.run.as_deref().unwrap()).await.unwrap();
    assert_eq!(crate::statuses(&run, &["said", "close"]), [("said".into(), "ok"), ("close".into(), "ok")]);
    // Every step of it ended, so nothing of it was cancelled.
    assert_eq!((run.status, ended.data["status"].as_str()), (RunStatus::Ok, Some("ok")));
    rig.event("instance.finished", |event| event.source.instance == Some(kept.instance.clone())).await;
    assert_eq!(rig.instance(&kept.instance).await.status, InstanceStatus::Stopped);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_instance_whose_last_step_stops_it_is_stopped_and_never_done_whichever_comes_first() {
    // Nothing keeps it once its steps are over, so it would be done by
    // itself then; and the step that stops it cannot wait for its own
    // stop, which is begun beside it. Which came first was the machine's
    // to say, and a stretch of a discussion was now and then said to be
    // done and not stopped. On threads of their own, many at
    // once, so that some stop is begun late.
    let rig = Rig::new().await;
    let source = "blueprint: closes\nsteps:\n  said: { do: noop }\n  close: { do: stop, needs: [said] }\n";
    let mut applied = Vec::new();
    for n in 0..48 {
        let request = conductor::Apply { key: Some(format!("closes-{n}")), ..rig.request(source, json!({})) };
        applied.push(rig.engine.apply(request).await.unwrap());
    }
    let mut ended = Vec::new();
    for one in &applied {
        let said =
            rig.event("instance.finished", |event| event.source.instance == Some(one.instance.clone())).await;
        ended.push(said.data["status"].as_str().unwrap_or_default().to_string());
    }
    let done: Vec<&String> = ended.iter().filter(|status| *status != "stopped").collect();
    assert!(done.is_empty(), "of {} that stopped themselves, these ended otherwise: {done:?}", ended.len());
    let stopped = rig.events("instance.stopped").await;
    assert_eq!(stopped.len(), applied.len(), "each said it was stopped, once");
}

#[tokio::test]
async fn an_instance_says_it_finished_after_any_ending_with_how_it_ended() {
    let rig = Rig::new().await;
    // Its steps went well and nothing keeps it.
    let done =
        rig.apply("blueprint: brief\nsteps:\n  go: { do: noop }\noutputs:\n  said: yes\n", json!({})).await;
    rig.finished(&done).await;
    let ended =
        rig.event("instance.finished", |event| event.source.instance == Some(done.instance.clone())).await;
    assert_eq!(
        (ended.data["status"].as_str(), &ended.data["outputs"]),
        (Some("done"), &json!({ "said": "yes" }))
    );

    // Its first steps failed.
    let failed = rig
        .apply("blueprint: broken\nsteps:\n  go: { do: test.flaky, with: { message: no } }\n", json!({}))
        .await;
    rig.finished(&failed).await;
    let ended =
        rig.event("instance.finished", |event| event.source.instance == Some(failed.instance.clone())).await;
    assert_eq!(
        (ended.data["status"].as_str(), ended.data["error"].as_str()),
        (Some("failed"), Some("step `go` failed: no"))
    );

    // It stays, and is stopped.
    let kept = rig.apply("blueprint: kept\non:\n  poke: { event: kept.poke, do: noop }\n", json!({})).await;
    rig.event("instance.active", |event| event.source.instance == Some(kept.instance.clone())).await;
    rig.engine.stop(&kept.instance).await.unwrap();
    let ended =
        rig.event("instance.finished", |event| event.source.instance == Some(kept.instance.clone())).await;
    assert_eq!(ended.data["status"], "stopped");

    let events = rig.events("instance.*").await;
    assert_eq!(of_instance(&events, &done.instance), ["instance.started", "instance.finished done"]);
    assert_eq!(
        of_instance(&events, &failed.instance),
        ["instance.started", "instance.failed", "instance.finished failed"]
    );
    assert_eq!(
        of_instance(&events, &kept.instance),
        ["instance.started", "instance.active", "instance.stopped", "instance.finished stopped"]
    );
    assert_eq!(rig.instance(&kept.instance).await.status, InstanceStatus::Stopped);
}

#[tokio::test]
async fn a_steps_hooks_are_done_by_how_it_went_and_on_finish_after_either() {
    let rig = Rig::new().await;
    let hooks = |name: &str| {
        format!(
            "    on_success:\n      - {{ do: test.count, with: {{ key: \"{name} went well\" }} }}\n    on_fail:\n      - {{ \
             do: test.count, with: {{ key: \"{name} failed\" }} }}\n    on_finish:\n      - {{ do: test.count, with: \
             {{ key: \"{name} finished ${{ self.status }}\" }} }}\n"
        )
    };
    let source = format!(
        "blueprint: hooked\nsteps:\n  good:\n    do: test.count\n    with: {{ key: good }}\n{}  flaky:\n    do: \
         test.flaky\n    with: {{ times: 1 }}\n    on_fail: [retry]\n    on_success:\n      - {{ do: test.count, with: \
         {{ key: flaky went well }} }}\n    on_finish:\n      - {{ do: test.count, with: {{ key: \"flaky finished \
         ${{ self.status }} after ${{ self.attempts }}\" }} }}\n  bad:\n    do: test.flaky\n    with: {{ message: no }}\n    \
         allow_failure: true\n{}",
        hooks("good"),
        hooks("bad"),
    );
    let run = rig.run(&source, json!({})).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let count = |key: &str| rig.shared.count(key);

    assert_eq!((count("good went well"), count("good failed"), count("good finished ok")), (1, 0, 1));
    assert_eq!((count("bad went well"), count("bad failed"), count("bad finished failed")), (0, 1, 1));
    // Tried again by its hook, it finished once: when it had ended.
    assert_eq!((count("flaky went well"), count("flaky finished ok after 2")), (1, 1));
    // `on_fail` and `on_success` are done before `on_finish`.
    let calls = rig.shared.calls();
    let order: Vec<&str> = calls.iter().map(String::as_str).filter(|call| call.starts_with("bad:")).collect();
    assert_eq!(order, ["bad:test.flaky", "bad:test.count (on_fail 0)", "bad:test.count (on_finish 0)"]);
}

#[tokio::test]
async fn a_blueprints_hooks_are_done_by_how_its_steps_went_and_on_finish_after_either() {
    let rig = Rig::new().await;
    let source = "\
blueprint: release
params: { broken: false }
steps:
  ship: { when: ${ params.broken }, do: test.flaky, with: { message: bad release } }
on_success:
  - { do: test.count, with: { key: shipped } }
on_fail:
  - { do: test.count, with: { key: rolled back } }
on_finish:
  - { do: test.count, with: { key: \"over, ${ run.status }\" } }
";
    assert_eq!(rig.run(source, json!({})).await.status, RunStatus::Ok);
    let count = |key: &str| rig.shared.count(key);
    assert_eq!((count("shipped"), count("rolled back"), count("over, ok")), (1, 0, 1));

    let bad = rig
        .engine
        .apply(conductor::Apply {
            key: Some("second".into()),
            ..rig.request(source, json!({ "broken": true }))
        })
        .await
        .unwrap();
    assert_eq!(rig.finished(&bad).await.status, RunStatus::Failed);
    assert_eq!((count("shipped"), count("rolled back"), count("over, failed")), (1, 1, 1));
}

#[tokio::test]
async fn a_run_ended_as_failed_and_stopped_in_its_hooks_is_still_failed_when_it_is_taken_up_again() {
    let mut rig = Rig::new().await;
    let source = "\
blueprint: unfit
on_success:
  - { do: test.count, with: { key: shipped } }
on_fail:
  - { do: test.count, with: { key: rolled back } }
on_finish:
  - { do: test.gate, with: { name: hook } }
steps:
  check: { do: test.count, with: { key: checked } }
  stop: { needs: [check], do: end, with: { status: failed, reason: not fit to ship } }
  ship: { needs: [stop], do: test.count, with: { key: shipped } }
";
    let applied = rig.apply(source, json!({})).await;
    let in_hook = rig.reached("hook").await.seq;
    assert_eq!((rig.shared.count("rolled back"), rig.shared.count("shipped")), (1, 0));

    // Stopped while what is done after any ending is under way, and
    // started again: that it ended, and how, is as it was.
    rig.restart().await;
    rig.event("test.reached", move |event| event.seq > in_hook && event.data["gate"] == "hook").await;
    rig.open("hook");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Failed, "a restart does not make it have gone well");
    assert_eq!(run.error.as_deref(), Some("not fit to ship"));
    assert_eq!(
        rig.shared.count("shipped"),
        0,
        "neither the step after its end nor what is done when it goes well"
    );
    assert_eq!(rig.instance(&applied.instance).await.status, InstanceStatus::Failed);
}

#[tokio::test]
async fn a_run_a_task_ended_and_stopped_in_its_hooks_is_still_ended_when_it_is_taken_up_again() {
    let mut rig = Rig::new().await;
    let source = "\
blueprint: within
on_success:
  - { do: test.count, with: { key: went well } }
on_finish:
  - { do: test.gate, with: { name: hook } }
steps:
  group:
    task:
      look: { do: test.count, with: { key: looked } }
      stop: { needs: [look], do: end, with: { status: failed, reason: nothing to do here } }
      more: { needs: [stop], do: test.count, with: { key: more } }
  after: { needs: [group], do: test.count, with: { key: after } }
";
    let applied = rig.apply(source, json!({})).await;
    let in_hook = rig.reached("hook").await.seq;
    rig.restart().await;
    rig.event("test.reached", move |event| event.seq > in_hook && event.data["gate"] == "hook").await;
    rig.open("hook");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.error.as_deref(), Some("nothing to do here"));
    let ran = ["more", "after", "went well"].map(|key| rig.shared.count(key));
    assert_eq!(ran, [0, 0, 0], "nothing after the end was run, in the task or after it");
}
