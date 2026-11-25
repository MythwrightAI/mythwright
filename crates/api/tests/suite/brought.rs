//! The blueprints a package brings: there while it is installed, applied
//! like any other, and gone again when it is removed.
//!
//! The package here is made up, for a product there is not. The real ones
//! kept beside the application are held to their own examples elsewhere.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A package that brings one blueprint and no agent. The blueprint says
/// where the daemon answers, and stays for as long as it is not stopped.
const WRITTEN: &str = r#"
extension: reach
title: Reach
description: Not a real product.
program: reach-which-is-not-installed
blueprints:
  open: |
    # Comments and all, the text is the blueprint's.
    blueprint: reach/open
    description: Say where the daemon answers.
    permissions: [shell]
    params:
      door: { type: int, default: 9, description: "Which door." }
    steps:
      ask: { do: shell, with: { cmd: ASKS } }
    on:
      knocked: { event: reach.knocked, do: noop }
    outputs:
      daemon: "${trim(steps.ask.out.stdout)}"
      door: ${ params.door }
"#;

/// The package as it is written, with the program its blueprint starts
/// to say where the daemon answers: the stand-in, which says a variable.
fn package() -> String {
    WRITTEN.replace("ASKS", &crate::standin::cmd(&["say", "%FACTORY_URL%"]))
}

fn listed(blueprints: &Value) -> Vec<&str> {
    blueprints["names"].as_array().unwrap().iter().filter_map(Value::as_str).collect()
}

