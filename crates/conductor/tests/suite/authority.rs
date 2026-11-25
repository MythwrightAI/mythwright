//! Whose authority an instance has: that of whoever applied the text it is
//! running, the first time or over what was there.
//!
//! `test.shell` stands for anything only some may do: it needs `shell`,
//! and says who it ran as.

use conductor::{
    Applied, Apply, InstanceFilter, InstanceInfo, InstanceStatus, RunInfo, RunStatus, StepStatus,
};
use kernel::{ErrorKind, Principal, PrincipalKind, Source};
use serde_json::json;

use crate::{Rig, step, user};

/// What the owner applies: it says what it needs, and goes on needing it
/// for as long as it lives.
const OWNERS: &str = "\
blueprint: svc
permissions: [shell]
params: { version: 1 }
steps:
  run: { do: test.shell }
on:
  poke:
    event: svc.poke
    steps:
      run: { do: test.shell }
";

/// The same service by someone who lists nothing, so it may use whatever
/// its applier may: its steps need nothing, its handler needs `shell`.
const QUIET: &str = "\
blueprint: svc
steps:
  note: { do: test.echo, with: { v: quiet } }
on:
  poke:
    event: svc.poke
    steps:
      run: { do: test.shell }
";

/// An agent as agents are unless given more: it may apply blueprints, and
/// may not run a shell.
fn builder() -> Principal {
    Principal {
        kind: PrincipalKind::Agent,
        id: "builder".into(),
        name: "builder".into(),
        scopes: vec!["blueprints".into()],
        key: None,
    }
}

fn by(who: Principal, rig: &Rig, source: &str) -> Apply {
    Apply { principal: who, ..rig.request(source, json!({})) }
}

/// Publishes what the handlers here wait for, and says where that came
/// among everything written.
async fn poke(rig: &Rig) -> u64 {
    rig.came(rig.journal.emit("svc.poke", Source::default(), json!({})).await.unwrap().seq)
}

/// The run by which `instance` answered the poke at `seq`, once it has finished.
async fn answered(rig: &Rig, instance: &str, seq: u64) -> RunInfo {
    let finished = rig
        .event("run.finished", |event| {
            event.seq > seq
                && event.data["flow"] == "on.poke"
                && event.source.instance.as_deref() == Some(instance)
        })
        .await;
    rig.engine.run(finished.source.run.as_deref().unwrap()).await.unwrap()
}

/// Pokes, and gives back the run of `instance`'s handler that answered.
async fn poked(rig: &Rig, instance: &str) -> RunInfo {
    let seq = poke(rig).await;
    answered(rig, instance, seq).await
}

/// The owner's service, applied and settled.
async fn owned(rig: &Rig) -> Applied {
    let applied = rig.apply(OWNERS, json!({})).await;
    assert_eq!(rig.finished(&applied).await.status, RunStatus::Ok);
    applied
}

async fn acts_as(rig: &Rig, instance: &str) -> String {
    rig.instance(instance).await.principal.name
}

/// `(version, who applied it, how it went, what it ran with)`, the first first.
async fn history(rig: &Rig, instance: &str) -> Vec<(u32, String, String, Vec<String>)> {
    let mut versions = rig.engine.versions(instance, 100).await.unwrap();
    versions.reverse();
    versions
        .into_iter()
        .map(|version| {
            let ran_with = version.principal.map(|principal| principal.scopes).unwrap_or_default();
            (version.version, version.by, version.outcome, ran_with)
        })
        .collect()
}

#[tokio::test]
async fn an_apply_over_by_someone_who_lacks_what_the_blueprint_lists_is_refused_and_changes_nothing() {
    let rig = Rig::new().await;
    let first = owned(&rig).await;
    let before = rig.instance(&first.instance).await;

    let theirs = OWNERS.replace("version: 1", "version: 2");
    let labels = [("experiment".to_string(), "theirs".to_string())].into_iter().collect();
    let refused = rig.engine.apply(Apply { labels, ..by(builder(), &rig, &theirs) }).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Denied);
    assert_eq!(
        refused.message,
        "builder may not use `shell`, which `svc` lists under `permissions`, so builder cannot apply it \
         over `svc`, which is running, and nothing was changed. Someone who holds `shell` can apply it."
    );

    let after = rig.instance(&first.instance).await;
    assert_eq!(
        (after.revision, after.params, after.status, after.updated, after.principal, after.labels),
        (before.revision, before.params, InstanceStatus::Active, before.updated, user(), before.labels)
    );
    assert_eq!(history(&rig, &first.instance).await.len(), 1, "it was not written down as a version");
    assert!(rig.events("instance.updated").await.is_empty());
    assert_eq!(rig.engine.runs(&first.instance, 10).await.unwrap().len(), 1, "and nothing ran");

    // It is still listening, as the owner.
    let run = poked(&rig, &first.instance).await;
    assert_eq!(step(&run, "run").output, json!({ "ran_as": "Ada" }));
}

