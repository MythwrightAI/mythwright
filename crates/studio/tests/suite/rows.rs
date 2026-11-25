//! A table's rows as they are taken (`studio::rows`, asked as
//! `rows.read`): what a row is given from its other cells, written as a
//! table in the blueprint that asks. A table of cases, with rows from
//! before among them: a cell emptied, and one never filled.

use serde_json::{Map, Value, json};
use studio::rows::told;

/// The fields of a table like the one a fleet keeps its stages in, and of
/// one like its agents: by name and kind, and no more.
fn fields() -> Value {
    json!([
        { "name": "title", "kind": "text" },
        { "name": "does", "kind": "text" },
        { "name": "seats", "kind": "number" },
        { "name": "rounds", "kind": "number" },
        { "name": "by_hand", "kind": "text" },
        { "name": "column", "kind": "text" },
        { "name": "listens", "kind": "flag" },
        { "name": "count", "kind": "number" },
    ])
}

fn rows(cells: &[Value]) -> Vec<(String, Map<String, Value>)> {
    cells
        .iter()
        .enumerate()
        .map(|(at, cells)| (format!("shop-rows-{}", at + 1), cells.as_object().cloned().unwrap()))
        .collect()
}

fn read(cells: &[Value], asked: Value) -> Value {
    told("rows", &fields(), &rows(cells), asked.as_object().unwrap()).unwrap()
}

/// What each row says in `cell`, in order.
fn each(read: &Value, cell: &str) -> Vec<Value> {
    read["rows"].as_array().unwrap().iter().map(|row| row[cell].clone()).collect()
}

#[test]
fn a_cell_that_says_nothing_is_filled_by_what_another_cell_of_its_row_says() {
    let asked =
        json!({ "fill": { "does": "review", "rounds": { "by": "does", "plan": 8, "otherwise": 2 } } });
    let read = read(
        &[
            // Says how often itself: kept, whatever it does.
            json!({ "title": "plan", "does": "plan", "rounds": 5 }),
            // Never filled, and emptied in the client: both filled by what it does.
            json!({ "title": "plan", "does": "plan" }),
            json!({ "title": "plan", "does": "plan", "rounds": "" }),
            json!({ "title": "work", "does": "work" }),
            // What it does is itself filled first, and then gone by.
            json!({ "title": "look" }),
            // A word nobody listed is `otherwise`.
            json!({ "title": "test", "does": "tests" }),
        ],
        asked,
    );
    assert_eq!(each(&read, "rounds"), [json!(5), json!(8), json!(8), json!(2), json!(2), json!(2)]);
    assert_eq!(each(&read, "does")[4], "review");
}

#[test]
fn a_row_is_given_cells_worked_out_from_it_once_it_is_marked_and_counted() {
    let agents = json!([
        { "role": "worker", "count": 3 }, { "role": "reviewer", "count": 4 }, { "role": "planner", "count": 1 },
    ]);
    let asked = json!({
        "once": "title",
        "fill": { "does": "review", "seats": 0, "role": { "like": "title" } },
        "counting": { "agents": { "of": agents, "by": "role", "sum": "count" } },
        "giving": {
            // How many go at once: one agent a card where it is worked, else
            // as many as sit together, and all of them where no number is said.
            "at_once": { "share": { "like": "agents" }, "each": { "by": "does", "work": 1, "otherwise": { "like": "seats" } } },
            "seated": { "like": "seats", "else": { "like": "agents" } },
            "known": { "by": "again", "true": { "like": "id" }, "otherwise": { "like": "title" } },
            "hears": { "by": "listens", "true": "the-channel", "otherwise": "" },
            "fixed": 7,
        },
    });
    let read = read(
        &[
            json!({ "title": "worker", "does": "work", "listens": true }),
            // Four who review, two to a card: two cards at once.
            json!({ "title": "reviewer", "seats": 2 }),
            // No number said, and one emptied: all four sit on one card.
            json!({ "title": "reviewer" }),
            json!({ "title": "reviewer", "seats": "" }),
            // More seats than there are agents: still one at a time.
            json!({ "title": "planner", "does": "plan", "seats": 5 }),
            // A role no row of agents has: nobody, and one at a time all the same.
            json!({ "title": "tester", "does": "test" }),
        ],
        asked,
    );
    assert_eq!(each(&read, "agents"), [json!(3), json!(4), json!(4), json!(4), json!(1), json!(0)]);
    assert_eq!(each(&read, "at_once"), [json!(3), json!(2), json!(1), json!(1), json!(1), json!(1)]);
    assert_eq!(each(&read, "seated"), [json!(3), json!(2), json!(4), json!(4), json!(5), json!(0)]);
    // A row called what an earlier one is called is known by its id.
    assert_eq!(
        each(&read, "known"),
        [
            json!("worker"),
            json!("reviewer"),
            json!("shop-rows-3"),
            json!("shop-rows-4"),
            json!("planner"),
            json!("tester")
        ]
    );
    assert_eq!(
        each(&read, "hears"),
        [json!("the-channel"), json!(""), json!(""), json!(""), json!(""), json!("")]
    );
    assert_eq!(each(&read, "fixed"), vec![json!(7); 6]);
}

