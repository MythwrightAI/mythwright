//! Which of the blueprints a library has for no project runs which.

use blueprint::{ActionSpec, ValueType};
use texts::library::{Library, MemoryLibrary};
use texts::listed::{APPLIED, listed};

fn library() -> MemoryLibrary {
    MemoryLibrary::with(&[
        ("std/slot", "blueprint: std/slot\nsteps:\n  watched:\n    call: shop/watched\n"),
        (
            "shop/host",
            "blueprint: shop/host\nsteps:\n  one:\n    call: std/slot\n  two:\n    call: std/slot\n",
        ),
        ("shop/watched", "blueprint: shop/watched\nsteps:\n  one:\n    do: work.give\n"),
        ("shop/rule", "blueprint: shop/rule\nparams:\n  who: { type: string, default: nobody }\n"),
        ("shop/broken", "blueprint: shop/broken\nsteps: [\n"),
        // Words a text links are no blueprint, though they are in the library.
        ("shop/prompts/told.md", "Do as `shop/host` does."),
    ])
}

fn actions() -> Vec<ActionSpec> {
    vec![
        ActionSpec::new("work.give", "Give work.").arg("to", ValueType::String, "whom").reaching("shop/rule"),
    ]
}

#[test]
fn what_each_runs_is_read_from_the_texts_there_are_each_name_once() {
    let found = listed(&library(), &actions());
    let runs: Vec<(&str, Vec<&str>)> = found
        .runs
        .iter()
        .map(|(name, runs)| (name.as_str(), runs.iter().map(String::as_str).collect()))
        .collect();
    assert_eq!(
        runs,
        [
            ("shop/broken", vec![]),
            ("shop/host", vec!["std/slot"]),
            ("shop/rule", vec![]),
            ("shop/watched", vec!["shop/rule"]),
            ("std/slot", vec!["shop/watched"]),
        ],
        "every blueprint is listed, one that does not read too; a linked file is none"
    );
    // What somebody applies though something runs it is said only of what is there.
    assert_eq!(found.applied, ["std/slot"]);
    assert_eq!(found.tops(), ["shop/broken", "shop/host", "std/slot"]);
    assert_eq!(found.parts("shop/host"), ["shop/rule", "shop/watched", "std/slot"]);
    assert_eq!(found.part_of("shop/rule"), ["shop/host", "std/slot"]);
    assert_eq!(found.part_of("std/slot"), ["shop/host"]);
    assert!(found.part_of("shop/host").is_empty());
}

#[test]
fn nothing_is_kept_so_a_text_that_changes_is_in_the_next_answer() {
    let library = library();
    library.put("shop/host", "blueprint: shop/host\nsteps:\n  one: { do: noop }\n");
    library.put("shop/more", "blueprint: shop/more\nuses:\n  hosted: shop/host\n");
    let found = listed(&library, &actions());
    assert_eq!(found.runs["shop/host"], [] as [&str; 0]);
    assert_eq!(found.runs["shop/more"], ["shop/host"]);
    assert_eq!(
        found.tops(),
        ["shop/broken", "shop/more", "std/slot"],
        "nothing runs the slot now, and it is still applied"
    );
    // With no action described, what only an action knows is nobody's part.
    assert_eq!(listed(&library, &[]).runs["shop/watched"], [] as [&str; 0]);
    assert!(library.names(None).iter().all(|name| !name.ends_with(".md")));
}

#[test]
fn what_is_applied_though_something_runs_it_is_a_few_names_each_with_why() {
    let standard = texts::library::standard();
    let names = standard.names(None);
    assert!(APPLIED.len() <= 4, "a line here is an exception: {}", APPLIED.len());
    for (name, why) in APPLIED {
        assert!(names.iter().any(|one| one == name), "`{name}` is a blueprint that ships");
        assert!(why.len() > 20 && !why.ends_with('.'), "`{name}` says why in a few words: {why}");
    }
    let mut sorted: Vec<&str> = APPLIED.iter().map(|(name, _)| *name).collect();
    sorted.dedup();
    assert!(sorted.is_sorted() && sorted.len() == APPLIED.len(), "each once, by name");
}
