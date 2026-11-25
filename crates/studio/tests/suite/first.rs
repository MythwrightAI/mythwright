//! The rows a table starts with, added so that it is the same done twice
//! (`studio::first`, asked as `rows.start`): which of them are still to
//! be added, given what the table's rows are called. A table of cases.

use serde_json::{Map, Value, json};
use studio::first::owed;

fn rows(rows: &[Value]) -> Vec<Map<String, Value>> {
    rows.iter().map(|row| row.as_object().cloned().unwrap()).collect()
}

/// The rows a fleet's `agents` might start with.
fn three() -> Vec<Map<String, Value>> {
    rows(&[
        json!({ "title": "architect", "count": 1 }),
        json!({ "title": "worker", "count": 2 }),
        json!({ "title": "reviewer", "count": 0 }),
    ])
}

#[test]
fn which_of_the_rows_a_table_starts_with_are_still_to_be_added_is_said_by_what_its_rows_are_called() {
    // What the table's rows are called, and which of the three are then
    // still to be added, by their places.
    let cases: &[(&str, Value, &[usize])] = &[
        ("an empty table is given them all, in order", json!([]), &[0, 1, 2]),
        ("done a second time, nothing is added", json!(["architect", "worker", "reviewer"]), &[]),
        ("stopped after the first, the rest are", json!(["architect"]), &[1, 2]),
        ("stopped after the second, the last is", json!(["architect", "worker"]), &[2]),
        ("the order the table has them in says nothing", json!(["reviewer", "architect"]), &[1]),
        ("a row of the table's own is nobody's of these", json!(["tester", "architect"]), &[1, 2]),
        ("one somebody called something else is not found", json!(["architect", "builder"]), &[1, 2]),
        ("a number a row is called by is found as it is written", json!([7, "worker"]), &[0, 2]),
    ];
    for (what, there, still) in cases {
        let there = there.as_array().unwrap();
        assert_eq!(owed(there, &three(), "title").unwrap(), *still, "{what}");
    }
}

#[test]
fn rows_called_the_same_are_each_there_once_for_each_row_of_that_name_the_table_has() {
    let twice =
        rows(&[json!({ "title": "worker" }), json!({ "title": "worker" }), json!({ "title": "critic" })]);
    let cases: &[(&str, Value, &[usize])] = &[
        ("neither is there, and both are added", json!([]), &[0, 1, 2]),
        ("one is there, and the other is added", json!(["worker"]), &[1, 2]),
        ("both are there", json!(["worker", "critic", "worker"]), &[]),
        ("a third of the name is nobody's of these", json!(["worker", "worker", "worker"]), &[2]),
    ];
    for (what, there, still) in cases {
        assert_eq!(owed(there.as_array().unwrap(), &twice, "title").unwrap(), *still, "{what}");
    }
}

#[test]
fn rows_are_known_by_the_cell_that_is_said_and_one_that_says_nothing_there_is_refused() {
    let named = rows(&[
        json!({ "title": "work", "column": "ready" }),
        json!({ "title": "look", "column": "review" }),
    ]);
    assert_eq!(owed(&[json!("review")], &named, "column").unwrap(), [0]);

    // A row that could not be found again is not added to be added twice.
    for nothing in
        [json!({ "count": 1 }), json!({ "title": "" }), json!({ "title": "  " }), json!({ "title": null })]
    {
        let unnamed = rows(&[json!({ "title": "worker" }), nothing.clone()]);
        let refused = owed(&[], &unnamed, "title").unwrap_err();
        assert!(
            refused.contains("row 2 says nothing there") && refused.contains("`title`"),
            "{nothing}: {refused}"
        );
    }
}
