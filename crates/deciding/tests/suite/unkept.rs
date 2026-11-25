//! What becomes of work that could not be kept (`deciding::unkept`, asked by
//! `std/keep` as `place.unkept`): a table of what keeping answered
//! and what that comes to, with the sentence that says so.

use deciding::unkept::{Came, next};

const UNNAMED: &str = "Git does not know who to commit as, which no agent can put right: tell it in the project \
                       (git config user.name, and git config user.email). The card is left in doing for a person.";

#[test]
fn work_that_was_not_kept_is_given_again_a_number_of_times_and_then_left_for_a_person() {
    // What git said was in the way, whether it knows who commits, how often
    // the work was given back and how often it may be, and who did it; and
    // what that comes to.
    type Case = ((&'static str, bool, i64, i64, &'static str), (Came, &'static str));
    let cases: [Case; 13] = [
        // Something in the work's way, which whoever did it can put right.
        (
            ("a hook refused it", true, 0, 2, "shop-worker-1"),
            (Came::Again, "shop-worker-1 is given the card again, to put that right."),
        ),
        (
            ("a hook refused it", true, 1, 2, "shop-worker-1"),
            (Came::Again, "shop-worker-1 is given the card again, to put that right."),
        ),
        (
            ("a hook refused it", true, 0, 1, ""),
            (Came::Again, "Whoever worked on it is given the card again, to put that right."),
        ),
        // Given back as often as it may be: left, saying how often.
        (
            ("a hook refused it", true, 2, 2, "shop-worker-1"),
            (Came::Left, "It is left in doing for a person, having been given back 2 times for it."),
        ),
        (
            ("a hook refused it", true, 1, 1, "shop-worker-1"),
            (Came::Left, "It is left in doing for a person, having been given back 1 time for it."),
        ),
        (
            ("a hook refused it", true, 5, 2, "shop-worker-1"),
            (Came::Left, "It is left in doing for a person, having been given back 5 times for it."),
        ),
        // Never to be given back: left the first time.
        (
            ("a hook refused it", true, 0, 0, "shop-worker-1"),
            (Came::Left, "It is left in doing for a person."),
        ),
        // Keeping failed some other way, with nothing said of a commit: nobody is asked again.
        (("", true, 0, 2, "shop-worker-1"), (Came::Left, "It is left in doing for a person.")),
        (
            ("", true, 1, 2, "shop-worker-1"),
            (Came::Left, "It is left in doing for a person, having been given back 1 time for it."),
        ),
        // Git has nobody to commit as: no agent's to put right, however few the tries.
        (("nobody to commit as", false, 0, 2, "shop-worker-1"), (Came::Unnamed, UNNAMED)),
        (("nobody to commit as", false, 2, 2, "shop-worker-1"), (Came::Unnamed, UNNAMED)),
        (("nobody to commit as", false, 0, 0, ""), (Came::Unnamed, UNNAMED)),
        (("", false, 0, 2, "shop-worker-1"), (Came::Unnamed, UNNAMED)),
    ];
    let mut reached = Vec::new();
    for ((unkept, named, tries, most, by), (came, says)) in cases {
        let next = next(unkept, named, tries, most, by).unwrap();
        assert_eq!(
            (next.came, next.says.as_str()),
            (came, says),
            "{unkept:?} {named} {tries} of {most} by {by:?}"
        );
        reached.push(next.came);
    }
    for word in Came::ALL {
        assert!(reached.contains(word), "no case reaches `{}`", word.word());
    }
}

#[test]
fn a_number_of_times_below_none_is_refused_in_a_sentence() {
    assert_eq!(
        next("x", true, -1, 2, "a").unwrap_err(),
        "`tries` is a number of times, and is not less than none; it is -1"
    );
    assert_eq!(
        next("x", true, 0, -2, "a").unwrap_err(),
        "`most` is a number of times, and is not less than none; it is -2"
    );
}
