//! `on_update`: a running instance taking another text in place of having
//! its steps run again, with what it was handed to it as `was`.

use std::task::Poll;

use conductor::{Apply, InstanceFilter, InstanceInfo, InstanceStatus, RunStatus, Takes};
use kernel::Source;
use serde_json::{Value, json};

use crate::Rig;
use crate::handlers::{active, handled};
use crate::stepped::Stepped;

/// Something asked that the test looks at itself.
type Reading<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Something that lives a long time: its steps claim, once, and after
/// that it counts what it hears.
pub const ONE: &str = "\
blueprint: lane
version: 1
params: { size: 1 }
state: { builds: 0 }
steps:
  claim: { do: test.count, with: { key: claimed } }
on:
  poke:
    event: test.poke
    steps:
      note: { do: set, with: { builds: ${ state.builds + 1 } } }
      say: { do: test.count, with: { key: old } }
outputs:
  claims: ${ steps.claim.out.count }
";

/// The next text: what it counts is called something else, its steps are
/// others, and it says how one that is running comes to it.
pub const TWO: &str = "\
blueprint: lane
version: 2
params: { size: 1 }
state: { rounds: 0, came_from: none, came_as: none, had_size: 0 }
steps:
  claim: { do: test.count, with: { key: claimed-again } }
  more: { do: test.count, with: { key: more } }
on:
  poke:
    event: test.poke
    steps:
      note: { do: set, with: { rounds: ${ state.rounds + 1 } } }
      say: { do: test.count, with: { key: new } }
on_update:
  - do: set
    with:
      rounds: ${ was.state.builds }
      came_from: ${ was.version }
      came_as: ${ was.revision }
      had_size: ${ was.params.size }
  - do: test.count
    with: { key: carried }
outputs:
  claims: ${ steps.claim.out.count }
";

/// A text that says something as it is taken, and brings the handler
/// that hears it.
const SAYS: &str = "\
blueprint: lane
version: 3
params: { size: 1 }
state: { builds: 0, had: 0 }
steps:
  claim: { do: test.count, with: { key: claimed } }
on:
  poke:
    event: test.poke
    steps:
      note: { do: set, with: { builds: ${ state.builds + 1 } } }
      say: { do: test.count, with: { key: old } }
  taken:
    event: lane.taken
    scope: own
    steps:
      note: { do: set, with: { had: ${ event.data.size } } }
      say: { do: test.count, with: { key: heard-taken } }
on_update:
  - do: emit
    with: { topic: lane.taken, size: ${ was.params.size } }
";

pub async fn poke(rig: &Rig) {
    rig.journal.emit("test.poke", Source::default(), json!({})).await.unwrap();
}

/// The first text applied, settled, and poked once.
async fn running(rig: &Rig) -> String {
    let applied = rig.apply(ONE, json!({})).await;
    active(rig, &applied.instance).await;
    poke(rig).await;
    handled(rig, "on.poke", 1).await;
    applied.instance
}

/// What was said of instances being carried over: `(instance, from, to)`.
pub async fn carried(rig: &Rig) -> Vec<(String, String, String)> {
    rig.events("instance.updated")
        .await
        .into_iter()
        .filter(|event| event.data["carried"] == true)
        .map(|event| {
            let text = |key: &str| event.data[key].as_str().unwrap().to_string();
            (event.source.instance.clone().unwrap(), text("from"), text("revision"))
        })
        .collect()
}

pub fn counts(rig: &Rig, keys: &[&str]) -> Vec<u64> {
    keys.iter().map(|key| rig.shared.count(key)).collect()
}

