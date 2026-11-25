//! An apply that comes to something while it is being stopped.
//!
//! A stop takes a while: what the instance's text says is done as it
//! stops is done, what is under it is stopped, what it held is given
//! back, and only then is it written as stopped. An apply that comes to
//! it in that while found it alive, and was taken by it and cut with it.
//! It waits now until the stop is over, and is then an apply where
//! nothing is running under the key: both happen, in the order they
//! were asked.
//!
//! The while is held open here by what the instance does as it stops: a
//! stand-in that waits at a gate, which a play opens only when it is at
//! rest with nothing else to do. So the apply is asked while the stop
//! waits, on any machine, and the stop goes on only once the apply has
//! come as far as it can.

use std::sync::Arc;

use blueprint::{ActionSpec, ValueType};
use conductor::{ActionError, Apply, InstanceFilter, InstanceStatus, Registry, RunStatus};
use serde_json::{Value, json};

use crate::anywhere::{Does, Found, Place, STILL, Scenario, Seen, anywhere, once, then};
use crate::stepped::Stepped;
use crate::{Rig, Shared, action};

/// The stand-ins these plays need, beside the rig's own.
pub fn stand_ins(registry: &mut Registry, shared: &Arc<Shared>) {
    action(
        registry,
        ActionSpec::new(
            "test.lingers",
            "Wait at the named gate, as `test.gate` does; done again after the conductor was stopped, or as \
             one comes up, go on at once: whatever it waited for went with the conductor.",
        )
        .required("name", ValueType::String, "which gate")
        .idempotent(),
        shared,
        |cx, args, shared| async move {
            if !cx.again() && !shared.starting.load(std::sync::atomic::Ordering::SeqCst) {
                let gate = shared.gate(args["name"].as_str().unwrap());
                cx.emit("test.reached", json!({ "gate": args["name"] })).await?;
                tokio::select! {
                    permit = gate.acquire() => permit.unwrap().forget(),
                    () = cx.cancel().cancelled() => return Err(ActionError::cancelled()),
                }
            }
            Ok(json!({ "passed": args["name"] }))
        },
    );
    action(
        registry,
        ActionSpec::new(
            "test.apply",
            "Apply a text that is there by name as an instance of its own, for the instance this step is \
             of, and wait for its steps.",
        )
        .required("blueprint", ValueType::String, "its name")
        .arg("params", ValueType::Object, "its params")
        .idempotent(),
        shared,
        |cx, args, _| async move {
            let engine = cx.engine().clone();
            let request = Apply {
                params: args.get("params").and_then(Value::as_object).cloned().unwrap_or_default(),
                project: cx.project().map(str::to_string),
                ..Apply::new(args["blueprint"].as_str().unwrap(), cx.principal().clone())
            };
            let applied = engine.apply_within(request, cx.instance()).await?;
            if let Some(run) = &applied.run {
                let finished = tokio::select! {
                    finished = engine.wait_run(run) => finished?,
                    () = cx.cancel().cancelled() => return Err(ActionError::cancelled()),
                };
                if finished.status != RunStatus::Ok {
                    let why = finished.error.unwrap_or_else(|| "its steps did not finish".into());
                    return Err(ActionError::new(format!("what was applied came to nothing: {why}")));
                }
            }
            // Nothing of whether it was made: done again where the
            // conductor was stopped, it finds what it made the first time.
            Ok(json!({ "instance": applied.instance }))
        },
    );
}

const SLOW: &str = r#"blueprint: slow
description: Something that stays, says that it is up, and takes a while to stop.
params: { n: 1 }
steps:
  begin: { do: test.mark, with: { key: "slow-began-${params.n}" } }
  tell: { needs: [begin], do: emit, with: { topic: slow.up, n: "${params.n}" } }
on:
  poke: { event: slow.poke, outside: true, do: noop }
on_stop:
  - do: test.lingers
    with: { name: stopping }
  - do: test.mark
    with: { key: slow-stopped }
outputs:
  n: ${ params.n }
"#;

/// The same under another text, whose steps are others: applied over
/// one that runs the first, its steps would run again.
const SLOW_AGAIN: &str = r#"blueprint: slow
description: Something that stays, says that it is up a second way, and takes a while to stop.
params: { n: 1 }
steps:
  anew: { do: test.mark, with: { key: "slow-anew-${params.n}" } }
  tell: { needs: [anew], do: emit, with: { topic: slow.up, n: "${params.n}" } }
