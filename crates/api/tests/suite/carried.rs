//! `on_update`, from the outside: a project's blueprint is a file, and an
//! instance that is running is carried over to the file as it is saved,
//! with what the file says done in place of its steps, when the file says
//! how. If that fails it is back as it was, state and all.

use kernel::Source;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Starts a worker when it is asked for one; none of its steps names it.
const WATCHER: &str = "\
blueprint: watcher
on:
  wanted:
    event: shop.wanted
    steps:
      go: { spawn: worker, as: the-worker }
";

/// The worker as it starts: it claims once, and counts what it hears.
const ONE: &str = "\
blueprint: worker
version: 1
state: { seen: 0 }
steps:
  claim: { do: emit, with: { topic: shop.claimed } }
on:
  poke:
    event: shop.poke
    steps:
      note: { do: set, with: { seen: ${ state.seen + 1 } } }
      say: { do: emit, with: { topic: shop.old } }
";

/// As it is saved next: other steps, what it counts called something
/// else, and how one that is running comes to it.
const TWO: &str = "\
blueprint: worker
version: 2
state: { heard: 0 }
steps:
  claim: { do: emit, with: { topic: shop.claimed_again } }
on:
  poke:
    event: shop.poke
    steps:
      note: { do: set, with: { heard: ${ state.heard + 1 } } }
      say: { do: emit, with: { topic: shop.new } }
