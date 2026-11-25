//! The other ways the suites are played: which are asked for, and what a
//! test program is told for each.

use std::path::PathBuf;

use xtask::ways::{WAYS, Way, asked, line, plays, told};

fn names(ways: &[&Way]) -> Vec<&'static str> {
    ways.iter().map(|way| way.name).collect()
}

fn args(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[test]
fn every_way_is_played_where_none_is_named_and_only_those_named_where_some_are() {
    assert_eq!(names(&asked(&[]).unwrap()), ["one", "own", "gathered", "ungathered"]);
    assert_eq!(names(&asked(&args(&["gathered", "one"])).unwrap()), ["gathered", "one"]);
    assert_eq!(names(&asked(&args(&["own", "own"])).unwrap()), ["own"], "named twice, played once");
}

#[test]
fn a_name_that_is_no_way_is_refused_with_those_there_are() {
    // Taken for nothing, the suites would be played the usual way and
    // said to hold another.
    let refused = asked(&args(&["one", "each"])).unwrap_err();
    assert!(refused.contains("`each`"), "{refused}");
    assert!(refused.contains("one, own, gathered, ungathered"), "{refused}");
}

#[test]
fn a_program_is_told_the_word_of_the_way_it_is_played_and_nothing_of_any_other() {
    let way = |name: &str| WAYS.iter().find(|way| way.name == name).unwrap();
    assert_eq!(told(way("one")), [("PROJECT_JOURNALS", Some("one")), ("JOURNAL_GATHERS", None)]);
    assert_eq!(told(way("own")), [("PROJECT_JOURNALS", Some("own")), ("JOURNAL_GATHERS", None)]);
    assert_eq!(told(way("gathered")), [("PROJECT_JOURNALS", None), ("JOURNAL_GATHERS", Some("all"))]);
    assert_eq!(told(way("ungathered")), [("PROJECT_JOURNALS", None), ("JOURNAL_GATHERS", Some("1"))]);
}

#[test]
fn a_way_only_some_suites_go_by_is_played_for_those_and_any_other_for_every_one() {
    let way = |name: &str| WAYS.iter().find(|way| way.name == name).unwrap();
    for krate in ["journal", "conductor", "api", "xtask"] {
        for name in ["one", "own", "gathered"] {
            assert!(plays(way(name), krate), "{krate} {name}");
        }
        assert_eq!(plays(way("ungathered"), krate), krate == "journal", "{krate}");
    }
}

#[test]
fn each_way_is_a_word_the_journal_reads_where_it_reads_what_a_program_was_told() {
    // The tooling stands on the standard library alone and cannot ask the
    // journal's crate, so the two are held together here: a word that
    // nothing reads would have the suites played the usual way under
    // another name.
    let journal = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../crates/journal/src");
    let sources: Vec<String> = std::fs::read_dir(&journal)
        .unwrap()
        .filter_map(|entry| std::fs::read_to_string(entry.ok()?.path()).ok())
        .collect();
    for way in WAYS {
        let says = format!("const SAYS: &str = \"{}\";", way.says);
        let word = format!("Some(\"{}\") =>", way.word);
        assert!(
            sources.iter().any(|source| source.contains(&says) && source.contains(&word)),
            "nothing in {} reads `{}` from `{}`, which is how the suites are played `{}`",
            journal.display(),
            way.word,
            way.says,
            way.name
        );
    }
}

#[test]
fn what_came_of_a_way_is_a_line_that_says_whether_it_held() {
    let way = &WAYS[0];
    assert_eq!(line(way, true, 61_234, "5310 tests"), "  ok   one          61234 ms  5310 tests");
    assert_eq!(line(way, false, 9, "no tests"), "  FAIL one              9 ms  no tests");
}
