//! A library: a collection whose records are the Markdown files of a
//! folder, as they are.

use std::path::PathBuf;

use kernel::ErrorKind;
use serde_json::{Map, Value, json};
use studio::collections::Spec;

use crate::{Rig, ada};

fn data(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

/// A project `shop` with a library called `docs` over the folder `folder`
/// of it, whose documents have tags and a number of readers.
async fn library(folder: Option<&str>) -> (Rig, PathBuf) {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let dir = rig.studio.projects.dir_of("shop").unwrap();
    // A new project is given a first document or two; these tests say what is in the folder themselves.
    let _ = std::fs::remove_dir_all(dir.join("docs"));
    declare(&rig, folder).await;
    (rig, dir)
}

async fn declare(rig: &Rig, folder: Option<&str>) -> studio::Collection {
    let fields = json!({
        "title": { "type": "text", "required": true },
        "body": { "type": "text" },
        "path": { "type": "text" },
        "tags": { "type": "list" },
        "readers": { "type": "number" },
    });
    let history = json!({
        "record.created": "{actor} wrote it",
        "record.updated": "{actor} changed its {changed}",
        "record.commented": "{actor} said: {body}",
        "record.removed": "{actor} took it away",
    });
    let spec = Spec::read(&fields, &json!(null), Some("documents"))
        .unwrap()
        .kept_at(folder)
        .with_history(&history)
        .unwrap();
    rig.studio.collections.ensure("shop", "docs", spec, None, &ada()).await.unwrap()
}

fn write(dir: &std::path::Path, path: &str, text: &str) {
    let file = dir.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, text).unwrap();
}

#[tokio::test]
async fn a_library_is_the_markdown_files_of_its_folder_as_they_are_each_known_by_where_it_is() {
    let (rig, dir) = library(None).await;
    write(&dir, "docs/welcome.md", "# Welcome\n\nStart here.\n");
    write(&dir, "docs/guides/install.md", "# Installing it\n\nRun the installer.\n");
    write(&dir, "docs/notes.md", "Just some words, with no heading.\n");
    write(
        &dir,
        "docs/guides/deep/tagged.md",
        "---\ntags: [how-to, first steps]\nlayout: wide\nreaders: 12\n---\n\n# Tagged\n\nBody.\n",
    );
    // What is not a document is not in the library, wherever it is.
    write(&dir, "docs/picture.png", "not a picture");
    write(&dir, "docs/data.csv", "a,b\n");
    write(&dir, "elsewhere.md", "# Not in the folder\n");
    write(&dir, "docs/target/built.md", "# In a folder that is never listed\n");

    let records = rig.studio.records.list("shop", "docs", &ada()).await.unwrap();
    let ids: Vec<&str> = records.iter().map(|record| record.id.as_str()).collect();
    assert_eq!(
        ids,
        ["shop-docs:guides:deep:tagged", "shop-docs:guides:install", "shop-docs:notes", "shop-docs:welcome"],
        "folders before files, each by its path with `:` between its parts"
    );
    let by = |id: &str| records.iter().find(|record| record.id == id).unwrap();
    let welcome = by("shop-docs:welcome");
    assert_eq!(welcome.data["title"], json!("Welcome"));
    assert_eq!(welcome.data["body"], json!("Start here."));
    assert_eq!(welcome.data["path"], json!("welcome.md"));
    assert_eq!(welcome.data["tags"], json!(null), "a file with no block of settings has none of them");
    // A document that begins with no heading is called what its file is.
    let notes = by("shop-docs:notes");
    assert_eq!(notes.data["title"], json!("notes"));
    assert_eq!(notes.data["body"], json!("Just some words, with no heading."));
    // What its collection declares is read from the block at its head,
    // forgivingly; what nobody declared (`layout`) is nobody's field.
    let tagged = by("shop-docs:guides:deep:tagged");
    assert_eq!(tagged.data["tags"], json!(["how-to", "first steps"]));
    assert_eq!(tagged.data["readers"], json!(12));
    assert_eq!(tagged.data["path"], json!("guides/deep/tagged.md"));
    assert!(!tagged.data.contains_key("layout"));

    // One is found by what it is known by, and that is the same after
    // everything is opened again over the same files.
    let found = rig.studio.records.get("shop-docs:guides:install", &ada()).await.unwrap();
    assert_eq!(found.data["title"], json!("Installing it"));
    let gone = rig.studio.records.get("shop-docs:guides:nothing", &ada()).await.unwrap_err();
    assert_eq!(gone.kind, ErrorKind::NotFound);
    for outside in ["shop-docs:..:elsewhere", "shop-docs:", "shop-docs:a/b"] {
        let refused = rig.studio.records.get(outside, &ada()).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::NotFound, "{outside}");
    }
}

