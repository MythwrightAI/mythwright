//! How long a card took, read from its history: a table of cases that
//! reaches every word it can answer, a card moved back and forth, one
//! with no history, one done twice, and the row that comes of each.

use std::collections::BTreeSet;

use deciding::timed::{Line, TIMED_WORDS, Timed, day, field, fields, row, timed};
use serde_json::{Value, json};

const MINUTE: i64 = 60_000;
const NOON: i64 = 1_795_435_200_000; // 2026-11-23 12:00 UTC

fn columns() -> Vec<String> {
    ["backlog", "ready", "doing", "review", "done"].iter().map(|one| one.to_string()).collect()
}

fn added(at: i64, status: &str) -> Line {
    Line {
        topic: "board.item.created".into(),
        ts: NOON + at * MINUTE,
        actor: "user:ada".into(),
        who: "Ada".into(),
        data: json!({ "status": status }),
    }
}

/// A move at `at` minutes, by a person, or through a blueprint where `through` says one.
fn moved(at: i64, from: &str, to: &str, by: &str, through: Option<&str>) -> Line {
    let mut data = json!({ "from": from, "to": to });
    if let Some(blueprint) = through {
        data["through"] = json!(blueprint);
    }
    let who = by.split(':').nth(1).unwrap_or_default().to_string();
    Line { topic: "board.item.moved".into(), ts: NOON + at * MINUTE, actor: by.into(), who, data }
}

fn worked(at: i64, agent: &str) -> Line {
    Line {
        topic: "card.worked".into(),
        ts: NOON + at * MINUTE,
        actor: "user:ada".into(),
        who: "the bot".into(),
        data: json!({ "agent": agent }),
    }
}

fn other(at: i64, topic: &str, data: Value) -> Line {
    Line { topic: topic.into(), ts: NOON + at * MINUTE, actor: "user:ada".into(), who: "Ada".into(), data }
}

/// How long it was in each column, in minutes, the board's in order.
fn spent(timed: &Timed) -> Vec<(String, i64)> {
    timed.spent.iter().map(|(column, ms)| (column.clone(), ms / MINUTE)).collect()
}

fn minutes(of: &[(&str, i64)]) -> Vec<(String, i64)> {
    of.iter().map(|(column, at)| (column.to_string(), *at)).collect()
}

