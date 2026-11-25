//! One conductor stopped and started while the others run, and what one
//! conductor asks of another.
//!
//! A project whose journal keeps what is run for it has a conductor of
//! its own (`conductor::set`). Each can be stopped alone and started
//! again on what it left, and what another asks of it meanwhile waits:
//! here a scenario with three conductors in it is played with one of
//! them stopped just after each place the play passes, the others never
//! stopped, and held to coming to what it comes to with nothing in its
//! way; only what the stopped one had under way may be asked once more.
//!
//! And the conductors of a factory start one after another. What a step
//! that was taken up asks of a conductor that has not started yet waits
//! for it; what a start does itself is asked and not waited for, or the
//! start would wait on what cannot begin until it is over. Each is a
//! play, stopped as the suite stops a factory, and a start that does
//! wait is a failure that says so ([`crate::Rig::stopped_there_told`]).
//!
//! Where the suite is played with every project in the factory's one
//! journal (`PROJECT_JOURNALS=one`) there is one conductor, and nothing
//! of this is played.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use kernel::Event;
use serde_json::json;

use crate::Rig;
use crate::anywhere::{Does, Place, Play, STILL, Scenario, Way, anywhere, differs, once, then};

/// The project whose conductor is stopped alone. It is started after
/// every other: what a start asks of it, it asks of one not yet begun.
const LAST: &str = "zone";
/// Another, which is never stopped alone.
const OTHER: &str = "west";

const HAND: &str = r#"blueprint: hand
description: Says that it is done with each thing it is given, and says and does more as it is stopped.
on:
  asked:
    event: a.work
    outside: true
    steps:
      mark: { do: test.mark, with: { key: "hand-${event.data.n}" } }
      tell: { do: emit, with: { topic: a.done, n: "${event.data.n}" } }
on_stop:
  - do: test.mark
    with: { key: hand-stopping }
  - do: emit
    with: { topic: a.gone }
"#;

const WATCHER: &str = r#"blueprint: watcher
description: Waits for something another project's instance says as it is stopped.
steps:
  gone: { do: wait, with: { event: a.gone, scope: fleet } }
  seen: { do: emit, with: { topic: a.seen } }
"#;

const BOSS: &str = r#"blueprint: boss
description: Hears every project, keeps who said it was done, and stops that one when it is told to.
state: { who: "" }
on:
  done:
    event: a.done
    scope: fleet
    steps:
      keep: { do: set, with: { who: "${event.source.instance}" } }
  end:
    event: a.end
    outside: true
    steps:
      stop: { do: stop, with: { instance: "${state.who}" } }
      after: { do: test.mark, with: { key: boss-after } }
"#;

/// Three conductors: one project's instance works and is stopped by
/// something that is not its own, and another project's waits for what
/// it says as it stops. It is stopped when nothing of its own is under
/// way, so that which of two things came first is not what a play holds.
fn stopped_by_another() -> Scenario {
    Scenario::of("boss", BOSS)
        .apart()
        .and(then(Does::ApplyFor(LAST, HAND, json!({}))))
        .and(then(Does::ApplyFor(OTHER, WATCHER, json!({}))))
        .and(once(
            |seen| seen.said("instance.active") >= 2,
            Does::Say("a.work", json!({ "act": 1, "n": "1", "project": LAST })),
        ))
        .and(once(|seen| seen.said("a.done") >= 1, Does::Say("a.end", json!({ "act": 2 }))))
}

/// The places of the stop itself: where the step that asked for it is
/// under way, and what the instance says as it is stopped.
fn of_the_stop(place: &Place) -> bool {
    (place.blueprint == "boss" && place.step == "stop")
        || (place.blueprint == "hand" && place.topic.starts_with("instance."))
        || place.topic == "a.gone"
}

/// Where the step that asks for the stop has begun, and nothing has
/// been asked yet.
fn as_the_stop_is_asked(place: &Place) -> bool {
    place.blueprint == "boss" && place.step == "stop" && place.topic == "step.started"
}

