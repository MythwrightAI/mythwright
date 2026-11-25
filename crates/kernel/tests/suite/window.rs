//! A table read a window at a time (`kernel::window`): the order its rows
//! come in, what a view of it shows, where a reading got to, how it is
//! read, and what is counted of it, case by case; the figures case for
//! case with the table the client reads too.

use std::collections::BTreeMap;

use kernel::window::figures::{Counting, figure, ordered_bands};
use kernel::window::read::{From, window};
use kernel::window::{
    Asked, Column, Key, Order, SMALL, Sort, Way, cursor, held, holds, key, mark, place_of, plan, shows,
};
use serde_json::{Map, Value, json};

fn columns() -> Vec<Column> {
    vec![
        Column::new("title", "text", &[]),
        Column::new("points", "number", &[]),
        Column::new("stage", "choice", &["draft", "ready", "done"]),
        Column::new("done", "flag", &[]),
        Column::new("labels", "list", &[]),
        Column::new("owner", "actor", &[]),
    ]
}

/// Rows, each its number and what it holds.
fn rows() -> Vec<(i64, Map<String, Value>)> {
    let data = |value: Value| value.as_object().cloned().unwrap();
    vec![
        (
            1,
            data(
                json!({ "title": "banana", "points": 3, "stage": "ready", "done": true, "labels": ["a", "b"] }),
            ),
        ),
        (2, data(json!({ "title": "Apple", "points": 10, "stage": "draft", "labels": ["b"] }))),
        (3, data(json!({ "title": "", "points": 3, "stage": "done", "done": false }))),
        (4, data(json!({ "title": "cherry", "stage": "gone", "labels": [] }))),
        (5, data(json!({ "title": "apple", "points": -1, "stage": "", "done": true, "labels": ["a"] }))),
        (6, data(json!({ "points": "", "stage": "ready", "owner": "user:ada" }))),
    ]
}

/// The numbers of the rows, in the order sorted as said, held to nothing.
fn sorted(sort: Option<(&str, bool)>) -> Vec<i64> {
    let sort = sort.map(|(field, down)| Sort { field: field.into(), down });
    let order = Order::of(&columns(), sort.as_ref()).unwrap();
    let rows = rows();
    let handed: Vec<(i64, &Map<String, Value>)> = rows.iter().map(|(number, data)| (*number, data)).collect();
    window(&order, &[], &handed, &From::Start, 100).rows.iter().map(|at| rows[*at].0).collect()
}

#[test]
fn a_view_nobody_sorts_is_read_by_the_number_each_row_was_given() {
    assert_eq!(sorted(None), [1, 2, 3, 4, 5, 6]);
}

#[test]
fn each_kind_is_ordered_by_what_it_holds_with_what_says_nothing_last_either_way() {
    // A number by its value, ties by number; nothing and empty text last.
    assert_eq!(sorted(Some(("points", false))), [5, 1, 3, 2, 4, 6]);
    // Descending turns the whole order round, ties among them; what says nothing stays last.
    assert_eq!(sorted(Some(("points", true))), [2, 3, 1, 5, 6, 4]);
    // Words by their words with capitals folded, ties by number: `Apple` and `apple` are one word.
    assert_eq!(sorted(Some(("title", false))), [2, 5, 1, 4, 3, 6]);
    assert_eq!(sorted(Some(("title", true))), [4, 1, 5, 2, 6, 3]);
    // A choice by where its option stands, what it no longer offers after, and what chose nothing last.
    assert_eq!(sorted(Some(("stage", false))), [2, 1, 6, 3, 4, 5]);
    assert_eq!(sorted(Some(("stage", true))), [3, 6, 1, 2, 4, 5]);
    // A tick: ticked ones, then the rest, as said nothing.
    assert_eq!(sorted(Some(("done", false))), [1, 5, 2, 3, 4, 6]);
    // A list by its things as words.
    assert_eq!(sorted(Some(("labels", false))), [5, 1, 2, 3, 4, 6]);
    // Somebody by who they are.
    assert_eq!(sorted(Some(("owner", false))), [6, 1, 2, 3, 4, 5]);
}

