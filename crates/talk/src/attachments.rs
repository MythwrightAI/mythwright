//! What is attached to messages: pictures, documents, anything.
//!
//! Each is a file kept beside the database under an id of its own, with its
//! name, its kind and whose it is in the database. It is a file, and not a
//! row, because whoever reads the message may need to read it too: an agent
//! is given the path, and opens it like any other file.
//!
//! Something is attached in two steps, because it arrives before the
//! message it belongs to: it is put here by whoever is writing, and claimed
//! by the message when that is posted. What was put here and never claimed
//! is taken away the next time the daemon starts.
//!
//! **A file of a project is posted as that file.** Where what somebody
//! attaches is a file of a project they may read, the row names it by the
//! project and its path (`project`, `path`) and keeps no copy of it:
//! nothing is written beside the database, and nothing of the file is
//! kept (its size is nought here). This crate knows no project's files,
//! so what such a file is now (there, gone, or kept from whoever reads)
//! and how big it is are read above it, as whoever reads the message may
//! read that file (`api::posted`). A copy is made a file of a project the
//! same way, once it is written there ([`crate::posted`]).

use std::collections::HashMap;
use std::path::PathBuf;

use journal::Journal;
use kernel::{Error, Principal, Result};
use rusqlite::params;
use serde::Serialize;

use crate::kept::Reader;

pub(crate) const SCHEMA: &[&str] = &[
    "
    CREATE TABLE attachments (
        id TEXT PRIMARY KEY,
        channel TEXT NOT NULL,
        -- The message it is attached to; none until that message is posted.
        message TEXT,
        name TEXT NOT NULL,
        media TEXT NOT NULL,
        size INTEGER NOT NULL,
        actor TEXT NOT NULL,
        created INTEGER NOT NULL
    );
    CREATE INDEX attachments_message ON attachments (message);
",
    "
    -- A file of a project posted as that file: which project, and its
    -- path from the top of it. None for something kept here as a copy.
    ALTER TABLE attachments ADD COLUMN project TEXT;
    ALTER TABLE attachments ADD COLUMN path TEXT;
",
];

/// What a view is: an attachment that is a page, drawn where it is posted.
pub const VIEW: &str = "text/html";

/// The largest thing that can be attached.
pub const MAX_ATTACHMENT_BYTES: usize = 32 * 1024 * 1024;
/// How many things one message can carry.
pub const MAX_PER_MESSAGE: usize = 10;
/// How long something put here may wait for its message before it is
/// taken away at the next start.
const UNCLAIMED_FOR_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Attachment {
    pub id: String,
    /// The name it had where it came from.
    pub name: String,
    /// What kind of thing it is, as a media type: `image/png`.
    pub media: String,
    pub size: u64,
    /// Where it is a file of a project posted as that file: which, and
    /// what it is as whoever reads it may read it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<Posted>,
}

/// A file of a project that something attached names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Posted {
    pub project: String,
    /// Its file from the top of the project, with `/` between its names.
    pub path: String,
    /// What it is now, as whoever it is handed to may read it, where that
    /// was looked at: `there`, `gone`, or `kept` from them. Never kept.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<&'static str>,
}

/// What is read of an attachment's row.
pub(crate) const COLUMNS: &str = "id, name, media, size, project, path";

#[derive(Clone)]
pub struct Attachments {
    journal: Journal,
    dir: PathBuf,
}

