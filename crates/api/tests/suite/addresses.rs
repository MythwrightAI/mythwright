//! The client's addresses are paths (`/shop/test-runs#newest`), which the daemon answers with the client's page wherever it has
//! no route of its own: every path the client's grammar
//! makes reaches the page, none of the daemon's own is taken for one, and
//! no project is called what the daemon's own begin with
//! (`kernel::paths`).

use reqwest::{Method, StatusCode};
use serde_json::json;

use crate::Rig;
use crate::client::rig_with_client;

/// Whether what a path is answered with is the client's page.
async fn is_page(rig: &Rig, path: &str) -> (StatusCode, bool) {
    let response = reqwest::get(format!("{}{path}", rig.url)).await.unwrap();
    let status = response.status();
    (status, response.text().await.unwrap().contains("name=\"build\" content=\"one\""))
}

#[tokio::test]
async fn every_address_the_client_makes_is_its_page_and_none_of_the_daemons_own_is() {
    let (rig, client) = rig_with_client("one").await;
    std::fs::create_dir_all(client.join("lib")).unwrap();
    std::fs::write(client.join("lib/chart.js"), "/* draws charts */").unwrap();
    // One of each the grammar makes (`addressOf` in `ui/src/tab.ts`), a document and a file with a dot in their
    // last name among them, and an address written before addresses were paths.
    for path in [
        "/shop",
        "/shop/",
        "/shop/board",
        "/shop/test-runs",
        "/shop/test-runs/",
        "/shop/tables/git",
        "/shop/docs/guides/install.md",
        "/shop/repo",
        "/shop/repo/README.md",
        "/shop/repo/%40blueprints/notes.bp",
        "/shop/repo/src/a%23b%3F.rs",
        "/shop/git",
        "/shop/environments",
        "/shop/pipelines",
        "/shop/blueprints/std/board",
        "/shop/blueprints/nudge",
        "/blueprint/std/board",
        "/blueprint/std/board?project=shop",
        "/run/run_1m85442mg8j93j1c",
        "/instance/in_4f2a",
        "/inbox/inb_01h2x",
        "/reference",
        "/reference/action/collection.ensure/fields/quiet",
        // A wiki, a page of one by its wiki's title and its own, a `/` in a title, and a page by its id.
        "/topic/Handbook",
        "/topic/Handbook/Rolling%20back",
        "/topic/Ops%20%2F%20Infra/A%2FB",
        "/topic/%40-topics-3",
        "/about",
        "/",
    ] {
        assert_eq!(is_page(&rig, path).await, (StatusCode::OK, true), "{path}");
    }
    // The client's own files are its files, and one that is not there is not the page.
    assert_eq!(is_page(&rig, "/lib/chart.js").await, (StatusCode::OK, false));
    assert_eq!(is_page(&rig, "/assets/app-one.js").await, (StatusCode::OK, false));
    assert_eq!(is_page(&rig, "/assets/app-gone.js").await.0, StatusCode::NOT_FOUND);
    assert_eq!(is_page(&rig, "/lib/gone").await.0, StatusCode::NOT_FOUND);
    assert_eq!(is_page(&rig, "/favicon.ico").await.0, StatusCode::NOT_FOUND);
    // The daemon's own answer for themselves: what is asked of it, a library's pages, mockups, what is said from outside.
    assert_eq!(is_page(&rig, "/api/meta").await, (StatusCode::OK, false));
    assert_eq!(is_page(&rig, "/api/no-such-route").await, (StatusCode::NOT_FOUND, false));
    for path in ["/library/shop/docs/", "/mockups/in/nothing/", "/in/nobody"] {
        assert!(!is_page(&rig, path).await.1, "{path}");
    }
}

#[tokio::test]
async fn no_project_is_called_what_the_factorys_own_addresses_begin_with() {
    let rig = Rig::new().await;
    for word in kernel::paths::RESERVED {
        let (status, refused) =
            rig.call(Method::POST, "/api/projects", Some(&rig.token), Some(json!({ "name": word }))).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{word}: {refused}");
        let said = refused["error"]["message"].as_str().unwrap();
        assert!(
            said.contains(&format!("cannot be called `{word}`")) && said.contains("Call it something else"),
            "{said}"
        );
    }
    // By its id, whatever it is written as.
    let (status, _) =
        rig.call(Method::POST, "/api/projects", Some(&rig.token), Some(json!({ "name": "  API " }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(rig.read("/api/projects").await["projects"], json!([]));
    // A word that only begins as one is a project's like any.
    rig.send("/api/projects", json!({ "name": "Run book" })).await;
    assert_eq!(rig.read("/api/projects/run-book").await["name"], "Run book");

    // Nor is one called so afterwards, and nothing is written.
    let (status, refused) = rig
        .call(Method::PATCH, "/api/projects/run-book", Some(&rig.token), Some(json!({ "name": "Settings" })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap().contains("`/settings`"), "{refused}");
    assert_eq!(rig.read("/api/projects/run-book").await["name"], "Run book");
}
