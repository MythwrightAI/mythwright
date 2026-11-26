//! What an item said to be due a review is owed: a table of cases that
//! reaches every word it can be answered with.

use std::collections::BTreeSet;

use deciding::reviews::{OWED_WORDS, Owed, OwedCame, owed};
use serde_json::json;

/// A card in `review`, where two can look and one looks at each, sent
/// back at most twice.
fn card() -> Owed {
    Owed {
        item: "shop-1".into(),
        found: true,
        status: "review".into(),
        here: "review".into(),
        agents: 2,
        seats: 1,
        needed: 1,
        most: 2,
        rounds: 0,
        back: vec![],
        kept: vec![],
        again: false,
        held: false,
        role: "reviewer".into(),
        by: String::new(),
    }
}

#[test]
fn what_an_item_said_to_be_due_a_review_is_owed() {
    let left = |rounds: i64| OwedCame::Left {
        rounds,
        said: format!(
            "shop-1 is in review again after being sent back {rounds} times. Nobody was asked this time: it is \
             left for a person to look at."
        ),
    };
    let person = |why: &str| OwedCame::Person {
        said: format!("shop-1 is in review, where {why}: it waits for a person to review it."),
    };
    let under_way = json!({ "card": "shop-1", "review": "review-shop-1-40" });
    let cases: Vec<(&str, Owed, OwedCame)> = vec![
        ("a card that came to be looked at", card(), OwedCame::Review),
        ("one that is not there", Owed { found: false, status: String::new(), ..card() }, OwedCame::Nothing),
        ("one moved on since", Owed { status: "done".into(), ..card() }, OwedCame::Nothing),
        (
            "with nobody to look, it waits for a person",
            Owed { agents: 0, ..card() },
            person("nobody in the fleet has the role reviewer"),
        ),
        (
            "with nobody to look and no role said",
            Owed { agents: 0, role: String::new(), ..card() },
            person("nobody in the fleet has the role that looks there"),
        ),
        (
            "with fewer than are to look, it waits for a person",
            Owed { seats: 3, ..card() },
            person("its stage has 3 look at each card, and the role reviewer has 2"),
        ),
        (
            "with more to approve than look, it waits for a person",
            Owed { needed: 2, ..card() },
            person("its stage needs 2 to approve, and 1 look"),
        ),
        (
            "said to wait for a person already when it came: it is not said again",
            Owed { agents: 0, kept: vec![json!({ "card": "shop-1", "review": "" })], ..card() },
            OwedCame::Nothing,
        ),
        (
            "with nobody to look and held, it waits to be pressed first",
            Owed { agents: 0, held: true, ..card() },
            OwedCame::Held,
        ),
        (
            "with nobody to look and sent back as often as is allowed",
            Owed { agents: 0, rounds: 2, ..card() },
            left(2),
        ),
        (
            "with nobody to look and moved on since",
            Owed { agents: 0, status: "done".into(), ..card() },
            OwedCame::Nothing,
        ),
        (
            "with nobody to look, one agent a rung names reviews it",
            Owed {
                agents: 0,
                by: "shop-lead-1".into(),
                kept: vec![json!({ "card": "shop-1", "review": "" })],
                again: true,
                ..card()
            },
            OwedCame::Review,
        ),
        (
            "one a rung names, and sent back as often as is allowed",
            Owed { agents: 0, by: "shop-lead-1".into(), rounds: 2, ..card() },
            left(2),
        ),
        (
            "said to be due while it has a review under way",
            Owed { kept: vec![under_way.clone()], ..card() },
            OwedCame::Nothing,
        ),
        (
            "its review says itself that another is to begin",
            Owed { kept: vec![under_way.clone()], again: true, ..card() },
            OwedCame::Review,
        ),
        (
            "another card has a review under way",
            Owed { kept: vec![json!({ "card": "shop-2", "review": "review-shop-2-41" })], ..card() },
            OwedCame::Review,
        ),
        ("held until something its column offers is pressed", Owed { held: true, ..card() }, OwedCame::Held),
        (
            "held with a review under way: it is not held back from what it has",
            Owed { held: true, kept: vec![under_way.clone()], ..card() },
            OwedCame::Nothing,
        ),
        (
            "held, and sent back as often as is allowed",
            Owed { held: true, rounds: 2, ..card() },
            OwedCame::Held,
        ),
        ("sent back as often as is allowed, by its row", Owed { rounds: 2, ..card() }, left(2)),
        (
            "sent back as often as is allowed, as whoever asks counted",
            Owed { rounds: 1, back: vec![json!("shop-1"), json!("shop-2"), json!("shop-1")], ..card() },
            left(2),
        ),
        (
            "counted as mappings that name it",
            Owed {
                back: vec![json!({ "card": "shop-1", "at": "12" }), json!({ "card": "shop-1", "at": "30" })],
                ..card()
            },
            left(2),
        ),
        (
            "left already when it came: it is not said again",
            Owed { rounds: 2, kept: vec![json!({ "card": "shop-1", "review": "" })], ..card() },
            OwedCame::Nothing,
        ),
        (
            "sent back only to be brought up to date, which is counted apart",
            Owed {
                rounds: 1,
                back: vec![
                    json!({ "card": "shop-1", "at": "12", "behind": true }),
                    json!({ "card": "shop-1", "at": "30", "behind": "true" }),
                    json!({ "card": "shop-1", "at": "44", "behind": false }),
                ],
                ..card()
            },
            OwedCame::Review,
        ),
        (
            "brought up to date as often as is allowed",
            Owed {
                back: (0..deciding::reviews::BEHIND)
                    .map(|at| json!({ "card": "shop-1", "at": at.to_string(), "behind": true }))
                    .collect(),
                ..card()
            },
            OwedCame::Left {
                rounds: 3,
                said: "shop-1 is in review again after its branch was brought up to date with the project's 3 \
                       times, having no longer merged once it was approved. Nobody was asked this time: it is left \
                       for a person to look at and merge."
                    .into(),
            },
        ),
        (
            "kept as the card itself, as a text from before kept it",
            Owed { kept: vec![json!("shop-1")], ..card() },
            OwedCame::Nothing,
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, meant) in cases {
        let came = owed(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(came, meant, "{what}");
        reached.insert(came.word());
    }
    assert_eq!(
        reached,
        OWED_WORDS.iter().copied().collect(),
        "every word it can be answered with has a case"
    );

    assert_eq!(
        owed(&Owed { here: String::new(), ..card() }).unwrap_err(),
        "`here` is the column whose items are looked at, and none is said"
    );
    assert_eq!(
        owed(&Owed { rounds: -1, ..card() }).unwrap_err(),
        "`rounds` is a count, and is not less than none; it is -1"
    );
    assert_eq!(
        owed(&Owed { status: String::new(), ..card() }).unwrap_err(),
        "the item was found, and its `status` says no column"
    );
}

#[test]
fn what_a_verdict_on_an_item_comes_to_and_what_is_said_of_it() {
    use deciding::reviews::{VERDICT_WORDS, Verdict, VerdictCame, verdict};
    // A card in `review`, approved by one who looked alone; it goes on to
    // `done` with its work merged first, and is sent back to `doing`.
    let passed = || Verdict {
        verdict: "passed".into(),
        item: "shop-1".into(),
        status: "review".into(),
        here: "review".into(),
        pass: "done".into(),
        fail: "doing".into(),
        merge: true,
        by: "shop-reviewer-1".into(),
        words: "It does what the card asks.".into(),
        approved: 1,
        seats: 1,
        ..Verdict::default()
    };
    let failed = || Verdict {
        verdict: "failed".into(),
        words: "The total is wrong.".into(),
        against: 1,
        approved: 0,
        ..passed()
    };
    let stuck = |why: &str| {
        VerdictCame::Stuck(format!(
            "shop-1 was approved by shop-reviewer-1, and its branch lane/shop-1 could not be merged into the \
             project's: {why}. It stays in review; when what is in the way is put right, merge it and move it on."
        ))
    };
    let cases: Vec<(&str, Verdict, VerdictCame, &str)> = vec![
        ("approved, its work to be merged first", passed(), VerdictCame::Merge, ""),
        (
            "merged: it goes on, and that is said as it is moved",
            Verdict { merged: "merged".into(), ..passed() },
            VerdictCame::On("approved by shop-reviewer-1; lane/shop-1 was merged".into()),
            "",
        ),
        (
            "nothing to merge: it goes on",
            Verdict { merged: "nothing".into(), ..passed() },
            VerdictCame::On("approved by shop-reviewer-1".into()),
            "",
        ),
        (
            "where nothing is merged it goes on at once, and several looked",
            Verdict { merge: false, seats: 3, approved: 2, by: "a, b".into(), ..passed() },
            VerdictCame::On("approved by a, b, 2 of 3".into()),
            "",
        ),
        (
            "its branch no longer merges: back to whoever worked it",
            Verdict { merged: "conflict".into(), conflict: "CONFLICT in total.rs".into(), ..passed() },
            VerdictCame::Back(
                "It was approved, and its branch lane/shop-1 no longer merges into the project's branch as that \
                 is now. Merge the project's branch into yours, put right what conflicts, and hand it in again. \
                 What git said: CONFLICT in total.rs"
                    .into(),
            ),
            "",
        ),
        (
            "its branch no longer merges, and the project's is being merged into it for whoever worked it",
            Verdict {
                merged: "conflict".into(),
                conflict: "CONFLICT in total.rs".into(),
                begun: true,
                files: vec!["total.rs".into(), "cart.rs".into()],
                ..passed()
            },
            VerdictCame::Back(
                "It was approved, and its branch lane/shop-1 no longer merges into the project's branch as that \
                 is now. The project's branch is being merged into yours where you work, and that merge is under \
                 way for you: what conflicts is marked in total.rs, cart.rs, as git marks it. Put each right, \
                 keeping what both sides meant, and leave no marker. Change files and nothing else: do not run git \
                 to commit, merge or call the merge off. What you leave is committed as the end of that merge, and \
                 it is looked at again. What git said: CONFLICT in total.rs"
                    .into(),
            ),
            "",
        ),
        (
            "and where that could not be begun, it is a person's to put right",
            Verdict { merged: "conflict".into(), why: "its place holds changes that were never committed".into(), ..passed() },
            stuck("its place holds changes that were never committed"),
            "",
        ),
        (
            "something is in the way of the merge",
            Verdict { merged: "stuck".into(), why: "the project's folder has changes of its own".into(), ..passed() },
            stuck("the project's folder has changes of its own"),
            "",
        ),
        ("the merge failed outright, with nothing said of why", Verdict { failed: true, ..passed() }, stuck("the merge failed"), ""),
        ("approved, and nowhere to go on to", Verdict { pass: String::new(), ..passed() }, VerdictCame::Nothing, ""),
        ("approved, and moved by somebody since", Verdict { status: "draft".into(), ..passed() }, VerdictCame::Nothing, ""),
        (
            "approved by one, and verdicts are said where the work is reported",
            Verdict { reports: true, ..passed() },
            VerdictCame::Merge,
            "shop-1 was approved by shop-reviewer-1: It does what the card asks.",
        ),
        (
            "approved by several, with nothing said",
            Verdict { reports: true, seats: 3, approved: 2, words: String::new(), ..passed() },
            VerdictCame::Merge,
            "shop-1 was approved, 2 of 3.",
        ),
        ("sent back, with what was said", failed(), VerdictCame::Back("The total is wrong.".into()), ""),
        (
            "sent back by several, and said where the work is reported",
            Verdict { reports: true, seats: 3, against: 2, ..failed() },
            VerdictCame::Back("The total is wrong.".into()),
            "shop-1 was sent back to doing, 2 of 3 against: The total is wrong.",
        ),
        (
            "sent back by one, and said there",
            Verdict { reports: true, ..failed() },
            VerdictCame::Back("The total is wrong.".into()),
            "shop-1 was sent back to doing: The total is wrong.",
        ),
        (
            "sent back, and moved by somebody since: nothing, and nothing said",
            Verdict { reports: true, status: "draft".into(), ..failed() },
            VerdictCame::Nothing,
            "",
        ),
        (
            "neither approved nor sent back",
            Verdict { verdict: "undecided".into(), words: "shop-reviewer-1: could not be asked".into(), ..passed() },
            VerdictCame::Left(
                "shop-1: those who looked at it neither approved it nor sent it back (shop-reviewer-1: could not \
                 be asked). It is left in review for a person."
                    .into(),
            ),
            "",
        ),
        (
            "and with nothing said",
            Verdict { verdict: "undecided".into(), words: String::new(), ..passed() },
            VerdictCame::Left(
                "shop-1: those who looked at it neither approved it nor sent it back. It is left in review for a \
                 person."
                    .into(),
            ),
            "",
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, meant, told) in cases {
        let (came, said) = verdict(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(came, meant, "{what}");
        assert_eq!(said, told, "{what}");
        reached.insert(came.word());
    }
    assert_eq!(
        reached,
        VERDICT_WORDS.iter().copied().collect(),
        "every word a verdict can come to has a case"
    );

    assert_eq!(
        verdict(&Verdict { verdict: "maybe".into(), ..passed() }).unwrap_err(),
        "`verdict` is `passed`, `failed` or `undecided`, and is `maybe`"
    );
    assert_eq!(
        verdict(&Verdict { merged: "half".into(), ..passed() }).unwrap_err(),
        "`merged` is what putting the work with the rest came to (`merged`, `nothing`, `conflict` or `stuck`), and \
         is `half`"
    );
    assert_eq!(
        verdict(&Verdict { seats: -1, ..passed() }).unwrap_err(),
        "`seats` is a count, and is not less than none; it is -1"
    );
}

#[test]
fn a_review_over_with_nothing_said_leaves_its_card_waiting_for_a_person_where_the_card_is_there_still() {
    use deciding::reviews::over;
    let kept = [
        json!({ "card": "shop-1", "review": "review-shop-1-40" }),
        json!({ "card": "shop-2", "review": "review-shop-2-41" }),
    ];
    let waits = "shop-1 is in review, and its review ended with nothing said of how it came out (it was stopped, \
                 or it failed): it waits for a person to review it.";
    // What it is, the review that is over, whether the card was found and
    // its column; and the card it was of, and what is said where it waits.
    type Case<'a> = (&'a str, &'a str, bool, &'a str, (&'a str, Option<&'a str>));
    let cases: Vec<Case> = vec![
        ("asked whose it was, before the card is read", "review-shop-1-40", false, "", ("shop-1", None)),
        ("the card is there still", "review-shop-1-40", true, "review", ("shop-1", Some(waits))),
        ("the card has gone on", "review-shop-1-40", true, "done", ("shop-1", None)),
        ("the card is gone", "review-shop-1-40", false, "", ("shop-1", None)),
        // Begun again as reviews are now arranged: kept under another name.
        ("another took its place", "review-shop-1-12", true, "review", ("", None)),
        ("no review named", "", true, "review", ("", None)),
    ];
    for (what, review, found, status, (item, said)) in cases {
        let (got, waiting) = over(&kept, review, found, status, "review");
        assert_eq!((got.as_str(), waiting.as_deref()), (item, said), "{what}");
    }
    // A card left for a person is kept with no review named, and no
    // review is ever called that.
    let left = [json!({ "card": "shop-1", "review": "" })];
    assert_eq!(over(&left, "", true, "review", "review"), (String::new(), None));
}
