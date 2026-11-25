//! An archived channel: read by whoever read it, written in by nobody,
//! and given back.
//!
//! **What it is** is kept on the channel's row (`channels.archived`:
//! when, and by whom), and archiving and giving it back are each one
//! write with what is said of it (`channel.archived`,
//! `channel.unarchived`), said as anything of the channel is: to whoever
//! may read channels, and of one kept to its members to its members.
//! It lets nobody in and keeps nobody out: whoever read it reads it,
//! finds it and searches it as before, and a channel kept to its
//! members stays so.
//!
//! **Nobody says anything there.** Every way a message is said comes to
//! one write ([`Channels::saying`]), which reads the channel's row first
//! and refuses where it is archived, in a sentence that says so and how
//! it is given back ([`refused`]), with nothing written: a person, an
//! agent, the factory's bot and a blueprint's step are each refused it
//! there, whoever read the channel a moment before. The bot is asked
//! before it is put in a channel by what it says
//! ([`Channels::post_speaker`]).
//!
//! **What is asked without a database.** Whether a channel is archived
//! is also held in memory, filled from the tables as channels are opened
//! and changed after each write, for what is asked as an agent is given
//! work ([`Channels::is_archived`]): no agent is asked there, and no
//! turn is queued that would answer there. It may say less than the
//! table for a moment, and the write that says a message is what holds.
//!
//! **What is counted.** A line of somebody's box whose place is an
//! archived channel stays, opens it, and is counted among what waits
//! unread no longer, and nothing raises it again ([`crate::inbox`]).
//!
//! Who may archive a channel and give it back is whoever may write in
//! it ([`Channels::may_write`]), which for one kept to its members is
//! one of them, or the factory's owner, as its door says; what is
//! running that posts there is asked about by the route, as taking a
//! channel away is, and whether an agent is asked something there by
//! whoever keeps agents (`Agents::archive_channel`), as keeping one is. Taking a channel away stays the one thing that
//! takes it away for good, and an archived one is taken away like any.

use kernel::{Error, Principal, Result, Source};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::channels::{Channel, Channels};
use crate::door::{as_it_is, called};
use crate::kept::Reader;

/// When a channel was archived, and by whom.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Archived {
    /// When, in milliseconds.
    pub at: i64,
    /// Who, as an actor.
    pub by: String,
    /// Who, by name.
    pub name: String,
}

/// What the column holds, as it is read: nothing where it is not
/// archived, or where what it holds cannot be read, which is then taken
/// for archived by nobody known.
pub(crate) fn read(kept: Option<String>) -> Option<Archived> {
    let kept = kept?;
    Some(serde_json::from_str(&kept).unwrap_or(Archived { at: 0, by: String::new(), name: String::new() }))
}

/// What is written of it in the column.
pub(crate) fn written(archived: Option<&Archived>) -> Option<String> {
    archived.map(|archived| json!(archived).to_string())
}

/// What saying something in the archived `channel` is refused with.
pub fn refused(channel: &Channel) -> Error {
    Error::conflict(format!(
        "{} is archived, and nothing is said in an archived channel: whoever may write in it gives it back first \
         (`{} channel unarchive {}`)",
        called(channel),
        brand::CLI,
        channel.name
    ))
}

/// What is said of a channel archived or given back.
fn said_of(channel: &Channel, by: &Principal) -> serde_json::Value {
    let mut data = json!({ "channel": channel.id, "name": channel.name, "by": by.actor() });
    if let Some(archived) = &channel.archived {
        data["at"] = json!(archived.at);
        data["by_name"] = json!(archived.name);
    }
    for (key, said) in [("title", channel.title.as_deref()), ("project", channel.project.as_deref())] {
        if let Some(said) = said {
            data[key] = json!(said);
        }
    }
    data
}

impl Channels {
    /// Archives the channel `reader` is leave to write in, as `by`: it is
    /// read as it always was and written in by nobody, until it is given
    /// back. One that is archived already is as it was, and nothing is
    /// said.
    pub async fn archive(&self, reader: &Reader, channel: &str, by: &Principal) -> Result<Channel> {
        let archived = Archived { at: self.journal.now_ms(), by: by.actor(), name: by.name.clone() };
        self.set_archived(reader, channel, Some(archived), by).await
    }

    /// Gives back the archived channel `reader` is leave to write in, as
    /// `by`: as it was, with everything it held. One that is not archived
    /// is as it was, and nothing is said.
    pub async fn unarchive(&self, reader: &Reader, channel: &str, by: &Principal) -> Result<Channel> {
        self.set_archived(reader, channel, None, by).await
    }

    async fn set_archived(
        &self,
        reader: &Reader,
        channel: &str,
        archived: Option<Archived>,
        by: &Principal,
    ) -> Result<Channel> {
        reader.writing()?;
        let found = self.get(reader, channel).await?;
        if found.archived.is_some() == archived.is_some() {
            return Ok(found);
        }
        let (row, by) = (found.clone(), by.clone());
        let wrote = self.journal.write(move |writing| {
            let changed = writing.transaction().execute(
                match archived {
                    Some(_) => "UPDATE channels SET archived = ?2 WHERE id = ?1 AND archived IS NULL",
                    None => "UPDATE channels SET archived = ?2 WHERE id = ?1 AND archived IS NOT NULL",
                },
                rusqlite::params![row.id, written(archived.as_ref())],
            )?;
            // As the write finds it: kept, or called something else, since it was read.
            let now = as_it_is(writing.transaction(), row)?;
            let source = Source::actor(by.actor()).taking(now.takes(&[]).as_deref());
            match (changed, now.archived.is_some()) {
                (0, _) => {}
                (_, true) => writing.say_here("channel.archived", source, said_of(&now, &by)).map(|_| ())?,
                (_, false) => {
                    writing.say_here("channel.unarchived", source, said_of(&now, &by)).map(|_| ())?
                }
            }
            Ok(now)
        });
        let (now, _) = wrote.await?;
        // After the write: until it, the table said what the memory did.
        self.memory.archived_as(&now);
        Ok(now)
    }

    /// Whether the channel `named`, by its id or its name, is archived,
    /// as the memory has it: for what is asked as an agent is given work,
    /// and never an answer to anybody. Nothing is asked of the database.
    pub fn is_archived(&self, named: &str) -> bool {
        let named = named.trim_start_matches('#');
        let archived = self.memory.archived.read().expect("members lock");
        !archived.is_empty() && (archived.contains_key(named) || archived.values().any(|name| name == named))
    }
}

impl crate::kept::Memory {
    /// What the memory holds of whether `channel` is archived, as it now
    /// is: by its id, with the name it has now.
    pub(crate) fn archived_as(&self, channel: &Channel) {
        let mut archived = self.archived.write().expect("members lock");
        match channel.archived {
            Some(_) => archived.insert(channel.id.clone(), channel.name.clone()),
            None => archived.remove(&channel.id),
        };
    }
}
