//! A rollout through `std/slot`: a new version is given a growing share of
//! the connections and watched at each stage. What it was sent is all a
//! stage has to judge it by, so one that was sent nothing has shown
//! nothing, and the slot says so instead of taking silence for evidence.
//!
//! The versions here are services whose programs do nothing; what answers
//! at each one's port is a listener of the test's, so that the test is the
//! traffic and knows where every connection went.

use std::time::Duration;

use conductor::{Applied, RunStatus};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::{Rig, soon};

const SLOT: &str = include_str!("../../../../blueprints/std/slot.bp");
const STAGE: &str = include_str!("../../../../blueprints/std/canary-stage.bp");

/// Something that answers every connection to its port with its name: the port.
async fn version(name: &'static str) -> u64 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let _ = stream.write_all(name.as_bytes()).await;
        }
    });
    u64::from(port)
}

/// Who answered each of `count` connections to the slot's address.
async fn ask(port: u64, count: usize) -> String {
    let mut answers = String::new();
    for _ in 0..count {
        let mut stream = TcpStream::connect(("127.0.0.1", port as u16)).await.unwrap();
        let mut answer = String::new();
        soon("an answer through the slot", stream.read_to_string(&mut answer)).await.unwrap();
        answers.push_str(&answer);
    }
    answers
}

/// A rollout under way: what was applied, and where the journal had got
/// to when it began, so that its stages are told from those of one before.
struct Rollout {
    applied: Applied,
    began: u64,
}

/// A slot called `web` whose version answers at `at`: applied, and applied
/// again as often as a new version is rolled out.
async fn roll_out(rig: &Rig, at: u64, more: Value) -> Rollout {
    let mut params = json!({
        "name": "web",
        "service": { "cmd": [crate::STANDIN, "idle"], "port": at, "restart": "never" },
        "weights": [50, 100],
        "observe": "20s",
    });
    for (key, value) in more.as_object().unwrap() {
        params[key] = value.clone();
    }
    let began = rig.journal.last_seq();
    Rollout { applied: rig.apply(SLOT, params).await, began }
}

/// Waits until the stage of a rollout that gives the new version `weight`
/// is watching it: where the journal had got to when the stage said so.
async fn watching(rig: &Rig, rollout: &Rollout, weight: u64) -> u64 {
    let began = rollout.began;
    let shifted =
        rig.event("slot.shifted", move |event| event.seq > began && event.data["weight"] == weight).await.seq;
    // The watch is parked on the clock, having taken its count of what the
    // new version had been sent so far: what is sent from here is what it
    // sees. (Nothing else here waits on the clock: the versions are
    // programs that do nothing and are not looked in on.)
    soon("the stage to start watching", async {
        loop {
            let started = rig.clock.sleep_count();
            if rig.clock.sleeping() > 0 {
                return;
            }
            rig.clock.sleeps_started(started + 1).await;
        }
    })
    .await;
    shifted
}

/// The twenty seconds of the stage that said it was watching at `shifted`
/// pass: how the stage ended.
async fn watched(rig: &Rig, shifted: u64) -> Value {
    rig.clock.advance(Duration::from_secs(20));
    let ended = rig.event("run.finished", move |event| {
        event.seq > shifted && event.data["blueprint"] == "std/canary-stage"
    });
    ended.await.data["status"].clone()
}

/// Lets the stage of a rollout that gives the new version `weight` run
/// its course, with `connections` made to the slot while it is watched.
/// Who answered them.
async fn stage(rig: &Rig, port: u64, rollout: &Rollout, weight: u64, connections: usize) -> String {
    let shifted = watching(rig, rollout, weight).await;
    let answers = ask(port, connections).await;
    watched(rig, shifted).await;
    answers
}

