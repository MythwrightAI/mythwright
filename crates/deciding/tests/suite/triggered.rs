//! When a row of a project's triggers fires: a table of cases for each
//! kind of thing that happens, for what a run is handed, for what whoever
//! runs one by hand is to hold, and for which rows go by the clock.

use blueprint::ValueType;
use deciding::triggered::{Fired, Happened, Row, fired, fit, holds, refusal, timed};
use serde_json::{Map, Value, json};

fn row(id: &str, pipeline: &str, on: &str, said: &[(&str, &str)]) -> Row {
    let mut cells = json!({ "id": id, "pipeline": pipeline, "on": on });
    for (name, value) in said {
        cells[*name] = if *name == "paused" { json!(*value == "true") } else { json!(value) };
    }
    Row::of(&cells)
}

fn with(pairs: &[(&str, &str)]) -> Map<String, Value> {
    pairs.iter().map(|(name, value)| (name.to_string(), json!(value))).collect()
}

fn rows() -> Vec<Row> {
    vec![
        row("p-triggers-1", "pipelines/gates", "hand", &[]),
        row("p-triggers-2", "pipelines/gates", "branch", &[("branch", "main")]),
        row("p-triggers-3", "pipelines/build", "branch", &[]),
        row("p-triggers-4", "pipelines/train", "column", &[("column", "reviewed"), ("key", "train")]),
        row("p-triggers-5", "pipelines/nightly", "at", &[("at", "02:00 Europe/Zagreb")]),
        row("p-triggers-6", "pipelines/sweep", "every", &[("every", "6h")]),
        row("p-triggers-7", "pipelines/gates", "column", &[("column", "reviewed"), ("paused", "true")]),
        row("p-triggers-8", "", "hand", &[]),
    ]
}

/// A run that is due, as a case says it: its row, pipeline, key and what it is handed.
type Run = (String, String, String, Map<String, Value>);

fn fired_of(happened: Happened) -> Vec<Run> {
    fired(&rows(), &happened)
        .unwrap()
        .into_iter()
        .map(|Fired { row, pipeline, key, with, .. }| (row, pipeline, key, with))
        .collect()
}

#[test]
fn each_kind_of_thing_that_happens_fires_the_rows_that_say_it_and_no_other() {
    type Meant<'a> = Vec<(&'a str, &'a str, &'a str, Map<String, Value>)>;
    let cases: Vec<(&str, Happened, Meant)> = vec![
        (
            "by hand, the row that runs that pipeline by hand, handed what was said",
            Happened::Hand { pipeline: "pipelines/gates".into(), with: with(&[("cards", "a")]) },
            vec![("p-triggers-1", "pipelines/gates", "pipelines-gates", with(&[("cards", "a")]))],
        ),
        (
            "a branch named and a row that names none both fire as main moves",
            Happened::Branch { branch: "main".into(), commit: "c1".into() },
            vec![
                (
                    "p-triggers-2",
                    "pipelines/gates",
                    "pipelines-gates",
                    with(&[("branch", "main"), ("commit", "c1")]),
                ),
                (
                    "p-triggers-3",
                    "pipelines/build",
                    "pipelines-build",
                    with(&[("branch", "main"), ("commit", "c1")]),
                ),
            ],
        ),
        (
            "another branch fires only the row that names none",
            Happened::Branch { branch: "lane/p-4".into(), commit: "c2".into() },
            vec![(
                "p-triggers-3",
                "pipelines/build",
                "pipelines-build",
                with(&[("branch", "lane/p-4"), ("commit", "c2")]),
            )],
        ),
        (
            "a card coming to a column fires under the key its row says, and a paused row fires nothing",
            Happened::Column { column: "reviewed".into(), item: "p-4".into() },
            vec![(
                "p-triggers-4",
                "pipelines/train",
                "train",
                with(&[("item", "p-4"), ("column", "reviewed")]),
            )],
        ),
        (
            "a card coming to a column no row names fires nothing",
            Happened::Column { column: "doing".into(), item: "p-4".into() },
            vec![],
        ),
        (
            "the time of a row of `at` coming fires that row",
            Happened::Due { row: "p-triggers-5".into() },
            vec![("p-triggers-5", "pipelines/nightly", "pipelines-nightly", Map::new())],
        ),
        (
            "and of a row of `every`",
            Happened::Due { row: "p-triggers-6".into() },
            vec![("p-triggers-6", "pipelines/sweep", "pipelines-sweep", Map::new())],
        ),
        ("a time that is no row's fires nothing", Happened::Due { row: "".into() }, vec![]),
        (
            "a row by hand whose time came is not one that goes by the clock",
            Happened::Due { row: "p-triggers-1".into() },
            vec![],
        ),
    ];
    for (case, happened, meant) in cases {
        let meant: Vec<Run> = meant
            .into_iter()
            .map(|(row, pipeline, key, with)| (row.into(), pipeline.into(), key.into(), with))
            .collect();
        assert_eq!(fired_of(happened), meant, "{case}");
    }
}

