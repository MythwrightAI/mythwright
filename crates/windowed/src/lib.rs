//! A table's rows read a window at a time from the database, as plain
//! values: a row as the record it is ([`Record`]), what a window is asked
//! and what it comes to as it is handed out, and the reading itself over
//! a connection, each of the three ways `kernel::window` plans (an index
//! walked a stretch at a time, the rows a view is held to gathered by an
//! index, a small table sorted in the statement), with what was counted
//! of views kept by the table's write mark. It came out of `studio`,
//! which plans a window of a table it knows, asks who may read it and
//! hands its rows out (`studio::window`), and which offers what is here
//! under the names it had (`studio::window::Asked`, `studio::Record`).

pub mod files;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};

use kernel::window::figures::Counting;
use kernel::window::read::From;
use kernel::window::statements::{self, Shape};
use kernel::window::{Column, Gather, Key, LOOK, PIECE, Place, Sort, Way, literal, path};
use rusqlite::types::ValueRef;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// A record of a table: one row, whatever it is a row of.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub project: String,
    pub collection: String,
    pub number: i64,
    pub data: Map<String, Value>,
    /// Order among the records it is shown with; smaller is earlier.
    pub position: f64,
    pub created_by: String,
    pub created: i64,
    pub updated: i64,
}

/// A record's id. A board's cards keep the short ids they always had.
pub fn record_id(project: &str, collection: &str, number: i64) -> String {
    if collection == "board" {
        format!("{project}-{number}")
    } else {
        format!("{project}-{collection}-{number}")
    }
}

/// The columns a record is read by, in the order [`read_record`] reads them.
pub const RECORD: &str = "id, project, collection, number, data, position, created_by, created, updated";

/// A record from a row read by [`RECORD`].
pub fn read_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
    Ok(Record {
        id: row.get(0)?,
        project: row.get(1)?,
        collection: row.get(2)?,
        number: row.get(3)?,
        data: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
        position: row.get(5)?,
        created_by: row.get(6)?,
        created: row.get(7)?,
        updated: row.get(8)?,
    })
}

/// Where a window is read from, as it is asked.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Reading {
    #[default]
    Start,
    After(String),
    Before(String),
    At(u64),
    /// About a row, by its id or its number: the rows before it and from it on.
    Around(String),
}

/// What a window of a table is asked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    /// The view it is of, by its name; every row where none is said.
    pub view: Option<String>,
    /// How the reader sorts it; by number where nobody does.
    pub sort: Option<Sort>,
    /// One band of a view gathered by a column: what the band is of.
    pub band: Option<String>,
    /// Words searched for among the columns the view shows.
    pub find: Option<String>,
    pub reading: Reading,
    pub limit: usize,
    /// Which fields each row is handed with; every one where none is said.
    pub fields: Option<Vec<String>>,
}

/// A window, as it is handed out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answer {
    pub rows: Vec<Value>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub looked: u64,
    pub short: bool,
    pub rank: Option<u64>,
    /// How many rows were read where a table is kept outside the
    /// database, where that is more than a window can be read from cheaply.
    pub whole_read: Option<usize>,
    /// The row a window about one was read about, and whether the view shows it.
    pub around: Option<Value>,
}

impl Answer {
    pub fn json(&self) -> Value {
        let mut said = json!({
            "rows": self.rows, "after": self.after, "before": self.before, "looked": self.looked, "short": self.short,
        });
        if let Some(rank) = self.rank {
            said["rank"] = json!(rank);
        }
        if let Some(read) = self.whole_read {
            said["whole_read"] = json!(read);
        }
        if let Some(around) = &self.around {
            said["around"] = around.clone();
        }
        said
    }
}

/// A key as a value of a row, to be said as words.
pub fn key_json(key: &Key) -> Value {
    match key {
        Key::Null => Value::Null,
        Key::Number(number) if number.fract() == 0.0 => json!(*number as i64),
        Key::Number(number) => json!(number),
        Key::Text(text) => json!(text),
    }
}

