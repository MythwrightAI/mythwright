//! A shadow test: one contender, its incumbent, whose work is the item's
//! whatever the judges say, and challengers beside it that are judged or
//! only counted, their work never the item's. Which entry is the
//! incumbent (`contest.ways`), and what becomes of each entry
//! (`contest.outcome`), each a table of cases.

use deciding::contested::outcome::{Asked, Entry as Kept, outcome};
use deciding::contested::ways::{Member, Setup, ways};

fn setup(contenders: &[&str], shadow: &str) -> Setup {
    let member = |name: &str, role: &str| Member { name: name.into(), role: role.into() };
    Setup {
        item: "shop-12".into(),
        contest: "shop-contests-3".into(),
        contenders: contenders.iter().map(|one| one.to_string()).collect(),
        members: vec![
            member("shop-worker-1", "worker"),
            member("shop-otter-1", "otter"),
            member("shop-slate-1", "slate"),
            member("shop-reviewer-1", "reviewer"),
        ],
        judges: "reviewer".into(),
        blind: true,
        shadow: shadow.into(),
        ..Default::default()
    }
}

#[test]
fn a_shadow_tests_incumbent_is_one_of_its_contenders_and_the_first_where_they_leave_it_out() {
    // What, the setup, and what comes of it: the ways in the order written, and the incumbent's.
    let cases: Vec<(&str, Setup, Vec<&str>, &str)> = vec![
        ("a test of equals has none", setup(&["otter", "slate"], ""), vec!["otter", "slate"], ""),
        ("one of the contenders", setup(&["otter", "slate"], "slate"), vec!["otter", "slate"], "slate"),
        (
            "the usual setup, which the contenders leave out, beside one challenger",
            setup(&["slate"], "worker"),
            vec!["worker", "slate"],
            "worker",
        ),
        (
            "an agent, written with or without its `@`",
            setup(&["@shop-otter-1", "slate"], "shop-otter-1"),
            vec!["shop-otter-1", "slate"],
            "shop-otter-1",
        ),
        (
            "an agent the contenders leave out",
            setup(&["slate"], "@shop-worker-1"),
            vec!["shop-worker-1", "slate"],
            "shop-worker-1",
        ),
    ];
    for (case, asked, written, incumbent) in cases {
        let came = ways(&asked).unwrap();
        assert_eq!(came.came, "ready", "{case}: {}", came.why);
        let mut each: Vec<&str> = came.entries.iter().map(|one| one.way.as_str()).collect();
        let mut wanted = written.clone();
        each.sort();
        wanted.sort();
        assert_eq!(each, wanted, "{case}");
        assert_eq!(came.incumbent, incumbent, "{case}");
        let marked: Vec<&str> =
            came.entries.iter().filter(|one| one.incumbent).map(|one| one.way.as_str()).collect();
        assert_eq!(marked, if incumbent.is_empty() { vec![] } else { vec![incumbent] }, "{case}");
        // Its agent works an entry, and so does not judge.
        if incumbent == "worker" {
            assert!(came.not.contains(&"shop-worker-1".to_string()), "{case}: {:?}", came.not);
        }
    }

    // What is wrong with one, each in a sentence: it is held as a contender is.
    let wrong: Vec<(&str, Setup, &str)> = vec![
        ("no role of the fleet's", setup(&["slate"], "glen"), "`glen` is no role of the fleet's"),
        ("no agent of the fleet's", setup(&["slate"], "@shop-nobody"), "`@shop-nobody` is no agent"),
        ("alone, with no challenger", setup(&[], "otter"), "this one has one, `otter`"),
        (
            "judged by the incumbent",
            Setup { judges: "worker".into(), ..setup(&["slate"], "worker") },
            "nobody judges",
        ),
    ];
    for (case, asked, why) in wrong {
        let came = ways(&asked).unwrap();
        assert_eq!(came.came, "wrong", "{case}");
        assert!(came.why.contains(why), "{case}: {}", came.why);
    }
}

fn kept(way: &str, letter: &str, state: &str) -> Kept {
    Kept { way: way.into(), letter: letter.into(), state: state.into(), ..Default::default() }
}

/// A shadow test of `otter` (A, the incumbent) and `slate` (B), as `entries` say.
fn shadow(entries: Vec<Kept>) -> Asked {
    Asked { entries, incumbent: "otter".into(), by: "shop-judge-1".into(), ..Default::default() }
}

fn two() -> Vec<Kept> {
    vec![kept("otter", "A", "kept"), kept("slate", "B", "kept")]
}