fn said(usual: &serde_json::Value, topic: &str) -> u64 {
    usual["said"][topic].as_u64().unwrap_or(0)
}

/// Whether there is more than one conductor to play this with: where
/// there is not, whoever asks is not played, and that is said.
pub(crate) fn several() -> bool {
    !crate::one_journal()
}

#[tokio::test]
async fn what_a_step_asks_of_another_conductor_waits_for_that_one_to_have_taken_up_what_it_keeps() {
    if !several() {
        return;
    }
    // Stopped, the whole factory, where the stop of another conductor's
    // instance is under way: started again, the step that asked is taken
    // up beside a conductor that has not started yet. Asked of that one
    // as it was written, the instance was stopped out of hand, and said
    // and did nothing as it stopped.
    let found = anywhere(stopped_by_another().only(as_the_stop_is_asked)).await;
    assert_eq!((said(&found.usual, "a.gone"), said(&found.usual, "a.seen")), (1, 1), "{}", found.usual);
    assert_eq!(found.usual["asked"][":test.mark (on_stop 0)"], 1, "{}", found.usual);
    found.held();
}

const ENDER: &str = r#"blueprint: ender
description: Keeps who said it was done, and stops that one as it is stopped itself.
state: { who: "" }
on:
  done:
    event: a.done
    scope: fleet
    steps:
      keep: { do: set, with: { who: "${event.source.instance}" } }
on_stop:
  - do: emit
    with: { topic: ender.going }
  - do: stop
    with: { instance: "${state.who}" }
  - do: emit
    with: { topic: ender.gone }
"#;

#[tokio::test]
async fn what_a_start_itself_asks_of_a_conductor_that_starts_after_it_is_asked_and_not_waited_for() {
    if !several() {
        return;
    }
    // Something that was being stopped when the factory went down is
    // finished by its conductor's start, before anything is taken up:
    // and what it does as it stops is to stop an instance of a project
    // whose conductor starts later. A start that waited for that one
    // would wait for ever, which the rig says of a start that does.
    let ender = Scenario::of("ender", ENDER)
        .apart()
        .and(then(Does::ApplyFor(LAST, HAND, json!({}))))
        .and(once(
            |seen| seen.said("instance.active") >= 2,
            Does::Say("a.work", json!({ "act": 1, "n": "1", "project": LAST })),
        ))
        .and(once(|seen| seen.said("a.done") >= 1, Does::Stop("ender")))
        .only(|place| place.topic == "ender.going");
    let found = anywhere(ender).await;
    let usual = &found.usual;
    assert_eq!((said(usual, "ender.gone"), said(usual, "a.gone")), (1, 1), "{usual}");
    found.held();
}

/// A rig in memory whose projects have journals of their own: no journal
/// is shut where one conductor is stopped alone, so none has to be a
/// file that can be opened again.
async fn in_memory() -> Rig {
    let rig = Rig::new().await;
    rig.keeps_projects().await;
    rig
}

/// A turn after which the one conductor is stopped: the one that kept
/// the place.
async fn rest(play: &mut Play, at_the_place: &AtomicBool, project: &str) -> bool {
    let (mut still, mut last, mut stopped) = (0, play.mark(), false);
    while still < STILL {
        let answered = play.stepped.turn(false).await.unwrap_or(0);
        // As the turn that kept the place ends, and before anything its
        // answers woke goes on: what the conductor had asked of its
        // database in that turn is written, as it is when a conductor
        // is halted, and nothing of it is done from here.
        if at_the_place.swap(false, Ordering::SeqCst) {
            stopped = play.rig.engine.halt_one(project);
        }
        let now = play.mark();
        (still, last) = if now == last && answered == 0 { (still + 1, last) } else { (0, now) };
    }
    stopped
}

