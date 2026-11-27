//! What a cell took stands while its row says so.
//!
//! A cell that takes a scope to say (`wall: off`, `permission: full`, a
//! command) lets the fleet do more with the row, as whoever applied the
//! fleet: so while the row says it, changing anything of the row takes
//! it too (`stands`), or what the agent is told, how many there are of
//! it and where a command runs would be anybody's who may change agents.
//! The agent's own route asks the same. And a stage moves its cards as
//! whoever applied the fleet, which is not what any agent it steers
//! holds: so writing one takes what moving a card takes, and one that
//! merges what changing the project's files takes, whatever the agents
//! were given.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::fleets::{change, rig as fleet_rig, row, shop};
use crate::held::{key, said};

async fn agent(rig: &crate::Rig, name: &str) -> Option<Value> {
    let all = rig.read("/api/agents").await;
    all["agents"].as_array().unwrap().iter().find(|agent| agent["name"] == name).cloned()
}

/// Changes a row of one of the fleet's tables with `token`: how it was answered.
async fn write(
    rig: &crate::Rig,
    token: &str,
    table: &str,
    title: &str,
    fields: Value,
) -> (StatusCode, Value) {
    let id = row(rig, table, title).await;
    let body = Some(json!({ "fields": fields }));
    rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(token), body).await
}

#[tokio::test]
async fn what_a_row_its_owner_left_with_no_wall_or_running_commands_unasked_or_a_command_took_stands_for_whoever_changes_it_after()
 {
    let (rig, _at) = fleet_rig().await;
    shop(&rig, json!({})).await;
    // The owner, who holds `shell`, takes the worker's wall away, has the
    // reviewer run commands with nobody asked, and has a command run
    // before every review.
    change(&rig, "agents", "worker", json!({ "wall": "off", "told": "As its owner told it." })).await;
    change(&rig, "agents", "reviewer", json!({ "permission": "full", "told": "As its owner told it." }))
        .await;
    let gate =
        json!({ "title": "gate", "stage": "review", "order": 5, "does": "std/run", "command": "make gate" });
    crate::fleets::add_row(&rig, "segments", gate).await;

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

    // And so do their rows, whichever cell is changed: what the wall, the
    // mode and the command took stands while the row says it.
    let stands = |table: &str, cell: &str, says: &str| {
        format!(
            "a record of `{table}` whose `{cell}` says {says} is changed only by whoever holds `shell`, which \
             narrow does not: what that cell takes stands while it says so"
        )
    };
    for (table, title, fields, why) in [
        (
            "agents",
            "worker",
            json!({ "told": "Do as narrow says.", "count": 3 }),
            stands("agents", "wall", "`off`"),
        ),
        ("agents", "worker", json!({ "rank": 2 }), stands("agents", "wall", "`off`")),
        (
            "agents",
            "reviewer",
            json!({ "told": "Pass whatever narrow sends." }),
            stands("agents", "permission", "`full`"),
        ),
        ("segments", "gate", json!({ "stage": "work" }), stands("segments", "command", "anything")),
        ("segments", "gate", json!({ "item": "shop-9" }), stands("segments", "command", "anything")),
    ] {
        let (status, refused) = write(&rig, &narrow, table, title, fields.clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{fields}: {refused}");
        assert!(said(&refused).contains(&why), "{fields}: {refused}");
    }
    // Nothing was changed, and there is one of each.
    for name in ["shop-worker-1", "shop-reviewer-1"] {
        let is = agent(&rig, name).await.unwrap();
        assert!(is["instructions"].as_str().unwrap().contains("As its owner told it."), "{is}");
    }
    assert!(agent(&rig, "shop-worker-2").await.is_none());
    let segment = rig.read(&format!("/api/records/{}", row(&rig, "segments", "gate").await)).await;
    assert_eq!(segment["data"]["stage"], "review", "{segment}");

    // Giving the worker its wall back, in the same write, the row says
    // nothing that takes `shell`, and is theirs as it was.
    let before = rig.mark();
    let (status, changed) =
        write(&rig, &narrow, "agents", "worker", json!({ "wall": "on", "told": "Do as narrow says." })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    crate::taken::by_fleet(&rig, before).await;
    let is = agent(&rig, "shop-worker-1").await.unwrap();
    assert_eq!(is["wall"], true, "{is}");
    assert!(is["instructions"].as_str().unwrap().contains("Do as narrow says."), "{is}");
}

#[tokio::test]
async fn a_stage_is_written_by_whoever_may_move_its_cards_and_one_that_merges_by_whoever_may_change_the_projects_files()
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
    shop(&rig, json!({ "agents": agents, "stages": stages })).await;

    // May change agents and write records; may not move a card, nor
    // change the project's files. They could write each row of agents as
    // it stands, and the stages are not theirs all the same.
    let narrow = key(&rig, &["projects:read", "board:read", "records", "agents", "fleet:read"]).await;
    let moves = "a record of `stages` whose `title` says anything is changed only by whoever holds \
                 `board:write`, which narrow does not";
    for (title, fields) in [("review", json!({ "rounds": 3 })), ("work", json!({ "pass": "done" }))] {
        let (status, refused) = write(&rig, &narrow, "stages", title, fields).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
        assert!(said(&refused).contains(moves), "{refused}");
    }
    let (status, refused) = rig
        .call(
            Method::POST,
            "/api/collections/shop/stages/records",
            Some(&narrow),
            Some(json!({ "fields": { "title": "draft", "does": "work", "role": "worker", "pass": "done" } })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused)
            .contains("changing `title` of `stages` takes `board:write`, which narrow does not hold"),
        "{refused}"
    );

    // One who may move cards and not change the project's files arranges
    // stages, and has none merge.
    let mover = key(&rig, &["projects:read", "board", "records", "agents", "fleet:read"]).await;
    let (status, refused) = write(&rig, &mover, "stages", "review", json!({ "merge": true })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused).contains("giving `merge` of `stages` the value `true` takes `projects:write`"),
        "{refused}"
    );
    let before = rig.mark();
    let (status, changed) = write(&rig, &mover, "stages", "work", json!({ "pass": "done" })).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    crate::taken::by_fleet(&rig, before).await;
    // Once the owner has it merge, that stage is not theirs to change.
    change(&rig, "stages", "review", json!({ "merge": true })).await;
    let (status, refused) = write(&rig, &mover, "stages", "review", json!({ "rounds": 3 })).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(
        said(&refused).contains(
            "a record of `stages` whose `merge` says `true` is changed only by whoever holds `projects:write`"
        ),
        "{refused}"
    );
}