#[test]
fn a_row_given_the_least_of_several_forms_is_given_the_least_number_above_nothing_of_those_that_say_one() {
    // How many go at once: what the row says, where it says a number, and
    // never more than its role has agents for.
    let agents = json!([{ "role": "worker", "count": 3 }]);
    let asked = json!({
        "fill": { "role": { "like": "title" } },
        "counting": { "agents": { "of": agents, "by": "role", "sum": "count" } },
        "giving": { "at_once": { "least": [{ "like": "seats" }, { "like": "agents" }] } },
    });
    let read = read(
        &[
            // Fewer than the agents: so many.
            json!({ "title": "worker", "seats": 1 }),
            // More than the agents: as many as there are.
            json!({ "title": "worker", "seats": 7 }),
            // Never said, emptied, nothing and less: as many as there are.
            json!({ "title": "worker" }),
            json!({ "title": "worker", "seats": "" }),
            json!({ "title": "worker", "seats": 0 }),
            json!({ "title": "worker", "seats": -2 }),
            // Nothing above nothing anywhere: nothing.
            json!({ "title": "nobody" }),
        ],
        asked,
    );
    assert_eq!(
        each(&read, "at_once"),
        [json!(1), json!(3), json!(3), json!(3), json!(3), json!(3), Value::Null]
    );
    // A number that is not whole is given as it is; a value is a form like any.
    let asked = json!({ "giving": { "x": { "least": [2.5, 4, "words", { "like": "seats" }] } } });
    let read = self::read(&[json!({ "title": "a", "seats": 3 })], asked);
    assert_eq!(each(&read, "x"), [json!(2.5)]);
}

#[test]
fn what_one_cell_says_is_gathered_over_the_rows_that_say_something_and_those_rows_are_given_whole() {
    let had = json!(["shop-rows-1", "shop-rows-9", "shop-rows-8"]);
    let asked = json!({
        "had": had,
        "fill": { "by_hand": "leave", "column": { "like": "title" } },
        "gathering": {
            "merged": { "of": "column", "saying": { "by_hand": "merge" } },
            "all": { "of": "column" },
            "none": { "of": "column", "saying": { "by_hand": "nothing-says-this" } },
        },
        "saying": { "by_hand": "merge" },
    });
    let read = read(
        &[
            json!({ "title": "work", "by_hand": "merge", "column": "ready" }),
            json!({ "title": "review" }),
            json!({ "title": "test", "by_hand": "merge" }),
            json!({ "title": "ship", "by_hand": "" }),
        ],
        asked,
    );
    assert_eq!(
        read["gathered"],
        json!({ "merged": ["ready", "test"], "all": ["ready", "review", "test", "ship"], "none": [] })
    );
    assert_eq!(read["those"], json!(["shop-rows-1", "shop-rows-3"]));
    let chosen: Vec<&Value> = read["chosen"].as_array().unwrap().iter().map(|row| &row["title"]).collect();
    assert_eq!(chosen, ["work", "test"]);
    // How many rows there are, and how many of those there were are gone.
    assert_eq!(
        (&read["count"], &read["went"], &read["gone"]),
        (&json!(4), &json!(2), &json!(["shop-rows-9", "shop-rows-8"]))
    );
}

#[test]
fn a_row_says_one_of_several_values_where_saying_asks_for_any_of_them() {
    let rows = [
        json!({ "title": "work", "by_hand": "merge" }),
        json!({ "title": "review", "by_hand": "leave" }),
        json!({ "title": "test" }),
        json!({ "title": "ship", "by_hand": "keep" }),
    ];
    let read = read(&rows, json!({ "saying": { "by_hand": { "in": ["merge", "keep", "nowhere"] } } }));
    assert_eq!(read["those"], json!(["shop-rows-1", "shop-rows-4"]));
    // A cell that says nothing is one of them only where nothing is among them.
    let read = self::read(&rows, json!({ "saying": { "by_hand": { "in": ["", "leave"] } } }));
    assert_eq!(read["those"], json!(["shop-rows-2", "shop-rows-3"]));
    let read = self::read(&rows, json!({ "saying": { "by_hand": { "in": [] } } }));
    assert_eq!(read["those"], json!([]), "of none, no row");
    // What is not a list is no form `in` takes.
    let asked = json!({ "saying": { "by_hand": { "in": "merge" } } });
    let refused = told("rows", &fields(), &self::rows(&rows), asked.as_object().unwrap()).unwrap_err();
    assert!(refused.contains("`{ in: [<value>, ...] }` for any one of those"), "{refused}");
    assert!(kernel::rows::saying_fits(json!({ "x": { "in": [1] } }).as_object().unwrap()).is_ok());
    assert!(kernel::rows::saying_fits(json!({ "x": { "in": 1 } }).as_object().unwrap()).is_err());
}