#[tokio::test]
async fn an_apply_over_that_lists_nothing_cannot_do_what_its_applier_may_not_and_the_owners_version_is_kept()
{
    let rig = Rig::new().await;
    let first = owned(&rig).await;

    // The owner's instance, the agent's text: it runs as the agent.
    let theirs =
        "blueprint: svc\nsteps:\n  note: { do: test.echo }\n  run: { needs: [note], do: test.shell }\n";
    let second = rig.engine.apply(by(builder(), &rig, theirs)).await.unwrap();
    assert_eq!((second.instance.as_str(), second.created), (first.instance.as_str(), false));
    let run = rig.finished(&second).await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(step(&run, "run").error.as_deref(), Some("builder may not use `shell`"));
    assert_eq!(rig.shared.calls(), ["run:test.shell", "note:test.echo"], "the shell ran once, for the owner");

    // The owner's version is back, with the owner's authority.
    let failed = rig.event("instance.update_failed", |_| true).await;
    assert_eq!(failed.source.actor.as_deref(), Some("user:ada"));
    let instance = rig.instance(&first.instance).await;
    assert_eq!((instance.status, instance.principal.clone()), (InstanceStatus::Active, user()));
    assert_eq!(
        instance.error.as_deref(),
        Some(
            "update failed, still running the previous version: step `run` failed: builder may not use `shell`"
        )
    );
    let run = poked(&rig, &first.instance).await;
    assert_eq!(step(&run, "run").output, json!({ "ran_as": "Ada" }));

    assert_eq!(
        history(&rig, &first.instance).await,
        [
            (1, "user:ada".to_string(), "applied".to_string(), vec!["*".to_string()]),
            (2, "agent:builder".to_string(), "reverted".to_string(), vec!["blueprints".to_string()]),
        ]
    );
}

#[tokio::test]
async fn an_instance_acts_as_whoever_applied_over_it_last_and_the_history_says_who_that_was() {
    let rig = Rig::new().await;
    let first = owned(&rig).await;

    // What the agent applies needs nothing the agent lacks, so it takes.
    let second = rig.engine.apply(by(builder(), &rig, QUIET)).await.unwrap();
    assert_eq!(rig.finished(&second).await.status, RunStatus::Ok);
    assert_eq!(acts_as(&rig, &first.instance).await, "builder");
    let updated = rig.event("instance.updated", |_| true).await;
    assert_eq!(updated.source.actor.as_deref(), Some("agent:builder"));

    // Its handler is the agent's text, and may do what the agent may.
    let run = poked(&rig, &first.instance).await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(step(&run, "run").error.as_deref(), Some("builder may not use `shell`"));

    // Asking for what is already running changes nothing, whoever asks:
    // it goes on as whoever applied it.
    for who in [builder(), user()] {
        let again = rig.engine.apply(by(who, &rig, QUIET)).await.unwrap();
        assert_eq!((again.changed, again.run), (false, None));
        assert_eq!(acts_as(&rig, &first.instance).await, "builder");
    }

    // The owner applying over it makes it the owner's again.
    let third = rig.engine.apply(by(user(), &rig, OWNERS)).await.unwrap();
    assert_eq!(
        (third.instance.as_str(), third.created, third.changed),
        (first.instance.as_str(), false, true)
    );
    assert_eq!(rig.finished(&third).await.status, RunStatus::Ok);
    assert_eq!(rig.instance(&first.instance).await.principal, user());
    let run = poked(&rig, &first.instance).await;
    assert_eq!(step(&run, "run").output, json!({ "ran_as": "Ada" }));

    assert_eq!(
        history(&rig, &first.instance).await,
        [
            (1, "user:ada".to_string(), "applied".to_string(), vec!["*".to_string()]),
            (2, "agent:builder".to_string(), "applied".to_string(), vec!["blueprints".to_string()]),
            (3, "user:ada".to_string(), "applied".to_string(), vec!["*".to_string()]),
        ]
    );
}

