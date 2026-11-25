//! Reading what was said: a message, a thread, a channel's messages,
//! and what a step of a run said already. Each is handed the leave it is
//! read by ([`Reader`]), and hands out nothing of a channel that is no
//! leave for.

use std::collections::{HashMap, HashSet};

use journal::optional;
use kernel::{Principal, Result, Source};
use rusqlite::params;
use serde_json::json;

use crate::channels::{Channel, Channels, Message, said_of};
use crate::kept::{Reader, no_message};
use crate::speaker::SaidBy;

pub(crate) fn read_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    read_from(row, 0)
}

/// A message read from a row whose first `skip` columns are something else.
fn read_from(row: &rusqlite::Row<'_>, skip: usize) -> rusqlite::Result<Message> {
    let said_by: Option<String> = row.get(skip + 9)?;
    Ok(Message {
        id: row.get(skip)?,
        channel: row.get(skip + 1)?,
        author: row.get(skip + 2)?,
        author_name: row.get(skip + 3)?,
        body: row.get(skip + 4)?,
        reply_to: row.get(skip + 5)?,
        created: row.get(skip + 6)?,
        model: row.get(skip + 7)?,
        thread: row.get(skip + 8)?,
        said_by: said_by.and_then(|said| serde_json::from_str(&said).ok()),
        mark: row.get(skip + 10)?,
        replies: row.get::<_, i64>(skip + 11)? as u32,
        // Read separately, for all the messages asked for at once.
        attachments: Vec::new(),
    })
}

/// The same, read back, with how many messages are under it.
const MESSAGE: &str = "id, channel, author, author_name, body, reply_to, created, model, thread, said_by, mark, \
                       (SELECT COUNT(*) FROM messages AS under WHERE under.thread = messages.id)";

impl Channels {
    /// Gives each of these messages what is attached to it.
    async fn with_attached(&self, mut messages: Vec<Message>) -> Result<Vec<Message>> {
        let ids: Vec<String> = messages.iter().map(|message| message.id.clone()).collect();
        let mut found = self.attachments.of(&ids).await?;
        for message in &mut messages {
            message.attachments = found.remove(&message.id).unwrap_or_default();
        }
        Ok(messages)
    }

    /// Says that a message was said: one that a journal from before
    /// holds written and never said ([`Self::said_already`]).
    async fn announce(&self, channel: &Channel, message: &Message) {
        let source = Source::actor(&message.author).taking(channel.takes(&[]).as_deref());
        let _ = self.journal.emit_here("channel.message", source, said_of(channel, message)).await;
    }

    /// The message a step of a run already said, for a step that is being
    /// done again because the daemon stopped in the middle of it: so that
    /// what it says is said once. A message and the saying of it are one
    /// write, so one that is there was said; a journal a program from
    /// before left may hold one written and never announced, a stop
    /// having come between the two, and that one is announced now. One
    /// said in a channel `reader` is no leave for is not found.
    pub async fn said_already(&self, reader: &Reader, by: &SaidBy) -> Result<Option<Message>> {
        // A run is known by an id nothing else has: what names it in what
        // was kept is that run's, and which step and which element is read.
        let holds = format!("{}:{}", json!("run"), json!(by.run));
        let said: Vec<Message> = self
            .journal
            .db()
            .call(move |connection| {
                let mut statement = connection.prepare(&format!(
                    "SELECT {MESSAGE} FROM messages WHERE said_by IS NOT NULL AND instr(said_by, ?1) > 0 \
                     ORDER BY rowid"
                ))?;
                let rows = statement.query_map([&holds], read_message)?;
                rows.collect()
            })
            .await?;
        // And which of the things done under that step's name: its own
        // action or an entry of what is done around it, at which try and
        // for which of a list. One a program from before this kept says
        // only which element, and is told by that.
        let same = |kept: &SaidBy| {
            kept.run == by.run
                && kept.step == by.step
                && match &kept.doing {
                    Some(doing) => Some(doing) == by.doing.as_ref(),
                    None => kept.at == by.at,
                }
        };
        let theirs =
            |message: &Message| reader.holds(&message.channel) && message.said_by.as_ref().is_some_and(same);
        let Some(found) = said.into_iter().find(theirs) else {
            return Ok(None);
        };
        if !self.journal.said_in_any("channel.message", "id", &found.id).await? {
            let channel = self.row(&found.channel).await?;
            self.announce(&channel, &found).await;
        }
        Ok(Some(self.with_attached(vec![found]).await?.remove(0)))
    }

    /// What the factory's bot said lately, under the name it has now,
    /// newest first and `most` at most: only in a channel `by` reads, so
    /// that what is handed is what reading each channel would hand them;
    /// nothing for somebody who reads no channel. Read a page at a time
    /// from the newest, for whoever reads about the bot.
    pub async fn said_by_the_bot(&self, by: &Principal, most: usize) -> Result<Vec<Message>> {
        if !by.can(crate::kept::READ) {
            return Ok(Vec::new());
        }
        let reads: HashSet<String> = self.readable(by).await?.into_iter().map(|channel| channel.id).collect();
        let author = crate::channels::bot_actor(&self.speaker.name());
        let (mut said, mut before) = (Vec::new(), i64::MAX);
        while said.len() < most {
            let author = author.clone();
            let page: Vec<(i64, Message)> = self
                .journal
                .db()
                .call(move |connection| {
                    let mut statement = connection.prepare(&format!(
                        "SELECT rowid, {MESSAGE} FROM messages WHERE author = ?1 AND rowid < ?2 ORDER BY rowid DESC LIMIT 200"
                    ))?;
                    let rows = statement.query_map(params![author, before], |row| Ok((row.get(0)?, read_from(row, 1)?)));
                    rows?.collect()
                })
                .await?;
            let Some(last) = page.last().map(|(at, _)| *at) else { break };
            before = last;
            said.extend(
                page.into_iter()
                    .map(|(_, message)| message)
                    .filter(|message| reads.contains(&message.channel)),
            );
        }
        said.truncate(most);
        Ok(said)
    }

