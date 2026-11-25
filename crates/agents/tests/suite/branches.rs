//! Which of a project's branches moved, as git lists them: what is said
//! of a branch moved, made and taken away, and nothing of the rest.

use places::branches::{Heads, Moved, heads_of, moved};

fn heads(pairs: &[(&str, &str)]) -> Heads {
    pairs.iter().map(|(name, commit)| (name.to_string(), commit.to_string())).collect()
}

fn one(branch: &str, from: &str, to: &str) -> Moved {
    Moved { branch: branch.into(), from: from.into(), to: to.into() }
}

#[test]
fn a_branch_moved_made_or_taken_away_is_said_and_one_that_stayed_is_not() {
    let was = heads(&[("main", "a1"), ("lane/p-1", "b1"), ("old", "c1")]);
    let now = heads(&[("main", "a2"), ("lane/p-1", "b1"), ("new", "d1")]);
    assert_eq!(moved(&was, &now), vec![one("main", "a1", "a2"), one("new", "", "d1"), one("old", "c1", "")]);
    assert_eq!(moved(&now, &now), Vec::<Moved>::new());
    assert_eq!(moved(&Heads::new(), &heads(&[("main", "a1")])), vec![one("main", "", "a1")]);
}

#[test]
fn what_git_lists_is_read_a_branch_a_line_and_what_is_no_branch_left_out() {
    let listed = "main\ta1\nlane/p-1\tb1\n\nbroken line\n\tnothing\n";
    assert_eq!(heads_of(listed), heads(&[("main", "a1"), ("lane/p-1", "b1")]));
}
