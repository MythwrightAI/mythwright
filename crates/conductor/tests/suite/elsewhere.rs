//! An event a step says that is kept in another journal than its run's
//! is said once, wherever the conductor is stopped.
//!
//! An instance of one project that says an event naming another has
//! said an event of the other, and it is kept where that project's are:
//! in its journal, where the project has one of its own. A step done
//! again after a stop looks first for what it had said, and the places
//! its own journal keeps for that (where the step began, where an
//! element of a list did, where what is done as it starts was all done)
//! mean nothing in the other. So where that journal had got to is kept
//! before anything is said there, with the step for its own action and
//! with how far its entries have come for one of what is done around
//! it, and what the step said there after that is what it said.
//!
//! Played stopped just after such an event is kept: said for each of a
//! list, by what is done as a step starts, by the step itself beside
//! that, by what its instance does as it stops, which is done in no
//! run, and by the rules that watch a step. A step done again after a
//! stop has its rules fire again from its new beginning, and a rule
//! keeps, each time it fires, where the journal it says something in
//! had got to: the same firing finds there what it said.

use serde_json::json;

use crate::anywhere::{Does, Place, Scenario, anywhere, once, then};

const SAYS: &str = r#"blueprint: says
description: Says things of another project: for each of a list, by a step and what is done as it starts, and as it stops.
steps:
  each:
    for_each: ${ [1, 2] }
    do: emit
    with: { topic: far.each, project: far, n: ${ item } }
  told:
    needs: [each]
    do: emit
    with: { topic: far.told, project: far, by: step }
    on_start:
      - emit: { topic: far.told, project: far, by: start }
on:
  poke: { event: says.poke, outside: true, do: noop }
on_stop:
  - emit: { topic: far.gone, project: far }
"#;

const HEARS: &str = r#"blueprint: hears
description: The other project's, which is asked once for each thing said of it.
on:
  said:
    event: far.*
    steps:
      heard: { do: test.mark, with: { key: "heard-${event.topic}" } }
"#;

/// Whoever hears is listening before anything is said.
fn scenario(says: Does) -> Scenario {
    let mut scenario = Scenario::of("says", SAYS).apart();
    scenario.acts = vec![
        then(Does::ApplyFor("far", HEARS, json!({}))),
        once(|seen| seen.said("instance.active") >= 1, says),
        once(|seen| seen.said("instance.active") >= 2, Does::Stop("says")),
    ];
    scenario
}

fn said(usual: &serde_json::Value, topic: &str) -> u64 {
    usual["said"][topic].as_u64().unwrap_or(0)
}

#[tokio::test]
async fn an_event_said_of_another_project_is_said_once_where_the_conductor_that_said_it_is_stopped() {
    if !crate::alone::several() {
        return;
    }
    // What says it is a project's too, and it is that project's
    // conductor that is stopped, just after the event is kept in the
    // other's journal: for the second of a list, as the step starts,
    // and by the step itself.
    let scenario = scenario(Does::ApplyFor("near", SAYS, json!({})));
    let at = |place: &Place| ["far.each", "far.told"].contains(&place.topic.as_str()) && place.nth <= 2;
    let (usual, places) = crate::alone::stopped_alone(&scenario, "near", at).await;
    assert_eq!((said(&usual, "far.each"), said(&usual, "far.told")), (2, 2), "{usual}");
    assert_eq!((usual["asked"]["heard:test.mark"].as_u64(), places), (Some(5), 4), "{usual}");
}

#[tokio::test]
async fn an_event_said_of_another_project_is_said_once_wherever_the_factory_is_stopped() {
    // Stopped, the whole factory, just after the step itself says it
    // beside the one said as it starts, and after what the instance
    // says as it stops, which is done in no run. Where there is one
    // journal the event is kept in the run's own, and is found as any
    // is: played all the same.
    let at = |place: &Place| (place.topic == "far.told" && place.nth == 2) || place.topic == "far.gone";
    let found = anywhere(scenario(Does::Apply(SAYS, json!({}))).only(at)).await;
    assert_eq!((said(&found.usual, "far.told"), said(&found.usual, "far.gone")), (2, 1), "{}", found.usual);
    found.held();
}

const WATCHES: &str = r#"blueprint: watches
description: Says things of another project by rules that watch its step, one on a line it prints and one once it has run a while.
steps:
  work:
    do: test.speak
    with: { lines: ["seen: it"], for: 90s }
    watch:
      - log: "seen:"
        emit: { topic: far.seen, project: far }
      - after: 1m
        emit: { topic: far.late, project: far }
"#;

/// Whoever hears is listening before the step that is watched begins.
fn watching(watches: Does) -> Scenario {
    let mut scenario = Scenario::of("watches", WATCHES).apart().over("3m", "10s");
    scenario.acts = vec![
        then(Does::ApplyFor("far", HEARS, json!({}))),
        once(|seen| seen.said("instance.active") >= 1, watches),
    ];
    scenario
}

#[tokio::test]
async fn what_a_rule_says_of_another_project_is_said_once_where_the_conductor_that_said_it_is_stopped() {
    if !crate::alone::several() {
        return;
    }
    // Stopped just after each rule's event is kept in the other's
    // journal, while the step it watches is under way: started again,
    // the step is done again and its rules fire again.
    let scenario = watching(Does::ApplyFor("near", WATCHES, json!({})));
    let at = |place: &Place| ["far.seen", "far.late"].contains(&place.topic.as_str());
    let (usual, places) = crate::alone::stopped_alone(&scenario, "near", at).await;
    assert_eq!((said(&usual, "far.seen"), said(&usual, "far.late"), places), (1, 1, 2), "{usual}");
}

#[tokio::test]
async fn what_a_rule_says_of_another_project_is_said_once_wherever_the_factory_is_stopped() {
    // Where there is one journal the event is kept in the run's own, and
    // a rule says it once there too.
    let at = |place: &Place| ["far.seen", "far.late"].contains(&place.topic.as_str());
    let found = anywhere(watching(Does::Apply(WATCHES, json!({}))).only(at)).await;
    assert_eq!((said(&found.usual, "far.seen"), said(&found.usual, "far.late")), (1, 1), "{}", found.usual);
    found.held();
}
