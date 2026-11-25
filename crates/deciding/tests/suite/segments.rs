//! What comes next for an item among the things done with it where it is,
//! what a segment is handed, what a command's ending comes to and what is
//! wrong with a list as written: each a function of what it is handed,
//! held by a table of cases that reaches every word it can answer.

use std::collections::BTreeSet;

use blueprint::ValueType;
use deciding::segments::{
    Asked, Came, IN_WORDS, RAN_WORDS, Ran, Run, RunIn, Said, Usual, WORDS, coming, fitted, next, ran, run_in,
    wrong,
};
use serde_json::{Map, Value, json};

/// A row of the reviewing column's list.
fn row(id: &str, title: &str, order: i64, does: &str) -> Said {
    Said { id: id.into(), title: title.into(), order, does: does.into(), ..Said::default() }
}

/// An item in `review`, whose usual list is a review that is begun and
/// heard from, with these rows.
fn asked(rows: Vec<Said>) -> Asked {
    Asked {
        rows,
        usual: vec![Usual { order: 10, does: "std/review".into(), spawned: true }],
        item: "shop-7".into(),
        found: true,
        status: "review".into(),
        here: "review".into(),
        moved: 4,
        ..Asked::default()
    }
}

/// The same, with the item at the row `id` (at `order`), which came to `came`.
fn ended(rows: Vec<Said>, id: &str, order: i64, came: &str) -> Asked {
    Asked { row: id.into(), order, came: came.into(), moves: 4, ..asked(rows) }
}

fn begins(rows: Vec<Said>) -> Asked {
    Asked { begin: true, ..asked(rows) }
}

/// A word, and of a segment that is run its name, its order and how
/// often it has been given again.
type Told = (String, String, i64, i64);

/// What a case expects: the word, and of a segment that is run its name,
/// its order and how often it has been given again.
fn told(came: &Came) -> Told {
    match came {
        Came::Run(Run { name, order, tries, .. }) => ("run".into(), name.clone(), *order, *tries),
        other => (other.word().into(), String::new(), 0, 0),
    }
}

fn run(name: &str, order: i64, tries: i64) -> Told {
    ("run".into(), name.into(), order, tries)
}

fn word(word: &str) -> Told {
    (word.into(), String::new(), 0, 0)
}

