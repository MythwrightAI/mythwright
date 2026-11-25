//! The Trash's plain values: how long, when a thing goes, where it goes back.

use kernel::trash::{DAY_MS, Keeping, MOST_DAYS, USUAL_DAYS, back_at, beside, days, due, goes};

#[test]
fn what_is_deleted_is_kept_thirty_days_where_nobody_said() {
    let usual = Keeping::default();
    assert_eq!(usual.days, Some(USUAL_DAYS));
    assert_eq!(USUAL_DAYS, 30);
    assert!(!usual.at_once());
    assert!(Keeping { days: Some(0), by: None, at: None }.at_once());
    assert!(!Keeping { days: None, by: None, at: None }.at_once());
}

#[test]
fn nought_days_and_for_ever_may_be_said_and_more_than_a_hundred_years_may_not() {
    assert_eq!(days(Some(0)).unwrap(), Some(0));
    assert_eq!(days(Some(7)).unwrap(), Some(7));
    assert_eq!(days(Some(MOST_DAYS)).unwrap(), Some(MOST_DAYS));
    assert_eq!(days(None).unwrap(), None);
    let refused = days(Some(MOST_DAYS + 1)).unwrap_err();
    assert!(refused.message.contains("always"), "{}", refused.message);
}

#[test]
fn a_thing_goes_when_its_days_are_over_and_never_where_it_is_kept_for_ever() {
    let at = 1_000;
    assert_eq!(goes(at, Some(30)), Some(at + 30 * DAY_MS));
    assert_eq!(goes(at, None), None);
    assert!(!due(at, Some(30), at + 30 * DAY_MS - 1));
    assert!(due(at, Some(30), at + 30 * DAY_MS));
    assert!(due(at, Some(0), at));
    assert!(!due(at, None, i64::MAX));
    assert_eq!(goes(i64::MAX - 5, Some(MOST_DAYS)), Some(i64::MAX));
}

#[test]
fn a_record_goes_back_to_its_place_or_after_the_last_where_that_is_taken() {
    assert_eq!(back_at(2.0, false, Some(5.0)), 2.0);
    assert_eq!(back_at(2.0, true, Some(5.0)), 6.0);
    assert_eq!(back_at(2.0, true, None), 1.0);
    assert_eq!(back_at(2.0, true, Some(-3.0)), 1.0);
}

#[test]
fn a_file_given_back_beside_another_is_called_restored_then_numbered() {
    let none = |_: &str| false;
    assert_eq!(beside("docs/notes.md", none), "docs/notes (restored).md");
    assert_eq!(beside("notes.md", none), "notes (restored).md");
    assert_eq!(beside("docs/pricing", none), "docs/pricing (restored)");
    assert_eq!(beside(".env", none), ".env (restored)");
    assert_eq!(beside("a/b.tar.gz", none), "a/b.tar (restored).gz");
    let taken = |path: &str| path == "notes (restored).md" || path == "notes (restored 2).md";
    assert_eq!(beside("notes.md", taken), "notes (restored 3).md");
}
