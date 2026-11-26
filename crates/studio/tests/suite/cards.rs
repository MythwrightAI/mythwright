//! What a card keeps beyond where it is, and what follows from it
//! (`studio::cards`, asked by `std/cards` as `board.came`, `board.said`,
//! `board.waits`, `board.after` and `board.held`): for each, a table of
//! what is handed and what it comes to, with no word left unreached. The
//! columns are handed, and some cases call them something else than a
//! board usually does: a name written into a block fails those.

use serde_json::{Map, Value, json};
use studio::cards::{
    After, Awaited, Came, Columns, Kept, Row, Said, after, came, held, said, turn, waiting_for,
    waits_on_others,
};

/// What a row keeps, from the cells as a record holds them.
fn kept(status: &str, cells: Value) -> Kept {
    let more: Map<String, Value> = cells.as_object().cloned().unwrap_or_default();
    Kept::of(status, &more)
}

const USUAL: Columns<'static> =
    Columns { worked: "doing", looked: "review", done: "done", before: &["backlog", "ready", "doing"] };
/// A board of somebody's own, whose columns are called other things.
const OWN: Columns<'static> =
    Columns { worked: "bench", looked: "inspection", done: "shipped", before: &["queue", "bench"] };
/// A board that has no column where work is looked over: nothing stands before it.
const UNLOOKED: Columns<'static> = Columns { worked: "doing", looked: "review", done: "done", before: &[] };

#[test]
fn a_cards_row_is_read_with_a_cell_emptied_and_one_never_filled_both_saying_nothing() {
    // Never filled: a card from before any of this was kept.
    assert_eq!(kept("ready", json!({})), Kept { status: "ready".into(), ..Kept::default() });
    // Emptied in the client: empty text, and a list with nothing in it.
    let emptied =
        json!({ "question": "", "asker": "", "taken": "", "waiting": "", "after": [], "rounds": null });
    assert_eq!(kept("ready", emptied), Kept { status: "ready".into(), ..Kept::default() });
    // Filled.
    let filled = json!({
        "question": "Which total?", "asker": "shop-worker-1", "taken": "ready", "rounds": 2,
        "waiting": "after shop-1, not done yet", "after": ["shop-1", "shop-2"],
    });
    assert_eq!(
        kept("doing", filled),
        Kept {
            status: "doing".into(),
            question: "Which total?".into(),
            asker: "shop-worker-1".into(),
            taken: "ready".into(),
            rounds: 2,
            waiting: "after shop-1, not done yet".into(),
            after: vec!["shop-1".into(), "shop-2".into()],
        }
    );
    // A count kept with a point in it, one below nothing, and one that is
    // no number; a board of somebody's own that names one card and no list.
    assert_eq!(kept("x", json!({ "rounds": 3.0 })).rounds, 3);
    assert_eq!(kept("x", json!({ "rounds": -1 })).rounds, 0);
    assert_eq!(kept("x", json!({ "rounds": "two" })).rounds, 0);
    assert_eq!(kept("x", json!({ "after": "shop-9" })).after, ["shop-9"]);
    assert_eq!(kept("x", json!({ "after": ["shop-9", ""] })).after, ["shop-9"]);
}

