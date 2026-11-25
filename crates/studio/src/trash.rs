//! The Trash: what somebody deleted, kept so that it can be given back,
//! and emptied for good once it has been there for the days the factory's
//! owner says (`trash.kept`, thirty unless said; `kernel::trash`).
//!
//! Four kinds of thing go there, and one table holds them all
//! (`trash`, a step of the records' tables), each row with what puts its
//! thing back where it was, who deleted it and when:
//!
//! - **a row of a table**, a card included. Its row is taken out of the
//!   records and kept here whole, in the one write that takes it out and
//!   says so, so that nothing that reads a table (a list, a view, a
//!   count, `rows.read`, the board) finds it, and nothing has to be told
//!   to leave it out. Its history, and what was said on it, are kept by
//!   its id as they always were. Given back, it is where it was: its
//!   column and its place there, or the bottom of the column where that
//!   place is somebody else's now; and what hears a card or a record
//!   being added hears it again (`restored`).
//! - **a file of a project**, or a folder (a mockup is one). It leaves
//!   the project's folder, since agents work there, and is kept where
//!   the factory keeps its own things (`trash/<id>/` in the data folder,
//!   with what says where it came from beside it). A document's record
//!   and what was said on it are known by its path, and are its again
//!   when it is given back there. Where something else is at its path
//!   by then, it is given back beside it only when that is asked for.
//! - **a project**, forgotten. Forgetting stops nothing, as it never
//!   did, and leaves its files where they are; the project's row and the
//!   workspace it was in are kept here, and given back it is in that
//!   workspace again, or the usual one where that is gone.
//! - **a channel**, with everything said and attached in it, which is
//!   `talk`'s to take out and put back (`talk::trashed`).
//!
//! A row is kept in the journal that keeps its project's records, beside
//! them (`Journal::records_of`); a project's is the factory's and says no
//! project. So a project moved apart takes its own rows (`KEPT_APART`),
//! and a copy of the factory takes all of them, the folder and the
//! setting.
//!
//! Who sees a thing in the Trash is whoever could read it, a private
//! table's rows only those who may read the table; who gives it back,
//! and who deletes it for good, is whoever could have deleted it.
//!
//! The Trash's rows, its folder, and what goes into it and comes back out
//! as files and as a project's row came out of here with the projects
//! (`projects::trashed`), and are offered under the names they had here.
//! What is here is a record's: taken out of its table into the Trash, and
//! given back into it.

use journal::Journal;
use kernel::trash::back_at;
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde_json::{Value, json};

pub use projects::trashed::{
    FOLDER, GOING, KEEPS, KEPT, Kind, KnownAt, Trashed, WHENCE, keep_for, keeping, new_entry, put_in, rows,
    take_out,
};

use crate::collections::{Collection, Collections, RECORD, Record, Say, Store, read_record};

/// The keeper of a model's records, as it was installed.
type Keeper = std::sync::Arc<dyn crate::factory::Written>;

/// What deleting a record came to: what it is in the Trash, nothing where
/// the factory keeps nothing deleted; and how many records went with it
/// (the pages under a page).
#[derive(Clone, Debug, Default)]
pub struct Gone {
    pub trash: Option<String>,
    pub with: usize,
}