#[tokio::test]
async fn a_running_instance_that_takes_a_text_with_on_update_is_carried_over_and_its_steps_are_not_run_again()
{
    let rig = Rig::new().await;
    let lane = running(&rig).await;
    assert_eq!(rig.instance(&lane).await.state, kernel::fields(json!({ "builds": 1 })));

    let applied = rig.apply(TWO, json!({})).await;
    assert_eq!((applied.instance.as_str(), applied.created, applied.changed), (lane.as_str(), false, true));
    let run = rig.finished(&applied).await;
    assert_eq!((run.flow.as_str(), run.status), ("on_update", RunStatus::Ok));
    assert_eq!(run.revision, blueprint::revision(TWO));
    assert!(run.steps.is_empty(), "none of its steps ran");
    assert_eq!(run.outputs, json!({ "claims": 1 }), "what its steps gave out when they ran is what it gives");

    assert_eq!(
        counts(&rig, &["claimed", "claimed-again", "more", "carried"]),
        [1, 0, 0, 1],
        "the steps of neither text were run; what `on_update` says was done, once"
    );
    let is = rig.instance(&lane).await;
    assert_eq!(
        (is.revision.as_str(), is.status),
        (blueprint::revision(TWO).as_str(), InstanceStatus::Active)
    );
    assert_eq!(
        is.state,
        kernel::fields(json!({
            "rounds": 1,
            "came_from": "1",
            "came_as": blueprint::revision(ONE),
            "had_size": 1,
        })),
        "the key it no longer declares is gone, and what it held was read from `was`"
    );
    assert_eq!(carried(&rig).await, [(lane.clone(), blueprint::revision(ONE), blueprint::revision(TWO))]);
    let versions = rig.engine.versions(&lane, 10).await.unwrap();
    assert_eq!((versions.len(), versions[0].outcome.as_str()), (2, "applied"));
    let flows: Vec<String> =
        rig.engine.runs(&lane, 10).await.unwrap().into_iter().rev().map(|run| run.flow).collect();
    assert_eq!(flows, ["main", "on.poke", "on_update"], "it is one of the instance's runs");
    assert!(
        rig.events("run.finished").await.iter().any(|event| event.data["flow"] == "on_update"),
        "and is said like any run"
    );

    // Its handlers are the new text's, from where the old ones had got to.
    poke(&rig).await;
    handled(&rig, "on.poke", 2).await;
    assert_eq!(counts(&rig, &["old", "new"]), [1, 1]);
    assert_eq!(rig.instance(&lane).await.state["rounds"], 2);

    // Asking again for what it now runs is nothing.
    let again = rig.engine.apply(rig.request(TWO, json!({}))).await.unwrap();
    assert_eq!((again.changed, again.run), (false, None));
    assert_eq!(rig.shared.count("carried"), 1);
}

#[tokio::test]
async fn a_handler_a_text_brings_hears_what_the_text_says_as_it_is_taken() {
    let rig = Rig::new().await;
    let lane = running(&rig).await;
    let applied = rig.apply(SAYS, json!({ "size": 4 })).await;
    let run = rig.finished(&applied).await;
    assert_eq!((run.flow.as_str(), run.status), ("on_update", RunStatus::Ok));
    // The handler is new with this text, and what `on_update` said was
    // said before the instance was active again: it is heard all the same.
    handled(&rig, "on.taken", 1).await;
    assert_eq!(rig.shared.count("heard-taken"), 1);
    assert_eq!(rig.instance(&lane).await.state["had"], 1, "what it was handed before, as the event said it");

    // Taken again with nothing changed is nothing; with other params it
    // is said again, to a handler that is no longer new.
    let again = rig.apply(SAYS, json!({ "size": 5 })).await;
    rig.finished(&again).await;
    handled(&rig, "on.taken", 2).await;
    assert_eq!(rig.instance(&lane).await.state["had"], 4);
}

