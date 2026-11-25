//! How a piece of work is finished, as whoever gave it says, and what an
//! agent's account of that comes to: the checklist it is told, what it
//! said of each step, and each ending, as a table of cases; and the same
//! asked through what giving an item to an agent came to (`worked`).

use std::collections::BTreeSet;

use deciding::finished::{Checked, Said, after, checked, cut, named, said, told};
use deciding::turn::{Worked, worked};

fn steps() -> Vec<String> {
    vec!["its tests pass".into(), "the docs it changes say what is new".into()]
}

#[test]
fn an_agent_is_told_how_its_work_is_finished_before_what_it_is_given_and_what_the_card_names_of_what_is_checked_later()
 {
    let later = vec!["the whole gate, run when the work lands".to_string()];
    let card = "Shop: the cart\nAdd a cart. The gate passes.";
    let said = told("shop-7", &steps(), &later, card);
    assert!(said.starts_with("Before you say shop-7 is done, see to each of these, in order:\n\n"), "{said}");
    assert!(said.contains("1. its tests pass\n2. the docs it changes say what is new\n"), "{said}");
    assert!(
        said.contains("never yours to wait on or to run: the whole gate, run when the work lands."),
        "{said}"
    );
    assert!(said.contains("What shop-7 says asks for the whole gate: that part is checked later"), "{said}");
    assert!(said.contains("`STEP <number>: not done: <why>`"), "{said}");
    // A card that names none of it is told nothing of that.
    let plain = told("shop-7", &steps(), &later, "Shop: the cart\nAdd a cart.");
    assert!(!plain.contains("asks for"), "{plain}");
    // Nothing is told where no step is said, whatever is checked later.
    assert_eq!(told("shop-7", &[], &later, card), "");
    // What a card names is what a thing is called, before its comma, by a
    // word of four letters or more.
    let later = ["the whole gate, run when the work lands", "a review by a person", "the CI run"];
    assert_eq!(named(&later, "the gate passes"), vec!["the whole gate"]);
    assert_eq!(named(&later, "a person looks"), vec!["a review by a person"]);
    assert_eq!(named(&later, "the ci run is green"), Vec::<&str>::new());
}

#[test]
fn what_an_agent_said_of_each_step_is_read_from_a_line_for_each_and_one_it_said_nothing_of_is_unsaid() {
    let cases: Vec<(&str, &str, Vec<Said>)> = vec![
        ("each said plainly", "I did it.\nSTEP 1: done\nSTEP 2: later", vec![Said::Done, Said::Later]),
        (
            "not done, with why",
            "STEP 1: done\nSTEP 2: not done: the guide is somebody else's",
            vec![Said::Done, Said::Not("the guide is somebody else's".into())],
        ),
        ("not done, with no why", "STEP 1: not done\nSTEP 2: Not Done.", vec![Said::Not(String::new()); 2]),
        (
            "in a list, and with weight",
            "- **STEP 1:** done\n* `STEP 2: later`",
            vec![Said::Done, Said::Later],
        ),
        ("one said nothing of", "STEP 2: done", vec![Said::Unsaid, Said::Done]),
        ("the last said of a step stands", "STEP 1: not done\nSTEP 1: done", vec![Said::Done, Said::Unsaid]),
        (
            "what is no step, a step there is not and a word that is none pass by",
            "STEP 3: done\nSTEP 0: done\nSTEPS 1: done\nSTEP 1: finished\nStep two: done\nSTEP 2 more: done",
            vec![Said::Unsaid, Said::Unsaid],
        ),
        ("nothing said", "", vec![Said::Unsaid, Said::Unsaid]),
    ];
    for (what, reply, expected) in cases {
        assert_eq!(said(reply, 2), expected, "{what}");
    }
}

