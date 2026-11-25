//! What a contest comes to: which of an item's contests is next, what its
//! contenders come to as ways and letters, what judges' picks, scores and
//! talk come to, and what becomes of each entry. Each a table of cases
//! that reaches every word its function can answer, for any number of
//! contenders.

use std::collections::BTreeSet;

use deciding::contested::described::{self, NO, YES};
use deciding::contested::outcome::{Asked, Entry as Kept, OUTCOME_WORDS, outcome};
use deciding::contested::pending::{Contest, PENDING_WORDS, pending};
use deciding::contested::scores::{Reply, SCORES_WORDS, scores};
use deciding::contested::shown::{Shown, seated, shown};
use deciding::contested::talked::{Said, TALKED_WORDS, heard, talked};
use deciding::contested::tally::{Pick, TALLY_WORDS, tally};
use deciding::contested::ways::{
    ALL_PATCH, LEAST_PATCH, MOST_PATCH, Member, Setup, WAYS_WORDS, own, owned, ways,
};
use deciding::contested::{Known, letter};

fn words(of: &[&str]) -> Vec<String> {
    of.iter().map(|one| one.to_string()).collect()
}

#[test]
fn the_letters_go_past_z_as_a_spreadsheets_columns_are_called() {
    // How many contenders there are, and the letter of the last of them.
    let cases =
        [(1, "A"), (2, "B"), (26, "Z"), (27, "AA"), (52, "AZ"), (53, "BA"), (702, "ZZ"), (703, "AAA")];
    for (many, last) in cases {
        assert_eq!(letter(many - 1), last, "{many} contenders");
    }
    // And every one of 703 has a letter of its own.
    let all: BTreeSet<String> = (0..703).map(letter).collect();
    assert_eq!(all.len(), 703);
}

fn contest(id: &str, card: &str, state: &str) -> Contest {
    Contest { id: id.into(), card: card.into(), state: state.into() }
}

#[test]
fn which_of_an_items_contests_is_next_is_one_under_way_and_else_the_oldest_waiting() {
    // What, the contests, and the word, the one to do and how many wait after it.
    type Case<'a> = (&'a str, Vec<Contest>, &'a str, Option<usize>, usize);
    let cases: Vec<Case> = vec![
        ("no contest: nothing", vec![], "none", None, 0),
        ("one waiting: it", vec![contest("c1", "shop-1", "")], "next", Some(0), 0),
        (
            "two waiting: the oldest, the other after it",
            vec![contest("c1", "shop-1", "waiting"), contest("c2", "shop-1", "")],
            "next",
            Some(0),
            1,
        ),
        (
            "one came and one waiting: the one waiting",
            vec![contest("c1", "shop-1", "came"), contest("c2", "shop-1", "waiting")],
            "next",
            Some(1),
            0,
        ),
        ("one left: nothing", vec![contest("c1", "shop-1", "left")], "none", None, 0),
        (
            "the newest came to a winner and nothing waits: done, as a round begun again after a stop finds it",
            vec![contest("c1", "shop-1", "left"), contest("c2", "shop-1", "came")],
            "done",
            Some(1),
            0,
        ),
        ("none at all: nothing", vec![contest("c1", "shop-2", "came")], "none", None, 0),
        (
            "one under way behind one waiting, as a stop left it: the one under way",
            vec![contest("c1", "shop-1", "waiting"), contest("c2", "shop-1", "judging")],
            "next",
            Some(1),
            1,
        ),
        (
            "another item's: passed by",
            vec![contest("c1", "shop-2", "waiting"), contest("c2", "shop-1", "asking")],
            "next",
            Some(1),
            0,
        ),
    ];
    let mut reached = BTreeSet::new();
    for (case, contests, came, at, waiting) in cases {
        let got = pending("shop-1", &contests).unwrap();
        assert_eq!((got.came, got.at, got.waiting), (came, at, waiting), "{case}");
        reached.insert(got.came);
    }
    assert_eq!(reached, PENDING_WORDS.iter().copied().collect());
    let wrong = pending("shop-1", &[contest("c1", "shop-1", "lost")]).unwrap_err();
    assert!(wrong.contains("says it is `lost`, which a contest is not"), "{wrong}");
}

fn member(name: &str, role: &str) -> Member {
    Member { name: name.into(), role: role.into() }
}

fn setup(contenders: &[&str]) -> Setup {
    Setup {
        item: "shop-12".into(),
        contest: "shop-contests-3".into(),
        contenders: words(contenders),
        members: vec![
            member("shop-otter-1", "otter"),
            member("shop-slate-1", "slate"),
            member("shop-fern-1", "fern"),
            member("shop-reviewer-1", "reviewer"),
        ],
        judges: "reviewer".into(),
        blind: true,
        ..Default::default()
    }
}

