//! The documents a channel keeps to hand: what somebody added to its
//! context, for everybody in it and for its agents.
//!
//! **What an entry is.** A document of a project's library, by its
//! record's id (`shop-docs:guides:install`), or a file of a project's
//! folder, by the project and its path (`shop/README.md`, which is how
//! it is written wherever one is said as text). A record's id never has
//! a `/` in it, so the two are told apart by that ([`Doc::parse`]). Each
//! is kept by that text and nothing else: this crate knows no library and
//! no project's files, so what an entry names is looked at by whoever
//! shows it, as the one who looks may read it, and one that names
//! something gone, or kept from them, is said to be so and never left
//! out (`agents::docs`).
//!
//! **Whose.** The list is the channel's, the same for everybody in it,
//! and kept with it: a row of `channel_docs` an entry, taken into the
//! Trash with the channel, given back with it and gone with it for good
//! (`crate::trashed`), kept in a copy of the factory as its other tables
//! are. Reading it is reading the channel; adding to it and taking from
//! it is writing in it ([`Channels::may_write`]), refused while the
//! channel is archived. Whether whoever adds an entry may read what it
//! names is asked above this crate, which knows no table or file: the
//! route asks it before it asks this.
//!
//! **One write.** Each change and what is said of it (`channel.docs`,
//! with what was done and the entry) are one write, said as anything of
//! the channel is: to whoever may read channels, and of one kept to its
//! members to its members alone. One added that is there already, and
//! one taken away that is not, write nothing and say nothing.

use kernel::{Error, Principal, Result, Source};
use serde::Serialize;
use serde_json::json;

use crate::channels::{Channel, Channels};
use crate::door::called;
use crate::kept::Reader;

/// The most entries a channel keeps.
pub const MOST: usize = 100;
/// The longest an entry is, as text.
const LONGEST: usize = 600;

/// What an entry names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Doc {
    /// A document of a library, by its record's id.
    Record(String),
    /// A file of a project's folder.
    File { project: String, path: String },
}

impl Doc {
    /// What `said` names: `<project>/<path>` a file, and anything else a
    /// record's id. Refused where it is neither: empty, too long, of
    /// several lines, or a path that leads out of its project.
    pub fn parse(said: &str) -> Result<Doc> {
        let said = said.trim();
        if said.is_empty() || said.len() > LONGEST || said.contains(['\n', '\r', '\\']) {
            return Err(Error::invalid(format!(
                "a document is said as a library's record (`shop-docs:guides:install`) or a project's file \
                 (`shop/README.md`), on one line and in {LONGEST} letters at most"
            )));
        }
        match said.split_once('/') {
            None => Ok(Doc::Record(said.to_string())),
            Some((project, path)) => Doc::file(project, path),
        }
    }

    /// A file of `project` at `path`, refused where the path is not one
    /// within it, said plainly.
    pub fn file(project: &str, path: &str) -> Result<Doc> {
        let path = path.trim().trim_start_matches("./");
        let sound = |part: &str| !part.is_empty() && part != "." && part != "..";
        if project.trim().is_empty()
            || project.contains(['/', ':', '\\'])
            || path.is_empty()
            || path.starts_with('/')
            || path.contains('\\')
            || !path.split('/').all(sound)
        {
            return Err(Error::invalid(format!(
                "`{project}/{path}` is no file of a project: a file is said by its project and its path within \
                 it, as `shop/docs/notes.md`, with no `..` and no leading `/`"
            )));
        }
        Ok(Doc::File { project: project.trim().to_string(), path: path.to_string() })
    }

    /// The text it is kept and said by.
    pub fn key(&self) -> String {
        match self {
            Doc::Record(id) => id.clone(),
            Doc::File { project, path } => format!("{project}/{path}"),
        }
    }
}

/// An entry of a channel's documents, as it is kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Kept {
    /// What it names, as [`Doc::key`] says it.
    pub doc: String,
    /// Who added it, as an actor.
    pub by: String,
    /// When, in milliseconds.
    pub added: i64,
}

