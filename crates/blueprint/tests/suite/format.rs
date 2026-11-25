//! Laying a blueprint out: the same for everyone, and never a change in
//! what it means.

use std::path::{Path, PathBuf};

use blueprint::format::{Expressions, OneLine, Order, SETTINGS, SETTINGS_FILE, not_yaml};
use blueprint::{Settings, Unformatted, format, meaning, means_the_same};

fn laid(source: &str) -> String {
    format(source, &Settings::default()).unwrap_or_else(|left| panic!("{}\n{source}", left.message()))
}

fn with(settings: &str, source: &str) -> String {
    let settings = Settings::read(settings).expect("the settings read");
    format(source, &settings).unwrap_or_else(|left| panic!("{}\n{source}", left.message()))
}

/// Every blueprint file under a folder of this repository.
pub fn blueprints(folder: &str) -> Vec<PathBuf> {
    fn under(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                under(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "bp") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    under(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(folder), &mut found);
    found.sort();
    found
}

#[test]
fn every_blueprint_here_means_what_it_meant_once_laid_out_and_laying_it_out_again_changes_nothing() {
    let mut seen = 0;
    for folder in ["blueprints", "examples", "crates/blueprint/tests/fixtures"] {
        for file in blueprints(folder) {
            let source = std::fs::read_to_string(&file).unwrap();
            for settings in ["", "width: 60\nindent: 4\nexpressions: spaced", "one_line: never"] {
                let once = with(settings, &source);
                assert_eq!(meaning(&once), meaning(&source), "{} changed what it means", file.display());
                // And it is a text YAML reads as this reader does, however
                // it was written.
                assert_eq!(not_yaml(&once), [], "{} is laid out with something YAML parts", file.display());
                assert_eq!(
                    with(settings, &once),
                    once,
                    "{} is not settled by one laying out",
                    file.display()
                );
            }
            // With its fields put in the language's order it says the same
            // things (the formatter holds itself to that, or hands nothing
            // back), and is settled as soon.
            let ordered = with("order: vocabulary", &source);
            assert_eq!(with("order: vocabulary", &ordered), ordered, "{}", file.display());
            seen += 1;
        }
    }
    assert!(seen >= 15, "the blueprints that ship were not found: only {seen} files");
}

#[test]
fn a_text_written_any_old_way_comes_out_the_one_way_with_every_comment_where_it_was() {
    let cases = [
        (
            include_str!("../fixtures/layout/hard.before.txt"),
            include_str!("../fixtures/layout/hard.after.txt"),
        ),
        (
            include_str!("../fixtures/layout/notes.before.txt"),
            include_str!("../fixtures/layout/notes.after.txt"),
        ),
        // A first line that is said to an editor: which schema the file
        // is checked against, by the common language server for YAML. And
        // values in quotes, which is what makes them YAML to that editor
        // where they hold `: ` or a `${ }` inside `{ }`: left in them.
        (
            include_str!("../fixtures/layout/pointed.before.txt"),
            include_str!("../fixtures/layout/pointed.after.txt"),
        ),
        // A value written plain that YAML takes for a key and its value,
        // or ends sooner than this reader does: in quotes, of the kind
        // that needs nothing in it written another way.
        (
            include_str!("../fixtures/layout/parted.before.txt"),
            include_str!("../fixtures/layout/parted.after.txt"),
        ),
        // A `${ }` written bare inside braces or brackets: in quotes
        // there, and bare still where its braces were opened into a block.
        (
            include_str!("../fixtures/layout/braced.before.txt"),
            include_str!("../fixtures/layout/braced.after.txt"),
        ),
        // A value inside braces that begins with what no value of a line
        // can, and a key there that no first key of a list's item can
        // begin with: in quotes where the braces are opened into a block,
        // or the text laid out would not read, or would read as another.
        (
            include_str!("../fixtures/layout/opened.before.txt"),
            include_str!("../fixtures/layout/opened.after.txt"),
        ),
        // `keep` and `drop`, each written both ways a mapping can be.
        (
            include_str!("../fixtures/layout/kept.before.txt"),
            include_str!("../fixtures/layout/kept.after.txt"),
        ),
    ];
    for (before, after) in cases {
        assert_eq!(laid(before), after);
        assert_eq!(laid(after), after, "what is laid out is left as it is");
        assert!(means_the_same(before, after), "laying it out changed what it means:\n{after}");
        assert_eq!(not_yaml(after), [], "what is laid out has something YAML parts:\n{after}");
        // What a file begins with before its first word is where it was:
        // a line there may be said to whatever reads the file first.
        let head = |text: &'static str| {
            let said = text.lines().skip_while(|line| line.trim().is_empty());
            said.take_while(|line| line.starts_with('#')).collect::<Vec<_>>()
        };
        assert_eq!(head(after), head(before), "what the file begins with was moved");
        for comment in before.lines().filter_map(|line| line.split_once(" # ").map(|(_, said)| said.trim())) {
            // Only what is a comment: a `#` inside quotes is the string's.
            if !comment.contains('"') {
                assert!(after.contains(comment), "the comment `{comment}` was lost");
            }
        }
    }
}

