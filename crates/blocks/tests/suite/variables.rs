//! The variables of a factory's profile: what its owner has said is
//! true of it, read by every blueprint there as `profile.vars`.

use conductor::{Apply, Library, RunInfo, RunStatus};
use kernel::principal::{AGENT_SCOPES, MEMBER_SCOPES};
use kernel::{ErrorKind, Principal, PrincipalKind, Source};
use serde_json::{Value, json};

use crate::handlers::{active, handled};
use crate::{Rig, step};

const READER: &str = "\
blueprint: reader
steps:
  read:
    do: test.echo
    with:
      api: \"${ profile.vars.api ?? 'http://localhost' }\"
      replicas: ${ profile.vars.replicas }
      careful: ${ profile.vars.careful }
      missing: ${ profile.vars.missing }
      all: ${ profile.vars }
";

/// The blueprint that ships for setting them, as the standard library has it.
fn standard() -> String {
    conductor::standard().load("std/profile", None).unwrap()
}

/// Applies it with `vars`, as the owner, and waits for it to finish.
async fn set(rig: &Rig, vars: Value) -> RunInfo {
    rig.run(&standard(), json!({ "vars": vars })).await
}

/// Every `system.profile` there has been, as what each says.
async fn said(rig: &Rig) -> Vec<Value> {
    rig.events("system.profile").await.into_iter().map(|event| event.data).collect()
}

#[tokio::test]
async fn a_blueprint_reads_a_variable_that_was_set_and_one_that_was_not_reads_as_nothing() {
    let rig = Rig::new().await;
    // Nothing has been set: the map is there and is empty, a name in it is
    // nothing, and `??` gives what to use instead.
    let run = rig.run(READER, json!({})).await;
    assert_eq!(
        step(&run, "read").output,
        json!({ "api": "http://localhost", "replicas": null, "careful": null, "missing": null, "all": {} })
    );

    let vars = json!({ "api": "https://api.example", "replicas": 3, "careful": true });
    assert_eq!(set(&rig, vars.clone()).await.status, RunStatus::Ok);
    // Each is what it was set as: text, a number, true.
    let run = rig.run(READER, json!({})).await;
    assert_eq!(
        step(&run, "read").output,
        json!({ "api": "https://api.example", "replicas": 3, "careful": true, "missing": null, "all": vars })
    );
}

#[tokio::test]
async fn applying_again_replaces_the_whole_set_and_says_what_changed_by_name_and_never_by_value() {
    let rig = Rig::new().await;
    let first = set(&rig, json!({ "api": "https://one.example", "replicas": 3 })).await;
    assert_eq!(
        first.outputs,
        json!({ "names": ["api", "replicas"], "set": ["api", "replicas"], "changed": [], "removed": [] })
    );

    // One changed, one new, and one left out, which is no longer set.
    let second = set(&rig, json!({ "api": "https://two.example", "region": "eu-west" })).await;
    assert_eq!(
        second.outputs,
        json!({ "names": ["api", "region"], "set": ["region"], "changed": ["api"], "removed": ["replicas"] })
    );
    assert_eq!(
        Value::Object(rig.engine.variables()),
        json!({ "api": "https://two.example", "region": "eu-west" })
    );

    // Each change is said once, with who made it, and by name only.
    let events = rig.events("system.profile").await;
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[1].data,
        json!({
            "set": ["region"], "changed": ["api"], "removed": ["replicas"],
            "by": "user:ada", "by_name": "Ada",
        })
    );
    assert_eq!(events[1].source.instance.as_deref(), Some(second.instance.as_str()));
    assert!(!format!("{:?}", said(&rig).await).contains("example"), "no value is in an event of its own");

    // Said again as it is, nothing changed and nothing is said.
    let same = set(&rig, json!({ "api": "https://two.example", "region": "eu-west" })).await;
    assert_eq!(same.outputs["changed"], json!([]));
    assert_eq!(said(&rig).await.len(), 2);

    // Applied with none, there are none.
    let cleared = rig.run(&standard(), json!({})).await;
    assert_eq!(
        cleared.outputs,
        json!({ "names": [], "set": [], "changed": [], "removed": ["api", "region"] })
    );
    assert!(rig.engine.variables().is_empty());
    let run = rig.run(READER, json!({})).await;
    assert_eq!(step(&run, "read").output["api"], "http://localhost");
}

#[tokio::test]
async fn a_value_that_is_not_plain_or_a_name_that_cannot_be_read_is_refused_in_a_sentence_and_nothing_changes()
 {
    let rig = Rig::new().await;
    set(&rig, json!({ "api": "https://api.example" })).await;

    for (vars, why) in [
        (
            json!({ "api": "x", "hosts": ["a", "b"] }),
            "the variable `hosts` is a list, and a variable is plain: text, a number, or true or false",
        ),
        (
            json!({ "database": { "host": "db" } }),
            "the variable `database` is a map, and a variable is plain",
        ),
        (json!({ "gone": null }), "the variable `gone` is nothing (null), and a variable is plain"),
        (
            json!({ "api-url": "x" }),
            "`api-url` cannot be the name of a variable. A name is letters, digits and `_`",
        ),
        (json!({ "2nd": "x" }), "`2nd` cannot be the name of a variable"),
        (json!({ "": "x" }), "`` cannot be the name of a variable"),
    ] {
        let run = set(&rig, vars.clone()).await;
        assert_eq!(run.status, RunStatus::Failed, "{vars}");
        let error = step(&run, "set").error.clone().unwrap();
        assert!(error.contains(why), "{vars}: {error}");
    }
    // What was set is still what there is, and nothing more was said.
    assert_eq!(Value::Object(rig.engine.variables()), json!({ "api": "https://api.example" }));
    assert_eq!(said(&rig).await.len(), 1);
}