async fn rig() -> (Rig, u64) {
    let rig = Rig::new().await;
    rig.library.put("std/canary-stage", STAGE);
    // The first version: nothing to compare it with, so no stages.
    let first = roll_out(&rig, version("a").await, json!({})).await;
    let run = rig.finished(&first.applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(run.outputs["unwatched"], 0);
    let port = run.outputs["port"].as_u64().unwrap();
    assert_eq!(ask(port, 2).await, "aa");
    (rig, port)
}

#[tokio::test]
async fn a_new_version_is_judged_by_the_connections_it_was_sent_and_takes_over_when_they_went_well() {
    let (rig, port) = rig().await;
    let second = roll_out(&rig, version("b").await, json!({})).await;
    // Half of what comes in, then all of it.
    let halves = stage(&rig, port, &second, 50, 4).await;
    assert_eq!((halves.matches('a').count(), halves.matches('b').count()), (2, 2), "{halves}");
    assert_eq!(stage(&rig, port, &second, 100, 3).await, "bbb");

    let run = rig.finished(&second.applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    let promoted = rig.event("slot.promoted", |event| event.data["generation"] == 2).await;
    assert_eq!((&promoted.data["stages"], &promoted.data["unwatched"]), (&json!(2), &json!(0)));
    assert_eq!(run.outputs["unwatched"], 0);
    assert!(rig.events("slot.unwatched").await.is_empty(), "it was watched, and that is evidence");
    assert_eq!(ask(port, 2).await, "bb");
}

#[tokio::test]
async fn a_new_version_that_was_sent_nothing_is_promoted_and_the_slot_says_nothing_was_learnt_of_it() {
    let (rig, port) = rig().await;
    let second = roll_out(&rig, version("b").await, json!({})).await;
    // Nobody connects while it is rolled out.
    stage(&rig, port, &second, 50, 0).await;
    stage(&rig, port, &second, 100, 0).await;

    // A service nobody is using yet has to be rolled out too: it takes over.
    let run = rig.finished(&second.applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(ask(port, 2).await, "bb");
    // And it is said, there to be heard and in what the slot gives out,
    // that its not having been rolled back says nothing.
    let promoted = rig.event("slot.promoted", |event| event.data["generation"] == 2).await;
    assert_eq!((&promoted.data["stages"], &promoted.data["unwatched"]), (&json!(2), &json!(2)));
    let unwatched = rig.events("slot.unwatched").await;
    assert_eq!(unwatched.len(), 1);
    assert_eq!(unwatched[0].data, json!({ "slot": "web", "service": "web-g2", "stages": 2 }));
    assert_eq!(run.outputs["unwatched"], 2);

    // Watched through one stage of two, it is not said to be unwatched:
    // the count says how much was seen.
    let third = roll_out(&rig, version("c").await, json!({})).await;
    stage(&rig, port, &third, 50, 0).await;
    assert_eq!(stage(&rig, port, &third, 100, 2).await, "cc");
    let run = rig.finished(&third.applied).await;
    assert_eq!(run.outputs["unwatched"], 1);
    assert_eq!(rig.events("slot.unwatched").await.len(), 1, "no second one");
}

#[tokio::test]
async fn told_how_many_connections_a_stage_must_see_a_slot_rolls_back_a_version_that_was_sent_fewer() {
    let (rig, port) = rig().await;
    let second = roll_out(&rig, version("b").await, json!({ "min_connections": 2 })).await;
    // One connection reaches the new version in its first stage, of two
    // that came in: not enough to go by.
    let halves = stage(&rig, port, &second, 50, 2).await;
    assert_eq!(halves.matches('b').count(), 1, "{halves}");

    let run = rig.finished(&second.applied).await;
    assert_eq!(run.status, RunStatus::Failed);
    let back = rig.event("slot.rolled_back", |_| true).await;
    let why = back.data["reason"].as_str().unwrap();
    assert!(why.ends_with("it received 1 connections; at least 2 were expected"), "{why}");
    // The version that was working has everything again, and the new one is gone.
    assert_eq!(ask(port, 3).await, "aaa");
    assert!(rig.events("slot.promoted").await.iter().all(|event| event.data["generation"] == 1));
    assert!(rig.events("slot.unwatched").await.is_empty());

    // Sent enough, the same version passes.
    let again = roll_out(&rig, version("b").await, json!({ "min_connections": 2 })).await;
    let halves = stage(&rig, port, &again, 50, 4).await;
    assert_eq!(halves.matches('b').count(), 2, "{halves}");
    assert_eq!(stage(&rig, port, &again, 100, 2).await, "bb");
    assert_eq!(rig.finished(&again.applied).await.status, RunStatus::Ok);
}

#[tokio::test]
async fn a_new_version_whose_process_ended_while_it_was_watched_is_rolled_back_though_it_refused_nothing() {
    let (rig, port) = rig().await;
    // A version that is started again when it ends, an hour later: for the
    // whole of this test its process has ended and its restart is to come.
    // Its program waits to be told, and then ends badly.
    let hold = crate::standin::Hold::new().await;
    let at = version("b").await;
    let waits = [crate::STANDIN, "held", &hold.address(), "web-g2", "exit", "1"];
    let service = json!({ "cmd": waits, "port": at, "restart_backoff": "1h" });
    let second = roll_out(&rig, at, json!({ "service": service })).await;
    let shifted = watching(&rig, &second, 50).await;
    // Everything it is sent while it is watched is answered,
    let halves = ask(port, 4).await;
    assert_eq!((halves.matches('a').count(), halves.matches('b').count()), (2, 2), "{halves}");
    // and then its process is gone, with the watch still on it. (What
    // answers at its port is the test's, so nothing is refused after.)
    let pid = rig.hosting.services().into_iter().find(|service| service.name == "web-g2").unwrap().pid;
    let held = hold.came().await;
    if cfg!(unix) {
        // The service leads its own group, which is ended without being
        // asked, as the factory ends one.
        hosting::group::stop(pid, true).unwrap();
        drop(held);
    } else {
        // Where a group is ended by letting go of it (Windows), nothing
        // tells a program ended so from one that ended well by itself,
        // and such a one is not started again and is sent nothing more:
        // there it is told to end, and ends badly.
        held.go().await;
    }
    rig.event("service.exited", |event| event.data["service"] == "web-g2").await;
    assert_eq!(ask(port, 2).await.matches('b').count(), 1, "still answered at its port");

    // Not one connection failed, it has not been started again, and it
    // was healthy when last looked at: it did not pass all the same.
    assert_eq!(watched(&rig, shifted).await, "failed");
    let run = rig.finished(&second.applied).await;
    assert_eq!(run.status, RunStatus::Failed);
    let back = rig.event("slot.rolled_back", |_| true).await;
    let why = back.data["reason"].as_str().unwrap();
    assert!(why.ends_with("`web-g2` did not pass observation: it exited while being watched"), "{why}");
    assert_eq!(rig.events("slot.shifted").await.len(), 1, "it was given no greater share");
    assert_eq!(ask(port, 3).await, "aaa");
}
