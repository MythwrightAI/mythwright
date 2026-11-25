//! What is ended begins nothing more, whatever it has at hand.
//!
//! A handler is ended with its generation: its instance is stopped, or
//! applied over so that its steps run again. It is one task, and chooses
//! between what ends it and the next event handed to it; where both are
//! there together, which it saw first was chance. One that was in the
//! middle of a run when its instance was applied over had that run cut,
//! came back to find itself ended and the next event waiting, and two
//! times in three took the event: a run was written for it, with the
//! handler's place moved past the event, and was cancelled as it began.
//! The handlers the instance was given when its steps had run again read
//! on from that place, and the event was nobody's.
//!
//! A step chooses the same way, between its action having ended and its
//! run being cut. Where both were there it was written as cancelled
//! three times in four, with what its action did done: an event it had
//! said was in the journal, said by a step that was cancelled.
//!
//! Nothing here goes by load: the second event is waiting behind the run
//! under way before the instance is applied over, on any machine. What a
//! stop under way does to a wait and a handler beside it is in
//! [`crate::cut_first`].

use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

use conductor::Apply;
use kernel::Source;
use serde_json::json;

use crate::handlers::active;
use crate::stepped::Stepped;
use crate::{Rig, soon};

const KEEPS: &str = "\
blueprint: keeps
description: Something that stays, and waits at the gate each thing it is told names before it counts it.
params: { n: 1, me: \"\" }
steps:
  begin: { do: test.mark, with: { key: \"began-${params.n}\" } }
on:
  told:
    event: keeps.told
    outside: true
    when: ${ event.data.to == params.me }
    steps:
      held: { do: test.gate, with: { name: \"${event.data.gate}\" } }
      took: { needs: [held], do: test.count, with: { key: \"${event.data.gate}\" } }
";

#[tokio::test]
async fn an_event_waiting_behind_a_run_that_an_apply_cuts_is_handled_once_the_steps_have_run_again() {
    let rig = Rig::new().await;
    for round in 0..8 {
        let me = format!("one-{round}");
        let (held, free) = (format!("held-{round}"), format!("free-{round}"));
        let asked =
            |n: u64| Apply { key: Some(me.clone()), ..rig.request(KEEPS, json!({ "n": n, "me": me })) };
        let instance = rig.engine.apply(asked(1)).await.unwrap().instance;
        active(&rig, &instance).await;
        let tell = async |gate: &str| {
            let told = json!({ "to": me, "gate": gate });
            rig.journal.emit("keeps.told", Source::actor("user:ada"), told).await.unwrap();
        };
        // One thing it is told, which it waits over; and another, whose
        // gate is open, handed to it behind the first.
        tell(&held).await;
        rig.reached(&held).await;
        rig.open(&free);
        tell(&free).await;
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        // Other params: its steps run again, and what it was in the
        // middle of is cut.
        let mut ended = rig.follow_after(rig.mark(), "run.finished");
        rig.engine.apply(asked(2)).await.unwrap();
        // The run that was cut, and a run for the second thing: by the
        // handler it has now, to its end. Whichever is said to have
        // ended first: the run that was cut is ended beside the steps
        // that run again, and what it says may come after the handler
        // started since has done with the second.
        let mut runs = Vec::new();
        rig.soon("two runs of its handler to end", async {
            while runs.len() < 2 {
                let event = ended.next().await.unwrap();
                if event.source.instance.as_deref() == Some(&instance) && event.data["flow"] == "on.told" {
                    runs.push(event.data["status"].as_str().unwrap_or_default().to_string());
                }
            }
        })
        .await;
        runs.sort_unstable();
        assert_eq!(
            (runs, rig.shared.count(&free)),
            (vec!["cancelled".to_string(), "ok".to_string()], 1),
            "round {round}: what waited behind the run that was cut was handled, once"
        );
    }
}

/// Says something when it is told to, and is applied with a number so
/// that its steps can be made to run again.
const SAYS: &str = "\
blueprint: says
params: { n: 1 }
on:
  told:
    event: says.told
    outside: true
    steps:
      say: { do: emit, with: { topic: says.said } }
";

/// The handler's run going forward `turns` turns, and then its instance
/// applied over by somebody who does not give way, so that the run is
/// cut with whatever its step had asked of the database answered and
/// not yet looked at. Whether the run had begun by then and whether it
/// was over, and of each run of the handler whether its step said what
/// it says and what the step was written as.
async fn cut_after(turns: usize) -> (bool, bool, Vec<(bool, String)>) {
    let rig = Rig::new().await;
    let asked = |n: u64| rig.request(SAYS, json!({ "n": n }));
    let instance = rig.engine.apply(asked(1)).await.unwrap().instance;
    active(&rig, &instance).await;
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    let mut stepped = Stepped::holding(&rig.journal);
    let told = rig.journal.emit("says.told", Source::actor("user:ada"), json!({}));
    stepped.through(false, told).await.expect("the journal is open").unwrap();
    for _ in 0..turns {
        stepped.turn(false).await;
    }
    let came = rig.mark();
    let mut over: Pin<Box<dyn Future<Output = kernel::Result<conductor::Applied>> + Send + '_>> =
        Box::pin(rig.engine.apply(asked(2)));
    let applied = match stepped.at_once(over.as_mut()) {
        Poll::Ready(applied) => applied,
        Poll::Pending => stepped.through(false, over).await.expect("the journal is open"),
    };
    applied.unwrap();
    let mut still = 0;
    while still < 12 {
        still = if stepped.turn(false).await == Some(0) { still + 1 } else { 0 };
    }
    drop(stepped);
    let before = |topic: &str, events: &[kernel::Event]| {
        events
            .iter()
            .any(|event| event.topic == topic && event.data["flow"] == "on.told" && event.seq <= came)
    };
    let events = rig.events("run.started,run.finished,says.said").await;
    let (begun, ended) = (before("run.started", &events), before("run.finished", &events));
    let mut steps = Vec::new();
    for run in rig.engine.runs(&instance, 20).await.unwrap() {
        if run.flow != "on.told" {
            continue;
        }
        let said = events
            .iter()
            .any(|event| event.topic == "says.said" && event.source.run.as_deref() == Some(run.id.as_str()));
        let whole = rig.engine.run(&run.id).await.unwrap();
        if let Some(step) = whole.steps.iter().find(|step| step.step == "say") {
            steps.push((said, format!("{:?}", step.status)));
        }
    }
    (begun, ended, steps)
}

#[tokio::test]
async fn a_step_whose_action_had_ended_as_its_run_was_cut_is_written_as_it_ended() {
    // Every amount of turns, from the run not begun until it is over: at
    // one of them what the step's action last asked is answered as the
    // run is cut, and the step is looked at with both at hand. Several
    // times each: left to chance, it was written as cancelled three
    // times in four.
    for turns in 0..=200 {
        let mut over = false;
        for round in 0..8 {
            let case = format!("{turns} turns, round {round}");
            let (begun, ended, steps) = soon(&case, cut_after(turns)).await;
            for (said, written) in steps {
                assert!(
                    !said || written == "Ok",
                    "{case}: the step's action had said what it says, and the step was written as {written}"
                );
            }
            assert!(turns > 0 || !begun, "{case}: the run had begun before it was given a turn");
            over = ended;
        }
        if over {
            return;
        }
    }
    panic!("the run was not over after 200 turns: the amounts no longer reach past it");
}
