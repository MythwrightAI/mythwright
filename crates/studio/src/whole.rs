//! A record written, and what is said of it, as one write.
//!
//! A record is put where it goes and an event says so: a card is moved
//! and `board.item.moved` is said. Written and then said, as two pieces
//! of work, a daemon stopped between them left a card where nobody had
//! been told it was, and whatever was to hear of it waited: a look at
//! every start put that right from what the rows held
//! ([`crate::board::Board::settle`]), and could only look where it knew
//! to. So whoever writes a record hands over what is to be said of it
//! ([`Say`]), and the two are one write of the journal's
//! ([`journal::Journal::write`]): both are kept, or neither is, wherever
//! the daemon is stopped.
//!
//! That is so of a table kept in the journal's database, which is where
//! a board's cards usually are. A table kept as files of the project, by
//! a store service, or as the documents or mockups of a folder is written
//! somewhere the journal's write does not reach: it is written first and
//! said after, as it always was, and the look at a start is what holds it.

use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde_json::Value;

use crate::collections::{Collection, Collections, RECORD, Record, Store, files};

/// What else the write that keeps a record does, and says, in its
/// transaction: where it fails, nothing of the write is kept (an approval spent with
/// the change it allows: `told::asks`).
pub type Also = Box<dyn for<'a, 'b> FnOnce(&'a mut journal::Writing<'b>) -> rusqlite::Result<()> + Send>;

/// Something to be said of a record in the write that keeps it.
#[derive(Clone, Debug)]
pub struct Say {
    pub topic: String,
    pub source: Source,
    pub data: Value,
}

impl Say {
    /// An event of `topic`, by whom and with what it says.
    pub fn new(topic: &str, (source, data): (Source, Value)) -> Say {
        Say { topic: topic.to_string(), source, data }
    }
}

impl Collections {
    /// [`Collections::put_saying`], with what else the write does.
    pub(crate) async fn put_also(
        &self,
        collection: &Collection,
        mut record: Record,
        say: impl FnOnce(&Record) -> Vec<Say> + Send,
        also: Option<Also>,
    ) -> Result<Record> {
        record.data = crate::fields::check(collection, record.data)?;
        self.pointed(collection, &record.data, Some(&record.id)).await?;
        record.updated = self.factory().now_ms();
        self.save_also(collection, &record, say(&record), also).await?;
        Ok(record)
    }

    /// Says what could not be kept with what it is of, after it.
    pub(crate) async fn say_after(&self, say: Vec<Say>) {
        for Say { topic, source, data } in say {
            let _ = self.factory().emit(&topic, source, data).await;
        }
    }

    /// Writes a record where its collection keeps its records, and says
    /// `say` of it: in the one write, where that is the journal's
    /// database.
    pub(crate) async fn save(&self, collection: &Collection, record: &Record, say: Vec<Say>) -> Result<()> {
        self.save_also(collection, record, say, None).await
    }

    /// The same, with what else the write does (`also`), which only a
    /// table kept in the journal's database can do in the same write: one
    /// kept anywhere else is refused, with nothing written.
    pub(crate) async fn save_also(
        &self,
        collection: &Collection,
        record: &Record,
        say: Vec<Say>,
        also: Option<Also>,
    ) -> Result<()> {
        let say: Vec<Say> = say.into_iter().map(|said| said.carrying(collection, &record.data)).collect();
        if also.is_some() && collection.store != Store::Local {
            return Err(Error::invalid(format!(
                "`{}` keeps its records outside the factory's database, where a change of one and what goes \
                 with it cannot be one write: this is not done for such a table",
                collection.name
            )));
        }
        match collection.store {
            Store::Local => {
                let row = record.clone();
                let counting = self.counted.writing(&collection.project, &collection.name);
                let model = self.also_for(collection, say.first().map(|said| &said.source));
                let written = self.rows(&collection.project).write(move |writing| {
                    if let Some(also) = also {
                        also(writing)?;
                    }
                    let before = match model {
                        Some(_) => crate::factory::before(writing, &row.id)?,
                        None => None,
                    };
                    // A row written over one that was there adds none.
                    let was: i64 = writing.transaction().query_row(
                        "SELECT count(*) FROM records WHERE id = ?1",
                        [&row.id],
                        |found| found.get(0),
                    )?;
                    writing.transaction().execute(
                        &format!("INSERT OR REPLACE INTO records ({RECORD}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"),
                        params![
                            row.id,
                            row.project,
                            row.collection,
                            row.number,
                            Value::Object(row.data.clone()).to_string(),
                            row.position,
                            row.created_by,
                            row.created,
                            row.updated
                        ],
                    )?;
                    for Say { topic, source, data } in say {
                        writing.say(&topic, source, data)?;
                    }
                    if let Some(model) = model {
                        model(writing, &row, before)?;
                    }
                    Ok(1 - was)
                });
                let (added, _) = written.await?;
                counting.moved(Some(added));
                return Ok(());
            }
            Store::Files => {
                let path = format!("{}/{}.md", collection.name, record.number);
                let text = files::write(collection, record);
                self.projects().write(&collection.project, &path, &text, &Principal::system()).await?;
            }
            Store::Service => self.service.put(collection, record).await?,
            Store::Documents => self.save_document(collection, record).await?,
            Store::Mockups => self.save_mockup(collection, record).await?,
        }
        self.say_after(say).await;
        Ok(())
    }

