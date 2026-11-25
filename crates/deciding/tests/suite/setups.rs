//! What a project is set up with, as rows: a table of cases for reading
//! them, for what a request asks of them and for what each row comes to
//! as it is reached.

use deciding::setups::{Came, Reached, Row, by_hand, came, chosen, refused, rows};
use serde_json::{Value, json};

fn usual() -> Vec<Row> {
    rows(&json!([
        { "name": "board", "title": "Board", "said": "a board", "blueprint": "std/board", "table": "board" },
        { "name": "fleet", "said": "a fleet", "blueprint": "fleet", "needs": ["board"], "why": "a fleet works a board",
          "told": ["project", "channel"] },
        { "name": "docs", "blueprint": "std/docs" },
        { "name": "features", "blueprint": "std/features", "on": false, "needs": ["board", "fleet"],
          "told": ["project"], "offered": "board" },
    ]))
    .unwrap()
}

fn names(rows: &[Row]) -> Vec<(&str, bool)> {
    rows.iter().map(|row| (row.name.as_str(), row.on)).collect()
}

#[test]
fn a_row_says_its_name_and_blueprint_and_the_rest_is_what_it_is_unless_said() {
    let row = Row::read(&json!({ "name": "notes", "blueprint": "std/docs" })).unwrap();
    assert_eq!(
        row.value(),
        json!({
            "name": "notes", "title": "notes", "said": "notes", "says": "", "blueprint": "std/docs", "on": true,
            "needs": [], "why": "", "table": "", "told": [], "offered": "adding", "with": {},
        })
    );
    // Read back as it is handed out, it is the same row.
    assert_eq!(Row::read(&row.value()).unwrap(), row);
    let titled =
        Row::read(&json!({ "name": "notes", "title": "Field notes", "blueprint": "std/docs" })).unwrap();
    assert_eq!((titled.title.as_str(), titled.said.as_str()), ("Field notes", "field notes"));
}

#[test]
fn a_row_that_cannot_be_read_is_refused_saying_why() {
    let cases: &[(&str, Value, &str)] = &[
        ("no mapping", json!("board"), "a row is a mapping"),
        ("no name", json!({ "blueprint": "std/board" }), "a row says its `name`"),
        (
            "a name with capitals",
            json!({ "name": "Board", "blueprint": "std/board" }),
            "`Board` is no name for a row",
        ),
        ("no blueprint", json!({ "name": "board" }), "`blueprint` of `board` is the name of the blueprint"),
        (
            "a blueprint with a space",
            json!({ "name": "board", "blueprint": "std board" }),
            "`blueprint` of `board`",
        ),
        (
            "a word no row says",
            json!({ "name": "board", "blueprint": "std/board", "colour": "red" }),
            "`board` says `colour`",
        ),
        (
            "on as words",
            json!({ "name": "board", "blueprint": "std/board", "on": "yes" }),
            "`on` of `board` is true or false",
        ),
        (
            "needs as words",
            json!({ "name": "board", "blueprint": "std/board", "needs": "fleet" }),
            "`needs` of `board` is a list",
        ),
        (
            "with as words",
            json!({ "name": "board", "blueprint": "std/board", "with": "x" }),
            "`with` of `board` is what",
        ),
        (
            "handed what it is told",
            json!({ "name": "fleet", "blueprint": "fleet", "told": ["project"], "with": { "project": "x" } }),
            "`project` of `fleet` is the project's",
        ),
    ];
    for (what, row, says) in cases {
        let refused = Row::read(row).unwrap_err();
        assert!(refused.contains(says), "{what}: {refused}");
    }
}

#[test]
fn a_list_of_rows_has_no_name_twice_and_nothing_a_row_needs_after_it() {
    assert!(rows(&json!({ "name": "board" })).unwrap_err().contains("the rows are a list"));
    let twice =
        json!([{ "name": "board", "blueprint": "std/board" }, { "name": "board", "blueprint": "std/docs" }]);
    assert!(rows(&twice).unwrap_err().contains("`board` is the name of two rows"));
    let after = json!([
        { "name": "fleet", "blueprint": "fleet", "needs": ["board"] },
        { "name": "board", "blueprint": "std/board" },
    ]);
    assert!(rows(&after).unwrap_err().contains("`fleet` needs `board`, which is itself or comes after it"));
    let itself = json!([{ "name": "board", "blueprint": "std/board", "needs": ["board"] }]);
    assert!(rows(&itself).is_err());
    // A need no row gives is what the project may have before, and is not refused.
    let absent = json!([{ "name": "fleet", "blueprint": "fleet", "needs": ["board"] }]);
    assert_eq!(rows(&absent).unwrap().len(), 1);
    assert_eq!(rows(&json!([])).unwrap(), Vec::<Row>::new());
}

