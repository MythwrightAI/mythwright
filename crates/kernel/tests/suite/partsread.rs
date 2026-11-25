//! The parts a told line holds beside its words that are read for whoever
//! reads it (`kernel::parts`): rows of a table, runs and measurements,
//! each checked as it is told; which tab of the line's page each is in;
//! and what each comes to of what is handed when it is read.

use kernel::parts::{
    Chosen, Measured, Part, Rows, Runs, RunsOf, chosen, columns, in_seconds, measured, read, run_called,
    tabbed, tabs,
};
use serde_json::{Value, json};

#[test]
fn a_part_of_rows_runs_or_measures_is_kept_with_what_it_leaves_unsaid_filled_in() {
    let said = json!([
        { "kind": "rows", "table": "test-runs", "saying": { "commit": { "in": ["a1", "b2"] } }, "columns": ["title", "seconds"], "tab": "Insights" },
        { "kind": "rows", "project": "mill", "table": "notes", "ids": ["mill-notes-2", "mill-notes-1"] },
        { "kind": "runs", "instance": "in_7", "tab": " Deployment " },
        { "kind": "runs", "key": "deploy-dev", "most": 3 },
        { "kind": "measures", "measures": ["agent.turn"], "labels": { "project": "shop", "seats": 2 } },
    ]);
    let kept = read(&said, Some("shop")).unwrap();
    assert_eq!(
        kept[0],
        json!({ "kind": "rows", "project": "shop", "table": "test-runs", "columns": ["title", "seconds"],
                "saying": { "commit": { "in": ["a1", "b2"] } }, "tab": "Insights" })
    );
    assert_eq!(
        kept[1],
        json!({ "kind": "rows", "project": "mill", "table": "notes", "columns": [], "ids": ["mill-notes-2", "mill-notes-1"] })
    );
    assert_eq!(
        kept[2],
        json!({ "kind": "runs", "project": "shop", "instance": "in_7", "most": 10, "tab": "Deployment" })
    );
    assert_eq!(kept[3], json!({ "kind": "runs", "project": "shop", "key": "deploy-dev", "most": 3 }));
    assert_eq!(
        kept[4],
        json!({ "kind": "measures", "measures": ["agent.turn"], "labels": { "project": "shop", "seats": "2" }, "last": 10 })
    );
    // Read back as the line keeps them, each with its tab.
    let back = tabbed(&json!(kept));
    let tabs_said: Vec<&str> = back.iter().map(|(tab, _)| tab.as_str()).collect();
    assert_eq!(tabs_said, ["Insights", "", "Deployment", "", ""]);
    let Part::Rows(rows) = &back[0].1 else { panic!("{back:?}") };
    assert_eq!((rows.ids.is_none(), rows.saying.is_some(), rows.columns.len()), (true, true, 2));
    assert_eq!(
        back[3].1,
        Part::Runs(Runs { project: "shop".into(), of: RunsOf::Key("deploy-dev".into()), most: 3 })
    );
    let Part::Measures(measures) = &back[4].1 else { panic!("{back:?}") };
    assert_eq!((measures.labels.len(), measures.last), (2, 10));
    // A run of no project's is named by its instance alone.
    let none = read(&json!([{ "kind": "runs", "instance": "in_1" }]), None).unwrap();
    assert_eq!(none[0]["project"], "");
}

#[test]
fn a_part_of_rows_runs_or_measures_that_cannot_be_told_with_is_refused_saying_why() {
    let cases: &[(Value, Option<&str>, &str)] = &[
        (json!([{ "kind": "rows", "ids": ["a"] }]), Some("shop"), "says the `table`"),
        (json!([{ "kind": "rows", "table": "t" }]), Some("shop"), "say one of the two"),
        (
            json!([{ "kind": "rows", "table": "t", "ids": ["a"], "saying": {} }]),
            Some("shop"),
            "say one of the two",
        ),
        (json!([{ "kind": "rows", "table": "t", "ids": ["a"] }]), None, "say `project`"),
        (
            json!([{ "kind": "rows", "table": "t", "saying": { "x": { "in": 3 } } }]),
            Some("shop"),
            "for any one of those",
        ),
        (
            json!([{ "kind": "rows", "table": "t", "saying": "x" }]),
            Some("shop"),
            "what a row is to say, by cell",
        ),
        (json!([{ "kind": "rows", "table": "t", "ids": [3] }]), Some("shop"), "a list of words"),
        (
            json!([{ "kind": "rows", "table": "t", "ids": ["a"], "fields": [] }]),
            Some("shop"),
            "and not `fields`",
        ),
        (json!([{ "kind": "runs" }]), Some("shop"), "say one of the three"),
        (json!([{ "kind": "runs", "instance": "a", "key": "b" }]), Some("shop"), "say one of the three"),
        (json!([{ "kind": "runs", "key": "b" }]), None, "by its `key` is of a project's"),
        (json!([{ "kind": "runs", "instance": "a", "most": 0 }]), None, "from 1 to 50"),
        (json!([{ "kind": "runs", "instance": "a", "most": 51 }]), None, "from 1 to 50"),
        (json!([{ "kind": "measures" }]), None, "says `measures`"),
        (
            json!([{ "kind": "measures", "measures": ["a"], "labels": ["x"] }]),
            None,
            "a mapping of each label",
        ),
        (
            json!([{ "kind": "measures", "measures": ["a"], "labels": { "x": [1] } }]),
            None,
            "each label's value as a word",
        ),
        (json!([{ "kind": "measures", "measures": ["a"], "last": 101 }]), None, "from 1 to 100"),
        (
            json!([{ "kind": "links", "links": ["https://a"], "tab": "a\nb" }]),
            None,
            "`tab` is the name of a tab",
        ),
        (
            json!([{ "kind": "links", "links": ["https://a"], "tab": "x".repeat(41) }]),
            None,
            "40 letters at most",
        ),
        (json!([{ "kind": "links", "links": ["https://a"], "tab": 3 }]), None, "`tab` of a part is a word"),
    ];
    for (parts, project, says) in cases {
        let refused = read(parts, *project).expect_err(&parts.to_string());
        assert!(refused.contains(says), "{parts}: {refused}");
    }
}