#[test]
fn how_long_a_card_took_is_read_from_its_history() {
    let fleet = Some("std/working");
    // What, the history, and the word, whether it says when it was added,
    // minutes from added to done, each column's minutes, sent back, came to
    // the last column, and who worked it.
    type Case<'a> = (&'a str, Vec<Line>, &'a str, bool, i64, Vec<(String, i64)>, i64, i64, Vec<&'a str>);
    let cases: Vec<Case> = vec![
        (
            "straight through: each column once, its agent worked it",
            vec![
                added(0, "backlog"),
                moved(10, "backlog", "ready", "user:ada", None),
                moved(15, "ready", "doing", "user:ada", fleet),
                worked(40, "shop-worker-1"),
                moved(40, "doing", "review", "user:ada", fleet),
                moved(55, "review", "done", "user:ada", fleet),
            ],
            "done",
            true,
            55,
            minutes(&[("backlog", 10), ("ready", 5), ("doing", 25), ("review", 15), ("done", 0)]),
            0,
            1,
            vec!["shop-worker-1"],
        ),
        (
            "moved back and forth: each time in a column added up, each move back counted",
            vec![
                added(0, "ready"),
                moved(5, "ready", "doing", "user:ada", fleet),
                worked(20, "shop-worker-1"),
                moved(20, "doing", "review", "user:ada", fleet),
                moved(30, "review", "doing", "user:ada", fleet),
                worked(50, "shop-worker-2"),
                moved(50, "doing", "review", "user:ada", fleet),
                moved(60, "review", "ready", "user:ada", None),
                moved(62, "ready", "doing", "user:grace", None),
                moved(70, "doing", "review", "user:grace", None),
                moved(80, "review", "done", "user:ada", None),
            ],
            "done",
            true,
            80,
            minutes(&[("backlog", 0), ("ready", 7), ("doing", 43), ("review", 30), ("done", 0)]),
            2,
            1,
            vec!["shop-worker-1", "shop-worker-2", "grace"],
        ),
        (
            "done twice: the time in done between counted, and to its last coming there",
            vec![
                added(0, "doing"),
                moved(10, "doing", "done", "user:ada", None),
                moved(30, "done", "doing", "user:ada", None),
                moved(45, "doing", "done", "user:ada", None),
            ],
            "done",
            true,
            45,
            minutes(&[("backlog", 0), ("ready", 0), ("doing", 25), ("review", 0), ("done", 20)]),
            1,
            2,
            vec!["ada"],
        ),
        (
            "not done: where it is is still counted",
            vec![added(0, "backlog"), moved(10, "backlog", "doing", "user:ada", None)],
            "on",
            true,
            0,
            minutes(&[("backlog", 10), ("ready", 0), ("doing", 0), ("review", 0), ("done", 0)]),
            0,
            0,
            vec!["ada"],
        ),
        (
            "done once and taken out again: on",
            vec![
                added(0, "doing"),
                moved(10, "doing", "done", "user:ada", None),
                moved(20, "done", "review", "user:ada", None),
            ],
            "on",
            true,
            0,
            minutes(&[("backlog", 0), ("ready", 0), ("doing", 10), ("review", 0), ("done", 10)]),
            1,
            1,
            vec![],
        ),
        (
            "added straight to the last column: done as it was added, in no time",
            vec![added(0, "done")],
            "done",
            true,
            0,
            minutes(&[("backlog", 0), ("ready", 0), ("doing", 0), ("review", 0), ("done", 0)]),
            0,
            1,
            vec![],
        ),
        ("no history", vec![], "unread", false, 0, vec![], 0, 0, vec![]),
        (
            "only what is said on it: nothing of where it was",
            vec![other(5, "record.commented", json!({ "body": "hm" }))],
            "unread",
            false,
            0,
            vec![],
            0,
            0,
            vec![],
        ),
        (
            "a history from before its board kept its adding: begun at the first move",
            vec![
                moved(30, "ready", "doing", "user:ada", fleet),
                moved(50, "doing", "done", "user:ada", fleet),
            ],
            "done",
            false,
            20,
            minutes(&[("backlog", 0), ("ready", 0), ("doing", 20), ("review", 0), ("done", 0)]),
            0,
            1,
            vec![],
        ),
        (
            "in the Trash for a while: that time in no column",
            vec![
                added(0, "doing"),
                other(10, "board.item.removed", json!({})),
                other(100, "board.item.created", json!({ "status": "doing", "restored": true })),
                moved(110, "doing", "done", "agent:shop-worker-1", None),
            ],
            "done",
            true,
            110,
            minutes(&[("backlog", 0), ("ready", 0), ("doing", 20), ("review", 0), ("done", 0)]),
            0,
            1,
            vec![],
        ),
        (
            "a column the board no longer has: kept after the board's",
            vec![
                added(0, "triage"),
                moved(15, "triage", "doing", "user:ada", None),
                moved(20, "doing", "done", "user:ada", None),
            ],
            "done",
            true,
            20,
            minutes(&[
                ("backlog", 0),
                ("ready", 0),
                ("doing", 5),
                ("review", 0),
                ("done", 0),
                ("triage", 15),
            ]),
            0,
            1,
            vec!["ada"],
        ),
    ];
    let mut reached = BTreeSet::new();
    for (case, lines, came, whole, total, each, back, times, by) in cases {
        let got = timed(&lines, &columns(), "doing").unwrap();
        let took = if got.came == "done" { (got.done - got.added) / MINUTE } else { 0 };
        let worked: Vec<&str> = got.worked.iter().map(String::as_str).collect();
        assert_eq!(
            (got.came, got.whole, took, spent(&got), got.back, got.reached, worked),
            (came, whole, total, each, back, times, by),
            "{case}"
        );
        reached.insert(got.came);
    }
    assert_eq!(reached, TIMED_WORDS.iter().copied().collect());
    let none = timed(&[added(0, "ready")], &[], "doing").unwrap_err();
    assert!(none.contains("none is handed"), "{none}");
}

#[test]
fn what_a_card_took_is_kept_as_a_row_of_the_columns_its_table_keeps() {
    let lines = vec![
        added(0, "backlog"),
        moved(90, "backlog", "doing", "user:ada", None),
        moved(150, "doing", "review", "user:ada", None),
        moved(165, "review", "done", "user:ada", None),
    ];
    let got = timed(&lines, &columns(), "doing").unwrap();
    let kept = row(&got, "shop-4", "Make the cart", &columns());
    let wanted = json!({
        "card": "shop-4", "title": "Make the cart", "added": "2026-11-23", "done": "2026-11-23", "total": 165.0,
        "in_backlog": 90.0, "in_ready": 0.0, "in_doing": 60.0, "in_review": 15.0,
        "back": 0, "reached": 1, "worked": ["ada"], "whole": true,
    });
    assert_eq!(Value::Object(kept.clone()), wanted);
    // Every field the row says is one its table declares.
    let declared = fields(&columns()).unwrap();
    assert!(kept.keys().all(|key| declared.contains_key(key)), "{kept:?} against {declared:?}");
    assert!(!declared.contains_key("in_done"), "nothing is kept of the time a card has been done");
    // A tenth of a minute, and a day in the clock's own reckoning.
    let quick = timed(
        &[added(0, "doing"), Line { ts: NOON + 4_500, ..moved(0, "doing", "done", "user:ada", None) }],
        &columns(),
        "doing",
    )
    .unwrap();
    assert_eq!(row(&quick, "shop-5", "", &columns())["total"], json!(0.1));
    assert_eq!(day(0), "1970-01-01");
    assert_eq!(day(NOON), "2026-11-23");
    assert_eq!(day(951_782_400_000), "2000-02-29");
    assert_eq!(field("In Review"), "in_in_review");
    let short = fields(&["done".to_string()]).unwrap_err();
    assert!(short.contains("two columns or more"), "{short}");
    let twice = fields(&["to do".to_string(), "to-do".to_string(), "done".to_string()]).unwrap_err();
    assert!(twice.contains("both kept as `in_to_do`"), "{twice}");
}