#[test]
fn what_a_request_asks_for_is_on_and_the_rest_off_in_the_order_of_the_rows_and_then_its_own() {
    let rows = usual();
    let off = |on: &[&str]| -> Vec<(&'static str, bool)> {
        ["board", "fleet", "docs", "features"].iter().map(|name| (*name, on.contains(name))).collect()
    };
    type Case<'a> = (&'a str, Value, Vec<(&'a str, bool)>);
    let cases: Vec<Case> = vec![
        ("nothing", json!([]), off(&[])),
        ("by names, whatever order they were named in", json!(["docs", "board"]), off(&["board", "docs"])),
        ("one that is off unless said, said", json!(["features"]), off(&["features"])),
        ("one named twice, once", json!(["board", "board"]), off(&["board"])),
        ("a row of its own after them", json!([{ "name": "notes", "blueprint": "std/docs" }, "board"]), {
            let mut all = off(&["board"]);
            all.push(("notes", true));
            all
        }),
    ];
    for (what, asked, meant) in cases {
        let chosen = chosen(&rows, "set_up", asked.as_array().unwrap()).unwrap();
        assert_eq!(names(&chosen), meant, "{what}");
    }
    // What its blueprint is handed is laid over what the row says.
    let handed = json!([{ "name": "board", "with": { "columns": ["a"] } }]);
    let board = &chosen(&rows, "set_up", handed.as_array().unwrap()).unwrap()[0];
    assert_eq!(board.with, json!({ "columns": ["a"] }).as_object().unwrap().clone());
}

#[test]
fn what_a_request_cannot_ask_for_is_refused_saying_what_may_be() {
    let rows = usual();
    let cases: &[(&str, Value, &str)] = &[
        (
            "a name that is no row",
            json!(["garden"]),
            "`names` names \"garden\", which is nothing a project starts with. It may name: board, fleet, docs, features",
        ),
        (
            "a row handed what it is told",
            json!([{ "name": "fleet", "with": { "channel": "x" } }]),
            "`channel` of `fleet` is the project's",
        ),
        (
            "handed what is no mapping",
            json!([{ "name": "board", "with": 3 }]),
            "`with` of `board` is what its blueprint is handed",
        ),
        (
            "a row of its own called what a row is",
            json!([{ "name": "board", "blueprint": "std/docs" }]),
            "`board` is a row already",
        ),
    ];
    for (what, asked, says) in cases {
        let refused = chosen(&rows, "names", asked.as_array().unwrap()).unwrap_err();
        assert!(refused.contains(says), "{what}: {refused}");
    }
}

#[test]
fn a_row_is_there_left_out_or_given_by_what_the_project_has() {
    let rows = usual();
    let row = |name: &str| rows.iter().find(|row| row.name == name).unwrap();
    let names = |all: &[&str]| all.iter().map(|name| name.to_string()).collect::<Vec<_>>();
    let reached =
        |given: &'static [&str], had: &'static [&str], alive: Option<&'static str>| (given, had, alive);
    type Case<'a> = (&'a str, &'a str, (&'a [&'a str], &'a [&'a str], Option<&'a str>), Came);
    let left_fleet = "`shop` was not given a fleet: a fleet works a board, and it has not got a board. With one: \
                      `mw fleet init shop`, then `mw apply fleet --project shop`";
    let left_features = "`shop` was not given features: it needs a fleet, and it has not got a fleet. With one: \
                         `mw apply std/features --project shop --param project=shop`";
    let cases: Vec<Case> = vec![
        ("a row that needs nothing", "board", reached(&[], &[], None), Came::Give),
        ("a row whose need was given now", "fleet", reached(&["board"], &[], None), Came::Give),
        ("a row whose need the project had before", "fleet", reached(&[], &["board"], None), Came::Give),
        ("a row whose need it has not got", "fleet", reached(&[], &[], None), Came::Left(left_fleet.into())),
        (
            "one of two needs lacking, said by the row's own words",
            "features",
            reached(&["board"], &[], None),
            Came::Left(left_features.into()),
        ),
        (
            "a row alive in the project already",
            "board",
            reached(&[], &[], Some("in_1")),
            Came::There("in_1".into()),
        ),
        (
            "alive though it lacks what it needs",
            "fleet",
            reached(&[], &[], Some("in_2")),
            Came::There("in_2".into()),
        ),
    ];
    for (what, name, (given, had, alive), meant) in cases {
        let (given, had) = (names(given), names(had));
        let at = Reached { project: "shop", given: &given, had: &had, alive, rows: &rows, cli: "mw" };
        assert_eq!(came(row(name), &at), meant, "{what}");
    }
}

#[test]
fn how_a_row_is_given_by_hand_and_what_somebody_who_may_not_apply_is_told() {
    let rows = usual();
    assert_eq!(by_hand(&rows[0], "shop", "mw"), "`mw apply std/board --project shop`");
    assert_eq!(by_hand(&rows[1], "shop", "mw"), "`mw fleet init shop`, then `mw apply fleet --project shop`");
    assert_eq!(
        by_hand(&rows[3], "shop", "mw"),
        "`mw apply std/features --project shop --param project=shop`"
    );
    assert_eq!(
        refused(&rows[..2], "Ada", "shop", "mw"),
        "`shop` was not given a board and a fleet: each is made by applying a blueprint, which takes \
         `blueprints:run`, and Ada does not hold it. Someone who does can give it them: `mw apply std/board \
         --project shop`, then `mw fleet init shop`, then `mw apply fleet --project shop`"
    );
}
