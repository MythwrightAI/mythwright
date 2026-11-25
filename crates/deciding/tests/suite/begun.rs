//! What is begun for an item where one thing is begun for each and heard
//! from, by the list it is in; what the things done about that one came
//! to where nothing is sent back; what is said of how much is done at
//! once, and of a place that was tidied away: each a function of what it
//! is handed, held by a table of cases.

use std::collections::BTreeSet;

use deciding::begun::left::{self, left};
use deciding::begun::tidied::{Tidied, tidied};
use deciding::begun::{ENDED_WORDS, Ended, First, ended, first};
use deciding::column::{Paced, paced};
use deciding::segments::{Asked, Said, Usual};
use serde_json::{Map, Value, json};

fn row(id: &str, order: i64, does: &str) -> Said {
    Said { id: id.into(), title: id.into(), order, does: does.into(), ..Said::default() }
}

/// An item whose usual list is a review that is begun and heard from,
/// and then what was approved put with the rest, with these rows.
fn asked(rows: Vec<Said>) -> Asked {
    Asked {
        rows,
        usual: vec![
            Usual { order: 10, does: "std/review".into(), spawned: true },
            Usual { order: 20, does: "std/merge".into(), spawned: false },
        ],
        item: "shop-7".into(),
        labels: vec!["payments".into()],
        ..Asked::default()
    }
}

fn own() -> Map<String, Value> {
    json!({ "role": "reviewer", "picks": "first", "seats": 2, "needed": 0 }).as_object().unwrap().clone()
}

/// What a case expects: whether the list is gone through, where what is
/// begun is, whether only what is usual comes after it, and who looks.
type Told<'a> = (bool, &'a str, i64, bool, Value);

fn told(first: &First) -> Told<'_> {
    (first.lists, first.row.as_str(), first.order, first.usual, Value::Object(first.with.clone()))
}

#[test]
fn what_is_begun_first_for_an_item_is_begun_at_once_unless_something_is_called_before_it() {
    let usual = json!({ "role": "reviewer", "picks": "first", "seats": 2, "needed": 0 });
    let through = (true, "", 0, false, json!({}));
    let held = |mut row: Said, label: &str| {
        row.labelled = label.into();
        row
    };
    let seated = |mut row: Said| {
        row.cells.insert("seats".into(), json!(3));
        row.cells.insert("role".into(), json!("auditor"));
        row
    };
    let cases: Vec<(&str, Asked, Told)> = vec![
        ("a row that is nobody's: the usual is begun", asked(vec![]), (false, "", 10, true, usual.clone())),
        (
            "something called before it: the list is gone through",
            asked(vec![row("tests", 5, "std/run")]),
            through.clone(),
        ),
        (
            "something called before it for items with another label: begun at once",
            asked(vec![held(row("tests", 5, "std/run"), "docs")]),
            (false, "", 10, true, usual.clone()),
        ),
        (
            "something called before it for items with this label: gone through",
            asked(vec![held(row("tests", 5, "std/run"), "payments")]),
            through.clone(),
        ),
        (
            "something laid after it only: begun at once, and more than the usual comes after",
            asked(vec![row("licences", 15, "std/run")]),
            (false, "", 10, false, usual.clone()),
        ),
        (
            "a row that stands in for it with seats of its own: begun as the row says",
            asked(vec![seated(row("payments", 10, "std/review"))]),
            (
                false,
                "payments",
                10,
                true,
                json!({ "role": "auditor", "picks": "first", "seats": 3, "needed": 0 }),
            ),
        ),
        (
            "a row that says of it what cannot be said: the going through says so",
            asked(vec![{
                let mut row = row("payments", 10, "std/review");
                row.with = vec!["fails=left".into()];
                row
            }]),
            through.clone(),
        ),
        (
            "a row that takes it out: nothing is begun, and the list says what comes",
            asked(vec![row("none", 10, "")]),
            through.clone(),
        ),
        (
            "said to be due for a second looking a row names: that one is begun",
            Asked { row: "second".into(), order: 12, ..asked(vec![seated(row("second", 12, "std/review"))]) },
            (
                false,
                "second",
                12,
                true,
                json!({ "role": "auditor", "picks": "first", "seats": 3, "needed": 0 }),
            ),
        ),
        (
            "said to be due from something called: gone through from there",
            Asked { row: "licences".into(), order: 15, ..asked(vec![row("licences", 15, "std/run")]) },
            (true, "licences", 15, false, json!({})),
        ),
    ];
    for (what, asked, came) in cases {
        let found = first(&asked, &own());
        assert_eq!(told(&found), came, "{what}");
    }
}

