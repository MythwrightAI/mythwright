//! What the suites that play wrote down, read as the tooling reads it.

use xtask::played::{Played, of, read};

fn one(krate: &str, test: &str, places: u64, plays: u64) -> Played {
    Played { krate: krate.into(), test: test.into(), places, plays }
}

#[test]
fn each_line_is_a_test_with_its_places_and_its_plays() {
    let read = read("conductor\t11\t42\tplays::steps_in_order\napi\t128\t35\tsimulated::a_card\n");
    assert_eq!(
        read,
        [one("conductor", "plays::steps_in_order", 11, 42), one("api", "simulated::a_card", 128, 35)]
    );
}

#[test]
fn a_test_that_played_two_things_is_both_of_them_together() {
    let read = read("conductor\t10\t30\tplays::two\nconductor\t4\t9\tplays::two\n");
    assert_eq!(read, [one("conductor", "plays::two", 14, 39)]);
}

#[test]
fn a_test_of_the_same_name_in_another_crate_is_another_test() {
    let read = read("conductor\t3\t7\twhose::heard\napi\t5\t6\twhose::heard\n");
    assert_eq!(read.len(), 2);
    assert_eq!(of(&read, "conductor"), (3, 7));
    assert_eq!(of(&read, "api"), (5, 6));
    assert_eq!(of(&read, "cli"), (0, 0));
}

#[test]
fn a_line_that_is_not_one_is_passed_over() {
    // Half a line, as a program stopped in the middle of writing leaves;
    // a count that is no number; and nothing at all.
    let read = read("conductor\t3\nconductor\tmany\t7\tplays::x\n\napi\t1\t2\ta::test\twith a tab in it\n");
    assert_eq!(read, [one("api", "a::test\twith a tab in it", 1, 2)]);
}
