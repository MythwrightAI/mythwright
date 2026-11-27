//! What a browser's cookie reads by itself, where a page is opened at its
//! address: a library's pages and a mockup's address. A browser hands the
//! cookie to every port of the machine, so whatever else listens there
//! (a program an agent wrote, served while it is worked on) has it.

use reqwest::header::{ACCEPT, LOCATION};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A person's notes: the days are theirs alone, the guides are the project's.
const NOTES: &str = r#"
blueprint: notes
permissions: [projects]
steps:
  days:
    do: collection.ensure
    with:
      name: days
      store: documents
      at: notes/days
      reads: private:journal
      fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } }
  guides:
    do: collection.ensure
    with:
      name: guides
      store: documents
      at: notes/guides
      fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } }
"#;

/// What a browser opening a page says it wants.
const PAGE: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";

/// The project `life` with a library kept private, one that is not, and a mockup.
async fn life(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Life" })).await;
    let applied = rig.send("/api/apply", json!({ "source": NOTES, "project": "life", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    for (path, content) in [
        ("notes/days/2026-11-14.md", "# Saturday\n\nThe marmalade is under the stairs.\n"),
        (
            "notes/guides/README.md",
            "# Guides\n\nStart with [bread](bread.md#flour), or [elsewhere](https://example.com/).\n",
        ),
        ("notes/guides/bread.md", "# Bread\n\n## Flour\n\n![a loaf](art/loaf.png)\n"),
        ("notes/guides/art/loaf.png", "not really a picture"),
        ("notes/guides/more/rye.md", "# Rye\n"),
        ("mockups/pricing/index.html", "<!doctype html><h1>Three plans</h1>\n"),
    ] {
        let (status, written) = rig
            .call(
                Method::PUT,
                "/api/projects/life/file",
                Some(&rig.token),
                Some(json!({ "path": path, "content": content })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{written}");
    }
    let mockups = json!({ "blueprint": "std/mockups", "project": "life", "wait": true });
    let applied = rig.send("/api/apply", mockups).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

#[tokio::test]
async fn a_librarys_pages_and_a_mockups_address_count_the_cookie_only_with_what_the_page_shows_and_their_links_say_it()
 {
    let rig = Rig::new().await;
    life(&rig).await;
    let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
    let at = |path: &str| format!("{}{path}", rig.url);
    let login = json!({ "email": "ada@example.com", "password": "correct horse", "page": true });
    let response = client.post(at("/api/auth/login")).json(&login).send().await.unwrap();
    let session = response.headers()["set-cookie"].to_str().unwrap().split(';').next().unwrap().to_string();
    let signed: Value = response.json().await.unwrap();
    let shows = signed["shows"].as_str().unwrap().to_string();
    let by = format!("by={shows}");
    let asked = |path: &str| client.get(at(path)).header("cookie", &session);
    let sent = |response: &reqwest::Response| response.headers()[LOCATION].to_str().unwrap().to_string();

    let pages = [
        "/library/life/guides/",
        "/library/life/guides/bread.md",
        "/library/life/guides/art/loaf.png",
        // The library kept private, which its owner holds.
        "/library/life/days/",
        "/library/life/days/2026-11-14.md",
    ];
    // With the cookie alone, nothing: not the library anybody of the
    // project may read, not the one kept private, not the way in to a mockup.
    for path in pages.iter().copied().chain(["/mockups/life-mockups:pricing"]) {
        let alone = asked(path).send().await.unwrap();
        assert_eq!(alone.status(), StatusCode::UNAUTHORIZED, "{path} was read with the cookie alone");
        let wrong = asked(&format!("{path}?by=0123abcd")).send().await.unwrap();
        assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED, "{path} with what another page shows");
        let no_cookie = client.get(at(&format!("{path}?{by}"))).send().await.unwrap();
        assert_eq!(no_cookie.status(), StatusCode::UNAUTHORIZED, "{path} with the address alone");
    }
    // With what the page shows beside it, as the page has its browser ask.
    for path in pages {
        let shown = asked(&format!("{path}?{by}")).send().await.unwrap();
        assert_eq!(shown.status(), StatusCode::OK, "{path}");
        // Its address holds what it was asked with: told to no page it leads on to.
        assert_eq!(shown.headers()["referrer-policy"], "no-referrer", "{path}");
    }
    let private = asked(&format!("/library/life/days/2026-11-14.md?{by}")).send().await.unwrap();
    assert!(private.text().await.unwrap().contains("marmalade"));
    let mockup = asked(&format!("/mockups/life-mockups:pricing?{by}")).send().await.unwrap();
    assert_eq!(mockup.status(), StatusCode::SEE_OTHER);
    let way = sent(&mockup);
    assert!(way.starts_with("/mockups/in/") && !way.contains("by="), "{way}");
    // A way in is no credential, and asks for none.
    let shown = client.get(at(&way)).send().await.unwrap();
    assert_eq!(shown.status(), StatusCode::OK);
    assert!(shown.text().await.unwrap().contains("Three plans"));

    // Every link of a page that leads somewhere here says it again: the
    // contents beside it, the way back up, what the document links to and
    // its pictures. A link elsewhere is as it was written.
    let front = asked(&format!("/library/life/guides/?{by}")).send().await.unwrap().text().await.unwrap();
    for link in [format!("href=\"bread.md?{by}#flour\""), format!("href=\"more/rye.md?{by}\"")] {
        assert!(front.contains(&link), "{link} in\n{front}");
    }
    assert!(front.contains("href=\"https://example.com/\""), "{front}");
    let bread =
        asked(&format!("/library/life/guides/bread.md?{by}")).send().await.unwrap().text().await.unwrap();
    for link in [format!("src=\"art/loaf.png?{by}\""), format!("href=\"./?{by}\"")] {
        assert!(bread.contains(&link), "{link} in\n{bread}");
    }
    let rye =
        asked(&format!("/library/life/guides/more/rye.md?{by}")).send().await.unwrap().text().await.unwrap();
    assert!(rye.contains(&format!("href=\"../?{by}\"")), "{rye}");
    // And where a page is sent on to where it is, it goes on saying it.
    let folder = asked(&format!("/library/life/guides/more?{by}")).send().await.unwrap();
    assert_eq!(sent(&folder), format!("/library/life/guides/more/?{by}"));
    let library = asked(&format!("/library/life/guides?{by}")).send().await.unwrap();
    assert_eq!(sent(&library), format!("/library/life/guides/?{by}"));

    // A token that is said reads them as it did, and its pages say nothing more.
    let said = client.get(at("/library/life/guides/bread.md")).bearer_auth(&rig.token).send().await.unwrap();
    assert_eq!(said.status(), StatusCode::OK);
    let said = said.text().await.unwrap();
    assert!(said.contains("src=\"art/loaf.png\"") && !said.contains("by="), "{said}");
}

#[tokio::test]
async fn a_page_opened_at_its_bare_address_is_sent_through_the_client_which_signs_in_whoever_is_not() {
    let rig = Rig::new().await;
    life(&rig).await;
    let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
    let at = |path: &str| format!("{}{path}", rig.url);
    let login = json!({ "email": "ada@example.com", "password": "correct horse", "page": true });
    let response = client.post(at("/api/auth/login")).json(&login).send().await.unwrap();
    let session = response.headers()["set-cookie"].to_str().unwrap().split(';').next().unwrap().to_string();

    // Opened as a page with the cookie alone, or with what another page
    // showed (an address kept from before), or by somebody not signed in:
    // sent to the client's page, told where to send the browser on to,
    // without what did not count.
    let cases = [
        ("/library/life/guides/bread.md", true, "/?to=/library/life/guides/bread.md"),
        ("/library/life/days/?by=0123abcd", true, "/?to=/library/life/days/"),
        ("/library/life/guides/?size=2&by=0123abcd", true, "/?to=/library/life/guides/%3Fsize%3D2"),
        ("/mockups/life-mockups:pricing", true, "/?to=/mockups/life-mockups%3Apricing"),
        ("/library/life/guides/", false, "/?to=/library/life/guides/"),
        ("/mockups/life-mockups:pricing", false, "/?to=/mockups/life-mockups%3Apricing"),
    ];
    for (path, with_cookie, wanted) in cases {
        let mut opened = client.get(at(path)).header(ACCEPT, PAGE);
        if with_cookie {
            opened = opened.header("cookie", &session);
        }
        let opened = opened.send().await.unwrap();
        assert_eq!(opened.status(), StatusCode::SEE_OTHER, "{path}");
        assert_eq!(opened.headers()[LOCATION].to_str().unwrap(), wanted, "{path}");
    }
    // Where that leads is the client's page.
    let page = client.get(at("/?to=/library/life/guides/")).header(ACCEPT, PAGE).send().await.unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    assert!(page.headers()["content-type"].to_str().unwrap().starts_with("text/html"));

    // What is not a browser opening a page is told to sign in, as anywhere;
    // and what is refused for what it is, not for who asks, is refused.
    let fetched =
        client.get(at("/library/life/guides/art/loaf.png")).header("cookie", &session).send().await.unwrap();
    assert_eq!(fetched.status(), StatusCode::UNAUTHORIZED);
    let narrow = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": ["search"] })).await;
    let narrow = narrow["token"].as_str().unwrap();
    let refused = client
        .get(at("/library/life/guides/"))
        .bearer_auth(narrow)
        .header(ACCEPT, PAGE)
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::FORBIDDEN);
}
