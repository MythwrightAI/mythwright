//! The actions a package declares, over the API: listed with every other
//! action while the package is installed, checked against, done by whoever
//! may use the package, and gone again when it is removed.
//!
//! The package here is made up, for a product there is not, and its
//! program is a script written here.
//!
//! Unix's alone: the package's program is a file of a Unix shell that
//! takes commands of its own, made a program by its mode.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A package that declares two actions, and the program they run: a script
/// that says what it was asked and where it was told the daemon is.
fn package_in(dir: &std::path::Path) -> String {
    let program = dir.join("example-program");
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in\n  status) printf '{{ \"state\": \"up\", \"daemon\": \"%s\" }}\\n' \"${}\" ;;\n  \
         serve) echo \"serving $3 on $2\" ;;\nesac\n",
        hosting::ADDRESS_ENV
    );
    std::fs::write(&program, script).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    format!(
        r#"
extension: example
title: Example
description: Not a real product.
program: {program}
actions:
  status:
    description: Say whether it is up.
    args: ["status"]
    answer: json
    fails:
      - when: ${{ answer.state != "up" }}
        says: "It is not up."
    out:
      state: ${{ answer.state }}
      told: ${{ answer.daemon }}
    repeatable: true
  serve:
    description: Serve something on a port.
    with:
      port: {{ type: int, required: true, description: "The port." }}
      to: {{ type: string, default: "${{ daemon.url }}", description: "What is served; the daemon, unless said." }}
    args: ["serve", "--port=${{ with.port }}", "${{ with.to }}"]
    out:
      to: ${{ with.to }}
      said: ${{ trim(answer) }}
blueprints:
  up: |
    blueprint: example/up
    permissions: [extensions:example]
    steps:
      ask: {{ do: example.status }}
      serve: {{ do: example.serve, with: {{ port: 8443 }} }}
    outputs:
      to: ${{ steps.serve.out.to }}
      said: ${{ steps.serve.out.said }}
      told: ${{ steps.ask.out.told }}
"#,
        program = program.display()
    )
}

/// The names of the actions in a list of them that are the package's.
fn theirs(actions: &Value) -> Vec<&str> {
    let all = actions.as_array().unwrap().iter().filter_map(|action| action["name"].as_str());
    all.filter(|name| name.starts_with("example.")).collect()
}

const ASKING: &str =
    "blueprint: asking\npermissions: [extensions:example]\nsteps:\n  ask: { do: example.status }\n";