#[test]
fn a_key_no_line_can_begin_with_is_quoted_where_its_mapping_is_written_a_line_to_a_key() {
    // Inside `{ }` a key may begin with what no line can; too long for its
    // line, the mapping is written out, and the key begins a line.
    let source = "blueprint: keys\nparams:\n  who: { type: string, ---default: \"\", description: \"Who moved it, as the event of the move says, which is long.\" }\n";
    let out = laid(source);
    assert_eq!(
        out,
        "blueprint: keys\n\nparams:\n  who:\n    type: string\n    \"---default\": \"\"\n    description: \"Who moved it, as the event of the move says, which is long.\"\n"
    );
    assert_eq!(meaning(&out), meaning(source));
    assert_eq!(laid(&out), out, "what is laid out is left as it is");
    // `...` hands on another mapping's entries, and is a key as it is.
    let handing =
        "blueprint: keys\nsteps:\n  a:\n    call: other\n    with:\n      ...: ${params}\n      word: a\n";
    assert!(laid(handing).contains("...: ${params}"), "{}", laid(handing));
}

#[test]
fn a_block_of_text_keeps_every_line_and_how_far_each_is_set_in_past_the_first() {
    let source = "blueprint: text\nsteps:\n    say:\n        do: shell\n        with:\n            run: |\n                  first\n                    deeper\n\n                  # not a comment\n            folded: >-\n              one\n              two\n";
    let out = laid(source);
    assert_eq!(
        out,
        "blueprint: text\n\nsteps:\n  say:\n    do: shell\n    with:\n      run: |\n        first\n          deeper\n\n        # not a comment\n      folded: >-\n        one\n        two\n"
    );
}

#[test]
fn what_is_inside_quotes_and_inside_an_expression_is_never_taken_for_layout() {
    let source = concat!(
        "blueprint: quotes\n",
        "steps:\n",
        "  say:\n",
        "    do: log\n",
        "    when: ${contains(params.words, \"a: b # c, d\")&&{ }!=null}\n",
        "    with: { a: \"x: y # z\", b: 'it''s, [here] {too}', c: \"$${ kept }\", d: \"\\\"${  left  }\\\"\", e: ${ {a: 1}[\"a\"] } }\n",
    );
    let out = laid(source);
    // Written plain, an expression is one thing to this reader whatever is
    // in it. YAML parts this one at its `: `, so it is put in quotes whole,
    // of the kind that leaves the quotes inside it as they are.
    assert!(out.contains("when: '${ contains(params.words, \"a: b # c, d\")&&{ }!=null }'"), "{out}");
    assert!(out.contains("a: \"x: y # z\""), "{out}");
    assert!(out.contains("b: 'it''s, [here] {too}'"), "{out}");
    assert!(out.contains("c: \"$${ kept }\""), "an escaped `${{` is text, and is left: {out}");
    assert!(
        out.contains("d: \"\\\"${  left  }\\\"\""),
        "a string with an escape in it is left exactly: {out}"
    );
    assert!(out.contains("e: '${ {a: 1}[\"a\"] }'"), "{out}");
}

#[test]
fn a_map_in_braces_stays_on_its_line_while_it_fits_and_opens_into_a_block_when_it_does_not() {
    let fits =
        "blueprint: fit\nsteps:\n  a:\n    do: emit\n    with: { topic: shop.said, to: \"${params.who}\" }\n";
    assert_eq!(
        laid(fits),
        "blueprint: fit\n\nsteps:\n  a:\n    do: emit\n    with: { topic: shop.said, to: \"${params.who}\" }\n"
    );

    let long = format!(
        "blueprint: long\nsteps:\n  a:\n    do: emit\n    with: {{ topic: shop.said, words: \"{}\", list: [one, two], inner: {{ deep: true }} }}\n",
        "w".repeat(70)
    );
    let out = laid(&long);
    assert!(
        out.ends_with(&format!(
            "    with:\n      topic: shop.said\n      words: \"{}\"\n      list: [one, two]\n      inner: {{ deep: true }}\n",
            "w".repeat(70)
        )),
        "{out}"
    );

    // A block is the author's choice and stays one, however short.
    let block = "blueprint: block\n\nsteps:\n  a:\n    do: emit\n    with:\n      topic: t\n";
    assert_eq!(laid(block), block);

    // Braces over several lines are one line again when that fits.
    let spread =
        "blueprint: spread\nsteps:\n  a:\n    do: emit\n    with: { topic: t,\n            to: x }\n";
    assert!(laid(spread).ends_with("    with: { topic: t, to: x }\n"));
}

