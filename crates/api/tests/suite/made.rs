//! What is made anew in a project's folder: an empty folder, and a file
//! that is not there yet. Each is for whoever may write the project's
//! files, stays inside the project, and never takes the place of what is
//! there under its name.

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

async fn made(rig: &Rig, token: &str, path: &str) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/projects/shop/folders", Some(token), Some(json!({ "path": path }))).await
}

async fn new_file(rig: &Rig, token: &str, path: &str, content: &str) -> (StatusCode, Value) {
    let body = json!({ "path": path, "content": content, "new": true });
    rig.call(Method::PUT, "/api/projects/shop/file", Some(token), Some(body)).await
}

fn listed(tree: &Value, path: &str) -> Option<bool> {
    tree["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["path"] == path)
        .map(|entry| entry["dir"] == true)
}

#[tokio::test]
async fn a_folder_is_made_empty_with_the_folders_it_is_in_and_said_to_have_been_made() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let mark = rig.mark();
    let (status, entry) = made(&rig, &rig.token, "notes/2026").await;
    assert_eq!(
        (status, entry["path"].as_str(), entry["dir"].as_bool()),
        (StatusCode::OK, Some("notes/2026"), Some(true)),
        "{entry}"
    );
    let tree = rig.read("/api/projects/shop/tree").await;
    assert_eq!((listed(&tree, "notes"), listed(&tree, "notes/2026")), (Some(true), Some(true)), "{tree}");
    let said = rig.event("project.folder.made", |event| event.seq > mark).await;
    assert_eq!(
        (said.data["project"].as_str(), said.data["path"].as_str()),
        (Some("shop"), Some("notes/2026"))
    );
    assert_eq!(said.data["folders"], json!(["notes"]));

    // A file made in it, new, is written there and said to have been added.
    let (status, file) = new_file(&rig, &rig.token, "notes/2026/today.md", "# Today\n").await;
    assert_eq!((status, file["path"].as_str()), (StatusCode::OK, Some("notes/2026/today.md")), "{file}");
    assert_eq!(rig.read("/api/projects/shop/file?path=notes/2026/today.md").await["content"], "# Today\n");
}

#[tokio::test]
async fn a_name_that_is_there_already_is_refused_and_what_is_there_is_left_as_it_was() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    made(&rig, &rig.token, "docs").await;
    let kept = json!({ "path": "docs/plan.md", "content": "kept\n" });
    assert_eq!(
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(kept)).await.0,
        StatusCode::OK
    );

    // A folder where a folder is, and where a file is.
    for path in ["docs", "docs/plan.md"] {
        let (status, error) = made(&rig, &rig.token, path).await;
        let said = error["error"]["message"].as_str().unwrap_or_default();
        assert_eq!(status, StatusCode::CONFLICT, "{path}: {error}");
        assert!(said.contains("already") && said.contains("something else"), "{said}");
    }
    // A new file where a file is, and where a folder is: nothing is written over.
    for path in ["docs/plan.md", "docs"] {
        let (status, error) = new_file(&rig, &rig.token, path, "new\n").await;
        assert_eq!(status, StatusCode::CONFLICT, "{path}: {error}");
    }
    assert_eq!(rig.read("/api/projects/shop/file?path=docs/plan.md").await["content"], "kept\n");
    // Nor inside something that is a file.
    let (status, error) = made(&rig, &rig.token, "docs/plan.md/inside").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
    assert!(error["error"]["message"].as_str().unwrap().contains("which is a file"), "{error}");
}

#[tokio::test]
async fn a_folder_cannot_be_made_outside_its_project_or_in_what_git_keeps() {
    let rig = Rig::new().await;
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = std::path::PathBuf::from(project["path"].as_str().unwrap());
    for path in ["../outside", "notes/../../outside", "/tmp/outside", ".git/hooks", ".GIT/x", "", "."] {
        let (status, error) = made(&rig, &rig.token, path).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path:?}: {error}");
    }
    let (status, error) = new_file(&rig, &rig.token, "../outside.md", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
    assert!(!dir.parent().unwrap().join("outside").exists());
    assert!(!dir.parent().unwrap().join("outside.md").exists());

    // Nor through a link inside the project that leads out of it.
    #[cfg(unix)]
    {
        let away = dir.parent().unwrap().join("beside-shop");
        std::fs::create_dir_all(&away).unwrap();
        std::os::unix::fs::symlink(&away, dir.join("away")).unwrap();
        let (status, error) = made(&rig, &rig.token, "away/inside").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
        assert!(!away.join("inside").exists());
    }
}

#[tokio::test]
async fn whoever_may_not_write_a_projects_files_makes_no_folder_and_no_file() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let reader = json!({ "name": "reader", "scopes": ["projects:read"] });
    let key = rig.send("/api/credentials", reader).await["token"].as_str().unwrap().to_string();
    let (status, error) = made(&rig, &key, "notes").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{error}");
    let (status, error) = new_file(&rig, &key, "notes.md", "").await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{error}");
    let tree = rig.read("/api/projects/shop/tree").await;
    assert_eq!((listed(&tree, "notes"), listed(&tree, "notes.md")), (None, None), "{tree}");

    // A member, who writes a project's files, makes both.
    let member = json!({ "name": "member", "scopes": kernel::principal::MEMBER_SCOPES });
    let key = rig.send("/api/credentials", member).await["token"].as_str().unwrap().to_string();
    assert_eq!(made(&rig, &key, "notes").await.0, StatusCode::OK);
    assert_eq!(new_file(&rig, &key, "notes/one.md", "").await.0, StatusCode::OK);
    // And a project that is not there is said to be so.
    let (status, _) = rig
        .call(Method::POST, "/api/projects/nowhere/folders", Some(&rig.token), Some(json!({ "path": "x" })))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
