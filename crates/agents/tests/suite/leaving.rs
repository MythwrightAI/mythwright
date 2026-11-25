//! How an agent stops being given work, and who work goes to when several
//! are asked about one thing.

use agents::{SlotSpec, TurnRequest};
use kernel::ErrorKind;
use serde_json::json;

use crate::{Rig, ada, soon};

fn work(slot: &str, thread: &str, prompt: &str) -> TurnRequest {
    TurnRequest {
        slot: slot.into(),
        kind: "task".into(),
        thread: thread.into(),
        prompt: prompt.into(),
        ..Default::default()
    }
}

/// An agent of `role` in the project, which is made with the first of them.
async fn in_role(rig: &Rig, name: &str, role: &str) {
    rig.agent(name, "fake").await;
    let spec = SlotSpec {
        name: name.into(),
        adapter: Some("fake".into()),
        project: Some("shop".into()),
        role: Some(role.into()),
        ..SlotSpec::default()
    };
    rig.agents.ensure(spec, &ada()).await.unwrap();
}

#[tokio::test]
async fn an_agent_told_to_leave_finishes_what_it_has_takes_no_more_and_is_then_stopped() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let token = std::fs::read_to_string(rig.dir.join("slots/builder/token")).unwrap();
    // One thing at work, and one waiting behind it.
    let first = rig.agents.enqueue(work("builder", "a", "the first [gate:first]")).await.unwrap();
    let second = rig.agents.enqueue(work("builder", "b", "the second")).await.unwrap();
    rig.fake.saw(1).await;

    assert!(!rig.agents.leave("builder").await.unwrap(), "it has work in hand, and is not gone yet");
    let leaving = rig.event("agent.slot.leaving", |_| true).await;
    // It says whose the agent is, as everything said of an agent does.
    assert_eq!(leaving.data, json!({ "slot": "builder", "in_hand": 2, "project": "shop" }));
    let slot = rig.agents.slot("builder").await.unwrap();
    assert!(slot.active && slot.leaving);
    // It still acts as itself, for what it has in hand.
    assert!(rig.studio.identity.authenticate(token.trim()).await.is_ok());
    // It is given no more: not by name, not as whoever is available, and
    // not as whoever a conversation was held with.
    let refused = rig.agents.enqueue(work("builder", "c", "a third")).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Conflict);
    assert!(refused.message.contains("is leaving"), "{}", refused.message);
    assert_eq!(rig.agents.available(Some("shop"), None).await.unwrap(), None);
    assert_eq!(rig.agents.holding("a").await.unwrap(), None);

    // What it had, it finishes, the one that waited included.
    rig.fake.open("first");
    let stopped = rig.event("agent.slot.stopped", |_| true).await;
    assert_eq!(stopped.data["slot"], "builder");
    for id in [&first, &second] {
        let turn = rig.agents.turn(id).await.unwrap();
        assert_eq!((turn.status.as_str(), turn.error), ("done", None), "{id}");
    }
    let slot = rig.agents.slot("builder").await.unwrap();
    assert!(!slot.active && !slot.leaving);
    assert!(rig.studio.identity.authenticate(token.trim()).await.is_err());

    // Brought in again it is back, and stays.
    rig.agent("builder", "fake").await;
    let slot = rig.agents.slot("builder").await.unwrap();
    assert!(slot.active && !slot.leaving);
}

#[tokio::test]
async fn an_agent_with_nothing_in_hand_that_is_told_to_leave_is_gone_at_once() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    assert!(rig.agents.leave("builder").await.unwrap());
    assert!(!rig.agents.slot("builder").await.unwrap().active);
    assert!(rig.events("agent.slot.leaving").await.is_empty(), "nothing was left to finish");
    // Told again, it is still gone, and nothing more is said.
    assert!(rig.agents.leave("builder").await.unwrap());
    assert_eq!(rig.events("agent.slot.stopped").await.len(), 1);
}

#[tokio::test]
async fn an_agent_stopped_at_once_is_stopped_where_it_is_and_what_waited_is_not_begun() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let first = rig.agents.enqueue(work("builder", "a", "the first [gate:never]")).await.unwrap();
    let second = rig.agents.enqueue(work("builder", "b", "the second")).await.unwrap();
    rig.fake.saw(1).await;

    rig.agents.deactivate("builder").await.unwrap();
    let ended = soon("both to end", async {
        (rig.agents.wait(&first).await.unwrap(), rig.agents.wait(&second).await.unwrap())
    })
    .await;
    assert_eq!(ended.0.error.as_deref(), Some("the turn was cancelled"));
    assert_eq!(ended.1.error.as_deref(), Some("its agent was stopped"));
    assert_eq!(rig.fake.prompts().len(), 1, "the one that waited never reached its program");
}

