//! What the conductor writes down and what it says of it, where the two
//! were still apart: stopped anywhere and started again.
//!
//! What a blueprint says is counted in every play. What the conductor
//! says of its own steps and runs mostly is not: a run taken up says so,
//! and that is no part of what a blueprint came to. So what is held of
//! it is held by whoever hears it: a handler that hears a step being
//! passed over, something done around a step that could not be done, or
//! the event that brought its text, is asked once for each, wherever
//! the conductor was stopped. And what every run says of itself is held
//! in every play ([`crate::amiss`]).

use serde_json::json;

use crate::anywhere::{Does, Found, Scenario, Seen, anywhere, once, then};
use crate::handlers::active;
use crate::judged::left_behind;
use crate::{Rig, user};

fn asked(found: &Found, what: &str) -> u64 {
    found.usual["asked"][what].as_u64().unwrap_or(0)
}

fn said(found: &Found, topic: &str) -> u64 {
    found.usual["said"][topic].as_u64().unwrap_or(0)
}

/// Whoever hears what the conductor says of steps and of what is done
/// around them, of anything there is.
const LISTENER: &str = r#"blueprint: listener
description: Hears each step that is passed over, and each thing done around a step that could not be done.
on:
  passed:
    event: step.skipped
    steps:
      skipped: { do: test.mark, with: { key: heard-skipped } }
  undone:
    event: hook.failed
    steps:
      failed: { do: test.mark, with: { key: heard-failed } }
"#;

const FRETS: &str = r#"blueprint: frets
description: Steps passed over alone and beside one at work, and things done around a step, as its steps end and as it stops that cannot be done.
steps:
  before: { when: ${ false }, do: test.mark, with: { key: never } }
  work:
    needs: [before]
    do: test.mark
    with: { key: fret-work }
    on_start:
      - do: test.fails
        with: { message: not as it starts }
      - do: test.mark
        with: { key: fret-started }
    on_success:
      - do: test.fails
        with: { message: not as it goes well }
  slow: { needs: [work], do: test.sleep, with: { for: 1m } }
  beside: { needs: [work], when: ${ false }, do: test.mark, with: { key: never } }
  after: { needs: [slow, beside], when: ${ false }, do: test.mark, with: { key: never } }
on:
  poke: { event: frets.poke, outside: true, do: noop }
on_finish:
  - do: test.fails
    with: { message: not as its steps end }
  - emit: { topic: frets.over }
on_stop:
  - do: test.fails
    with: { message: not as it stops }
  - emit: { topic: frets.gone }
"#;

#[tokio::test]
async fn a_step_passed_over_and_something_around_a_step_that_could_not_be_done_are_heard_once_wherever_the_conductor_is_stopped()
 {
    let telling = Scenario::of("tells", LISTENER)
        .and(then(Does::Apply(FRETS, json!({}))))
        .and(once(|seen: &Seen| seen.said("frets.over") > 0, Does::Stop("frets")))
        .over("2m", "1m");
    let found = anywhere(telling).await;
    assert_eq!(asked(&found, "skipped:test.mark"), 3, "each step passed over was heard: {}", found.usual);
    assert_eq!(asked(&found, "failed:test.mark"), 4, "and each thing that could not be done");
    assert_eq!((said(&found, "frets.over"), said(&found, "frets.gone")), (1, 1));
    found.held();
}

const DROPS: &str = r#"blueprint: drops
description: Something that stays, with a handler whose events take turns by what they are about.
version: 1
steps:
  begin: { do: test.mark, with: { key: drops-began } }
on:
  work:
    event: drops.go
    outside: true
    turn: "${ event.data.row }"
    steps:
      slow: { do: test.sleep, with: { for: 1m } }
      mark: { do: test.mark, with: { key: drops-worked } }
"#;

const DROPPED: &str = r#"blueprint: drops
description: Its next text, which has that handler no longer, and is taken where it is.
version: 2
steps:
  begin: { do: test.mark, with: { key: drops-began-again } }
on:
  poke: { event: drops.poke, outside: true, do: noop }
on_update: []
"#;

#[tokio::test]
async fn a_run_of_a_handler_its_text_no_longer_has_ends_as_a_run_does_wherever_the_conductor_is_stopped() {
    let dropping = Scenario::of("drops", DROPS)
        .and(then(Does::Say("drops.go", json!({ "act": 1, "row": "a" }))))
        .and(once(|seen: &Seen| seen.of_step("step.started", "slow") > 0, Does::Apply(DROPPED, json!({}))))
        .over("6m", "1m")
        .judged(left_behind);
    let found = anywhere(dropping).await;
    // With nothing in its way the run that was in hand is finished
    // beside the text that no longer has its handler.
    assert_eq!(asked(&found, "mark:test.mark"), 1, "{}", found.usual);
    assert_eq!(asked(&found, "begin:test.mark"), 1, "its steps were not run again");
    found.held();
}

