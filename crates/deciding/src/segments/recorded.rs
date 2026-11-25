//! What is kept of a command that was run for an item, as a row of a
//! table: a run id for its title (when it began and which item), the day
//! it began as a date and the time as words, the item, the commit it ran
//! on, the command, whether it passed and with what code, how long it took
//! and the last of what it printed. A function of what it is handed, with
//! its cases.
//!
//! It counts and never scores: a run passed or failed, as the command's
//! ending said, and nothing is made of how long it took.

use blueprint::{ActionSpec, ValueType};
use conductor::Registry;
use serde_json::{Map, Value, json};

use super::{flag, text};
use crate::add;

/// The two words a run's `result` is.
pub const RESULTS: &[&str] = &["passed", "failed"];

/// How many of the last lines of what was printed are kept, and how many
/// characters at most: enough to say why a test failed, and never a
/// whole log in a cell.
pub const LAST_LINES: usize = 40;
pub const MOST_PRINTED: usize = 4000;

/// What [`recorded`] is asked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recorded {
    pub item: String,
    /// The command, as it was said.
    pub run: String,
    pub ok: bool,
    /// The code it ended with, where it ended with one.
    pub code: Option<i64>,
    /// How long it ran.
    pub duration_ms: i64,
    /// What it printed, and what it printed where errors go.
    pub printed: String,
    pub also: String,
    /// Why it could not be run, where it could not.
    pub why: String,
    /// The commit it ran on, where where it ran is a repository.
    pub commit: String,
}

/// The row kept of a run that ended at `now` (milliseconds): each field
/// by its name. Its title is a run id, when it began and the item
/// (`20261122-140310-shop-7`), which is read as one run and sorted as
/// when it began; the day is a date, so that a table filters and sorts
/// by it as one, and the time beside it is words. What is not known (a
/// code where there was none, a commit where it ran in no repository) is
/// left out, so that a cell says nothing rather than something made up.
pub fn recorded(asked: &Recorded, now: i64) -> Map<String, Value> {
    let began = now - asked.duration_ms.max(0);
    let mut row = Map::new();
    let (day, time) = when(began);
    row.insert(
        "title".into(),
        json!(format!("{}-{}-{}", day.replace('-', ""), time.replace(':', ""), asked.item)),
    );
    row.insert("day".into(), json!(day));
    row.insert("time".into(), json!(format!("{time} UTC")));
    row.insert("card".into(), json!(asked.item));
    row.insert("result".into(), json!(if asked.ok { RESULTS[0] } else { RESULTS[1] }));
    if let Some(code) = asked.code {
        row.insert("code".into(), json!(code));
    }
    row.insert("seconds".into(), json!(asked.duration_ms.max(0) as f64 / 1000.0));
    row.insert("command".into(), json!(asked.run.trim()));
    if !asked.commit.trim().is_empty() {
        row.insert("commit".into(), json!(asked.commit.trim()));
    }
    row.insert("printed".into(), json!(last_of(asked)));
    row
}

/// A moment's day and time in UTC: `2026-11-22` and `14:03:12`.
fn when(ms: i64) -> (String, String) {
    let said = kernel::clock::rfc3339(ms);
    let (day, time) = said.split_once('T').unwrap_or((&said, ""));
    (day.to_string(), time.get(..8).unwrap_or(time).to_string())
}

/// The last of what was printed, both where it goes and where errors go:
/// so many lines and so many characters at most, cut where a line begins
/// where it can be. Where nothing was printed, why it could not be run.
fn last_of(asked: &Recorded) -> String {
    let all = [asked.printed.trim_end(), asked.also.trim_end()]
        .into_iter()
        .filter(|said| !said.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if all.trim().is_empty() {
        return asked.why.trim().to_string();
    }
    let lines: Vec<&str> = all.lines().collect();
    let last = lines[lines.len().saturating_sub(LAST_LINES)..].join("\n");
    let Some((from, _)) = last.char_indices().rev().nth(MOST_PRINTED - 1) else { return last };
    let rest = &last[from..];
    if last[..from].ends_with('\n') {
        return rest.to_string();
    }
    match rest.find('\n') {
        Some(at) if at + 1 < rest.len() => rest[at + 1..].to_string(),
        _ => rest.to_string(),
    }
}

/// Adds what is asked here to what a blueprint can do.
pub(super) fn register(registry: &mut Registry) {
    use ValueType as T;
    add(
        registry,
        ActionSpec::new(
            "segments.recorded",
            "Say what is kept of a command that was run for an item, as a row of a table: a run id, the day \
             and the time it began, the item, the commit it ran on, the command, whether it passed and with what code, how long it took \
             and the last of what it printed.",
        )
        .deciding()
        .required("item", T::String, "the item")
        .arg("command", T::String, "the command, as it was said")
        .required("ok", T::Bool, "whether it passed")
        .arg("code", T::Any, "the code it ended with; nothing where it ended with none")
        .arg("duration_ms", T::Int, "how long it ran")
        .arg("printed", T::String, "what it printed")
        .arg("also", T::String, "and what it printed where errors go")
        .arg("why", T::String, "why it could not be run, where it could not")
        .arg("commit", T::String, "the commit it ran on, where it ran in a repository")
        .output(
            "row",
            "the row, each field by its name: `title` (a run id: when it began and the item), `day` (the day it \
             began, a date), `time` (the time it began, in UTC), `card`, `result` (`passed` or `failed`), \
             `code`, `seconds`, `command`, `commit` and `printed`; what is not known is left out",
        )
        .idempotent(),
        move |cx, args| async move {
            let asked = Recorded {
                item: text(&args, "item"),
                run: text(&args, "command"),
                ok: flag(&args, "ok"),
                code: args.get("code").and_then(Value::as_i64),
                duration_ms: args.get("duration_ms").and_then(Value::as_i64).unwrap_or(0),
                printed: text(&args, "printed"),
                also: text(&args, "also"),
                why: text(&args, "why"),
                commit: text(&args, "commit"),
            };
            Ok(json!({ "row": recorded(&asked, cx.clock().now_ms()) }))
        },
    );
}
