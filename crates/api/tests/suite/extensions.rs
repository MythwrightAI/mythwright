//! Extension packages over the API: installing what agents run on.
//!
//! Unix's alone: the stand-in for a package's program is a file of a Unix
//! shell, made a program by its mode.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A package for a made-up agent: a script that answers like one, and says
/// which models it has when asked.
fn package_in(dir: &std::path::Path) -> String {
    let program = dir.join("standin");
    let script = r#"#!/bin/sh
if [ "$1" = "models" ]; then
  read asked
  printf '{"models":[{"key":"quick","name":"Quick","usual":true},{"key":"deep","name":"Deep"}]}\n'
  sleep 300
  exit 0
fi
cat > /dev/null
printf '{"kind":"end","text":"Done by the stand-in, on %s."}\n' "${2:-its usual model}"
"#;
    std::fs::write(&program, script).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    format!(
        r#"
extension: standin
title: Stand-in
description: Not a real agent.
agent:
  program: {program}
  args:
    - when: ${{ turn.model != null }}
      args: ["--model", "${{ turn.model }}"]
  input: ${{ turn.prompt }}
  output:
    - when: ${{ event.kind == "end" }}
      reply: ${{ event.text }}
  models:
    ask:
      args: ["models"]
      send:
        - {{ ask: models }}
      answer: ${{ line.models != null }}
      list: ${{ line.models }}
      id: ${{ model.key }}
      label: ${{ model.name }}
      default: ${{ model.usual == true }}
"#,
        program = program.display()
    )
}

fn package(rig: &Rig) -> String {
    package_in(&rig.dir)
}

fn names(listing: &Value, key: &str) -> Vec<String> {
    listing[key].as_array().unwrap().iter().map(|entry| entry["name"].as_str().unwrap().to_string()).collect()
}