#[tokio::test]
async fn a_packages_actions_are_listed_checked_against_and_done_while_it_is_installed() {
    let rig = Rig::with("api-actions", |config, dir| {
        let offered = dir.join("offered");
        std::fs::create_dir_all(offered.join("example")).unwrap();
        std::fs::write(offered.join("example").join("extension.yaml"), package_in(dir)).unwrap();
        config.packages_dir = Some(offered);
    })
    .await;
    let rig = Rig::owned(rig).await;

    // On offer, it says what it would bring, and brings nothing yet. A
    // blueprint that does one of its actions is told what to install.
    let offered = rig.read("/api/extensions").await;
    assert_eq!(theirs(&offered["offered"][0]["actions"]), ["example.serve", "example.status"]);
    assert!(theirs(&rig.read("/api/actions").await["actions"]).is_empty());
    let checked = rig.send("/api/blueprints/check", json!({ "source": ASKING })).await;
    assert_eq!(
        (&checked["ok"], &checked["diagnostics"][0]["code"], &checked["diagnostics"][0]["message"]),
        (
            &json!(false),
            &json!("unknown-action"),
            &json!(format!(
                "`example.status` comes with the extension package `example`, which is not installed on this \
                 device; install it with `{} extension add example`",
                brand::CLI
            ))
        )
    );

    // Installed, its actions are among the device's, described as every
    // action is, wherever actions are listed.
    let added = rig.send("/api/extensions", json!({ "name": "example" })).await;
    assert_eq!(theirs(&added["actions"]), ["example.serve", "example.status"]);
    for path in ["/api/actions", "/api/blueprints/language"] {
        let listed = rig.read(path).await;
        assert_eq!(theirs(&listed["actions"]), ["example.serve", "example.status"], "{path}");
        let serve =
            listed["actions"].as_array().unwrap().iter().find(|action| action["name"] == "example.serve");
        assert_eq!(
            serve.unwrap(),
            &json!({
                "name": "example.serve",
                "summary": "Serve something on a port.",
                "args": [
                    { "name": "port", "type": "int", "required": true, "help": "The port." },
                    { "name": "to", "type": "string", "required": false, "help": "What is served; the daemon, unless said." },
                ],
                "open": false,
                "capability": "extensions:example",
                "outputs": [["said", ""], ["to", ""]],
                "idempotent": false,
            })
        );
    }
    // And what an editor of somebody's own is handed says them too, each
    // with what it takes.
    let schema = rig.read("/api/blueprints/schema").await;
    let named = schema["definitions"]["an-action"]["anyOf"].as_array().unwrap();
    let serve = named.iter().find(|action| action["const"] == "example.serve").expect("the schema names it");
    assert_eq!(serve["description"], "Serve something on a port.");
    let taken = schema["definitions"]["what-an-action-takes"]["allOf"].as_array().unwrap();
    let serve = taken.iter().find(|action| action["if"]["properties"]["do"]["const"] == "example.serve");
    let with = &serve.expect("the schema says what it takes")["then"]["properties"]["with"];
    assert_eq!(with["properties"]["port"]["description"], "The port. Needed.");
    assert_eq!(with["additionalProperties"], false);
    assert_eq!(rig.send("/api/blueprints/check", json!({ "source": ASKING })).await["ok"], true);
    let added = rig.event("extension.added", |_| true).await;
    assert_eq!(added.data["actions"], json!(["example.serve", "example.status"]));

    // Its blueprint does them: the program is run, told where the daemon
    // is, and what is left out is the daemon's own address.
    let applied = rig.send("/api/apply", json!({ "blueprint": "example/up", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(
        applied["finished"]["outputs"],
        json!({ "to": rig.url, "said": format!("serving {} on --port=8443", rig.url), "told": rig.url })
    );

    // Whoever applies a blueprint lends it their authority and no more. An
    // agent may not use a package unless it is given leave for it, and
    // leave for the package is all it needs: it has no shell.
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/channels/general/agents", json!({ "name": "helper", "scopes": ["blueprints"] })).await;
    let trusted = json!({ "name": "trusted", "scopes": ["blueprints", "extensions:example"] });
    rig.send("/api/channels/general/agents", trusted).await;
    let apply_as = async |slot: &str| {
        let token = std::fs::read_to_string(rig.dir.join(format!("data/slots/{slot}/token"))).unwrap();
        // Keyed apart from the agent's own text, which is keyed by its name.
        let body = json!({ "source": ASKING, "key": format!("asked-by-{slot}"), "wait": true });
        let (status, applied) = rig.call(Method::POST, "/api/apply", Some(token.trim()), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{applied}");
        applied["finished"]["steps"][0].clone()
    };
    assert_eq!(apply_as("helper").await["error"], "helper may not use `extensions:example`");
    let done = apply_as("trusted").await;
    assert_eq!((&done["status"], &done["output"]["state"]), (&json!("ok"), &json!("up")), "{done}");

    // Removed, they are gone from every list, and a blueprint that does
    // one is told again what to install.
    let (status, _) = rig.call(Method::DELETE, "/api/extensions/example", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(theirs(&rig.read("/api/actions").await["actions"]).is_empty());
    assert_eq!(rig.send("/api/blueprints/check", json!({ "source": ASKING })).await["ok"], false);
    let (status, refused) = rig.post("/api/apply", json!({ "source": ASKING, "key": "late" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(
        refused["error"]["message"].as_str().unwrap().contains("which is not installed on this device"),
        "{refused}"
    );
}

#[tokio::test]
async fn a_package_whose_actions_do_not_check_is_not_installed_and_says_what_is_wrong_with_them() {
    let rig = Rig::new().await;
    let refused = async |source: String| {
        let (status, error) = rig.post("/api/extensions", json!({ "source": source })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
        error["error"]["message"].as_str().unwrap().to_string()
    };
    let package = package_in(&rig.dir);

    let said = refused(package.replace("answer: json", "answer: xml")).await;
    assert!(said.contains("`actions.status.answer` must be `json`"), "{said}");
    // Its blueprints may do only what it declares, and have to say they use it.
    let said = refused(package.replace("do: example.status", "do: example.state")).await;
    assert!(said.contains("this package declares no action called `state`"), "{said}");
    let said = refused(package.replace("permissions: [extensions:example]", "permissions: []")).await;
    assert!(
        said.contains("this uses `extensions:example`, which is not listed under `permissions`"),
        "{said}"
    );

    // A name the device's own actions are under is not a package's to take.
    let said = {
        let taken = package.replace("example", "board");
        let (status, error) = rig.post("/api/extensions", json!({ "source": taken })).await;
        assert_eq!(status, StatusCode::CONFLICT, "{error}");
        error["error"]["message"].as_str().unwrap().to_string()
    };
    assert!(said.contains("`board` is already what actions of this device's own are called under"), "{said}");
    assert!(rig.read("/api/extensions").await["extensions"].as_array().unwrap().is_empty());
}
