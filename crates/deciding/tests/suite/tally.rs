//! What the votes on a piece of work come to: a function of the votes
//! cast, how many look and how many have to approve, and of which words
//! are counted for it and against it, held by a table of cases that
//! reaches every word it can answer.

use std::collections::BTreeSet;

use deciding::tally::{Counted, NO, TALLY_WORDS, Tallied, Vote, YES, tally};

/// A vote: who, what they chose and what they said.
fn vote(by: &str, decision: &str, words: &str) -> Vote {
    Vote { by: by.into(), decision: decision.into(), words: words.into() }
}

/// What these votes come to, of `seats` and `needed`, with the usual words.
fn of(votes: &[Vote], seats: i64, needed: i64) -> Result<Tallied, String> {
    tally(&Counted { votes, seats, needed, yes: YES, no: NO })
}

fn came(came: &'static str, by: &str, words: &str, approved: i64, against: i64) -> Tallied {
    Tallied { came, by: by.into(), words: words.into(), approved, against }
}

#[test]
fn what_the_votes_on_a_piece_of_work_come_to_is_counted_and_decided_as_soon_as_it_can_be() {
    let (a, b, c) = ("shop-reviewer-1", "shop-reviewer-2", "shop-reviewer-3");
    let cases: Vec<(&str, Vec<Vote>, i64, i64, Tallied)> = vec![
        (
            "one who looks alone and approves: passed, with its words as they are",
            vec![vote(a, "approve", "It does what the card asks.")],
            1,
            0,
            came("passed", a, "It does what the card asks.", 1, 0),
        ),
        (
            "one who looks alone and sends it back: failed, with who said what",
            vec![vote(a, "send_back", "The total is wrong.")],
            1,
            0,
            came("failed", a, &format!("{a}: The total is wrong."), 0, 1),
        ),
        (
            "one who looks alone and chooses neither: undecided, with what it said",
            vec![vote(a, "", "I cannot tell.")],
            1,
            0,
            came("undecided", "", &format!("{a}: I cannot tell."), 0, 0),
        ),
        (
            "of three, more than half having to approve, one approves: it waits",
            vec![vote(a, "approve", "Fine.")],
            3,
            0,
            came("", "", "", 1, 0),
        ),
        (
            "of three, one sends it back: two may still approve, and it waits",
            vec![vote(a, "send_back", "No.")],
            3,
            0,
            came("", "", "", 0, 1),
        ),
        (
            "of three, two approve: passed before the third has spoken, with each one's words",
            vec![vote(a, "approve", "Fine."), vote(b, "approve", "Good.")],
            3,
            0,
            came("passed", &format!("{a}, {b}"), &format!("{a}: Fine.\n{b}: Good."), 2, 0),
        ),
        (
            "of three, two send it back: failed, since two can no longer approve",
            vec![vote(a, "send_back", "No."), vote(b, "send_back", "Not yet.")],
            3,
            0,
            came("failed", &format!("{a}, {b}"), &format!("{a}: No.\n\n{b}: Not yet."), 0, 2),
        ),
        (
            "of three, two choose neither: enough can no longer approve, nobody said what is wrong, and it waits",
            vec![vote(a, "", "Hm."), vote(b, "", "Hm.")],
            3,
            0,
            came("", "", "", 0, 0),
        ),
        (
            "of three that all chose neither: undecided, with what each said",
            vec![vote(a, "", "Hm."), vote(b, "", "Hm?"), vote(c, "", "Hm!")],
            3,
            0,
            came("undecided", "", &format!("{a}: Hm.\n{b}: Hm?\n{c}: Hm!"), 0, 0),
        ),
        (
            "of two that both have to approve, one approves and one sends it back: failed",
            vec![vote(a, "approve", "Fine."), vote(b, "send_back", "No.")],
            2,
            2,
            came("failed", b, &format!("{b}: No."), 1, 1),
        ),
        (
            "of four, none said to be needed: three have to approve, and two is not enough",
            vec![vote(a, "approve", "Fine."), vote(b, "approve", "Fine.")],
            4,
            0,
            came("", "", "", 2, 0),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, votes, seats, needed, expected) in cases {
        let tallied = of(&votes, seats, needed).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(tallied, expected, "{what}");
        reached.insert(tallied.came);
    }
    assert_eq!(reached, TALLY_WORDS.iter().copied().collect(), "every word it can come to has a case");
}

#[test]
fn the_words_counted_for_and_against_the_work_are_those_whoever_asks_says_and_any_other_is_neither() {
    let (a, b) = ("shop-reviewer-1", "shop-reviewer-2");
    let told = |votes: &[Vote]| tally(&Counted { votes, seats: 2, needed: 2, yes: "agree", no: "object" });
    assert_eq!(
        told(&[vote(a, "agree", "Fine."), vote(b, "agree", "Good.")]),
        Ok(came("passed", &format!("{a}, {b}"), &format!("{a}: Fine.\n{b}: Good."), 2, 0))
    );
    assert_eq!(
        told(&[vote(a, "agree", "Fine."), vote(b, "object", "No.")]),
        Ok(came("failed", b, &format!("{b}: No."), 1, 1))
    );
    // The usual words are no longer counted: they are neither.
    assert_eq!(
        told(&[vote(a, "approve", "Fine."), vote(b, "send_back", "No.")]),
        Ok(came("undecided", "", &format!("{a}: Fine.\n{b}: No."), 0, 0))
    );
}

#[test]
fn votes_are_not_counted_of_nobody_looking_or_with_one_word_for_both() {
    let one = [vote("shop-reviewer-1", "approve", "Fine.")];
    assert_eq!(
        of(&one, 0, 0).unwrap_err(),
        "`seats` is how many look at the card, and is at least one; it is 0"
    );
    let both = tally(&Counted { votes: &one, seats: 1, needed: 0, yes: "approve", no: "approve" });
    assert_eq!(
        both.unwrap_err(),
        "what is counted for the work and what against it are two words; they are `approve` and `approve`"
    );
    let none = tally(&Counted { votes: &one, seats: 1, needed: 0, yes: "", no: "send_back" });
    assert!(none.is_err());
}
