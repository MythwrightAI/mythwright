//! A table read a window at a time: what a view of it asks, in what
//! order its rows come, which of them it shows, where a reading of it got
//! to and how it is read.
//!
//! A table may hold a million rows. So it is never read whole to show a
//! page of it: the database is asked for the rows of one window, in the
//! order the view is read in, from where the reader got to, and that
//! costs the same whatever the table holds. The order is said here once
//! and both ways: as a function of a row's values ([`Order::place`]),
//! which is what a table kept anywhere but the database is read by and
//! what the statements are held to, and as the expression the database
//! orders and indexes by ([`key_sql`]). What a statement is is the
//! studio's to make (`studio::window`); everything that decides is here,
//! with its cases in the kernel's suite.
//!
//! **The order.** A view is read by the number each row was given where
//! nobody sorts it; sorted by a column, by what that column holds: a
//! number, a date or a tick by its value, a choice by where its option
//! stands among those it offers, a text by its words with capitals
//! folded (`lower`, which folds ASCII alone: `Émile` after `zulu`), and
//! somebody or a record by the id held. Rows that say nothing there come
//! last, either way. Descending turns the whole order round, ties among
//! them; what says nothing stays last. The order is a few stretches
//! ([`Stretch`]), each walked by the column's index in one direction: a
//! choice is a stretch for each option, so that its options put in
//! another order need no index made again.
//!
//! **What a column is keyed by** ([`How`]): its value, its words folded,
//! or of a tick only `true`. A column that holds several things (a list,
//! several records) can be keyed by nothing an index keeps, and sorts
//! only a small table, in the statement.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::{Error, Result};

pub mod figures;
pub mod found;
pub mod read;
pub mod statements;

/// Below this many rows a table is sorted in the statement that reads it,
/// with no index made: a few milliseconds, and nothing to keep.
pub const SMALL: u64 = 10_000;
/// How many entries of an index one answer looks at before it says it
/// fell short and where it got to: some 15 to 35 milliseconds.
pub const LOOK: u64 = 20_000;
/// Where a view is held to values of an indexed column and that comes to
/// at most this many rows, they are read by that column's index and
/// sorted in the statement.
pub const GATHER: u64 = 10_000;
/// At most this many columns of one table are indexed.
pub const MOST_INDEXED: usize = 8;
/// How many rows one window has, where nobody says.
pub const USUAL: usize = 100;
/// How many rows one window has at most.
pub const MOST: usize = 200;
/// How many rows one piece of counting reads (figures, bands, a count
/// that cannot be read off an index): some 45 milliseconds.
pub const PIECE: u64 = 50_000;
/// How many pieces one answer of counting goes through before it says
/// what it has so far.
pub const PIECES: usize = 8;
/// How many bands a view gathered by a column is said in at most.
pub const BANDS: usize = 500;
/// A table kept anywhere but the database is read whole to be read a
/// window at a time; past this many rows the answer says so.
pub const WHOLE_READ: usize = 20_000;

/// A column of a table, as much of it as reading it in order takes: its
/// name, the kind of thing it holds, and of a choice what it offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub kind: String,
    pub of: Vec<String>,
}

impl Column {
    pub fn new(name: &str, kind: &str, of: &[&str]) -> Column {
        Column {
            name: name.to_string(),
            kind: kind.to_string(),
            of: of.iter().map(|one| one.to_string()).collect(),
        }
    }

    /// It holds several things at once: a list of words, or several records.
    pub fn several(&self) -> bool {
        self.kind == "list" || self.kind == "records"
    }

    /// How it is keyed, where an index can key it.
    pub fn how(&self) -> Option<How> {
        match self.kind.as_str() {
            "list" | "records" => None,
            "text" => Some(How::Words),
            "flag" => Some(How::Flag),
            _ => Some(How::Value),
        }
    }
}

/// How a column is keyed, in its index and wherever it is ordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    /// As it is held: a number, a date, an option, an id.
    Value,
    /// Its words, with capitals folded (`lower`).
    Words,
    /// A tick: `true` is said, and anything else is not.
    Flag,
}

/// A value as the database orders it: nothing, then numbers, then words
/// by their bytes. What a row holds in a column comes to one of these
/// ([`key`]), the same as the database's expression for it ([`key_sql`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    Null,
    Number(f64),
    Text(String),
}

