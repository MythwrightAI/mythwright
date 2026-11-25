//! What one card cost, by round (`deciding::spent`): which turns are the
//! card's by their conversation, what its history says was decided and
//! when, and which round each turn is of. A table of cases for each.

use deciding::spent::{Came, Does, Said, Turn, begun_by, decided, does, laid, rounds, way};
use serde_json::{Value, json};

fn columns() -> Vec<String> {
    ["draft", "ready", "doing", "review", "done"].iter().map(|one| one.to_string()).collect()
}

/// Everything here is said in one millisecond, as on a busy machine and
/// on a clock that stands still: nothing is placed by when.
const NOW: i64 = 1_000;

fn line(seq: u64, topic: &str, who: &str, data: Value) -> Said {
    Said { seq, topic: topic.into(), ts: NOW, who: who.into(), said: format!("{topic} at {seq}"), data }
}

fn moved(seq: u64, who: &str, from: &str, to: &str, note: &str) -> Said {
    line(seq, "board.item.moved", who, json!({ "from": from, "to": to, "note": note }))
}

fn worked(seq: u64) -> Said {
    line(seq, "card.worked", "shop-worker-1", json!({ "agent": "shop-worker-1" }))
}

/// A turn written `nth` of all, whose asking names the event `begun`.
fn turn(agent: &str, does: Does, nth: i64, begun: Option<u64>, tokens: (u64, u64), cost: f64) -> Turn {
    Turn {
        id: format!("turn-{nth}"),
        agent: agent.into(),
        does,
        status: "done".into(),
        chose: None,
        sent: tokens.0,
        wrote: tokens.1,
        cost_usd: cost,
        cost_said: true,
        asked: NOW,
        nth,
        begun,
        took_ms: Some(10),
        kept_from: false,
        way: None,
    }
}

#[test]
fn a_turn_is_the_cards_by_the_conversation_it_is_part_of_and_a_seat_is_somebody_looking() {
    let cases: [(&str, Option<Does>); 19] = [
        ("lane:shop-4", Some(Does::Works)),
        ("plan:shop-4", Some(Does::Plans)),
        ("review:shop-4:1", Some(Does::Looks)),
        ("review:shop-4:12", Some(Does::Looks)),
        // Several looking at once where the card is worked: a seat all the same.
        ("lane:shop-4:2", Some(Does::Looks)),
        ("review:shop-4", Some(Does::Looks)),
        // Another card, whose id begins the same, is not this one.
        ("lane:shop-41", None),
        ("review:shop-41:1", None),
        // Somewhere else altogether.
        ("channel:shop", None),
        ("thread:shop-4", None),
        ("lane:", None),
        ("review:shop-4:1:2", None),
        // One of several pieces of work on it at once, by its way; and its judges, alone or seated.
        ("contend:shop-4:otter", Some(Does::Contends)),
        ("contend:shop-4", Some(Does::Contends)),
        ("contend:shop-41:otter", None),
        ("contend:shop-4:otter:2", None),
        ("judge:shop-4", Some(Does::Judges)),
        ("judge:shop-4:3", Some(Does::Judges)),
        ("judge:shop-4:3:1", None),
    ];
    for (thread, then) in cases {
        assert_eq!(does(thread, "shop-4"), then, "{thread}");
    }
}

#[test]
fn which_piece_of_work_a_turn_contended_with_is_what_follows_the_card_and_nothing_else_says_one() {
    let cases: [(&str, Option<&str>); 6] = [
        ("contend:shop-4:otter", Some("otter")),
        ("contend:shop-4:slate-5", Some("slate-5")),
        ("contend:shop-4", None),
        ("contend:shop-41:otter", None),
        ("lane:shop-4:otter", None),
        ("judge:shop-4:1", None),
    ];
    for (thread, then) in cases {
        assert_eq!(way(thread, "shop-4").as_deref(), then, "{thread}");
    }
}