#[tokio::test]
async fn a_document_is_written_changed_and_taken_away_through_the_library_and_its_file_says_so() {
    let (rig, dir) = library(None).await;
    let records = &rig.studio.records;
    let mut written = rig.journal.follow(0, studio::history::TOPIC);

    // Named by its title, unless it is said where it goes.
    let made = json!({ "title": "Getting Started", "body": "First, read this." });
    let made = records.add("shop", "docs", data(made), &ada(), None).await.unwrap();
    assert_eq!(made.id, "shop-docs:getting-started");
    assert_eq!(made.created_by, "user:ada");
    let text = std::fs::read_to_string(dir.join("docs/getting-started.md")).unwrap();
    assert_eq!(
        text, "# Getting Started\n\nFirst, read this.\n",
        "no block of settings where there is nothing for one"
    );
    let placed = json!({ "title": "Install", "path": "guides/install", "tags": ["how-to"] });
    let placed = records.add("shop", "docs", data(placed), &ada(), None).await.unwrap();
    assert_eq!(placed.id, "shop-docs:guides:install");
    let text = std::fs::read_to_string(dir.join("docs/guides/install.md")).unwrap();
    assert_eq!(text, "---\ntags: [\"how-to\"]\n---\n\n# Install\n");

    // One that is there is not written over; one with nothing to be named by is refused.
    let again = records.add("shop", "docs", data(json!({ "title": "Getting started" })), &ada(), None).await;
    let again = again.unwrap_err();
    assert_eq!(again.kind, ErrorKind::Conflict);
    assert!(
        again.message.contains("there is already a document at `docs/getting-started.md`"),
        "{}",
        again.message
    );
    for bad in ["../outside", "a:b", "/etc/passwd"] {
        let refused = json!({ "title": "Somewhere else", "path": bad });
        let refused = records.add("shop", "docs", data(refused), &ada(), None).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{bad}: {}", refused.message);
    }

    // Changing one writes its file again, keeping the lines at its head
    // that are nobody's field here.
    std::fs::write(
        dir.join("docs/guides/install.md"),
        "---\nlayout: wide\ntags: [\"how-to\"]\n---\n\n# Install\n\nOld words.\n",
    )
    .unwrap();
    let changed = json!({ "body": "New words.", "tags": [], "readers": 3 });
    let changed = records.update(&placed.id, data(changed), &ada(), None).await.unwrap();
    assert_eq!(changed.data["readers"], json!(3));
    let text = std::fs::read_to_string(dir.join("docs/guides/install.md")).unwrap();
    assert_eq!(text, "---\nlayout: wide\nreaders: 3\n---\n\n# Install\n\nNew words.\n");

    // A document is where its file is: it is not moved by saying so.
    let moved = records.update(&placed.id, data(json!({ "path": "guides/setup.md" })), &ada(), None).await;
    let moved = moved.unwrap_err();
    assert_eq!(moved.kind, ErrorKind::Invalid);
    assert!(moved.message.contains("Move the file itself"), "{}", moved.message);

    // Something is said on it, and it is taken away; each is a line of
    // its history, which outlasts the file.
    records.comment(&placed.id, "Does this cover Windows?", false, &ada(), None).await.unwrap();
    records.remove(&placed.id, &ada(), None).await.unwrap();
    assert!(!dir.join("docs/guides/install.md").exists());
    for _ in 0..4 {
        written.next().await.unwrap();
    }
    // The first line written was the other document's.
    written.next().await.unwrap();
    let history = rig.studio.collections.history(&placed.id).await.unwrap();
    let said: Vec<&str> = history.iter().map(|line| line.said.as_str()).collect();
    assert_eq!(
        said,
        [
            "ada wrote it",
            "ada changed its body, readers, tags",
            "ada said: Does this cover Windows?",
            "ada took it away"
        ]
    );
}