#[test]
fn a_row_says_the_environment_its_runs_are_under_and_its_key_keeps_two_environments_apart() {
    let rows = [
        row("p-triggers-1", "pipelines/deploy", "branch", &[("branch", "main"), ("environment", "prod")]),
        row("p-triggers-2", "pipelines/deploy", "branch", &[("environment", "staging")]),
        row("p-triggers-3", "pipelines/deploy", "hand", &[("environment", "prod"), ("key", "ship")]),
        row("p-triggers-4", "pipelines/gates", "hand", &[("environment", "  ")]),
    ];
    // What fires, as a case says it: each row with its environment and key.
    type Meant<'a> = Vec<(&'a str, &'a str, &'a str)>;
    let cases: Vec<(&str, Happened, Meant)> = vec![
        (
            "each row fires under its own environment, the pipeline's name and the environment's its key",
            Happened::Branch { branch: "main".into(), commit: "c1".into() },
            vec![
                ("p-triggers-1", "prod", "pipelines-deploy-prod"),
                ("p-triggers-2", "staging", "pipelines-deploy-staging"),
            ],
        ),
        (
            "another branch, the row that names none under its environment",
            Happened::Branch { branch: "lane/p-4".into(), commit: "c2".into() },
            vec![("p-triggers-2", "staging", "pipelines-deploy-staging")],
        ),
        (
            "a key a row says is its key whatever its environment",
            Happened::Hand { pipeline: "pipelines/deploy".into(), with: Map::new() },
            vec![("p-triggers-3", "prod", "ship")],
        ),
        (
            "an environment said as nothing is none, and the key the pipeline's name alone",
            Happened::Hand { pipeline: "pipelines/gates".into(), with: Map::new() },
            vec![("p-triggers-4", "", "pipelines-gates")],
        ),
    ];
    for (case, happened, meant) in cases {
        let came: Vec<(String, String, String)> = fired(&rows, &happened)
            .unwrap()
            .into_iter()
            .map(|Fired { row, environment, key, .. }| (row, environment, key))
            .collect();
        let meant: Vec<(String, String, String)> =
            meant.into_iter().map(|(row, under, key)| (row.into(), under.into(), key.into())).collect();
        assert_eq!(came, meant, "{case}");
    }
}

#[test]
fn a_pipeline_asked_for_by_hand_that_no_row_runs_so_is_refused_saying_what_to_add() {
    for pipeline in ["pipelines/build", "pipelines/elsewhere", ""] {
        let why =
            fired(&rows(), &Happened::Hand { pipeline: pipeline.into(), with: Map::new() }).unwrap_err();
        assert!(why.contains(&format!("pipeline: {pipeline}")) && why.contains("on: hand"), "{why}");
    }
    let paused = vec![row("p-triggers-1", "pipelines/gates", "hand", &[("paused", "true")])];
    let why =
        fired(&paused, &Happened::Hand { pipeline: "pipelines/gates".into(), with: Map::new() }).unwrap_err();
    assert!(why.contains("paused"), "{why}");
}

#[test]
fn a_run_is_handed_only_what_its_pipeline_takes_and_by_hand_a_name_it_does_not_take_is_refused() {
    let takes = [("branch".to_string(), ValueType::String), ("cards".to_string(), ValueType::String)];
    let handed = with(&[("branch", "main"), ("commit", "c1")]);
    assert_eq!(fit(&[], &handed, &takes, false, "pipelines/build").unwrap(), with(&[("branch", "main")]));
    assert_eq!(fit(&[], &handed, &[], false, "pipelines/build").unwrap(), Map::new());
    let asked = with(&[("cards", "a"), ("colour", "red")]);
    let why = fit(&[], &asked, &takes, true, "pipelines/gates").unwrap_err();
    assert_eq!(why, "`pipelines/gates` takes no param called `colour`: it takes `branch`, `cards`");
    let why = fit(&[], &asked, &[], true, "pipelines/gates").unwrap_err();
    assert!(why.ends_with("it takes none"), "{why}");
    assert_eq!(
        fit(&[], &with(&[("cards", "a")]), &takes, true, "pipelines/gates").unwrap(),
        with(&[("cards", "a")])
    );
}

