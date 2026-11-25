//! What runs under none of its project's environments starts its children
//! under the one its steps name (`environment:`), stopped anywhere and
//! started again.

use serde_json::json;

use crate::anywhere::{Does, Scenario, anywhere, then};

const TARGETED: &str = r#"blueprint: targeted
description: Applied under none of its project's environments, with what it calls and what it starts each under one.
steps:
  read: { do: test.echo, with: { name: ${ environment.name } } }
  called: { call: beneath, as: called, environment: prod }
  kept: { spawn: beneath, as: kept, environment: prod }
  same: { call: beneath, as: same }
outputs:
  called: ${ steps.called.out.name }
  same: ${ steps.same.out.name }
"#;

#[tokio::test]
async fn what_runs_under_none_starts_its_children_under_the_environment_its_steps_name_wherever_the_conductor_is_stopped()
 {
    let mut targeted =
        Scenario::of("targeted", TARGETED).with(&[crate::under::BENEATH]).judged(crate::judged::rested);
    targeted.acts = vec![then(Does::ApplyBeside("shop", "prod", TARGETED, json!({})))];
    let found = anywhere(targeted).await;
    let read = &found.usual["runs"]["targeted[targeted]"]["main"][0];
    assert_eq!(read["outputs"], json!({ "called": "prod", "same": "" }), "{}", found.usual);
    assert_eq!(read["steps"]["read"]["gave"], json!({ "name": "" }));
    let gave = &found.usual["runs"]["targeted[targeted] > beneath[kept]"]["main"][0]["steps"]["read"]["gave"];
    assert_eq!(gave, &json!({ "name": "prod" }), "{}", found.usual);
    assert_eq!(found.usual["instances"]["targeted[targeted]"]["status"], json!("done"), "{}", found.usual);
    found.held();
}
