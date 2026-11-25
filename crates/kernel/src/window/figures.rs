//! What is counted of the rows a view shows: how many, how many in each
//! band of the column it is gathered by, and the figure at the foot of a
//! column (`crate::columns::FIGURES`).
//!
//! The rules are the client's (`ui/src/figures.ts`, `bands` of
//! `ui/src/ordered.ts`) said again, and both are held to one table of
//! cases (`ui/src/figurecases.json`): a row says nothing in a column where
//! it holds nothing, an empty text or an empty list; a figure with nothing
//! to go by is no figure, the mean of no numbers not nought; what no
//! numbers add up to is nought. It counts and never scores.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use super::{BANDS, Column, Held, holds, said, shows};
use crate::columns::fits;

/// Whether a value says something.
pub fn says(value: Option<&Value>) -> bool {
    !said(value).trim().is_empty()
}

/// The number a value is, where it is one or is written as one.
pub fn number(value: Option<&Value>) -> Option<f64> {
    if !says(value) {
        return None;
    }
    match value? {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok().filter(|number| number.is_finite()),
        _ => None,
    }
}

/// What is kept of the rows of one column to say its figure.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tally {
    pub rows: u64,
    pub filled: u64,
    pub ticked: u64,
    pub numbers: u64,
    pub sum: f64,
    pub least: Option<f64>,
    pub greatest: Option<f64>,
    pub different: BTreeSet<String>,
}

impl Tally {
    /// One more row, which holds `value` in `column`.
    pub fn take(&mut self, column: &Column, value: Option<&Value>) {
        self.rows += 1;
        if says(value) {
            self.filled += 1;
        }
        if value == Some(&Value::Bool(true)) {
            self.ticked += 1;
        }
        if let Some(number) = number(value) {
            self.numbers += 1;
            self.sum += number;
            self.least = Some(self.least.map_or(number, |least| least.min(number)));
            self.greatest = Some(self.greatest.map_or(number, |greatest| greatest.max(number)));
        }
        if column.kind != "number" && column.kind != "flag" {
            self.different.extend(holds(column, value));
        }
    }

    /// The figure `which` of a column of `kind`; nothing where the column
    /// cannot say it or there is nothing to go by.
    pub fn figure(&self, kind: &str, which: &str) -> Option<f64> {
        if !fits(kind, which) {
            return None;
        }
        match which {
            "filled" => Some(self.filled as f64),
            "empty" => Some((self.rows - self.filled) as f64),
            "ticked" => Some(self.ticked as f64),
            "different" => Some(self.different.len() as f64),
            "sum" => Some(self.sum),
            "mean" => (self.numbers > 0).then(|| self.sum / self.numbers as f64),
            "least" => self.least,
            "greatest" => self.greatest,
            _ => None,
        }
    }
}

/// The figure `which` of `column` over `rows`.
pub fn figure(rows: &[Map<String, Value>], column: &Column, which: &str) -> Option<f64> {
    let mut tally = Tally::default();
    for row in rows {
        tally.take(column, row.get(&column.name));
    }
    tally.figure(&column.kind, which)
}

/// What a counting of a view has come to so far: how many rows it
/// shows, how many in each band, and what is kept for each figure asked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counting {
    /// How many rows of the table were gone through.
    pub looked: u64,
    pub count: u64,
    pub bands: BTreeMap<String, u64>,
    pub tallies: Vec<Tally>,
}

impl Counting {
    /// One more row of the table, holding `data`: counted where the view
    /// held to `held` shows it, in the bands of `group` and the figures
    /// of `figured`.
    pub fn take(
        &mut self,
        held: &[Held],
        group: Option<&Column>,
        figured: &[Column],
        data: &Map<String, Value>,
    ) {
        if !shows(held, data) {
            return;
        }
        self.count += 1;
        if let Some(group) = group {
            let things = holds(group, data.get(&group.name));
            if things.is_empty() {
                *self.bands.entry(String::new()).or_default() += 1;
            }
            for thing in things {
                *self.bands.entry(thing).or_default() += 1;
            }
        }
        self.tallies.resize(figured.len(), Tally::default());
        for (tally, column) in self.tallies.iter_mut().zip(figured) {
            tally.take(column, data.get(&column.name));
        }
    }
}

/// Bands in the order they are shown, at most [`BANDS`], with how many
/// more there are: a choice in the order of what it offers, then what it
/// no longer offers; a record by the number its id ends in; anything
/// else by its words; and last the band of rows that name nothing.
pub fn ordered_bands(column: &Column, counts: &BTreeMap<String, u64>) -> (Vec<(String, u64)>, usize) {
    let place = |key: &str| -> (u64, i64) {
        if column.kind == "choice" {
            return (column.of.iter().position(|one| one == key).unwrap_or(column.of.len()) as u64, 0);
        }
        if column.kind == "record" || column.kind == "records" {
            let number = key.rsplit('-').next().and_then(|number| number.parse::<i64>().ok());
            return number.map_or((1, 0), |number| (0, number));
        }
        (0, 0)
    };
    let mut some: Vec<(String, u64)> =
        counts.iter().filter(|(key, _)| !key.is_empty()).map(|(key, count)| (key.clone(), *count)).collect();
    some.sort_by(|(one, _), (two, _)| {
        place(one)
            .cmp(&place(two))
            .then_with(|| one.to_lowercase().cmp(&two.to_lowercase()))
            .then_with(|| one.cmp(two))
    });
    let more = some.len().saturating_sub(BANDS);
    some.truncate(BANDS);
    if let Some(none) = counts.get("") {
        some.push((String::new(), *none));
    }
    (some, more)
}
