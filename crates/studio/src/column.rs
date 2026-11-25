//! The records of one column of a table, those that say they wait on
//! others, and where the last of a table is, each read by what it says
//! and not by reading the table whole.
//!
//! A card's place in its column was found by reading its board whole,
//! as it was added and at each move, and so were what a stage finds in
//! its column and what waits on a card that is finished: what one card
//! cost the board's own reads grew with every card beside it. Of a table
//! kept in the journal's database they are asked of an index each
//! (`records_column`, `records_waiting`, the last step of the records'
//! tables); of one kept anywhere else the table is read as it always was,
//! and what is asked is worked out from that.

use journal::optional;
use kernel::Result;
use rusqlite::params;

use crate::collections::{Collection, Collections, RECORD, Record, Store, read_record};

/// What says which column a record is in, as the index has it: the
/// field a board's cards say it in.
const COLUMN: &str = "json_extract(data, '$.status')";

/// What says a record waits on others, word for word what the index
/// holds (its `WHERE`), so that the index is the one asked: what a card
/// says it waits for, where it is other cards, begins with the word the
/// board writes for that ([`crate::cards::waits_on_others`]).
const WAITS: &str =
    "json_extract(data, '$.waiting') >= 'after ' AND json_extract(data, '$.waiting') < 'after!'";

/// Whether two texts are the same, where only a constant can ask.
const fn same(one: &str, other: &str) -> bool {
    let (one, other) = (one.as_bytes(), other.as_bytes());
    if one.len() != other.len() {
        return false;
    }
    let mut at = 0;
    while at < one.len() {
        if one[at] != other[at] {
            return false;
        }
        at += 1;
    }
    true
}

// The index is of the board's own field and of the word it writes: one
// changed without the other would leave the index asked of nothing.
const _: () = assert!(same(crate::board::COLUMN, "status"));
const _: () = assert!(same(crate::cards::AFTER, "after "));

impl Collections {
    /// The records of `column`, in their order: by position, and of two
    /// at one place the one made first first.
    pub(crate) async fn in_column(&self, collection: &Collection, column: &str) -> Result<Vec<Record>> {
        if collection.store != Store::Local {
            let mut found: Vec<Record> =
                self.records(collection).await?.into_iter().filter(|record| said(record, column)).collect();
            found.sort_by(|one, other| one.position.total_cmp(&other.position));
            return Ok(found);
        }
        let (project, name, column) =
            (collection.project.clone(), collection.name.clone(), column.to_string());
        let asked = format!(
            "SELECT {RECORD} FROM records WHERE project = ?1 AND collection = ?2 AND {COLUMN} = ?3
             ORDER BY position, number"
        );
        let found = self
            .rows(&project)
            .db()
            .call(move |connection| {
                let mut statement = connection.prepare(&asked)?;
                let rows = statement.query_map(params![project, name, column], read_record)?;
                rows.collect()
            })
            .await?;
        Ok(declared(collection, found))
    }

    /// The highest position of a record in `column`, leaving `except`
    /// out and, where `below` is said, of those above it alone; none
    /// where there is none.
    pub(crate) async fn highest_in(
        &self,
        collection: &Collection,
        column: &str,
        except: Option<&str>,
        below: Option<f64>,
    ) -> Result<Option<f64>> {
        let except = except.unwrap_or_default().to_string();
        let below = below.unwrap_or(f64::INFINITY);
        if collection.store != Store::Local {
            let records = self.records(collection).await?;
            let placed = records.iter().filter(|record| said(record, column) && record.id != except);
            let above = placed.map(|record| record.position).filter(|position| *position < below);
            return Ok(above.fold(None, |highest: Option<f64>, position| {
                Some(highest.map_or(position, |was| was.max(position)))
            }));
        }
        let (project, name, column) =
            (collection.project.clone(), collection.name.clone(), column.to_string());
        let asked = format!(
            "SELECT position FROM records
             WHERE project = ?1 AND collection = ?2 AND {COLUMN} = ?3 AND position < ?4 AND id != ?5
             ORDER BY position DESC LIMIT 1"
        );
        self.rows(&project)
            .db()
            .call(move |connection| {
                optional(
                    connection
                        .query_row(&asked, params![project, name, column, below, except], |row| row.get(0)),
                )
            })
            .await
    }

