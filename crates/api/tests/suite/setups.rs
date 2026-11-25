//! Setting a project up is a blueprint, `std/project`, applied for it as
//! whoever adds it, whose rows are what it starts with: those that ship,
//! a row of somebody's own blueprint beside them, and setups the factory
//! keeps, one of them its default for new projects.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// The instances of `blueprint` in `project`, alive or not.
async fn of(rig: &Rig, project: &str, blueprint: &str) -> Vec<Value> {
    let all = rig.read(&format!("/api/instances?project={project}")).await;
    all["instances"].as_array().unwrap().iter().filter(|one| one["blueprint"] == blueprint).cloned().collect()
}

/// A row of one's own: a library kept in another folder, under another name.
fn notes() -> Value {
    json!({ "name": "notes", "title": "Notes", "blueprint": "std/docs", "with": { "folder": "notes" } })
}

#[tokio::test]
async fn a_project_is_set_up_by_std_project_applied_for_it_with_a_row_of_its_own_beside_the_usual() {
    let rig = Rig::new().await;
    let made = rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["board", notes()] })).await;
    let given = &made["set_up"];
    assert!(given["board"].is_string() && given["notes"].is_string(), "{made}");
    assert!(given["fleet"].is_null() && given["docs"].is_null() && given["left"].is_null(), "{made}");
    // The row of its own was applied for the project as it says, by whoever added it, at the top.
    let library = rig.read(&format!("/api/instances/{}", given["notes"].as_str().unwrap())).await;
    assert_eq!(library["instance"]["blueprint"], "std/docs", "{library}");
    assert_eq!(library["instance"]["params"]["folder"], "notes", "{library}");
    assert!(library["instance"]["parent"].is_null(), "{library}");
    // What set it up is `std/project`, applied once, whose run gave out what came of each, and which is over.
    let set_up = of(&rig, "shop", "std/project").await;
    assert_eq!(set_up.len(), 1, "{set_up:?}");
    assert_ne!(set_up[0]["status"], "active", "{set_up:?}");
    let live = rig.read("/api/instances?project=shop&live=true").await;
    assert!(!live.to_string().contains("std/project"), "{live}");
    // Its rows are what ships unless said, each off but what was asked for: the fleet was not asked for.
    let rows = &set_up[0]["params"]["rows"];
    let on: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["on"] == true)
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(on, ["board", "notes"], "{rows}");

    // A row of its own called what a row is, and one that cannot be read, add nothing.
    let mut twice = notes();
    twice["name"] = json!("board");
    let (status, refused) = rig.post("/api/projects", json!({ "name": "Yard", "set_up": [twice] })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap().contains("`board` is a row already"), "{refused}");
    let (status, refused) = rig
        .post("/api/projects", json!({ "name": "Yard", "set_up": [{ "name": "x", "blueprint": "a b" }] }))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(rig.read("/api/projects").await.to_string().matches("\"yard\"").count() == 0);
}

