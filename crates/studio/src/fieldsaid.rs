//! What is said of a record carries the fields its table says to carry.
//!
//! An event about a record says which record it is (`id`, `project`,
//! `collection`) and what was done to it, put together in Rust from a
//! list that does not change. A field a blueprint adds to its table is
//! not on that list, so a handler that is to be chosen by it (`for:`, a
//! plain `when`) had to read the record first, and whatever followed the
//! events from outside had to ask. A field that says it is `said` with
//! them is in every event about a record of its table, by its name under one key of
//! the event's data ([`KEY`]), so that nothing a blueprint adds can come
//! to mean something the event already says. It is put there in one
//! place every such event goes through ([`Say::carrying`], asked where a
//! record is written, taken away or given back, and of what is said on
//! one), with what the record holds as the event is said.
//!
//! What hears it is what hears the event: an event of a table kept
//! private takes what reading it takes, and one that carries anything
//! is made to take it too
//! ([`kernel::Source::taking`]). A field kept for itself (`quiet`) and
//! one that says what giving it a value takes are refused the word where
//! the table is declared, and passed over here as well. A text longer
//! than [`MOST`] is cut there, and anything else that long is left out:
//! an event is no copy of a document. A table that says it of no field
//! says nothing more than it always did.

use kernel::Source;
use serde_json::{Map, Value};

use crate::collections::{Collection, Say};
use crate::fields::Field;

/// What an event carries its fields under.
pub const KEY: &str = "fields";

/// The most of a field an event carries, in bytes of what it is written
/// as: a text is cut to it, and anything else longer is left out.
pub const MOST: usize = 1024;

/// What is put at the end of a text that was cut.
const CUT: &str = "…";

/// Whether an event carries this field.
fn carries(field: &Field) -> bool {
    field.said && !field.quiet && field.takes.is_empty() && field.takes_of.is_empty() && !field.takes_each
}

/// What `holds` comes to as an event carries it, by each field the table
/// says is carried; nothing where it says none.
pub fn of(collection: &Collection, holds: &Map<String, Value>) -> Option<Map<String, Value>> {
    if !collection.fields.iter().any(carries) {
        return None;
    }
    let mut carried = Map::new();
    for field in collection.fields.iter().filter(|field| carries(field)) {
        let value = holds.get(&field.name).cloned().unwrap_or(Value::Null);
        if let Some(value) = bounded(value) {
            carried.insert(field.name.clone(), value);
        }
    }
    Some(carried)
}

/// A value as it is carried: a text cut to [`MOST`], and anything else
/// longer than that left out.
fn bounded(value: Value) -> Option<Value> {
    match value {
        Value::String(text) if text.len() > MOST => {
            let mut end = MOST - CUT.len();
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            Some(Value::String(format!("{}{CUT}", &text[..end])))
        }
        Value::String(_) | Value::Null | Value::Bool(_) | Value::Number(_) => Some(value),
        other => (other.to_string().len() <= MOST).then_some(other),
    }
}

/// What an event about a record of `collection`, said by `source` with
/// `data`, comes to once it carries what `holds` says of the fields the
/// table carries: as it was where it carries none.
pub fn onto(
    collection: &Collection,
    holds: &Map<String, Value>,
    (source, mut data): (Source, Value),
) -> (Source, Value) {
    let Some(carried) = of(collection, holds) else { return (source, data) };
    if let Some(said) = data.as_object_mut() {
        said.insert(KEY.into(), Value::Object(carried));
    }
    // What every event of a table kept private takes already; said again
    // here, so that one put together without it carries nothing to anybody
    // the table is kept from.
    (source.taking(collection.reads.as_deref()), data)
}

impl Say {
    /// The same, said with the fields `collection` carries as `holds`
    /// has them ([`onto`]).
    pub fn carrying(self, collection: &Collection, holds: &Map<String, Value>) -> Say {
        let Say { topic, source, data } = self;
        let (source, data) = onto(collection, holds, (source, data));
        Say { topic, source, data }
    }
}
