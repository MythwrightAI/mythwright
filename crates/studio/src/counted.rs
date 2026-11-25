//! How many records a table kept in this database has, kept by the daemon.
//!
//! Counted by a statement it walks every row of the table's index to
//! answer, which for a table of a million rows is a million steps; and it
//! is asked each time the table is declared again. So it is counted once,
//! the first time it is asked after the daemon starts, and from then moved
//! by the daemon's own writes that add a row or take one away (`whole`,
//! `trash`), each by what that write found and did in its own transaction.
//! It is kept in memory and nowhere else: a daemon started again, a copy
//! of a factory and a project moved to a journal of its own (which is done
//! with the factory stopped) each count it again from the rows.
//!
//! A count is kept only where nothing that could move it was under way
//! between asking for it and having it: a write that had begun and not
//! ended, or one that ended meanwhile, leaves it to be counted again next
//! time. A write that ends with nothing known of what it did (it failed,
//! or whoever waited for it went away) forgets what was kept.
//!
//! What is kept is this `Collections`' own: rows written by anything else
//! than it (a test's statement, a second studio opened over the same
//! journal) are not known to it until the daemon starts again.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use kernel::Result;
use rusqlite::params;

use crate::collections::{Collection, Collections, Store};

type Key = (String, String);

#[derive(Default)]
struct Kept {
    count: Option<i64>,
    /// Writes of the table begun and not yet ended.
    writing: u32,
    /// Moved each time a write begins or ends.
    turns: u64,
}

#[derive(Clone, Default)]
pub(crate) struct Counted(Arc<Mutex<HashMap<Key, Kept>>>);

impl Counted {
    fn with<T>(&self, key: &Key, then: impl FnOnce(&mut Kept) -> T) -> T {
        let mut kept = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        then(kept.entry(key.clone()).or_default())
    }

    /// A write of `project`'s table `table` that may add a row or take
    /// one away, begun: said to have ended by [`Writing::moved`].
    pub(crate) fn writing(&self, project: &str, table: &str) -> Writing {
        let key = (project.to_string(), table.to_string());
        self.with(&key, |kept| {
            kept.writing += 1;
            kept.turns += 1;
        });
        Writing { counted: self.clone(), key, ended: false }
    }

    /// A mark of how far the table's rows have been written: it moves
    /// with each write of them the daemon makes. Nothing while one is
    /// under way, since what is read meanwhile may be of either side of
    /// it. What is worked out of the rows is kept by it (`window`).
    pub(crate) fn mark(&self, project: &str, table: &str) -> Option<u64> {
        let key = (project.to_string(), table.to_string());
        self.with(&key, |kept| (kept.writing == 0).then_some(kept.turns))
    }
}

/// A write under way, as [`Counted`] is told of it.
pub(crate) struct Writing {
    counted: Counted,
    key: Key,
    ended: bool,
}

impl Writing {
    /// The write ended, and moved the table's count by `by` rows; or, where
    /// it failed, by nothing anybody knows.
    pub(crate) fn moved(mut self, by: Option<i64>) {
        self.ended = true;
        self.end(by);
    }

    fn end(&self, by: Option<i64>) {
        self.counted.with(&self.key, |kept| {
            kept.writing -= 1;
            kept.turns += 1;
            kept.count = match (kept.count, by) {
                (Some(count), Some(by)) => Some(count + by),
                _ => None,
            };
        });
    }
}

impl Drop for Writing {
    fn drop(&mut self) {
        if !self.ended {
            self.end(None);
        }
    }
}

impl Collections {
    /// How many records `table` has: kept, for one kept in this database,
    /// and counted where they are kept for any other.
    pub async fn count(&self, table: &Collection) -> Result<usize> {
        if table.store != Store::Local {
            return Ok(self.records(table).await?.len());
        }
        let key = (table.project.clone(), table.name.clone());
        let asked = self.counted.with(&key, |kept| match kept.count {
            Some(count) => Err(count),
            None => Ok((kept.writing, kept.turns)),
        });
        let (writing, turns) = match asked {
            Err(count) => return Ok(count as usize),
            Ok(asked) => asked,
        };
        let (project, name) = key.clone();
        let counted: i64 = self
            .rows(&table.project)
            .db()
            .call(move |connection| {
                connection.query_row(
                    "SELECT count(*) FROM records WHERE project = ?1 AND collection = ?2",
                    params![project, name],
                    |row| row.get(0),
                )
            })
            .await?;
        self.counted.with(&key, |kept| {
            if writing == 0 && kept.writing == 0 && kept.turns == turns {
                kept.count = Some(counted);
            }
        });
        Ok(counted as usize)
    }
}
