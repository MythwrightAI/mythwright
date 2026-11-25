use std::time::Duration;

use blueprint::ActionSpec;
use conductor::{ActionError, ActionResult, Engine, RunStatus, StepStatus};
use kernel::Source;
use kernel::{BoxFut, Clock};
use serde_json::json;

use crate::{Rig, registry, statuses, step};

#[tokio::test]
async fn a_step_that_overruns_its_timeout_fails_and_runs_the_timeout_hooks() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: slow
steps:
  build:
    do: test.sleep
    with: { for: 10m }
    timeout: 1m
    on_timeout:
      - emit: { topic: build.gave_up, why: ${ self.error } }
    on_fail:
      - emit: { topic: build.failed }
",
            json!({}),
        )
        .await;
    // Two sleeps: the action's own, and the timeout watching it.
    rig.advance_after(since, 2, "1m").await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(step(&run, "build").error.as_deref(), Some("timed out after 1m"));
    assert_eq!(rig.event("build.gave_up", |_| true).await.data["why"], "timed out after 1m");
    assert!(rig.events("build.failed").await.is_empty(), "`on_timeout` replaces `on_fail` for a timeout");
    assert_eq!(rig.event("step.failed", |_| true).await.data["timed_out"], true);
}

/// A rig whose actions include one that moves the clock on as it begins,
/// before it first waits, and then waits until its step is cut: an
/// action that has the clock move before anything about it is first
/// looked at, as a program at work does on a machine that is busy.
async fn moving() -> Rig {
    let mut rig = Rig::new().await;
    let mut actions = registry(&rig.shared);
    let clock = rig.clock.clone();
    actions.add_fn(
        ActionSpec::new("test.moves", "Move the clock on 45s as it begins, and wait to be cut."),
        move |cx, _| -> BoxFut<'static, ActionResult> {
            clock.advance(Duration::from_secs(45));
            Box::pin(async move {
                cx.cancel().cancelled().await;
                Err(ActionError::cancelled())
            })
        },
    );
    rig.engine.halt();
    rig.engine =
        Engine::new(rig.journal.clone(), rig.logs.clone(), actions, rig.library.clone()).await.unwrap();
    rig
}

#[tokio::test]
async fn a_steps_timeout_is_timed_from_before_its_action_begins_whenever_it_is_first_looked_at() {
    let rig = moving().await;
    let before = rig.clock.now_ms();
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply("blueprint: limited\nsteps:\n  build:\n    do: test.moves\n    timeout: 1m\n", json!({}))
        .await;
    // The action had the clock move 45s as it began; 20s more is past the
    // minute from before it began, which is the step's to be timed by.
    rig.advance_after(since, 1, "20s").await;
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "build").error.as_deref(), Some("timed out after 1m"));
    assert_eq!(rig.clock.now_ms() - before, 65_000);
}

#[tokio::test]
async fn a_watch_rule_fires_while_the_step_is_still_running() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: kv
steps:
  build:
    do: test.gate
    with: { name: build }
    watch:
      - after: 5m
        emit: { topic: build.slow, msg: build slow, code: 5, elapsed: ${ self.duration_ms } }
",
            json!({}),
        )
        .await;
    rig.advance_after(since, 1, "5m").await;
    let slow = rig.event("build.slow", |_| true).await;
    assert_eq!(slow.data, json!({ "msg": "build slow", "code": 5, "elapsed": 300_000 }));
    assert_eq!(slow.source.step.as_deref(), Some("build"));
    assert_eq!(rig.engine.run(applied.run.as_ref().unwrap()).await.unwrap().status, RunStatus::Running);

    rig.open("build");
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
    assert_eq!(rig.events("build.slow").await.len(), 1, "`after` fires once");
}

#[tokio::test]
async fn a_watch_rules_time_may_be_worked_out_as_the_step_begins_and_one_that_comes_to_nothing_is_never_timed()
 {
    let rig = Rig::new().await;
    let text = |after: &str| {
        format!(
            "blueprint: told\nparams:\n  after: {{ type: string, default: \"\" }}\nsteps:\n  build:\n    do: test.gate\n    with: {{ name: {after}-build }}\n    watch:\n      - after: \"${{params.after}}\"\n        emit: {{ topic: build.slow, after: \"${{params.after}}\" }}\n"
        )
    };
    // As its param says: fired once, then.
    let since = rig.clock.sleep_count();
    let applied = rig.apply(&text("timed"), json!({ "after": "20m" })).await;
    rig.advance_after(since, 1, "20m").await;
    assert_eq!(rig.event("build.slow", |_| true).await.data, json!({ "after": "20m" }));
    rig.open("timed-build");
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);

    // Said as nothing, it is not timed: nothing waits, and nothing fires
    // however long the step runs.
    let since = rig.clock.sleep_count();
    let applied = rig.apply(&text("untimed"), json!({ "after": "" })).await;
    rig.advance_after(since, 0, "1d").await;
    rig.open("untimed-build");
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
    assert_eq!(rig.events("build.slow").await.len(), 1);

    // One that is no time fails its step before its action begins.
    let run = rig.run(&text("wrong"), json!({ "after": "soon" })).await;
    assert_eq!(run.status, RunStatus::Failed);
    let why = step(&run, "build").error.clone().unwrap_or_default();
    assert!(why.contains("a watch rule's time, `${params.after}`: `soon` is not a duration"), "{why}");
}

