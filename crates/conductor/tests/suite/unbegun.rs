//! An instance that cannot be begun has failed and says so, and never
//! keeps a conductor from starting; and a `stop` that is done again goes
//! by what was stopped since its step began.

use conductor::{InstanceFilter, InstanceStatus, RunStatus};
use kernel::Source;
use serde_json::json;

use crate::{Rig, step};

const GEAR: &str = "\
blueprint: gear
on:
  turned: { event: gear.turn, do: test.count, with: { key: turned } }
";

const HOST: &str = "\
blueprint: host
uses:
  g: gear
on:
  go: { event: host.go, do: test.count, with: { key: went } }
";

const BESIDE: &str = "\
blueprint: beside
on:
  go: { event: beside.go, do: test.count, with: { key: beside } }
";

async fn active(rig: &Rig, id: &str) {
    rig.event("instance.active", |event| event.source.instance.as_deref() == Some(id)).await;
}

async fn said_of(rig: &Rig, topic: &str, id: &str) -> usize {
    let said = rig.events(topic).await;
    said.iter().filter(|event| event.source.instance.as_deref() == Some(id)).count()
}

#[tokio::test]
async fn a_conductor_starts_though_what_one_instance_attaches_cannot_be_brought_up() {
    let mut rig = Rig::new().await;
    rig.library.put("gear", GEAR);
    let host = rig.apply(HOST, json!({})).await;
    let beside = rig.apply(BESIDE, json!({})).await;
    active(&rig, &host.instance).await;
    active(&rig, &beside.instance).await;

    // As a conductor stopped between writing the host down and its first
    // run left it: said to be starting, with no run, and what it attaches
    // not there. And the text it attaches reads no longer.
    let id = host.instance.clone();
    rig.journal
        .db()
        .call(move |db| {
            db.execute("DELETE FROM steps WHERE run IN (SELECT id FROM runs WHERE instance = ?1)", [&id])?;
            db.execute("DELETE FROM runs WHERE instance = ?1", [&id])?;
            db.execute("UPDATE instances SET status = 'stopped' WHERE parent = ?1", [&id])?;
            db.execute("UPDATE instances SET status = 'starting' WHERE id = ?1", [&id])
        })
        .await
        .unwrap();
    rig.library.put("gear", "blueprint: gear\nsteps: [\n");

    // It starts: `restart` fails the test where the conductor's start fails.
    rig.restart().await;
    rig.event("instance.finished", |event| event.source.instance.as_deref() == Some(host.instance.as_str()))
        .await;
    let left = rig.instance(&host.instance).await;
    assert_eq!(left.status, InstanceStatus::Failed);
    let why = left.error.unwrap_or_default();
    assert!(why.starts_with("could not be begun: "), "it says why: {why}");
    assert_eq!(said_of(&rig, "instance.failed", &host.instance).await, 1);

    // What was running beside it is taken up as ever.
    rig.journal.emit("beside.go", Source::default(), json!({})).await.unwrap();
    rig.event("run.finished", |event| {
        event.source.instance.as_deref() == Some(beside.instance.as_str()) && event.data["flow"] == "on.go"
    })
    .await;
    assert_eq!(rig.shared.count("beside"), 1);

    // And the next start has nothing left to meet.
    rig.restart().await;
    assert_eq!(said_of(&rig, "instance.failed", &host.instance).await, 1, "said once");
    assert_eq!(said_of(&rig, "instance.finished", &host.instance).await, 1);
}

#[tokio::test]
async fn an_apply_that_cannot_bring_up_what_it_attaches_leaves_nothing_said_to_be_starting() {
    let rig = Rig::new().await;
    // Handed something the text attached has no param for.
    rig.library.put("gear", GEAR);
    let asked =
        rig.request("blueprint: host\nuses:\n  g: { blueprint: gear, with: { teeth: 12 } }\n", json!({}));
    let refused = rig.engine.apply(asked).await;
    assert!(refused.is_err(), "whoever applied it is told");
    let filter = InstanceFilter { limit: 100, ..Default::default() };
    for one in rig.engine.instances(filter).await.unwrap() {
        assert!(
            !one.status.is_live(),
            "`{}` was left {:?}, with nothing to begin it",
            one.blueprint,
            one.status
        );
        assert_eq!(said_of(&rig, "instance.finished", &one.id).await, 1, "and it said that it is over");
    }
}