#[tokio::test]
async fn on_update_tells_which_version_it_comes_from_so_that_several_can_be_jumped_at_once() {
    let rig = Rig::new().await;
    let three = "\
blueprint: lane
version: 3
state: { laps: 0 }
on:
  poke: { event: test.poke, do: test.count, with: { key: newest } }
on_update:
  - do: set
    when: ${ was.version == \"1\" }
    with: { laps: ${ was.state.builds } }
  - do: set
    when: ${ was.version == \"2\" }
    with: { laps: ${ was.state.rounds } }
";
    // One that is two versions back, and one that is one back.
    let first = running(&rig).await;
    let other = rig.request(ONE, json!({}));
    let second = rig.engine.apply(conductor::Apply { key: Some("second".into()), ..other }).await.unwrap();
    active(&rig, &second.instance).await;
    let to_two = rig.request(TWO, json!({}));
    let moved = rig.engine.apply(conductor::Apply { key: Some("second".into()), ..to_two }).await.unwrap();
    rig.finished(&moved).await;
    // The first has heard one poke already; both hear three more.
    for _ in 0..3 {
        poke(&rig).await;
    }
    handled(&rig, "on.poke", 7).await;
    assert_eq!(rig.instance(&first).await.state["builds"], 4);
    assert_eq!(rig.instance(&second.instance).await.state["rounds"], 3);

    rig.finished(&rig.apply(three, json!({})).await).await;
    let to_three = rig.request(three, json!({}));
    let last = rig.engine.apply(conductor::Apply { key: Some("second".into()), ..to_three }).await.unwrap();
    rig.finished(&last).await;
    assert_eq!(rig.instance(&first).await.state, kernel::fields(json!({ "laps": 4 })), "from version 1");
    assert_eq!(
        rig.instance(&second.instance).await.state,
        kernel::fields(json!({ "laps": 3 })),
        "from version 2"
    );
}

#[tokio::test]
async fn an_on_update_that_fails_puts_back_the_text_and_the_state_the_instance_had() {
    let rig = Rig::new().await;
    let lane = running(&rig).await;
    let failing = TWO.replace(
        "  - do: test.count\n    with: { key: carried }\n",
        "  - do: test.flaky\n    with: { message: no }\n  - do: test.count\n    with: { key: carried }\n",
    );
    assert_ne!(failing, TWO);

    let applied = rig.apply(&failing, json!({})).await;
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.error.as_deref(), Some("`on_update` entry 2, `test.flaky`: no"));
    assert_eq!(rig.shared.count("carried"), 0, "what came after the one that failed was not done");

    let is = rig.instance(&lane).await;
    assert_eq!(
        (is.revision.as_str(), is.status),
        (blueprint::revision(ONE).as_str(), InstanceStatus::Active)
    );
    assert_eq!(
        is.state,
        kernel::fields(json!({ "builds": 1 })),
        "the state it had, with the key the other text dropped and nothing of what that text set"
    );
    assert_eq!(
        is.error.as_deref(),
        Some("update failed, still running the previous version: `on_update` entry 2, `test.flaky`: no")
    );
    let failed = rig
        .event("instance.update_failed", |event| event.source.instance.as_deref() == Some(lane.as_str()))
        .await;
    assert_eq!(failed.data["kept"], blueprint::revision(ONE));
    let versions = rig.engine.versions(&lane, 10).await.unwrap();
    assert_eq!(versions[0].outcome, "reverted");
    assert_eq!(counts(&rig, &["claimed", "claimed-again"]), [1, 0], "and no steps were run for it");

    // It goes on as it was: the old text's handler, on the old text's state.
    poke(&rig).await;
    handled(&rig, "on.poke", 2).await;
    assert_eq!(counts(&rig, &["old", "new"]), [2, 0]);
    assert_eq!(rig.instance(&lane).await.state, kernel::fields(json!({ "builds": 2 })));

    // And can be carried over by a text that does not fail.
    rig.finished(&rig.apply(TWO, json!({})).await).await;
    assert_eq!(rig.instance(&lane).await.state["rounds"], 2);
}

#[tokio::test]
async fn a_first_apply_runs_the_steps_and_never_on_update() {
    let rig = Rig::new().await;
    let run = rig.run(TWO, json!({})).await;
    assert_eq!((run.flow.as_str(), run.status), ("main", RunStatus::Ok));
    assert_eq!(counts(&rig, &["claimed-again", "more", "carried"]), [1, 1, 0]);
    assert_eq!(rig.instance(&run.instance).await.state["came_from"], "none");
}

#[tokio::test]
async fn the_same_text_with_other_params_is_carried_and_sees_the_params_it_had() {
    let rig = Rig::new().await;
    let applied = rig.apply(TWO, json!({ "size": 3 })).await;
    active(&rig, &applied.instance).await;

    let again = rig.apply(TWO, json!({ "size": 3 })).await;
    assert_eq!((again.changed, again.run), (false, None), "the same text with the same params is nothing");

    let run = rig.finished(&rig.apply(TWO, json!({ "size": 5 })).await).await;
    assert_eq!(run.flow, "on_update");
    let is = rig.instance(&applied.instance).await;
    assert_eq!(is.params["size"], 5, "its params are the new ones");
    assert_eq!(is.state["had_size"], 3, "and `was.params` the ones it had");
    assert_eq!(is.state["came_as"], blueprint::revision(TWO), "the text it had is the one it has");
    assert_eq!(counts(&rig, &["claimed-again", "carried"]), [1, 1]);
}

#[tokio::test]
async fn on_update_waits_for_a_run_under_way_which_finishes_on_the_text_it_began_with() {
    let rig = Rig::new().await;
    let held = ONE.replace(
        "      note: { do: set, with: { builds: ${ state.builds + 1 } } }\n",
        "      note: { do: set, with: { builds: ${ state.builds + 1 } } }\n      wait: { do: test.gate, with: { name: hold } }\n",
    );
    let applied = rig.apply(&held, json!({})).await;
    let lane = applied.instance;
    active(&rig, &lane).await;
    poke(&rig).await;
    rig.reached("hold").await;
    // A second, while the first is still being answered.
    poke(&rig).await;

    let asked = rig.apply(TWO, json!({})).await;
    let waiting = rig.engine.run(asked.run.as_ref().unwrap()).await.unwrap();
    assert_eq!((waiting.flow.as_str(), waiting.status), ("on_update", RunStatus::Running));
    let is = rig.instance(&lane).await;
    assert_eq!(
        (is.revision.as_str(), is.status, is.state["builds"].clone()),
        (blueprint::revision(&held).as_str(), InstanceStatus::Active, json!(1)),
        "until it is between runs it is as it was"
    );
    assert_eq!(counts(&rig, &["carried", "old"]), [0, 0], "nothing was cut short and nothing begun");

    rig.open("hold");
    let run = rig.finished(&asked).await;
    assert_eq!(run.status, RunStatus::Ok);
    handled(&rig, "on.poke", 2).await;
    assert_eq!(
        counts(&rig, &["old", "new", "carried"]),
        [1, 1, 1],
        "the run under way finished as the old text; what came in meanwhile was the new text's"
    );
    assert_eq!(rig.instance(&lane).await.state["rounds"], 2, "one counted before, one after");
    // In that order: the old run ended before the instance was anything else.
    let first = |events: Vec<kernel::Event>, key: &str, is: Value| {
        events.into_iter().find(|event| event.data[key] == is).map(|event| event.seq).unwrap()
    };
    let old_ended = first(rig.events("run.finished").await, "flow", json!("on.poke"));
    let taken = first(rig.events("instance.updated").await, "carried", json!(true));
    let began = first(rig.events("run.started").await, "flow", json!("on_update"));
    assert!(old_ended < taken && taken < began, "{old_ended} < {taken} < {began}");
}

#[tokio::test]
async fn a_later_apply_calls_off_a_text_the_instance_was_still_waiting_to_be_carried_over_to() {
    let rig = Rig::new().await;
    let held = ONE.replace(
        "      say: { do: test.count",
        "      wait: { do: test.gate, with: { name: hold } }\n      say: { do: test.count",
    );
    let lane = rig.apply(&held, json!({})).await.instance;
    active(&rig, &lane).await;
    poke(&rig).await;
    rig.reached("hold").await;

    let first = rig.apply(TWO, json!({})).await;
    let other = TWO.replace("key: carried", "key: carried-later");
    let second = rig.apply(&other, json!({})).await;
    let called_off = rig.finished(&first).await;
    assert_eq!(called_off.status, RunStatus::Cancelled);
    assert_eq!(
        called_off.error.as_deref(),
        Some("it was applied over, or stopped, before it had been carried over to this text")
    );

    rig.open("hold");
    assert_eq!(rig.finished(&second).await.status, RunStatus::Ok);
    assert_eq!(counts(&rig, &["carried", "carried-later"]), [0, 1]);
    assert_eq!(rig.instance(&lane).await.revision, blueprint::revision(&other));
    assert_eq!(carried(&rig).await.len(), 1);
}

#[tokio::test]
async fn an_update_of_its_steps_or_a_stop_also_calls_off_a_text_it_was_waiting_to_be_carried_over_to() {
    let rig = Rig::new().await;
    let why = Some("it was applied over, or stopped, before it had been carried over to this text");
    for stopped in [false, true] {
        let key = if stopped { "to-stop" } else { "to-force" };
        // A gate of its own, so that each is known to be in the middle of a run.
        let held = ONE.replace(
            "      say: { do: test.count",
            &format!(
                "      wait: {{ do: test.gate, with: {{ name: {key} }} }}\n      say: {{ do: test.count"
            ),
        );
        let keyed =
            |source: &str| conductor::Apply { key: Some(key.into()), ..rig.request(source, json!({})) };
        let lane = rig.engine.apply(keyed(&held)).await.unwrap().instance;
        active(&rig, &lane).await;
        poke(&rig).await;
        rig.reached(key).await;
        let waiting = rig.engine.apply(keyed(TWO)).await.unwrap();
        if stopped {
            rig.engine.stop(&lane).await.unwrap();
        } else {
            // Its steps are asked for: the run under way is cut short, as
            // an update of the steps always cut it.
            let forced = conductor::Apply { force: true, ..keyed(TWO) };
            let run = rig.finished(&rig.engine.apply(forced).await.unwrap()).await;
            assert_eq!((run.flow.as_str(), run.status), ("main", RunStatus::Ok));
        }
        let called_off = rig.finished(&waiting).await;
        assert_eq!((called_off.status, called_off.error.as_deref()), (RunStatus::Cancelled, why), "{key}");
        assert_eq!(rig.shared.count("carried"), 0, "{key}");
        // The other lane, if it is there, hears nothing of this one's poke.
        rig.engine.stop(&lane).await.unwrap();
    }
    assert!(
        rig.events("run.finished")
            .await
            .iter()
            .any(|event| event.data["flow"] == "on_update" && event.data["status"] == "cancelled"),
        "its end is said as any run's is"
    );
}

#[tokio::test]
async fn asking_for_the_steps_runs_them_whatever_the_text_says_and_a_text_that_says_nothing_is_as_it_always_was()
 {
    let rig = Rig::new().await;
    let lane = running(&rig).await;

    // Forced: an update like any other.
    let forced = conductor::Apply { force: true, ..rig.request(TWO, json!({})) };
    let run = rig.finished(&rig.engine.apply(forced).await.unwrap()).await;
    assert_eq!((run.flow.as_str(), run.status), ("main", RunStatus::Ok));
    assert_eq!(counts(&rig, &["claimed", "claimed-again", "more", "carried"]), [1, 1, 1, 0]);
    assert_eq!(rig.instance(&lane).await.state["came_from"], "none", "`on_update` was not what was done");
    assert!(carried(&rig).await.is_empty());

    // A text with no `on_update`: its steps run again, and the state it no
    // longer declares is gone with nothing said of it.
    let silent = ONE.replace("version: 1", "version: 4").replace("key: claimed", "key: claimed-anew");
    let run = rig.finished(&rig.apply(&silent, json!({})).await).await;
    assert_eq!((run.flow.as_str(), run.status), ("main", RunStatus::Ok));
    assert_eq!(rig.shared.count("claimed-anew"), 1);
    assert_eq!(rig.instance(&lane).await.state, kernel::fields(json!({ "builds": 0 })));
    assert!(carried(&rig).await.is_empty());
}

#[tokio::test]
async fn an_instance_whose_steps_have_not_settled_is_not_carried_over() {
    let rig = Rig::new().await;
    let starting = "\
blueprint: lane
state: { builds: 0 }
steps:
  wait: { do: test.gate, with: { name: first } }
on:
  poke: { event: test.poke, do: noop }
";
    let applied = rig.apply(starting, json!({})).await;
    rig.reached("first").await;
    // There is nothing yet to carry on from: the steps of the new text run.
    let run = rig.finished(&rig.apply(TWO, json!({})).await).await;
    assert_eq!((run.flow.as_str(), run.status), ("main", RunStatus::Ok));
    assert_eq!(counts(&rig, &["claimed-again", "carried"]), [1, 0]);
    assert_eq!(rig.instance(&applied.instance).await.revision, blueprint::revision(TWO));
}

/// A parent whose handler starts a lane for each card.
const FLEET: &str = "\
blueprint: fleet
on:
  ready:
    event: board.item.moved
    steps:
      go: { spawn: lane, as: \"lane-${ event.data.id }\" }
";

async fn moved(rig: &Rig, id: u64) {
    rig.journal.emit("board.item.moved", Source::default(), json!({ "id": id })).await.unwrap();
}

pub async fn keyed(rig: &Rig, key: &str) -> InstanceInfo {
    let live = rig.engine.instances(InstanceFilter { live_only: true, ..Default::default() }).await.unwrap();
    live.into_iter().find(|one| one.key == key).unwrap_or_else(|| panic!("no live `{key}`"))
}

/// The fleet applied, and one lane of it started by its handler, settled
/// and poked once: `(fleet, lane)`.
async fn fleet(rig: &Rig) -> (String, String) {
    rig.library.put("fleet", FLEET);
    rig.library.put("lane", ONE);
    let applied = rig.apply(FLEET, json!({})).await;
    active(rig, &applied.instance).await;
    moved(rig, 1).await;
    handled(rig, "on.ready", 1).await;
    let lane = keyed(rig, "lane-1").await;
    active(rig, &lane.id).await;
    poke(rig).await;
    handled(rig, "on.poke", 1).await;
    (applied.instance, lane.id)
}

#[tokio::test]
async fn applying_a_parent_again_carries_over_what_its_handler_started_when_the_text_now_there_says_how() {
    let rig = Rig::new().await;
    let (_, lane) = fleet(&rig).await;
    rig.library.put("lane", TWO);

    let behind = rig.engine.shown_one(rig.instance(&lane).await).behind.expect("it is behind");
    assert_eq!(behind.takes, Takes::Carried);
    assert_eq!(serde_json::to_value(behind.takes).unwrap(), "carried");
    assert_eq!(
        behind.why,
        "The text of `lane` now there says how one that is running takes it (`on_update`). It is carried \
         over to that text where it is, with that done in place of its steps, which are not run again, \
         when what started it, or it itself, is applied again."
    );

    let again = rig.engine.apply(rig.request(FLEET, json!({}))).await.unwrap();
    assert!(!again.changed, "the fleet's own text is what it was");
    handled(&rig, "on_update", 1).await;
    let is = rig.instance(&lane).await;
    assert_eq!(
        (is.revision.as_str(), is.status),
        (blueprint::revision(TWO).as_str(), InstanceStatus::Active)
    );
    assert_eq!(is.state["rounds"], 1);
    assert_eq!(is.principal.name, "Ada", "it acts as whoever asked for the fleet to be applied");
    assert_eq!(
        counts(&rig, &["claimed", "claimed-again", "carried"]),
        [1, 0, 1],
        "its steps were not run, of either text"
    );
    assert_eq!(carried(&rig).await.len(), 1);
    assert_eq!(rig.engine.shown_one(rig.instance(&lane).await).behind, None);

    // Applying the fleet once more finds nothing to do.
    rig.engine.apply(rig.request(FLEET, json!({}))).await.unwrap();
    poke(&rig).await;
    handled(&rig, "on.poke", 2).await;
    assert_eq!(counts(&rig, &["new", "carried"]), [1, 1]);
}

#[tokio::test]
async fn a_text_that_says_how_it_is_taken_is_taken_that_way_even_when_it_begins_as_the_one_before() {
    let rig = Rig::new().await;
    let (_, lane) = fleet(&rig).await;
    // The same params, steps and state: only what it does later, and a
    // word on how to come to it.
    let mended = format!(
        "{}on_update:\n  - do: set\n    with: {{ builds: ${{ was.state.builds * 10 }} }}\n",
        ONE.replace("version: 1", "version: 5")
    );
    rig.library.put("lane", &mended);
    assert_eq!(rig.engine.shown_one(rig.instance(&lane).await).behind.unwrap().takes, Takes::Carried);
    rig.engine.apply(rig.request(FLEET, json!({}))).await.unwrap();
    handled(&rig, "on_update", 1).await;
    assert_eq!(rig.instance(&lane).await.state["builds"], 10, "what it says was done");
    assert_eq!(rig.shared.count("claimed"), 1);
}

#[tokio::test]
async fn a_spawn_of_a_child_that_is_running_carries_it_over_when_its_text_says_how_and_a_call_runs_its_steps()
{
    let rig = Rig::new().await;
    let (_, lane) = fleet(&rig).await;
    rig.library.put("lane", TWO);
    // The card comes round again: the fleet's handler spawns the lane it has.
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 2).await;
    handled(&rig, "on_update", 1).await;
    assert_eq!(rig.instance(&lane).await.revision, blueprint::revision(TWO));
    assert_eq!(
        counts(&rig, &["claimed", "claimed-again", "carried"]),
        [1, 0, 1],
        "it was carried over; its steps were not run again"
    );

    // A call asks for the steps to be run and waits for what they give:
    // it is not answered by `on_update`.
    let caller = "\
blueprint: caller
steps:
  kid: { call: lane }
on:
  poke: { event: test.poke, do: noop }
outputs:
  got: ${ steps.kid.out.claims }
";
    rig.library.put("lane", ONE);
    let first = rig.run(caller, json!({})).await;
    assert_eq!(first.outputs, json!({ "got": 2 }));
    rig.library.put("lane", TWO);
    let forced = conductor::Apply { force: true, ..rig.request(caller, json!({})) };
    let second = rig.finished(&rig.engine.apply(forced).await.unwrap()).await;
    assert_eq!(second.status, RunStatus::Ok);
    assert_eq!(second.outputs, json!({ "got": 1 }), "what the child's steps gave when they ran now");
    assert_eq!(
        counts(&rig, &["claimed", "claimed-again", "carried"]),
        [2, 1, 1],
        "the steps of the text now there were run, and nothing was carried"
    );
}

#[tokio::test]
async fn an_instance_being_carried_over_when_the_conductor_stops_is_carried_over_when_it_starts_again() {
    let mut rig = Rig::new().await;
    let lane = running(&rig).await;
    let slow = TWO.replace(
        "  - do: test.count\n    with: { key: carried }\n",
        "  - do: set\n    with: { rounds: ${ state.rounds + 100 } }\n  - do: test.gate\n    with: { name: mid }\n  - do: test.count\n    with: { key: carried }\n",
    );
    let applied = rig.apply(&slow, json!({})).await;
    rig.reached("mid").await;
    assert_eq!(rig.instance(&lane).await.state["rounds"], 101);

    rig.restart().await;
    // It begins `on_update` again, from what the instance was.
    let again = rig.soon("the gate to be reached again", async {
        let mut follow = rig.follow("test.reached");
        let mut seen = 0;
        while seen < 2 {
            if follow.next().await.unwrap().data["gate"] == "mid" {
                seen += 1;
            }
        }
    });
    again.await;
    rig.open("mid");
    let run = rig.finished(&applied).await;
    assert_eq!((run.flow.as_str(), run.status), ("on_update", RunStatus::Ok));
    let is = rig.instance(&lane).await;
    assert_eq!(is.status, InstanceStatus::Active);
    assert_eq!(is.state["rounds"], 101, "what it had set before the stop was not counted twice");
    assert_eq!(counts(&rig, &["claimed", "claimed-again", "carried"]), [1, 0, 1]);
    poke(&rig).await;
    handled(&rig, "on.poke", 2).await;
    assert_eq!(rig.shared.count("new"), 1);
}

#[tokio::test]
async fn what_is_below_an_instance_that_was_carried_over_is_looked_at_as_after_any_apply() {
    let rig = Rig::new().await;
    let (fleet, lane) = fleet(&rig).await;
    // The fleet's own text now says how it is taken, and the lane's
    // differs only in what it does later.
    let fleet_two = format!("{FLEET}on_update:\n  - do: test.count\n    with: {{ key: fleet-carried }}\n");
    let lane_later = ONE.replace("key: old", "key: new").replace("version: 1", "version: 6");
    rig.library.put("fleet", &fleet_two);
    rig.library.put("lane", &lane_later);

    let run = rig.finished(&rig.apply(&fleet_two, json!({})).await).await;
    assert_eq!((run.flow.as_str(), run.status), ("on_update", RunStatus::Ok));
    let took: Option<Value> = rig
        .soon("the lane to take the text", async {
            let mut follow = rig.follow("instance.updated");
            loop {
                let event = follow.next().await.unwrap();
                if event.source.instance.as_deref() == Some(lane.as_str()) {
                    return Some(event.data.clone());
                }
            }
        })
        .await;
    assert_eq!(took.unwrap()["in_place"], true);
    assert_eq!(rig.instance(&lane).await.revision, blueprint::revision(&lane_later));
    assert_eq!(rig.instance(&fleet).await.status, InstanceStatus::Active);
    assert_eq!(counts(&rig, &["fleet-carried", "claimed"]), [1, 1]);
    // And the fleet still hears: its handler is the new text's.
    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;
    keyed(&rig, "lane-2").await;
}

/// Something with no steps that hears that it took another text, and
/// counts it under what it is called.
const HEARS: &str = "\
blueprint: hears
version: 1
on:
  taken:
    event: instance.updated
    scope: own
    when: ${ event.source.instance == instance.id }
    do: test.count
    with: { key: \"took-${instance.key}\" }
on_update:
  - do: test.count
    with: { key: carried }
";

/// Has `work` done where it stands: polled until it is over with nothing
/// else of this thread's polled meanwhile, the database's thread
/// answering beside it. Which is what a busy machine does to whoever
/// asks the database several things in a row: each answer is there
/// before it is looked for, and whoever asks never gives way.
pub(crate) fn at_once<T>(work: impl Future<Output = T>) -> T {
    let mut work = std::pin::pin!(work);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let began = std::time::Instant::now();
    loop {
        if let std::task::Poll::Ready(done) = work.as_mut().poll(&mut cx) {
            return done;
        }
        assert!(
            began.elapsed().as_secs() < 20,
            "what was to be done at once waits on something of this thread's"
        );
        std::thread::yield_now();
    }
}

// Its handlers are started a moment after an instance is written as
// active, by the run that wrote it, when that run is next looked at. A
// text asked for in that moment is carried over by somebody who told
// its handlers to take no more when there were none yet; they were then
// started on the text it had, heard that the text was taken, and were
// ended with the generation in the middle of that. What they had begun
// was cancelled and its event was nobody's: a pool given another limit
// just as it came up gave out no place, once in fifty on a busy machine.
//
// The order is made here by the database answered a turn at a time
// ([`Stepped`]): the instance is read without anything else being
// looked at, and as soon as it reads as active the text is asked for
// the same way, so that the run that wrote it active has not been
// looked at since it asked. That it came about is measured, which is
// all that says so, and is held of each: each is played until it is at
// rest before the next is begun, since what is measured is measured
// when its handlers are started in the end, and a failure then says
// which instance it was and what it said. Made by asking where the test
// stood while the database's thread answered beside it, the order came
// about only where the run was not looked at again in the same turn of
// the thread as it asked: for seven or eight of eight here, and for
// none of eight once on a machine with much else to do.
#[tokio::test]
async fn a_text_asked_for_as_an_instance_becomes_active_is_heard_to_be_taken_by_its_own_handler() {
    let rig = Rig::new().await;
    let next = HEARS.replace("version: 1", "version: 2");
    let keys: Vec<String> = (0..8).map(|nth| format!("one-{nth}")).collect();
    let mut stepped = Stepped::holding(&rig.journal);
    for (nth, key) in keys.iter().enumerate() {
        let asked = Apply { key: Some(key.clone()), ..rig.request(HEARS, json!({})) };
        let applied = stepped.through(false, rig.engine.apply(asked)).await.expect("the journal is open");
        let instance = applied.unwrap().instance;
        let mut turns = 0;
        rig.soon("the instance to read as active", async {
            loop {
                let mut read: Reading<'_, InstanceInfo> = Box::pin(rig.instance(&instance));
                let now = match stepped.at_once(read.as_mut()) {
                    Poll::Ready(now) => now,
                    Poll::Pending => panic!("an instance is read by asking the database and nothing else"),
                };
                if now.status == InstanceStatus::Active {
                    break;
                }
                stepped.turn(false).await;
                turns += 1;
            }
        })
        .await;
        let asked = Apply { key: Some(key.clone()), ..rig.request(&next, json!({})) };
        let mut over: Reading<'_, kernel::Result<conductor::Applied>> = Box::pin(rig.engine.apply(asked));
        let (applied, at_once) = match stepped.at_once(over.as_mut()) {
            Poll::Ready(applied) => (applied, true),
            Poll::Pending => (stepped.through(false, over).await.expect("the journal is open"), false),
        };
        assert_eq!(applied.unwrap().instance, instance);
        // It hears that it took the text, and is at rest, before the
        // next: how long its handlers were held back is measured when
        // they are started, which is by then.
        rig.soon("it to hear that it took the text", async {
            let mut still = 0;
            while still < 6 || rig.shared.count(&format!("took-{key}")) < 1 {
                still = if stepped.turn(false).await == Some(0) { still + 1 } else { 0 };
            }
        })
        .await;
        let query = journal::MetricQuery { name: "instance.listening_late".into(), ..Default::default() };
        let measured =
            stepped.through(false, rig.journal.query_metrics(query)).await.expect("the journal is open");
        let held: u64 = measured.unwrap().iter().map(|series| series.total.count).sum();
        if held != nth as u64 + 1 {
            let said = stepped.through(false, rig.events("")).await.expect("the journal is open");
            let said: Vec<String> = said
                .iter()
                .filter(|event| event.source.instance.as_deref() == Some(instance.as_str()))
                .map(|event| format!("{} {}", event.topic, event.data))
                .collect();
            panic!(
                "`{key}` was asked for the text after its handlers had been started: the order this is there \
                 for did not come about. It read as active after {turns} turns; whoever applied the text did \
                 so at once: {at_once}; and it said:\n{}",
                said.join("\n")
            );
        }
    }
    drop(stepped);
    handled(&rig, "on.taken", keys.len()).await;
    let times: Vec<u64> = keys.iter().map(|key| rig.shared.count(&format!("took-{key}"))).collect();
    assert_eq!(times, vec![1; keys.len()], "each heard it once");
    assert_eq!(counts(&rig, &["carried"]), [keys.len() as u64]);
}