    /// Who answered a message in the channel itself, newest first.
    pub async fn answered_by(&self, reader: &Reader, message: &str) -> Result<Vec<String>> {
        let (message, within) = (message.to_string(), reader.channel().map(str::to_string));
        self.journal
            .db()
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT author FROM messages
                     WHERE reply_to = ?1 AND thread IS NULL AND (?2 IS NULL OR channel = ?2) ORDER BY rowid DESC",
                )?;
                let rows = statement.query_map(params![message, within], |row| row.get(0))?;
                rows.collect()
            })
            .await
    }

    /// A thread, oldest first: the message it hangs from, then what has
    /// been said under it. Any message of the thread names it, and one
    /// of a channel `reader` is no leave for names nothing.
    pub async fn thread(&self, reader: &Reader, message: &str) -> Result<Vec<Message>> {
        let found = self.message(reader, message).await?;
        let root = found.thread.unwrap_or(found.id);
        let said: Vec<Message> = self
            .journal
            .db()
            .call(move |connection| {
                let mut statement = connection.prepare(&format!(
                    "SELECT {MESSAGE} FROM messages WHERE id = ?1 OR thread = ?1 ORDER BY thread IS NOT NULL, rowid"
                ))?;
                let rows = statement.query_map([&root], read_message)?;
                rows.collect()
            })
            .await?;
        self.with_attached(said).await
    }

    /// Up to `limit` messages of the channel itself, oldest first: the most
    /// recent ones, or the ones just before message `before`. What was said
    /// in threads is not among them; each message says how much is under it.
    pub async fn messages(
        &self,
        reader: &Reader,
        channel: &str,
        before: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Message>> {
        let channel = self.get(reader, channel).await?;
        let (id, before, limit) = (channel.id, before.map(str::to_string), limit.clamp(1, 500));
        let mut messages: Vec<Message> = self
            .journal
            .db()
            .call(move |connection| {
                let cutoff: i64 = match &before {
                    Some(before) => optional(connection.query_row(
                        "SELECT rowid FROM messages WHERE id = ?1 AND channel = ?2",
                        [before, &id],
                        |row| row.get(0),
                    ))?
                    .unwrap_or(i64::MAX),
                    None => i64::MAX,
                };
                let mut statement = connection.prepare(&format!(
                    "SELECT {MESSAGE} FROM messages
                     WHERE channel = ?1 AND rowid < ?2 AND thread IS NULL ORDER BY rowid DESC LIMIT {limit}"
                ))?;
                let rows = statement.query_map(params![id, cutoff], read_message)?;
                rows.collect()
            })
            .await?;
        messages.reverse();
        self.with_attached(messages).await
    }

    /// Every message `reader` is leave for, oldest first, a page at a
    /// time: for indexing, which is the daemon's own.
    pub async fn all_messages(
        &self,
        reader: &Reader,
        after_rowid: i64,
        limit: usize,
    ) -> Result<Vec<(i64, Message)>> {
        let within = reader.channel().map(str::to_string);
        self.journal
            .db()
            .call(move |connection| {
                let mut statement = connection.prepare(&format!(
                    "SELECT rowid, {MESSAGE} FROM messages
                     WHERE rowid > ?1 AND (?2 IS NULL OR channel = ?2) ORDER BY rowid LIMIT {limit}"
                ))?;
                let rows = statement
                    .query_map(params![after_rowid, within], |row| Ok((row.get(0)?, read_from(row, 1)?)))?;
                rows.collect()
            })
            .await
    }

    /// What was said in each of the messages `ids` name, by the
    /// message's id, in one reading of them all: for what lists many
    /// things that each name the message it came of (an agent's turns,
    /// each asked by one). A message that is gone, or is of a channel
    /// `reader` is no leave for, is not among them.
    pub async fn said_in(&self, reader: &Reader, ids: &[String]) -> Result<HashMap<String, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let asked = ids.to_vec();
        let found: Vec<(String, String, String)> = self
            .journal
            .db()
            .call(move |c| {
                let places = vec!["?"; asked.len()].join(", ");
                let mut statement =
                    c.prepare(&format!("SELECT id, channel, body FROM messages WHERE id IN ({places})"))?;
                let rows = statement.query_map(rusqlite::params_from_iter(asked.iter()), |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?;
                rows.collect()
            })
            .await?;
        Ok(found
            .into_iter()
            .filter(|(_, channel, _)| reader.holds(channel))
            .map(|(id, _, body)| (id, body))
            .collect())
    }

    /// A message by its id: one of a channel `reader` is no leave for is
    /// not there.
    pub async fn message(&self, reader: &Reader, id: &str) -> Result<Message> {
        let lookup = id.to_string();
        let found = self
            .journal
            .db()
            .call(move |c| {
                optional(c.query_row(
                    &format!("SELECT {MESSAGE} FROM messages WHERE id = ?1"),
                    [&lookup],
                    read_message,
                ))
            })
            .await?
            .filter(|found| reader.holds(&found.channel))
            .ok_or_else(|| no_message(id))?;
        Ok(self.with_attached(vec![found]).await?.remove(0))
    }
}
