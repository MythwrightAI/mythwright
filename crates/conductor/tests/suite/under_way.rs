//! Asking an instance for the text and params it is already in the middle
//! of. Nothing is started over, and whoever asked is given the run that is
//! going, to wait on: a person who applies twice, a child spawned or asked
//! for twice at once, and every `call` that was under way when the
//! conductor stopped.

use conductor::{Apply, InstanceFilter, InstanceStatus, RunInfo, RunStatus};
use kernel::Source;
use serde_json::json;

use crate::Rig;
use crate::handlers::handled;

/// Steps that can be caught in the middle: they count, wait at a gate the
/// test opens, and count again.
const SLOW: &str = "\
blueprint: slow
params: { to: 1 }
steps:
  begin: { do: test.count, with: { key: begun } }
  hold: { do: test.gate, with: { name: hold } }
  end: { do: test.count, with: { key: ended } }
outputs:
  begun: ${ steps.begin.out.count }
";

/// The same, with a wait that is picked up after a restart where it was.
const STAGED: &str = "\
blueprint: staged
steps:
  begin: { do: test.count, with: { key: begun } }
  build: { do: test.durable, with: { name: build } }
  end: { do: test.count, with: { key: ended } }
outputs:
  begun: ${ steps.begin.out.count }
";

async fn mains(rig: &Rig, instance: &str) -> Vec<RunInfo> {
    let runs = rig.engine.runs(instance, 50).await.unwrap();
    runs.into_iter().filter(|run| run.flow == "main").collect()
}

async fn keyed(rig: &Rig, key: &str) -> Vec<conductor::InstanceInfo> {
    let all = rig.engine.instances(InstanceFilter::default()).await.unwrap();
    all.into_iter().filter(|one| one.key == key).collect()
}

/// Waits until the gate has been reached `times` times.
async fn reached(rig: &Rig, gate: &str, times: usize) {
    let mut follow = rig.follow("test.reached");
    let mut seen = 0;
    rig.soon(&format!("`{gate}` to be reached {times} time(s)"), async {
        while seen < times {
            if follow.next().await.unwrap().data["gate"] == gate {
                seen += 1;
            }
        }
    })
    .await;
}

#[tokio::test]
async fn applying_what_an_instance_is_in_the_middle_of_starts_nothing_over() {
    let rig = Rig::new().await;
    let first = rig.apply(SLOW, json!({})).await;
    reached(&rig, "hold", 1).await;

    let same = rig.apply(SLOW, json!({})).await;
    assert_eq!(same.instance, first.instance);
    assert_eq!((same.created, same.changed, same.took), (false, false, None));
    assert_eq!(same.run, first.run, "whoever asked is given the run that is going");

    rig.open("hold");
    let run = rig.finished(&same).await;
    assert_eq!((run.status, run.outputs), (RunStatus::Ok, json!({ "begun": 1 })));
    assert_eq!((rig.shared.count("begun"), rig.shared.count("ended")), (1, 1));
    assert_eq!(mains(&rig, &first.instance).await.len(), 1, "its steps ran once");
    assert!(rig.topics("step.cancelled").await.is_empty(), "and nothing of them was cancelled");
    assert!(rig.topics("instance.updated").await.is_empty(), "nor was it said to have been updated");
}

#[tokio::test]
async fn other_params_and_force_still_start_it_over_while_its_steps_run() {
    let rig = Rig::new().await;
    let first = rig.apply(SLOW, json!({})).await;
    reached(&rig, "hold", 1).await;

    let other = rig.apply(SLOW, json!({ "to": 2 })).await;
    assert!(other.changed && other.run != first.run, "other params are something else to do");
    reached(&rig, "hold", 2).await;

    let forced =
        rig.engine.apply(Apply { force: true, ..rig.request(SLOW, json!({ "to": 2 })) }).await.unwrap();
    assert!(forced.changed && forced.run != other.run, "and force is asking for the steps again");
    reached(&rig, "hold", 3).await;

    rig.open("hold");
    assert_eq!(rig.finished(&forced).await.status, RunStatus::Ok);
    let statuses: Vec<RunStatus> = mains(&rig, &first.instance).await.iter().map(|run| run.status).collect();
    assert_eq!(statuses, [RunStatus::Ok, RunStatus::Cancelled, RunStatus::Cancelled], "the latest first");
    assert_eq!((rig.shared.count("begun"), rig.shared.count("ended")), (3, 1));
}

