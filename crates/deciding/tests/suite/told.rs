//! Whom something that waits for a person is told to (`deciding::told`,
//! asked by `std/told` as `tells.whom`): a table of the rows that say
//! who is told what, what happened and what was said of it, and whom
//! that comes to with the sentence.

use deciding::told::{Of, whom};
use serde_json::{Map, Value, json};

/// A command of more than one line, longer than a sentence says.
const SLOW_CUT: &str = "shop-4: `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx…` has been running for 1m30s, and goes on.";

fn said(carried: Value) -> Map<String, Value> {
    carried.as_object().cloned().unwrap_or_default()
}

#[test]
fn a_thing_is_told_to_whoever_the_rows_say_is_told_of_it_in_a_sentence_of_its_own() {
    // The usual row, and one that names somebody for some things only.
    let usual = json!([{ "title": "applier", "whom": "applier", "of": [] }]);
    let two = json!([
        { "title": "applier", "whom": "applier", "of": [] },
        { "title": "lead", "whom": "@grace", "of": ["left", "failed"] },
    ]);
    let asked =
        json!({ "card": "shop-4", "project": "shop", "asker": "shop-worker-1", "question": "With tax?" });
    // The rows, which thing, what was said of it and by which instance;
    // and whom it is told to, the sentence, and what it is about.
    type Case = (
        (Value, &'static str, Value, &'static str),
        (Of, &'static [&'static str], &'static str, &'static str),
    );
    let cases: Vec<Case> = vec![
        // Something asked: a card by its agent, a feature by whoever plans it, with and without words.
        (
            (usual.clone(), "asked", asked.clone(), "in_go"),
            (Of::Asked, &["applier"], "shop-4: shop-worker-1 asks: With tax?", "shop-4"),
        ),
        (
            (two.clone(), "asked", asked.clone(), "in_go"),
            (Of::Asked, &["applier"], "shop-4: shop-worker-1 asks: With tax?", "shop-4"),
        ),
        (
            (
                usual.clone(),
                "asked",
                json!({ "record": "shop-features-2", "agent": "shop-architect-1", "question": "Which shop?" }),
                "",
            ),
            (
                Of::Asked,
                &["applier"],
                "shop-features-2: shop-architect-1 asks: Which shop?",
                "shop-features-2",
            ),
        ),
        (
            (usual.clone(), "asked", json!({ "card": "shop-4", "question": "With tax?" }), ""),
            (Of::Asked, &["applier"], "shop-4 asks: With tax?", "shop-4"),
        ),
        (
            (
                usual.clone(),
                "asked",
                json!({ "card": "shop-4", "asker": "shop-worker-1", "question": " " }),
                "",
            ),
            (
                Of::Asked,
                &["applier"],
                "shop-4: shop-worker-1 asked something, and waits for an answer.",
                "shop-4",
            ),
        ),
        (
            (usual.clone(), "asked", json!({ "card": "shop-4" }), ""),
            (Of::Asked, &["applier"], "shop-4: something was asked, and waits for an answer.", "shop-4"),
        ),
        // A card left: sent back as often as it may be, or for a reason that was said.
        (
            (two.clone(), "left", json!({ "card": "shop-4", "project": "shop", "rounds": 2 }), "in_review"),
            (
                Of::Left,
                &["applier", "@grace"],
                "shop-4 was sent back 2 times, which is as often as it may be: it is left for a person.",
                "shop-4",
            ),
        ),
        (
            (usual.clone(), "left", json!({ "card": "shop-4", "rounds": 1 }), ""),
            (
                Of::Left,
                &["applier"],
                "shop-4 was sent back 1 time, which is as often as it may be: it is left for a person.",
                "shop-4",
            ),
        ),
        (
            (usual.clone(), "left", json!({ "card": "shop-4", "rounds": 0 }), ""),
            (Of::Left, &["applier"], "shop-4 is left for a person.", "shop-4"),
        ),
        (
            (
                usual.clone(),
                "left",
                json!({ "card": "shop-4", "rounds": 2, "why": "its work could not be kept" }),
                "",
            ),
            (Of::Left, &["applier"], "shop-4 is left for a person: its work could not be kept", "shop-4"),
        ),
        // A card its agent could not do what it needed for: told as that,
        // and to a row that tells of a card left too.
        (
            (
                two.clone(),
                "left",
                json!({ "card": "shop-4", "as": "blocked", "why": "I was not let run ./check" }),
                "",
            ),
            (
                Of::Blocked,
                &["applier", "@grace"],
                "shop-4: its agent could not do what it needed, and it is left for a person: I was not let run ./check",
                "shop-4",
            ),
        ),
        (
            (usual.clone(), "left", json!({ "card": "shop-4", "as": "blocked" }), ""),
            (
                Of::Blocked,
                &["applier"],
                "shop-4: its agent could not do what it needed, and it is left for a person.",
                "shop-4",
            ),
        ),
        // Left for another reason, it is told as a card left.
        (
            (
                usual.clone(),
                "left",
                json!({ "card": "shop-4", "as": "nobody", "why": "nobody was there" }),
                "",
            ),
            (Of::Left, &["applier"], "shop-4 is left for a person: nobody was there", "shop-4"),
        ),
        // A card that waits for a person to review it, and a row that is
        // told of something else is not told of it.
        (
            (
                two.clone(),
                "review",
                json!({ "card": "shop-4", "why": "shop-4 is in review, where nobody in the fleet has the role reviewer: it waits for a person to review it." }),
                "",
            ),
            (
                Of::Review,
                &["applier"],
                "shop-4 is in review, where nobody in the fleet has the role reviewer: it waits for a person to review it.",
                "shop-4",
            ),
        ),
        (
            (usual.clone(), "review", json!({ "card": "shop-4" }), ""),
            (Of::Review, &["applier"], "shop-4 waits for a person to review it.", "shop-4"),
        ),
        // A card that waits for somebody to press what its column offers.
        (
            (
                usual.clone(),
                "held",
                json!({ "card": "shop-4", "says": "shop-4 waits in ready for Signed, which a person presses." }),
                "",
            ),
            (Of::Held, &["applier"], "shop-4 waits in ready for Signed, which a person presses.", "shop-4"),
        ),
        (
            (two.clone(), "held", json!({ "card": "shop-4" }), ""),
            (Of::Held, &["applier"], "shop-4 waits for somebody to press what its column offers.", "shop-4"),
        ),
        // A plan that waits.
        (
            (
                usual.clone(),
                "plan",
                json!({ "record": "shop-features-2", "project": "shop", "agent": "shop-architect-1" }),
                "",
            ),
            (
                Of::Plan,
                &["applier"],
                "shop-features-2: shop-architect-1 has said how it would be done, and the plan waits. Move it on to accept it, or say on it what should be otherwise.",
                "shop-features-2",
            ),
        ),
        (
            (usual.clone(), "plan", json!({ "card": "shop-9" }), ""),
            (
                Of::Plan,
                &["applier"],
                "shop-9 has a plan, which waits. Move it on to accept it, or say on it what should be otherwise.",
                "shop-9",
            ),
        ),
        // Something failed: about what failed, by its reason alone where it says one.
        (
            (
                two.clone(),
                "failed",
                json!({ "error": "step `a`: step `b`: no such file", "reason": "no such file" }),
                "in_round",
            ),
            (
                Of::Failed,
                &["applier", "@grace"],
                "Something in the fleet for shop failed: no such file",
                "in_round",
            ),
        ),
        (
            (usual.clone(), "failed", json!({ "error": "no such file", "flow": "on.go" }), "in_stage"),
            (Of::Failed, &["applier"], "Something in the fleet for shop failed: no such file", "in_stage"),
        ),
        (
            (usual.clone(), "failed", json!({}), ""),
            (Of::Failed, &["applier"], "Something in the fleet for shop failed.", ""),
        ),
        // A command run past its time: which, how long so far to the
        // second, and how long it may run before anybody is told; its
        // first line alone, cut where it is long; and with less said.
        (
            (
                two.clone(),
                "slow",
                json!({ "card": "shop-4", "command": "cargo xtask check", "after": "20m", "running_ms": 1_260_400 }),
                "in_run",
            ),
            (
                Of::Slow,
                &["applier"],
                "shop-4: `cargo xtask check` has been running for 21m, past the 20m it may run before somebody is told, and goes on.",
                "shop-4",
            ),
        ),
        (
            (
                usual.clone(),
                "slow",
                json!({ "card": "shop-4", "command": format!("{}\nand more", "x".repeat(130)), "running_ms": 90_000 }),
                "",
            ),
            (Of::Slow, &["applier"], SLOW_CUT, "shop-4"),
        ),
        (
            (usual.clone(), "slow", json!({ "card": "shop-4" }), ""),
            (Of::Slow, &["applier"], "shop-4: its command is still running, and goes on.", "shop-4"),
        ),
        // Nobody: no row, a row that says nobody, a row of other things.
        (
            (json!([]), "asked", asked.clone(), ""),
            (Of::Asked, &[], "shop-4: shop-worker-1 asks: With tax?", "shop-4"),
        ),
        (
            (
                json!([{ "title": "applier", "whom": "", "of": [] }, { "title": "nobody", "whom": "  " }, { "title": "unsaid" }]),
                "asked",
                asked.clone(),
                "",
            ),
            (Of::Asked, &[], "shop-4: shop-worker-1 asks: With tax?", "shop-4"),
        ),
        (
            (
                json!([{ "title": "lead", "whom": "@grace", "of": ["left", "failed"] }]),
                "plan",
                json!({ "record": "shop-features-2" }),
                "",
            ),
            (
                Of::Plan,
                &[],
                "shop-features-2 has a plan, which waits. Move it on to accept it, or say on it what should be otherwise.",
                "shop-features-2",
            ),
        ),
        // Somebody two rows say is told once; a word of `of` is taken
        // whatever its case and the room about it, and one written as
        // text is the words between its commas.
        (
            (
                json!([
                    { "whom": "@grace", "of": [" Asked "] },
                    { "whom": " @grace ", "of": [] },
                    { "whom": "user:acct_1", "of": "left, asked" },
                    { "whom": "agent:scout", "of": ["", " "] },
                    { "whom": "@nell", "of": ["plans"] },
                ]),
                "asked",
                asked.clone(),
                "",
            ),
            (
                Of::Asked,
                &["@grace", "user:acct_1", "agent:scout"],
                "shop-4: shop-worker-1 asks: With tax?",
                "shop-4",
            ),
        ),
    ];
    let mut reached = Vec::new();
    for ((rows, of, carried, from), (word, to, says, about)) in cases {
        let telling = whom(rows.as_array().unwrap(), of, &said(carried.clone()), from, "shop").unwrap();
        assert_eq!(
            (
                telling.of,
                telling.to.iter().map(String::as_str).collect::<Vec<_>>(),
                telling.says.as_str(),
                telling.about.as_str()
            ),
            (word, to.to_vec(), says, about),
            "{rows} {of} {carried}"
        );
        reached.push(telling.of);
    }
    for word in Of::ALL {
        assert!(reached.contains(word), "no case comes to `{}`", word.word());
    }
    // In no project, it says of no project.
    let nowhere = whom(&[], "failed", &said(json!({ "reason": "no such file" })), "in_round", "").unwrap();
    assert_eq!(nowhere.says, "Something in the fleet failed: no such file");
}

