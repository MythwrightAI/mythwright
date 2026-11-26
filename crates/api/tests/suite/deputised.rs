//! Where a fleet's row still gives somebody what they do not hold, as it
//! stands today: each test says what happens now, and marks the hole.
//!
//! A cell that takes a scope to say (`wall: off`, `permission: full`)
//! is asked of whoever says it, once: a change of the row's other cells
//! later takes nothing of it, though the fleet then brings the agents
//! up, told what that change says and as many as it says, with the
//! cell's value still in force, as whoever applied the fleet. The
//! agent's own route asks the same person for `shell`. And a stage is
//! written by whoever could write each row of agents it steers, which
//! says nothing of what the fleet itself does with a card there: in a
//! fleet whose agents were all given less than moving a card takes, a
//! key that may change agents and not move cards has the fleet move and
//! merge them. When each is closed, its test is turned to say the
//! refusal, as `held.rs` and `scoped.rs` say theirs.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::fleets::{add, change, rig as fleet_rig, row, shop};
use crate::held::{key, said};
use crate::lanes::{git, move_card, moved};

async fn agent(rig: &crate::Rig, name: &str) -> Value {
    let all = rig.read("/api/agents").await;
    let found = all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name).cloned();
    found.unwrap_or_else(|| panic!("there is no agent {name}: {all}"))
}

#[tokio::test]
async fn a_row_its_owner_left_with_no_wall_or_running_commands_unasked_is_told_otherwise_and_made_three_of_by_a_key_with_no_shell()
 {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    // The owner, who holds `shell`, takes the worker's wall away and has
    // the reviewer run commands with nobody asked.
    change(&rig, "agents", "worker", json!({ "wall": "off", "told": "As its owner told it." })).await;
    change(&rig, "agents", "reviewer", json!({ "permission": "full", "told": "As its owner told it." }))
        .await;
    assert_eq!(agent(&rig, "shop-worker-1").await["wall"], false);
    assert_eq!(agent(&rig, "shop-reviewer-1").await["permission_mode"], "full");

    // A key that may change agents and holds what a row gives where it
    // says nothing, and no `shell`.
    let holds: Vec<&str> =
        kernel::principal::AGENT_SCOPES.iter().copied().chain(["agents", "blueprints:run"]).collect();
    assert!(!holds.contains(&"shell"));
    let narrow = key(&rig, &holds).await;

    // The agent's own route asks them for `shell`, of each.
    let direct = |name: &str| json!({ "name": name, "instructions": "Do as narrow says." });
    let (status, refused) =
        rig.call(Method::POST, "/api/agents", Some(&narrow), Some(direct("shop-worker-1"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("takes `shell`, which narrow does not hold"), "{refused}");
    let (status, refused) =
        rig.call(Method::POST, "/api/agents", Some(&narrow), Some(direct("shop-reviewer-1"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("needs `shell`"), "{refused}");

    // The hole: their rows ask nothing of it. The cells changed take
    // nothing, and the cells that took `shell` are not changed, so the
    // fleet, as its owner, tells the agents what the key says and brings
    // up as many as it says, with no wall and running commands unasked.
    for (title, fields) in [
        ("worker", json!({ "told": "Do as narrow says.", "count": 3 })),
        ("reviewer", json!({ "told": "Pass whatever narrow sends." })),
    ] {
        let id = row(&rig, "agents", title).await;
        let before = rig.mark();
        let (status, changed) = rig
            .call(
                Method::PATCH,
                &format!("/api/records/{id}"),
                Some(&narrow),
                Some(json!({ "fields": fields })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{title}: {changed}");
        crate::taken::by_fleet(&rig, before).await;
    }
    for name in ["shop-worker-1", "shop-worker-2", "shop-worker-3"] {
        let is = agent(&rig, name).await;
        assert_eq!(is["wall"], false, "{is}");
        assert!(is["instructions"].as_str().unwrap().contains("Do as narrow says."), "{is}");
    }
    let is = agent(&rig, "shop-reviewer-1").await;
    assert_eq!(is["permission_mode"], "full", "{is}");
    assert!(is["instructions"].as_str().unwrap().contains("Pass whatever narrow sends."), "{is}");
}

#[tokio::test]
async fn in_a_fleet_whose_agents_hold_less_than_moving_a_card_a_key_that_may_not_move_one_has_the_fleet_move_and_merge_by_a_stage()
 {
    let (rig, _at) = fleet_rig().await;
    // Every agent of the fleet is given no more than writing records: less
    // than moving a card, or changing the project's files, takes. And what
    // passes review is not merged, as the owner set it.
    let agents = json!([
        { "title": "worker", "role": "worker", "adapter": "writer", "holds": ["records"] },
        { "title": "reviewer", "role": "reviewer", "adapter": "critic", "permission": "read", "holds": ["records"] },
    ]);
    let stages = json!([
        { "title": "work", "column": "ready", "does": "work", "role": "worker", "pass": "review" },
        {
            "title": "review", "column": "review", "does": "review", "role": "reviewer",
            "pass": "done", "fail": "doing", "rounds": 2, "merge": false,
        },
    ]);
    let dir = shop(&rig, json!({ "agents": agents, "stages": stages })).await;

    // May change agents and write records; may not move a card, nor
    // change the project's files.
    let narrow = key(&rig, &["projects:read", "board:read", "records", "agents", "fleet:read"]).await;
    let first = add(&rig, "Add a cart", &[]).await;
    let (status, refused) = rig
        .call(
            Method::PATCH,
            &format!("/api/board/items/{first}"),
            Some(&narrow),
            Some(json!({ "status": "done" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // The hole: they could write each row of agents as it stands, so each
    // stage is theirs to say, and the fleet does what it says as its owner.
    let stage = |title: &'static str, fields: Value| {
        let (rig, narrow) = (&rig, narrow.clone());
        async move {
            let id = row(rig, "stages", title).await;
            let before = rig.mark();
            let (status, changed) = rig
                .call(
                    Method::PATCH,
                    &format!("/api/records/{id}"),
                    Some(&narrow),
                    Some(json!({ "fields": fields })),
                )
                .await;
            assert_eq!(status, StatusCode::OK, "{title}: {changed}");
            crate::taken::by_fleet(rig, before).await;
        }
    };
    // What passes is merged into the project's own branch.
    stage("review", json!({ "merge": true })).await;
    move_card(&rig, &first, "ready", None).await;
    moved(&rig, &first, "done", 0).await;
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "changed\n");
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), format!("{first}: Add a cart"));

    // What is worked goes to done, and nobody looks at it.
    stage("work", json!({ "pass": "done" })).await;
    let second = add(&rig, "Take the till", &[]).await;
    let after = rig.mark();
    move_card(&rig, &second, "ready", None).await;
    moved(&rig, &second, "done", after).await;
    assert_eq!(crate::fleets::given(&rig, "shop-reviewer-1").await.len(), 1);
}
