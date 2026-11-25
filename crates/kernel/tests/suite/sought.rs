//! What a search finds by what something is called, and where a passage
//! it found stands in a document.

use kernel::sought::{answers, heading_at, marked, words};

const MARKS: (char, char) = ('[', ']');

/// A passage, and the line and heading it stands under.
type Under<'a> = (&'a str, Option<(usize, Option<&'a str>)>);

#[test]
fn what_was_typed_is_read_as_its_words_in_small_letters() {
    let cases: &[(&str, &[&str])] = &[
        ("Shop", &["shop"]),
        ("  riverside   SHOP ", &["riverside", "shop"]),
        ("@ada-lovelace", &["ada", "lovelace"]),
        ("run_01ab", &["run_01ab"]),
        ("Ärger über", &["ärger", "über"]),
        ("— …", &[]),
    ];
    for (typed, said) in cases {
        assert_eq!(words(typed), *said, "{typed}");
    }
}

#[test]
fn something_answers_when_each_word_typed_begins_a_word_of_its_names_the_whole_name_first() {
    let cases: &[(&str, &[&str], Option<u8>)] = &[
        // One of its names is what was typed.
        ("shop", &["Shop"], Some(0)),
        ("Riverside shop", &["riverside-shop", "Riverside Shop"], Some(0)),
        // One begins with it.
        ("river", &["Riverside Shop"], Some(1)),
        ("riverside sh", &["Riverside Shop"], Some(1)),
        // Every word begins a word, of any of its names, in any order.
        ("shop river", &["Riverside Shop"], Some(2)),
        ("preview dev", &["run_1", "std/preview", "dev"], Some(2)),
        ("love", &["Ada Lovelace", "ada-lovelace"], Some(2)),
        // A word that begins none, or is inside one, is not an answer.
        ("ada babbage", &["Ada Lovelace"], None),
        ("side", &["Riverside Shop"], None),
        ("", &["Shop"], None),
        ("shop", &[], None),
    ];
    for (typed, names, answer) in cases {
        assert_eq!(answers(&words(typed), names), *answer, "{typed} of {names:?}");
    }
}

#[test]
fn a_name_is_marked_where_a_word_typed_begins_one_of_its_words() {
    let cases: &[(&str, &str, &str)] = &[
        ("Riverside Shop", "river sh", "[River]side [Sh]op"),
        ("riverside-shop", "shop", "riverside-[shop]"),
        ("#general", "gen", "#[gen]eral"),
        ("Ada Lovelace", "lovelace ada", "[Ada] [Lovelace]"),
        ("Ärger", "är", "[Är]ger"),
        ("Shop", "depot", "Shop"),
        // The longest word typed that begins it.
        ("Shopping", "s shop", "[Shop]ping"),
    ];
    for (name, typed, said) in cases {
        assert_eq!(marked(name, &words(typed), MARKS), *said, "{name} for {typed}");
    }
}

#[test]
fn a_passage_found_in_a_document_stands_under_the_heading_nearest_above_it() {
    let text = "# Handbook\n\nIntro words.\n\n## Rolling back\n\nStop the service, then restore the snapshot.\n\n### Notes\n\nNothing here.\n";
    let cases: &[Under] = &[
        // Under the second heading, found by the piece that matched.
        ("Stop the service, then [restore] the snapshot.", Some((5, Some("Rolling back")))),
        // Cut at both ends: the piece with the match is the one looked for.
        ("Intro words … Stop the [service], then … Nothing", Some((5, Some("Rolling back")))),
        ("[Intro] words.", Some((1, Some("Handbook")))),
        ("[Nothing] here.", Some((9, Some("Notes")))),
        // Not in it.
        ("[Elsewhere] entirely", None),
        // Nothing marked.
        ("Stop the service", None),
    ];
    for (passage, said) in cases {
        let found = heading_at(text, passage, MARKS);
        let found = found.as_ref().map(|(line, heading)| (*line, heading.as_deref()));
        assert_eq!(found, *said, "{passage}");
    }
    // With no heading above it, the line the passage is on.
    assert_eq!(heading_at("one\ntwo\nthree x", "three [x]", MARKS), Some((3, None)));
    // A passage that is the whole of it: where in it what matched is.
    assert_eq!(
        heading_at(text, text.replace("restore", "[restore]").as_str(), MARKS),
        Some((5, Some("Rolling back".into())))
    );
    // A `#` that begins no heading is no heading.
    assert_eq!(heading_at("#tag\nwords", "[words]", MARKS), Some((2, None)));
}
