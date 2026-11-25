//! An environment's life once Run has begun loading it: ready, with where it is reached and the cards it holds,
//! or failed, with why; taken down once it was ready; each said once as an
//! event any blueprint can hear and measured by project, environment and
//! branch; and a load whose end a stop left unsaid said by the daemon that
//! comes back.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::lanes::git;

/// The project's own deploy: it waits to be told to go where it is handed
/// `wait`, fails for the branch called `broken`, and stays, listening,
/// once it has gone well, as something served does.
const LIVE: &str = "\
blueprint: live
params:
  branch: { type: string }
steps:
  held:
    description: Waits to be told to go, where the branch says so.
    when: ${ params.branch == \"wait\" }
    do: wait
    with: { event: shop.go }
  broke:
    description: The build that does not build.
    needs: [held]
    when: ${ params.branch == \"broken\" }
    do: fail
    with: { message: the branch broken does not build }
on:
  poked:
    event: shop.poked
    do: noop
outputs:
  address: http://127.0.0.1:9/live
  cards: [shop-1, shop-2]
";

/// A project that is a repository, with the branches `main`, `broken` and
/// `wait`, and an environment `dev` loaded by `live`.
async fn shop(rig: &Rig) {
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = std::path::PathBuf::from(project["path"].as_str().unwrap());
    git(&dir, &["init", "--quiet", "-b", "main", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    git(&dir, &["commit", "--quiet", "--allow-empty", "-m", "first"]);
    git(&dir, &["branch", "broken"]);
    git(&dir, &["branch", "wait"]);
    let body = json!({ "path": format!("@blueprints/{}", brand::blueprint_file("live")), "content": LIVE });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let base = "/api/projects/shop/environments";
    rig.send(base, json!({ "name": "dev" })).await;
    let said = json!({ "blueprint": "live" });
    let (status, set) =
        rig.call(Method::PUT, &format!("{base}/dev/deploy"), Some(&rig.token), Some(said)).await;
    assert_eq!(status, StatusCode::OK, "{set}");
}

/// Runs `dev` from `branch`, and gives the run it began.
async fn run(rig: &Rig, branch: &str) -> String {
    let ran = rig.send("/api/projects/shop/environments/dev/run", json!({ "branch": branch })).await;
    ran["applied"]["run"].as_str().unwrap().to_string()
}

/// What was measured of `name` for `shop`'s `dev`, by branch.
async fn measured(rig: &Rig, name: &str) -> Value {
    rig.read(&format!("/api/metrics?name={name}&by=branch&labels=project:shop,environment:dev")).await
}

#[tokio::test]
async fn an_environment_says_once_it_is_ready_failed_or_taken_down_and_each_load_is_measured() {
    let rig = Rig::new().await;
    shop(&rig).await;

    let first = run(&rig, "main").await;
    let ready = rig.event("project.environment.ready", |event| event.data["run"] == first.as_str()).await;
    assert_eq!(
        (&ready.data["environment"], &ready.data["branch"], &ready.data["address"], &ready.data["cards"]),
        (&json!("dev"), &json!("main"), &json!("http://127.0.0.1:9/live"), &json!(["shop-1", "shop-2"])),
        "{:?}",
        ready.data
    );
    assert!(ready.data["took_ms"].as_i64().is_some_and(|took| took >= 0), "{:?}", ready.data);
    let kept = rig.read("/api/projects/shop/environments/dev").await;
    assert_eq!(kept["ran"]["ended"]["ready"], true, "{kept}");

    // Stopped, what it was loaded as was taken down after it was ready.
    let instance = ready.data["instance"].as_str().unwrap().to_string();
    rig.send(&format!("/api/instances/{instance}/stop"), json!({})).await;
    let down =
        rig.event("project.environment.down", |event| event.data["instance"] == instance.as_str()).await;
    assert!(down.data["up_ms"].as_i64().is_some_and(|up| up >= 0), "{:?}", down.data);

    // A branch that does not build: failed, with why, and nothing more said of it.
    let second = run(&rig, "broken").await;
    let failed = rig.event("project.environment.failed", |event| event.data["run"] == second.as_str()).await;
    assert!(failed.data["reason"].as_str().unwrap().contains("does not build"), "{:?}", failed.data);
    let said = |topic: &str| journal::EventQuery { topics: topic.into(), limit: 100, ..Default::default() };
    assert_eq!(rig.said(said("project.environment.ready")).await.len(), 1);
    assert_eq!(rig.said(said("project.environment.down")).await.len(), 1);

    // Each load measured by its branch, counted: one went well and one did not.
    let loaded = measured(&rig, "project.environment.loaded").await;
    let series = loaded["series"].as_array().unwrap();
    let of = |branch: &str| series.iter().find(|one| one["labels"]["branch"] == branch).cloned();
    assert_eq!(of("main").unwrap()["total"]["count"], 1, "{loaded}");
    assert_eq!(of("broken").unwrap()["total"]["errors"], 1, "{loaded}");
    let up = measured(&rig, "project.environment.up").await;
    assert_eq!(up["series"][0]["total"]["count"], 1, "{up}");
}

#[tokio::test]
async fn a_load_whose_end_a_stop_left_unsaid_is_said_by_the_daemon_that_comes_back() {
    let mut rig = Rig::new().await;
    shop(&rig).await;
    let waiting = run(&rig, "wait").await;
    rig.event("step.waiting", |event| event.source.run.as_deref() == Some(waiting.as_str())).await;
    rig.restart().await;
    rig.send("/api/emit/shop.go", json!({})).await;
    let ready = rig.event("project.environment.ready", |event| event.data["run"] == waiting.as_str()).await;
    assert_eq!(ready.data["branch"], "wait", "{:?}", ready.data);
    let said =
        journal::EventQuery { topics: "project.environment.ready".into(), limit: 100, ..Default::default() };
    assert_eq!(rig.said(said).await.len(), 1);
}