#[test]
fn what_comes_next_for_an_item_is_said_by_the_usual_list_and_the_rows_laid_into_it() {
    let check = || row("r1", "Tests", 5, "std/run");
    let lint = || row("r2", "Lint", 7, "shop/lint");
    let held = |mut row: Said, item: &str, labelled: &str, part_of: &str| {
        (row.item, row.labelled, row.part_of) = (item.into(), labelled.into(), part_of.into());
        row
    };
    let labelled = |mut asked: Asked, labels: &[&str]| {
        asked.labels = labels.iter().map(|label| label.to_string()).collect();
        asked
    };
    let cases: Vec<(&str, Asked, Told)> = vec![
        // Begun at the head.
        ("no row: the usual thing is begun", begins(vec![]), run("std/review", 10, 0)),
        ("a row before the usual thing is begun first", begins(vec![check()]), run("std/run", 5, 0)),
        ("of two rows the lower order", begins(vec![lint(), check()]), run("std/run", 5, 0)),
        (
            "a row at the usual order stands in for it",
            begins(vec![row("r3", "By hand", 10, "shop/sign-off")]),
            run("shop/sign-off", 10, 0),
        ),
        ("a row there that names nothing takes it out", begins(vec![row("r3", "None", 10, "")]), word("on")),
        (
            "which leaves what another row adds",
            begins(vec![row("r3", "None", 10, ""), check()]),
            run("std/run", 5, 0),
        ),
        (
            "a row naming what is usually begun here is one more of those",
            begins(vec![row("r3", "Council", 10, "std/review")]),
            run("std/review", 10, 0),
        ),
        // Held to some items.
        (
            "a row held to another item is passed over",
            begins(vec![held(check(), "shop-9", "", "")]),
            run("std/review", 10, 0),
        ),
        (
            "a row held to this item is its",
            begins(vec![held(check(), "shop-7", "", "")]),
            run("std/run", 5, 0),
        ),
        (
            "a row held to a label the item has not",
            begins(vec![held(check(), "", "payments", "")]),
            run("std/review", 10, 0),
        ),
        (
            "a row held to a label the item has",
            labelled(begins(vec![held(check(), "", "payments", "")]), &["docs", "payments"]),
            run("std/run", 5, 0),
        ),
        (
            "a row held to what the item is part of",
            Asked {
                part_of: "shop-features-2".into(),
                ..begins(vec![held(check(), "", "", "shop-features-2")])
            },
            run("std/run", 5, 0),
        ),
        (
            "held to two things, both have to be so",
            labelled(begins(vec![held(check(), "shop-9", "payments", "")]), &["payments"]),
            run("std/review", 10, 0),
        ),
        (
            "of two rows at one order the one held to more stands",
            labelled(
                begins(vec![
                    row("r4", "Any", 5, "shop/any"),
                    held(row("r5", "Payments", 5, "shop/pay"), "", "payments", ""),
                ]),
                &["payments"],
            ),
            run("shop/pay", 5, 0),
        ),
        (
            "and of two held to as much, the first read",
            begins(vec![row("r4", "Any", 5, "shop/any"), row("r5", "Other", 5, "shop/other")]),
            run("shop/any", 5, 0),
        ),
        // Begun where the chain says.
        (
            "begun at a row",
            Asked { row: "r2".into(), order: 7, ..begins(vec![check(), lint()]) },
            run("shop/lint", 7, 0),
        ),
        (
            "begun at a row that is gone: what is at its order",
            Asked { row: "gone".into(), order: 7, ..begins(vec![check(), lint()]) },
            run("shop/lint", 7, 0),
        ),
        (
            "or the next after it",
            Asked { row: "gone".into(), order: 6, ..begins(vec![check(), lint()]) },
            run("shop/lint", 7, 0),
        ),
        (
            "begun again keeps how often",
            Asked { row: "r1".into(), order: 5, tries: 1, ..begins(vec![check()]) },
            run("std/run", 5, 1),
        ),
        // What an ending comes to.
        ("on: the next of the list", ended(vec![check(), lint()], "r1", 5, "on"), run("shop/lint", 7, 0)),
        (
            "a segment that says no word came to on",
            ended(vec![check()], "r1", 5, ""),
            run("std/review", 10, 0),
        ),
        (
            "on at the last: the item goes on",
            ended(vec![row("r3", "By hand", 10, "shop/sign-off")], "r3", 10, "on"),
            word("on"),
        ),
        (
            "on from a row that was taken away meanwhile: what is after its order",
            ended(vec![lint()], "r1", 5, "on"),
            run("shop/lint", 7, 0),
        ),
        (
            "on from a row moved later in the list: what is after it now",
            ended(vec![Said { order: 8, ..check() }, lint()], "r1", 5, "on"),
            run("std/review", 10, 0),
        ),
        ("back", ended(vec![check()], "r1", 5, "back"), word("back")),
        ("left", ended(vec![check()], "r1", 5, "left"), word("left")),
        ("it could not be called", Asked { failed: true, ..ended(vec![check()], "r1", 5, "") }, word("left")),
        ("asks, where nobody takes that up", ended(vec![check()], "r1", 5, "asks"), word("left")),
        (
            "asks, where whoever asks does",
            Asked { asking: true, ..ended(vec![check()], "r1", 5, "asks") },
            word("asks"),
        ),
        ("again: the same once more", ended(vec![check()], "r1", 5, "again"), run("std/run", 5, 1)),
        (
            "again, a second time",
            Asked { tries: 1, ..ended(vec![check()], "r1", 5, "again") },
            run("std/run", 5, 2),
        ),
        (
            "again, as often as is allowed: left",
            Asked { tries: 2, ..ended(vec![check()], "r1", 5, "again") },
            word("left"),
        ),
        (
            "how often is the row's to say",
            Asked { tries: 2, ..ended(vec![Said { rounds: 3, ..check() }], "r1", 5, "again") },
            run("std/run", 5, 3),
        ),
        (
            "or whoever asks",
            Asked { tries: 1, most: 1, ..ended(vec![check()], "r1", 5, "again") },
            word("left"),
        ),
        (
            "again from a row that is gone: what is after it",
            ended(vec![lint()], "r1", 5, "again"),
            run("shop/lint", 7, 0),
        ),
        // How many things at most are called in one going through: the
        // asking that would call one more leaves the item for a person.
        (
            "the last thing a going through calls at most is called",
            Asked { nth: 6, most_done: 6, ..ended(vec![check(), lint()], "r1", 5, "on") },
            run("shop/lint", 7, 0),
        ),
        (
            "one more than that is not: the item is left for a person",
            Asked { nth: 7, most_done: 6, ..ended(vec![check(), lint()], "r1", 5, "on") },
            word("left"),
        ),
        (
            "and so for the same given again",
            Asked { nth: 3, most_done: 2, ..ended(vec![check()], "r1", 5, "again") },
            word("left"),
        ),
        (
            "or the first, where a going through does nothing at all",
            Asked { nth: 1, most_done: 1, row: "r1".into(), order: 5, ..begins(vec![check()]) },
            run("std/run", 5, 0),
        ),
        (
            "what is begun and heard from by whoever asks is not called, and not counted",
            Asked { nth: 7, most_done: 6, ..ended(vec![check()], "r1", 5, "on") },
            run("std/review", 10, 0),
        ),
        (
            "an item that goes on has nothing more called",
            Asked {
                nth: 9,
                most_done: 2,
                ..ended(vec![row("r3", "By hand", 10, "shop/sign-off")], "r3", 10, "on")
            },
            word("on"),
        ),
        (
            "nothing is held to a number where whoever asks says none",
            Asked { nth: 40, ..ended(vec![check(), lint()], "r1", 5, "on") },
            run("shop/lint", 7, 0),
        ),
        (
            "a row that names what goes through the list is not run inside it",
            begins(vec![row("r6", "Again", 5, "std/through")]),
            word("left"),
        ),
        // Nothing.
        ("the item is gone", Asked { found: false, ..begins(vec![check()]) }, word("nothing")),
        (
            "it has gone on",
            Asked { status: "done".into(), ..ended(vec![check()], "r1", 5, "on") },
            word("nothing"),
        ),
        (
            "it was moved since this began",
            Asked { moved: 6, ..ended(vec![check()], "r1", 5, "on") },
            word("nothing"),
        ),
        (
            "begun for a move that is not its last",
            Asked { moves: 2, ..begins(vec![check()]) },
            word("nothing"),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, expected) in cases {
        let came = next(&asked).unwrap_or_else(|why| panic!("{what}: {why}"));
        assert_eq!(told(&came), expected, "{what}");
        reached.insert(came.word());
    }
    let every: BTreeSet<&str> = WORDS.iter().copied().collect();
    assert_eq!(reached, every, "a word no case reaches is a word nothing holds");
}

#[test]
fn what_is_said_with_a_word_says_which_item_and_what_was_in_the_way() {
    let check = || row("r1", "Tests", 5, "std/run");
    let says = |asked: Asked| match next(&asked).unwrap() {
        Came::On { says } | Came::Back { says } | Came::Left { says } => says,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        says(Asked { says: "2 tests failed".into(), ..ended(vec![check()], "r1", 5, "back") }),
        "2 tests failed"
    );
    assert_eq!(
        says(Asked { failed: true, why: "no such blueprint".into(), ..ended(vec![check()], "r1", 5, "") }),
        "shop-7: Tests could not be done: no such blueprint"
    );
    assert_eq!(
        says(Asked { question: "Which currency?".into(), ..ended(vec![check()], "r1", 5, "asks") }),
        "shop-7: Tests asks, and it is left for a person: Which currency?"
    );
    assert_eq!(
        says(Asked {
            tries: 2,
            why: "nothing to commit as".into(),
            ..ended(vec![check()], "r1", 5, "again")
        }),
        "shop-7: Tests was given again 2 times and is left for a person: nothing to commit as"
    );
    // One with more to do than is done in one going through says how many
    // are, and how to raise that on the record that says it, or where.
    let lint = || row("r2", "Lint", 7, "shop/lint");
    let beyond = |most_of: &str| Asked {
        nth: 7,
        most_done: 6,
        most_of: most_of.into(),
        ..ended(vec![check(), lint()], "r1", 5, "on")
    };
    assert_eq!(
        says(beyond("shop-stages-2")),
        format!(
            "shop-7: there is more to do with it in review than is gone through at once (6 things at most), and \
             Lint was not begun: it is left for a person. Take a row of `segments` away, put two of them in one \
             blueprint, or raise how many are done: `{} record set shop-stages-2 most=7`",
            brand::CLI
        )
    );
    assert!(says(beyond("")).ends_with("or raise `most` on the row of `stages` that says how many are done"));
    assert!(
        says(Asked { most_done: 1, nth: 2, ..ended(vec![check(), lint()], "r1", 5, "on") })
            .contains("(1 thing at most)")
    );
    // And a word no segment ends with is refused, saying which there are.
    let why = next(&ended(vec![check()], "r1", 5, "maybe")).unwrap_err();
    assert!(why.contains("Tests came to `maybe` for shop-7"), "{why}");
}

fn mapping(said: Value) -> Map<String, Value> {
    said.as_object().cloned().unwrap_or_default()
}

#[test]
fn a_segment_is_handed_what_its_row_says_and_of_the_rest_only_what_it_declares() {
    let declared = vec![
        ("project".to_string(), ValueType::String),
        ("item".to_string(), ValueType::String),
        ("command".to_string(), ValueType::String),
        ("fails".to_string(), ValueType::String),
        ("times".to_string(), ValueType::Int),
        ("strict".to_string(), ValueType::Bool),
        ("channel".to_string(), ValueType::String),
        ("role".to_string(), ValueType::String),
    ];
    let run = |cells: Value, with: &[&str]| Run {
        title: "Tests".into(),
        name: "std/run".into(),
        cells: mapping(cells),
        with: with.iter().map(|one| one.to_string()).collect(),
        ..Run::default()
    };
    let own = mapping(json!({ "role": "reviewer", "picks": "first", "project": "other" }));
    let journey = mapping(json!({ "channel": "ch_1", "note": "from doing" }));
    let fit = |run: &Run| fitted(run, &own, &journey, &declared);

    // What the row says, by cell and in `with`, as the kind of thing each
    // param is; of the rest what is declared, and never the item itself.
    assert_eq!(
        fit(&run(json!({ "command": "cargo test" }), &["fails=left", "times = 3", "strict=yes"])).unwrap(),
        mapping(json!({
            "command": "cargo test", "fails": "left", "times": 3, "strict": true, "channel": "ch_1", "role": "reviewer",
        }))
    );
    // What the row says stands over what whoever asks says of itself.
    assert_eq!(fit(&run(json!({ "role": "tester" }), &[])).unwrap()["role"], "tester");
    // And what is so of this going through stands over that too: both
    // may say a thing by one name.
    let both = mapping(json!({ "channel": "the stage's own", "role": "reviewer" }));
    assert_eq!(fitted(&run(json!({}), &[]), &both, &journey, &declared).unwrap()["channel"], "ch_1");
    // How many things its stage does at most in one going through is how
    // the list is gone through, and handed to no segment that counts
    // something else by that name; a row may still say one in `with`.
    let counts = [&declared[..], &[("most".to_string(), ValueType::Int)]].concat();
    let staged = mapping(json!({ "most": 5, "role": "reviewer" }));
    let unhanded = fitted(&run(json!({}), &[]), &staged, &Map::new(), &counts).unwrap();
    assert_eq!(unhanded.get("most"), None, "{unhanded:?}");
    assert_eq!(fitted(&run(json!({}), &["most=3"]), &staged, &Map::new(), &counts).unwrap()["most"], 3);

    let refused: Vec<(Run, &str)> = vec![
        (
            run(json!({ "seats": 3 }), &[]),
            "Tests says `seats`, and `std/run` takes nothing called that: what it takes is channel, command, fails, \
             role, strict, times",
        ),
        (run(json!({}), &["colour=red"]), "Tests says `colour`, and `std/run` takes nothing called that"),
        (run(json!({}), &["item=shop-9"]), "Tests says `item`, and `std/run` takes nothing called that"),
        (run(json!({}), &["fails"]), "Tests says `fails` in `with`, where each thing is `name=value`"),
        (run(json!({}), &["times=often"]), "Tests says `times` is `often`, and it is a whole number"),
        (run(json!({}), &["strict=maybe"]), "Tests says `strict` is `maybe`, and it is `true` or `false`"),
    ];
    for (run, expected) in refused {
        let why = fit(&run).unwrap_err();
        assert!(why.starts_with(expected), "{why}");
    }
    // A number of what is so of this going through is handed to a param
    // that is text as its words: which coming of the item it is.
    let asked = [&declared[..], &[("asking".to_string(), ValueType::String)]].concat();
    let coming = mapping(json!({ "asking": 1439 }));
    assert_eq!(fitted(&run(json!({}), &[]), &Map::new(), &coming, &asked).unwrap()["asking"], "1439");
    // A time is handed only where it is one, as a row's cell says it.
    let timed = [&declared[..], &[("warn_after".to_string(), ValueType::String)]].concat();
    let fit_timed = |cells: Value| fitted(&run(cells, &[]), &Map::new(), &Map::new(), &timed);
    assert_eq!(fit_timed(json!({ "warn_after": "1h30m" })).unwrap()["warn_after"], "1h30m");
    assert_eq!(
        fit_timed(json!({ "warn_after": "soon" })).unwrap_err(),
        "Tests says `warn_after` is `soon`, which is no time: write it like `20m` or `1h30m`"
    );
    // One that declares nothing beside the item says so.
    let bare = vec![("project".to_string(), ValueType::String), ("item".to_string(), ValueType::String)];
    let why = fitted(&run(json!({ "command": "true" }), &[]), &own, &journey, &bare).unwrap_err();
    assert!(why.ends_with("it takes nothing more than the item"), "{why}");
}

#[test]
fn a_list_is_handed_to_a_segments_param_that_is_text_as_its_things_joined_by_a_comma() {
    let declared = vec![("labels".to_string(), ValueType::String), ("tags".to_string(), ValueType::List)];
    let run = Run { title: "Tests".into(), name: "std/run".into(), ..Run::default() };
    let cases: Vec<(&str, Value, Value)> = vec![
        ("several", json!(["docs", "cart"]), json!("docs,cart")),
        ("one", json!(["docs"]), json!("docs")),
        ("none", json!([]), json!("")),
        ("text as it is", json!("docs"), json!("docs")),
    ];
    for (what, said, handed) in cases {
        let journey = mapping(json!({ "labels": said.clone(), "tags": said.clone() }));
        let fit = fitted(&run, &Map::new(), &journey, &declared).unwrap();
        assert_eq!(fit["labels"], handed, "{what}");
        assert_eq!(fit["tags"], said, "{what}: a list to a list as it is");
    }
}

#[test]
fn what_a_segment_is_handed_of_an_items_coming_says_its_labels_and_its_column_where_whoever_asks_said_neither()
 {
    let declared = vec![("here".to_string(), ValueType::String), ("labels".to_string(), ValueType::String)];
    let run = Run { title: "Merge".into(), name: "std/merge".into(), ..Run::default() };
    let labels = vec!["docs".to_string()];
    // (what, what whoever asks said of the coming, what is handed)
    let cases: Vec<(&str, Value, Value)> = vec![
        // As the item came to be looked at: nothing said of either.
        ("said nothing", json!({ "title": "Add a cart" }), json!({ "here": "review", "labels": "docs" })),
        ("said its column", json!({ "here": "staging" }), json!({ "here": "staging", "labels": "docs" })),
        ("said its labels", json!({ "labels": ["cart"] }), json!({ "here": "review", "labels": "cart" })),
    ];
    for (what, said, handed) in cases {
        let came = coming(&mapping(said), &labels, "review");
        let fit = fitted(&run, &Map::new(), &came, &declared).unwrap();
        assert_eq!(Value::Object(fit), handed, "{what}");
    }
}

#[test]
fn where_a_command_is_run_is_where_the_item_is_worked_or_the_projects_folder_as_its_row_says() {
    let asked = |within: &str, dir: &str, there: bool, worktree: bool| RunIn {
        item: "shop-7".into(),
        run: "cargo fmt".into(),
        within: within.into(),
        dir: dir.into(),
        there,
        worktree,
    };
    let none = "shop-7: `cargo fmt` was not run, for it has no place of its own here: it is not worked yet, \
                or its place was tidied away. A row that says `in=project` in `with` has it run in the \
                project's folder; it is left for a person";
    let cases: Vec<(&str, RunIn, Result<&str, &str>)> = vec![
        (
            "where it is worked, where nothing is said",
            asked("", ".lanes/shop-7", true, true),
            Ok(".lanes/shop-7"),
        ),
        ("or where the row says so", asked("card", ".lanes/shop-7", true, true), Ok(".lanes/shop-7")),
        ("in the project's folder, where it says so", asked("project", ".lanes/shop-7", true, true), Ok("")),
        ("and where it has no place yet", asked("project", "", false, true), Ok("")),
        ("worked in the project itself, where it is no repository", asked("card", "", true, true), Ok("")),
        ("or where it is worked in no place of its own", asked("card", "", false, false), Ok("")),
        ("not run, where it has no place yet and would have one", asked("card", "", false, true), Err(none)),
        (
            "nor where the row says something else",
            asked("home", "", true, true),
            Err(
                "shop-7: `cargo fmt` was not run, for its row says `in` is `home`: it is `card`, where it is \
                 worked, or `project`, the project's own folder; it is left for a person",
            ),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, expected) in cases {
        let came = run_in(&asked);
        assert_eq!(came.as_deref().map_err(String::as_str), expected, "{what}");
        if came.is_ok() {
            reached.insert(if asked.within.is_empty() { "card".to_string() } else { asked.within.clone() });
        }
    }
    assert_eq!(reached, IN_WORDS.iter().map(|word| word.to_string()).collect::<BTreeSet<_>>());
}

#[test]
fn what_with_says_is_more_of_the_rows_own_and_never_a_cell_of_its_or_what_its_stage_says_of_itself() {
    let declared: Vec<(String, ValueType)> =
        ["project", "item", "command", "role", "ends", "yes", "no", "key", "remote", "agent", "fails"]
            .iter()
            .map(|name| (name.to_string(), ValueType::String))
            .collect();
    let run = |with: &[&str]| Run {
        title: "Keyed".into(),
        name: "std/work".into(),
        with: with.iter().map(|one| one.to_string()).collect(),
        ..Run::default()
    };
    // What the stage says of itself, from its row and what the fleet was told.
    let own = mapping(json!({ "role": "worker", "key": "", "remote": "origin" }));
    let journey = mapping(json!({ "agent": "" }));
    let fit = |with: &[&str]| fitted(&run(with), &own, &journey, &declared);
    // More of the row's own, and what stands over what this going through is.
    assert_eq!(
        fit(&["fails=left", "agent=shop-worker-2"]).unwrap(),
        mapping(
            json!({ "fails": "left", "agent": "shop-worker-2", "role": "worker", "key": "", "remote": "origin" })
        )
    );
    let cell =
        |name: &str| format!("Keyed says `{name}` in `with`, which is a cell of its own: say it there");
    let stage = |name: &str| {
        format!(
            "Keyed says `{name}` in `with`, which its stage says of itself: a row says what more its own \
             blueprint is handed, and not what its stage, or the fleet over it, says"
        )
    };
    for (with, expected) in [
        ("command=rm -rf /", cell("command")),
        ("role=ops", cell("role")),
        ("ends=passed=on", cell("ends")),
        ("yes=ship", cell("yes")),
        ("no=hold", cell("no")),
        ("key=deploy", stage("key")),
        ("remote=elsewhere", stage("remote")),
    ] {
        assert_eq!(fit(&[with]).unwrap_err(), expected, "{with}");
    }
    // A cell said as a cell is the row's, as it always was.
    let celled = Run { cells: mapping(json!({ "role": "ops" })), ..run(&[]) };
    assert_eq!(fitted(&celled, &own, &journey, &declared).unwrap()["role"], "ops");
}

#[test]
fn what_a_turn_is_offered_and_how_many_look_at_once_and_by_which_words_are_cells_of_a_row_and_one_left_empty_is_not_said()
 {
    let row = json!({
        "id": "shop-segments-1", "title": "Second look", "stage": "work", "order": 15, "does": "std/turn",
        "role": "reviewer", "seats": 2, "needed": 2, "ends": ["passed=on failed=back"], "yes": "ship",
        "no": "hold", "command": "",
    });
    let said = Said::read(&row);
    assert_eq!(
        said.cells,
        mapping(json!({
            "role": "reviewer", "seats": 2, "needed": 2, "ends": ["passed=on failed=back"], "yes": "ship",
            "no": "hold",
        }))
    );
    // A list emptied in the client says nothing, as an emptied text does:
    // a segment that takes no such thing is not refused for it.
    let emptied = Said::read(&json!({ "title": "Tests", "does": "std/run", "ends": [], "seats": "" }));
    assert!(emptied.cells.is_empty(), "{:?}", emptied.cells);
    // And a turn is handed them as the kinds its params are.
    let declared: Vec<(String, ValueType)> = vec![
        ("project".into(), ValueType::String),
        ("item".into(), ValueType::String),
        ("role".into(), ValueType::String),
        ("seats".into(), ValueType::Int),
        ("needed".into(), ValueType::Int),
        ("ends".into(), ValueType::List),
        ("yes".into(), ValueType::String),
        ("no".into(), ValueType::String),
    ];
    let run =
        Run { title: "Second look".into(), name: "std/turn".into(), cells: said.cells, ..Run::default() };
    assert_eq!(
        fitted(&run, &Map::new(), &Map::new(), &declared).unwrap(),
        mapping(json!({
            "role": "reviewer", "seats": 2, "needed": 2, "ends": ["passed=on failed=back"], "yes": "ship",
            "no": "hold",
        }))
    );
    let typed =
        Run { cells: mapping(json!({ "ends": "passed=on, failed=back", "seats": "2" })), ..run.clone() };
    assert_eq!(
        fitted(&typed, &Map::new(), &Map::new(), &declared).unwrap(),
        mapping(json!({ "ends": ["passed=on", "failed=back"], "seats": 2 }))
    );
}

#[test]
fn what_a_commands_ending_comes_to_is_said_by_how_it_went_and_what_its_row_says_of_failing() {
    let asked = |ok: bool, fails: &str, printed: &str| Ran {
        item: "shop-7".into(),
        title: "Tests".into(),
        run: "cargo test".into(),
        ok,
        fails: fails.into(),
        printed: printed.into(),
        why: String::new(),
        unrun: String::new(),
    };
    let cases: Vec<(&str, Ran, &str, &str)> = vec![
        ("it went well", asked(true, "", "ok"), "on", ""),
        (
            "it did not: back, where nothing is said",
            asked(false, "", "2 failed"),
            "back",
            "`cargo test` did not pass: 2 failed",
        ),
        (
            "or where the row says so",
            asked(false, "back", "2 failed"),
            "back",
            "`cargo test` did not pass: 2 failed",
        ),
        (
            "left, where the row says so",
            asked(false, "left", "2 failed"),
            "left",
            "shop-7: `cargo test` did not pass, and it is left for a person: 2 failed",
        ),
        (
            "it printed nothing",
            asked(false, "", " \n"),
            "back",
            "`cargo test` did not pass: it printed nothing",
        ),
        (
            "it could not be run",
            Ran { why: "no such file".into(), ..asked(false, "", "") },
            "back",
            "`cargo test` did not pass: no such file",
        ),
        (
            "no command was said",
            Ran { run: " ".into(), ..asked(true, "", "") },
            "left",
            "shop-7: Tests says no command to run (`command`), and it is left for a person",
        ),
        (
            "it was not run at all, whatever the row says of failing",
            Ran { unrun: "shop-7: `cargo test` was not run".into(), ..asked(false, "back", "") },
            "left",
            "shop-7: `cargo test` was not run",
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, came, says) in cases {
        let (word, said) = ran(&asked).unwrap_or_else(|why| panic!("{what}: {why}"));
        assert_eq!((word, said.as_str()), (came, says), "{what}");
        reached.insert(word);
    }
    assert_eq!(reached, RAN_WORDS.iter().copied().collect::<BTreeSet<_>>());
    // Only the end of a great deal printed is said.
    let long = format!("{}the end", "x".repeat(5000));
    let (_, said) = ran(&asked(false, "", &long)).unwrap();
    assert!(said.len() < 1300 && said.ends_with("the end"), "{}", said.len());
    let why = ran(&asked(false, "retry", "")).unwrap_err();
    assert!(why.starts_with("Tests says `fails` is `retry`"), "{why}");
}

#[test]
fn what_is_wrong_with_a_list_as_written_is_said_a_sentence_for_each_row_that_will_never_be_done() {
    let usual = vec![Usual { order: 10, does: "std/review".into(), spawned: true }];
    let rows = vec![
        row("r1", "Tests", 5, "std/run"),
        row("r2", "Notes", 15, "shop/notes"),
        row("r3", "Council", 10, "std/review"),
        row("r4", "Empty", 7, ""),
        row("r5", "None", 10, ""),
    ];
    // A row after what is usually begun, and one that names it, are done; a row
    // that names nothing where nothing is usual is all that is never done.
    let said = wrong(&rows, &usual, "review", true);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].starts_with("The row Empty of `segments` names nothing to do"), "{}", said[0]);
    assert!(wrong(&rows[..1], &usual, "review", true).is_empty());
    // A row that names what goes through such a list is said, whichever it
    // names, and a round of a project's own under a name of its own is not.
    for does in ["std/through", "std/segments", "std/lane-round"] {
        let said = wrong(&[row("r6", "Again", 15, does)], &usual, "review", true);
        assert_eq!(
            said,
            [format!(
                "The row Again of `segments` names `{does}`, which is what goes through such a list itself: \
                 named in it, it would go through the list inside itself, and it is not run. Name the thing \
                 to be done."
            )]
        );
    }
    assert!(wrong(&[row("r6", "Round", 10, "shop/round-of-before")], &usual, "review", true).is_empty());
    // Where cards are not gone through such a list at all, every row that is for the column is said.
    let said = wrong(&rows[..1], &[], "doing", false);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].starts_with("The row Tests of `segments` is for doing"), "{}", said[0]);
}

