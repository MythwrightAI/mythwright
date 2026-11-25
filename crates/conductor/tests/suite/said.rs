//! An instance is said to be active once, wherever the conductor was
//! stopped as its steps ended.
//!
//! That it is active is written down, and then said. A conductor stopped
//! between the two comes back to an instance that is active and never
//! said so: what it begins when it hears that of itself was never begun.

use serde_json::json;

use crate::Rig;

/// Something that stays, and begins what it is for when it is active.
const KEPT: &str = "\
blueprint: kept
steps:
  begin: { do: test.count, with: { key: begun } }
on:
  first:
    event: instance.active
    scope: own
    do: test.count
    with: { key: heard }
";

#[tokio::test]
async fn an_instance_whose_steps_ended_as_the_conductor_stopped_is_said_to_be_active_when_it_is_back() {
    let mut rig = Rig::on_a_file().await;
    // The journal takes nothing after the steps' run is said to have
    // finished: the conductor is gone before it says the instance is active.
    rig.journal.shut_after(|event| event.topic == "run.finished" && event.data["flow"] == "main");
    let applied = rig.engine.apply(rig.request(KEPT, json!({}))).await.unwrap();
    rig.soon("the journal to shut", rig.journal.shut()).await;
    assert_eq!(rig.shared.count("heard"), 0);

    rig.stopped_there().await;

    let its = applied.instance.clone();
    rig.event("instance.active", move |event| event.source.instance.as_deref() == Some(its.as_str())).await;
    let heard = async {
        while rig.shared.count("heard") < 1 {
            tokio::task::yield_now().await;
        }
    };
    rig.soon("what it begins when it is active to have begun", heard).await;
    // Its steps were not done again for that.
    assert_eq!(rig.shared.count("begun"), 1);

    // And once: started again with nothing in the way, it is not said twice.
    rig.restart().await;
    let again = rig
        .apply("blueprint: after\nsteps:\n  go: { do: test.count, with: { key: after } }\n", json!({}))
        .await;
    rig.finished(&again).await;
    assert_eq!(
        rig.events("instance.active")
            .await
            .iter()
            .filter(|event| event.source.instance == Some(applied.instance.clone()))
            .count(),
        1
    );
    assert_eq!(rig.shared.count("heard"), 1);
}