impl Collections {
    /// Deletes a record, saying `say` of it: into the Trash, where the
    /// factory keeps what is deleted, in the one write that takes it out
    /// of the records; at once where it keeps nothing. Whatever its
    /// model's keeper says goes with it (the pages under a page,
    /// [`crate::factory::Written::goes_with`]) goes in the same write,
    /// kept whole in its entry and given back with it.
    pub(crate) async fn trash_saying(
        &self,
        collection: &Collection,
        record: &Record,
        say: Vec<Say>,
        by: &Principal,
    ) -> Result<Gone> {
        let say: Vec<Say> = say.into_iter().map(|said| said.carrying(collection, &record.data)).collect();
        let keeper = self.keeper_of(collection).filter(|_| collection.store == Store::Local);
        // Held until it is written: what is under it cannot change meanwhile, nor any of it be changed.
        let _turns = match &keeper {
            Some(keeper) => self.arranging(keeper, collection, record).await?,
            None => Vec::new(),
        };
        if keeping(self.factory()).await.at_once() {
            let Some(keeper) = keeper else {
                self.remove_saying(collection, record, say).await?;
                // What was attached to it goes with it, as it would from the Trash.
                self.forget_attached(&collection.project, &record.id).await?;
                return Ok(Gone::default());
            };
            let (table, top) = (collection.clone(), record.clone());
            let counting = self.counted.writing(&collection.project, &collection.name);
            let written = self.rows(&collection.project).write(move |writing| {
                let (with, gone) = taken_out(writing, keeper.as_ref(), &table, &top, say, None)?;
                let mut ids: Vec<String> = with.iter().map(|one| one.id.clone()).collect();
                ids.insert(0, top.id.clone());
                keeper.forgotten(writing, &ids)?;
                Ok((ids, gone))
            });
            let ((ids, gone), _) = written.await?;
            counting.moved(Some(-gone));
            for id in &ids {
                self.forget_attached(&collection.project, id).await?;
            }
            return Ok(Gone { trash: None, with: ids.len() - 1 });
        }
        // A document and a mockup are files, and go as files do.
        let file = match collection.store {
            Store::Documents => crate::documents::path_of(collection, &record.id)
                .map(|path| crate::documents::file_of(collection, &path)),
            Store::Mockups => crate::mockups::name_of(collection, &record.id)
                .map(|name| crate::mockups::folder_of(collection, &name)),
            _ => None,
        };
        if let Some(file) = file {
            let trashed = self.projects().trash_path(&collection.project, &file, by).await?;
            self.say_after(say).await;
            return Ok(Gone { trash: trashed.map(|trashed| trashed.id), with: 0 });
        }
        let mut entry = new_entry(self.factory(), Kind::Record, &record.id, &collection.project, by);
        entry.table = Some(collection.name.clone());
        entry.called = title_of(record);
        entry.reads = collection.reads.clone();
        entry.kept = json!(record);
        if collection.store != Store::Local {
            // Kept somewhere the journal's write does not reach: taken
            // out there, and then kept here.
            self.remove_saying(collection, record, say).await?;
            let id = entry.id.clone();
            let written = self.rows(&collection.project).write(move |writing| put_in(writing, &entry));
            written.await?;
            return Ok(Gone { trash: Some(id), with: 0 });
        }
        let (table, top, trashed) = (collection.clone(), record.clone(), entry.id.clone());
        let counting = self.counted.writing(&collection.project, &collection.name);
        let written = self.rows(&collection.project).write(move |writing| {
            let (with, gone) = match &keeper {
                Some(keeper) => taken_out(writing, keeper.as_ref(), &table, &top, say, Some(&entry.id))?,
                None => {
                    let gone =
                        writing.transaction().execute("DELETE FROM records WHERE id = ?1", [&top.id])?;
                    for Say { topic, source, data } in say {
                        writing.say(&topic, source, data)?;
                    }
                    (Vec::new(), gone as i64)
                }
            };
            let went = with.len();
            if went > 0 {
                entry.kept["with"] = json!(with);
            }
            put_in(writing, &entry).map(|_| (went, gone))
        });
        let ((with, gone), _) = written.await?;
        counting.moved(Some(-gone));
        Ok(Gone { trash: Some(trashed), with })
    }

    /// The turn the keeper of `collection`'s model has its records
    /// arranged under, and then the turn of `record` and of each record
    /// that goes with it, read as that one is held: none where the keeper
    /// arranges nothing.
    async fn arranging(
        &self,
        keeper: &Keeper,
        collection: &Collection,
        record: &Record,
    ) -> Result<Vec<tokio::sync::OwnedMutexGuard<()>>> {
        let Some(arranging) = keeper.arranging() else { return Ok(Vec::new()) };
        let mut turns = vec![self.changing(arranging).await];
        let (keeper, table, top) = (keeper.clone(), collection.clone(), record.clone());
        let asked = move |connection: &mut rusqlite::Connection| keeper.goes_with(connection, &table, &top);
        let mut ids = self.rows(&collection.project).db().call(asked).await?;
        ids.push(record.id.clone());
        turns.extend(self.changing_all(&[arranging], &ids).await);
        Ok(turns)
    }