impl Key {
    /// The order the database keeps.
    pub fn order(&self, other: &Key) -> Ordering {
        match (self, other) {
            (Key::Null, Key::Null) => Ordering::Equal,
            (Key::Null, _) => Ordering::Less,
            (_, Key::Null) => Ordering::Greater,
            (Key::Number(one), Key::Number(two)) => one.partial_cmp(two).unwrap_or(Ordering::Equal),
            (Key::Number(_), Key::Text(_)) => Ordering::Less,
            (Key::Text(_), Key::Number(_)) => Ordering::Greater,
            (Key::Text(one), Key::Text(two)) => one.as_bytes().cmp(two.as_bytes()),
        }
    }

    /// It as a literal of a statement.
    pub fn sql(&self) -> String {
        match self {
            Key::Null => "NULL".into(),
            Key::Number(number) => written(*number),
            Key::Text(text) => literal(text),
        }
    }

    fn json(&self) -> Value {
        match self {
            Key::Null => Value::Null,
            Key::Number(number) => json!(number),
            Key::Text(text) => json!(text),
        }
    }

    fn from_json(value: &Value) -> Option<Key> {
        match value {
            Value::Null => Some(Key::Null),
            Value::Number(number) => number.as_f64().map(Key::Number),
            Value::String(text) => Some(Key::Text(text.clone())),
            _ => None,
        }
    }
}

/// A number as a statement says it, which reads back as the same number.
fn written(number: f64) -> String {
    if number.fract() == 0.0 && number.abs() < 1e15 {
        format!("{}", number as i64)
    } else {
        format!("{number:?}")
    }
}

/// Words as a literal of a statement: between single quotes, each one in
/// them doubled.
pub fn literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// The path of a column among what a row holds, as a literal.
pub fn path(column: &str) -> String {
    literal(&format!("$.\"{}\"", column.replace('"', "\\\"")))
}

/// A number held as words of text, as the database writes it.
fn as_words(number: &serde_json::Number) -> String {
    match (number.as_i64(), number.as_u64()) {
        (Some(whole), _) => whole.to_string(),
        (_, Some(whole)) => whole.to_string(),
        _ => number.as_f64().map(written).unwrap_or_default(),
    }
}

/// What a row holds in a column as the database takes it before it is
/// keyed: nothing and empty words are nothing.
fn plain(value: Option<&Value>) -> Key {
    match value {
        None | Some(Value::Null) => Key::Null,
        Some(Value::String(text)) if text.is_empty() => Key::Null,
        Some(Value::String(text)) => Key::Text(text.clone()),
        Some(Value::Number(number)) => number.as_f64().map_or(Key::Null, Key::Number),
        Some(Value::Bool(yes)) => Key::Number(if *yes { 1.0 } else { 0.0 }),
        Some(other) => Key::Text(other.to_string()),
    }
}

/// What a row's `value` in `column` is ordered by.
pub fn key(column: &Column, value: Option<&Value>) -> Key {
    match column.how() {
        Some(How::Value) => plain(value),
        Some(How::Words) => match value {
            Some(Value::Number(number)) => Key::Text(as_words(number)),
            _ => match plain(value) {
                Key::Text(text) => Key::Text(text.to_ascii_lowercase()),
                other => other,
            },
        },
        Some(How::Flag) => match plain(value) {
            Key::Number(0.0) => Key::Null,
            other => other,
        },
        None => {
            let joined = match value {
                Some(Value::Array(items)) => {
                    let items: Vec<String> = items.iter().filter_map(item_words).collect();
                    (!items.is_empty()).then(|| items.join(", "))
                }
                None | Some(Value::Null) => None,
                Some(one) => item_words(one),
            };
            joined
                .filter(|joined| !joined.is_empty())
                .map_or(Key::Null, |joined| Key::Text(joined.to_ascii_lowercase()))
        }
    }
}

/// One thing of what a column of several holds, as words.
fn item_words(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(as_words(number)),
        Value::Bool(yes) => Some(if *yes { "1" } else { "0" }.to_string()),
        other => Some(other.to_string()),
    }
}

