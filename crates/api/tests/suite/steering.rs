//! What a fleet's rows hand on is held as the cells beside it are. A row
//! of `segments` names a small blueprint that a stage runs as whoever
//! applied the fleet, and hands it what the row says in its cells and in
//! `with`; a row of `stages` is handed whole to what runs the stage, with
//! what it leaves unsaid filled in from what the fleet was told. So what
//! `with` hands is asked of its writer as a command is, and as naming a
//! secret is (`scoped.rs` has the cells), while what a row says as
//! words of its own (how many look at once, by which two words, what
//! they come to together) takes no more than the row; a row says nothing
//! in `with` that its stage says of itself (the secret a remote is asked
//! with, among others), and a
//! column somebody added to one of the tables is never what the fleet
//! takes a row to say.

use std::path::Path;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, rig as fleet_rig, row, shop};
use crate::held::{key, said};
use crate::lanes::{move_card, said as reported};

/// Somebody who may change the fleet's rows, apply blueprints and read
/// what is said, and holds neither `shell` nor `secrets:use`.
const ARRANGES: &[&str] =
    &["projects", "channels", "board", "records", "blueprints", "agents", "fleet", "search"];

/// A command that writes a file where nobody asked for one.
fn writes(file: &Path) -> String {
    crate::standin::line(&["write", &file.to_string_lossy(), "ran"])
}

/// Writes with `token` and waits for the fleet to have taken it.
async fn as_them(rig: &Rig, token: &str, method: Method, path: &str, fields: Value) -> (StatusCode, Value) {
    let before = rig.mark();
    let (status, answer) = rig.call(method, path, Some(token), Some(json!({ "fields": fields }))).await;
    if status == StatusCode::OK {
        crate::taken::by_fleet(rig, before).await;
    }
    (status, answer)
}

/// A card made ready, and waited for until it is done or left for a person.
async fn played(rig: &Rig, title: &str) -> String {
    let id = add(rig, title, &[]).await;
    move_card(rig, &id, "ready", None).await;
    let card = id.clone();
    rig.event("card.left,board.item.moved", move |event| {
        let of = event.data["card"] == card.as_str() || event.data["id"] == card.as_str();
        of && (event.topic == "card.left" || event.data["to"] == "done")
    })
    .await;
    id
}

/// What each card's work was begun with as the secret its remote is asked with.
async fn keys_worked_with(rig: &Rig) -> Vec<Value> {
    let query = journal::EventQuery { topics: "instance.started".into(), limit: 1000, ..Default::default() };
    let mut keys = Vec::new();
    for event in rig.said(query).await.iter().filter(|event| event.data["blueprint"] == "std/work") {
        let id = event.source.instance.clone().unwrap();
        keys.push(rig.read(&format!("/api/instances/{id}")).await["instance"]["params"]["key"].clone());
    }
    keys
}

#[tokio::test]
async fn what_a_row_of_segments_hands_its_blueprint_beside_its_cells_takes_shell_as_its_command_does() {
    let (rig, at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let narrow = key(&rig, ARRANGES).await;
    let ran = at.join("ran-by-with");
    // The command said in `with` and not in its cell, for `std/run`.
    let row = json!({
        "title": "Tests", "stage": "review", "order": 5, "does": "std/run",
        "with": [format!("command={}", writes(&ran))],
    });
    let segments = "/api/collections/shop/segments/records";
    let (status, refused) = as_them(&rig, &narrow, Method::POST, segments, row).await;
    // Whatever was written, the card is worked, and nothing it would run
    // was run as whoever applied the fleet.
    played(&rig, "Add a cart").await;
    assert!(!ran.exists(), "a command said in `with` by a key without `shell` was run: {refused}");
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused).contains(
            "changing `with` of `segments` takes `shell` and `secrets:use`, which narrow does not hold"
        ),
        "{refused}"
    );
    // Nor is anything else said there by them: what it hands may be run,
    // or may be where the card's agent works.
    let fails =
        json!({ "title": "Tests", "stage": "review", "order": 5, "does": "std/run", "with": ["fails=left"] });
    let (status, refused) = as_them(&rig, &narrow, Method::POST, segments, fails).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
}

#[tokio::test]
async fn a_row_of_segments_says_nothing_in_with_that_its_stage_says_of_itself_such_as_the_secret_its_remote_is_asked_with()
 {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    // The owner keeps a secret, which the fleet was not told to ask a remote with.
    let value = Some(json!({ "value": "s3cr3t" }));
    let (status, answer) = rig.call(Method::PUT, "/api/secrets/deploy", Some(&rig.token), value).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // Somebody who may run commands and name a secret: what the stage
    // says of itself is still not theirs to say in its place.
    let shell: Vec<&str> = ARRANGES.iter().copied().chain(["shell", "secrets:use"]).collect();
    let runs = key(&rig, &shell).await;
    let row = json!({
        "title": "Keyed", "stage": "work", "order": 10, "does": "std/work", "with": ["key=deploy"],
    });
    let (status, added) =
        as_them(&rig, &runs, Method::POST, "/api/collections/shop/segments/records", row).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let id = played(&rig, "Add a cart").await;
    assert!(
        keys_worked_with(&rig).await.iter().all(|key| key != "deploy"),
        "a card was worked with a secret its row named: {:?}",
        keys_worked_with(&rig).await
    );
    // It is a row standing in for its stage, which is so whoever wrote it:
    // the card is left for a person, saying why.
    let line = "Keyed says `key` in `with`, which its stage says of itself: a row says what more its own \
                blueprint is handed, and not what its stage, or the fleet over it, says";
    let all = reported(&rig).await;
    assert!(all.contains(&id) && all.contains(line), "{all}");
}

