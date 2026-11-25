//! What a blueprint reaches is checked when it is applied, and a running
//! instance goes on with the last text that checked when one it calls no
//! longer does.

use conductor::{Apply, InstanceFilter, InstanceStatus, RunStatus};
use journal::MetricQuery;
use kernel::Source;
use serde_json::{Value, json};

use crate::handlers::{active, handled};
use crate::{Rig, step, user};

const WORKER: &str = "\
blueprint: worker
params: { id: 0 }
steps:
  work: { do: test.count, with: { key: worked } }
";

/// The same, written for a device whose `test.count` takes `loudly`.
const NEWER_WORKER: &str = "\
blueprint: worker
params: { id: 0 }
steps:
  work: { do: test.count, with: { key: worked, loudly: true } }
";

const LATER_WORKER: &str = "\
blueprint: worker
params: { id: 0 }
steps:
  work: { do: test.count, with: { key: worked-later } }
";

const WATCHER: &str = "\
blueprint: watcher
on:
  ready:
    event: board.item.moved
    steps:
      go: { call: worker, as: the-worker, with: { id: ${ event.data.id } } }
";

const WRONG: &str = "worker.bp:4:3: error: `test.count` has no argument `loudly`; it takes: key";

fn newer() -> String {
    format!(
        "`worker` may have been written for a newer version than this one, which is {}",
        env!("CARGO_PKG_VERSION")
    )
}

async fn refused(rig: &Rig, source: &str) -> String {
    let refused = rig.engine.apply(rig.request(source, json!({}))).await.expect_err("it is refused");
    assert!(rig.engine.instances(InstanceFilter::default()).await.unwrap().is_empty(), "nothing was started");
    refused.message
}

async fn moved(rig: &Rig, id: u64) {
    rig.journal.emit("board.item.moved", Source::default(), json!({ "id": id })).await.unwrap();
}

/// What was said each time a call went on with an earlier text.
async fn kept(rig: &Rig) -> Vec<Value> {
    rig.events("instance.call_kept").await.into_iter().map(|event| event.data).collect()
}

/// A watcher that has called `worker` once, with the text it then had.
async fn watching(rig: &Rig) -> String {
    rig.library.put("worker", WORKER);
    let applied = rig.apply(WATCHER, json!({})).await;
    active(rig, &applied.instance).await;
    moved(rig, 1).await;
    handled(rig, "on.ready", 1).await;
    assert_eq!(rig.shared.count("worked"), 1);
    applied.instance
}

#[tokio::test]
async fn applying_is_refused_when_a_blueprint_it_calls_does_not_check_here_and_says_which_step_reaches_it() {
    let rig = Rig::new().await;
    rig.library.put("worker", NEWER_WORKER);
    let said = refused(&rig, "blueprint: parent\nsteps:\n  kid: { call: worker }\n").await;
    assert_eq!(
        said,
        format!(
            "`parent` has 1 problem(s):\nparent.bp:3:3: error: this step calls `worker`, which has 1 \
             problem(s): {WRONG}. {}",
            newer()
        )
    );
}

#[tokio::test]
async fn what_is_wrong_further_down_is_said_with_every_step_on_the_way_to_it() {
    let rig = Rig::new().await;
    rig.library.put("worker", NEWER_WORKER);
    rig.library.put(
        "middle",
        "blueprint: middle\non:\n  ready:\n    event: board.item.moved\n    steps:\n      go: { spawn: worker }\n",
    );
    let said = refused(&rig, "blueprint: parent\nsteps:\n  kid: { call: middle }\n").await;
    assert!(
        said.contains(&format!(
            "this step calls `middle`, whose step `go` spawns `worker`, which has 1 problem(s): {WRONG}"
        )),
        "{said}"
    );
}

