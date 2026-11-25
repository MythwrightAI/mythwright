//! A text something was waiting to be carried over to, where the
//! conductor stopped while it waited, or as something else came to it.
//!
//! Whoever asked was answered, so what it was to be carried over to is
//! kept with the run that will do it ([`crate::carried`] has `on_update`
//! with nothing stopped, and [`crate::answered`] the plays). Here: it is
//! carried over by a conductor that starts; the same asked again is
//! answered by the run there is; a later asking calls it off, a text
//! taken where the instance is included, stopped or not; and the run
//! ends, once, where a start takes down or stops what it was a run of.

use conductor::{InstanceStatus, RunStatus};
use kernel::Source;
use serde_json::json;

use crate::Rig;
use crate::carried::{ONE, TWO, carried, counts, keyed, poke};
use crate::handlers::{active, handled};

/// The first text with a run of its handler held at a gate the conductor
/// can be stopped at, and the next text asked for while it is held.
async fn waiting(rig: &Rig) -> (String, String, conductor::Applied) {
    let held = ONE.replace(
        "      say: { do: test.count",
        "      wait: { do: test.durable, with: { name: hold } }\n      say: { do: test.count",
    );
    let lane = rig.apply(&held, json!({})).await.instance;
    active(rig, &lane).await;
    poke(rig).await;
    rig.reached("hold").await;
    let asked = rig.apply(TWO, json!({})).await;
    (lane, held, asked)
}

/// Waits until the run that was held at `gate` when the conductor stopped
/// has been taken up there by the one started since.
async fn taken_up(rig: &Rig, gate: &str) {
    let key = format!("taken-up:{gate}");
    let there = async {
        while rig.shared.count(&key) < 1 {
            tokio::task::yield_now().await;
        }
    };
    rig.soon("the run that was under way to be taken up", there).await;
}

// Whoever asked was answered: the run that will carry it over was there.
// So what it is to be carried over to is kept with that run, and a
// conductor that starts carries it over, once what was under way has
// ended on the text it began with. It was left as it was, saying to
// apply again, and a step that had asked and ended never did.
#[tokio::test]
async fn one_that_was_still_waiting_when_the_conductor_stopped_is_carried_over_when_it_starts_again() {
    let mut rig = Rig::new().await;
    let (lane, held, asked) = waiting(&rig).await;

    rig.restart().await;
    // Until the run that was under way is over, it is as it was.
    taken_up(&rig, "hold").await;
    let is = rig.instance(&lane).await;
    assert_eq!(
        (is.revision.as_str(), is.status),
        (blueprint::revision(&held).as_str(), InstanceStatus::Active)
    );
    assert_eq!(counts(&rig, &["old", "carried"]), [0, 0]);
    // That run is carried on, as the text it began with; then the text
    // is taken, by the run whoever asked was given.
    rig.open("hold");
    let run = rig.finished(&asked).await;
    assert_eq!((run.flow.as_str(), run.status), ("on_update", RunStatus::Ok));
    handled(&rig, "on.poke", 1).await;
    let is = rig.instance(&lane).await;
    assert_eq!(
        (is.revision.as_str(), is.status),
        (blueprint::revision(TWO).as_str(), InstanceStatus::Active)
    );
    assert_eq!(is.state["rounds"], 1);
    assert_eq!(counts(&rig, &["old", "carried", "claimed-again"]), [1, 1, 0]);
    assert_eq!(carried(&rig).await.len(), 1);

    // And once: a conductor that starts after that carries nothing.
    rig.restart().await;
    poke(&rig).await;
    handled(&rig, "on.poke", 2).await;
    assert_eq!(counts(&rig, &["new", "carried"]), [1, 1]);
    assert_eq!(carried(&rig).await.len(), 1);
}