#[tokio::test]
async fn only_whoever_holds_admin_sets_the_variables_and_a_blueprint_has_to_say_that_it_does() {
    let rig = Rig::new().await;
    assert!(!AGENT_SCOPES.contains(&"admin") && !MEMBER_SCOPES.contains(&"admin"));
    let scopes = |held: &[&str]| -> Vec<String> { held.iter().map(|scope| scope.to_string()).collect() };
    let agent = Principal {
        kind: PrincipalKind::Agent,
        id: "builder".into(),
        name: "builder".into(),
        // An agent whose slot says it may apply: by itself it starts nothing.
        scopes: scopes(AGENT_SCOPES).into_iter().chain(["blueprints:run".to_string()]).collect(),
        key: None,
    };
    let member = Principal {
        kind: PrincipalKind::User,
        id: "grace".into(),
        name: "Grace".into(),
        scopes: scopes(MEMBER_SCOPES),
        key: None,
    };
    for (who, name) in [(agent, "builder"), (member, "Grace")] {
        let request = rig.request(&standard(), json!({ "vars": { "api": "https://elsewhere.example" } }));
        let applied = rig.engine.apply(Apply { principal: who, ..request }).await.unwrap();
        let run = rig.finished(&applied).await;
        assert_eq!(run.status, RunStatus::Failed);
        assert_eq!(step(&run, "set").error.as_deref(), Some(format!("{name} may not use `admin`").as_str()));
    }
    assert!(rig.engine.variables().is_empty());
    assert!(said(&rig).await.is_empty());

    // Nor does a blueprint that does not say it takes `admin` get to.
    let quiet = "blueprint: quiet\npermissions: [fleet]\nsteps:\n  set: { do: profile.set, with: { vars: { a: 1 } } }\n";
    let err = rig.engine.apply(rig.request(quiet, json!({}))).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Invalid);
    assert!(
        err.message.contains("this uses `admin`, which is not listed under `permissions`"),
        "{}",
        err.message
    );
}

#[tokio::test]
async fn the_variables_are_there_when_the_conductor_starts_again_and_a_run_it_picks_up_reads_them() {
    let mut rig = Rig::new().await;
    set(&rig, json!({ "api": "https://api.example", "replicas": 3 })).await;
    let applied = rig
        .apply(
            "\
blueprint: slow
steps:
  build: { do: test.durable, with: { name: build } }
  read: { do: test.echo, needs: [build], with: { api: \"${ profile.vars.api }\" } }
",
            json!({}),
        )
        .await;
    rig.reached("build").await;

    rig.restart().await;
    // Held from the moment there is a conductor again, not from the first
    // time something asks.
    assert_eq!(Value::Object(rig.engine.variables()), json!({ "api": "https://api.example", "replicas": 3 }));
    rig.open("build");
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "read").output, json!({ "api": "https://api.example" }));
    assert_eq!(said(&rig).await.len(), 1, "starting again is not a change");
}

#[tokio::test]
async fn an_instance_that_is_running_reads_the_new_value_the_next_time_it_works_an_expression_out() {
    let rig = Rig::new().await;
    let applied = rig
        .apply(
            "\
blueprint: listener
steps:
  once: { do: test.count, with: { key: once } }
on:
  asked:
    event: reader.asked
    do: test.echo
    with: { api: \"${ profile.vars.api ?? 'not set' }\" }
",
            json!({}),
        )
        .await;
    active(&rig, &applied.instance).await;
    let answers = || async {
        let mut answers = Vec::new();
        for run in rig.engine.runs(&applied.instance, 10).await.unwrap().into_iter().rev() {
            if run.flow == "on.asked" {
                answers.push(rig.engine.run(&run.id).await.unwrap().steps[0].output["api"].clone());
            }
        }
        answers
    };
    rig.journal.emit("reader.asked", Source::default(), json!({})).await.unwrap();
    handled(&rig, "on.asked", 1).await;
    assert_eq!(answers().await, ["not set"]);

    // Setting them runs nothing again in an instance that is alive.
    set(&rig, json!({ "api": "https://api.example" })).await;
    assert_eq!(rig.shared.count("once"), 1);
    assert_eq!(
        rig.engine.runs(&applied.instance, 10).await.unwrap().len(),
        2,
        "its own, and the one asked for"
    );

    rig.journal.emit("reader.asked", Source::default(), json!({})).await.unwrap();
    handled(&rig, "on.asked", 2).await;
    assert_eq!(answers().await, ["not set", "https://api.example"]);
    rig.engine.stop(&applied.instance).await.unwrap();
}
