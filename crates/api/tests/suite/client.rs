//! Replacing the client while the daemon runs.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::Event;
use reqwest::StatusCode;

use crate::Rig;

/// Puts a build of the client in `dir` the way the client's own build does:
/// the files first, then the entry page, in one step.
fn put_client(dir: &Path, build: &str) {
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("assets").join(format!("app-{build}.js")), format!("// {build}")).unwrap();
    let page = format!(
        "<!doctype html><meta name=\"build\" content=\"{build}\" data-live=\"no\" />\
         <script type=\"module\" src=\"/assets/app-{build}.js\"></script>"
    );
    std::fs::write(dir.join("index.html.new"), page).unwrap();
    std::fs::rename(dir.join("index.html.new"), dir.join("index.html")).unwrap();
}

/// A node serving a client of its own.
pub(crate) async fn rig_with_client(build: &str) -> (Rig, PathBuf) {
    let rig = Rig::with("api-client", |config, dir| {
        put_client(&dir.join("client"), build);
        config.ui_dir = Some(dir.join("client"));
    })
    .await;
    let client = rig.dir.join("client");
    (Rig::owned(rig).await, client)
}

/// Waits for the daemon to say, unasked, that the client is now `build`.
/// Where this system will not say that files changed (its limit on what
/// can be followed is used up by the tests run beside this one), the
/// daemon sees a new client when somebody next asks which build is served,
/// as every page does when it loads: so that is asked.
async fn announced(rig: &Rig, build: &str) -> Arc<Event> {
    if rig.node().app.hosting.watch().problem().is_some() {
        rig.read("/api/meta").await;
    }
    rig.event("client.updated", |event| event.data["build"] == build).await
}

async fn page(rig: &Rig, path: &str) -> (StatusCode, String) {
    let response = reqwest::get(format!("{}{path}", rig.url)).await.unwrap();
    (response.status(), response.text().await.unwrap())
}

#[tokio::test]
async fn a_new_build_of_the_client_is_served_and_announced_without_restarting_the_daemon() {
    let (rig, client) = rig_with_client("one").await;
    assert_eq!(rig.read("/api/meta").await["client_build"], "one");
    assert!(page(&rig, "/").await.1.contains("app-one.js"));
    let started =
        rig.read("/api/events/recent?topics=system.started").await["events"].as_array().unwrap().len();

    put_client(&client, "two");
    let event = announced(&rig, "two").await;
    assert_eq!(event.data["was"], "one");
    assert!(event.is_durable(), "kept, so a page that was away is told when it comes back");

    assert_eq!(rig.read("/api/meta").await["client_build"], "two");
    assert!(page(&rig, "/").await.1.contains("app-two.js"));
    // A page still on the build before can fetch the rest of itself.
    assert_eq!(page(&rig, "/assets/app-one.js").await, (StatusCode::OK, "// one".to_string()));
    let now = rig.read("/api/events/recent?topics=system.started").await["events"].as_array().unwrap().len();
    assert_eq!(now, started, "the daemon did not restart");
}

#[tokio::test]
async fn the_client_may_frame_only_what_this_daemon_serves() {
    // A view shown in a channel is in a frame. Were it to send itself
    // somewhere else, the frame must not go there.
    let (rig, client) = rig_with_client("one").await;
    std::fs::create_dir_all(client.join("lib")).unwrap();
    std::fs::write(client.join("lib/chart.js"), "/* draws charts */").unwrap();
    let policy = async |path: &str| {
        let response = rig.client.get(format!("{}{path}", rig.url)).send().await.unwrap();
        let said = response
            .headers()
            .get("content-security-policy")
            .map(|value| value.to_str().unwrap().to_string());
        (response.status(), said)
    };
    // And it fetches from nowhere else: a picture in a document somebody
    // else wrote is a request its reader's browser makes.
    let held = api::ui::CLIENT_POLICY;
    assert!(held.contains("frame-src 'self'") && held.contains("img-src 'self' data: blob:"), "{held}");
    assert!(held.contains("connect-src 'self'"), "{held}");
    assert_eq!(policy("/").await, (StatusCode::OK, Some(held.to_string())));
    assert_eq!(policy("/some/tab/of/the/client").await.1.as_deref(), Some(held));
    // What a view draws with is beside the client, for anyone: a view has nobody's session to ask with.
    assert_eq!(policy("/lib/chart.js").await, (StatusCode::OK, None));
}

#[tokio::test]
async fn only_a_change_of_client_is_announced() {
    let (rig, client) = rig_with_client("one").await;
    // Neither the client the daemon started with nor the same build put in
    // place again is news.
    put_client(&client, "one");
    assert_eq!(rig.read("/api/meta").await["client_build"], "one");
    put_client(&client, "two");
    announced(&rig, "two").await;
    let events = rig.read("/api/events/recent?topics=client.updated").await;
    let builds: Vec<&str> = events["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| event["data"]["build"].as_str())
        .collect();
    assert_eq!(builds, ["two"]);
}

#[tokio::test]
async fn a_client_replaced_while_the_daemon_was_down_is_announced_when_it_comes_back() {
    let (mut rig, client) = rig_with_client("one").await;
    let before = rig.read("/api/events/recent?limit=1").await["last_seq"].as_u64().unwrap();
    rig.stop().await;
    put_client(&client, "two");
    rig.start().await;

    // A page that was open reconnects from where it had read to, and this is
    // what it finds waiting.
    let events = rig.read(&format!("/api/events/recent?topics=client.updated&after={before}")).await;
    let events = events["events"].as_array().unwrap();
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!((&events[0]["data"]["build"], &events[0]["data"]["was"]), (&"two".into(), &"one".into()));
}

#[tokio::test]
async fn an_entry_page_caught_half_replaced_changes_nothing() {
    let (rig, client) = rig_with_client("one").await;
    // Not how the client's build writes it, but a copy made by other means
    // might: the page exists and does not yet say which build it is.
    std::fs::write(client.join("index.html"), "<!doctype html>").unwrap();
    assert_eq!(rig.read("/api/meta").await["client_build"], "one");

    put_client(&client, "two");
    assert_eq!(announced(&rig, "two").await.data["was"], "one");
}

#[tokio::test]
async fn a_client_folder_removed_and_built_again_is_still_followed() {
    let (rig, client) = rig_with_client("one").await;
    std::fs::remove_dir_all(&client).unwrap();
    put_client(&client, "two");
    assert_eq!(announced(&rig, "two").await.data["was"], "one");
    // And the folder that came back is the one being watched now.
    put_client(&client, "three");
    assert_eq!(announced(&rig, "three").await.data["was"], "two");
}
