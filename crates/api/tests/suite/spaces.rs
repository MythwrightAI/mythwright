//! Workspaces by the routes: a name for the projects that belong together
//! and for the channels that are of no one project of them. Grouping and
//! nothing else: nothing here lets anybody in or keeps anybody out. A
//! factory always has one, called after its owner to begin with, and
//! every project is in one, and every channel is a project's or a
//! workspace's own (`settled.rs` holds how that comes about).

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::{key, said};

async fn workspaces(rig: &Rig) -> Vec<Value> {
    rig.read("/api/workspaces").await["workspaces"].as_array().unwrap().clone()
}

async fn workspace(rig: &Rig, id: &str) -> Value {
    workspaces(rig)
        .await
        .into_iter()
        .find(|one| one["id"] == id)
        .unwrap_or_else(|| panic!("no workspace {id}"))
}

async fn put(rig: &Rig, token: &str, path: &str, body: Value) -> (StatusCode, Value) {
    rig.call(Method::PUT, path, Some(token), Some(body)).await
}

/// The owner's workspace, which every rig has and is the usual one, and
/// two more; a project in each, the one nothing was said of in the usual
/// one; a channel of a project, one of a workspace that was said, and
/// one nothing was said of, which is the usual workspace's own.
const USUAL: &str = "ada-s-workspace";

async fn laid_out(rig: &Rig) {
    rig.send("/api/workspaces", json!({ "name": "Front" })).await;
    rig.send("/api/workspaces", json!({ "name": "Back" })).await;
    rig.send("/api/projects", json!({ "name": "Shop", "workspace": "front" })).await;
    rig.send("/api/projects", json!({ "name": "Depot", "workspace": "back" })).await;
    rig.send("/api/projects", json!({ "name": "Shed" })).await;
    rig.send("/api/channels", json!({ "name": "shop-talk", "project": "shop" })).await;
    rig.send("/api/channels", json!({ "name": "plans", "workspace": "front" })).await;
    rig.send("/api/channels", json!({ "name": "chat" })).await;
}

#[tokio::test]
async fn a_factory_with_its_owners_workspace_alone_answers_for_its_projects_and_channels_as_it_always_did() {
    let rig = Rig::new().await;
    let made = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let talk = rig.send("/api/channels", json!({ "name": "shop-talk", "project": "shop" })).await;
    let all = workspaces(&rig).await;
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(
        (&all[0]["id"], &all[0]["usual"], &all[0]["projects"]),
        (&json!(USUAL), &json!(true), &json!(["shop"]))
    );
    assert_eq!(rig.read("/api/workspaces").await["usual"], USUAL);
    // Nothing new is said of a project or a channel where they are
    // listed: what a project says is its own (whose its folder is, and
    // whether it is a repository, among it), and a project that is added
    // says where it was put.
    let fields = |one: &Value| one.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    assert_eq!(
        fields(&made),
        ["brownfield", "created", "folder", "id", "name", "path", "repository", "trusted", "workspace"]
    );
    let listed = rig.read("/api/projects").await;
    assert_eq!(
        fields(&listed["projects"][0]),
        ["brownfield", "created", "folder", "id", "name", "path", "repository", "trusted"]
    );
    assert_eq!(fields(&talk), ["created", "id", "members", "name", "project", "topic"]);
    let channels = rig.read("/api/channels").await;
    assert_eq!(fields(&channels["channels"][0]), ["created", "id", "members", "name", "project", "topic"]);
    // The workspace made for the owner and the project put in it, and nothing of a channel.
    assert_eq!(rig.count_of("workspace.*").await, 1);
    assert_eq!(rig.count_of("project.placed").await, 1);
    assert_eq!(rig.count_of("channel.placed").await, 0);
}

