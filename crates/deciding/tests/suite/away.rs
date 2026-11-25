//! What becomes of a round when its item is moved away from it while
//! its agent is at it, and what that item comes to when it comes back:
//! each a function of the move and the item's row, with a table of
//! cases that reaches every word it can answer.

use std::collections::BTreeSet;

use deciding::work::{
    AWAY_WORDS, Away, AwayCame, Ended, EndedCame, Held, Turn, TurnCame, away, ended, left, moved_since, turn,
};

/// An item worked from `ready`, in hand in `doing`, a round of which has
/// it, moved out of `doing` to `to` by somebody.
fn moved_to(to: &str) -> Away {
    Away {
        from: "doing".into(),
        to: to.into(),
        here: "ready".into(),
        doing: "doing".into(),
        taken: "ready".into(),
        again: vec!["heard".into(), "2".into()],
        through: false,
        how: String::new(),
    }
}

fn stopped(to: &str) -> AwayCame {
    AwayCame::Stop(vec!["away".into(), to.into()])
}

#[test]
fn an_item_moved_out_of_hand_while_its_round_has_it_has_its_round_stopped_unless_its_stage_lets_it_finish() {
    let cases: Vec<(&str, Away, AwayCame)> = vec![
        ("moved back to where it waited, with nothing said: stopped", moved_to("ready"), stopped("ready")),
        ("moved further back", moved_to("backlog"), stopped("backlog")),
        ("moved to done by a person", moved_to("done"), stopped("done")),
        ("moved on to be looked at by a person", moved_to("review"), stopped("review")),
        ("and as the row says it", Away { how: "stop".into(), ..moved_to("ready") }, stopped("ready")),
        (
            "a row that lets a round finish",
            Away { how: "finish".into(), ..moved_to("ready") },
            AwayCame::Nothing,
        ),
        (
            "handed on by its round, which is a blueprint's move",
            Away { through: true, ..moved_to("review") },
            AwayCame::Nothing,
        ),
        (
            "an item no round has: its agent asked, and it waits for a person",
            Away { again: vec!["asked".into()], ..moved_to("ready") },
            AwayCame::Nothing,
        ),
        (
            "one left for a person",
            Away { again: vec!["left".into(), "its round failed".into()], ..moved_to("ready") },
            AwayCame::Nothing,
        ),
        (
            "one with nothing kept of its rounds",
            Away { again: vec![], ..moved_to("ready") },
            AwayCame::Nothing,
        ),
        (
            "one another column's stage took, whose round is not this one's",
            Away { taken: "plan".into(), ..moved_to("ready") },
            AwayCame::Nothing,
        ),
        (
            "moved within the column it is in while in hand",
            Away { to: "doing".into(), ..moved_to("ready") },
            AwayCame::Nothing,
        ),
        (
            "moved into it, from somewhere else",
            Away { from: "review".into(), to: "doing".into(), ..moved_to("ready") },
            AwayCame::Nothing,
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, meant) in cases {
        let came = away(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(came, meant, "{what}");
        reached.insert(came.word());
    }
    assert_eq!(
        reached,
        AWAY_WORDS.iter().copied().collect(),
        "every word a move away can come to has a case"
    );

    // What makes no sense is refused in a sentence.
    assert_eq!(
        away(&Away { how: "wait".into(), ..moved_to("ready") }).unwrap_err(),
        "`moved_away` says what becomes of an item's round when it is moved away while its agent is at it, \
         `stop` or `finish`; it says `wait`"
    );
    assert_eq!(
        away(&Away { doing: String::new(), ..moved_to("ready") }).unwrap_err(),
        "`here` and `doing` are the column whose items are worked and the one an item is in while in hand, \
         and one is not said"
    );
}

/// An item worked from `ready`, whose row says it was taken out of its
/// agent's hands, as it comes again.
fn back_in(status: &str, moved: &[&str]) -> Turn {
    let words = |said: &[&str]| said.iter().map(|word| word.to_string()).collect();
    Turn {
        found: true,
        status: status.into(),
        here: "ready".into(),
        doing: "doing".into(),
        taken: "ready".into(),
        again: words(&["away", "backlog"]),
        moved: words(moved),
        most: 0,
        to: String::new(),
        else_to: "review".into(),
        at: 41,
    }
}

#[test]
fn an_item_taken_out_of_its_agents_hands_is_taken_up_where_it_was_left_when_it_comes_back() {
    let round = |asked: Turn| match turn(&asked).unwrap() {
        TurnCame::Round(round) => round,
        other => panic!("a round, not {other:?}"),
    };
    // Put back where it waits: taken up again where it was left, told what was said, and not as sent back.
    let again = round(back_in("ready", &["backlog", "user", "Go on with it.", "7"]));
    assert_eq!(
        (again.mine, again.resumed, again.back, again.note.as_str(), again.heard),
        (false, true, false, "Go on with it.", 7)
    );
    assert_eq!(again.again, ["heard", "7"]);
    // Put back in hand by somebody: the same.
    let again = round(back_in("doing", &["backlog", "user", "", "8"]));
    assert_eq!((again.resumed, again.back, again.note.as_str()), (true, false, ""));
    // Put back in hand by a stage that looked at it and sent it back: it is sent back, with what was said.
    let again = round(back_in("doing", &["review", "blueprint", "The total is wrong.", "9"]));
    assert_eq!((again.resumed, again.back, again.note.as_str()), (false, true, "The total is wrong."));
    // Still away: nothing.
    assert_eq!(turn(&back_in("backlog", &["doing", "user", "", "6"])).unwrap(), TurnCame::Gone);
}

#[test]
fn a_round_stopped_because_its_item_was_moved_away_leaves_nothing_and_gives_its_place_back() {
    // The round was cut: its call says it did not finish.
    let cut =
        Ended { failed: true, reason: "the called blueprint did not finish".into(), ..Ended::default() };
    assert_eq!(
        left(&cut).as_deref(),
        Some("the called blueprint did not finish"),
        "cut with the item where it was"
    );
    let away = Ended { gone: true, ..cut };
    assert_eq!(left(&away), None, "moved away, nothing is left for a person: somebody has it");
    assert_eq!(ended(&away).unwrap(), EndedCame::Freed, "its place is given back");
    // Nor is anything said of one that ended by itself, having found it moved.
    assert_eq!(left(&Ended { gone: true, ..Ended::default() }), None);
}

#[test]
fn whether_somebody_moved_an_item_since_its_round_was_told_of_it_is_said_by_its_moves_wherever_it_went() {
    let at = |status: &str, moved: &[&str]| Held {
        status: status.into(),
        from: "ready".into(),
        doing: "doing".into(),
        moved: moved.iter().map(|word| word.to_string()).collect(),
        heard: Some(4),
    };
    let cases = [
        ("in hand as the round took it", at("doing", &["ready", "blueprint", "", "5"]), false),
        ("moved away by somebody", at("draft", &["doing", "user", "", "6"]), true),
        ("moved to done", at("done", &["doing", "user", "", "6"]), true),
        ("moved back to where it waits", at("ready", &["doing", "user", "", "6"]), true),
        (
            "still where it waits, never taken: its round failed before that",
            at("ready", &["draft", "user", "", "4"]),
            false,
        ),
        ("put back in hand by somebody", at("doing", &["draft", "user", "", "7"]), true),
        (
            "where the round was told of nothing",
            Held { heard: None, ..at("ready", &["doing", "user", "", "6"]) },
            false,
        ),
    ];
    for (what, asked, meant) in cases {
        assert_eq!(moved_since(&asked), meant, "{what}");
    }
}
