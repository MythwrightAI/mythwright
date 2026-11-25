//! `explain`: a blueprint said in plain words, from a terminal, with the
//! real program.

use crate::{Rig, run_in};

/// A text of somebody's own with something for each part of a summary.
const WATCH: &str = "\
blueprint: watch
description: Keep a table of what was seen, and say so.
permissions: [projects, channels, shell]
params:
  table: { type: string, default: seen, description: What the table is called. }
steps:
  seen:
    do: collection.ensure
    with: { name: \"${params.table}\", fields: { title: { type: text } } }
  look:
    description: Look at what changed. It reads and writes nothing.
    do: shell
    with: { cmd: [git, status] }
on:
  moved:
    event: board.item.moved
    do: channel.post
    with: { channel: general, body: moved }
";

#[tokio::test(flavor = "multi_thread")]
async fn a_blueprint_is_said_in_plain_words_from_a_terminal_by_its_file_and_by_its_name() {
    let rig = Rig::signed_in().await;

    // One that ships, by its name, with nothing applied anywhere.
    let board = rig.run(&["explain", "std/board"]).await.ok();
    assert!(
        board.starts_with(
            "std/board\n\nWhat it is\n  Give a project a board: its cards, its columns, and where they are kept.\n"
        ),
        "{board}"
    );
    for line in [
        "  Whoever applies it may tell it 3 things:\n    - columns: The columns, left to right. Unless it is said: \
         `backlog`, `ready`, `doing`, `review` and `done`.\n",
        // Each table it makes with its fields under it, as the action that makes it says they are read.
        "\nWhat it makes\n  A table called `board`. [board]\n    | `",
        "\n    | `title`: text",
        "\n  Another blueprint, `std/cards`, started under it as `cards` and left running. [cards]\n",
        "\nWhat it does when it is applied\n  1. The table a card is a row of, and the ways it is laid out; the board \
         is the rows by their status. [board]\n",
        "\nWhat it may do\n  It runs as whoever applies it, and can do no more than they can.\n",
        "  Create projects and change their files. [projects:write]\n",
    ] {
        assert!(board.contains(line), "`{line}` in:\n{board}");
    }
    assert!(!board.contains("${") && !board.contains("collection.ensure"), "{board}");
    // One that another runs says what it is a part of, after what it says it is; the board, which somebody
    // applies, says nothing of it.
    let cards = rig.run(&["explain", "std/cards"]).await.ok();
    assert!(cards.contains("\n  A part of `std/board`.\n"), "{cards}");
    assert!(!board.contains("A part of"), "{board}");
    let slot = rig.run(&["explain", "std/slot"]).await.ok();
    assert!(
        slot.contains("\n  A part of 2 blueprints:\n    - `std/log-feed`\n    - `std/store`\n"),
        "{slot}"
    );
    assert!(rig.run(&["instances"]).await.ok().trim().lines().count() <= 1, "nothing was applied for it");

    // A file, read where it is: what it hears, and what somebody deciding
    // to apply it most needs to see, marked.
    let file = rig.dir.join("watch.bp");
    std::fs::write(&file, WATCH).unwrap();
    let file = file.to_str().unwrap();
    let said = rig.run(&["explain", file]).await.ok();
    for line in [
        "\nWhat it makes\n  A table called what the param `table` says. [seen]\n",
        "  2. Look at what changed. [look]\n",
        "  When this happens: a card was moved to another column (`board.item.moved`). Heard from its own project, and \
         from the factory itself. [moved]\n    1. Post a message to a channel. [run]\n",
        "  Run commands on a device. [shell]\n  ! It runs a command: `git status`. [look]\n",
    ] {
        assert!(said.contains(line), "`{line}` in:\n{said}");
    }

    // As the daemon hands it out, for whatever draws it.
    let handed = rig.run(&["explain", file, "--json"]).await.ok();
    let handed: serde_json::Value = serde_json::from_str(&handed).expect("JSON");
    assert_eq!(handed["ok"], true);
    assert_eq!(handed["summary"]["parts"][0]["kind"], "is");
    assert_eq!(handed["summary"]["name"], "watch");

    // One that does not check has none: what is wrong is printed as `check`
    // prints it, and the command fails saying why there is nothing more.
    let wrong = rig.dir.join("wrong.bp");
    std::fs::write(&wrong, WATCH.replace("do: shell", "do: shel")).unwrap();
    let wrong = wrong.to_str().unwrap();
    let refused = rig.run(&["explain", wrong]).await;
    assert_eq!(refused.code, 1, "{}", refused.out);
    assert!(
        refused.out.contains(&format!(
            "{wrong}:10:3: error: there is no action called `shel`; did you mean `shell`?"
        )),
        "{}",
        refused.out
    );
    assert!(!refused.out.contains("What it is"), "{}", refused.out);
    assert!(
        refused
            .err
            .contains("does not check, so there is no summary of it: what is wrong with it is said above"),
        "{}",
        refused.err
    );

    // A name nothing has is said to be that, and so is saying nothing.
    let nobody = rig.run(&["explain", "shop/no-such"]).await;
    assert_eq!(nobody.code, 1);
    assert!(nobody.err.contains("there is no blueprint called `shop/no-such`"), "{}", nobody.err);
    let nothing = rig.run(&["explain"]).await;
    assert!(
        nothing.err.contains("missing the blueprint to explain: a file, or a blueprint's name"),
        "{}",
        nothing.err
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn with_no_daemon_a_file_is_said_from_its_text_alone_and_it_says_that_this_is_less() {
    let rig = Rig::signed_in().await;
    let file = rig.dir.join("watch.bp");
    std::fs::write(&file, WATCH).unwrap();
    let file = file.to_str().unwrap();

    let alone = run_in(&rig.dir, "http://127.0.0.1:1", &[], &["explain", file]).await;
    assert_eq!(alone.code, 0, "{}", alone.err);
    // What the text says of itself, and what the language's own words mean, are said as ever.
    for line in [
        "watch\n\nWhat it is\n  Keep a table of what was seen, and say so.\n",
        "  2. Look at what changed. [look]\n",
        "  When `board.item.moved` is said. Heard from its own project, and from the factory itself. [moved]\n",
    ] {
        assert!(alone.out.contains(line), "`{line}` in:\n{}", alone.out);
    }
    // What only the daemon knows is not made up: an action is named, and
    // nothing is said of what it makes, runs or takes leave for.
    assert!(
        alone.out.contains("  1. Do `collection.ensure`, an action this was read without. [seen]\n"),
        "{}",
        alone.out
    );
    assert!(!alone.out.contains("What it makes") && !alone.out.contains("! It runs"), "{}", alone.out);
    assert!(
        alone.err.contains(
            "this is said from the file alone, with no action known but those that are part of the language"
        ),
        "{}",
        alone.err
    );

    // A name is found by the daemon, and with none there is nobody to ask.
    let named = run_in(&rig.dir, "http://127.0.0.1:1", &[], &["explain", "std/board"]).await;
    assert_eq!(named.code, 1);
    assert!(named.err.contains("nothing is answering at http://127.0.0.1:1"), "{}", named.err);
    // A file that does not read is said so, as `check` says it there.
    std::fs::write(rig.dir.join("worse.bp"), "blueprint: worse\nsteps: [\n").unwrap();
    let worse = rig.dir.join("worse.bp");
    let unread = run_in(&rig.dir, "http://127.0.0.1:1", &[], &["explain", worse.to_str().unwrap()]).await;
    assert_eq!(unread.code, 1);
    assert!(unread.out.contains("worse.bp:3:1: error: this `[` or `{` is never closed"), "{}", unread.out);
    assert!(
        unread.err.contains("does not read as a blueprint, so there is no summary of it"),
        "{}",
        unread.err
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn help_says_how_a_blueprint_is_explained_on_a_line_that_was_there() {
    let rig = Rig::new().await;
    let overview = rig.run(&["help"]).await.ok();
    assert!(overview.lines().count() < 70, "help has room for no more lines: {}", overview.lines().count());
    let line =
        overview.lines().find(|line| line.contains("explain <file|name>")).expect("a line for `explain`");
    assert!(line.contains("check <file>...") && line.contains("it in plain words (help explain)"), "{line}");
    let topic = rig.run(&["help", "explain"]).await.ok();
    assert!(
        topic.starts_with(&format!("{} explain <file|name> [--project P] [--json]", brand::CLI)),
        "{topic}"
    );
    for said in
        ["Says a blueprint in plain words", "Nothing is run and nothing is written", "begins with `!`"]
    {
        assert!(topic.contains(said), "`{said}` in:\n{topic}");
    }
    assert_eq!(rig.run(&["explain", "--help"]).await.ok(), topic, "and asked of the command itself");
}