// Asked again for the very thing it is waiting to be carried over to,
// as a step taken up before it had kept its answer asks: the run there
// is answers, and nothing is called off.
#[tokio::test]
async fn asked_again_for_what_it_is_waiting_to_be_carried_over_to_the_run_there_is_answers() {
    let mut rig = Rig::new().await;
    let (lane, _, asked) = waiting(&rig).await;
    let again = rig.apply(TWO, json!({})).await;
    assert_eq!(again.run, asked.run);
    assert_eq!(again.took, Some(conductor::Took::Carried));

    // And where the first asking was before the conductor stopped.
    rig.restart().await;
    let after = rig.apply(TWO, json!({})).await;
    assert_eq!(after.run, asked.run);
    taken_up(&rig, "hold").await;
    rig.open("hold");
    let run = rig.finished(&asked).await;
    assert_eq!((run.flow.as_str(), run.status), ("on_update", RunStatus::Ok));
    let carries = rig.engine.runs(&lane, 100).await.unwrap();
    assert_eq!(carries.iter().filter(|run| run.flow == "on_update").count(), 1, "one run carried it over");
    assert_eq!(counts(&rig, &["carried", "claimed-again"]), [1, 0]);
}

// What was kept is called off as it is with nothing stopped: by another
// text asked for, which is the one that stands.
#[tokio::test]
async fn a_later_apply_calls_off_a_text_kept_from_before_the_conductor_stopped() {
    let mut rig = Rig::new().await;
    let (lane, _, first) = waiting(&rig).await;
    rig.restart().await;
    let other = TWO.replace("key: carried", "key: carried-later");
    let second = rig.apply(&other, json!({})).await;
    let called_off = rig.finished(&first).await;
    assert_eq!(called_off.status, RunStatus::Cancelled);
    assert_eq!(
        called_off.error.as_deref(),
        Some("it was applied over, or stopped, before it had been carried over to this text")
    );
    taken_up(&rig, "hold").await;
    rig.open("hold");
    assert_eq!(rig.finished(&second).await.status, RunStatus::Ok);
    assert_eq!(counts(&rig, &["carried", "carried-later"]), [0, 1]);
    assert_eq!(rig.instance(&lane).await.revision, blueprint::revision(&other));

    // Nor is it taken by a conductor that starts after it was called off.
    rig.restart().await;
    poke(&rig).await;
    handled(&rig, "on.poke", 2).await;
    assert_eq!(counts(&rig, &["carried", "carried-later"]), [0, 1]);
    assert_eq!(rig.instance(&lane).await.revision, blueprint::revision(&other));
}

/// The held text's next, which begins as it does and says nothing of
/// how it is taken: one that is running takes it where it is.
fn later(held: &str) -> String {
    held.replace("version: 1", "version: 4").replace("key: old", "key: older")
}

const CALLED_OFF: &str = "it was applied over, or stopped, before it had been carried over to this text";

// A text taken where the instance is is a later asking too, and the one
// that stands: what the instance was waiting to be carried over to is
// let go of with it. It went on waiting, and the earlier text landed
// after the later one.
#[tokio::test]
async fn a_text_taken_where_the_instance_is_calls_off_one_it_was_waiting_to_be_carried_over_to() {
    let rig = Rig::new().await;
    let (lane, held, first) = waiting(&rig).await;
    let other = later(&held);
    let second = rig.apply(&other, json!({})).await;
    assert_eq!(second.took, Some(conductor::Took::InPlace));
    let called_off = rig.finished(&first).await;
    assert_eq!((called_off.status, called_off.error.as_deref()), (RunStatus::Cancelled, Some(CALLED_OFF)));

    rig.open("hold");
    handled(&rig, "on.poke", 1).await;
    // The next thing it hears is handled by the text it took, which
    // waits at the same gate.
    poke(&rig).await;
    rig.open("hold");
    handled(&rig, "on.poke", 2).await;
    assert_eq!(counts(&rig, &["old", "older", "carried", "new"]), [1, 1, 0, 0]);
    assert_eq!(rig.instance(&lane).await.revision, blueprint::revision(&other));
    assert!(carried(&rig).await.is_empty());
}