#[test]
fn a_text_that_does_not_read_is_left_as_it_is_and_says_what_the_reader_said() {
    for unread in ["", "\n# only a comment\n", "blueprint: x\n  steps: oops\n", "steps:\n\tdo: x\n"] {
        match format(unread, &Settings::default()) {
            Err(Unformatted::Unreadable(said)) => assert!(!said.message.is_empty()),
            other => panic!("`{unread}` should not have been laid out: {other:?}"),
        }
    }
    let said = format("blueprint: x\nsteps: [\n", &Settings::default()).unwrap_err().message();
    assert!(said.contains("never closed"), "{said}");
}

#[test]
fn windows_line_endings_and_spaces_at_the_ends_of_lines_are_not_kept() {
    let out = laid("blueprint: x   \r\nsteps:\r\n  a:   \r\n    do: noop\r\n\r\n\r\n");
    assert_eq!(out, "blueprint: x\n\nsteps:\n  a:\n    do: noop\n");
}

const SHAPED: &str = "\
blueprint: shaped
params:
  port: 1
  name: { type: string, default: x }
steps:
  one: { do: noop }
  two:
    description: Says it.
    when: ${  params.port>0  }
    do: log
    with: { message: \"${ params.name }\", long: [aaaaaaaaaa, bbbbbbbbbb, cccccccccc, dddddddddd] }
  three: { do: noop }
";

#[test]
fn each_thing_a_project_can_say_of_the_layout_does_what_it_says() {
    // As it is for everyone.
    let usual = laid(SHAPED);
    assert!(usual.contains(
        "  one: { do: noop }\n\n  two:\n    description: Says it.\n    when: ${ params.port>0 }\n"
    ));
    assert!(usual.contains("message: \"${params.name}\""), "{usual}");

    let narrow = with("width: 60", SHAPED);
    assert!(narrow.contains("    with:\n      message: \"${params.name}\"\n      long: [aaaaaaaaaa, bbbbbbbbbb, cccccccccc, dddddddddd]\n"));
    let narrower = with("width: 40", SHAPED);
    assert!(narrower.contains("      long:\n        - aaaaaaaaaa\n        - bbbbbbbbbb\n"), "{narrower}");

    let deep = with("indent: 4", SHAPED);
    assert!(
        deep.contains("\nsteps:\n    one: { do: noop }\n\n    two:\n        description: Says it.\n"),
        "{deep}"
    );

    let tight = with("blank_lines: 0", SHAPED);
    assert!(
        tight.contains(
            "  port: 1\n  name: { type: string, default: x }\nsteps:\n  one: { do: noop }\n  two:\n"
        ),
        "{tight}"
    );
    let wide = with("blank_lines: 2", SHAPED);
    assert!(wide.contains("  one: { do: noop }\n\n\n  two:\n"), "{wide}");
    assert!(wide.contains("  port: 1\n  name: {"), "two that are each on one line are not set apart: {wide}");

    let blocks = with("one_line: never", SHAPED);
    assert!(blocks.contains("  one:\n    do: noop\n"), "{blocks}");
    assert!(
        blocks.contains("      long: [aaaaaaaaaa, bbbbbbbbbb, cccccccccc, dddddddddd]\n"),
        "a list goes by the width: {blocks}"
    );
    let lines = with("one_line: always\nwidth: 40", SHAPED);
    assert!(lines.contains(
        "    with: { message: \"${params.name}\", long: [aaaaaaaaaa, bbbbbbbbbb, cccccccccc, dddddddddd] }\n"
    ));

    assert!(with("expressions: spaced", SHAPED).contains("message: \"${ params.name }\""));
    assert!(with("expressions: tight", SHAPED).contains("when: ${params.port>0}\n"));
    let written = with("expressions: written", SHAPED);
    assert!(written.contains("when: ${  params.port>0  }\n") && written.contains("\"${ params.name }\""));

    let ordered = with("order: vocabulary", SHAPED);
    assert!(ordered.contains("  two:\n    do: log\n    with:"), "{ordered}");
    assert!(ordered.find("    when:").unwrap() < ordered.find("    description:").unwrap(), "{ordered}");
    assert!(
        ordered.find("  one:").unwrap() < ordered.find("  two:").unwrap()
            && ordered.find("  two:").unwrap() < ordered.find("  three:").unwrap(),
        "names of the author's own stay in the order written: {ordered}"
    );
}

#[test]
fn settings_say_what_they_are_and_refuse_what_is_not_one() {
    assert_eq!(Settings::read("").unwrap(), Settings::default());
    assert_eq!(Settings::read("# nothing yet\n\n").unwrap(), Settings::default());
    let read = Settings::read(
        "width: 120\nindent: 4\nblank_lines: 0\none_line: never\nexpressions: tight\norder: vocabulary\n",
    )
    .unwrap();
    assert_eq!(
        read,
        Settings {
            width: 120,
            indent: 4,
            blank_lines: 0,
            one_line: OneLine::Never,
            expressions: Expressions::Tight,
            order: Order::Vocabulary,
        }
    );

    let unknown = Settings::read("tabs: true").unwrap_err();
    assert!(unknown.contains("`tabs` is not something the layout of a blueprint can be told"), "{unknown}");
    for setting in SETTINGS {
        assert!(unknown.contains(setting.name), "the refusal lists `{}`: {unknown}", setting.name);
    }
    assert_eq!(Settings::read("width: wide").unwrap_err(), "`width` must be a whole number from 40 to 1000");
    assert_eq!(Settings::read("indent: 0").unwrap_err(), "`indent` must be a whole number from 1 to 8");
    assert_eq!(
        Settings::read("expressions: roomy").unwrap_err(),
        "`expressions` must be `mixed`, `spaced`, `tight` or `written`"
    );
    assert!(Settings::read("width: [").unwrap_err().starts_with("line 1:"));
    assert_eq!(SETTINGS_FILE, "blueprint-format.yaml");
}

#[test]
fn the_documentation_lists_every_setting_as_it_is_here() {
    let docs =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/blueprints.md"))
            .unwrap();
    for setting in SETTINGS {
        let row =
            format!("| `{}` | `{}` | {} | {} |", setting.name, setting.default, setting.may_be, setting.help);
        assert!(
            docs.contains(&row),
            "docs/blueprints.md is out of step with the settings of the layout. Its table should have this row:\n{row}"
        );
    }
}

#[test]
fn two_texts_mean_the_same_when_only_layout_comments_and_the_spaces_in_an_expression_differ() {
    let one = "blueprint: same\nsteps:\n  a: { do: log, with: { message: \"${ params.x }\" } }\n  b: { do: noop }\n";
    let other = "# said first\nblueprint:   same\n\nsteps:\n  # why\n  a:\n    do: log\n    with:\n      message: '${params.x}'   # quoted the other way\n  b: { do: noop }\n";
    assert!(means_the_same(one, other));
    assert!(means_the_same(one, &laid(other)));

    assert!(!means_the_same(one, &one.replace("noop", "log")), "another action is another text");
    assert!(!means_the_same(one, &one.replace("params.x", "params.y")), "another expression is another text");
    let swapped = "blueprint: same\nsteps:\n  b: { do: noop }\n  a: { do: log, with: { message: \"${ params.x }\" } }\n";
    assert!(!means_the_same(one, swapped), "the order of steps is part of what is said");
    assert!(
        !means_the_same(one, &one.replace("\"${ params.x }\"", "\"${ params.x } \"")),
        "a space outside the braces is the string's"
    );
    assert!(
        !means_the_same("not: [", "not:  ["),
        "texts that do not read are the same only letter for letter"
    );
    assert!(means_the_same("not: [", "not: ["));
}

#[test]
fn what_yaml_does_not_read_as_written_is_found_by_its_line_and_is_gone_once_laid_out() {
    let lines = |source: &str| not_yaml(source).iter().map(|span| span.start.line).collect::<Vec<_>>();
    let source = "\
blueprint: parted
description: A board: its cards
steps:
  say:
    do: emit
    when: ${ params.n > 1 ? true : false }
    with: { topic: shop.said, data: { rounds: ${ params.rounds + 1 }, why: plain } }
  told:
    do: emit
    with:
      topic: shop.told
      data:
        whole: ${params.item}
        quoted: \"as: it was\"
        count: 3
        block: |
          a line: of a block
";
    // A value with `: ` in it, an expression with one, and a `${ }` bare
    // inside braces; not one bare in a block, nor what is in quotes, nor a
    // number, nor the lines of a block of text.
    assert_eq!(lines(source), [2, 6, 7]);
    let once = laid(source);
    assert_eq!(lines(&once), [] as [u32; 0], "{once}");
    assert!(means_the_same(source, &once));
    // In quotes of the kind that needs nothing in it written another way:
    // double, unless the text has one or a backslash in it.
    let quoted =
        laid("blueprint: q\nparams:\n  a: { default: he said \"no: so\" }\n  b: { default: a\\d: b }\n");
    assert!(
        quoted.contains("default: 'he said \"no: so\"'") && quoted.contains("default: 'a\\d: b'"),
        "{quoted}"
    );
    // What does not read says nothing of it: the reader's own complaint is the one to hear.
    assert_eq!(lines("blueprint: [never closed"), [] as [u32; 0]);
}