    /// Writes several records of one table back, changed, and says `say`
    /// of them, as one write: each through what its model's keeper has a
    /// write of one do (a topic's revision and status), as one written
    /// alone is. Each said thing names the record it is of, by where it is
    /// in `records`. Only a table kept in the journal's database can do it:
    /// a page moved with every page under it, which would otherwise be a
    /// tree half moved wherever the daemon stopped.
    pub async fn put_many_saying(
        &self,
        collection: &Collection,
        records: Vec<Record>,
        say: impl FnOnce(&[Record]) -> Vec<(usize, Say)> + Send,
    ) -> Result<Vec<Record>> {
        if collection.store != Store::Local {
            return Err(Error::invalid(format!(
                "`{}` keeps its records outside the factory's database, where several of them cannot be written \
                 as one: this is not done for such a table",
                collection.name
            )));
        }
        let now = self.factory().now_ms();
        let mut written = Vec::with_capacity(records.len());
        for mut record in records {
            record.data = crate::fields::check(collection, record.data)?;
            self.pointed(collection, &record.data, Some(&record.id)).await?;
            record.updated = now;
            written.push(record);
        }
        let mut said: Vec<Vec<Say>> = written.iter().map(|_| Vec::new()).collect();
        for (of, one) in say(&written) {
            if let Some(record) = written.get(of) {
                said[of].push(one.carrying(collection, &record.data));
            }
        }
        let models: Vec<_> =
            said.iter().map(|says| self.also_for(collection, says.first().map(|one| &one.source))).collect();
        let rows = written.clone();
        let counting = self.counted.writing(&collection.project, &collection.name);
        let kept = self.rows(&collection.project).write(move |writing| {
            let mut added = 0;
            for ((row, says), model) in rows.into_iter().zip(said).zip(models) {
                let before = match model {
                    Some(_) => crate::factory::before(writing, &row.id)?,
                    None => None,
                };
                // A row written over one that was there adds none.
                added += 1 - writing.transaction().query_row(
                    "SELECT count(*) FROM records WHERE id = ?1",
                    [&row.id],
                    |found| found.get::<_, i64>(0),
                )?;
                writing.transaction().execute(
                    &format!("INSERT OR REPLACE INTO records ({RECORD}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"),
                    params![
                        row.id,
                        row.project,
                        row.collection,
                        row.number,
                        Value::Object(row.data.clone()).to_string(),
                        row.position,
                        row.created_by,
                        row.created,
                        row.updated
                    ],
                )?;
                for Say { topic, source, data } in says {
                    writing.say(&topic, source, data)?;
                }
                if let Some(model) = model {
                    model(writing, &row, before)?;
                }
            }
            Ok(added)
        });
        let (added, _) = kept.await?;
        counting.moved(Some(added));
        Ok(written)
    }

    /// Takes a record away from where its collection keeps its records,
    /// and says `say` of that: in the one write, where that is the
    /// journal's database.
    pub async fn remove_saying(&self, collection: &Collection, record: &Record, say: Vec<Say>) -> Result<()> {
        let say: Vec<Say> = say.into_iter().map(|said| said.carrying(collection, &record.data)).collect();
        match collection.store {
            Store::Local => {
                let id = record.id.clone();
                let counting = self.counted.writing(&collection.project, &collection.name);
                let written = self.rows(&collection.project).write(move |writing| {
                    let gone = writing.transaction().execute("DELETE FROM records WHERE id = ?1", [&id])?;
                    for Say { topic, source, data } in say {
                        writing.say(&topic, source, data)?;
                    }
                    Ok(gone as i64)
                });
                let (gone, _) = written.await?;
                counting.moved(Some(-gone));
                return Ok(());
            }
            Store::Files => {
                let path = format!("{}/{}.md", collection.name, record.number);
                self.projects().remove(&collection.project, &path, &Principal::system()).await?;
            }
            Store::Service => self.service.remove(collection, &record.id).await?,
            Store::Documents => self.remove_document(collection, record).await?,
            Store::Mockups => self.remove_mockup(collection, record).await?,
        }
        self.say_after(say).await;
        Ok(())
    }
}