#[test]
fn what_contenders_come_to_as_ways_letters_places_and_patches_or_what_is_wrong_with_them() {
    // Two: each a way that hands its role, a letter each from A, a place
    // of its own and not the other's, and the patch shared between them.
    let two = ways(&setup(&["otter", "slate"])).unwrap();
    assert_eq!((two.came, two.comparison.as_str()), ("ready", "ab-shop-contests-3"));
    let mut by_way: Vec<(String, String)> =
        two.entries.iter().map(|one| (one.way.clone(), one.letter.clone())).collect();
    by_way.sort();
    let letters: BTreeSet<String> = by_way.iter().map(|(_, letter)| letter.clone()).collect();
    assert_eq!(letters, BTreeSet::from(["A".to_string(), "B".to_string()]));
    let otter = two.entries.iter().find(|one| one.way == "otter").unwrap();
    assert_eq!(otter.hands, Some(("role", "otter".to_string())));
    // Blind: its place is called after its letter, which says nothing of whose it is.
    let slate = two.entries.iter().find(|one| one.way == "slate").unwrap();
    let key = |of: &deciding::contested::ways::Entry| format!("shop-12--{}", of.letter.to_lowercase());
    assert_eq!((otter.key.clone(), otter.name.as_str()), (key(otter), "shop-contests-3-otter"));
    assert_eq!(otter.hidden, [format!(".lanes/{}", key(slate))]);
    // Not blind, after its way.
    let seen = ways(&Setup { blind: false, ..setup(&["otter", "slate"]) }).unwrap();
    let opus_seen = seen.entries.iter().find(|one| one.way == "otter").unwrap();
    assert_eq!(opus_seen.key, "shop-12--otter");
    assert_eq!(opus_seen.hidden, [".lanes/shop-12--slate"]);
    assert_eq!(otter.patch, ALL_PATCH / 2);
    assert_eq!(two.not, ["shop-otter-1", "shop-slate-1"]);

    // Ten: a letter each, A to J, each shown its share of the patch and
    // none of the nine other places it is not.
    let ten_roles: Vec<String> = (1..=10).map(|at| format!("m{at}")).collect();
    let mut ten = setup(&[]);
    ten.contenders = ten_roles.clone();
    ten.members = ten_roles.iter().map(|role| member(&format!("shop-{role}-1"), role)).collect();
    ten.members.push(member("shop-reviewer-1", "reviewer"));
    let came = ways(&ten).unwrap();
    assert_eq!(came.came, "ready");
    let letters: Vec<&str> = came.entries.iter().map(|one| one.letter.as_str()).collect();
    assert_eq!(letters, ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"]);
    assert!(
        came.entries
            .iter()
            .all(|one| one.hidden.len() == 9 && !one.hidden.contains(&format!(".lanes/{}", one.key)))
    );
    assert!(came.entries.iter().all(|one| one.patch == ALL_PATCH / 10));

    // The order a judge is shown them in is turned by the item: the same
    // for an item every time, and not the same for every item.
    let order = |item: &str| {
        let came = ways(&Setup { item: item.into(), ..ten.clone() }).unwrap();
        came.entries.iter().map(|one| one.way.clone()).collect::<Vec<_>>()
    };
    assert_eq!(order("shop-12"), order("shop-12"));
    let firsts: BTreeSet<String> = (1..=20).map(|at| order(&format!("shop-{at}"))[0].clone()).collect();
    assert!(firsts.len() > 3, "the first shown is not always the same contender: {firsts:?}");

    // Not blind: a judge knows each by its way.
    let open = ways(&Setup { blind: false, ..setup(&["otter", "@shop-slate-1"]) }).unwrap();
    let mut shown: Vec<&str> = open.entries.iter().map(|one| one.letter.as_str()).collect();
    shown.sort();
    assert_eq!(shown, ["otter", "shop-slate-1"]);
    let slate = open.entries.iter().find(|one| one.way == "shop-slate-1").unwrap();
    assert_eq!(slate.hands, Some(("agent", "shop-slate-1".to_string())));

    // A shared comparison's own way hands nothing here: it says it itself.
    let shared =
        ways(&Setup { compared: "models".into(), known: words(&["mine"]), ..setup(&["mine", "otter"]) })
            .unwrap();
    assert_eq!(shared.comparison, "models");
    assert_eq!(shared.entries.iter().find(|one| one.way == "mine").unwrap().hands, None);

    // The patch is never less than enough to read, nor more than a place gives.
    let many: Vec<String> = (1..=100).map(|at| format!("w{at}")).collect();
    let wide =
        ways(&Setup { contenders: many, members: Vec::new(), judges: String::new(), ..setup(&[]) }).unwrap();
    assert!(wide.entries.iter().all(|one| one.patch == LEAST_PATCH));
    let big = ways(&Setup { patch: 64 << 20, ..setup(&["otter", "slate"]) }).unwrap();
    assert!(big.entries.iter().all(|one| one.patch == MOST_PATCH));

    // What is wrong, each in a sentence.
    let cases: Vec<(&str, Setup, &str)> = vec![
        ("none", setup(&[]), "this one has none"),
        ("one", setup(&["otter"]), "this one has one, `otter`"),
        ("one twice", setup(&["otter", "slate", "otter"]), "`otter` is written twice"),
        ("no role", setup(&["otter", "glen"]), "`glen` is no role of the fleet's"),
        ("no agent", setup(&["otter", "@shop-nobody"]), "`@shop-nobody` is no agent of the fleet's"),
        (
            "not a word",
            Setup { members: Vec::new(), ..setup(&["otter", "Slate"]) },
            "`Slate` cannot be a way",
        ),
        (
            "judged by its contenders",
            Setup { judges: "otter".into(), ..setup(&["otter", "slate"]) },
            "nobody judges their own work",
        ),
        (
            "judged by an agent that works",
            Setup { judges: "@shop-slate-1".into(), ..setup(&["otter", "slate"]) },
            "nobody judges their own work",
        ),
        (
            "a comparison of its own whose name is too long",
            Setup {
                contest: "a-project-with-a-very-long-name-contests-12".into(),
                ..setup(&["otter", "slate"])
            },
            "name one in `compared`",
        ),
    ];
    let mut reached = BTreeSet::from(["ready"]);
    for (case, asked, why) in cases {
        let came = ways(&asked).unwrap();
        assert_eq!(came.came, "wrong", "{case}");
        assert!(came.why.contains(why), "{case}: {}", came.why);
        reached.insert(came.came);
    }
    assert_eq!(reached, WAYS_WORDS.iter().copied().collect());
    // A judge of the same role as a contender may still judge where another of it does not work.
    let shared_role = Setup {
        judges: "otter".into(),
        members: vec![
            member("shop-otter-1", "otter"),
            member("shop-otter-2", "otter"),
            member("shop-slate-1", "slate"),
        ],
        ..setup(&["@shop-otter-1", "slate"])
    };
    assert_eq!(ways(&shared_role).unwrap().came, "ready");
    assert!(ways(&Setup { item: String::new(), ..setup(&["otter", "slate"]) }).is_err());

    // Taken up again after a stop: an entry whose row says its work is over is not started again,
    // and one still at work, or with no row yet, is.
    for (state, over) in [
        ("kept", true),
        ("unkept", true),
        ("failed", true),
        ("judged", true),
        ("working", false),
        ("", false),
    ] {
        let rows = vec![("shop-contests-3-otter".to_string(), state.to_string())];
        let again = ways(&Setup { rows, ..setup(&["otter", "slate"]) }).unwrap();
        let each: Vec<(&str, bool)> = again.entries.iter().map(|one| (one.way.as_str(), one.over)).collect();
        assert!(each.contains(&("otter", over)) && each.contains(&("slate", false)), "{state}: {each:?}");
    }
}

fn entries(ways: &[(&str, &str)]) -> Vec<Known> {
    ways.iter().map(|(way, letter)| Known { way: way.to_string(), letter: letter.to_string() }).collect()
}

fn pick(by: &str, pick: &str) -> Pick {
    Pick { by: by.into(), pick: pick.into(), words: format!("{by} says {pick}") }
}

#[test]
fn what_judges_who_each_pick_an_entry_come_to() {
    let two = entries(&[("otter", "A"), ("slate", "B")]);
    let ten: Vec<Known> = (0..10).map(|at| Known { way: format!("w{at}"), letter: letter(at) }).collect();
    // What, the entries, the picks, how many are needed, and the word, the winner and each verdict.
    type Case<'a> = (&'a str, &'a [Known], Vec<Pick>, i64, &'a str, &'a str, Vec<&'a str>);
    let cases: Vec<Case> = vec![
        ("one judge", &two, vec![pick("j1", "B")], 0, "won", "slate", vec!["lost", "won"]),
        (
            "unanimous",
            &two,
            vec![pick("j1", "A"), pick("j2", "A"), pick("j3", "A")],
            3,
            "won",
            "otter",
            vec!["won", "lost"],
        ),
        (
            "the most, with nobody saying how many are enough",
            &ten,
            vec![pick("j1", "J"), pick("j2", "J"), pick("j3", "C")],
            0,
            "won",
            "w9",
            vec!["lost", "lost", "lost", "lost", "lost", "lost", "lost", "lost", "lost", "won"],
        ),
        (
            "a tie",
            &two,
            vec![pick("j1", "A"), pick("j2", "B")],
            0,
            "undecided",
            "",
            vec!["undecided", "undecided"],
        ),
        (
            "too few agree",
            &two,
            vec![pick("j1", "A"), pick("j2", "A"), pick("j3", "B")],
            3,
            "undecided",
            "",
            vec!["undecided", "undecided"],
        ),
        (
            "a judge that picked nothing is counted for nothing",
            &two,
            vec![pick("j1", ""), pick("j2", "B")],
            0,
            "won",
            "slate",
            vec!["lost", "won"],
        ),
        (
            "a pick of no entry, and nobody else",
            &two,
            vec![pick("j1", "Q")],
            0,
            "undecided",
            "",
            vec!["undecided", "undecided"],
        ),
    ];
    let mut reached = BTreeSet::new();
    for (case, judged, picks, needed, came, winner, verdicts) in cases {
        let got = tally(judged, &picks, needed).unwrap();
        let said: Vec<&str> = got.verdicts.iter().map(|(verdict, _)| *verdict).collect();
        assert_eq!((got.came, got.winner.as_str(), said), (came, winner, verdicts), "{case}");
        assert_eq!(got.why.is_empty(), came == "won", "{case}: {}", got.why);
        reached.insert(got.came);
    }
    assert_eq!(reached, TALLY_WORDS.iter().copied().collect());
    let astray = tally(&two, &[pick("j1", "Q"), pick("j2", "A")], 0).unwrap();
    assert_eq!(astray.astray, ["j1: Q"]);
    assert_eq!(astray.verdicts[0].1, ["j2"]);
    assert!(tally(&[], &[pick("j1", "A")], 0).is_err());
}

