//! What a view says of its columns (`kernel::columns`): read as a
//! blueprint writes it and as a page sends it, and held to the table it
//! is a view of. Functions of plain values, each with its cases.

use kernel::columns::{Columns, FIGURES, NARROWEST, WIDEST, fits};
use serde_json::{Value, json};

fn read(about: Value) -> Result<Columns, String> {
    Columns::read("rows", about.as_object().unwrap()).map_err(|error| error.to_string())
}

const FIELDS: &[(&str, &str)] =
    &[("title", "text"), ("points", "number"), ("done", "flag"), ("status", "choice"), ("labels", "list")];

fn checked(kind: &str, about: Value) -> Result<(), String> {
    read(about)?.check("rows", kind, FIELDS).map_err(|error| error.to_string())
}

#[test]
fn a_view_that_says_nothing_of_its_columns_says_nothing_and_is_written_back_as_it_was() {
    for nothing in [
        json!({}),
        json!({ "name": "rows", "kind": "table" }),
        json!({ "columns": null, "freezes": null, "figures": null, "widths": null }),
        json!({ "columns": [], "freezes": " ", "figures": {}, "widths": {} }),
    ] {
        let said = read(nothing.clone()).unwrap();
        assert_eq!(said, Columns::default(), "{nothing}");
        // A view kept before any of this could be said is not given a key by being read.
        assert_eq!(serde_json::to_value(&said).unwrap(), json!({}), "{nothing}");
    }
    // And one kept before reads as saying none.
    let before: Columns =
        serde_json::from_value(json!({ "name": "rows", "kind": "table", "by": null })).unwrap();
    assert_eq!(before, Columns::default());
}

#[test]
fn the_four_are_read_as_they_are_written_and_kept_under_the_words_they_are_said_by() {
    let said = read(json!({
        "columns": ["title", " points ", "title", "", "gone"],
        "freezes": " points ",
        "figures": { "points": " sum ", "done": "ticked" },
        "widths": { "title": 320, " points ": 60, "gone": 2000 },
    }))
    .unwrap();
    // Each column once, without the space round it; a name that is no column is kept and passed over where it is read.
    assert_eq!(
        said.order.as_deref(),
        Some(&["title".to_string(), "points".to_string(), "gone".to_string()][..])
    );
    assert_eq!(said.freezes.as_deref(), Some("points"));
    let kept = serde_json::to_value(&said).unwrap();
    assert_eq!(
        kept,
        json!({ "columns": ["title", "points", "gone"], "freezes": "points", "figures": { "points": "sum", "done": "ticked" }, "widths": { "gone": 2000, "points": 60, "title": 320 } })
    );
    assert_eq!(serde_json::from_value::<Columns>(kept).unwrap(), said);
}

#[test]
fn what_is_written_another_way_is_refused_in_a_sentence_that_says_how_it_is_written() {
    let cases = [
        (
            json!({ "columns": "title" }),
            "says `columns`, which is a list of its columns in the order they stand, as in `columns: [title, due]`",
        ),
        (json!({ "columns": ["title", 3] }), "says `columns`, which is a list of its columns"),
        (
            json!({ "freezes": ["title"] }),
            "says `freezes`, which is the one column it is frozen up to, as in `freezes: title`",
        ),
        (json!({ "freezes": true }), "says `freezes`, which is the one column"),
        (
            json!({ "figures": ["sum"] }),
            "says `figures`, which is a map from a column to the figure at its foot, as in `figures: { points: sum }`",
        ),
        (json!({ "figures": { "points": 3 } }), "says `figures`, which is a map from a column to the figure"),
        (
            json!({ "widths": [320] }),
            "says `widths`, which is a map from a column to how wide it is in whole pixels, as in `widths: { title: 320 }`",
        ),
        (json!({ "widths": { "title": "wide" } }), "says `widths`, which is a map from a column to how wide"),
        (json!({ "widths": { "title": 120.5 } }), "says `widths`, which is a map from a column to how wide"),
        (json!({ "widths": { "title": -4 } }), "says `widths`, which is a map from a column to how wide"),
        (
            json!({ "widths": { "title": 59 } }),
            "says `title` is 59 pixels wide, and a column is from 60 to 2000",
        ),
        (
            json!({ "widths": { "title": 2001 } }),
            "says `title` is 2001 pixels wide, and a column is from 60 to 2000",
        ),
        (json!({ "widths": { "title": 0 } }), "says `title` is 0 pixels wide"),
    ];
    for (about, wanted) in cases {
        let said = read(about.clone()).unwrap_err();
        assert!(said.contains("the view `rows`") && said.contains(wanted), "{about}: {said}");
    }
}

