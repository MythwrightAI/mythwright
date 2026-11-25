//! Several agents asked about one thing at once, each for itself, and
//! what they choose counted (`agent.task` with `seats`): decided as soon
//! as it can be, the rest called off, and asked again the same done twice.

use agents::SlotSpec;
use conductor::{RunInfo, RunStatus};
use serde_json::{Value, json};

use crate::{Rig, ada, soon};

/// Three who look, each its own agent of the role, of whom two have to
/// approve. Each agent is held at a gate of its own name, so that the
/// test says who answers and when. `round` is asked of nobody: a text
/// applied again with another round has its steps run again.
const COUNCIL: &str = "\
blueprint: council
permissions: [agents]
params:
  round: { type: int, default: 1 }
steps:
  looked:
    do: agent.task
    with:
      role: reviewer
      seats: 3
      needed: 2
      thread: review:shop-1
      asking: shop-1
      prompt: \"Look at it. [gate:slot]\"
";

async fn council() -> Rig {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    for name in ["reviewer-1", "reviewer-2", "reviewer-3"] {
        let spec = SlotSpec {
            name: name.into(),
            adapter: Some("fake".into()),
            role: Some("reviewer".into()),
            project: Some("shop".into()),
            ..SlotSpec::default()
        };
        rig.agents.ensure(spec, &ada()).await.unwrap();
    }
    rig
}

fn looked(run: &RunInfo) -> Value {
    run.steps.iter().find(|step| step.step == "looked").expect("the step that looked").output.clone()
}

/// How many turns each of the three was given.
async fn given(rig: &Rig) -> Vec<usize> {
    let mut given = Vec::new();
    for name in ["reviewer-1", "reviewer-2", "reviewer-3"] {
        given.push(rig.agents.turns(name, 100).await.unwrap().len());
    }
    given
}

#[tokio::test]
async fn a_council_is_decided_as_soon_as_enough_agree_and_whoever_is_still_looking_is_called_off() {
    let rig = council().await;
    for _ in 0..3 {
        rig.fake.will_say("It does what the card asks.\nDECISION: approve");
    }
    let applied = rig.apply(COUNCIL, json!({})).await;
    rig.fake.saw(3).await;
    // Each seat is a conversation of its own, with an agent of its own.
    let mut threads: Vec<String> = Vec::new();
    for name in ["reviewer-1", "reviewer-2", "reviewer-3"] {
        for turn in rig.agents.turns(name, 10).await.unwrap() {
            threads.push(format!("{name} {}", turn.thread));
        }
    }
    assert_eq!(
        threads,
        ["reviewer-1 review:shop-1:1", "reviewer-2 review:shop-1:2", "reviewer-3 review:shop-1:3"]
    );
    rig.fake.open("reviewer-1");
    rig.fake.open("reviewer-3");
    let run = soon("the council", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();

    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let out = looked(&run);
    assert_eq!((&out["came"], &out["approved"], &out["against"]), (&json!("passed"), &json!(2), &json!(0)));
    assert_eq!(out["by"], "reviewer-1, reviewer-3", "in the order of their seats");
    let seats: Vec<u64> =
        out["votes"].as_array().unwrap().iter().map(|vote| vote["seat"].as_u64().unwrap()).collect();
    assert_eq!(seats, [1, 3], "the one still looking is not counted");
    // The one still looking is called off, which is not its agent failing.
    let off = rig.event("agent.turn.finished", |event| event.data["slot"] == "reviewer-2").await;
    assert_eq!(off.data["called_off"], true, "{}", off.data);
}

#[tokio::test]
async fn a_council_asked_again_counts_a_seat_called_off_for_its_decision_as_no_vote_and_asks_nobody_again() {
    let rig = council().await;
    rig.fake.will_say("Nothing tests it.\nDECISION: send_back");
    rig.fake.will_say("The total is wrong.\nDECISION: send_back");
    let first = rig.apply(COUNCIL, json!({})).await;
    rig.fake.saw(3).await;
    rig.fake.open("reviewer-2");
    rig.fake.open("reviewer-3");
    let run = soon("the council", rig.engine.wait_run(first.run.as_ref().unwrap())).await.unwrap();
    let out = looked(&run);
    assert_eq!((&out["came"], &out["by"]), (&json!("failed"), &json!("reviewer-2, reviewer-3")));
    rig.event("agent.turn.finished", |event| event.data["slot"] == "reviewer-1").await;

    // Asked the same again, in the same word, by the same: every seat finds
    // the turn it had as it is by now, and the one that was called off is
    // no vote, and no work to ask for again.
    let again = rig.apply(COUNCIL, json!({ "round": 2 })).await;
    let run = soon("the council again", rig.engine.wait_run(again.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let out = looked(&run);
    assert_eq!((&out["came"], &out["by"]), (&json!("failed"), &json!("reviewer-2, reviewer-3")));
    assert_eq!(given(&rig).await, [1, 1, 1], "nobody was asked a second time");
    assert_eq!(rig.fake.seen.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn a_seat_with_nobody_to_ask_has_chosen_neither_and_a_council_of_none_decides_nothing() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let run = rig.run(COUNCIL, json!({})).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let out = looked(&run);
    assert_eq!((&out["came"], &out["approved"]), (&json!("undecided"), &json!(0)));
    let words = out["words"].as_str().unwrap();
    assert!(words.contains("no agent in the role `reviewer` is active"), "{words}");
    assert_eq!(out["votes"].as_array().unwrap().len(), 3);

    let wrong = COUNCIL.replace("seats: 3", "seats: -1");
    let run = rig.run(&wrong, json!({})).await;
    assert_eq!(run.status, RunStatus::Failed);
    assert!(run.error.as_deref().unwrap_or_default().contains("is at least one"), "{:?}", run.error);
    // No seats is no council: one agent is asked, as where nothing is
    // said, and here there is none of the role.
    let one = COUNCIL.replace("seats: 3", "seats: 0");
    let run = rig.run(&one, json!({})).await;
    assert_eq!(run.status, RunStatus::Failed);
    let why = run.error.as_deref().unwrap_or_default();
    assert!(why.contains("no agent in the role `reviewer` is active"), "{why}");
    assert!(rig.fake.seen.lock().unwrap().is_empty(), "and nobody was asked anything");
}
