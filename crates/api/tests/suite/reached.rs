//! A blueprint and the build that reads it, from the outside: a project's
//! own blueprints are files, and a file changes whenever somebody saves it.
//! Applying one checks everything it reaches, and an instance that is
//! running goes on with the project's files as they were when it was
//! applied.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

const WATCHER: &str = "\
blueprint: watcher
on:
  said:
    event: channel.message
    steps:
      go: { call: worker, as: the-worker }
";

fn worker(about: &str) -> String {
    format!("blueprint: worker\nsteps:\n  note: {{ do: emit, with: {{ topic: shop.worked{about} }} }}\n")
}

/// A worker written for a build whose `log` takes `loudly`.
const NEWER: &str =
    "blueprint: worker\nsteps:\n  note: { do: log, with: { message: worked, loudly: true } }\n";

const WRONG: &str = "`log` has no argument `loudly`";

/// A project whose blueprints are these files, and a channel to speak in.
async fn shop(rig: &Rig, files: &[(&str, &str)]) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    for (name, content) in files {
        let path = format!("@blueprints/{}", brand::blueprint_file(name));
        let body = json!({ "path": path, "content": content });
        let (status, answer) =
            rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
    }
}

/// Where the project keeps the blueprint called `name`, on disk.
fn file(rig: &Rig, name: &str) -> std::path::PathBuf {
    rig.placed("shop").join(brand::blueprint_file(name))
}

async fn apply(rig: &Rig, name: &str) -> (StatusCode, Value) {
    let path = format!("@blueprints/{}", brand::blueprint_file(name));
    rig.post("/api/apply", json!({ "path": path, "project": "shop", "wait": true })).await
}

/// Says something in the channel and waits until `count` runs of the
/// watcher's handler have finished.
async fn said(rig: &Rig, count: usize) -> Vec<Value> {
    rig.send("/api/channels/general/messages", json!({ "body": "go" })).await;
    let mut follow = rig.follow("run.finished");
    crate::soon(&format!("{count} runs of the handler"), async {
        let mut runs = Vec::new();
        while runs.len() < count {
            let run = follow.next().await.unwrap();
            if run.data["flow"] == "on.said" {
                runs.push(run.data.clone());
            }
        }
        runs
    })
    .await
}

async fn events(rig: &Rig, topic: &str) -> Vec<Value> {
    let query = journal::EventQuery { topics: topic.to_string(), limit: 1000, ..Default::default() };
    rig.said(query).await.into_iter().map(|event| event.data).collect()
}

