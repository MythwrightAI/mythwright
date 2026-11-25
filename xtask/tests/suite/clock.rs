//! How long each test took, worked out from what a test program printed.
//!
//! The lines here are written as a test program prints them, each with
//! when it arrived; nothing is run and no clock is read.

use xtask::clock::{Line, Run, Took, runs, whose};

fn lines(printed: &[(u64, &str)]) -> Vec<Line> {
    printed.iter().map(|(at, line)| (*at, line.to_string())).collect()
}

/// What each test of the one run took, by name.
fn took(run: &Run) -> Vec<(&str, u64)> {
    let mut took: Vec<(&str, u64)> = run.tests.iter().map(|took| (took.test.as_str(), took.millis)).collect();
    took.sort_unstable();
    took
}

#[test]
fn a_test_among_the_first_began_with_the_run_and_took_until_its_line() {
    let printed = lines(&[
        (100, "running 2 tests"),
        (350, "test board::a_card_is_moved ... ok"),
        (900, "test board::a_card_is_added ... ok"),
        (
            900,
            "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.80s",
        ),
    ]);
    let found = runs(&printed, 8);
    assert_eq!(found.len(), 1);
    assert_eq!(took(&found[0]), vec![("board::a_card_is_added", 800), ("board::a_card_is_moved", 250)]);
    assert_eq!(found[0].millis, 800);
}

#[test]
fn a_later_test_began_when_the_one_that_made_room_for_it_finished() {
    // Two at a time: `a` and `b` begin with the run, `c` when the first of
    // them finishes and `d` when the second does.
    let printed = lines(&[
        (100, "running 4 tests"),
        (300, "test b ... ok"),
        (500, "test a ... ok"),
        (600, "test c ... ok"),
        (900, "test d ... ok"),
        (
            900,
            "test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.80s",
        ),
    ]);
    assert_eq!(took(&runs(&printed, 2)[0]), vec![("a", 400), ("b", 200), ("c", 300), ("d", 400)]);
}

#[test]
fn tests_are_taken_in_the_order_of_their_names_whatever_order_they_finish_in() {
    // One at a time, each begins when the one named before it finished:
    // the lines come in that order, and every second is somebody's.
    let printed = lines(&[
        (0, "running 3 tests"),
        (40, "test access::first ... ok"),
        (100, "test access::second ... ok"),
        (1_100, "test fleets::third ... ok"),
        (
            1_100,
            "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.10s",
        ),
    ]);
    let found = runs(&printed, 1);
    assert_eq!(
        took(&found[0]),
        vec![("access::first", 40), ("access::second", 60), ("fleets::third", 1_000)]
    );
    assert_eq!(found[0].tests.iter().map(|took| took.millis).sum::<u64>(), found[0].millis);

    // Three at a time and finishing last first, each still began with the run.
    let printed =
        lines(&[(0, "running 3 tests"), (10, "test c ... ok"), (20, "test b ... ok"), (30, "test a ... ok")]);
    assert_eq!(took(&runs(&printed, 3)[0]), vec![("a", 30), ("b", 20), ("c", 10)]);
}

#[test]
fn each_program_is_a_run_of_its_own() {
    let printed = lines(&[
        (0, "running 1 test"),
        (200, "test kernel::an_id_is_unique ... ok"),
        (
            200,
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s",
        ),
        (210, ""),
        (5_000, "running 2 tests"),
        (5_300, "test journal::an_event_is_kept ... ok"),
        (5_900, "test journal::an_event_is_read ... ok"),
        (
            5_900,
            "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.90s",
        ),
    ]);
    let found = runs(&printed, 4);
    assert_eq!(found.len(), 2);
    assert_eq!(took(&found[0]), vec![("kernel::an_id_is_unique", 200)]);
    // The second began when it said so, and not when the first did.
    assert_eq!(took(&found[1]), vec![("journal::an_event_is_kept", 300), ("journal::an_event_is_read", 900)]);
    assert_eq!((found[0].millis, found[1].millis), (200, 900));
}

#[test]
fn a_test_that_failed_or_was_passed_over_is_timed_like_any_other() {
    let printed = lines(&[
        (0, "running 4 tests"),
        (0, "test a_left_out ... ignored"),
        (0, "test b_left_out_and_why ... ignored, needs a screen"),
        (700, "test c_wrong ... FAILED"),
        (800, "test d_meant_to_stop - should panic ... ok"),
        (
            800,
            "test result: FAILED. 1 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.80s",
        ),
    ]);
    assert_eq!(
        took(&runs(&printed, 4)[0]),
        vec![("a_left_out", 0), ("b_left_out_and_why", 0), ("c_wrong", 700), ("d_meant_to_stop", 800)]
    );
}

#[test]
fn what_is_printed_beside_the_tests_is_not_taken_for_one_finishing() {
    let printed = lines(&[
        (0, "   Compiling kernel v0.1.0"),
        (0, "test before_anything_ran ... ok"),
        (50, "running 1 test"),
        (60_050, "test slow has been running for over 60 seconds"),
        (61_000, "test slow ... ok"),
        (61_000, ""),
        (61_000, "failures:"),
        (61_000, "---- slow stdout ----"),
        (61_000, "test of the pipe ... not a result"),
        (
            61_000,
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 60.95s",
        ),
        (61_010, "test after_it_ended ... ok"),
    ]);
    let found = runs(&printed, 2);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].tests, vec![Took { test: "slow".into(), millis: 60_950 }]);
}

#[test]
fn a_program_that_stopped_before_saying_how_it_went_is_still_a_run() {
    let printed = lines(&[(10, "running 3 tests"), (40, "test a ... ok"), (90, "test b ... ok")]);
    let found = runs(&printed, 1);
    assert_eq!(found.len(), 1);
    assert_eq!(took(&found[0]), vec![("a", 30), ("b", 50)]);
    assert_eq!(found[0].millis, 80);
    // And nothing printed is no run at all.
    assert_eq!(runs(&[], 4), vec![]);
}

#[test]
fn told_no_threads_it_takes_the_tests_one_at_a_time() {
    let printed = lines(&[(0, "running 2 tests"), (5, "test a ... ok"), (9, "test b ... ok")]);
    assert_eq!(took(&runs(&printed, 0)[0]), vec![("a", 5), ("b", 4)]);
}

#[test]
fn each_run_cargo_made_is_the_crates_whose_program_it_said_it_was_running() {
    // What cargo says aside as it goes, with what it says of building.
    let aside = "\
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.31s
     Running unittests src/main.rs (target/debug/deps/factory-0a1b2c3d4e5f6a7b)
     Running tests/suite/main.rs (target/debug/deps/suite-1111111111111111)
     Running tests/suite/main.rs (target/debug/deps/suite-2222222222222222)
   Doc-tests kernel
";
    let programs = [
        ("api".to_string(), "/work/target/debug/deps/suite-2222222222222222".to_string()),
        ("conductor".to_string(), "/work/target/debug/deps/suite-1111111111111111".to_string()),
    ];
    // In the order cargo ran them, which is not the order they were
    // built in; what is no test program of a crate's is nobody's.
    assert_eq!(whose(aside, &programs), [None, Some("conductor".to_string()), Some("api".to_string()), None]);
}