#[tokio::test]
async fn a_blueprint_named_in_a_handler_in_a_task_or_by_uses_is_checked_as_one_named_in_a_step_is() {
    let rig = Rig::new().await;
    rig.library.put("worker", NEWER_WORKER);
    let handler = "blueprint: parent\non:\n  ready:\n    event: board.item.moved\n    steps:\n      go: { spawn: worker }\n";
    assert!(refused(&rig, handler).await.contains("this step spawns `worker`, which has 1 problem(s)"));
    let task = "blueprint: parent\nsteps:\n  build:\n    task:\n      kid: { call: worker }\n";
    assert!(refused(&rig, task).await.contains("this step calls `worker`, which has 1 problem(s)"));
    let uses = "blueprint: parent\nuses:\n  beside: { blueprint: worker }\nsteps:\n  one: { do: noop }\n";
    assert!(refused(&rig, uses).await.contains("this attaches `worker`, which has 1 problem(s)"));
}

#[tokio::test]
async fn blueprints_that_reach_each_other_are_each_checked_once_and_apply() {
    let rig = Rig::new().await;
    let reaching = |name: &str, other: &str| {
        format!(
            "blueprint: {name}\non:\n  ready:\n    event: board.item.moved\n    steps:\n      go: {{ spawn: {other}, as: other }}\n"
        )
    };
    rig.library.put("one", &reaching("one", "two"));
    rig.library.put("two", &reaching("two", "one"));
    let applied = rig.apply(&reaching("one", "two"), json!({})).await;
    active(&rig, &applied.instance).await;

    // And what is wrong in the circle is still found, where it is.
    rig.library.put("two", &reaching("two", "one").replace("spawn: one, as: other", "do: test.nothing"));
    let said = rig.engine.apply(rig.request(&reaching("one", "two"), json!({}))).await.unwrap_err().message;
    assert!(said.contains("this step spawns `two`, which has 1 problem(s)"), "{said}");
}

#[tokio::test]
async fn a_name_that_is_only_known_when_the_step_runs_is_left_to_that_moment() {
    let rig = Rig::new().await;
    rig.library.put("worker", NEWER_WORKER);
    let run = rig
        .run(
            "blueprint: parent\nparams: { which: worker }\nsteps:\n  kid: { call: \"${ params.which }\" }\n",
            json!({}),
        )
        .await;
    assert_eq!(run.status, RunStatus::Failed);
    // Nothing was remembered of it, so there is nothing to go on with: it
    // fails, and says what an apply would have said.
    assert_eq!(
        step(&run, "kid").error.as_deref(),
        Some(format!("`worker` has 1 problem(s):\n{WRONG}\n{}", newer()).as_str())
    );
    assert!(kept(&rig).await.is_empty());
}

#[tokio::test]
async fn a_blueprint_that_is_not_found_is_warned_about_as_before_and_does_not_refuse_the_apply() {
    let rig = Rig::new().await;
    let source = "blueprint: parent\nsteps:\n  kid: { call: worker }\n";
    let checked = rig.engine.check(source, None);
    assert!(!checked.has_errors());
    assert!(
        checked.diagnostics.iter().any(|said| said.code == "unknown-blueprint"),
        "{:?}",
        checked.diagnostics
    );
    let run = rig.run(source, json!({})).await;
    assert_eq!(step(&run, "kid").error.as_deref(), Some("there is no blueprint called `worker`"));
}

#[tokio::test]
async fn what_a_step_gives_the_blueprint_it_names_is_held_to_its_params_when_applied() {
    let rig = Rig::new().await;
    rig.library
        .put("worker", "blueprint: worker\nparams:\n  id:\n  loud: false\nsteps:\n  work: { do: noop }\n");
    let said =
        refused(&rig, "blueprint: parent\nsteps:\n  kid: { call: worker, with: { id: 1, lowd: true } }\n")
            .await;
    assert!(
        said.contains("this step calls `worker`, which has no param `lowd`; it takes: id, loud"),
        "{said}"
    );
    let said =
        refused(&rig, "blueprint: parent\nsteps:\n  kid: { call: worker, with: { loud: true } }\n").await;
    assert!(
        said.contains("this step calls `worker`, which needs param `id`, which is not given here"),
        "{said}"
    );
    // What is filled in when the step runs is checked then, as it was.
    let whole = "blueprint: parent\nparams: { given: { type: object, default: { id: 1 } } }\nsteps:\n  kid: { call: worker, with: \"${ params.given }\" }\n";
    assert_eq!(rig.run(whole, json!({})).await.status, RunStatus::Ok);
}

