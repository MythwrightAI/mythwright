//! What a turn that came for an item, and a round that ended, come to:
//! each a function of what the item's row says, held by a table of cases
//! that reaches every word it can answer.

use std::collections::BTreeSet;

use deciding::work::{ENDED_WORDS, Ended, EndedCame, Round, TURN_WORDS, Turn, TurnCame, ended, turn};

/// An item that was found, whose items are worked from `ready` and are in
/// `doing` while in hand, with what its row says.
fn row(status: &str, taken: &str, again: &[&str], moved: &[&str]) -> Turn {
    let words = |said: &[&str]| said.iter().map(|word| word.to_string()).collect();
    Turn {
        found: true,
        status: status.into(),
        here: "ready".into(),
        doing: "doing".into(),
        taken: taken.into(),
        again: words(again),
        moved: words(moved),
        most: 0,
        to: String::new(),
        else_to: "review".into(),
        at: 41,
    }
}

/// A round as these cases expect one: whether the item is taken now,
/// whether it is given again, and what it is told.
fn round(mine: bool, back: bool, note: &str, heard: &str) -> Round {
    Round {
        mine,
        back,
        resumed: false,
        answer: false,
        unkept: false,
        note: note.into(),
        tries: 0,
        most: 2,
        to: "review".into(),
        again: vec!["heard".into(), heard.into()],
        asking: "41".into(),
        heard: heard.parse().unwrap(),
    }
}

