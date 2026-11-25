//! What is wrong with a row that says what is done with the items of a
//! column: a case for each sentence it can say, in the words it says.

use std::collections::BTreeSet;

use deciding::column::{CHANGED_WORDS, Changed, Row, Taken, WRONG, changed, wrong};

/// A case: what it is, the row, and each thing that is wrong with it by
/// what it is about and the sentence.
type Case = (&'static str, Row, Vec<(&'static str, String)>);

/// A row that can work: cards in `review` are looked at by one of two,
/// go on to `done` and are sent back to `doing`.
fn row() -> Row {
    Row {
        title: "review".into(),
        column: "review".into(),
        pass: "done".into(),
        fail: "doing".into(),
        columns: Some(["draft", "ready", "doing", "review", "done"].iter().map(|c| c.to_string()).collect()),
        role: "reviewer".into(),
        agents: 2,
        seats: 1,
        needed: 1,
        reviews: true,
        works: false,
    }
}

#[test]
fn what_is_wrong_with_a_row_is_said_in_a_sentence_for_each_thing_and_nothing_is_said_of_one_that_can_work() {
    let cols = "its columns are draft, ready, doing, review, done";
    let astray = |which: &str| {
        format!(
            "The stage review names a column the board has not got ({cols}): {which}. Put it right in the project's stages."
        )
    };
    let cases: Vec<Case> = vec![
        ("a row that can work", row(), vec![]),
        (
            "its own column is not there",
            Row { column: "looked".into(), title: String::new(), ..row() },
            vec![(
                "astray",
                format!("The stage looked names a column the board has not got ({cols}): its own, looked. Put it right in the project's stages."),
            )],
        ),
        (
            "where a card goes on to is not there",
            Row { pass: "shipped".into(), ..row() },
            vec![("astray", astray("where a card goes on to, shipped"))],
        ),
        (
            "where a card is sent back to is not there, where cards are looked at",
            Row { fail: "redo".into(), ..row() },
            vec![("astray", astray("where a card is sent back to, redo"))],
        ),
        ("that is nothing where cards are worked", Row { fail: "redo".into(), reviews: false, ..row() }, vec![]),
        ("with no board to read, nothing is said of columns", Row { column: "looked".into(), columns: None, ..row() }, vec![]),
        (
            "more look than there are",
            Row { seats: 3, ..row() },
            vec![(
                "short",
                "The stage review has 3 look at each card, and its role, reviewer, has 2: nothing is reviewed there until that is put right, and a card waits for a person.".to_string(),
            )],
        ),
        (
            "more have to approve than look",
            Row { needed: 2, ..row() },
            vec![(
                "short",
                "The stage review needs 2 to approve, and 1 look: nothing is reviewed there until that is put right, and a card waits for a person.".to_string(),
            )],
        ),
        ("with nobody to look, nothing is said of how many do", Row { agents: 0, seats: 3, ..row() }, vec![]),
        (
            "nobody to work what is worked here",
            Row { column: "ready".into(), role: "worker".into(), agents: 0, reviews: false, works: true, ..row() },
            vec![(
                "idle",
                "Nobody in the fleet has the role worker, which works the cards in ready: a card put there is not begun until somebody has. Give a row of the project's agents that role.".to_string(),
            )],
        ),
        (
            "nobody in the role of a stage that begins something for each card, though what it begins may ask nobody",
            Row {
                title: "checks".into(),
                column: "ready".into(),
                role: "checker".into(),
                agents: 0,
                reviews: false,
                works: false,
                ..row()
            },
            vec![(
                "idle",
                "Nobody in the fleet has the role checker, which the stage checks is for: it begins nothing for a card in ready until somebody has, whether or not what it begins asks an agent. Give a row of the project's agents that role, or say a role somebody has.".to_string(),
            )],
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, meant) in cases {
        let found = wrong(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(found, meant, "{what}");
        reached.extend(found.iter().map(|(about, _)| *about));
    }
    assert_eq!(reached, WRONG.iter().copied().collect(), "every sentence it can say has a case");

    assert_eq!(
        wrong(&Row { column: String::new(), ..row() }).unwrap_err(),
        "`column` is the column whose items the row is for, and none is said"
    );
    assert_eq!(
        wrong(&Row { seats: -1, ..row() }).unwrap_err(),
        "`seats` is a count, and is not less than none; it is -1"
    );
}

/// A row taken again: three looked at each card of `review` and two had
/// to approve; from now two look, with three in the role as before.
fn taken() -> Taken {
    Taken {
        column: "review".into(),
        was: "std/review/3/2".into(),
        now: "std/review/2/0".into(),
        seats: 2,
        needed: 0,
        agents: 3,
        had: 3,
        restart: false,
    }
}

#[test]
fn what_a_row_taken_again_came_to_is_said_once_where_cards_are_looked_at_another_way() {
    let told = |words: &str| Changed::Arranged { told: Some(words.to_string()) };
    let cases: Vec<(&str, Taken, Changed)> = vec![
        ("looked at as they were", Taken { was: "std/review/2/0".into(), ..taken() }, Changed::Same),
        (
            "fewer look, and more than half of them pass it where none is said",
            taken(),
            told(
                "From now each card in review is looked at by a council of 2, and passes when 2 approve. Reviews in \
                 session finish as they began.",
            ),
        ),
        (
            "how many have to approve is said",
            Taken { now: "std/review/3/1".into(), seats: 3, needed: 1, ..taken() },
            told(
                "From now each card in review is looked at by a council of 3, and passes when 1 approve. Reviews in \
                 session finish as they began.",
            ),
        ),
        (
            "one looks",
            Taken { now: "std/review/1/0".into(), seats: 1, ..taken() },
            told(
                "From now each card in review is looked at by a council of one. Reviews in session finish as they \
                 began.",
            ),
        ),
        (
            "what is under way is begun again",
            Taken { now: "std/review/1/0".into(), seats: 1, restart: true, ..taken() },
            told(
                "From now each card in review is looked at by a council of one. Reviews under way are begun again \
                 that way.",
            ),
        ),
        (
            "another blueprint looks",
            Taken { now: "shop/look/3/2".into(), seats: 3, needed: 2, ..taken() },
            told(
                "From now each card in review is looked at by a council of 3, and passes when 2 approve. Reviews in \
                 session finish as they began.",
            ),
        ),
        (
            "nobody was looking: nothing is said",
            Taken { had: 0, ..taken() },
            Changed::Arranged { told: None },
        ),
        (
            "nobody looks now: nothing is said",
            Taken { agents: 0, ..taken() },
            Changed::Arranged { told: None },
        ),
        (
            "a row from before that said nothing of it",
            Taken { was: String::new(), ..taken() },
            told(
                "From now each card in review is looked at by a council of 2, and passes when 2 approve. Reviews in \
                 session finish as they began.",
            ),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, taken, came) in cases {
        let found = changed(&taken).unwrap_or_else(|why| panic!("{what}: {why}"));
        assert_eq!(found, came, "{what}");
        reached.insert(found.word());
    }
    assert_eq!(reached, CHANGED_WORDS.iter().copied().collect::<BTreeSet<_>>(), "a word no case reaches");
}

#[test]
fn a_row_taken_again_that_makes_no_sense_is_refused_in_a_sentence() {
    let cases: Vec<(Taken, &str)> = vec![
        (
            Taken { column: String::new(), ..taken() },
            "`column` is the column whose items the row is for, and none is said",
        ),
        (
            Taken { now: String::new(), ..taken() },
            "`now` is how items are looked at from now, and nothing is said",
        ),
        (Taken { seats: -1, ..taken() }, "`seats` is a count, and is not less than none; it is -1"),
        (Taken { had: -2, ..taken() }, "`had` is a count, and is not less than none; it is -2"),
    ];
    for (taken, why) in cases {
        assert_eq!(changed(&taken), Err(why.to_string()));
    }
}
