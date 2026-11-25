//! What an agent is told by the texts that ship, held to what it was told
//! when the words were typed out in the texts themselves: each text's own
//! parts, and the files it links, put together for what it is handed.

use blueprint::{Blueprint, Step};
use conductor::Library;
use conductor::words::{Files, linked_as, links, put};
use serde_json::{Map, Value, json};

/// The step called `step` of the text that ships as `name`, wherever in
/// the text it is.
fn step_of(name: &str, step: &str) -> (Blueprint, Step) {
    let source = conductor::standard().load(name, None).unwrap_or_else(|| panic!("`{name}` ships"));
    let checked = blueprint::check(&source, &blueprint::NoCatalog);
    let blueprint = checked.blueprint.unwrap_or_else(|| panic!("`{name}` reads"));
    let found =
        blueprint.flows().flat_map(|(_, flow)| flow.steps.iter()).find(|found| found.id == step).cloned();
    (blueprint.clone(), found.unwrap_or_else(|| panic!("`{name}` has a step `{step}`")))
}

/// What the step `step` of `name` tells its agent: `within` is
/// everything an expression of the step can name.
fn told_within(name: &str, step: &str, within: Value) -> String {
    let (blueprint, step) = step_of(name, step);
    let scope: Map<String, Value> = within.as_object().cloned().unwrap_or_default();
    let with = blueprint::template::render(&step.with, &scope).unwrap();
    let mut files = Files::new();
    for from in links(&with["parts"]).unwrap() {
        let linked = linked_as(&blueprint.name, &from);
        let text = conductor::standard().load(&linked, None).unwrap_or_else(|| panic!("`{linked}` ships"));
        files.insert(from, text);
    }
    put(&with["parts"], None, &files).unwrap().text
}

const DECIDE: &str = "Decide whether it does what the card asks, and whether anything is wrong with it. If it is \
                      right, choose `approve` and say in a sentence why. If something is wrong, choose `send_back` \
                      and say exactly what, for whoever worked on it: they are given the card again with your \
                      words. Change nothing yourself.\n";

#[test]
fn a_reviewer_is_told_what_it_was_told_before_its_words_were_a_file() {
    // What a review hands what it asks: the card, what its seats are
    // shown of what was changed and where, and how many seats there are.
    let with = |more: Value| {
        let said = |key: &str, or: Value| more.get(key).cloned().unwrap_or(or);
        json!({
            "params": { "item": "shop-12", "seats": said("among", json!(1)) },
            "steps": {
                "card": { "out": { "title": "A cart" } },
                "asks": { "out": { "text": said("body", json!("With tax.")) } },
                "shown": { "out": { "changed": said("changed", json!("")), "dir": said("dir", json!("")) } },
            },
        })
    };
    // Alone, with nothing to show of what was changed, in the project itself.
    assert_eq!(
        told_within("std/review", "asked", with(json!({ "changed": "", "dir": "", "among": 1 }))),
        format!(
            "Card shop-12: A cart\n\nWith tax.\n\n\n\nThis card has been worked on and is in review. It was worked \
             in the project itself: look at what is there. {DECIDE}"
        )
    );
    // One of three, shown what was changed, in the card's own worktree.
    let all = json!({ "changed": "diff --git a/cart b/cart", "dir": "/work/shop-12", "among": 3 });
    assert_eq!(
        told_within("std/review", "asked", with(all)),
        format!(
            "Card shop-12: A cart\n\nWith tax.\n\nWhat was changed, against the project's own branch:\n\ndiff --git \
             a/cart b/cart\n\nThis card has been worked on and is in review. You are in the worktree it was worked \
             in, on its own branch, lane/shop-12: `git log` and `git diff` against the branch it was cut from show \
             what was done. You are one of 3 looking at it, each for yourself: say what you find, and not what you \
             expect the others to. {DECIDE}"
        )
    );
    // Each of the three that is only sometimes said, by itself.
    let alone = told_within(
        "std/review",
        "asked",
        with(json!({ "changed": "", "dir": "/work/shop-12", "among": 1 })),
    );
    assert!(
        alone.contains("is in review. You are in the worktree") && alone.contains("was done. Decide"),
        "{alone}"
    );
    let two = told_within("std/review", "asked", with(json!({ "changed": "", "dir": "", "among": 2 })));
    assert!(
        two.contains("what is there. You are one of 2 looking at it") && two.contains("to. Decide"),
        "{two}"
    );
    // What a card says is said as it is, holes and all: it is no part of the file.
    let odd = with(json!({ "body": "Charge ${price} and $${tax}.", "changed": "", "dir": "", "among": 1 }));
    assert!(told_within("std/review", "asked", odd).contains("\n\nCharge ${price} and $${tax}.\n\n"));
}