    /// The highest number a record of the table has that is in the Trash,
    /// those that went there with another among them: the next one made is
    /// numbered after it, so that it can be given back under the id it had.
    pub(crate) fn trashed_number_sql() -> &'static str {
        "SELECT coalesce(max(number), 0) FROM (
           SELECT json_extract(kept, '$.number') AS number FROM trash
           WHERE kind = 'record' AND project = ?1 AND of_table = ?2
           UNION ALL
           SELECT json_extract(went.value, '$.number') FROM trash, json_each(trash.kept, '$.with') AS went
           WHERE trash.kind = 'record' AND trash.project = ?1 AND trash.of_table = ?2)"
    }

    /// Gives a record back from the Trash into its table, with every
    /// record that went with it, in one write: where it was, or as its
    /// model's keeper says it is to be now
    /// ([`crate::factory::Written::given_back`]), which may say so in a
    /// sentence, handed back.
    pub async fn restore_record(
        &self,
        journal: &Journal,
        entry: &Trashed,
        by: &Principal,
    ) -> Result<Option<String>> {
        let table = entry.table.clone().unwrap_or_default();
        let unreadable = || Error::internal(format!("what is kept of `{}` cannot be read", entry.thing));
        let mut record: Record = serde_json::from_value(entry.kept.clone()).map_err(|_| unreadable())?;
        let mut with: Vec<Record> = match entry.kept.get("with") {
            Some(with) => serde_json::from_value(with.clone()).map_err(|_| unreadable())?,
            None => Vec::new(),
        };
        let collection = self.find(&entry.project, &table).await?.ok_or_else(|| {
            Error::conflict(format!(
                "`{}` was a row of `{table}`, which `{}` no longer has: it cannot be given back until the \
                 table is there again",
                entry.thing, entry.project
            ))
        })?;
        let keeper = self.keeper_of(&collection).filter(|_| collection.store == Store::Local);
        // Held until it is written, as it was taken out.
        let mut _turns = Vec::new();
        if let Some(arranging) = keeper.as_ref().and_then(|keeper| keeper.arranging()) {
            _turns.push(self.changing(arranging).await);
            let ids: Vec<String> = std::iter::once(&record).chain(&with).map(|one| one.id.clone()).collect();
            _turns.extend(self.changing_all(&[arranging], &ids).await);
        }
        for one in std::iter::once(&record).chain(&with) {
            if self.one(&collection, &one.id, one.number).await?.is_some() {
                return Err(Error::conflict(format!(
                    "there is a record `{}` again, and it is not written over",
                    one.id
                )));
            }
        }
        for one in std::iter::once(&mut record).chain(&mut with) {
            one.data.retain(|name, _| collection.field(name).is_some());
        }
        // Back in its place, or after the last of what it is shown with.
        let column = record.data.get(crate::board::COLUMN).and_then(Value::as_str).map(str::to_string);
        let (taken, highest) = match &column {
            Some(column) => {
                let beside = self.in_column(&collection, column).await?;
                let taken = beside.iter().any(|other| other.position == record.position);
                (taken, beside.iter().map(|other| other.position).reduce(f64::max))
            }
            // Of the whole table, each asked of an index and not by reading it.
            None => (
                self.placed_at(&collection, record.position).await?,
                Some(self.last_position(&collection).await?),
            ),
        };
        record.position = back_at(record.position, taken, highest);
        let actor = by.actor();
        let said = if collection.name == crate::board::BOARD {
            let title = record.data.get("title").cloned().unwrap_or(Value::Null);
            let data = json!({
                "id": record.id, "project": record.project, "title": title,
                "status": column, "by": actor, "restored": true,
            });
            Say::new("board.item.created", (Source::actor(&actor), data)).carrying(&collection, &record.data)
        } else {
            created(&collection, &record, &actor, None)
        };
        let entry = entry.clone();
        if collection.store != Store::Local {
            self.save(&collection, &record, vec![said]).await?;
            let written =
                journal.write(move |writing| take_out(writing, &entry, "trash.restored", &actor, json!({})));
            written.await?;
            return Ok(None);
        }
        let counting = self.counted.writing(&collection.project, &collection.name);
        let back = 1 + with.len() as i64;
        let written = journal.write(move |writing| {
            let mut all: Vec<Record> = std::iter::once(record).chain(with).collect();
            let (top, sentence) = match &keeper {
                Some(keeper) => {
                    let sentence = keeper.given_back(writing.transaction(), &collection, &mut all)?;
                    (all[0].id.clone(), sentence)
                }
                None => (all[0].id.clone(), None),
            };
            for (at, one) in all.iter().enumerate() {
                writing.transaction().execute(
                    &format!("INSERT INTO records ({RECORD}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"),
                    params![
                        one.id,
                        one.project,
                        one.collection,
                        one.number,
                        Value::Object(one.data.clone()).to_string(),
                        one.position,
                        one.created_by,
                        one.created,
                        one.updated
                    ],
                )?;
                let said = match at {
                    0 if keeper.is_some() => created(&collection, one, &actor, None),
                    0 => said.clone(),
                    _ => created(&collection, one, &actor, Some(&top)),
                };
                writing.say(&said.topic, said.source, said.data)?;
            }
            let mut more = json!({});
            if all.len() > 1 {
                more["with"] = json!(all.len() - 1);
            }
            if let Some(sentence) = &sentence {
                more["said"] = json!(sentence);
            }
            take_out(writing, &entry, "trash.restored", &actor, more)?;
            Ok(sentence)
        });
        let (sentence, _) = written.await?;
        counting.moved(Some(back));
        Ok(sentence)
    }
}

