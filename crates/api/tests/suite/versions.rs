//! The texts that ship say their version, and a version is one meaning.
//!
//! Somebody whose factory goes on with the blueprints it was applied with
//! is told by version what a later program ships (`std/lane-round 3 to
//! 4`), so a number has to mean one text: the same number over two
//! meanings would say nothing moved when something did. What each version
//! of each text comes to, layout and comments aside, is written down the
//! way the contract is (`contract/versions.txt`, and beside a package's
//! contract).
//!
//! The words a blueprint links (`prompts/`, a Markdown file) ship too, and
//! each has its versions in the same file, a line each. Such a file has
//! nowhere to say its number, so the file of versions says it: its
//! version is the last of its lines, and what a line holds is the words
//! themselves, every one of them, since there is no layout to set aside.
//! A prompt that changes is another version of the prompt and not of the
//! blueprint that links it, whose own text is as it was.
//!
//! Whoever changes what a text that ships means does one thing:
//!
//!   WRITE_VERSIONS=1 cargo test -p api the_versions
//!
//! which gives each text that means something else than its number did
//! the next number, in the text itself, writes its line, and gives a text
//! that says no version its first. The text and the file are committed
//! together. Run without it, the test fails on such a text and says that
//! one command. It never changes a line that is written: a version that
//! was given out stays what it was.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What a text comes to: the same for two that differ only in how they
/// are laid out and in their comments.
fn comes_to(text: &str) -> String {
    blueprint::revision(&blueprint::meaning(text).expect("a text that ships can be read"))
}

/// A text that ships: its name, where it is kept, and the text.
struct Shipped {
    name: String,
    /// The blueprint's own file, or the manifest of the package that brings it.
    file: PathBuf,
    text: String,
}

impl Shipped {
    /// Whether it is words a blueprint links, and no blueprint.
    fn is_words(&self) -> bool {
        conductor::library::is_linked(&self.name)
    }
}

/// Every file under `folder` that a blueprint could link, by its path from there.
fn words_under(folder: &Path, within: &str, found: &mut Vec<(String, PathBuf)>) {
    for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let (file, name) = (entry.path(), entry.file_name().to_string_lossy().into_owned());
        match file.is_dir() {
            true => words_under(&file, &format!("{within}{name}/"), found),
            false if name.ends_with(conductor::words::KIND) => found.push((format!("{within}{name}"), file)),
            false => {}
        }
    }
}

/// What is to be done about the words of a file whose versions are
/// `written`, the last of them last, and which now come to `came_to`:
/// nothing where the last version is these words, and otherwise the number
/// they are to be. Words put back as an earlier version had them are a
/// new version all the same: a number only goes up, and the last line is
/// what the file is.
fn next_words(written: &[(u64, String)], came_to: &str) -> Option<u64> {
    match written.last() {
        Some((_, last)) if last == came_to => None,
        Some((last, _)) => Some(last + 1),
        None => Some(1),
    }
}

/// Every text that ships, by where its versions are written down.
fn shipped() -> BTreeMap<PathBuf, Vec<Shipped>> {
    let mut all: BTreeMap<PathBuf, Vec<Shipped>> = BTreeMap::new();
    let standard = repo().join("blueprints/std");
    for entry in std::fs::read_dir(&standard).unwrap().flatten() {
        let file = entry.path();
        let stem = file.file_name().and_then(|name| name.to_str()).and_then(|name| name.strip_suffix(".bp"));
        if let Some(stem) = stem {
            let (name, text) = (format!("std/{stem}"), std::fs::read_to_string(&file).unwrap());
            all.entry(repo().join("contract/versions.txt")).or_default().push(Shipped { name, file, text });
        }
    }
    // What they link, in `prompts/` beside them.
    let mut linked = Vec::new();
    words_under(&standard.join(conductor::words::FOLDER), "", &mut linked);
    for (within, file) in linked {
        let name = format!("std/{}/{within}", conductor::words::FOLDER);
        let text = std::fs::read_to_string(&file).unwrap();
        all.entry(repo().join("contract/versions.txt")).or_default().push(Shipped { name, file, text });
    }
    let (packages, wrong) = agents::extension::load_all(&repo().join("extensions"));
    assert!(wrong.is_empty(), "{wrong:?}");
    for (package, extension) in packages {
        let folder = repo().join("extensions").join(&package);
        let (kept, file) = (folder.join("contract/versions.txt"), folder.join(agents::MANIFEST));
        // Its blueprints, and the files they link, which it brings under `prompts:`.
        for (name, text) in extension.blueprints.iter().chain(&extension.prompts) {
            let one = Shipped { name: name.clone(), file: file.clone(), text: text.clone() };
            all.entry(kept.clone()).or_default().push(one);
        }
    }
    all.values_mut().for_each(|texts| texts.sort_by(|one, other| one.name.cmp(&other.name)));
    all
}