#[tokio::test]
async fn a_document_that_began_with_no_heading_is_given_none_for_being_called_what_its_file_is() {
    let (rig, dir) = library(None).await;
    write(&dir, "docs/notes.md", "Words with no heading.\n");
    let records = &rig.studio.records;
    let tagged =
        records.update("shop-docs:notes", data(json!({ "tags": ["rough"] })), &ada(), None).await.unwrap();
    assert_eq!(tagged.data["title"], json!("notes"));
    let text = std::fs::read_to_string(dir.join("docs/notes.md")).unwrap();
    assert_eq!(text, "---\ntags: [\"rough\"]\n---\n\nWords with no heading.\n");
    // Given a title of its own, it has a heading from then on.
    records.update("shop-docs:notes", data(json!({ "title": "Rough notes" })), &ada(), None).await.unwrap();
    let text = std::fs::read_to_string(dir.join("docs/notes.md")).unwrap();
    assert_eq!(text, "---\ntags: [\"rough\"]\n---\n\n# Rough notes\n\nWords with no heading.\n");
}

#[tokio::test]
async fn a_library_is_of_the_folder_it_is_said_to_be_of_and_never_of_one_outside_the_project() {
    let (rig, dir) = library(Some("notes/design")).await;
    write(&dir, "notes/design/plan.md", "# The plan\n");
    write(&dir, "docs/other.md", "# In another folder\n");
    let records = rig.studio.records.list("shop", "docs", &ada()).await.unwrap();
    assert_eq!(records.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(), ["shop-docs:plan"]);

    // Declared again without saying, it stays where it was; said again, it is of the other folder.
    assert_eq!(declare(&rig, None).await.at.as_deref(), Some("notes/design"));
    assert_eq!(declare(&rig, Some("./docs/")).await.at.as_deref(), Some("docs"));
    let records = rig.studio.records.list("shop", "docs", &ada()).await.unwrap();
    assert_eq!(records.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(), ["shop-docs:other"]);

    // The project's own folder is `.`.
    assert_eq!(declare(&rig, Some(".")).await.at.as_deref(), Some("."));
    let ids: Vec<String> = rig
        .studio
        .records
        .list("shop", "docs", &ada())
        .await
        .unwrap()
        .into_iter()
        .map(|record| record.id)
        .collect();
    // With whatever else the project holds: a new one has a page or two of its own.
    for id in ["shop-docs:docs:other", "shop-docs:notes:design:plan", "shop-docs:README"] {
        assert!(ids.iter().any(|one| one == id), "{id} is not among {ids:?}");
    }

    let fields = json!({ "title": { "type": "text" } });
    for outside in ["../elsewhere", "/etc", "docs/../../up", ".git/hooks"] {
        let spec = Spec::read(&fields, &json!(null), Some("documents")).unwrap().kept_at(Some(outside));
        let refused = rig.studio.collections.ensure("shop", "other", spec, None, &ada()).await.unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Invalid, "{outside}");
        assert!(refused.message.contains("is not a folder of the project"), "{}", refused.message);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_link_in_a_librarys_folder_that_leads_out_of_it_is_not_a_document_of_it() {
    let (rig, dir) = library(None).await;
    write(&dir, "docs/inside.md", "# Inside\n");
    write(&dir, "secret.md", "# Not the library's\n");
    std::os::unix::fs::symlink(dir.join("secret.md"), dir.join("docs/link.md")).unwrap();
    let records = rig.studio.records.list("shop", "docs", &ada()).await.unwrap();
    assert_eq!(records.iter().map(|record| record.id.as_str()).collect::<Vec<_>>(), ["shop-docs:inside"]);
    assert!(rig.studio.records.get("shop-docs:link", &ada()).await.is_err());
}

#[tokio::test]
async fn a_file_written_through_the_daemon_is_said_of_the_document_it_is_and_one_written_back_as_it_was_is_not()
 {
    let (rig, dir) = library(None).await;
    let records = &rig.studio.records;
    let projects = &rig.studio.projects;
    let mut written = rig.journal.follow(0, studio::history::TOPIC);

    let first = "# Welcome\n\nStart here.\n";
    projects.write("shop", "docs/welcome.md", first, &ada()).await.unwrap();
    records.file_written("shop", "docs/welcome.md", None, first, &ada()).await;
    written.next().await.unwrap();
    // Exactly as it was: nothing happened to the document.
    records.file_written("shop", "docs/welcome.md", Some(first), first, &ada()).await;
    let second = "# Welcome back\n\nStart here.\n";
    projects.write("shop", "docs/welcome.md", second, &ada()).await.unwrap();
    records.file_written("shop", "docs/welcome.md", Some(first), second, &ada()).await;
    written.next().await.unwrap();
    // A file that is no document's says nothing.
    records.file_written("shop", "README.md", None, "# Not in the library\n", &ada()).await;
    records.file_written("shop", "docs/data.csv", None, "a,b\n", &ada()).await;
    records.file_removed("shop", "docs/welcome.md", &ada()).await;
    written.next().await.unwrap();

    let history = rig.studio.collections.history("shop-docs:welcome").await.unwrap();
    let said: Vec<&str> = history.iter().map(|line| line.said.as_str()).collect();
    assert_eq!(said, ["ada wrote it", "ada changed its title", "ada took it away"]);
    assert_eq!(rig.topics("record.*").await.len(), 3, "and nothing was said of anything else");
    let _ = dir;
}

#[tokio::test]
async fn a_documents_file_written_through_the_daemon_says_what_its_changed_choice_and_flag_now_say() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let dir = rig.studio.projects.dir_of("shop").unwrap();
    let _ = std::fs::remove_dir_all(dir.join("docs"));
    let fields = json!({
        "title": { "type": "text", "required": true },
        "body": { "type": "text" },
        "path": { "type": "text" },
        "stage": { "type": "choice", "of": ["draft", "final"] },
        "public": { "type": "flag" },
    });
    let spec = Spec::read(&fields, &json!(null), Some("documents")).unwrap();
    rig.studio.collections.ensure("shop", "docs", spec, None, &ada()).await.unwrap();
    let (records, projects) = (&rig.studio.records, &rig.studio.projects);
    let save = async |was: Option<&str>, now: &str| {
        projects.write("shop", "docs/plan.md", now, &ada()).await.unwrap();
        records.file_written("shop", "docs/plan.md", was, now, &ada()).await;
    };

    let first = "---\nstage: draft\n---\n# Plan\n\nWhat is to be done.\n";
    save(None, first).await;
    // Marked public and made final in the file's own head, as somebody who edits the text does.
    let second = "---\nstage: final\npublic: true\n---\n# Plan\n\nWhat is to be done.\n";
    save(Some(first), second).await;
    // Only its words: a change, and neither a choice's nor a flag's.
    let third = "---\nstage: final\npublic: true\n---\n# Plan\n\nWhat was done.\n";
    save(Some(second), third).await;
    // The flag's line taken out: it says nothing now, and is said to have changed to that.
    let fourth = "---\nstage: final\n---\n# Plan\n\nWhat was done.\n";
    save(Some(third), fourth).await;

    let said: Vec<Value> = rig.events("record.updated").await.into_iter().map(|event| event.data).collect();
    assert_eq!(said.len(), 3, "{said:?}");
    assert_eq!(said[0]["id"], "shop-docs:plan");
    assert_eq!(said[0]["chose"], json!({ "stage": "final" }));
    assert_eq!(said[0]["flags"], json!({ "public": true }));
    assert_eq!(said[1]["changed"], json!(["body"]));
    assert!(said[1].get("chose").is_none() && said[1].get("flags").is_none(), "{}", said[1]);
    assert_eq!(said[2]["flags"], json!({ "public": null }));
    assert!(said[2].get("chose").is_none(), "{}", said[2]);
}
