//! What the blueprints that ship work out for themselves, read by the
//! language and said for the gate that holds it (`plain`, in `xtask`).
//!
//! The gate is the tooling's, and the tooling is the standard library and
//! nothing else, so it cannot read an expression; the language can
//! (`blueprint::plain`). This is where the two meet: every text that ships
//! is read here, and each expression of one that is not plain is said on a
//! line of its own, which the gate counts against what it still lets
//! through. Run by itself it holds that every such text can be read.

use std::path::{Path, PathBuf};

use blueprint::plain::{Unplain, unplain};
use blueprint::syntax;

/// Before each line said for the gate: the file, the line, why, and the
/// expression as written, with a tab between.
const SAID: &str = "::plain ";
/// Before the line that says how many texts were read.
const READ: &str = "::plain-read ";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

/// The files of `dir` whose names end as told, by name.
fn files(dir: &Path, ending: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
        .unwrap_or_default();
    found.retain(|path| path.to_string_lossy().ends_with(ending));
    found.sort();
    found
}

/// The blueprints a package brings, each with the line of its manifest it
/// begins on: what is written under `blueprints:`, a text for each name.
fn brought(manifest: &str, file: &str) -> Vec<(String, u32, String)> {
    let tree = syntax::parse(manifest)
        .unwrap_or_else(|err| panic!("{file}: line {}: {}", err.span.start.line, err.message));
    let Some(entries) = tree.as_map() else { return Vec::new() };
    let Some(blueprints) = entries.iter().find(|entry| entry.key == "blueprints") else { return Vec::new() };
    blueprints
        .value
        .as_map()
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| {
            let text = entry.value.as_str()?;
            // A text written under `name: |` begins on the line after.
            Some((entry.key.clone(), entry.value.span.start.line, text.to_string()))
        })
        .collect()
}

fn say(file: &str, from: u32, found: Vec<Unplain>) {
    for one in found {
        println!("{SAID}{file}\t{}\t{}\t{}", from + one.line, one.why, one.source.replace(['\t', '\n'], " "));
    }
}

#[test]
fn the_expressions_of_what_ships_are_read_and_said_for_the_gate() {
    let root = root();
    let mut read = 0;
    for path in files(&root.join("blueprints/std"), ".bp") {
        let file = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(&path).unwrap();
        let found = unplain(&text).unwrap_or_else(|why| panic!("{file}: {why}"));
        say(&file, 0, found);
        read += 1;
    }
    let packages = std::fs::read_dir(root.join("extensions"))
        .map(|entries| entries.flatten().map(|entry| entry.path()).collect::<Vec<_>>())
        .unwrap_or_default();
    for package in packages {
        let path = package.join("extension.yaml");
        let Ok(manifest) = std::fs::read_to_string(&path) else { continue };
        let file = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        for (name, from, text) in brought(&manifest, &file) {
            let found = unplain(&text).unwrap_or_else(|why| panic!("{file}, the blueprint `{name}`: {why}"));
            say(&file, from, found);
            read += 1;
        }
    }
    assert!(read > 0, "no blueprint that ships was found under {}", root.display());
    println!("{READ}{read}");
}