#[tokio::test]
async fn a_setup_saved_under_a_name_is_chosen_by_it_and_the_factorys_default_is_what_nothing_else_said_sets_up()
 {
    let rig = Rig::new().await;
    // Kept under a name, as rows: the board and a library of notes.
    let research = json!({ "rows": [{ "name": "board", "blueprint": "std/board" }, notes()] });
    let saved = rig.call(Method::PUT, "/api/setups/research", Some(&rig.token), Some(research)).await;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let said = rig.event("project.setup.saved", |event| event.data["name"] == "research").await;
    assert_eq!(said.data["rows"], 2);
    let listed = rig.read("/api/setups").await;
    assert_eq!(listed["setups"][0]["name"], "research", "{listed}");
    assert_eq!(listed["usual"], listed["ships"], "nothing saved as the default: {listed}");

    // Chosen by its name, the project starts with its rows that are on.
    let made = rig.send("/api/projects", json!({ "name": "Lab", "set_up": { "setup": "research" } })).await;
    let given = &made["set_up"];
    assert!(given["board"].is_string() && given["notes"].is_string() && given["fleet"].is_null(), "{made}");
    // And with what it names of them.
    let made = rig
        .send(
            "/api/projects",
            json!({ "name": "Desk", "set_up": { "setup": "research", "rows": ["notes"] } }),
        )
        .await;
    assert!(made["set_up"]["notes"].is_string() && made["set_up"]["board"].is_null(), "{made}");
    // One the factory does not keep is refused, saying what it keeps.
    let (status, refused) =
        rig.post("/api/projects", json!({ "name": "Yard", "set_up": { "setup": "garden" } })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert_eq!(refused["error"]["message"], "there is no setup called `garden`: there are `research`");

    // Saved as the factory's default by whoever manages it, it is what `true` sets a project up with.
    let default = json!({ "rows": [{ "name": "board", "blueprint": "std/board" }, { "name": "docs", "blueprint": "std/docs", "on": false }] });
    rig.call(Method::PUT, "/api/setups/default", Some(&rig.token), Some(default)).await;
    let made = rig.send("/api/projects", json!({ "name": "Till", "set_up": true })).await;
    let given = &made["set_up"];
    assert!(given["board"].is_string() && given["fleet"].is_null() && given["docs"].is_null(), "{made}");
    let starts = rig.read("/api/meta").await["project_starts"].clone();
    assert_eq!(starts.as_array().map(Vec::len), Some(2), "the client is handed the default: {starts}");
    // Taken away, the default is what ships again.
    rig.call(Method::DELETE, "/api/setups/default", Some(&rig.token), None).await;
    let listed = rig.read("/api/setups").await;
    assert_eq!(listed["usual"], listed["ships"], "{listed}");
    let starts = rig.read("/api/meta").await["project_starts"].clone();
    assert_eq!(starts.as_array().map(Vec::len), Some(9), "{starts}");
}

#[tokio::test]
async fn a_setup_is_saved_by_whoever_may_and_one_that_cannot_be_read_is_refused_saying_why() {
    let rig = Rig::new().await;
    let rows = json!({ "rows": [{ "name": "board", "blueprint": "std/board" }] });
    // A member may keep a setup with a name, and not the factory's default.
    let member = crate::held::key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let (status, saved) = rig.call(Method::PUT, "/api/setups/mine", Some(&member), Some(rows.clone())).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let (status, refused) =
        rig.call(Method::PUT, "/api/setups/default", Some(&member), Some(rows.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Somebody who may not apply a blueprint keeps none: it is a list of blueprints applied as whoever uses it.
    let reader = crate::held::key(&rig, &["projects"]).await;
    let (status, refused) =
        rig.call(Method::PUT, "/api/setups/theirs", Some(&reader), Some(rows.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, refused) = rig.call(Method::DELETE, "/api/setups/mine", Some(&reader), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    let cases = [
        ("Research", rows.clone(), "`Research` is no name for a setup"),
        ("empty", json!({ "rows": [] }), "a setup has at least one row"),
        (
            "after",
            json!({ "rows": [{ "name": "fleet", "blueprint": "fleet", "needs": ["board"] }, { "name": "board", "blueprint": "std/board" }] }),
            "`fleet` needs `board`, which is itself or comes after it",
        ),
        (
            "worded",
            json!({ "rows": [{ "name": "board", "blueprint": "std/board", "colour": "red" }] }),
            "`board` says `colour`",
        ),
        ("unsaid", json!({ "nothing": [] }), "`rows` is needed"),
    ];
    for (name, body, says) in cases {
        let (status, refused) =
            rig.call(Method::PUT, &format!("/api/setups/{name}"), Some(&rig.token), Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {refused}");
        assert!(refused["error"]["message"].as_str().unwrap().contains(says), "{name}: {refused}");
    }
    // Taken away, and a second time refused.
    rig.call(Method::DELETE, "/api/setups/mine", Some(&member), None).await;
    rig.event("project.setup.removed", |event| event.data["name"] == "mine").await;
    let (status, refused) = rig.call(Method::DELETE, "/api/setups/mine", Some(&member), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert_eq!(rig.read("/api/setups").await["setups"], json!([]));
}

#[tokio::test]
async fn a_setup_with_a_row_of_its_own_gives_somebody_who_may_not_apply_the_project_alone_and_says_why() {
    let rig = Rig::new().await;
    rig.call(Method::PUT, "/api/setups/research", Some(&rig.token), Some(json!({ "rows": [notes()] }))).await;
    // Somebody who may add a project and may not apply a blueprint is given the project alone, told why.
    let narrow = crate::held::key(&rig, &["projects"]).await;
    let asked = Some(json!({ "name": "Lab", "set_up": { "setup": "research" } }));
    let (status, made) = rig.call(Method::POST, "/api/projects", Some(&narrow), asked).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    let left = made["set_up"]["left"][0].as_str().unwrap_or_default();
    assert!(
        left.starts_with(
            "`lab` was not given notes: each is made by applying a blueprint, which takes `blueprints:run`"
        ) && left.ends_with(&format!("`{} apply std/docs --project lab`", brand::CLI)),
        "{made}"
    );
    assert_eq!(of(&rig, "lab", "std/project").await, Vec::<Value>::new());
    assert_eq!(of(&rig, "lab", "std/docs").await, Vec::<Value>::new());
    // And a row whose blueprint is not there is left out with why, the rest given.
    let asked =
        json!({ "name": "Desk", "set_up": ["board", { "name": "gone", "blueprint": "shop/nowhere" }] });
    let made = rig.send("/api/projects", asked).await;
    assert!(made["set_up"]["board"].is_string(), "{made}");
    let left = made["set_up"]["left"][0].as_str().unwrap_or_default();
    assert!(left.starts_with("`desk` was not given gone: "), "{made}");
    assert!(
        left.ends_with(&format!("By hand: `{} apply shop/nowhere --project desk`", brand::CLI)),
        "{made}"
    );
}