#[test]
fn what_a_review_hands_its_seats_of_the_card_is_what_it_handed_them_before() {
    let of = |card: Value| told_within("std/review", "asks", json!({ "steps": { "card": { "out": card } } }));
    // What the card asks, and how it was to be done where that was planned.
    assert_eq!(
        of(json!({ "body": "With tax.", "more": { "plan": "A column for it." } })),
        "With tax.\n\nHow it was to be done, as was said when it was planned:\n\nA column for it.\n"
    );
    // Never planned, with a plan emptied, and with nothing more kept of it at all.
    for card in [
        json!({ "body": "With tax.", "more": {} }),
        json!({ "body": "With tax.", "more": { "plan": "" } }),
        json!({ "body": "With tax." }),
    ] {
        assert_eq!(of(card.clone()), "With tax.\n\n\n\n\n", "{card}");
    }
}

const CHOOSE_CARD: &str = "End by choosing: `propose`, and what you wrote is kept on the card as how it is to be done; \
                           `ask`, and what you wrote is put on the card as a question, which waits for an answer; or \
                           `leave`, and nothing is changed.\n";

#[test]
fn whoever_plans_a_card_is_told_what_it_was_told_before_its_words_were_a_file() {
    let asked = |params: Value, card: Value| {
        told_within(
            "std/proposal-round",
            "asks",
            json!({ "params": params, "steps": { "card": { "out": card } } }),
        )
    };
    let card = json!({ "title": "A cart", "body": "With tax.", "more": {} });
    let params =
        |why: &str, note: &str| json!({ "item": "shop-12", "why": why, "note": note, "told": "Told." });
    // New to it, with nothing said and nothing yet planned.
    assert_eq!(
        asked(params("new", ""), card.clone()),
        format!(
            "This card is yours to plan.\n\n\n\nCard shop-12: A cart\n\nWith tax.\n\n\n\n\n\nTold.\n\n{CHOOSE_CARD}"
        )
    );
    // Sent back with something said, and with what it last said of it.
    let planned = json!({ "title": "A cart", "body": "With tax.", "more": { "plan": "A column." } });
    assert_eq!(
        asked(params("back", "Use a table."), planned),
        format!(
            "This card was sent back to you, with this said:\n\nUse a table.\n\nCard shop-12: A cart\n\nWith \
             tax.\n\nWhat you last said of how it would be done:\n\nA column.\n\nTold.\n\n{CHOOSE_CARD}"
        )
    );
    // What brought it, each way there is.
    for (why, note, begins) in [
        (
            "changed",
            "",
            "What this card asks was changed. Read it again; if what you said of it still holds, leave it.\n\n\n\nCard",
        ),
        ("back", "", "This card was sent back to you.\n\n\n\nCard"),
        // Come to where it is planned: new with nothing said, sent back with something.
        ("came", "", "This card is yours to plan.\n\n\n\nCard"),
        ("came", "Use a table.", "This card was sent back to you, with this said:\n\nUse a table.\n\nCard"),
        ("said", "Why a cart?", "Somebody said this on the card:\n\nWhy a cart?\n\nCard"),
        ("anything else", "x", "Somebody said this on the card:\n\nx\n\nCard"),
    ] {
        let told = asked(params(why, note), card.clone());
        assert!(told.starts_with(begins), "`{why}`: {told}");
    }
}

const CHOOSE_FEATURE: &str = "End by choosing: `propose`, and what you wrote is kept as the feature's plan, to be \
                              accepted; `ask`, and what you wrote is put on the feature as a question, which waits \
                              for an answer; or `leave`, and nothing is changed.\n";

#[test]
fn whoever_plans_a_feature_is_told_what_it_was_told_before_its_words_were_a_file() {
    let asked_of = |params: Value, feature: Value, named: Value| {
        let within = json!({ "params": params, "steps": { "now": { "out": { "data": feature } }, "named": { "out": named } } });
        told_within("std/feature-round", "asks", within)
    };
    // A feature that gathers nothing is told what it always was.
    let asked = |params: Value, feature: Value| asked_of(params, feature, json!({ "count": 0, "shown": "" }));
    let params =
        |why: &str, note: &str| json!({ "id": "shop-f-1", "why": why, "note": note, "told": "Told." });
    assert_eq!(
        asked(params("new", ""), json!({ "title": "Checkout" })),
        format!(
            "This feature is yours to plan.\n\n\n\nFeature shop-f-1: Checkout\n\n\n\n\n\n\n\nTold.\n\n{CHOOSE_FEATURE}"
        )
    );
    let planned = json!({ "title": "Checkout", "body": "Pay by card.", "plan": "Three pieces." });
    assert_eq!(
        asked(params("said", "And by cash?"), planned.clone()),
        format!(
            "Somebody said this on the feature:\n\nAnd by cash?\n\nFeature shop-f-1: Checkout\n\nPay by \
             card.\n\nWhat you last said of how it would be built:\n\nThree pieces.\n\nTold.\n\n{CHOOSE_FEATURE}"
        )
    );
    for (why, begins) in [
        (
            "changed",
            "What this feature asks was changed. Read it again; if what you said of it still holds, leave it.\n\n",
        ),
        ("back", "This feature was sent back to be planned again.\n\n"),
    ] {
        let told = asked(params(why, ""), json!({ "title": "Checkout" }));
        assert!(told.starts_with(begins), "`{why}`: {told}");
    }
    // One that gathers cards on the board already is shown them, each
    // as it stands, between what it asks and what was last said of it.
    let shown = "shop-3, in ready: The till\nTakes money.\n\nshop-4, in backlog: The shelves";
    assert_eq!(
        asked_of(params("new", ""), planned, json!({ "count": 2, "shown": shown })),
        format!(
            "This feature is yours to plan.\n\n\n\nFeature shop-f-1: Checkout\n\nPay by card.\n\nIt gathers these \
             cards, which are on the board already. Put them in order with whatever else is needed. In your \
             tickets, one that is a card there already says `\"card\"` and its id in place of a title, and says \
             what it waits on as any ticket does; a card you leave out of your tickets is taken as it \
             stands.\n\n{shown}\n\nWhat you last said of how it would be built:\n\nThree pieces.\n\nTold.\n\n{CHOOSE_FEATURE}"
        )
    );
}

