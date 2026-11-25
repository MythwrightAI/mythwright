//! A library (`std/docs`): a project's folder of Markdown files as
//! documents, over HTTP, and the same library as pages.

use std::path::{Path, PathBuf};

use reqwest::{Method, StatusCode, header};
use serde_json::{Value, json};

use crate::Rig;

fn write(dir: &Path, path: &str, text: &str) {
    let file = dir.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

/// A project `shop` whose `docs` folder holds only what the test puts in
/// it, and where that folder is.
async fn shop(rig: &Rig) -> PathBuf {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = rig.node().app.studio.projects.dir_of("shop").unwrap();
    // A new project has a page or two of its own to begin with.
    let _ = std::fs::remove_dir_all(dir.join("docs"));
    dir
}

async fn library(rig: &Rig) {
    let asked = json!({ "blueprint": "std/docs", "project": "shop", "wait": true });
    let applied = rig.send("/api/apply", asked).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

async fn ids(rig: &Rig) -> Vec<String> {
    let records = rig.read("/api/collections/shop/docs/records").await;
    records["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["id"].as_str().unwrap().to_string())
        .collect()
}

/// Waits until `said` has been written into the history of `record`.
async fn kept(rig: &Rig, record: &str, said: &str) {
    rig.event("collection.history", |event| event.data["record"] == record && event.data["said"] == said)
        .await;
}

/// Waits until the library has been told of a change to `path`. Where
/// this system will not say that files changed (its limit on what can be
/// followed is used up by the tests run beside this one), the project's
/// files are looked over as they are after a run, which finds it too.
async fn noticed(rig: &Rig, path: &'static str, change: &'static str) {
    if rig.node().app.hosting.watch().problem().is_some() {
        rig.send("/api/projects/shop/scan", json!({})).await;
    }
    rig.event("project.file.changed", |event| event.data["path"] == path && event.data["change"] == change)
        .await;
}

/// A page of the library, as a browser that does not follow where it is
/// sent would get it: the status, a header, and what came.
async fn fetch(
    rig: &Rig,
    path: &str,
    token: Option<&str>,
    wanted: header::HeaderName,
) -> (StatusCode, String, String) {
    let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
    let mut request = client.get(format!("{}{path}", rig.url));
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request.send().await.unwrap();
    let status = response.status();
    let header =
        response.headers().get(&wanted).and_then(|value| value.to_str().ok()).unwrap_or_default().to_string();
    (status, header, response.text().await.unwrap())
}

async fn page(rig: &Rig, path: &str) -> String {
    let (status, kind, text) = fetch(rig, path, Some(&rig.token), header::CONTENT_TYPE).await;
    assert_eq!(status, StatusCode::OK, "GET {path}: {text}");
    assert_eq!(kind, "text/html; charset=utf-8", "GET {path}");
    text
}

#[tokio::test]
async fn std_docs_makes_a_library_of_a_projects_folder_and_a_document_is_written_changed_spoken_on_and_removed_in_it()
 {
    let rig = Rig::new().await;
    let dir = shop(&rig).await;
    write(&dir, "docs/welcome.md", "# Welcome\n\nStart here.\n");
    write(&dir, "docs/guides/install.md", "---\ntags: [how-to]\n---\n\n# Installing it\n");
    write(&dir, "docs/notes.md", "No heading at all.\n");
    write(&dir, "docs/data.csv", "a,b\n");
    library(&rig).await;

    assert_eq!(ids(&rig).await, ["shop-docs:guides:install", "shop-docs:notes", "shop-docs:welcome"]);
    let declared = rig.read("/api/collections?project=shop").await;
    let docs = declared["collections"].as_array().unwrap().iter().find(|one| one["name"] == "docs").unwrap();
    assert_eq!((docs["store"].as_str(), docs["at"].as_str()), (Some("documents"), Some("docs")));
    let install = rig.read("/api/records/shop-docs:guides:install").await;
    assert_eq!(install["data"]["title"], "Installing it");
    assert_eq!(install["data"]["tags"], json!(["how-to"]));
    assert_eq!(rig.read("/api/records/shop-docs:notes").await["data"]["title"], "notes");

    // Written through the daemon, it is a file; changed, the file is.
    let made =
        json!({ "fields": { "title": "Release notes", "body": "Nothing yet.", "path": "guides/releases" } });
    let made = rig.send("/api/collections/shop/docs/records", made).await;
    let id = made["id"].as_str().unwrap();
    assert_eq!(id, "shop-docs:guides:releases");
    assert_eq!(
        std::fs::read_to_string(dir.join("docs/guides/releases.md")).unwrap(),
        "# Release notes\n\nNothing yet.\n"
    );
    let change = json!({ "fields": { "body": "The first one.", "tags": ["news"] } });
    let (status, changed) =
        rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(&rig.token), Some(change)).await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    assert_eq!(
        std::fs::read_to_string(dir.join("docs/guides/releases.md")).unwrap(),
        "---\ntags: [\"news\"]\n---\n\n# Release notes\n\nThe first one.\n"
    );
    rig.send(&format!("/api/records/{id}/comments"), json!({ "body": "Is this the place for 0.2?" })).await;

    // Edited as a file by someone signed in, it is the same document's change.
    let edited = json!({ "path": "docs/guides/releases.md", "content": "# Releases\n\nThe first one.\n" });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(edited)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    kept(&rig, id, "Ada changed its tags, title").await;

    let (status, answer) =
        rig.call(Method::DELETE, &format!("/api/records/{id}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert!(!dir.join("docs/guides/releases.md").exists());
    kept(&rig, id, "Ada took it away").await;
    let history = rig.read(&format!("/api/records/{id}/history")).await;
    let said: Vec<&str> =
        history["history"].as_array().unwrap().iter().map(|line| line["said"].as_str().unwrap()).collect();
    assert_eq!(
        said,
        [
            "Ada wrote it",
            "Ada changed its body, tags",
            "Ada said: Is this the place for 0.2?",
            "Ada changed its tags, title",
            "Ada took it away"
        ]
    );
    assert!(!ids(&rig).await.contains(&id.to_string()));
}

#[tokio::test]
async fn a_document_another_program_wrote_is_in_the_library_without_anyone_asking_and_is_the_same_one_after_a_restart()
 {
    let mut rig = Rig::new().await;
    let dir = shop(&rig).await;
    write(&dir, "docs/welcome.md", "# Welcome\n");
    library(&rig).await;
    assert_eq!(ids(&rig).await, ["shop-docs:welcome"]);

    // Something else writes into the folder: an editor, an agent, a merge.
    write(&dir, "docs/later.md", "# Written later\n");
    noticed(&rig, "docs/later.md", "added").await;
    assert_eq!(ids(&rig).await, ["shop-docs:later", "shop-docs:welcome"]);
    // In a folder that was not there when the library was made, too.
    write(&dir, "docs/deep/down/found.md", "# Found\n");
    noticed(&rig, "docs/deep/down/found.md", "added").await;
    std::fs::remove_file(dir.join("docs/later.md")).unwrap();
    noticed(&rig, "docs/later.md", "removed").await;
    assert_eq!(ids(&rig).await, ["shop-docs:deep:down:found", "shop-docs:welcome"]);

    // What was said on one is still said of it when everything is started again.
    rig.send("/api/records/shop-docs:welcome/comments", json!({ "body": "Kept." })).await;
    kept(&rig, "shop-docs:welcome", "Ada said: Kept.").await;
    rig.restart().await;
    assert_eq!(ids(&rig).await, ["shop-docs:deep:down:found", "shop-docs:welcome"]);
    let history = rig.read("/api/records/shop-docs:welcome/history").await;
    assert_eq!(history["history"][0]["said"], "Ada said: Kept.");
    // And the folder is followed again.
    write(&dir, "docs/after.md", "# After the restart\n");
    noticed(&rig, "docs/after.md", "added").await;
}

#[tokio::test]
async fn a_library_is_served_as_pages_with_its_links_and_pictures_and_nothing_outside_its_folder() {
    let rig = Rig::new().await;
    let dir = shop(&rig).await;
    write(
        &dir,
        "docs/welcome.md",
        "# Welcome\n\nSee [how to install](guides/install.md).\n\n<script>alert(1)</script>\n",
    );
    write(
        &dir,
        "docs/guides/install.md",
        "# Installing it\n\n![the box](box.png)\n\nBack to [the start](../welcome.md).\n",
    );
    write(&dir, "docs/guides/box.png", "PNG");
    write(&dir, "docs/notes with spaces.md", "Words.\n");
    write(&dir, "secret.md", "# Not the library's\n");
    library(&rig).await;

    // The library's address is a folder's.
    let (status, to, _) = fetch(&rig, "/library/shop/docs", Some(&rig.token), header::LOCATION).await;
    assert_eq!((status, to.as_str()), (StatusCode::TEMPORARY_REDIRECT, "/library/shop/docs/"));

    // With no front page of its own, the front page is what is in it, by folder.
    let front = page(&rig, "/library/shop/docs/").await;
    assert!(front.contains("<h1>Shop docs</h1>"), "{front}");
    assert!(front.contains("<h2>guides</h2>"), "{front}");
    assert!(front.contains("<a href=\"guides/install.md\">Installing it</a>"), "{front}");
    assert!(front.contains("<a href=\"notes%20with%20spaces.md\">notes with spaces</a>"), "{front}");
    assert!(front.contains("<a href=\"welcome.md\">Welcome</a>"), "{front}");

    // A document is the page publishing would make of it, and what it
    // links to is where the link says.
    let welcome = page(&rig, "/library/shop/docs/welcome.md").await;
    assert!(welcome.contains("<title>Welcome</title>") && welcome.contains("<h1>Welcome</h1>"), "{welcome}");
    assert!(welcome.contains("<a href=\"guides/install.md\">how to install</a>"), "{welcome}");
    assert!(welcome.contains("<a href=\"./\">Shop docs</a>"), "the way back to the front page: {welcome}");
    let install = page(&rig, "/library/shop/docs/guides/install.md").await;
    assert!(install.contains("<img src=\"box.png\" alt=\"the box\""), "{install}");
    assert!(install.contains("<a href=\"../\">Shop docs</a>"), "{install}");
    page(&rig, "/library/shop/docs/notes%20with%20spaces.md").await;
    // Whatever a document holds, nothing in its page runs.
    let (_, policy, _) =
        fetch(&rig, "/library/shop/docs/welcome.md", Some(&rig.token), header::CONTENT_SECURITY_POLICY).await;
    assert!(policy.starts_with("default-src 'none'") && !policy.contains("script"), "{policy}");
    // The picture beside a document is handed over as what it is.
    let (status, kind, bytes) =
        fetch(&rig, "/library/shop/docs/guides/box.png", Some(&rig.token), header::CONTENT_TYPE).await;
    assert_eq!((status, kind.as_str(), bytes.as_str()), (StatusCode::OK, "image/png", "PNG"));
    // A folder of it has a page too, and is asked for as a folder.
    let (status, to, _) = fetch(&rig, "/library/shop/docs/guides", Some(&rig.token), header::LOCATION).await;
    assert_eq!((status, to.as_str()), (StatusCode::TEMPORARY_REDIRECT, "/library/shop/docs/guides/"));
    let guides = page(&rig, "/library/shop/docs/guides/").await;
    assert!(
        guides.contains("<h1>guides</h1>") && guides.contains("<a href=\"install.md\">Installing it</a>"),
        "{guides}"
    );

    // Beside every page are the library's contents, as a library written
    // out has them, each reached from where the page is and the page
    // showing marked.
    assert!(welcome.contains("<a href=\"welcome.md\" aria-current=\"page\">Welcome</a>"), "{welcome}");
    assert!(install.contains("<a href=\"install.md\" aria-current=\"page\">Installing it</a>"), "{install}");
    assert!(install.contains("<a href=\"../welcome.md\">Welcome</a>"), "{install}");
    assert!(install.contains("<a href=\"./\" class=\"page-nav-folder\">guides</a>"), "{install}");
    assert!(
        guides.contains("<a href=\"./\" class=\"page-nav-folder\" aria-current=\"page\">guides</a>"),
        "{guides}"
    );

    // A folder with a front page of its own shows that.
    write(&dir, "docs/README.md", "# The Shop handbook\n\nRead [this](welcome.md) first.\n");
    let front = page(&rig, "/library/shop/docs/").await;
    assert!(front.contains("<h1>The Shop handbook</h1>") && !front.contains("<h2>guides</h2>"), "{front}");
    // And says the order of its folder in its head, which the contents beside each page go by.
    write(&dir, "docs/README.md", "---\norder: [welcome, guides/]\n---\n\n# The Shop handbook\n");
    let front = page(&rig, "/library/shop/docs/").await;
    let at = |words: &str| front.find(words).unwrap_or_else(|| panic!("{words} is not in {front}"));
    assert!(
        at(">Welcome</a>") < at(">guides</a>") && at(">guides</a>") < at(">notes with spaces</a>"),
        "{front}"
    );
    assert!(front.contains("<a href=\"./\" aria-current=\"page\">Shop docs</a>"), "{front}");

    // Whatever is asked for is held inside the folder.
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.join("secret.md"), dir.join("docs/link.md")).unwrap();
    for outside in
        ["..%2Fsecret.md", "guides%2F..%2F..%2Fsecret.md", "%2Fetc%2Fpasswd", "link.md", ".git%2Fconfig"]
    {
        if outside == "link.md" && !cfg!(unix) {
            continue;
        }
        let (status, _, text) =
            fetch(&rig, &format!("/library/shop/docs/{outside}"), Some(&rig.token), header::CONTENT_TYPE)
                .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{outside}: {text}");
        assert!(!text.contains("Not the library's"), "{outside}");
    }
    let (status, _, _) =
        fetch(&rig, "/library/shop/docs/nothing.md", Some(&rig.token), header::CONTENT_TYPE).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Nothing here is public: it is for whoever may read the project's records.
    let (status, _, text) = fetch(&rig, "/library/shop/docs/", None, header::CONTENT_TYPE).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{text}");
    let (status, _, _) = fetch(&rig, "/library/shop/docs/guides/box.png", None, header::CONTENT_TYPE).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let narrow =
        rig.send("/api/credentials", json!({ "name": "narrow", "scopes": ["board", "projects"] })).await;
    let (status, _, text) =
        fetch(&rig, "/library/shop/docs/", narrow["token"].as_str(), header::CONTENT_TYPE).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{text}");
    let reader = rig.send("/api/credentials", json!({ "name": "reader", "scopes": ["records:read"] })).await;
    let (status, _, _) =
        fetch(&rig, "/library/shop/docs/welcome.md", reader["token"].as_str(), header::CONTENT_TYPE).await;
    assert_eq!(status, StatusCode::OK);
    // And only a library has pages.
    rig.board("shop").await;
    let (status, _, text) = fetch(&rig, "/library/shop/board/", Some(&rig.token), header::CONTENT_TYPE).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{text}");
}

#[tokio::test]
async fn a_watch_says_in_a_channel_what_happens_to_a_document() {
    let rig = Rig::new().await;
    shop(&rig).await;
    library(&rig).await;
    let params = json!({ "collection": "docs", "channel": "general" });
    let asked = json!({ "blueprint": "std/watch", "project": "shop", "key": "watch-docs", "params": params, "wait": true });
    assert_eq!(rig.send("/api/apply", asked).await["finished"]["status"], "ok");

    let made = json!({ "fields": { "title": "Returns", "body": "Thirty days." } });
    rig.send("/api/collections/shop/docs/records", made).await;
    rig.send("/api/records/shop-docs:returns/comments", json!({ "body": "Forty, surely." })).await;

    let mut posted = rig.follow("channel.message");
    let said: Vec<String> = crate::soon("the watch to say both", async {
        loop {
            let messages = rig.read("/api/channels/general/messages").await;
            let said: Vec<String> = messages["messages"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|message| message["author"] == "bot:watch")
                .map(|message| message["body"].as_str().unwrap().to_string())
                .collect();
            if said.len() >= 2 {
                return said;
            }
            posted.next().await.unwrap();
        }
    })
    .await;
    assert_eq!(said, ["shop-docs:returns: Ada wrote it", "shop-docs:returns: Ada said: Forty, surely."]);
    let _: Value = json!(null);
}
