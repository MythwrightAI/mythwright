//! A factory always has a workspace, every project is always in one,
//! and every channel is a project's or a workspace's own.
//!
//! The first is made as the factory's owner registers, called after
//! them, and a project added with nothing said is put in the workspace
//! that was made first and is still there, as a channel made for no
//! project is made its own. A factory an earlier program made, whose
//! projects were in a workspace or in none and whose channels might
//! belong to nothing, is brought to that as this program starts on it,
//! once: every project in a workspace and every such channel the usual
//! one's own, each move said, and nothing else of any project or channel
//! changed. Started again, nothing more is written or said.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::said;

async fn workspaces(rig: &Rig) -> Vec<Value> {
    rig.read("/api/workspaces").await["workspaces"].as_array().unwrap().clone()
}

/// What the factory's own file keeps of workspaces: every row of the
/// three tables, as text.
async fn rows(rig: &Rig) -> Vec<String> {
    let read = |connection: &mut rusqlite::Connection| {
        let mut all = Vec::new();
        for (table, columns) in [
            ("workspaces", "id || ' ' || name || ' ' || created"),
            ("workspace_projects", "project || ' in ' || workspace"),
            ("workspace_channels", "channel || ' of ' || workspace"),
        ] {
            let mut statement = connection.prepare(&format!("SELECT {columns} FROM {table} ORDER BY 1"))?;
            let found = statement.query_map([], |row| row.get::<_, String>(0))?;
            for one in found {
                all.push(format!("{table}: {}", one?));
            }
        }
        Ok(all)
    };
    rig.node().app.journal.db().call(read).await.unwrap()
}

/// How many events there are on each of the topics a workspace is said
/// by, of projects: one made, a project placed, one taken away.
async fn told(rig: &Rig) -> [usize; 3] {
    [
        rig.count_of("workspace.created").await,
        rig.count_of("project.placed").await,
        rig.count_of("workspace.removed").await,
    ]
}

/// What each `channel.placed` there was says: who said it, which
/// channel by name, and what it belongs to now.
async fn channels_placed(rig: &Rig) -> Vec<(String, String, String)> {
    let query = journal::EventQuery { topics: "channel.placed".into(), ..Default::default() };
    let all = rig.said(query).await;
    let text = |value: &Value| value.as_str().unwrap_or("").to_string();
    all.iter()
        .map(|event| {
            let to = match event.data["project"].as_str() {
                Some(project) => format!("project {project}"),
                None => format!("workspace {}", text(&event.data["workspace"])),
            };
            (event.source.actor.clone().unwrap_or_default(), text(&event.data["name"]), to)
        })
        .collect()
}

/// The actor the rig is signed in as.
async fn owner(rig: &Rig) -> String {
    rig.read("/api/me").await["actor"].as_str().unwrap().to_string()
}

/// Takes every workspace out of a stopped factory's file, with what was
/// in each: as a factory an earlier program made holds its projects,
/// where nobody had made a workspace.
fn as_before_any_workspace(rig: &Rig) {
    let connection = journal::copy::Connection::open(rig.config.data_dir.join("journal.db")).unwrap();
    for table in ["workspace_projects", "workspace_channels", "workspaces"] {
        connection.execute(&format!("DELETE FROM {table}"), []).unwrap();
    }
}