#[test]
fn what_was_decided_is_read_from_the_history_in_its_order_once_the_work_was_handed_on() {
    type Case = (&'static str, Vec<Said>, Vec<(Came, &'static str, &'static str)>);
    let cases: Vec<Case> = vec![
        (
            "worked, sent back by its review, worked again and approved: two rounds",
            vec![
                moved(1, "Ada", "draft", "ready", ""),
                moved(2, "std/working", "ready", "doing", ""),
                worked(3),
                moved(4, "std/working", "doing", "review", ""),
                moved(5, "std/reviewing", "review", "doing", "shop-reviewer-1: The total is wrong."),
                worked(6),
                moved(7, "std/working", "doing", "review", ""),
                line(
                    8,
                    "card.approved",
                    "shop-reviewer-1",
                    json!({ "by": "shop-reviewer-1", "words": "It does." }),
                ),
                moved(9, "std/reviewing", "review", "done", "approved by shop-reviewer-1"),
            ],
            vec![
                (Came::Back, "std/reviewing", "shop-reviewer-1: The total is wrong."),
                (Came::Passed, "shop-reviewer-1", "It does."),
            ],
        ),
        (
            "a person sends it back and then passes it by moving it to the last column",
            vec![
                worked(1),
                moved(2, "Ada", "review", "ready", "Not yet."),
                worked(3),
                moved(4, "Ada", "review", "done", ""),
            ],
            vec![(Came::Back, "Ada", "Not yet."), (Came::Passed, "Ada", "")],
        ),
        (
            "pulled out of work before anything was handed on decides nothing",
            vec![moved(1, "std/working", "ready", "doing", ""), moved(2, "Ada", "doing", "ready", "")],
            vec![],
        ),
        (
            "left for a person, having been sent back as often as it may be",
            vec![
                worked(1),
                moved(2, "std/reviewing", "review", "doing", "Wrong."),
                worked(3),
                line(4, "card.left", "std/reviewing", json!({ "rounds": 1 })),
            ],
            vec![(Came::Back, "std/reviewing", "Wrong."), (Came::Left, "std/reviewing", "")],
        ),
        (
            "left by a round that failed, with why, before anything was handed on",
            vec![line(1, "card.left", "std/working", json!({ "why": "Its place could not be made." }))],
            vec![(Came::Left, "std/working", "Its place could not be made.")],
        ),
        (
            "a move to a column the board has not got decides nothing",
            vec![worked(1), moved(2, "Ada", "review", "elsewhere", "")],
            vec![],
        ),
        (
            "approved, its move to the last column after is no second decision",
            vec![
                worked(1),
                line(2, "card.approved", "a", json!({ "by": "a" })),
                moved(3, "std/reviewing", "review", "done", ""),
            ],
            vec![(Came::Passed, "a", "")],
        ),
    ];
    for (what, said, then) in cases {
        let came: Vec<(Came, String, String)> =
            decided(&columns(), &said).into_iter().map(|one| (one.came, one.by, one.words)).collect();
        let then: Vec<(Came, String, String)> =
            then.into_iter().map(|(came, by, words)| (came, by.to_string(), words.to_string())).collect();
        assert_eq!(came, then, "{what}");
    }
}

#[test]
fn the_event_an_asking_names_is_read_from_its_word_or_else_from_the_name_of_what_asked() {
    // Its word and who asked, and the number of the event that comes to.
    type Case = ((Option<&'static str>, Option<&'static str>), Option<u64>);
    let cases: [Case; 10] = [
        // A worker's word is the number of the event that gave the card its turn.
        ((Some("123"), Some("u_ada//working-shop-stages-1/round-shop-4")), Some(123)),
        // A seat of a council laid into a round says it with its seat.
        ((Some("123:2"), None), Some(123)),
        // A plan's round.
        ((Some("round-77"), None), Some(77)),
        // A seat at a review is asked by the review's instance, whose name says it.
        ((Some("in_0abc12:1"), Some("u_ada/in_0def34/review-shop-4-55")), Some(55)),
        ((Some("in_0abc12:1"), Some("u_ada/in_0def34/review-shop-41-56")), Some(56)),
        // Nothing that ends in a number.
        ((Some("in_0abc12"), Some("u_ada//cards")), None),
        ((Some("hello"), None), None),
        ((Some(""), Some("")), None),
        ((None, Some("u_ada/in_0def34/review-shop-4-")), None),
        ((None, None), None),
    ];
    for ((asking, asked_by), then) in cases {
        assert_eq!(begun_by(asking, asked_by), then, "{asking:?} {asked_by:?}");
    }
}

#[test]
fn each_turn_is_of_the_round_decided_after_the_event_its_asking_names_whatever_millisecond_it_was() {
    // Every line and every turn in one millisecond.
    let said = vec![
        worked(10),
        moved(20, "std/reviewing", "review", "doing", "Wrong."),
        worked(30),
        line(40, "card.approved", "shop-reviewer-1", json!({ "by": "shop-reviewer-1" })),
    ];
    let turns = vec![
        // Given out of order: each round has its own in the order they were written.
        turn("shop-reviewer-1", Does::Looks, 2, Some(15), (100, 10), 0.01),
        turn("shop-worker-1", Does::Works, 1, Some(5), (1000, 200), 0.25),
        turn("shop-worker-1", Does::Works, 3, Some(25), (1200, 300), 0.3),
        // Its review asked for by the event just before the line that decided it.
        turn("shop-reviewer-1", Does::Looks, 4, Some(39), (90, 9), 0.01),
        // Begun after the last: a round under way.
        turn("shop-worker-1", Does::Works, 5, Some(41), (0, 0), 0.0),
    ];
    let all = rounds(&columns(), &said, turns);
    let shape: Vec<(usize, Came, Vec<i64>, usize)> = all
        .iter()
        .map(|round| {
            (round.n, round.decided.came, round.turns.iter().map(|one| one.nth).collect(), round.by_order)
        })
        .collect();
    assert_eq!(
        shape,
        vec![(1, Came::Back, vec![1, 2], 0), (2, Came::Passed, vec![3, 4], 0), (3, Came::Open, vec![5], 0)]
    );
    // What each round came to is counted, and a turn that said nothing of
    // what it took is counted as that and not as nothing taken.
    let first = &all[0].spent;
    assert_eq!((first.turns, first.sent, first.wrote, first.took_ms, first.unsaid), (2, 1100, 210, 20, 0));
    assert!((first.cost_usd - 0.26).abs() < 1e-9);
    assert_eq!((all[2].spent.turns, all[2].spent.unsaid), (1, 1));

    // Laid out once, with what all of them came to.
    let out = laid("shop-4", &all);
    assert_eq!(out["card"], "shop-4");
    assert_eq!(
        (&out["spent"]["turns"], &out["spent"]["sent"], &out["spent"]["unsaid"]),
        (&json!(5), &json!(2390), &json!(1))
    );
    assert_eq!(out["by_order"], 0);
    assert_eq!(
        out["rounds"][0]["decided"],
        json!({ "came": "back", "by": "std/reviewing", "words": "Wrong.", "said": "board.item.moved at 20", "at": NOW })
    );
    assert_eq!(out["rounds"][2]["decided"], json!({ "came": "open" }));
    assert_eq!(out["rounds"][0]["turns"][1]["does"], "looks");
    assert_eq!(out["rounds"][2]["turns"][0]["says"], false);
}

#[test]
fn a_turn_from_before_that_names_no_asking_goes_with_the_turn_written_beside_it() {
    let said = vec![worked(10), moved(20, "std/reviewing", "review", "doing", "Wrong."), worked(30)];
    let turns = vec![
        // The first says nothing, and goes with the one after it.
        turn("shop-worker-1", Does::Works, 1, None, (1, 1), 0.0),
        turn("shop-worker-1", Does::Works, 2, Some(5), (1, 1), 0.0),
        // Saying nothing, it goes with the one written just before it.
        turn("shop-reviewer-1", Does::Looks, 3, None, (1, 1), 0.0),
        turn("shop-worker-1", Does::Works, 4, Some(25), (1, 1), 0.0),
    ];
    let all = rounds(&columns(), &said, turns);
    let shape: Vec<(Came, Vec<i64>)> = all
        .iter()
        .map(|round| (round.decided.came, round.turns.iter().map(|one| one.nth).collect()))
        .collect();
    assert_eq!(shape, vec![(Came::Back, vec![1, 2, 3]), (Came::Open, vec![4])]);
}

#[test]
fn where_no_turn_of_the_card_names_its_asking_they_are_put_in_rounds_by_the_order_they_were_written_and_said_to_be()
 {
    let said = vec![
        worked(10),
        moved(20, "std/reviewing", "review", "doing", "Wrong."),
        worked(30),
        line(40, "card.approved", "shop-reviewer-1", json!({ "by": "shop-reviewer-1" })),
    ];
    let turns = vec![
        turn("shop-worker-1", Does::Works, 1, None, (1, 1), 0.0),
        // Asked again in the same round, after a question: no new round.
        turn("shop-worker-1", Does::Works, 2, None, (1, 1), 0.0),
        turn("shop-reviewer-1", Does::Looks, 3, None, (1, 1), 0.0),
        turn("shop-reviewer-2", Does::Looks, 4, None, (1, 1), 0.0),
        // Work after somebody looked begins the next round.
        turn("shop-worker-1", Does::Works, 5, None, (1, 1), 0.0),
        turn("shop-reviewer-1", Does::Looks, 6, None, (1, 1), 0.0),
        // More work than there were decisions: the round under way.
        turn("shop-worker-1", Does::Works, 7, None, (1, 1), 0.0),
    ];
    let all = rounds(&columns(), &said, turns);
    let shape: Vec<(Came, Vec<i64>, usize)> = all
        .iter()
        .map(|round| (round.decided.came, round.turns.iter().map(|one| one.nth).collect(), round.by_order))
        .collect();
    assert_eq!(
        shape,
        vec![(Came::Back, vec![1, 2, 3, 4], 4), (Came::Passed, vec![5, 6], 2), (Came::Open, vec![7], 1)]
    );
    assert_eq!(laid("shop-4", &all)["by_order"], 7);
}

#[test]
fn a_card_with_nothing_decided_has_its_turns_in_one_round_and_one_with_nothing_has_none() {
    let turns = vec![
        turn("a", Does::Works, 1, Some(1), (1, 1), 0.0),
        turn("b", Does::Looks, 2, Some(2), (1, 1), 0.0),
    ];
    let all = rounds(&columns(), &[], turns);
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].n, all[0].decided.came, all[0].turns.len()), (1, Came::Open, 2));
    assert!(rounds(&columns(), &[], Vec::new()).is_empty());
    // Decided with no turn of its own (a person sent it back before any agent looked): a round of none.
    let said = vec![worked(1), moved(2, "Ada", "review", "doing", "")];
    let all = rounds(&columns(), &said, vec![turn("a", Does::Works, 3, Some(3), (1, 1), 0.0)]);
    let shape: Vec<(Came, usize)> = all.iter().map(|round| (round.decided.came, round.turns.len())).collect();
    assert_eq!(shape, vec![(Came::Back, 0), (Came::Open, 1)]);
}