// And where the conductor stopped in the moment the later text was
// written as taken, before the run that was to carry the instance over
// had ended: what it was to carry is kept no longer, and a conductor
// that starts ends the run, and carries nothing.
#[tokio::test]
async fn and_calls_it_off_where_the_conductor_stopped_as_the_later_text_was_taken() {
    let mut rig = Rig::on_a_file().await;
    let (lane, held, first) = waiting(&rig).await;
    let other = later(&held);
    rig.journal.shut_after(|event| event.topic == "instance.updated");
    let (engine, request) = (rig.engine.clone(), rig.request(&other, json!({})));
    let asking = tokio::spawn(async move { engine.apply(request).await.map(|_| ()) });
    rig.soon("the journal to shut", rig.journal.shut()).await;
    asking.abort();

    rig.stopped_there().await;
    let called_off = rig.finished(&first).await;
    assert_eq!((called_off.status, called_off.error.as_deref()), (RunStatus::Cancelled, Some(CALLED_OFF)));
    taken_up(&rig, "hold").await;
    rig.open("hold");
    handled(&rig, "on.poke", 1).await;
    // The next thing it hears is handled by the text it took, which
    // waits at the same gate.
    poke(&rig).await;
    rig.open("hold");
    handled(&rig, "on.poke", 2).await;
    assert_eq!(counts(&rig, &["old", "older", "carried", "new"]), [1, 1, 0, 0]);
    assert_eq!(rig.instance(&lane).await.revision, blueprint::revision(&other));
    assert!(carried(&rig).await.is_empty());
}

/// Something that starts a lane, and asks for it again as the text then
/// is for each thing it hears.
const HOST: &str = "\
blueprint: host
steps:
  kid: { spawn: lane, as: the-lane }
on:
  moved:
    event: host.moved
    steps:
      again: { spawn: lane, as: the-lane }
";

/// The host with a lane under it whose handler is held in the middle of
/// a run, and the lane's next text asked for by a step of the host's
/// that was answered and is over: `(host, lane, the run that will carry
/// the lane over)`.
async fn host_with_a_lane_waiting(rig: &Rig, host: &str) -> (String, String, String) {
    let held = ONE.replace(
        "      say: { do: test.count",
        "      wait: { do: test.durable, with: { name: hold } }\n      say: { do: test.count",
    );
    rig.library.put("lane", &held);
    let host = rig.apply(host, json!({})).await.instance;
    active(rig, &host).await;
    let lane = keyed(rig, "the-lane").await.id;
    active(rig, &lane).await;
    poke(rig).await;
    rig.reached("hold").await;
    rig.library.put("lane", TWO);
    rig.journal.emit("host.moved", Source::default(), json!({})).await.unwrap();
    let asked = rig.event("run.finished", |event| event.data["flow"] == "on.moved").await;
    assert_eq!(asked.data["status"], "ok", "the step that asked was answered, and is over");
    let runs = rig.engine.runs(&lane, 100).await.unwrap();
    let carry = runs.iter().find(|run| run.flow == "on_update").expect("the run that will carry it over");
    assert_eq!(carry.status, RunStatus::Running);
    (host, lane, carry.id.clone())
}

/// How often it was said that `run` finished.
async fn finished_times(rig: &Rig, run: &str) -> usize {
    let said = rig.events("run.finished").await;
    said.iter().filter(|event| event.source.run.as_deref() == Some(run)).count()
}

