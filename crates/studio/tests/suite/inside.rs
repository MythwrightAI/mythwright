//! What is said of the words written inside the arguments that declare a
//! table and read its rows (`kernel::inside`), held to the code that
//! reads those words: a word that is read and not said, or said and not
//! read, fails here.
//!
//! What is read is found in the source, by how each reading asks for a
//! word, between two lines that begin and end it. A reading that comes to
//! ask another way is not found, and fails for the words it then lacks.

use std::collections::BTreeSet;

use blueprint::Within;
use kernel::inside::{COUNTING, FIELDS, GATHERING, GIVEN, Row, SAYING, VIEWS};

/// A file of a crate's source, by where it is from this crate.
fn source(path: &str) -> String {
    let path = format!("{}/{path}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path}: {err}"))
}

/// The words a stretch of source asks for: what is in quotes straight
/// after one of `asks`, from the first line that holds `from` to the next
/// that holds `to`.
fn asked(source: &str, from: &str, to: &str, asks: &[&str]) -> BTreeSet<String> {
    let begins = source.find(from).unwrap_or_else(|| panic!("the source has `{from}`"));
    let ends = source[begins..].find(to).unwrap_or_else(|| panic!("the source has `{to}` after `{from}`"));
    let stretch = &source[begins..begins + ends];
    let mut found = BTreeSet::new();
    for ask in asks {
        for (at, _) in stretch.match_indices(ask) {
            let rest = &stretch[at + ask.len()..];
            if let Some(word) = rest.strip_prefix('"').and_then(|rest| rest.split('"').next()) {
                found.insert(word.to_string());
            }
        }
    }
    found
}

/// The keys a table says each thing takes: its rows one part below `*`
/// that are neither what each thing is nor a word.
fn keys(table: &[Row]) -> BTreeSet<String> {
    table
        .iter()
        .filter(|(_, kind, _)| !["each", "word"].contains(kind))
        .filter_map(|(path, ..)| path.strip_prefix("*."))
        .filter(|key| !key.contains('.'))
        .map(str::to_string)
        .collect()
}

/// The words a table says `under` may be, in the order it says them.
fn words<'a>(table: &'a [Row], under: &str) -> Vec<&'a str> {
    table
        .iter()
        .filter(|(_, kind, _)| *kind == "word")
        .filter_map(|(path, ..)| path.strip_prefix(under))
        .collect()
}

fn set(words: &[&str]) -> BTreeSet<String> {
    words.iter().map(|word| word.to_string()).collect()
}

#[test]
fn each_table_is_read_whole_and_says_a_sentence_for_every_key() {
    let tables: &[(&str, &[Row])] = &[
        ("FIELDS", FIELDS),
        ("VIEWS", VIEWS),
        ("GIVEN", GIVEN),
        ("COUNTING", COUNTING),
        ("GATHERING", GATHERING),
        ("SAYING", SAYING),
    ];
    for (name, table) in tables {
        let owned: Vec<(String, String, String)> = table
            .iter()
            .map(|(path, kind, says)| (path.to_string(), kind.to_string(), says.to_string()))
            .collect();
        assert_eq!(
            Within::of(table).rows(),
            owned,
            "`kernel::inside::{name}` has a row that fits nowhere, or its rows are in another order than they are \
             read in: what each thing is, each key with its words, then what each key holds"
        );
        for (path, kind, says) in *table {
            assert!(*kind == "word" || !says.is_empty(), "`{path}` of `{name}` says nothing of itself");
        }
    }
}

#[test]
fn what_is_said_of_a_field_is_every_word_a_field_is_read_by_and_no_other() {
    let read = asked(&source("src/fields.rs"), "pub(crate) fn read(", "\n}\n", &["about.get(", "flag("]);
    assert_eq!(keys(FIELDS), read, "the keys `kernel::inside::FIELDS` says, and those `fields::read` reads");
    assert_eq!(
        words(FIELDS, "*.type."),
        studio::collections::KINDS,
        "the kinds of field there are, in their order"
    );
}

#[test]
fn what_is_said_of_a_view_is_every_word_a_view_is_read_by_and_no_other() {
    let declared = source("src/declared.rs");
    let mut read = asked(
        &declared,
        "for (name, about) in if views.is_null()",
        "looked.push(view)",
        &["about.get(", "said("],
    );
    // What a view says of its columns is read where the three are plain values.
    let columns = source("../kernel/src/columns.rs");
    read.extend(asked(&columns, "pub fn read(view: &str", "\n    }\n", &["said("]));
    assert_eq!(keys(VIEWS), read, "the keys `kernel::inside::VIEWS` says, and those a view is read by");
    assert_eq!(
        set(&words(VIEWS, "*.kind.")),
        set(studio::collections::LAYOUTS),
        "the ways a table is laid out"
    );
    assert_eq!(
        words(VIEWS, "*.figures.*."),
        kernel::columns::FIGURES,
        "the figures there are, in their order"
    );
}

#[test]
fn what_is_said_of_a_reading_of_rows_is_every_word_its_forms_are_read_by_and_no_other() {
    // What `rows.read` comes to is worked out in the kernel (`kernel::rows`).
    let rows = source("../kernel/src/rows.rs");
    let given = asked(&rows, "fn comes_to(", "\n}\n", &["said.get("]);
    assert_eq!(keys(GIVEN), given, "the forms a cell is given by");
    let counting = asked(&rows, "in asked.counting", "in &mut rows", &["of.get("]);
    assert_eq!(keys(COUNTING), counting, "what a figure over other rows says");
    let gathering = asked(&rows, "in asked.gathering", "gathered.insert(", &["of.get("]);
    assert_eq!(keys(GATHERING), gathering, "what a list of one cell's values says");
    let saying = asked(&rows, "fn wanted<", "\n}\n", &["form.get("]);
    assert_eq!(keys(SAYING), saying, "what a row is to say in a cell");
}