/// An item being worked: taken out of `ready` into `doing`, where the
/// usual list is the work, the keeping of it and, where that is asked
/// for, the sending.
fn worked(rows: Vec<Said>) -> Asked {
    let usual = |order: i64, does: &str| Usual { order, does: does.into(), spawned: false };
    Asked {
        rows,
        usual: vec![usual(10, "std/work"), usual(20, "std/keep"), usual(30, "std/send")],
        item: "shop-7".into(),
        found: true,
        status: "doing".into(),
        here: "ready".into(),
        doing: "doing".into(),
        asking: true,
        ..Asked::default()
    }
}

#[test]
fn a_list_that_is_gone_through_for_an_item_in_hand_is_the_usual_things_with_the_rows_between_them() {
    let tidy = || row("r1", "Tidy", 15, "std/run");
    let docs = || Said { labelled: "docs".into(), ..row("r2", "Docs", 10, "shop/write") };
    let at = |rows: Vec<Said>, order: i64, came: &str| Asked { order, came: came.into(), ..worked(rows) };
    let cases: Vec<(&str, Asked, Told)> = vec![
        ("it begins with the work", Asked { begin: true, ..worked(vec![]) }, run("std/work", 10, 0)),
        ("worked, it is kept", at(vec![], 10, "on"), run("std/keep", 20, 0)),
        ("kept, it is sent", at(vec![], 20, "on"), run("std/send", 30, 0)),
        ("sent, it goes on", at(vec![], 30, "on"), word("on")),
        (
            "a row between the work and the keeping is done between them",
            at(vec![tidy()], 10, "on"),
            run("std/run", 15, 0),
        ),
        ("and the keeping comes after it", at(vec![tidy()], 15, "on"), run("std/keep", 20, 0)),
        (
            "a row at the place of the work, held to a label, stands in for it for an item that has the label",
            Asked { begin: true, labels: vec!["docs".into()], ..worked(vec![docs()]) },
            run("shop/write", 10, 0),
        ),
        (
            "and not for one that has not",
            Asked { begin: true, ..worked(vec![docs()]) },
            run("std/work", 10, 0),
        ),
        (
            "answered, it begins where it asked",
            Asked { begin: true, row: "r1".into(), order: 15, ..worked(vec![tidy()]) },
            run("std/run", 15, 0),
        ),
        ("what works it asks, and whoever asks takes that up", at(vec![], 10, "asks"), word("asks")),
        (
            "sent back by something done with it, it goes back to whoever works it",
            at(vec![tidy()], 15, "back"),
            word("back"),
        ),
        (
            "sent back as often as it may be given again, it is left for a person",
            Asked { given: 2, most_given: 2, ..at(vec![tidy()], 15, "back") },
            word("left"),
        ),
        (
            "and once fewer, it goes back",
            Asked { given: 1, most_given: 2, ..at(vec![tidy()], 15, "back") },
            word("back"),
        ),
        ("what was to be done found it gone", at(vec![], 10, "nothing"), word("nothing")),
        (
            "neither in its column nor in hand, it is no longer this list's",
            Asked { status: "review".into(), ..at(vec![], 10, "on") },
            word("nothing"),
        ),
    ];
    for (name, asked, expected) in cases {
        assert_eq!(told(&next(&asked).unwrap()), expected, "{name}");
    }
    let left = next(&Asked {
        given: 2,
        most_given: 2,
        says: "it is not laid out".into(),
        ..at(vec![tidy()], 15, "back")
    });
    let Ok(Came::Left { says }) = left else { panic!("{left:?}") };
    assert_eq!(
        says,
        "shop-7: Tidy sent it back, and it has been given again 2 times already: it is left for a person. it is not laid out"
    );
}