#[tokio::test]
async fn what_a_row_of_segments_hands_its_blueprint_takes_secrets_use_beside_shell_since_a_param_may_name_a_secret()
 {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let value = Some(json!({ "value": "s3cr3t" }));
    let (status, answer) = rig.call(Method::PUT, "/api/secrets/deploy", Some(&rig.token), value).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // A segment of the project's own may call a param anything and hand
    // it on as the name of a secret: `token` here is one, and nothing but
    // whoever wrote the segment can say so.
    let row = json!({
        "title": "Deploy", "stage": "review", "order": 15, "does": "deploy", "with": ["token=deploy"],
    });
    let segments = "/api/collections/shop/segments/records";
    // Somebody who may run commands and is not given what the factory keeps secret.
    let shell: Vec<&str> = ARRANGES.iter().copied().chain(["shell"]).collect();
    let runs = key(&rig, &shell).await;
    let (status, refused) = as_them(&rig, &runs, Method::POST, segments, row.clone()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(
        said(&refused),
        "changing `with` of `segments` takes `shell` and `secrets:use`, and narrow does not hold `secrets:use`",
        "{refused}"
    );
    let rows = rig.read(segments).await;
    let rows = rows["records"].as_array().unwrap();
    assert!(rows.iter().all(|one| one["data"]["title"] != "Deploy"), "{rows:?}");
    // Nor is a row that says it changed to say it by them.
    let quiet = json!({ "title": "Deploy", "stage": "review", "order": 15, "does": "deploy" });
    let (status, added) = as_them(&rig, &runs, Method::POST, segments, quiet).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let path = format!("/api/records/{}", added["id"].as_str().unwrap());
    let (status, refused) =
        as_them(&rig, &runs, Method::PATCH, &path, json!({ "with": ["token=deploy"] })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Somebody who holds both writes it.
    let both: Vec<&str> = shell.iter().copied().chain(["secrets:use"]).collect();
    let names = key(&rig, &both).await;
    let (status, changed) =
        as_them(&rig, &names, Method::PATCH, &path, json!({ "with": ["token=deploy"] })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    assert_eq!(changed["data"]["with"], json!(["token=deploy"]), "{changed}");
}

#[tokio::test]
async fn a_column_somebody_added_to_a_fleets_table_is_never_what_the_fleet_takes_a_row_to_say() {
    let (rig, at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let value = Some(json!({ "value": "s3cr3t" }));
    let (status, answer) = rig.call(Method::PUT, "/api/secrets/deploy", Some(&rig.token), value).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    // Somebody who may add a column and change the fleet's rows, and
    // holds neither `shell` nor `secrets:use`.
    let holds: Vec<&str> = ARRANGES.iter().copied().chain(["columns"]).collect();
    let narrow = key(&rig, &holds).await;
    // Two columns of theirs, called what the fleet says of every stage
    // (`key`) and what a segment that runs something takes (`command`).
    for label in ["Key", "Command"] {
        let body = Some(json!({ "label": label, "kind": "text" }));
        let (status, added) =
            rig.call(Method::POST, "/api/collections/shop/stages/fields", Some(&narrow), body).await;
        assert_eq!(status, StatusCode::OK, "{added}");
        assert_eq!(added["field"], label.to_lowercase(), "{added}");
    }
    let ran = at.join("ran-by-a-column");
    for (title, fields) in
        [("work", json!({ "key": "deploy" })), ("review", json!({ "command": writes(&ran) }))]
    {
        let path = format!("/api/records/{}", row(&rig, "stages", title).await);
        let (status, changed) = as_them(&rig, &narrow, Method::PATCH, &path, fields).await;
        assert_eq!(status, StatusCode::OK, "{changed}");
    }
    // A thing done before the review that says no command of its own.
    let check = json!({ "title": "Tests", "stage": "review", "order": 5, "does": "std/run" });
    let (status, added) =
        as_them(&rig, &narrow, Method::POST, "/api/collections/shop/segments/records", check).await;
    assert_eq!(status, StatusCode::OK, "{added}");

    played(&rig, "Add a cart").await;
    assert!(!ran.exists(), "a command said in a column somebody added was run");
    let keys = keys_worked_with(&rig).await;
    assert!(!keys.is_empty() && keys.iter().all(|key| key == ""), "{keys:?}");
    // What they wrote there is theirs and is kept: the fleet reads it as nothing.
    let work = rig.read(&format!("/api/records/{}", row(&rig, "stages", "work").await)).await;
    assert_eq!(work["data"]["key"], "deploy");
}

#[tokio::test]
async fn how_many_look_at_once_and_what_they_come_to_are_words_of_a_row_of_segments_and_take_no_shell() {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({ "reviewers": 2, "seats": 1 })).await;
    let narrow = key(&rig, ARRANGES).await;
    // A council of two in the stage that works, said by somebody who may
    // not run commands or name a secret: what it is handed is a count and
    // words, which reach neither.
    let row = json!({
        "title": "Second look", "stage": "work", "order": 15, "does": "std/turn", "role": "reviewer",
        "seats": 2, "needed": 2, "ends": ["passed=on failed=back"], "yes": "approve", "no": "send_back",
    });
    let segments = "/api/collections/shop/segments/records";
    let (status, added) = as_them(&rig, &narrow, Method::POST, segments, row).await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let id = played(&rig, "Add a cart").await;
    assert_eq!(crate::fleets::card(&rig, &id).await["status"], "done");
    for reviewer in ["shop-reviewer-1", "shop-reviewer-2"] {
        let turns = rig.read(&format!("/api/agents/{reviewer}/turns")).await;
        let looked = turns["turns"].as_array().unwrap().iter().any(|turn| {
            turn["thread"].as_str().is_some_and(|thread| thread.starts_with(&format!("turn:{id}:")))
        });
        assert!(looked, "{reviewer} looked at the work: {turns}");
    }
}

#[tokio::test]
async fn a_turn_that_is_one_of_several_pieces_of_work_on_a_card_holds_a_conversation_of_its_own_after_the_card()
 {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({ "reviewers": 1 })).await;
    // Laid in by somebody who may run commands, since what a row hands a
    // segment by name (`with`) may be anything its blueprint takes.
    let row = json!({
        "title": "A look of its own", "stage": "work", "order": 15, "does": "std/turn", "role": "reviewer",
        "with": ["way=second-look"],
    });
    let before = rig.mark();
    rig.send("/api/collections/shop/segments/records", json!({ "fields": row })).await;
    rig.event("fleet.arranged", move |event| event.seq > before).await;
    let id = played(&rig, "Add a cart").await;
    let turns = rig.read("/api/agents/shop-reviewer-1/turns").await;
    let threads: Vec<&str> =
        turns["turns"].as_array().unwrap().iter().filter_map(|turn| turn["thread"].as_str()).collect();
    assert!(threads.contains(&format!("turn:{id}:second-look").as_str()), "{threads:?}");
}

#[tokio::test]
async fn a_stage_that_names_its_own_review_runs_it_only_as_it_was_vouched_for_so_a_key_that_holds_less_names_no_text_of_its_own()
 {
    let (rig, at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    let narrow = key(&rig, ARRANGES).await;
    // A review of their own, saved by them through the daemon: it runs a
    // command, which they may not.
    let ran = at.join("ran-by-a-review");
    let text = format!(
        "blueprint: sneaky\npermissions: [shell]\nparams:\n  project: {{ type: string }}\n  item: {{ type: \
         string }}\n  stage: {{ type: string, default: review }}\n  role: {{ type: string, default: reviewer \
         }}\n  picks: {{ type: string, default: first }}\n  seats: {{ type: int, default: 1 }}\n  needed: {{ \
         type: int, default: 0 }}\n  pool: {{ type: string, default: \"\" }}\n  arranged: {{ type: string, \
         default: \"\" }}\nsteps:\n  ran: {{ do: shell, with: {{ cmd: {} }} }}\n",
        crate::standin::cmd(&["write", &ran.to_string_lossy(), "ran"]),
    );
    let saved =
        Some(json!({ "path": format!("@blueprints/{}", brand::blueprint_file("sneaky")), "content": text }));
    let (status, answer) = rig.call(Method::PUT, "/api/projects/shop/file", Some(&narrow), saved).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    // Naming a blueprint for a stage takes what any of its cells takes:
    // they may, as they may name one in `does`.
    let path = format!("/api/records/{}", row(&rig, "stages", "review").await);
    let (status, changed) = as_them(&rig, &narrow, Method::PATCH, &path, json!({ "review": "sneaky" })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");

    // But what a stage starts is run as whoever applied the fleet only as
    // they vouched for it, and nobody who holds that much did: it is not
    // run, and the card's review could not be begun.
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let failed = rig
        .event("run.failed", |event| {
            event.data["blueprint"] == "std/reviewing" && event.data["flow"] == "on.due"
        })
        .await;
    assert!(failed.data["reason"].as_str().unwrap_or_default().contains("sneaky"), "{}", failed.data);
    assert!(!ran.exists(), "a review saved by a key without `shell` was run as whoever applied the fleet");
    let card = rig.read("/api/board/shop").await;
    let card =
        card["items"].as_array().unwrap().iter().find(|card| card["id"] == id.as_str()).cloned().unwrap();
    assert_eq!(card["status"], "review", "and it waits there for a person");
}
