//! What becomes of an instance as its steps end is decided in the write
//! that says so, and not before it.
//!
//! Whoever works out what an instance becomes looks first whether its
//! run still speaks for it, and then takes a while: what an update that
//! failed had started is handed back, what it has under it is counted.
//! An apply or a stop that comes in that while has the instance in hand
//! from there. Decided before the wait, an update that failed went back
//! over what had been applied since, and was said to have failed where
//! the instance had been stopped and nothing went back.
//!
//! The while is held open here by what hands back what an update had
//! come to hold: a stop hook of the rig's that waits to be let go.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use conductor::{InstanceStatus, RunStatus};
use serde_json::json;
use tokio::sync::Notify;

use crate::Rig;

/// Something that stays; handed `broken`, its steps come to hold
/// something and then fail.
const RELEASE: &str = "\
blueprint: release
params: { version: 1, broken: false }
steps:
  hold: { when: ${ params.broken }, do: test.hold, with: { what: \"service:new\" } }
  boom: { when: ${ params.broken }, do: test.fails, with: { message: bad release } }
on:
  ping: { event: release.ping, do: test.count, with: { key: pings } }
";

/// What waits where a failed update hands back what it had come to hold.
#[derive(Default)]
struct Held {
    parked: AtomicBool,
    reached: Notify,
    go: Notify,
}

/// A rig whose first handing back waits until it is let go, and says
/// when it has come to that.
async fn rig() -> (Rig, Arc<Held>) {
    let rig = Rig::new().await;
    let held = Arc::new(Held::default());
    rig.engine.on_stop({
        let held = held.clone();
        Arc::new(move |_| {
            let held = held.clone();
            Box::pin(async move {
                if !held.parked.swap(true, Ordering::SeqCst) {
                    held.reached.notify_one();
                    held.go.notified().await;
                }
            })
        })
    });
    (rig, held)
}

async fn said_of(rig: &Rig, topic: &str, id: &str) -> usize {
    let said = rig.events(topic).await;
    said.iter().filter(|event| event.source.instance.as_deref() == Some(id)).count()
}

/// The first version settled, and an update of it that fails and is
/// handing back what it had come to hold: the instance, and that update.
async fn handing_back(rig: &Rig, held: &Held) -> (String, conductor::Applied) {
    let first = rig.apply(RELEASE, json!({})).await;
    assert_eq!(rig.finished(&first).await.status, RunStatus::Ok);
    let second = rig.apply(RELEASE, json!({ "version": 2, "broken": true })).await;
    rig.soon("the update that failed to be handing back what it had come to hold", held.reached.notified())
        .await;
    (first.instance, second)
}

#[tokio::test]
async fn an_instance_stopped_while_an_update_of_it_that_failed_was_handing_back_is_stopped_and_no_more() {
    let (rig, held) = rig().await;
    let (id, failing) = handing_back(&rig, &held).await;
    // Stopped in that while: whoever stops it has it from here.
    rig.engine.stop(&id).await.unwrap();
    held.go.notify_one();
    assert_eq!(rig.finished(&failing).await.status, RunStatus::Failed);

    let instance = rig.instance(&id).await;
    assert_eq!(instance.status, InstanceStatus::Stopped);
    assert_eq!(instance.error, None, "it is stopped, and says nothing of an update");
    assert_eq!(
        said_of(&rig, "instance.update_failed", &id).await,
        0,
        "nothing went back: it was stopped, and is not said to have gone back to anything"
    );
    assert_eq!(said_of(&rig, "instance.finished", &id).await, 1, "it is over, once");
    assert_eq!(said_of(&rig, "instance.stopped", &id).await, 1);
}

#[tokio::test]
async fn what_is_applied_while_an_update_that_failed_was_handing_back_is_what_the_instance_then_runs() {
    let (rig, held) = rig().await;
    let (id, failing) = handing_back(&rig, &held).await;
    // Applied over in that while, with something that works.
    let third = rig.apply(RELEASE, json!({ "version": 3 })).await;
    assert_eq!(third.instance, id);
    assert_eq!(rig.finished(&third).await.status, RunStatus::Ok);
    held.go.notify_one();
    assert_eq!(rig.finished(&failing).await.status, RunStatus::Failed);

    // The update that failed says nothing more of the instance: it runs
    // what was applied since, and did not go back over it.
    let instance = rig.instance(&id).await;
    assert_eq!(instance.status, InstanceStatus::Active);
    assert_eq!(instance.params["version"], 3, "what was applied last is what it runs");
    assert_eq!(instance.error, None);
    assert_eq!(said_of(&rig, "instance.update_failed", &id).await, 0);
    assert_eq!(
        instance.previous.map(|previous| previous.params["version"].clone()),
        None,
        "and it is settled"
    );

    // And one after it that fails goes back to that, as it would have.
    let fourth = rig.apply(RELEASE, json!({ "version": 4, "broken": true })).await;
    assert_eq!(rig.finished(&fourth).await.status, RunStatus::Failed);
    let instance = rig.instance(&id).await;
    assert_eq!((instance.status, &instance.params["version"]), (InstanceStatus::Active, &json!(3)));
    assert_eq!(said_of(&rig, "instance.update_failed", &id).await, 1);
}