pub fn key_of(value: ValueRef<'_>) -> Key {
    match value {
        ValueRef::Null => Key::Null,
        ValueRef::Integer(whole) => Key::Number(whole as f64),
        ValueRef::Real(number) => Key::Number(number),
        ValueRef::Text(text) | ValueRef::Blob(text) => Key::Text(String::from_utf8_lossy(text).into_owned()),
    }
}

/// A row read with where it stands.
pub type Found = (Record, Place);

/// What walking found.
pub struct Walked {
    pub rows: Vec<Found>,
    pub looked: u64,
    /// Every stretch was gone through to its end.
    pub ended: bool,
    /// The last entry looked at, where the walk stopped short.
    pub last: Option<Place>,
}

/// Walks the order's stretches from a place, one way, until `need` rows
/// are found, [`LOOK`] entries were looked at, or the order ends.
pub fn walk(
    connection: &rusqlite::Connection,
    shape: &Shape,
    from: Option<(Place, bool)>,
    forward: bool,
    need: usize,
) -> rusqlite::Result<Walked> {
    let count = shape.order.stretches.len();
    let first =
        from.as_ref().map_or(if forward { 0 } else { count - 1 }, |(place, _)| place.stretch.min(count - 1));
    let stretches: Vec<usize> = if forward { (first..count).collect() } else { (0..=first).rev().collect() };
    let mut walked = Walked { rows: Vec::new(), looked: 0, ended: true, last: None };
    for stretch in stretches {
        let start = from
            .as_ref()
            .filter(|(place, _)| place.stretch == stretch)
            .map(|(place, inclusive)| (place, *inclusive));
        let statement = shape.walk(stretch, start, forward != shape.order.down);
        let mut prepared = connection.prepare(&statement)?;
        let mut rows = prepared.query([])?;
        while let Some(row) = rows.next()? {
            walked.looked += 1;
            let key = key_of(row.get_ref(9)?);
            let number: i64 = row.get(3)?;
            let place = Place { stretch, key, number };
            if row.get::<_, Option<bool>>(10)?.unwrap_or(false) {
                walked.rows.push((read_record(row)?, place.clone()));
            }
            if walked.rows.len() >= need {
                walked.ended = false;
                return Ok(walked);
            }
            if walked.looked >= LOOK {
                walked.ended = false;
                walked.last = Some(place);
                return Ok(walked);
            }
        }
    }
    Ok(walked)
}

/// Reads rows sorted in the statement: `take` of them, one more than
/// asked, so that the end is known.
pub fn sorted(connection: &rusqlite::Connection, statement: &str) -> rusqlite::Result<Vec<Found>> {
    let mut prepared = connection.prepare(statement)?;
    let found = prepared.query_map([], |row| {
        let record = read_record(row)?;
        let place = Place {
            stretch: row.get::<_, i64>(10)? as usize,
            key: key_of(row.get_ref(9)?),
            number: record.number,
        };
        Ok((record, place))
    })?;
    found.collect()
}

/// How many things were said on each of `ids`: lines of each one's
/// history said as `topic` (`record.commented`).
pub fn commented(
    connection: &rusqlite::Connection,
    ids: &[String],
    topic: &str,
) -> rusqlite::Result<Vec<u64>> {
    let mut asked =
        connection.prepare("SELECT count(*) FROM record_history WHERE record = ?1 AND topic = ?2")?;
    ids.iter()
        .map(|id| asked.query_row(rusqlite::params![id, topic], |row| row.get::<_, i64>(0)).map(|n| n as u64))
        .collect()
}

/// What a window of a table kept in the database read.
pub struct Read {
    pub rows: Vec<(Found, u64)>,
    pub after: Option<Place>,
    pub before: Option<Place>,
    pub looked: u64,
    pub short: bool,
    pub rank: Option<u64>,
}