#[tokio::test]
async fn whose_authority_an_instance_has_is_the_same_after_the_conductor_is_started_again() {
    let mut rig = Rig::new().await;
    let first = owned(&rig).await;
    let second = rig.engine.apply(by(builder(), &rig, QUIET)).await.unwrap();
    rig.finished(&second).await;

    rig.restart().await;
    assert_eq!(acts_as(&rig, &first.instance).await, "builder");
    let run = poked(&rig, &first.instance).await;
    assert_eq!(step(&run, "run").error.as_deref(), Some("builder may not use `shell`"));

    // The same text, which the owner asks to have run again all the same:
    // that is an update, and the owner's.
    let third = rig.engine.apply(Apply { force: true, ..by(user(), &rig, QUIET) }).await.unwrap();
    rig.finished(&third).await;
    rig.restart().await;
    assert_eq!(acts_as(&rig, &first.instance).await, "Ada");
    let run = poked(&rig, &first.instance).await;
    assert_eq!(step(&run, "run").output, json!({ "ran_as": "Ada" }));
}

#[tokio::test]
async fn an_update_that_fails_gives_the_instance_back_the_authority_of_the_version_it_keeps() {
    let rig = Rig::new().await;
    let first = owned(&rig).await;

    // An update of the agent's that fails for a reason of its own.
    let broken = format!("{QUIET}on_fail:\n  - do: test.shell\n").replace(
        "  note: { do: test.echo, with: { v: quiet } }",
        "  note: { do: test.flaky, with: { message: bad release } }",
    );
    let second = rig.engine.apply(by(builder(), &rig, &broken)).await.unwrap();
    assert_eq!(rig.finished(&second).await.status, RunStatus::Failed);
    let said = rig.event("hook.failed", |_| true).await;
    assert_eq!(
        said.data["error"], "builder may not use `shell`",
        "what it does on failing it does as the agent"
    );
    assert_eq!(rig.instance(&first.instance).await.principal, user());
    let run = poked(&rig, &first.instance).await;
    assert_eq!(step(&run, "run").output, json!({ "ran_as": "Ada" }));
}

#[tokio::test]
async fn an_update_cut_short_by_a_restart_goes_back_to_the_authority_that_was_kept_for_it() {
    let mut rig = Rig::new().await;
    let first = owned(&rig).await;

    // While the agent's update is under way the instance is the agent's.
    let parked = "blueprint: svc\nsteps:\n  wait: { do: test.gate, with: { name: door } }\n";
    rig.engine.apply(by(builder(), &rig, parked)).await.unwrap();
    rig.reached("door").await;
    assert_eq!(acts_as(&rig, &first.instance).await, "builder");
    rig.restart().await;
    rig.event("instance.update_failed", |event| {
        event.data["error"].as_str().is_some_and(|error| error.contains("interrupted by a restart"))
    })
    .await;
    assert_eq!(rig.instance(&first.instance).await.principal, user());
    let run = poked(&rig, &first.instance).await;
    assert_eq!(step(&run, "run").output, json!({ "ran_as": "Ada" }));
    let outcomes: Vec<String> = history(&rig, &first.instance)
        .await
        .into_iter()
        .map(|(_, by, outcome, _)| format!("{by} {outcome}"))
        .collect();
    assert_eq!(outcomes, ["user:ada applied", "agent:builder reverted"]);
}

#[tokio::test]
async fn a_run_under_way_when_its_instance_is_applied_over_is_ended_and_does_nothing_as_the_new_applier() {
    let rig = Rig::new().await;
    // The agent's text would run a shell once it is let through the door.
    let theirs = "\
blueprint: svc
steps:
  wait: { do: test.gate, with: { name: door } }
  then: { needs: [wait], do: test.shell }
";
    let first = rig.engine.apply(by(builder(), &rig, theirs)).await.unwrap();
    rig.reached("door").await;

    let second = rig.apply(OWNERS, json!({})).await;
    assert_eq!(second.instance, first.instance);
    assert_eq!(rig.finished(&second).await.status, RunStatus::Ok);
    rig.open("door");

    let run = rig.finished(&first).await;
    assert_eq!(run.status, RunStatus::Cancelled);
    assert_eq!(
        (step(&run, "wait").status, step(&run, "then").status),
        (StepStatus::Cancelled, StepStatus::Cancelled)
    );
    let shells: Vec<String> =
        rig.shared.calls().into_iter().filter(|call| call.ends_with(":test.shell")).collect();
    assert_eq!(shells, ["run:test.shell"], "only the owner's own step ran a shell");
    assert_eq!(rig.instance(&first.instance).await.principal, user());
}