#[tokio::test]
async fn the_check_an_editor_asks_for_says_it_against_the_step_that_reaches_it() {
    let rig = Rig::new().await;
    rig.library.put("worker", NEWER_WORKER);
    let checked =
        rig.engine.check("blueprint: parent\nsteps:\n  first: { do: noop }\n  kid: { call: worker }\n", None);
    let errors: Vec<_> = checked.errors().collect();
    assert_eq!(errors.len(), 1);
    assert_eq!((errors[0].code, errors[0].span.start.line), ("reaches-unfit", 4));
    assert!(checked.blueprint.is_some(), "the graph can still be drawn");
}

#[tokio::test]
async fn a_running_instance_goes_on_with_the_last_text_that_checked_when_the_one_it_calls_no_longer_does() {
    let rig = Rig::new().await;
    let watcher = watching(&rig).await;

    rig.library.put("worker", NEWER_WORKER);
    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;

    assert_eq!(rig.shared.count("worked"), 2, "the work was done, by the text there was");
    let runs = rig.events("run.finished").await;
    assert!(runs.iter().all(|run| run.data["status"] == "ok"), "{runs:?}");
    let said = rig.events("instance.call_kept").await;
    assert_eq!(said.len(), 1);
    assert_eq!(
        (said[0].source.instance.as_deref(), said[0].source.step.as_deref()),
        (Some(watcher.as_str()), Some("go")),
        "it is said of the instance that called, by the step that did"
    );
    assert_eq!(
        said[0].data,
        json!({
            "blueprint": "worker",
            "key": "the-worker",
            "error": format!("`worker` has 1 problem(s):\n{WRONG}\n{}", newer()),
            "kept": blueprint::revision(WORKER),
        })
    );
    assert_eq!(rig.instance(&watcher).await.status, InstanceStatus::Active);
}

#[tokio::test]
async fn a_person_who_applies_that_text_is_still_refused_while_the_instance_carries_on() {
    let rig = Rig::new().await;
    watching(&rig).await;
    rig.library.put("worker", NEWER_WORKER);

    let by_name = rig.engine.apply(Apply::new("worker", user())).await.unwrap_err().message;
    assert_eq!(by_name, format!("`worker` has 1 problem(s):\n{WRONG}\n{}", newer()));
    let reaching = rig
        .engine
        .apply(rig.request("blueprint: other\nsteps:\n  kid: { call: worker }\n", json!({})))
        .await
        .unwrap_err()
        .message;
    assert!(reaching.contains("this step calls `worker`, which has 1 problem(s)"), "{reaching}");

    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;
    assert_eq!(rig.shared.count("worked"), 2);
}

#[tokio::test]
async fn what_an_apply_reached_is_remembered_before_any_step_has_called_it() {
    let rig = Rig::new().await;
    rig.library.put("worker", WORKER);
    let applied = rig.apply(WATCHER, json!({})).await;
    active(&rig, &applied.instance).await;

    rig.library.put("worker", NEWER_WORKER);
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    assert_eq!(rig.shared.count("worked"), 1);
    assert_eq!(kept(&rig).await.len(), 1);
}

#[tokio::test]
async fn with_nothing_remembered_the_call_fails_and_says_what_is_wrong() {
    let rig = Rig::new().await;
    // Not there when the watcher is applied, which is only warned about.
    let applied = rig.apply(WATCHER, json!({})).await;
    active(&rig, &applied.instance).await;

    rig.library.put("worker", NEWER_WORKER);
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let failed = rig.event("run.finished", |run| run.data["flow"] == "on.ready").await;
    assert_eq!(
        failed.data["error"],
        format!("step `go` failed: `worker` has 1 problem(s):\n{WRONG}\n{}", newer())
    );
    assert!(kept(&rig).await.is_empty());
    assert_eq!(rig.shared.count("worked"), 0);
}