#[test]
fn a_shadow_tests_incumbent_goes_on_whatever_is_judged_and_its_challengers_work_is_never_the_items() {
    let judged =
        |judged: &str, winner: &str| Asked { judged: judged.into(), winner: winner.into(), ..shadow(two()) };
    // What, the asking, and what it comes to: the word, the segment's word,
    // the row's state, the way taken, and each entry's verdict and fate.
    type Wanted<'a> = (&'a str, &'a str, &'a str, &'a str, Vec<(&'a str, &'a str)>);
    let cases: Vec<(&str, Asked, Wanted)> = vec![
        (
            "nothing judged yet, the challengers judged: judge both",
            shadow(two()),
            ("judge", "", "judging", "", vec![("", "wait"), ("", "wait")]),
        ),
        (
            "the challengers only counted: the incumbent taken with nothing judged",
            Asked { challengers: "counted".into(), ..shadow(two()) },
            ("won", "on", "came", "otter", vec![("", "take"), ("", "keep")]),
        ),
        (
            "the judges take the challenger: the incumbent taken all the same, the challenger's branch kept",
            judged("won", "slate"),
            ("won", "on", "came", "otter", vec![("lost", "take"), ("won", "keep")]),
        ),
        (
            "the judges take the incumbent",
            judged("won", "otter"),
            ("won", "on", "came", "otter", vec![("won", "take"), ("lost", "keep")]),
        ),
        (
            "the challenger's work thrown away, as the test says, though the judges took it",
            Asked { losers: "discard".into(), ..judged("won", "slate") },
            ("won", "on", "came", "otter", vec![("lost", "take"), ("won", "discard")]),
        ),
        (
            "the judges decided nothing: nobody is asked, the incumbent goes on",
            Asked { why: "a tie".into(), ..judged("undecided", "") },
            ("won", "on", "came", "otter", vec![("undecided", "take"), ("undecided", "keep")]),
        ),
        (
            "scores left to a person: nobody is asked in a shadow test",
            Asked { undecided: "person".into(), ..judged("person", "") },
            ("won", "on", "came", "otter", vec![("undecided", "take"), ("undecided", "keep")]),
        ),
        (
            "a person's answer takes nothing in a shadow test",
            Asked { answer: "B".into(), ..shadow(two()) },
            ("judge", "", "judging", "", vec![("", "wait"), ("", "wait")]),
        ),
        (
            "no challenger's work kept: the incumbent taken with nothing judged",
            shadow(vec![kept("otter", "A", "kept"), kept("slate", "B", "failed")]),
            ("won", "on", "came", "otter", vec![("", "take"), ("", "keep")]),
        ),
        (
            "the incumbent's work not kept: left for a person, no challenger taken in its place",
            Asked {
                alone: "take".into(),
                ..shadow(vec![kept("otter", "A", "unkept"), kept("slate", "B", "kept")])
            },
            ("left", "left", "left", "", vec![("", "keep"), ("", "keep")]),
        ),
    ];
    for (case, asking, (came, ends, state, winner, each)) in cases {
        let got = outcome(&asking).unwrap();
        assert_eq!(
            (got.came, got.ends, got.state, got.winner.as_str(), got.entries.clone()),
            (came, ends, state, winner, each),
            "{case}"
        );
        if came != "judge" {
            assert!(!got.said.is_empty() && got.question.is_empty(), "{case}");
        }
    }

    // What is said of it: that it was a shadow test, what the judges took, and that the incumbent goes on.
    let said = outcome(&judged("won", "slate")).unwrap();
    assert_eq!(
        said.said,
        "its shadow test of otter and slate was judged by shop-judge-1: slate; its incumbent, otter, goes on"
    );
    assert!(
        said.why.starts_with("a shadow test") && said.why.contains("the judges took slate"),
        "{}",
        said.why
    );
    let counted = outcome(&Asked { challengers: "counted".into(), ..shadow(two()) }).unwrap();
    assert!(counted.said.contains("was counted and not judged"), "{}", counted.said);
    let unkept = outcome(&shadow(vec![kept("otter", "A", "failed"), kept("slate", "B", "kept")])).unwrap();
    assert!(
        unkept.why.contains("its incumbent otter's work could not be kept (otter failed)"),
        "{}",
        unkept.why
    );

    // What is wrong, each in a sentence.
    let nobody = outcome(&Asked { incumbent: "lark".into(), ..shadow(two()) }).unwrap_err();
    assert!(nobody.contains("the incumbent `lark` is none of the contest's entries"), "{nobody}");
    let astray = outcome(&judged("won", "glen")).unwrap_err();
    assert!(astray.contains("`glen` won, which is no entry"), "{astray}");
    let setting = outcome(&Asked { challengers: "ignored".into(), ..shadow(two()) }).unwrap_err();
    assert!(setting.contains("`challengers` is one of judged, counted"), "{setting}");
}
