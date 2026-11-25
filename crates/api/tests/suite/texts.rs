//! How what is running comes by new texts, over HTTP: the factory's
//! setting, who may change it, and a project's own file taken as it
//! changes only where the owner has said texts are followed.
//!
//! Following gives up what vouching is for (`vouched.rs`): a file written
//! by somebody who holds less is then run with the authority of whoever
//! applied what calls it. So it is tried here with a key that holds what
//! an agent does, and turned on with keys that hold less than it takes.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A key of the rig's owner that holds `scopes` and nothing else.
async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

/// What an agent holds unless its slot says otherwise.
const AGENTS: &[&str] =
    &["projects", "channels", "board", "records", "blueprints", "agents:read", "fleet:read", "search"];

/// Starts a worker when it is asked for one, and says where its tree reports.
const WATCHER: &str = "\
blueprint: watcher
params: { channel: general }
on:
  wanted:
    event: shop.wanted
    steps:
      go: { spawn: worker, as: the-worker }
";

fn worker(version: u32, says: &str) -> String {
    format!(
        "blueprint: worker\nversion: {version}\nsteps:\n  claim: {{ do: emit, with: {{ topic: shop.claimed }} }}\non:\n  \
         poke:\n    event: shop.poke\n    steps:\n      say: {{ do: emit, with: {{ topic: {says} }} }}\n"
    )
}