#[test]
fn a_column_is_keyed_as_the_database_keys_it() {
    let [title, points, _, done, labels, ..] = &columns()[..] else { unreachable!() };
    assert_eq!(key(title, Some(&json!("Émile"))), Key::Text("Émile".to_ascii_lowercase()));
    assert_eq!(key(title, Some(&json!(""))), Key::Null);
    assert_eq!(key(title, Some(&json!(12))), Key::Text("12".into()));
    assert_eq!(key(points, Some(&json!(4.5))), Key::Number(4.5));
    assert_eq!(key(points, None), Key::Null);
    assert_eq!(key(done, Some(&json!(false))), Key::Null);
    assert_eq!(key(done, Some(&json!(true))), Key::Number(1.0));
    assert_eq!(key(labels, Some(&json!(["B", "a"]))), Key::Text("b, a".into()));
    assert_eq!(key(labels, Some(&json!([]))), Key::Null);
    // Nothing, then numbers, then words by their bytes: `Émile` after `zulu`.
    let order =
        [Key::Null, Key::Number(-1.0), Key::Number(2.0), Key::Text("zulu".into()), Key::Text("émile".into())];
    for pair in order.windows(2) {
        assert!(pair[0].order(&pair[1]).is_lt(), "{pair:?}");
    }
    assert_eq!(Key::Text("it's".into()).sql(), "'it''s'");
    assert_eq!(Key::Number(3.0).sql(), "3");
    assert_eq!(Key::Number(4.5).sql(), "4.5");
}

#[test]
fn a_view_shows_the_rows_that_hold_one_of_what_it_is_held_to_in_each_column() {
    let columns = columns();
    let rows = rows();
    let shown = |only: Value, band: Option<(&str, &str)>| -> Vec<i64> {
        let only: BTreeMap<String, Vec<String>> = serde_json::from_value(only).unwrap();
        let held = held(&columns, &only, band);
        rows.iter().filter(|(_, data)| shows(&held, data)).map(|(number, _)| *number).collect()
    };
    assert_eq!(shown(json!({}), None), [1, 2, 3, 4, 5, 6]);
    assert_eq!(shown(json!({ "stage": ["ready"] }), None), [1, 6]);
    assert_eq!(shown(json!({ "labels": ["a"] }), None), [1, 5]);
    assert_eq!(shown(json!({ "labels": ["a"], "stage": ["ready", "draft"] }), None), [1]);
    // An empty list, and a column the table has not got, hold nothing back.
    assert_eq!(shown(json!({ "stage": [], "gone": ["x"] }), None), [1, 2, 3, 4, 5, 6]);
    // One band of a view gathered by a column, and the band of what names nothing.
    assert_eq!(shown(json!({}), Some(("labels", "b"))), [1, 2]);
    assert_eq!(shown(json!({}), Some(("labels", ""))), [3, 4, 6]);
    assert_eq!(holds(&columns[4], Some(&json!([" a ", "a", ""]))), ["a"]);
    assert_eq!(holds(&columns[3], Some(&json!(true))), ["yes"]);
}

#[test]
fn a_window_is_read_on_and_back_from_where_a_reading_got_to_and_from_a_rank() {
    let order = Order::of(&columns(), Some(&Sort { field: "points".into(), down: false })).unwrap();
    let rows = rows();
    let handed: Vec<(i64, &Map<String, Value>)> = rows.iter().map(|(number, data)| (*number, data)).collect();
    let numbers = |read: &kernel::window::read::Window| -> Vec<i64> {
        read.rows.iter().map(|at| rows[*at].0).collect()
    };
    let first = window(&order, &[], &handed, &From::Start, 2);
    assert_eq!(numbers(&first), [5, 1]);
    assert!(first.before.is_none());
    let next = window(&order, &[], &handed, &From::After(first.after.clone().unwrap()), 2);
    assert_eq!(numbers(&next), [3, 2]);
    let back = window(&order, &[], &handed, &From::Before(next.before.clone().unwrap()), 2);
    assert_eq!(numbers(&back), [5, 1]);
    let last = window(&order, &[], &handed, &From::After(next.after.clone().unwrap()), 2);
    assert_eq!(numbers(&last), [4, 6]);
    assert!(last.after.is_none(), "the end says so");
    let ranked = window(&order, &[], &handed, &From::At(3), 2);
    assert_eq!((numbers(&ranked), ranked.rank), (vec![2, 4], Some(3)));
}