on:
  poke: { event: slow.poke, outside: true, do: noop }
on_stop:
  - do: test.lingers
    with: { name: stopping }
  - do: test.mark
    with: { key: slow-stopped }
outputs:
  n: ${ params.n }
"#;

/// The first with a handler more: it begins as the first does, and one
/// that runs the first would take it where it is.
const SLOW_ALIKE: &str = r#"blueprint: slow
description: Something that stays, says that it is up, hears one thing more, and takes a while to stop.
params: { n: 1 }
steps:
  begin: { do: test.mark, with: { key: "slow-began-${params.n}" } }
  tell: { needs: [begin], do: emit, with: { topic: slow.up, n: "${params.n}" } }
on:
  poke: { event: slow.poke, outside: true, do: noop }
  nudge: { event: slow.nudge, outside: true, do: noop }
on_stop:
  - do: test.lingers
    with: { name: stopping }
  - do: test.mark
    with: { key: slow-stopped }
outputs:
  n: ${ params.n }
"#;

/// The first saying how one that is running takes it: one that runs the
/// first would be carried over to it.
const SLOW_CARRIED: &str = r#"blueprint: slow
description: Something that stays, says that it is up, takes a while to stop, and says how it is taken.
params: { n: 1 }
steps:
  begin: { do: test.mark, with: { key: "slow-began-${params.n}" } }
  tell: { needs: [begin], do: emit, with: { topic: slow.up, n: "${params.n}" } }
on:
  poke: { event: slow.poke, outside: true, do: noop }
on_update:
  - do: test.mark
    with: { key: slow-carried }
on_stop:
  - do: test.lingers
    with: { name: stopping }
  - do: test.mark
    with: { key: slow-stopped }
outputs:
  n: ${ params.n }
"#;

const CALLS: &str = r#"blueprint: calls
description: It starts something that stays, stops it when it is told to, and calls it again when it is told to.
steps:
  kid: { spawn: slow, as: the-kid }
on:
  halt:
    event: play.halt
    outside: true
    steps:
      stop: { do: stop, with: { instance: the-kid } }
      after: { needs: [stop], do: test.mark, with: { key: halted } }
  again:
    event: play.again
    outside: true
    steps:
      back: { call: slow, as: the-kid, with: { n: 2 } }
      after: { needs: [back], do: test.mark, with: { key: "back-${steps.back.out.n}" } }
"#;

const SPAWNS: &str = r#"blueprint: spawns
description: It starts something that stays, stops it when it is told to, and starts it again when it is told to.
steps:
  kid: { spawn: slow, as: the-kid }
on:
  halt:
    event: play.halt
    outside: true
    steps:
      stop: { do: stop, with: { instance: the-kid } }
      after: { needs: [stop], do: test.mark, with: { key: halted } }
  again:
    event: play.again
    outside: true
    steps:
      back: { spawn: slow, as: the-kid, with: { n: 2 } }
      after: { needs: [back], do: test.mark, with: { key: back } }
"#;

const KEEPER: &str = r#"blueprint: keeper
description: Keeps which instance first said it was up, stops that one when it is told to, and applies its text again when it is told to.
state: { who: "" }
on:
  up:
    event: slow.up
    steps:
      keep: { when: ${ state.who == "" }, do: set, with: { who: "${event.source.instance}" } }
  halt:
    event: play.halt
    outside: true
    steps:
      stop: { do: stop, with: { instance: "${state.who}" } }
      after: { needs: [stop], do: test.mark, with: { key: halted } }
  again:
    event: play.again
    outside: true
    steps:
      back: { do: test.apply, with: { blueprint: slow, params: { n: 2 } } }
      after: { needs: [back], do: test.mark, with: { key: back } }
"#;

