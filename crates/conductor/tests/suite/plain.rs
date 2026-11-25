//! An instance that keeps nothing as it goes (`durable: false`): it does
//! what its text says and is seen to, but nothing is written down for a
//! step as it is done, its state is its own while it lives, and it is not
//! picked up after a restart. What each step did is written once, when the
//! run ends. For work that is short and can simply be done again.

use conductor::{InstanceStatus, RunStatus, StepStatus};
use kernel::Source;
use serde_json::json;

use crate::Rig;
use crate::handlers::{active, handled};

/// How many rows the conductor keeps of the steps of every run.
async fn rows(rig: &Rig) -> (i64, i64) {
    rig.journal
        .db()
        .call(|connection| {
            let steps = connection.query_row("SELECT count(*) FROM steps", [], |row| row.get(0))?;
            let elements =
                connection.query_row("SELECT count(*) FROM step_elements", [], |row| row.get(0))?;
            Ok((steps, elements))
        })
        .await
        .unwrap()
}

const WORK: &str = "\
steps:
  count: { do: test.count, with: { key: counted } }
  each:
    for_each: ${ params.names }
    do: test.echo
    with: { name: ${ item } }
  group:
    task:
      one: { do: set, with: { done: ${ state.done + 1 } } }
      two: { do: test.flaky, with: { times: 0 } }
outputs:
  counted: ${ steps.count.out.count }
  names: ${ len(steps.each.out) }
  done: ${ state.done }
";

fn text(name: &str, word: &str) -> String {
    format!(
        "blueprint: {name}\n{word}params:\n  names: {{ type: list, default: [a, b, c] }}\nstate: {{ done: 0 }}\n{WORK}"
    )
}

#[tokio::test]
async fn an_instance_that_keeps_nothing_does_its_work_and_says_afterwards_what_each_step_did() {
    let rig = Rig::new().await;
    let run = rig.run(&text("plain", "durable: false\n"), json!({})).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(run.outputs, json!({ "counted": 1, "names": 3, "done": 1 }), "it gave what it gives");

    // Its run is written down, and how it went, and with that what each
    // of its steps did: as much as is said of the steps of any run.
    let did = |run: &conductor::RunInfo| -> Vec<(String, StepStatus, serde_json::Value)> {
        run.steps.iter().map(|step| (step.step.clone(), step.status, step.output.clone())).collect()
    };
    let steps = did(&run);
    assert_eq!(steps.len(), 5, "{steps:?}");
    assert!(steps.iter().all(|(_, status, _)| *status == StepStatus::Ok), "{steps:?}");
    let count = run.steps.iter().find(|step| step.step == "count").unwrap();
    assert_eq!(count.output, json!({ "count": 1 }));
    assert!(count.finished.is_some_and(|finished| finished >= count.started), "{count:?}");
    // Not how far through a loop it was, nor what an action saved to go on
    // from, nor what it said of each step as it went: those are for going
    // on with, and it does not go on.
    assert_eq!(rows(&rig).await, (5, 0));
    assert!(run.steps.iter().all(|step| step.saved.is_null()), "{:?}", run.steps);
    assert!(rig.events("step.*").await.is_empty(), "what a step said was passed on and not kept");
    let said = rig.topics("run.*,instance.*").await;
    assert_eq!(said, ["instance.started", "run.started", "run.finished", "instance.finished"]);

    // One that says nothing of it is written down as it goes, as it always
    // was, and says the same of its steps.
    let kept = rig.run(&text("kept", ""), json!({})).await;
    assert_eq!(
        kept.outputs,
        json!({ "counted": 2, "names": 3, "done": 1 }),
        "the same work, the same answers"
    );
    let names = |run: &conductor::RunInfo| -> Vec<String> {
        let mut names: Vec<String> = run.steps.iter().map(|step| step.step.clone()).collect();
        names.sort();
        names
    };
    assert_eq!(names(&kept), names(&run), "the same steps are said of both");
    assert!(!rig.events("step.*").await.is_empty());
}

