//! A comparison's figures laid beside each other: what a person decides
//! from, on one screen. The daemon lays them out once, for the command
//! line and the client alike, and sandbox tests played side by side are
//! laid out the same way ([`sandboxes`]).
//!
//! What was measured under a comparison is many series: one for each
//! measurement, way, text, count and choice. Read as they come they are a
//! long list. Here they are laid out once, for the command line and the
//! client alike: a column for each text of each way, with how many things
//! it was given; and a row for each thing that was measured, in four
//! kinds, in this order:
//!
//! - `came`: what came of the things (`system.comparison.came`, by what
//!   was counted and what it came to), in the order it comes about: how
//!   many, and on average how long after the thing was given its way;
//! - `chose`: a measurement taken with something chosen (an agent's turn,
//!   by its `decision`): how many, how many failed, how long on average;
//! - `spent`: whatever is not a length of time, as its name says
//!   ([`blueprint::labels::unit`]): what it came to in all, and for each
//!   thing the column was given;
//! - `accepted`, where the comparison says what of what it counts is a
//!   thing being accepted ([`kernel::compared::accepted`]): how many of
//!   the column's things were, each once, and that over how many it was
//!   given;
//! - `per_accepted`, beside it: each row of `spent` again, what it came
//!   to in all over how many of the column's things were accepted. What
//!   was spent on things that were not accepted is in it: it is what the
//!   way spent for each thing that was taken;
//! - `took`: everything else, as it was measured.
//!
//! And where the comparison's shares change by a rule, or were said again
//! by somebody, `rolled`: each way's part of a hundred, the rule in a
//! sentence with what it has read of its two ways since the shares were
//! last set, and each time they were said again with why
//! ([`crate::rolled`]).
//!
//! Nothing here is a figure of merit. No column is ordered, marked or
//! named as the better one, and nothing is added across the texts a way
//! ran: which way did better depends on what whoever started the
//! comparison wanted to find out.

use std::collections::BTreeMap;

use crate::{ACCEPTED, CAME, Comparison, GIVEN, TEXT, WAY};
use journal::{MetricSeries, MetricStats};
use serde_json::{Value, json};

/// The labels a series under a comparison is told apart by, beside its
/// name: the way and the text, what was counted and what it came to, and
/// what was chosen.
pub const BY: [&str; 5] = [WAY, TEXT, "counted", "came", "decision"];

struct Column<'a> {
    way: &'a str,
    text: String,
    version: Option<&'a str>,
    /// Whether it is the text the way runs now, said where it ran several.
    now: bool,
    given: Option<u64>,
    /// How many of its things were accepted, where any was.
    accepted: Option<u64>,
    /// Where the way reads other words in place of a file that is linked:
    /// the file, the one read in its place, and the start of the revision
    /// of the words this column was run with.
    reads: Vec<Value>,
}

fn label<'a>(series: &'a MetricSeries, name: &str) -> &'a str {
    series.labels.get(name).map(String::as_str).unwrap_or_default()
}

/// Which text of its way a series is of. One that says none (measured
/// before texts were told apart) is of the way's one text, where the way
/// has run only one: it was that.
fn text_of<'a>(one: &'a Comparison, series: &'a MetricSeries) -> &'a str {
    let said = label(series, TEXT);
    let way = one.ways.iter().find(|way| way.name == label(series, WAY));
    match way.map(|way| way.texts.as_slice()) {
        Some([only]) if said.is_empty() => &only.text,
        _ => said,
    }
}

/// A column for each text of each way, in the order the ways were said
/// and their texts came, and then any text something was measured under
/// that the way does not list.
fn columns<'a>(one: &'a Comparison, measured: &'a [MetricSeries]) -> Vec<Column<'a>> {
    let mut all = Vec::new();
    for way in &one.ways {
        let mut texts: Vec<&str> = way.texts.iter().map(|ran| ran.text.as_str()).collect();
        let mut seen: Vec<&str> = measured
            .iter()
            .filter(|series| label(series, WAY) == way.name)
            .map(|series| text_of(one, series))
            .collect();
        seen.sort_unstable();
        seen.dedup();
        texts.extend(seen.into_iter().filter(|text| !way.texts.iter().any(|ran| ran.text == *text)));
        // A way nothing has gone yet has its place all the same.
        if texts.is_empty() {
            texts.push("");
        }
        let alone = texts.len() == 1;
        for text in texts {
            let of = |counted: &str| {
                measured.iter().find(|series| {
                    series.name == CAME
                        && label(series, "counted") == counted
                        && label(series, WAY) == way.name
                        && text_of(one, series) == text
                })
            };
            let counted = of(GIVEN);
            let ran = way.texts.iter().find(|ran| ran.text == text);
            let words = |linked: &str| {
                let revision = ran.and_then(|ran| ran.prompts.get(linked));
                revision.map(|revision| revision.chars().take(8).collect::<String>())
            };
            let reads = way
                .prompts
                .iter()
                .map(|(linked, reads)| json!({ "for": linked, "name": reads.name, "words": words(linked) }))
                .collect();
            all.push(Column {
                way: &way.name,
                text: text.to_string(),
                reads,
                version: way.texts.iter().find(|ran| ran.text == text).and_then(|ran| ran.version.as_deref()),
                now: way.texts.len() > 1 && way.texts.last().is_some_and(|last| last.text == text),
                // One begun before things were counted as given says how
                // many went the way, where the way ran one text.
                given: counted.map(|series| series.total.count).or(alone.then_some(way.went)),
                accepted: of(ACCEPTED).map(|series| series.total.count),
            });
        }
    }
    all
}

