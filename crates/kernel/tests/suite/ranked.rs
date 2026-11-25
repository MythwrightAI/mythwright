//! Where a word of a choice stands among the others as a number, and what
//! a number written before it was a choice comes to (`kernel::ranked`).

use kernel::ranked::{rank, word_of};
use serde_json::{Value, json};

fn words(of: &[&str]) -> Vec<String> {
    of.iter().map(|one| one.to_string()).collect()
}

const PRIORITY: &[&str] = &["urgent", "high", "normal", "low"];

#[test]
fn a_word_ranks_by_where_it_stands_the_first_the_highest_and_nothing_where_the_usual_word_does() {
    let of = words(PRIORITY);
    // What each comes to, as a table: what is chosen, what nothing stands
    // for, the rank.
    let cases: &[(Value, Option<&str>, i64)] = &[
        (json!("urgent"), Some("normal"), 4),
        (json!("high"), Some("normal"), 3),
        (json!("normal"), Some("normal"), 2),
        (json!("low"), Some("normal"), 1),
        (Value::Null, Some("normal"), 2),
        (json!(""), Some("normal"), 2),
        // A word the choice does not offer is taken for nothing chosen.
        (json!("whenever"), Some("normal"), 2),
        // With no word for nothing, nothing is below every word.
        (Value::Null, None, 0),
        (json!("low"), None, 1),
        // A word for nothing the choice does not offer is no word.
        (Value::Null, Some("never"), 0),
        // A number from before ranks as the word it comes to.
        (json!(2), Some("normal"), 4),
        (json!(1), Some("normal"), 3),
        (json!(0), Some("normal"), 2),
        (json!(-4), Some("normal"), 1),
        (json!(3), None, 0),
        (json!(true), Some("normal"), 2),
    ];
    for (chosen, unsaid, ranked) in cases {
        assert_eq!(rank(chosen, &of, *unsaid), *ranked, "{chosen} with {unsaid:?} for nothing");
    }
}

#[test]
fn a_choice_put_in_another_order_ranks_by_that_order() {
    let of = words(&["low", "normal", "high", "urgent"]);
    assert_eq!(rank(&json!("low"), &of, Some("normal")), 4);
    assert_eq!(rank(&json!("urgent"), &of, Some("normal")), 1);
    assert_eq!(rank(&Value::Null, &of, Some("normal")), 3);
}

#[test]
fn a_number_from_before_comes_to_the_word_as_far_from_the_usual_one_as_it_was_from_nought() {
    let of = words(PRIORITY);
    let cases: &[(f64, Option<&str>)] = &[
        (0.0, Some("normal")),
        (1.0, Some("high")),
        (0.5, Some("high")),
        (2.0, Some("urgent")),
        (9.0, Some("urgent")),
        (1e300, Some("urgent")),
        (-1.0, Some("low")),
        (-0.25, Some("low")),
        (-7.0, Some("low")),
        (-1e300, Some("low")),
    ];
    for (number, word) in cases {
        assert_eq!(word_of(*number, &of, Some("normal")).as_deref(), *word, "{number}");
    }
    // With no word for nothing a number has nowhere to be counted from.
    assert_eq!(word_of(1.0, &of, None), None);
    assert_eq!(word_of(1.0, &of, Some("never")), None);
    // Counted from the first or the last, it goes no further than either.
    assert_eq!(word_of(1.0, &of, Some("urgent")).as_deref(), Some("urgent"));
    assert_eq!(word_of(-1.0, &of, Some("urgent")).as_deref(), Some("high"));
    assert_eq!(word_of(-1.0, &of, Some("low")).as_deref(), Some("low"));
}
