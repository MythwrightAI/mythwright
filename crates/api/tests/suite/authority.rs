//! Applying over an instance somebody else started: it is the asker's
//! token that says what the instance may then do.
//!
//! The owner's instance here sets the variables of the profile, which
//! takes `admin`; a key for a script acts as a member, and has none.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::variables::read;

/// What the owner applies: it says what it needs, sets the variables when
/// it starts, and again whenever it is told to.
const KEEPER: &str = "\
blueprint: keeper
permissions: [admin]
steps:
  set: { do: profile.set, with: { vars: { api: \"https://api.example\" } } }
on:
  told:
    event: keeper.told
    steps:
      set: { do: profile.set, with: { vars: { api: \"${event.data.api}\" } } }
";

/// The same instance by someone who lists nothing: its steps need
/// nothing, and its handler still wants to set the variables.
const QUIET: &str = "\
blueprint: keeper
steps:
  note: { do: noop }
on:
  told:
    event: keeper.told
    steps:
      set: { do: profile.set, with: { vars: { api: \"${event.data.api}\" } } }
";

/// The owner's instance, and a member's key.
async fn kept() -> (Rig, String, String) {
    let rig = Rig::new().await;
    let applied = rig.send("/api/apply", json!({ "source": KEEPER, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    // A key holds what its maker does unless it is given less: this one is
    // given what a member holds, which is everything but `admin`.
    let member = json!({ "name": "ci", "scopes": kernel::principal::MEMBER_SCOPES });
    let key = rig.send("/api/credentials", member).await["token"].as_str().unwrap().to_string();
    let (_, me) = rig.call(Method::GET, "/api/me", Some(&key), None).await;
    let scopes = me["principal"]["scopes"].to_string();
    assert!(scopes.contains("blueprints") && !scopes.contains("admin"), "{me}");
    (rig, applied["instance"].as_str().unwrap().to_string(), key)
}

async fn apply_as(rig: &Rig, token: &str, source: &str) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/apply", Some(token), Some(json!({ "source": source, "wait": true }))).await
}

/// Has the owner publish what the keeper's handler waits for, and gives
/// back the run of the handler that answered.
async fn told(rig: &Rig, instance: &str, api: &str) -> Value {
    let mark = rig.mark();
    let teller = format!(
        "blueprint: teller\npermissions: []\nsteps:\n  tell: {{ do: emit, with: {{ topic: keeper.told, api: \"{api}\" }} }}\n"
    );
    let applied = rig.send("/api/apply", json!({ "source": teller, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let finished = rig
        .event("run.finished", |event| {
            event.seq > mark
                && event.data["flow"] == "on.told"
                && event.source.instance.as_deref() == Some(instance)
        })
        .await;
    rig.read(&format!("/api/runs/{}", finished.source.run.as_deref().unwrap())).await
}

/// `who applied it: how it went`, for each version, the first first.
async fn history(rig: &Rig, instance: &str) -> Vec<String> {
    let versions = rig.read(&format!("/api/instances/{instance}/versions")).await;
    let mut said: Vec<String> = versions["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|version| {
            format!("{}: {}", version["by_name"].as_str().unwrap(), version["outcome"].as_str().unwrap())
        })
        .collect();
    said.reverse();
    said
}

#[tokio::test]
async fn a_key_without_admin_cannot_apply_over_the_owners_instance_to_do_what_only_the_owner_may() {
    let (rig, instance, key) = kept().await;
    let before = rig.read(&format!("/api/instances/{instance}")).await["instance"].clone();

    // Its own text under the instance's name, saying it needs `admin`:
    // refused before anything is changed.
    let (status, refused) =
        apply_as(&rig, &key, &KEEPER.replace("https://api.example", "https://elsewhere.example")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(refused["error"]["kind"], "denied");
    assert_eq!(
        refused["error"]["message"],
        "ci may not use `admin`, which `keeper` lists under `permissions`, so ci cannot apply it over \
         `keeper`, which is running, and nothing was changed. Someone who holds `admin` can apply it."
    );
    assert_eq!(rig.read(&format!("/api/instances/{instance}")).await["instance"], before);
    assert_eq!(history(&rig, &instance).await, ["Ada: applied"]);

    // Saying nothing of what it needs: it is the same instance, it runs
    // as the key, and the step only the owner may do is refused.
    let quiet = KEEPER
        .replace("permissions: [admin]\n", "")
        .replace("https://api.example", "https://elsewhere.example");
    let (status, applied) = apply_as(&rig, &key, &quiet).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(
        (applied["instance"].as_str(), applied["created"].as_bool()),
        (Some(instance.as_str()), Some(false))
    );
    assert_eq!(applied["finished"]["status"], "failed", "{applied}");
    assert_eq!(
        applied["finished"]["error"].as_str().unwrap().split(": ").last(),
        Some("ci may not use `admin`"),
        "{applied}"
    );
    assert_eq!(read(&rig).await["all"], json!({ "api": "https://api.example" }));

    // The owner's version is still the one running, as the owner.
    let after = rig.read(&format!("/api/instances/{instance}")).await["instance"].clone();
    assert_eq!((&after["status"], &after["revision"]), (&json!("active"), &before["revision"]));
    assert!(
        after["error"].as_str().unwrap().starts_with("update failed, still running the previous version"),
        "{after}"
    );
    assert_eq!(history(&rig, &instance).await, ["Ada: applied", "ci: reverted"]);
    let run = told(&rig, &instance, "https://told.example").await;
    assert_eq!(run["status"], "ok", "{run}");
    assert_eq!(read(&rig).await["all"], json!({ "api": "https://told.example" }));
}

#[tokio::test]
async fn an_instance_a_key_applied_over_does_what_the_key_may_until_the_owner_applies_over_it_again() {
    let (mut rig, instance, key) = kept().await;

    // What needs only what the key holds takes, and the instance is the key's.
    let (status, applied) = apply_as(&rig, &key, QUIET).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(
        (applied["instance"].as_str(), applied["finished"]["status"].as_str()),
        (Some(instance.as_str()), Some("ok"))
    );

    // Its handler may not do what only the owner may, now or after the
    // daemon is started again.
    for again in [false, true] {
        if again {
            rig.restart().await;
        }
        let run = told(&rig, &instance, "https://elsewhere.example").await;
        assert_eq!(run["status"], "failed", "{run}");
        assert_eq!(run["steps"][0]["error"], "ci may not use `admin`", "{run}");
        assert_eq!(read(&rig).await["all"], json!({ "api": "https://api.example" }));
    }

    // Asking for what is already running changes nothing, whoever asks.
    let applied = rig.send("/api/apply", json!({ "source": QUIET, "wait": true })).await;
    assert_eq!(
        (applied["instance"].as_str(), applied["changed"].as_bool()),
        (Some(instance.as_str()), Some(false))
    );
    let run = told(&rig, &instance, "https://elsewhere.example").await;
    assert_eq!(run["steps"][0]["error"], "ci may not use `admin`", "{run}");

    // The owner has it applied all the same: it is the owner's again.
    let applied = rig.send("/api/apply", json!({ "source": QUIET, "wait": true, "force": true })).await;
    assert_eq!(
        (applied["instance"].as_str(), applied["changed"].as_bool(), applied["finished"]["status"].as_str()),
        (Some(instance.as_str()), Some(true), Some("ok")),
        "{applied}"
    );
    let run = told(&rig, &instance, "https://told.example").await;
    assert_eq!(run["status"], "ok", "{run}");
    assert_eq!(read(&rig).await["all"], json!({ "api": "https://told.example" }));
    rig.restart().await;
    let run = told(&rig, &instance, "https://told-again.example").await;
    assert_eq!(run["status"], "ok", "{run}");
    assert_eq!(read(&rig).await["all"], json!({ "api": "https://told-again.example" }));

    assert_eq!(history(&rig, &instance).await, ["Ada: applied", "ci: applied", "Ada: applied"]);
}
