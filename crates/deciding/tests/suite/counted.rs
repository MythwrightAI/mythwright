//! What so many agents of one name are called, and which of those there
//! were are no longer counted (`deciding::counted`, asked by `std/role` as
//! `agent.counted`): a table of what is handed and what it comes to.

use deciding::counted::{Counted, counted};

fn names(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| word.to_string()).collect()
}

#[test]
fn those_counted_are_numbered_from_one_and_those_past_the_count_or_under_an_old_name_leave() {
    // name, what stands for it, count; how many there were, under what; and
    // what it comes to: what they are called, who is counted, who leaves.
    type Case = (
        (&'static str, &'static str, i64, i64, &'static str),
        (&'static str, &'static [&'static str], &'static [&'static str]),
    );
    let cases: [Case; 14] = [
        // Brought up for the first time: nothing was kept.
        (("", "worker", 2, 0, ""), ("worker", &["worker-1", "worker-2"], &[])),
        (("", "worker", 1, 0, ""), ("worker", &["worker-1"], &[])),
        // A name of their own stands before the role's word.
        (("otter", "reviewer", 2, 0, ""), ("otter", &["otter-1", "otter-2"], &[])),
        // None is a count: nobody is brought up, and nobody leaves where nobody was.
        (("", "worker", 0, 0, ""), ("worker", &[], &[])),
        // As many as there were, more, and fewer: those past the count leave.
        (("", "worker", 3, 3, "worker"), ("worker", &["worker-1", "worker-2", "worker-3"], &[])),
        (("", "worker", 3, 1, "worker"), ("worker", &["worker-1", "worker-2", "worker-3"], &[])),
        (("", "worker", 1, 3, "worker"), ("worker", &["worker-1"], &["worker-2", "worker-3"])),
        (("", "worker", 0, 2, "worker"), ("worker", &[], &["worker-1", "worker-2"])),
        // One that kept how many and not under what name (from before the
        // name was kept) had them under the name they have now.
        (("", "worker", 1, 3, ""), ("worker", &["worker-1"], &["worker-2", "worker-3"])),
        // Called something else: all under the old name leave, and as many come up under the new.
        (
            ("fast", "worker", 2, 3, "worker"),
            ("fast", &["fast-1", "fast-2"], &["worker-1", "worker-2", "worker-3"]),
        ),
        (("", "worker", 2, 2, "fast"), ("worker", &["worker-1", "worker-2"], &["fast-1", "fast-2"])),
        (("fast", "worker", 0, 1, "slow"), ("fast", &[], &["slow-1"])),
        // Another name with nobody under the old one: nobody leaves.
        (("fast", "worker", 1, 0, "worker"), ("fast", &["fast-1"], &[])),
        // The name it has, given as its name where the role's word was used before.
        (("worker", "worker", 1, 2, "worker"), ("worker", &["worker-1"], &["worker-2"])),
    ];
    for ((name, otherwise, count, had, called), (named, counted_now, past)) in cases {
        let comes_to = Counted { named: named.to_string(), names: names(counted_now), past: names(past) };
        assert_eq!(
            counted(name, otherwise, count, had, called),
            Ok(comes_to),
            "{count} of `{name}` or `{otherwise}`, where {had} were `{called}`"
        );
    }
}

#[test]
fn a_count_that_makes_no_sense_and_a_name_that_says_nothing_are_refused_in_a_sentence() {
    type Handed<'a> = (&'a str, &'a str, i64, i64, &'a str);
    let cases: [(Handed<'_>, &str); 6] = [
        (
            ("", "", 1, 0, ""),
            "what they are called is `name`, or `role` where no name is given, and neither says anything",
        ),
        (
            ("", "worker", -1, 0, ""),
            "`count` is how many agents there are in the role, and is not less than none; it is -1.",
        ),
        (
            ("", "worker", 1, -2, "worker"),
            "`had` is how many there were, and is not less than none; it is -2",
        ),
        (("", "worker", 1001, 0, ""), "there are 1000 of one name at most, and 1001 were asked for"),
        (("", "worker", 1, 5000, "worker"), "there are 1000 of one name at most, and 5000 were asked for"),
        // The most there may be is taken.
        (("", "worker", 1000, 1000, "worker"), ""),
    ];
    for ((name, otherwise, count, had, called), why) in cases {
        match counted(name, otherwise, count, had, called) {
            Err(refused) => assert_eq!(refused, why, "{count} of `{name}` or `{otherwise}`"),
            Ok(taken) => {
                assert_eq!(why, "", "{count} of `{name}` or `{otherwise}` was taken");
                assert_eq!((taken.names.len(), taken.past.len()), (1000, 0));
            }
        }
    }
}
