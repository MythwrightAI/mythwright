//! Something attached as a copy made a file of a project: once the copy
//! is written among a project's files (above this crate, through the door
//! that writes them, which asks `projects:write`), the attachment is
//! changed to name that file, in one write with what is said of it
//! (`channel.attachment.placed`), said as anything of the channel is, so
//! that the message names the file from then on, for everybody who reads
//! it, and its copy here goes.
//!
//! The file is written first and the row changed after it: a daemon
//! stopped between the two leaves the file written and the message
//! carrying its copy as it did, which is how it was before anybody asked,
//! and asked again the file is there already and is said so.

use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde_json::json;

use crate::attachments::{Attachment, COLUMNS, Posted, read};
use crate::channels::Channels;
use crate::kept::Reader;

impl Channels {
    /// The channel the attachment `id` was put in, for `by`, who reads it:
    /// one of a channel kept from them is not there, as
    /// [`Channels::attached`] answers.
    pub async fn where_attached(&self, id: &str, by: &Principal) -> Result<String> {
        by.require(crate::kept::READ)?;
        let (_, channel, _) = self.attachments.get(id).await?;
        match self.found(&channel, by).await {
            Ok(found) => Ok(found.id),
            Err(error) if error.kind == kernel::ErrorKind::NotFound => Err(crate::kept::no_attachment(id)),
            Err(error) => Err(error),
        }
    }

    /// The attachment `id`, in the channel `reader` is leave to read, and
    /// which message carries it: refused as one that is not there where it
    /// is in another channel, or is part of no message yet.
    pub async fn attachment_in(&self, reader: &Reader, id: &str) -> Result<(Attachment, String)> {
        let (found, channel, _) = self.attachments.get(id).await?;
        if !reader.holds(&channel) {
            return Err(crate::kept::no_attachment(id));
        }
        let lookup = id.to_string();
        let message: Option<String> = self
            .journal
            .db()
            .call(move |connection| {
                connection
                    .query_row("SELECT message FROM attachments WHERE id = ?1", [&lookup], |row| row.get(0))
            })
            .await?;
        let message = message.ok_or_else(|| crate::kept::no_attachment(id))?;
        Ok((found, message))
    }

    /// Makes the attachment `id`, a copy in the channel `reader` is leave
    /// to write in, name the file of `project` at `path`, which has been
    /// written with what the copy holds, as `by`; and then takes the copy
    /// away. Refused where it names a file already, or the channel is
    /// archived as the write finds it.
    pub async fn placed_file(
        &self,
        reader: &Reader,
        id: &str,
        project: &str,
        path: &str,
        by: &Principal,
    ) -> Result<Attachment> {
        reader.writing()?;
        let (found, message) = self.attachment_in(reader, id).await?;
        if found.file.is_some() {
            return Err(already(&found));
        }
        let crate::docs::Doc::File { project, path } = crate::docs::Doc::file(project, path)? else {
            return Err(Error::invalid("that is not a file of a project"));
        };
        let (_, channel_id, _) = self.attachments.get(id).await?;
        let channel = self.get(reader, &channel_id).await?;
        let (lookup, by_now) = (id.to_string(), by.clone());
        let wrote = self.journal.write(move |writing| {
            let channel = crate::door::as_it_is(writing.transaction(), channel)?;
            if channel.archived.is_some() {
                return Ok(Err(crate::archived::refused(&channel)));
            }
            let transaction = writing.transaction();
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            let changed = transaction.execute(
                "UPDATE attachments SET project = ?1, path = ?2, name = ?3, size = 0
                 WHERE id = ?4 AND project IS NULL",
                params![project, path, name, lookup],
            )?;
            if changed != 1 {
                return Ok(Err(Error::conflict("that attachment names a file of a project already")));
            }
            let now = transaction.query_row(
                &format!("SELECT {COLUMNS} FROM attachments WHERE id = ?1"),
                [&lookup],
                read,
            )?;
            let mut data = json!({
                "channel": channel.id, "name": channel.name, "message": message, "attachment": now.id,
                "project": project, "path": path, "file_name": now.name,
                "by": by_now.actor(), "by_name": by_now.name,
            });
            if let Some(title) = &channel.title {
                data["title"] = json!(title);
            }
            let source = Source::actor(by_now.actor()).taking(channel.takes(&[]).as_deref());
            writing.say_here("channel.attachment.placed", source, data)?;
            Ok(Ok(now))
        });
        let placed = wrote.await?.0?;
        // The copy is nobody's now: the message names the file.
        let _ = std::fs::remove_file(self.attachments.path(&placed.id));
        Ok(placed)
    }
}

/// What is said where an attachment is a file of a project already.
fn already(found: &Attachment) -> Error {
    let Posted { project, path, .. } = found.file.as_ref().expect("a file");
    Error::conflict(format!(
        "`{}` is the file `{path}` of `{project}` already, and is not added to a project again",
        found.name
    ))
}