#[tokio::test]
async fn a_turn_called_off_at_its_work_is_counted_and_not_among_those_that_failed() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let held = rig.agents.enqueue(work("builder", "a", "the first [gate:never]")).await.unwrap();
    rig.fake.saw(1).await;
    rig.agents.deactivate("builder").await.unwrap();
    let ended = soon("it to end", rig.agents.wait(&held)).await.unwrap();
    assert_eq!(ended.error.as_deref(), Some("the turn was cancelled"));
    let said = rig.event("agent.turn.finished", |event| event.data["turn"] == held).await;
    assert_eq!((&said.data["ok"], &said.data["called_off"]), (&json!(false), &json!(true)));

    let query = journal::MetricQuery { name: "agent.turn".into(), ..Default::default() };
    let turns = rig.journal.query_metrics(query).await.unwrap();
    assert_eq!((turns[0].total.count, turns[0].total.errors), (1, 0));
}

#[tokio::test]
async fn several_asked_about_one_thing_are_each_a_different_agent_while_there_are_enough() {
    let rig = Rig::new().await;
    for name in ["reviewer-1", "reviewer-2", "reviewer-3"] {
        in_role(&rig, name, "reviewer").await;
    }
    // Three seats on one card, asked side by side: each of a role, each a
    // conversation of its own, and each apart from the others.
    let seats = "\
blueprint: seats
permissions: [agents]
params:
  seat: { type: int }
steps:
  look:
    do: agent.task
    with:
      role: reviewer
      thread: \"review:shop-1:${params.seat}\"
      apart: \"review:shop-1:\"
      prompt: \"look [gate:look]\"
outputs:
  by: ${steps.look.out.slot}
";
    let mut asked = Vec::new();
    for seat in 1..=3 {
        let request = conductor::Apply {
            source: Some(seats.to_string()),
            params: kernel::fields(json!({ "seat": seat })),
            project: Some("shop".into()),
            key: Some(format!("seat-{seat}")),
            ..conductor::Apply::new("seats", ada())
        };
        asked.push(tokio::spawn({
            let engine = rig.engine.clone();
            async move { engine.apply(request).await.unwrap() }
        }));
    }
    rig.fake.saw(3).await;
    let mut seated: Vec<String> =
        rig.fake.seen.lock().unwrap().iter().map(|seen| seen.slot.clone()).collect();
    seated.sort();
    assert_eq!(seated, ["reviewer-1", "reviewer-2", "reviewer-3"]);
    for _ in 0..3 {
        rig.fake.open("look");
    }
    for one in asked {
        let applied = one.await.unwrap();
        soon("a seat", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    }

    // Asked again, each seat is whoever it was before: a conversation is
    // carried on where it was held.
    let held = |seat: u32| {
        let agents = rig.agents.clone();
        async move { agents.holding(&format!("review:shop-1:{seat}")).await.unwrap().unwrap().name }
    };
    let before = [held(1).await, held(2).await, held(3).await];
    let mut distinct = before.to_vec();
    distinct.sort();
    assert_eq!(distinct, ["reviewer-1", "reviewer-2", "reviewer-3"]);
    // A fourth seat has nobody left to be apart from the others with: one
    // of them is asked twice, and it is not refused.
    let beside = rig.agents.beside("review:shop-1:", "review:shop-1:4").await.unwrap();
    assert_eq!(beside.len(), 3, "{beside:?}");
    let fourth = rig
        .agents
        .available_apart(Some("shop"), Some("reviewer"), &beside, agents::Order::First)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fourth.name, "reviewer-1");
    // Another card's seats are none of this one's business.
    assert!(rig.agents.beside("review:shop-2:", "review:shop-2:1").await.unwrap().is_empty());
}

#[tokio::test]
async fn work_for_whoever_is_available_goes_to_the_one_with_least_in_hand() {
    let rig = Rig::new().await;
    rig.agent("first", "fake").await;
    rig.agent("second", "fake").await;
    // Two for the first, one for the second: none of them finishing.
    rig.agents.enqueue(work("first", "a", "one [gate:never]")).await.unwrap();
    rig.agents.enqueue(work("first", "b", "two")).await.unwrap();
    rig.agents.enqueue(work("second", "c", "three [gate:never]")).await.unwrap();
    rig.fake.saw(2).await;
    let next = rig.agents.available(Some("shop"), None).await.unwrap().unwrap();
    assert_eq!(next.name, "second", "both are at work, and the second has less waiting behind it");
}