#[tokio::test]
async fn an_every_rule_keeps_firing() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "blueprint: beat\nsteps:\n  work:\n    do: test.gate\n    with: { name: work }\n    watch:\n      - every: 1m\n        do: test.count\n        with: { key: beats }\n",
            json!({}),
        )
        .await;
    for beat in 1..=3 {
        rig.advance_after(since, beat, "1m").await;
    }
    // The fourth sleep only starts once the third beat has been handled.
    rig.soon("the third beat", rig.clock.sleeps_started(since + 4)).await;
    assert_eq!(rig.shared.count("beats"), 3);
    rig.open("work");
    rig.finished(&applied).await;
}

#[tokio::test]
async fn a_log_rule_fires_once_however_many_lines_match() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: logs
steps:
  build:
    do: test.gate
    with:
      name: build
      lines: [\"warning: unused import\", \"error[E0308]: mismatched types\", \"error: aborting\"]
    watch:
      - log: \"^error\"
        emit: { topic: build.error }
",
            json!({}),
        )
        .await;
    rig.event("build.error", |_| true).await;
    rig.open("build");
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
    assert_eq!(rig.events("build.error").await.len(), 1);
}

#[tokio::test]
async fn a_watch_rule_can_stop_the_step() {
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: strict
steps:
  build:
    do: test.gate
    with: { name: never, lines: [\"error: could not compile\"] }
    watch:
      - log: \"^error\"
        fail: true
",
            json!({}),
        )
        .await;
    assert_eq!(
        step(&run, "build").error.as_deref(),
        Some("stopped by a watch rule: its output matched `^error`")
    );
}

#[tokio::test]
async fn a_long_wait_survives_a_restart_and_still_ends_on_time() {
    let mut rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: patient
steps:
  pause: { do: wait, with: { for: 300m } }
  after: { do: test.echo, with: { waited: ${ steps.pause.out.waited_ms } } }
",
            json!({}),
        )
        .await;
    rig.advance_after(since, 1, "100m").await;

    rig.restart().await;
    let since = rig.clock.sleep_count();
    rig.advance_after(since, 0, "199m").await;
    rig.soon("the wait to resume", rig.clock.sleeps_started(since + 1)).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    let run = rig.engine.run(applied.run.as_ref().unwrap()).await.unwrap();
    assert_eq!(run.status, RunStatus::Running, "299 of 300 minutes have passed");

    rig.clock.advance(Duration::from_secs(60));
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(step(&run, "after").output, json!({ "waited": 18_000_000 }));
    assert_eq!(step(&run, "pause").attempt, 1, "resumed, not retried");
}

const ASK: &str = "\
blueprint: ask
steps:
  check: { do: test.echo, with: { ok: false } }
  feedback:
    when: ${ !steps.check.out.ok }
    do: wait
    with: { event: feedback.given, where: event.data.about == instance.id, timeout: 300m }
  give_up:
    when: ${ steps.feedback.out.timed_out }
    do: end
    with: { reason: nobody answered }
  act:
    do: test.echo
    with: { answer: ${ steps.feedback.out.event.data.answer } }
";

#[tokio::test]
async fn waiting_for_an_event_continues_when_it_arrives() {
    let rig = Rig::new().await;
    let applied = rig.apply(ASK, json!({})).await;
    let waiting = rig.event("step.waiting", |_| true).await;
    assert_eq!(waiting.data["event"], "feedback.given");

    rig.journal
        .emit("feedback.given", Source::default(), json!({ "about": "someone else", "answer": "no" }))
        .await
        .unwrap();
    rig.journal
        .emit("feedback.given", Source::default(), json!({ "about": applied.instance, "answer": "go" }))
        .await
        .unwrap();
    let run = rig.finished(&applied).await;

    assert_eq!(step(&run, "feedback").output["timed_out"], false);
    assert_eq!(step(&run, "give_up").status, StepStatus::Skipped);
    assert_eq!(step(&run, "act").output, json!({ "answer": "go" }));
}

