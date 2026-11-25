//! A command a row of `segments` runs, kept as a row of a table each time
//! it is run (`record_to`, handed to `std/run`): when it began, the card,
//! the commit it ran on, the command, whether it passed and with what
//! code, how long it took and the last of what it printed.
//!
//! The table is made the first time a run is kept, as a table of the
//! project that is listed under it; one the project has of that name
//! already keeps its own fields, and only those are filled. A run is kept
//! whether it passed or failed, and the card goes on or back as it would
//! with nothing kept. A table that guards what is written in it is never
//! written by a blueprint for whoever named it, and naming one at all
//! takes what saying the command does.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, add_row, card, rig, shop};
use crate::held::{key, said as refused};
use crate::lanes::{move_card, moved};

/// A row of `segments` that runs a command before a card is reviewed and
/// keeps each run in `test-runs`.
fn recorded(run: &str) -> Value {
    json!({
        "title": "Tests", "stage": "review", "order": 5, "does": "std/run", "command": run,
        "record_to": "test-runs",
    })
}

/// The rows of one of the project's tables, oldest first: what each holds.
async fn rows(rig: &Rig, table: &str) -> Vec<Value> {
    let (status, answer) = rig
        .call(Method::GET, &format!("/api/collections/shop/{table}/records"), Some(&rig.token), None)
        .await;
    if status == StatusCode::NOT_FOUND {
        return Vec::new();
    }
    assert_eq!(status, StatusCode::OK, "{answer}");
    answer["records"].as_array().unwrap().iter().map(|row| row["data"].clone()).collect()
}

/// The project's table called this, as the daemon lists its tables.
async fn table(rig: &Rig, name: &str) -> Option<Value> {
    let listed = rig.read("/api/collections?project=shop").await;
    listed["collections"].as_array().unwrap().iter().find(|table| table["name"] == name).cloned()
}

#[tokio::test]
async fn each_run_of_a_command_a_row_says_to_keep_is_a_row_of_a_table_made_the_first_time_not_written_by_hand()
 {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    // The worker adds a line each time it is given the card: the first
    // time there is one, which this does not pass.
    let run = crate::standin::line(&["fewer", "CHANGES.md", "2", "only one change"]);
    add_row(&rig, "segments", recorded(&run)).await;
    assert!(table(&rig, "test-runs").await.is_none(), "nothing is made before anything is run");

    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    let back = moved(&rig, &id, "doing", reviewed).await;
    let again = moved(&rig, &id, "review", back).await;
    moved(&rig, &id, "done", again).await;

    // The card went back and on as it would have with nothing kept.
    assert_eq!(card(&rig, &id).await["more"]["rounds"], 1);
    let made = table(&rig, "test-runs").await.expect("the table was made");
    assert!(made["beside"].is_null(), "it is the project's own, listed under it: {made}");
    assert_eq!(made["by_hand"], json!(false), "its rows are the command's, read by people: {made}");
    let kept = rows(&rig, "test-runs").await;
    assert_eq!(kept.len(), 2, "{kept:?}");
    let (failed, passed) = (&kept[0], &kept[1]);
    for one in &kept {
        assert_eq!(one["card"], id.as_str(), "{one}");
        assert_eq!(one["command"], run.as_str(), "{one}");
        let commit = one["commit"].as_str().unwrap_or_default();
        assert!(commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit()), "{one}");
        assert!(one["seconds"].as_f64().is_some_and(|seconds| seconds >= 0.0), "{one}");
        // A run id: when it began and the card, with the day as a date and the time beside it.
        let title = one["title"].as_str().unwrap_or_default();
        let day = one["day"].as_str().unwrap_or_default();
        assert!(title.ends_with(&format!("-{id}")) && title.starts_with(&day.replace('-', "")), "{one}");
        assert!(day.len() == 10 && one["time"].as_str().is_some_and(|time| time.ends_with(" UTC")), "{one}");
        let field =
            |name: &str| made["fields"].as_array().unwrap().iter().find(|f| f["name"] == name).cloned();
        assert_eq!(field("day").map(|f| f["kind"].clone()), Some(json!("date")), "{made}");
    }
    assert_eq!(failed["result"], "failed", "{failed}");
    assert!(failed["code"].as_i64().is_some_and(|code| code != 0), "{failed}");
    assert!(failed["printed"].as_str().unwrap().contains("only one change"), "{failed}");
    assert_eq!(passed["result"], "passed", "{passed}");
    assert_eq!(passed["code"], 0, "{passed}");
    // The second ran on the work done since: another commit.
    assert_ne!(failed["commit"], passed["commit"]);
}