/// Gives the text of `one` the number `version` where it is kept, and
/// hands back the text as it then is. In a file of its own the line comes
/// after the blueprint's name; in a package's manifest the same, set in
/// as the text is.
fn number(one: &Shipped, version: u64) -> String {
    let file = std::fs::read_to_string(&one.file).unwrap();
    let mut lines: Vec<String> = file.lines().map(str::to_string).collect();
    let named = |line: &str| line.trim_start() == format!("blueprint: {}", one.name);
    let at = lines
        .iter()
        .position(|line| named(line))
        .unwrap_or_else(|| panic!("`{}` is not named in its file", one.name));
    let set_in = lines[at].len() - lines[at].trim_start().len();
    let said = format!("{}version: {version}", " ".repeat(set_in));
    match lines.get(at + 1).is_some_and(|next| next.trim_start().starts_with("version:")) {
        true => lines[at + 1] = said,
        false => lines.insert(at + 1, said),
    }
    std::fs::write(&one.file, lines.join("\n") + "\n").unwrap();
    // The text as it now is: its own lines, with what a manifest set it in by taken off.
    let own = |line: &String| line.get(set_in..).unwrap_or_default().to_string();
    let mut text: Vec<String> = match set_in {
        0 => lines,
        _ => lines[at..]
            .iter()
            .take_while(|line| line.trim().is_empty() || line.len() - line.trim_start().len() >= set_in)
            .map(own)
            .collect(),
    };
    while text.last().is_some_and(|line| line.trim().is_empty()) {
        text.pop();
    }
    text.join("\n") + "\n"
}

const HEAD: &str = "\
# The versions of the blueprints that ship here: each version of each, and
# what its text comes to, layout and comments aside. A number is one
# meaning. Written by a test, never by hand. After changing what a text
# that ships means:
#   WRITE_VERSIONS=1 cargo test -p api the_versions
# gives the text its next number, in the text, and writes its line here.
# Commit both. A line is never changed, and is taken out by hand only for
# a version nobody was ever given.
#
# The files they link (prompts/, what an agent is told) have their lines
# here too. Such a file says no number: its version is the last of its
# lines, and the same command writes the next when its words change. A
# prompt that changes is not another version of the blueprint that links it.
";

/// What is written down: for each name, each version and what it came to.
fn written(kept: &Path) -> BTreeMap<String, Vec<(u64, String)>> {
    let mut all: BTreeMap<String, Vec<(u64, String)>> = BTreeMap::new();
    let text = std::fs::read_to_string(kept).unwrap_or_default();
    for line in text.lines().filter(|line| !line.starts_with('#') && !line.trim().is_empty()) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let [name, version, came_to] = parts[..] else {
            panic!("{}: not a line of it: {line}", kept.display())
        };
        all.entry(name.to_string()).or_default().push((version.parse().unwrap(), came_to.to_string()));
    }
    all
}

/// What is said when the test fails: the one thing to run.
const RUN: &str = "Run `WRITE_VERSIONS=1 cargo test -p api the_versions`: it gives each such text its next number, in the \
                   text (a prompt's is written in the versions file alone), and writes its line. Commit the texts and \
                   the versions file together.";

#[test]
fn the_versions_of_the_texts_that_ship_are_each_one_meaning() {
    let write = std::env::var_os("WRITE_VERSIONS").is_some();
    let mut wrong = Vec::new();
    for (kept, texts) in shipped() {
        let mut known = written(&kept);
        let mut added = false;
        for one in &texts {
            let name = &one.name;
            let versions = known.entry(name.clone()).or_default();
            // Words a blueprint links: the file of versions says which they are.
            if one.is_words() {
                let came_to = blueprint::revision(&one.text);
                match (next_words(versions, &came_to), write) {
                    (None, _) => {}
                    (Some(next), true) => {
                        versions.push((next, came_to));
                        added = true;
                    }
                    (Some(1), false) => wrong.push(format!("`{name}` ships and its words are not written down")),
                    (Some(next), false) => wrong.push(format!(
                        "the words of `{name}` are other than its version {} has them; they are to be version {next}",
                        next - 1
                    )),
                }
                continue;
            }
            let last = versions.iter().map(|(written, _)| *written).max();
            let says = blueprint::version(&one.text).and_then(|said| said.parse::<u64>().ok());
            let came_to = comes_to(&one.text);
            // As it is written down: nothing to do.
            if says.is_some_and(|says| versions.contains(&(says, came_to.clone()))) {
                continue;
            }
            // Otherwise it is new, says no version, or means something else than its number did.
            let next = last.map_or(1, |last| last + 1).max(says.unwrap_or(1));
            let taken = says.is_some_and(|says| versions.iter().any(|(written, _)| *written == says));
            if write {
                let now = match (says, taken) {
                    (Some(says), false) if last.is_none_or(|last| says > last) => (says, came_to),
                    _ => (next, comes_to(&number(one, next))),
                };
                versions.push(now);
                added = true;
                continue;
            }
            wrong.push(match (says, taken) {
                (None, _) => format!("`{name}` ships and does not say which version it is"),
                (Some(says), true) => format!(
                    "`{name}` says version {says} and means something else than version {says} did; it is to be version {next}"
                ),
                (Some(says), false) if last.is_some_and(|last| says < last) => {
                    format!("`{name}` says version {says}, and version {} is written down: a version only goes up", last.unwrap_or(says))
                }
                (Some(says), false) => format!(
                    "version {says} of `{name}` is not written down ({})",
                    kept.strip_prefix(repo()).unwrap_or(&kept).display()
                ),
            });
        }
        if added {
            let mut out = HEAD.to_string();
            for (name, versions) in &known {
                out.push('\n');
                for (version, came_to) in versions {
                    out.push_str(&format!("{name} {version} {came_to}\n"));
                }
            }
            std::fs::create_dir_all(kept.parent().unwrap()).unwrap();
            std::fs::write(&kept, out).unwrap();
        }
    }
    assert!(wrong.is_empty(), "\n{}\n{RUN}", wrong.join("\n"));
}