/// What a row is of: its kind, and what it is called.
fn row_of(series: &MetricSeries) -> (u8, String) {
    row_named(&series.name, |name| label(series, name))
}

/// What a row of a measurement called `name` is of, by the labels it
/// carries (`label`).
fn row_named<'a>(name: &str, label: impl Fn(&str) -> &'a str) -> (u8, String) {
    let (counted, came, decision) = (label("counted"), label("came"), label("decision"));
    if name == CAME {
        return (
            0,
            [counted, came].iter().filter(|word| !word.is_empty()).copied().collect::<Vec<_>>().join(" "),
        );
    }
    match (decision.is_empty(), blueprint::labels::unit(name)) {
        (_, Some(_)) => (SPENT, name.to_string()),
        (false, None) => (1, format!("{name} {decision}")),
        (true, None) => (TOOK, name.to_string()),
    }
}

const KINDS: [&str; 6] = ["came", "chose", "spent", "accepted", "per_accepted", "took"];

/// Where each kind that is not `came` or `chose` is among [`KINDS`].
const SPENT: u8 = 2;
const ACCEPTED_KIND: u8 = 3;
const PER_ACCEPTED: u8 = 4;
const TOOK: u8 = 5;

/// What was measured under one column for one row, added up: a series
/// that says no text falls where its way's one text is, beside the rest.
#[derive(Clone, Copy, Default)]
struct Figures {
    count: u64,
    errors: u64,
    sum: f64,
}

impl Figures {
    fn add(&mut self, stats: &MetricStats) {
        self.count += stats.count;
        self.errors += stats.errors;
        self.sum += stats.sum;
    }

    fn cell(&self, kind: u8, given: Option<u64>) -> Value {
        let avg = if self.count == 0 { 0.0 } else { self.sum / self.count as f64 };
        let mut cell = json!({ "count": self.count, "errors": self.errors, "avg": avg, "sum": self.sum });
        if kind == SPENT
            && let Some(given) = given.filter(|given| *given > 0)
        {
            cell["per_thing"] = json!(self.sum / given as f64);
        }
        cell
    }
}

/// A row as it is put together: the measurement it is of, and for each
/// column its figures, where anything was measured there.
struct Row {
    measured: String,
    by: Vec<Option<Figures>>,
}

/// Everything measured under `one`, laid out: `columns` (each `way`,
/// `text`, `version`, `now`, `given`) and `rows` (each `kind`, `what`,
/// `unit` where it is not a time, and `by`: for each column in order, its
/// figures or nothing).
pub fn beside(one: &Comparison, measured: &[MetricSeries]) -> Value {
    let columns = columns(one, measured);
    let mut rows: BTreeMap<(u8, String), Row> = BTreeMap::new();
    for series in measured {
        if series.name == CAME && [GIVEN, ACCEPTED].contains(&label(series, "counted")) {
            continue;
        }
        let at = columns
            .iter()
            .position(|column| column.way == label(series, WAY) && column.text == text_of(one, series));
        let Some(at) = at else { continue };
        let row = rows
            .entry(row_of(series))
            .or_insert_with(|| Row { measured: series.name.clone(), by: vec![None; columns.len()] });
        row.by[at].get_or_insert_with(Figures::default).add(&series.total);
    }
    let given: Vec<Option<u64>> = columns.iter().map(|column| column.given).collect();
    // What of its things each column had accepted, where the comparison says what is.
    let accepted: Option<Vec<Option<u64>>> =
        one.accepts().then(|| columns.iter().map(|column| column.accepted).collect());
    let columns: Vec<Value> = columns
        .iter()
        .map(|column| {
            json!({
                "way": column.way, "text": column.text, "version": column.version, "now": column.now,
                "given": column.given, "reads": column.reads,
            })
        })
        .collect();
    // What came of the things is read in the order the counts were said in.
    let said = |name: &str| one.counts.iter().position(|count| count.name == name).unwrap_or(usize::MAX);
    let mut laid = laid(columns, &given, accepted.as_deref(), rows, said);
    // Its shares as they are, the rule they change by and each time they were said again.
    if one.rule.is_some() || !one.shared.is_empty() {
        laid["rolled"] = crate::rolled::shown(one);
    }
    laid
}

