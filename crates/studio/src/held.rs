//! What a table's rows hold of one field, asked where a declaration or a
//! change of a column needs it, and only then.
//!
//! A table declared again is declared at each apply of its blueprint, and
//! a table may hold a million rows. So what its rows hold is asked only of
//! a field that is to be something else than it was: whether a row still
//! chooses an option going away, whether every row fits a field of another
//! kind, whether one has nothing where something is newly needed, which
//! hold a number a choice now takes as a word. A declaration that changes
//! none of that asks the rows nothing, and a table kept in this database
//! is asked by a statement over its rows and never read whole; one kept as
//! files, by a store service or as documents is read as it always was,
//! since nothing else can be asked of it.

use std::sync::LazyLock;

use kernel::Result;
use rusqlite::params;
use serde_json::Value;

use crate::collections::{Collection, Collections, Store};
use crate::fields::Field;

/// Where a field's value is in a record's `data`, as the journal's own
/// functions are told it; none for a name that cannot be said so, whose
/// rows are then read whole.
fn cell(field: &str) -> Option<String> {
    (!field.contains('"')).then(|| format!("$.\"{field}\""))
}

/// Every letter `str::trim` takes off, which is what `fields::nothing`
/// counts as no words: the database's `trim` is told them, so that the
/// two say the same of a cell.
static BLANK: LazyLock<String> =
    LazyLock::new(|| (0..=0x10FFFF_u32).filter_map(char::from_u32).filter(|c| c.is_whitespace()).collect());

/// The fields of `now` a number is to be read as a word of, because they
/// are choices that say what nothing stands for and were declared before
/// as something else, or with other words, or are still being reworded
/// where a stop left them (`crate::reworded`). A field declared for the
/// first time held nothing a record kept.
pub(crate) fn rewordable(was: &[Field], now: &[Field]) -> Vec<String> {
    let changed = |field: &&Field| {
        was.iter().find(|old| old.name == field.name).is_some_and(|old| {
            old.rewording || old.kind != field.kind || old.of != field.of || old.unsaid != field.unsaid
        })
    };
    now.iter()
        .filter(|field| field.kind == "choice" && !field.unsaid.is_empty())
        .filter(changed)
        .map(|field| field.name.clone())
        .collect()
}

impl Collections {
    /// How many records of `table` hold `option` as their `field`.
    pub(crate) async fn chose(&self, table: &Collection, field: &str, option: &str) -> Result<usize> {
        let Some(cell) = cell(field).filter(|_| table.store == Store::Local) else {
            let chose =
                |record: &&crate::Record| record.data.get(field).and_then(Value::as_str) == Some(option);
            return Ok(self.records(table).await?.iter().filter(chose).count());
        };
        let (project, name, option) = (table.project.clone(), table.name.clone(), option.to_string());
        let asked = "SELECT count(*) FROM records WHERE project = ?1 AND collection = ?2
                     AND json_type(data, ?3) = 'text' AND json_extract(data, ?3) = ?4";
        let counted: i64 = self
            .rows(&table.project)
            .db()
            .call(move |connection| {
                connection.query_row(asked, params![project, name, cell, option], |row| row.get(0))
            })
            .await?;
        Ok(counted as usize)
    }

