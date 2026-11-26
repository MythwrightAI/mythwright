//! What a cell took, asked again while the record says it (`stands`).
//!
//! A field that says what giving it a value takes is asked that of
//! whoever gives it the value, once. Where what reads the record acts
//! with more authority than whoever may write it, that is not enough: an
//! agent's row that its owner left with no wall is brought up with no
//! wall whoever then changes what it is told or how many there are of
//! it, and those cells take nothing. So a field may say that what it took
//! stands: while the record says that value, adding the record or
//! changing any cell of it takes it, checked against the record as it is
//! to be written. A write that also sets the cell to a value that takes
//! nothing is whoever's may write the record. A list that says
//! `takes: each` stands by itself, and says nothing more by `stands`.

use std::collections::BTreeMap;

use kernel::{Error, Principal, Result};
use serde_json::{Map, Value};

use crate::collections::Collection;
use crate::fields::Field;
use crate::takes::{Takes, listed};

/// Reads `stands` of the field `name`, said or not: refused of a field
/// that says nothing it takes, which it would guard with nothing.
pub(crate) fn read(
    name: &str,
    said: bool,
    takes_each: bool,
    takes: &Takes,
    takes_of: &BTreeMap<String, Takes>,
) -> Result<bool> {
    if said && !takes_each && takes.is_empty() && takes_of.is_empty() {
        return Err(Error::invalid(format!(
            "the field `{name}` says `stands`, which is for a field that says what giving it a value takes, so \
             that what it took is asked of whoever changes the record while it says so: say that too, as in \
             `takes: {{ off: shell }}`, or leave `stands` out"
        )));
    }
    Ok(said && !takes_each)
}

/// Whether a cell says nothing: no value, words with nothing in them, or
/// an empty list.
fn nothing(value: &Value) -> bool {
    crate::fields::nothing(value) || value.as_array().is_some_and(Vec::is_empty)
}

impl Field {
    /// The scopes what this cell says takes while it says it, where it
    /// stands: those of any value where it says anything, and those of
    /// the value it says. None where it does not stand or says nothing.
    pub fn standing(&self, value: Option<&Value>) -> Vec<&str> {
        let Some(value) = value.filter(|value| self.stands && !nothing(value)) else { return Vec::new() };
        let written = blueprint::expr::display(value);
        let mut scopes: Vec<&str> = self.takes.scopes().iter().map(String::as_str).collect();
        for scope in self.takes_of.get(&written).map(Takes::scopes).unwrap_or_default() {
            if !scopes.contains(&scope.as_str()) {
                scopes.push(scope);
            }
        }
        scopes
    }
}

/// Refuses unless `by` holds what each standing cell of `record`, as it
/// is to be written, took.
pub(crate) fn held(collection: &Collection, record: &Map<String, Value>, by: &Principal) -> Result<()> {
    for field in collection.fields.iter().filter(|field| field.stands) {
        let value = record.get(&field.name);
        let lacking: Vec<&str> = field.standing(value).into_iter().filter(|scope| !by.can(scope)).collect();
        if lacking.is_empty() {
            continue;
        }
        let written = value.map(blueprint::expr::display).unwrap_or_default();
        let says = if field.takes_of.contains_key(&written) {
            format!("says `{written}`")
        } else {
            "says anything".to_string()
        };
        return Err(Error::denied(format!(
            "a record of `{}` whose `{}` {says} is changed only by whoever holds {}, which {} does not: what \
             that cell takes stands while it says so",
            collection.name,
            field.name,
            listed(lacking),
            by.name
        )));
    }
    Ok(())
}

/// Refuses a field that stood being declared again as `now`, so that it
/// does not, by one who does not hold each scope it took: whoever holds
/// those could change every record anyway, and whoever does not would
/// then have what the cell let act for them.
pub(crate) fn still_standing(table: &str, was: &Field, now: Option<&Field>, by: &Principal) -> Result<()> {
    if !was.stands || now.is_some_and(|now| now.stands) {
        return Ok(());
    }
    let mut took: Vec<&str> = was.takes.scopes().iter().map(String::as_str).collect();
    for scope in was.takes_of.values().flat_map(Takes::scopes) {
        if !took.contains(&scope.as_str()) {
            took.push(scope);
        }
    }
    let lacking: Vec<&str> = took.iter().copied().filter(|scope| !by.can(scope)).collect();
    if lacking.is_empty() {
        return Ok(());
    }
    Err(Error::denied(format!(
        "what `{}` of `{table}` takes stands while a record says it, and declaring `{table}` again so that it \
         does not takes holding {}, which {} does not hold",
        was.name,
        listed(lacking),
        by.name
    )))
}
