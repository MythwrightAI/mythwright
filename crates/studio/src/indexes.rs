//! The indexes a table's columns are kept in order by, so that a table
//! of many rows is read a window at a time by any column (`window`).
//!
//! One is an index of the one table every record is a row of, for one
//! column of one table, and holds that table's rows alone:
//!
//! ```sql
//! CREATE INDEX "records_by shop/test-runs/title"
//!   ON records (project, collection, lower(nullif(json_extract(data, '$."title"'), '')), number)
//!   WHERE project = 'shop' AND collection = 'test-runs'
//! ```
//!
//! The project and the table are in it though every row it holds says the
//! same of them: a statement says them, as it has to for the index to be
//! the one it reads by, and the database reads every row a statement goes
//! through for what it says of anything the index has not got, which was
//! every row of a count read off it.
//!
//! **It is made by asking, and named for what it is.** What a table is
//! sorted by is not known when its file is made, so no step of the
//! table's makes one: the daemon makes it when a window of a table of
//! [`kernel::window::SMALL`] rows or more is first read by the column, or
//! as the table is declared where a field says `indexed: true`. The
//! file's own list of what it holds (`sqlite_master`) is the list of what
//! is indexed: nothing is kept beside it to copy, move or put right, a
//! copy of the factory and a database written out again carry each
//! index with the file, and a project moved to a journal of its own has
//! them made again there as they are first asked. Whatever writes a row
//! (a record added, changed, taken to the Trash and back, a column
//! renamed in every row) has it kept right by the database.
//!
//! **What is made is said.** `collection.indexed`, in the write that
//! makes it, and how long that took (`collection.index_made`): a table of
//! a million rows holds its project's journal for a second and a half,
//! once a column.
//!
//! At most [`MOST_INDEXED`] of a table's columns are indexed. One more
//! asked for takes the place of the one asked for least lately that its
//! declaration does not ask for; a declaration asks for no more than
//! that. A column taken away, or made another kind, takes its index with
//! it ([`dropped`]), and one whose column is no longer as it was is let
//! go as the table is first read after a start.

use std::time::Instant;

use kernel::window::statements::{NAMED, dropped, made, name};
use kernel::window::{Column, MOST_INDEXED};
use kernel::{Error, Principal, Result, Source};
use serde_json::json;

use crate::collections::{Collection, Collections, Store};

/// A column as reading in order takes it.
pub(crate) fn column(field: &crate::fields::Field) -> Column {
    Column { name: field.name.clone(), kind: field.kind.clone(), of: field.of.clone() }
}

pub(crate) use ::windowed::Indexes;

impl Collections {
    /// The columns of `table` that are indexed as they are now, each said
    /// to be asked for. An index whose column is gone or is another kind
    /// is taken away.
    pub(crate) async fn indexed(&self, table: &Collection) -> Result<Vec<String>> {
        if table.store != Store::Local {
            return Ok(Vec::new());
        }
        self.read_indexes(&table.project).await?;
        let key = (table.project.clone(), table.name.clone());
        let known = self.indexes.known(|known| known.made.get(&key).cloned().unwrap_or_default());
        let (kept, stale): (Vec<_>, Vec<_>) = known.into_iter().partition(|(field, sql, _)| {
            table.field(field).is_some_and(|field| {
                let column = column(field);
                column.how().is_some() && *sql == made(&table.project, &table.name, &column)
            })
        });
        if !stale.is_empty() {
            let _making = self.indexes.making().lock().await;
            let gone: String =
                stale.iter().map(|(field, ..)| dropped(&table.project, &table.name, field)).collect();
            self.rows(&table.project).db().call(move |connection| connection.execute_batch(&gone)).await?;
            self.indexes.known(|known| {
                if let Some(made) = known.made.get_mut(&key) {
                    made.retain(|(field, ..)| !stale.iter().any(|(gone, ..)| gone == field));
                }
            });
        }
        Ok(kept.into_iter().map(|(field, ..)| field).collect())
    }

    /// Says that the indexes of `fields` of `table` were asked for now,
    /// so that the one asked for least lately is the one let go of.
    pub(crate) fn asked_by(&self, table: &Collection, fields: &[String]) {
        self.indexes.known(|known| {
            known.asked += 1;
            let now = known.asked;
            if let Some(made) = known.made.get_mut(&(table.project.clone(), table.name.clone())) {
                for (_, _, asked) in made.iter_mut().filter(|(field, ..)| fields.contains(field)) {
                    *asked = now;
                }
            }
        });
    }