fn reply(by: &str, text: &str) -> Reply {
    Reply { by: by.into(), text: text.into() }
}

#[test]
fn what_judges_who_each_score_every_entry_come_to_by_the_contests_rule() {
    let two = entries(&[("otter", "A"), ("slate", "B")]);
    let three = entries(&[("otter", "A"), ("slate", "B"), ("fern", "C")]);
    // What, the entries, the replies, the rule, and the word, the winner and each entry's scores.
    type Case<'a> = (&'a str, &'a [Known], Vec<Reply>, &'a str, &'a str, &'a str, Vec<Vec<&'a str>>);
    let cases: Vec<Case> = vec![
        (
            "well formed, and a person picks: the usual",
            &two,
            vec![reply("j1", "Both work.\nA: 7\nB: 9")],
            "",
            "person",
            "",
            vec![vec!["7"], vec!["9"]],
        ),
        (
            "the highest mean",
            &two,
            vec![reply("j1", "A: 7\nB: 9"), reply("j2", "A: 8\nB: 7.")],
            "highest",
            "won",
            "slate",
            vec![vec!["7", "8"], vec!["9", "7"]],
        ),
        (
            "a line missing: that entry is scored by the rest",
            &two,
            vec![reply("j1", "A: 5\nB: 6"), reply("j2", "B: 2")],
            "highest",
            "won",
            "otter",
            vec![vec!["5", ""], vec!["6", "2"]],
        ),
        (
            "a score out of scale is not read",
            &two,
            vec![reply("j1", "A: 11\nB: 3")],
            "highest",
            "won",
            "slate",
            vec![vec![""], vec!["3"]],
        ),
        (
            "a tie under the highest goes to a person",
            &two,
            vec![reply("j1", "A: 6\nB: 8"), reply("j2", "A: 8\nB: 6")],
            "highest",
            "person",
            "",
            vec![vec!["6", "8"], vec!["8", "6"]],
        ),
        (
            "the entry most judges put first",
            &three,
            vec![
                reply("j1", "A: 9\nB: 1\nC: 1"),
                reply("j2", "A: 6\nB: 7\nC: 1"),
                reply("j3", "A: 10\nB: 2\nC: 2"),
            ],
            "most",
            "won",
            "otter",
            vec![vec!["9", "6", "10"], vec!["1", "7", "2"], vec!["1", "1", "2"]],
        ),
        (
            "nobody put one first: a person",
            &two,
            vec![reply("j1", "A: 5\nB: 5")],
            "most",
            "person",
            "",
            vec![vec!["5"], vec!["5"]],
        ),
        (
            "nothing could be read",
            &two,
            vec![reply("j1", "Both are fine.")],
            "highest",
            "undecided",
            "",
            vec![vec![""], vec![""]],
        ),
    ];
    let mut reached = BTreeSet::new();
    for (case, judged, replies, then, came, winner, said) in cases {
        let got = scores(judged, &replies, 10, then).unwrap();
        let wanted: Vec<Vec<String>> = said.iter().map(|one| words(one)).collect();
        assert_eq!((got.came, got.winner.as_str(), &got.scores), (came, winner, &wanted), "{case}");
        if came == "person" {
            assert!(got.question.contains("Which entry is taken"), "{case}: {}", got.question);
        }
        reached.insert(got.came);
    }
    assert_eq!(reached, SCORES_WORDS.iter().copied().collect());
    let tie = scores(&two, &[reply("j1", "A: 6\nB: 6")], 10, "highest").unwrap();
    assert!(tie.why.contains("A and B came out alike"), "{}", tie.why);
    assert!(tie.question.contains("A: 6; B: 6"), "{}", tie.question);
    let rule = scores(&two, &[reply("j1", "A: 6")], 10, "loudest").unwrap_err();
    assert!(rule.contains("`loudest` is no rule"), "{rule}");
}

