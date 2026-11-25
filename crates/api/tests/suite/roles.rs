//! A role: so many agents that do the same kind of work, brought up by a
//! blueprint, and given work as a role and not by name. The factory knows
//! what a slot is and that slots can share a word; what the word means is
//! the instructions those slots are given and the blueprints that give
//! them work.

use serde_json::{Value, json};

use crate::Rig;

/// A node whose agents can be run by `echo`, a program that says what it
/// was handed back.
async fn rig() -> Rig {
    let rig = Rig::with("api-roles", |config, _| {
        config.adapters.push(("echo".into(), crate::standin::argv(&["swallow", "say", "done"])));
    })
    .await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig
}

/// `std/role` applied for reviewers, so many of them.
fn reviewers(count: i64) -> Value {
    json!({
        "blueprint": "std/role", "project": "shop", "key": "reviewers", "wait": true,
        "params": {
            "role": "reviewer", "count": count, "adapter": "echo", "permission": "read",
            "instructions": "You review changes. You change nothing yourself.",
        },
    })
}

/// Each agent there is: its name, its role, and whether it takes work.
async fn agents(rig: &Rig) -> Vec<(String, String, bool)> {
    let all = rig.read("/api/agents").await;
    let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
    let mut all: Vec<(String, String, bool)> = all["agents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|agent| (text(&agent["name"]), text(&agent["role"]), agent["active"] == true))
        .collect();
    all.sort();
    all
}

#[tokio::test]
async fn a_role_is_as_many_agents_as_its_blueprint_says_each_told_and_allowed_the_same() {
    let rig = rig().await;
    let applied = rig.send("/api/apply", reviewers(3)).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(applied["finished"]["outputs"], json!({ "role": "reviewer", "agents": 3 }));
    let up = |name: &str, active: bool| (name.to_string(), "reviewer".to_string(), active);
    assert_eq!(agents(&rig).await, [up("reviewer-1", true), up("reviewer-2", true), up("reviewer-3", true)]);
    // Each is a slot like any other, with what the role is told and may do.
    let one = rig.read("/api/agents").await["agents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|agent| agent["name"] == "reviewer-2")
        .cloned()
        .unwrap();
    assert_eq!(
        (&one["instructions"], &one["permission_mode"], &one["project"], &one["adapter"]),
        (
            &json!("You review changes. You change nothing yourself."),
            &json!("read"),
            &json!("shop"),
            &json!("echo")
        )
    );

    // Fewer: those past the count take no more work. More again: they are back.
    let applied = rig.send("/api/apply", reviewers(1)).await;
    assert_eq!(applied["finished"]["outputs"]["agents"], 1, "{applied}");
    assert_eq!(
        agents(&rig).await,
        [up("reviewer-1", true), up("reviewer-2", false), up("reviewer-3", false)]
    );
    rig.send("/api/apply", reviewers(2)).await;
    assert_eq!(agents(&rig).await, [up("reviewer-1", true), up("reviewer-2", true), up("reviewer-3", false)]);

    // Fewer than none there cannot be, and applying that takes nothing away.
    let fewer = rig.send("/api/apply", reviewers(-1)).await;
    assert_eq!(fewer["finished"]["status"], "failed", "{fewer}");
    assert!(fewer.to_string().contains("is not less than none"), "{fewer}");
    assert_eq!(agents(&rig).await[..2], [up("reviewer-1", true), up("reviewer-2", true)]);
    // None is a number like another: nobody is in the role.
    let none = rig.send("/api/apply", reviewers(0)).await;
    assert_eq!(none["finished"]["outputs"]["agents"], 0, "{none}");
    assert!(agents(&rig).await.iter().all(|(_, _, active)| !active), "{:?}", agents(&rig).await);
    let applied = rig.send("/api/apply", reviewers(2)).await;
    assert_eq!(agents(&rig).await[..2], [up("reviewer-1", true), up("reviewer-2", true)]);

    // Stopped, the role is nobody.
    let instance = applied["instance"].as_str().unwrap();
    rig.send(&format!("/api/instances/{instance}/stop"), json!({})).await;
    assert!(agents(&rig).await.iter().all(|(_, _, active)| !active), "{:?}", agents(&rig).await);
}

#[tokio::test]
async fn work_is_given_to_a_role_and_goes_to_one_of_its_agents_and_a_role_nobody_is_in_says_so() {
    let rig = rig().await;
    rig.send("/api/apply", reviewers(2)).await;
    // Another role beside it, and an agent in none.
    let workers = json!({
        "blueprint": "std/role", "project": "shop", "key": "workers", "wait": true,
        "params": { "role": "worker", "count": 1, "adapter": "echo" },
    });
    rig.send("/api/apply", workers).await;
    rig.send("/api/agents", json!({ "name": "aaa-first-by-name", "adapter": "echo", "project": "shop" }))
        .await;

    let ask = |role: &str| {
        let source = format!(
            "blueprint: ask\nsteps:\n  look:\n    do: agent.task\n    with: {{ role: {role}, prompt: \"Look at this.\" }}\noutputs:\n  by: ${{ steps.look.out.slot }}\n"
        );
        json!({ "source": source, "project": "shop", "key": format!("ask-{role}"), "wait": true })
    };
    let reviewed = rig.send("/api/apply", ask("reviewer")).await;
    assert_eq!(reviewed["finished"]["outputs"]["by"], "reviewer-1", "{reviewed}");
    let worked = rig.send("/api/apply", ask("worker")).await;
    assert_eq!(worked["finished"]["outputs"]["by"], "worker-1", "{worked}");

    // Nobody is an architect here.
    let nobody = rig.send("/api/apply", ask("architect")).await;
    assert_eq!(nobody["finished"]["status"], "failed", "{nobody}");
    assert!(nobody.to_string().contains("no agent in the role `architect` is active"), "{nobody}");
}