#[test]
fn what_a_segment_gave_out_is_read_as_the_word_it_ended_with_and_a_whole_round_of_before_as_what_it_did() {
    use deciding::segments::{Gave, gave};
    let read = |out: Value| gave(&mapping(out));
    let word = |came: &str| Gave { came: came.into(), ..Gave::default() };
    // One that gives no word came to `on`, which `next` reads the empty word as.
    assert_eq!(read(json!({})), word(""));
    assert_eq!(read(json!({ "came": "on", "by": "shop-worker-1" })), word("on"));
    assert_eq!(
        read(json!({ "came": "asks", "asker": "shop-worker-1", "question": "Which total?" })),
        Gave { asker: "shop-worker-1".into(), question: "Which total?".into(), ..word("asks") }
    );
    // What is said as an item goes back is what was said, or why.
    assert_eq!(
        read(json!({ "came": "back", "why": "index.lock is in the way" })),
        Gave {
            says: "index.lock is in the way".into(),
            why: "index.lock is in the way".into(),
            ..word("back")
        }
    );
    assert_eq!(
        read(json!({ "came": "back", "says": "2 failed", "why": "x" })),
        Gave { says: "2 failed".into(), why: "x".into(), ..word("back") }
    );
    // A whole round as it was: it handed its item on itself.
    let round = |asked: bool, again: bool| json!({ "available": true, "asked": asked, "again": again, "asker": "shop-worker-1", "question": "Which?", "why": "git said" });
    assert_eq!(read(round(false, false)).came, "nothing");
    assert_eq!(read(round(true, false)).came, "asks");
    assert_eq!(read(round(true, false)).question, "Which?");
    assert_eq!(read(round(false, true)).came, "back");
    assert_eq!(read(round(false, true)).says, "git said");
    // And one that never got as far as asking anybody gives nothing either.
    assert_eq!(read(json!({ "available": null, "asked": false })).came, "nothing");
}