/// The gathering a view is read by, where the rows it is held to by an
/// index are few enough to be gathered.
pub fn gathered(
    connection: &rusqlite::Connection,
    shape: &Shape,
    gather: Option<Gather>,
) -> rusqlite::Result<Option<Gather>> {
    let Some(gather) = gather else { return Ok(None) };
    let many: i64 = connection.query_row(&shape.gathered(&gather), [], |row| row.get(0))?;
    Ok((many as u64 <= kernel::window::GATHER).then_some(gather))
}

/// Walks to a window, from where it is read.
pub fn walked(
    connection: &rusqlite::Connection,
    shape: &Shape,
    from: &From,
    limit: usize,
) -> rusqlite::Result<Read> {
    let (start, forward, rank) = match from {
        From::Start => (None, true, None),
        From::After(place) => (Some((place.clone(), false)), true, None),
        From::Before(place) => (Some((place.clone(), false)), false, None),
        From::Of(place) => (Some((place.clone(), true)), true, None),
        From::At(rank) => match place_at(connection, shape, *rank)? {
            Some(place) => (Some((place, true)), true, Some(*rank)),
            None => {
                return Ok(Read {
                    rows: Vec::new(),
                    after: None,
                    before: None,
                    looked: 0,
                    short: false,
                    rank: Some(*rank),
                });
            }
        },
    };
    let read_from = start.clone();
    let walked = walk(connection, shape, start, forward, limit)?;
    let short = walked.last.is_some();
    let mut rows: Vec<(Found, u64)> = walked.rows.into_iter().map(|found| (found, 0)).collect();
    let (after, before);
    if forward {
        let last = rows.last().map(|((_, place), _)| place.clone());
        after = if walked.ended { None } else { walked.last.or(last) };
        before = match (&read_from, rows.first()) {
            (None, _) | (_, None) => None,
            (Some(_), Some(((_, place), _))) => Some(place.clone()),
        };
    } else {
        rows.reverse();
        let first = rows.first().map(|((_, place), _)| place.clone());
        before = if walked.ended { None } else { walked.last.or(first) };
        after = rows.last().map(|((_, place), _)| place.clone());
    }
    Ok(Read { rows, after, before, looked: walked.looked, short, rank })
}

/// Where the row at `rank` of the walk stands, a stretch at a time.
pub fn place_at(
    connection: &rusqlite::Connection,
    shape: &Shape,
    rank: u64,
) -> rusqlite::Result<Option<Place>> {
    let mut left = rank;
    for stretch in 0..shape.order.stretches.len() {
        let walk = shape.walk(stretch, None, !shape.order.down);
        let first = format!("SELECT k, number FROM ({walk}) WHERE ok LIMIT 1 OFFSET {left}");
        let found =
            connection.query_row(&first, [], |row| Ok((key_of(row.get_ref(0)?), row.get::<_, i64>(1)?)));
        match journal::optional(found)? {
            Some((key, number)) => return Ok(Some(Place { stretch, key, number })),
            None => {
                let many: i64 =
                    connection.query_row(&format!("SELECT count(*) FROM ({walk}) WHERE ok"), [], |row| {
                        row.get(0)
                    })?;
                left = left.saturating_sub(many as u64);
            }
        }
    }
    Ok(None)
}

/// A window sorted in the statement.
pub fn in_statement(
    connection: &rusqlite::Connection,
    shape: &Shape,
    gather: Option<&Gather>,
    from: &From,
    limit: usize,
) -> rusqlite::Result<Read> {
    let (start, forward, skip, rank) = match from {
        From::Start => (None, true, 0, None),
        From::After(place) => (Some((place, false)), true, 0, None),
        From::Before(place) => (Some((place, false)), false, 0, None),
        From::Of(place) => (Some((place, true)), true, 0, None),
        From::At(rank) => (None, true, *rank, Some(*rank)),
    };
    let statement = shape.sorted(gather, start, forward, limit + 1, skip);
    let mut found = sorted(connection, &statement)?;
    let more = found.len() > limit;
    found.truncate(limit);
    let mut rows: Vec<(Found, u64)> = found.into_iter().map(|found| (found, 0)).collect();
    let place = |row: Option<&(Found, u64)>| row.map(|((_, place), _)| place.clone());
    let (after, before) = if forward {
        let before = if start.is_some() || skip > 0 { place(rows.first()) } else { None };
        (if more { place(rows.last()) } else { None }, before)
    } else {
        rows.reverse();
        (place(rows.last()), if more { place(rows.first()) } else { None })
    };
    Ok(Read { rows, after, before, looked: 0, short: false, rank })
}

