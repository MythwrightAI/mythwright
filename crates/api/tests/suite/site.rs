//! A library written out as pages (`library.export`, `std/site`): only
//! what is published, every link leading to a file that is there, the
//! folder replaced as a whole and only when it is an export's own.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

fn write(dir: &Path, path: &str, text: &str) {
    let file = dir.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

/// A document that says of itself that it is published.
fn public(heading: &str, rest: &str) -> String {
    format!("---\npublic: true\n---\n\n# {heading}\n\n{rest}\n")
}

/// A project `shop` with a library over its `docs` folder, which holds
/// only what the test puts in it; and where the project is.
async fn shop(rig: &Rig, documents: &[(&str, &str)]) -> PathBuf {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = rig.node().app.studio.projects.dir_of("shop").unwrap();
    // A new project has a page or two of its own to begin with.
    let _ = std::fs::remove_dir_all(dir.join("docs"));
    for (path, text) in documents {
        write(&dir, &format!("docs/{path}"), text);
    }
    let asked = json!({ "blueprint": "std/docs", "project": "shop", "wait": true });
    assert_eq!(rig.send("/api/apply", asked).await["finished"]["status"], "ok");
    dir
}

/// Asks for the library to be written out, as a blueprint of one step
/// does, and gives what came of the run.
async fn export(rig: &Rig, token: &str, with: Value) -> Value {
    let source = format!(
        "blueprint: write-out\npermissions: [projects, records]\nsteps:\n  out:\n    do: library.export\n    with: {with}\noutputs:\n  out: ${{ steps.out.out }}\n"
    );
    let asked = json!({ "source": source, "project": "shop", "wait": true, "force": true });
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(token), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    applied["finished"].clone()
}

/// What was written out, having been: the step's own answer.
async fn exported(rig: &Rig, with: Value) -> Value {
    let finished = export(rig, &rig.token, with).await;
    assert_eq!(finished["status"], "ok", "{finished}");
    finished["outputs"]["out"].clone()
}

/// Why it was not written out.
async fn refused(rig: &Rig, with: Value) -> String {
    let finished = export(rig, &rig.token, with).await;
    assert_eq!(finished["status"], "failed", "{finished}");
    finished["error"].as_str().unwrap_or_default().to_string()
}

/// Every file under `dir`, by where it is from there.
fn files(dir: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut pending = vec![(dir.to_path_buf(), String::new())];
    while let Some((folder, within)) = pending.pop() {
        for item in std::fs::read_dir(folder).unwrap() {
            let item = item.unwrap();
            let name = format!("{within}{}", item.file_name().to_string_lossy());
            match item.path().is_dir() {
                true => pending.push((item.path(), format!("{name}/"))),
                false => drop(found.insert(name)),
            }
        }
    }
    found
}

fn named(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

/// What every `href` and `src` of a page leads to, as it is written.
fn leads(page: &str) -> Vec<String> {
    let mut found = Vec::new();
    for mark in ["href=\"", "src=\""] {
        let mut rest = page;
        while let Some(at) = rest.find(mark) {
            let after = &rest[at + mark.len()..];
            let end = after.find('"').unwrap();
            found.push(after[..end].replace("&amp;", "&"));
            rest = &after[end..];
        }
    }
    found
}

/// A link as the file it names, from the folder of the page it is in.
fn file_of(folder: &Path, link: &str) -> PathBuf {
    let path = link.split(['#', '?']).next().unwrap();
    let mut file = folder.to_path_buf();
    for part in path.split('/') {
        let part = part.replace("%20", " ");
        match part.as_str() {
            "" | "." => {}
            ".." => drop(file.pop()),
            name => file.push(name),
        }
    }
    file
}

#[tokio::test]
async fn only_the_documents_that_say_they_are_public_are_written_out_unless_all_are_asked_for() {
    let rig = Rig::new().await;
    let dir = shop(
        &rig,
        &[
            ("welcome.md", &public("Welcome", "Start here.")),
            ("guides/install.md", &public("Installing it", "Run it.")),
            ("plan.md", "# The plan\n\nNot said aloud yet.\n"),
            ("notes/private.md", "---\npublic: false\n---\n\nKept in.\n"),
            ("data.csv", "a,b\n"),
        ],
    )
    .await;

    let out = exported(&rig, json!({ "collection": "docs" })).await;
    assert_eq!((out["to"].as_str(), out["from"].as_str()), (Some("site"), Some("docs")), "{out}");
    assert_eq!((out["pages"].as_u64(), out["files"].as_u64()), (Some(4), Some(1)), "{out}");
    assert_eq!(
        files(&dir.join("site")),
        named(&[
            ".exported",
            "guides/index.html",
            "guides/install.html",
            "index.html",
            "page.css",
            "welcome.html"
        ]),
        "a page a published document, a front page a folder that has one, and the one style sheet"
    );
    // Nothing of what is not published is anywhere in what was written.
    for file in files(&dir.join("site")) {
        let text = std::fs::read_to_string(dir.join("site").join(&file)).unwrap();
        assert!(!text.contains("Not said aloud") && !text.contains("Kept in"), "{file}");
        assert!(
            !text.contains("The plan") && !text.contains("private"),
            "{file} names what is not published"
        );
    }
    let front = std::fs::read_to_string(dir.join("site/index.html")).unwrap();
    assert!(front.contains("<a href=\"welcome.html\">Welcome</a>"), "{front}");
    assert!(front.contains("<a href=\"guides/install.html\">Installing it</a>"), "{front}");

    // All of them, asked for: every document, and a front page for every folder.
    let out = exported(&rig, json!({ "collection": "docs", "publish": "all" })).await;
    assert_eq!(out["pages"], 7, "{out}");
    let all = files(&dir.join("site"));
    for page in ["plan.html", "notes/private.html", "notes/index.html"] {
        assert!(all.contains(page), "{page} is not among {all:?}");
    }
    assert!(!all.iter().any(|file| file.ends_with(".md") || file.ends_with(".csv")), "{all:?}");

    let unknown = refused(&rig, json!({ "collection": "docs", "publish": "some" })).await;
    assert!(unknown.contains("there are `marked`") && unknown.contains("and `all`"), "{unknown}");
}

#[tokio::test]
async fn a_link_to_what_is_not_written_out_is_its_words_and_is_named_in_the_answer() {
    let rig = Rig::new().await;
    let text = "See [the plan](plan.md), [the notes](notes/), [nothing](gone.md), \
                [the top](/library/shop/docs/), [outside](../fleet.bp), [a script](run.js), \
                [the guide](guides/install.md#first) and [elsewhere](https://example.com/a).\n\n\
                ![far](https://example.com/far.png) ![near](pictures/dot.png) ![drawn](pictures/live.svg)";
    let dir = shop(
        &rig,
        &[
            ("welcome.md", &public("Welcome", text)),
            ("guides/install.md", &public("Installing it", "Back to [the start](../welcome.md).")),
            ("plan.md", "# The plan\n"),
            ("notes/private.md", "Kept in.\n"),
            ("run.js", "alert(1)\n"),
            ("pictures/dot.png", "not really a picture"),
            ("pictures/live.svg", "<svg xmlns=\"http://www.w3.org/2000/svg\" onload=\"alert(1)\"></svg>"),
            ("pictures/unused.png", "nobody shows this"),
        ],
    )
    .await;

    let out = exported(&rig, json!({ "collection": "docs" })).await;
    let page = std::fs::read_to_string(dir.join("site/welcome.html")).unwrap();
    // What is written out is linked to, where it is among the files.
    assert!(page.contains("<a href=\"guides/install.html#first\">the guide</a>"), "{page}");
    assert!(page.contains("<a href=\"https://example.com/a\">elsewhere</a>"), "{page}");
    assert!(page.contains("<img src=\"pictures/dot.png\" alt=\"near\""), "{page}");
    let back = std::fs::read_to_string(dir.join("site/guides/install.html")).unwrap();
    assert!(back.contains("<a href=\"../welcome.html\">the start</a>"), "{back}");
    // What is not is the words it was.
    for words in ["the plan", "the notes", "nothing", "the top", "outside", "a script", "far", "drawn"] {
        assert!(page.contains(words), "`{words}` is gone from {page}");
    }
    for link in ["plan", "notes/", "gone", "/library", "fleet.bp", "run.js", "far.png", "live.svg"] {
        assert!(
            !page.contains(&format!("\"{link}")) && !page.contains(&format!("/{link}\"")),
            "{link}: {page}"
        );
    }
    // And each is named, with why.
    let why = |link: &str| -> String {
        let left = out["left_out"].as_array().unwrap().iter().find(|left| left["link"] == link);
        let left = left.unwrap_or_else(|| panic!("`{link}` is not named in {}", out["left_out"]));
        assert_eq!(left["page"], "welcome.md");
        left["why"].as_str().unwrap().to_string()
    };
    assert_eq!(why("plan.md"), "that document is not published");
    assert_eq!(why("notes/"), "nothing in that folder is written out");
    assert_eq!(why("gone.md"), "there is no such document");
    assert!(why("/library/shop/docs/").contains("from the top of an address"));
    assert_eq!(why("../fleet.bp"), "it leads out of the library");
    assert_eq!(why("run.js"), "a file of that kind is not written out");
    assert_eq!(why("https://example.com/far.png"), "a picture kept somewhere else is not fetched");
    assert!(why("pictures/live.svg").contains("holds script"));
    assert_eq!(out["left_out"].as_array().unwrap().len(), 8, "{}", out["left_out"]);
    // A picture is copied where a page that is written uses it, and only there.
    let written = files(&dir.join("site"));
    assert!(written.contains("pictures/dot.png"), "{written:?}");
    assert!(
        !written.contains("pictures/unused.png") && !written.contains("pictures/live.svg"),
        "{written:?}"
    );
    assert!(!written.contains("run.js"), "{written:?}");
}

#[tokio::test]
async fn every_link_and_picture_of_every_page_written_leads_to_a_file_that_is_there_wherever_the_folder_is_put()
 {
    let rig = Rig::new().await;
    let dir = shop(
        &rig,
        &[
            ("README.md", "# The shop\n\n[Install](guides/install.md), [the guides](guides/), [deep](guides/more/deep%20one.md).\n"),
            ("guides/install.md", "# Installing it\n\n[Up](../), [home](../README.md), ![dot](../pictures/dot.png), [deep](more/deep%20one.md).\n"),
            ("guides/more/deep one.md", "# Deep\n\n[Top](../../README.md) and [install](../install.md#steps). ![dot](../../pictures/dot.png)\n"),
            ("guides/more/index.md", "# More\n\n[Deep](deep%20one.md)\n"),
            ("pictures/dot.png", "not really a picture"),
        ],
    )
    .await;
    let out = exported(&rig, json!({ "collection": "docs", "publish": "all", "to": "out/pages" })).await;
    assert_eq!(out["left_out"], json!([]), "everything these pages link to is written");
    let site = dir.join("out/pages");
    let pages: Vec<String> = files(&site).into_iter().filter(|file| file.ends_with(".html")).collect();
    assert_eq!(
        pages,
        [
            "README.html",
            "guides/index.html",
            "guides/install.html",
            "guides/more/deep one.html",
            "guides/more/index.html",
            "index.html"
        ],
        "a folder's front page is its document called `index` or `README`, or the list of what is in it"
    );
    let host = rig.url.trim_start_matches("http://");
    let mut followed = 0;
    for page in &pages {
        let text = std::fs::read_to_string(site.join(page)).unwrap();
        let folder = site.join(page).parent().unwrap().to_path_buf();
        assert!(!text.contains(host) && !text.contains("/library/"), "{page} names the daemon");
        for link in leads(&text) {
            // A way to somewhere in the page itself is to what is there.
            if let Some(id) = link.strip_prefix('#') {
                assert!(text.contains(&format!("id=\"{id}\"")), "{page}: `{link}` leads to nothing in it");
                continue;
            }
            // From wherever the page is: nothing from the top of an address, nothing by name.
            assert!(!link.starts_with('/') && !link.contains("://"), "{page}: `{link}` is not beside it");
            let file = file_of(&folder, &link);
            assert!(file.starts_with(&site), "{page}: `{link}` leads out of the folder");
            assert!(file.is_file(), "{page}: `{link}` leads to {}, which is not there", file.display());
            followed += 1;
        }
    }
    assert!(followed >= 20, "only {followed} links were looked at");
    // The front page of the whole is the document that is one, and the
    // same document has a page under its own name.
    let front = std::fs::read_to_string(site.join("index.html")).unwrap();
    assert!(front.contains("<h1>The shop</h1>") && !front.contains("page-up"), "{front}");
    let deep = std::fs::read_to_string(site.join("guides/more/deep one.html")).unwrap();
    assert!(deep.contains("<a href=\"../../index.html\">Shop docs</a>"), "{deep}");
    assert!(deep.contains("href=\"../../page.css\""), "{deep}");
}

#[tokio::test]
async fn nothing_in_a_page_that_is_written_out_can_run_or_fetch_from_elsewhere() {
    let rig = Rig::new().await;
    let dir = shop(
        &rig,
        &[
            ("welcome.md", &public("Welcome", "Plain words, and [a way in](javascript:alert(1)).")),
            ("guides/install.md", &public("Installing it", "A table:\n\n| a | b |\n|---|---|\n| 1 | 2 |")),
        ],
    )
    .await;
    let out = exported(&rig, json!({ "collection": "docs" })).await;
    assert_eq!(out["left_out"][0]["why"], "a link of that kind is not written out", "{out}");
    let site = dir.join("site");
    for file in files(&site) {
        let text = std::fs::read_to_string(site.join(&file)).unwrap();
        // Nothing that is written is, or brings, anything that runs.
        assert!(!text.contains("<script") && !text.contains("javascript:"), "{file}: {text}");
        assert!(!file.ends_with(".js"), "{file}");
        if !file.ends_with(".html") {
            continue;
        }
        // And each page says so of itself, for a browser to hold it to
        // whatever a document comes to hold: only itself, its pictures
        // and its style sheet from beside it.
        let policy = text
            .split("http-equiv=\"Content-Security-Policy\" content=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_else(|| panic!("{file} says nothing of what it may do"));
        assert!(
            policy.starts_with("default-src &#39;none&#39;") || policy.starts_with("default-src 'none'"),
            "{policy}"
        );
        assert!(!policy.contains("script-src") && !policy.contains("unsafe"), "{policy}");
        assert!(!policy.contains("http") && !policy.contains('*'), "{policy}");
        // Its style is the sheet beside it, and none of its own.
        assert!(text.contains("<link rel=\"stylesheet\" href=\"") && !text.contains("<style"), "{file}");
    }
}

#[tokio::test]
async fn a_second_export_takes_away_what_is_gone_and_leaves_alone_what_is_as_it_was() {
    let rig = Rig::new().await;
    let dir = shop(
        &rig,
        &[("welcome.md", &public("Welcome", "Start here.")), ("old.md", &public("Old", "On its way out."))],
    )
    .await;
    let first = exported(&rig, json!({ "collection": "docs" })).await;
    assert_eq!(first["changed"], true);
    let site = dir.join("site");
    assert!(files(&site).contains("old.html"));
    let stamp = |file: &str| std::fs::metadata(site.join(file)).unwrap().modified().unwrap();
    let before = stamp("welcome.html");
    // The list of what was written is how the next export knows its own.
    let listed = std::fs::read_to_string(site.join(".exported")).unwrap();
    assert_eq!(listed, "index.html\nold.html\npage.css\nwelcome.html\n");

    // Asked again with nothing changed, nothing is written.
    let same = exported(&rig, json!({ "collection": "docs" })).await;
    assert_eq!((same["changed"].as_bool(), same["pages"].as_u64()), (Some(false), Some(3)), "{same}");
    assert_eq!(stamp("welcome.html"), before);

    // Another document's words changed: its page is written, and a page
    // that is as it was is not said to have changed.
    write(&dir, "docs/old.md", &public("Old", "Still on its way out."));
    let words = exported(&rig, json!({ "collection": "docs" })).await;
    assert_eq!(words["changed"], true);
    assert!(std::fs::read_to_string(site.join("old.html")).unwrap().contains("Still on its way out."));
    assert_eq!(stamp("welcome.html"), before, "a page that is as it was is not said to have changed");

    // A document taken away, and one no longer published: their pages go,
    // and every page's contents say so.
    std::fs::remove_file(dir.join("docs/old.md")).unwrap();
    write(&dir, "docs/new.md", &public("New", "Just in."));
    let second = exported(&rig, json!({ "collection": "docs" })).await;
    assert_eq!(second["changed"], true);
    assert_eq!(files(&site), named(&[".exported", "index.html", "new.html", "page.css", "welcome.html"]));
    let welcome = std::fs::read_to_string(site.join("welcome.html")).unwrap();
    assert!(welcome.contains(">New</a>") && !welcome.contains(">Old</a>"), "{welcome}");
    // Nothing is left beside it of the making.
    let beside: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|item| item.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!beside.iter().any(|name| name.starts_with(".site")), "{beside:?}");

    // A library whose folder has gone is not one with no documents: the
    // pages are left as they were.
    std::fs::rename(dir.join("docs"), dir.join("put-away")).unwrap();
    let why = refused(&rig, json!({ "collection": "docs" })).await;
    assert!(why.contains("is not there, so there is nothing to write out"), "{why}");
    assert!(files(&site).contains("welcome.html"));
}

#[tokio::test]
async fn a_folder_that_holds_what_no_export_put_there_is_left_as_it_is() {
    let rig = Rig::new().await;
    let dir = shop(&rig, &[("welcome.md", &public("Welcome", "Start here."))]).await;
    // Somebody's own folder is not written over.
    write(&dir, "pages/mine.txt", "written by hand\n");
    let why = refused(&rig, json!({ "collection": "docs", "to": "pages" })).await;
    assert!(why.contains("`pages` holds `mine.txt`, which no export put there"), "{why}");
    assert_eq!(files(&dir.join("pages")), named(&["mine.txt"]));

    // Nor is an export's, once something else is in it.
    exported(&rig, json!({ "collection": "docs" })).await;
    write(&dir, "site/drafts/notes.txt", "mine\n");
    write(&dir, "docs/welcome.md", &public("Welcome", "Changed since."));
    let why = refused(&rig, json!({ "collection": "docs" })).await;
    assert!(why.contains("`site` holds `drafts/notes.txt`") && why.contains("Move it out"), "{why}");
    let kept = std::fs::read_to_string(dir.join("site/welcome.html")).unwrap();
    assert!(kept.contains("Start here.") && !kept.contains("Changed since."), "the pages are as they were");
    assert_eq!(std::fs::read_to_string(dir.join("site/drafts/notes.txt")).unwrap(), "mine\n");

    // A file where the folder would be is not one.
    write(&dir, "book", "a file\n");
    let why = refused(&rig, json!({ "collection": "docs", "to": "book" })).await;
    assert!(why.contains("`book` is not a folder"), "{why}");
}

#[tokio::test]
async fn a_library_is_written_out_only_inside_its_project_and_beside_its_own_folder() {
    let rig = Rig::new().await;
    let dir = shop(&rig, &[("welcome.md", &public("Welcome", "Start here."))]).await;
    let outside = dir.parent().unwrap().join("elsewhere");
    for to in ["../elsewhere", outside.to_str().unwrap(), "site/../../elsewhere"] {
        let why = refused(&rig, json!({ "collection": "docs", "to": to })).await;
        assert!(why.contains("is not where `docs` can be written out"), "{to}: {why}");
        assert!(why.contains("from the top of the project"), "{to}: {why}");
    }
    assert!(!outside.exists());
    let why = refused(&rig, json!({ "collection": "docs", "to": "." })).await;
    assert!(why.contains("that is the project itself"), "{why}");
    // Not in the library's own folder, where the pages would be taken
    // for part of it, nor around it, where it would be taken away.
    let why = refused(&rig, json!({ "collection": "docs", "to": "docs/site" })).await;
    assert!(why.contains("it is in the folder the library `docs` is the documents of"), "{why}");
    let why = refused(&rig, json!({ "collection": "docs", "to": "docs" })).await;
    assert!(why.contains("it is in the folder"), "{why}");
    assert!(dir.join("docs/welcome.md").is_file(), "the library is as it was");
    // Nor through a link that leads out.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dir.parent().unwrap(), dir.join("way-out")).unwrap();
        let why = refused(&rig, json!({ "collection": "docs", "to": "way-out/elsewhere" })).await;
        assert!(why.contains("leads outside through a link"), "{why}");
        assert!(!outside.exists());
    }
    // What is not a library has no pages.
    let board = json!({ "blueprint": "std/board", "project": "shop", "wait": true });
    assert_eq!(rig.send("/api/apply", board).await["finished"]["status"], "ok");
    let why = refused(&rig, json!({ "collection": "board" })).await;
    assert!(why.contains("is not a library"), "{why}");
}

#[tokio::test]
async fn std_site_writes_a_library_out_and_again_when_a_document_of_it_changes() {
    let rig = Rig::new().await;
    let dir = shop(&rig, &[("welcome.md", &public("Welcome", "Start here."))]).await;
    let applied =
        rig.send("/api/apply", json!({ "blueprint": "std/site", "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(applied["finished"]["outputs"]["pages"], 2, "{applied}");
    let page = || std::fs::read_to_string(dir.join("site/welcome.html")).unwrap_or_default();
    assert!(page().contains("Start here."));
    // What says that it was written out again: waited for, not guessed at.
    let written_after = async |seq: u64| {
        rig.event("collection.exported", move |event| event.seq > seq && event.data["project"] == "shop")
            .await
    };

    // Changed through the factory.
    let now = rig.mark();
    let body = json!({ "path": "docs/welcome.md", "content": public("Welcome", "Changed in the editor.") });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    written_after(now).await;
    assert!(page().contains("Changed in the editor."), "{}", page());

    // A document added by another program: found when the project's files
    // are looked over, which is what the daemon does when it is told a
    // folder of a library changed.
    let now = rig.mark();
    write(&dir, "docs/guides/install.md", &public("Installing it", "Run it."));
    rig.send("/api/projects/shop/scan", json!({})).await;
    let said = written_after(now).await;
    assert_eq!(said.data["pages"], 4, "{:?}", said.data);
    assert!(dir.join("site/guides/install.html").is_file());

    // A file of the project that is no document's business changes nothing.
    let now = rig.mark();
    let body = json!({ "path": "notes.txt", "content": "not in the library" });
    rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    let body = json!({ "path": "docs/welcome.md", "content": public("Welcome", "And once more.") });
    rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    written_after(now).await;
    // This failed once in some dozens of runs, with a page that was not
    // there: a library written out for another reason, in the moment the
    // file was being saved, read it emptied and left its page out. A save
    // now puts the file in place whole (`studio::projects`); this still
    // says what there was, should it ever fail again.
    assert!(
        page().contains("And once more."),
        "the page:\n{}\nwhat happened last, the latest first:\n{}",
        page(),
        rig.lately().await
    );
    let exports = rig.count_of("collection.exported").await;
    assert_eq!(exports, 4, "once when applied, and once for each change to the library");
    // Each was measured.
    let measured = rig.read("/api/metrics?name=library.export").await;
    assert!(measured.to_string().contains("library.export"), "{measured}");
}

#[tokio::test]
async fn writing_a_library_out_is_for_whoever_may_read_it_and_write_the_projects_files() {
    let rig = Rig::new().await;
    let dir = shop(&rig, &[("welcome.md", &public("Welcome", "Start here."))]).await;
    let key = async |scopes: &[&str]| {
        let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
        made["token"].as_str().unwrap().to_string()
    };
    // Reading the library is not enough: it writes files of the project.
    let reader = key(&["blueprints", "records", "projects:read"]).await;
    let asked = json!({ "collection": "docs" });
    let source = format!(
        "blueprint: write-out\npermissions: [projects, records]\nsteps:\n  out:\n    do: library.export\n    with: {asked}\n"
    );
    let apply = json!({ "source": source, "project": "shop", "wait": true });
    let (status, answer) = rig.call(Method::POST, "/api/apply", Some(&reader), Some(apply.clone())).await;
    let said = answer.to_string();
    assert!(status != StatusCode::OK || answer["finished"]["status"] == "failed", "{answer}");
    assert!(said.contains("projects:write"), "{said}");
    assert!(!dir.join("site").exists());

    // Nor is writing files: what is written out is what the library holds.
    let writer = key(&["blueprints", "projects"]).await;
    let (status, answer) = rig.call(Method::POST, "/api/apply", Some(&writer), Some(apply.clone())).await;
    let said = answer.to_string();
    assert!(status != StatusCode::OK || answer["finished"]["status"] == "failed", "{answer}");
    assert!(said.contains("records"), "{said}");
    assert!(!dir.join("site").exists());

    let both = key(&["blueprints", "projects", "records:read"]).await;
    let (status, answer) = rig.call(Method::POST, "/api/apply", Some(&both), Some(apply)).await;
    assert_eq!((status, &answer["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{answer}");
    assert!(dir.join("site/welcome.html").is_file());
}

#[tokio::test]
async fn every_page_written_out_has_the_contents_beside_it_in_the_order_each_folders_front_page_says() {
    let rig = Rig::new().await;
    let dir = shop(
        &rig,
        &[
            ("README.md", "---\npublic: true\norder: [zebra, guides/]\n---\n\n# The shop\n\nWelcome.\n"),
            ("about.md", &public("About us", "Who.")),
            ("zebra.md", &public("Zebra", "Last by title, first by order.")),
            ("plan.md", "# The plan\n\nNot published.\n"),
            (
                "guides/index.md",
                "---\npublic: true\norder:\n  - usage\n  - install\n---\n\n# All the guides\n",
            ),
            ("guides/install.md", &public("Installing it", "Run it.")),
            ("guides/usage.md", &public("Using it", "Use it.")),
            ("notes/b.md", &public("A note", "First by title.")),
            ("notes/a.md", &public("Another note", "Then.")),
        ],
    )
    .await;
    exported(&rig, json!({ "collection": "docs" })).await;
    let site = dir.join("site");
    let read = |page: &str| std::fs::read_to_string(site.join(page)).unwrap();
    // The contents of a page: what each link of its navigation is called, in order.
    let contents = |page: &str| -> Vec<String> {
        let text = read(page);
        let nav =
            text.split("<nav class=\"page-nav\"").nth(1).unwrap_or_else(|| panic!("{page} has no contents"));
        let nav = nav.split("</nav>").next().unwrap();
        nav.split("\">")
            .skip(1)
            .filter_map(|rest| rest.split_once("</a>").map(|(words, _)| words.to_string()))
            .collect()
    };
    let wanted = [
        "Shop docs",
        "Zebra",
        "All the guides",
        "Using it",
        "Installing it",
        "About us",
        "notes",
        "A note",
        "Another note",
    ];
    // What is said comes first, the rest after by title, documents before
    // folders; a folder is called what its front page is; and every page
    // has the same contents, with nothing of what is not published.
    for page in [
        "index.html",
        "README.html",
        "zebra.html",
        "guides/index.html",
        "guides/install.html",
        "notes/index.html",
    ] {
        assert_eq!(contents(page), wanted, "{page}");
    }
    // The page showing is marked, a front page's being its folder, and
    // every way is from where the page is.
    let install = read("guides/install.html");
    assert!(
        install.contains("<a href=\"install.html\" aria-current=\"page\">Installing it</a>"),
        "{install}"
    );
    assert!(install.contains("<a href=\"../index.html\">Shop docs</a>"), "{install}");
    assert!(
        install.contains("<a href=\"../notes/index.html\" class=\"page-nav-folder\">notes</a>"),
        "{install}"
    );
    let guides = read("guides/index.html");
    assert!(
        guides.contains(
            "<a href=\"index.html\" class=\"page-nav-folder\" aria-current=\"page\">All the guides</a>"
        ),
        "{guides}"
    );
    assert!(read("index.html").contains("<a href=\"index.html\" aria-current=\"page\">Shop docs</a>"));
    assert!(read("README.html").contains("<a href=\"index.html\" aria-current=\"page\">Shop docs</a>"));
    // A folder with no front page lists its documents in the same order.
    let notes = read("notes/index.html");
    let main = notes.split("</main>").next().unwrap();
    assert!(main.find(">A note</a>").unwrap() < main.find(">Another note</a>").unwrap(), "{notes}");
    // On a phone the contents are under the page, and its head leads to them.
    assert!(
        install.contains("<a href=\"#contents\">Contents</a>") && install.contains("id=\"contents\""),
        "{install}"
    );
    assert!(install.contains("<body class=\"page navigated\">"), "{install}");
    assert!(!read("index.html").contains("The plan"));
}
