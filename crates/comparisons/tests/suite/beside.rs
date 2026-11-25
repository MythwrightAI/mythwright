//! Sandbox tests side by side, laid out (`comparisons::beside::sandboxes`):
//! a function of what each sandbox handed, so its cases are what a
//! sandbox hands.

use comparisons::beside::{Played, sandboxes};
use serde_json::{Value, json};

/// What a sandbox hands of what ran, as `measured` says it: one series
/// a name, of the blueprint, with its count.
fn shown(measured: &[(&str, &str, u64)]) -> Value {
    let series: Vec<Value> = measured
        .iter()
        .map(|(name, blueprint, count)| {
            json!({ "name": name, "labels": { "blueprint": blueprint }, "total": { "count": count, "errors": 0, "sum": 0.0 } })
        })
        .collect();
    json!({ "began": 0, "happened": [], "measured": series, "asked": [] })
}

fn whats(laid: &Value) -> Vec<String> {
    laid["rows"].as_array().unwrap().iter().map(|row| row["what"].as_str().unwrap().to_string()).collect()
}

/// The same text played twice, in the two orders a sandbox may come to:
/// the child a step spawns begins to listen before its parent says it
/// is active, and is woken by that too (2), or a moment after, and is not
/// (1). What ran is the same in both.
#[test]
fn what_the_runner_measures_of_its_own_work_is_not_laid_out_so_one_text_played_twice_is_one_column_twice() {
    let early = shown(&[
        ("handler.event", "std/cards", 2),
        ("handler.catch_up", "std/cards", 20),
        ("instance.listening_late", "std/cards", 1),
        ("run", "std/cards", 2),
        ("step", "std/board", 2),
    ]);
    let late = shown(&[
        ("handler.event", "std/cards", 1),
        ("handler.catch_up", "std/cards", 19),
        ("run", "std/cards", 2),
        ("step", "std/board", 2),
    ]);
    let text = "blueprint: board\nsteps:\n  a: { do: emit, with: { topic: a } }\n";
    let laid = sandboxes(&[
        Played { called: "as it stands", source: text, shown: &early },
        Played { called: "as it was saved", source: text, shown: &late },
    ]);

    assert_eq!(whats(&laid), ["run", "step"], "{laid}");
    for row in laid["rows"].as_array().unwrap() {
        assert_eq!(row["by"][0], row["by"][1], "the same text played twice is the same column twice: {row}");
    }
}

#[test]
fn what_ran_and_what_was_said_are_laid_out_beside_each_other_whatever_the_runner_measured_of_itself() {
    let fewer = shown(&[("handler.event", "std/board", 3), ("step", "std/board", 1)]);
    let more = shown(&[("step", "std/board", 4), ("step", "std/cards", 2)]);
    let mut more_said = more.clone();
    more_said["happened"] =
        json!([{ "topic": "card.taken", "ts": 0 }, { "topic": "handler.failed", "ts": 0 }]);
    let laid = sandboxes(&[
        Played { called: "fork", source: "blueprint: a\n", shown: &fewer },
        Played { called: "default", source: "blueprint: b\n", shown: &more_said },
    ]);

    assert_eq!(whats(&laid), ["card.taken", "step"], "{laid}");
    let step = laid["rows"].as_array().unwrap().iter().find(|row| row["what"] == "step").unwrap();
    assert_eq!(
        (step["by"][0]["count"].as_u64(), step["by"][1]["count"].as_u64()),
        (Some(1), Some(6)),
        "{step}"
    );
}