#[test]
fn what_a_row_says_in_with_is_handed_as_the_kind_its_param_is_under_what_happened_and_what_it_cannot_be_is_refused()
 {
    let takes = [
        ("target".to_string(), ValueType::String),
        ("times".to_string(), ValueType::Int),
        ("quick".to_string(), ValueType::Bool),
        ("branch".to_string(), ValueType::String),
    ];
    let said = |all: &[&str]| all.iter().map(|one| one.to_string()).collect::<Vec<_>>();
    let fitted = fit(
        &said(&["target=prod", " times = 3", "quick=yes", "branch=dev"]),
        &with(&[("branch", "main")]),
        &takes,
        false,
        "p",
    )
    .unwrap();
    assert_eq!(
        Value::Object(fitted),
        json!({ "target": "prod", "times": 3, "quick": true, "branch": "main" })
    );
    let cases = [
        (said(&["colour=red"]), "says `colour` in `with`, and it takes no param called that"),
        (said(&["target"]), "says `target` in `with`, where each thing is `name=value`"),
        (said(&["times=many"]), "says `times` is `many`, and it is a whole number"),
    ];
    for (row, why) in cases {
        let refused = fit(&row, &Map::new(), &takes, false, "p").unwrap_err();
        assert!(refused.contains(why), "{refused}");
    }
}

#[test]
fn whoever_asked_by_hand_is_told_why_nothing_ran_or_what_they_do_not_hold() {
    let lacks = |all: &[&str]| all.iter().map(|one| one.to_string()).collect::<Vec<_>>();
    let cases = [
        ("pipelines/gates", "it says nothing", lacks(&[]), "`pipelines/gates` was not run: it says nothing"),
        (
            "pipelines/gates",
            "",
            lacks(&["shell"]),
            "`pipelines/gates` was not run: it takes `shell`, which you do not hold",
        ),
        (
            "pipelines/gates",
            "",
            lacks(&["blueprints:run", "shell", "projects:write"]),
            "`pipelines/gates` was not run: it takes `blueprints:run`, `shell` and `projects:write`, which you do not hold",
        ),
        ("", "", lacks(&[]), "No pipeline was run: what may run it could not be asked"),
    ];
    for (pipeline, says, lacking, meant) in cases {
        assert_eq!(refusal(pipeline, says, &lacking), meant);
    }
}

#[test]
fn whoever_runs_a_pipeline_by_hand_holds_what_it_says_it_runs_with_and_one_that_says_nothing_is_run_by_hand_by_nobody()
 {
    let said = ["shell".to_string(), "projects:write".to_string()];
    assert_eq!(holds(Some(&said), "pipelines/gates").unwrap(), said.to_vec());
    assert_eq!(holds(Some(&[]), "pipelines/gates").unwrap(), Vec::<String>::new());
    let why = holds(None, "pipelines/gates").unwrap_err();
    assert!(
        why.starts_with("`pipelines/gates` says nothing of what it may use") && why.contains("permissions:"),
        "{why}"
    );
}

#[test]
fn the_rows_that_go_by_the_clock_and_by_a_branch_are_said_and_what_cannot_be_gone_by_said_with_why() {
    let mut all = rows();
    all.extend([
        row("p-triggers-9", "pipelines/a", "every", &[("every", "30s")]),
        row("p-triggers-10", "pipelines/a", "every", &[("every", "often")]),
        row("p-triggers-11", "pipelines/a", "at", &[("at", "02:00")]),
        row("p-triggers-12", "pipelines/a", "every", &[("every", "1h"), ("paused", "true")]),
    ]);
    let timed = timed(&all);
    assert_eq!(timed.every, vec![("p-triggers-6".to_string(), "6h".to_string())]);
    assert_eq!(timed.at, vec![("p-triggers-5".to_string(), "02:00 Europe/Zagreb".to_string())]);
    assert!(timed.follows);
    let wrong: Vec<&str> = timed.wrong.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(wrong, ["p-triggers-9", "p-triggers-10", "p-triggers-11"]);
    assert!(timed.wrong[0].1.contains("once a minute"), "{}", timed.wrong[0].1);
    assert!(timed.wrong[2].1.contains("says no zone"), "{}", timed.wrong[2].1);
    let none = deciding::triggered::timed(&[row("p-triggers-1", "pipelines/gates", "hand", &[])]);
    assert!(!none.follows && none.every.is_empty() && none.at.is_empty() && none.wrong.is_empty());
}