#[tokio::test]
async fn a_text_that_checks_again_is_used_and_remembered_and_nothing_more_is_said() {
    let rig = Rig::new().await;
    watching(&rig).await;
    rig.library.put("worker", NEWER_WORKER);
    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;
    assert_eq!(kept(&rig).await.len(), 1);

    rig.library.put("worker", LATER_WORKER);
    moved(&rig, 3).await;
    handled(&rig, "on.ready", 3).await;
    assert_eq!((rig.shared.count("worked"), rig.shared.count("worked-later")), (2, 1));
    assert_eq!(kept(&rig).await.len(), 1, "a text that checks is called and nothing is said");

    // And it is now the one gone on with.
    rig.library.put("worker", NEWER_WORKER);
    moved(&rig, 4).await;
    handled(&rig, "on.ready", 4).await;
    assert_eq!((rig.shared.count("worked"), rig.shared.count("worked-later")), (2, 2));
    let said = kept(&rig).await;
    assert_eq!(said.len(), 2);
    assert_eq!(said[1]["kept"], blueprint::revision(LATER_WORKER));
}

#[tokio::test]
async fn the_calling_instance_says_of_itself_what_it_went_on_with_until_the_text_checks_again() {
    let mut rig = Rig::new().await;
    let watcher = watching(&rig).await;
    rig.library.put("worker", NEWER_WORKER);
    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;
    let says = format!(
        "`worker` cannot be applied here as it is now, so `go` went on with it as it last checked.\n\
         `worker` has 1 problem(s):\n{WRONG}\n{}",
        newer()
    );
    let instance = rig.instance(&watcher).await;
    assert_eq!((instance.status, instance.error.as_deref()), (InstanceStatus::Active, Some(says.as_str())));

    // It is still said after a restart, and taken back by whichever
    // conductor finds the text checking again.
    rig.restart().await;
    assert_eq!(rig.instance(&watcher).await.error.as_deref(), Some(says.as_str()));
    rig.library.put("worker", LATER_WORKER);
    moved(&rig, 3).await;
    handled(&rig, "on.ready", 3).await;
    let instance = rig.instance(&watcher).await;
    assert_eq!((instance.status, instance.error), (InstanceStatus::Active, None));
}

#[tokio::test]
async fn a_text_that_checks_and_then_fails_is_a_failure_like_any_other() {
    let rig = Rig::new().await;
    watching(&rig).await;
    rig.library.put(
        "worker",
        "blueprint: worker\nparams: { id: 0 }\nsteps:\n  work: { do: test.flaky, with: { message: no route to host } }\n",
    );
    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;
    let runs = rig.events("run.finished").await;
    let last = runs.iter().rfind(|run| run.data["flow"] == "on.ready").unwrap();
    assert_eq!(last.data["error"], "step `go` failed: step `work` failed: no route to host");
    assert!(kept(&rig).await.is_empty());
    assert_eq!(rig.shared.count("worked"), 1);
}

