use std::path::PathBuf;

use kernel::ErrorKind;
use projects::{LONGEST_NAME, Roots, holds, named, templates};

/// A project's folder, made, and the factory's place for its own, not made
/// yet: nothing makes that until something is written there.
fn roots(called: &str) -> Roots {
    let top = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("projects-suite").join(called);
    let _ = std::fs::remove_dir_all(&top);
    std::fs::create_dir_all(top.join("shop")).unwrap();
    let dir = kernel::whole::whole(&top.join("shop")).unwrap();
    Roots { place: dir.parent().unwrap().join("data/blueprints/projects/shop"), dir }
}

#[test]
fn a_path_that_begins_at_the_place_for_its_own_blueprints_is_in_the_factorys_folder_and_not_the_projects() {
    let roots = roots("placed");
    let own = roots.resolve("@blueprints/fleet.bp").unwrap();
    assert!(own.starts_with(&roots.place), "{}", own.display());
    assert_eq!(roots.called(&own).as_deref(), Some("@blueprints/fleet.bp"));
    let theirs = roots.resolve("docs/overview.md").unwrap();
    assert!(theirs.starts_with(&roots.dir), "{}", theirs.display());
    assert_eq!(roots.called(&theirs).as_deref(), Some("docs/overview.md"));
}

#[test]
fn neither_folder_is_a_file_and_nothing_leads_out_of_either() {
    let roots = roots("tops");
    assert!(roots.is_top(&roots.dir.clone()));
    assert!(roots.is_top(&roots.place));
    assert_eq!(roots.resolve("@blueprints").unwrap_err().kind, ErrorKind::Invalid);
    assert!(roots.resolve("../elsewhere").is_err());
    assert!(roots.resolve("@blueprints/../../elsewhere").is_err());
}

#[test]
fn a_folder_holds_what_is_under_it_and_nothing_beside_it() {
    assert!(holds("notes", "notes/private"));
    assert!(holds("notes/private", "notes/private"));
    assert!(holds("", "notes"));
    assert!(!holds("notes/private", "notes"));
    assert!(!holds("note", "notes"));
}

#[test]
fn a_project_is_called_one_line_of_words_and_no_longer_than_its_most() {
    assert_eq!(named("  Shop front ").unwrap(), "Shop front");
    assert_eq!(named(" ").unwrap_err().kind, ErrorKind::Invalid);
    assert_eq!(named("two\nlines").unwrap_err().kind, ErrorKind::Invalid);
    assert!(named(&"a".repeat(LONGEST_NAME)).is_ok());
    assert_eq!(named(&"a".repeat(LONGEST_NAME + 1)).unwrap_err().kind, ErrorKind::Invalid);
}

#[test]
fn a_new_project_is_given_its_first_documents_only_where_it_is_to_have_them() {
    let with: Vec<String> = templates::project("Shop", true).into_iter().map(|(path, _)| path).collect();
    assert!(with.iter().any(|path| path == "docs/overview.md"), "{with:?}");
    let without: Vec<String> = templates::project("Shop", false).into_iter().map(|(path, _)| path).collect();
    assert_eq!(without, ["README.md"]);
    let fleet = templates::fleet("Shop front", "shop");
    assert!(
        fleet.contains("shop") && !fleet.contains("__project__") && !fleet.contains("__name__"),
        "{fleet}"
    );
}
