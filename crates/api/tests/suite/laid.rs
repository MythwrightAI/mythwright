//! Laying a blueprint out, as the editor asks for it: the text comes back
//! the one way, or as the project it belongs to says; and a running
//! instance is not behind a file that was only laid out again.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// Written carelessly: set in by four, braces with no room in them, and a
/// map too long for its line.
const WORKER: &str = "\
blueprint: worker
params:
    room: {type: string,default: general}
steps:
    claim: {do: emit, with: {topic: shop.claimed, room: \"${ params.room }\", told: \"a sentence that is long enough to carry this line past a hundred columns\"}}
on:
    poke:
        event: shop.poke
        do: emit
        with: {topic: shop.poked}
";

const LAID: &str = "\
blueprint: worker

params:
  room: { type: string, default: general }

steps:
  claim:
    do: emit
    with:
      topic: shop.claimed
      room: \"${params.room}\"
      told: \"a sentence that is long enough to carry this line past a hundred columns\"

on:
  poke:
    event: shop.poke
    do: emit
    with: { topic: shop.poked }
";

async fn put(rig: &Rig, path: &str, content: &str) {
    let body = json!({ "path": path, "content": content });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

fn worker_file() -> String {
    format!("@blueprints/{}", brand::blueprint_file("worker"))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_text_is_laid_out_the_one_way_or_as_its_project_says_and_one_that_does_not_read_is_left() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;

    // A text that is nobody's file is laid out as it is for everyone.
    let laid = rig.send("/api/blueprints/format", json!({ "source": WORKER })).await;
    assert_eq!(laid["formatted"], LAID, "{laid}");
    // What is laid out comes back as it went.
    let again = rig.send("/api/blueprints/format", json!({ "source": LAID })).await;
    assert_eq!(again["formatted"], LAID);

    // The project says a few things of its own, in a file of its
    // repository; the nearest to the blueprint is the one that counts.
    put(&rig, blueprint::format::SETTINGS_FILE, "indent: 4\nexpressions: spaced\n").await;
    let asked = json!({ "source": WORKER, "project": "shop", "path": worker_file() });
    let theirs = rig.send("/api/blueprints/format", asked.clone()).await;
    let theirs = theirs["formatted"].as_str().unwrap();
    assert!(theirs.contains("\nparams:\n    room: { type: string, default: general }\n"), "{theirs}");
    assert!(theirs.contains("room: \"${ params.room }\""), "{theirs}");
    put(&rig, &format!("@blueprints/{}", blueprint::format::SETTINGS_FILE), "width: 200\n").await;
    let nearer = rig.send("/api/blueprints/format", asked.clone()).await;
    let nearer = nearer["formatted"].as_str().unwrap();
    assert!(nearer.contains("\n  claim: { do: emit, with: { topic: shop.claimed"), "{nearer}");
    assert!(nearer.contains("room: \"${params.room}\""), "the file further up is not read as well: {nearer}");

    // A setting that is not one is said, with the file that says it.
    put(&rig, &format!("@blueprints/{}", blueprint::format::SETTINGS_FILE), "tabs: true\n").await;
    let (status, refused) = rig.post("/api/blueprints/format", asked).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let said = refused["error"]["message"].as_str().unwrap_or_default();
    assert!(
        said.contains(blueprint::format::SETTINGS_FILE) && said.contains("`tabs` is not something"),
        "{refused}"
    );
    assert!(said.contains("width, indent"), "it lists what can be set: {refused}");

    // One that does not read is not laid out; the check is what shows why.
    let unread =
        rig.send("/api/blueprints/format", json!({ "source": "blueprint: x\n  steps: oops\n" })).await;
    assert_eq!(unread["formatted"], Value::Null, "{unread}");
    assert!(unread["why"].as_str().unwrap().contains("unexpected indentation"), "{unread}");

    // Only ever looked for inside the project, and only by someone who may read blueprints.
    let outside = json!({ "source": LAID, "project": "shop", "path": "../../elsewhere/x.bp" });
    assert_eq!(rig.post("/api/blueprints/format", outside).await.0, StatusCode::BAD_REQUEST);
    let nobody =
        rig.call(Method::POST, "/api/blueprints/format", None, Some(json!({ "source": LAID }))).await;
    assert_eq!(nobody.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_instance_is_not_behind_a_file_that_was_only_laid_out_again_and_applying_it_does_nothing() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    put(&rig, &worker_file(), WORKER).await;
    let applied =
        rig.send("/api/apply", json!({ "path": worker_file(), "project": "shop", "wait": true })).await;
    assert_eq!(applied["created"], true, "{applied}");
    rig.count("shop.claimed", 1).await;
    let shown = |listed: Value| listed["instances"].as_array().unwrap()[0].clone();
    let before = shown(rig.read("/api/instances?project=shop&live=true").await);
    assert_eq!(before["behind"], Value::Null, "{before}");

    // The file is laid out again, and given a comment: it says what it
    // said. The instance is not behind it, and applying it does nothing.
    let file = rig.placed("shop").join(brand::blueprint_file("worker"));
    std::fs::write(&file, format!("# Laid out, and nothing more.\n{LAID}")).unwrap();
    let after = shown(rig.read("/api/instances?project=shop&live=true").await);
    assert_eq!(after["behind"], Value::Null, "laying a blueprint out is not an update: {after}");
    let again =
        rig.send("/api/apply", json!({ "path": worker_file(), "project": "shop", "wait": true })).await;
    assert_eq!((&again["created"], &again["changed"]), (&json!(false), &json!(false)), "{again}");
    assert_eq!(again["run"], Value::Null, "nothing is run: {again}");
    rig.count("shop.claimed", 1).await;

    // A file that says something else is another matter.
    std::fs::write(&file, LAID.replace("shop.poked", "shop.prodded")).unwrap();
    let moved = shown(rig.read("/api/instances?project=shop&live=true").await);
    assert_eq!(moved["behind"]["takes"], "in_place", "{moved}");
}