#[tokio::test]
async fn a_child_that_is_running_is_left_as_it_is_when_its_text_no_longer_checks() {
    let rig = Rig::new().await;
    let listener = "blueprint: listener\non:\n  told:\n    event: lane.wanted\n    steps:\n      note: { do: test.count, with: { key: told } }\n";
    rig.library.put("listener", listener);
    let parent = "blueprint: parent\non:\n  ready:\n    event: board.item.moved\n    steps:\n      go: { spawn: listener, as: the-listener }\n";
    let applied = rig.apply(parent, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let child = |rig: &Rig| {
        let parent = applied.instance.clone();
        let engine = rig.engine.clone();
        async move {
            let filter = InstanceFilter { parent: Some(parent), ..Default::default() };
            engine.instances(filter).await.unwrap().remove(0)
        }
    };
    // A spawn goes on without waiting for what it started: the child is
    // running once it says it is active, which may be after the parent's
    // run is over (read as starting in a gate held to four).
    let before = child(&rig).await;
    active(&rig, &before.id).await;

    rig.library.put("listener", &listener.replace("key: told", "key: told, loudly: true"));
    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;

    let after = child(&rig).await;
    assert_eq!((after.id.as_str(), after.status), (before.id.as_str(), InstanceStatus::Active));
    assert_eq!(after.revision, blueprint::revision(listener), "it runs the text it ran");
    assert!(rig.events("instance.updated,instance.update_failed").await.is_empty());
    assert_eq!(kept(&rig).await.len(), 1);
    let runs = rig.events("run.finished").await;
    assert!(runs.iter().all(|run| run.data["status"] == "ok"), "{runs:?}");
}

#[tokio::test]
async fn a_child_still_starting_is_left_as_it_is_when_its_text_no_longer_checks_and_comes_to_be_active_on_its_own()
 {
    let rig = Rig::new().await;
    // Its steps wait at a gate: it is starting until the gate is opened.
    let listener = "blueprint: listener\nsteps:\n  first: { do: test.gate, with: { name: begun } }\non:\n  told:\n    event: lane.wanted\n    steps:\n      note: { do: test.count, with: { key: told } }\n";
    rig.library.put("listener", listener);
    let parent = "blueprint: parent\non:\n  ready:\n    event: board.item.moved\n    steps:\n      go: { spawn: listener, as: the-listener }\n";
    let applied = rig.apply(parent, json!({})).await;
    active(&rig, &applied.instance).await;
    moved(&rig, 1).await;
    handled(&rig, "on.ready", 1).await;
    let filter = InstanceFilter { parent: Some(applied.instance.clone()), ..Default::default() };
    let before = rig.engine.instances(filter.clone()).await.unwrap().remove(0);
    assert_eq!(before.status, InstanceStatus::Starting);

    // Asked for again with a text that does not check while it is still
    // starting: it is left as it is, starting, on the text it began with.
    rig.library.put("listener", &listener.replace("key: told", "key: told, loudly: true"));
    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;
    let still = rig.engine.instances(filter.clone()).await.unwrap().remove(0);
    assert_eq!((still.id.as_str(), still.status), (before.id.as_str(), InstanceStatus::Starting));
    assert_eq!(still.revision, blueprint::revision(listener));
    assert_eq!(kept(&rig).await.len(), 1);

    // And once its steps are over it is active, on that text.
    rig.open("begun");
    active(&rig, &before.id).await;
    let after = rig.engine.instances(filter).await.unwrap().remove(0);
    assert_eq!((after.id.as_str(), after.status), (before.id.as_str(), InstanceStatus::Active));
    assert_eq!(after.revision, blueprint::revision(listener), "it runs the text it began with");
    assert!(rig.events("instance.updated,instance.update_failed").await.is_empty());
}

#[tokio::test]
async fn the_last_text_that_checked_is_still_kept_after_a_restart() {
    let mut rig = Rig::new().await;
    watching(&rig).await;
    rig.restart().await;
    rig.library.put("worker", NEWER_WORKER);
    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;
    assert_eq!(rig.shared.count("worked"), 2);
    assert_eq!(kept(&rig).await.len(), 1);
}

#[tokio::test]
async fn whether_the_text_a_step_called_checked_is_measured_by_the_blueprint_called() {
    let rig = Rig::new().await;
    watching(&rig).await;
    rig.library.put("worker", NEWER_WORKER);
    moved(&rig, 2).await;
    handled(&rig, "on.ready", 2).await;
    let query = MetricQuery {
        name: "blueprint.fit".into(),
        group_by: vec!["blueprint".into()],
        ..Default::default()
    };
    let series = rig.journal.query_metrics(query).await.unwrap();
    assert_eq!(series.len(), 1);
    assert_eq!(series[0].labels["blueprint"], "worker");
    assert_eq!((series[0].total.count, series[0].total.errors), (2, 1));
}
