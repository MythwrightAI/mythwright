//! What is attached to a record: a picture, a video, a log, anything,
//! added by whoever may change the record and read by whoever may read
//! it. A card is a record, so this is a card's too.
//!
//! It is kept the way a channel's attachments are (`talk::attachments`):
//! a file of its own beside the database, in the same folder of the data
//! folder, under an id that is its alone, with its name, its kind, its
//! size and who added it in a row of `record_attachments`. The row is its
//! record's project's, and is kept where that project's records are
//! (`crate::apart`); added and taken away each in one write with what is
//! said of it (`record.attached`, `record.detached`), said as what is said
//! of the record is, so that of a table kept private only whoever may
//! read the table hears it.
//!
//! It is kept by its record's id, as a record's history is: a record in
//! the Trash keeps what is attached to it, which nothing reads while the
//! record is not there, and has it again when it is given back. A record
//! deleted for good, or at once where the factory keeps nothing that is
//! deleted, takes it away with it ([`Collections::forget_attached`]).
//! One taken away goes into the Trash, and is given back from there by
//! the `trash` crate (`trash::attached`).

use std::path::PathBuf;

use blueprint::{ActionSpec, ValueType as T};
use conductor::Registry;
use journal::{Journal, optional};
use kernel::{Error, Principal, Result};
use rusqlite::params;
use serde::Serialize;
use serde_json::json;

use crate::Studio;
use crate::actions::{add, text, through};
use crate::board::Through;
use crate::collections::{Collection, Collections, Record, Say, Store};
use crate::records::Records;
use crate::trash::{Kind, Trashed};

/// What is said when something is attached to a record.
pub const ATTACHED: &str = "record.attached";
/// What is said when something attached to a record is taken away.
pub const DETACHED: &str = "record.detached";
/// What the id of something attached to a record begins with: a channel's
/// begin otherwise, and whatever reads one by its id goes by this.
pub const PREFIX: &str = "ratt";
/// The most one record carries.
pub const MOST: usize = 100;

const COLUMNS: &str = "id, record, name, media, size, actor, who, created";

/// Something attached to a record, as it is handed out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Attached {
    pub id: String,
    pub record: String,
    /// The name it had where it came from.
    pub name: String,
    /// What kind of thing it is, as a media type: `image/png`.
    pub media: String,
    pub size: u64,
    /// Who added it, as the journal names them, and by name as they were then.
    pub by: String,
    pub who: String,
    pub at: i64,
}

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Attached> {
    Ok(Attached {
        id: row.get(0)?,
        record: row.get(1)?,
        name: row.get(2)?,
        media: row.get(3)?,
        size: row.get::<_, i64>(4)? as u64,
        by: row.get(5)?,
        who: row.get(6)?,
        at: row.get(7)?,
    })
}