/// How a card's row says it came to be where it is, by its move of this count.
fn moved(count: u64) -> Vec<String> {
    vec!["ready".into(), "user".into(), String::new(), count.to_string()]
}

fn done(came: &str, says: &str) -> Ended {
    Ended {
        item: "shop-7".into(),
        found: true,
        status: "backlog".into(),
        here: "backlog".into(),
        pass: "ready".into(),
        came: came.into(),
        says: says.into(),
        ..Ended::default()
    }
}

#[test]
fn what_the_things_done_came_to_where_nothing_is_sent_back_is_on_left_or_nothing() {
    let cases: Vec<(&str, Ended, (&str, &str))> = vec![
        (
            "it goes on with what was said",
            done("on", "planned by shop-architect-1"),
            ("on", "planned by shop-architect-1"),
        ),
        ("no word is on", done("", ""), ("on", "")),
        (
            "nothing said by what was done after: what was begun said it",
            Ended { said: "planned by shop-architect-1".into(), ..done("on", "") },
            ("on", "planned by shop-architect-1"),
        ),
        ("nowhere to go on to", Ended { pass: String::new(), ..done("on", "") }, ("nothing", "")),
        ("moved meanwhile", Ended { status: "ready".into(), ..done("on", "") }, ("nothing", "")),
        ("gone", Ended { found: false, ..done("left", "x") }, ("nothing", "")),
        (
            "moved since whoever asks first looked, and here by another coming",
            Ended { had: moved(2), moved: moved(4), ..done("on", "planned") },
            ("nothing", ""),
        ),
        (
            "never moved when first looked, and moved since",
            Ended { moved: moved(1), ..done("left", "x") },
            ("nothing", ""),
        ),
        (
            "as it was when first looked",
            Ended { had: moved(2), moved: moved(2), ..done("on", "a") },
            ("on", "a"),
        ),
        ("the item was gone from under what was done", done("nothing", ""), ("nothing", "")),
        (
            "left, with the sentence",
            done("left", "shop-7: `lint` did not pass"),
            ("left", "shop-7: `lint` did not pass"),
        ),
        (
            "left, with nothing said",
            done("left", ""),
            ("left", "shop-7: it is left in backlog for a person."),
        ),
        (
            "sent back, where nothing is sent back from",
            done("back", "`lint` did not pass: no title"),
            (
                "left",
                "shop-7: what is done with a card in backlog sent it back, and nothing is sent back from there: it is \
                 left for a person. `lint` did not pass: no title",
            ),
        ),
        (
            "something more to begin",
            done("review", ""),
            (
                "left",
                "shop-7: a row of `segments` names something more to begin for a card in backlog once the first has \
                 done, and one thing is begun for a card there: it is left for a person.",
            ),
        ),
        (
            "the list could not be gone through",
            Ended { failed: true, why: "no such blueprint".into(), ..done("", "") },
            (
                "left",
                "shop-7: what is done with a card in backlog could not be gone through: no such blueprint",
            ),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, (word, says)) in cases {
        let found = ended(&asked).unwrap_or_else(|why| panic!("{what}: {why}"));
        assert_eq!(found, (word, says.to_string()), "{what}");
        reached.insert(found.0);
    }
    assert_eq!(reached, ENDED_WORDS.iter().copied().collect::<BTreeSet<_>>(), "a word no case reaches");
    assert_eq!(
        ended(&done("again", "")),
        Err("what was done with shop-7 in backlog came to `again`, which is none of `on`, `back`, `left` or `nothing`"
            .to_string())
    );
    assert!(ended(&Ended { here: String::new(), ..done("on", "") }).is_err());
}

#[test]
fn how_much_is_done_at_once_is_said_in_one_sentence_for_each_thing_heard() {
    let said = |topic: &str, reviews: bool| Paced {
        topic: topic.into(),
        column: "review".into(),
        reviews,
        why: "the remote is down".into(),
        limit: "2".into(),
    };
    let cases = [
        (
            said("pool.paused", true),
            "No new review is being started in review for now: the remote is down. What is under way goes on.",
        ),
        (
            said("pool.paused", false),
            "No new work is being started for now: the remote is down. What is under way goes on.",
        ),
        (said("pool.open", true), "Cards in review are reviewed 2 at a time from now."),
        (said("pool.open", false), "Cards in review are worked 2 at a time from now."),
    ];
    for (asked, says) in cases {
        assert_eq!(paced(&asked), Ok(says.to_string()));
    }
    assert_eq!(
        paced(&said("pool.asked", false)),
        Err("`topic` is what was said of how much is done at once, `pool.paused` or `pool.open`; it is `asked`"
            .to_string())
    );
}

#[test]
fn a_place_tidied_away_is_said_only_where_something_was_kept_or_could_not_be_done() {
    let came = |came: &str, why: &str| Tidied {
        item: "shop-7".into(),
        came: came.into(),
        why: why.into(),
        remote: "origin".into(),
        ..Tidied::default()
    };
    let cases = [
        (came("cleared", ""), ""),
        (came("nothing", ""), ""),
        (came("", ""), ""),
        (
            came("place_kept", "work-shop-7.txt was changed and never committed"),
            "shop-7: its worktree .lanes/shop-7 and its branch lane/shop-7 were kept, because the worktree could not \
             be removed without losing something: work-shop-7.txt was changed and never committed",
        ),
        (
            came("branch_kept", ""),
            "shop-7: its branch lane/shop-7 was kept, because it has work that is neither in the project's own \
             branch nor on origin",
        ),
        (
            came("untidied", "fatal: not a git repository"),
            "shop-7: its worktree and branch could not be tidied away: fatal: not a git repository",
        ),
        (
            came("untidied", ""),
            "shop-7: its worktree and branch could not be tidied away: the command failed",
        ),
        (
            Tidied { failed: true, error: "git could not be asked".into(), ..came("", "") },
            "shop-7: its worktree and branch could not be tidied away: git could not be asked",
        ),
    ];
    for (asked, says) in cases {
        assert_eq!(tidied(&asked), says, "{asked:?}");
    }
}

#[test]
fn what_was_left_in_hand_is_what_a_round_was_begun_for_and_said_nothing_more_of() {
    let row = |id: &str, status: &str, taken: &str, again: &[&str]| json!({ "id": id, "status": status, "taken": taken, "again": again });
    let items = vec![
        row("shop-1", "doing", "ready", &["heard", "2"]),
        row("shop-2", "review", "ready", &["heard", "2"]),
        row("shop-3", "doing", "ready", &["asked", ""]),
        row("shop-4", "doing", "ready", &["left", "it could not be committed"]),
        row("shop-5", "doing", "", &[]),
        row("shop-6", "doing", "queue", &["heard", "4"]),
        row("shop-7", "ready", "ready", &["heard", "1"]),
        row("shop-8", "done", "", &[]),
    ];
    let asked = |worked: &[&str]| left::Asked {
        items: items.clone(),
        doing: "doing".into(),
        worked: worked.iter().map(|one| one.to_string()).collect(),
    };
    // In hand from this column with a round begun: not one that asked or
    // was left, one somebody put there, another column's, or one that
    // waits its turn.
    assert_eq!(left(&asked(&["ready"])).given, ["shop-1"]);
    assert_eq!(left(&asked(&["ready", "queue"])).given, ["shop-1", "shop-6"]);
    // Nothing left: nothing is given.
    assert_eq!(left(&asked(&["elsewhere"])), left::Left::default());
    assert_eq!(left(&left::Asked { doing: String::new(), ..asked(&["ready"]) }).given, Vec::<String>::new());
}
