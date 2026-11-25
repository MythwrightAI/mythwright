//! A repository's branches as a person chooses one, read from what git
//! says of them, and what a fetch brought.

use kernel::git::{Branch, branches, fetched};

/// A line as git answers `kernel::git::BRANCHES`.
fn line(refname: &str, commit: &str, at: i64, subject: &str) -> String {
    format!("{refname}\0{commit}\0{at}\0{subject}")
}

fn named(found: &[Branch]) -> Vec<&str> {
    found.iter().map(|branch| branch.name.as_str()).collect()
}

#[test]
fn local_branches_come_first_and_then_each_remotes_by_name_with_what_its_head_points_at_left_out() {
    let said = [
        line("refs/remotes/origin/dev", "b2", 1_700_000_100, "dev, as origin has it"),
        line("refs/heads/main", "a1", 1_700_000_000, "the first"),
        line("refs/remotes/origin/HEAD", "a1", 1_700_000_000, "the first"),
        line("refs/heads/dev", "b1", 1_700_000_050, "dev here"),
        line("refs/remotes/origin/feature/cart", "c1", 1_700_000_200, "a branch with a slash"),
    ]
    .join("\n");
    let found = branches(&said);
    assert_eq!(named(&found), ["dev", "main", "origin/dev", "origin/feature/cart"]);
    let theirs = &found[2];
    assert_eq!(theirs.remote.as_deref(), Some("origin"));
    assert_eq!(theirs.refname, "refs/remotes/origin/dev");
    assert_eq!((theirs.commit.as_str(), theirs.at), ("b2", 1_700_000_100_000), "in milliseconds");
    assert_eq!(found[0].remote, None);
}

#[test]
fn a_subject_is_taken_whole_and_a_line_that_does_not_read_as_a_branch_is_left_out() {
    let said = [
        line("refs/heads/dev", "b1", 1, "a subject\twith a tab: and a colon"),
        "not a branch at all".to_string(),
        "refs/heads/x\0c1\0soon\0no time".to_string(),
        line("refs/tags/v1", "d1", 1, "a tag is no branch"),
        line("refs/heads/", "e1", 1, "no name"),
        String::new(),
    ]
    .join("\n");
    let found = branches(&said);
    assert_eq!(named(&found), ["dev"]);
    assert_eq!(found[0].subject, "a subject\twith a tab: and a colon");
    assert!(branches("").is_empty(), "a repository with no commit has no branch");
}

#[test]
fn a_fetch_says_which_of_the_remotes_branches_are_new_moved_on_or_gone_and_nothing_of_local_ones() {
    let before = branches(
        &[
            line("refs/heads/dev", "l1", 1, ""),
            line("refs/remotes/origin/dev", "a1", 1, ""),
            line("refs/remotes/origin/old", "o1", 1, ""),
            line("refs/remotes/origin/main", "m1", 1, ""),
        ]
        .join("\n"),
    );
    let after = branches(
        &[
            line("refs/heads/dev", "l2", 1, ""),
            line("refs/remotes/origin/dev", "a2", 2, ""),
            line("refs/remotes/origin/main", "m1", 1, ""),
            line("refs/remotes/origin/new", "n1", 3, ""),
        ]
        .join("\n"),
    );
    let came = fetched(&before, &after);
    assert_eq!(
        (came.new, came.moved, came.gone),
        (vec!["origin/new".to_string()], vec!["origin/dev".to_string()], vec!["origin/old".to_string()])
    );
    let none = fetched(&after, &after);
    assert_eq!(none.said(), "Fetched: every remote's branches are as they were.");
    assert_eq!(
        fetched(&before, &after).said(),
        "Fetched. new: `origin/new`; moved on: `origin/dev`; gone: `origin/old`."
    );
}