/// The database's expression for what a row is ordered by in `column`:
/// what its index is made of and every statement that reads by it says,
/// word for word, so that the index is the one asked.
pub fn key_sql(column: &Column) -> String {
    let at = path(&column.name);
    match column.how() {
        Some(How::Value) => format!("nullif(json_extract(data, {at}), '')"),
        Some(How::Words) => format!("lower(nullif(json_extract(data, {at}), ''))"),
        Some(How::Flag) => format!("nullif(nullif(json_extract(data, {at}), ''), 0)"),
        None => format!(
            "lower(nullif((SELECT group_concat(CAST(value AS TEXT), ', ') FROM json_each(data, {at})), ''))"
        ),
    }
}

/// How a reader sorts a view: by a column, and which way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sort {
    pub field: String,
    pub down: bool,
}

/// A stretch of the order, walked by one index in one direction.
#[derive(Clone, Debug, PartialEq)]
pub enum Stretch {
    /// Every row, by its number: a view nobody sorts.
    Numbered,
    /// Every row that says something in the column, by what it says.
    Said,
    /// The rows that chose this option of a choice.
    Option(String),
    /// The rows that hold something a choice no longer offers, by what.
    Others(Vec<String>),
    /// The rows that say nothing in the column.
    Empty,
}

/// The order a view is read in.
#[derive(Clone, Debug, PartialEq)]
pub struct Order {
    /// The column it is sorted by, where it is.
    pub column: Option<Column>,
    pub down: bool,
    pub stretches: Vec<Stretch>,
}

/// Where a row stands in an order: its stretch, what it is keyed by and
/// its number. Two rows never stand in one place.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub stretch: usize,
    pub key: Key,
    pub number: i64,
}

impl Order {
    /// The order of a view of a table with these `columns`, sorted as said.
    pub fn of(columns: &[Column], sort: Option<&Sort>) -> Result<Order> {
        let Some(sort) = sort else {
            return Ok(Order { column: None, down: false, stretches: vec![Stretch::Numbered] });
        };
        let Some(column) = columns.iter().find(|column| column.name == sort.field) else {
            let names: Vec<&str> = columns.iter().map(|column| column.name.as_str()).collect();
            return Err(Error::invalid(format!(
                "`{}` is not a column of this table, and it is sorted by one of its columns: {}",
                sort.field,
                names.join(", ")
            )));
        };
        let stretches = match column.kind.as_str() {
            "choice" => {
                let mut options: Vec<Stretch> = column.of.iter().cloned().map(Stretch::Option).collect();
                if sort.down {
                    options.reverse();
                }
                options.push(Stretch::Others(column.of.clone()));
                options.push(Stretch::Empty);
                options
            }
            _ => vec![Stretch::Said, Stretch::Empty],
        };
        Ok(Order { column: Some(column.clone()), down: sort.down, stretches })
    }

    /// Where a row holding `data` and numbered `number` stands.
    pub fn place(&self, data: &Map<String, Value>, number: i64) -> Place {
        let Some(column) = &self.column else { return Place { stretch: 0, key: Key::Null, number } };
        let key = key(column, data.get(&column.name));
        let stretch = self
            .stretches
            .iter()
            .position(|stretch| match (stretch, &key) {
                (Stretch::Empty, Key::Null) => true,
                (Stretch::Said, Key::Number(_) | Key::Text(_)) => true,
                (Stretch::Option(option), Key::Text(text)) => option == text,
                (Stretch::Others(offered), Key::Text(text)) => !offered.contains(text),
                (Stretch::Others(_), Key::Number(_)) => true,
                _ => false,
            })
            .unwrap_or(self.stretches.len() - 1);
        Place { stretch, key, number }
    }

    /// Which of two places comes first.
    pub fn cmp(&self, one: &Place, two: &Place) -> Ordering {
        one.stretch.cmp(&two.stretch).then_with(|| {
            let within = one.key.order(&two.key).then(one.number.cmp(&two.number));
            if self.down { within.reverse() } else { within }
        })
    }
}

/// What a view is held to in one column: the things a row has to hold
/// one of there to be shown, or (`nothing`) that it holds nothing there.
#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    pub column: Column,
    pub any: Vec<String>,
    pub nothing: bool,
}