#[tokio::test]
async fn applying_a_projects_blueprint_is_refused_while_a_file_it_calls_does_not_check_and_the_editor_is_told_the_same()
 {
    let rig = Rig::new().await;
    shop(&rig, &[("watcher", WATCHER), ("worker", NEWER)]).await;

    let (status, refused) = apply(&rig, "watcher").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let why = refused["error"]["message"].as_str().unwrap();
    assert!(
        why.contains(&format!(
            "this step calls `worker`, which has 1 problem(s): worker.bp:3:3: error: {WRONG}"
        )),
        "{why}"
    );
    assert!(
        why.contains(&format!(
            "`worker` may have been written for a newer version than this one, which is {}",
            env!("CARGO_PKG_VERSION")
        )),
        "{why}"
    );
    assert_eq!(rig.read("/api/instances?project=shop").await["instances"], json!([]), "nothing was started");

    let checked = rig.send("/api/blueprints/check", json!({ "source": WATCHER, "project": "shop" })).await;
    assert_eq!(checked["ok"], false);
    let found: Vec<&Value> = checked["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|found| found["severity"] == "error")
        .collect();
    assert_eq!(found.len(), 1, "{checked}");
    assert_eq!(
        (found[0]["code"].as_str(), found[0]["span"]["start"]["line"].as_u64()),
        (Some("reaches-unfit"), Some(6)),
        "against the step that calls it"
    );

    // Mended where it is, it applies.
    std::fs::write(file(&rig, "worker"), worker("")).unwrap();
    let (status, applied) = apply(&rig, "watcher").await;
    assert_eq!((status, &applied["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{applied}");
}

/// Saves the project's blueprint called `name` through the daemon, as its owner.
async fn save(rig: &Rig, name: &str, content: &str) {
    let path = format!("@blueprints/{}", brand::blueprint_file(name));
    let body = json!({ "path": path, "content": content });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

/// The watcher, as it is shown.
async fn watcher(rig: &Rig) -> Value {
    let shown = rig.read("/api/instances?project=shop").await;
    let all = shown["instances"].as_array().unwrap();
    all.iter().find(|one| one["key"] == "watcher").cloned().unwrap()
}

#[tokio::test]
async fn a_running_instance_goes_on_with_a_projects_file_as_it_was_applied_until_somebody_who_may_says_otherwise()
 {
    let rig = Rig::new().await;
    shop(&rig, &[("watcher", WATCHER), ("worker", &worker(""))]).await;
    let (status, applied) = apply(&rig, "watcher").await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    said(&rig, 1).await;
    assert_eq!(events(&rig, "shop.worked").await.len(), 1);

    // The file is changed where it is, and not through the daemon: a
    // merge, an agent's program, an editor. Nobody who holds what the
    // watcher runs with has said that this is to be run.
    std::fs::write(file(&rig, "worker"), worker(", again: true")).unwrap();
    let runs = said(&rig, 2).await;
    assert_eq!(runs[1]["status"], "ok", "{runs:?}");
    let worked = events(&rig, "shop.worked").await;
    assert_eq!(worked.len(), 2, "the work was done, as the file was when the watcher was applied");
    assert!(worked.iter().all(|data| data["again"].is_null()), "{worked:?}");
    let kept = events(&rig, "instance.call_kept").await;
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0]["blueprint"], "worker");
    let why = kept[0]["error"].as_str().unwrap();
    assert!(
        why.contains("changed after `watcher` was applied") && why.contains("apply `watcher` again"),
        "{why}"
    );
    let note = watcher(&rig).await["error"].as_str().unwrap_or_default().to_string();
    assert!(note.contains("so `go` went on with it as it was applied"), "{note}");

    // Written for another build, it is as little the file that was applied.
    std::fs::write(file(&rig, "worker"), NEWER).unwrap();
    said(&rig, 3).await;
    assert_eq!(events(&rig, "shop.worked").await.len(), 3);
    // Whoever asks for that file to be applied is told, and nothing of it runs.
    let (status, refused) = apply(&rig, "worker").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap().contains(WRONG), "{refused}");

    // Taken away altogether, it is stood in for the same.
    std::fs::remove_file(file(&rig, "worker")).unwrap();
    said(&rig, 4).await;
    assert_eq!(events(&rig, "shop.worked").await.len(), 4);
    let kept = events(&rig, "instance.call_kept").await;
    assert_eq!(kept.len(), 3);
    assert!(kept[2]["error"].as_str().unwrap().contains("is not one now"), "{kept:?}");

    // Saved through the daemon by the one who applied the watcher, it is
    // the one that runs, and nothing is said: they could have applied it.
    save(&rig, "worker", &worker(", again: true")).await;
    said(&rig, 5).await;
    let worked = events(&rig, "shop.worked").await;
    assert_eq!(worked.iter().filter(|data| data["again"] == true).count(), 1, "{worked:?}");
    assert_eq!(events(&rig, "instance.call_kept").await.len(), 3);
    let shown = watcher(&rig).await;
    assert!(shown["error"].is_null(), "what it said of itself is taken back: {shown}");

    // Changed where it is once more, and the watcher applied again: whoever
    // applies is told which of the project's files they have taken up.
    // Changed where it is once more, and the watcher applied again: what
    // was asked for is the watcher, and the file it calls is left as it
    // was, with whoever applies told that it has changed.
    std::fs::write(file(&rig, "worker"), worker(", once_more: true")).unwrap();
    let (status, applied) = apply(&rig, "watcher").await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["vouched"], json!({ "left": ["worker"] }), "{applied}");
    said(&rig, 6).await;
    assert!(events(&rig, "shop.worked").await.iter().all(|data| data["once_more"].is_null()));
    assert_eq!(events(&rig, "instance.call_kept").await.len(), 4);

    // Said to take the project's files as they are, it does, and says which.
    let path = format!("@blueprints/{}", brand::blueprint_file("watcher"));
    let taking = json!({ "path": path, "project": "shop", "wait": true, "take": true });
    let (status, applied) = rig.post("/api/apply", taking).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["vouched"], json!({ "taken": ["worker"] }), "{applied}");
    said(&rig, 7).await;
    let worked = events(&rig, "shop.worked").await;
    assert_eq!(worked.iter().filter(|data| data["once_more"] == true).count(), 1, "{worked:?}");
    assert_eq!(events(&rig, "instance.call_kept").await.len(), 4);
}
