//! Words searched for among a view's rows, and a row gone to by its id:
//! what a search shows, as the database checks it and as a row in hand
//! is checked, and where a row stands among those a view shows.
//!
//! A row is found where every word is in one of the columns searched, as
//! words with capitals folded (`lower`, ASCII alone, as the order folds
//! them), anywhere in them: a person looking for a commit types the
//! middle of one. The columns searched are those a view shows that say
//! something in words: text, a choice, somebody, a list, a date and a
//! number as its digits, and never a tick or what names a record by its
//! id. It is checked row by row as a window is walked, so a search of a
//! large table goes in answers that each look at so many rows.

use serde_json::{Map, Value};

use super::read::From;
use super::{Column, Held, Order, Place, literal, path, said, shows};

/// At most this many words are looked for.
pub const WORDS: usize = 16;

/// What a search looks for, and where.
#[derive(Clone, Debug, PartialEq)]
pub struct Find {
    /// Each word, folded, once.
    pub words: Vec<String>,
    pub columns: Vec<Column>,
}

/// Whether a column of `kind` is looked through.
pub fn searched(kind: &str) -> bool {
    !matches!(kind, "flag" | "record" | "records")
}

/// The words of what was typed, folded, each once and none empty.
pub fn words(typed: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for word in typed.split_whitespace().map(str::to_ascii_lowercase) {
        if !words.contains(&word) && words.len() < WORDS {
            words.push(word);
        }
    }
    words
}

impl Find {
    /// A search for what was `typed` among `columns` but those the view
    /// leaves out (`hides`); nothing where nothing was typed.
    pub fn of(columns: &[Column], hides: &[String], typed: &str) -> Option<Find> {
        let words = words(typed);
        if words.is_empty() {
            return None;
        }
        let columns = columns
            .iter()
            .filter(|column| searched(&column.kind) && !hides.contains(&column.name))
            .cloned()
            .collect();
        Some(Find { words, columns })
    }

    /// Whether a row holding `data` is found.
    pub fn finds(&self, data: &Map<String, Value>) -> bool {
        let texts: Vec<String> =
            self.columns.iter().map(|column| said(data.get(&column.name)).to_ascii_lowercase()).collect();
        self.words.iter().all(|word| texts.iter().any(|text| text.contains(word.as_str())))
    }

    /// The same, as the database checks it of each row.
    pub fn sql(&self) -> String {
        if self.columns.is_empty() {
            return "0".into();
        }
        let texts: Vec<String> = self
            .columns
            .iter()
            .map(|column| {
                let at = path(&column.name);
                let text = if column.several() {
                    format!("(SELECT group_concat(CAST(value AS TEXT), ', ') FROM json_each(data, {at}))")
                } else {
                    format!("CAST(json_extract(data, {at}) AS TEXT)")
                };
                format!("lower(coalesce({text}, ''))")
            })
            .collect();
        let each: Vec<String> = self
            .words
            .iter()
            .map(|word| {
                let word = literal(word);
                let any: Vec<String> =
                    texts.iter().map(|text| format!("instr({text}, {word}) > 0")).collect();
                format!("({})", any.join(" OR "))
            })
            .collect();
        each.join(" AND ")
    }
}

/// What a view checks of each row, as the database says it: what it is
/// held to, and what is searched for.
pub fn shows_sql(held: &[Held], find: Option<&Find>) -> String {
    match find {
        None => super::shows_sql(held),
        Some(find) => format!("({}) AND ({})", super::shows_sql(held), find.sql()),
    }
}

/// Whether a view held to `held` and searched for `find` shows a row.
pub fn shown(held: &[Held], find: Option<&Find>, data: &Map<String, Value>) -> bool {
    shows(held, data) && find.is_none_or(|find| find.finds(data))
}

/// What keeps a row out of a view: each column it is held to that the
/// row holds none of what it is held to in, by name, and `find` where it
/// is not found. Nothing where the view shows it.
pub fn kept_out(held: &[Held], find: Option<&Find>, data: &Map<String, Value>) -> Vec<String> {
    let mut out: Vec<String> = held
        .iter()
        .filter(|one| !shows(std::slice::from_ref(*one), data))
        .map(|one| one.column.name.clone())
        .collect();
    out.dedup();
    if find.is_some_and(|find| !find.finds(data)) {
        out.push("find".into());
    }
    out
}

/// Where a row stands among those of `records` a view shows: how many of
/// them come before it.
pub fn rank_of(order: &Order, held: &[Held], records: &[(i64, &Map<String, Value>)], place: &Place) -> u64 {
    records
        .iter()
        .filter(|(_, data)| shows(held, data))
        .filter(|(number, data)| order.cmp(&order.place(data, *number), place).is_lt())
        .count() as u64
}

/// The two readings a window about a row is read by: those before it,
/// and from it on.
pub fn about(place: &Place) -> (From, From) {
    (From::Before(place.clone()), From::Of(place.clone()))
}