#[test]
fn a_place_is_handed_back_as_it_was_and_is_no_place_in_another_reading() {
    let columns = columns();
    let sort = Sort { field: "points".into(), down: true };
    let order = Order::of(&columns, Some(&sort)).unwrap();
    let marked = mark("rows", &[], Some(&sort), &[]);
    let place = order.place(&rows()[1].1, 2);
    let text = cursor(&marked, &place);
    assert_eq!(place_of(&text, &marked).unwrap(), place);
    let other = mark("rows", &[], Some(&Sort { field: "points".into(), down: false }), &[]);
    let refused = place_of(&text, &other).unwrap_err();
    assert_eq!(
        refused.message,
        "the view was changed, or is sorted another way, since this was read: read it from the start"
    );
    assert!(place_of("not a place", &marked).is_err());
    let held_to = held(&columns, &BTreeMap::from([("stage".to_string(), vec!["ready".to_string()])]), None);
    assert_ne!(
        mark("rows", &held_to, Some(&sort), &[]),
        marked,
        "what a view is held to is part of its mark"
    );
}

#[test]
fn a_window_is_walked_by_an_index_and_a_large_table_is_never_sorted_in_a_statement() {
    let columns = columns();
    let sort = |field: &str| Sort { field: field.into(), down: false };
    let asked = |sort: Option<&Sort>, count: u64, indexed: &[String], held: &[kernel::window::Held]| {
        plan(&Asked { columns: &columns, held, sort, count, indexed })
    };
    let none: Vec<String> = Vec::new();
    // By number, at any size: the index every table has.
    let by_number = asked(None, SMALL * 10, &none, &[]).unwrap();
    assert_eq!((by_number.way, by_number.make.len()), (Way::Walk, 0));
    // A small table is sorted in the statement where nothing is indexed.
    let small = asked(Some(&sort("points")), SMALL - 1, &none, &[]).unwrap();
    assert_eq!((small.way, small.make), (Way::Sorted, Vec::<String>::new()));
    // A large one has the column indexed first, and is walked by it.
    let large = asked(Some(&sort("points")), SMALL, &none, &[]).unwrap();
    assert_eq!((large.way, large.make), (Way::Walk, vec!["points".to_string()]));
    // One indexed is walked at any size.
    let indexed = asked(Some(&sort("points")), 3, &["points".to_string()], &[]).unwrap();
    assert_eq!(indexed.way, Way::Walk);
    // What holds several things sorts a small table only.
    assert_eq!(asked(Some(&sort("labels")), 3, &none, &[]).unwrap().way, Way::Sorted);
    let refused = asked(Some(&sort("labels")), SMALL, &none, &[]).unwrap_err();
    assert!(
        refused.message.starts_with("`labels` holds several things, which no index keeps in order"),
        "{refused}"
    );
    // A view held to an indexed column may be gathered by it; one held to a list may not.
    let only = BTreeMap::from([("stage".to_string(), vec!["ready".to_string()])]);
    let held_to = held(&columns, &only, None);
    let gathered =
        asked(Some(&sort("points")), 3, &["stage".to_string(), "points".to_string()], &held_to).unwrap();
    assert_eq!(gathered.gather.unwrap().keys, [Key::Text("ready".into())]);
    let large = asked(None, SMALL, &none, &held_to).unwrap();
    assert_eq!(large.make, ["stage"], "a large table has a column it is held to indexed");
    let listed = held(&columns, &BTreeMap::from([("labels".to_string(), vec!["a".to_string()])]), None);
    assert!(asked(None, SMALL, &none, &listed).unwrap().gather.is_none());
    let unknown = asked(Some(&sort("nothing")), 3, &none, &[]).unwrap_err();
    assert!(unknown.message.starts_with("`nothing` is not a column of this table"), "{unknown}");
}

/// The table of figures the client reads too.
fn table() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ui/src/figurecases.json");
    let kept = std::fs::read_to_string(path).expect("the table of figures is in the client's sources");
    serde_json::from_str(&kept).expect("the table is a mapping of rows and cases")
}

#[test]
fn every_figure_comes_to_what_the_table_of_cases_the_client_is_held_to_says() {
    let table = table();
    let rows: Vec<Map<String, Value>> =
        table["rows"].as_array().unwrap().iter().map(|row| row.as_object().cloned().unwrap()).collect();
    let cases = table["cases"].as_array().unwrap();
    assert!(cases.len() > 20, "the table is there");
    for case in cases {
        let column = Column::new(case[0].as_str().unwrap(), case[1].as_str().unwrap(), &[]);
        let came = figure(&rows, &column, case[2].as_str().unwrap());
        assert_eq!(came, case[3].as_f64(), "{case}");
    }
}

