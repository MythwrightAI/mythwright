//! A blueprint that ships only gets shorter.
//!
//! A long text is what the fleet was cut out of: one stage of sixteen
//! hundred lines that nobody could change a part of. What it does is now
//! small texts, each one thing, with what is decided asked of a block and
//! what is gone through a list of rows. That holds only while nothing
//! grows back, and a text grows a step at a time, each one reasonable.
//!
//! So how long each text that ships is, is written down
//! (`contract/lines.txt`, a line each: its name and its lines), and a
//! text that is longer than its line fails here. What was to be added is
//! a block's to decide, a text of its own, or a row. One that is shorter
//! is held to the shorter from then on, and a new text is held to what it
//! is first written at:
//!
//!   WRITE_LINES=1 cargo test -p api the_lines
//!
//! writes both, and takes away the line of a text that no longer ships.
//! It never writes a text as longer. Where one has to be, somebody says
//! why, and the reason is kept beside what it was held to before
//! (`contract/grown.txt`):
//!
//!   GROW_LINES="why" cargo test -p api the_lines

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const KEPT: &str = "contract/lines.txt";
const GROWN: &str = "contract/grown.txt";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every blueprint that ships with the program, by its name, with how
/// many lines its file is.
fn shipped() -> BTreeMap<String, usize> {
    let folder = repo().join("blueprints/std");
    let mut found = BTreeMap::new();
    for entry in std::fs::read_dir(&folder).expect("the blueprints that ship").flatten() {
        let file = entry.path();
        let Some(stem) = file.file_name().and_then(|name| name.to_str()).and_then(brand::blueprint_stem)
        else {
            continue;
        };
        let text = std::fs::read_to_string(&file).expect("a blueprint that ships");
        found.insert(format!("std/{stem}"), text.lines().count());
    }
    found
}

/// What is written down: each text's name and the lines it is held to.
fn kept() -> BTreeMap<String, usize> {
    let written = std::fs::read_to_string(repo().join(KEPT)).unwrap_or_default();
    written
        .lines()
        .filter_map(|line| {
            let (name, lines) = line.split_once(' ')?;
            Some((name.to_string(), lines.trim().parse().ok()?))
        })
        .collect()
}

/// What is wrong between the texts as they are and what they are held
/// to, a sentence for each thing, and what the file should say where
/// writing it is all that is owed.
fn held(shipped: &BTreeMap<String, usize>, kept: &BTreeMap<String, usize>) -> (Vec<String>, Vec<String>) {
    let (mut grown, mut owed) = (Vec::new(), Vec::new());
    for (name, lines) in shipped {
        match kept.get(name) {
            None => owed.push(format!("`{name}` ships and is held to no number of lines yet (it is {lines})")),
            Some(was) if lines > was => grown.push(format!(
                "`{name}` is {lines} lines and is held to {was}: a blueprint that ships only gets shorter. What was \
                 added is a block's to decide (a function with its cases), a text of its own that this one calls, \
                 or a row of a table"
            )),
            Some(was) if lines < was => {
                owed.push(format!("`{name}` is {lines} lines, shorter than the {was} it is held to"))
            }
            Some(_) => {}
        }
    }
    for name in kept.keys().filter(|name| !shipped.contains_key(*name)) {
        owed.push(format!("`{name}` is held to a number of lines and no longer ships"));
    }
    (grown, owed)
}

fn write(shipped: &BTreeMap<String, usize>) {
    let written: String = shipped.iter().map(|(name, lines)| format!("{name} {lines}\n")).collect();
    std::fs::write(repo().join(KEPT), written).expect("the file of lines written");
}

#[test]
fn the_lines_of_a_blueprint_that_ships_only_go_down() {
    let (shipped, kept) = (shipped(), kept());
    let (grown, owed) = held(&shipped, &kept);
    let why = std::env::var("GROW_LINES").ok().filter(|why| !why.trim().is_empty());
    if let Some(why) = &why {
        // Said once for each text that grew, with what it was held to.
        let mut said = std::fs::read_to_string(repo().join(GROWN)).unwrap_or_default();
        for (name, lines) in
            shipped.iter().filter(|(name, lines)| kept.get(*name).is_some_and(|was| *lines > was))
        {
            said.push_str(&format!("{name} {} to {lines}: {}\n", kept[name], why.trim()));
        }
        std::fs::write(repo().join(GROWN), said).expect("the reasons written");
        write(&shipped);
        return;
    }
    assert!(
        grown.is_empty(),
        "{}.\nWhere one has to be longer, say why, and the reason is kept (`{GROWN}`):\n  GROW_LINES=\"why\" cargo test -p api the_lines",
        grown.join(".\n")
    );
    if std::env::var_os("WRITE_LINES").is_some() {
        write(&shipped);
        return;
    }
    assert!(
        owed.is_empty(),
        "{}.\nWrite what each is now held to (`{KEPT}`), and commit it with the text:\n  WRITE_LINES=1 cargo test -p api the_lines",
        owed.join(".\n")
    );
}

#[test]
fn what_is_wrong_with_the_lines_is_said_for_each_text_and_a_longer_one_is_never_only_owed() {
    let of = |pairs: &[(&str, usize)]| pairs.iter().map(|(name, lines)| (name.to_string(), *lines)).collect();
    let kept: BTreeMap<String, usize> = of(&[("std/a", 10), ("std/b", 10), ("std/c", 10), ("std/gone", 4)]);
    let shipped: BTreeMap<String, usize> = of(&[("std/a", 10), ("std/b", 12), ("std/c", 7), ("std/new", 3)]);
    let (grown, owed) = held(&shipped, &kept);
    assert_eq!(grown.len(), 1, "{grown:?}");
    assert!(grown[0].starts_with("`std/b` is 12 lines and is held to 10"), "{grown:?}");
    assert_eq!(
        owed,
        [
            "`std/c` is 7 lines, shorter than the 10 it is held to",
            "`std/new` ships and is held to no number of lines yet (it is 3)",
            "`std/gone` is held to a number of lines and no longer ships",
        ]
    );
    // As it is written, nothing is wrong.
    assert_eq!(held(&kept, &kept), (Vec::new(), Vec::new()));
}