#[tokio::test]
async fn a_factory_has_a_workspace_called_after_its_owner_and_a_project_added_with_nothing_said_is_in_it() {
    let rig = Rig::fresh().await;
    // Nobody has registered: there is nobody to call one after.
    assert_eq!(rows(&rig).await, Vec::<String>::new());
    let rig = Rig::owned(rig).await;
    let all = workspaces(&rig).await;
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!((&all[0]["id"], &all[0]["name"]), (&json!("ada-s-workspace"), &json!("Ada's workspace")));
    assert_eq!(all[0]["projects"], json!([]));
    assert_eq!(told(&rig).await, [1, 0, 0]);

    // A channel made with nothing said is its own, and is said to be;
    // one made for a project is the project's, and no workspace's own.
    let chat = rig.send("/api/channels", json!({ "name": "chat" })).await;
    assert_eq!(workspaces(&rig).await[0]["channels"], json!([chat["id"]]));
    assert_eq!(
        channels_placed(&rig).await,
        [(owner(&rig).await, "chat".to_string(), "workspace ada-s-workspace".to_string())]
    );

    // Added with nothing said, a project is in it, and is said to be.
    let made = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(made["workspace"], "ada-s-workspace", "{made}");
    assert_eq!(workspaces(&rig).await[0]["projects"], json!(["shop"]));
    let placed = rig.event("project.placed", |event| event.data["project"] == "shop").await;
    assert_eq!(placed.data["workspace"], "ada-s-workspace");
    assert_eq!(told(&rig).await, [1, 1, 0]);

    // With one said, it is in that one and was never in another.
    rig.send("/api/workspaces", json!({ "name": "Back" })).await;
    let made = rig.send("/api/projects", json!({ "name": "Depot", "workspace": "back" })).await;
    assert_eq!(made["workspace"], "back", "{made}");
    assert_eq!(told(&rig).await, [2, 2, 0]);
    let plans = rig.send("/api/channels", json!({ "name": "plans", "workspace": "Back" })).await;
    let talk = rig.send("/api/channels", json!({ "name": "depot-talk", "project": "depot" })).await;
    let back = workspaces(&rig).await.into_iter().find(|one| one["id"] == "back").unwrap();
    assert_eq!(back["channels"], json!([plans["id"]]));
    assert_eq!((&talk["project"], channels_placed(&rig).await.len()), (&json!("depot"), 2));
    // And the usual one is still the one made first, whatever each is called.
    let made = rig.send("/api/projects", json!({ "name": "Annex" })).await;
    assert_eq!(made["workspace"], "ada-s-workspace", "{made}");
    // Somebody who registers after the owner is given no workspace.
    assert_eq!(workspaces(&rig).await.len(), 2);
}

