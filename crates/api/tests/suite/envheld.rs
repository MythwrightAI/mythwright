//! A run held by an environment's rule, stopped anywhere in its being let
//! through and started again (`environments::rules`).
//!
//! Shop's `prod` lets `main` alone run under it, and Grace alone approves
//! what is started under it. A pipeline applied for Shop under none of its
//! environments, as what the daemon applies for a project is, waits for a
//! go-ahead under prod and then starts its `deploy` under prod by a step
//! (`environment: prod`), handed `main`. Grace lets it go on. The start is
//! held in the turn that holds prod: the run's go-ahead is read and not
//! spent, so a step taken up after a stop finds it again. Wherever the
//! daemon goes, prod is loaded once from main, by one instance under it,
//! and the pipeline ends well.

use kernel::Event;
use reqwest::Method;
use serde_json::{Value, json};

use crate::found::Grain;
use crate::kept::person;
use crate::observed::node_with_stand_ins;
use crate::simulation::{Came, Place, Play, Scenario, simulate};

const DEPLOY: &str = "blueprint: deploy
params:
  branch: { type: string, default: \"\" }
steps:
  loaded: { do: emit, with: { topic: shop.loaded, branch: \"${params.branch}\" } }
";

const RELEASE: &str = "blueprint: release
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

/// The pipeline applied, and Grace letting it go on.
#[derive(Clone)]
struct Held;

impl Scenario for Held {
    const ACTS: usize = 2;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-envheld").await;
        rig.send("/api/projects", json!({ "name": "Shop" })).await;
        rig.send("/api/tier", json!({ "tier": "premium" })).await;
        let grace = person(&rig, "Grace").await;
        rig.send("/api/projects/shop/environments", json!({ "name": "prod" })).await;
        let rules = json!({ "approve": ["Grace"], "branches": ["main"] });
        let (status, set) = rig
            .call(Method::PUT, "/api/projects/shop/environments/prod/rules", Some(&rig.token), Some(rules))
            .await;
        assert_eq!(status, reqwest::StatusCode::OK, "{set}");
        let saved = json!({ "path": "@blueprints/deploy.bp", "content": DEPLOY });
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("grace", grace.token);
        play
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        match nth {
            // What the daemon applies for a project is applied under none,
            // which no request says, so it is applied by the runner itself
            // here; applied again under its key, it is the instance there is.
            0 => {
                let rig = &play.rig;
                let who = rig.node().app.studio.identity.authenticate(&rig.token).await.unwrap();
                let request = conductor::Apply {
                    source: Some(RELEASE.to_string()),
                    key: Some("release".into()),
                    project: Some("shop".into()),
                    environment: Some(String::new()),
                    ..conductor::Apply::new("release", who)
                };
                rig.node().app.engine.apply(request).await.unwrap();
            }
            // Found by the line she was told by, which is there however it
            // was decided: done again once decided, it is as it was.
            _ => {
                let grace = play.note("grace").to_string();
                let (_, inbox) = play.rig.call(Method::GET, "/api/inbox?all=true", Some(&grace), None).await;
                let line = inbox["lines"].as_array().into_iter().flatten().find_map(|line| {
                    line["why"].as_str().and_then(|why| why.strip_prefix("asked ")).map(str::to_string)
                });
                let ask = line.unwrap_or_else(|| panic!("Grace was told of no run that waits: {inbox}"));
                let path = format!("/api/asked/{ask}/approve");
                let (status, came) = play.rig.call(Method::POST, &path, Some(&grace), Some(json!({}))).await;
                assert_eq!(status, reqwest::StatusCode::OK, "{came}");
            }
        }
    }

    fn heard(&self, nth: usize, _: &Play, event: &Event) -> bool {
        match nth {
            0 => event.topic == "inbox.told",
            _ => event.topic == "shop.loaded",
        }
    }

    async fn came_to(&self, play: &Play) -> Came {
        let rig = &play.rig;
        let loaded: Vec<Value> = rig
            .said(journal::EventQuery { topics: "shop.loaded".into(), limit: 100, ..Default::default() })
            .await
            .into_iter()
            .map(|event| event.data["branch"].clone())
            .collect();
        let instances = rig.read("/api/instances?project=shop").await;
        let mut ran: Vec<Value> = instances["instances"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|one| json!([one["blueprint"], one["key"], one["environment"], one["status"]]))
            .collect();
        ran.sort_by_key(Value::to_string);
        let asked = rig.read("/api/asked").await;
        let asks: Vec<Value> = asked["asked"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|ask| json!([ask["kind"], ask["status"]]))
            .collect();
        let shown = rig.read("/api/projects/shop/environments/prod").await;
        let under: Vec<Value> = shown["runs"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|run| json!([run["what"], run["status"]]))
            .collect();
        Came::default()
            .with("loaded", json!(loaded))
            .with("ran", json!(ran))
            .with("asks", json!(asks))
            .with("under prod", json!(under))
    }
}

/// Where the go-ahead is decided and heard, the child is started under
/// prod, and it and the pipeline end.
fn in_the_holding(place: &Place) -> bool {
    place.topic.starts_with("project.approval")
        || place.topic.starts_with("instance.")
        || ["shop.loaded", "run.finished"].contains(&place.topic.as_str())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_run_its_environments_rule_lets_through_starts_under_it_once_wherever_the_daemon_is_stopped() {
    let grain = Grain::asked_or(Grain::Where(in_the_holding));
    simulate("a run held by a rule", Held, grain).await.holds();
}
