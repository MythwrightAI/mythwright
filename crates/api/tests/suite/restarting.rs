//! The factory started again by a step of its own (`factory.restart`), with
//! the test standing in for what keeps the daemon running: the node is
//! stopped and another started on its data when it asks, as `up` does when
//! the daemon ends saying it is to come back. The run that asked is not
//! lost by it: the next node takes it up, and the step answers that the
//! factory was started again, once, wherever a stop fell.

use api::folder::{Keep, Keeping};
use kernel::Event;
use serde_json::{Value, json};

use crate::{Rig, soon};

/// A run that does something, has the factory started again, and does
/// something after.
const TAKEN_UP: &str = "\
blueprint: taken-up
permissions: [admin]
steps:
  before:
    do: log
    with: { message: before }
  restarted:
    needs: [before]
    do: factory.restart
  after:
    needs: [restarted]
    do: log
    with: { message: after }
";

/// The test, as what keeps the factory's daemon running.
fn keeper(rig: &Rig) -> Keeping {
    match api::folder::keep(&rig.config.data_dir).unwrap() {
        Keep::Kept(keeping) => keeping,
        _ => panic!("something else keeps this factory"),
    }
}

/// Applies `source` and answers its run, not waiting for it.
async fn applied(rig: &Rig, source: &str) -> String {
    let went = rig.send("/api/apply", json!({ "source": source })).await;
    went["run"].as_str().unwrap_or_else(|| panic!("{went}")).to_string()
}

/// Lets the run go on to its end, starting the daemon again each time a
/// step asks, as a keeper would: the run as it ended, and how often it was
/// asked.
async fn kept_until_over(rig: &mut Rig, run: &str) -> (conductor::RunInfo, usize) {
    let mut asked = 0;
    loop {
        let engine = rig.node().app.engine.clone();
        let over = tokio::select! {
            over = engine.wait_run(run) => Some(over.unwrap()),
            () = rig.node().asked_again() => None,
        };
        match over {
            Some(over) => return (over, asked),
            None => {
                asked += 1;
                rig.restart().await;
            }
        }
    }
}

fn output<'a>(run: &'a conductor::RunInfo, step: &str) -> &'a Value {
    &run.steps.iter().find(|one| one.step == step).unwrap_or_else(|| panic!("no step {step}: {run:?}")).output
}

#[tokio::test]
async fn a_step_has_the_factory_started_again_and_its_run_goes_on_in_the_one_that_comes_back() {
    let mut rig = Rig::new().await;
    let _keeping = keeper(&rig);
    let run = applied(&rig, TAKEN_UP).await;
    let (over, asked) = soon("the run to end", kept_until_over(&mut rig, &run)).await;
    assert_eq!(over.status, conductor::RunStatus::Ok, "{over:?}");
    assert_eq!(asked, 1);
    let came = output(&over, "restarted");
    assert_eq!(came["came"], "restarted", "{came}");
    assert_eq!(came["version"], env!("CARGO_PKG_VERSION"), "{came}");
    // Said once, by the daemon that was asked, and the next one started.
    assert_eq!(rig.count_of("system.restarting").await, 1);
    assert!(rig.count_of("system.started").await >= 2);
}

#[tokio::test]
async fn a_factory_nothing_keeps_running_is_not_stopped_by_a_step() {
    let rig = Rig::new().await;
    let went = rig.send("/api/apply", json!({ "source": TAKEN_UP, "wait": true })).await;
    let step =
        went["finished"]["steps"].as_array().unwrap().iter().find(|one| one["step"] == "restarted").cloned();
    let step = step.unwrap_or_else(|| panic!("{went}"));
    assert_eq!(step["status"], "failed", "{went}");
    assert!(step["error"].as_str().unwrap().contains("nothing keeps this factory running"), "{step}");
    assert_eq!(rig.count_of("system.restarting").await, 0);
}

#[tokio::test]
async fn at_rest_it_waits_until_nothing_else_is_under_way() {
    let mut rig = Rig::new().await;
    let _keeping = keeper(&rig);
    // Something else under way, waiting to be told to go on.
    let waiting = "\
blueprint: still-going
steps:
  told:
    do: wait
    with: { event: errand.go }
";
    let other = applied(&rig, waiting).await;
    rig.event("step.waiting", |event| event.source.step.as_deref() == Some("told")).await;
    let run = applied(&rig, TAKEN_UP).await;
    // It says what it waits for, and asks nothing yet.
    let (logs, id) = (rig.node().app.engine.logs().clone(), format!("{run}.restarted"));
    soon("the step to say what it waits for", async {
        while !logs.read("runs", &id, 0, 1000).iter().any(|line| line.text.contains("a run of still-going")) {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(rig.count_of("system.restarting").await, 0);
    let go = "blueprint: go-on\npermissions: [emit:errand.go]\nsteps:\n  go:\n    do: emit\n    with: { topic: errand.go }\n";
    rig.send("/api/apply", json!({ "source": go, "wait": true })).await;
    let (over, asked) = soon("the run to end", kept_until_over(&mut rig, &run)).await;
    assert_eq!((over.status, asked), (conductor::RunStatus::Ok, 1), "{over:?}");
    let other = rig.node().app.engine.run(&other).await.unwrap();
    assert_eq!(other.status, conductor::RunStatus::Ok, "{other:?}");
}

/// Where a play of [`TAKEN_UP`] may be stopped (each step beginning and
/// ending, and the asking), and how often the step asks to be started
/// again after: once where it had not begun, and never where it had, since
/// the start that follows the stop is one after it began. As the step
/// before it ends, whether the next had begun is the machine's to say: the
/// next is written as begun in the moment after, which a stop may or may
/// not keep.
fn places() -> Vec<(&'static str, Option<&'static str>, &'static [usize])> {
    vec![
        ("step.started", Some("before"), &[1]),
        ("step.finished", Some("before"), &[0, 1]),
        ("step.started", Some("restarted"), &[0]),
        ("system.restarting", None, &[0]),
    ]
}

#[tokio::test]
async fn stopped_anywhere_from_before_the_step_to_its_asking_it_comes_to_one_start_after_it_and_its_run_goes_on()
 {
    for (topic, step, again) in places() {
        let mut rig = Rig::new().await;
        let _keeping = keeper(&rig);
        let journal = rig.node().app.journal.clone();
        let at = move |event: &Event| {
            event.topic == topic && step.is_none_or(|step| event.source.step.as_deref() == Some(step))
        };
        journal.shut_after(at);
        let run = applied(&rig, TAKEN_UP).await;
        // The daemon goes at the place, whatever else came of it.
        tokio::select! {
            () = journal.shut() => {}
            () = rig.node().asked_again() => {}
        }
        assert!(
            journal.is_shut() || topic == "system.restarting",
            "{topic}: the play never came to its place"
        );
        rig.restart().await;
        let (over, asked) = soon("the run to end", kept_until_over(&mut rig, &run)).await;
        let place = format!("{topic} {}", step.unwrap_or_default());
        assert!(again.contains(&asked), "{place}: asked {asked} times");
        assert_eq!(over.status, conductor::RunStatus::Ok, "{place}: {over:?}");
        assert_eq!(output(&over, "restarted")["came"], "restarted", "{place}");
        // The step that followed was done once.
        let after = rig.node().app.engine.logs().read("runs", &format!("{run}.after"), 0, 1000);
        assert_eq!(after.iter().filter(|line| line.text.contains("after")).count(), 1, "{place}: {after:?}");
        // Asked at most once, however the stop fell.
        assert!(rig.count_of("system.restarting").await <= 1, "{place}");
    }
}