    /// The highest position of a record of the table, 0 where it has
    /// none: one row read of an index (`records_placed`), however many
    /// records it has.
    pub async fn last_position(&self, collection: &Collection) -> Result<f64> {
        if collection.store != Store::Local {
            let records = self.records(collection).await?;
            return Ok(records.iter().map(|record| record.position).fold(0.0, f64::max));
        }
        let (project, name) = (collection.project.clone(), collection.name.clone());
        let asked = "SELECT position FROM records WHERE project = ?1 AND collection = ?2
             ORDER BY position DESC LIMIT 1";
        let last = self.rows(&project).db().call(move |connection| {
            optional(connection.query_row(asked, params![project, name], |row| row.get::<_, f64>(0)))
        });
        Ok(last.await?.map_or(0.0, |last| last.max(0.0)))
    }

    /// Whether a record of the table is at `position`: one row looked for.
    pub(crate) async fn placed_at(&self, collection: &Collection, position: f64) -> Result<bool> {
        if collection.store != Store::Local {
            return Ok(self.records(collection).await?.iter().any(|record| record.position == position));
        }
        let (project, name) = (collection.project.clone(), collection.name.clone());
        let asked =
            "SELECT EXISTS (SELECT 1 FROM records WHERE project = ?1 AND collection = ?2 AND position = ?3)";
        let journal = self.rows(&project);
        journal
            .db()
            .call(move |connection| {
                connection.query_row(asked, params![project, name, position], |row| row.get(0))
            })
            .await
    }

    /// The records that say they wait on others, in the order they were
    /// made.
    pub(crate) async fn waiting_on_others(&self, collection: &Collection) -> Result<Vec<Record>> {
        if collection.store != Store::Local {
            let records = self.records(collection).await?;
            let waits = |record: &Record| {
                record
                    .data
                    .get("waiting")
                    .and_then(|said| said.as_str())
                    .is_some_and(crate::cards::waits_on_others)
            };
            return Ok(records.into_iter().filter(waits).collect());
        }
        let (project, name) = (collection.project.clone(), collection.name.clone());
        let asked = format!(
            "SELECT {RECORD} FROM records WHERE project = ?1 AND collection = ?2 AND {WAITS} ORDER BY number"
        );
        let found = self
            .rows(&project)
            .db()
            .call(move |connection| {
                let mut statement = connection.prepare(&asked)?;
                let rows = statement.query_map(params![project, name], read_record)?;
                rows.collect()
            })
            .await?;
        Ok(declared(collection, found))
    }

    /// One record of a collection by its id, without reading the rest:
    /// none where the id is not one of the collection's, or names
    /// nothing.
    pub(crate) async fn record_of(&self, collection: &Collection, id: &str) -> Result<Option<Record>> {
        if !matches!(collection.store, Store::Local | Store::Files) {
            // Known by something else than a number (a document by where
            // its file is), and read whole wherever it is kept.
            return Ok(self.records(collection).await?.into_iter().find(|record| record.id == id));
        }
        let number = id.rsplit_once('-').and_then(|(_, number)| number.parse::<i64>().ok());
        match number {
            Some(number) if collection.owns(id) => self.one(collection, id, number).await,
            _ => Ok(None),
        }
    }
}

/// Whether a record says it is in `column`.
fn said(record: &Record, column: &str) -> bool {
    record.data.get(crate::board::COLUMN).and_then(|said| said.as_str()) == Some(column)
}

/// The records as they are read everywhere: with only what the
/// collection declares now ([`Collections::written_since`]).
fn declared(collection: &Collection, mut records: Vec<Record>) -> Vec<Record> {
    for record in &mut records {
        record.data.retain(|name, _| collection.field(name).is_some());
    }
    records
}