const COG: &str = "\
blueprint: cog
params:
  teeth: { type: int, default: 1 }
on:
  turned: { event: cog.turn, do: test.count, with: { key: turned } }
";

const OVER: &str = "\
blueprint: host
params:
  raw: { type: string, default: not json }
uses:
  g: { blueprint: cog, with: { teeth: \"${ from_json(params.raw).teeth }\" } }
steps:
  begun: { do: test.count, with: { key: begun-over } }
on:
  go: { event: host.go, do: test.count, with: { key: went } }
";

#[tokio::test]
async fn an_instance_applied_over_that_cannot_be_begun_goes_back_to_what_it_had_and_stays_up() {
    let mut rig = Rig::new().await;
    rig.library.put("cog", COG);
    let host = rig.apply(&HOST.replace("gear", "cog"), json!({})).await;
    active(&rig, &host.instance).await;
    let before = rig.instance(&host.instance).await;
    let filter = || InstanceFilter { parent: Some(host.instance.clone()), limit: 100, ..Default::default() };
    let had = rig.engine.instances(filter()).await.unwrap();
    assert_eq!(had.len(), 1, "what it attaches was brought up");

    // Applied over with a text that hands what it attaches a param it has
    // not got: whoever applied it is told, and it goes on as it was.
    let refused = rig.engine.apply(rig.request(OVER, json!({}))).await;
    let why = refused.err().map(|err| err.message).unwrap_or_default();
    assert!(why.contains("not JSON"), "whoever applied it is told why what it attaches could not be: {why}");
    let holds = |rig: &Rig| {
        let (id, revision, gear) = (host.instance.clone(), before.revision.clone(), had[0].id.clone());
        let engine = rig.engine.clone();
        async move {
            let now = engine.instance(&id).await.unwrap();
            assert_eq!(now.status, InstanceStatus::Active, "it stays up: {:?}", now.error);
            assert_eq!(now.revision, revision, "on the text it had");
            let gear = engine.instance(&gear).await.unwrap();
            assert!(gear.status.is_live(), "and what it had running is left running");
        }
    };
    holds(&rig).await;
    assert_eq!(said_of(&rig, "instance.failed", &host.instance).await, 0);

    // And a conductor that starts finds nothing to fail or to take down.
    rig.restart().await;
    holds(&rig).await;
    assert_eq!(said_of(&rig, "instance.failed", &host.instance).await, 0);
    rig.journal.emit("host.go", Source::default(), json!({})).await.unwrap();
    rig.event("run.finished", |event| {
        event.source.instance.as_deref() == Some(host.instance.as_str()) && event.data["flow"] == "on.go"
    })
    .await;
    assert_eq!(rig.shared.count("went"), 1, "its handler hears as it did");
}

const STOPPER: &str = "\
blueprint: stopper
steps:
  kid: { spawn: child, as: the-kid }
  first: { do: stop, with: { instance: the-kid }, needs: [kid] }
  again:
    do: stop
    with: { instance: the-kid }
    needs: [first]
    on_start:
      - { do: test.gate, with: { name: hold } }
";

#[tokio::test]
async fn a_stop_done_again_does_not_count_what_was_stopped_before_its_step_began() {
    let mut rig = Rig::new().await;
    rig.library
        .put("child", "blueprint: child\non:\n  go: { event: child.go, do: test.count, with: { key: c } }\n");
    let applied = rig.apply(STOPPER, json!({})).await;
    // `first` stopped the child; `again` has begun and stopped nothing
    // yet. The conductor is stopped there.
    rig.reached("hold").await;
    rig.restart().await;
    rig.open("hold");
    rig.open("hold");
    let run = rig.finished(&applied).await;
    // As with nothing in its way: nothing alive is called that, and the
    // one that was is not what this step stopped.
    assert_eq!(run.status, RunStatus::Failed);
    let error = step(&run, "again").error.clone().unwrap_or_default();
    assert!(error.contains("the-kid"), "it says what it could not find: {error}");
}