#[test]
fn what_a_council_that_talks_it_over_came_to_in_a_stretch() {
    let two = entries(&[("otter", "A"), ("slate", "B")]);
    // What, each seat's last word, how many are needed, the turns and the most, and the word and the winner.
    type Case<'a> = (&'a str, Vec<&'a str>, i64, i64, i64, &'a str, &'a str);
    let cases: Vec<Case> = vec![
        ("all agree at once", vec!["A", "A", "A"], 0, 3, 9, "won", "otter"),
        ("not yet agreed, with turns left: another stretch", vec!["A", "B", "on"], 0, 3, 9, "again", ""),
        ("agreed after two stretches", vec!["B", "B", "B"], 0, 6, 9, "won", "slate"),
        ("enough of them agree", vec!["B", "B", "A"], 2, 3, 9, "won", "slate"),
        ("split at the most turns", vec!["A", "B", "B"], 0, 9, 9, "undecided", ""),
        ("a seat that ended with no letter agrees with nobody", vec!["A", "", "A"], 0, 9, 9, "undecided", ""),
    ];
    let mut reached = BTreeSet::new();
    for (case, last, needed, turns, most, came, winner) in cases {
        let got = talked(&two, &words(&last), needed, turns, most).unwrap();
        assert_eq!((got.came, got.winner.as_str()), (came, winner), "{case}");
        reached.insert(got.came);
    }
    assert_eq!(reached, TALKED_WORDS.iter().copied().collect());
    let split = talked(&two, &words(&["A", "B", "B"]), 0, 9, 9).unwrap();
    assert!(split.why.contains("after 9 turns") && split.why.contains("A 1, B 2"), "{}", split.why);
    assert!(talked(&two, &words(&["A"]), 0, 1, 0).is_err());
}