#[test]
fn the_parts_with_no_tab_are_on_a_first_tab_and_the_rest_on_tabs_named_as_each_is_first_said() {
    let said = |tabs: &[&str]| tabs.iter().map(|tab| tab.to_string()).collect::<Vec<_>>();
    let named = |listed: Vec<(String, Vec<usize>)>| listed;
    assert_eq!(named(tabs(&said(&["", ""]))), [(String::new(), vec![0, 1])], "one tab: no row of tabs");
    assert_eq!(
        tabs(&said(&["To test", "", "Links", "To test"])),
        [(String::new(), vec![1]), ("To test".into(), vec![0, 3]), ("Links".into(), vec![2])]
    );
    assert_eq!(
        tabs(&said(&["Links", "Insights"])),
        [("Links".into(), vec![0]), ("Insights".into(), vec![1])]
    );
    assert!(tabs(&[]).is_empty(), "no part, no tab");
}

#[test]
fn a_part_of_rows_lists_its_ids_in_their_order_or_the_rows_chosen_and_the_columns_it_names_that_there_are() {
    let ids = ["t-1", "t-2", "t-3"];
    let by_ids = Rows { ids: Some(vec!["t-3".into(), "t-9".into(), "t-1".into()]), ..Rows::default() };
    assert_eq!(
        chosen(&by_ids, &ids, &[]),
        (vec![Chosen::At(2), Chosen::Gone("t-9".into()), Chosen::At(0)], 0)
    );
    let saying = Rows { saying: Some(serde_json::Map::new()), ..Rows::default() };
    assert_eq!(
        chosen(&saying, &ids, &["t-3", "t-1"]),
        (vec![Chosen::At(2), Chosen::At(0)], 0),
        "in what was handed"
    );
    let many: Vec<String> = (0..130).map(|n| format!("t-{n}")).collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();
    let (listed, more) = chosen(&saying, &many, &many);
    assert_eq!((listed.len(), more), (100, 30), "at most a hundred, and how many more");
    // The columns it names that the table has, in its order, and those it has not got.
    let declared = ["title", "result", "seconds", "commit"];
    let named = Rows { columns: vec!["seconds".into(), "lost".into(), "title".into()], ..Rows::default() };
    assert_eq!(columns(&named, &declared), (vec!["seconds".into(), "title".into()], vec!["lost".into()]));
    let own: Vec<String> = declared.iter().map(|cell| cell.to_string()).collect();
    assert_eq!(columns(&Rows::default(), &declared), (own, vec![]), "where it names none, the table's own");
    let wide: Vec<String> = (0..20).map(|n| format!("c{n}")).collect();
    let wide: Vec<&str> = wide.iter().map(String::as_str).collect();
    assert_eq!(columns(&Rows::default(), &wide).0.len(), 12);
    // A number whose name says it is of seconds is drawn as a length.
    for (name, kind, is) in [
        ("seconds", "number", true),
        ("run_seconds", "number", true),
        ("seconds", "text", false),
        ("secondsish", "number", false),
        ("count", "number", false),
    ] {
        assert_eq!(in_seconds(name, kind), is, "{name} {kind}");
    }
}

#[test]
fn a_run_is_called_by_what_was_applied_and_the_handler_whose_run_it_is() {
    for (blueprint, key, flow, called) in [
        ("preview", "preview", "main", "preview"),
        ("preview", "", "main", "preview"),
        ("preview", "deploy-dev", "main", "preview as deploy-dev"),
        ("std/fleet", "fleet", "on.changed", "std/fleet as fleet, on changed"),
        ("preview", "preview", "on_update", "preview, taking a new text"),
    ] {
        assert_eq!(run_called(blueprint, key, flow), called);
    }
}

#[test]
fn a_measurement_comes_to_its_latest_value_beside_the_median_of_its_last_few() {
    assert_eq!(measured(&[], 10), None, "nothing measured, nothing said");
    let one = measured(&[(5, 3.0)], 10).unwrap();
    assert_eq!(one, Measured { latest: 3.0, at: 5, median: 3.0, of: 1 });
    // Odd: the middle one. Even: the mean of the middle two. Of the last few alone.
    let values = [(1, 100.0), (2, 4.0), (3, 1.0), (4, 9.0), (5, 2.0)];
    assert_eq!(measured(&values, 3).unwrap(), Measured { latest: 2.0, at: 5, median: 2.0, of: 3 });
    assert_eq!(measured(&values, 4).unwrap().median, 3.0);
    assert_eq!(measured(&values, 10).unwrap(), Measured { latest: 2.0, at: 5, median: 4.0, of: 5 });
}

#[test]
fn a_cell_of_many_words_is_handed_cut_and_anything_else_as_it_is() {
    let long = "a".repeat(400);
    let cut = kernel::parts::brief(&json!(long));
    assert_eq!(cut.as_str().unwrap().chars().count(), kernel::parts::MOST_SAID + 1);
    assert!(cut.as_str().unwrap().ends_with('…'));
    for kept in [json!("short"), json!(3), json!(["x"]), Value::Null] {
        assert_eq!(kernel::parts::brief(&kept), kept);
    }
}
