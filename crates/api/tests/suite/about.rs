//! What the program says of itself for the page about it: what it is for and the guides marked public, each a
//! page, to anybody signed in; the packages installed to whoever may read
//! them; and where the factory keeps its data to whoever may manage it
//! and nobody else. All of it came with the program.

use reqwest::{Method, StatusCode};

use crate::Rig;
use crate::held::key;

#[tokio::test]
async fn the_program_says_what_it_is_for_and_its_guides_to_anybody_and_where_its_data_is_to_whoever_manages_it()
 {
    let rig = Rig::new().await;
    let told = rig.read("/api/about").await;
    // What it is for, as a page of `docs/about.md`, which says the program's name, and nothing in it runs or is fetched.
    let page = told["html"].as_str().unwrap();
    assert!(page.contains(&format!("What is {}", brand::DISPLAY)), "{page}");
    assert!(!page.contains("<script") && !page.contains("src=\"http"), "{page}");
    // The guides are every public guide but that one, in the order they are kept, each by what it calls itself.
    let listed: Vec<&str> =
        told["guides"].as_array().unwrap().iter().map(|one| one["name"].as_str().unwrap()).collect();
    let kept: Vec<&str> =
        manual::guides::GUIDES.iter().map(|(name, _)| *name).filter(|name| *name != "about").collect();
    assert_eq!(listed, kept);
    for one in told["guides"].as_array().unwrap() {
        let name = one["name"].as_str().unwrap();
        let guide = rig.read(&format!("/api/about/guides/{name}")).await;
        assert_eq!(guide["title"], one["title"], "{guide}");
        let html = guide["html"].as_str().unwrap();
        assert!(html.contains("<h1") && !html.contains("<script"), "{name}: {html}");
    }
    // The owner is told where the data is kept, whole, and what is installed.
    let data = told["data"].as_str().expect("the owner is told where the data is");
    assert!(std::path::Path::new(data).is_absolute(), "{data}");
    assert!(told["packages"].is_array(), "{told}");
    // A member is told the rest, and not where the data is.
    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let (status, theirs) = rig.call(Method::GET, "/api/about", Some(&member), None).await;
    assert_eq!(status, StatusCode::OK, "{theirs}");
    assert!(theirs.get("data").is_none(), "{theirs}");
    assert_eq!((&theirs["html"], &theirs["guides"]), (&told["html"], &told["guides"]));
    // Somebody who holds little is told what it is and its guides alone.
    let narrow = key(&rig, &["channels:read"]).await;
    let (status, little) = rig.call(Method::GET, "/api/about", Some(&narrow), None).await;
    assert_eq!(status, StatusCode::OK, "{little}");
    assert!(little.get("data").is_none() && little.get("packages").is_none(), "{little}");
    // A guide that is not there, and the page about the program asked for as a guide, are not found.
    for name in ["nothing", "about"] {
        let (status, refused) =
            rig.call(Method::GET, &format!("/api/about/guides/{name}"), Some(&narrow), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    }
    // Nobody who is not signed in is told anything.
    let (status, refused) = rig.call(Method::GET, "/api/about", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
}

#[tokio::test]
async fn a_guide_links_to_other_guides_and_to_nothing_no_copy_of_the_program_carries() {
    let rig = Rig::new().await;
    let first = rig.read("/api/about/guides/first-run").await;
    let html = first["html"].as_str().unwrap();
    // Another guide is a link still, and the README, which only the repository has, is its words alone.
    assert!(html.contains("href=\"extensions.md\""), "{html}");
    assert!(!html.contains("README.md") && html.contains("README"), "{html}");
    // Nothing is drawn that a page would fetch.
    assert!(!html.contains("<img"), "{html}");
}
