//! What is said to the factory's bot, read by how each command is written:
//! fixed words, one word, the rest of them, pairs and flags, a word left
//! out by a flag or by the channel's project, and a fixed word in the middle.

use talk::taken::{begins, key, read};

fn words(said: &str) -> Vec<&str> {
    said.split(' ').collect()
}

#[test]
fn each_part_of_how_a_command_is_written_reads_what_is_said_to_it() {
    let moved = read("agent move <name> <project>|--none", &words("agent move helper --none"), None).unwrap();
    assert_eq!((moved.word("name"), moved.has("none"), moved.word("project")), ("helper", true, ""));
    let to = read("agent move <name> <project>|--none", &words("agent move helper depot"), None).unwrap();
    assert_eq!((to.word("project"), to.has("none")), ("depot", false));
    // A flag's value is every word up to the next flag, and one said again is kept each time.
    let added = read(
        "board add <project> <title...> [--body TEXT] [--status COLUMN] [--label L]... [--field name=value]...",
        &words("board add A till --body Where money is --label a --label b --field size=3"),
        Some("shop"),
    )
    .unwrap();
    assert_eq!(
        (added.word("project"), added.word("title"), added.flag("body"), added.every("label")),
        ("shop", "A till", Some("Where money is"), &["a".to_string(), "b".to_string()][..])
    );
    assert_eq!(added.every("field"), ["size=3"]);
    // Flags inside flags, and a fixed word in the middle.
    let applied = read(
        "apply <file|name> [--project P [--environment E]] [--param k=v]...",
        &words("apply std/board --environment dev --param a=1"),
        Some("shop"),
    )
    .unwrap();
    assert_eq!(
        (applied.word("file|name"), applied.flag("environment"), applied.every("param")),
        ("std/board", Some("dev"), &["a=1".to_string()][..])
    );
    let ran = read(
        "project environment <project> <env> run <branch>",
        &words("project environment dev run main"),
        Some("shop"),
    )
    .unwrap();
    assert_eq!((ran.word("project"), ran.word("env"), ran.word("branch")), ("shop", "dev", "main"));
    assert!(
        read(
            "project environment <project> <env> run <branch>",
            &words("project environment dev go main"),
            Some("shop")
        )
        .is_err()
    );
    // A fixed word that may be left out, and a rest that may be none.
    assert!(
        read("compare [list]", &words("compare"), None).is_ok()
            && read("compare [list]", &words("compare list"), None).is_ok()
    );
    assert_eq!(
        read("board columns <project> [<column>...]", &words("board columns"), Some("shop"))
            .unwrap()
            .word("column"),
        ""
    );
    // What a command is known by, and whether what is said begins with it.
    assert_eq!(key("project environment <project> <env> run <branch>"), "project environment run");
    assert_eq!(key("compare [list]"), "compare");
    assert_eq!(
        (
            begins("board move <card> <column>", &words("board move")),
            begins("board move <card>", &words("board"))
        ),
        (Some(2), None)
    );
    // What is not taken is said, with how the command is written.
    let unknown =
        read("board give <card> <agent>|nobody", &words("board give shop-1 nobody --at once"), None)
            .unwrap_err();
    assert!(unknown.starts_with("`--at` is not something `board give` takes"), "{unknown}");
    let more = read("record show <record>", &words("record show a b"), None).unwrap_err();
    assert!(more.starts_with("`b` is more than `record show` takes"), "{more}");
}
