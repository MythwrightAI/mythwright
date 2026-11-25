//! The factory's own place for tables (`kernel::place::FACTORY`, `@`), and
//! what more the write of a record of a model does where a keeper of that
//! model asks.
//!
//! A table is a project's, or the factory's: a forum's topics are a
//! workspace's or a project's and are moved between the two as a channel
//! is, so they are one table of the factory's and their place is a cell.
//! Declaring a table there takes `admin` beside what declaring any table
//! takes, since it is everybody's; it is kept in the factory's own journal
//! (`Journal::records_of` answers the factory's for a place with no
//! journal of its own) and in its database alone, never as files, since
//! the factory has no folder of its own for them.
//!
//! **One write for every change of a record of a model** whose keeper
//! says so ([`Written`]): a topic changes by its page, a row of a table
//! view, `record set`, a CSV read in, a blueprint's step and an agent, and
//! what goes with a change of it (a revision, what its status came to) is
//! written in the write that keeps the record whichever door it came by,
//! asked of the record as it was there and as it is written. Nothing of
//! it is kept true only by being written through one route.

use journal::Writing;
use kernel::{Error, Principal, Result, Source};
use serde_json::{Map, Value};

use crate::Projects;
use crate::collections::{Collection, Collections, Record, Store};

/// What more the write that keeps a record of a model does, in its
/// transaction: installed once, by the keeper of the models it names.
pub trait Written: Send + Sync {
    /// Whether it is asked of a record of `model`.
    fn keeps(&self, model: &str) -> bool;

    /// What more is written and said, in the write that keeps `record` of
    /// `collection`: `before` is what the row held there, nothing for a
    /// record made, and `source` who it is said to have been written by.
    fn written(
        &self,
        writing: &mut Writing<'_>,
        collection: &Collection,
        before: Option<&Map<String, Value>>,
        record: &Record,
        source: &Source,
    ) -> rusqlite::Result<()>;

    /// The turn whatever changes how the model's records stand among one
    /// another takes (a wiki's pages, under one another), which taking
    /// one into the Trash and giving it back take too, before anything is
    /// read: none for a model whose records stand alone.
    fn arranging(&self) -> Option<&'static str> {
        None
    }

    /// The records that go into the Trash with `record`, and come back
    /// with it, by their ids: read inside the write that takes it out (and
    /// once before it, holding [`Written::arranging`], to take each one's
    /// turn), each before the records under it. None for most.
    fn goes_with(
        &self,
        _connection: &rusqlite::Connection,
        _collection: &Collection,
        _record: &Record,
    ) -> rusqlite::Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// What the records given back from the Trash together are written
    /// as, the one taken out first and those that went with it after:
    /// changed where they are to be, inside the write that gives them
    /// back, as things stand there; and a sentence saying where the first
    /// came to be, where that is not where it was.
    fn given_back(
        &self,
        _connection: &rusqlite::Connection,
        _collection: &Collection,
        _records: &mut [Record],
    ) -> rusqlite::Result<Option<String>> {
        Ok(None)
    }

    /// What more goes with records of the model deleted for good, in the
    /// write that deletes them: what the keeper keeps beside each.
    fn forgotten(&self, _writing: &mut Writing<'_>, _ids: &[String]) -> rusqlite::Result<()> {
        Ok(())
    }
}

/// The place a table is declared at, held to what it is: a project that
/// is there, or the factory's own, which takes `admin` and is kept in the
/// factory's database alone.
pub(crate) async fn placed(projects: &Projects, place: &str, store: Store, by: &Principal) -> Result<()> {
    if !kernel::place::is_factory(place) {
        projects.get(place).await?;
        return Ok(());
    }
    by.require("admin")?;
    if store != Store::Local {
        return Err(Error::invalid(
            "a table of the factory's own (`@`) is kept in the factory's database, and nowhere else: the factory \
             has no folder of its own for files, and no store service is its",
        ));
    }
    Ok(())
}

impl Collections {
    /// Says what more is written with a record of a model, from now on: once.
    pub fn written_with(&self, written: std::sync::Arc<dyn Written>) {
        let _ = self.written.set(written);
    }