    /// How many records of `table` hold something that `now`, a field of
    /// it as it is to be, does not take (`fields::misfits`). What each row
    /// holds is read here only where it holds something.
    pub(crate) async fn misfit(&self, table: &Collection, now: &Field) -> Result<usize> {
        let (project, name) = (table.project.clone(), table.name.clone());
        let Some(cell) = cell(&now.name).filter(|_| table.store == Store::Local) else {
            let held = |record: &crate::Record| record.data.get(&now.name).cloned().unwrap_or(Value::Null);
            let records = self.records(table).await?;
            return Ok(crate::fields::misfits(&project, &name, now, records.iter().map(held)));
        };
        let (asked_project, asked_name) = (project.clone(), name.clone());
        let held: Vec<String> = self
            .rows(&table.project)
            .db()
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT data -> ?3 FROM records WHERE project = ?1 AND collection = ?2
                     AND json_type(data, ?3) != 'null'",
                )?;
                let rows = statement.query_map(params![asked_project, asked_name, cell], |row| row.get(0))?;
                rows.collect()
            })
            .await?;
        let values = held.iter().map(|text| serde_json::from_str(text).unwrap_or(Value::Null));
        Ok(crate::fields::misfits(&project, &name, now, values))
    }

    /// How many records of `table` have nothing of `field`: nothing at
    /// all, or words with nothing in them (`fields::nothing`).
    pub(crate) async fn lacking(&self, table: &Collection, field: &str) -> Result<usize> {
        let Some(cell) = cell(field).filter(|_| table.store == Store::Local) else {
            let held = |record: &crate::Record| record.data.get(field).cloned().unwrap_or(Value::Null);
            let records = self.records(table).await?;
            return Ok(records.iter().filter(|record| crate::fields::nothing(&held(record))).count());
        };
        let (project, name, blank) = (table.project.clone(), table.name.clone(), BLANK.clone());
        let asked = "SELECT count(*) FROM records WHERE project = ?1 AND collection = ?2
                     AND (json_type(data, ?3) IS NULL OR json_type(data, ?3) = 'null'
                          OR (json_type(data, ?3) = 'text' AND trim(json_extract(data, ?3), ?4) = ''))";
        let counted: i64 = self
            .rows(&table.project)
            .db()
            .call(move |connection| {
                connection.query_row(asked, params![project, name, cell, blank], |row| row.get(0))
            })
            .await?;
        Ok(counted as usize)
    }

    /// The records of `table` that hold a number of any of `fields`, by
    /// their ids and numbers in the order they were made: those whose
    /// numbers a choice may come to take as its words.
    pub(crate) async fn numbered(&self, table: &Collection, fields: &[String]) -> Result<Vec<(String, i64)>> {
        let mut found = Vec::new();
        for field in fields {
            let Some(cell) = cell(field).filter(|_| table.store == Store::Local) else {
                let number = |record: &&crate::Record| record.data.get(field).is_some_and(Value::is_number);
                let records = self.records(table).await?;
                found.extend(records.iter().filter(number).map(|record| (record.id.clone(), record.number)));
                continue;
            };
            let (project, name) = (table.project.clone(), table.name.clone());
            let these: Vec<(String, i64)> = self
                .rows(&table.project)
                .db()
                .call(move |connection| {
                    let mut statement = connection.prepare(
                        "SELECT id, number FROM records WHERE project = ?1 AND collection = ?2
                         AND json_type(data, ?3) IN ('integer', 'real')",
                    )?;
                    let rows = statement
                        .query_map(params![project, name, cell], |row| Ok((row.get(0)?, row.get(1)?)))?;
                    rows.collect()
                })
                .await?;
            found.extend(these);
        }
        found.sort_by_key(|(_, number)| *number);
        found.dedup();
        Ok(found)
    }

    /// Says that nothing a choice of `was` can no longer be is still the
    /// choice of a record: it would be there and nowhere to be seen.
    pub(crate) async fn still_chosen(&self, was: &Collection, now: &[Field]) -> Result<()> {
        for field in was.fields.iter().filter(|field| field.kind == "choice") {
            let now = now.iter().find(|new| new.name == field.name);
            // A field that is no longer a choice takes any word, the ones it
            // could be among them: nothing a record chose is out of reach,
            // and whether each still fits is asked of it as it is refitted.
            if now.is_some_and(|new| new.kind != "choice") {
                continue;
            }
            for gone in field.of.iter().filter(|option| !now.is_some_and(|new| new.of.contains(option))) {
                let left = self.chose(was, &field.name, gone).await?;
                if left > 0 {
                    return Err(kernel::Error::conflict(format!(
                        "`{gone}` is still the `{}` of {left} record(s) of `{}`; change them before taking it away",
                        field.name, was.name
                    )));
                }
            }
        }
        Ok(())
    }

    /// Says that the records `was` already has can still be written once
    /// its fields are declared again as `now`. A record is checked whole
    /// every time it is written, so one that holds what a field no longer
    /// takes, or nothing where something is newly needed, could not be
    /// changed or moved at all afterwards; that is found here, when the
    /// blueprint is applied, and said of the field. A field that is no
    /// longer declared needs nothing said: what records held of it is no
    /// longer read. A field that is as it was asks the rows nothing.
    pub(crate) async fn refit(&self, was: &Collection, now: &[Field]) -> Result<()> {
        let name = &was.name;
        for field in now {
            let before = was.fields.iter().find(|old| old.name == field.name);
            if let Some(before) = before.filter(|before| before.kind != field.kind) {
                let left = self.misfit(was, field).await?;
                if left > 0 {
                    return Err(kernel::Error::conflict(format!(
                        "`{}` was a `{}` and is now declared a `{}`, which is not what {left} record(s) of `{name}` hold of it; change them before declaring it so",
                        field.name, before.kind, field.kind
                    )));
                }
            }
            if field.required && !before.is_some_and(|before| before.required) {
                // One declared for the first time is held by no record: what
                // a row may say under its name is of a field that went.
                let left = match before {
                    Some(_) => self.lacking(was, &field.name).await?,
                    None => self.count(was).await?,
                };
                if left > 0 {
                    return Err(kernel::Error::conflict(format!(
                        "`{}` is now declared as needed, and {left} record(s) of `{name}` have none; declare it without `required`, give each of them one, and then say it is needed",
                        field.name
                    )));
                }
            }
        }
        Ok(())
    }
}