#[tokio::test]
async fn waiting_for_an_event_gives_up_at_its_timeout() {
    let rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig.apply(ASK, json!({})).await;
    rig.advance_after(since, 1, "300m").await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok, "ending early is not a failure");
    assert_eq!(
        statuses(&run, &["feedback", "give_up", "act"]),
        [("feedback".into(), "ok"), ("give_up".into(), "ok"), ("act".into(), "skipped")]
    );
    assert_eq!(step(&run, "feedback").output, json!({ "timed_out": true, "event": null }));
}

#[tokio::test]
async fn an_event_that_arrives_while_the_conductor_is_down_still_ends_the_wait() {
    let mut rig = Rig::new().await;
    let applied = rig.apply(ASK, json!({})).await;
    rig.event("step.waiting", |_| true).await;

    rig.engine.halt();
    rig.journal
        .emit("feedback.given", Source::default(), json!({ "about": applied.instance, "answer": "late" }))
        .await
        .unwrap();
    rig.restart().await;

    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "act").output, json!({ "answer": "late" }));
}

/// Answers whatever asks it, to the instance that asked.
const DESK: &str = "\
blueprint: desk
on:
  asked:
    event: desk.question
    do: emit
    with: { topic: desk.answer, to: \"${event.source.instance}\", twice: ${ event.data.n * 2 } }
";

/// Asks, and waits for its answer, in one step.
const ASKS: &str = "\
blueprint: asks
params: { n: { type: int, default: 21 } }
steps:
  answer:
    do: wait
    with:
      event: desk.answer
      where: event.data.to == instance.id
      say: { topic: desk.question, n: ${params.n} }
outputs:
  twice: ${ steps.answer.out.event.data.twice }
";

#[tokio::test]
async fn a_wait_that_says_what_it_asks_hears_the_answer_however_soon_it_comes() {
    let rig = Rig::new().await;
    let desk = rig.apply(DESK, json!({})).await;
    crate::handlers::active(&rig, &desk.instance).await;

    // The desk answers at once. Said by a step before the wait, the
    // question could be answered before the wait had begun to listen.
    let applied = rig.apply(ASKS, json!({})).await;
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(run.outputs, json!({ "twice": 42 }));
    // It was said once, by the step that waits, as the instance it is.
    let asked = rig.events("desk.question").await;
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].data, json!({ "n": 21 }));
    assert_eq!(asked[0].source.instance.as_deref(), Some(applied.instance.as_str()));
    assert_eq!(asked[0].source.step.as_deref(), Some("answer"));
    // And after the wait had said it was waiting.
    let waiting = rig.event("step.waiting", |_| true).await;
    assert!(waiting.seq < asked[0].seq, "{} then {}", waiting.seq, asked[0].seq);
}

#[tokio::test]
async fn a_wait_taken_up_after_a_restart_does_not_ask_again_and_hears_what_was_answered_meanwhile() {
    let mut rig = Rig::new().await;
    // Nobody is there to answer yet.
    let applied = rig.apply(ASKS, json!({ "n": 4 })).await;
    rig.event("desk.question", |_| true).await;

    rig.restart().await;
    rig.soon("the wait to be taken up", async {
        while rig.events("step.waiting").await.len() < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let answer = json!({ "to": applied.instance, "twice": 8 });
    rig.journal.emit("desk.answer", Source::default(), answer).await.unwrap();

    let run = rig.finished(&applied).await;
    assert_eq!(run.outputs, json!({ "twice": 8 }));
    assert_eq!(rig.events("desk.question").await.len(), 1, "asked once, though the wait began twice");
}

#[tokio::test]
async fn what_a_wait_says_is_held_to_what_an_emit_may_say() {
    let rig = Rig::new().await;
    let asks = |say: &str| {
        format!(
            "blueprint: wrong\nsteps:\n  answer:\n    do: wait\n    with: {{ event: desk.answer, say: {say} }}\n"
        )
    };
    for (say, why) in [
        // The system's own are its own to say, here as anywhere.
        ("{ topic: instance.failed }", "emitted by the system"),
        ("{ topic: Not A Topic }", "is not a topic"),
        ("{ n: 2 }", "`say` needs a `topic`"),
    ] {
        let refused = rig.engine.apply(rig.request(&asks(say), json!({}))).await.unwrap_err();
        assert!(refused.message.contains(why), "{say}: {}", refused.message);
    }
    assert!(rig.events("instance.failed").await.is_empty());
}