#[test]
fn a_turn_said_to_be_done_goes_on_is_given_again_once_or_is_left_by_what_its_agent_said_of_each_step() {
    let on = || Checked { came: "on", ..Checked::default() };
    let cases: Vec<(&str, &str, i64, bool, Checked)> = vec![
        ("every step done", "STEP 1: done\nSTEP 2: done", 0, false, on()),
        ("one waits only on something checked later", "STEP 1: done\nSTEP 2: later", 0, false, on()),
        ("given again once already, and done now", "STEP 1: done\nSTEP 2: done", 1, false, on()),
        (
            "nearly done: given again once, told what is left",
            "STEP 1: done\nSTEP 2: not done: no time",
            0,
            false,
            Checked {
                came: "again",
                why: "not all of how it is finished is done; what is left: 2. the docs it changes say what is new \
                      (not done: no time)"
                    .into(),
                nearly: true,
                ..Checked::default()
            },
        ),
        (
            "a step it said nothing of is not done",
            "STEP 1: done",
            0,
            false,
            Checked {
                came: "again",
                why: "not all of how it is finished is done; what is left: 2. the docs it changes say what is new \
                      (it said nothing of it)"
                    .into(),
                nearly: true,
                ..Checked::default()
            },
        ),
        (
            "nearly done again after it was given again once: left, saying what is left",
            "STEP 1: done\nSTEP 2: not done",
            1,
            false,
            Checked {
                came: "left",
                says: "shop-7: shop-worker-1 was given it again once for what was left, and this is left still: 2. \
                       the docs it changes say what is new (not done), and it is left for a person"
                    .into(),
                why: "what is left of how it is finished: 2. the docs it changes say what is new (not done)".into(),
                left_as: "unfinished".into(),
                nearly: false,
            },
        ),
        (
            "none of it done: left",
            "STEP 1: not done: the tests hang\nSTEP 2: not done",
            0,
            false,
            Checked {
                came: "left",
                says: "shop-7: shop-worker-1 said none of how it is finished is done: 1. its tests pass (not done: \
                       the tests hang); 2. the docs it changes say what is new (not done), and it is left for a \
                       person"
                    .into(),
                why: "none of how it is finished is done: 1. its tests pass (not done: the tests hang); 2. the docs \
                      it changes say what is new (not done)"
                    .into(),
                left_as: "unfinished".into(),
                nearly: false,
            },
        ),
        (
            "its time ran out and asked it said what is done: nearly done, given again once",
            "STEP 1: done",
            0,
            true,
            Checked {
                came: "again",
                why: "its time ran out, and not all of how it is finished is done; what is left: 2. the docs it \
                      changes say what is new (it said nothing of it)"
                    .into(),
                nearly: true,
                ..Checked::default()
            },
        ),
        ("its time ran out and every step is done all the same", "STEP 1: done\nSTEP 2: later", 0, true, on()),
        (
            "its time ran out and asked then it said nothing: left",
            "  ",
            0,
            true,
            Checked {
                came: "left",
                says: "shop-7: the time shop-worker-1 was given ran out, and asked then it said nothing of what it \
                       had done, and it is left for a person"
                    .into(),
                why: "its time ran out, and it said nothing of what it had done".into(),
                left_as: "cut".into(),
                nearly: false,
            },
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, reply, given, was_cut, expected) in cases {
        let came = checked("shop-7", "shop-worker-1", &steps(), reply, given, was_cut);
        assert_eq!(came.as_ref(), Some(&expected), "{what}");
        reached.insert((expected.came, expected.left_as.clone()));
    }
    let every: BTreeSet<(&str, String)> = [
        ("on", String::new()),
        ("again", String::new()),
        ("left", "unfinished".to_string()),
        ("left", "cut".to_string()),
    ]
    .into_iter()
    .collect();
    assert_eq!(reached, every, "every ending has a case");
    // Where no step is said there is nothing to check.
    assert_eq!(checked("shop-7", "", &[], "STEP 1: not done", 0, false), None);
}

#[test]
fn a_turn_whose_time_ran_out_with_no_step_said_and_no_word_chosen_is_given_again_once_with_what_it_said_is_left()
 {
    assert_eq!(
        cut("shop-7", "shop-worker-1", "The cart is half made.", 0),
        Checked {
            came: "again",
            why: "its time ran out before it was done; what it said is left: The cart is half made.".into(),
            nearly: true,
            ..Checked::default()
        }
    );
    let twice = cut("shop-7", "shop-worker-1", "Still the cart.", 1);
    assert_eq!((twice.came, twice.left_as.as_str()), ("left", "unfinished"));
    assert_eq!(
        twice.says,
        "shop-7: the time shop-worker-1 was given ran out again after it was given it again once: Still the cart., \
         and it is left for a person"
    );
    let silent = cut("shop-7", "", "", 0);
    assert_eq!((silent.came, silent.left_as.as_str()), ("left", "cut"));
    assert!(silent.says.contains("the time its agent was given ran out"), "{}", silent.says);
}

#[test]
fn what_giving_an_item_to_an_agent_came_to_holds_a_turn_that_came_to_on_to_how_it_is_finished() {
    let given = |decision: &str, reply: &str| Worked {
        item: "shop-7".into(),
        available: true,
        decision: decision.into(),
        by: "shop-worker-1".into(),
        reply: reply.into(),
        finish: steps(),
        ..Worked::default()
    };
    let came = |asked: Worked| {
        let came = worked(&asked).unwrap();
        (came.came, came.left_as, came.nearly)
    };
    assert_eq!(came(given("done", "STEP 1: done\nSTEP 2: later")), ("on", String::new(), false));
    assert_eq!(came(given("done", "STEP 1: done")), ("again", String::new(), true));
    assert_eq!(
        came(Worked { given: 1, ..given("done", "STEP 1: done") }),
        ("left", "unfinished".into(), false)
    );
    // A word that is not the work done is what it always came to: a
    // question is asked, whatever is said of the steps.
    assert_eq!(came(given("ask", "Which total?")), ("asks", String::new(), false));
    // Where no step is said, a turn is as it always was.
    assert_eq!(came(Worked { finish: Vec::new(), ..given("done", "") }), ("on", String::new(), false));
    // Several who look at once are not held to it.
    let council = Worked { seats: 2, decided: "passed".into(), ..given("", "") };
    assert_eq!(came(council), ("on", String::new(), false));
    // Its time ran out and, asked then, it chose no word: with no step
    // said, what it said is what is left; with steps, each is read.
    let cut_off = |reply: &str| Worked { cut: true, finish: Vec::new(), ..given("", reply) };
    assert_eq!(came(cut_off("Half done.")), ("again", String::new(), true));
    assert_eq!(came(cut_off("")), ("left", "cut".into(), false));
    assert_eq!(
        came(Worked { cut: true, ..given("", "STEP 1: done\nSTEP 2: done") }),
        ("on", String::new(), false)
    );
    // And one that chose to ask when it was asked what it had done asks.
    assert_eq!(came(Worked { cut: true, ..given("ask", "Which total?") }), ("asks", String::new(), false));
}

#[test]
fn an_agent_whose_time_ran_out_is_asked_to_do_no_more_and_say_what_is_done_and_what_is_left() {
    let checklist = told("shop-7", &steps(), &[], "");
    let asked = after("shop-7", &checklist);
    assert!(
        asked.starts_with(
            "The time you were given for shop-7 ran out, and your program was stopped. Do no more"
        )
    );
    assert!(asked.contains("as you were told:\n\nBefore you say shop-7 is done"), "{asked}");
    assert_eq!(
        after("shop-7", ""),
        "The time you were given for shop-7 ran out, and your program was stopped. Do no more work on it now. Say \
         briefly what of it you have done and what is left."
    );
}
