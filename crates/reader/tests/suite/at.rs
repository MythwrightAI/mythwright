//! A time of day a handler fires at, with its zone: what is read and what
//! is refused, and when it comes, through a change of the clocks.

use reader::at::read;

/// 2026-11-23 12:00 UTC, a Monday; Zagreb is an hour ahead in winter.
const MONDAY_NOON: i64 = 1_795_435_200_000;
const HOUR: i64 = 3_600_000;
const DAY: i64 = 24 * HOUR;

#[test]
fn a_time_is_read_with_its_zone_and_the_days_it_is_on_and_said_in_words() {
    let cases = [
        ("02:00 Europe/Zagreb", "every day at 02:00 in Europe/Zagreb"),
        ("mon 09:30 UTC", "on Monday at 09:30 in UTC"),
        (
            "mon,THU,sat 23:59 America/New_York",
            "on Monday, Thursday and Saturday at 23:59 in America/New_York",
        ),
        ("  00:00   UTC ", "every day at 00:00 in UTC"),
    ];
    for (written, said) in cases {
        assert_eq!(read(written).unwrap().said(), said, "{written}");
    }
}

#[test]
fn a_time_with_no_zone_or_that_is_no_time_is_refused_saying_how_one_is_written() {
    let cases = [
        (
            "02:00",
            "says no zone: write the zone the time is in after it, as `02:00 Europe/Zagreb` or `02:00 UTC`",
        ),
        ("2:00 UTC", "is no time of day"),
        ("24:00 UTC", "is no time of day"),
        ("02:60 UTC", "is no time of day"),
        ("noon UTC", "is no time of day"),
        ("02:00 Europe/Zgreb", "is no zone this machine knows"),
        ("fri,fri 02:00 UTC", "is said twice"),
        ("friday 02:00 UTC", "is no day"),
        ("", "is not a time of day"),
        ("mon 02:00 UTC extra", "is not a time of day"),
    ];
    for (written, why) in cases {
        let refused = read(written).unwrap_err();
        assert!(refused.contains(why), "{written}: {refused}");
    }
}

#[test]
fn the_next_time_comes_after_a_moment_and_never_at_it_on_the_days_it_is_on() {
    let daily = read("13:00 Europe/Zagreb").unwrap();
    // 12:00 UTC is 13:00 in Zagreb: that moment is not after itself.
    assert_eq!(daily.next_after(MONDAY_NOON), Some(MONDAY_NOON + DAY));
    assert_eq!(daily.next_after(MONDAY_NOON - 1), Some(MONDAY_NOON));
    assert_eq!(daily.last_upto(MONDAY_NOON), Some(MONDAY_NOON));
    assert_eq!(daily.last_upto(MONDAY_NOON - 1), Some(MONDAY_NOON - DAY));
    let thursdays = read("thu 12:00 UTC").unwrap();
    assert_eq!(thursdays.next_after(MONDAY_NOON), Some(MONDAY_NOON + 3 * DAY));
    assert_eq!(thursdays.last_upto(MONDAY_NOON), Some(MONDAY_NOON - 4 * DAY));
}

#[test]
fn a_time_in_a_zone_keeps_to_its_clock_as_the_clocks_change() {
    // 2027-03-28 is when Zagreb goes from an hour ahead to two.
    let before = 1_806_141_600_000; // 2027-03-27 10:00 UTC, 11:00 there
    let daily = read("11:00 Europe/Zagreb").unwrap();
    assert_eq!(daily.next_after(before), Some(before + DAY - HOUR));
    // The hour that is not there that night is the first moment after it.
    let gap = read("02:30 Europe/Zagreb").unwrap();
    let night = daily.next_after(before).unwrap() - 9 * HOUR; // 2027-03-28 00:00 UTC
    assert_eq!(gap.next_after(night), Some(night + HOUR + HOUR / 2));
}

#[test]
fn a_handler_goes_from_when_it_last_fired_at_the_same_time_from_now_at_another_and_from_its_making_the_first_time()
 {
    use reader::at::{begun, kept};
    let (now, made) = (MONDAY_NOON, MONDAY_NOON - HOUR);
    let same = kept("02:00 UTC", MONDAY_NOON - DAY);
    assert_eq!(begun(Some(&same), "02:00 UTC", now, Some(made)), (MONDAY_NOON - DAY, false));
    assert_eq!(begun(Some(&same), "03:00 UTC", now, Some(made)), (now, true));
    assert_eq!(begun(None, "02:00 UTC", now, Some(made)), (made, true));
    assert_eq!(begun(None, "02:00 UTC", now, None), (now, true));
}

#[test]
fn what_a_handler_says_with_holes_is_read_once_they_are_filled_and_refused_where_it_is_no_time() {
    use reader::at::filled;
    let scope = serde_json::json!({ "params": { "at": "thu 12:00 UTC", "n": 3 } });
    let scope = scope.as_object().unwrap();
    let (text, at) = filled("${params.at}", scope).unwrap();
    assert_eq!((text.as_str(), at.said().as_str()), ("thu 12:00 UTC", "on Thursday at 12:00 in UTC"));
    assert!(filled("${params.n}", scope).unwrap_err().contains("not text"));
    assert!(filled("${params.gone}", scope).is_err());
}
