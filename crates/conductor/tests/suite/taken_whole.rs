//! A tree taken to what ships now, whole: somebody who applies at its top
//! saying to take the texts as they now are brings every part of it to
//! them, whatever began otherwise, in an order in which what a step calls
//! has its new text before the step runs; and nothing applied without
//! saying so moves a part back to a text it had moved on from.

use conductor::{Apply, InstanceFilter, InstanceInfo};
use kernel::Source;
use serde_json::json;

use crate::handlers::active;
use crate::{Rig, user};

/// What somebody applies: its step calls the fleet, which stays.
const TOP: &str = "\
blueprint: top
steps:
  fleet: { call: fleet, as: fleet }
on:
  poke: { event: test.poke, do: test.count, with: { key: top-poked } }
";

/// The fleet: its step calls what arranges it.
const FLEET: &str = "\
blueprint: fleet
version: 1
steps:
  arrange: { call: arrangement, as: arrangement }
  mark: { do: test.count, with: { key: fleet-1 } }
on:
  poke: { event: test.poke, do: test.count, with: { key: fleet-poked } }
";

/// What arranges it: calls a role, and spawns a stage by a name only
/// known as the step runs, which is held to from where it is first reached.
const ARRANGEMENT: &str = "\
blueprint: arrangement
version: 1
params: { kind: stage }
steps:
  roles: { call: role, as: worker }
  stage: { spawn: \"${ params.kind }\", as: stage }
on:
  poke: { event: test.poke, do: test.count, with: { key: arrangement-poked } }
";

const ROLE: &str = "\
blueprint: role
version: 1
steps:
  mark: { do: test.count, with: { key: role-1 } }
on:
  poke: { event: test.poke, do: test.count, with: { key: role-poked } }
";

/// A stage: what its handler starts, no step of anything names.
const STAGE: &str = "\
blueprint: stage
version: 1
steps:
  mark: { do: test.count, with: { key: stage-1 } }
on:
  came:
    event: test.came
    steps:
      go: { spawn: round, as: \"round-${ event.data.id }\" }
";

const ROUND: &str = "\
blueprint: round
version: 1
steps:
  mark: { do: test.count, with: { key: round-1 } }
on:
  poke: { event: test.poke, do: test.count, with: { key: round-poked } }
";

/// Each text as a later program ships it, every one of which begins
/// otherwise: the arrangement tells the role something the role of
/// before never took.
fn later() -> Vec<(&'static str, String)> {
    let next = |text: &str, from: &str, to: &str| text.replace("version: 1", "version: 2").replace(from, to);
    vec![
        ("fleet", next(FLEET, "key: fleet-1", "key: fleet-2")),
        (
            "arrangement",
            next(ARRANGEMENT, "call: role, as: worker }", "call: role, as: worker, with: { prompt: hi } }"),
        ),
        (
            "role",
            next(
                ROLE,
                "steps:\n  mark: { do: test.count, with: { key: role-1 } }",
                "params:\n  prompt: { type: string }\nsteps:\n  mark: { do: test.count, with: { key: role-2 } }",
            ),
        ),
        ("stage", next(STAGE, "key: stage-1", "key: stage-2")),
        ("round", next(ROUND, "key: round-1", "key: round-2")),
    ]
}

async fn live(rig: &Rig) -> Vec<InstanceInfo> {
    rig.engine.instances(InstanceFilter { live_only: true, ..Default::default() }).await.unwrap()
}

async fn keyed(rig: &Rig, key: &str) -> InstanceInfo {
    live(rig).await.into_iter().find(|one| one.key == key).unwrap_or_else(|| panic!("no live `{key}`"))
}

/// Each live part by its key, with the version of the text it runs.
async fn versions(rig: &Rig) -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = live(rig)
        .await
        .into_iter()
        .map(|one| (one.key.clone(), blueprint::version(&one.source).unwrap_or_else(|| "none".into())))
        .collect();
    all.sort();
    all
}

/// Waits until every live part is active and on `version`, but the top,
/// which says none: whatever comes of an apply is under way when it is
/// answered.
async fn comes_to(rig: &Rig, version: &str) {
    let mut follow = rig.follow("run.finished,instance.updated,instance.active");
    let wanted = |all: &[InstanceInfo]| {
        all.iter().all(|one| {
            one.status == conductor::InstanceStatus::Active
                && (one.key == "top" || blueprint::version(&one.source).as_deref() == Some(version))
        })
    };
    rig.soon(&format!("every part to come to version {version}"), async {
        loop {
            if wanted(&live(rig).await) {
                return;
            }
            follow.next().await.unwrap();
        }
    })
    .await;
}

/// The tree as the first program shipped it, applied, with a round its
/// stage's handler started: the top's id.
async fn applied(rig: &Rig) -> String {
    rig.library.ship();
    for (name, text) in [("top", TOP), ("fleet", FLEET), ("arrangement", ARRANGEMENT), ("role", ROLE)] {
        rig.library.put(name, text);
    }
    rig.library.put("stage", STAGE);
    rig.library.put("round", ROUND);
    let top = rig.apply(TOP, json!({})).await;
    active(rig, &top.instance).await;
    let stage = keyed_soon(rig, "stage").await;
    active(rig, &stage.id).await;
    rig.journal.emit("test.came", Source::default(), json!({ "id": 1 })).await.unwrap();
    keyed_soon(rig, "round-1").await;
    comes_to(rig, "1").await;
    top.instance
}

/// The live part under `key`, once it is there.
async fn keyed_soon(rig: &Rig, key: &str) -> InstanceInfo {
    let mut follow = rig.follow("instance.*");
    rig.soon(&format!("`{key}` to be there"), async {
        loop {
            if let Some(one) = live(rig).await.into_iter().find(|one| one.key == key) {
                return one;
            }
            follow.next().await.unwrap();
        }
    })
    .await
}

fn ship_later(rig: &Rig) {
    for (name, text) in later() {
        rig.library.put(name, &text);
    }
}

fn taking(rig: &Rig) -> Apply {
    Apply { take: true, ..rig.request(TOP, json!({})) }
}

#[tokio::test]
async fn taking_what_ships_at_the_top_brings_every_part_of_the_tree_to_it_steps_run_again_where_they_begin_otherwise()
 {
    let rig = Rig::new().await;
    applied(&rig).await;
    ship_later(&rig);

    // The top's own text is what it was: the answer says what moved under it.
    let taken = rig.engine.apply(taking(&rig)).await.unwrap();
    assert!(!taken.changed);
    let mut moved: Vec<&str> = taken.reached.taken.iter().map(|one| one.name.as_str()).collect();
    moved.sort();
    assert_eq!(moved, ["arrangement", "fleet", "role", "round", "stage"], "{:?}", taken.reached);

    // Every part comes to what ships, the round a handler started among them.
    comes_to(&rig, "2").await;
    assert_eq!(
        versions(&rig).await,
        [
            ("arrangement".into(), "2".into()),
            ("fleet".into(), "2".into()),
            ("round-1".into(), "2".into()),
            ("stage".into(), "2".into()),
            ("top".into(), "none".into()),
            ("worker".into(), "2".into()),
        ]
    );
    // Each once, on the text it came to; and nothing failed on the way.
    let counted = |key: &str| rig.shared.count(key);
    assert_eq!(
        [counted("fleet-2"), counted("role-2"), counted("stage-2"), counted("round-2")],
        [1, 1, 1, 1],
        "every part's steps ran once on what ships now"
    );
    let failed = rig.events("run.failed,instance.failed,instance.update_failed").await;
    assert!(failed.is_empty(), "{:?}", failed.iter().map(|event| &event.data).collect::<Vec<_>>());
    // Nothing is said to be behind once it is taken.
    for shown in rig.engine.shown(live(&rig).await) {
        assert!(shown.behind.is_none(), "`{}` is behind: {:?}", shown.instance.key, shown.behind);
    }
}

#[tokio::test]
async fn a_part_taken_by_itself_brings_what_is_under_it_whatever_was_held_under_it_before() {
    let rig = Rig::new().await;
    applied(&rig).await;
    // The arrangement was once taken by itself: it holds what it reached then.
    let arrangement = keyed(&rig, "arrangement").await;
    rig.engine.apply_again_taking(&arrangement.id, user(), false, true).await.unwrap();
    comes_to(&rig, "1").await;
    ship_later(&rig);

    // The fleet taken by itself: what its steps call is what ships now,
    // down to the role the new arrangement tells something new.
    let fleet = keyed(&rig, "fleet").await;
    let taken = rig.engine.apply_again_taking(&fleet.id, user(), false, true).await.unwrap();
    let run = rig.finished(&taken).await;
    assert_eq!(run.status, conductor::RunStatus::Ok, "{:?}", run.error);
    comes_to(&rig, "2").await;
    let failed = rig.events("run.failed,instance.failed").await;
    assert!(failed.is_empty(), "{:?}", failed.iter().map(|event| &event.data).collect::<Vec<_>>());
    assert_eq!(rig.shared.count("role-2"), 1);
}

#[tokio::test]
async fn the_top_applied_again_without_taking_leaves_a_part_on_what_it_was_moved_on_to() {
    let rig = Rig::new().await;
    applied(&rig).await;
    ship_later(&rig);
    let fleet = keyed(&rig, "fleet").await;
    let taken = rig.engine.apply_again_taking(&fleet.id, user(), false, true).await.unwrap();
    rig.finished(&taken).await;
    assert_eq!(keyed(&rig, "fleet").await.revision, blueprint::revision(&later()[0].1));

    // Its steps run again, as somebody asked: what its step calls is
    // what that is on now, and not the text the top was applied with.
    let again = rig.engine.apply(Apply { force: true, ..rig.request(TOP, json!({})) }).await.unwrap();
    rig.finished(&again).await;
    assert_eq!(keyed(&rig, "fleet").await.revision, blueprint::revision(&later()[0].1), "not moved back");
    comes_to(&rig, "2").await;
    let failed = rig.events("run.failed,instance.failed").await;
    assert!(failed.is_empty(), "{:?}", failed.iter().map(|event| &event.data).collect::<Vec<_>>());
}