/// The stop waits; or is over, where the conductor went as it began
/// and finished it as it came back, with nobody left to wait for.
/// The same, asking for what is there by name with the params it has:
/// for the text that is there then, which may be another.
const KEEPER_ASKS_THE_SAME: &str = r#"blueprint: keeper
description: Keeps which instance first said it was up, stops that one when it is told to, and applies what is there under its name when it is told to.
state: { who: "" }
on:
  up:
    event: slow.up
    steps:
      keep: { when: ${ state.who == "" }, do: set, with: { who: "${event.source.instance}" } }
  halt:
    event: play.halt
    outside: true
    steps:
      stop: { do: stop, with: { instance: "${state.who}" } }
      after: { needs: [stop], do: test.mark, with: { key: halted } }
  again:
    event: play.again
    outside: true
    steps:
      back: { do: test.apply, with: { blueprint: slow } }
      after: { needs: [back], do: test.mark, with: { key: back } }
"#;

fn held(seen: &Seen) -> bool {
    seen.said("test.reached") >= 1 || seen.said("instance.stopped") >= 1
}

/// Something stopped by a step of what started it, and asked for again
/// by another step of that while the stop waits.
fn by_its_parent(name: &'static str, text: &'static str) -> Scenario {
    Scenario::of(name, text)
        .with(&[SLOW])
        .and(once(|seen| seen.said("slow.up") >= 1, Does::Say("play.halt", json!({ "act": 1 }))))
        .and(once(held, Does::Say("play.again", json!({ "act": 2 }))))
}

/// Something of its own stopped by a step of another tree; `asked` is
/// what asks for it again while the stop waits.
fn of_its_own(name: &'static str, asked: Does) -> Scenario {
    Scenario::of(name, KEEPER)
        .with(&[SLOW])
        .and(then(Does::Apply(SLOW, json!({}))))
        .and(once(|seen| seen.said("instance.state") >= 1, Does::Say("play.halt", json!({ "act": 1 }))))
        .and(once(held, asked))
}

/// The same, asked for by a step that asks for what is there by name,
/// with the params it has, once `there` has been put where texts are
/// found: nothing, for the text it runs.
fn asked_as_it_is(name: &'static str, there: Option<&'static str>) -> Scenario {
    let again = Does::Say("play.again", json!({ "act": 2 }));
    let scenario = Scenario::of(name, KEEPER_ASKS_THE_SAME)
        .with(&[SLOW])
        .and(then(Does::Apply(SLOW, json!({}))))
        .and(once(|seen| seen.said("instance.state") >= 1, Does::Say("play.halt", json!({ "act": 1 }))));
    match there {
        Some(there) => scenario.and(once(held, Does::Put(there))).and(then(again)),
        None => scenario.and(once(held, again)),
    }
}

/// What a play came to, with nothing in its way: the one that was there
/// was stopped whole, once, and never took what was applied; and there
/// is one after it under its name, alive, whose steps ran. `updates` is
/// how many texts were taken in all: by what was applied over, and never
/// by what was being stopped.
fn stopped_and_made_anew(found: &Found, slow: &str, updates: u64) {
    let usual = &found.usual;
    let said = |topic: &str| usual["said"][topic].as_u64().unwrap_or(0);
    let asked = |what: &str| usual["asked"][what].as_u64().unwrap_or(0);
    assert_eq!(usual["instances"][slow]["status"], "active", "{usual}");
    assert_eq!(usual["runs"][slow]["main"], json!([usual["runs"][slow]["main"][0]]), "one run of its steps");
    assert_eq!(usual["runs"][slow]["main"][0]["status"], "ok", "{usual}");
    let before = usual["earlier"][slow].as_array().cloned().unwrap_or_default();
    assert_eq!(before.len(), 1, "the one that was stopped is there, before it: {usual}");
    assert_eq!(before[0]["instance"]["status"], "stopped", "{usual}");
    assert_eq!(before[0]["runs"]["main"].as_array().map(Vec::len), Some(1), "it took nothing: {usual}");
    let taken = usual["said"]["instance.updated"].as_u64().unwrap_or(0);
    assert_eq!((said("instance.stopped"), taken, said("instance.update_failed")), (1, updates, 0), "{usual}");
    assert_eq!(said("run.failed"), 0, "nobody was told that anything failed: {usual}");
    assert_eq!(
        (asked(":test.lingers (on_stop 0)"), asked(":test.mark (on_stop 1)")),
        (1, 1),
        "what it does as it stops was done whole, once: {usual}"
    );
    assert_eq!(usual["amiss"], json!([]));
}

