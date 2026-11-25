//! A blueprint's file may be called more than one way.
//!
//! `greet.bp` is the blueprint `greet`, and so is `greet.bp.yml`, which
//! every editor takes for the YAML a blueprint is a strict subset of. It
//! is the same blueprint under either name: applied, listed, vouched for,
//! and taken into a copy. A file called `.yml` and no more is nobody's
//! here, and a name with two files is read from neither.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

const GREET: &str = "blueprint: greet\nsteps:\n  said: { do: emit, with: { topic: shop.greeted } }\n";

async fn write(rig: &Rig, path: &str, text: &str) {
    let body = json!({ "path": path, "content": text });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

async fn shop() -> Rig {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig
}

/// What a refusal says.
fn said(answer: &Value) -> &str {
    answer["error"]["message"].as_str().unwrap_or_default()
}

/// What the library says of one name, as the client is told.
async fn listed(rig: &Rig, name: &str) -> Value {
    let all = rig.read("/api/blueprints?project=shop").await;
    let found = all["blueprints"].as_array().unwrap().iter().find(|one| one["name"] == name).cloned();
    found.unwrap_or(Value::Null)
}

#[tokio::test]
async fn a_projects_blueprint_called_as_yaml_is_the_blueprint_of_its_name() {
    let rig = shop().await;
    write(&rig, "@blueprints/greet.bp.yml", GREET).await;
    write(&rig, "@blueprints/tidy.bp.yaml", &GREET.replacen("greet", "tidy", 1)).await;
    // Somebody else's YAML, which a project is full of, is nobody's here.
    write(&rig, "compose.yml", "blueprint: compose\nsteps: {}\n").await;
    write(&rig, "@blueprints/ci.yaml", "blueprint: ci\nsteps: {}\n").await;

    // Listed by what it is called, with the file it is.
    assert_eq!(listed(&rig, "greet").await["file"], "@blueprints/greet.bp.yml");
    assert_eq!(listed(&rig, "tidy").await["file"], "@blueprints/tidy.bp.yaml");
    assert_eq!(listed(&rig, "compose").await, Value::Null);
    assert_eq!(listed(&rig, "ci").await, Value::Null);
    let used = rig.read("/api/blueprints/used").await;
    let own = used["projects"]["shop"]["own"].as_array().unwrap();
    let files: Vec<&str> = own.iter().map(|one| one["file"].as_str().unwrap()).collect();
    assert!(
        files.contains(&"@blueprints/greet.bp.yml") && files.contains(&"@blueprints/tidy.bp.yaml"),
        "{used}"
    );
    assert!(!files.iter().any(|file| file.ends_with("compose.yml") || file.ends_with("ci.yaml")), "{used}");

    // Applied by its name, whichever name its file is called by.
    for name in ["greet", "tidy"] {
        let applied =
            rig.send("/api/apply", json!({ "blueprint": name, "project": "shop", "wait": true })).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    }
    assert_eq!(rig.count_of("shop.greeted").await, 2);
    let (status, answer) = rig.post("/api/apply", json!({ "blueprint": "compose", "project": "shop" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{answer}");

    // What is wrong with it is said of the file as it is called.
    write(&rig, "@blueprints/greet.bp.yml", "blueprint: greet\nsteps:\n  said: { do: no.such.thing }\n")
        .await;
    let (status, answer) = rig.post("/api/apply", json!({ "blueprint": "greet", "project": "shop" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert!(said(&answer).contains("greet.bp.yml:3:"), "{answer}");
}

#[tokio::test]
async fn a_name_with_two_files_is_read_from_neither_and_both_are_named() {
    let rig = shop().await;
    write(&rig, "@blueprints/greet.bp", GREET).await;
    write(&rig, "@blueprints/greet.bp.yml", &GREET.replace("shop.greeted", "shop.greeted.twice")).await;

    let asked = json!({ "blueprint": "greet", "project": "shop", "wait": true });
    let (status, answer) = rig.post("/api/apply", asked.clone()).await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    let why = said(&answer);
    assert!(
        why.contains("in `shop` there is more than one file for the blueprint `greet`")
            && why.contains("`@blueprints/greet.bp` and `@blueprints/greet.bp.yml`")
            && why.contains("Take all but one away"),
        "{why}"
    );
    let (status, answer) = rig.get("/api/blueprints/source?name=greet&project=shop").await;
    assert_eq!(status, StatusCode::CONFLICT, "{answer}");
    assert!(said(&answer).contains("`@blueprints/greet.bp` and `@blueprints/greet.bp.yml`"), "{answer}");
    // It is one name, listed once, with no file said to be it.
    let all = rig.read("/api/blueprints?project=shop").await;
    let names = all["names"].as_array().unwrap();
    assert_eq!(names.iter().filter(|name| *name == "greet").count(), 1, "{all}");
    assert_eq!(listed(&rig, "greet").await["file"], Value::Null);

    // Something that would start it by its name is told so where it does,
    // as what is wrong with it; and of either file itself it is said and
    // no more, since whoever reads it may be about to take the other away.
    let caller = "blueprint: caller\nsteps:\n  greet: { call: greet }\n";
    let checked = rig.send("/api/blueprints/check", json!({ "source": caller, "project": "shop" })).await;
    let found = |checked: &Value, severity: &str| {
        let all = checked["diagnostics"].as_array().unwrap();
        all.iter().any(|one| one["code"] == "several-files" && one["severity"] == severity)
    };
    assert!(checked["ok"] == false && found(&checked, "error"), "{checked}");
    let checked = rig.send("/api/blueprints/check", json!({ "source": GREET, "project": "shop" })).await;
    assert!(checked["ok"] == true && found(&checked, "warning"), "{checked}");
    assert_eq!(rig.count_of("shop.greeted").await + rig.count_of("shop.greeted.twice").await, 0);

    // One of them taken away, the other is the blueprint.
    let (status, answer) = rig
        .call(Method::DELETE, "/api/projects/shop/file?path=@blueprints/greet.bp", Some(&rig.token), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let applied = rig.send("/api/apply", asked).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(rig.count_of("shop.greeted.twice").await, 1);
}

#[tokio::test]
async fn a_folder_that_came_with_a_fleet_blueprint_called_as_yaml_is_not_given_a_second_and_theirs_is_not_applied()
 {
    let rig = Rig::new().await;
    let theirs = rig.dir.join("theirs");
    std::fs::create_dir_all(&theirs).unwrap();
    let text = "blueprint: fleet\nsteps:\n  up:\n    service: { name: theirs, cmd: [sleep, '600'] }\n";
    std::fs::write(theirs.join("fleet.bp.yml"), text).unwrap();

    let made = rig.send("/api/projects", json!({ "name": "Theirs", "path": theirs, "set_up": true })).await;
    assert!(made["set_up"]["board"].is_string() && made["set_up"]["fleet"].is_null(), "{made}");
    let left = made["set_up"]["left"][0].as_str().unwrap_or_default();
    assert!(left.contains("a fleet blueprint of its own (`fleet.bp.yml`)"), "{made}");
    // Their file is as they wrote it, and no other was written beside it.
    assert_eq!(std::fs::read_to_string(theirs.join("fleet.bp.yml")).unwrap(), text);
    assert!(!theirs.join(brand::blueprint_file("fleet")).exists());
    // Setting the fleet up by hand finds theirs, and writes no second.
    let (status, answer) = rig.post("/api/projects/theirs/fleet", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["path"], "fleet.bp.yml", "{answer}");
    assert!(!theirs.join(brand::blueprint_file("fleet")).exists());
}

#[tokio::test]
async fn a_blueprint_called_as_yaml_is_taken_into_a_copy_of_the_factory_and_is_the_same_blueprint_there() {
    use crate::snapshots::{Shared, taken};
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    prod.send("/api/projects", json!({ "name": "Shop" })).await;
    write(&prod, "@blueprints/greet.bp.yml", GREET).await;

    let at = shared.snapshot("first");
    taken(&prod, &at).await;
    let copy = shared.copy("staging", &at).await;
    assert_eq!(listed(&copy, "greet").await["file"], "@blueprints/greet.bp.yml");
    let applied =
        copy.send("/api/apply", json!({ "blueprint": "greet", "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(copy.count_of("shop.greeted").await, 1);
}
