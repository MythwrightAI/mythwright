//! What a view of a table says of its columns: the order they stand in,
//! which of them it is frozen up to, the figure at the foot of each, and
//! how wide each is.
//!
//! A view is the table's, kept with it for everybody, and these are four
//! things it may say beside how it is laid out. They are plain values and
//! functions of them, so they are here and not where views are kept: a
//! list of names, a name, a map from a name to one of [`FIGURES`], and a
//! map from a name to a width. Nothing here knows a table; whoever keeps
//! views hands over what the table's columns are called and what kind
//! each is.
//!
//! The order and the widths are passed over where they name a column the
//! table has not got, and never refused for it: a table may be declared
//! again without a column, and the view outlasts that. The frozen edge
//! and the figures name columns that are there when they are said, and
//! are refused otherwise, in a sentence. A width is refused where it is
//! no width a column can sensibly be drawn at ([`NARROWEST`],
//! [`WIDEST`]), wherever it names.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Error, Result};

/// What can be said at the foot of a column, of the rows a view shows:
/// how many say something there and how many say nothing; what a number
/// adds up to, its mean, the least and the greatest; how many of a flag
/// are ticked; and how many different things are said.
pub const FIGURES: &[&str] = &["filled", "empty", "sum", "mean", "least", "greatest", "ticked", "different"];

/// The narrowest a column may be said to be, in pixels at the usual size
/// of text: room for the mark of its kind and two or three letters of
/// what it is called, with the edge it is dragged by, and no less. A
/// column narrower than that is a line with nothing in it to press.
pub const NARROWEST: u32 = 60;

/// The widest a column may be said to be, in pixels at the usual size of
/// text: wider than a column is ever dragged on any screen a table is
/// read on, so that nothing anybody drags to is refused, and a number
/// typed with a figure too many is.
pub const WIDEST: u32 = 2000;

/// Whether a column of this kind can have this figure at its foot. How
/// many rows say something, and how many nothing, can be counted of
/// anything; only a number is added up or has a least and a greatest;
/// only a flag is ticked; and how many different things are said is
/// counted of anything that is not one of those two.
pub fn fits(kind: &str, figure: &str) -> bool {
    match figure {
        "filled" | "empty" => true,
        "sum" | "mean" | "least" | "greatest" => kind == "number",
        "ticked" => kind == "flag",
        "different" => kind != "number" && kind != "flag",
        _ => false,
    }
}

/// What a view says of its columns. Each is nothing where it is not said,
/// and a view kept before any of them could be said reads as saying none.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Columns {
    /// The columns in the order they stand (`columns`), by name. One it
    /// does not name stands after those it names, where it always stood
    /// among the rest; a name that is no column is passed over.
    #[serde(default, rename = "columns", skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    /// The column it is frozen up to (`freezes`): that one and those
    /// before it stay where they are while the rest are scrolled sideways.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freezes: Option<String>,
    /// The figure at the foot of a column (`figures`): for each column
    /// named, one of [`FIGURES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub figures: Option<BTreeMap<String, String>>,
    /// How wide a column is (`widths`): for each column named, whole
    /// pixels at the usual size of text, which a page draws as that many
    /// times the size its reader's look says. One it does not name is as
    /// wide as what is in it; a name that is no column is passed over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widths: Option<BTreeMap<String, u32>>,
}

