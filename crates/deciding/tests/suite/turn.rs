//! What an agent's turn came to, by the word it ended with: a function of
//! the words it may end with (`ends`) and of its answer, held by a table
//! of cases that reaches every word it can come to.

use std::collections::BTreeSet;

use deciding::turn::{ASKED_NOTHING, Answer, CAME_WORDS, Came, Ends, came};

fn ends(said: &[&str]) -> Ends {
    Ends::read(&said.iter().map(|one| one.to_string()).collect::<Vec<_>>()).unwrap()
}

fn answer(decision: &str, reply: &str) -> Answer {
    Answer {
        item: "shop-7".into(),
        decision: decision.into(),
        by: "shop-worker-1".into(),
        reply: reply.into(),
    }
}

#[test]
fn what_a_turn_came_to_is_what_the_word_its_agent_ended_with_comes_to() {
    let work = Ends::usual();
    let plan = ends(&["propose=on", "ask=asks", "leave=left"]);
    let more = ends(&["done=on blocked=left", "again=again", "redo=back"]);
    let words = |came: &'static str, chose: &str| Came { came, chose: chose.into(), ..Came::default() };
    let cases: Vec<(&str, &Ends, Answer, Came)> = vec![
        ("the work done", &work, answer("done", "I added the cart."), words("on", "done")),
        ("an agent that chose neither is taken to have chosen the first", &work, answer("", "Done."), words("on", "done")),
        ("one that chose a word it was not offered is taken the same way", &work, answer("finished", ""), words("on", "done")),
        (
            "its agent asked in place of working",
            &work,
            answer("ask", "Which total?"),
            Came { asker: "shop-worker-1".into(), question: "Which total?".into(), ..words("asks", "ask") },
        ),
        (
            "and one that asked and said nothing has asked what is wanted",
            &work,
            answer("ask", "  "),
            Came { asker: "shop-worker-1".into(), question: ASKED_NOTHING.into(), ..words("asks", "ask") },
        ),
        ("a plan proposed", &plan, answer("propose", "First the model."), words("on", "propose")),
        (
            "a card its planner left as it is",
            &plan,
            answer("leave", "There is nothing to plan."),
            Came {
                says: "shop-7: shop-worker-1 chose `leave`, and it is left for a person: There is nothing to plan.".into(),
                why: "There is nothing to plan.".into(),
                ..words("left", "leave")
            },
        ),
        (
            "a third word that leaves it, said with nothing",
            &more,
            answer("blocked", ""),
            Came {
                says: "shop-7: shop-worker-1 chose `blocked`, and it is left for a person".into(),
                why: "shop-worker-1 chose `blocked` and said nothing more".into(),
                ..words("left", "blocked")
            },
        ),
        (
            "a word that sends it back, with what was said",
            &more,
            answer("redo", "The total is wrong."),
            Came { says: "The total is wrong.".into(), ..words("back", "redo") },
        ),
        (
            "and with nothing said",
            &more,
            Answer { by: String::new(), ..answer("redo", "") },
            Came { says: "its agent chose `redo` and said nothing more".into(), ..words("back", "redo") },
        ),
        (
            "a word that has it done once more, with why",
            &more,
            answer("again", "The tests did not run."),
            Came { why: "The tests did not run.".into(), ..words("again", "again") },
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, ends, answer, expected) in cases {
        let came = came(&answer, ends).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(came, expected, "{what}");
        reached.insert(came.came);
    }
    assert_eq!(reached, CAME_WORDS.iter().copied().collect(), "every word it can come to has a case");
    assert_eq!(
        came(&Answer::default(), &Ends::usual()).unwrap_err(),
        "the `item` the turn was on is not said"
    );
}

#[test]
fn the_words_a_turn_may_end_with_are_offered_in_the_order_they_are_said_and_are_refused_written_wrongly() {
    assert_eq!(Ends::usual().words(), ["done", "ask"]);
    assert_eq!(ends(&["propose=on, ask=asks", "leave=left"]).words(), ["propose", "ask", "leave"]);
    let refused =
        |said: &[&str]| Ends::read(&said.iter().map(|one| one.to_string()).collect::<Vec<_>>()).unwrap_err();
    assert_eq!(
        refused(&["done"]),
        "`done` in `ends` says no word and what it comes to: each is `word=what it comes to`, as `done=on`"
    );
    assert_eq!(
        refused(&["done=on", "stuck=stuck"]),
        "`stuck=stuck` in `ends` comes to `stuck`, which is none of the words a turn comes to: `on`, `back`, \
         `again`, `asks`, `left`"
    );
    assert_eq!(refused(&["done=on", "done=back"]), "`ends` says what `done` comes to twice");
    assert_eq!(refused(&[" ", ""]), "`ends` says no word the turn may end with, as `done=on`");
    assert_eq!(
        refused(&["=on"]),
        "`=on` in `ends` says no word and what it comes to: each is `word=what it comes to`, as `done=on`"
    );
}

#[test]
fn what_several_who_look_at_once_came_to_together_is_what_ends_says_of_it_and_the_usual_where_it_says_nothing()
 {
    let council = |said: &[&str]| Ends::council(&said.iter().map(|one| one.to_string()).collect::<Vec<_>>());
    let usual = council(&[]).unwrap();
    assert!(usual.of_a_council() && !Ends::usual().of_a_council());
    assert_eq!(usual.words(), ["passed", "failed", "undecided"]);
    // What a row says, with the usual for what it does not.
    let said = council(&["passed=on failed=again"]).unwrap();
    assert_eq!(said.words(), ["passed", "failed", "undecided"]);
    let looked = |decided: &str, words: &str| Answer {
        item: "shop-7".into(),
        decision: decided.into(),
        by: String::new(),
        reply: words.into(),
    };
    let words = |came: &'static str, chose: &str| Came { came, chose: chose.into(), ..Came::default() };
    let cases: Vec<(&str, &Ends, Answer, Came)> = vec![
        ("enough approved", &usual, looked("passed", "shop-reviewer-1: Good."), words("on", "passed")),
        (
            "enough no longer can, and it goes back with what those against it said",
            &usual,
            looked("failed", "shop-reviewer-2: The total is wrong."),
            Came { says: "shop-reviewer-2: The total is wrong.".into(), ..words("back", "failed") },
        ),
        (
            "and with nothing said, saying that they came to it",
            &usual,
            looked("failed", ""),
            Came {
                says: "those who looked came to `failed` and said nothing more".into(),
                ..words("back", "failed")
            },
        ),
        (
            "neither, which leaves it for a person",
            &usual,
            looked("undecided", ""),
            Came {
                says: "shop-7: those who looked came to `undecided`, and it is left for a person".into(),
                why: "those who looked came to `undecided` and said nothing more".into(),
                ..words("left", "undecided")
            },
        ),
        (
            "what a row says of one",
            &said,
            looked("failed", "Not yet."),
            Came { why: "Not yet.".into(), ..words("again", "failed") },
        ),
    ];
    for (what, ends, answer, expected) in cases {
        assert_eq!(came(&answer, ends).unwrap(), expected, "{what}");
    }
    assert_eq!(
        council(&["approve=on"]).unwrap_err(),
        "`ends` says what `approve` comes to, and several who look at once (`seats`) come to one of `passed`, \
         `failed` or `undecided` together: say what each of those comes to, as `passed=on failed=back`, and the \
         two words they are counted by with `yes` and `no`"
    );
    assert_eq!(council(&["passed"]).unwrap_err(), Ends::read(&["passed".to_string()]).unwrap_err());
}