/// What was counted of views, by what each was asked and the table's
/// write mark then: a table written since is counted again.
#[derive(Clone, Default)]
pub struct Counts(Arc<Mutex<HashMap<String, Kept>>>);

/// A counting kept: the table's write mark, what it came to, the number
/// it got to and whether it went through every row.
type Kept = (u64, Counting, i64, bool);

/// How many countings are kept at most.
const KEPT: usize = 64;

impl Counts {
    pub fn kept(&self, asked: &str, written: Option<u64>) -> Option<(Counting, i64, bool)> {
        let kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let (mark, counting, from, whole) = kept.get(asked)?;
        (Some(*mark) == written).then(|| (counting.clone(), *from, *whole))
    }

    pub fn keep(&self, asked: String, written: Option<u64>, (counting, from, whole): (Counting, i64, bool)) {
        let Some(written) = written else { return };
        let mut kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if kept.len() >= KEPT && !kept.contains_key(&asked) {
            kept.clear();
        }
        kept.insert(asked, (written, counting, from, whole));
    }
}

/// A window read `from` where said, the way it was planned, each row with
/// how many things were said on it (lines of its history said as `topic`).
pub fn read_window(
    connection: &rusqlite::Connection,
    shape: &Shape,
    way: Way,
    gather: Option<Gather>,
    from: &From,
    limit: usize,
    topic: &str,
) -> rusqlite::Result<Read> {
    let gather = gathered(connection, shape, gather)?;
    let read = match (way, gather) {
        (Way::Walk, None) => walked(connection, shape, from, limit)?,
        (_, gather) => in_statement(connection, shape, gather.as_ref(), from, limit)?,
    };
    let ids: Vec<String> = read.rows.iter().map(|((record, _), _)| record.id.clone()).collect();
    let said = commented(connection, &ids, topic)?;
    Ok(Read {
        rows: read.rows.into_iter().zip(said).map(|((found, _), said)| (found, said)).collect(),
        ..read
    })
}

/// How many rows the view a window is planned for shows before `place`.
pub fn rank_before(
    connection: &rusqlite::Connection,
    shape: &Shape,
    way: Way,
    gather: Option<Gather>,
    place: &Place,
) -> rusqlite::Result<u64> {
    let count = |statement: &str| connection.query_row(statement, [], |row| row.get::<_, i64>(0));
    let gather = gathered(connection, shape, gather)?;
    if way == Way::Sorted || gather.is_some() {
        return Ok(count(&shape.sorted_before(gather.as_ref(), place))? as u64);
    }
    let mut before = count(&shape.before(place.stretch, Some(place)))?;
    for stretch in 0..place.stretch {
        before += count(&shape.before(stretch, None))?;
    }
    Ok(before as u64)
}

/// What is counted of a view read off the indexes: how many rows each band
/// holds, and each figure.
pub type OffIndex = Option<(BTreeMap<String, u64>, Vec<Option<f64>>)>;