#[tokio::test]
async fn a_call_that_comes_to_what_is_being_stopped_waits_and_starts_it_anew_wherever_the_conductor_is_stopped()
 {
    let found = anywhere(by_its_parent("calls", CALLS)).await;
    stopped_and_made_anew(&found, "calls[calls] > slow[the-kid]", 0);
    let again = &found.usual["runs"]["calls[calls]"]["on.again"][0];
    assert_eq!(again["status"], "ok", "{again}");
    assert_eq!(again["steps"]["back"]["gave"]["n"], 2, "the call is given what the steps it asked for gave");
    found.held();
}

#[tokio::test]
async fn a_spawn_that_comes_to_what_is_being_stopped_waits_and_starts_it_anew_wherever_the_conductor_is_stopped()
 {
    let found = anywhere(by_its_parent("spawns", SPAWNS)).await;
    stopped_and_made_anew(&found, "spawns[spawns] > slow[the-kid]", 0);
    assert_eq!(found.usual["runs"]["spawns[spawns]"]["on.again"][0]["status"], "ok", "{}", found.usual);
    found.held();
}

#[tokio::test]
async fn a_step_of_another_tree_that_applies_what_is_being_stopped_waits_and_makes_it_anew_wherever_the_conductor_is_stopped()
 {
    let found = anywhere(of_its_own("stepped", Does::Say("play.again", json!({ "act": 2 })))).await;
    stopped_and_made_anew(&found, "slow[slow]", 0);
    assert_eq!(found.usual["runs"]["keeper[keeper]"]["on.again"][0]["status"], "ok", "{}", found.usual);
    found.held();
}

#[tokio::test]
async fn somebody_who_applies_what_is_being_stopped_is_answered_once_it_is_made_anew_wherever_the_conductor_is_stopped()
 {
    let found = anywhere(of_its_own("asked", Does::Asks(SLOW_AGAIN, json!({ "n": 2 })))).await;
    stopped_and_made_anew(&found, "slow[slow]", 0);
    assert_eq!(found.usual["answered"], json!(["made"]), "{}", found.usual);
    found.held();
}

#[tokio::test]
async fn whichever_way_it_would_have_taken_what_is_asked_something_being_stopped_takes_nothing_and_is_made_anew()
 {
    // Asked for as it is, it would have been answered that nothing
    // changed; for a text that begins as its own does, it would have
    // taken it where it is; for one that says how it is taken, it would
    // have been carried over. Each of them over something that was then
    // stopped, with whoever asked told all was well.
    for (name, there, marks) in
        [("as it is", None, 0), ("begins alike", Some(SLOW_ALIKE), 0), ("is carried", Some(SLOW_CARRIED), 0)]
    {
        let found = anywhere(asked_as_it_is(name, there).only(|_| false)).await;
        stopped_and_made_anew(&found, "slow[slow]", 0);
        assert_eq!(found.usual["runs"]["keeper[keeper]"]["on.again"][0]["status"], "ok", "{name}");
        assert_eq!(found.usual["asked"]["begin:test.mark"], 2, "{name}: its steps ran, in the one made anew");
        let carried = found.usual["asked"][":test.mark (on_update 0)"].as_u64().unwrap_or(0);
        assert_eq!(carried, marks, "{name}: nothing was carried over to anything");
    }
}

const COMES_BACK: &str = r#"blueprint: comes-back
description: Something that stays, and asks for itself again as it is stopped.
steps:
  begin: { do: test.mark, with: { key: came } }
on:
  poke: { event: comes-back.poke, outside: true, do: noop }
on_stop:
  - do: test.apply
    with: { blueprint: comes-back }
  - do: test.mark
    with: { key: went }
"#;

/// How a refusal for being part of the stop begins and ends.
fn refused(error: &str, key: &str) {
    assert!(
        error.contains(&format!("`{key}` was not applied: it is being stopped"))
            && error.contains("Nothing was changed")
            && error.contains("`instance.finished`"),
        "it says why, and what to do: {error}"
    );
}

async fn of(rig: &Rig, blueprint: &str) -> Vec<conductor::InstanceInfo> {
    let filter = InstanceFilter { blueprint: Some(blueprint.to_string()), limit: 100, ..Default::default() };
    rig.engine.instances(filter).await.unwrap()
}