#[tokio::test]
async fn what_an_instance_does_as_it_stops_it_does_as_whoever_applied_it_last() {
    let rig = Rig::new().await;
    let first = owned(&rig).await;
    let theirs = format!("{QUIET}on_stop:\n  - do: test.shell\n");
    let second = rig.engine.apply(by(builder(), &rig, &theirs)).await.unwrap();
    rig.finished(&second).await;

    rig.engine.stop(&first.instance).await.unwrap();
    let said = rig.event("hook.failed", |_| true).await;
    assert_eq!(said.data["error"], "builder may not use `shell`");
    let shells = rig.shared.calls().into_iter().filter(|call| call.ends_with(":test.shell")).count();
    assert_eq!(shells, 1, "the owner's first run, and nothing since");
}

/// A child that goes on living, and tries what needs `shell` when it
/// starts and whenever it is poked.
const WORKER: &str = "\
blueprint: worker
params: { n: 1 }
steps:
  try: { do: test.shell, allow_failure: true }
on:
  poke:
    event: svc.poke
    steps:
      run: { do: test.shell }
";

async fn children(rig: &Rig, parent: &str) -> Vec<InstanceInfo> {
    let mut children = rig
        .engine
        .instances(InstanceFilter { parent: Some(parent.to_string()), ..Default::default() })
        .await
        .unwrap();
    children.sort_by(|a, b| a.key.cmp(&b.key));
    children
}

/// How the child's latest run of its own steps went at `try`, once it has finished.
async fn tried(rig: &Rig, child: &InstanceInfo) -> (usize, Option<String>) {
    let runs = rig.engine.runs(&child.id, 10).await.unwrap();
    let mains: Vec<&RunInfo> = runs.iter().filter(|run| run.flow == "main").collect();
    let latest = rig.soon("the child's steps to finish", rig.engine.wait_run(&mains[0].id)).await.unwrap();
    (mains.len(), step(&latest, "try").error.clone())
}

#[tokio::test]
async fn a_child_started_or_changed_by_an_apply_over_acts_as_its_applier_and_one_left_alone_keeps_what_it_had()
 {
    let rig = Rig::new().await;
    rig.library.put("worker", WORKER);
    let first = rig
        .apply(
            "blueprint: crew\nsteps:\n  a: { spawn: worker, as: kept }\n  b: { spawn: worker, as: changed }\n",
            json!({}),
        )
        .await;
    rig.finished(&first).await;

    // The agent's text says nothing of `kept`, gives `changed` other
    // params, and starts `fresh`.
    let theirs = "\
blueprint: crew
steps:
  b: { spawn: worker, as: changed, with: { n: 2 } }
  c: { spawn: worker, as: fresh }
";
    let second = rig.engine.apply(by(builder(), &rig, theirs)).await.unwrap();
    assert_eq!(rig.finished(&second).await.status, RunStatus::Ok);

    let kids = children(&rig, &first.instance).await;
    let (changed, fresh, kept) = (&kids[0], &kids[1], &kids[2]);
    assert_eq!((changed.key.as_str(), fresh.key.as_str(), kept.key.as_str()), ("changed", "fresh", "kept"));
    assert_eq!(tried(&rig, kept).await, (1, None), "not run again, and its one run was the owner's");
    assert_eq!(tried(&rig, changed).await, (2, Some("builder may not use `shell`".into())));
    assert_eq!(tried(&rig, fresh).await, (1, Some("builder may not use `shell`".into())));
    assert_eq!(
        [&kept.principal.name, &changed.principal.name, &fresh.principal.name],
        ["Ada", "builder", "builder"]
    );
    assert_eq!(changed.params["n"], 2);

    // And so for as long as each lives.
    let seq = poke(&rig).await;
    assert_eq!(step(&answered(&rig, &kept.id, seq).await, "run").output, json!({ "ran_as": "Ada" }));
    for child in [changed, fresh] {
        let run = answered(&rig, &child.id, seq).await;
        assert_eq!(step(&run, "run").error.as_deref(), Some("builder may not use `shell`"));
    }
}