#[test]
fn a_form_that_cannot_be_read_is_refused_saying_which_row_and_what_a_form_is() {
    let one = [json!({ "title": "work", "does": "work" })];
    for (giving, why) in [
        (
            json!({ "x": { "by": "does", "plan": 8 } }),
            "`does` says `work`, which it has nothing beside, and it says no `otherwise`",
        ),
        (json!({ "x": { "share": { "like": "count" } } }), "`share` goes with `each`"),
        (json!({ "x": { "share": 4, "each": 2, "more": 1 } }), "`share` goes with `each` and nothing else"),
        (json!({ "x": { "like": ["a", "b"] } }), "`like` names one cell"),
        (json!({ "x": { "like": "title", "or": 3 } }), "`like` names one cell"),
        (json!({ "x": { "slug_of": 3 } }), "`slug_of` names one cell"),
        (json!({ "x": { "least": 3 } }), "`least` is a list of forms, and goes with nothing else"),
        (
            json!({ "x": { "least": [1], "else": 2 } }),
            "`least` is a list of forms, and goes with nothing else",
        ),
    ] {
        let asked = json!({ "giving": giving });
        let refused = told("rows", &fields(), &rows(&one), asked.as_object().unwrap()).unwrap_err();
        assert!(
            refused.contains("the row shop-rows-1 of `rows`") && refused.contains(why),
            "{giving}: {refused}"
        );
    }
    // A mapping that says none of a form's words is a value, and is given as it is.
    let read = read(&one, json!({ "giving": { "x": { "plan": 8 } } }));
    assert_eq!(each(&read, "x"), [json!({ "plan": 8 })]);
    // What `by` goes by is what the cell says written out: a flag, a number, nothing.
    let asked = json!({ "giving": {
        "flag": { "by": "listens", "false": "no", "otherwise": "yes" },
        "number": { "by": "seats", "2": "two", "otherwise": "other" },
        "unsaid": { "by": "column", "": "nowhere", "otherwise": "somewhere" },
    } });
    let read = self::read(&[json!({ "title": "a", "seats": 2 })], asked);
    assert_eq!(
        (&read["rows"][0]["flag"], &read["rows"][0]["number"], &read["rows"][0]["unsaid"]),
        (&json!("no"), &json!("two"), &json!("nowhere"))
    );
}

#[test]
fn a_column_somebody_added_is_not_handed_and_is_never_what_a_cell_whoever_asks_fills_is_taken_to_say() {
    // Two columns of somebody's own: one called what is filled (`key`),
    // and one called what nothing here names (`command`).
    let mut fields = fields();
    for name in ["key", "command"] {
        fields.as_array_mut().unwrap().push(json!({ "name": name, "kind": "text", "added": true }));
    }
    let cells = rows(&[
        json!({ "title": "work", "key": "deploy", "command": "rm -rf /" }),
        json!({ "title": "review" }),
    ]);
    let asked = json!({ "fill": { "key": "the fleet's" } });
    let read = told("rows", &fields, &cells, asked.as_object().unwrap()).unwrap();
    assert_eq!(each(&read, "key"), [json!("the fleet's"), json!("the fleet's")]);
    assert_eq!(each(&read, "command"), [Value::Null, Value::Null]);
    // What the blueprint declares is read as it always was.
    assert_eq!(each(&read, "title"), [json!("work"), json!("review")]);
}

#[test]
fn what_the_kernel_writes_out_of_the_language_is_the_languages_own() {
    for text in ["Corner Shop!", "  a__b  ", "Ünïcode word", "x1-y2", "", "--"] {
        assert_eq!(kernel::rows::slug(text), blueprint::expr::slug(text), "{text:?}");
    }
    for value in
        [json!("text"), json!(null), json!(true), json!(0), json!(1.5), json!(["a"]), json!({ "a": 1 })]
    {
        assert_eq!(kernel::rows::display(&value), blueprint::expr::display(&value), "{value}");
    }
}