#[test]
fn what_makes_no_sense_is_refused_in_a_sentence() {
    let usual = [json!({ "whom": "applier" })];
    let refused =
        |rows: &[Value], of: &str, carried: Value| whom(rows, of, &said(carried), "", "shop").unwrap_err();
    assert_eq!(
        refused(&usual, "idle", json!({ "card": "shop-4" })),
        "`of` is which thing somebody is told of, `asked`, `left`, `blocked`, `review`, `held`, `plan`, `failed` or `slow`; it was given `idle`"
    );
    assert_eq!(
        refused(&[json!("applier")], "asked", json!({ "card": "shop-4" })),
        "`rows` is the rows that say who is told what, each a row; one of them is \"applier\""
    );
    // Something asked, left or planned is a card or a record.
    for of in ["asked", "left", "blocked", "review", "held", "plan", "slow"] {
        assert_eq!(
            refused(&usual, of, json!({ "question": "With tax?" })),
            format!("what was said names nothing that is `{of}`: it says neither `card` nor `record`")
        );
    }
}

#[test]
fn a_command_that_runs_long_is_one_telling_for_its_cards_coming_whichever_run_tells_it() {
    let usual = [json!({ "whom": "applier" })];
    let once = |carried: Value, of: &str| whom(&usual, of, &said(carried), "in_run", "shop").unwrap().once;
    let slow = json!({ "card": "shop-4", "command": "make test", "after": "20m", "asking": "812" });
    // By the card, the coming it was given at and the command: the same
    // three, said again by a command begun again after a stop, are the
    // same telling; another coming or another command is another.
    assert_eq!(once(slow.clone(), "slow"), "slow shop-4 812 make test");
    let mut later = slow.clone();
    later["asking"] = json!("907");
    assert_ne!(once(later, "slow"), once(slow.clone(), "slow"));
    let mut other = slow.clone();
    other["command"] = json!("make lint");
    assert_ne!(once(other, "slow"), once(slow.clone(), "slow"));
    // With no coming said, each run tells as the step it is: and nothing
    // else is told once.
    let mut unsaid = slow.clone();
    unsaid["asking"] = json!("");
    assert_eq!(once(unsaid, "slow"), "");
    assert_eq!(once(slow, "left"), "");
}