#[tokio::test]
async fn a_table_the_project_has_of_that_name_keeps_its_own_fields_and_only_those_are_filled() {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    let ours = r#"
blueprint: ours
steps:
  runs:
    do: collection.ensure
    with:
      name: test-runs
      fields:
        title: { type: text }
        result: { type: choice, of: [passed, failed] }
        notes: { type: text }
"#;
    let applied = rig.send("/api/apply", json!({ "source": ours, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    add_row(&rig, "segments", recorded("exit 0")).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    moved(&rig, &id, "done", reviewed).await;

    let made = table(&rig, "test-runs").await.unwrap();
    let fields: Vec<&str> =
        made["fields"].as_array().unwrap().iter().map(|f| f["name"].as_str().unwrap()).collect();
    assert_eq!(fields, ["notes", "result", "title"], "{made}");
    assert!(made["by_hand"].is_null(), "a table that is there keeps what it says of itself: {made}");
    let kept = rows(&rig, "test-runs").await;
    assert_eq!(kept.len(), 1, "{kept:?}");
    let filled: Vec<&String> =
        kept[0].as_object().unwrap().keys().filter(|name| !kept[0][*name].is_null()).collect();
    assert_eq!(filled, ["result", "title"], "{}", kept[0]);
    assert_eq!(kept[0]["result"], "passed");
}

#[tokio::test]
async fn a_table_that_guards_what_is_written_in_it_is_not_written_for_whoever_named_it_and_the_card_goes_on()
{
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    let agents = rows(&rig, "agents").await.len();
    let mut row = recorded("exit 0");
    row["record_to"] = json!("agents");
    add_row(&rig, "segments", row).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let reviewed = moved(&rig, &id, "review", 0).await;
    moved(&rig, &id, "done", reviewed).await;
    assert_eq!(rows(&rig, "agents").await.len(), agents, "nothing was written among the fleet's agents");
    // The step that would have kept it says why.
    let query =
        journal::EventQuery { topics: "instance.started".into(), limit: 10_000, ..Default::default() };
    let started = rig.said(query).await;
    let run = started.iter().find(|event| event.data["blueprint"] == "std/run").expect("std/run ran");
    let instance = run.data["instance"].as_str().or(run.source.instance.as_deref()).unwrap().to_string();
    let runs = rig.read(&format!("/api/runs?instance={instance}")).await;
    let id = runs["runs"][0]["id"].as_str().unwrap().to_string();
    let steps = rig.read(&format!("/api/runs/{id}")).await;
    let kept = steps["steps"].as_array().unwrap().iter().find(|step| step["step"] == "kept").cloned();
    let why = kept.as_ref().and_then(|step| step["error"].as_str()).unwrap_or_default();
    assert!(why.contains("`agents` says what writing or reading it takes"), "{kept:?}");
}

#[tokio::test]
async fn where_a_commands_runs_are_kept_is_written_into_a_row_only_by_somebody_who_holds_shell() {
    let (rig, _) = rig().await;
    shop(&rig, json!({})).await;
    // Somebody who may change the fleet's rows and holds no `shell`.
    let holds = ["projects", "channels", "board", "records", "blueprints", "agents", "fleet", "search"];
    let narrow = key(&rig, &holds).await;
    let row =
        json!({ "title": "Ours", "stage": "review", "order": 5, "does": "check", "record_to": "test-runs" });
    let body = json!({ "fields": row });
    let (status, why) =
        rig.call(Method::POST, "/api/collections/shop/segments/records", Some(&narrow), Some(body)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{why}");
    assert!(refused(&why).contains("`shell`"), "{why}");
    // Nor set on a row that is there.
    let mut ours = row.clone();
    ours.as_object_mut().unwrap().remove("record_to");
    let (status, added) = rig
        .call(
            Method::POST,
            "/api/collections/shop/segments/records",
            Some(&narrow),
            Some(json!({ "fields": ours })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{added}");
    let id = added["id"].as_str().unwrap();
    let set = json!({ "fields": { "record_to": "test-runs" } });
    let (status, why) =
        rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(&narrow), Some(set)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{why}");
    assert!(refused(&why).contains("`shell`"), "{why}");
}