#[tokio::test]
async fn what_an_instance_does_as_it_stops_is_refused_the_instance_and_the_stop_goes_on() {
    let rig = Rig::new().await;
    rig.library.put("comes-back", COMES_BACK);
    let first = rig.apply(COMES_BACK, json!({})).await;
    rig.finished(&first).await;
    // Waited for, the stop would wait for what it does, and that for the
    // stop: it would never be over.
    rig.soon(
        "the stop of something that asks for itself as it stops to be over: it waits for itself",
        rig.engine.stop(&first.instance),
    )
    .await
    .unwrap();
    let could_not = rig.events("hook.failed").await;
    assert_eq!(could_not.len(), 1, "{could_not:?}");
    refused(could_not[0].data["error"].as_str().unwrap(), "comes-back");
    let all = of(&rig, "comes-back").await;
    assert_eq!(all.len(), 1, "nothing was made in its place");
    assert_eq!(all[0].status, InstanceStatus::Stopped);
    assert_eq!((rig.shared.count("came"), rig.shared.count("went")), (1, 1), "and the rest of it was done");
}

const SENDS: &str = r#"blueprint: sends
description: Something that stays, and as it is stopped starts something that asks for it again, and waits for that.
steps:
  begin: { do: test.mark, with: { key: sends-came } }
on:
  poke: { event: sends.poke, outside: true, do: noop }
on_stop:
  - do: test.apply
    with: { blueprint: errand }
  - do: test.mark
    with: { key: sends-went }
"#;

const ERRAND: &str = r#"blueprint: errand
description: Asks for something that is there by name.
steps:
  back: { do: test.apply, with: { blueprint: sends } }
"#;

#[tokio::test]
async fn what_a_stop_began_and_waits_for_is_refused_the_instance_being_stopped() {
    let rig = Rig::new().await;
    rig.library.put("sends", SENDS);
    rig.library.put("errand", ERRAND);
    let first = rig.apply(SENDS, json!({})).await;
    rig.finished(&first).await;
    // The stop waits for what it started, whose step asks for what is
    // being stopped: waited for, each would wait for the other.
    rig.soon(
        "the stop of something that starts what asks for it, and waits for that, to be over: they wait for \
         each other",
        rig.engine.stop(&first.instance),
    )
    .await
    .unwrap();
    let errand = of(&rig, "errand").await;
    assert_eq!(errand.len(), 1);
    let run = rig.engine.runs(&errand[0].id, 10).await.unwrap().remove(0);
    assert_eq!(run.status, RunStatus::Failed);
    refused(&run.error.unwrap_or_default(), "sends");
    let all = of(&rig, "sends").await;
    assert_eq!(all.len(), 1, "nothing was made in its place");
    assert_eq!(all[0].status, InstanceStatus::Stopped);
    assert_eq!(rig.shared.count("sends-went"), 1, "and the rest of what it does as it stops was done");
}

const ROOF: &str = r#"blueprint: roof
description: Something that stays over something else.
steps:
  kid: { spawn: under, as: the-kid }
on:
  poke: { event: roof.poke, outside: true, do: noop }
"#;

const UNDER: &str = r#"blueprint: under
description: Something that stays, and asks for what it is under as it is stopped.
on:
  poke: { event: under.poke, outside: true, do: noop }
on_stop:
  - do: test.apply
    with: { blueprint: roof }
  - do: test.mark
    with: { key: under-went }
"#;

#[tokio::test]
async fn what_something_under_it_does_as_it_is_stopped_is_refused_the_instance_being_stopped() {
    let rig = Rig::new().await;
    rig.library.put("roof", ROOF);
    rig.library.put("under", UNDER);
    let first = rig.apply(ROOF, json!({})).await;
    rig.finished(&first).await;
    rig.event("instance.active", |event| event.source.instance.as_deref() != Some(&first.instance)).await;
    // What is under it is stopped as part of its stop, and asks for it
    // as that is done: waited for, the stop would wait for itself.
    rig.soon(
        "the stop of something whose child asks for it as it is stopped to be over: it waits for itself",
        rig.engine.stop(&first.instance),
    )
    .await
    .unwrap();
    let could_not = rig.events("hook.failed").await;
    assert_eq!(could_not.len(), 1, "{could_not:?}");
    refused(could_not[0].data["error"].as_str().unwrap(), "roof");
    let all = of(&rig, "roof").await;
    assert_eq!(all.len(), 1, "nothing was made in its place");
    assert_eq!(all[0].status, InstanceStatus::Stopped);
    assert_eq!(of(&rig, "under").await[0].status, InstanceStatus::Stopped);
    assert_eq!(rig.shared.count("under-went"), 1, "and the rest of what it does as it stops was done");
}