#[tokio::test]
async fn workspaces_are_made_filled_called_something_else_and_taken_away_and_each_is_said() {
    let rig = Rig::new().await;
    laid_out(&rig).await;

    // By name, each with what is in it; a channel of a workspace's
    // project is the project's and not the workspace's own.
    let all = workspaces(&rig).await;
    let ids: Vec<&str> = all.iter().map(|one| one["id"].as_str().unwrap()).collect();
    assert_eq!(ids, [USUAL, "back", "front"]);
    let plans = rig.read("/api/channels/plans").await;
    assert_eq!(all[2]["projects"], json!(["shop"]));
    assert_eq!(all[2]["channels"], json!([plans["id"]]));
    // A project nothing was said of is in the usual one, and a channel
    // nothing was said of is its own.
    assert_eq!(all[0]["projects"], json!(["shed"]));
    assert_eq!(all[0]["channels"], json!([rig.read("/api/channels/chat").await["id"]]));
    assert_eq!(plans["project"], Value::Null);
    assert_eq!(rig.read("/api/channels/shop-talk").await["project"], "shop");
    // One with nothing in it is listed.
    rig.send("/api/workspaces", json!({ "name": "Attic" })).await;
    let attic = workspace(&rig, "attic").await;
    assert_eq!((&attic["projects"], &attic["channels"]), (&json!([]), &json!([])));

    // A project is put in another, and nothing else of it moves. It is
    // never taken out of all of them.
    let before = rig.read("/api/projects/shed").await;
    let (status, placed) =
        put(&rig, &rig.token, "/api/projects/shed/workspace", json!({ "workspace": "back" })).await;
    assert_eq!((status, &placed["workspace"]), (StatusCode::OK, &json!("back")), "{placed}");
    assert_eq!(workspace(&rig, "back").await["projects"], json!(["depot", "shed"]));
    assert_eq!(rig.read("/api/projects/shed").await, before);
    assert_eq!(workspace(&rig, USUAL).await["projects"], json!([]));
    let (status, refused) =
        put(&rig, &rig.token, "/api/projects/shed/workspace", json!({ "workspace": null })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("a project is always in a workspace"), "{refused}");
    assert_eq!(workspace(&rig, "back").await["projects"], json!(["depot", "shed"]));
    let (status, unsaid) = put(&rig, &rig.token, "/api/projects/shed/workspace", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{unsaid}");

    // A channel is a project's or a workspace's and never both: said, and
    // changed afterwards.
    let (status, both) = put(
        &rig,
        &rig.token,
        "/api/channels/chat/belongs",
        json!({ "project": "shed", "workspace": "back" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{both}");
    assert!(said(&both).contains("to a project or to a workspace, and not to both"), "{both}");
    let (_, now) = put(&rig, &rig.token, "/api/channels/chat/belongs", json!({ "workspace": "back" })).await;
    assert_eq!((&now["project"], &now["workspace"]), (&Value::Null, &json!("back")), "{now}");
    let (_, now) = put(&rig, &rig.token, "/api/channels/chat/belongs", json!({ "project": "shed" })).await;
    assert_eq!((&now["project"], &now["workspace"]), (&json!("shed"), &Value::Null), "{now}");
    assert!(workspace(&rig, "back").await["channels"].as_array().unwrap().is_empty());
    // Belonging to nothing is refused: it is the project's still.
    let (status, nothing) = put(&rig, &rig.token, "/api/channels/chat/belongs", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{nothing}");
    assert!(
        said(&nothing).contains("a channel is a project's or a workspace's, and never nobody's"),
        "{nothing}"
    );
    assert_eq!(rig.read("/api/channels/chat").await["project"], "shed");

    // Called something else, its id is what it was.
    let (status, named) = rig
        .call(
            Method::PATCH,
            "/api/workspaces/front",
            Some(&rig.token),
            Some(json!({ "name": "The shop floor" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{named}");
    assert_eq!((&named["id"], &named["name"]), (&json!("front"), &json!("The shop floor")));
    assert_eq!(workspace(&rig, "front").await["projects"], json!(["shop"]));

    // Taken away, what was in it is in the workspace that was said and
    // is otherwise as it was.
    let to = Some(json!({ "to": "Back" }));
    let (status, gone) = rig.call(Method::DELETE, "/api/workspaces/front", Some(&rig.token), to).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!((&gone["projects"], &gone["to"]), (&json!(["shop"]), &json!("back")));
    assert!(workspaces(&rig).await.iter().all(|one| one["id"] != "front"));
    let back = workspace(&rig, "back").await;
    assert_eq!(
        (&back["projects"], &back["channels"]),
        (&json!(["depot", "shed", "shop"]), &json!([plans["id"]]))
    );
    assert_eq!(rig.read("/api/projects/shop").await["id"], "shop");
    assert_eq!(rig.read("/api/channels/plans").await["id"], plans["id"]);
    let (status, missing) = rig.get("/api/workspaces/front").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");

    // Each thing done was said, once.
    // (The owner's workspace and three more; three projects made, one
    // put in another and one moved with its workspace; two channels made
    // a workspace's own, one of them said to be another's twice, and one
    // moved with its workspace.)
    assert_eq!(rig.count_of("workspace.created").await, 4);
    assert_eq!(rig.count_of("workspace.renamed").await, 1);
    assert_eq!(rig.count_of("workspace.removed").await, 1);
    assert_eq!(rig.count_of("project.placed").await, 5);
    assert_eq!(rig.count_of("channel.placed").await, 5);
}

#[tokio::test]
async fn nothing_is_made_for_a_workspace_there_is_not() {
    let rig = Rig::new().await;
    let (status, refused) =
        rig.post("/api/projects", json!({ "name": "Shop", "workspace": "nowhere" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert!(said(&refused).contains("there is no workspace `nowhere`"), "{refused}");
    assert_eq!(rig.read("/api/projects").await["projects"], json!([]));
    let (status, refused) =
        rig.post("/api/channels", json!({ "name": "plans", "workspace": "nowhere" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    // Nor for a project there is not: a channel is a project's or a
    // workspace's, and one that said a project nobody has would be neither.
    let (status, refused) = rig.post("/api/channels", json!({ "name": "plans", "project": "nowhere" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert!(said(&refused).contains("there is no project `nowhere`"), "{refused}");
    rig.send("/api/workspaces", json!({ "name": "Front" })).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let both = json!({ "name": "plans", "project": "shop", "workspace": "front" });
    let (status, refused) = rig.post("/api/channels", both).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert_eq!(rig.read("/api/channels").await["channels"], json!([]));
}

#[tokio::test]
async fn a_key_that_holds_less_makes_and_fills_no_workspace_and_says_what_no_channel_belongs_to() {
    let rig = Rig::new().await;
    laid_out(&rig).await;
    // Reads everything, and changes neither projects nor channels.
    let reader = key(&rig, &["projects:read", "channels:read", "board", "records"]).await;
    let asks: [(Method, &str, Value); 8] = [
        (Method::POST, "/api/workspaces", json!({ "name": "Mine" })),
        (Method::PATCH, "/api/workspaces/front", json!({ "name": "Mine" })),
        (Method::DELETE, "/api/workspaces/front", Value::Null),
        (Method::DELETE, "/api/workspaces/front", json!({ "to": "back" })),
        (Method::PUT, "/api/projects/shed/workspace", json!({ "workspace": "front" })),
        (Method::PUT, "/api/projects/shed/workspace", json!({ "workspace": null })),
        (Method::POST, "/api/projects", json!({ "name": "Other", "workspace": "front" })),
        (Method::PUT, "/api/channels/chat/belongs", json!({ "workspace": "front" })),
    ];
    for (method, path, body) in asks {
        let body = (!body.is_null()).then_some(body);
        let (status, refused) = rig.call(method.clone(), path, Some(&reader), body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {refused}");
        let needs = if path.starts_with("/api/channels") { "channels:write" } else { "projects:write" };
        assert!(said(&refused).contains(needs), "{method} {path}: {refused}");
    }
    // And is not told by the refusal whether there is such a workspace.
    let (status, refused) = rig
        .call(
            Method::POST,
            "/api/projects",
            Some(&reader),
            Some(json!({ "name": "O", "workspace": "nowhere" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, refused) = rig
        .call(
            Method::POST,
            "/api/channels",
            Some(&reader),
            Some(json!({ "name": "o", "workspace": "nowhere" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Nothing moved.
    assert_eq!(workspace(&rig, "front").await["name"], "Front");
    assert_eq!(workspace(&rig, "front").await["projects"], json!(["shop"]));
    assert_eq!(workspace(&rig, USUAL).await["projects"], json!(["shed"]));
    assert_eq!(workspaces(&rig).await.len(), 3);
    // And one that may add a project, and nothing more, does each of
    // them: placing, adding and removing take what they always took.
    let adder = key(&rig, &["projects"]).await;
    let (status, placed) =
        put(&rig, &adder, "/api/projects/shed/workspace", json!({ "workspace": "back" })).await;
    assert_eq!(status, StatusCode::OK, "{placed}");
    let to = Some(json!({ "to": USUAL }));
    let (status, gone) = rig.call(Method::DELETE, "/api/workspaces/back", Some(&adder), to).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!(workspace(&rig, USUAL).await["projects"], json!(["depot", "shed"]));
    // Taking away one that holds a channel of its own would move the
    // channel: that is asked what placing the channel by itself asks, so
    // a key that may add a project and may not change a channel is
    // refused both in the same words, and nothing is moved.
    let to = Some(json!({ "to": USUAL }));
    let (status, by_placing) =
        put(&rig, &adder, "/api/channels/plans/belongs", json!({ "workspace": USUAL })).await;
    let (removing, by_removing) =
        rig.call(Method::DELETE, "/api/workspaces/front", Some(&adder), to.clone()).await;
    assert_eq!((status, removing), (StatusCode::FORBIDDEN, StatusCode::FORBIDDEN), "{by_removing}");
    assert_eq!(said(&by_removing), said(&by_placing));
    assert!(said(&by_removing).contains("channels:write"), "{by_removing}");
    let front = workspace(&rig, "front").await;
    assert_eq!((&front["projects"], front["channels"].as_array().unwrap().len()), (&json!(["shop"]), 1));
    assert_eq!(rig.count_of("workspace.removed").await, 1);
    // One that may do both takes it away, with its channel.
    let both = key(&rig, &["projects", "channels"]).await;
    let (status, gone) = rig.call(Method::DELETE, "/api/workspaces/front", Some(&both), to).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!(workspace(&rig, USUAL).await["channels"].as_array().unwrap().len(), 2);
    assert_eq!(workspaces(&rig).await.len(), 1);
    // It may read them, as it may read projects.
    let (status, read) = rig.call(Method::GET, "/api/workspaces", Some(&reader), None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    let blind = key(&rig, &["board"]).await;
    let (status, refused) = rig.call(Method::GET, "/api/workspaces", Some(&blind), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
}

#[tokio::test]
async fn a_workspace_lets_nobody_in_and_keeps_nobody_out() {
    let rig = Rig::new().await;
    laid_out(&rig).await;
    for channel in ["shop-talk", "plans", "chat"] {
        rig.send(
            &format!("/api/channels/{channel}/messages"),
            json!({ "body": format!("said in {channel}") }),
        )
        .await;
    }
    // Whoever may read channels reads those of every workspace and of
    // none, and whoever may read projects sees every project: with
    // workspaces there as before there were any.
    let reader = key(&rig, &["channels:read", "projects:read"]).await;
    for channel in ["shop-talk", "plans", "chat"] {
        let (status, read) =
            rig.call(Method::GET, &format!("/api/channels/{channel}/messages"), Some(&reader), None).await;
        assert_eq!(status, StatusCode::OK, "{read}");
        assert_eq!(read["messages"][0]["body"], format!("said in {channel}"));
    }
    let (_, projects) = rig.call(Method::GET, "/api/projects", Some(&reader), None).await;
    assert_eq!(projects["projects"].as_array().unwrap().len(), 3, "{projects}");
    // And whoever may write in one writes in any.
    let writer = key(&rig, &["channels"]).await;
    let (status, posted) = rig
        .call(Method::POST, "/api/channels/plans/messages", Some(&writer), Some(json!({ "body": "and I" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{posted}");
}

#[tokio::test]
async fn a_project_forgotten_leaves_its_channels_to_its_workspace_and_what_is_made_again_under_a_name_is_placed_anew()
 {
    let rig = Rig::new().await;
    laid_out(&rig).await;
    let talk = rig.read("/api/channels/shop-talk").await;
    let mark = rig.mark();
    let (status, gone) = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    let (status, gone) = rig.call(Method::DELETE, "/api/channels/plans", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    // The project's channel is the own of the workspace the project was
    // in, with what was said of that, and is otherwise as it was.
    let front = workspace(&rig, "front").await;
    assert_eq!((&front["projects"], &front["channels"]), (&json!([]), &json!([talk["id"]])));
    let now = rig.read("/api/channels/shop-talk").await;
    assert_eq!((&now["project"], &now["id"], &now["members"]), (&Value::Null, &talk["id"], &talk["members"]));
    let moved = rig.event("channel.placed", move |event| event.seq > mark).await;
    assert_eq!(
        (&moved.data["channel"], &moved.data["workspace"], &moved.data["project"]),
        (&talk["id"], &json!("front"), &Value::Null)
    );
    // One made again under the name is not where the one before it was:
    // the project is in the usual workspace, as any that nothing is said
    // of, and the channel is the usual workspace's own.
    let made = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(made["workspace"], USUAL);
    let plans = rig.send("/api/channels", json!({ "name": "plans" })).await;
    let front = workspace(&rig, "front").await;
    assert_eq!((&front["projects"], &front["channels"]), (&json!([]), &json!([talk["id"]])));
    let usual = workspace(&rig, USUAL).await;
    assert_eq!(usual["projects"], json!(["shed", "shop"]));
    assert!(usual["channels"].as_array().unwrap().contains(&plans["id"]), "{usual}");
}

/// What the bot answered to the message `id`.
async fn answer_to(rig: &Rig, id: &str) -> String {
    let id = id.to_string();
    let answer = rig
        .event("channel.message", move |event| {
            event.data["reply_to"] == id.as_str() && event.data["author"] == "bot:conductor"
        })
        .await;
    answer.data["body"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn in_a_workspaces_channel_the_bot_is_the_factorys_and_in_a_projects_it_is_that_projects() {
    let rig = Rig::new().await;
    laid_out(&rig).await;
    let ask = |channel: &'static str| {
        let rig = &rig;
        async move {
            let message = rig
                .send(&format!("/api/channels/{channel}/messages"), json!({ "body": "@conductor help" }))
                .await;
            answer_to(rig, message["id"].as_str().unwrap()).await
        }
    };
    assert!(ask("shop-talk").await.starts_with("This is the channel of `shop`"));
    assert!(ask("plans").await.starts_with("I take these"));
    // Made a workspace's, a project's channel is the project's no more.
    let (status, now) =
        put(&rig, &rig.token, "/api/channels/shop-talk/belongs", json!({ "workspace": "front" })).await;
    assert_eq!(status, StatusCode::OK, "{now}");
    assert!(ask("shop-talk").await.starts_with("I take these"));
}

#[tokio::test]
async fn a_projects_own_channel_stays_its_own_while_its_fleet_reports_there() {
    let (rig, _at) = crate::fleets::rig().await;
    crate::fleets::shop(&rig, json!({})).await;
    // A factory whose owner has added one project with a fleet has one
    // channel: the project's, where its fleet reports. It is of the
    // owner's workspace through its project, and no workspace's own.
    let channels = rig.read("/api/channels").await;
    let named: Vec<&str> =
        channels["channels"].as_array().unwrap().iter().map(|one| one["name"].as_str().unwrap()).collect();
    assert_eq!(named, ["shop"]);
    let only = workspaces(&rig).await;
    assert_eq!(only.len(), 1, "{only:?}");
    assert_eq!(
        (&only[0]["id"], &only[0]["projects"], &only[0]["channels"]),
        (&json!(USUAL), &json!(["shop"]), &json!([]))
    );
    rig.send("/api/workspaces", json!({ "name": "Front" })).await;
    assert_eq!(rig.read("/api/channels/shop").await["project"], "shop");

    let (status, refused) =
        put(&rig, &rig.token, "/api/channels/shop/belongs", json!({ "workspace": "front" })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    let why = said(&refused);
    assert!(why.starts_with("#shop is the channel of `shop`: "), "{why}");
    assert!(why.contains("is running for that project and posts there. Stop it first"), "{why}");
    // And belonging to nothing is refused whatever runs.
    let (status, refused) = put(&rig, &rig.token, "/api/channels/shop/belongs", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    // Saying what it already is, is no change, and is not refused.
    let (status, same) =
        put(&rig, &rig.token, "/api/channels/shop/belongs", json!({ "project": "shop" })).await;
    assert_eq!(status, StatusCode::OK, "{same}");
    assert_eq!(rig.read("/api/channels/shop").await["project"], "shop");
    assert_eq!(rig.count_of("channel.placed").await, 0);

    // With what posts there stopped, it is said as any channel's is.
    let all = rig.read("/api/instances?project=shop").await;
    for one in all["instances"].as_array().unwrap().iter().filter(|one| one["parent"].is_null()) {
        rig.send(&format!("/api/instances/{}/stop", one["id"].as_str().unwrap()), json!({})).await;
    }
    let (status, now) =
        put(&rig, &rig.token, "/api/channels/shop/belongs", json!({ "workspace": "front" })).await;
    assert_eq!(status, StatusCode::OK, "{now}");
    assert_eq!((&now["project"], &now["workspace"]), (&Value::Null, &json!("front")), "{now}");
}