    /// Reads which indexes the file that keeps `project`'s tables holds,
    /// once after a start.
    async fn read_indexes(&self, project: &str) -> Result<()> {
        if self.indexes.known(|known| known.read.contains(project)) {
            return Ok(());
        }
        let (from, to) = (format!("{NAMED}{project}/"), format!("{NAMED}{project}0"));
        let found: Vec<(String, String)> = self
            .rows(project)
            .db()
            .call(move |connection| {
                connection
                    .prepare("SELECT name, sql FROM sqlite_master WHERE type = 'index' AND name >= ?1 AND name < ?2")?
                    .query_map([from, to], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect()
            })
            .await?;
        let prefix = format!("{NAMED}{project}/");
        self.indexes.known(|known| {
            for (name, sql) in found {
                let Some((table, field)) = name.strip_prefix(&prefix).and_then(|rest| rest.split_once('/'))
                else {
                    continue;
                };
                let made = known.made.entry((project.to_string(), table.to_string())).or_default();
                if !made.iter().any(|(one, ..)| one == field) {
                    // What was made by the program before is the same
                    // statement but for how it began.
                    let sql = sql.replacen("CREATE INDEX ", "CREATE INDEX IF NOT EXISTS ", 1);
                    made.push((field.to_string(), sql, 0));
                }
            }
            known.read.insert(project.to_string());
        });
        Ok(())
    }

    /// Makes the indexes of `fields` of `table` that are not there, as
    /// whoever asked: in a write each that says so. Where that would make
    /// more than [`MOST_INDEXED`], the one asked for least lately that the
    /// table's declaration does not ask for is taken away in the same
    /// write; where there is none, nothing is made and that is said.
    pub(crate) async fn index(&self, table: &Collection, fields: &[String], by: &Principal) -> Result<()> {
        if table.store != Store::Local || fields.is_empty() {
            return Ok(());
        }
        let _making = self.indexes.making().lock().await;
        let there = self.indexes_now(table).await?;
        for field in fields.iter().filter(|field| !there.contains(field)) {
            let Some(column) = table.field(field).map(column).filter(|column| column.how().is_some()) else {
                continue;
            };
            let key = (table.project.clone(), table.name.clone());
            let made_now = self.indexes.known(|known| known.made.get(&key).cloned().unwrap_or_default());
            let mut let_go = None;
            if made_now.len() >= MOST_INDEXED {
                let free = made_now
                    .iter()
                    .filter(|(one, ..)| {
                        !fields.contains(one) && !table.field(one).is_some_and(|field| field.indexed)
                    })
                    .min_by_key(|(.., asked)| *asked);
                let Some((free, ..)) = free else {
                    return Err(Error::invalid(format!(
                        "`{}` has {MOST_INDEXED} columns kept in order already, which is as many as a table has, \
                         and none of them can be let go of: sort it by one of those, or hold the view to fewer rows",
                        table.name
                    )));
                };
                let_go = Some(free.clone());
            }
            let statement = made(&table.project, &table.name, &column);
            let gone =
                let_go.as_ref().map(|free| dropped(&table.project, &table.name, free)).unwrap_or_default();
            let named = name(&table.project, &table.name, field);
            let source = Source::actor(by.actor()).taking(table.reads.as_deref());
            let said = json!({ "project": table.project, "name": table.name, "field": field });
            let began = Instant::now();
            let written = self.rows(&table.project).write(move |writing| {
                let transaction = writing.transaction();
                transaction.execute_batch(&gone)?;
                let there: bool = transaction.query_row(
                    "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ?1)",
                    [&named],
                    |row| row.get(0),
                )?;
                if !there {
                    transaction.execute_batch(&statement)?;
                    writing.say("collection.indexed", source, said)?;
                }
                Ok(!there)
            });
            let written = written.await;
            let labels =
                [("project", table.project.as_str()), ("collection", table.name.as_str()), ("field", field)];
            let took = began.elapsed().as_secs_f64() * 1000.0;
            if matches!(written, Ok((true, _)) | Err(_)) {
                self.factory().measure("collection.index_made", &labels, took, written.is_ok());
            }
            written?;
            let kept = made(&table.project, &table.name, &column);
            self.indexes.known(|known| {
                known.asked += 1;
                let now = known.asked;
                let all = known.made.entry(key).or_default();
                all.retain(|(one, ..)| one != field && Some(one) != let_go.as_ref());
                all.push((field.clone(), kept, now));
            });
        }
        Ok(())
    }

    /// The columns indexed as they are now, read as they are first asked.
    async fn indexes_now(&self, table: &Collection) -> Result<Vec<String>> {
        // Asked holding `making`: what it takes away it takes without it.
        self.read_indexes(&table.project).await?;
        let key = (table.project.clone(), table.name.clone());
        let known = self.indexes.known(|known| known.made.get(&key).cloned().unwrap_or_default());
        Ok(known
            .into_iter()
            .filter(|(field, sql, _)| {
                table.field(field).is_some_and(|one| *sql == made(&table.project, &table.name, &column(one)))
            })
            .map(|(field, ..)| field)
            .collect())
    }

    /// Makes the indexes a table's declaration asks for (`indexed: true`),
    /// as it is declared, whatever rows it has: so that its first rows,
    /// and its millionth, are sorted by them at once. And lets go of what
    /// is no longer as the declaration says.
    pub(crate) async fn index_declared(&self, table: &Collection, by: &Principal) -> Result<()> {
        if table.store != Store::Local {
            return Ok(());
        }
        self.indexed(table).await?;
        let asked: Vec<String> =
            table.fields.iter().filter(|field| field.indexed).map(|field| field.name.clone()).collect();
        self.index(table, &asked, by).await
    }
}