#[test]
fn what_a_turn_that_came_for_an_item_comes_to_is_said_by_its_row_whatever_was_heard_of_its_last_move() {
    let the_rounds_own = ["ready", "blueprint", "", "2"];
    let sent_back = ["review", "user", "The total is wrong.", "4"];
    let cases: Vec<(&str, Turn, TurnCame)> = vec![
        ("an item that is not there", Turn { found: false, ..row("", "", &[], &[]) }, TurnCame::Gone),
        (
            "one that has gone on to where it is looked at",
            row("review", "ready", &["heard", "2"], &[]),
            TurnCame::Gone,
        ),
        (
            "in its column for the first time, with nothing said",
            row("ready", "", &[], &[]),
            TurnCame::Round(round(true, false, "", "0")),
        ),
        (
            "in its column for the first time, put there with something said",
            row("ready", "", &[], &["draft", "user", "It is wanted this week.", "1"]),
            TurnCame::Round(round(true, false, "It is wanted this week.", "1")),
        ),
        (
            "put back in its column, having been worked from here",
            row("ready", "ready", &["heard", "2"], &["doing", "user", "Not yet.", "3"]),
            TurnCame::Round(round(false, true, "Not yet.", "3")),
        ),
        (
            "put back in its column with its round over and something said: that is what it is told",
            row("ready", "ready", &["asked"], &["doing", "user", "The total is wrong.", "5"]),
            TurnCame::Round(round(false, true, "The total is wrong.", "5")),
        ),
        (
            "put back in its column having been left, with nothing said",
            row("ready", "ready", &["left", "its place could not be made"], &["doing", "user", "", "5"]),
            TurnCame::Round(round(false, true, "", "5")),
        ),
        (
            "in hand, its agent having asked",
            row("doing", "ready", &["asked"], &the_rounds_own),
            TurnCame::Over,
        ),
        (
            "in hand, left for a person",
            row("doing", "ready", &["left", "it could not be committed"], &the_rounds_own),
            TurnCame::Over,
        ),
        (
            "in hand, sent back by something done with it where it is: told what that said, and how often that is",
            row("doing", "ready", &["again", "`cargo fmt --check` did not pass", "1"], &the_rounds_own),
            TurnCame::Round(Round {
                tries: 1,
                ..round(false, true, "`cargo fmt --check` did not pass", "2")
            }),
        ),
        (
            "in hand, left there by whoever had it before and taken up again: not one sent back",
            row("doing", "ready", &["resumed", ""], &the_rounds_own),
            TurnCame::Round(Round { resumed: true, ..round(false, false, "", "2") }),
        ),
        (
            "in hand, what it asked answered",
            row("doing", "ready", &["answer", "Without tax."], &the_rounds_own),
            TurnCame::Round(Round { answer: true, ..round(false, true, "Without tax.", "2") }),
        ),
        (
            "in hand, what was done not kept, for the second time",
            row("doing", "ready", &["unkept", "git has no name to commit as", "1"], &the_rounds_own),
            TurnCame::Round(Round {
                unkept: true,
                tries: 1,
                ..round(false, true, "git has no name to commit as", "2")
            }),
        ),
        (
            "sent back with something said, and no round told of it yet",
            row("doing", "ready", &["heard", "2"], &sent_back),
            TurnCame::Round(round(false, true, "The total is wrong.", "4")),
        ),
        (
            "sent back, with nothing kept of its rounds since it was tidied",
            row("doing", "ready", &[], &sent_back),
            TurnCame::Round(round(false, true, "The total is wrong.", "4")),
        ),
        (
            "sent back, and a round already told of it: a turn nobody meant",
            row("doing", "ready", &["heard", "4"], &sent_back),
            TurnCame::Over,
        ),
        (
            "in hand as its own round took it: a second turn for one arrival",
            row("doing", "ready", &["heard", "1"], &the_rounds_own),
            TurnCame::Over,
        ),
        (
            "in hand and never this column's: a person took it themselves",
            row("doing", "", &[], &["draft", "user", "", "1"]),
            TurnCame::Over,
        ),
        // Rows as a text from before left them: what was said of a move
        // was written by a handler, and the row said nothing of the move.
        (
            "from before: sent back, as the handler that heard it wrote",
            row("doing", "ready", &["back", "The total is wrong."], &[]),
            TurnCame::Round(round(false, true, "The total is wrong.", "0")),
        ),
        (
            "from before: put back in its column, as the handler that heard it wrote",
            row("ready", "ready", &["back", "Not yet."], &[]),
            TurnCame::Round(round(false, true, "Not yet.", "0")),
        ),
        (
            "from before, and moved since: the move's own word is what it is told",
            row("ready", "ready", &["back", "Not yet."], &["doing", "user", "Now.", "1"]),
            TurnCame::Round(round(false, true, "Now.", "1")),
        ),
        (
            "from before: put in its column with something said",
            row("ready", "", &["said", "It is wanted this week."], &[]),
            TurnCame::Round(round(true, false, "It is wanted this week.", "0")),
        ),
        (
            "from before: in hand, with nothing kept and no word of a move",
            row("doing", "ready", &[], &[]),
            TurnCame::Over,
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, meant) in cases {
        let came = turn(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(came, meant, "{what}");
        reached.insert(came.word());
    }
    assert_eq!(reached, TURN_WORDS.iter().copied().collect(), "every word a turn can come to has a case");

    // Where it goes on to, and how often at most, are the row's to say.
    let on = Turn { to: "weighing".into(), most: 3, ..row("ready", "", &[], &[]) };
    let TurnCame::Round(told) = turn(&on).unwrap() else { panic!("a round") };
    assert_eq!((told.to.as_str(), told.most), ("weighing", 3));

    // What makes no sense is refused in a sentence.
    let refused = |asked: Turn| turn(&asked).unwrap_err();
    assert_eq!(
        refused(Turn { here: String::new(), ..row("ready", "", &[], &[]) }),
        "`here` is the column whose items are worked, and none is said"
    );
    assert_eq!(
        refused(Turn { doing: String::new(), ..row("ready", "", &[], &[]) }),
        "`doing` is the column an item is in while it is in hand, and none is said"
    );
    assert_eq!(
        refused(Turn { most: -1, ..row("ready", "", &[], &[]) }),
        "`most` is how often at most an item is given again, and is not less than none; it is -1"
    );
    assert_eq!(refused(row("", "", &[], &[])), "the item was found, and its `status` says no column");
}

#[test]
fn what_a_round_that_ended_comes_to_for_whoever_asked_for_it() {
    let cases: Vec<(&str, Ended, EndedCame)> = vec![
        ("the round did what there was to do", Ended::default(), EndedCame::Done),
        (
            "what was done could not be kept, the first time",
            Ended { again: true, why: "nothing to commit as".into(), ..Ended::default() },
            EndedCame::Again(vec!["unkept".into(), "nothing to commit as".into(), "1".into()]),
        ),
        (
            "and the third",
            Ended { again: true, why: "still".into(), tries: 2, ..Ended::default() },
            EndedCame::Again(vec!["unkept".into(), "still".into(), "3".into()]),
        ),
        (
            "something done with it sent it back: it is given again, and its row says that and not that it was not kept",
            Ended {
                again: true,
                back: true,
                why: "`cargo fmt --check` did not pass".into(),
                tries: 1,
                ..Ended::default()
            },
            EndedCame::Again(vec!["again".into(), "`cargo fmt --check` did not pass".into(), "2".into()]),
        ),
        ("the round failed", Ended { failed: true, ..Ended::default() }, EndedCame::Freed),
        ("the item could not be taken", Ended { untaken: true, ..Ended::default() }, EndedCame::Freed),
        (
            "a round that failed is not given again, whatever it said",
            Ended { failed: true, again: true, ..Ended::default() },
            EndedCame::Freed,
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, meant) in cases {
        let came = ended(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(came, meant, "{what}");
        reached.insert(came.word());
    }
    assert_eq!(reached, ENDED_WORDS.iter().copied().collect(), "every word an ending can come to has a case");
    assert_eq!(
        ended(&Ended { tries: -1, ..Ended::default() }).unwrap_err(),
        "`tries` is how often an item has been given again, and is not less than none; it is -1"
    );
}

#[test]
fn what_an_item_said_to_be_due_comes_to_where_it_is_worked() {
    use deciding::work::{DUE_WORDS, Due, DueCame, due};
    let item = |status: &str, after: &[&str], clear: bool| Due {
        found: true,
        status: status.into(),
        here: "ready".into(),
        after: after.iter().map(|word| word.to_string()).collect(),
        clear,
        held: false,
    };
    let cases: Vec<(&str, Due, DueCame)> = vec![
        ("an item that is not there", Due { found: false, ..item("", &[], false) }, DueCame::Nothing),
        ("one moved on since it was said to be due", item("doing", &[], false), DueCame::Nothing),
        ("in its column, waiting on nothing", item("ready", &[], false), DueCame::Turn),
        ("in its column, waiting on others", item("ready", &["shop-1"], false), DueCame::Waits),
        ("what it waited on done", item("ready", &["shop-1"], true), DueCame::Turn),
        (
            "held until something its column offers is pressed",
            Due { held: true, ..item("ready", &[], false) },
            DueCame::Held,
        ),
        (
            "held and waiting on others: held first",
            Due { held: true, ..item("ready", &["shop-1"], false) },
            DueCame::Held,
        ),
        ("held, and moved on since", Due { held: true, ..item("doing", &[], false) }, DueCame::Nothing),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, meant) in cases {
        let came = due(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(came, meant, "{what}");
        reached.insert(came.word());
    }
    assert_eq!(reached, DUE_WORDS.iter().copied().collect(), "every word a due item can come to has a case");
    assert_eq!(
        due(&Due { here: String::new(), ..item("ready", &[], false) }).unwrap_err(),
        "`here` is the column whose items are worked, and none is said"
    );
    assert_eq!(
        due(&item("", &[], false)).unwrap_err(),
        "the item was found, and its `status` says no column"
    );
}

#[test]
fn something_is_started_for_an_item_once_for_each_time_it_comes_and_only_while_it_is_there() {
    use deciding::work::{COMING_WORDS, Coming, ComingCame, coming};
    use serde_json::json;
    let words = |said: &[&str]| said.iter().map(|word| word.to_string()).collect::<Vec<_>>();
    let item = |status: &str, moved: &[&str], kept: serde_json::Value| Coming {
        item: "shop-2".into(),
        found: true,
        status: status.into(),
        here: "check".into(),
        moved: words(moved),
        kept: kept.as_array().unwrap().clone(),
        held: false,
    };
    let begin = |at: &str| ComingCame::Begin { at: at.into() };
    let came = ["ready", "user", "", "3"];
    let cases: Vec<(&str, Coming, ComingCame)> = vec![
        (
            "an item that is not there",
            Coming { found: false, ..item("", &[], json!([])) },
            ComingCame::Nothing,
        ),
        (
            "one said to be due for a move from before, which has gone on since",
            item("review", &["check", "blueprint", "", "4"], json!([])),
            ComingCame::Nothing,
        ),
        (
            "one that went on after something was started for it",
            item("review", &["check", "blueprint", "", "4"], json!([{ "card": "shop-2", "at": "3" }])),
            ComingCame::Nothing,
        ),
        (
            "in the column, come there by a move nothing was started for",
            item("check", &came, json!([])),
            begin("3"),
        ),
        (
            "in the column, said to be due a second time for the one coming",
            item("check", &came, json!([{ "card": "shop-2", "at": "3" }])),
            ComingCame::Had,
        ),
        (
            "come there again after it was started for the time before",
            item("check", &["review", "user", "Not yet.", "5"], json!([{ "card": "shop-2", "at": "3" }])),
            begin("5"),
        ),
        (
            "in the column with another item begun for at the same count",
            item("check", &came, json!([{ "card": "shop-9", "at": "3" }])),
            begin("3"),
        ),
        ("added in the column and never moved", item("check", &[], json!([])), begin("0")),
        (
            "held until something its column offers is pressed",
            Coming { held: true, ..item("check", &came, json!([])) },
            ComingCame::Held,
        ),
        (
            "held, where this coming was begun for already: it has what it has",
            Coming { held: true, ..item("check", &came, json!([{ "card": "shop-2", "at": "3" }])) },
            ComingCame::Had,
        ),
        (
            "added there, and said to be due a second time",
            item("check", &[], json!([{ "card": "shop-2", "at": "0" }])),
            ComingCame::Had,
        ),
        (
            "on a board that keeps something else where a move is said: each saying begins",
            item("check", &["yesterday"], json!([{ "card": "shop-2", "at": "" }])),
            begin(""),
        ),
        (
            "what is kept being the items themselves, as a text from before might keep them",
            item("check", &came, json!(["shop-2"])),
            begin("3"),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, meant) in cases {
        let came = coming(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(came, meant, "{what}");
        reached.insert(came.word());
    }
    assert_eq!(reached, COMING_WORDS.iter().copied().collect(), "every word a coming can come to has a case");
    assert_eq!(
        coming(&Coming { here: String::new(), ..item("check", &[], json!([])) }).unwrap_err(),
        "`here` is the column whose items something is started for, and none is said"
    );
    assert_eq!(
        coming(&Coming { item: String::new(), ..item("check", &[], json!([])) }).unwrap_err(),
        "`item` is what was said to be due, and none is said"
    );
    assert_eq!(
        coming(&item("", &[], json!([]))).unwrap_err(),
        "the item was found, and its `status` says no column"
    );
}

#[test]
fn what_is_started_for_an_item_is_called_after_what_it_is_and_goes_round_as_often_as_its_row_says_or_the_usual()
 {
    use deciding::work::{called, most};
    assert_eq!(called("weighed", "shop-1").unwrap(), "weighed-shop-1");
    assert_eq!(called("Acme/Weigh It", "shop-1").unwrap(), "acme-weigh-it-shop-1");
    assert_eq!(
        called("", "shop-1").unwrap_err(),
        "what is started for an item is called after what it is and the item; they are `` and `shop-1`"
    );
    assert_eq!((most(0, 8).unwrap(), most(3, 8).unwrap()), (8, 3));
    assert_eq!(
        most(-1, 8).unwrap_err(),
        "how often at most is not less than none, and the usual is at least once; they are -1 and 8"
    );
}

#[test]
fn whether_an_item_is_still_as_a_round_has_it_is_said_by_its_moves_and_not_by_where_it_is() {
    use deciding::work::{HELD_WORDS, Held, HeldCame, held};
    let item = |status: &str, moved: &[&str], heard: Option<u64>| Held {
        status: status.into(),
        from: "ready".into(),
        doing: "doing".into(),
        moved: moved.iter().map(|word| word.to_string()).collect(),
        heard,
    };
    let cases: Vec<(&str, Held, HeldCame)> = vec![
        (
            "in its column: the round takes it",
            item("ready", &["draft", "user", "", "1"], Some(1)),
            HeldCame::Take,
        ),
        (
            "in hand as the round itself took it",
            item("doing", &["ready", "blueprint", "", "2"], Some(1)),
            HeldCame::Mine,
        ),
        (
            "in hand as it was sent back, which is the move the round was told of",
            item("doing", &["review", "user", "The total is wrong.", "4"], Some(4)),
            HeldCame::Mine,
        ),
        (
            "handed on already: a round begun again finds it where it put it",
            item("review", &["doing", "blueprint", "", "3"], Some(1)),
            HeldCame::Gone,
        ),
        (
            "handed on and sent back since: in hand again, and no longer this round's",
            item("doing", &["review", "user", "The total is wrong.", "4"], Some(1)),
            HeldCame::Gone,
        ),
        (
            "taken in hand by a person themselves after its turn came",
            item("doing", &["ready", "user", "", "2"], Some(1)),
            HeldCame::Gone,
        ),
        ("taken out of the way", item("draft", &["doing", "user", "", "3"], Some(1)), HeldCame::Gone),
        (
            "taken out by somebody and put back where the round expects it: its moves have gone on by two",
            item("doing", &["draft", "user", "", "6"], Some(4)),
            HeldCame::Gone,
        ),
        (
            "with nothing said of which move, it goes by where the item is, as it did",
            item("doing", &["review", "user", "", "4"], None),
            HeldCame::Mine,
        ),
        (
            "an item never moved, taken where its row keeps no word of moves",
            item("doing", &[], Some(0)),
            HeldCame::Mine,
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, meant) in cases {
        let came = held(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(came, meant, "{what}");
        reached.insert(came.word());
    }
    assert_eq!(
        reached,
        HELD_WORDS.iter().copied().collect(),
        "every word it can be answered with has a case"
    );
    assert_eq!(held(&item("", &[], None)).unwrap_err(), "the item's `status` says no column");

    // A round that never got as far as asking anybody has not given its
    // place back: whoever asked for it does.
    use deciding::work::{Ended, EndedCame, ended};
    assert_eq!(ended(&Ended { unreached: true, ..Ended::default() }).unwrap(), EndedCame::Freed);
}

#[test]
fn what_a_rounds_agent_asked_is_for_whoever_asked_for_the_round_to_say_where_the_round_says_who() {
    use deciding::work::{Ended, asks};
    let asked = |asker: &str, question: &str| Ended {
        asked: true,
        asker: asker.into(),
        question: question.into(),
        ..Ended::default()
    };
    assert_eq!(
        asks(&asked("shop-worker-1", "Which total?")),
        Some(("shop-worker-1".to_string(), "Which total?".to_string()))
    );
    // One that chose to ask and said nothing has asked what is wanted.
    assert_eq!(
        asks(&asked("shop-worker-1", "")).unwrap().1,
        "What is wanted here? (It chose to ask, and did not say what.)"
    );
    // A round that worked asks nothing; one that says its question itself
    // gives no asker; and one that failed says nothing of what it came to.
    assert_eq!(asks(&Ended::default()), None);
    assert_eq!(asks(&asked("", "Which total?")), None);
    assert_eq!(asks(&Ended { failed: true, ..asked("shop-worker-1", "Which total?") }), None);
}

#[test]
fn what_giving_an_item_to_an_agent_came_to_is_that_it_was_worked_that_the_agent_asked_or_who_was_not_there() {
    use deciding::work::{WORKED_WORDS, Worked, worked};
    let given = |available: bool, decision: &str| Worked {
        item: "shop-7".into(),
        available,
        decision: decision.into(),
        by: "shop-worker-1".into(),
        reply: "Which total?".into(),
        ..Worked::default()
    };
    // What a case expects: the word, the sentence, and who asked.
    let cases: Vec<(&str, Worked, (&str, &str, &str))> = vec![
        ("it was worked", given(true, "done"), ("on", "", "")),
        ("an agent that chose neither is taken to have done the work", given(true, ""), ("on", "", "")),
        ("its agent asked in place of working", given(true, "ask"), ("asks", "", "shop-worker-1")),
        (
            "its agent ended with a word that sends it back, which it is told the item may end with",
            Worked { ends: vec!["done=on redo=back".into()], ..given(true, "redo") },
            ("back", "Which total?", ""),
        ),
        (
            "or with one that has it given once more",
            Worked { ends: vec!["done=on".into(), "again=again".into()], ..given(true, "again") },
            ("again", "", ""),
        ),
        (
            "or with one that leaves it, which is said where it stays",
            Worked {
                ends: vec!["done=on blocked=left".into()],
                stays: "in draft".into(),
                ..given(true, "blocked")
            },
            ("left", "shop-7: shop-worker-1 chose `blocked`, and it is left for a person: Which total?", ""),
        ),
        (
            "there was nobody to give it to",
            given(false, ""),
            ("left", "shop-7: no agent is available to take this card; it stays in doing", ""),
        ),
        (
            "whoever the item names is not up",
            Worked { named: "shop-worker-9".into(), ..given(false, "") },
            (
                "left",
                "shop-7: shop-worker-9, whom the card is given to, is not an agent that is up; it stays in doing",
                "",
            ),
        ),
        (
            "whoever the item names is an agent the factory does not start",
            Worked { named: "walls".into(), external: true, ..given(false, "") },
            (
                "left",
                "shop-7: it is given to walls, an external agent, which takes up its own cards and is asked \
                 nothing here; it stays in doing",
                "",
            ),
        ),
        (
            "several looked at it at once and enough approved, whatever any one of them chose",
            Worked { seats: 2, decided: "passed".into(), ..given(true, "ask") },
            ("on", "", ""),
        ),
        (
            "or they no longer could, and it goes back with what those against it said",
            Worked {
                seats: 2,
                decided: "failed".into(),
                words: "shop-reviewer-2: The total is wrong.".into(),
                ..given(true, "")
            },
            ("back", "shop-reviewer-2: The total is wrong.", ""),
        ),
        (
            "or they came to neither, which a row has come to something else",
            Worked {
                seats: 2,
                decided: "undecided".into(),
                ends: vec!["passed=on failed=back undecided=again".into()],
                ..given(true, "")
            },
            ("again", "", ""),
        ),
        (
            "whoever was named for it by whoever asks is not up",
            Worked { named: "shop-worker-9".into(), told: true, ..given(false, "") },
            (
                "left",
                "shop-7: shop-worker-9, whom its row names, is not an agent that is up; it stays in doing",
                "",
            ),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, (word, says, asker)) in cases {
        let came = worked(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!((came.came, came.says.as_str(), came.asker.as_str()), (word, says, asker), "{what}");
        // Why, in words the item's row keeps: nobody was there, or the
        // one it is given to takes it up itself.
        if word == "left" && !asked.available {
            let why = match asked.external {
                true => "it is given to walls, an external agent, which takes it up itself",
                false => "there was nobody to give it to",
            };
            assert_eq!(came.why, why, "{what}");
        }
        if word == "asks" {
            assert_eq!(came.question, "Which total?", "{what}");
        }
        // Which way it was left, for whoever is told: nobody was there, an
        // external agent takes it up, or the word its agent chose; and
        // nothing where it was not left.
        let left_as = match (word, asked.available, asked.external) {
            ("left", false, true) => "external".to_string(),
            ("left", false, false) => "nobody".to_string(),
            ("left", true, _) => asked.decision.clone(),
            _ => String::new(),
        };
        assert_eq!(came.left_as, left_as, "{what}");
        reached.insert(came.came);
    }
    assert_eq!(reached, WORKED_WORDS.iter().copied().collect(), "every word it can come to has a case");
    assert_eq!(worked(&Worked::default()).unwrap_err(), "the `item` that was given to an agent is not said");
    // Where nobody was there, it stays where whoever asks says.
    let nobody = Worked { stays: "where it is".into(), ..given(false, "") };
    assert_eq!(
        worked(&nobody).unwrap().says,
        "shop-7: no agent is available to take this card; it stays where it is"
    );
}

#[test]
fn that_a_round_left_its_item_for_a_person_is_for_whoever_asked_for_it_to_say_with_why() {
    use deciding::work::{Ended, left};
    // The round, and why it left its item, where it did.
    let cases: [(&str, Ended, Option<&str>); 7] = [
        ("a round that worked leaves nothing", Ended::default(), None),
        (
            "one that says it left its item, with why",
            Ended { left: true, leaves: "its work could not be kept".into(), ..Ended::default() },
            Some("its work could not be kept"),
        ),
        ("one that says so and no why", Ended { left: true, ..Ended::default() }, Some("")),
        (
            "one that failed leaves its item, by why it failed",
            Ended { failed: true, reason: "its place could not be made".into(), ..Ended::default() },
            Some("its place could not be made"),
        ),
        (
            "one that failed and says nothing of why",
            Ended { failed: true, ..Ended::default() },
            Some("its round failed"),
        ),
        (
            "one that failed having said it left its item says why it failed",
            Ended {
                failed: true,
                left: true,
                leaves: "left".into(),
                reason: "cut".into(),
                ..Ended::default()
            },
            Some("cut"),
        ),
        // No round was begun: the item was never taken, which is said where the stage reports.
        ("an item that could not be taken", Ended { failed: true, untaken: true, ..Ended::default() }, None),
    ];
    for (what, asked, leaves) in cases {
        assert_eq!(left(&asked).as_deref(), leaves, "{what}");
    }
}

#[test]
fn a_turn_whose_program_was_refused_something_goes_on_as_its_agent_said_unless_its_stage_counts_it_as_blocked()
 {
    use deciding::work::{Worked, worked};
    let refused = Worked {
        item: "shop-7".into(),
        available: true,
        decision: "done".into(),
        by: "shop-worker-1".into(),
        reply: "I did what I could.".into(),
        refused: vec!["Bash".into(), "WebFetch".into()],
        ..Worked::default()
    };
    // Said nothing of, and as `note`: it goes on, and nothing is left.
    for means in ["", "note"] {
        let came = worked(&Worked { refused_means: means.into(), ..refused.clone() }).unwrap();
        assert_eq!((came.came, came.left_as.as_str()), ("on", ""), "{means}");
    }
    // As `blocked`: left for a person, as one whose agent said so.
    let blocked = worked(&Worked { refused_means: "blocked".into(), ..refused.clone() }).unwrap();
    assert_eq!((blocked.came, blocked.left_as.as_str()), ("left", "blocked"));
    assert_eq!(
        blocked.says,
        "shop-7: its program was refused Bash, WebFetch, and it is left for a person: I did what I could."
    );
    assert_eq!(blocked.why, "its program was refused Bash, WebFetch: I did what I could.");
    // Nothing refused, or the agent asked: as its agent said.
    let nothing =
        worked(&Worked { refused: vec![], refused_means: "blocked".into(), ..refused.clone() }).unwrap();
    assert_eq!(nothing.came, "on");
    let asked = Worked { decision: "ask".into(), refused_means: "blocked".into(), ..refused.clone() };
    assert_eq!(worked(&asked).unwrap().came, "asks");
    assert_eq!(
        worked(&Worked { refused_means: "maybe".into(), ..refused }).unwrap_err(),
        "`refused_means` is what a turn whose program was refused something comes to where its agent went on, \
         `note` or `blocked`; it was given `maybe`"
    );
}
