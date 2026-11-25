//! A table called something else for people (`title_chosen`,
//! [`crate::shown`]), for everybody, by whoever may arrange the project
//! (`projects:write`, which the side panel asks of everything there) and
//! may change the table, as a column is called something else: its name
//! is what everything else knows it by and is never changed. A board, a
//! library and mockups keep their words.
//!
//! The row and what is said of it (`collection.titled`) are one write of
//! the journal that keeps the table, and only how it is shown is written
//! there: what else the row says is as whoever wrote it last left it.

use journal::optional;
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde_json::json;

use crate::board::BOARD;
use crate::collections::{Collection, Collections, Store, missing};
use crate::shown::Shown;

impl Collections {
    /// Calls the table `name` of `project` something else for people, or,
    /// with nothing (or nothing but spaces), takes that away and what it
    /// is declared to be called is said again. One that comes to what is
    /// kept is no write and says nothing.
    pub async fn title(
        &self,
        project: &str,
        name: &str,
        title: Option<&str>,
        by: &Principal,
    ) -> Result<Collection> {
        by.require("projects:write")?;
        let table = self.looked_at(project, name, by).await?;
        if name == BOARD || matches!(table.store, Store::Documents | Store::Mockups) {
            return Err(Error::invalid(format!(
                "`{name}` is a board, a library or mockups, which are called what they are: only a table of plain \
                 records is called something else"
            )));
        }
        let chosen = title.map(crate::shown::titled).transpose()?.flatten();
        let now = self.factory().now_ms();
        let (of_project, called, actor) = (project.to_string(), name.to_string(), by.actor());
        let reads = table.reads.clone();
        let (written, _) = self
            .rows(project)
            .write(move |writing| {
                let transaction = writing.transaction();
                let kept: Option<Option<String>> = optional(transaction.query_row(
                    "SELECT beside FROM collections WHERE project = ?1 AND name = ?2",
                    [&of_project, &called],
                    |row| row.get(0),
                ))?;
                let Some(kept) = kept else { return Ok(Err(missing(&of_project, &called))) };
                let mut shown = Shown::read(kept);
                // What is declared, said again, is no choice of anybody's.
                let chosen = chosen.filter(|chosen| Some(chosen) != shown.title.as_ref());
                if shown.title_chosen == chosen {
                    return Ok(Ok(false));
                }
                shown.title_chosen = chosen.clone();
                transaction.execute(
                    "UPDATE collections SET beside = ?3, updated = ?4 WHERE project = ?1 AND name = ?2",
                    params![of_project, called, shown.kept(), now],
                )?;
                let mut said = json!({ "project": of_project, "name": called, "by": actor });
                if let Some(chosen) = chosen.or(shown.title) {
                    said["title"] = json!(chosen);
                }
                // Of a table kept private it is heard by whoever may read the table.
                writing.say(
                    "collection.titled",
                    Source::actor(actor.clone()).taking(reads.as_deref()),
                    said,
                )?;
                Ok(Ok(true))
            })
            .await?;
        written?;
        self.get(project, name).await
    }
}