/// What kind of thing a file is, by the end of its name, for one that
/// arrived without saying.
pub fn media_of(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        // A page: shown in the channel as one, in a box it cannot get out of.
        "html" | "htm" => VIEW,
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "txt" | "log" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "json" => "application/json",
        "csv" => "text/csv",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

/// A name as it is kept: the last part of a path, without what could not
/// be part of a file's name.
pub fn plain_name(name: &str) -> String {
    let last = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let kept: String = last.chars().filter(|c| !c.is_control()).take(200).collect();
    let kept = kept.trim().to_string();
    if kept.is_empty() || kept == "." || kept == ".." { "attachment".to_string() } else { kept }
}

impl Attachments {
    pub async fn open(journal: Journal, dir: impl Into<PathBuf>) -> Result<Attachments> {
        journal.db().migrate("attachments", SCHEMA).await?;
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        let attachments = Attachments { journal, dir };
        attachments.sweep().await?;
        Ok(attachments)
    }

    /// Where the file of an attachment is.
    pub fn path(&self, id: &str) -> PathBuf {
        self.dir.join(id)
    }

    /// Takes away what was put here long ago for a message that never came.
    async fn sweep(&self) -> Result<()> {
        let before = self.journal.now_ms() - UNCLAIMED_FOR_MS;
        let stale: Vec<String> = self
            .journal
            .db()
            .call(move |connection| {
                let ids: Vec<String> = connection
                    .prepare("SELECT id FROM attachments WHERE message IS NULL AND created < ?1")?
                    .query_map([before], |row| row.get(0))?
                    .collect::<rusqlite::Result<_>>()?;
                connection
                    .execute("DELETE FROM attachments WHERE message IS NULL AND created < ?1", [before])?;
                Ok(ids)
            })
            .await?;
        for id in stale {
            let _ = std::fs::remove_file(self.path(&id));
        }
        Ok(())
    }

    /// Takes away everything put in `channel`, attached to a message or
    /// not, and its files with it: for a channel that is being removed.
    pub(crate) async fn forget_channel(&self, channel: &str) -> Result<usize> {
        let channel = channel.to_string();
        let gone: Vec<String> = self
            .journal
            .db()
            .call(move |connection| {
                let ids: Vec<String> = connection
                    .prepare("SELECT id FROM attachments WHERE channel = ?1")?
                    .query_map([&channel], |row| row.get(0))?
                    .collect::<rusqlite::Result<_>>()?;
                connection.execute("DELETE FROM attachments WHERE channel = ?1", [&channel])?;
                Ok(ids)
            })
            .await?;
        for id in &gone {
            let _ = std::fs::remove_file(self.path(id));
        }
        Ok(gone.len())
    }

    /// Keeps a file for `by` to attach to a message in `channel`, which is
    /// the channel's id and the one `reader` is leave to write in. `media`
    /// is what it said it was, if it said.
    pub async fn add(
        &self,
        reader: &Reader,
        channel: &str,
        by: &Principal,
        name: &str,
        media: Option<&str>,
        bytes: Vec<u8>,
    ) -> Result<Attachment> {
        by.require(crate::kept::WRITE)?;
        self.add_as(reader, channel, &by.actor(), name, media, bytes).await
    }

    /// The same for whoever is speaking, when that is not somebody signed
    /// in: a bot, or a blueprint giving notice.
    pub async fn add_as(
        &self,
        reader: &Reader,
        channel: &str,
        actor: &str,
        name: &str,
        media: Option<&str>,
        bytes: Vec<u8>,
    ) -> Result<Attachment> {
        reader.writing()?;
        if !reader.holds(channel) {
            return Err(crate::kept::missing(channel));
        }
        if bytes.is_empty() {
            return Err(Error::invalid("there is nothing in that file to attach"));
        }
        if bytes.len() > MAX_ATTACHMENT_BYTES {
            return Err(Error::invalid(format!(
                "that is too large to attach: the most is {} MB. Put it in the project and say where it is",
                MAX_ATTACHMENT_BYTES / (1024 * 1024)
            )));
        }
        let name = plain_name(name);
        // What it says it is, without how it is written down (`; charset=...`).
        let said = media
            .map(|media| media.split(';').next().unwrap_or_default().trim())
            .filter(|media| !media.is_empty() && *media != "application/octet-stream");
        let attachment = Attachment {
            id: self.journal.new_id("att"),
            media: said.map_or_else(|| media_of(&name).to_string(), str::to_string),
            size: bytes.len() as u64,
            name,
            file: None,
        };
        std::fs::write(self.path(&attachment.id), bytes)?;
        let (row, channel, actor, now) =
            (attachment.clone(), channel.to_string(), actor.to_string(), self.journal.now_ms());
        let kept = self
            .journal
            .db()
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO attachments (id, channel, name, media, size, actor, created)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![row.id, channel, row.name, row.media, row.size as i64, actor, now],
                )
            })
            .await;
        if let Err(error) = kept {
            let _ = std::fs::remove_file(self.path(&attachment.id));
            return Err(error);
        }
        Ok(attachment)
    }

    /// Keeps a file of `project` at `path` for `actor` to attach to a
    /// message in `channel`, as that file: by its name, and nothing of
    /// what is in it. Whether it is one and whether they may read it is
    /// asked above this crate, which knows no project's files, before it
    /// is asked this.
    pub async fn add_file(
        &self,
        reader: &Reader,
        channel: &str,
        actor: &str,
        project: &str,
        path: &str,
    ) -> Result<Attachment> {
        reader.writing()?;
        if !reader.holds(channel) {
            return Err(crate::kept::missing(channel));
        }
        let crate::docs::Doc::File { project, path } = crate::docs::Doc::file(project, path)? else {
            return Err(Error::invalid("that is not a file of a project"));
        };
        let name = plain_name(&path);
        let attachment = Attachment {
            id: self.journal.new_id("att"),
            media: media_of(&name).to_string(),
            size: 0,
            name,
            file: Some(Posted { project, path, state: None }),
        };
        let (row, channel, actor, now) =
            (attachment.clone(), channel.to_string(), actor.to_string(), self.journal.now_ms());
        self.journal
            .db()
            .call(move |connection| {
                let file = row.file.as_ref().expect("a file");
                connection.execute(
                    "INSERT INTO attachments (id, channel, name, media, size, actor, created, project, path)
                     VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8)",
                    params![row.id, channel, row.name, row.media, actor, now, file.project, file.path],
                )
            })
            .await?;
        Ok(attachment)
    }

    /// Whether a message may carry these: no more than a message can.
    pub(crate) fn may_carry(ids: &[String]) -> Result<()> {
        if ids.len() > MAX_PER_MESSAGE {
            return Err(Error::invalid(format!("a message can carry {MAX_PER_MESSAGE} attachments at most")));
        }
        Ok(())
    }

    /// What is said where something is not whoever's to attach.
    pub(crate) fn not_theirs() -> Error {
        Error::invalid(
            "one of those is not something you attached here, or is part of a message already; attach it again",
        )
    }

    /// Makes what `actor` put here for `channel` part of `message`, in
    /// the write that keeps the message: all of it, or nothing where
    /// something is not theirs to attach here or is part of a message
    /// already, and then nothing has been written. It is looked at
    /// before any of it is changed, since whoever writes the message
    /// keeps what was written here with it.
    pub(crate) fn claim_in(
        connection: &rusqlite::Connection,
        ids: &[String],
        channel: &str,
        actor: &str,
        message: &str,
    ) -> rusqlite::Result<Option<Vec<Attachment>>> {
        let free = "SELECT count(*) FROM attachments
                    WHERE id = ?1 AND channel = ?2 AND actor = ?3 AND message IS NULL";
        let distinct: std::collections::BTreeSet<&String> = ids.iter().collect();
        if distinct.len() != ids.len() {
            return Ok(None);
        }
        for id in ids {
            if connection.query_row(free, params![id, channel, actor], |row| row.get::<_, i64>(0))? != 1 {
                return Ok(None);
            }
        }
        let mut claimed = Vec::new();
        for id in ids {
            connection.execute("UPDATE attachments SET message = ?1 WHERE id = ?2", params![message, id])?;
            claimed.push(connection.query_row(
                &format!("SELECT {COLUMNS} FROM attachments WHERE id = ?1"),
                [id],
                read,
            )?);
        }
        Ok(Some(claimed))
    }

    /// What is attached to each of these messages, in the order it was
    /// attached: for whoever hands the messages out, which asked.
    pub(crate) async fn of(&self, messages: &[String]) -> Result<HashMap<String, Vec<Attachment>>> {
        if messages.is_empty() {
            return Ok(HashMap::new());
        }
        let messages = messages.to_vec();
        self.journal
            .db()
            .call(move |connection| {
                let mut found: HashMap<String, Vec<Attachment>> = HashMap::new();
                let mut statement = connection.prepare(&format!(
                    "SELECT {COLUMNS} FROM attachments WHERE message = ?1 ORDER BY rowid"
                ))?;
                for message in messages {
                    let attached: Vec<Attachment> =
                        statement.query_map([&message], read)?.collect::<rusqlite::Result<_>>()?;
                    if !attached.is_empty() {
                        found.insert(message, attached);
                    }
                }
                Ok(found)
            })
            .await
    }

    /// An attachment, the channel it is in, and where its file is: for
    /// [`crate::Channels::attached`], which asks who reads that channel.
    pub(crate) async fn get(&self, id: &str) -> Result<(Attachment, String, PathBuf)> {
        let lookup = id.to_string();
        let found = self
            .journal
            .db()
            .call(move |connection| {
                journal::optional(connection.query_row(
                    &format!("SELECT {COLUMNS}, channel FROM attachments WHERE id = ?1"),
                    [&lookup],
                    |row| Ok((read(row)?, row.get::<_, String>(6)?)),
                ))
            })
            .await?;
        let (attachment, channel) = found.ok_or_else(|| crate::kept::no_attachment(id))?;
        let path = self.path(&attachment.id);
        Ok((attachment, channel, path))
    }
}

pub(crate) fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Attachment> {
    let (project, path) = (row.get::<_, Option<String>>(4)?, row.get::<_, Option<String>>(5)?);
    Ok(Attachment {
        id: row.get(0)?,
        name: row.get(1)?,
        media: row.get(2)?,
        size: row.get::<_, i64>(3)? as u64,
        file: project.zip(path).map(|(project, path)| Posted { project, path, state: None }),
    })
}