// A conductor's start says of everything alive that it has not come to
// it yet, and what waits to carry something over waits for that. Every
// way a start leaves an instance comes to it. One whose parent cannot be
// begun is taken down with the parent before the start comes to it: the
// run that was to carry it over ends, once, and nothing is carried.
#[tokio::test]
async fn one_owed_a_text_whose_parent_cannot_be_begun_at_a_start_has_the_run_that_was_to_carry_it_end() {
    let mut rig = Rig::new().await;
    rig.library.put("gear", "blueprint: gear\non:\n  turned: { event: gear.turn, do: noop }\n");
    let attaching = HOST.replace("steps:\n  kid", "uses:\n  g: gear\nsteps:\n  kid");
    let (host, lane, carry) = host_with_a_lane_waiting(&rig, &attaching).await;

    // As a conductor stopped between writing the host down and its first
    // run left it, but that the lane is there: starting, with no run, and
    // what it attaches not there. And the text it attaches reads no
    // longer, so that it cannot be begun.
    let id = host.clone();
    rig.journal
        .db()
        .call(move |db| {
            db.execute("DELETE FROM steps WHERE run IN (SELECT id FROM runs WHERE instance = ?1)", [&id])?;
            db.execute("DELETE FROM runs WHERE instance = ?1", [&id])?;
            db.execute(
                "UPDATE instances SET status = 'stopped' WHERE parent = ?1 AND blueprint = 'gear'",
                [&id],
            )?;
            db.execute("UPDATE instances SET status = 'starting' WHERE id = ?1", [&id])
        })
        .await
        .unwrap();
    rig.library.put("gear", "blueprint: gear\nsteps: [\n");

    rig.restart().await;
    let ended = rig.soon("the run to finish", rig.engine.wait_run(&carry)).await.unwrap();
    assert_eq!((ended.status, ended.error.as_deref()), (RunStatus::Cancelled, Some(CALLED_OFF)));
    assert_eq!(rig.instance(&host).await.status, InstanceStatus::Failed);
    assert_eq!(rig.instance(&lane).await.status, InstanceStatus::Stopped);
    assert_eq!(counts(&rig, &["carried", "claimed-again"]), [0, 0]);
    // And once, whatever starts after.
    rig.restart().await;
    assert_eq!(finished_times(&rig, &carry).await, 1);
    assert!(carried(&rig).await.is_empty());
}

// One that is stopped by the start itself, before the start goes over
// what is alive: what started it keeps nothing, and is over with what it
// started. The run that was to carry it over ends with it, once.
#[tokio::test]
async fn one_owed_a_text_that_a_start_stops_has_the_run_that_was_to_carry_it_end() {
    let mut rig = Rig::new().await;
    let keeping_nothing = HOST.replace("blueprint: host\n", "blueprint: host\ndurable: false\n");
    let (host, lane, carry) = host_with_a_lane_waiting(&rig, &keeping_nothing).await;

    rig.restart().await;
    let ended = rig.soon("the run to finish", rig.engine.wait_run(&carry)).await.unwrap();
    assert_eq!(ended.status, RunStatus::Cancelled);
    assert_eq!(rig.instance(&host).await.status, InstanceStatus::Stopped);
    assert_eq!(rig.instance(&lane).await.status, InstanceStatus::Stopped);
    assert_eq!(counts(&rig, &["carried", "claimed-again"]), [0, 0]);
    rig.restart().await;
    assert_eq!(finished_times(&rig, &carry).await, 1);
    assert!(carried(&rig).await.is_empty());
}

// A run that a program from before this made has nothing kept of what
// it was to carry its instance over to: it ends as it always did.
#[tokio::test]
async fn one_a_program_from_before_left_waiting_is_left_as_it_was_and_says_to_apply_again() {
    let mut rig = Rig::new().await;
    let (lane, held, asked) = waiting(&rig).await;
    rig.journal.db().call(|db| db.execute("UPDATE runs SET carries = NULL", [])).await.unwrap();

    rig.restart().await;
    let run = rig.finished(&asked).await;
    assert_eq!(run.status, RunStatus::Cancelled);
    assert_eq!(
        run.error.as_deref(),
        Some("the conductor stopped before it had been carried over to this text; apply it again")
    );
    let is = rig.instance(&lane).await;
    assert_eq!(
        (is.revision.as_str(), is.status),
        (blueprint::revision(&held).as_str(), InstanceStatus::Active)
    );
    // The run that was under way is carried on, as the text it began with.
    rig.open("hold");
    handled(&rig, "on.poke", 1).await;
    assert_eq!(counts(&rig, &["old", "carried"]), [1, 0]);

    // Applied again, it is carried over.
    let run = rig.finished(&rig.apply(TWO, json!({})).await).await;
    assert_eq!((run.flow.as_str(), run.status), ("on_update", RunStatus::Ok));
    assert_eq!(rig.instance(&lane).await.state["rounds"], 1);
}
