//! Sign-ins over HTTP: the owner's to manage, and a value that goes in
//! once and is then nowhere but the file it is kept in and the environment
//! of the program it is handed to.
//!
//! Unix's alone: the stand-in for an agent's program is a file of a Unix
//! shell, made a program by its mode.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::snapshots::{files_under, holds, taken};

/// A package for a made-up agent whose program can be handed a key, and
/// writes down in its own folder what it was handed. It says it has
/// reached a limit when the key it holds is the one in `limited`.
fn package(dir: &Path) -> String {
    let here = dir.join("standin");
    std::fs::create_dir_all(&here).unwrap();
    let program = here.join("program");
    let script = r#"#!/bin/sh
here="HERE"
cat > /dev/null
printf '%s\n' "${STANDIN_KEY-}" >> "$here/handed"
if [ "${STANDIN_KEY-}" = "$(cat "$here/limited" 2>/dev/null)" ]; then
  printf '{"kind":"end","bad":true,"text":"limit reached"}\n'
  exit 0
fi
printf '{"kind":"end","bad":false,"text":"Done by the stand-in."}\n'
"#;
    std::fs::write(&program, script.replace("HERE", &here.to_string_lossy())).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let manifest = r#"
extension: standin
title: Stand-in
description: Not a real agent.
agent:
  program: PROGRAM
  input: ${ turn.prompt }
  output:
    - when: ${ event.kind == "end" }
      reply: ${ event.text }
      failed: ${ event.bad == true }
      error: ${ event.text }
  signin:
    key: STANDIN_KEY
    limit:
      when: ${ matches(error, "limit reached") }
      for: 1h
"#;
    manifest.replace("PROGRAM", &program.to_string_lossy())
}

fn handed(rig: &Rig) -> Vec<String> {
    let written = std::fs::read_to_string(rig.dir.join("standin/handed")).unwrap_or_default();
    written.lines().map(str::to_string).collect()
}

/// A blueprint that has an agent do a piece of work.
fn blueprint(slot: &str) -> String {
    format!(
        "blueprint: work\npermissions: [agents]\nsteps:\n  it:\n    do: agent.task\n    with: {{ slot: {slot}, prompt: build the cart }}\n"
    )
}

/// Has an agent do a piece of work, and waits for it.
async fn work(rig: &Rig, slot: &str) -> Value {
    let body = json!({ "source": blueprint(slot), "wait": true, "force": true });
    rig.send("/api/apply", body).await["finished"].clone()
}