#[test]
fn giving_a_text_its_number_writes_the_line_after_its_name_wherever_the_text_is_kept() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/api-versions");
    std::fs::create_dir_all(&dir).unwrap();
    // A blueprint in a file of its own, which says no version yet; and the same asked twice.
    let file = dir.join("shop.bp");
    std::fs::write(&file, "# A shop.\nblueprint: std/shop\nsteps:\n  go: { do: noop }\n").unwrap();
    let one = |text: &str| Shipped { name: "std/shop".into(), file: file.clone(), text: text.into() };
    let first = number(&one(""), 1);
    assert_eq!(first, "# A shop.\nblueprint: std/shop\nversion: 1\nsteps:\n  go: { do: noop }\n");
    assert_eq!(number(&one(&first), 2), first.replace("version: 1", "version: 2"));
    // One a package brings, set in under its name in the manifest.
    let manifest = dir.join("extension.yaml");
    let before = "extension: reach\nblueprints:\n  open: |\n    blueprint: reach/open\n    version: 3\n    steps:\n      knock: { do: noop }\nactions: {}\n";
    std::fs::write(&manifest, before).unwrap();
    let brought = Shipped { name: "reach/open".into(), file: manifest.clone(), text: String::new() };
    assert_eq!(number(&brought, 4), "blueprint: reach/open\nversion: 4\nsteps:\n  knock: { do: noop }\n");
    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), before.replace("version: 3", "version: 4"));
}

#[test]
fn laying_a_text_out_or_writing_a_comment_in_it_is_not_another_version() {
    let text = "blueprint: shop\nversion: 3\nsteps:\n  go: { do: set, with: { done: true } }\n";
    let laid = "# What a shop does.\nblueprint: shop\nversion: 3\n\nsteps:\n  go:\n    do: set\n    with: { done: true }\n";
    assert_eq!(comes_to(text), comes_to(laid));
    assert_ne!(comes_to(text), comes_to(&text.replace("done: true", "done: false")));
}

#[test]
fn the_words_a_blueprint_links_are_the_version_the_last_of_their_lines_says() {
    let lines = |written: &[(u64, &str)]| -> Vec<(u64, String)> {
        written.iter().map(|(version, came_to)| (*version, came_to.to_string())).collect()
    };
    // Never written down, they are the first; as the last line has them, there is nothing to do.
    assert_eq!(next_words(&[], "aaaa"), Some(1));
    assert_eq!(next_words(&lines(&[(1, "aaaa")]), "aaaa"), None);
    assert_eq!(next_words(&lines(&[(1, "aaaa"), (2, "bbbb")]), "bbbb"), None);
    // Other words are the next number, and so are words put back as an earlier version had them.
    assert_eq!(next_words(&lines(&[(1, "aaaa")]), "bbbb"), Some(2));
    assert_eq!(next_words(&lines(&[(1, "aaaa"), (2, "bbbb")]), "aaaa"), Some(3));
    // Every word counts: there is no layout to set aside, as there is in a blueprint.
    assert_ne!(blueprint::revision("Look at ${item}.\n"), blueprint::revision("Look  at ${item}.\n"));
    // Which a text is, is told by its name: a file of a `prompts/`, and no blueprint.
    assert!(
        conductor::library::is_linked("std/prompts/review-seat.md")
            && !conductor::library::is_linked("std/review")
    );
}
