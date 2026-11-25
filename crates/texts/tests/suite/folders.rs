//! Where a project keeps blueprints of its own, as plain values: what is
//! asked, what is kept and read back, where the factory keeps them, and
//! what the nearest library that reads a project's own says.

use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use texts::library::folders::{Folder, Folders, PLACE, in_place, kept_at, refused, write_as};
use texts::library::{Layers, Library, MemoryLibrary, folder_of};

#[test]
fn what_is_asked_is_a_folder_of_the_repository_none_or_the_factorys_place() {
    assert_eq!(Folder::asked(&json!({ "folder": "tools/bp/" })), Ok(Folder::In("tools/bp".into())));
    // The folder a project from before read where it said nothing is a folder like any now.
    assert_eq!(Folder::asked(&json!({ "folder": "blueprints" })), Ok(Folder::In("blueprints".into())));
    assert_eq!(Folder::asked(&json!({ "none": true })), Ok(Folder::Nowhere));
    assert_eq!(Folder::asked(&json!({ "usual": true })), Ok(Folder::Usual));
    for wrong in [json!({}), json!({ "folder": "a", "none": true }), json!({ "none": true, "usual": true })] {
        assert!(Folder::asked(&wrong).unwrap_err().starts_with("say one of"), "{wrong}");
    }
    for (folder, says) in [
        ("", "is no folder of blueprints"),
        (".", "is no folder of blueprints"),
        ("/etc", "is not a path within the project"),
        ("@blueprints", "is not a path within the project"),
        ("a\\b", "is not a path within the project"),
        ("C:", "is not a path within the project"),
        ("../out", "does not stay within the project"),
        ("a/../../out", "does not stay within the project"),
        ("a//b", "does not stay within the project"),
    ] {
        assert!(refused(folder).is_some_and(|why| why.contains(says)), "{folder}: {:?}", refused(folder));
    }
    assert_eq!(refused(".mine/blueprints"), None);
}

#[test]
fn what_is_kept_is_read_back_as_it_was_and_the_usual_is_kept_as_nothing() {
    for folder in [Folder::In("tools/bp".into()), Folder::Nowhere] {
        assert_eq!(Folder::read(&folder.kept().unwrap()), folder);
    }
    assert_eq!(Folder::Usual.kept(), None);
    // What cannot be read is the usual.
    assert_eq!(Folder::read(&json!({ "folder": "../out" })), Folder::Usual);
    assert_eq!(Folder::read(&json!(3)), Folder::Usual);
    assert_eq!(Folder::Nowhere.shown(), json!({ "said": "none", "folder": null, "written": null }));
    assert_eq!(Folder::Usual.shown(), json!({ "said": "usual", "folder": null, "written": PLACE }));
    assert_eq!(
        Folder::In("bp".into()).shown(),
        json!({ "said": "folder", "folder": "bp", "written": PLACE })
    );
}

#[test]
fn each_project_writes_its_own_in_the_factorys_place_whatever_folder_of_its_repository_it_said() {
    let folders = Folders::default();
    folders.put("shop", Folder::Nowhere);
    folders.put("depot", Folder::In("bp".into()));
    assert_eq!(folders.blueprints_of("shop"), None);
    assert_eq!(folders.blueprints_of("depot").as_deref(), Some(PLACE));
    assert_eq!(folders.of("depot").repo(), Some("bp"));
    assert_eq!(folders.blueprints_of("other").as_deref(), Some(PLACE));
    assert_eq!(folders.of("other").repo(), None);
    folders.put("depot", Folder::Usual);
    assert_eq!(folders.of("depot"), Folder::Usual);
}

#[test]
fn the_factorys_place_is_in_its_data_folder_and_named_by_a_path_of_the_projects_that_begins_with_it() {
    assert_eq!(
        kept_at(Path::new("/data"), "shop"),
        Path::new("/data").join("blueprints").join("projects").join("shop")
    );
    assert_eq!(in_place("@blueprints/std/board.bp"), Some("std/board.bp"));
    assert_eq!(in_place("@blueprints"), Some(""));
    for not in ["blueprints/std/board.bp", "@blueprintsx/a.bp", "x/@blueprints/a.bp", "fleet.bp"] {
        assert_eq!(in_place(not), None, "{not}");
    }
}

#[test]
fn a_library_that_reads_no_projects_own_says_nothing_and_where_to_write_a_file_follows_the_project() {
    let layers = Layers::new().then(Arc::new(MemoryLibrary::new()));
    assert_eq!(layers.folder(Some("shop")), None);
    assert_eq!(folder_of(&layers, Some("shop")), Folder::Usual);
    assert_eq!(
        write_as(&Folder::Usual, "x/prompts/a.md", Some("shop")),
        "Write it as `@blueprints/x/prompts/a.md` in the project"
    );
    assert_eq!(
        write_as(&Folder::In("bp".into()), "a.md", Some("shop")),
        "Write it as `@blueprints/a.md` in the project"
    );
    let none = write_as(&Folder::Nowhere, "a.md", Some("shop"));
    assert!(
        none.starts_with("The project keeps no blueprints of its own")
            && none.ends_with("project blueprints shop --usual`"),
        "{none}"
    );
}