#[tokio::test]
async fn the_steps_of_a_run_that_keeps_nothing_are_seen_while_it_goes_and_written_once_when_it_ends() {
    let rig = Rig::new().await;
    let source = "\
blueprint: slow
durable: false
steps:
  first: { do: test.count, with: { key: slow } }
  wait: { needs: [first], do: test.gate, with: { name: door } }
  last: { needs: [wait], do: fail, with: { message: no further } }
";
    let applied = rig.apply(source, json!({})).await;
    rig.reached("door").await;

    // In the middle of it: whoever asks is told how far it has got, and
    // nothing has been written for any step.
    let going = rig.engine.run(applied.run.as_ref().unwrap()).await.unwrap();
    let how: Vec<(&str, StepStatus)> =
        going.steps.iter().map(|step| (step.step.as_str(), step.status)).collect();
    assert_eq!(how, [("first", StepStatus::Ok), ("wait", StepStatus::Running)]);
    assert_eq!(rows(&rig).await, (0, 0));

    rig.open("door");
    let run = rig.soon("the run", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(run.status, RunStatus::Failed);
    // Ended, and it is all there: the one that failed with why. (Steps
    // that began in the same instant are told in the order they began,
    // and not in the order of their names: `ordered.rs`.)
    let how: Vec<(&str, StepStatus)> =
        run.steps.iter().map(|step| (step.step.as_str(), step.status)).collect();
    assert_eq!(how, [("first", StepStatus::Ok), ("wait", StepStatus::Ok), ("last", StepStatus::Failed)]);
    assert_eq!(run.steps[2].error.as_deref(), Some("no further"));
    assert_eq!(rows(&rig).await, (3, 0));
}

#[tokio::test]
async fn a_handler_of_an_instance_that_keeps_nothing_runs_for_each_event_with_its_state_in_hand() {
    let rig = Rig::new().await;
    let source = "\
blueprint: tally
durable: false
state: { seen: 0 }
on:
  hit:
    event: tally.hit
    steps:
      add: { do: set, with: { seen: ${ state.seen + 1 } } }
      say: { do: emit, with: { topic: tally.now, seen: ${ state.seen } } }
";
    let applied = rig.apply(source, json!({})).await;
    active(&rig, &applied.instance).await;
    for _ in 0..3 {
        rig.journal.emit("tally.hit", Source::default(), json!({})).await.unwrap();
    }
    handled(&rig, "on.hit", 3).await;
    // Its state is its own, from one run to the next, while it lives.
    let seen: Vec<i64> =
        rig.events("tally.now").await.iter().map(|event| event.data["seen"].as_i64().unwrap()).collect();
    assert_eq!(seen, [1, 2, 3]);
    assert_eq!(rig.instance(&applied.instance).await.state["seen"], 3, "and is shown as it is");
    // Each of its three runs wrote down its two steps as it ended.
    assert_eq!(rows(&rig).await, (6, 0));
    // What it says itself is an event like any other; what is said of its
    // state is not kept.
    assert!(rig.events("instance.state").await.is_empty());
}

#[tokio::test]
async fn after_a_restart_an_instance_that_keeps_nothing_is_over_and_says_so_and_what_is_kept_goes_on() {
    let mut rig = Rig::new().await;
    let listens = |name: &str, word: &str| {
        format!(
            "blueprint: {name}\n{word}on:\n  poke: {{ event: {name}.poke, do: test.count, with: {{ key: {name} }} }}\n"
        )
    };
    let plain = rig.apply(&listens("plain", "durable: false\n"), json!({})).await;
    let kept = rig.apply(&listens("kept", ""), json!({})).await;
    active(&rig, &plain.instance).await;
    active(&rig, &kept.instance).await;
    // And one caught in the middle of its steps.
    let busy = rig
        .apply(
            "blueprint: busy\ndurable: false\nsteps:\n  wait: { do: test.gate, with: { name: door } }\n",
            json!({}),
        )
        .await;
    rig.reached("door").await;

    rig.restart().await;

    for id in [&plain.instance, &busy.instance] {
        let is = rig.instance(id).await;
        assert_eq!(is.status, InstanceStatus::Stopped, "{}", is.blueprint);
        let why = is.error.unwrap_or_default();
        assert!(why.contains("keeps nothing across a restart"), "{why}");
    }
    let run = rig.soon("its run to be ended", rig.engine.wait_run(busy.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(run.status, RunStatus::Cancelled);
    let ended = rig.events("instance.finished").await;
    assert_eq!(ended.iter().filter(|event| event.data["status"] == "stopped").count(), 2);

    // What is kept is there, and hears what happens next; what is not,
    // does not.
    assert_eq!(rig.instance(&kept.instance).await.status, InstanceStatus::Active);
    rig.journal.emit("kept.poke", Source::default(), json!({})).await.unwrap();
    rig.journal.emit("plain.poke", Source::default(), json!({})).await.unwrap();
    handled(&rig, "on.poke", 1).await;
    assert_eq!((rig.shared.count("kept"), rig.shared.count("plain")), (1, 0));
}

#[tokio::test]
async fn after_a_restart_everything_that_had_called_something_that_keeps_nothing_calls_it_again() {
    let mut rig = Rig::new().await;
    rig.library.put(
        "busy",
        "blueprint: busy\ndurable: false\nparams:\n  door:\nsteps:\n  wait: { do: test.gate, with: { name: \"${params.door}\" } }\n",
    );
    // Several that are kept, each in the middle of calling one that keeps
    // nothing. One alone is not enough to see it: each looks, as it goes
    // on, at the run of what it called, and they do not all look at once.
    let mut callers = Vec::new();
    for n in 0..5 {
        let caller = format!(
            "blueprint: caller-{n}\nsteps:\n  work: {{ call: busy, as: child, with: {{ door: door-{n} }} }}\n"
        );
        callers.push(rig.apply(&caller, json!({})).await);
        rig.reached(&format!("door-{n}")).await;
    }
    let before = rig.mark();

    rig.restart().await;

    // What each had called is over, and its run with it. So each calls
    // again, and none is left waiting on a run that nothing will ever end.
    for n in 0..5 {
        let door = format!("door-{n}");
        rig.event("test.reached", move |event| event.seq > before && event.data["gate"] == door.as_str())
            .await;
        rig.open(&format!("door-{n}"));
    }
    for caller in &callers {
        let run =
            rig.soon("a caller to finish", rig.engine.wait_run(caller.run.as_ref().unwrap())).await.unwrap();
        assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    }
}
