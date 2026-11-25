//! The variables of a factory's profile: set whole by its owner with
//! `std/profile`, and read by every blueprint there as
//! `profile.vars`.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A blueprint that needs nothing, and gives out what it reads.
const READER: &str = "\
blueprint: reader
permissions: []
steps:
  read: { do: noop }
outputs:
  api: \"${ profile.vars.api ?? 'http://localhost' }\"
  replicas: ${ profile.vars.replicas }
  all: ${ profile.vars }
";

fn asking(vars: Value) -> Value {
    json!({ "blueprint": "std/profile", "params": { "vars": vars }, "wait": true })
}

/// Sets the variables as the owner, and gives back how that went.
pub(crate) async fn set(rig: &Rig, vars: Value) -> Value {
    rig.send("/api/apply", asking(vars)).await["finished"].clone()
}

/// What a blueprint applied in this factory reads of them.
pub(crate) async fn read(rig: &Rig) -> Value {
    let applied = rig.send("/api/apply", json!({ "source": READER, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    applied["finished"]["outputs"].clone()
}

async fn said(rig: &Rig) -> Vec<Value> {
    let found = rig.read("/api/events/recent?topics=system.profile").await;
    let mut events = found["events"].as_array().unwrap().clone();
    events.sort_by_key(|event| event["seq"].as_u64());
    events
}

#[tokio::test]
async fn the_owner_says_what_is_true_of_the_profile_and_every_blueprint_there_reads_it() {
    let rig = Rig::new().await;
    // Nothing set: an empty map, and `??` gives what to use instead.
    assert_eq!(read(&rig).await, json!({ "api": "http://localhost", "replicas": null, "all": {} }));

    let finished = set(&rig, json!({ "api": "https://api.example", "replicas": 3, "careful": true })).await;
    assert_eq!(finished["status"], "ok", "{finished}");
    assert_eq!(finished["outputs"]["set"], json!(["api", "careful", "replicas"]));
    assert_eq!(
        read(&rig).await,
        json!({
            "api": "https://api.example", "replicas": 3,
            "all": { "api": "https://api.example", "replicas": 3, "careful": true },
        })
    );

    // A command is not handed them: it has what a blueprint passes it.
    // Of everything the first is started with, it says what the factory
    // tells every program, and whatever holds the address: which is
    // nothing.
    let all = crate::standin::cmd(&["env", "FACTORY_PROFILE", "env", "api.example"]);
    let one = crate::standin::cmd(&["say", "%API%"]);
    let shell = format!(
        "\
blueprint: commands
permissions: [shell]
steps:
  all: {{ do: shell, with: {{ cmd: {all} }} }}
  one: {{ do: shell, with: {{ cmd: {one}, env: {{ API: \"${{ profile.vars.api }}\" }} }} }}
outputs:
  all: ${{ steps.all.out.stdout }}
  one: ${{ steps.one.out.stdout }}
"
    );
    let ran = rig.send("/api/apply", json!({ "source": shell, "wait": true })).await["finished"].clone();
    assert_eq!(ran["status"], "ok", "{ran}");
    let all = ran["outputs"]["all"].as_str().unwrap();
    assert!(all.contains("FACTORY_PROFILE=") && !all.contains("api.example"), "{all}");
    assert_eq!(ran["outputs"]["one"].as_str().unwrap().trim(), "https://api.example");

    // Applied again, what is given is the whole of what there is.
    let finished = set(&rig, json!({ "api": "https://other.example", "region": "eu-west" })).await;
    assert_eq!(
        finished["outputs"],
        json!({ "names": ["api", "region"], "set": ["region"], "changed": ["api"], "removed": ["careful", "replicas"] })
    );
    assert_eq!(
        read(&rig).await,
        json!({
            "api": "https://other.example", "replicas": null,
            "all": { "api": "https://other.example", "region": "eu-west" },
        })
    );

    // Each change is an event: what was set, changed and removed, by name,
    // and who did it.
    let events = said(&rig).await;
    assert_eq!(events.len(), 2, "{events:?}");
    let me = rig.read("/api/me").await;
    assert_eq!(
        events[1]["data"],
        json!({
            "set": ["region"], "changed": ["api"], "removed": ["careful", "replicas"],
            "by": me["actor"], "by_name": "Ada",
        })
    );
    assert_eq!(events[1]["source"]["actor"], me["actor"]);

    // A value that is not plain is refused in a sentence, and nothing changes.
    let finished = set(&rig, json!({ "api": "https://other.example", "hosts": ["a", "b"] })).await;
    assert_eq!(finished["status"], "failed");
    let error = finished["error"].as_str().unwrap();
    assert!(
        error.contains(
            "the variable `hosts` is a list, and a variable is plain: text, a number, or true or false"
        ),
        "{error}"
    );
    assert_eq!(read(&rig).await["all"], json!({ "api": "https://other.example", "region": "eu-west" }));

    // No instance is left alive holding them: they are the factory's, and
    // are there until they are set again. Applied with none, there are none.
    assert_eq!(rig.read("/api/instances?live=true").await["instances"], json!([]));
    let finished = rig.send("/api/apply", json!({ "blueprint": "std/profile", "wait": true })).await;
    assert_eq!(finished["finished"]["outputs"]["removed"], json!(["api", "region"]));
    assert_eq!(read(&rig).await["all"], json!({}));
    assert_eq!(said(&rig).await.len(), 3);
}

#[tokio::test]
async fn a_members_key_and_an_agent_may_not_set_the_variables_or_take_them_away() {
    let rig = Rig::new().await;
    let actions = rig.read("/api/actions").await;
    let action = actions["actions"].as_array().unwrap().iter().find(|a| a["name"] == "profile.set").unwrap();
    assert_eq!(action["capability"], "admin", "the strictest there is: {action}");
    assert!(
        !kernel::principal::AGENT_SCOPES.contains(&"admin"),
        "which no agent holds unless it is given it"
    );
    assert!(!kernel::principal::MEMBER_SCOPES.contains(&"admin"), "and no member but the owner");

    let owners = rig.send("/api/apply", asking(json!({ "api": "https://api.example" }))).await;
    assert_eq!(owners["finished"]["status"], "ok", "{owners}");

    // A key for a script given what a member holds; an agent as its slot allows.
    let member = json!({ "name": "ci", "scopes": kernel::principal::MEMBER_SCOPES });
    let key = rig.send("/api/credentials", member).await["token"].as_str().unwrap().to_string();
    rig.send("/api/channels", json!({ "name": "general" })).await;
    // An agent starts nothing unless its slot says it may; this one may.
    let helper = json!({ "name": "helper", "scopes": ["projects", "channels", "blueprints"] });
    rig.send("/api/channels/general/agents", helper).await;
    let agent = std::fs::read_to_string(rig.dir.join("data/slots/helper/token")).unwrap();
    for (token, name) in [(&key, "ci"), (&agent, "helper")] {
        // Neither other values, nor none.
        for vars in [json!({ "api": "https://elsewhere.example" }), json!({})] {
            let (status, applied) =
                rig.call(Method::POST, "/api/apply", Some(token), Some(asking(vars))).await;
            assert_eq!(status, StatusCode::OK, "{applied}");
            assert_eq!(applied["finished"]["status"], "failed", "{name}: {applied}");
            assert_eq!(
                applied["finished"]["error"].as_str().unwrap().split(": ").last(),
                Some(format!("{name} may not use `admin`").as_str()),
                "{applied}"
            );
        }
        // And there is nothing of the owner's to stop: stopping the
        // instance that set them, which has finished, takes nothing away.
        let stop = format!("/api/instances/{}/stop", owners["instance"].as_str().unwrap());
        let (status, _) = rig.call(Method::POST, &stop, Some(token), Some(json!({}))).await;
        assert_eq!(status, StatusCode::OK);
    }
    // Nor may a blueprint that does not say it takes `admin`.
    let quiet = "blueprint: quiet\npermissions: [fleet]\nsteps:\n  set: { do: profile.set, with: { vars: { a: 1 } } }\n";
    let checked = rig.send("/api/blueprints/check", json!({ "source": quiet })).await;
    assert!(checked["diagnostics"].to_string().contains("not listed under `permissions`"), "{checked}");

    assert_eq!(read(&rig).await["all"], json!({ "api": "https://api.example" }));
    assert_eq!(said(&rig).await.len(), 1, "one change, the owner's");
}

#[tokio::test]
async fn the_variables_are_still_there_after_the_daemon_is_started_again() {
    let mut rig = Rig::new().await;
    set(&rig, json!({ "api": "https://api.example", "replicas": 3 })).await;
    rig.restart().await;
    assert_eq!(
        read(&rig).await,
        json!({ "api": "https://api.example", "replicas": 3, "all": { "api": "https://api.example", "replicas": 3 } })
    );
    assert_eq!(said(&rig).await.len(), 1, "starting again is not a change");
}