#[test]
fn what_is_counted_of_a_view_is_of_the_rows_it_shows_and_its_bands_come_in_the_order_they_are_shown() {
    let columns = columns();
    let rows = rows();
    let only = BTreeMap::from([(
        "stage".to_string(),
        vec!["ready".to_string(), "draft".to_string(), "gone".to_string()],
    )]);
    let held_to = held(&columns, &only, None);
    let mut counting = Counting::default();
    for (_, data) in &rows {
        counting.take(&held_to, Some(&columns[2]), &[columns[1].clone()], data);
    }
    assert_eq!(counting.count, 4);
    assert_eq!(counting.tallies[0].figure("number", "sum"), Some(13.0));
    let (bands, more) = ordered_bands(&columns[2], &counting.bands);
    assert_eq!(bands, [("draft".to_string(), 1), ("ready".to_string(), 2), ("gone".to_string(), 1)]);
    assert_eq!(more, 0);
    // A record's band by the number its id ends in; what names nothing last.
    let named = Column::new("sprint", "record", &["sprints"]);
    let counts = BTreeMap::from([
        ("shop-sprints-10".to_string(), 1),
        ("shop-sprints-9".to_string(), 1),
        (String::new(), 2),
    ]);
    let (bands, _) = ordered_bands(&named, &counts);
    let keys: Vec<&str> = bands.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(keys, ["shop-sprints-9", "shop-sprints-10", ""]);
}

#[test]
fn a_search_finds_a_row_where_every_word_is_somewhere_in_the_columns_it_looks_through() {
    use kernel::window::found::{Find, kept_out, rank_of, words};
    let columns = columns();
    let rows = rows();
    let found = |typed: &str, hides: &[&str]| -> Vec<i64> {
        let hides: Vec<String> = hides.iter().map(|one| one.to_string()).collect();
        let find = Find::of(&columns, &hides, typed).unwrap();
        rows.iter().filter(|(_, data)| find.finds(data)).map(|(number, _)| *number).collect()
    };
    // Capitals folded, anywhere in the words: the middle of one.
    assert_eq!(found("PPL", &[]), [2, 5]);
    // Every word, each in any column: a number as its digits, a list's things.
    assert_eq!(found("apple 10", &[]), [2]);
    assert_eq!(found("a ready", &[]), [1, 6]);
    assert_eq!(found("ada", &[]), [6]);
    // A column the view leaves out, and a tick, are not looked through.
    assert_eq!(found("apple", &["title"]), Vec::<i64>::new());
    assert_eq!(found("yes", &[]), Vec::<i64>::new());
    assert!(Find::of(&columns, &[], "  ").is_none(), "nothing typed is no search");
    assert_eq!(words("Ab ab  cd"), ["ab", "cd"]);
    // What keeps a row out: each column it is held to, and the search.
    let held_to = held(&columns, &BTreeMap::from([("stage".to_string(), vec!["ready".to_string()])]), None);
    let find = Find::of(&columns, &[], "cherry").unwrap();
    assert_eq!(kept_out(&held_to, Some(&find), &rows[1].1), ["stage", "find"]);
    assert!(kept_out(&held_to, None, &rows[0].1).is_empty());
    // Where a row stands among those shown: how many come before it.
    let order = Order::of(&columns, Some(&Sort { field: "points".into(), down: false })).unwrap();
    let handed: Vec<(i64, &Map<String, Value>)> = rows.iter().map(|(number, data)| (*number, data)).collect();
    let place = order.place(&rows[2].1, 3);
    assert_eq!(rank_of(&order, &[], &handed, &place), 2, "after 5 and 1, which come before it");
    // From a place on, the row there among them.
    let from = window(&order, &[], &handed, &From::Of(place), 2);
    assert_eq!(from.rows.iter().map(|at| rows[*at].0).collect::<Vec<_>>(), [3, 2]);
}

#[test]
fn a_search_is_part_of_the_mark_of_a_reading_and_a_view_searched_for_nothing_is_marked_as_it_was() {
    let sort = Sort { field: "points".into(), down: true };
    let plain = mark("rows", &[], Some(&sort), &[]);
    assert_ne!(mark("rows", &[], Some(&sort), &["ab".to_string()]), plain);
    assert_eq!(plain, mark("rows", &[], Some(&sort), &[]));
}
