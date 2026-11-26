//! Whether an item waiting where items are worked may be taken now, or
//! waits behind items worked from there that are not merged
//! (`deciding::ahead`, asked by `std/working` as `work.ahead`).

use std::collections::BTreeSet;

use deciding::ahead::{AHEAD_WORDS, Ahead, AheadCame, ahead};
use serde_json::{Value, json};

/// An item on the board: its id, its column and the column it was taken from.
fn on(id: &str, status: &str, taken: &str) -> Value {
    match taken {
        "" => json!({ "id": id, "status": status, "more": {} }),
        taken => json!({ "id": id, "status": status, "more": { "taken": taken } }),
    }
}

/// `shop-3`, in `status`, asked about where `ready` is worked and items go
/// on to `review`, with the row saying `next_after` and the board these.
fn asked(status: &str, next_after: &str, items: &[Value]) -> Ahead {
    Ahead {
        item: "shop-3".into(),
        status: status.into(),
        here: "ready".into(),
        doing: "doing".into(),
        hands_to: "review".into(),
        done: "done".into(),
        next_after: next_after.into(),
        items: items.to_vec(),
        waiting: String::new(),
    }
}

fn behind(ids: &[&str], why: &str) -> AheadCame {
    AheadCame::Behind { ids: ids.iter().map(|id| id.to_string()).collect(), why: why.into(), news: true }
}

#[test]
fn an_item_is_taken_once_none_worked_from_its_column_waits_unmerged_where_its_row_says_so() {
    let in_review = || on("shop-1", "review", "ready");
    let one = "shop-1, worked from here, is not merged yet";
    let cases: Vec<(&str, Ahead, AheadCame)> = vec![
        // As it always was: the next is taken as soon as the last is handed on.
        ("nothing said, one in review", asked("ready", "", &[in_review()]), AheadCame::Go),
        ("`handed`, one in review", asked("ready", "handed", &[in_review()]), AheadCame::Go),
        // Once the last is merged: one in review, or still in hand, holds it back.
        ("`merged`, one in review", asked("ready", "merged", &[in_review()]), behind(&["shop-1"], one)),
        (
            "`merged`, one in hand",
            asked("ready", "merged", &[on("shop-1", "doing", "ready")]),
            behind(&["shop-1"], one),
        ),
        (
            "`merged`, one in hand and one in review, in the order of the board",
            asked("ready", "merged", &[on("shop-2", "doing", "ready"), in_review()]),
            behind(&["shop-2", "shop-1"], "shop-2, shop-1, worked from here, are not merged yet"),
        ),
        // What holds nothing back: finished, waiting with it, put elsewhere by a person.
        (
            "`merged`, the last is done",
            asked("ready", "merged", &[on("shop-1", "done", "ready")]),
            AheadCame::Go,
        ),
        (
            "`merged`, another waiting in the column",
            asked("ready", "merged", &[on("shop-2", "ready", "")]),
            AheadCame::Go,
        ),
        (
            "`merged`, one worked from here put back in the backlog",
            asked("ready", "merged", &[on("shop-1", "backlog", "ready")]),
            AheadCame::Go,
        ),
        // Nor what is not this column's: put in doing by a person, or taken from another.
        (
            "`merged`, one a person put in doing",
            asked("ready", "merged", &[on("shop-1", "doing", "")]),
            AheadCame::Go,
        ),
        (
            "`merged`, one in review taken from another column",
            asked("ready", "merged", &[on("shop-1", "review", "planned")]),
            AheadCame::Go,
        ),
        // The item itself, sent back from review or in hand, waits on nobody.
        (
            "`merged`, the item itself on the board in doing",
            asked("ready", "merged", &[on("shop-3", "doing", "ready")]),
            AheadCame::Go,
        ),
        // Only an item waiting in the column is held back: one taken already goes on.
        ("`merged`, the item sent back to doing", asked("doing", "merged", &[in_review()]), AheadCame::Go),
        // Where items are handed straight to where they are finished, only one in hand holds it back.
        (
            "`merged`, handed on to done",
            Ahead { hands_to: "done".into(), ..asked("ready", "merged", &[on("shop-1", "done", "ready")]) },
            AheadCame::Go,
        ),
        (
            "`merged`, handed on to nowhere said",
            Ahead { hands_to: String::new(), ..asked("ready", "merged", &[on("shop-1", "doing", "ready")]) },
            behind(&["shop-1"], one),
        ),
        // Saying it already, it is not told again.
        (
            "`merged`, the item says so already",
            Ahead {
                waiting: format!("no work is being started here for now: {one}"),
                ..asked("ready", "merged", &[in_review()])
            },
            AheadCame::Behind { ids: vec!["shop-1".into()], why: one.into(), news: false },
        ),
        (
            "`merged`, the item says it waits its turn",
            Ahead {
                waiting: "one card is worked at a time here".into(),
                ..asked("ready", "merged", &[in_review()])
            },
            behind(&["shop-1"], one),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, meant) in cases {
        let came = ahead(&asked).unwrap_or_else(|why| panic!("{what}: refused, {why}"));
        assert_eq!(came, meant, "{what}");
        reached.insert(came.word());
    }
    assert_eq!(reached, AHEAD_WORDS.iter().copied().collect(), "every word it can answer has a case");
}

#[test]
fn what_is_not_said_or_not_a_word_of_the_row_is_refused_with_a_sentence() {
    let refused = |asked: Ahead| ahead(&asked).unwrap_err();
    assert!(
        refused(Ahead { item: String::new(), ..asked("ready", "merged", &[]) }).starts_with("`item`, `here`")
    );
    assert!(
        refused(Ahead { here: String::new(), ..asked("ready", "merged", &[]) }).starts_with("`item`, `here`")
    );
    assert!(
        refused(Ahead { doing: String::new(), ..asked("ready", "merged", &[]) })
            .starts_with("`item`, `here`")
    );
    assert_eq!(
        refused(asked("ready", "soon", &[])),
        "`next_after` says when the next card is taken where cards are worked: `handed`, as soon as the last is \
         handed on, or `merged`, once none worked from there waits unmerged; it says `soon`"
    );
}