impl Kept {
    /// What it names.
    pub fn named(&self) -> Option<Doc> {
        Doc::parse(&self.doc).ok()
    }
}

/// What is said of a change to a channel's documents.
fn said_of(channel: &Channel, doc: &str, did: &str, by: &Principal) -> serde_json::Value {
    let mut data = json!({
        "channel": channel.id, "name": channel.name, "doc": doc, "did": did, "by": by.actor(), "by_name": by.name,
    });
    for (key, said) in [("title", channel.title.as_deref()), ("project", channel.project.as_deref())] {
        if let Some(said) = said {
            data[key] = json!(said);
        }
    }
    data
}

fn read_kept(connection: &rusqlite::Connection, channel: &str) -> rusqlite::Result<Vec<Kept>> {
    let mut statement = connection
        .prepare("SELECT doc, by, added FROM channel_docs WHERE channel = ?1 ORDER BY added, rowid")?;
    let rows = statement
        .query_map([channel], |row| Ok(Kept { doc: row.get(0)?, by: row.get(1)?, added: row.get(2)? }))?;
    rows.collect()
}

impl Channels {
    /// The documents of the channel `reader` is leave to read, in the
    /// order they were added.
    pub async fn docs(&self, reader: &Reader, channel: &str) -> Result<Vec<Kept>> {
        let found = self.get(reader, channel).await?;
        self.journal.db().call(move |connection| read_kept(connection, &found.id)).await
    }

    /// Adds `doc` to the documents of the channel `reader` is leave to
    /// write in, as `by`. Whether it was not there and is now.
    pub async fn add_doc(&self, reader: &Reader, channel: &str, doc: &Doc, by: &Principal) -> Result<bool> {
        self.change_doc(reader, channel, doc.key(), true, by).await
    }

    /// Takes the entry said as `doc` from the documents of the channel
    /// `reader` is leave to write in, as `by`. Whether it was there.
    pub async fn remove_doc(
        &self,
        reader: &Reader,
        channel: &str,
        doc: &str,
        by: &Principal,
    ) -> Result<bool> {
        self.change_doc(reader, channel, doc.trim().to_string(), false, by).await
    }

    async fn change_doc(
        &self,
        reader: &Reader,
        channel: &str,
        doc: String,
        add: bool,
        by: &Principal,
    ) -> Result<bool> {
        reader.writing()?;
        let found = self.get(reader, channel).await?;
        if found.archived.is_some() {
            return Err(Error::conflict(format!(
                "{} is archived, and nothing is added to it or taken from it: whoever may write in it gives it \
                 back first (`{} channel unarchive {}`)",
                called(&found),
                brand::CLI,
                found.name
            )));
        }
        let (row, by_now, now) = (found, by.clone(), self.journal.now_ms());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let changed = match add {
                true => {
                    let held: i64 = transaction
                        .query_row("SELECT COUNT(*) FROM channel_docs WHERE channel = ?1", [&row.id], |one| one.get(0))?;
                    if held as usize >= MOST {
                        return Ok(None);
                    }
                    transaction.execute(
                        "INSERT OR IGNORE INTO channel_docs (channel, doc, by, added) VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![row.id, doc, by_now.actor(), now],
                    )?
                }
                false => transaction
                    .execute("DELETE FROM channel_docs WHERE channel = ?1 AND doc = ?2", rusqlite::params![row.id, doc])?,
            };
            if changed == 1 {
                let source = Source::actor(by_now.actor()).taking(row.takes(&[]).as_deref());
                let did = if add { "added" } else { "removed" };
                writing.say_here("channel.docs", source, said_of(&row, &doc, did, &by_now))?;
            }
            Ok(Some(changed == 1))
        });
        match wrote.await?.0 {
            Some(changed) => Ok(changed),
            None => Err(Error::conflict(format!(
                "a channel keeps {MOST} documents at most: take one away before another is added"
            ))),
        }
    }
}