#[tokio::test]
async fn a_package_is_installed_and_agents_run_on_it_with_a_model_from_its_own_list() {
    let rig = Rig::new().await;
    // Out of the box: a program configured on this device (the rig's) and
    // the built-in. No agent anyone would know by name.
    assert_eq!(names(&rig.read("/api/agents").await, "providers"), ["worker", "builtin"]);
    assert!(rig.read("/api/extensions").await["extensions"].as_array().unwrap().is_empty());

    let added = rig.send("/api/extensions", json!({ "source": package(&rig) })).await;
    assert_eq!(
        (&added["name"], &added["available"], &added["models"]),
        (&json!("standin"), &json!(true), &json!("asked"))
    );
    let agents = rig.read("/api/agents").await;
    assert_eq!(names(&agents, "providers"), ["standin", "worker", "builtin"]);
    assert_eq!(agents["providers"][0]["title"], "Stand-in");

    // An agent left on its provider's usual model: which one that is, is
    // not known until the program has been asked, and nothing asks unasked.
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/channels/general/agents", json!({ "name": "usual", "adapter": "standin" })).await;
    let model_of = async |name: &str| {
        let agents = rig.read("/api/agents").await;
        let agent = agents["agents"].as_array().unwrap().iter().find(|a| a["name"] == name).unwrap().clone();
        agent["model_name"].clone()
    };
    assert_eq!(model_of("usual").await, Value::Null);

    // Its models, as its own program lists them.
    let models = rig.read("/api/providers/standin/models").await;
    assert_eq!(
        models["models"],
        json!([
            { "id": "quick", "label": "Quick", "note": "", "default": true },
            { "id": "deep", "label": "Deep", "note": "", "default": false },
        ])
    );
    assert_eq!(
        rig.read("/api/providers/builtin/models").await["models"],
        json!([]),
        "the built-in has none to offer"
    );
    let (status, _) = rig.get("/api/providers/nonesuch/models").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // An agent on it, on a model from that list, does its work.
    rig.send(
        "/api/channels/general/agents",
        json!({ "name": "helper", "adapter": "standin", "model": "deep" }),
    )
    .await;
    rig.send("/api/channels/general/messages", json!({ "body": "@helper are you there?" })).await;
    let reply = rig.event("channel.message", |event| event.data["author"] == "agent:helper").await;
    assert_eq!(reply.data["body"], "Done by the stand-in, on deep.");

    // Each agent's model is called what its provider calls it: the one
    // chosen, or the provider's usual one now that it has said which that
    // is. Its messages say so too, where the program itself does not.
    assert_eq!(model_of("helper").await, "Deep");
    assert_eq!(model_of("usual").await, "Quick");
    assert_eq!(reply.data["model"], "Deep");
    rig.send("/api/channels/general/messages", json!({ "body": "@usual and you?" })).await;
    let reply = rig.event("channel.message", |event| event.data["author"] == "agent:usual").await;
    assert_eq!(reply.data["model"], "Quick");

    // The package survives a restart, with what its program said it
    // offers, and goes when it is removed.
    let mut rig = rig;
    rig.restart().await;
    assert_eq!(names(&rig.read("/api/extensions").await, "extensions"), ["standin"]);
    let agents = rig.read("/api/agents").await;
    let usual = agents["agents"].as_array().unwrap().iter().find(|a| a["name"] == "usual").unwrap();
    assert_eq!(usual["model_name"], "Quick", "known without asking the program again");
    let (status, _) = rig.call(Method::DELETE, "/api/extensions/standin", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(names(&rig.read("/api/agents").await, "providers"), ["worker", "builtin"]);
}

#[tokio::test]
async fn an_agent_is_not_run_more_freely_than_it_was_meant_to_be() {
    let rig = Rig::new().await;
    // A program that asks nobody's leave, whatever it is told, says so.
    let unheld = package(&rig).replace("  input:", "  permissions: [full]\n  input:");
    rig.send("/api/extensions", json!({ "source": unheld })).await;
    let listed = rig.read("/api/agents").await;
    let standin = listed["providers"].as_array().unwrap().iter().find(|p| p["name"] == "standin").unwrap();
    assert_eq!(standin["permissions"], json!(["full"]));
    let builtin = listed["providers"].as_array().unwrap().iter().find(|p| p["name"] == "builtin").unwrap();
    assert_eq!(
        builtin["permissions"],
        json!(["read", "edit", "full"]),
        "anything else is held to all of them"
    );

    // Asked for nothing in particular, an agent is meant to change files and
    // no more. This program cannot be held to that, so it is not started.
    let (status, error) = rig.post("/api/agents", json!({ "name": "loose", "adapter": "standin" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
    assert_eq!(
        error["error"]["message"],
        "an agent on Stand-in cannot be held to `edit`; its program can only be given `full` (anything, unasked). \
         Choose that for this agent if it is what you mean"
    );
    let (status, _) = rig
        .post("/api/agents", json!({ "name": "loose", "adapter": "standin", "permission_mode": "read" }))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(rig.read("/api/agents").await["agents"].as_array().unwrap().is_empty(), "and none was made");

    // Given it knowingly, it is.
    let made = rig
        .send("/api/agents", json!({ "name": "loose", "adapter": "standin", "permission_mode": "full" }))
        .await;
    assert_eq!(made["permission_mode"], "full", "{made}");

    // A package cannot claim a mode there is not.
    let (status, error) = rig
        .post(
            "/api/extensions",
            json!({ "source": package(&rig).replace("  input:", "  permissions: [most]\n  input:") }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("`agent.permissions` lists \"most\", which is not one of read, edit, full"),
        "{error}"
    );
}

#[tokio::test]
async fn a_package_decides_what_the_daemon_runs_so_only_the_owner_installs_one_and_only_if_it_checks() {
    let rig = Rig::new().await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/channels/general/agents", json!({ "name": "helper" })).await;
    let token = std::fs::read_to_string(rig.dir.join("data/slots/helper/token")).unwrap();
    let source = package(&rig);
    let (status, error) = rig
        .call(Method::POST, "/api/extensions", Some(token.trim()), Some(json!({ "source": source })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{error}");

    let (status, error) =
        rig.post("/api/extensions", json!({ "source": "extension: broken\nagent:\n  output: []\n" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let message = error["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("`agent.program` is needed") && message.contains("at least one rule with `reply`"),
        "{message}"
    );

    let named_builtin = source.replace("extension: standin", "extension: worker");
    let (status, error) = rig.post("/api/extensions", json!({ "source": named_builtin })).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a package cannot take the name of a configured program: {error}"
    );
}

#[tokio::test]
async fn a_package_that_came_with_the_product_is_offered_and_installed_by_name() {
    // What ships beside the application is not in use, only on offer.
    let rig = Rig::with("api-offered", |config, dir| {
        let offered = dir.join("offered");
        std::fs::create_dir_all(offered.join("standin")).unwrap();
        std::fs::write(offered.join("standin").join("extension.yaml"), package_in(dir)).unwrap();
        config.packages_dir = Some(offered);
    })
    .await;
    let rig = Rig::owned(rig).await;
    let listed = rig.read("/api/extensions").await;
    assert!(listed["extensions"].as_array().unwrap().is_empty(), "nothing is installed until someone asks");
    assert_eq!(names(&listed, "offered"), ["standin"]);
    assert_eq!(
        names(&rig.read("/api/agents").await, "providers"),
        ["worker", "builtin"],
        "and nothing runs on it"
    );

    let (status, error) = rig.post("/api/extensions", json!({ "name": "nonesuch" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(error["error"]["message"].as_str().unwrap().ends_with("there are: standin"), "{error}");

    let added = rig.send("/api/extensions", json!({ "name": "standin" })).await;
    assert_eq!(added["title"], "Stand-in");
    let listed = rig.read("/api/extensions").await;
    assert_eq!(names(&listed, "extensions"), ["standin"]);
    assert!(listed["offered"].as_array().unwrap().is_empty(), "once installed it is no longer on offer");
    assert_eq!(names(&rig.read("/api/agents").await, "providers"), ["standin", "worker", "builtin"]);

    // Removed, it is on offer again.
    let (status, _) = rig.call(Method::DELETE, "/api/extensions/standin", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(names(&rig.read("/api/extensions").await, "offered"), ["standin"]);
}
