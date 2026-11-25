//! Which text an instance runs, from the outside: a project's blueprints
//! are files, and an instance started from one goes on with the text it
//! was started with when the file is saved again. It says so wherever it
//! is shown, takes the file's text where it is when what started it is
//! applied again and only what it does later has changed, and can be
//! applied again by itself when more than that has. And what an update
//! that fails leaves: the instance on the text it had, with what the
//! update started stopped.

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
      go: { spawn: worker, as: the-worker, with: { room: \"${ event.data.room }\" } }
";

fn worker(version: u32, claims: &str, says: &str) -> String {
    format!(
        "blueprint: worker\nversion: {version}\nparams:\n  room: {{ type: string }}\nsteps:\n  claim: {{ do: emit, with: {{ \
         topic: {claims}, room: \"${{params.room}}\" }} }}\non:\n  poke:\n    event: shop.poke\n    steps:\n      say: {{ \
         do: emit, with: {{ topic: {says} }} }}\n"
    )
}

async fn shop(rig: &Rig, files: &[(&str, &str)]) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    for (name, content) in files {
        let path = format!("@blueprints/{}", brand::blueprint_file(name));
        let body = json!({ "path": path, "content": content });
        let (status, answer) =
            rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
    }
}

/// Somebody saves the file of the blueprint called `name`, where it is.
fn save(rig: &Rig, name: &str, text: &str) {
    let file = rig.placed("shop").join(brand::blueprint_file(name));
    std::fs::write(file, text).unwrap();
}

async fn apply_watcher(rig: &Rig) -> Value {
    let path = format!("@blueprints/{}", brand::blueprint_file("watcher"));
    rig.send("/api/apply", json!({ "path": path, "project": "shop", "wait": true })).await
}

async fn emit(rig: &Rig, topic: &str, data: Value) {
    rig.node().app.journal.emit(topic, Source::default(), data).await.unwrap();
}

/// The worker as `/api/instances` shows it.
async fn the_worker(rig: &Rig) -> Value {
    let listed = rig.read("/api/instances?project=shop&live=true").await;
    listed["instances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|one| one["key"] == "the-worker")
        .unwrap_or_else(|| panic!("no worker in {listed}"))
        .clone()
}

/// The watcher applied and a worker started by its handler and settled:
/// `(watcher, worker)`.
async fn working(rig: &Rig) -> (String, String) {
    shop(rig, &[("watcher", WATCHER), ("worker", &worker(1, "shop.claimed", "shop.old"))]).await;
    let applied = apply_watcher(rig).await;
    emit(rig, "shop.wanted", json!({ "room": "general" })).await;
    rig.count("shop.claimed", 1).await;
    let id = the_worker(rig).await["id"].as_str().unwrap().to_string();
    rig.event("instance.active", |event| event.source.instance.as_deref() == Some(id.as_str())).await;
    (applied["instance"].as_str().unwrap().to_string(), id)
}

#[tokio::test]
async fn an_instance_is_shown_with_the_text_it_runs_and_takes_a_saved_file_when_what_started_it_is_applied_again()
 {
    let rig = Rig::new().await;
    let (watcher, id) = working(&rig).await;
    let first = worker(1, "shop.claimed", "shop.old");
    let is = the_worker(&rig).await;
    assert_eq!(
        (&is["revision"], &is["version"], &is["behind"]),
        (&json!(blueprint::revision(&first)), &json!("1"), &Value::Null),
        "{is}"
    );

    // Somebody saves the file with another handler. Nothing is told; it
    // is said when it is asked, everywhere the instance is shown.
    let second = worker(2, "shop.claimed", "shop.new");
    save(&rig, "worker", &second);
    let behind = json!({
        "revision": blueprint::revision(&second),
        "version": "2",
        "takes": "in_place",
        "why": "The text of `worker` now there differs from the one this runs only in what it does later: \
                its handlers, what it does when it is stopped, what it may use. It takes that text where \
                it is, with none of its steps run again, when what started it, or it itself, is applied \
                again.",
        // What moved between the two, a sentence for each thing.
        "changed": ["the handler `poke` is changed"],
    });
    assert_eq!(the_worker(&rig).await["behind"], behind);
    assert_eq!(rig.read(&format!("/api/instances/{id}")).await["instance"]["behind"], behind);
    let parent = rig.read(&format!("/api/instances/{watcher}")).await;
    assert_eq!(parent["instance"]["behind"], Value::Null, "the watcher's own file was not touched");
    assert_eq!(parent["children"][0]["behind"], behind);
    let fleet = rig.read("/api/fleet").await;
    let drawn = fleet["instances"].as_array().unwrap().iter().find(|one| one["id"] == id.as_str()).unwrap();
    assert_eq!((&drawn["behind"], &drawn["version"]), (&behind, &json!("1")));

    // Applying the watcher again, as it is, is what brings the worker to it.
    let again = apply_watcher(&rig).await;
    assert_eq!(again["changed"], false, "{again}");
    let is = the_worker(&rig).await;
    assert_eq!(
        (&is["revision"], &is["version"], &is["behind"], &is["status"]),
        (&json!(blueprint::revision(&second)), &json!("2"), &Value::Null, &json!("active")),
        "{is}"
    );
    emit(&rig, "shop.poke", json!({})).await;
    rig.count("shop.new", 1).await;
    let journal = &rig.node().app.journal;
    let count = async |topic: &str| {
        let query = journal::EventQuery { topics: topic.to_string(), limit: 100, ..Default::default() };
        journal.read_every(query).await.unwrap().len()
    };
    assert_eq!(
        (count("shop.claimed").await, count("shop.old").await),
        (1, 0),
        "its steps were not run again"
    );
    let versions = rig.read(&format!("/api/instances/{id}/versions")).await;
    assert_eq!(versions["versions"].as_array().unwrap().len(), 2, "what it took is its next version");
}

#[tokio::test]
async fn an_instance_whose_file_now_begins_otherwise_stays_behind_until_it_is_applied_again_by_itself() {
    let rig = Rig::new().await;
    let (_, id) = working(&rig).await;
    let first = worker(1, "shop.claimed", "shop.old");

    // Its steps are changed in the file.
    let second = worker(2, "shop.claimed_again", "shop.new");
    save(&rig, "worker", &second);
    apply_watcher(&rig).await;
    let is = the_worker(&rig).await;
    assert_eq!(is["revision"], blueprint::revision(&first), "applying what started it leaves it");
    assert_eq!(is["behind"]["takes"], "again");
    assert_eq!(
        is["behind"]["why"],
        "The text of `worker` now there begins otherwise than the one this runs: its steps, its params, or \
         something else it says at the top. It runs that text only when it is itself applied again, which \
         runs its steps again."
    );

    // Applied again by itself, it is given what it was started with.
    let applied = rig.send(&format!("/api/instances/{id}/apply"), json!({ "wait": true })).await;
    assert_eq!(
        (&applied["instance"], &applied["created"], &applied["changed"], &applied["finished"]["status"]),
        (&json!(id), &json!(false), &json!(true), &json!("ok")),
        "{applied}"
    );
    let claimed = rig.event("shop.claimed_again", |_| true).await;
    assert_eq!(claimed.data["room"], "general", "with the params it had");
    let is = the_worker(&rig).await;
    assert_eq!((&is["revision"], &is["behind"]), (&json!(blueprint::revision(&second)), &Value::Null));
    let all = rig.read("/api/instances?project=shop&live=true").await;
    assert_eq!(all["instances"].as_array().unwrap().len(), 2, "no second worker was started");

    // A file that does not check here: it says so, goes on, and is not applied over.
    let newer = "do: log, with: { message: poked, loudly: true }";
    save(&rig, "worker", &second.replace("do: emit, with: { topic: shop.new }", newer));
    let is = the_worker(&rig).await;
    assert_eq!(is["behind"]["takes"], "unfit");
    let why = is["behind"]["why"].as_str().unwrap();
    assert!(
        why.starts_with(
            "The text of `worker` now there cannot be applied here, so this goes on with the one it has."
        ) && why.contains("`log` has no argument `loudly`"),
        "{why}"
    );
    let (status, refused) = rig.post(&format!("/api/instances/{id}/apply"), json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap().contains("`log` has no argument `loudly`"));
    assert_eq!(the_worker(&rig).await["revision"], blueprint::revision(&second), "it is as it was");

    // It needs what applying needs, and a credential to begin with.
    let (status, _) = rig.call(Method::POST, &format!("/api/instances/{id}/apply"), None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, missing) = rig.post("/api/instances/in_nothing/apply", json!({})).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");
}

#[tokio::test]
async fn applying_twice_what_is_still_at_work_does_its_steps_once_and_gives_the_run_that_is_going() {
    let rig = Rig::new().await;
    // It says it has begun, then waits to be told to go on.
    let source = "\
blueprint: patient
steps:
  begin: { do: emit, with: { topic: shop.begun } }
  hold: { do: wait, with: { event: shop.go } }
  end: { do: emit, with: { topic: shop.ended } }
";
    let first = rig.send("/api/apply", json!({ "source": source })).await;
    rig.event("step.started", |event| event.source.step.as_deref() == Some("hold")).await;

    // Asked for again while it waits: nothing is started over, and the run
    // that is going is the one given, for whoever asked to wait on.
    let again = rig.send("/api/apply", json!({ "source": source })).await;
    assert_eq!(again["instance"], first["instance"]);
    assert_eq!((&again["changed"], &again["created"]), (&json!(false), &json!(false)), "{again}");
    assert_eq!(again["run"], first["run"], "{again}");

    emit(&rig, "shop.go", json!({})).await;
    rig.count("shop.ended", 1).await;
    assert_eq!(rig.count_of("shop.begun").await, 1, "its steps were begun once");
    assert_eq!(rig.count_of("step.cancelled").await, 0, "and none of them was cancelled");
    assert_eq!(rig.count_of("instance.updated").await, 0, "nor was it said to have been updated");
}

#[tokio::test]
async fn a_failed_update_stops_the_service_it_started_and_leaves_the_one_that_was_running() {
    let rig = Rig::new().await;
    // Each version is a service of its own name, as a rollout has them.
    let source = &"\
blueprint: host
permissions: [fleet]
params:
  v: { type: string, default: a }
  broken: { type: bool, default: false }
steps:
  serve: { do: service.start, with: { name: \"host-${ params.v }\", cmd: IDLES, restart: never } }
  boom: { when: ${ params.broken }, do: fail, with: { message: broke after starting the new one } }
"
    .replace("IDLES", &crate::standin::cmd(&["idle"]));
    let first = rig.send("/api/apply", json!({ "source": source, "wait": true })).await;
    assert_eq!(first["finished"]["status"], "ok", "{first}");
    let id = first["instance"].as_str().unwrap();
    let names = async || -> Vec<String> {
        let fleet = rig.read("/api/fleet").await;
        let mut names: Vec<String> = fleet["services"]
            .as_array()
            .unwrap()
            .iter()
            .map(|service| service["name"].as_str().unwrap_or_default().to_string())
            .collect();
        names.sort();
        names
    };
    assert_eq!(names().await, ["host-a"]);

    let body = json!({ "source": source, "params": { "v": "b", "broken": true }, "wait": true });
    let second = rig.send("/api/apply", body).await;
    assert_eq!(second["finished"]["status"], "failed", "{second}");
    rig.event("instance.update_failed", |event| event.source.instance.as_deref() == Some(id)).await;

    let instance = rig.read(&format!("/api/instances/{id}")).await;
    let instance = if instance["instance"].is_object() { &instance["instance"] } else { &instance };
    assert_eq!(instance["holds"], json!(["service:host-a"]), "{instance}");
    assert_eq!(instance["params"]["v"], "a", "{instance}");
    assert_eq!(names().await, ["host-a"], "the one the failed update started is stopped and gone");

    rig.send(&format!("/api/instances/{id}/stop"), json!({})).await;
    assert_eq!(names().await, Vec::<String>::new());
}

#[tokio::test]
async fn an_apply_says_which_instance_it_came_to_by_its_blueprint_and_its_key_whether_or_not_anything_changed()
 {
    let rig = Rig::new().await;
    shop(&rig, &[("watcher", WATCHER)]).await;
    let first = apply_watcher(&rig).await;
    assert_eq!(
        (&first["created"], &first["blueprint"], &first["key"]),
        (&json!(true), &json!("watcher"), &json!("watcher")),
        "{first}"
    );
    // Applied again as it is: nothing changed, and it is still said which.
    let again = apply_watcher(&rig).await;
    assert_eq!((&again["created"], &again["changed"]), (&json!(false), &json!(false)), "{again}");
    assert_eq!(
        (&again["instance"], &again["blueprint"], &again["key"]),
        (&first["instance"], &json!("watcher"), &json!("watcher")),
        "{again}"
    );
    // And under a key of its own, by that key.
    let path = format!("@blueprints/{}", brand::blueprint_file("watcher"));
    let keyed = rig.send("/api/apply", json!({ "path": path, "project": "shop", "key": "night" })).await;
    assert_eq!((&keyed["blueprint"], &keyed["key"]), (&json!("watcher"), &json!("night")), "{keyed}");
}