/// Something that stays with work under way, and listens for a good
/// deal: a conductor's start has that many places in the journal to
/// look up between keeping the run it finds under way for its handlers
/// and starting them, and what had stopped it needs fewer answers of
/// the database to stop it again. A conductor that is started again in a
/// play has its database answered a turn at a time ([`crate::stepped`]),
/// so that is so on any machine.
const WARD: &str = r#"blueprint: ward
description: Something that stays, with work under way, and many handlers.
steps:
  begin: { do: test.mark, with: { key: ward-began } }
on:
  go:
    event: ward.go
    outside: true
    steps:
      long: { do: test.sleep, with: { for: 10m } }
      after: { do: test.mark, with: { key: ward-went-on } }
  h01: { event: ward.h01, outside: true, do: noop }
  h02: { event: ward.h02, outside: true, do: noop }
  h03: { event: ward.h03, outside: true, do: noop }
  h04: { event: ward.h04, outside: true, do: noop }
  h05: { event: ward.h05, outside: true, do: noop }
  h06: { event: ward.h06, outside: true, do: noop }
  h07: { event: ward.h07, outside: true, do: noop }
  h08: { event: ward.h08, outside: true, do: noop }
"#;

const WARDS: &str = r#"blueprint: wards
description: It starts something that stays, and a handler of it stops that while it is at work.
steps:
  kid: { spawn: ward, as: the-kid }
on:
  end:
    event: wards.end
    outside: true
    steps:
      enough: { do: stop, with: { instance: the-kid } }
      after: { do: test.mark, with: { key: wards-ended } }
"#;

// What a handler stops may be stopped in the moment a conductor's start
// is coming to it: the handler's run is taken up first, since what it
// stops was started after it. The run the start found under way there
// is kept for handlers that are then never started, and ends with what
// it is a run of. It was handed to nobody, and read as under way for good.
#[tokio::test]
async fn a_run_under_way_of_something_stopped_as_the_conductor_comes_to_it_is_ended_wherever_it_is_stopped() {
    let warding = Scenario::of("wards", WARDS)
        .with(&[WARD])
        .and(then(Does::Say("ward.go", json!({ "act": 1 }))))
        .and(once(
            |seen: &Seen| seen.of_step("step.started", "long") > 0,
            Does::Say("wards.end", json!({ "act": 2 })),
        ))
        .over("1m", "1m");
    let found = anywhere(warding.there(ended_by_what_stopped_it)).await;
    let under_way = &found.usual["runs"]["wards[wards] > ward[the-kid]"]["on.go"][0];
    assert_eq!(under_way["status"], "cancelled", "{}", found.usual);
    assert_eq!(asked(&found, "after:test.mark"), 1, "the handler that stopped it went on: {}", found.usual);
    found.held();
    assert!(
        found.noted > 0,
        "wards: in no play was what a handler stops stopped again between a start keeping its run and \
         starting its handlers: the order this play is there for was not come to, and it has held nothing \
         of it. Give `ward` more handlers, or what stops it less to ask before it does"
    );
}

/// Whether, in what a play kept, the run that was under way in what a
/// handler stops was ended by the stopping and by no handler: a
/// conductor was started while it was under way (some other run says it
/// was taken up before this one ended), and it never says so itself.
fn ended_by_what_stopped_it(kept: &[kernel::Event]) -> bool {
    let of_go = |event: &&kernel::Event, topic: &str| event.topic == topic && event.data["flow"] == "on.go";
    let Some(began) = kept.iter().find(|event| of_go(event, "run.started")) else { return false };
    let run = &began.source.run;
    let ended = kept.iter().find(|event| of_go(event, "run.finished") && &event.source.run == run);
    let Some(ended) = ended else { return false };
    let taken_up: Vec<&kernel::Event> = kept
        .iter()
        .filter(|event| event.topic == "run.resumed" && event.seq > began.seq && event.seq < ended.seq)
        .collect();
    !taken_up.is_empty() && taken_up.iter().all(|event| &event.source.run != run)
}