/// Rows put in their order and written out beside `columns`, each of
/// which was given so many things (`given`). What came of the things is
/// read in the order it comes about: by the order the counts were said
/// in (`said`, of what a row counts), and then by how long after a thing
/// was given its way it came, over every column together. That is the
/// order of a thing's journey and says nothing of any column.
fn laid(
    columns: Vec<Value>,
    given: &[Option<u64>],
    accepted: Option<&[Option<u64>]>,
    rows: BTreeMap<(u8, String), Row>,
    said: impl Fn(&str) -> usize,
) -> Value {
    let place = |kind: u8, what: &str, row: &Row| {
        if kind != 0 {
            return (0, 0);
        }
        let name = what.split(' ').next().unwrap_or_default();
        let (sum, count) = row
            .by
            .iter()
            .flatten()
            .fold((0.0, 0), |(sum, count), figures| (sum + figures.sum, count + figures.count));
        (said(name), if count == 0 { 0 } else { (sum / count as f64) as u64 })
    };
    let mut rows: Vec<_> = rows.into_iter().collect();
    rows.sort_by_cached_key(|((kind, what), row)| (*kind, place(*kind, what, row), what.clone()));
    let written = |kind: u8, what: &str, row: &Row, by: Vec<Value>| {
        let mut laid =
            json!({ "kind": KINDS[kind as usize], "what": what, "measured": row.measured, "by": by });
        if let Some(unit) = blueprint::labels::unit(&row.measured) {
            laid["unit"] = json!(unit);
        }
        laid
    };
    let mut laid = Vec::new();
    let mut owed = accepted;
    for ((kind, what), row) in &rows {
        // What was accepted, and what was spent for each thing accepted,
        // come after what was spent and before the rest.
        if *kind == TOOK
            && let Some(accepted) = owed.take()
        {
            laid.extend(accepting(&rows, given, accepted, &written));
        }
        let by = row
            .by
            .iter()
            .zip(given)
            .map(|(figures, given)| figures.map_or(Value::Null, |figures| figures.cell(*kind, *given)));
        laid.push(written(*kind, what, row, by.collect()));
    }
    if let Some(accepted) = owed {
        laid.extend(accepting(&rows, given, accepted, &written));
    }
    json!({ "columns": columns, "rows": laid })
}

/// The rows of `accepted` and `per_accepted`: how many of each column's
/// things were accepted, and that over how many it was given (where it
/// was given any); and each row of `spent` again over how many were
/// accepted (where any was). A column given nothing has nothing here.
fn accepting(
    rows: &[((u8, String), Row)],
    given: &[Option<u64>],
    accepted: &[Option<u64>],
    written: &impl Fn(u8, &str, &Row, Vec<Value>) -> Value,
) -> Vec<Value> {
    let them = Row { measured: CAME.to_string(), by: Vec::new() };
    let how_many: Vec<Value> = given
        .iter()
        .zip(accepted)
        .map(|(given, accepted)| match (given.filter(|given| *given > 0), accepted.unwrap_or(0)) {
            (None, 0) => Value::Null,
            (None, count) => json!({ "count": count }),
            (Some(given), count) => {
                json!({ "count": count, "given": given, "rate": count as f64 / given as f64 })
            }
        })
        .collect();
    let mut laid = vec![written(ACCEPTED_KIND, ACCEPTED, &them, how_many)];
    for ((_, what), row) in rows.iter().filter(|((kind, _), _)| *kind == SPENT) {
        let by = row.by.iter().zip(accepted).map(|(figures, accepted)| match figures {
            None => Value::Null,
            Some(figures) => match accepted.filter(|accepted| *accepted > 0) {
                Some(accepted) => json!({ "sum": figures.sum, "accepted": accepted, "per_accepted": figures.sum / accepted as f64 }),
                None => json!({ "sum": figures.sum, "accepted": 0 }),
            },
        });
        laid.push(written(PER_ACCEPTED, what, row, by.collect()));
    }
    laid
}

/// One text of a side by side: what it is called there, its text, and
/// everything that came of it in its sandbox, as the sandbox hands it.
pub struct Played<'a> {
    pub called: &'a str,
    pub source: &'a str,
    pub shown: &'a Value,
}

