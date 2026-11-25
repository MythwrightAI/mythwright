//! Whether several items handed on together were all passed: a table of
//! cases that reaches every word it can answer, an item gone from the
//! table, one said twice, and what is refused.

use std::collections::{BTreeMap, BTreeSet};

use deciding::passed::{Came, passed};

fn named(items: &[&str]) -> Vec<String> {
    items.iter().map(|one| one.to_string()).collect()
}

fn at(rows: &[(&str, &str)]) -> BTreeMap<String, String> {
    rows.iter().map(|(id, here)| (id.to_string(), here.to_string())).collect()
}

/// What `items` come to on the table `rows`, items passing to `done` and
/// waiting in `staging`: the word, and the sentence.
/// A case: the items, where each is, the word and the sentence.
type Case = (&'static [&'static str], &'static [(&'static str, &'static str)], Came, &'static str);

fn came(items: &[&str], rows: &[(&str, &str)]) -> (Came, String) {
    let decided = passed(&named(items), &at(rows), "done", &named(&["staging"])).unwrap();
    (decided.came, decided.said())
}

#[test]
fn several_items_come_to_a_word_by_where_each_of_them_is() {
    let cases: &[Case] = &[
        (&["a", "b"], &[("a", "done"), ("b", "done")], Came::Passed, "All 2 were passed."),
        (&["a"], &[("a", "done")], Came::Passed, "It was passed."),
        (
            &["a", "b"],
            &[("a", "done"), ("b", "staging")],
            Came::Waiting,
            "1 of 2 passed so far; `b` still waits.",
        ),
        (
            &["a", "b", "c"],
            &[("a", "staging"), ("b", "staging"), ("c", "done")],
            Came::Waiting,
            "1 of 3 passed so far; `a`, `b` still wait.",
        ),
        // One sent back anywhere but where items wait is back, whatever
        // the others are.
        (&["a", "b"], &[("a", "done"), ("b", "ready")], Came::Back, "`b` went back to ready."),
        (&["a", "b"], &[("a", "staging"), ("b", "doing")], Came::Back, "`b` went back to doing."),
        // One the table has no row of can never pass.
        (&["a", "b"], &[("a", "done")], Came::Back, "`b` is gone."),
        (
            &["a", "b", "c"],
            &[("a", "ready"), ("c", "done")],
            Came::Back,
            "`a` went back to ready; `b` is gone.",
        ),
        // One said twice is counted once.
        (&["a", "a", "b"], &[("a", "done"), ("b", "done")], Came::Passed, "All 2 were passed."),
    ];
    let mut reached = BTreeSet::new();
    for (items, rows, word, sentence) in cases {
        let (got, said) = came(items, rows);
        assert_eq!((got, said.as_str()), (*word, *sentence), "{items:?} on {rows:?}");
        reached.insert(got.word());
    }
    let every: BTreeSet<&str> = Came::ALL.iter().map(|one| one.word()).collect();
    assert_eq!(reached, every, "a word no case reaches");
}

#[test]
fn what_each_item_is_said_to_be_is_listed_in_the_order_they_were_handed() {
    let rows = at(&[("c", "done"), ("a", "done"), ("b", "staging"), ("d", "ready")]);
    let decided = passed(&named(&["c", "a", "b", "d", "e"]), &rows, "done", &named(&["staging"])).unwrap();
    assert_eq!(decided.passed, named(&["c", "a"]));
    assert_eq!(decided.waiting, named(&["b"]));
    assert_eq!(decided.back, vec![("d".to_string(), "ready".to_string())]);
    assert_eq!(decided.gone, named(&["e"]));
    assert_eq!(decided.came, Came::Back);
}

#[test]
fn nothing_handed_nowhere_to_pass_to_and_one_column_said_both_ways_are_refused() {
    let rows = at(&[("a", "done")]);
    let waits = named(&["staging"]);
    let none = passed(&[], &rows, "done", &waits).unwrap_err();
    assert!(none.contains("none is handed"), "{none}");
    let nowhere = passed(&named(&["a"]), &rows, "", &waits).unwrap_err();
    assert!(nowhere.contains("`passes`"), "{nowhere}");
    let both = passed(&named(&["a"]), &rows, "staging", &waits).unwrap_err();
    assert!(both.contains("both"), "{both}");
}