/// `record.created` of a record given back from the Trash: with the one
/// it went with and comes back with, where it did.
fn created(collection: &Collection, record: &Record, actor: &str, with: Option<&str>) -> Say {
    let mut data = json!({
        "id": record.id, "project": record.project, "collection": collection.name, "restored": true,
    });
    if let Some(with) = with {
        data["with"] = json!(with);
    }
    Say::new("record.created", (Source::actor(actor).taking(collection.reads.as_deref()), data))
        .carrying(collection, &record.data)
}

/// Takes `top` out of the records in `writing`, saying `say` of it, and
/// with it every record its model's keeper says goes with it, each said
/// to have been taken away with it (`with`, and what it is in the Trash,
/// `trash`, where it was moved there): those records, as they were, and
/// how many rows went.
fn taken_out(
    writing: &mut journal::Writing<'_>,
    keeper: &dyn crate::factory::Written,
    collection: &Collection,
    top: &Record,
    say: Vec<Say>,
    trash: Option<&str>,
) -> rusqlite::Result<(Vec<Record>, i64)> {
    let ids = keeper.goes_with(writing.transaction(), collection, top)?;
    let mut with = Vec::with_capacity(ids.len());
    for id in &ids {
        let asked = format!("SELECT {RECORD} FROM records WHERE id = ?1");
        let found = journal::optional(writing.transaction().query_row(&asked, [id], read_record))?;
        with.extend(found);
    }
    let source = say.first().map(|said| said.source.clone()).unwrap_or_else(|| Source::actor("system"));
    let mut gone = 0;
    for record in std::iter::once(top).chain(&with) {
        gone += writing.transaction().execute("DELETE FROM records WHERE id = ?1", [&record.id])? as i64;
    }
    for Say { topic, source, data } in say {
        writing.say(&topic, source, data)?;
    }
    for record in &with {
        let mut data = json!({
            "id": record.id, "project": record.project, "collection": record.collection, "with": top.id,
        });
        if let Some(trash) = trash {
            data["trash"] = json!(trash);
            data["into"] = json!("the Trash");
        }
        let said = Say::new("record.removed", (source.clone(), data)).carrying(collection, &record.data);
        writing.say(&said.topic, said.source, said.data)?;
    }
    Ok((with, gone))
}

/// What a record is called in the Trash: its title, where it has one.
fn title_of(record: &Record) -> String {
    let said = ["title", "name"].iter().find_map(|name| record.data.get(*name).and_then(Value::as_str));
    match said.map(str::trim).filter(|said| !said.is_empty()) {
        Some(said) => said.to_string(),
        None => record.id.clone(),
    }
}

impl KnownAt for Collections {
    /// The record a file of a project is, by where it is: a library's
    /// document, or a mockup's folder.
    async fn known_at(&self, project: &str, path: &str) -> Option<String> {
        if let Ok(Some((_, id))) = self.document_at(project, path).await {
            return Some(id);
        }
        self.mockup_at(project, path).await.ok().flatten().map(|(_, id)| id)
    }
}