/// What the runner says of its own work, as it does it: the same of any
/// text, and no part of what came of one. What it measures of its own
/// work is left out by the same words (`handler.event`, how often a
/// handler was woken, which counts what it then passed over; how it
/// caught up; when an instance began to listen): how often the runner
/// was woken for a text is the machine's order and not the text's. A
/// child's handler that began to listen before its parent said it was
/// active was woken by that too, and one that began a moment after was
/// not, so the same text played twice counted one or two, and side by
/// side a column differed from itself. What ran (`run`, `step`) is the
/// text's, and stays.
const ITS_OWN: [&str; 6] = ["run.", "step.", "instance.", "handler.", "hook.", "system."];

/// Whether a series is the runner's measure of its own work.
fn its_own(name: &str) -> bool {
    ITS_OWN.iter().any(|own| name.starts_with(own))
}

/// Sandbox tests side by side, laid out as a comparison is: a column for
/// each text, each of which was given one thing (the one set of acts
/// played against it), and a row for each thing counted, in the same
/// four kinds. A sandbox measures nothing under a comparison, so what is
/// laid out is what it hands:
///
/// - `came`: each event said in it that is not the runner's own saying
///   of its work, by its topic: how many, and on average how long after
///   the sandbox was made on its clock, which only the acts move;
/// - `chose`, `spent`, `took`: what was measured of what ran, of a
///   blueprint, by the same rule as a comparison's, and none of what
///   the runner measures of its own work;
/// - and, as `took`, each action that was stood in, by how many times it
///   was asked and how many of those it was answered to fail: a stand-in
///   measures nothing, and how often a text asks one is what came of the
///   text.
///
/// Nothing here is a figure of merit, and nothing orders, marks or names
/// a column as better: the columns are in the order they were asked for.
pub fn sandboxes(played: &[Played]) -> Value {
    let mut rows: BTreeMap<(u8, String), Row> = BTreeMap::new();
    let mut add = |at: usize, kind: u8, what: String, measured: &str, count: u64, errors: u64, sum: f64| {
        let row = rows
            .entry((kind, what))
            .or_insert_with(|| Row { measured: measured.to_string(), by: vec![None; played.len()] });
        let figures = row.by[at].get_or_insert_with(Figures::default);
        figures.add(&MetricStats {
            count,
            errors,
            sum,
            error_rate: 0.0,
            avg: 0.0,
            min: 0.0,
            max: 0.0,
            p50: 0.0,
            p95: 0.0,
        });
    };
    let list =
        |shown: &Value, name: &str| shown.get(name).and_then(Value::as_array).cloned().unwrap_or_default();
    let number = |value: &Value| value.as_f64().unwrap_or_default();
    for (at, one) in played.iter().enumerate() {
        let began = one.shown.get("began").and_then(Value::as_i64).unwrap_or_default();
        for event in list(one.shown, "happened") {
            let topic = event.get("topic").and_then(Value::as_str).unwrap_or_default();
            if topic.is_empty() || its_own(topic) {
                continue;
            }
            let after = event.get("ts").and_then(Value::as_i64).unwrap_or(began) - began;
            add(at, 0, topic.to_string(), topic, 1, 0, after.max(0) as f64);
        }
        for series in list(one.shown, "measured") {
            let name = series.get("name").and_then(Value::as_str).unwrap_or_default();
            if its_own(name) {
                continue;
            }
            let labels = series.get("labels").cloned().unwrap_or_default();
            let label = |key: &str| labels.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let (counted, came, decision) = (label("counted"), label("came"), label("decision"));
            let said = |key: &str| match key {
                "counted" => counted.as_str(),
                "came" => came.as_str(),
                "decision" => decision.as_str(),
                _ => "",
            };
            let (kind, what) = row_named(name, said);
            let total = &series["total"];
            add(
                at,
                kind,
                what,
                name,
                number(&total["count"]) as u64,
                number(&total["errors"]) as u64,
                number(&total["sum"]),
            );
        }
        for asking in list(one.shown, "asked") {
            let action = asking.get("action").and_then(Value::as_str).unwrap_or_default();
            let failed = asking.get("failed").is_some_and(|why| !why.is_null());
            add(at, TOOK, format!("{action} stood in"), "stood", 1, u64::from(failed), 0.0);
        }
    }
    let columns: Vec<Value> = played
        .iter()
        .map(|one| {
            let revision: String = blueprint::revision(one.source).chars().take(8).collect();
            json!({
                "way": one.called, "text": revision, "version": blueprint::version(one.source),
                "now": false, "given": 1, "reads": [],
            })
        })
        .collect();
    laid(columns, &vec![Some(1); played.len()], None, rows, |_| 0)
}
