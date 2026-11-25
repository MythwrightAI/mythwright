use std::sync::{Arc, Mutex};

use conductor::{Engine, InstanceFilter, InstanceStatus, Registry, RunStatus, StepStatus};
use kernel::Source;
use serde_json::json;

use crate::{Rig, step};

const SERVICE: &str = "\
blueprint: svc
params: { version: 1 }
state: { deploys: 0, last: 0 }
steps:
  deploy: { do: set, with: { deploys: ${ state.deploys + 1 }, last: ${ params.version } } }
on:
  ping: { event: svc.ping, do: test.count, with: { key: \"pings-v${params.version}\" } }
";

#[tokio::test]
async fn applying_again_updates_the_same_instance_and_keeps_its_state() {
    let rig = Rig::new().await;
    let first = rig.apply(SERVICE, json!({})).await;
    rig.finished(&first).await;
    rig.event("instance.active", |_| true).await;

    // Nothing changed: nothing runs.
    let same = rig.apply(SERVICE, json!({})).await;
    assert_eq!(
        (same.instance.as_str(), same.run.is_none(), same.created, same.changed),
        (first.instance.as_str(), true, false, false)
    );

    // A new version: the same instance runs its steps again.
    let second = rig.apply(SERVICE, json!({ "version": 2 })).await;
    assert_eq!(
        (second.instance.as_str(), second.created, second.changed),
        (first.instance.as_str(), false, true)
    );
    rig.finished(&second).await;
    let instance = rig.instance(&first.instance).await;
    assert_eq!(instance.state, kernel::fields(json!({ "deploys": 2, "last": 2 })), "state carried over");
    assert_eq!(instance.params["version"], 2);
    assert_eq!(rig.engine.instances(InstanceFilter::default()).await.unwrap().len(), 1);

    // Its handlers now belong to the new revision.
    let updated = rig.event("instance.updated", |_| true).await;
    rig.event("instance.active", |event| event.seq > updated.seq).await;
    rig.journal.emit("svc.ping", Source::default(), json!({})).await.unwrap();
    rig.event("run.finished", |event| event.data["flow"] == "on.ping").await;
    assert_eq!((rig.shared.count("pings-v1"), rig.shared.count("pings-v2")), (0, 1));

    // `force` runs the steps again even though nothing changed.
    let forced = rig
        .engine
        .apply(conductor::Apply { force: true, ..rig.request(SERVICE, json!({ "version": 2 })) })
        .await
        .unwrap();
    rig.finished(&forced).await;
    assert_eq!(rig.instance(&first.instance).await.state["deploys"], 3);
}

#[tokio::test]
async fn a_second_key_is_a_second_instance() {
    let rig = Rig::new().await;
    let a = rig.apply(SERVICE, json!({})).await;
    let b = rig
        .engine
        .apply(conductor::Apply { key: Some("blue".into()), ..rig.request(SERVICE, json!({})) })
        .await
        .unwrap();
    assert_ne!(a.instance, b.instance);
    assert!(b.created);
}

#[tokio::test]
async fn an_instance_that_holds_something_stays_alive_and_gives_it_back_when_stopped() {
    let rig = Rig::new().await;
    let returned = Arc::new(Mutex::new(Vec::new()));
    rig.engine.on_stop({
        let returned = returned.clone();
        Arc::new(move |info| {
            let returned = returned.clone();
            Box::pin(async move { returned.lock().unwrap().push((info.blueprint, info.holds)) })
        })
    });
    let applied = rig
        .apply(
            "blueprint: keeper\nsteps:\n  take: { do: test.hold, with: { what: \"service:kv\" } }\n",
            json!({}),
        )
        .await;
    rig.finished(&applied).await;
    rig.event("instance.active", |_| true).await;
    assert_eq!(rig.instance(&applied.instance).await.holds, ["service:kv"]);

    rig.engine.stop(&applied.instance).await.unwrap();
    assert_eq!(rig.instance(&applied.instance).await.status, InstanceStatus::Stopped);
    assert_eq!(*returned.lock().unwrap(), [("keeper".to_string(), vec!["service:kv".to_string()])]);
    // Stopping twice is not an error.
    rig.engine.stop(&applied.instance).await.unwrap();
}

#[tokio::test]
async fn stopping_cancels_what_is_running_cleans_up_and_takes_the_children_too() {
    let rig = Rig::new().await;
    rig.library.put("listener", "blueprint: listener\non:\n  x: { event: a.b, do: noop }\n");
    let applied = rig
        .apply(
            "\
blueprint: parent
steps:
  kid: { spawn: listener }
  work: { do: test.gate, with: { name: never } }
  after: { do: test.echo }
on_stop:
  - emit: { topic: parent.cleanup, key: ${ instance.key } }
",
            json!({}),
        )
        .await;
    rig.reached("never").await;
    rig.engine.stop(&applied.instance).await.unwrap();

    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Cancelled);
    assert_eq!(step(&run, "work").status, StepStatus::Cancelled);
    assert_eq!(step(&run, "after").status, StepStatus::Cancelled);
    assert_eq!(rig.event("parent.cleanup", |_| true).await.data["key"], "parent");

    let kids = rig
        .engine
        .instances(InstanceFilter { parent: Some(applied.instance.clone()), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(kids[0].status, InstanceStatus::Stopped);
    let stopped: Vec<String> = rig
        .events("instance.stopped")
        .await
        .iter()
        .map(|event| event.data["blueprint"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(stopped, ["listener", "parent"], "children first");
    assert!(
        rig.engine
            .instances(InstanceFilter { live_only: true, ..Default::default() })
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn a_failed_instance_does_not_leave_its_children_running() {
    let rig = Rig::new().await;
    rig.library.put("listener", "blueprint: listener\non:\n  x: { event: a.b, do: noop }\n");
    let applied = rig
        .apply(
            "blueprint: doomed\nsteps:\n  kid: { spawn: listener }\n  bad: { do: test.flaky }\n",
            json!({}),
        )
        .await;
    rig.finished(&applied).await;
    rig.event("instance.failed", |_| true).await;
    rig.event("instance.stopped", |event| event.data["blueprint"] == "listener").await;
    assert!(
        rig.engine
            .instances(InstanceFilter { live_only: true, ..Default::default() })
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn after_a_restart_finished_steps_are_not_repeated_and_safe_ones_are() {
    let mut rig = Rig::new().await;
    let since = rig.clock.sleep_count();
    let applied = rig
        .apply(
            "\
blueprint: resumable
steps:
  once: { do: test.count, with: { key: once } }
  nap: { do: test.sleep, with: { for: 10m } }
  last: { do: test.count, with: { key: last } }
",
            json!({}),
        )
        .await;
    rig.soon("the nap to start", rig.clock.sleeps_started(since + 1)).await;

    rig.restart().await;
    let since = rig.clock.sleep_count();
    rig.advance_after(since, 1, "10m").await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(rig.shared.count("once"), 1, "it had finished before the restart");
    assert_eq!(
        rig.shared.calls().iter().filter(|call| *call == "nap:test.sleep").count(),
        2,
        "safe to repeat, so repeated"
    );
    assert_eq!(rig.shared.count("last"), 1);
    assert_eq!(rig.topics("run.*").await, ["run.started", "run.resumed", "run.finished"]);
}

#[tokio::test]
async fn after_a_restart_a_step_that_is_not_safe_to_repeat_fails_and_says_why() {
    let mut rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: fragile
steps:
  deploy:
    do: test.gate
    with: { name: deploy }
    on_fail:
      - emit: { topic: deploy.interrupted, why: ${ self.error } }
",
            json!({}),
        )
        .await;
    rig.reached("deploy").await;
    rig.restart().await;
    let run = rig.finished(&applied).await;

    let why = "was interrupted by a restart, and this action is not safe to repeat automatically";
    assert_eq!(step(&run, "deploy").error.as_deref(), Some(why));
    assert_eq!(rig.event("deploy.interrupted", |_| true).await.data["why"], why);
    assert_eq!(rig.shared.calls(), ["deploy:test.gate"], "it was not run a second time");
}

#[tokio::test]
async fn after_a_restart_a_step_with_a_checkpoint_is_picked_up_where_it_was() {
    let mut rig = Rig::new().await;
    let applied = rig
        .apply(
            "blueprint: adopted\nsteps:\n  build: { do: test.durable, with: { name: build } }\n",
            json!({}),
        )
        .await;
    rig.reached("build").await;
    rig.restart().await;
    rig.open("build");
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "build").output, json!({ "resumed_from": { "pid": 4242 } }));
    assert_eq!((rig.shared.count("durable:run"), rig.shared.count("durable:resume")), (1, 1));
}

#[tokio::test]
async fn an_instance_whose_blueprint_no_longer_checks_is_failed_not_guessed_at() {
    let rig = Rig::new().await;
    let applied = rig.apply(SERVICE, json!({})).await;
    rig.finished(&applied).await;
    rig.event("instance.active", |_| true).await;
    rig.engine.halt();

    // The next conductor has none of the actions this blueprint uses.
    let logs = journal::Logs::new(std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("unused"));
    let bare = Engine::new(rig.journal.clone(), logs, Registry::new(), rig.library.clone()).await.unwrap();
    assert_eq!(bare.resume().await.unwrap(), 0);
    let instance = bare.instance(&applied.instance).await.unwrap();
    assert_eq!(instance.status, InstanceStatus::Failed);
    assert!(
        instance.error.as_deref().unwrap().starts_with("cannot resume: `svc` has 1 problem(s)"),
        "{:?}",
        instance.error
    );
}

#[tokio::test]
async fn the_same_blueprint_applied_for_two_projects_is_two_instances() {
    let rig = Rig::new().await;
    // It stays alive, as a fleet does, so applying it again would find it.
    let source = "blueprint: fleet\nsteps:\n  up: { do: test.echo }\non:\n  tick:\n    event: nothing.ever\n    do: test.echo\n";
    let for_project =
        |project: &str| conductor::Apply { project: Some(project.into()), ..rig.request(source, json!({})) };
    let shop = rig.engine.apply(for_project("shop")).await.unwrap();
    let blog = rig.engine.apply(for_project("blog")).await.unwrap();
    assert_ne!(shop.instance, blog.instance, "each project has a fleet of its own");
    let of = async |id: &str| rig.engine.instance(id).await.unwrap().project;
    assert_eq!(
        (of(&shop.instance).await.as_deref(), of(&blog.instance).await.as_deref()),
        (Some("shop"), Some("blog"))
    );

    // Applied again for a project, it is that project's that is found.
    let again = rig.engine.apply(for_project("blog")).await.unwrap();
    assert_eq!(again.instance, blog.instance);
    // And one applied for no project is neither.
    let loose = rig.apply(source, json!({})).await;
    assert!(loose.instance != shop.instance && loose.instance != blog.instance);
}

#[tokio::test]
async fn a_failed_update_leaves_the_previous_version_running() {
    let rig = Rig::new().await;
    let stops = Arc::new(Mutex::new(0));
    rig.engine.on_stop({
        let stops = stops.clone();
        Arc::new(move |_| {
            let stops = stops.clone();
            Box::pin(async move { *stops.lock().unwrap() += 1 })
        })
    });
    let source = "\
blueprint: release
params: { version: 1, broken: false }
state: { deploys: 0 }
steps:
  hold: { do: test.hold, with: { what: \"service:x\" } }
  deploy: { when: ${ !params.broken }, do: set, with: { deploys: ${ state.deploys + 1 } } }
  boom: { when: ${ params.broken }, do: test.flaky, with: { message: bad release } }
on:
  ping: { event: release.ping, do: test.count, with: { key: \"pings-v${params.version}\" } }
";
    let first = rig.apply(source, json!({})).await;
    rig.finished(&first).await;

    let second = rig.apply(source, json!({ "version": 2, "broken": true })).await;
    let run = rig.finished(&second).await;
    assert_eq!(run.status, RunStatus::Failed);

    // Not failed, not torn down: back on version 1, saying what happened.
    let instance = rig.instance(&first.instance).await;
    assert_eq!(instance.status, InstanceStatus::Active);
    assert_eq!(instance.params["version"], 1);
    assert_eq!(instance.holds, ["service:x"]);
    assert_eq!(
        instance.error.as_deref(),
        Some("update failed, still running the previous version: step `boom` failed: bad release")
    );
    assert_eq!(*stops.lock().unwrap(), 0, "nothing it held was taken away");
    let failed = rig.event("instance.update_failed", |_| true).await;
    assert_eq!(failed.data["error"], "step `boom` failed: bad release");
    assert!(rig.events("instance.failed").await.is_empty());

    // Its handlers are the previous version's.
    rig.journal.emit("release.ping", Source::default(), json!({})).await.unwrap();
    rig.event("run.finished", |event| event.data["flow"] == "on.ping").await;
    assert_eq!((rig.shared.count("pings-v1"), rig.shared.count("pings-v2")), (1, 0));

    // And a good update still goes through afterwards.
    let third = rig.apply(source, json!({ "version": 3 })).await;
    assert_eq!(rig.finished(&third).await.status, RunStatus::Ok);
    let instance = rig.instance(&first.instance).await;
    assert_eq!((instance.params["version"].as_i64(), instance.error.as_deref()), (Some(3), None));
    assert_eq!(instance.state["deploys"], 2);
}

#[tokio::test]
async fn a_failed_update_puts_back_the_state_the_instance_had_with_the_text() {
    let mut rig = Rig::new().await;
    let one = "\
blueprint: counter
state: { builds: 0, room: none }
steps:
  build: { do: set, with: { builds: ${ state.builds + 1 }, room: general } }
on:
  ping: { event: counter.ping, do: test.count, with: { key: pings } }
";
    // The next text no longer keeps `room`, counts twice over, and fails.
    let two = "\
blueprint: counter
state: { builds: 0, rounds: 5 }
steps:
  build: { do: set, with: { builds: ${ state.builds + 10 }, rounds: 6 } }
  boom: { needs: [build], do: test.flaky, with: { message: bad text } }
on:
  ping: { event: counter.ping, do: test.count, with: { key: pings } }
";
    let first = rig.apply(one, json!({})).await;
    rig.finished(&first).await;
    let had = kernel::fields(json!({ "builds": 1, "room": "general" }));
    assert_eq!(rig.instance(&first.instance).await.state, had);

    let second = rig.apply(two, json!({})).await;
    assert_eq!(rig.finished(&second).await.status, RunStatus::Failed);
    let is = rig.instance(&first.instance).await;
    assert_eq!(is.status, InstanceStatus::Active);
    assert_eq!(is.revision, blueprint::revision(one));
    assert_eq!(is.state, had, "the key the failed text dropped is back, and what its steps set is undone");
    assert_eq!(is.previous, None, "and there is nothing left to go back to");

    // What runs next sees that state, and so does a conductor started later.
    let third = rig.apply(&one.replace("state.builds + 1", "state.builds + 2"), json!({})).await;
    rig.finished(&third).await;
    let now = kernel::fields(json!({ "builds": 3, "room": "general" }));
    assert_eq!(rig.instance(&first.instance).await.state, now);
    rig.restart().await;
    assert_eq!(rig.instance(&first.instance).await.state, now);
}

#[tokio::test]
async fn a_first_apply_that_fails_has_nothing_to_fall_back_to() {
    let rig = Rig::new().await;
    let applied = rig.apply("blueprint: stillborn\nsteps:\n  bad: { do: test.flaky }\n", json!({})).await;
    rig.finished(&applied).await;
    assert_eq!(rig.instance(&applied.instance).await.status, InstanceStatus::Failed);
}

#[test]
fn the_standard_library_is_well_formed() {
    use conductor::Library;
    let library = conductor::standard();
    let names = library.names(None);
    assert!(names.contains(&"std/slot".to_string()) && names.contains(&"std/canary-stage".to_string()));
    for name in names {
        let source = library.load(&name, None).unwrap();
        let checked = blueprint::check(&source, &blueprint::NoCatalog);
        let errors: Vec<String> = checked.errors().map(|d| d.render(&name)).collect();
        assert_eq!(errors, Vec::<String>::new());
        assert_eq!(checked.blueprint.unwrap().name, name, "a blueprint is filed under its own name");
    }
}

/// Holds something named for its version and starts a helper named for
/// it; told to, it then stops at a gate, or fails.
const HOLDER: &str = "\
blueprint: holder
params: { version: 1, broken: false, held_up: false }
steps:
  hold: { do: test.hold, with: { what: \"service:v${ params.version }\" } }
  helper: { spawn: helper, as: \"helper-${ params.version }\" }
  gate: { when: ${ params.held_up }, do: test.gate, with: { name: held-up } }
  boom: { when: ${ params.broken }, do: test.flaky, with: { message: bad release } }
";

/// Something that stays: it has a handler.
const HELPER: &str = "blueprint: helper\non:\n  ping: { event: helper.ping, do: noop }\n";

/// What every stop hook was handed to stop, in order.
fn handed_back(rig: &Rig) -> Arc<Mutex<Vec<Vec<String>>>> {
    let handed: Arc<Mutex<Vec<Vec<String>>>> = Arc::default();
    rig.engine.on_stop({
        let handed = handed.clone();
        Arc::new(move |info| {
            let handed = handed.clone();
            Box::pin(async move { handed.lock().unwrap().push(info.holds) })
        })
    });
    handed
}

async fn live_keys(rig: &Rig, parent: &str) -> Vec<String> {
    let filter = InstanceFilter { parent: Some(parent.to_string()), live_only: true, ..Default::default() };
    let mut keys: Vec<String> =
        rig.engine.instances(filter).await.unwrap().into_iter().map(|child| child.key).collect();
    keys.sort();
    keys
}

#[tokio::test]
async fn a_failed_update_gives_back_what_it_came_to_hold_and_stops_the_children_it_made() {
    let rig = Rig::new().await;
    rig.library.put("helper", HELPER);
    let handed = handed_back(&rig);
    let first = rig.apply(HOLDER, json!({})).await;
    rig.finished(&first).await;
    assert_eq!(rig.instance(&first.instance).await.holds, ["service:v1"]);
    assert_eq!(live_keys(&rig, &first.instance).await, ["helper-1"]);

    let second = rig.apply(HOLDER, json!({ "version": 2, "broken": true })).await;
    assert_eq!(rig.finished(&second).await.status, RunStatus::Failed);
    rig.event("instance.update_failed", |_| true).await;

    let is = rig.instance(&first.instance).await;
    assert_eq!((is.status, is.params["version"].as_i64()), (InstanceStatus::Active, Some(1)));
    assert_eq!(is.holds, ["service:v1"], "what the failed text came to hold is not held any more");
    let stopped: Vec<Vec<String>> =
        handed.lock().unwrap().iter().filter(|holds| !holds.is_empty()).cloned().collect();
    assert_eq!(
        stopped,
        [["service:v2"]],
        "it was handed back to be stopped, and what was held before was not"
    );
    assert_eq!(
        live_keys(&rig, &first.instance).await,
        ["helper-1"],
        "the child the failed text made is stopped"
    );

    // A good update afterwards keeps what it starts, beside what was there.
    let third = rig.apply(HOLDER, json!({ "version": 3 })).await;
    assert_eq!(rig.finished(&third).await.status, RunStatus::Ok);
    assert_eq!(rig.instance(&first.instance).await.holds, ["service:v1", "service:v3"]);
    assert_eq!(live_keys(&rig, &first.instance).await, ["helper-1", "helper-3"]);
}

#[tokio::test]
async fn an_update_that_fails_after_a_restart_still_gives_back_what_it_had_started() {
    let mut rig = Rig::new().await;
    rig.library.put("helper", HELPER);
    let first = rig.apply(HOLDER, json!({})).await;
    rig.finished(&first).await;

    // The update holds its own, starts its helper, and is caught at a step
    // that cannot be picked up: the conductor stops there.
    let second = rig.apply(HOLDER, json!({ "version": 2, "held_up": true })).await;
    rig.reached("held-up").await;
    assert_eq!(rig.instance(&first.instance).await.holds, ["service:v1", "service:v2"]);
    rig.restart().await;
    let handed = handed_back(&rig);
    let run = rig.finished(&second).await;
    assert_eq!(run.status, RunStatus::Failed, "{:?}", run.error);
    rig.event("instance.update_failed", |_| true).await;

    let is = rig.instance(&first.instance).await;
    assert_eq!((is.status, is.params["version"].as_i64()), (InstanceStatus::Active, Some(1)));
    assert_eq!(is.holds, ["service:v1"]);
    let stopped: Vec<Vec<String>> =
        handed.lock().unwrap().iter().filter(|holds| !holds.is_empty()).cloned().collect();
    assert_eq!(stopped, [["service:v2"]]);
    assert_eq!(live_keys(&rig, &first.instance).await, ["helper-1"]);
}
