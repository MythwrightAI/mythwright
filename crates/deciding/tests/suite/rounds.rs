//! What is kept of something put to an agent a number of times at most
//! (`deciding::rounds`, asked by `std/proposal` and `std/feature` as
//! `agent.round`): a table of what was kept and said, and what is kept now.

use deciding::rounds::{Change, News, Round, news, round};

fn kept(rounds: i64, agent: &str, next: &str, its: &str, open: bool, whom: &str) -> Round {
    Round {
        rounds,
        agent: agent.to_string(),
        next: next.to_string(),
        its: its.to_string(),
        open,
        whom: whom.to_string(),
    }
}

#[test]
fn a_round_is_counted_and_whoever_took_it_has_the_next_while_there_may_be_one() {
    // How many there were, how many there may be, whether one more counts;
    // who took it, who had them, who was named; and what is kept.
    type Case = ((i64, i64, bool, Option<&'static str>, &'static str, &'static str), Round);
    let cases: [Case; 12] = [
        // The first, taken by one of the role: it has the rest.
        (
            (0, 8, true, Some("shop-architect-1"), "", ""),
            kept(
                1,
                "shop-architect-1",
                "shop-architect-1",
                "agent:shop-architect-1",
                true,
                "shop-architect-1",
            ),
        ),
        // Another, by the same: counted, and still its own.
        (
            (1, 8, true, Some("shop-architect-1"), "shop-architect-1", ""),
            kept(
                2,
                "shop-architect-1",
                "shop-architect-1",
                "agent:shop-architect-1",
                true,
                "shop-architect-1",
            ),
        ),
        // The last there may be, and one past it.
        ((7, 8, true, Some("a"), "a", ""), kept(8, "a", "a", "agent:a", false, "a")),
        ((8, 8, true, Some("a"), "a", ""), kept(9, "a", "a", "agent:a", false, "a")),
        // A round that came to no end: counted, and whoever had them keeps them.
        ((2, 8, true, None, "a", "b"), kept(3, "a", "a", "agent:a", true, "a")),
        // Nobody was there to take it: nobody has it, and the next is put to whoever was named.
        ((0, 8, true, Some(""), "", "b"), kept(1, "", "b", "agent:", true, "its agent")),
        ((3, 8, true, Some(""), "a", "b"), kept(4, "", "b", "agent:", true, "its agent")),
        ((0, 8, true, Some(""), "", ""), kept(1, "", "", "agent:", true, "its agent")),
        // Asked with nothing to count: what is kept, as it stands by the number there may be now.
        ((3, 8, false, None, "a", "b"), kept(3, "a", "a", "agent:a", true, "a")),
        ((3, 3, false, None, "a", "b"), kept(3, "a", "a", "agent:a", false, "a")),
        ((0, 8, false, None, "", "b"), kept(0, "", "b", "agent:", true, "its agent")),
        // None may be had: the first is the last.
        ((0, 0, true, Some("a"), "", ""), kept(1, "a", "a", "agent:a", false, "a")),
    ];
    for ((had, most, counts, took, has, named), comes_to) in cases {
        assert_eq!(
            round(had, most, counts, took, has, named),
            Ok(comes_to),
            "{had} of {most}, {counts}, took {took:?}, has {has:?}, named {named:?}"
        );
    }
}

#[test]
fn a_number_of_rounds_below_none_is_refused_in_a_sentence() {
    assert_eq!(
        round(-1, 8, true, None, "", "").unwrap_err(),
        "`had` is a number of rounds, and is not less than none; it is -1"
    );
    assert_eq!(
        round(0, -8, true, None, "", "").unwrap_err(),
        "`most` is a number of rounds, and is not less than none; it is -8"
    );
}

#[test]
fn a_change_is_news_to_its_agent_only_where_somebody_else_made_it_and_it_may_be_told() {
    // Told where nothing is in the way: open, not settled, by somebody else, after a round.
    let told = Change {
        open: true,
        settled: false,
        by: "user:1",
        its: "agent:a",
        rounds: 1,
        reworded: false,
        moved: false,
        here: false,
    };
    let cases = [
        // What it asks was changed.
        (Change { reworded: true, ..told }, News::Changed),
        // Put back where its agent has it, having been there before.
        (Change { moved: true, here: true, ..told }, News::Back),
        // Put there before any round: its first round is what tells its agent.
        (Change { moved: true, here: true, rounds: 0, ..told }, News::None),
        // Moved somewhere else, or changed in something that is not what it asks.
        (Change { moved: true, ..told }, News::None),
        (Change { here: true, ..told }, News::None),
        (told, News::None),
        // Reworded and moved in one change is told as its coming back, wherever it went.
        (Change { reworded: true, moved: true, ..told }, News::Back),
        (Change { reworded: true, moved: true, here: true, ..told }, News::Back),
        // Its own agent's doing is no news to it.
        (Change { reworded: true, by: "agent:a", ..told }, News::None),
        (Change { moved: true, here: true, by: "agent:a", ..told }, News::None),
        // Before anybody has it, nobody's change is its own agent's.
        (Change { reworded: true, by: "agent:b", its: "agent:", ..told }, News::Changed),
        // Settled, or put to its agent as often as it may be: nothing more is told.
        (Change { reworded: true, settled: true, ..told }, News::None),
        (Change { reworded: true, open: false, ..told }, News::None),
        (Change { moved: true, here: true, open: false, ..told }, News::None),
    ];
    let mut reached = Vec::new();
    for (change, comes_to) in cases {
        assert_eq!(news(change), comes_to, "{change:?}");
        reached.push(comes_to);
    }
    for word in News::ALL {
        assert!(reached.contains(word), "no case reaches `{}`", word.word());
    }
}