#[tokio::test]
async fn a_child_spawned_again_while_its_steps_run_goes_on_with_them() {
    let rig = Rig::new().await;
    rig.library.put("slow", SLOW);
    let parent = "\
blueprint: parent
on:
  ready:
    event: card.ready
    steps:
      go: { spawn: slow, as: \"kid-${ event.data.id }\" }
";
    let applied = rig.apply(parent, json!({})).await;
    crate::handlers::active(&rig, &applied.instance).await;
    for round in 1..=3 {
        rig.journal.emit("card.ready", Source::default(), json!({ "id": 7 })).await.unwrap();
        handled(&rig, "on.ready", round).await;
    }
    reached(&rig, "hold", 1).await;

    let kids = keyed(&rig, "kid-7").await;
    assert_eq!(kids.len(), 1, "one child for one card");
    rig.open("hold");
    let runs = mains(&rig, &kids[0].id).await;
    assert_eq!(runs.len(), 1, "whose steps were started once");
    assert_eq!(rig.soon("its steps", rig.engine.wait_run(&runs[0].id)).await.unwrap().status, RunStatus::Ok);
    assert_eq!((rig.shared.count("begun"), rig.shared.count("ended")), (1, 1));
    assert!(rig.topics("step.cancelled").await.is_empty());
}

#[tokio::test]
async fn two_handlers_that_ask_for_the_same_child_at_once_are_given_one() {
    let rig = Rig::new().await;
    rig.library.put("slow", SLOW);
    // Two handlers of one instance run side by side, and both hear this.
    let parent = "\
blueprint: parent
on:
  one:
    event: card.ready
    steps:
      go: { spawn: slow, as: \"kid-${ event.data.id }\" }
  two:
    event: card.ready
    steps:
      go: { spawn: slow, as: \"kid-${ event.data.id }\" }
";
    let applied = rig.apply(parent, json!({})).await;
    crate::handlers::active(&rig, &applied.instance).await;
    for round in 1..=4 {
        rig.journal.emit("card.ready", Source::default(), json!({ "id": round })).await.unwrap();
    }
    handled(&rig, "on.one", 4).await;
    handled(&rig, "on.two", 4).await;

    for round in 1..=4 {
        let kids = keyed(&rig, &format!("kid-{round}")).await;
        assert_eq!(kids.len(), 1, "card {round} has one child, whoever asked first");
        assert_eq!(mains(&rig, &kids[0].id).await.len(), 1, "card {round}: and its steps were started once");
    }
    assert_eq!(rig.shared.count("begun"), 4);
    assert!(rig.topics("step.cancelled").await.is_empty());
}

#[tokio::test]
async fn after_a_restart_a_call_that_was_under_way_waits_for_its_child_to_go_on_where_it_was() {
    let mut rig = Rig::new().await;
    rig.library.put("staged", STAGED);
    let caller = "\
blueprint: caller
steps:
  kid: { call: staged }
  after: { do: test.count, with: { key: after } }
outputs:
  got: ${ steps.kid.out.begun }
";
    let applied = rig.apply(caller, json!({})).await;
    reached(&rig, "build", 1).await;

    rig.restart().await;
    rig.open("build");
    let run = rig.finished(&applied).await;

    assert_eq!((run.status, run.outputs), (RunStatus::Ok, json!({ "got": 1 })));
    assert_eq!(rig.shared.count("begun"), 1, "what the child had done was not done again");
    assert_eq!(
        (rig.shared.count("durable:run"), rig.shared.count("durable:resume")),
        (1, 1),
        "and what it was in the middle of was picked up where it was"
    );
    assert_eq!((rig.shared.count("ended"), rig.shared.count("after")), (1, 1));
    let kid = &keyed(&rig, "kid").await[0];
    assert_eq!(mains(&rig, &kid.id).await.len(), 1, "the child's steps were one run throughout");
    assert_eq!(kid.status, InstanceStatus::Done);
    assert!(rig.topics("step.cancelled").await.is_empty());
}

#[tokio::test]
async fn after_a_restart_a_child_that_cannot_go_on_fails_the_call_that_waited_for_it() {
    let mut rig = Rig::new().await;
    rig.library.put("slow", SLOW);
    let applied = rig.apply("blueprint: caller\nsteps:\n  kid: { call: slow }\n", json!({})).await;
    reached(&rig, "hold", 1).await;

    rig.restart().await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Failed);
    let why = crate::step(&run, "kid").error.clone().unwrap_or_default();
    assert!(why.contains("was interrupted by a restart"), "{why}");
    assert_eq!(rig.shared.count("begun"), 1, "its steps were not begun again to find that out");
}

#[tokio::test]
async fn after_a_restart_a_call_whose_child_had_finished_is_given_what_it_gave_and_does_not_call_it_again() {
    let mut rig = Rig::new().await;
    rig.library.put(
        "quick",
        "blueprint: quick\nsteps:\n  begin: { do: test.count, with: { key: begun } }\noutputs:\n  begun: ${ \
         steps.begin.out.count }\n",
    );
    // The child has finished and the call has what it gave, but the step
    // is not yet written down as done: it has something of its own to do
    // first, and the conductor stops there. Started again, the child is
    // not running, and is not there to be found by its name.
    let caller = "\
blueprint: caller
steps:
  kid:
    call: quick
    on_success:
      - { do: test.gate, with: { name: held } }
outputs:
  got: ${ steps.kid.out.begun }
";
    let applied = rig.apply(caller, json!({})).await;
    reached(&rig, "held", 1).await;

    rig.restart().await;
    reached(&rig, "held", 2).await;
    rig.open("held");
    let run = rig.finished(&applied).await;

    assert_eq!((run.status, run.outputs), (RunStatus::Ok, json!({ "got": 1 })));
    assert_eq!(rig.shared.count("begun"), 1, "the child that had finished was not called a second time");
    assert_eq!(keyed(&rig, "kid").await.len(), 1, "and no second child was made");
}