#[test]
fn a_move_is_a_card_taken_sent_back_or_finished_by_its_row_and_the_columns_it_is_handed() {
    // from, to, by hand, the row, the columns; what it came to, settles, rounds, rests.
    type Case = (&'static str, &'static str, bool, Value, Columns<'static>, Came, bool, Option<u64>, bool);
    let cases: Vec<Case> = vec![
        // Taken for the first time: nothing says where from yet.
        ("ready", "doing", false, json!({}), USUAL, Came::Taken, false, None, false),
        ("ready", "doing", false, json!({ "taken": "" }), USUAL, Came::Taken, false, None, false),
        // By a person, straight from backlog: taken all the same.
        ("backlog", "doing", true, json!({}), USUAL, Came::Taken, false, None, false),
        // Sent back from where work is looked over, by anyone, counted from nothing.
        ("review", "doing", false, json!({ "taken": "ready" }), USUAL, Came::Back, false, Some(1), false),
        (
            "review",
            "doing",
            true,
            json!({ "taken": "ready", "rounds": 2 }),
            USUAL,
            Came::Back,
            false,
            Some(3),
            false,
        ),
        // Sent back from there before anything said where it was taken from.
        ("review", "doing", true, json!({}), USUAL, Came::Back, false, Some(1), false),
        // Sent back by whatever looked at it after: from anywhere but where it was taken.
        (
            "tested",
            "doing",
            false,
            json!({ "taken": "ready", "rounds": 1 }),
            USUAL,
            Came::Back,
            false,
            Some(2),
            false,
        ),
        ("done", "doing", true, json!({ "taken": "ready" }), USUAL, Came::Back, false, Some(1), false),
        // Sent back from where work is looked over to a column before it
        // that is not where cards are worked, as its stage may say: by
        // anyone, and counted the same.
        (
            "review",
            "ready",
            false,
            json!({ "taken": "ready", "rounds": 1 }),
            USUAL,
            Came::Back,
            false,
            Some(2),
            false,
        ),
        ("review", "backlog", true, json!({ "taken": "ready" }), USUAL, Came::Back, false, Some(1), false),
        ("inspection", "queue", true, json!({ "taken": "queue" }), OWN, Came::Back, false, Some(1), false),
        // Put in such a column from anywhere else, it was not: from where
        // it is worked, by its own round giving it up or by a person.
        ("doing", "ready", false, json!({ "taken": "ready" }), USUAL, Came::Elsewhere, false, None, false),
        // And on from where work is looked over, to a column after it.
        ("review", "tested", false, json!({ "taken": "ready" }), USUAL, Came::Elsewhere, false, None, false),
        // On a board that has no such column nothing stands before it.
        ("review", "ready", true, json!({ "taken": "ready" }), UNLOOKED, Came::Elsewhere, false, None, false),
        // Put back where it is worked from where it was taken: taken up again, and no more.
        (
            "ready",
            "doing",
            false,
            json!({ "taken": "ready", "rounds": 1 }),
            USUAL,
            Came::Elsewhere,
            false,
            None,
            false,
        ),
        // Finished; and finished while it still says it waits.
        ("review", "done", false, json!({ "taken": "ready" }), USUAL, Came::Finished, false, None, false),
        (
            "review",
            "done",
            true,
            json!({ "waiting": "one card is worked at a time here" }),
            USUAL,
            Came::Finished,
            false,
            None,
            true,
        ),
        // Anywhere else is nothing kept.
        ("backlog", "ready", true, json!({}), USUAL, Came::Elsewhere, false, None, false),
        ("doing", "review", false, json!({ "taken": "ready" }), USUAL, Came::Elsewhere, false, None, false),
        // A person's move settles what was asked, wherever they put it; a blueprint's does not.
        (
            "doing",
            "ready",
            true,
            json!({ "question": "Which total?" }),
            USUAL,
            Came::Elsewhere,
            true,
            None,
            false,
        ),
        (
            "doing",
            "ready",
            false,
            json!({ "question": "Which total?" }),
            USUAL,
            Came::Elsewhere,
            false,
            None,
            false,
        ),
        (
            "doing",
            "done",
            true,
            json!({ "question": "Which total?" }),
            USUAL,
            Came::Finished,
            true,
            None,
            false,
        ),
        (
            "review",
            "doing",
            true,
            json!({ "question": "?", "taken": "ready" }),
            USUAL,
            Came::Back,
            true,
            Some(1),
            false,
        ),
        // An emptied question is none.
        ("doing", "ready", true, json!({ "question": "" }), USUAL, Came::Elsewhere, false, None, false),
        // A board whose columns are called other things: the usual names mean nothing there.
        ("queue", "bench", false, json!({}), OWN, Came::Taken, false, None, false),
        (
            "inspection",
            "bench",
            false,
            json!({ "taken": "queue", "rounds": 4 }),
            OWN,
            Came::Back,
            false,
            Some(5),
            false,
        ),
        (
            "inspection",
            "shipped",
            false,
            json!({ "waiting": "after a-1, not done yet" }),
            OWN,
            Came::Finished,
            false,
            None,
            true,
        ),
        ("review", "doing", false, json!({ "taken": "queue" }), OWN, Came::Elsewhere, false, None, false),
        ("review", "done", false, json!({}), OWN, Came::Elsewhere, false, None, false),
        ("review", "bench", false, json!({}), OWN, Came::Taken, false, None, false),
    ];
    let mut reached = Vec::new();
    for (from, to, by_hand, row, columns, word, settles, rounds, rests) in cases {
        let arrived = came(from, to, by_hand, false, &kept(to, row.clone()), columns);
        assert_eq!(
            (arrived.came, arrived.settles, arrived.rounds, arrived.rests),
            (word, settles, rounds, rests),
            "{from} to {to}, by hand {by_hand}, {row}, {columns:?}"
        );
        reached.push(arrived.came);
    }
    for word in Came::ALL {
        assert!(reached.contains(word), "no case reaches `{}`", word.word());
    }
    // Sent back by a blueprint that says it is no verdict on the work (to
    // be brought up to date): nothing is counted, nor kept. Said of a
    // person's move, it is not heeded; and a card finished is finished.
    let row = json!({ "taken": "ready", "rounds": 1 });
    let uncounted =
        |from: &str, to: &str, by_hand: bool| came(from, to, by_hand, true, &kept(to, row.clone()), USUAL);
    assert_eq!(
        (uncounted("review", "doing", false).came, uncounted("review", "doing", false).rounds),
        (Came::Elsewhere, None)
    );
    assert_eq!(uncounted("review", "doing", true).rounds, Some(2));
    assert_eq!(uncounted("review", "done", false).came, Came::Finished);
}

#[test]
fn what_is_said_on_a_card_answers_what_was_asked_unless_nothing_was_or_the_asker_spoke() {
    let cases = [
        ("user:ada", json!({ "question": "Which total?", "asker": "shop-worker-1" }), Said::Answer),
        (
            "agent:shop-reviewer-1",
            json!({ "question": "Which total?", "asker": "shop-worker-1" }),
            Said::Answer,
        ),
        // Whoever asked says more: that is not the answer.
        (
            "agent:shop-worker-1",
            json!({ "question": "Which total?", "asker": "shop-worker-1" }),
            Said::Nothing,
        ),
        // Nothing was asked, or it was emptied.
        ("user:ada", json!({}), Said::Nothing),
        ("user:ada", json!({ "question": "", "asker": "shop-worker-1" }), Said::Nothing),
        // A board that keeps no asker takes anybody's words for the answer.
        ("agent:shop-worker-1", json!({ "question": "Which total?" }), Said::Answer),
        ("user:ada", json!({ "question": "Which total?", "asker": "" }), Said::Answer),
        // A person of the same name as the agent that asked is not that agent.
        ("user:shop-worker-1", json!({ "question": "?", "asker": "shop-worker-1" }), Said::Answer),
    ];
    let mut reached = Vec::new();
    for (by, row, word) in cases {
        let came = said(by, &kept("doing", row.clone()));
        assert_eq!(came, word, "{by}, {row}");
        reached.push(came);
    }
    for word in Said::ALL {
        assert!(reached.contains(word), "no case reaches `{}`", word.word());
    }
}

#[test]
fn a_card_that_waits_its_turn_says_how_many_are_worked_at_once_or_why_nothing_is_started() {
    for (why, limit, says) in [
        ("", 1, "one card is worked at a time here"),
        ("", 3, "3 cards are worked at a time here"),
        ("", 2, "2 cards are worked at a time here"),
        // Why nothing is started comes before how many.
        ("a release is going out", 1, "no work is being started here for now: a release is going out"),
        ("a release is going out", 5, "no work is being started here for now: a release is going out"),
        // A pool that says no number.
        ("", 0, "0 cards are worked at a time here"),
    ] {
        assert_eq!(turn(why, limit), says, "`{why}`, {limit}");
    }
}

#[test]
fn what_a_card_says_it_waits_on_is_read_back_as_waiting_on_cards_and_nothing_else_is() {
    let one = waiting_for(&["shop-1".to_string()]);
    let two = waiting_for(&["shop-1".to_string(), "shop-4".to_string()]);
    assert_eq!(one, "after shop-1, not done yet");
    assert_eq!(two, "after shop-1, shop-4, not done yet");
    assert!(waits_on_others(&one) && waits_on_others(&two));
    // Waiting its turn is not waiting on cards; neither is nothing.
    for other in [turn("", 1), turn("", 3), turn("paused", 1), String::new(), "afterwards".to_string()] {
        assert!(!waits_on_others(&other), "`{other}`");
    }
}

#[test]
fn a_card_asked_whether_it_may_begin_is_told_when_nothing_is_left_and_says_what_it_waits_for_meanwhile() {
    let (one, two) = (vec!["shop-1".to_string()], vec!["shop-1".to_string(), "shop-2".to_string()]);
    let none: Vec<String> = Vec::new();
    let waits = |on: &[String]| json!({ "waiting": waiting_for(on) });
    // asked, left, the row, whether the board keeps `waiting`; came, says, lifts, news, answers.
    type Case<'a> = (bool, &'a [String], Value, bool, Awaited, &'static str, bool, bool, bool);
    let cases: Vec<Case<'_>> = vec![
        // Asked with nothing left: told at once, nothing written.
        (true, &none, json!({}), true, Awaited::Clear, "", false, false, true),
        // Asked and it has to wait: it says so.
        (true, &one, json!({}), true, Awaited::Waits, "after shop-1, not done yet", false, true, false),
        (
            true,
            &two,
            json!({ "waiting": "" }),
            true,
            Awaited::Waits,
            "after shop-1, shop-2, not done yet",
            false,
            true,
            false,
        ),
        // Asked again while it says exactly that: nothing new.
        (true, &one, waits(&one), true, Awaited::Waits, "after shop-1, not done yet", false, false, false),
        // What it waits on is fewer than it said: said again.
        (true, &one, waits(&two), true, Awaited::Waits, "after shop-1, not done yet", false, true, false),
        // Asked while it said it waited its turn: what it waits for now is cards.
        (
            true,
            &one,
            json!({ "waiting": "one card is worked at a time here" }),
            true,
            Awaited::Waits,
            "after shop-1, not done yet",
            false,
            true,
            false,
        ),
        // Asked, it said it waited, and nothing is left: lifted and told.
        (true, &none, waits(&one), true, Awaited::Clear, "", true, false, true),
        // Looked at again, it said it waited, and nothing is left: lifted and told.
        (false, &none, waits(&two), true, Awaited::Clear, "", true, false, true),
        // Looked at again a second time, for a second card finished in the
        // same moment: it says nothing now, nobody is waiting, nobody is told.
        (false, &none, json!({ "waiting": "" }), true, Awaited::Clear, "", false, false, false),
        (false, &none, json!({}), true, Awaited::Clear, "", false, false, false),
        // Looked at again and still waiting on fewer: said again.
        (false, &one, waits(&two), true, Awaited::Waits, "after shop-1, not done yet", false, true, false),
        (false, &one, waits(&one), true, Awaited::Waits, "after shop-1, not done yet", false, false, false),
        // Looked at again, nobody having asked: a card in backlog whose plan
        // names others is left as it is, whatever is left.
        (false, &one, json!({}), true, Awaited::Waits, "after shop-1, not done yet", false, false, false),
        (
            false,
            &two,
            json!({ "waiting": "one card is worked at a time here" }),
            true,
            Awaited::Waits,
            "after shop-1, shop-2, not done yet",
            false,
            false,
            false,
        ),
        // A board that keeps nothing of what a card waits for: asked, it is
        // told as any is; looked at again with nothing left it is told too,
        // since it could not have said it was asked.
        (true, &none, json!({}), false, Awaited::Clear, "", false, false, true),
        (true, &one, json!({}), false, Awaited::Waits, "after shop-1, not done yet", false, true, false),
        (false, &none, json!({}), false, Awaited::Clear, "", false, false, true),
        (false, &one, json!({}), false, Awaited::Waits, "after shop-1, not done yet", false, false, false),
    ];
    let mut reached = Vec::new();
    for (asked, left, row, keeps, came, says, lifts, news, answers) in cases {
        let got = after(asked, left, &kept("ready", row.clone()), keeps);
        let want = After { came, says: says.to_string(), lifts, news, answers };
        assert_eq!(got, want, "asked {asked}, left {left:?}, {row}, keeps {keeps}");
        reached.push(got.came);
    }
    for word in Awaited::ALL {
        assert!(reached.contains(word), "no case reaches `{}`", word.word());
    }
}