on_update:
  - do: set
    with: { heard: ${ was.state.seen } }
  - emit: { topic: shop.carried, from: \"${ was.version }\" }
";

async fn shop(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    for (name, content) in [("watcher", WATCHER), ("worker", ONE)] {
        let path = format!("@blueprints/{}", brand::blueprint_file(name));
        let body = json!({ "path": path, "content": content });
        let (status, answer) =
            rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
    }
}

/// Somebody saves the worker's file, where it is.
fn save(rig: &Rig, text: &str) {
    let file = rig.placed("shop").join(brand::blueprint_file("worker"));
    std::fs::write(file, text).unwrap();
}

async fn apply_watcher(rig: &Rig) -> Value {
    let path = format!("@blueprints/{}", brand::blueprint_file("watcher"));
    rig.send("/api/apply", json!({ "path": path, "project": "shop", "wait": true })).await
}

async fn emit(rig: &Rig, topic: &str) {
    rig.node().app.journal.emit(topic, Source::default(), json!({})).await.unwrap();
}

async fn the_worker(rig: &Rig) -> Value {
    let listed = rig.read("/api/instances?project=shop&live=true").await;
    let all = listed["instances"].as_array().unwrap();
    all.iter()
        .find(|one| one["key"] == "the-worker")
        .unwrap_or_else(|| panic!("no worker in {listed}"))
        .clone()
}

async fn count(rig: &Rig, topic: &str) -> usize {
    let query = journal::EventQuery { topics: topic.to_string(), limit: 100, ..Default::default() };
    rig.said(query).await.len()
}

#[tokio::test]
async fn an_instance_is_carried_over_to_a_saved_file_that_says_how_and_is_put_back_when_that_fails() {
    let rig = Rig::new().await;
    shop(&rig).await;
    apply_watcher(&rig).await;
    emit(&rig, "shop.wanted").await;
    rig.count("shop.claimed", 1).await;
    let id = the_worker(&rig).await["id"].as_str().unwrap().to_string();
    rig.event("instance.active", |event| event.source.instance.as_deref() == Some(id.as_str())).await;
    emit(&rig, "shop.poke").await;
    rig.count("shop.old", 1).await;

    // The file is saved with other steps, and says how to come to it.
    save(&rig, TWO);
    let behind = the_worker(&rig).await["behind"].clone();
    assert_eq!(
        behind,
        json!({
            "revision": blueprint::revision(TWO),
            "version": "2",
            "takes": "carried",
            "changed": [
                "the step `claim` is changed",
                "the handler `poke` is changed",
                "it now says what it does when it is taken by one that is running (`on_update`)",
            ],
            "why": "The text of `worker` now there says how one that is running takes it (`on_update`). It \
                    is carried over to that text where it is, with that done in place of its steps, which \
                    are not run again, when what started it, or it itself, is applied again.",
        })
    );

    // Applying what started it carries it over.
    assert_eq!(apply_watcher(&rig).await["changed"], false);
    let said = rig.event("shop.carried", |_| true).await;
    assert_eq!(said.data["from"], "1");
    rig.event("run.finished", |event| event.data["flow"] == "on_update").await;
    let is = the_worker(&rig).await;
    assert_eq!(
        (&is["revision"], &is["version"], &is["behind"], &is["status"], &is["state"]),
        (
            &json!(blueprint::revision(TWO)),
            &json!("2"),
            &Value::Null,
            &json!("active"),
            &json!({ "heard": 1 })
        ),
        "{is}"
    );
    assert_eq!(
        (count(&rig, "shop.claimed").await, count(&rig, "shop.claimed_again").await),
        (1, 0),
        "the steps of neither text were run"
    );
    let runs = rig.read(&format!("/api/runs?instance={id}&limit=10")).await;
    let flows: Vec<&str> =
        runs["runs"].as_array().unwrap().iter().rev().map(|run| run["flow"].as_str().unwrap()).collect();
    assert_eq!(flows, ["main", "on.poke", "on_update"], "{runs}");
    emit(&rig, "shop.poke").await;
    rig.count("shop.new", 1).await;
    assert_eq!(count(&rig, "shop.old").await, 1, "its handler is the new text's");

    // Saved again with something in `on_update` that cannot be done, and
    // applied by itself: it is back on what it had, with the state it had.
    let failing = TWO.replace("version: 2", "version: 3").replace(
        "  - emit: { topic: shop.carried",
        "  - do: set\n    with: { heard: 99 }\n  - do: channel.post\n    with: { channel: nowhere, body: lost }\n  - emit: { topic: shop.carried",
    );
    save(&rig, &failing);
    let (status, refused) = rig.post(&format!("/api/instances/{id}/apply"), json!({ "wait": true })).await;
    assert_eq!(status, StatusCode::OK, "{refused}");
    assert_eq!(
        (&refused["finished"]["flow"], &refused["finished"]["status"]),
        (&json!("on_update"), &json!("failed"))
    );
    let error = refused["finished"]["error"].as_str().unwrap();
    assert!(error.starts_with("`on_update` entry 3, `channel.post`: "), "{error}");
    let is = the_worker(&rig).await;
    assert_eq!(
        (&is["revision"], &is["status"], &is["state"]),
        (&json!(blueprint::revision(TWO)), &json!("active"), &json!({ "heard": 2 })),
        "{is}"
    );
    let says = is["error"].as_str().unwrap();
    assert!(
        says.starts_with("update failed, still running the previous version: `on_update` entry 3"),
        "{says}"
    );
    assert_eq!(is["behind"]["takes"], "carried", "the file is still another, and still says how");
    assert_eq!(count(&rig, "shop.carried").await, 1);

    // Mended, it is carried over; and asked for its steps, they run.
    let mended = failing.replace("  - do: channel.post\n    with: { channel: nowhere, body: lost }\n", "");
    save(&rig, &mended);
    let applied = rig.send(&format!("/api/instances/{id}/apply"), json!({ "wait": true })).await;
    assert_eq!(
        (&applied["finished"]["flow"], &applied["finished"]["status"]),
        (&json!("on_update"), &json!("ok"))
    );
    assert_eq!(the_worker(&rig).await["state"], json!({ "heard": 99 }));
    assert_eq!(count(&rig, "shop.claimed_again").await, 0);
    let forced =
        rig.send(&format!("/api/instances/{id}/apply"), json!({ "wait": true, "force": true })).await;
    assert_eq!((&forced["finished"]["flow"], &forced["finished"]["status"]), (&json!("main"), &json!("ok")));
    assert_eq!(count(&rig, "shop.claimed_again").await, 1);
}
