//! An environment loaded from a branch: the branches of a project that is
//! a repository, fetched fresh from a remote that is another repository in
//! the rig's folder, the blueprint the environment says it is loaded by,
//! and Run, which applies that blueprint under the environment handed the
//! branch; what it last ran said with it and how that went read from the
//! run; and who may do each.

use std::path::Path;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::{key, said};
use crate::lanes::git;

/// The project's own deploy: it takes the branch and the commit, says it
/// built the one it was handed, and fails for the branch called `broken`.
const DEPLOY: &str = "\
blueprint: deploy
params:
  branch: { type: string }
  commit: { type: string }
steps:
  built:
    description: The branch built.
    do: emit
    with: { topic: shop.built, branch: \"${params.branch}\", commit: \"${params.commit}\" }
  served:
    description: The build served.
    needs: [built]
    when: ${ params.branch != \"broken\" }
    do: emit
    with: { topic: shop.served, branch: \"${params.branch}\" }
  broke:
    description: The build that does not build.
    needs: [built]
    when: ${ params.branch == \"broken\" }
    do: fail
    with: { message: the branch broken does not build }
outputs:
  address: http://127.0.0.1:9/preview
";

/// One that takes nothing, which Run cannot hand a branch.
const PLAIN: &str = "\
blueprint: plain
steps:
  done: { do: emit, with: { topic: shop.plain } }
";

async fn write(rig: &Rig, name: &str, content: &str) {
    let body = json!({ "path": format!("@blueprints/{}", brand::blueprint_file(name)), "content": content });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

/// Waits for `run` to end, and gives how, as the run says.
async fn ended(rig: &Rig, run: &str) -> Value {
    rig.event("run.finished", |event| event.source.run.as_deref() == Some(run)).await;
    rig.read(&format!("/api/runs/{run}")).await
}

fn commit_of(dir: &Path, name: &str) -> String {
    git(dir, &["rev-parse", name])
}

#[tokio::test]
async fn an_environment_is_loaded_from_a_branch_by_the_blueprint_it_says_and_says_what_it_last_ran() {
    let rig = Rig::new().await;
    // A folder somebody said, which is no repository until it is made one.
    let dir = rig.dir.join("shop");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("README.md"), "# Shop\n").unwrap();
    rig.send("/api/projects", json!({ "name": "Shop", "path": dir })).await;
    let base = "/api/projects/shop/environments";
    rig.send(base, json!({ "name": "dev" })).await;

    // A project that is no repository has no branch, and says so; nothing runs there.
    let none = rig.read("/api/projects/shop/branches").await;
    assert_eq!((&none["repository"], &none["branches"]), (&json!(false), &json!([])), "{none}");
    assert!(none["said"].as_str().unwrap().contains("is no repository"), "{none}");

    // The blueprint it is loaded by is one there is for the project: a name
    // misspelt is said now, and one that cannot be a name refused.
    let deploy = format!("{base}/dev/deploy");
    let (status, refused) =
        rig.call(Method::PUT, &deploy, Some(&rig.token), Some(json!({ "blueprint": "deploy" }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert!(said(&refused).contains("there is no blueprint `deploy` for `shop`"), "{refused}");
    let (status, refused) =
        rig.call(Method::PUT, &deploy, Some(&rig.token), Some(json!({ "blueprint": "../up" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    write(&rig, "deploy", DEPLOY).await;
    write(&rig, "plain", PLAIN).await;
    let (status, set) =
        rig.call(Method::PUT, &deploy, Some(&rig.token), Some(json!({ "blueprint": "deploy" }))).await;
    assert_eq!(status, StatusCode::OK, "{set}");
    assert_eq!((&set["environment"]["deploy"], &set["changed"]["deploy"]), (&json!("deploy"), &json!(true)));
    let changed = rig.event("project.environment.changed", |event| event.data["deploy"] == true).await;
    assert_eq!(changed.data["environment"], "dev");

    // Run with no repository says so.
    let run = format!("{base}/dev/run");
    let (status, refused) = rig.post(&run, json!({ "branch": "main" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("is no repository"), "{refused}");

    // Made one: its own branches, and a remote whose branches are come by
    // only when somebody asks for them.
    git(&dir, &["init", "--quiet", "-b", "main", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "first"]);
    git(&dir, &["branch", "broken"]);
    let remote = rig.dir.join("remote.git");
    git(&rig.dir, &["init", "--quiet", "--bare", remote.to_str().unwrap()]);
    git(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
    let other = rig.dir.join("other");
    git(&rig.dir, &["clone", "--quiet", remote.to_str().unwrap(), other.to_str().unwrap()]);
    git(&other, &["config", "user.name", "Grace"]);
    git(&other, &["config", "user.email", "grace@example.com"]);
    git(&other, &["checkout", "--quiet", "-b", "dev"]);
    git(&other, &["commit", "--quiet", "--allow-empty", "-m", "dev, as somebody else made it"]);
    git(&other, &["push", "--quiet", "origin", "dev"]);

    let listed = rig.read("/api/projects/shop/branches").await;
    let names = |listed: &Value| -> Vec<String> {
        listed["branches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|one| one["name"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(listed["repository"], true);
    assert_eq!(names(&listed), ["broken", "main"], "nothing of the remote's until it is asked");
    assert_eq!(listed["remotes"], json!(["origin"]));
    let (status, fetched) = rig.post("/api/projects/shop/branches/fetch", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{fetched}");
    assert_eq!(fetched["fetched"]["new"], json!(["origin/dev"]), "{fetched}");
    assert_eq!(fetched["said"], "Fetched. new: `origin/dev`.");
    assert_eq!(names(&fetched), ["broken", "main", "origin/dev"]);
    let theirs = &fetched["branches"][2];
    assert_eq!(
        (&theirs["remote"], &theirs["subject"]),
        (&json!("origin"), &json!("dev, as somebody else made it"))
    );

    // A branch the project has not got is said, with how to have it.
    let (status, refused) = rig.post(&run, json!({ "branch": "origin/nowhere" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert!(said(&refused).contains("`--fetch` asks"), "{refused}");

    // Run: the blueprint applied for the project under the environment,
    // handed the branch and its commit, its steps begun.
    let (status, ran) = rig.post(&run, json!({ "branch": "origin/dev" })).await;
    assert_eq!(status, StatusCode::OK, "{ran}");
    let run_id = ran["applied"]["run"].as_str().unwrap().to_string();
    let commit = commit_of(&dir, "origin/dev");
    let last = &ran["environment"]["ran"];
    assert_eq!(
        (&last["branch"], &last["commit"], &last["blueprint"]),
        (&json!("origin/dev"), &json!(commit), &json!("deploy"))
    );
    assert_eq!(last["run"], json!(run_id));
    let told = rig.event("project.environment.ran", |_| true).await;
    assert_eq!((&told.data["branch"], &told.data["run"]), (&json!("origin/dev"), &json!(run_id)));
    let built = rig.event("shop.built", |_| true).await;
    assert_eq!((&built.data["branch"], &built.data["commit"]), (&json!("origin/dev"), &json!(commit)));
    let finished = ended(&rig, &run_id).await;
    assert_eq!(finished["status"], "ok", "{finished}");
    assert_eq!(finished["outputs"]["address"], "http://127.0.0.1:9/preview");
    let instance =
        rig.read(&format!("/api/instances/{}", ran["applied"]["instance"].as_str().unwrap())).await;
    let instance = if instance["instance"].is_object() { &instance["instance"] } else { &instance };
    assert_eq!(
        (&instance["key"], &instance["environment"]),
        (&json!("deploy-dev"), &json!("dev")),
        "{instance}"
    );

    // What it last ran is said with the environment, with how that went, as the run says.
    let dev = rig.read(&format!("{base}/dev")).await;
    assert_eq!((&dev["ran"]["status"], &dev["ran"]["by_name"]), (&json!("ok"), &json!("Ada")), "{dev}");
    assert_eq!((&dev["last"]["kind"], &dev["last"]["run"]), (&json!("loaded"), &json!(run_id)), "{dev}");
    assert!(dev["last"]["said"].as_str().unwrap().starts_with("origin/dev at "), "{dev}");
    let all = rig.read(base).await;
    assert_eq!(all["environments"][0]["ran"]["status"], "ok", "{all}");

    // A deploy that ran with nothing kept of it (a daemon stopped between the
    // apply and the keeping, played by applying it by hand under its key):
    // its run is newer than what the environment keeps, and is what is shown.
    let main = commit_of(&dir, "main");
    let by_hand = json!({
        "blueprint": "deploy", "project": "shop", "environment": "dev", "key": "deploy-dev", "force": true,
        "params": { "branch": "main", "commit": main },
    });
    let applied = rig.send("/api/apply", by_hand).await;
    let newer = applied["run"].as_str().unwrap().to_string();
    ended(&rig, &newer).await;
    let dev = rig.read(&format!("{base}/dev")).await;
    let shown = &dev["ran"];
    assert_eq!(
        (&shown["run"], &shown["branch"], &shown["commit"]),
        (&json!(newer), &json!("main"), &json!(main)),
        "{dev}"
    );
    assert_eq!(shown["status"], "ok", "{dev}");
    let all = rig.read(base).await;
    assert_eq!(all["environments"][0]["ran"]["run"], json!(newer), "{all}");

    // Run again, another branch: under the same key, a deploy that is over
    // being one done anew, and a step that fails said by the run.
    let (status, again) = rig.post(&run, json!({ "branch": "broken" })).await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again["applied"]["key"], "deploy-dev", "{again}");
    let failed = ended(&rig, again["applied"]["run"].as_str().unwrap()).await;
    assert_eq!(failed["status"], "failed", "{failed}");
    let broke = failed["steps"].as_array().unwrap().iter().find(|step| step["step"] == "broke").unwrap();
    assert!(broke["error"].as_str().unwrap().contains("the branch broken does not build"), "{broke}");
    let dev = rig.read(&format!("{base}/dev")).await;
    assert_eq!((&dev["ran"]["branch"], &dev["ran"]["status"]), (&json!("broken"), &json!("failed")), "{dev}");

    // A blueprint that does not take a branch is not handed one.
    rig.call(Method::PUT, &deploy, Some(&rig.token), Some(json!({ "blueprint": "plain" }))).await;
    let (status, refused) = rig.post(&run, json!({ "branch": "main" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("`plain` does not declare a param `branch`"), "{refused}");
    // And one that says none has nothing to run.
    let (_, unset) =
        rig.call(Method::PUT, &deploy, Some(&rig.token), Some(json!({ "blueprint": null }))).await;
    assert_eq!(unset["environment"]["deploy"], Value::Null, "{unset}");
    let (status, refused) = rig.post(&run, json!({ "branch": "main" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("says no blueprint it is loaded by"), "{refused}");
}

#[tokio::test]
async fn running_an_environment_takes_what_applying_takes_and_fetching_takes_writing_the_project_and_shell() {
    let rig = Rig::new().await;
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = std::path::PathBuf::from(project["path"].as_str().unwrap());
    git(&dir, &["init", "--quiet", "-b", "main", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    git(&dir, &["commit", "--quiet", "--allow-empty", "-m", "first"]);
    let remote = rig.dir.join("remote.git");
    git(&rig.dir, &["init", "--quiet", "--bare", remote.to_str().unwrap()]);
    git(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
    let base = "/api/projects/shop/environments";
    rig.send(base, json!({ "name": "dev" })).await;
    write(&rig, "deploy", DEPLOY).await;
    rig.call(
        Method::PUT,
        &format!("{base}/dev/deploy"),
        Some(&rig.token),
        Some(json!({ "blueprint": "deploy" })),
    )
    .await;

    // Reading the branches takes reading the project, and nothing else does.
    let reader = key(&rig, &["projects:read"]).await;
    let (status, listed) = rig.call(Method::GET, "/api/projects/shop/branches", Some(&reader), None).await;
    assert_eq!((status, listed["branches"][0]["name"].as_str()), (StatusCode::OK, Some("main")), "{listed}");
    let nobody = key(&rig, &["channels:read"]).await;
    let (status, _) = rig.call(Method::GET, "/api/projects/shop/branches", Some(&nobody), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Run takes what applying takes, refused before anything is read or run.
    let (status, refused) = rig
        .call(Method::POST, &format!("{base}/dev/run"), Some(&reader), Some(json!({ "branch": "main" })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("blueprints:run"), "{refused}");
    assert!(rig.read(&format!("{base}/dev")).await["ran"].is_null(), "nothing was run");

    // Saying how it is loaded takes what changing an environment takes.
    let writer = key(&rig, &["projects"]).await;
    let (status, refused) = rig
        .call(Method::PUT, &format!("{base}/dev/deploy"), Some(&writer), Some(json!({ "blueprint": null })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // Fetching reaches the network through what the repository says, so it
    // takes `shell` beside writing the project; a remote that has nothing
    // brings nothing.
    let (status, refused) =
        rig.call(Method::POST, "/api/projects/shop/branches/fetch", Some(&writer), Some(json!({}))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("`shell`"), "{refused}");
    let (status, fetched) = rig.post("/api/projects/shop/branches/fetch", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{fetched}");
    assert_eq!(fetched["said"], "Fetched: every remote's branches are as they were.");

    // A remote that cannot be reached is said in git's words, and nothing is changed.
    git(&dir, &["remote", "set-url", "origin", rig.dir.join("gone.git").to_str().unwrap()]);
    let (status, refused) = rig.post("/api/projects/shop/branches/fetch", json!({})).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{refused}");
    assert!(said(&refused).contains("could not be asked what they have"), "{refused}");
}

#[tokio::test]
async fn an_environments_last_run_says_what_ran_under_it_however_it_was_started_and_never_that_nothing_did() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let base = "/api/projects/shop/environments";
    rig.send(base, json!({ "name": "dev" })).await;
    let last = |shown: &Value| shown["last"].clone();

    // Nothing has run, and it names no blueprint it is loaded by: said, with the command that names one.
    let dev = rig.read(&format!("{base}/dev")).await;
    assert_eq!(last(&dev)["kind"], "unsaid", "{dev}");
    let command = format!("`{} project environment shop dev deploy <blueprint>`", brand::CLI);
    assert_eq!(
        last(&dev)["said"],
        format!("nothing has run under it, and no blueprint loads it from a branch: {command} names one")
    );

    // Something applied under it by its own key, and not through Run: its run is what it last ran.
    write(&rig, "plain", PLAIN).await;
    let preview =
        json!({ "blueprint": "plain", "project": "shop", "environment": "dev", "key": "dev-preview" });
    let applied = rig.send("/api/apply", preview).await;
    let run = applied["run"].as_str().unwrap().to_string();
    ended(&rig, &run).await;
    let all = rig.read(base).await;
    let said = last(&all["environments"][0]);
    assert_eq!(
        (&said["kind"], &said["run"], &said["instance"]),
        (&json!("applied"), &json!(run), &applied["instance"])
    );
    assert!(said["said"].as_str().unwrap().starts_with("plain as dev-preview, went well, by "), "{said}");
    assert!(said["at"].is_number(), "{said}");
    assert!(all["environments"][0]["ran"].is_null(), "nothing was loaded through Run: {all}");

    // Whoever may not be told what runs is told what the environment says of itself, and nothing of what ran.
    let reader = key(&rig, &["projects:read"]).await;
    let (status, read) = rig.call(Method::GET, &format!("{base}/dev"), Some(&reader), None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    assert_eq!(
        last(&read)["said"],
        format!("No blueprint loads it from a branch: {command} names one"),
        "{read}"
    );
    assert!(last(&read)["run"].is_null(), "{read}");

    // Named, and still nothing loaded through it: what ran under it is said all the same.
    write(&rig, "deploy", DEPLOY).await;
    let deploy = format!("{base}/dev/deploy");
    let (_, set) =
        rig.call(Method::PUT, &deploy, Some(&rig.token), Some(json!({ "blueprint": "deploy" }))).await;
    assert_eq!(last(&set["environment"])["kind"], "applied", "{set}");

    // Another environment, named and with nothing under it, never ran.
    rig.send(base, json!({ "name": "staging" })).await;
    rig.call(
        Method::PUT,
        &format!("{base}/staging/deploy"),
        Some(&rig.token),
        Some(json!({ "blueprint": "deploy" })),
    )
    .await;
    let staging = rig.read(&format!("{base}/staging")).await;
    assert_eq!(
        (&last(&staging)["kind"], &last(&staging)["said"]),
        (&json!("never"), &json!("never run")),
        "{staging}"
    );
}