/// Starts the conductor that was stopped alone on what it left, the
/// databases answered a turn at a time as they are for any start.
async fn start_again(play: &mut Play, project: &'static str) {
    let engine = play.rig.engine.clone();
    engine.halted_one(project).await;
    let db = play.rig.journal.of_project(project).expect("the project has a journal of its own").db().clone();
    let owed = db.call(|connection| connection.query_row(crate::STOPPING, [], |row| row.get(0)));
    let stopping: i64 = play.stepped.through(false, owed).await.and_then(Result::ok).unwrap_or(0);
    {
        let mut passed = play.passed.lock().unwrap();
        passed.started_again();
        passed.stopping(stopping as usize);
    }
    let starting = tokio::spawn(async move { engine.start_one(project).await.map(|_| ()) });
    let mut idle = 0;
    while !starting.is_finished() {
        idle = if play.stepped.turn(false).await == Some(0) { idle + 1 } else { 0 };
        assert!(idle < crate::STALLED, "the conductor started alone did not come to the end of its start");
    }
    starting.await.unwrap().unwrap_or_else(|err| panic!("it was not started again: {}", err.message));
}

/// Plays `scenario` with the conductor of `project` stopped alone just
/// after `place` and started again when the play is next at rest: where
/// what it came to is not what it comes to with nothing in its way.
async fn stopped_alone_at(
    scenario: &Scenario,
    project: &'static str,
    place: &Place,
    usual: &serde_json::Value,
) -> Result<(), Vec<String>> {
    let mut play = Play::on(in_memory().await, scenario, Some(place), Way::Shut, None).await;
    play.passed.lock().unwrap().alone = Some(project);
    // The journal is told no place to shut at: the place is where the
    // one conductor is stopped, and every journal goes on.
    let at_the_place = Arc::new(AtomicBool::new(false));
    let (watching, there) = (play.passed.clone(), at_the_place.clone());
    play.rig.journal.shut_after(move |event: &Event| {
        if watching.lock().unwrap().take(event) {
            there.store(true, Ordering::SeqCst);
        }
        false
    });
    let mut stopped = false;
    for _ in 0..4_000 {
        if rest(&mut play, &at_the_place, project).await {
            start_again(&mut play, project).await;
            stopped = true;
            continue;
        }
        match play.next(scenario).await {
            None => continue,
            Some(Err(why)) => {
                return Err(vec![why, play.written_down(scenario.name, place).await]);
            }
            Some(Ok(())) => break,
        }
    }
    if !stopped {
        return Err(vec!["it came to rest without ever being where it was to be stopped".to_string()]);
    }
    let again: BTreeMap<String, usize> = play.passed.lock().unwrap().under_way.clone();
    let mut apart = differs(usual, &play.came().await, &again);
    if apart.is_empty() {
        return Ok(());
    }
    apart.push(format!(
        "everything kept in that play is in {}",
        play.written_down(scenario.name, place).await
    ));
    Err(apart)
}

/// Plays `scenario` with nothing in its way, on journals in memory, and
/// then once for each place it passed that `at` says, with the
/// conductor of `project` stopped alone just after it and started
/// again: each held to coming to the same, with only what that
/// conductor had under way let be asked once more. What it comes to
/// with nothing in its way, and how many places were played.
pub(crate) async fn stopped_alone(
    scenario: &Scenario,
    project: &'static str,
    at: impl Fn(&Place) -> bool,
) -> (serde_json::Value, usize) {
    let mut play = Play::on(in_memory().await, scenario, None, Way::Shut, None).await;
    let usual = loop {
        play.rest().await;
        match play.next(scenario).await {
            None => continue,
            Some(Err(why)) => panic!("with nothing in its way, {why}"),
            Some(Ok(())) => break play.came().await,
        }
    };
    let mut places = play.passed.lock().unwrap().places.clone();
    places.dedup();
    places.retain(at);
    drop(play);
    let mut faults = Vec::new();
    for place in &places {
        if let Err(why) = stopped_alone_at(scenario, project, place, &usual).await {
            faults.push(format!("  stopped alone after {place}:\n      {}", why.join("\n      ")));
        }
    }
    assert!(
        faults.is_empty(),
        "{}: with the conductor of `{project}` stopped alone and started again, it did not come to what it \
         comes to with nothing in its way, at {} of {} places:\n{}\nwith nothing in its way: {usual}",
        scenario.name,
        faults.len(),
        places.len(),
        faults.join("\n")
    );
    (usual, places.len())
}