async fn refused(rig: &Rig, source: &str) -> String {
    let (status, error) = rig.post("/api/extensions", json!({ "source": source })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
    error["error"]["message"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn a_package_brings_blueprints_that_are_applied_by_name_while_it_is_installed() {
    // On offer, it brings nothing yet.
    let rig = Rig::with("api-brought", |config, dir| {
        let offered = dir.join("offered");
        std::fs::create_dir_all(offered.join("reach")).unwrap();
        std::fs::write(offered.join("reach").join("extension.yaml"), package()).unwrap();
        config.packages_dir = Some(offered);
    })
    .await;
    let mut rig = Rig::owned(rig).await;
    let offered = rig.read("/api/extensions").await;
    assert_eq!(offered["offered"][0]["blueprints"], json!(["reach/open"]), "it says what it would bring");
    assert!(!listed(&rig.read("/api/blueprints").await).contains(&"reach/open"));
    let by_name = json!({ "blueprint": "reach/open", "wait": true });
    let (status, _) = rig.post("/api/apply", by_name.clone()).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "what is only on offer cannot be applied");

    // Installed, it says what it brings, and no agent can run on it.
    let added = rig.send("/api/extensions", json!({ "name": "reach" })).await;
    assert_eq!(
        (&added["agent"], &added["blueprints"], &added["available"]),
        (&json!(false), &json!(["reach/open"]), &json!(false))
    );
    let providers = rig.read("/api/agents").await;
    let providers: Vec<&str> =
        providers["providers"].as_array().unwrap().iter().filter_map(|p| p["name"].as_str()).collect();
    assert_eq!(providers, ["worker", "builtin"], "it brings no agent");
    let (status, error) = rig.post("/api/agents", json!({ "name": "nobody", "adapter": "reach" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");

    // Its blueprint is among those that can be applied, described as the
    // others are, beside the ones that ship.
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    for path in ["/api/blueprints", "/api/blueprints?project=shop"] {
        let blueprints = rig.read(path).await;
        assert!(listed(&blueprints).contains(&"reach/open") && listed(&blueprints).contains(&"std/slot"));
        let described = blueprints["blueprints"]
            .as_array()
            .unwrap()
            .iter()
            .find(|one| one["name"] == "reach/open")
            .unwrap();
        assert_eq!(
            (&described["description"], &described["file"], &described["params"][0]["name"]),
            (&json!("Say where the daemon answers."), &Value::Null, &json!("door"))
        );
    }
    let source = rig.read("/api/blueprints/source?name=reach/open").await;
    assert!(
        source["source"].as_str().unwrap().starts_with("# Comments and all, the text is the blueprint's.\n")
    );

    // Applied by name, it runs like any other, and what it runs is told
    // where the daemon answers.
    let applied = rig.send("/api/apply", by_name.clone()).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(applied["finished"]["outputs"], json!({ "daemon": rig.url, "door": 9 }));
    let instance = applied["instance"].as_str().unwrap().to_string();

    // A project's own file of that name comes first, as it does for the
    // ones that ship.
    let own = "blueprint: reach/open\nsteps:\n  ask: { do: noop }\noutputs:\n  daemon: the shop's own\n";
    let written = json!({ "path": "@blueprints/reach/open.bp", "content": own });
    let (status, _) = rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(written)).await;
    assert_eq!(status, StatusCode::OK);
    let applied =
        rig.send("/api/apply", json!({ "blueprint": "reach/open", "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["outputs"]["daemon"], "the shop's own");

    // Whoever applies it lends it their authority and no more: an agent
    // has no shell, so this one does nothing for it.
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/channels/general/agents", json!({ "name": "helper", "scopes": ["blueprints"] })).await;
    let token = std::fs::read_to_string(rig.dir.join("data/slots/helper/token")).unwrap();
    let body = json!({ "blueprint": "reach/open", "key": "theirs", "wait": true });
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(token.trim()), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["finished"]["steps"][0]["error"], "helper may not use `shell`");

    // Removed, its blueprints are gone. What was applied keeps the
    // definition it was given and runs on, across a restart too.
    let (status, _) = rig.call(Method::DELETE, "/api/extensions/reach", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!listed(&rig.read("/api/blueprints").await).contains(&"reach/open"));
    let (status, error) = rig.post("/api/apply", json!({ "blueprint": "reach/open", "key": "again" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{error}");
    let status_of =
        async |rig: &Rig| rig.read(&format!("/api/instances/{instance}")).await["instance"]["status"].clone();
    assert_eq!(status_of(&rig).await, "active");
    rig.restart().await;
    assert_eq!(status_of(&rig).await, "active");
    assert!(
        listed(&rig.read("/api/blueprints?project=shop").await).contains(&"reach/open"),
        "the project's own is still the project's"
    );
}

#[tokio::test]
async fn a_package_whose_blueprints_do_not_check_is_not_installed_and_says_what_is_wrong_with_them() {
    let rig = Rig::new().await;

    // The blueprint's own problems, where in it they are.
    let misspelt = package().replace("permissions: [shell]", "permisions: [shell]");
    let said = refused(&rig, &misspelt).await;
    assert!(
        said.contains("`blueprints.open` does not check: line 4: ")
            && said.contains("did you mean `permissions`?"),
        "{said}"
    );
    // Looked up by one name, it may not call itself another.
    let misnamed = package().replace("blueprint: reach/open", "blueprint: open");
    assert!(
        refused(&rig, &misnamed)
            .await
            .contains("`blueprints.open` must call itself `reach/open`, not `open`")
    );
    let not_text = "extension: reach\nblueprints:\n  open: { blueprint: reach/open }\n";
    assert!(refused(&rig, not_text).await.contains("`blueprints.open` must be a blueprint's text"));
    let shouting = "extension: reach\nblueprints:\n  Open Sesame: |\n    blueprint: reach/open\n";
    assert!(refused(&rig, shouting).await.contains("must be named in lowercase letters, digits and dashes"));

    // A package brings an agent, blueprints, or both; never nothing.
    let said = refused(&rig, "extension: reach\ntitle: Reach\n").await;
    assert!(said.contains("brings nothing: it needs `agent`"), "{said}");
    assert!(refused(&rig, "extension: reach\nblueprints: {}\n").await.contains("`blueprints` is empty"));
    // And none may put its blueprints among the ones that ship.
    let standard = package().replace("extension: reach", "extension: std").replace("reach/open", "std/open");
    assert!(refused(&rig, &standard).await.contains("not `builtin`, `std` or `external`"));
    // Nor be called what an agent no program here runs is said to run on.
    let outside =
        package().replace("extension: reach", "extension: external").replace("reach/open", "external/open");
    assert!(refused(&rig, &outside).await.contains("not `builtin`, `std` or `external`"));

    assert!(rig.read("/api/extensions").await["extensions"].as_array().unwrap().is_empty());
    assert!(!listed(&rig.read("/api/blueprints").await).iter().any(|name| name.starts_with("reach/")));
}
