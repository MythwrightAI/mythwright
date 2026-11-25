//! How a program's tests are put in shares, and how many run at once.

use xtask::sharing::{Room, room, shares};

fn tests(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("module_{:02}::test_{index:04}", index / 20)).collect()
}

#[test]
fn every_test_is_in_exactly_one_share_whatever_the_numbers() {
    for processors in [1, 2, 4, 8, 12, 16, 20, 32, 64, 128] {
        for count in [0, 1, 63, 64, 65, 128, 129, 251, 1000] {
            let all = tests(count);
            let Some(shared) = shares(&all, room(processors)) else { continue };
            let mut found: Vec<String> = shared.iter().flatten().cloned().collect();
            found.sort();
            assert_eq!(found, all, "{count} tests on {processors} processors");
            // And no share is more than one test bigger than another.
            let sizes: Vec<usize> = shared.iter().map(Vec::len).collect();
            let (least, most) = (sizes.iter().min().unwrap(), sizes.iter().max().unwrap());
            assert!(most - least <= 1, "{count} tests on {processors} processors came to {sizes:?}");
        }
    }
}

#[test]
fn a_program_with_few_tests_is_run_whole_and_one_with_many_as_no_more_processes_than_there_is_room_for() {
    let room = room(32);
    assert_eq!(shares(&tests(64), room), None);
    assert_eq!(shares(&tests(65), room).map(|shares| shares.len()), Some(2));
    assert_eq!(shares(&tests(251), room).map(|shares| shares.len()), Some(4));
    assert_eq!(shares(&tests(5000), room).map(|shares| shares.len()), Some(4));
}

#[test]
fn the_tests_of_one_module_are_spread_between_the_shares() {
    // Twenty to a module, in the order of their names: no share is given a
    // module to itself.
    let shared = shares(&tests(251), room(32)).unwrap();
    for share in &shared {
        let of_the_first = share.iter().filter(|test| test.starts_with("module_00::")).count();
        assert_eq!(of_the_first, 5);
    }
}

#[test]
fn on_fewer_than_sixteen_processors_the_tests_run_as_cargo_runs_them() {
    for processors in [1, 2, 4, 8, 15] {
        assert_eq!(room(processors), Room { shares: 1, at_once: 1, threads: processors });
        assert_eq!(shares(&tests(1000), room(processors)), None);
    }
    // Told of none, it is still one.
    assert_eq!(room(0), Room { shares: 1, at_once: 1, threads: 1 });
}

#[test]
fn with_more_processors_processes_run_side_by_side_on_eight_threads_each_and_a_quarter_more_at_once() {
    assert_eq!(room(16), Room { shares: 2, at_once: 2, threads: 8 });
    assert_eq!(room(20), Room { shares: 2, at_once: 2, threads: 10 });
    assert_eq!(room(32), Room { shares: 4, at_once: 5, threads: 8 });
    assert_eq!(room(64), Room { shares: 8, at_once: 10, threads: 8 });
}