#[test]
fn what_is_kept_of_a_command_that_was_run_is_a_run_id_the_day_it_began_whether_it_passed_how_long_and_the_last_it_printed()
 {
    use deciding::segments::{LAST_LINES, MOST_PRINTED, RESULTS, Recorded, recorded};
    let asked = |ok: bool, code: Option<i64>| Recorded {
        item: "shop-7".into(),
        run: " cargo test ".into(),
        ok,
        code,
        duration_ms: 2500,
        printed: "running 3 tests\nok".into(),
        also: String::new(),
        why: String::new(),
        commit: "0123abcd".into(),
    };
    // 2026-11-22 14:03:12.500 UTC, as it ended.
    let now = 1_795_356_192_500;
    let cases: Vec<(&str, Recorded, Value)> = vec![
        (
            "it passed",
            asked(true, Some(0)),
            json!({
                "title": "20261122-140310-shop-7", "day": "2026-11-22", "time": "14:03:10 UTC", "card": "shop-7", "result": "passed", "code": 0,
                "seconds": 2.5, "command": "cargo test", "commit": "0123abcd", "printed": "running 3 tests\nok",
            }),
        ),
        (
            "it failed, with what it printed where errors go after the rest",
            Recorded { also: "1 failed".into(), ..asked(false, Some(101)) },
            json!({
                "title": "20261122-140310-shop-7", "day": "2026-11-22", "time": "14:03:10 UTC", "card": "shop-7", "result": "failed", "code": 101,
                "seconds": 2.5, "command": "cargo test", "commit": "0123abcd",
                "printed": "running 3 tests\nok\n1 failed",
            }),
        ),
        (
            "it ended with no code and ran in no repository: neither is said",
            Recorded { commit: " ".into(), ..asked(false, None) },
            json!({
                "title": "20261122-140310-shop-7", "day": "2026-11-22", "time": "14:03:10 UTC", "card": "shop-7", "result": "failed",
                "seconds": 2.5, "command": "cargo test", "printed": "running 3 tests\nok",
            }),
        ),
        (
            "it could not be run, and printed nothing: why is what it printed",
            Recorded {
                printed: " \n".into(),
                why: "the working directory does not exist".into(),
                duration_ms: 0,
                ..asked(false, None)
            },
            json!({
                "title": "20261122-140312-shop-7", "day": "2026-11-22", "time": "14:03:12 UTC", "card": "shop-7", "result": "failed",
                "seconds": 0.0, "command": "cargo test", "commit": "0123abcd",
                "printed": "the working directory does not exist",
            }),
        ),
    ];
    let mut reached = BTreeSet::new();
    for (what, asked, row) in cases {
        let kept = Value::Object(recorded(&asked, now));
        assert_eq!(kept, row, "{what}");
        reached.insert(kept["result"].as_str().unwrap().to_string());
    }
    assert_eq!(reached, RESULTS.iter().map(|word| word.to_string()).collect::<BTreeSet<_>>());

    // Only the last lines of a great deal printed are kept, and no more
    // than so many characters of them, cut where a line begins.
    let lines: Vec<String> = (1..=100).map(|n| format!("line {n}")).collect();
    let many = Recorded { printed: lines.join("\n"), ..asked(true, Some(0)) };
    let kept = recorded(&many, now)["printed"].as_str().unwrap().to_string();
    assert_eq!(kept.lines().count(), LAST_LINES);
    assert!(
        kept.starts_with(&format!("line {}\n", 101 - LAST_LINES)) && kept.ends_with("line 100"),
        "{kept}"
    );
    let long: Vec<String> = (1..=10).map(|n| format!("{n} {}", "x".repeat(1000))).collect();
    let wide = Recorded { printed: long.join("\n"), ..asked(false, Some(1)) };
    let kept = recorded(&wide, now)["printed"].as_str().unwrap().to_string();
    assert!(kept.chars().count() <= MOST_PRINTED && kept.starts_with("8 x"), "{}", &kept[..20]);
    assert!(kept.ends_with(&"x".repeat(10)));
}
