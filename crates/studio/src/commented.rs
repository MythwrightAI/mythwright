//! How many things were said on each record of a table.
//!
//! A list of records shows which of them something was said on, and how
//! often, and is not to ask for each record's history to know: this is
//! one question of the histories, answered for the table. Nothing is kept
//! for it. What was said on a record is a line of its history
//! ([`crate::records::COMMENTED`]), and a line is kept by its record, so
//! the lines of one table's records lie together: its records are known
//! by ids that begin the same way, and only those lines are read, however
//! much else the project's histories hold.

use std::collections::BTreeMap;

use kernel::Result;
use rusqlite::params;

use crate::collections::{Collection, Collections, Store, record_id};
use crate::records::COMMENTED;

/// How the id of every record of `collection` begins. Others may begin
/// so too (a board's cards are `shop-4`, and `shop-notes-4` is a note):
/// whose a line is, is asked of the collection.
fn begins(collection: &Collection) -> String {
    match collection.store {
        Store::Documents | Store::Mockups => format!("{}-{}:", collection.project, collection.name),
        _ => {
            let first = record_id(&collection.project, &collection.name, 1);
            first[..first.len() - 1].to_string()
        }
    }
}

impl Collections {
    /// How many things were said on each record of `collection`, by the
    /// record's id: only those something was said on, and one that is
    /// gone with them, since its history is kept. Whoever asks has said
    /// who may read the table.
    pub async fn commented(&self, collection: &Collection) -> Result<BTreeMap<String, u64>> {
        let begins = begins(collection);
        // Above every id that begins so: the last character there is.
        let end = format!("{begins}{}", char::MAX);
        let counted: Vec<(String, u64)> = self
            .rows(&collection.project)
            .db()
            .call(move |connection| {
                connection
                    .prepare(
                        "SELECT record, COUNT(*) FROM record_history
                         WHERE record >= ?1 AND record < ?2 AND topic = ?3 GROUP BY record",
                    )?
                    .query_map(params![begins, end, COMMENTED], |row| {
                        Ok((row.get(0)?, row.get::<_, i64>(1)? as u64))
                    })?
                    .collect()
            })
            .await?;
        Ok(counted.into_iter().filter(|(record, _)| collection.owns(record)).collect())
    }
}