/// What a card's worker was told when the words were typed out in the
/// round itself, as they were written there: one text with a choice in
/// each hole.
const WORKER_BEFORE: &str = r#"${ !params.back ? (params.note != '' ? 'Please take this card. What was said as it was put where you work:' : 'Please take this card.') : params.unkept ? 'What you changed on this card could not be committed, so it has not gone on to anyone. Put right what is in the way of the commit, where you worked, and say when you have. What git said:' : params.answer ? 'You asked something on this card, and it has been answered. Take the card up again where you left it. The answer:' : params.note != '' ? 'This card was sent back, for you to take up again where you left it. What was said:' : 'This card was sent back with nothing said. Look at the card, and at what you did and said before, to find what is wrong, and put it right. If you cannot tell what is wrong, do not hand it on as it is: ask, as is said below.' }

${params.note}

Card ${params.item}: ${steps.card.out.title}

${steps.card.out.body}

${ (steps.card.out.more.plan ?? '') != '' ? 'How it is to be done, as was said when it was planned:' : '' }

${ steps.card.out.more.plan ?? '' }

${ steps.made.out.dir != '' ? 'You are in ' + steps.made.out.dir + ', a worktree of the project that is for this card alone, on a branch of its own: do the work here.' : 'This card has no worktree of its own: do the work in the project.' } Where a screenshot, a short video or a log would show that what you changed works, attach it to the card with the factory's own command (`record attach ${params.item} <file>`), so that whoever looks at your work sees it. When you are finished, say briefly what you did and what is left, if anything, and choose `done`. If you cannot tell what is wanted, or why the card came back to you, do not guess: change nothing, say what you need to know as a question to whoever can answer it, and choose `ask`. Your question is put on the card, which waits where it is, and you are given the card again with the answer.${ params.channel != '' ? ' What you say is posted in the channel ' + params.channel + ' under your name, so begin it with the card\'s id, ' + params.item + ', for whoever reads it there.' : '' }
"#;

#[test]
fn whoever_works_a_card_is_told_what_it_was_told_before_its_words_were_a_file() {
    // Every way a round is asked for, with and without each thing that is
    // only sometimes there: what is put together from the files is, letter
    // for letter, what the one text came to.
    let mut compared = 0;
    for back in [false, true] {
        for (unkept, answer) in [(false, false), (true, false), (false, true), (true, true)] {
            for note in ["", "Use a table."] {
                for plan in [json!(null), json!(""), json!("A column for it.")] {
                    for (dir, channel) in
                        [("", ""), ("/work/.lanes/shop-12", ""), ("", "shop"), ("/w", "shop")]
                    {
                        let params = json!({
                            "item": "shop-12", "back": back, "unkept": unkept, "answer": answer,
                            "note": note, "channel": channel,
                        });
                        let card =
                            json!({ "title": "A cart", "body": "With ${tax}.", "more": { "plan": plan } });
                        let within = json!({
                            "params": params,
                            "steps": { "card": { "out": card }, "made": { "out": { "dir": dir } } },
                        });
                        let before =
                            blueprint::template::render(&json!(WORKER_BEFORE), within.as_object().unwrap());
                        assert_eq!(json!(told_within("std/work", "asks", within.clone())), before.unwrap());
                        compared += 1;
                    }
                }
            }
        }
    }
    assert_eq!(compared, 192);
    // And one of them written out, so that the words are read here too.
    let within = json!({
        "params": { "item": "shop-12", "back": false, "unkept": false, "answer": false, "note": "", "channel": "" },
        "steps": { "card": { "out": { "title": "A cart", "body": "With tax." } }, "made": { "out": { "dir": "" } } },
    });
    assert_eq!(
        told_within("std/work", "asks", within),
        "Please take this card.\n\n\n\nCard shop-12: A cart\n\nWith tax.\n\n\n\n\n\nThis card has no worktree of \
         its own: do the work in the project. Where a screenshot, a short video or a log would show that what you \
         changed works, attach it to the card with the factory's own command (`record attach shop-12 <file>`), so \
         that whoever looks at your work sees it. When you are finished, say briefly what you did and what is left, if \
         anything, and choose `done`. If you cannot tell what is wanted, or why the card came back to you, do not \
         guess: change nothing, say what you need to know as a question to whoever can answer it, and choose \
         `ask`. Your question is put on the card, which waits where it is, and you are given the card again with \
         the answer.\n"
    );
}
