//! What a part of rows, of runs and of measures comes to when it is read,
//! and which tab of a line's page each part is in: functions of what is
//! handed, read for whoever reads the line by whoever keeps the table, the
//! runs and the measurements (`told`).

use super::{MOST_COLUMNS, MOST_ROWS, Rows};

/// The tabs of a line's page, in order, each with the parts in it by
/// their places among the parts (counted from 0): first the one with no
/// name, where a part says none, and then each named as it is first said.
/// A page with one tab draws no row of tabs.
pub fn tabs(said: &[String]) -> Vec<(String, Vec<usize>)> {
    let mut tabs: Vec<(String, Vec<usize>)> = Vec::new();
    if said.iter().any(String::is_empty) {
        tabs.push((String::new(), Vec::new()));
    }
    for (at, tab) in said.iter().enumerate() {
        match tabs.iter_mut().find(|(name, _)| name == tab) {
            Some((_, parts)) => parts.push(at),
            None => tabs.push((tab.clone(), vec![at])),
        }
    }
    tabs
}

/// The columns a part of rows shows, of those its table has
/// (`declared`, in the table's order), and those it names that the table
/// has not got: what it names, in its order, or where it names none the
/// table's own, as many as are shown at most.
pub fn columns(rows: &Rows, declared: &[&str]) -> (Vec<String>, Vec<String>) {
    if rows.columns.is_empty() {
        return (declared.iter().take(MOST_COLUMNS).map(|cell| cell.to_string()).collect(), Vec::new());
    }
    rows.columns.iter().cloned().partition(|cell| declared.contains(&cell.as_str()))
}

/// One row a part of rows lists: where it is among the rows read, or its
/// id where no row of the table has it any more.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chosen {
    At(usize),
    Gone(String),
}

/// The rows a part lists, of the table's rows (`ids`, in the table's
/// order) and of those `saying` chose (`those`, as `rows.read` gives
/// them): by its ids in their order, one no row has said to be gone; or
/// those chosen, in the table's order. At most [`MOST_ROWS`], with how
/// many more there are.
pub fn chosen(rows: &Rows, ids: &[&str], those: &[&str]) -> (Vec<Chosen>, usize) {
    let at = |id: &str| ids.iter().position(|one| *one == id);
    let listed: Vec<Chosen> = match &rows.ids {
        Some(asked) => {
            asked.iter().map(|id| at(id).map_or_else(|| Chosen::Gone(id.clone()), Chosen::At)).collect()
        }
        None => those.iter().filter_map(|id| at(id).map(Chosen::At)).collect(),
    };
    let more = listed.len().saturating_sub(MOST_ROWS);
    (listed.into_iter().take(MOST_ROWS).collect(), more)
}

/// Whether a column holds seconds, to be drawn as a length of time: a
/// number whose name says so, as a measurement says what it is of by how
/// its name ends (`seconds`, `run_seconds`).
pub fn in_seconds(name: &str, kind: &str) -> bool {
    kind == "number" && (name == "seconds" || name.ends_with("_seconds"))
}

/// What a run is a run of, in words: what was applied (`preview`, or
/// `preview as dev-preview` where its key is another), and after it the
/// handler whose run it is, or that it took a new text.
pub fn run_called(blueprint: &str, key: &str, flow: &str) -> String {
    let what = match key.is_empty() || key == blueprint {
        true => blueprint.to_string(),
        false => format!("{blueprint} as {key}"),
    };
    match flow {
        "main" | "" => what,
        "on_update" => format!("{what}, taking a new text"),
        handler => format!("{what}, on {}", handler.trim_start_matches("on.")),
    }
}

/// A measurement's latest value and the median of its last few.
#[derive(Clone, Debug, PartialEq)]
pub struct Measured {
    pub latest: f64,
    /// When the latest was measured, in milliseconds.
    pub at: i64,
    pub median: f64,
    /// How many values the median is of: `last`, or fewer where fewer were measured.
    pub of: usize,
}

/// What a measurement's values come to, each with when it was measured,
/// oldest first: the latest, and the median of the `last` latest, the
/// latest among them (of an even number, the mean of the middle two).
/// Nothing, where nothing was measured. It counts, and says nothing of
/// whether either is good.
pub fn measured(values: &[(i64, f64)], last: usize) -> Option<Measured> {
    let &(at, latest) = values.last()?;
    let mut lately: Vec<f64> = values.iter().rev().take(last.max(1)).map(|(_, value)| *value).collect();
    lately.sort_by(f64::total_cmp);
    let middle = lately.len() / 2;
    let median = match lately.len() % 2 {
        1 => lately[middle],
        _ => (lately[middle - 1] + lately[middle]) / 2.0,
    };
    Some(Measured { latest, at, median, of: lately.len() })
}

/// The most letters of a cell's words a part of rows hands: a row is
/// listed on a line, and the whole of it is its form.
pub const MOST_SAID: usize = 300;

/// A cell as a part of rows hands it: words cut at [`MOST_SAID`] letters
/// with an ellipsis, and anything else as it is.
pub fn brief(cell: &serde_json::Value) -> serde_json::Value {
    match cell.as_str() {
        Some(words) if words.chars().count() > MOST_SAID => {
            serde_json::Value::String(format!("{}…", words.chars().take(MOST_SAID).collect::<String>()))
        }
        _ => cell.clone(),
    }
}