const STAYS_OVER: &str = r#"blueprint: stays-over
description: Something that stays over something else, and takes a while to stop.
steps:
  kid: { spawn: under-it, as: the-kid }
on:
  poke: { event: stays-over.poke, outside: true, do: noop }
on_stop:
  - do: test.lingers
    with: { name: stopping }
"#;

const UNDER_IT: &str = r#"blueprint: under-it
description: Something that stays.
steps:
  begin: { do: test.count, with: { key: under-it-began } }
on:
  poke: { event: under-it.poke, outside: true, do: noop }
"#;

/// Waits until nothing the conductor has running asks anything more of
/// its database: whatever was asked for has come as far as it can.
async fn rested(rig: &Rig) {
    let (mut stepped, mut still) = (Stepped::holding(&rig.journal), 0);
    while still < STILL {
        still = if stepped.turn(false).await == Some(0) { still + 1 } else { 0 };
    }
}

#[tokio::test]
async fn somebody_who_applies_what_is_under_something_being_stopped_waits_for_that_and_is_told_it_is_gone() {
    let rig = Rig::new().await;
    rig.library.put("under-it", UNDER_IT);
    let first = rig.apply(STAYS_OVER, json!({})).await;
    rig.finished(&first).await;
    let kid = rig.event("instance.active", |event| event.source.instance.as_deref() != Some(&first.instance));
    let kid = kid.await.source.instance.clone().unwrap();
    let stopping = tokio::spawn({
        let (engine, id) = (rig.engine.clone(), first.instance.clone());
        async move { engine.stop(&id).await }
    });
    rig.reached("stopping").await;
    // What is under it has been called off, and has not been come to:
    // asked for now with its steps to run again, they were cancelled as
    // they began, under something that was then stopped.
    let asking = tokio::spawn({
        let (engine, kid) = (rig.engine.clone(), kid.clone());
        async move { engine.apply_again(&kid, crate::user(), true).await }
    });
    rested(&rig).await;
    assert!(!asking.is_finished(), "it waits for the stop it is part of");
    rig.open("stopping");
    rig.soon("the stop to be over", stopping).await.unwrap().unwrap();
    let answered = rig.soon("whoever asked to be answered", asking).await.unwrap();
    let refused = answered.expect_err("what it was under is gone").message;
    assert!(refused.contains("what it was to be started under was stopped"), "{refused}");
    let all = of(&rig, "under-it").await;
    assert_eq!(all.len(), 1, "nothing was made in its place");
    assert_eq!(all[0].status, InstanceStatus::Stopped);
    assert_eq!(rig.shared.count("under-it-began"), 1, "and its steps were not asked for again");
    assert_eq!(rig.events("instance.updated").await.len(), 0);
    assert_eq!(rig.engine.runs(&kid, 10).await.unwrap().len(), 1, "no run of it was begun to be cancelled");
}

const OVER: &str = r#"blueprint: over
description: It starts something that stays, and stops it when it is told to.
steps:
  kid: { spawn: slow, as: the-kid }
on:
  halt:
    event: play.halt
    outside: true
    steps:
      stop: { do: stop, with: { instance: the-kid } }
"#;

/// The same hearing one thing more: over one that runs the first it is
/// taken where it is, and what is under it is looked at.
const OVER_AGAIN: &str = r#"blueprint: over
description: It starts something that stays, stops it when it is told to, and hears one thing more.
steps:
  kid: { spawn: slow, as: the-kid }
on:
  halt:
    event: play.halt
    outside: true
    steps:
      stop: { do: stop, with: { instance: the-kid } }
  nudge: { event: over.nudge, outside: true, do: noop }
"#;