/// What a view held to `only` (for each column named, the things a row
/// is to hold one of) and read for one `band` (of the column it is
/// gathered by: what the band is of, empty for those that name nothing)
/// is held to. A column the table no longer has, and an empty list,
/// hold nothing back.
pub fn held(
    columns: &[Column],
    only: &BTreeMap<String, Vec<String>>,
    band: Option<(&str, &str)>,
) -> Vec<Held> {
    let column = |name: &str| columns.iter().find(|column| column.name == name).cloned();
    let mut held: Vec<Held> = only
        .iter()
        .filter(|(_, any)| !any.is_empty())
        .filter_map(|(name, any)| {
            column(name).map(|column| Held {
                column,
                any: any.iter().map(|one| one.trim().to_string()).collect(),
                nothing: false,
            })
        })
        .collect();
    if let Some((by, key)) = band
        && let Some(column) = column(by)
    {
        let key = key.trim();
        held.push(Held {
            column,
            any: if key.is_empty() { Vec::new() } else { vec![key.to_string()] },
            nothing: key.is_empty(),
        });
    }
    held
}

/// A value as words, as a cell says it: nothing for nothing and for a
/// tick not ticked, `yes` for one ticked, a list's things joined.
pub fn said(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(yes)) => {
            if *yes {
                "yes".into()
            } else {
                String::new()
            }
        }
        Some(Value::Number(number)) => as_words(number),
        Some(Value::Array(items)) => items.iter().map(|item| said(Some(item))).collect::<Vec<_>>().join(", "),
        Some(other) => other.to_string(),
    }
}

/// The things a row holds in a column: each of what a column of several
/// holds, what one of one says; each once, none empty.
pub fn holds(column: &Column, value: Option<&Value>) -> Vec<String> {
    let all: Vec<String> = match value {
        Some(Value::Array(items)) if column.several() => items.iter().map(|item| said(Some(item))).collect(),
        _ => vec![said(value)],
    };
    let mut found: Vec<String> = Vec::new();
    for one in all.into_iter().map(|one| one.trim().to_string()).filter(|one| !one.is_empty()) {
        if !found.contains(&one) {
            found.push(one);
        }
    }
    found
}

/// Whether a row holding `data` is shown by a view held to `held`.
pub fn shows(held: &[Held], data: &Map<String, Value>) -> bool {
    held.iter().all(|one| {
        let holds = holds(&one.column, data.get(&one.column.name));
        (one.nothing && holds.is_empty()) || holds.iter().any(|thing| one.any.contains(thing))
    })
}

/// What a view held to `held` checks of each row, as the database says
/// it: true where the row is shown.
pub fn shows_sql(held: &[Held]) -> String {
    if held.is_empty() {
        return "1".into();
    }
    let each: Vec<String> = held
        .iter()
        .map(|one| {
            let at = path(&one.column.name);
            let any = one.any.iter().map(|thing| literal(thing)).collect::<Vec<_>>().join(", ");
            if one.column.several() {
                let item = "trim(coalesce(CAST(value AS TEXT), ''))";
                let mut ways = Vec::new();
                if !one.any.is_empty() {
                    ways.push(format!(
                        "EXISTS (SELECT 1 FROM json_each(data, {at}) WHERE {item} IN ({any}))"
                    ));
                }
                if one.nothing {
                    ways.push(format!("NOT EXISTS (SELECT 1 FROM json_each(data, {at}) WHERE {item} != '')"));
                }
                format!("({})", if ways.is_empty() { "0".into() } else { ways.join(" OR ") })
            } else {
                let words = format!("trim(coalesce(json_extract(data, {at}), ''))");
                let mut ways = Vec::new();
                if !one.any.is_empty() {
                    ways.push(format!("{words} IN ({any})"));
                }
                if one.nothing {
                    ways.push(format!("{words} = ''"));
                }
                format!("({})", if ways.is_empty() { "0".into() } else { ways.join(" OR ") })
            }
        })
        .collect();
    each.join(" AND ")
}