#[test]
fn a_question_is_said_in_its_own_words_and_one_asked_of_nobody_says_so_where_it_was_asked_and_where_the_fleet_reports()
 {
    let usual = [json!({ "whom": "applier" })];
    let asked = |carried: Value, of: &str| whom(&usual, of, &said(carried), "in_go", "shop").unwrap();
    let cli = brand::CLI;
    let how = format!(
        "no row of the project's `tells` names anybody to tell of a question. Answer it in its Comments; and say who is asked from now on with `{cli} record add shop tells title=lead whom=@<somebody> of=asked`."
    );
    // The question as it was asked; what is said of it where nobody is asked,
    // on the card and, naming the card and who asked, where the fleet reports.
    let one =
        asked(json!({ "card": "shop-4", "asker": "shop-worker-1", "question": " With tax? " }), "asked");
    assert_eq!(one.asks, "With tax?");
    assert_eq!(one.unasked, format!("Nobody is asked this: {how}"));
    assert_eq!(one.untold, format!("shop-4: shop-worker-1 asked something, and nobody is asked it: {how}"));
    // Asked with no words and by nobody named: it still says something.
    let bare = asked(json!({ "card": "shop-4" }), "asked");
    assert_eq!(bare.asks, "Something was asked, and waits for an answer.");
    assert_eq!(bare.untold, format!("shop-4: something was asked, and nobody is asked it: {how}"));
    // Anything else says none of it.
    let left = asked(json!({ "card": "shop-4", "why": "no" }), "left");
    assert_eq!((left.asks.as_str(), left.unasked.as_str(), left.untold.as_str()), ("", "", ""));
}