/// Writes one of the project's own blueprints with `token`.
async fn write(rig: &Rig, token: &str, name: &str, text: &str) {
    let path = format!("@blueprints/{}", brand::blueprint_file(name));
    let body = json!({ "path": path, "content": text });
    let (status, answer) = rig.call(Method::PUT, "/api/projects/shop/file", Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

/// A shop whose watcher has started its worker: the worker.
async fn shop(rig: &Rig) -> Value {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    write(rig, &rig.token, "watcher", WATCHER).await;
    write(rig, &rig.token, "worker", &worker(1, "shop.old")).await;
    let path = format!("@blueprints/{}", brand::blueprint_file("watcher"));
    let applied = rig.send("/api/apply", json!({ "project": "shop", "path": path, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    rig.send("/api/emit/shop.wanted", json!({})).await;
    rig.event("shop.claimed", |_| true).await;
    the_worker(rig).await
}

async fn the_worker(rig: &Rig) -> Value {
    let all = rig.read("/api/instances?live=true&project=shop").await;
    let found = all["instances"].as_array().unwrap().iter().find(|one| one["key"] == "the-worker");
    found.unwrap_or_else(|| panic!("no worker: {all}")).clone()
}

/// Waits for the worker to say it runs `text`.
async fn runs(rig: &Rig, text: &str) {
    let revision = blueprint::revision(text);
    rig.event("instance.updated", move |event| event.data["revision"] == revision.as_str()).await;
}

async fn set(rig: &Rig, token: &str, asked: Value) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/system/texts", Some(token), Some(asked)).await
}

#[tokio::test]
async fn a_projects_file_is_taken_as_it_changes_only_where_the_owner_said_texts_are_followed() {
    let rig = Rig::new().await;
    let first = shop(&rig).await;
    assert_eq!(first["revision"], blueprint::revision(&worker(1, "shop.old")));
    let usual = rig.read("/api/system/texts").await;
    assert_eq!(
        (&usual["come"], &usual["land"], &usual["usual"]),
        (&json!("usual"), &json!("in_place"), &json!("held"))
    );

    // Somebody who holds what an agent does writes the worker's file. Nothing
    // was said of following: what is running goes on as it was vouched for.
    let agent = key(&rig, AGENTS).await;
    let second = worker(2, "shop.new");
    write(&rig, &agent, "worker", &second).await;
    rig.event("project.file.changed", |event| {
        event.data["path"].as_str().is_some_and(|path| path.contains("worker"))
    })
    .await;
    assert_eq!(the_worker(&rig).await["revision"], first["revision"]);

    // The owner says texts are followed here. From then the file as it
    // stands is what runs, taken where the worker is, and nobody applied.
    let (status, now) = set(&rig, &rig.token, json!({ "come": "follow" })).await;
    assert_eq!(status, StatusCode::OK, "{now}");
    assert_eq!((&now["come"], &now["not_taken"]), (&json!("follow"), &json!([])));
    assert_eq!(the_worker(&rig).await["revision"], blueprint::revision(&second));
    // And each change after, as it is written, by whoever may write the file.
    let third = worker(3, "shop.newer");
    write(&rig, &agent, "worker", &third).await;
    runs(&rig, &third).await;
    rig.send("/api/emit/shop.poke", json!({})).await;
    rig.event("shop.newer", |_| true).await;
    let said = rig.event("system.texts", |_| true).await;
    assert!(said.data["gives_up"].as_str().unwrap().contains("an agent among them"), "{}", said.data);

    // Said to be held again, the next change is left, as before.
    let (status, _) = set(&rig, &rig.token, json!({ "come": "held" })).await;
    assert_eq!(status, StatusCode::OK);
    let before = rig.mark();
    write(&rig, &agent, "worker", &worker(4, "shop.newest")).await;
    rig.event("project.file.changed", move |event| event.seq > before).await;
    assert_eq!(the_worker(&rig).await["revision"], blueprint::revision(&third));
}

#[tokio::test]
async fn a_followed_file_that_does_not_check_is_not_taken_and_is_said_once_where_its_tree_reports() {
    let rig = Rig::new().await;
    let first = shop(&rig).await;
    set(&rig, &rig.token, json!({ "come": "follow" })).await;
    let broken = worker(2, "shop.new").replace("do: emit, with: { topic: shop.new }", "do: shop.nothing");
    write(&rig, &rig.token, "worker", &broken).await;
    let told = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().is_some_and(|body| body.contains("does not check"))
        })
        .await;
    let body = told.data["body"].as_str().unwrap();
    assert!(
        body.contains("`the-worker` goes on with the text it has") && body.contains("shop.nothing"),
        "{body}"
    );
    // It goes on with the last that did, and says so beside itself.
    let now = the_worker(&rig).await;
    assert_eq!(now["revision"], first["revision"]);
    assert!(
        now["error"].as_str().unwrap_or_default().starts_with("`worker` was changed and does not check"),
        "{now}"
    );
    rig.send("/api/emit/shop.poke", json!({})).await;
    rig.event("shop.old", |_| true).await;
    // Put right, it is taken and nothing more is said of it.
    let mended = worker(3, "shop.new");
    write(&rig, &rig.token, "worker", &mended).await;
    runs(&rig, &mended).await;
    assert_eq!(the_worker(&rig).await["error"], Value::Null);
    let left = rig.read("/api/events/recent?topics=instance.text_left&limit=20").await;
    assert_eq!(left["events"].as_array().unwrap().len(), 1, "{left}");
}

#[tokio::test]
async fn the_setting_is_the_owners_and_following_takes_what_taking_a_wall_away_takes() {
    let rig = Rig::new().await;
    // Whoever may only read is told it, and may not change it.
    let member = key(&rig, &["blueprints", "projects"]).await;
    let (status, read) = rig.call(Method::GET, "/api/system/texts", Some(&member), None).await;
    assert_eq!((status, &read["come"]), (StatusCode::OK, &json!("usual")));
    let (status, _) = set(&rig, &member, json!({ "come": "held" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Somebody who runs the factory and holds no `shell`: the rest is theirs, following is not.
    let admin = key(&rig, &["admin", "blueprints"]).await;
    let (status, refused) = set(&rig, &admin, json!({ "come": "follow" })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap().contains("takes `shell`"), "{refused}");
    assert_eq!(rig.read("/api/system/texts").await["come"], "usual", "nothing was changed");
    let (status, now) = set(&rig, &admin, json!({ "come": "held", "land": "again" })).await;
    assert_eq!(status, StatusCode::OK, "{now}");
    assert_eq!((&now["come"], &now["land"]), (&json!("held"), &json!("again")));
    let (status, unknown) = set(&rig, &rig.token, json!({ "land": "sideways" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{unknown}");
}

#[tokio::test]
async fn making_a_blueprint_that_ships_a_projects_own_takes_writing_its_files_and_saying_how_they_differ_reading()
 {
    let rig = Rig::new().await;
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let ask = "/api/projects/shop/own";
    let reader = key(&rig, &["projects:read", "blueprints"]).await;
    let (status, _) =
        rig.call(Method::POST, ask, Some(&reader), Some(json!({ "blueprint": "std/pass-by-hand" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "it writes a file of the project's");

    let made = rig.send(ask, json!({ "blueprint": "std/pass-by-hand" })).await;
    let file = format!("@blueprints/{}", brand::blueprint_file("std/pass-by-hand"));
    assert_eq!(
        (&made["path"], &made["held"], &made["differs"]["same"]),
        (&json!(file), &json!(false), &json!(true))
    );
    // Where the factory keeps the project's own, and not in its folder.
    let on_disk = rig.placed("shop").join(brand::blueprint_file("std/pass-by-hand"));
    assert!(!std::path::Path::new(project["path"].as_str().unwrap()).join("blueprints").exists());
    let text = std::fs::read_to_string(&on_disk).unwrap();
    assert_eq!(made["version"], json!(blueprint::version(&text)), "it says which version it copied");
    assert!(text.contains("blueprint: std/pass-by-hand"), "{text}");

    // Saying where the two differ only reads; taking the copy away writes.
    let say = json!({ "blueprint": "std/pass-by-hand", "say": true });
    let (status, said) = rig.call(Method::POST, ask, Some(&reader), Some(say)).await;
    assert_eq!((status, &said["differs"]["same"]), (StatusCode::OK, &json!(true)), "{said}");
    let back = json!({ "blueprint": "std/pass-by-hand", "back": true });
    let (status, _) = rig.call(Method::POST, ask, Some(&reader), Some(back.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(rig.send(ask, back).await["removed"], json!(file));
    assert!(!on_disk.exists());
    let (status, nothing) =
        rig.call(Method::POST, ask, Some(&rig.token), Some(json!({ "blueprint": "shop/nothing" }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{nothing}");
}

#[tokio::test]
async fn a_name_the_project_has_two_files_for_is_not_copied_over_said_of_or_taken_away() {
    let rig = Rig::new().await;
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // Somebody who may write the project's files has put the same blueprint there under two of its file's names.
    let writer = key(&rig, &["projects", "blueprints"]).await;
    let ships = include_str!("../../../../blueprints/std/pass-by-hand.bp");
    let files: Vec<String> = brand::blueprint_files("std/pass-by-hand")
        .into_iter()
        .take(2)
        .map(|file| format!("@blueprints/{file}"))
        .collect();
    assert_eq!(files.len(), 2, "a blueprint's file may have more names than one: {files:?}");
    for file in &files {
        let body = json!({ "path": file, "content": ships });
        let (status, answer) =
            rig.call(Method::PUT, "/api/projects/shop/file", Some(&writer), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
    }
    // Every form of it is refused, naming both, and nothing is written or taken away.
    for asked in
        [json!({}), json!({ "say": true }), json!({ "back": true }), json!({ "back": true, "force": true })]
    {
        let mut body = asked.clone();
        body["blueprint"] = json!("std/pass-by-hand");
        let (status, refused) =
            rig.call(Method::POST, "/api/projects/shop/own", Some(&writer), Some(body)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{asked}: {refused}");
        let said = refused["error"]["message"].as_str().unwrap();
        assert!(said.contains("more than one file for the blueprint `std/pass-by-hand`"), "{said}");
        assert!(files.iter().all(|file| said.contains(file.as_str())), "{said}");
    }
    let _ = project;
    let folder = rig.placed("shop").join("std");
    let mut there: Vec<String> = std::fs::read_dir(&folder)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    there.sort();
    assert_eq!(there.len(), 2, "{there:?}");
}
