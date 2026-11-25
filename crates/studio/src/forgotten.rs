//! A project's tables taken away with it, once it is deleted for good.
//!
//! A table is declared by a blueprint and has no way of its own to go:
//! nothing deletes one. So a project deleted for good left every table it
//! had, with its records and their histories, and a project made again
//! under its name found them there, declared again over rows that were
//! somebody else's work. What goes is what is the project's of the tables
//! that tell rows apart by project ([`crate::apart::KEPT_APART`], read by
//! each table's line of [`crate::apart::TABLES`] and never worked out):
//! each line of history, each record, each table, what somebody asked to
//! change of one, and the index each of its columns was kept in order by.
//! It is one write in each journal that may keep them, with what is said
//! of each table that went (`collection.forgotten`), so a stop between
//! leaves a journal with all of a project's rows or none. What is
//! attached to its records, and what of them is in the Trash, is taken
//! away by whoever asks this, before it (`trash::Trash::project_gone`).
//!
//! A table that keeps its records as files of the project, or at a store
//! service, has those left where they are: they are the project's files
//! or the service's, and only what the factory kept of the table goes.

use journal::apart::{Told, told};
use kernel::window::statements::NAMED;
use kernel::{Result, Source};
use serde_json::json;

use crate::collections::{Collection, Collections};

/// What is said of a table that went with its project.
pub const FORGOTTEN: &str = "collection.forgotten";

/// The tables a project's rows are taken out of, in the order they are:
/// a line of history is told apart by its record and the tables there
/// are, so it goes before either. What is attached to a record goes with
/// its files, before this.
const TAKEN: &[&str] = &["record_history", "asked", "records", "collections"];

impl Collections {
    /// Takes away every table of `project`, with its records and what was
    /// said of each, saying of each table that it went. Gives back how many
    /// tables went. For a project deleted for good, and nothing else: a
    /// project of its name that is there now is another, and whoever asks
    /// looks first.
    pub async fn forget_project(&self, project: &str) -> Result<usize> {
        let tables = self.list(Some(project)).await?;
        let mut journals = vec![self.rows(project)];
        if journals[0].name() != self.factory().name() {
            journals.push(self.factory().clone());
        }
        for (at, journal) in journals.into_iter().enumerate() {
            // Each table is said once, in the write of the journal it was read from.
            let gone = if at == 0 { tables.clone() } else { Vec::new() };
            let project = project.to_string();
            let written = journal.write(move |writing| {
                let within = writing.transaction();
                // The indexes kept for its columns, by their names: a project
                // made again under its name makes its own as it is read.
                let (from, to) = (format!("{NAMED}{project}/"), format!("{NAMED}{project}0"));
                let indexes: Vec<String> = within
                    .prepare(
                        "SELECT name FROM sqlite_master WHERE type = 'index' AND name >= ?1 AND name < ?2",
                    )?
                    .query_map([&from, &to], |row| row.get(0))?
                    .collect::<rusqlite::Result<_>>()?;
                for index in indexes {
                    within.execute_batch(&format!(
                        "DROP INDEX IF EXISTS \"{}\";",
                        index.replace('"', "\"\"")
                    ))?;
                }
                for table in TAKEN {
                    let Some(whose) = told(crate::apart::TABLES, table).filter(|told| *told != Told::Factory)
                    else {
                        continue;
                    };
                    let meant = format!("DELETE FROM {table} AS kept WHERE {} IS ?1", whose.reads());
                    within.execute(&meant, [&project])?;
                }
                for table in gone.iter().filter(|table| table.project == project) {
                    writing.say("collection.forgotten", source(table), said(table))?;
                }
                Ok(())
            });
            written.await?;
        }
        for table in &tables {
            self.indexes.forget(project, &table.name, None);
            // Nothing is counted of a table that is not there.
            self.counted.writing(project, &table.name).moved(None);
        }
        Ok(tables.len())
    }
}

/// Who may hear that `table` went: whoever may read its records.
fn source(table: &Collection) -> Source {
    Source::actor("system").taking(table.reads.as_deref())
}

fn said(table: &Collection) -> serde_json::Value {
    json!({ "project": table.project, "name": table.name, "store": table.store.as_str() })
}
