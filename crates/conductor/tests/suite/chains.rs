//! Events that each come of handling the one before are a chain, and one
//! that grows without end, faster than anything that does work could, is
//! stopped and said: two instances that answer each other, or a pool that
//! starts a worker whenever one finishes when its workers finish at once.
//! Many events from one cause are not a chain, and neither is one that
//! takes its time.

use kernel::Source;
use serde_json::json;

use crate::Rig;
use crate::handlers::active;

const PING: &str = "\
blueprint: ping
on:
  answer: { event: pong.said, do: emit, with: { topic: ping.said } }
";

const PONG: &str = "\
blueprint: pong
on:
  answer: { event: ping.said, do: emit, with: { topic: pong.said } }
";

async fn both(rig: &Rig, pong: &str) {
    for text in [PING, pong] {
        let applied = rig.apply(text, json!({})).await;
        active(rig, &applied.instance).await;
    }
}

/// Waits until `count` events on `topic` have happened.
async fn count(rig: &Rig, topic: &str, count: usize) {
    let mut follow = rig.follow(topic);
    rig.soon(&format!("{count} events on `{topic}`"), async {
        for _ in 0..count {
            follow.next().await.unwrap();
        }
    })
    .await;
}

#[tokio::test]
async fn two_instances_that_answer_each_other_without_end_are_stopped_and_it_is_said() {
    let rig = Rig::new().await;
    both(&rig, PONG).await;
    rig.journal.emit("ping.said", Source::default(), json!({})).await.unwrap();

    let stopped = rig.event("handler.runaway", |_| true).await;
    assert_eq!(stopped.data["depth"], 100, "{}", stopped.data);
    let why = stopped.data["error"].as_str().unwrap();
    assert!(
        why.contains("is the 100th in a chain of events each said while the one before was being handled")
            && why.contains("It was not handled"),
        "{why}"
    );
    // The chain ended there: a hundred answers to the one from outside.
    let said = rig.events("ping.said").await.len() + rig.events("pong.said").await.len();
    assert_eq!(said, 101);

    // Neither has failed or stopped, and what comes from outside next is
    // answered as the first was.
    rig.journal.emit("ping.said", Source::default(), json!({})).await.unwrap();
    count(&rig, "handler.runaway", 2).await;
    let said = rig.events("ping.said").await.len() + rig.events("pong.said").await.len();
    assert_eq!(said, 202);
}

#[tokio::test]
async fn a_chain_that_takes_its_time_is_not_stopped() {
    let rig = Rig::new().await;
    // One of the two does something that takes a while before it answers.
    let pong = "\
blueprint: pong
on:
  answer:
    event: ping.said
    steps:
      work: { do: wait, with: { for: 11s } }
      say: { do: emit, with: { topic: pong.said } }
";
    both(&rig, pong).await;
    let since = rig.clock.sleep_count();
    rig.journal.emit("ping.said", Source::default(), json!({})).await.unwrap();
    for round in 1..=130 {
        rig.advance_after(since, round, "11s").await;
    }
    count(&rig, "pong.said", 130).await;
    assert!(rig.events("handler.runaway").await.is_empty(), "260 events, none of them refused");
}

#[tokio::test]
async fn many_events_from_one_cause_are_not_a_chain() {
    let rig = Rig::new().await;
    let fan = "\
blueprint: fan
params:
  jobs: { type: list }
on:
  go:
    event: fan.go
    steps:
      each: { for_each: ${ params.jobs }, do: emit, with: { topic: fan.job, n: ${ item } } }
";
    let worker = "\
blueprint: worker
on:
  job: { event: fan.job, do: emit, with: { topic: fan.done, n: ${ event.data.n } } }
";
    let jobs: Vec<u32> = (0..250).collect();
    let applied = rig.apply(fan, json!({ "jobs": jobs })).await;
    active(&rig, &applied.instance).await;
    let applied = rig.apply(worker, json!({})).await;
    active(&rig, &applied.instance).await;

    rig.journal.emit("fan.go", Source::default(), json!({})).await.unwrap();
    count(&rig, "fan.done", 250).await;
    assert!(rig.events("handler.runaway").await.is_empty());
}

#[tokio::test]
async fn a_pool_whose_workers_finish_at_once_does_not_start_them_for_ever() {
    let rig = Rig::new().await;
    rig.library
        .put("worker", "blueprint: worker\nsteps:\n  work: { do: test.count, with: { key: worked } }\n");
    // Whenever a worker finishes, another is started.
    let pool = "\
blueprint: pool
on:
  begin:
    event: pool.go
    steps:
      first: { spawn: worker, as: first }
  next:
    event: instance.finished
    scope: own
    steps:
      more: { spawn: worker, as: \"after-${ event.seq }\" }
";
    let applied = rig.apply(pool, json!({})).await;
    active(&rig, &applied.instance).await;
    rig.journal.emit("pool.go", Source::default(), json!({})).await.unwrap();

    let stopped = rig.event("handler.runaway", |_| true).await;
    assert_eq!(stopped.data["handler"], "next");
    assert_eq!(stopped.source.instance.as_deref(), Some(applied.instance.as_str()));
    let worked = rig.shared.count("worked");
    assert!((99..=101).contains(&worked), "about a hundred workers and no more: {worked}");
}