// Something that was being stopped when the conductor went, and does
// nothing as it stops, used to be brought back when one started, with
// the run it had: a run whose step was cancelled, which went on to the
// step after that one and ended well. A run taken up so is cancelled
// still; and a stop that has begun is now finished by the conductor
// that starts, whatever the instance does as it stops, so the run is
// ended with its instance and nothing of it is taken up. Either way it
// comes to what it comes to with nothing in its way.
#[tokio::test]
async fn a_run_whose_step_was_cancelled_is_cancelled_still_when_it_is_taken_up_wherever_the_conductor_is_stopped()
 {
    let halting = Scenario::of("ward", WARD)
        .and(then(Does::Say("ward.go", json!({ "act": 1 }))))
        .and(once(|seen: &Seen| seen.of_step("step.started", "long") > 0, Does::Stop("ward")))
        .over("1m", "1m");
    let found = anywhere(halting).await;
    let under_way = &found.usual["runs"]["ward[ward]"]["on.go"][0];
    assert_eq!(under_way["status"], "cancelled", "{}", found.usual);
    assert_eq!(asked(&found, "after:test.mark"), 0, "what came after the step it was in was not done");
    // And it was stopped where it is there to be: with the step it was
    // in written as cancelled and its run not yet ended.
    let there = |topic: &str| found.places.iter().any(|place| place.topic == topic && place.step == "long");
    assert!(
        there("step.cancelled") && there("step.finished"),
        "the play never passed the step being cancelled: it has held nothing of what it is there for"
    );
    found.held();
}

const KID: &str = "\
blueprint: kid
version: 1
steps:
  begin: { do: test.count, with: { key: kid-began } }
on:
  poke: { event: kid.poke, do: noop }
";

/// Its next text: taken where it is, with a handler for the very event
/// that has its parent bring it the text.
const KID_NEXT: &str = "\
blueprint: kid
version: 2
steps:
  begin: { do: test.count, with: { key: kid-began } }
on:
  poke: { event: kid.poke, do: noop }
  moved:
    event: mover.moved
    do: test.count
    with: { key: kid-heard }
on_update:
  - do: test.count
    with: { key: kid-taken }
";

const MOVER: &str = "\
blueprint: mover
steps:
  begin: { do: test.count, with: { key: mover-began } }
on:
  moved:
    event: mover.moved
    steps:
      kid: { spawn: kid, as: the-kid }
";

/// What the parent hears, said by somebody.
async fn moved(rig: &Rig, n: u64) {
    rig.journal.emit("mover.moved", kernel::Source::actor(user().actor()), json!({ "n": n })).await.unwrap();
}

/// The parent, with what it started for the first thing it heard on its
/// first text, and that text's next put where texts are found.
async fn with_a_kid(rig: &Rig) {
    rig.library.put("kid", KID);
    let applied = rig.apply(MOVER, json!({})).await;
    active(rig, &applied.instance).await;
    moved(rig, 1).await;
    let settled = async {
        loop {
            let filter = conductor::InstanceFilter { live_only: true, ..Default::default() };
            let all = rig.engine.instances(filter).await.unwrap();
            if all.iter().any(|one| one.blueprint == "kid" && one.status == conductor::InstanceStatus::Active)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    };
    rig.soon("what it started to be active", settled).await;
    rig.library.put("kid", KID_NEXT);
}

async fn counted(rig: &Rig, key: &'static str, times: u64) {
    let there = async {
        while rig.shared.count(key) < times {
            tokio::task::yield_now().await;
        }
    };
    rig.soon(key, there).await;
}

// A text that an event brought, through a handler's step, begins the
// handlers it brings just before that event: the handler it brings for
// the event hears it. Its place is written with the text. Written after
// it, a conductor stopped between the two gave it a place from when it
// started, and the event was never heard.
#[tokio::test]
async fn a_handler_a_text_brings_hears_the_event_that_brought_the_text() {
    let rig = Rig::new().await;
    with_a_kid(&rig).await;
    moved(&rig, 2).await;
    counted(&rig, "kid-heard", 1).await;
    assert_eq!((rig.shared.count("kid-taken"), rig.shared.count("kid-began")), (1, 1));
}

#[tokio::test]
async fn and_hears_it_where_the_conductor_stopped_as_the_text_was_written_as_taken() {
    let mut rig = Rig::on_a_file().await;
    with_a_kid(&rig).await;
    // The journal takes nothing after the text is said to be taken: the
    // conductor is gone before anything else of the taking is written.
    rig.journal.shut_after(|event| event.topic == "instance.updated" && event.data["carried"] == true);
    moved(&rig, 2).await;
    rig.soon("the journal to shut", rig.journal.shut()).await;
    assert_eq!(rig.shared.count("kid-heard"), 0);

    rig.stopped_there().await;

    counted(&rig, "kid-heard", 1).await;
    counted(&rig, "kid-taken", 1).await;
    assert_eq!(rig.shared.count("kid-began"), 1, "its steps were not run again");
    // And once: started again with nothing in the way, it is not heard twice.
    rig.restart().await;
    let again = rig
        .apply("blueprint: after\nsteps:\n  go: { do: test.count, with: { key: after } }\n", json!({}))
        .await;
    rig.finished(&again).await;
    assert_eq!((rig.shared.count("kid-heard"), rig.shared.count("kid-taken")), (1, 1));
}
