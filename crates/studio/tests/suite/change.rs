//! What a change to a record was, in words (`studio::change`, asked by
//! `std/fleet` as `record.change` for a row it could not take): a table of
//! what is handed and what it comes to, with no word left unreached; and
//! what a changed file says of its kind, which `std/fleet` goes by to
//! know a plan.

use studio::change::{Came, said};
use studio::projects::kind;

/// The event, what the row is called, its table, the cells changed; and
/// what it came to, in a word and in words.
type Case = (&'static str, &'static str, &'static str, &'static [&'static str], Came, &'static str);

#[test]
fn a_change_to_a_record_is_said_as_the_new_row_the_change_to_it_or_its_taking_away_with_its_cells() {
    let cases: [Case; 9] = [
        ("record.created", "worker", "agents", &[], Came::Added, "the new row `worker` of `agents`"),
        (
            "record.removed",
            "shop-stages-2",
            "stages",
            &[],
            Came::Removed,
            "the taking away of the row `shop-stages-2` of `stages`",
        ),
        (
            "record.updated",
            "worker",
            "agents",
            &["count"],
            Came::Changed,
            "the change to the row `worker` of `agents` (`count`)",
        ),
        // Several cells, in the order they were said.
        (
            "record.updated",
            "review",
            "stages",
            &["role", "seats", "rounds"],
            Came::Changed,
            "the change to the row `review` of `stages` (`role`, `seats`, `rounds`)",
        ),
        // A change that says no cells says none.
        (
            "record.updated",
            "worker",
            "agents",
            &[],
            Came::Changed,
            "the change to the row `worker` of `agents`",
        ),
        // Whatever is neither an adding nor a taking away is a change: a
        // topic of somebody's own, or one said on a record.
        (
            "record.commented",
            "worker",
            "agents",
            &[],
            Came::Changed,
            "the change to the row `worker` of `agents`",
        ),
        ("", "worker", "agents", &[], Came::Changed, "the change to the row `worker` of `agents`"),
        // The cells of a new row are said where they are handed.
        (
            "record.created",
            "tester",
            "agents",
            &["role"],
            Came::Added,
            "the new row `tester` of `agents` (`role`)",
        ),
        // What it is called is said as it is, whatever is in it.
        (
            "record.updated",
            "who `works`",
            "a b",
            &[],
            Came::Changed,
            "the change to the row `who `works`` of `a b`",
        ),
    ];
    let mut reached = Vec::new();
    for (topic, title, collection, changed, came, says) in cases {
        let changed: Vec<String> = changed.iter().map(|cell| (*cell).to_string()).collect();
        let said = said(topic, title, collection, &changed);
        assert_eq!((said.came, said.says.as_str()), (came, says), "`{topic}`, `{title}`, {changed:?}");
        reached.push(said.came);
    }
    for word in Came::ALL {
        assert!(reached.contains(word), "no case reaches `{}`", word.word());
    }
}

#[test]
fn what_kind_of_file_a_path_names_is_what_follows_the_last_dot_of_its_name() {
    for (path, is) in [
        ("docs/plans/cart.md", "md"),
        ("README.md", "md"),
        ("src/main.rs", "rs"),
        // The last dot, of the name and not of a folder.
        ("pkg/archive.tar.gz", "gz"),
        ("docs.v2/plans/cart", ""),
        ("docs.v2/plans/cart.md", "md"),
        // No dot, and a name that only begins with one, are of no kind.
        ("Makefile", ""),
        (".gitignore", ""),
        ("docs/.keep", ""),
        // One that ends in a dot says nothing after it.
        ("notes.", ""),
        ("", ""),
    ] {
        assert_eq!(kind(path), is, "`{path}`");
    }
}
