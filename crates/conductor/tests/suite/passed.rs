//! A step that is passed over (its `when` did not hold) costs nothing on
//! the disk by itself. It is said as it happens to whoever is watching, it
//! is among its run's steps for whoever asks, and it is written down with
//! the next thing its run writes: never with a write of its own, and never
//! later than anything that takes time. Its event is kept in the journal
//! only while the text of some instance could be owed it, whoever is
//! listening at the instant it is said.

use conductor::{RunStatus, StepStatus};
use serde_json::json;

use crate::handlers::{active, handled};
use crate::{Rig, statuses};

const SOME: &str = "\
blueprint: some
steps:
  first: { do: test.count, with: { key: first } }
  over: { needs: [first], when: ${ false }, do: test.count, with: { key: never } }
  again: { needs: [over], when: ${ false }, do: test.count, with: { key: never } }
  door: { needs: [again], do: test.gate, with: { name: door } }
  last: { needs: [door], when: ${ false }, do: test.count, with: { key: never } }
";

/// How many rows of steps are written down, and how many events are kept.
async fn written(rig: &Rig) -> (i64, i64) {
    rig.journal
        .db()
        .call(|connection| {
            let steps = connection.query_row("SELECT count(*) FROM steps", [], |row| row.get(0))?;
            let events = connection.query_row("SELECT count(*) FROM events", [], |row| row.get(0))?;
            Ok((steps, events))
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn a_step_that_is_passed_over_is_said_as_it_happens_and_is_not_kept_as_an_event() {
    let rig = Rig::new().await;
    let mut said = rig.follow("step.skipped");
    let applied = rig.apply(SOME, json!({})).await;
    let mut heard = Vec::new();
    for _ in 0..2 {
        let event = rig.soon("a step to be said to have been passed over", said.next()).await.unwrap();
        assert_eq!(event.seq, 0, "it is passed on, and has no place in the journal");
        assert_eq!(event.data["when"], "false", "it says the `when` that did not hold");
        assert_eq!(event.source.run, applied.run, "and whose step it was");
        heard.push(event.source.step.clone().unwrap());
    }
    assert_eq!(heard, ["over", "again"]);
    rig.reached("door").await;
    rig.open("door");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let last = rig.soon("the last step to be said to have been passed over", said.next()).await.unwrap();
    assert_eq!(last.source.step.as_deref(), Some("last"));

    assert_eq!(rig.shared.count("never"), 0, "none of them was run");
    assert!(rig.topics("step.skipped").await.is_empty(), "nobody listened, so none of it was kept");
    // What is kept of the run is what it did.
    assert_eq!(
        rig.topics("step.*").await,
        ["step.started", "step.finished", "step.started", "step.finished"],
        "the two steps that ran, and nothing of the three that did not"
    );
}

#[tokio::test]
async fn a_step_that_is_passed_over_is_among_its_runs_steps_while_it_goes_and_afterwards() {
    let rig = Rig::new().await;
    let applied = rig.apply(SOME, json!({})).await;
    rig.reached("door").await;
    let id = applied.run.clone().unwrap();
    let order = ["first", "over", "again", "door"];
    let going = rig.engine.run(&id).await.unwrap();
    assert_eq!(
        statuses(&going, &order),
        [("first", "ok"), ("over", "skipped"), ("again", "skipped"), ("door", "running")]
            .map(|(step, status)| (step.to_string(), status)),
        "while it goes, whoever asks is told of the steps it passed over"
    );
    // Written with the step that ran after them, in its write: a row for
    // each step so far, and the run had written three times, not five.
    assert_eq!(written(&rig).await.0, 4);

    rig.open("door");
    let run = rig.finished(&applied).await;
    let order = ["first", "over", "again", "door", "last"];
    assert_eq!(
        statuses(&run, &order),
        [("first", "ok"), ("over", "skipped"), ("again", "skipped"), ("door", "ok"), ("last", "skipped")]
            .map(|(step, status)| (step.to_string(), status)),
        "and afterwards of every one, the last with the run's ending"
    );
    assert_eq!(rig.engine.run(&id).await.unwrap().steps.len(), 5, "as it is written down");
    assert_eq!(written(&rig).await.0, 5);
    let over = crate::step(&run, "over");
    assert_eq!((over.status, over.attempt, over.finished), (StepStatus::Skipped, 0, Some(over.started)));
}

#[tokio::test]
async fn a_run_that_passes_over_every_step_writes_only_that_it_began_and_ended() {
    let rig = Rig::new().await;
    let source = "\
blueprint: none
steps:
  one: { when: ${ false }, do: test.count, with: { key: never } }
  two: { when: ${ false }, do: test.count, with: { key: never } }
  three: { when: ${ false }, do: test.count, with: { key: never } }
";
    let before = written(&rig).await;
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(run.steps.len(), 3, "each is written down as passed over");
    assert!(run.steps.iter().all(|step| step.status == StepStatus::Skipped));
    let after = written(&rig).await;
    assert_eq!(after.0 - before.0, 3, "with the run's ending");
    let kept = rig.events("*").await;
    assert!(
        kept.iter().all(|event| event.source.step.is_none()),
        "and no event of a step is kept: {:?}",
        kept.iter().map(|event| &event.topic).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn after_a_restart_a_run_goes_on_past_the_steps_it_had_passed_over() {
    let mut rig = Rig::new().await;
    // What the second step goes by is so when it is looked at, and not
    // afterwards: looked at again after the restart, it would be run.
    let source = "\
blueprint: once
state: { open: false }
steps:
  over: { when: ${ state.open }, do: test.count, with: { key: never } }
  opened: { needs: [over], do: set, with: { open: true } }
  door: { needs: [opened], do: test.durable, with: { name: door } }
  after: { needs: [door], when: ${ !state.open }, do: test.count, with: { key: never } }
";
    let applied = rig.apply(source, json!({})).await;
    rig.reached("door").await;
    rig.restart().await;
    rig.open("door");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(rig.shared.count("never"), 0, "what it passed over before the restart stayed passed over");
    let order = ["over", "opened", "door", "after"];
    assert_eq!(
        statuses(&run, &order),
        [("over", "skipped"), ("opened", "ok"), ("door", "ok"), ("after", "skipped")]
            .map(|(step, status)| (step.to_string(), status))
    );
}

#[tokio::test]
async fn a_step_that_is_passed_over_is_kept_as_an_event_where_a_handler_listens_for_it() {
    let rig = Rig::new().await;
    let listener = "\
blueprint: listener
state: { seen: [] }
on:
  passed:
    event: step.skipped
    steps:
      note: { do: set, with: { seen: ${ state.seen + [event.source.step] } } }
";
    let listening = rig.apply(listener, json!({})).await;
    active(&rig, &listening.instance).await;
    let source = "\
blueprint: one
steps:
  over: { when: ${ false }, do: test.count, with: { key: never } }
";
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    handled(&rig, "on.passed", 1).await;
    let kept = rig.events("step.skipped").await;
    assert_eq!(kept.len(), 1, "somebody listens, so it has its place in the journal");
    assert!(kept[0].seq > 0);
    assert_eq!(kept[0].source.step.as_deref(), Some("over"));
    assert_eq!(rig.instance(&listening.instance).await.state["seen"], json!(["over"]));
}

#[tokio::test]
async fn a_step_that_is_passed_over_is_kept_as_an_event_where_a_step_waits_for_it() {
    let rig = Rig::new().await;
    let waiter = "\
blueprint: waiter
steps:
  heard: { do: wait, with: { event: step.skipped } }
outputs:
  step: ${ steps.heard.out.event.source.step }
";
    let waiting = rig.apply(waiter, json!({})).await;
    rig.event("step.waiting", |_| true).await;
    let source = "\
blueprint: one
steps:
  over: { when: ${ false }, do: test.count, with: { key: never } }
";
    rig.run(source, json!({})).await;
    let run = rig.finished(&waiting).await;
    assert_eq!(run.outputs, json!({ "step": "over" }), "the wait was given it");
    assert_eq!(rig.topics("step.skipped").await.len(), 1, "from its place in the journal");
}

#[tokio::test]
async fn a_step_its_run_ended_before_is_passed_over_the_same_way() {
    let rig = Rig::new().await;
    let source = "\
blueprint: early
steps:
  first: { do: test.count, with: { key: first } }
  stop: { needs: [first], do: end, with: { reason: enough } }
  never: { needs: [stop], do: test.count, with: { key: never } }
  nor_this: { needs: [never], do: test.count, with: { key: never } }
";
    let mut said = rig.follow("step.skipped");
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(rig.shared.count("never"), 0);
    let order = ["first", "stop", "never", "nor_this"];
    assert_eq!(
        statuses(&run, &order),
        [("first", "ok"), ("stop", "ok"), ("never", "skipped"), ("nor_this", "skipped")]
            .map(|(step, status)| (step.to_string(), status)),
        "each is among the run's steps, written with its ending"
    );
    let mut heard = Vec::new();
    for _ in 0..2 {
        let event = rig.soon("a step to be said to have been passed over", said.next()).await.unwrap();
        assert_eq!(event.seq, 0, "said as it happens, with no place in the journal");
        heard.push(event.source.step.clone().unwrap());
    }
    heard.sort();
    assert_eq!(heard, ["never", "nor_this"]);
    assert!(rig.topics("step.skipped").await.is_empty());
}

#[tokio::test]
async fn a_step_passed_over_beside_one_that_is_at_work_is_written_at_once() {
    let rig = Rig::new().await;
    // The two have nothing to wait for in each other: one is at work for
    // as long as its door is shut, and the other is passed over meanwhile.
    let source = "\
blueprint: beside
steps:
  slow: { needs: [], do: test.gate, with: { name: door } }
  over: { needs: [], when: ${ false }, do: test.count, with: { key: never } }
  after: { needs: [slow, over], when: ${ false }, do: test.count, with: { key: never } }
";
    let mut said = rig.follow("step.skipped");
    let applied = rig.apply(source, json!({})).await;
    rig.reached("door").await;
    rig.soon("the step beside it to be passed over", said.next()).await.unwrap();
    // Whichever of the two came first, both are written by now: passed
    // over first, it went with the other's beginning; second, at once. A
    // restart while the first is at work finds the other passed over.
    assert_eq!(written(&rig).await.0, 2);
    rig.open("door");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let order = ["slow", "over", "after"];
    assert_eq!(
        statuses(&run, &order),
        [("slow", "ok"), ("over", "skipped"), ("after", "skipped")]
            .map(|(step, status)| (step.to_string(), status))
    );
}

#[tokio::test]
async fn a_run_that_ended_early_and_is_stopped_in_its_hooks_does_not_run_what_came_after_its_end() {
    let mut rig = Rig::new().await;
    // That a run ended early is not itself written down: what says so
    // after a restart is the record of each step it passed over, which is
    // there before the run's hooks begin, however long they take.
    let source = "\
blueprint: early
on_finish:
  - { do: test.gate, with: { name: hook } }
steps:
  check: { do: test.count, with: { key: checked } }
  stop: { needs: [check], do: end, with: { reason: enough } }
  ship: { needs: [stop], do: test.count, with: { key: shipped } }
";
    let applied = rig.apply(source, json!({})).await;
    let in_hook = rig.reached("hook").await.seq;
    assert_eq!(written(&rig).await.0, 3, "what it passed over is written before its hooks");

    rig.restart().await;
    rig.event("test.reached", move |event| event.seq > in_hook && event.data["gate"] == "hook").await;
    rig.open("hook");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        (rig.shared.count("checked"), rig.shared.count("shipped")),
        (1, 0),
        "nothing after its end was run"
    );
    let order = ["check", "stop", "ship"];
    assert_eq!(
        statuses(&run, &order),
        [("check", "ok"), ("stop", "ok"), ("ship", "skipped")]
            .map(|(step, status)| (step.to_string(), status))
    );
}

#[tokio::test]
async fn a_last_step_that_was_passed_over_stays_so_though_a_hook_changes_what_its_when_reads() {
    let mut rig = Rig::new().await;
    let source = "\
blueprint: late
state: { done: false }
on_finish:
  - { do: set, with: { done: true } }
  - { do: test.gate, with: { name: hook } }
steps:
  work: { do: test.count, with: { key: worked } }
  late: { needs: [work], when: ${ state.done }, do: test.count, with: { key: late } }
";
    let applied = rig.apply(source, json!({})).await;
    let in_hook = rig.reached("hook").await.seq;
    rig.restart().await;
    rig.event("test.reached", move |event| event.seq > in_hook && event.data["gate"] == "hook").await;
    rig.open("hook");
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(rig.shared.count("late"), 0, "its `when` is not looked at again after what the hook set");
    assert_eq!(crate::step(&run, "late").status, StepStatus::Skipped);
}

/// A run that is at a door when the daemon is stopped, and passes a step
/// over as soon as it is taken up again with the door open.
const SKIPPER: &str = "\
blueprint: skipper
steps:
  door: { do: test.durable, with: { name: door } }
  over: { needs: [door], when: ${ false }, do: test.count, with: { key: never } }
";

/// Applies the skipper, which is the first thing there is and so the
/// first taken up again, and then a dozen instances that hear nothing of
/// this: whatever is applied afterwards is taken up long after the
/// skipper has gone on.
async fn a_skipper_and_those_between(rig: &Rig) {
    rig.apply(SKIPPER, json!({})).await;
    rig.reached("door").await;
    for n in 0..12 {
        let bystander = format!(
            "blueprint: bystander-{n}\non:\n  never:\n    event: test.never\n    steps:\n      \
             nothing: {{ do: test.count, with: {{ key: never }} }}\n"
        );
        let applied = rig.apply(&bystander, json!({})).await;
        active(rig, &applied.instance).await;
    }
}

#[tokio::test]
async fn a_handler_is_owed_a_step_passed_over_while_it_was_not_yet_listening_again_after_a_restart() {
    let mut rig = Rig::new().await;
    a_skipper_and_those_between(&rig).await;
    let listener = "\
blueprint: listener
state: { seen: [] }
on:
  passed:
    event: step.skipped
    steps:
      note: { do: set, with: { seen: ${ state.seen + [event.source.step] } } }
";
    let listening = rig.apply(listener, json!({})).await;
    active(&rig, &listening.instance).await;

    // The door is open as the daemon comes back: the skipper passes its
    // step over at once, before the listener's handler is listening again.
    rig.open("door");
    rig.restart().await;
    handled(&rig, "on.passed", 1).await;
    assert_eq!(rig.instance(&listening.instance).await.state["seen"], json!(["over"]));
    assert_eq!(
        rig.topics("step.skipped").await.len(),
        1,
        "it was kept for the handler to read from its place"
    );
}

#[tokio::test]
async fn a_wait_is_owed_a_step_passed_over_before_it_was_waiting_again_after_a_restart() {
    let mut rig = Rig::new().await;
    a_skipper_and_those_between(&rig).await;
    let waiter = "\
blueprint: waiter
steps:
  heard: { do: wait, with: { event: step.skipped } }
outputs:
  step: ${ steps.heard.out.event.source.step }
";
    let waiting = rig.apply(waiter, json!({})).await;
    rig.event("step.waiting", |_| true).await;

    rig.open("door");
    rig.restart().await;
    let run = rig.finished(&waiting).await;
    assert_eq!(run.outputs, json!({ "step": "over" }), "the wait was given it from its place in the journal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn steps_passed_over_beside_steps_at_work_are_all_written_however_they_fall_together() {
    // Whether a passed-over step may be held, and holding it, are one
    // thing: on threads of their own, half a dozen of each at once.
    let rig = Rig::new().await;
    let mut source = String::from("blueprint: together\nsteps:\n");
    for n in 0..6 {
        source += &format!("  slow_{n}: {{ needs: [], do: test.gate, with: {{ name: door_{n} }} }}\n");
        source += &format!(
            "  over_{n}: {{ needs: [], when: ${{ false }}, do: test.count, with: {{ key: never }} }}\n"
        );
    }
    let applied = rig.apply(&source, json!({})).await;
    for n in 0..6 {
        let door = format!("door_{n}");
        rig.event("test.reached", move |event| event.data["gate"] == door.as_str()).await;
    }
    let id = applied.run.clone().unwrap();
    rig.soon("every step passed over to be among the run's", async {
        while rig.engine.run(&id).await.unwrap().steps.len() < 12 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(written(&rig).await.0, 12, "each beside one at work, and none left held");
    for n in 0..6 {
        rig.open(&format!("door_{n}"));
    }
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
}
