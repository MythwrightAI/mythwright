//! A blueprint's file may be called more than one way.
//!
//! `slot.bp` is the blueprint `slot`, and so is `slot.bp.yml` or
//! `slot.bp.yaml`: a blueprint is a strict subset of YAML, and under that
//! name every editor takes it for what it is. Never `slot.yml` alone,
//! which is anybody's file. A folder that holds two of them for one name
//! has said which is meant with neither.

use std::path::PathBuf;
use std::sync::Arc;

use texts::library::{DirLibrary, Layers, Library, MemoryLibrary, Named, named};

fn folder(label: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("named-{label}-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(dir.join("std")).unwrap();
    dir
}

fn text(name: &str) -> String {
    format!("blueprint: {name}\n")
}

#[test]
fn a_blueprint_is_found_under_any_of_the_names_its_file_may_have_and_never_as_yaml_alone() {
    let dir = folder("found");
    std::fs::write(dir.join("fleet.bp"), text("fleet")).unwrap();
    std::fs::write(dir.join("release.bp.yml"), text("release")).unwrap();
    std::fs::write(dir.join("std/slot.bp.yaml"), text("std/slot")).unwrap();
    // Somebody else's files, which a project is full of.
    std::fs::write(dir.join("compose.yml"), "services: {}\n").unwrap();
    std::fs::write(dir.join("ci.yaml"), "on: push\n").unwrap();
    std::fs::write(dir.join(".bp.yml"), text("nameless")).unwrap();

    let library = DirLibrary::new(&dir, "bp");
    assert_eq!(library.names(None), ["fleet", "release", "std/slot"]);
    for name in ["fleet", "release", "std/slot"] {
        assert_eq!(library.load(name, None), Some(text(name)), "{name}");
        assert_eq!(library.refused(name, None), None, "{name}");
    }
    assert_eq!(library.load("compose", None), None);
    assert_eq!(library.load("ci", None), None);
    // What its file is called, for whoever is told where in it something is.
    assert_eq!(library.called("release", None).as_deref(), Some("release.bp.yml"));
    assert_eq!(library.called("std/slot", None).as_deref(), Some("std/slot.bp.yaml"));
    assert_eq!(named(&dir, "fleet", "bp"), Named::File("fleet.bp".into()));
    assert_eq!(named(&dir, "nobody", "bp"), Named::Nothing);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_name_with_two_files_is_read_from_neither_and_nothing_behind_it_stands_in() {
    let dir = folder("twice");
    std::fs::write(dir.join("std/slot.bp"), text("one")).unwrap();
    std::fs::write(dir.join("std/slot.bp.yml"), text("another")).unwrap();
    let files = DirLibrary::new(&dir, "bp");

    assert_eq!(files.load("std/slot", None), None);
    assert_eq!(files.names(None), ["std/slot"], "it is one name, however many files it has");
    let why = files.refused("std/slot", None).unwrap();
    assert!(
        why.contains("more than one file for the blueprint `std/slot`")
            && why.contains("`std/slot.bp` and `std/slot.bp.yml`")
            && why.contains("Take all but one away"),
        "{why}"
    );

    // Behind it is one that ships under the name. It is not what was
    // meant by either file, and is not given in their place.
    let shipped = MemoryLibrary::new();
    shipped.put("std/slot", &text("shipped"));
    shipped.put("std/other", &text("other"));
    let layers = Layers::new().then(Arc::new(files)).then(Arc::new(shipped));
    assert_eq!(layers.load("std/slot", None), None);
    assert_eq!(layers.refused("std/slot", None), Some(why));
    assert_eq!(layers.load("std/other", None), Some(text("other")));
    assert_eq!(layers.refused("std/other", None), None);

    // One of them taken away, the other is the blueprint.
    std::fs::remove_file(dir.join("std/slot.bp")).unwrap();
    assert_eq!(layers.load("std/slot", None), Some(text("another")));
    assert_eq!(layers.refused("std/slot", None), None);
    let _ = std::fs::remove_dir_all(dir);
}
