//! A blueprint reads the profile of the factory it runs in, and, beside it,
//! the environment of its project's it was applied under.

use conductor::{RunStatus, StepStatus};
use serde_json::json;

use crate::{Rig, step};

const CAREFUL: &str = "\
blueprint: careful
state:
  where: \"${ profile.name }\"
steps:
  loose: { do: test.echo, when: ${ profile.name != \"prod\" }, with: { said: let loose } }
  hand:
    do: test.echo
    with: { agent: \"${ profile.name == 'prod' ? 'careful' : 'builder' }\", all: ${ profile } }
outputs:
  agent: ${ steps.hand.out.agent }
";

#[tokio::test]
async fn a_blueprint_reads_what_its_factory_is_called_in_a_guard_and_in_a_value() {
    let rig = Rig::new().await;
    rig.engine.in_profile("prod");
    let applied = rig.apply(CAREFUL, json!({})).await;
    let run = rig.finished(&applied).await;

    assert_eq!(run.status, RunStatus::Ok);
    assert_eq!(step(&run, "loose").status, StepStatus::Skipped, "not in prod");
    assert_eq!(run.outputs, json!({ "agent": "careful" }));
    // The profile is a map, of which what it is called is one part; the
    // other is its variables, of which none has been set.
    assert_eq!(step(&run, "hand").output["all"], json!({ "name": "prod", "vars": {} }));
    assert_eq!(rig.instance(&applied.instance).await.state["where"], "prod");
}

#[tokio::test]
async fn the_same_blueprint_does_otherwise_in_a_factory_called_something_else() {
    let rig = Rig::new().await;
    rig.engine.in_profile("dev1");
    let run = rig.run(CAREFUL, json!({})).await;

    assert_eq!(step(&run, "loose").status, StepStatus::Ok);
    assert_eq!(run.outputs, json!({ "agent": "builder" }));
}

#[tokio::test]
async fn a_factory_with_no_profile_is_called_the_empty_text() {
    // Nobody has said what it is called: it is a factory as they all were.
    let rig = Rig::new().await;
    let run = rig
        .run(
            "\
blueprint: unnamed
steps:
  named: { do: test.echo, when: ${ profile.name != \"\" } }
  said: { do: test.echo, with: { name: \"[${ profile.name }]\", length: ${ len(profile.name) } } }
",
            json!({}),
        )
        .await;

    assert_eq!(step(&run, "named").status, StepStatus::Skipped);
    assert_eq!(step(&run, "said").output, json!({ "name": "[]", "length": 0 }));
}

#[tokio::test]
async fn the_environment_is_none_and_never_the_profile_until_a_project_says_its_environments() {
    // Before 0.1.0 `environment` was what `profile` is now. In a factory
    // with a profile it reads none of it: it is the environment of the
    // project's an instance was applied under, and nothing names one yet.
    let rig = Rig::new().await;
    rig.engine.in_profile("prod");
    let run = rig
        .run(
            "\
blueprint: staged
steps:
  staged: { do: test.echo, when: ${ environment.name == \"prod\" } }
  said: { do: test.echo, with: { all: ${ environment }, api: \"[${ environment.vars.api ?? '' }]\" } }
",
            json!({}),
        )
        .await;

    assert_eq!(step(&run, "staged").status, StepStatus::Skipped);
    assert_eq!(step(&run, "said").output, json!({ "all": { "name": "", "vars": {} }, "api": "[]" }));
}
