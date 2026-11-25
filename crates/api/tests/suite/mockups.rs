//! Mockups (`std/mockups`): a project's folders of plain web files as
//! pages to try an idea with, over HTTP; and each shown in a place of its
//! own, through a way in that reads its folder and nothing else.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use kernel::ManualClock;
use reqwest::{Method, StatusCode, header};
use serde_json::{Value, json};

use crate::Rig;

const PRICING: &str = "<!doctype html>\n<html><head><title>Pricing  page</title><link rel=\"stylesheet\" href=\"style.css\"></head>\n<body><h1>Three plans</h1><img src=\"art/mark.svg\"><script src=\"app.js\"></script></body></html>\n";

fn write(dir: &Path, path: &str, text: &str) {
    let file = dir.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

/// A project `shop` with its mockups made, and where its folder is.
async fn shop(rig: &Rig) -> PathBuf {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let asked = json!({ "blueprint": "std/mockups", "project": "shop", "wait": true });
    let applied = rig.send("/api/apply", asked).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    rig.node().app.studio.projects.dir_of("shop").unwrap()
}

/// The pricing page, with a style sheet, a script and a picture beside it.
fn pricing(dir: &Path) {
    write(dir, "mockups/pricing/index.html", PRICING);
    write(dir, "mockups/pricing/style.css", "h1 { color: teal; }\n");
    write(dir, "mockups/pricing/app.js", "document.title = 'ran';\n");
    write(dir, "mockups/pricing/art/mark.svg", "<svg xmlns=\"http://www.w3.org/2000/svg\"/>\n");
}

async fn mockups(rig: &Rig) -> Vec<Value> {
    rig.read("/api/collections/shop/mockups/records").await["records"].as_array().unwrap().clone()
}

/// A way in to a mockup, for the rig's owner: its path.
async fn way(rig: &Rig, id: &str) -> String {
    rig.send("/api/mockups/way", json!({ "id": id })).await["path"].as_str().unwrap().to_string()
}

/// Asks for something with no credential at all, following nothing.
async fn fetch(rig: &Rig, path: &str) -> reqwest::Response {
    let plain = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
    plain.get(format!("{}{path}", rig.url)).send().await.unwrap()
}

fn said(body: &Value) -> String {
    body["error"]["message"].as_str().unwrap_or_default().to_string()
}

#[tokio::test]
async fn a_folder_that_holds_a_page_is_a_mockup_and_one_that_does_not_is_not() {
    let rig = Rig::new().await;
    let dir = shop(&rig).await;
    assert!(mockups(&rig).await.is_empty(), "there are none until a folder holds a page");

    pricing(&dir);
    write(&dir, "mockups/notes/readme.txt", "not a page\n");
    write(&dir, "mockups/loose.html", "<p>a file, not a folder</p>\n");
    write(&dir, "mockups/.hidden/index.html", "<p>hidden</p>\n");

    let found = mockups(&rig).await;
    assert_eq!(found.len(), 1, "{found:?}");
    let one = &found[0];
    assert_eq!(one["id"], "shop-mockups:pricing");
    // Called what its page says, at the first of the stages, a variant of nothing.
    assert_eq!(one["data"]["title"], "Pricing page");
    assert_eq!(one["data"]["path"], "pricing");
    assert_eq!(one["data"]["status"], "draft");
    assert_eq!(one["data"]["of"], Value::Null);
    assert!(!dir.join("mockups/pricing/mockup.yaml").exists(), "nothing was added to it for it to be one");

    // What it says of itself in the file beside its page is what is kept of it.
    write(&dir, "mockups/pricing/mockup.yaml", "title: The pricing page\nstatus: review\nowner: someone\n");
    let (_, one) = rig.get("/api/records/shop-mockups:pricing").await;
    assert_eq!(one["data"]["title"], "The pricing page", "{one}");
    assert_eq!(one["data"]["status"], "review");
}

#[tokio::test]
async fn a_variant_is_a_copy_of_what_it_varies_that_says_which() {
    let rig = Rig::new().await;
    let dir = shop(&rig).await;
    pricing(&dir);
    write(&dir, "mockups/pricing/mockup.yaml", "status: review\n");

    let asked = json!({ "fields": { "title": "Pricing, darker", "of": "shop-mockups:pricing" } });
    let made = rig.send("/api/collections/shop/mockups/records", asked).await;
    assert_eq!(made["id"], "shop-mockups:pricing-darker", "{made}");
    assert_eq!(made["data"]["of"], "shop-mockups:pricing");
    assert_eq!(made["data"]["status"], "draft", "how far the other had got is the other's");

    // Everything the page uses came with it, to be changed.
    let there = dir.join("mockups/pricing-darker");
    assert_eq!(std::fs::read_to_string(there.join("index.html")).unwrap(), PRICING);
    assert!(there.join("style.css").is_file() && there.join("art/mark.svg").is_file());
    let settings = std::fs::read_to_string(there.join("mockup.yaml")).unwrap();
    assert!(settings.contains("of: pricing") && settings.contains("title: Pricing, darker"), "{settings}");

    // It is never written over what is there, and is a variant only of a mockup.
    let (status, again) = rig
        .post(
            "/api/collections/shop/mockups/records",
            json!({ "fields": { "title": "x", "path": "pricing-darker" } }),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{again}");
    assert!(said(&again).contains("a mockup is never written over"), "{again}");
    let (status, none) = rig
        .post(
            "/api/collections/shop/mockups/records",
            json!({ "fields": { "title": "y", "of": "shop-mockups:nothing" } }),
        )
        .await;
    assert_ne!(status, StatusCode::OK, "{none}");
    assert!(!dir.join("mockups/y").exists(), "nothing was made of a variant of nothing");

    // A person who writes the line by hand names the other by its folder.
    write(&dir, "mockups/pricing-lighter/index.html", "<title>Lighter</title>\n");
    write(&dir, "mockups/pricing-lighter/mockup.yaml", "of: pricing\n");
    let (_, lighter) = rig.get("/api/records/shop-mockups:pricing-lighter").await;
    assert_eq!(lighter["data"]["of"], "shop-mockups:pricing", "{lighter}");
}

#[tokio::test]
async fn a_mockup_is_made_spoken_on_and_moved_on_and_its_history_says_so() {
    let rig = Rig::new().await;
    let dir = shop(&rig).await;

    let made =
        rig.send("/api/collections/shop/mockups/records", json!({ "fields": { "title": "Checkout" } })).await;
    let id = made["id"].as_str().unwrap();
    assert_eq!(id, "shop-mockups:checkout");
    let page = std::fs::read_to_string(dir.join("mockups/checkout/index.html")).unwrap();
    assert!(page.contains("<title>Checkout</title>") && page.contains("<h1>Checkout</h1>"), "{page}");

    rig.send(&format!("/api/records/{id}/comments"), json!({ "body": "The button is too small." })).await;
    let (status, moved) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{id}"),
            Some(&rig.token),
            Some(json!({ "fields": { "status": "review" } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{moved}");
    assert_eq!(moved["data"]["status"], "review");
    let settings = std::fs::read_to_string(dir.join("mockups/checkout/mockup.yaml")).unwrap();
    assert!(settings.contains("status: review"), "{settings}");

    // What is kept is written when its event is: that is what is waited for.
    rig.event("collection.history", |event| {
        event.data["record"] == "shop-mockups:checkout" && event.data["topic"] == "record.updated"
    })
    .await;
    let history = rig.read(&format!("/api/records/{id}/history")).await;
    let lines: Vec<&str> =
        history["history"].as_array().unwrap().iter().filter_map(|line| line["said"].as_str()).collect();
    assert_eq!(
        lines,
        ["Ada made it", "Ada said: The button is too small.", "Ada changed its status"],
        "{history}"
    );

    // A stage there is not is refused, and it cannot be moved by saying so.
    let (status, wrong) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{id}"),
            Some(&rig.token),
            Some(json!({ "fields": { "status": "shipped" } })),
        )
        .await;
    assert_ne!(status, StatusCode::OK, "{wrong}");
    let (status, moved) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{id}"),
            Some(&rig.token),
            Some(json!({ "fields": { "path": "elsewhere" } })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{moved}");
    assert!(said(&moved).contains("known by where it is"), "{moved}");

    // Taken away, its folder goes with it.
    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/records/{id}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert!(!dir.join("mockups/checkout").exists());
    assert!(mockups(&rig).await.is_empty());
}

#[tokio::test]
async fn a_mockup_is_shown_through_a_way_in_that_reads_its_own_folder_and_nothing_else() {
    let rig = Rig::new().await;
    let dir = shop(&rig).await;
    pricing(&dir);
    write(&dir, "mockups/other/index.html", "<title>Another</title>\n");
    write(&dir, "secret.txt", "the project's own\n");

    let path = way(&rig, "shop-mockups:pricing").await;
    assert!(path.starts_with("/mockups/in/") && path.ends_with('/'), "{path}");
    assert_eq!(way(&rig, "shop-mockups:pricing").await, path, "asked for again, the same way in is given");

    // Its page and what is beside it, to whoever holds the way in: no credential is shown.
    let page = fetch(&rig, &path).await;
    assert_eq!(page.status(), StatusCode::OK);
    let policy = page.headers()[header::CONTENT_SECURITY_POLICY].to_str().unwrap().to_string();
    assert!(policy.starts_with("sandbox allow-scripts"), "{policy}");
    assert!(!policy.contains("allow-same-origin"), "it is given no place among this daemon's own pages");
    assert!(policy.contains(&format!("connect-src 127.0.0.1:{}{path};", rig.node().addr.port())), "{policy}");
    assert_eq!(page.headers()[header::REFERRER_POLICY], "no-referrer");
    assert_eq!(page.text().await.unwrap(), PRICING);
    for (file, kind) in
        [("style.css", "text/css"), ("app.js", "text/javascript"), ("art/mark.svg", "image/svg+xml")]
    {
        let got = fetch(&rig, &format!("{path}{file}")).await;
        assert_eq!(got.status(), StatusCode::OK, "{file}");
        assert!(got.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with(kind), "{file}");
        // A drawing can hold a script: every file is put in the same place.
        assert!(got.headers()[header::CONTENT_SECURITY_POLICY].to_str().unwrap().starts_with("sandbox"));
    }

    // Nothing outside its folder, however it is asked for.
    for outside in [
        "../other/index.html",
        "..%2Fother%2Findex.html",
        "art/../../other/index.html",
        "../../secret.txt",
        "%2e%2e/%2e%2e/secret.txt",
        "/etc/hostname",
        "%2Fetc%2Fhostname",
        "art%5C..%5C..%5Csecret.txt",
        "C:%5Csecret.txt",
    ] {
        let got = fetch(&rig, &format!("{path}{outside}")).await;
        let (status, body) = (got.status(), got.text().await.unwrap());
        // Refused as it is asked, or not there; and one that a browser
        // would have taken out of the way in before asking is asked of the
        // daemon as a stranger, who is shown nothing.
        assert!(
            [StatusCode::BAD_REQUEST, StatusCode::NOT_FOUND, StatusCode::UNAUTHORIZED].contains(&status),
            "{outside} answered {status}: {body}"
        );
        assert!(!body.contains("Another") && !body.contains("the project's own"), "{outside}: {body}");
    }
    #[cfg(unix)]
    {
        // A link in the folder that leads out of it is not followed.
        std::os::unix::fs::symlink(dir.join("secret.txt"), dir.join("mockups/pricing/leak.txt")).unwrap();
        std::os::unix::fs::symlink(dir.join("mockups/other"), dir.join("mockups/pricing/door")).unwrap();
        for through in ["leak.txt", "door/index.html"] {
            let got = fetch(&rig, &format!("{path}{through}")).await;
            assert_ne!(got.status(), StatusCode::OK, "{through} was followed out of the folder");
        }
    }

    // A way in is no credential: it opens nothing of the API.
    let name = path.trim_start_matches("/mockups/in/").trim_end_matches('/');
    for route in ["/api/me", "/api/projects", "/api/collections/shop/mockups/records"] {
        let (status, _) = rig.call(Method::GET, route, Some(name), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{route} took a way in for a credential");
    }
    // And one nobody was given leads nowhere.
    let got = fetch(&rig, "/mockups/in/not-a-way-in/").await;
    assert_eq!(got.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_way_in_is_for_whoever_may_read_the_project_and_lasts_a_while() {
    let clock = ManualClock::new();
    let ticking = clock.clone();
    let rig =
        Rig::owned(Rig::with("api-mockups", move |config, _| config.clock = Some(Arc::new(ticking))).await)
            .await;
    let dir = shop(&rig).await;
    pricing(&dir);
    let id = "shop-mockups:pricing";

    // Nobody, and a key that may not read records, are given none.
    let (status, _) = rig.call(Method::POST, "/api/mockups/way", None, Some(json!({ "id": id }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let narrow = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": ["search"] })).await;
    let narrow = narrow["token"].as_str().unwrap();
    let (status, refused) =
        rig.call(Method::POST, "/api/mockups/way", Some(narrow), Some(json!({ "id": id }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    // Nor to what is not a mockup.
    let (status, missing) = rig.post("/api/mockups/way", json!({ "id": "shop-mockups:nothing" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");

    // Its own address sends whoever is signed in to a way in, and nobody else.
    let nobody = fetch(&rig, &format!("/mockups/{id}")).await;
    assert_eq!(nobody.status(), StatusCode::UNAUTHORIZED);
    let plain = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
    let signed_in =
        plain.get(format!("{}/mockups/{id}", rig.url)).bearer_auth(&rig.token).send().await.unwrap();
    assert_eq!(signed_in.status(), StatusCode::SEE_OTHER);
    let path = signed_in.headers()[header::LOCATION].to_str().unwrap().to_string();
    assert_eq!(fetch(&rig, &path).await.status(), StatusCode::OK);

    // It lasts a while and no longer.
    clock.advance(Duration::from_secs(59 * 60));
    assert_eq!(fetch(&rig, &path).await.status(), StatusCode::OK);
    clock.advance(Duration::from_secs(2 * 60));
    let late = fetch(&rig, &path).await;
    assert_eq!(late.status(), StatusCode::NOT_FOUND);
    let body: Value = late.json().await.unwrap();
    assert!(said(&body).contains("it has run out"), "{body}");
    // Asked for then, there is another.
    assert_ne!(way(&rig, id).await, path);
}

#[tokio::test]
async fn an_agent_asked_for_a_mockup_leaves_one_where_it_was_told_one_goes() {
    // An agent that keeps what it was told and writes a page where a mockup goes.
    let page = "<title>Landing, by an agent</title><h1>Hello</h1>";
    let rig = crate::lanes::rig_whose_agent(&[
        "read",
        "write",
        "told.txt",
        "%:read%",
        "write",
        "mockups/landing/index.html",
        page,
        "write",
        "mockups/landing-bold/index.html",
        page,
        "write",
        "mockups/landing-bold/mockup.yaml",
        "of: landing",
        "say",
        "I made two.",
    ])
    .await;
    let dir = shop(&rig).await;
    rig.send("/api/agents", json!({ "name": "builder", "adapter": "writer", "project": "shop" })).await;
    let source = "blueprint: errand\nsteps:\n  go:\n    do: agent.task\n    with: { slot: builder, prompt: \"Make a mockup of a landing page, and a bolder variant.\" }\n";
    let applied = rig.send("/api/apply", json!({ "source": source, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");

    // It was told where one goes and what makes one.
    let told = std::fs::read_to_string(dir.join("told.txt")).unwrap();
    assert!(told.contains("`mockups/<name>/`") && told.contains("`index.html`"), "{told}");
    assert!(told.contains("of: <the other's folder>"), "{told}");

    let found = mockups(&rig).await;
    let titles: Vec<&str> = found.iter().filter_map(|one| one["data"]["title"].as_str()).collect();
    assert_eq!(titles, ["Landing, by an agent", "Landing, by an agent"], "{found:?}");
    assert_eq!(found[1]["id"], "shop-mockups:landing-bold");
    assert_eq!(found[1]["data"]["of"], "shop-mockups:landing");
}