    /// What more the write of a record of `collection` does, where its
    /// model's keeper said so: asked inside that write, after the record
    /// is written, with what its row held before ([`before`]).
    pub(crate) fn also_for(&self, collection: &Collection, source: Option<&Source>) -> Option<Also> {
        let written = self.written.get()?.clone();
        if !collection.model.as_deref().is_some_and(|model| written.keeps(model)) {
            return None;
        }
        let collection = collection.clone();
        let source = source.cloned().unwrap_or_else(|| Source::actor("system"));
        Some(Box::new(move |writing, record, before| {
            written.written(writing, &collection, before.as_ref(), record, &source)
        }))
    }
}

impl Collections {
    /// The keeper of `collection`'s model, where one said what more the
    /// write of its records does.
    pub(crate) fn keeper_of(&self, collection: &Collection) -> Option<std::sync::Arc<dyn Written>> {
        let written = self.written.get()?.clone();
        collection.model.as_deref().is_some_and(|model| written.keeps(model)).then_some(written)
    }

    /// What more goes with the records `ids` of `collection` deleted for
    /// good, to be done in the write that deletes them: nothing where its
    /// model's keeper keeps nothing beside them.
    pub fn forgotten_with(&self, collection: &Collection, ids: Vec<String>) -> Option<Forgotten> {
        let keeper = self.keeper_of(collection)?;
        Some(Box::new(move |writing| keeper.forgotten(writing, &ids)))
    }
}

/// What more goes with records deleted for good, in the write that deletes them.
pub type Forgotten = Box<dyn FnOnce(&mut Writing<'_>) -> rusqlite::Result<()> + Send>;

impl Collections {
    /// Each record of a table kept in the journal's database, by its id,
    /// with its position and the cells `named` and nothing else of it: what
    /// arranges the records among one another (a wiki's pages) reads no
    /// body. A table kept anywhere else is read whole and cut down.
    pub async fn cells(
        &self,
        collection: &Collection,
        named: &[&str],
    ) -> Result<Vec<(String, f64, Map<String, Value>)>> {
        let keep = |data: &Map<String, Value>| -> Map<String, Value> {
            named
                .iter()
                .filter_map(|name| data.get(*name).map(|value| (name.to_string(), value.clone())))
                .collect()
        };
        if collection.store != Store::Local {
            let records = self.records(collection).await?;
            return Ok(records
                .iter()
                .map(|record| (record.id.clone(), record.position, keep(&record.data)))
                .collect());
        }
        let picked: Vec<String> =
            named.iter().map(|name| format!("'{name}', json_extract(data, '$.\"{name}\"')")).collect();
        let asked = format!(
            "SELECT id, position, json_object({}) FROM records WHERE project = ?1 AND collection = ?2 ORDER BY position, id",
            picked.join(", ")
        );
        let (project, name) = (collection.project.clone(), collection.name.clone());
        let rows: Vec<(String, f64, String)> = self
            .rows(&project)
            .db()
            .call(move |connection| {
                let mut statement = connection.prepare(&asked)?;
                let rows = statement
                    .query_map([&project, &name], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
                rows.collect()
            })
            .await?;
        Ok(rows
            .into_iter()
            .map(|(id, position, cells)| {
                let cells = serde_json::from_str::<Map<String, Value>>(&cells).unwrap_or_default();
                (id, position, cells.into_iter().filter(|(_, value)| !value.is_null()).collect())
            })
            .collect())
    }
}

/// What more the write that keeps a record does, handed what its row held before.
pub(crate) type Also =
    Box<dyn FnOnce(&mut Writing<'_>, &Record, Option<Map<String, Value>>) -> rusqlite::Result<()> + Send>;

/// What a record's row holds in the write, before it is written again.
pub(crate) fn before(writing: &mut Writing<'_>, id: &str) -> rusqlite::Result<Option<Map<String, Value>>> {
    let held: Option<String> = journal::optional(writing.transaction().query_row(
        "SELECT data FROM records WHERE id = ?1",
        [id],
        |row| row.get(0),
    ))?;
    Ok(held.and_then(|text| serde_json::from_str(&text).ok()))
}