#[test]
fn the_cards_that_wait_on_others_are_those_not_finished_that_name_cards_and_say_they_wait_on_them() {
    let row = |id: &str, status: &str, cells: Value| Row { id: id.to_string(), kept: kept(status, cells) };
    let waits = |on: &[&str]| waiting_for(&on.iter().map(|id| id.to_string()).collect::<Vec<_>>());
    let rows = vec![
        // Waits on one, and says so.
        row("shop-2", "ready", json!({ "after": ["shop-1"], "waiting": waits(&["shop-1"]) })),
        // Waits on two.
        row(
            "shop-3",
            "ready",
            json!({ "after": ["shop-1", "shop-2"], "waiting": waits(&["shop-1", "shop-2"]) }),
        ),
        // Names a card and was never asked: in backlog, saying nothing.
        row("shop-4", "backlog", json!({ "after": ["shop-1"] })),
        // Names a card and waits its turn, not on cards.
        row(
            "shop-5",
            "ready",
            json!({ "after": ["shop-1"], "waiting": "one card is worked at a time here" }),
        ),
        // Finished, with what it said left on it.
        row("shop-6", "done", json!({ "after": ["shop-1"], "waiting": waits(&["shop-1"]) })),
        // Says it waits on cards and names none any more.
        row("shop-7", "ready", json!({ "after": [], "waiting": waits(&["shop-1"]) })),
        // Names nothing, says nothing.
        row("shop-1", "doing", json!({})),
    ];
    // Every one that says it waits on others.
    assert_eq!(held(&rows, "done", None, true), ["shop-2", "shop-3"]);
    // Those that wait on one card; on another; on one nobody names.
    assert_eq!(held(&rows, "done", Some("shop-1"), true), ["shop-2", "shop-3"]);
    assert_eq!(held(&rows, "done", Some("shop-2"), true), ["shop-3"]);
    assert_eq!(held(&rows, "done", Some("shop-9"), true), Vec::<String>::new());
    // A board whose last column is called something else: one "done" there is not finished.
    assert_eq!(held(&rows, "shipped", None, true), ["shop-2", "shop-3", "shop-6"]);
    // A board that keeps nothing of what a card waits for: those that name
    // the card are all there are to go by, and with no card named, none.
    assert_eq!(held(&rows, "done", Some("shop-1"), false), ["shop-2", "shop-3", "shop-4", "shop-5"]);
    assert_eq!(held(&rows, "done", None, false), Vec::<String>::new());
    // No rows, none.
    assert_eq!(held(&[], "done", None, true), Vec::<String>::new());
}