#[tokio::test]
async fn sign_ins_are_the_owners_to_manage_over_http_and_a_value_is_never_given_out() {
    let rig = Rig::new().await;
    rig.send("/api/extensions", json!({ "source": package(&rig.dir) })).await;
    let key = "sk-secret-value-of-the-first-key";
    let added = rig
        .send("/api/signins", json!({ "name": "paid", "for": "standin", "kind": "key", "value": key }))
        .await;
    assert_eq!(
        (added["name"].as_str(), added["for"].as_str(), added["kind"].as_str(), added["at_once"].as_u64()),
        (Some("paid"), Some("standin"), Some("key"), Some(1)),
        "one at a time unless said"
    );
    let listed = rig.read("/api/signins").await;
    assert_eq!(listed["signins"].as_array().unwrap().len(), 1);
    assert!(!format!("{added}{listed}").contains(key), "what is given back never has the value");

    // An agent has neither leave: not to manage them, and not to make
    // another agent draw one.
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/channels/general/agents", json!({ "name": "helper", "scopes": ["agents", "channels"] }))
        .await;
    let agent = std::fs::read_to_string(rig.dir.join("data/slots/helper/token")).unwrap();
    let agent = Some(agent.trim());
    let body = json!({ "name": "other", "for": "standin", "kind": "key", "value": "another" });
    for (method, path, body) in [
        (Method::GET, "/api/signins", None),
        (Method::POST, "/api/signins", Some(body)),
        (Method::DELETE, "/api/signins/paid", None),
        (Method::POST, "/api/signins/paid/restore", Some(json!({}))),
    ] {
        let (status, error) = rig.call(method.clone(), path, agent, body.clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {error}");
        assert!(error["error"]["message"].as_str().unwrap().contains("needs `admin`"), "{error}");
        let (status, _) = rig.call(method, path, None, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}: nobody at all");
    }
    let draws = json!({ "name": "minion", "adapter": "standin", "signins": ["key"] });
    let (status, error) = rig.call(Method::POST, "/api/agents", agent, Some(draws.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{error}");
    assert!(error["error"]["message"].as_str().unwrap().contains("needs `secrets:use`"), "{error}");

    // The owner tells one to, and it is handed the key for its work.
    let minion = rig.send("/api/agents", draws).await;
    assert_eq!(minion["signins"], json!(["key"]));
    assert_eq!(work(&rig, "minion").await["status"], "ok");
    assert_eq!(handed(&rig), [key]);
    let listed = rig.read("/api/signins").await;
    assert!(listed["signins"][0]["last_used"].is_i64() && listed["signins"][0]["held_by"] == json!([]));

    // At a limit it is set aside, and with no other to draw the work
    // waits for it. Made good again by hand, the work goes on.
    std::fs::write(rig.dir.join("standin/limited"), key).unwrap();
    rig.send("/api/apply", json!({ "source": blueprint("minion"), "force": true })).await;
    let aside = rig.event("agent.signin.set_aside", |_| true).await;
    assert_eq!(aside.data["signin"], "paid");
    rig.event("agent.signin.waiting", |_| true).await;
    assert!(rig.read("/api/signins").await["signins"][0]["aside_until"].is_i64());
    std::fs::remove_file(rig.dir.join("standin/limited")).unwrap();
    rig.send("/api/signins/paid/restore", json!({})).await;
    let finished = rig.event("agent.turn.finished", |event| event.seq > aside.seq).await;
    assert_eq!(finished.data["ok"], true, "{:?}", finished.data);
    assert_eq!(handed(&rig), [key, key, key], "it began again on the same, good again");
    assert!(rig.read("/api/signins").await["signins"][0]["aside_until"].is_null());
    let (status, _) = rig.call(Method::DELETE, "/api/signins/paid", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(rig.read("/api/signins").await["signins"].as_array().unwrap().is_empty());
    assert!(!rig.dir.join("data/signins/paid").exists(), "its value goes with it");
    let (status, error) = rig.call(Method::DELETE, "/api/signins/paid", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{error}");
}

#[tokio::test]
async fn the_value_of_a_sign_in_is_in_no_event_measurement_log_row_or_copy_of_the_factory() {
    let mut rig = Rig::new().await;
    rig.send("/api/extensions", json!({ "source": package(&rig.dir) })).await;
    let (first, second) = ("sk-secret-value-of-the-first-key", "sk-secret-value-of-the-second-key");
    for (name, value) in [("first", first), ("second", second)] {
        rig.send("/api/signins", json!({ "name": name, "for": "standin", "kind": "key", "value": value }))
            .await;
    }
    rig.send("/api/agents", json!({ "name": "minion", "adapter": "standin", "signins": ["key"] })).await;
    // Everything that can happen to one: handed over, a limit, set aside,
    // the work begun again on the next, given back.
    std::fs::write(rig.dir.join("standin/limited"), first).unwrap();
    assert_eq!(work(&rig, "minion").await["status"], "ok");
    assert_eq!(handed(&rig), [first, second], "the program was handed each");
    let aside = rig.event("agent.signin.set_aside", |_| true).await;
    assert_eq!(aside.data["signin"], "first");

    // Where it is kept: a file of its own that only its owner can read,
    // sealed, so that the file is not the value.
    let kept = rig.dir.join("data/signins");
    let on_disk = std::fs::read(kept.join("first")).unwrap();
    assert!(on_disk.starts_with(b"sealed 1\n"));
    assert!(!String::from_utf8_lossy(&on_disk).contains(first));
    let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!((mode(&kept), mode(&kept.join("first"))), (0o700, 0o600));

    // A copy of the factory has neither the values nor the rows.
    let to = rig.dir.join("copy");
    let manifest = taken(&rig, &to).await;
    let never = manifest["left_out"]["never"].to_string();
    assert!(never.contains("the sign-ins the factory holds"), "it says so: {never}");
    assert!(!to.join("signins").exists());
    let copied = journal::copy::open(&to.join(api::snapshot::DATABASE)).unwrap();
    let rows: i64 = copied.query_row("SELECT count(*) FROM signins", [], |row| row.get(0)).unwrap();
    assert_eq!(rows, 0, "no row about a sign-in travels");
    let draws: String =
        copied.query_row("SELECT signins FROM slots WHERE name = 'minion'", [], |row| row.get(0)).unwrap();
    assert_eq!(draws, r#"["key"]"#, "the agent still says what it draws, and will be told there is none");
    drop(copied);

    // And nothing the factory wrote anywhere has a value: its database
    // (events, turns, measurements), its logs, the copy.
    rig.stop().await;
    let mut searched = 0;
    for (path, bytes) in files_under(&rig.dir) {
        if path.starts_with(&kept) || path.starts_with(rig.dir.join("standin")) {
            continue;
        }
        searched += 1;
        for value in [first, second] {
            assert!(!holds(&bytes, value), "{} holds the value of a sign-in", path.display());
        }
    }
    assert!(searched > 3, "the database, the logs and the copy were looked through");
}

#[tokio::test]
async fn a_project_shares_sign_ins_out_by_its_own_copy_of_the_rule() {
    let rig = Rig::new().await;
    rig.send("/api/extensions", json!({ "source": package(&rig.dir) })).await;
    for name in ["first", "second"] {
        let value = format!("value-of-{name}");
        rig.send("/api/signins", json!({ "name": name, "for": "standin", "kind": "key", "value": value }))
            .await;
    }
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let inside = json!({ "name": "inside", "adapter": "standin", "project": "shop", "signins": ["key"] });
    rig.send("/api/agents", inside).await;
    rig.send("/api/agents", json!({ "name": "outside", "adapter": "standin", "signins": ["key"] })).await;

    // The project's own rule: its agents work on `second`, and on nothing else.
    let rule = r#"
blueprint: std/signin
permissions: []
params:
  signins: { type: list, default: [] }
outputs:
  take: ${ where(where(params.signins, "free", true), "name", "second")[0].name }
  wait: true
"#;
    let blueprints = rig.placed("shop").join("std");
    std::fs::create_dir_all(&blueprints).unwrap();
    std::fs::write(blueprints.join(brand::blueprint_file("signin")), rule).unwrap();

    for agent in ["inside", "inside", "outside", "outside"] {
        assert_eq!(work(&rig, agent).await["status"], "ok");
    }
    assert_eq!(
        handed(&rig),
        ["value-of-second", "value-of-second", "value-of-first", "value-of-second"],
        "the project's agent by the project's rule; the other by the one that ships, each in turn"
    );
    // And the rule that ships is there to be read, copied and checked.
    let shipped = rig.read("/api/blueprints").await.to_string();
    assert!(shipped.contains("std/signin"), "{shipped}");
}