#[tokio::test]
async fn the_look_below_what_somebody_applied_asks_nothing_of_what_is_being_stopped_and_says_nothing() {
    // What is under it is behind the text now there, which it would
    // take where it is; and it is being stopped as the look comes to it.
    let looked = Scenario::of("over", OVER)
        .with(&[SLOW])
        .and(once(|seen| seen.said("slow.up") >= 1, Does::Say("play.halt", json!({ "act": 1 }))))
        .and(once(held, Does::Put(SLOW_ALIKE)))
        .and(then(Does::Asks(OVER_AGAIN, json!({}))));
    let usual = anywhere(looked.only(|_| false)).await.usual;
    assert_eq!(usual["answered"], json!(["applied over what was there"]), "{usual}");
    let kid = &usual["instances"]["over[over] > slow[the-kid]"];
    assert_eq!(kid["status"], "stopped", "{usual}");
    assert_eq!(usual["earlier"], json!({}), "nothing was made in its place");
    let said = |topic: &str| usual["said"][topic].as_u64().unwrap_or(0);
    assert_eq!(said("instance.updated"), 1, "the one that was applied took its text, and no other: {usual}");
    assert_eq!(said("instance.update_failed"), 0, "and nothing is said of the one being stopped: {usual}");
    assert_eq!(usual["asked"][":test.mark (on_stop 1)"], 1, "which was stopped whole: {usual}");
}

const TURNS: &str = r#"blueprint: turns
description: It starts something that stays, and stops it when it is told to.
steps:
  kid: { spawn: slow, as: the-kid }
on:
  halt:
    event: play.halt
    outside: true
    steps:
      stop: { do: stop, with: { instance: the-kid } }
      after: { needs: [stop], do: test.mark, with: { key: halted } }
"#;

/// The same with other steps, which start what the first started with
/// other params: applied over one that runs the first, its steps run
/// again, and what its handlers had under way is cut short.
const TURNS_AGAIN: &str = r#"blueprint: turns
description: It marks that it began, starts something that stays, and stops it when it is told to.
steps:
  mark: { do: test.mark, with: { key: turned } }
  kid: { needs: [mark], spawn: slow, as: the-kid, with: { n: 2 } }
on:
  halt:
    event: play.halt
    outside: true
    steps:
      stop: { do: stop, with: { instance: the-kid } }
      after: { needs: [stop], do: test.mark, with: { key: halted } }
"#;

/// Something stopped by a step that is cut short while the stop waits,
/// by an apply whose steps then ask for the very thing being stopped.
fn cut_short() -> Scenario {
    Scenario::of("turns", TURNS)
        .with(&[SLOW])
        .and(once(|seen| seen.said("slow.up") >= 1, Does::Say("play.halt", json!({ "act": 1 }))))
        .and(once(held, Does::Asks(TURNS_AGAIN, json!({}))))
}

#[tokio::test]
async fn a_stop_whose_asker_is_cut_short_goes_on_to_its_end_and_what_asks_for_the_same_thing_waits_for_it() {
    // Stopped where the step that asked for the stop has been cut short,
    // which is what this is of: from the first thing the steps that run
    // again say. Before that, a conductor that comes back finishes the
    // stop before anybody applies, and nothing is cut. (And stopped in
    // the write that takes the text, before the run that asked for the
    // stop is written as cut, it takes that run up and ends it well:
    // what becomes of a handler's run under way where its instance's
    // steps are to run again and the conductor stops between the two
    // is not held, and is said in `docs/status.md`'s list.)
    let since_cut = |place: &Place| {
        let again = place.nth == 2 && (place.step == "kid" || place.blueprint == "slow");
        place.step == "mark" || again
    };
    let found = anywhere(cut_short().only(since_cut)).await;
    assert!(
        found.tried.len() >= 20,
        "it was stopped where the stop went on by itself: {}",
        found.tried.len()
    );
    stopped_and_made_anew(&found, "turns[turns] > slow[the-kid]", 1);
    let halt = &found.usual["runs"]["turns[turns]"]["on.halt"][0];
    assert_eq!(halt["status"], "cancelled", "the step that asked for the stop was cut short: {halt}");
    assert_eq!(found.usual["instances"]["turns[turns]"]["status"], "active", "{}", found.usual);
    found.held();
}