fn kept(way: &str, letter: &str, state: &str) -> Kept {
    Kept { way: way.into(), letter: letter.into(), state: state.into(), ..Default::default() }
}

fn asked(entries: Vec<Kept>) -> Asked {
    Asked { entries, ..Default::default() }
}

#[test]
fn what_becomes_of_a_contest_and_each_of_its_entries() {
    let two = || vec![kept("otter", "A", "kept"), kept("slate", "B", "kept")];
    let won = |losers: &str| Asked {
        judged: "won".into(),
        winner: "slate".into(),
        losers: losers.into(),
        by: "shop-reviewer-1".into(),
        ..asked(two())
    };
    // What, the asking, and what it comes to: the word, the segment's word,
    // the row's state, the winner, and each entry's verdict and fate.
    type Wanted<'a> = (&'a str, &'a str, &'a str, &'a str, Vec<(&'a str, &'a str)>);
    let cases: Vec<(&str, Asked, Wanted)> = vec![
        (
            "two kept and nothing judged yet: judge them",
            asked(two()),
            ("judge", "", "judging", "", vec![("", "wait"), ("", "wait")]),
        ),
        (
            "a winner, the losers kept as branches",
            won(""),
            ("won", "on", "came", "slate", vec![("lost", "keep"), ("won", "take")]),
        ),
        (
            "a winner, the losers thrown away",
            won("discard"),
            ("won", "on", "came", "slate", vec![("lost", "discard"), ("won", "take")]),
        ),
        (
            "undecided, for a person",
            Asked { judged: "undecided".into(), why: "a tie".into(), ..asked(two()) },
            ("asks", "asks", "asking", "", vec![("", "wait"), ("", "wait")]),
        ),
        (
            "undecided, and left",
            Asked {
                judged: "undecided".into(),
                undecided: "left".into(),
                why: "a tie".into(),
                losers: "discard".into(),
                ..asked(two())
            },
            ("left", "left", "left", "", vec![("", "keep"), ("", "keep")]),
        ),
        (
            "scores for a person to pick from",
            Asked {
                judged: "person".into(),
                question: "The judges scored A: 7; B: 9. Which?".into(),
                ..asked(two())
            },
            ("asks", "asks", "asking", "", vec![("", "wait"), ("", "wait")]),
        ),
        (
            "every entry failed",
            asked(vec![kept("otter", "A", "failed"), kept("slate", "B", "unkept")]),
            ("left", "left", "left", "", vec![("", "keep"), ("", "keep")]),
        ),
        (
            "one entry left, for a person",
            asked(vec![kept("otter", "A", "failed"), kept("slate", "B", "kept")]),
            ("asks", "asks", "asking", "", vec![("", "wait"), ("", "wait")]),
        ),
        (
            "one entry left, taken",
            Asked {
                alone: "take".into(),
                ..asked(vec![kept("otter", "A", "failed"), kept("slate", "B", "kept")])
            },
            ("won", "on", "came", "slate", vec![("", "keep"), ("won", "take")]),
        ),
        (
            "a person's answer, by its letter",
            Asked { judged: "undecided".into(), answer: "A".into(), ..asked(two()) },
            ("won", "on", "came", "otter", vec![("won", "take"), ("lost", "keep")]),
        ),
        (
            "an answer that is no entry: asked again",
            Asked { judged: "undecided".into(), answer: "Q".into(), ..asked(two()) },
            ("asks", "asks", "asking", "", vec![("", "wait"), ("", "wait")]),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (case, asking, (came, ends, state, winner, each)) in cases {
        let got = outcome(&asking).unwrap();
        assert_eq!(
            (got.came, got.ends, got.state, got.winner.as_str(), got.entries.clone()),
            (came, ends, state, winner, each),
            "{case}"
        );
        if came == "asks" {
            assert!(
                got.question.contains('?') && got.question.contains("Answer"),
                "{case}: {}",
                got.question
            );
        }
        if came == "left" || came == "won" {
            assert!(!got.said.is_empty(), "{case}");
        }
        reached.insert(got.came);
    }
    assert_eq!(reached, OUTCOME_WORDS.iter().copied().collect());

    // Ten, three of them kept: those three are judged.
    let ten: Vec<Kept> = (0..10)
        .map(|at| kept(&format!("w{at}"), &letter(at), if at % 3 == 0 && at > 0 { "kept" } else { "failed" }))
        .collect();
    assert_eq!(outcome(&asked(ten)).unwrap().judge, [3, 6, 9]);
    let said = outcome(&won("")).unwrap().said;
    assert_eq!(said, "its A/B test of otter and slate was judged by shop-reviewer-1: slate");
    let failed = outcome(&asked(vec![kept("otter", "A", "failed"), kept("slate", "B", "unkept")])).unwrap();
    assert!(
        failed.why.contains("none of its 2 contenders' work could be kept (otter failed, slate unkept)"),
        "{}",
        failed.why
    );
    let wrong = outcome(&Asked { losers: "burn".into(), ..asked(two()) }).unwrap_err();
    assert!(wrong.contains("`losers` is one of branch, discard"), "{wrong}");
    let astray = outcome(&Asked { judged: "won".into(), winner: "glen".into(), ..asked(two()) }).unwrap_err();
    assert!(astray.contains("`glen` won, which is no entry"), "{astray}");
}

/// An entry as its row says it once its contender has ended.
fn worked(way: &str, letter: &str, state: &str, agent: &str, stat: &str, said: &str) -> Kept {
    Kept {
        agent: agent.into(),
        branch: format!("lane/shop-371--{}", letter.to_lowercase()),
        changed: stat.into(),
        said: said.into(),
        ..kept(way, letter, state)
    }
}

const STAT: &str = " src/cart.rs | 600 ++++++\n src/till.rs | 60 +++---\n 11 files changed, 634 insertions(+), 26 deletions(-)\n";

#[test]
fn what_a_person_asked_about_a_contest_is_told_of_each_entry_and_what_they_may_answer() {
    // What `git diff --stat` says, in a few words; nothing where it says nothing so.
    for (stat, said) in [
        (STAT, "11 files, +634 \u{2212}26"),
        (" a | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)", "1 file, +1 \u{2212}1"),
        (" 2 files changed, 5 deletions(-)", "2 files, +0 \u{2212}5"),
        (" 3 files changed, 7 insertions(+)", "3 files, +7 \u{2212}0"),
        ("", ""),
        ("nothing here", ""),
    ] {
        assert_eq!(described::changed(stat), said, "{stat:?}");
    }
    // What an agent said, briefly: its first line that says something, cut at a word.
    assert_eq!(
        described::brief("\n  I could not   finish the tests.\nMore."),
        "I could not finish the tests."
    );
    let long = "word ".repeat(60);
    let cut = described::brief(&long);
    assert!(cut.ends_with('…') && cut.chars().count() <= described::BRIEF + 1, "{cut}");
    assert_eq!(described::brief("  \n "), "");

    let otter = || worked("otter", "B", "kept", "shop-otter-1", STAT, "Done: the cart is there.");
    let slate =
        || worked("slate", "A", "unkept", "shop-slate-1", "", "I could not finish: the tests would not run.");
    let one = || Asked { entries: vec![slate(), otter()], ..Default::default() };

    // One kept, for a person: what each is, in words that need nothing but the card, answered yes or no.
    let asked = outcome(&one()).unwrap();
    assert_eq!((asked.came, asked.ends), ("asks", "asks"));
    assert_eq!(
        asked.question,
        "Only otter's work could be kept: B is otter's work (shop-otter-1, branch lane/shop-371--b, 11 files, +634 \u{2212}26); \
         slate's was not kept (it ended with nothing kept; its agent said \u{201c}I could not finish: the tests would not run.\u{201d}). \
         Take otter's work? Each is marked in the card's A/B testing tab. Answer yes or no."
    );
    // A row that says little is said by what it says.
    let bare = Asked {
        entries: vec![kept("slate", "A", "failed"), kept("otter", "B", "kept")],
        ..Default::default()
    };
    assert_eq!(
        outcome(&bare).unwrap().question,
        "Only otter's work could be kept: B is otter's work; slate's was not kept (it failed). Take otter's work? Each is marked in the card's A/B testing tab. Answer yes or no."
    );

    // Each word that says yes takes it, as its letter and its way do; each that says no leaves the card for a person.
    for answer in YES.iter().copied().chain(["Yes", " yes. ", "OK!", "B", "otter"]) {
        let took = outcome(&Asked { answer: answer.into(), ..one() }).unwrap();
        assert_eq!(
            (took.came, took.ends, took.winner.as_str(), took.entries.clone()),
            ("won", "on", "otter", vec![("", "keep"), ("won", "take")]),
            "{answer:?}"
        );
    }
    for answer in NO.iter().copied().chain(["No.", "LEAVE"]) {
        let left = outcome(&Asked { answer: answer.into(), ..one() }).unwrap();
        assert_eq!((left.came, left.ends, left.state), ("left", "left", "left"), "{answer:?}");
        assert_eq!(left.entries, vec![("", "keep"), ("", "keep")], "{answer:?}");
        assert!(
            left.why.contains("otter's work is not taken: the card is left for a person"),
            "{}",
            left.why
        );
        assert!(left.said.contains("came to nothing"), "{}", left.said);
    }
    // What is neither is asked again, saying so.
    let again = outcome(&Asked { answer: "maybe".into(), ..one() }).unwrap();
    assert_eq!(again.came, "asks");
    assert!(
        again.question.starts_with("`maybe` is not yes or no, nor one of the entries. Only otter's work"),
        "{}",
        again.question
    );
    // A letter of an entry whose work was not kept is none to take.
    assert_eq!(outcome(&Asked { answer: "A".into(), ..one() }).unwrap().came, "asks");

    // Several kept: each on a line of its own by its letter, those not kept with why, answered with the letter.
    let three = || Asked {
        entries: vec![
            worked("otter", "A", "kept", "shop-otter-1", STAT, ""),
            worked("slate", "B", "kept", "shop-slate-1", " 1 file changed, 2 insertions(+)", ""),
            worked("lark", "C", "failed", "shop-lark-1", "", "The build broke."),
        ],
        judged: "undecided".into(),
        why: "a tie".into(),
        ..Default::default()
    };
    let asked = outcome(&three()).unwrap();
    assert_eq!(
        asked.question.lines().collect::<Vec<_>>(),
        [
            "The judges decided nothing (a tie). Which entry is taken?",
            "A is otter's work (shop-otter-1, branch lane/shop-371--a, 11 files, +634 \u{2212}26).",
            "B is slate's work (shop-slate-1, branch lane/shop-371--b, 1 file, +2 \u{2212}0).",
            "lark's was not kept (it failed; its agent said \u{201c}The build broke.\u{201d}).",
            "Each is marked in the card's A/B testing tab.",
            "Answer with the letter of the one to take, or no to take none.",
        ]
    );
    // What the judging asked leads, with each said under it.
    let scored = Asked {
        judged: "person".into(),
        question: "The judges scored A: 7; B: 7. Which entry is taken (A, B)?".into(),
        ..three()
    };
    let asked = outcome(&scored).unwrap().question;
    assert!(
        asked.starts_with("The judges scored A: 7; B: 7. Which entry is taken (A, B)?\nA is otter's work"),
        "{asked}"
    );
    // Yes says not which; a letter, in either case, or a way does; no leaves it.
    let yes = outcome(&Asked { answer: "yes".into(), ..three() }).unwrap();
    assert_eq!(yes.came, "asks");
    assert!(
        yes.question
            .starts_with("`yes` says not which: the work of 2 entries was kept.\nWhich entry is taken?"),
        "{}",
        yes.question
    );
    for (answer, winner) in [("B", "slate"), ("b", "slate"), ("otter", "otter"), ("a.", "otter")] {
        assert_eq!(outcome(&Asked { answer: answer.into(), ..three() }).unwrap().winner, winner, "{answer}");
    }
    let none = outcome(&Asked { answer: "no".into(), ..three() }).unwrap();
    assert_eq!((none.came, none.state), ("left", "left"));
    assert!(none.why.contains("no entry's work is taken"), "{}", none.why);
    let astray = outcome(&Asked { answer: "Q".into(), ..three() }).unwrap();
    assert!(
        astray.question.starts_with("`Q` is none of the entries whose work was kept.\nWhich entry is taken?"),
        "{}",
        astray.question
    );
}

fn entry(way: &str, letter: &str, state: &str, unhidden: bool) -> Shown {
    Shown {
        way: way.into(),
        letter: letter.into(),
        state: state.into(),
        key: format!("shop-1--{way}"),
        said: format!("{way} did it."),
        changed: " CHANGES.md | 1 +".into(),
        patch: "+changed".into(),
        unhidden,
    }
}

#[test]
fn judges_are_shown_each_entry_whose_work_was_kept_by_its_letter_and_never_told_one_was_not_held_apart_silently()
 {
    let entries = [
        entry("otter", "A", "kept", false),
        entry("slate", "B", "kept", true),
        entry("fern", "C", "unkept", false),
    ];
    // Blind: by letters alone, the one that failed to keep its work not shown.
    let (text, letters) = shown(&entries, true);
    assert_eq!(letters, ["A", "B"]);
    assert!(
        text.contains("## Entry A") && text.contains("## Entry B") && !text.contains("## Entry C"),
        "{text}"
    );
    assert!(!text.contains("otter") || text.contains("shop-1--otter"), "{text}");
    assert!(!text.contains("Done as"), "{text}");
    assert!(text.contains("`.lanes/shop-1--slate`, on the branch `lane/shop-1--slate`"), "{text}");
    assert!(text.contains("+changed") && text.contains("slate did it."), "{text}");
    // The one not held apart says so, under its own letter and no other.
    let b = &text[text.find("## Entry B").unwrap()..];
    assert!(b.contains("Not held apart"), "{text}");
    assert!(!text[..text.find("## Entry B").unwrap()].contains("Not held apart"), "{text}");
    // Not blind: each by its way as well.
    let (text, _) = shown(&entries, false);
    assert!(text.contains("Done as `otter`") && text.contains("Done as `slate`"), "{text}");
    // An entry that said nothing, changed nothing and has no patch is shown as that.
    let quiet = Shown {
        said: String::new(),
        changed: String::new(),
        patch: String::new(),
        ..entry("otter", "A", "kept", false)
    };
    let (text, _) = shown(&[quiet], true);
    assert!(text.contains("(nothing)") && !text.contains("```"), "{text}");
    // Whose an entry is, is said nowhere a judge reads where judging is blind.
    let blind = [
        Shown { key: "shop-1--a".into(), said: "Done.".into(), ..entry("otter", "A", "kept", false) },
        Shown { key: "shop-1--b".into(), said: "Done.".into(), ..entry("slate", "B", "kept", false) },
    ];
    let (text, _) = shown(&blind, true);
    assert!(!text.contains("otter") && !text.contains("slate"), "{text}");
    // One seat at least, each named.
    assert_eq!(seated(0), ["seat-1"]);
    assert_eq!(seated(3), ["seat-1", "seat-2", "seat-3"]);
}

#[test]
fn what_a_contest_came_to_says_which_entries_were_not_held_apart_and_says_nothing_of_it_where_all_were() {
    let asked = |unhidden: bool, judged: &str| Asked {
        entries: vec![kept("otter", "A", "kept"), Kept { unhidden, ..kept("slate", "B", "kept") }],
        judged: judged.into(),
        winner: "otter".into(),
        why: "the judges were split".into(),
        undecided: "left".into(),
        ..Default::default()
    };
    let held = outcome(&asked(false, "won")).unwrap();
    assert_eq!(held.said, "its A/B test of otter and slate was judged: otter");
    let won = outcome(&asked(true, "won")).unwrap();
    assert_eq!(
        won.said,
        "its A/B test of otter and slate was judged: otter; not held apart, and so could see the other contenders' work: slate"
    );
    let left = outcome(&asked(true, "undecided")).unwrap();
    assert!(
        left.said.ends_with("not held apart, and so could see the other contenders' work: slate"),
        "{}",
        left.said
    );
}

#[test]
fn what_a_talking_council_has_said_is_shown_to_each_seat_a_stretch_at_a_time() {
    let seat =
        |by: &str, pick: &str, words: &str| Said { by: by.into(), pick: pick.into(), words: words.into() };
    // What, what was said before, the stretch, what each seat said, and what each is shown after it.
    type Case<'a> = (&'a str, &'a str, i64, Vec<Said>, &'a str);
    let cases: Vec<Case> = vec![
        (
            "the first stretch, a seat that would take nothing yet",
            "",
            1,
            vec![seat("shop-judge-1", "A", "A is right. "), seat("shop-judge-2", "", "Not sure yet.")],
            "Stretch 1:\n- shop-judge-1 (would take A): A is right.\n- shop-judge-2 (would take none yet): Not sure yet.",
        ),
        (
            "the second, after the first",
            "Stretch 1:\n- shop-judge-1 (would take A): A.",
            2,
            vec![seat("shop-judge-1", "B", "B, on reflection.")],
            "Stretch 1:\n- shop-judge-1 (would take A): A.\n\nStretch 2:\n- shop-judge-1 (would take B): B, on reflection.",
        ),
    ];
    for (case, so_far, stretch, said, shown) in cases {
        assert_eq!(heard(so_far, stretch, &said), shown, "{case}");
    }
}

#[test]
fn the_comparison_a_contest_counts_under_by_itself_is_said_in_one_place_and_only_for_one_that_names_none() {
    // (the contests as handed, the comparisons of their own they come to)
    let cases: &[(serde_json::Value, &[&str])] = &[
        // A row that names no comparison, and one that names an empty one, have one of their own.
        (serde_json::json!([{ "id": "shop-contests-1" }]), &["ab-shop-contests-1"]),
        (serde_json::json!([{ "id": "shop-contests-1", "compared": "" }]), &["ab-shop-contests-1"]),
        // One counted under a comparison it names has none.
        (serde_json::json!([{ "id": "shop-contests-2", "compared": "otter-slate" }]), &[]),
        // A contest that is gone is known by its id alone.
        (serde_json::json!(["shop-contests-3"]), &["ab-shop-contests-3"]),
        // In their order, what names nothing being left out.
        (
            serde_json::json!([{ "id": "b" }, { "id": "c", "compared": "x" }, "a", "", { "compared": "" }, 7]),
            &["ab-b", "ab-a"],
        ),
        (serde_json::json!([]), &[]),
    ];
    for (tests, wanted) in cases {
        let came = owned(tests.as_array().unwrap());
        assert_eq!(came, wanted.iter().map(|one| one.to_string()).collect::<Vec<_>>(), "{tests}");
    }
    // And what the contenders come to is counted under the same name.
    let mut lone = setup(&["otter", "slate"]);
    lone.compared = String::new();
    assert_eq!(ways(&lone).unwrap().comparison, own(&lone.contest));
}

#[test]
fn a_contender_is_handed_of_how_its_work_is_finished_only_what_is_said() {
    use deciding::contested::ways::finishing;
    use serde_json::json;
    let steps = [json!("its tests pass")];
    let later = [json!("the whole gate, run when the work lands")];
    assert_eq!(
        serde_json::Value::Object(finishing(&steps, &later, " 45m ")),
        json!({ "finish": ["its tests pass"], "later": ["the whole gate, run when the work lands"], "lasts": "45m" })
    );
    assert_eq!(
        serde_json::Value::Object(finishing(&steps, &[], "")),
        json!({ "finish": ["its tests pass"] })
    );
    assert_eq!(serde_json::Value::Object(finishing(&[], &[], "  ")), json!({}));
}
