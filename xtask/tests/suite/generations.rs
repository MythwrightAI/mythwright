//! Which builds of the client an installed copy keeps: the rule the
//! client's build keeps `ui/dist` by, with the same figures and the same
//! cases (`ui/scripts/generations.test.ts`).

use std::path::Path;

use xtask::generations::{self, Build, DAY, KEPT_FOR, KEPT_FOR_DAYS, KEPT_MOST};

const NOW: u64 = 1_795_219_200_000;

fn build(id: &str, at: Option<u64>, files: &[&str]) -> Build {
    Build { id: id.into(), at, files: files.iter().map(|file| (*file).to_string()).collect() }
}

fn ids(builds: &[Build]) -> Vec<&str> {
    builds.iter().map(|build| build.id.as_str()).collect()
}

/// What `generations.ts` says a figure is, by its name.
fn said_in_the_client(name: &str) -> u64 {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/scripts/generations.ts");
    let text = std::fs::read_to_string(&file).unwrap();
    let line = format!("export const {name} = ");
    let at = text.find(&line).unwrap_or_else(|| panic!("{} says no `{line}`", file.display()));
    let rest = &text[at + line.len()..];
    rest[..rest.find(';').unwrap()]
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("`{name}` is a number written out"))
}

#[test]
fn an_installed_copy_keeps_its_builds_for_as_long_and_as_many_as_the_clients_own_build_does() {
    assert_eq!(KEPT_FOR_DAYS, said_in_the_client("KEPT_FOR_DAYS"), "change both figures together");
    assert_eq!(KEPT_MOST as u64, said_in_the_client("KEPT_MOST"), "change both figures together");
    assert_eq!((KEPT_FOR_DAYS, KEPT_MOST), (7, 20));
    assert_eq!(KEPT_FOR, 7 * DAY);
}

#[test]
fn a_page_two_and_three_installs_behind_still_has_every_part_of_its_build() {
    let one = build("one", Some(NOW - 3 * DAY), &["assets/app-1.js", "assets/chunk-editor.js"]);
    let two = build("two", Some(NOW - 2 * DAY), &["assets/app-2.js", "assets/chunk-editor.js"]);
    let three = build("three", None, &["assets/app-3.js", "assets/chunk-editor-new.js"]);
    let kept = generations::generations(&[two.clone(), one.clone()], &three, NOW);
    assert_eq!(ids(&kept), ["three", "two", "one"]);
    assert_eq!(kept[0].at, Some(NOW), "the build installed is made now, whenever it was built");
    let present: Vec<String> = [&one, &two, &three].iter().flat_map(|build| build.files.clone()).collect();
    assert!(generations::leftovers(&kept, &present).is_empty());
}

#[test]
fn a_build_whose_place_was_taken_a_week_or_more_before_the_new_one_is_removed_and_every_build_before_it() {
    let old = build("old", Some(NOW - 10 * DAY), &["assets/app-old.js"]);
    let replaced = build("replaced", Some(NOW - KEPT_FOR), &["assets/app-replaced.js"]);
    let young = build("young", Some(NOW - KEPT_FOR + 1), &["assets/app-young.js"]);
    let new = build("new", None, &["assets/app-new.js"]);
    let kept = generations::generations(&[young, replaced.clone(), old.clone()], &new, NOW);
    assert_eq!(ids(&kept), ["new", "young", "replaced"]);
    assert_eq!(generations::leftovers(&kept, &old.files), old.files);
    let kept = generations::generations(&[replaced, old], &new, NOW);
    assert_eq!(
        ids(&kept),
        ["new", "replaced"],
        "the build the new one replaces is kept however long it was current"
    );
}

#[test]
fn twenty_builds_before_the_current_one_are_kept_at_most_the_newest_of_them() {
    let before: Vec<Build> = (0..KEPT_MOST as u64 + 5)
        .map(|at| build(&format!("b{at}"), Some(NOW - (at + 1) * 60 * 60 * 1000), &[]))
        .collect();
    let kept = generations::generations(&before, &build("new", None, &[]), NOW);
    assert_eq!(kept.len(), 1 + KEPT_MOST);
    assert_eq!(kept[1..], before[..KEPT_MOST]);
}

#[test]
fn installing_the_same_build_again_pushes_none_out_and_going_back_keeps_what_it_replaces() {
    let one = build("one", Some(NOW - 3 * DAY), &[]);
    let two = build("two", Some(NOW - 2 * DAY), &[]);
    assert_eq!(ids(&generations::generations(&[two.clone(), one.clone()], &two, NOW)), ["two", "one"]);
    assert_eq!(ids(&generations::generations(&[two, one.clone()], &one, NOW)), ["one", "two"]);
}

#[test]
fn what_the_copy_keeps_is_read_back_as_it_was_written_and_a_list_from_before_is_one_build_made_now() {
    let builds = vec![
        build("two", Some(NOW), &["assets/app-2.js", "index.html"]),
        build("one", Some(NOW - DAY), &["assets/app-1.js"]),
    ];
    assert_eq!(generations::read(&generations::write(&builds)), builds);

    let from_before = generations::read("assets/app-1.js\nlib/chart.js\n");
    assert_eq!(from_before, [build("before", None, &["assets/app-1.js", "lib/chart.js"])]);
    let kept = generations::generations(&from_before, &build("two", None, &[]), NOW);
    assert_eq!(kept[1].at, Some(NOW), "and is kept a week from the install that read it");
}
