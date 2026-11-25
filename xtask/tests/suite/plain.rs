//! What the blueprints that ship work out for themselves, held to what is
//! still let through: asked of the part that holds it, with what a reading
//! of the texts would say.

use xtask::plain::{Let, READ, SAID, STILL, Unplain, heard, held, of_package};

fn let_(file: &str, count: usize) -> Let {
    Let { file: file.into(), count, becomes: "next".into(), written: "the list".into() }
}

fn one(file: &str, line: u32) -> Unplain {
    Unplain { file: file.into(), line, why: "it calls a function".into(), source: "len(state.back)".into() }
}

#[test]
fn what_the_reader_said_is_read_back_a_line_an_expression() {
    let printed = format!(
        "running 1 test\n{SAID}blueprints/std/pool.bp\t41\tit calls a function\tlen(state.back) > 0\n\
         {SAID}extensions/shelf/extension.yaml\t120\tit chooses with `? :`\ta ? b : c\n{READ}37\ntest ok\n"
    );
    let (read, found) = heard(&printed).unwrap();
    assert_eq!(read, 37);
    assert_eq!(found.len(), 2);
    assert_eq!(
        found[0],
        Unplain {
            file: "blueprints/std/pool.bp".into(),
            line: 41,
            why: "it calls a function".into(),
            source: "len(state.back) > 0".into()
        }
    );
    assert_eq!(found[1].file, "extensions/shelf/extension.yaml");
}

#[test]
fn what_the_reader_says_first_is_read_though_it_comes_after_the_tests_name_on_one_line() {
    // What runs one test at a time writes the test's name and no end of
    // line before the test says anything.
    let printed = format!(
        "running 1 test\ntest plainly::the_expressions ... {SAID}blueprints/std/arrangement.bp\t9\tit calls a \
         function\tlen(x)\n{SAID}blueprints/std/bot.bp\t52\tit calls a function\tslug(y)\n{READ}37\nok\n"
    );
    let (read, found) = heard(&printed).unwrap();
    assert_eq!(read, 37);
    let files: Vec<&str> = found.iter().map(|one| one.file.as_str()).collect();
    assert_eq!(files, ["blueprints/std/arrangement.bp", "blueprints/std/bot.bp"]);
}

#[test]
fn a_reader_that_did_not_run_to_its_end_is_not_taken_for_one_that_found_nothing() {
    let why = heard("running 1 test\nthread panicked\n").unwrap_err();
    assert!(why.contains("did not say how many it read"), "{why}");
    let why = heard(&format!("{SAID}only two\tparts\n{READ}1\n")).unwrap_err();
    assert!(why.contains("a line this cannot read"), "{why}");
}

#[test]
fn with_nothing_found_and_nothing_let_through_the_gate_says_how_many_texts_are_plain() {
    assert_eq!(held(37, &[], &[]).unwrap(), "37 blueprints that ship work nothing out for themselves");
}

#[test]
fn what_is_as_listed_passes_and_says_how_much_is_still_to_do() {
    let found = [one("a.bp", 3), one("a.bp", 9), one("b.bp", 1)];
    let still = [let_("a.bp", 2), let_("b.bp", 1)];
    assert_eq!(
        held(37, &found, &still).unwrap(),
        "3 expressions in 2 files are still to be made plain, of 37 blueprints that ship"
    );
    assert_eq!(
        held(37, &found[2..], &still[1..]).unwrap(),
        "1 expression in 1 file is still to be made plain, of 37 blueprints that ship"
    );
}

#[test]
fn a_file_that_is_not_listed_and_works_something_out_fails_with_each_place_and_why() {
    let problems = held(37, &[one("new.bp", 12)], &[]).unwrap_err();
    assert_eq!(problems.len(), 1);
    assert!(
        problems[0].starts_with("new.bp works something out for itself, in 1 expression:"),
        "{problems:?}"
    );
    assert!(problems[0].contains("  line 12: it calls a function: `len(state.back)`"), "{problems:?}");
    assert!(problems[0].contains("asks a building block"), "{problems:?}");
}

#[test]
fn a_file_with_more_than_it_is_let_fails_and_names_every_one_so_that_the_new_one_is_found() {
    let found = [one("a.bp", 3), one("a.bp", 9), one("a.bp", 20)];
    let problems = held(37, &found, &[let_("a.bp", 2)]).unwrap_err();
    assert_eq!(problems.len(), 1);
    assert!(
        problems[0].starts_with("a.bp has 3 expressions that are not plain, and 2 are still let through")
    );
    for line in ["line 3:", "line 9:", "line 20:"] {
        assert!(problems[0].contains(line), "{problems:?}");
    }
}

#[test]
fn a_file_with_fewer_than_it_is_let_fails_until_the_list_says_what_is_so() {
    let problems = held(37, &[one("a.bp", 3)], &[let_("a.bp", 2), let_("gone.bp", 4)]).unwrap_err();
    assert_eq!(
        problems,
        [
            "a.bp has 1 expression that is not plain, and is still let 2: lower its number in the list to 1",
            "gone.bp is plain now, and is still let 4 expressions: take that out of the list",
        ]
    );
}

#[test]
fn what_is_still_let_through_names_each_file_once_with_what_is_to_become_of_it() {
    let mut files: Vec<&str> = STILL.iter().map(|(file, _, _)| *file).collect();
    files.sort_unstable();
    let listed = files.len();
    files.dedup();
    assert_eq!(files.len(), listed, "a file is listed twice");
    for (file, count, becomes) in STILL {
        assert!(*count > 0, "{file} is listed with nothing to make plain");
        assert!(!becomes.trim().is_empty(), "{file} is listed with nothing said of what is to become of it");
    }
}

#[test]
fn a_package_says_in_one_line_what_its_own_blueprints_are_still_let_and_anything_else_is_refused() {
    let written = "extensions/shelf/contract/plain.txt";
    let manifest = "extensions/shelf/extension.yaml";
    let said =
        of_package(manifest, written, "# what is left\n\n3 to be made plain before the first release\n");
    assert_eq!(
        said.unwrap(),
        Let {
            file: manifest.into(),
            count: 3,
            becomes: "to be made plain before the first release".into(),
            written: written.into()
        }
    );
    for unread in ["", "# only a note\n", "three, soon\n", "0 nothing\n", "4\n"] {
        let why = of_package(manifest, written, unread).unwrap_err();
        assert!(why.starts_with("extensions/shelf/contract/plain.txt says what"), "{unread:?}: {why}");
    }
}
