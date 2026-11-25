//! What runs under one of its project's environments, stopped anywhere
//! and started again: it and what it starts stay under it, and read it.

use serde_json::json;

use crate::anywhere::{Does, Scenario, anywhere, then};

const UNDER: &str = r#"blueprint: under
description: Applied under an environment of its project's, with what it calls and what it starts under it too.
steps:
  read: { do: test.echo, with: { name: ${ environment.name }, api: ${ environment.vars.api } } }
  called: { call: beneath, as: called }
  kept: { spawn: beneath, as: kept }
outputs:
  name: ${ environment.name }
  called: ${ steps.called.out.name }
"#;

pub(crate) const BENEATH: &str = r#"blueprint: beneath
steps:
  read: { do: test.echo, with: { name: ${ environment.name } } }
outputs:
  name: ${ steps.read.out.name }
"#;

#[tokio::test]
async fn what_runs_under_an_environment_and_what_it_starts_stay_under_it_wherever_the_conductor_is_stopped() {
    let mut under = Scenario::of("under", UNDER).with(&[BENEATH]).judged(crate::judged::rested);
    under.acts = vec![then(Does::ApplyUnder("shop", "prod", UNDER, json!({})))];
    let found = anywhere(under).await;
    let read = &found.usual["runs"]["under[under]"]["main"][0];
    assert_eq!(read["outputs"], json!({ "name": "prod", "called": "prod" }), "{}", found.usual);
    assert_eq!(read["steps"]["read"]["gave"], json!({ "name": "prod", "api": "https://prod.example" }));
    for child in ["under[under] > beneath[called]", "under[under] > beneath[kept]"] {
        let gave = &found.usual["runs"][child]["main"][0]["steps"]["read"]["gave"];
        assert_eq!(gave, &json!({ "name": "prod" }), "{child}: {}", found.usual);
    }
    assert_eq!(found.usual["instances"]["under[under]"]["status"], json!("done"), "{}", found.usual);
    found.held();
}

impl crate::Rig {
    /// Makes each environment a play applies something under, with one
    /// variable, `api`, before the play begins.
    pub async fn makes_environments(&self, acts: &[crate::anywhere::Act]) {
        for act in acts {
            if let Does::ApplyUnder(project, environment, ..) | Does::ApplyBeside(project, environment, ..) =
                &act.does
            {
                if self.engine.environment_of(project, environment).await.unwrap().is_some() {
                    continue;
                }
                let source = kernel::Source::actor(crate::user().actor());
                self.engine
                    .make_environment(project, environment, &crate::user(), source.clone())
                    .await
                    .unwrap();
                let api =
                    kernel::fields(serde_json::json!({ "api": format!("https://{environment}.example") }));
                let set = |held: &mut conductor::environs::Environment| held.set(&api, None);
                self.engine
                    .change_environment(project, environment, set, &crate::user(), source)
                    .await
                    .unwrap();
            }
        }
    }
}