#[tokio::test]
async fn a_child_that_lists_what_the_new_applier_lacks_is_not_applied_over_and_the_update_is_undone() {
    let rig = Rig::new().await;
    rig.library.put("worker", &WORKER.replace("params:", "permissions: [shell]\nparams:"));
    let first = rig.apply("blueprint: crew\nsteps:\n  a: { call: worker, as: one }\n", json!({})).await;
    assert_eq!(rig.finished(&first).await.status, RunStatus::Ok);

    let theirs = "blueprint: crew\nsteps:\n  a: { call: worker, as: one, with: { n: 2 } }\n";
    let second = rig.engine.apply(by(builder(), &rig, theirs)).await.unwrap();
    let run = rig.finished(&second).await;
    assert_eq!(run.status, RunStatus::Failed);
    let error = step(&run, "a").error.clone().unwrap();
    assert!(
        error.starts_with("builder may not use `shell`, which `worker` lists under `permissions`"),
        "{error}"
    );

    let kids = children(&rig, &first.instance).await;
    assert_eq!((kids.len(), kids[0].params["n"].as_i64(), kids[0].principal.clone()), (1, Some(1), user()));
    assert_eq!(tried(&rig, &kids[0]).await, (1, None));
    assert_eq!(rig.instance(&first.instance).await.principal, user(), "and the parent is the owner's again");
}

#[tokio::test]
async fn a_database_from_before_each_version_kept_its_authority_still_opens_and_its_instances_act_as_they_did()
 {
    let mut rig = Rig::new().await;
    let first = owned(&rig).await;
    // An update of the owner's that is under way when the conductor stops.
    rig.apply("blueprint: svc\nsteps:\n  wait: { do: test.gate, with: { name: door } }\n", json!({})).await;
    rig.reached("door").await;
    rig.engine.halt();

    // The tables as the build before left them: no authority with a
    // version, none with what an update falls back to. (Nor what came
    // after that build: the table of what a loop's elements gave, how far
    // what is done around a step has come, what an instance that failed
    // still owes, how far what one does as it stops has come, what one
    // is to be carried over to, whose each row is, which of its run's
    // steps a step was to begin.)
    rig.journal
        .db()
        .call(|connection| {
            connection.execute_batch(crate::ordered::BEFORE_ASKERS_WERE_KEPT)?;
            connection.execute_batch(crate::ordered::BEFORE_ENVIRONMENTS)?;
            connection.execute_batch(crate::ordered::BEFORE_BEGINNINGS_WERE_WRITTEN)?;
            connection.execute_batch(crate::ordered::BEFORE_RUNS_KEPT_THEIR_PLACE)?;
            connection.execute_batch(crate::ordered::BEFORE_TEXTS_WERE_KEPT_BY_VERSION)?;
            connection.execute_batch(crate::ordered::BEFORE_STEPS_SAID_THEIR_PLACE)?;
            connection.execute_batch(crate::BEFORE_KEPT_BESIDE)?;
            connection.execute_batch(crate::BEFORE_ROWS_WERE_APART)?;
            connection.execute_batch(
                "ALTER TABLE instances DROP COLUMN heard;
                 ALTER TABLE runs DROP COLUMN carries;
                 ALTER TABLE instances DROP COLUMN hooks;
                 DROP INDEX instances_owing;
                 ALTER TABLE instances DROP COLUMN owes;
                 DROP TABLE step_elements;
                 ALTER TABLE steps DROP COLUMN hooks;
                 ALTER TABLE runs DROP COLUMN hooks;
                 ALTER TABLE instance_versions DROP COLUMN principal;
                 UPDATE instances SET previous = json_remove(previous, '$.principal');
                 UPDATE schema_versions SET version = 4 WHERE namespace = 'conductor';",
            )
        })
        .await
        .unwrap();

    rig.restart().await;
    rig.event("instance.update_failed", |_| true).await;
    let instance = rig.instance(&first.instance).await;
    assert_eq!((instance.status, instance.principal), (InstanceStatus::Active, user()));
    let run = poked(&rig, &first.instance).await;
    assert_eq!(step(&run, "run").output, json!({ "ran_as": "Ada" }));
    // Every version there was ran with what the instance had.
    assert_eq!(
        history(&rig, &first.instance).await,
        [
            (1, "user:ada".to_string(), "applied".to_string(), vec!["*".to_string()]),
            (2, "user:ada".to_string(), "reverted".to_string(), vec!["*".to_string()]),
        ]
    );

    // And from here on an apply over it is an apply like any other.
    let third = rig.engine.apply(by(builder(), &rig, QUIET)).await.unwrap();
    rig.finished(&third).await;
    assert_eq!(acts_as(&rig, &first.instance).await, "builder");
}
