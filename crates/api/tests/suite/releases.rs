//! The notes of each release came with the program: read from the daemon
//! by anybody signed in, with nothing fetched from anywhere, and listed
//! by the headings of the one file they are written in.

use reqwest::{Method, StatusCode};

use crate::Rig;
use crate::held::key;

/// The notes as they are in the repository.
fn written() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/releases.md");
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

#[tokio::test]
async fn the_notes_of_each_release_are_read_from_the_daemon_by_anybody_signed_in_and_by_nobody_else() {
    let rig = Rig::new().await;
    let told = rig.read("/api/releases").await;
    // The list is the headings of the file, in the order they are written there.
    let headings: Vec<String> = written()
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .map(|title| title.trim().to_string())
        .collect();
    let listed: Vec<String> = told["releases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| one["title"].as_str().unwrap().to_string())
        .collect();
    assert!(!headings.is_empty(), "the notes tell of no release");
    assert_eq!(listed, headings);
    // The page is the whole of the notes, each release where the list says it is, and nothing in it runs or is fetched.
    let page = told["html"].as_str().unwrap();
    for one in told["releases"].as_array().unwrap() {
        let (title, at) = (one["title"].as_str().unwrap(), one["at"].as_str().unwrap());
        let part = page
            .split(&format!("<section id=\"{at}\">"))
            .nth(1)
            .unwrap_or_else(|| panic!("no part {at}: {page}"));
        let heading = part.split("</h2>").next().unwrap();
        assert!(
            heading.contains(&studio::pages::escape(title)),
            "{at} does not begin with {title}: {heading}"
        );
    }
    assert!(!page.contains("<script") && !page.contains("src=\"http"), "{page}");
    // Somebody who holds little reads them too: they say nothing of this factory.
    let narrow = key(&rig, &["channels:read"]).await;
    let (status, same) = rig.call(Method::GET, "/api/releases", Some(&narrow), None).await;
    assert_eq!((status, &same), (StatusCode::OK, &told));
    // Nobody who is not signed in is told anything.
    let (status, refused) = rig.call(Method::GET, "/api/releases", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{refused}");
}

#[test]
fn the_newest_release_the_notes_tell_of_is_this_program() {
    // A version with no notes is found here: the first heading is what the client says is new.
    let newest = api::releases::releases(api::releases::NOTES).into_iter().next().expect("a release");
    let version = env!("CARGO_PKG_VERSION");
    assert!(
        newest.title == version || newest.title.starts_with(&format!("{version}:")),
        "the newest release in docs/releases.md is headed `{}`, and this program is {version}: write what {version} is \
         under a heading of its own, `## {version}`, above the others",
        newest.title
    );
}

#[tokio::test]
async fn a_client_is_told_where_support_is_asked_for_as_whoever_built_the_program_said() {
    let rig = Rig::new().await;
    // Before anybody is signed in, with the name: the empty text where nowhere was said.
    let (status, meta) = rig.call(Method::GET, "/api/meta", None, None).await;
    assert_eq!(status, StatusCode::OK, "{meta}");
    assert_eq!(meta["support"], brand::SUPPORT, "{meta}");
}

#[test]
fn a_release_is_a_heading_of_the_second_level_and_nothing_inside_a_block_of_code_is_one() {
    let notes = "# Notes\n\nBefore any.\n\n## 2.0: two\n\nNewer.\n\n```\n## not one\n```\n\n### Smaller\n\n## 1.0\n\nOlder.\n";
    let told = api::releases::releases(notes);
    let said: Vec<(&str, &str)> = told.iter().map(|one| (one.title.as_str(), one.at.as_str())).collect();
    assert_eq!(said, [("2.0: two", "release-1"), ("1.0", "release-2")]);
    let page = api::releases::page(notes);
    let first = page.split("<section id=\"release-1\">").nth(1).unwrap().split("</section>").next().unwrap();
    assert!(
        first.contains("<h2>2.0: two</h2>")
            && first.contains("## not one")
            && first.contains("<h3>Smaller</h3>"),
        "{first}"
    );
    assert!(!first.contains("Older"), "{first}");
    assert!(page.split("<section").next().unwrap().contains("Before any."), "{page}");
    // Notes that tell of none are a page all the same, and a list of nothing.
    assert!(api::releases::releases("# Notes\n\nNothing yet.\n").is_empty());
    assert!(api::releases::page("# Notes\n\nNothing yet.\n").contains("Nothing yet."));
}

#[test]
fn a_nightly_says_its_version_from_what_its_package_says_and_nothing_else_is_taken_for_one() {
    use api::place::version_in;
    // As the tooling writes it beside a nightly's programs (`xtask::package::note`,
    // whose case in `xtask/tests/suite/systems.rs` writes these lines).
    for (note, said) in [
        (
            "1.2.3-nightly.20261124+abc1234, a nightly, revision abc1234",
            Some("1.2.3-nightly.20261124+abc1234"),
        ),
        ("1.2.3-nightly.20261124+abc1234, a nightly", Some("1.2.3-nightly.20261124+abc1234")),
        (
            "1.2.3-nightly.20261124+abc1234, a nightly, revision abc1234, built on Linux for Windows, for trying and never a release",
            Some("1.2.3-nightly.20261124+abc1234"),
        ),
        // A pinned release's says its revision, and its version is the program's.
        ("revision abc1234", None),
        ("revision abc1234, built on Linux for Windows, for trying and never a release", None),
        // Another version than the program was built as is not believed, nor anything half a nightly.
        ("9.9.9-nightly.20261124+abc1234, a nightly", None),
        ("1.2.3-nightly.2026112+abc1234, a nightly", None),
        ("1.2.3-nightly.20261124, a nightly", None),
        ("1.2.3-nightly.20261124+, a nightly", None),
        ("1.2.3-nightly.20261124+not hex, a nightly", None),
        ("1.2.3, a nightly", None),
        ("", None),
    ] {
        assert_eq!(version_in(note, "1.2.3"), said, "{note}");
    }
    // A program run from where it was built is the version it was built as,
    // which the notes of releases are held to above.
    assert_eq!(api::place::version(), env!("CARGO_PKG_VERSION"));
}