/// Writes the row of something attached to a record of `project`, in a
/// write that says so: as it is added, and as it is given back
/// (`trash::attached`).
pub fn put(writing: &mut journal::Writing<'_>, project: &str, row: &Attached) -> rusqlite::Result<()> {
    let sql = format!(
        "INSERT INTO record_attachments (project, {COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    );
    let values =
        params![project, row.id, row.record, row.name, row.media, row.size as i64, row.by, row.who, row.at];
    writing.transaction().execute(&sql, values).map(|_| ())
}

/// Whether an id is of something attached to a record, and not to a message.
pub fn is_of_a_record(id: &str) -> bool {
    id.strip_prefix(PREFIX).is_some_and(|rest| rest.starts_with('_'))
}

impl Collections {
    /// Where the files of what is attached are kept, as the studio was opened.
    pub fn attached_in(&self) -> Result<PathBuf> {
        self.attached.get().cloned().ok_or_else(|| Error::internal("the factory keeps nothing attached here"))
    }

    /// What is attached to `record` of `collection`, in the order it was added.
    pub(crate) async fn attached_to(&self, collection: &Collection, record: &str) -> Result<Vec<Attached>> {
        let record = record.to_string();
        let sql = format!("SELECT {COLUMNS} FROM record_attachments WHERE record = ?1 ORDER BY created, id");
        self.rows(&collection.project)
            .db()
            .call(move |connection| connection.prepare(&sql)?.query_map([&record], read)?.collect())
            .await
    }

    /// Whether anything is attached to the record `id`, asked of the
    /// journal of each project the id may be of and of nothing else, with
    /// nothing asked of whose it is: for what is told of a record on the
    /// way of a factory at work (an agent's turn on a card), which reads
    /// the record and who may read it only where something is attached.
    pub async fn any_attached(&self, id: &str) -> Result<bool> {
        let dashes = id.match_indices('-').map(|(at, _)| &id[..at]);
        let mut asked: Vec<Journal> = Vec::new();
        for journal in dashes.map(|project| self.rows(project)) {
            if asked.iter().any(|one| one.name() == journal.name()) {
                continue;
            }
            let record = id.to_string();
            let sql = "SELECT EXISTS (SELECT 1 FROM record_attachments WHERE record = ?1)";
            if journal
                .db()
                .call(move |connection| connection.query_row(sql, [&record], |row| row.get(0)))
                .await?
            {
                return Ok(true);
            }
            asked.push(journal);
        }
        Ok(false)
    }

    /// Takes away what is attached to the documents and the mockups whose
    /// files are `path` of `project` or are under it, a folder, saying
    /// nothing: for a file or a folder deleted for good, or at once. A
    /// file inside a mockup's folder is no mockup, and takes nothing away.
    /// Which file a record is, is asked of its table, and whether it is
    /// under `path` of `kernel::whole::within`, a part of a path at a time.
    pub async fn forget_attached_at(&self, project: &str, path: &str) -> Result<usize> {
        let deleted = std::path::PathBuf::from(path.trim_end_matches('/'));
        let tables = self.list(Some(project)).await?;
        let owner = project.to_string();
        let records: Vec<String> = self
            .rows(project)
            .db()
            .call(move |connection| {
                let sql = "SELECT DISTINCT record FROM record_attachments WHERE project = ?1";
                connection.prepare(sql)?.query_map([&owner], |row| row.get(0))?.collect()
            })
            .await?;
        let mut forgotten = 0;
        for record in records {
            let file = tables.iter().find_map(|table| match table.store {
                Store::Documents => crate::documents::path_of(table, &record)
                    .map(|path| crate::documents::file_of(table, &path)),
                Store::Mockups => crate::mockups::name_of(table, &record)
                    .map(|name| crate::mockups::folder_of(table, &name)),
                _ => None,
            });
            if file.is_some_and(|file| kernel::whole::within(std::path::Path::new(&file), &deleted)) {
                forgotten += self.forget_attached(project, &record).await?;
            }
        }
        Ok(forgotten)
    }

    /// Takes away everything attached to the records of `project`, its rows
    /// and its files, saying nothing: for a project deleted for good, from
    /// the Trash or at once, whose other files go with it.
    pub async fn forget_project_attached(&self, project: &str) -> Result<usize> {
        let mut gone: Vec<String> = Vec::new();
        let journals = [self.rows(project), self.factory().clone()];
        for (at, journal) in journals.iter().enumerate() {
            if at > 0 && journal.name() == journals[0].name() {
                continue;
            }
            let project = project.to_string();
            gone.extend(
                journal
                    .db()
                    .call(move |connection| {
                        let ids: Vec<String> = connection
                            .prepare("SELECT id FROM record_attachments WHERE project = ?1")?
                            .query_map([&project], |row| row.get(0))?
                            .collect::<rusqlite::Result<_>>()?;
                        connection
                            .execute("DELETE FROM record_attachments WHERE project = ?1", [&project])?;
                        Ok(ids)
                    })
                    .await?,
            );
        }
        if let Ok(dir) = self.attached_in() {
            for id in &gone {
                let _ = std::fs::remove_file(dir.join(id));
            }
        }
        Ok(gone.len())
    }

    /// Takes away what is attached to `record` of `project`, its rows and
    /// its files, saying nothing: for a record that is gone for good,
    /// which said so itself.
    pub async fn forget_attached(&self, project: &str, record: &str) -> Result<usize> {
        let record = record.to_string();
        let gone: Vec<String> = self
            .rows(project)
            .db()
            .call(move |connection| {
                let ids: Vec<String> = connection
                    .prepare("SELECT id FROM record_attachments WHERE record = ?1")?
                    .query_map([&record], |row| row.get(0))?
                    .collect::<rusqlite::Result<_>>()?;
                connection.execute("DELETE FROM record_attachments WHERE record = ?1", [&record])?;
                Ok(ids)
            })
            .await?;
        if let Ok(dir) = self.attached_in() {
            for id in &gone {
                let _ = std::fs::remove_file(dir.join(id));
            }
        }
        Ok(gone.len())
    }
}

impl Records {
    /// The record `id` and its table, for `by`, who is told nothing of
    /// one where they may neither read nor change that kind of record.
    async fn found(&self, id: &str, by: &Principal) -> Result<(Collection, Record)> {
        let read = ["board:read", "board:write", "records:read", "records:write"];
        if !read.iter().any(|scope| by.can(scope)) {
            by.require("records:read")?;
        }
        self.collections.record(id).await
    }

    /// What is said of something attached, as what is said of its record is.
    pub fn said(
        collection: &Collection,
        attached: &Attached,
        by: &Principal,
        through: Option<&Through>,
    ) -> Say {
        let data = json!({
            "id": attached.record, "project": collection.project, "collection": collection.name,
            "attachment": attached.id, "name": attached.name, "media": attached.media, "size": attached.size,
            "by": attached.by,
        });
        let (source, data) = crate::board::told(by, through, data);
        Say { topic: String::new(), source: source.taking(collection.reads.as_deref()), data }
    }

    /// Attaches a file to the record `id`, for `by`, who may change it:
    /// its name, what it says it is where it says, and what it holds.
    pub async fn attach(
        &self,
        id: &str,
        (name, media): (&str, Option<&str>),
        bytes: Vec<u8>,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<Attached> {
        let (collection, record) = self.found(id, by).await?;
        collection.may_change(by)?;
        if bytes.is_empty() {
            return Err(Error::invalid("there is nothing in that file to attach"));
        }
        let most = talk::attachments::MAX_ATTACHMENT_BYTES;
        if bytes.len() > most {
            return Err(Error::invalid(format!(
                "that is too large to attach: the most is {} MB. Put it in the project and say where it is",
                most / (1024 * 1024)
            )));
        }
        let collections = &self.collections;
        let _turn = collections.changing(&record.id).await;
        if collections.attached_to(&collection, &record.id).await?.len() >= MOST {
            return Err(Error::invalid(format!(
                "`{}` carries {MOST} attachments, which is the most a record carries: take one away first",
                record.id
            )));
        }
        let name = talk::attachments::plain_name(name);
        let said = media
            .map(|media| media.split(';').next().unwrap_or_default().trim())
            .filter(|media| !media.is_empty() && *media != "application/octet-stream");
        let journal = &self.journal;
        let attached = Attached {
            id: journal.new_id(PREFIX),
            record: record.id.clone(),
            media: said.map_or_else(|| talk::attachments::media_of(&name).to_string(), str::to_string),
            size: bytes.len() as u64,
            name,
            by: match (through, &by.key) {
                (None, Some(key)) => format!("key:{key}"),
                _ => by.actor(),
            },
            who: by.name.clone(),
            at: journal.now_ms(),
        };
        let file = collections.attached_in()?.join(&attached.id);
        std::fs::write(&file, bytes)?;
        let say = Self::said(&collection, &attached, by, through).carrying(&collection, &record.data);
        let (row, project) = (attached.clone(), collection.project.clone());
        let written = collections.rows(&collection.project).write(move |writing| {
            put(writing, &project, &row)?;
            writing.say("record.attached", say.source, say.data)
        });
        if let Err(error) = written.await {
            let _ = std::fs::remove_file(&file);
            return Err(error);
        }
        Ok(attached)
    }

    /// What is attached to the record `id`, for `by`, who may read it.
    pub async fn attachments(&self, id: &str, by: &Principal) -> Result<Vec<Attached>> {
        let (collection, record) = self.found(id, by).await?;
        collection.may_read(by)?;
        self.collections.attached_to(&collection, &record.id).await
    }

    /// Something attached to a record, by its id, and where its file is,
    /// for `by`, who may read the record. One of a record that is not
    /// there (in the Trash, or gone) answers as one that is not.
    pub async fn attachment(&self, id: &str, by: &Principal) -> Result<(Attached, PathBuf)> {
        let missing = || Error::not_found(format!("there is nothing attached called `{id}`"));
        let mut found = None;
        for journal in self.journal.every_records() {
            let lookup = id.to_string();
            let sql = format!("SELECT {COLUMNS} FROM record_attachments WHERE id = ?1");
            found = journal
                .db()
                .call(move |connection| optional(connection.query_row(&sql, [&lookup], read)))
                .await?;
            if found.is_some() {
                break;
            }
        }
        let attached = found.ok_or_else(missing)?;
        let (collection, _) = self.found(&attached.record, by).await.map_err(|err| match err.kind {
            kernel::ErrorKind::NotFound => missing(),
            _ => err,
        })?;
        collection.may_read(by)?;
        let file = self.collections.attached_in()?.join(&attached.id);
        Ok((attached, file))
    }

    /// Takes something attached to the record `record` away, for `by`, who
    /// may change it.
    pub async fn detach(
        &self,
        record: &str,
        id: &str,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<(Attached, Option<String>)> {
        let (collection, found) = self.found(record, by).await?;
        collection.may_change(by)?;
        let collections = &self.collections;
        let _turn = collections.changing(&found.id).await;
        let attached = collections.attached_to(&collection, &found.id).await?;
        let attached = attached
            .into_iter()
            .find(|one| one.id == id)
            .ok_or_else(|| Error::not_found(format!("`{}` has nothing attached called `{id}`", found.id)))?;
        let mut say = Self::said(&collection, &attached, by, through).carrying(&collection, &found.data);
        let file = collections.attached_in()?.join(&attached.id);
        // Into the Trash where the factory keeps what is deleted: its row
        // there with what is said in one write, and then its file moved
        // into the entry's folder, so that a stop between leaves it where
        // it was, which giving it back and deleting it for good look in too.
        let folder = collections.projects().trash_folder();
        let keeps = !crate::trash::keeping(collections.factory()).await.at_once();
        let entry = folder.as_ref().filter(|_| keeps).map(|folder| {
            let at = self.journal.now_ms();
            let mut entry = Trashed::new(
                self.journal.new_id("trash"),
                Kind::Attachment,
                &found.id,
                &collection.project,
                &by.actor(),
                at,
            );
            entry.table = Some(collection.name.clone());
            entry.called = attached.name.clone();
            entry.reads = collection.reads.clone();
            entry.kept = json!(attached);
            (entry, folder.clone())
        });
        if let Some((entry, _)) = &entry {
            say.data["trash"] = json!(entry.id);
            say.data["into"] = json!("the Trash");
        }
        let (gone, put) = (attached.id.clone(), entry.as_ref().map(|(entry, _)| entry.clone()));
        let written = collections.rows(&collection.project).write(move |writing| {
            writing.transaction().execute("DELETE FROM record_attachments WHERE id = ?1", [&gone])?;
            writing.say("record.detached", say.source, say.data)?;
            match &put {
                Some(entry) => crate::trash::put_in(writing, entry),
                None => Ok(()),
            }
        });
        written.await?;
        match entry {
            Some((entry, folder)) => {
                let kept = folder.join(&entry.id);
                std::fs::create_dir_all(&kept)?;
                std::fs::rename(&file, kept.join(crate::trash::KEPT))?;
                Ok((attached, Some(entry.id)))
            }
            None => {
                let _ = std::fs::remove_file(&file);
                Ok((attached, None))
            }
        }
    }
}

/// The action a blueprint asks to attach a file of a project to a record,
/// which reaches the project's files: a stand-in in a sandbox.
pub(crate) fn register(registry: &mut Registry, studio: &Studio) {
    add(
        registry,
        studio,
        ActionSpec::new(
            "record.attach",
            "Attach a file of the record's project to a record, a card included, as a copy: a picture, a video or a log that shows something. Needs what changing the record needs.",
        )
        .example(kernel::asked::RECORD_ATTACH)
        .required("id", T::String, "the record")
        .required("path", T::String, "the file, from the top of the record's project")
        .arg("name", T::String, "what it is called where it is attached (default: the file's own name)")
        .capability("projects:read")
        .output("id", "what is attached, by its id: what reads it (`attachment`)")
        .output("name", "what it is called")
        .output("size", "how big it is, in bytes"),
        |studio, cx, args| async move {
            let (id, path) = (text(&args, "id").unwrap_or_default(), text(&args, "path").unwrap_or_default());
            let by = cx.principal();
            by.require("projects:read")?;
            let (collection, _) = studio.collections.record(&id).await?;
            let projects = &studio.projects;
            projects.may_read_path(&collection.project, &path, by).await?;
            let bytes = projects.read_bytes(&collection.project, &path).await?;
            let name = text(&args, "name").unwrap_or_else(|| path.rsplit('/').next().unwrap_or(&path).to_string());
            let attached = studio.records.attach(&id, (&name, None), bytes, by, Some(&through(&cx))).await?;
            Ok(json!({ "id": attached.id, "name": attached.name, "size": attached.size }))
        },
    );
}

/// What the files of what is attached to records in a database are,
/// for a copy: none where it keeps no records.
pub fn attached_ids(connection: &rusqlite::Connection) -> rusqlite::Result<Vec<String>> {
    let made: bool = connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'record_attachments')",
        [],
        |row| row.get(0),
    )?;
    if !made {
        return Ok(Vec::new());
    }
    let mut statement = connection.prepare("SELECT id FROM record_attachments ORDER BY created")?;
    let rows = statement.query_map([], |row| row.get(0))?;
    rows.collect()
}