#[test]
fn a_turn_kept_from_whoever_reads_is_laid_out_with_its_figures_and_says_so() {
    let mut kept = turn("a", Does::Works, 1, None, (10, 2), 0.5);
    kept.kept_from = true;
    let out = laid("shop-4", &rounds(&columns(), &[], vec![kept]));
    let one = &out["rounds"][0]["turns"][0];
    assert_eq!((&one["kept_from"], &one["sent"], &one["chose"]), (&json!(true), &json!(10), &Value::Null));
}

#[test]
fn a_turn_whose_program_said_nothing_of_what_it_cost_is_counted_as_not_reported_and_never_as_nought() {
    // One that said, and two that said tokens alone, whose nought is no cost.
    let said = turn("shop-worker-1", Does::Works, 1, None, (1000, 100), 0.25);
    let unsaid =
        |nth| Turn { cost_said: false, ..turn("shop-worker-1", Does::Works, nth, None, (900, 90), 0.0) };
    let out = laid("shop-4", &rounds(&columns(), &[], vec![said, unsaid(2), unsaid(3)]));
    assert_eq!(out["spent"]["cost_usd"], 0.25);
    assert_eq!(out["spent"]["cost_unsaid"], 2);
    let turns = out["rounds"][0]["turns"].as_array().unwrap();
    assert_eq!(
        turns.iter().map(|one| (one["cost_said"].clone(), one["says"].clone())).collect::<Vec<_>>(),
        [(json!(true), json!(true)), (json!(false), json!(true)), (json!(false), json!(true))],
        "tokens alone are something said of what it took"
    );
    // One in hand has not said yet, and is none of them.
    let in_hand = Turn { status: "running".into(), ..unsaid(5) };
    let out = laid("shop-4", &rounds(&columns(), &[], vec![unsaid(2), in_hand]));
    assert_eq!(out["spent"]["cost_unsaid"], 1);
    // Of turns none of which said, the cost is not reported, and nought is in no figure.
    let none = laid("shop-4", &rounds(&columns(), &[], vec![unsaid(4)]));
    assert_eq!(
        (none["spent"]["cost_usd"].clone(), none["spent"]["cost_unsaid"].clone()),
        (json!(0.0), json!(1))
    );
}