#[tokio::test]
async fn a_project_is_always_in_a_workspace_and_the_last_workspace_stays() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // Taken out of its workspace it would be in none: refused, saying where to put it.
    let (status, refused) = rig
        .call(
            Method::PUT,
            "/api/projects/shop/workspace",
            Some(&rig.token),
            Some(json!({ "workspace": null })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("a project is always in a workspace"), "{refused}");
    assert!(said(&refused).contains("workspace put"), "{refused}");
    assert_eq!(workspaces(&rig).await[0]["projects"], json!(["shop"]));

    // The last workspace cannot be taken away.
    let (status, refused) =
        rig.call(Method::DELETE, "/api/workspaces/ada-s-workspace", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(said(&refused).contains("the only workspace"), "{refused}");
    assert!(said(&refused).contains("workspace rename"), "{refused}");

    // One that holds something goes only with where that goes.
    rig.send("/api/workspaces", json!({ "name": "Back" })).await;
    rig.send("/api/channels", json!({ "name": "plans", "workspace": "ada-s-workspace" })).await;
    let (status, refused) =
        rig.call(Method::DELETE, "/api/workspaces/ada-s-workspace", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(said(&refused).contains("--to"), "{refused}");
    let (status, refused) = rig
        .call(
            Method::DELETE,
            "/api/workspaces/ada-s-workspace",
            Some(&rig.token),
            Some(json!({ "to": "ada-s-workspace" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let (status, refused) = rig
        .call(
            Method::DELETE,
            "/api/workspaces/ada-s-workspace",
            Some(&rig.token),
            Some(json!({ "to": "nowhere" })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert_eq!(workspaces(&rig).await.len(), 2, "nothing was taken away by a request that was refused");

    let before = [rig.count_of("project.placed").await, rig.count_of("channel.placed").await];
    let (status, gone) = rig
        .call(
            Method::DELETE,
            "/api/workspaces/ada-s-workspace",
            Some(&rig.token),
            Some(json!({ "to": "back" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!((&gone["projects"], &gone["to"]), (&json!(["shop"]), &json!("back")), "{gone}");
    let all = workspaces(&rig).await;
    assert_eq!(all.len(), 1);
    let plans = rig.read("/api/channels/plans").await;
    assert_eq!((&all[0]["projects"], &all[0]["channels"]), (&json!(["shop"]), &json!([plans["id"]])));
    // Each thing that moved is said to have, once.
    let moved = rig.event("project.placed", |event| event.data["was"] == "ada-s-workspace").await;
    assert_eq!((&moved.data["project"], &moved.data["workspace"]), (&json!("shop"), &json!("back")));
    let after = [rig.count_of("project.placed").await, rig.count_of("channel.placed").await];
    assert_eq!([after[0] - before[0], after[1] - before[1]], [1, 1]);
    // And the one that is left is where a project goes when nobody says.
    let made = rig.send("/api/projects", json!({ "name": "Depot" })).await;
    assert_eq!(made["workspace"], "back", "{made}");
    // An empty one is taken away as it always was.
    rig.send("/api/workspaces", json!({ "name": "Attic" })).await;
    let (status, gone) = rig.call(Method::DELETE, "/api/workspaces/attic", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
}

#[tokio::test]
async fn a_factory_from_before_is_started_with_every_project_and_channel_in_a_workspace_and_started_again_nothing_more()
 {
    let mut rig = Rig::new().await;
    for name in ["Shop", "Yard"] {
        rig.send("/api/projects", json!({ "name": name })).await;
    }
    rig.send("/api/channels", json!({ "name": "shop-talk", "project": "shop" })).await;
    rig.send("/api/channels", json!({ "name": "chat" })).await;
    let projects = rig.read("/api/projects").await;
    let channels = rig.read("/api/channels").await;
    rig.stop().await;
    as_before_any_workspace(&rig);

    rig.start().await;
    let all = workspaces(&rig).await;
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!((&all[0]["name"], &all[0]["projects"]), (&json!("Ada's workspace"), &json!(["shop", "yard"])));
    // The channel that belonged to nothing is the workspace's own, said
    // once as the system; the project's is its project's, with nothing said.
    let chat = channels["channels"].as_array().unwrap().iter().find(|one| one["name"] == "chat").unwrap();
    assert_eq!(all[0]["channels"], json!([chat["id"]]));
    let moved: Vec<_> = channels_placed(&rig).await.into_iter().filter(|(by, ..)| by == "system").collect();
    assert_eq!(moved, [("system".to_string(), "chat".to_string(), "workspace ada-s-workspace".to_string())]);
    // Nothing else of a project or a channel changed.
    assert_eq!(rig.read("/api/projects").await, projects);
    assert_eq!(rig.read("/api/channels").await, channels);
    // Each move was said, once, by the system.
    let query = journal::EventQuery { topics: "project.placed".into(), ..Default::default() };
    let placed = rig.said(query).await;
    let late: Vec<_> =
        placed.iter().filter(|event| event.source.actor.as_deref() == Some("system")).collect();
    let which: Vec<&str> = late.iter().map(|event| event.data["project"].as_str().unwrap()).collect();
    assert_eq!(which, ["shop", "yard"]);
    assert!(late.iter().all(|event| event.data["workspace"] == all[0]["id"]));

    // Started again, there is nothing to bring about: no row is written and nothing is said.
    let (kept, counted, said) = (rows(&rig).await, told(&rig).await, channels_placed(&rig).await);
    rig.restart().await;
    assert_eq!(rows(&rig).await, kept);
    assert_eq!(told(&rig).await, counted);
    assert_eq!(channels_placed(&rig).await, said);
    assert_eq!(rig.read("/api/projects").await, projects);
    assert_eq!(rig.read("/api/channels").await, channels);
}

/// Every topic said since `mark`, once the factory is at rest, in order of name.
async fn said_since(rig: &Rig, mark: u64) -> Vec<String> {
    crate::simulation::rested(rig).await;
    let query = journal::EventQuery { after: mark, limit: 100_000, ..Default::default() };
    let mut topics: Vec<String> = rig.said(query).await.into_iter().map(|event| event.topic).collect();
    topics.sort();
    topics
}

#[tokio::test]
async fn a_start_that_puts_projects_in_a_workspace_says_nothing_else_of_them() {
    let mut rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig.send("/api/board/shop/items", json!({ "title": "a card from before" })).await;
    rig.send("/api/channels", json!({ "name": "chat" })).await;
    rig.send("/api/channels/chat/messages", json!({ "body": "said before" })).await;
    crate::simulation::rested(&rig).await;
    // What a start says of a factory whose every project is in a workspace.
    rig.stop().await;
    let mark = rig.mark();
    rig.start().await;
    let plain = said_since(&rig, mark).await;
    assert!(plain.contains(&"system.started".to_string()), "{plain:?}");

    // And of one whose projects are in none and whose channel belongs
    // to nothing: the same, and that a workspace was made and the
    // project and the channel put in it. Nothing of the project's own
    // (its board, its card) or of the channel's (what was said there) is
    // said late or again for it.
    rig.stop().await;
    as_before_any_workspace(&rig);
    let mark = rig.mark();
    rig.start().await;
    let mut settled = said_since(&rig, mark).await;
    for topic in ["channel.placed", "project.placed", "workspace.created"] {
        let at = settled.iter().position(|one| one == topic).unwrap_or_else(|| panic!("no {topic}"));
        settled.remove(at);
    }
    assert_eq!(settled, plain);
}
