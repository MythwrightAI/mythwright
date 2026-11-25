//! A repository as a project's Git page shows it, read from what git
//! answers: the branch the folder is on, what is changed there, the
//! commits of a branch and the remotes.

use kernel::git::{Changed, MOST_CHANGES, commits, remotes, shown, status};

/// Entries as git answers `kernel::git::STATUS`, each ended by a nul.
fn said(entries: &[&str]) -> String {
    entries.iter().map(|entry| format!("{entry}\0")).collect()
}

fn change(path: &str, staged: &'static str, unstaged: &'static str) -> Changed {
    Changed { path: path.to_string(), was: None, staged, unstaged }
}

#[test]
fn the_branch_a_folder_is_on_is_said_with_how_it_stands_against_the_one_it_follows() {
    let found = status(&said(&[
        "# branch.oid 4b825dc642cb6eb9a060e54bf8d69288fbee4904",
        "# branch.head main",
        "# branch.upstream origin/main",
        "# branch.ab +2 -1",
    ]));
    assert_eq!(found.head.branch.as_deref(), Some("main"));
    assert_eq!(found.head.commit.as_deref(), Some("4b825dc642cb6eb9a060e54bf8d69288fbee4904"));
    assert_eq!(found.head.upstream.as_deref(), Some("origin/main"));
    assert_eq!((found.head.ahead, found.head.behind), (Some(2), Some(1)));
    assert!(found.changes.is_empty());
}

#[test]
fn a_folder_on_no_branch_and_a_repository_with_no_commit_say_none_of_either() {
    let detached = status(&said(&["# branch.oid abc", "# branch.head (detached)"]));
    assert_eq!((detached.head.branch, detached.head.commit.as_deref()), (None, Some("abc")));
    let new = status(&said(&["# branch.oid (initial)", "# branch.head main"]));
    assert_eq!((new.head.branch.as_deref(), new.head.commit), (Some("main"), None));
    assert_eq!(new.head.upstream, None, "one that follows nothing says nothing of it");
    assert_eq!((new.head.ahead, new.head.behind), (None, None));
}

#[test]
fn each_change_says_what_is_staged_and_what_is_not_in_words_with_paths_taken_whole() {
    let found = status(&said(&[
        "# branch.head main",
        "1 M. N... 100644 100644 100644 a1 b1 shop/cart.rs",
        "1 .M N... 100644 100644 100644 a1 a1 a file with spaces.md",
        "1 AD N... 000000 100644 000000 00 c1 added then removed",
        "2 R. N... 100644 100644 100644 a1 a1 R100 new/name.rs",
        "old/name.rs",
        "u UU N... 100644 100644 100644 100644 a1 b1 c1 both.rs",
        "? notes/todo.txt",
        "! target/",
    ]));
    assert_eq!(
        found.changes,
        [
            change("shop/cart.rs", "modified", ""),
            change("a file with spaces.md", "", "modified"),
            change("added then removed", "added", "deleted"),
            Changed { was: Some("old/name.rs".into()), ..change("new/name.rs", "renamed", "") },
            change("both.rs", "unmerged", "unmerged"),
            change("notes/todo.txt", "", "untracked"),
        ],
        "and what git ignores is no change"
    );
    assert_eq!(found.more, 0);
}

#[test]
fn a_working_tree_with_more_changes_than_are_kept_says_how_many_more() {
    let entries: Vec<String> = (0..MOST_CHANGES + 7).map(|n| format!("? file-{n}")).collect();
    let found = status(&said(&entries.iter().map(String::as_str).collect::<Vec<_>>()));
    assert_eq!((found.changes.len(), found.more), (MOST_CHANGES, 7));
    assert_eq!(status("").changes, [], "a clean tree has none");
}

#[test]
fn commits_are_read_newest_first_with_their_parents_and_a_line_that_does_not_read_is_left_out() {
    let said = [
        "c3\0Ada\0ada@example.com\x001700000300\0c2 b9\0Merge the cart",
        "not a commit",
        "c2\0Bo\0bo@example.com\0soon\0c1\0no time",
        "c1\0Bo\0bo@example.com\x001700000100\0\0The first: with a colon\tand a tab",
    ]
    .join("\n");
    let found = commits(&said);
    assert_eq!(found.iter().map(|one| one.commit.as_str()).collect::<Vec<_>>(), ["c3", "c1"]);
    assert_eq!(found[0].parents, ["c2", "b9"], "a merge has two");
    assert_eq!((found[0].author.as_str(), found[0].at), ("Ada", 1_700_000_300_000), "in milliseconds");
    assert!(found[1].parents.is_empty(), "the first has none");
    assert_eq!(found[1].subject, "The first: with a colon\tand a tab");
    assert!(commits("").is_empty());
}

#[test]
fn a_remote_is_said_once_with_where_it_is_pushed_only_where_that_is_elsewhere() {
    let said = "origin\thttps://example.org/acme/shop.git (fetch)\norigin\thttps://example.org/acme/shop.git (push)\n\
                mirror\tgit@example.net:acme/shop.git (fetch)\nmirror\tssh://example.net/push/shop.git (push)\n";
    let found = remotes(said);
    assert_eq!(found.len(), 2);
    assert_eq!(
        (found[0].name.as_str(), found[0].fetch.as_str(), &found[0].push),
        ("origin", "https://example.org/acme/shop.git", &None)
    );
    assert_eq!(found[1].fetch, "git@example.net:acme/shop.git");
    assert_eq!(found[1].push.as_deref(), Some("ssh://example.net/push/shop.git"));
    assert!(remotes("").is_empty());
}

#[test]
fn an_address_is_handed_on_without_a_name_and_password_or_token_written_into_it() {
    assert_eq!(shown("https://ada:secret@example.org/acme/shop.git"), "https://example.org/acme/shop.git");
    assert_eq!(shown("https://ghx_token@example.org/acme/shop.git"), "https://example.org/acme/shop.git");
    assert_eq!(
        shown("https://example.org/a@b/shop.git"),
        "https://example.org/a@b/shop.git",
        "an @ in the path is the path's"
    );
    assert_eq!(
        shown("git@example.org:acme/shop.git"),
        "git@example.org:acme/shop.git",
        "an account, and no secret"
    );
    assert_eq!(shown("/srv/git/shop.git"), "/srv/git/shop.git");
    let said = "origin\thttps://ada:secret@example.org/shop.git (fetch)\norigin\thttps://ada:secret@example.org/shop.git (push)\n";
    assert_eq!(remotes(said)[0].fetch, "https://example.org/shop.git", "and so is every remote listed");
}