impl Columns {
    /// Reads the four as a blueprint writes them in a view and as a page
    /// sends them: `columns: [title, due]`, `freezes: title`, `figures:
    /// { points: sum }`, `widths: { title: 320 }`. Nothing, an empty list
    /// and an empty map are each nothing said; anything written another
    /// way, and a width no column can be drawn at, is refused in a
    /// sentence that says how it is written.
    pub fn read(view: &str, about: &Map<String, Value>) -> Result<Columns> {
        let said = |key: &str| about.get(key).filter(|said| !said.is_null());
        let wrong = |key: &str, shape: &str| {
            Error::invalid(format!("the view `{view}` says `{key}`, which is {shape}"))
        };
        let order = match said("columns") {
            None => None,
            Some(Value::Array(names)) if names.iter().all(Value::is_string) => {
                let mut order: Vec<String> = Vec::new();
                for name in names.iter().filter_map(Value::as_str).map(str::trim) {
                    if !name.is_empty() && !order.iter().any(|kept| kept == name) {
                        order.push(name.to_string());
                    }
                }
                (!order.is_empty()).then_some(order)
            }
            Some(_) => {
                return Err(wrong(
                    "columns",
                    "a list of its columns in the order they stand, as in `columns: [title, due]`",
                ));
            }
        };
        let freezes = match said("freezes") {
            None => None,
            Some(Value::String(name)) => Some(name.trim().to_string()).filter(|name| !name.is_empty()),
            Some(_) => {
                return Err(wrong("freezes", "the one column it is frozen up to, as in `freezes: title`"));
            }
        };
        let figures = match said("figures") {
            None => None,
            Some(Value::Object(map)) if map.values().all(Value::is_string) => {
                let figures: BTreeMap<String, String> = map
                    .iter()
                    .map(|(name, figure)| {
                        (name.clone(), figure.as_str().unwrap_or_default().trim().to_string())
                    })
                    .collect();
                (!figures.is_empty()).then_some(figures)
            }
            Some(_) => {
                return Err(wrong(
                    "figures",
                    "a map from a column to the figure at its foot, as in `figures: { points: sum }`",
                ));
            }
        };
        let widths = match said("widths") {
            None => None,
            Some(Value::Object(map)) => {
                let mut widths = BTreeMap::new();
                for (name, width) in map {
                    let name = name.trim();
                    let Some(width) = width.as_u64().and_then(|width| u32::try_from(width).ok()) else {
                        return Err(wrong(
                            "widths",
                            "a map from a column to how wide it is in whole pixels, as in `widths: { title: 320 }`",
                        ));
                    };
                    if !(NARROWEST..=WIDEST).contains(&width) {
                        return Err(Error::invalid(format!(
                            "the view `{view}` says `{name}` is {width} pixels wide, and a column is from {NARROWEST} to {WIDEST}"
                        )));
                    }
                    if !name.is_empty() {
                        widths.insert(name.to_string(), width);
                    }
                }
                (!widths.is_empty()).then_some(widths)
            }
            Some(_) => {
                return Err(wrong(
                    "widths",
                    "a map from a column to how wide it is in whole pixels, as in `widths: { title: 320 }`",
                ));
            }
        };
        Ok(Columns { order, freezes, figures, widths })
    }

    /// Says what is wrong with these for the view `view`, laid out as a
    /// `kind`, of a table whose columns are `fields`, each by its name and
    /// its kind; nothing where nothing is. Asked as a view is declared or
    /// saved, and never of one that is kept: a column may be taken away
    /// later, and what names it is then passed over where it is read.
    pub fn check(&self, view: &str, kind: &str, fields: &[(&str, &str)]) -> Result<()> {
        let has = |named: &str| fields.iter().find(|(name, _)| *name == named).map(|(_, kind)| *kind);
        let columns = || fields.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(", ");
        if let Some(named) = self.freezes.as_deref() {
            if kind != "table" {
                return Err(Error::invalid(format!(
                    "the view `{view}` says `freezes: {named}`, which is for a table: the column it is frozen up to, which stays with those before it while the rest are scrolled sideways"
                )));
            }
            if has(named).is_none() {
                return Err(Error::invalid(format!(
                    "the view `{view}` is frozen up to `{named}`, which is not a column of the table; it has: {}",
                    columns()
                )));
            }
        }
        for (named, figure) in self.figures.iter().flatten() {
            if kind != "table" {
                return Err(Error::invalid(format!(
                    "the view `{view}` says `figures`, which is for a table: a figure at the foot of a column"
                )));
            }
            let Some(of) = has(named) else {
                return Err(Error::invalid(format!(
                    "the view `{view}` says a figure at the foot of `{named}`, which is not a column of the table; it has: {}",
                    columns()
                )));
            };
            if !FIGURES.contains(&figure.as_str()) {
                return Err(Error::invalid(format!(
                    "the view `{view}` says `{figure}` at the foot of `{named}`, which is not a figure; there are: {}",
                    FIGURES.join(", ")
                )));
            }
            if !fits(of, figure) {
                let can: Vec<&str> = FIGURES.iter().copied().filter(|figure| fits(of, figure)).collect();
                return Err(Error::invalid(format!(
                    "the view `{view}` says `{figure}` at the foot of `{named}`, which is a `{of}`: a column of that kind can say {}",
                    can.join(", ")
                )));
            }
        }
        if self.widths.is_some() && kind != "table" {
            return Err(Error::invalid(format!(
                "the view `{view}` says `widths`, which is for a table: how wide each of its columns is"
            )));
        }
        Ok(())
    }
}
