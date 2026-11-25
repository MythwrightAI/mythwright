//! What an environment holds to (`environments::rules`), through the routes: the branches it lets run and who approves
//! what goes under it, said by whoever may change its secrets and held
//! where something comes to run under it, from outside or by a step of a
//! run under none that says `environment:`; a step under one environment
//! starting nothing under another; and the runs that acted under it,
//! listed with it, its last run the first of them.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::said;
use crate::kept::person;

/// What an environment is loaded by, handed a branch.
const DEPLOY: &str = "\
blueprint: deploy
params:
  branch: { type: string, default: \"\" }
steps:
  loaded: { do: emit, with: { topic: shop.loaded, branch: \"${params.branch}\" } }
";

/// A pipeline under none that waits for a go-ahead under prod, and then
/// loads prod from main.
const RELEASE: &str = "\
blueprint: release
steps:
  approve:
    do: approval.ask
    with: { says: Load prod?, environment: prod }
  deploy:
    needs: [approve]
    call: deploy
    as: deploy-prod
    environment: prod
    with: { branch: main }
";

/// The same, with no go-ahead asked.
const HASTY: &str = "\
blueprint: hasty
steps:
  deploy: { call: deploy, as: deploy-prod, environment: prod, with: { branch: main } }
";

async fn shop(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    rig.send("/api/projects/shop/environments", json!({ "name": "prod" })).await;
    let saved = json!({ "path": "@blueprints/deploy.bp", "content": DEPLOY });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

async fn rules(rig: &Rig, token: &str, body: Value) -> (StatusCode, Value) {
    rig.call(Method::PUT, "/api/projects/shop/environments/prod/rules", Some(token), Some(body)).await
}

/// `deploy` applied under prod as `token`, handed `branch` where said.
async fn deployed(rig: &Rig, token: &str, branch: Option<&str>) -> (StatusCode, Value) {
    let mut body = json!({ "source": DEPLOY, "project": "shop", "environment": "prod", "wait": true });
    if let Some(branch) = branch {
        body["params"] = json!({ "branch": branch });
    }
    rig.call(Method::POST, "/api/apply", Some(token), Some(body)).await
}

/// `source` applied for Shop under none of its environments, as the
/// owner, by the runner itself: as what the daemon applies for a project
/// (a pipeline's trigger) is, since a person who applies in a project with
/// environments names one. The run of its steps.
async fn under_none(rig: &Rig, source: &str, key: &str) -> String {
    let who = rig.node().app.studio.identity.authenticate(&rig.token).await.unwrap();
    let request = conductor::Apply {
        source: Some(source.to_string()),
        key: Some(key.to_string()),
        project: Some("shop".into()),
        environment: Some(String::new()),
        ..conductor::Apply::new(key, who)
    };
    rig.node().app.engine.apply(request).await.unwrap().run.unwrap()
}

async fn ended(rig: &Rig, run: &str) -> Value {
    rig.event("run.finished", |event| event.source.run.as_deref() == Some(run)).await;
    rig.read(&format!("/api/runs/{run}")).await
}

fn step<'a>(run: &'a Value, name: &str) -> &'a Value {
    run["steps"].as_array().unwrap().iter().find(|step| step["step"] == name).unwrap()
}

#[tokio::test]
async fn the_branches_an_environment_lets_run_are_said_by_whoever_may_change_its_secrets_and_held_where_it_is_applied_under()
 {
    let rig = Rig::new().await;
    shop(&rig).await;
    let grace = person(&rig, "Grace").await;
    let (status, _) = rules(&rig, &grace.token, json!({ "branches": ["main"] })).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, refused) = rules(&rig, &rig.token, json!({ "branches": ["a b"] })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("cannot be a branch an environment lets run"), "{refused}");
    let (status, refused) = rules(&rig, &rig.token, json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let (status, set) = rules(&rig, &rig.token, json!({ "branches": ["main", "release/*", "main"] })).await;
    assert_eq!(status, StatusCode::OK, "{set}");
    assert_eq!(set["environment"]["rules"]["branches"], json!(["main", "release/*"]));
    assert_eq!(set["changed"]["rules"], json!(true));
    // Who approves is left as it was: nobody.
    assert_eq!(set["environment"]["rules"].get("approve"), None, "{set}");

    let (status, refused) = deployed(&rig, &rig.token, Some("feature/x")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("`feature/x` may not be run under `prod` of `shop`"), "{refused}");
    assert_eq!(rig.count_of("shop.loaded").await, 0);
    for branch in ["release/2", "main"] {
        let (status, ran) = deployed(&rig, &rig.token, Some(branch)).await;
        assert_eq!((status, &ran["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{ran}");
    }
    // Handed none, it is held to who approves alone.
    let (status, ran) = deployed(&rig, &rig.token, None).await;
    assert_eq!(status, StatusCode::OK, "{ran}");

    // Said as any, every branch runs.
    let (_, set) = rules(&rig, &rig.token, json!({ "branches": [] })).await;
    assert_eq!(set["environment"]["rules"], json!({}), "{set}");
    let (status, ran) = deployed(&rig, &rig.token, Some("feature/x")).await;
    assert_eq!(status, StatusCode::OK, "{ran}");
}

#[tokio::test]
async fn what_is_started_under_an_environment_that_says_who_approves_is_started_by_one_of_them_or_by_a_run_they_let_go_on()
 {
    let rig = Rig::new().await;
    shop(&rig).await;
    let grace = person(&rig, "Grace").await;
    let (status, set) = rules(&rig, &rig.token, json!({ "approve": ["Grace"], "branches": ["main"] })).await;
    assert_eq!(status, StatusCode::OK, "{set}");

    // Ada is not one of them, and starts nothing under it herself.
    let (status, refused) = deployed(&rig, &rig.token, Some("main")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused).contains("approved by those its rule names, and you are not one of them"),
        "{refused}"
    );
    // Grace is, and does.
    let (status, ran) = deployed(&rig, &grace.token, Some("main")).await;
    assert_eq!((status, &ran["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{ran}");

    // A run under none that starts its child under prod with no go-ahead
    // fails that step, saying what would let it.
    let hasty = under_none(&rig, HASTY, "hasty").await;
    let finished = ended(&rig, &hasty).await;
    assert_eq!(finished["status"], "failed", "{finished}");
    let why = step(&finished, "deploy")["error"].as_str().unwrap_or_default().to_string();
    assert!(why.contains("this run holds no go-ahead for it"), "{why}");

    // One that waits for Grace's go-ahead under prod starts it once she gives it.
    let release = under_none(&rig, RELEASE, "release").await;
    let told = rig.event("project.approval.asked", |event| event.data["run"] == json!(release)).await;
    assert_eq!(told.data["to"], json!([grace.actor]), "{told:?}");
    let ask = told.data["ask"].as_str().unwrap();
    let (status, approved) = rig
        .call(Method::POST, &format!("/api/asked/{ask}/approve"), Some(&grace.token), Some(json!({})))
        .await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    let finished = ended(&rig, &release).await;
    assert_eq!(finished["status"], "ok", "{finished}");
    let child = step(&finished, "deploy")["output"]["instance"].as_str().unwrap().to_string();
    let child = rig.read(&format!("/api/instances/{child}")).await;
    let child = &child["instance"];
    assert_eq!((&child["environment"], &child["key"]), (&json!("prod"), &json!("deploy-prod")), "{child}");

    // Its page lists the runs that acted under it, the newest first: the
    // child's, said as what was applied under it, and not the pipeline's,
    // which ran under none; its last run is the first of them.
    let shown = rig.read("/api/projects/shop/environments/prod").await;
    let runs = shown["runs"].as_array().unwrap();
    assert_eq!(runs[0]["what"], json!("deploy as deploy-prod"), "{shown}");
    assert_eq!(runs[0]["status"], json!("ok"), "{shown}");
    assert!(runs.iter().all(|run| run["run"] != json!(release) && run["run"] != json!(hasty)), "{shown}");
    assert_eq!(runs.len(), 2, "Grace's deploy and the release's: {shown}");
    assert_eq!(shown["last"]["run"], runs[0]["run"], "{shown}");
    // Somebody who may not be told what runs is handed none of it.
    let narrow = crate::kept::key(&rig, &grace.token, &["projects:read"]).await;
    let (_, shown) = rig.call(Method::GET, "/api/projects/shop/environments/prod", Some(&narrow), None).await;
    assert_eq!(shown.get("runs"), None, "{shown}");
}

#[tokio::test]
async fn a_step_under_one_environment_starts_nothing_under_another() {
    let rig = Rig::new().await;
    shop(&rig).await;
    rig.send("/api/projects/shop/environments", json!({ "name": "dev" })).await;
    let body = json!({ "source": HASTY, "project": "shop", "environment": "dev", "wait": true });
    let (status, ran) = rig.call(Method::POST, "/api/apply", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{ran}");
    assert_eq!(ran["finished"]["status"], "failed", "{ran}");
    let why = ran["finished"]["error"].as_str().unwrap_or_default();
    assert!(why.contains("runs under dev, and this asks for prod"), "{ran}");
    assert_eq!(rig.count_of("shop.loaded").await, 0);
}