#[tokio::test]
async fn a_conductor_stopped_alone_anywhere_and_started_again_comes_to_the_same_and_the_others_go_on() {
    if !several() {
        return;
    }
    // Stopped just after each thing its own journal kept, and each
    // place of the stop another conductor asked it for.
    let at = |place: &Place| place.blueprint == "hand" || of_the_stop(place);
    let (usual, places) = stopped_alone(&stopped_by_another(), LAST, at).await;
    assert_eq!(
        (said(&usual, "a.done"), said(&usual, "a.gone"), said(&usual, "a.seen")),
        (1, 1, 1),
        "{usual}"
    );
    assert!(places >= 12, "{places} places were played");
}

const SLOW: &str = r#"blueprint: slow
description: Waits where it can be taken up again, and then goes on.
steps:
  held: { do: test.durable, with: { name: held } }
  after: { do: test.mark, with: { key: slow-after } }
"#;

#[tokio::test]
async fn what_is_asked_of_a_conductor_stopped_alone_is_answered_once_it_has_been_started_again() {
    if !several() {
        return;
    }
    let rig = in_memory().await;
    rig.journal.own(LAST).await.unwrap();
    let of_it =
        |source: &str| conductor::Apply { project: Some(LAST.into()), ..rig.request(source, json!({})) };
    let slow = rig.engine.apply(of_it(SLOW)).await.unwrap();
    rig.reached("held").await;
    // Somebody waits for its run, through the factory's conductor.
    let (engine, run) = (rig.engine.clone(), slow.run.clone().unwrap());
    let waited = tokio::spawn(async move { engine.wait_run(&run).await });

    assert!(rig.engine.halt_one(LAST), "it has a conductor of its own");
    assert!(!rig.engine.halt_one(LAST), "which is stopped once");
    assert!(!rig.engine.halt_one("nowhere"), "and a project with none has none to stop");
    rig.engine.halted_one(LAST).await;
    // What it keeps is still there, as it is written: nothing of it is in hand.
    assert!(rig.engine.is_there(&slow.instance).await);
    // An apply for the project and a list of what it runs wait for it,
    // as whoever waits for its run does.
    let engine = rig.engine.clone();
    let applied = tokio::spawn({
        let asked = of_it(HAND);
        async move { engine.apply(asked).await }
    });
    let engine = rig.engine.clone();
    let listed = tokio::spawn(async move {
        let its =
            conductor::InstanceFilter { project: Some(LAST.into()), live_only: true, ..Default::default() };
        engine.instances(its).await
    });
    for _ in 0..500 {
        tokio::task::yield_now().await;
    }
    assert!(!waited.is_finished() && !applied.is_finished() && !listed.is_finished());

    // Started again, it takes up what was under way, and each is answered.
    assert_eq!(rig.engine.start_one(LAST).await.unwrap(), 1);
    let hand = rig.soon("the apply that waited", applied).await.unwrap().unwrap();
    let listed = rig.soon("the list that waited", listed).await.unwrap().unwrap();
    assert!(listed.iter().any(|one| one.id == slow.instance), "{listed:?}");
    assert!(rig.engine.is_there(&hand.instance).await);
    rig.open("held");
    let run = rig.soon("the run that was waited for", waited).await.unwrap().unwrap();
    assert_eq!(run.status, conductor::RunStatus::Ok, "{:?}", run.error);
    assert_eq!(rig.shared.count("slow-after"), 1);

    // One that is running is not started a second time, and is told why.
    let refused = rig.engine.start_one(LAST).await.unwrap_err();
    assert!(
        refused.message.contains("only one that was stopped by itself is started by itself"),
        "{refused}"
    );
}