#[test]
fn every_kind_of_column_says_how_many_rows_are_filled_and_only_what_else_its_kind_can() {
    let table: &[(&str, &[&str])] = &[
        ("number", &["filled", "empty", "sum", "mean", "least", "greatest"]),
        ("flag", &["filled", "empty", "ticked"]),
        ("text", &["filled", "empty", "different"]),
        ("choice", &["filled", "empty", "different"]),
        ("list", &["filled", "empty", "different"]),
        ("actor", &["filled", "empty", "different"]),
        ("date", &["filled", "empty", "different"]),
        ("record", &["filled", "empty", "different"]),
        ("records", &["filled", "empty", "different"]),
    ];
    for (kind, can) in table {
        let fitting: Vec<&str> = FIGURES.iter().copied().filter(|figure| fits(kind, figure)).collect();
        assert_eq!(&fitting[..], *can, "{kind}");
    }
    // Every figure there is fits some kind, and nothing else is a figure.
    for figure in FIGURES {
        assert!(table.iter().any(|(kind, _)| fits(kind, figure)), "{figure}");
    }
    assert!(!fits("number", "median") && !fits("text", ""));
}

#[test]
fn what_a_view_says_of_its_columns_is_held_to_the_table_as_it_is_declared_or_saved() {
    // All of it said of a table whose columns are there.
    let all = json!({ "columns": ["points", "title"], "freezes": "points", "figures": { "points": "mean", "done": "ticked", "labels": "different", "title": "filled" }, "widths": { "title": 300 } });
    assert_eq!(checked("table", all), Ok(()));
    // A width of a column the table has not got is passed over where it is read, as the order is, never refused.
    assert_eq!(checked("table", json!({ "widths": { "nowhere": 200, "title": 80 } })), Ok(()));
    // The order names what it likes, laid out as anything: a name that is no column is passed over, never refused.
    for kind in ["table", "list", "gallery", "board"] {
        assert_eq!(checked(kind, json!({ "columns": ["nowhere", "title"] })), Ok(()), "{kind}");
    }
    let cases = [
        (
            "table",
            json!({ "freezes": "nowhere" }),
            "is frozen up to `nowhere`, which is not a column of the table; it has: title, points, done, status, labels",
        ),
        ("list", json!({ "freezes": "title" }), "says `freezes: title`, which is for a table"),
        (
            "gallery",
            json!({ "widths": { "title": 200 } }),
            "says `widths`, which is for a table: how wide each of its columns is",
        ),
        (
            "board",
            json!({ "figures": { "points": "sum" } }),
            "says `figures`, which is for a table: a figure at the foot of a column",
        ),
        (
            "table",
            json!({ "figures": { "nowhere": "filled" } }),
            "says a figure at the foot of `nowhere`, which is not a column of the table; it has: title, points",
        ),
        (
            "table",
            json!({ "figures": { "points": "median" } }),
            "says `median` at the foot of `points`, which is not a figure; there are: filled, empty, sum, mean, least, greatest, ticked, different",
        ),
        (
            "table",
            json!({ "figures": { "title": "sum" } }),
            "says `sum` at the foot of `title`, which is a `text`: a column of that kind can say filled, empty, different",
        ),
        (
            "table",
            json!({ "figures": { "points": "ticked" } }),
            "says `ticked` at the foot of `points`, which is a `number`: a column of that kind can say filled, empty, sum, mean, least, greatest",
        ),
        (
            "table",
            json!({ "figures": { "done": "different" } }),
            "says `different` at the foot of `done`, which is a `flag`: a column of that kind can say filled, empty, ticked",
        ),
    ];
    for (kind, about, wanted) in cases {
        let said = checked(kind, about.clone()).unwrap_err();
        assert!(said.contains("the view `rows`") && said.contains(wanted), "{kind} {about}: {said}");
    }
}

#[test]
fn a_width_is_from_room_for_a_mark_and_a_few_letters_to_more_than_any_screen_and_said_so_where_it_is_described()
 {
    // What an editor and the Reference say of a width says the same numbers as what is held.
    let said = kernel::inside::VIEWS
        .iter()
        .find(|(path, _, _)| *path == "*.widths.*")
        .map(|(_, _, sentence)| *sentence)
        .unwrap();
    assert!(said.contains(&format!("from {NARROWEST} to {WIDEST}")), "{said}");
    assert!(read(json!({ "widths": { "title": NARROWEST, "due": WIDEST } })).is_ok());
}