/// What is counted of a view held to nothing, read off the indexes of the
/// columns asked about: the rows of each band of `group` (not more bands
/// than are shown), and each figure by the statement `asked` says for it,
/// of `of` rows. Nothing where there are more bands than that.
pub fn off_index(
    connection: &rusqlite::Connection,
    project: &str,
    name: &str,
    group: Option<&Column>,
    asked: &[(String, String)],
    of: u64,
) -> rusqlite::Result<OffIndex> {
    let mut bands = BTreeMap::new();
    if let Some(group) = group {
        let mut last = Key::Null;
        loop {
            let next =
                connection.query_row(&statements::next_value(project, name, group, &last), [], |row| {
                    Ok(key_of(row.get_ref(0)?))
                });
            let Some(next) = journal::optional(next)? else { break };
            if bands.len() > kernel::window::BANDS {
                return Ok(None);
            }
            let many: i64 =
                connection
                    .query_row(&statements::holding(project, name, group, &next), [], |row| row.get(0))?;
            let band = match &next {
                Key::Text(text) => text.trim().to_string(),
                Key::Number(_) | Key::Null => kernel::window::said(Some(&key_json(&next))),
            };
            *bands.entry(band).or_insert(0) += many as u64;
            last = next;
        }
        let none: i64 =
            connection
                .query_row(&statements::holding(project, name, group, &Key::Null), [], |row| row.get(0))?;
        if none > 0 {
            *bands.entry(String::new()).or_insert(0) += none as u64;
        }
    }
    let mut said = Vec::new();
    for (statement, which) in asked {
        let found = connection.query_row(statement, [], |row| Ok(key_of(row.get_ref(0)?)));
        let figure = match (journal::optional(found)?, which.as_str()) {
            (Some(Key::Number(filled)), "empty") => Some(of as f64 - filled),
            (Some(Key::Number(number)), _) => Some(number),
            _ => None,
        };
        said.push(figure);
    }
    Ok(Some((bands, said)))
}

/// One piece of a table's rows after the one numbered `from`, by number,
/// [`PIECE`] of them, with what each holds of `wanted`.
pub fn piece(
    connection: &rusqlite::Connection,
    project: &str,
    table: &str,
    wanted: &[String],
    from: i64,
) -> rusqlite::Result<Vec<(i64, Map<String, Value>)>> {
    let cells: String = wanted.iter().map(|name| format!(", data -> {}", path(name))).collect();
    let statement = format!(
        "SELECT number{cells} FROM records INDEXED BY records_numbered WHERE project = {} AND collection = {} \
         AND number > {from} ORDER BY number LIMIT {PIECE}",
        literal(project),
        literal(table)
    );
    let mut prepared = connection.prepare(&statement)?;
    let rows = prepared.query_map([], |row| {
        let mut data = Map::new();
        for (at, name) in wanted.iter().enumerate() {
            let said: Option<String> = row.get(at + 1)?;
            if let Some(value) = said.and_then(|said| serde_json::from_str(&said).ok()) {
                data.insert(name.clone(), value);
            }
        }
        Ok((row.get(0)?, data))
    })?;
    rows.collect()
}

/// What is known of the indexes there are, held in memory: which of each
/// table's columns are kept in order by one (`studio::indexes`, which
/// makes and takes them away).
#[derive(Clone, Default)]
pub struct Indexes(Arc<Inner>);

#[derive(Default)]
struct Inner {
    known: Mutex<Known>,
    /// Held by whoever makes or takes one away.
    making: tokio::sync::Mutex<()>,
}

/// A column indexed: its name, the statement that made its index, and
/// when it was last asked for, by a count.
pub type Made = (String, String, u64);

#[derive(Default)]
pub struct Known {
    /// The projects whose indexes were read from their file.
    pub read: HashSet<String>,
    /// By project and table: each column indexed, what made its index,
    /// and when it was last asked for, by a count.
    pub made: HashMap<(String, String), Vec<Made>>,
    pub asked: u64,
}

impl Indexes {
    /// Held by whoever makes an index or takes one away.
    pub fn making(&self) -> &tokio::sync::Mutex<()> {
        &self.0.making
    }

    /// `then` with what is known, held while it is asked.
    pub fn known<T>(&self, then: impl FnOnce(&mut Known) -> T) -> T {
        then(&mut self.0.known.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Forgets what is known of the index of `field` of a table, which is
    /// read again from the file where it is next asked.
    pub fn forget(&self, project: &str, table: &str, field: Option<&str>) {
        self.known(|known| {
            known.read.remove(project);
            if let Some(made) = known.made.get_mut(&(project.to_string(), table.to_string())) {
                made.retain(|(one, ..)| field.is_some_and(|field| field != one));
            }
        });
    }
}