/// A mark of what a reading of a view goes by: the view, what it is held
/// to, how it is sorted and what is searched for. A cursor of one reading is no cursor of
/// another.
pub fn mark(view: &str, held: &[Held], sort: Option<&Sort>, words: &[String]) -> String {
    let mut said = json!({
        "view": view,
        "held": held.iter().map(|one| json!([one.column.name, one.any, one.nothing])).collect::<Vec<_>>(),
        "sort": sort.map(|sort| json!([sort.field, sort.down])),
    });
    // What is searched for, said only where something is: a mark of a view searched for nothing is what it was.
    if !words.is_empty() {
        said["find"] = json!(words);
    }
    // FNV-1a: a mark the same on every machine and for every program.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in said.to_string().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// A place handed out as text, to be handed back unread: it says the
/// reading it is of (`mark`) and where it stands in it.
pub fn cursor(mark: &str, place: &Place) -> String {
    let said = json!([mark, place.stretch, place.key.json(), place.number]).to_string();
    said.bytes().map(|byte| format!("{byte:02x}")).collect()
}

/// A place handed back, of the reading marked `mark`.
pub fn place_of(text: &str, mark: &str) -> Result<Place> {
    let bytes: Option<Vec<u8>> = (0..text.len())
        .step_by(2)
        .map(|at| text.get(at..at + 2).and_then(|two| u8::from_str_radix(two, 16).ok()))
        .collect();
    let said: Option<Value> = bytes.and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let read = said.as_ref().and_then(Value::as_array).and_then(|said| {
        let place = Place {
            stretch: said.get(1)?.as_u64()? as usize,
            key: Key::from_json(said.get(2)?)?,
            number: said.get(3)?.as_i64()?,
        };
        Some((said.first()?.as_str()?.to_string(), place))
    });
    match read {
        None => Err(Error::invalid(
            "that is not a place in a table's rows: hand back an `after` or a `before` as it was given",
        )),
        Some((was, _)) if was != mark => Err(Error::conflict(
            "the view was changed, or is sorted another way, since this was read: read it from the start",
        )),
        Some((_, place)) => Ok(place),
    }
}

/// How a window is read: by walking an index in the order asked, or in a
/// statement that sorts what it reads, which is so only of a small table
/// or of the rows a view is held to by an index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Way {
    Walk,
    Sorted,
}

/// The rows a view is held to by an indexed column, read by its index.
#[derive(Clone, Debug, PartialEq)]
pub struct Gather {
    pub column: Column,
    /// What the rows hold there, as the index keys it.
    pub keys: Vec<Key>,
}

/// How a window of a view is read.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub order: Order,
    pub way: Way,
    /// The columns to index before it is read, which are not yet.
    pub make: Vec<String>,
    /// Where it may be read by gathering, which is so where they come to
    /// no more than [`GATHER`] rows: whoever reads counts first.
    pub gather: Option<Gather>,
}

/// What a window of a view is asked of.
#[derive(Clone, Debug)]
pub struct Asked<'a> {
    pub columns: &'a [Column],
    pub held: &'a [Held],
    pub sort: Option<&'a Sort>,
    /// How many rows the table has.
    pub count: u64,
    /// The columns of it that are indexed.
    pub indexed: &'a [String],
}

/// How a window is read. A table of [`SMALL`] rows or more is never sorted
/// in a statement: the column it is sorted by, and one it is held to, are
/// indexed first. One that holds several things can be indexed by
/// nothing, and is refused there.
pub fn plan(asked: &Asked<'_>) -> Result<Plan> {
    let order = Order::of(asked.columns, asked.sort)?;
    let big = asked.count >= SMALL;
    let indexed = |name: &str| asked.indexed.iter().any(|one| one == name);
    let mut make = Vec::new();
    let way = match &order.column {
        None => Way::Walk,
        Some(column) if column.how().is_none() => {
            if big {
                return Err(Error::invalid(format!(
                    "`{}` holds several things, which no index keeps in order, and a table of {} rows or more \
                     is not sorted by such a column: hold the view to fewer of its rows, or sort it by another",
                    column.name, SMALL
                )));
            }
            Way::Sorted
        }
        Some(column) if indexed(&column.name) => Way::Walk,
        Some(column) if big => {
            make.push(column.name.clone());
            Way::Walk
        }
        Some(_) => Way::Sorted,
    };
    let gather = asked
        .held
        .iter()
        .filter(|one| !one.nothing && !one.any.is_empty() && one.column.how().is_some())
        .find(|one| indexed(&one.column.name) || big)
        .map(|one| Gather {
            column: one.column.clone(),
            keys: one.any.iter().map(|thing| key(&one.column, Some(&Value::String(thing.clone())))).collect(),
        });
    if let Some(gather) = &gather
        && !indexed(&gather.column.name)
        && !make.contains(&gather.column.name)
    {
        make.push(gather.column.name.clone());
    }
    Ok(Plan { order, way, make, gather })
}
