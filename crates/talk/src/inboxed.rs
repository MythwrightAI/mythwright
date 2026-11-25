//! What the channels put in somebody's box, and a box as its owner is
//! shown it.
//!
//! **Named.** A message that names a person with `@` puts a line in
//! that person's box ([`crate::inbox`]), in the write that keeps the
//! message: both are there or neither. Who a word names is not this
//! crate's to know (an account is `people`'s), so it is asked of what
//! was installed ([`Channels::names_with`]) inside that write, and with
//! nothing installed nobody is named. Whoever is named is told only
//! where they read the channel as it is then, by the one question every
//! reading of a channel asks ([`crate::kept`]): the channels' scope,
//! and for a channel kept to its members being in it. Not whoever said
//! the message, and nothing where it names nobody: such a message costs
//! no reading and no line. An agent is not told here: one the factory
//! starts is given a turn by what is said to it, and one it does not
//! start is told by whoever keeps agents, through the same door
//! ([`crate::Inbox::tell`]).
//!
//! **A conversation.** What is said in a conversation
//! ([`crate::conversed`]) raises the one line each of its members has
//! for it, in the same write: of everybody in it but whoever said it,
//! a person and an agent the factory does not start, whatever they hold
//! just then (a line is passed over while its owner does not read the
//! conversation, and is theirs when they do). An agent the factory
//! starts is given a turn by what is said to it, and no line. Somebody
//! a message there names who is in it is told by that line and no
//! other. And somebody who is added to one has its line from the write
//! that brings them in ([`tell_joined`]), by the latest thing somebody
//! else said there.
//!
//! **Shown.** A line of something said keeps no words. Its owner is
//! shown the message as they may read it when they look
//! ([`Channels::inbox_of`]): whole, with what its channel is called,
//! where they read that channel now; and not at all where the message
//! is gone or the channel has been kept from them since, which is to
//! them a channel that is not there (the line is passed over:
//! [`crate::inbox`]).

use std::collections::HashMap;
use std::sync::Arc;

use kernel::{Principal, Result};
use serde::Serialize;

use crate::channels::{Channel, Channels, Member, Message, STORED, mentions};
use crate::inbox::{Inbox, Kind, Line, Tell};

/// What says who a word after an `@` names among the people with an
/// account, each as they are now with what they hold: asked in the
/// write that keeps a message, of that write's own reading.
pub type Naming = dyn Fn(&rusqlite::Connection, &[String]) -> rusqlite::Result<Vec<Principal>> + Send + Sync;

/// What a line of something said is shown with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Said {
    /// Who said it, as an actor and by name.
    pub author: String,
    pub author_name: String,
    pub body: String,
    /// The message it was said under, where it is part of a thread.
    pub thread: Option<String>,
    pub created: i64,
    /// What its channel is called: its name, and for one kept to its
    /// members its title.
    pub channel_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_title: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub channel_kept: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Whether its channel is a conversation, and then who is in it.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub conversation: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<Member>,
    /// Whether its channel is archived: the line opens it still, and
    /// waits unread for nobody.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub channel_archived: bool,
}

/// A line as its owner is shown it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Shown {
    #[serde(flatten)]
    pub line: Line,
    /// What was said, for a line of something said.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub said: Option<Said>,
    /// What was said, for a line of something said on a record; for one of
    /// a page somebody follows, the change that raised it last.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_record: Option<crate::commented::OnRecord>,
}

/// Tells whoever `message`, said in `channel` as it is in this write, is
/// for, each in their box: every member of a conversation but whoever
/// said it, and each person it names who reads the channel. How many
/// were told.
pub(crate) fn tell_of(
    writing: &mut journal::Writing<'_>,
    inbox: &Inbox,
    naming: Option<&Naming>,
    channel: &Channel,
    message: &Message,
) -> rusqlite::Result<usize> {
    let names = mentions(&message.body);
    let naming = naming.filter(|_| !names.is_empty());
    // A message that names nobody, in a channel that is no
    // conversation, costs no reading and no line.
    if naming.is_none() && !channel.conversation {
        return Ok(0);
    }
    let transaction = writing.transaction();
    // Who is in it, for one that is kept: read here, as the write finds it.
    let mut within: Vec<String> = Vec::new();
    if channel.kept {
        let mut members = transaction.prepare("SELECT member FROM channel_members WHERE channel = ?1")?;
        within = members.query_map([&channel.id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    }
    let people = match naming {
        Some(naming) => naming(transaction, &names)?,
        None => Vec::new(),
    };
    let mut told = 0;
    if channel.conversation {
        // Whoever is in it has it in their box whatever they hold just
        // now: it is theirs, and is passed over while they do not read it.
        let boxed = |actor: &&String| match crate::channels::kind_of(actor) {
            "user" => true,
            "agent" => inbox.memory.outside(actor),
            _ => false,
        };
        for actor in within.iter().filter(|actor| **actor != message.author).filter(boxed) {
            let tell = Tell::conversed(actor, &channel.id, &message.id, &message.author);
            told += usize::from(inbox.tell_in(writing, &tell, message.created)?.is_some());
        }
    }
    for person in people {
        let actor = person.actor();
        let member = within.contains(&actor);
        let reads = inbox.memory.reads_as(channel.kept, member, &actor, &person.scopes);
        // Somebody in a conversation is told by its line, and no other.
        if actor == message.author || !reads || (channel.conversation && member) {
            continue;
        }
        let tell = Tell::named(&actor, &message.id, &channel.id, &message.author);
        told += usize::from(inbox.tell_in(writing, &tell, message.created)?.is_some());
    }
    Ok(told)
}

/// Puts the conversation `channel` in the box of `actor`, who has just
/// come into it, in the write that brings them in: by the latest thing
/// somebody else said there, so that whoever is added to one finds it
/// where everybody it was written to did. Nothing where the channel is
/// no conversation, where `actor` has no box of this, or where the line
/// they had of it was told by that very message.
pub(crate) fn tell_joined(
    writing: &mut journal::Writing<'_>,
    inbox: &Inbox,
    channel: &Channel,
    actor: &str,
    now: i64,
) -> rusqlite::Result<()> {
    let boxed = match crate::channels::kind_of(actor) {
        "user" => true,
        "agent" => inbox.memory.outside(actor),
        _ => false,
    };
    if !channel.conversation || !boxed {
        return Ok(());
    }
    let latest: Option<(String, String)> = journal::optional(writing.transaction().query_row(
        "SELECT id, author FROM messages WHERE channel = ?1 AND author != ?2 ORDER BY created DESC, rowid DESC LIMIT 1",
        [&channel.id, actor],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ))?;
    if let Some((message, author)) = latest {
        inbox.tell_in(writing, &Tell::conversed(actor, &channel.id, &message, &author), now)?;
    }
    Ok(())
}

/// The message a line of something said is shown with, by its id: what
/// it is about for somebody named, and for a conversation the one that
/// raised it last.
fn message_of(line: &Line) -> Option<&str> {
    match line.kind {
        Kind::Named => Some(&line.about),
        Kind::Message => line.said_by.as_deref(),
        Kind::Told | Kind::Commented | Kind::Followed | Kind::Following => None,
    }
}

impl Channels {
    /// Says what answers who a word after an `@` names among the people
    /// with an account, for every keeper of these channels. Said once; a
    /// second is not taken.
    pub fn names_with(&self, naming: Arc<Naming>) {
        let _ = self.naming.set(naming);
    }

    /// `by`'s own box as they are shown it, the latest told first, and
    /// how many of its lines wait unread: each line of something said
    /// with the message as they may read it now. With `all`, what was
    /// put away too.
    pub async fn inbox_of(&self, by: &Principal, all: bool) -> Result<(Vec<Shown>, u64)> {
        let lines = self.inbox.list(by, all).await?;
        let unread = self.inbox.unread(by).await?;
        Ok((self.shown_to(by, lines).await?, unread))
    }

    /// One line of `by`'s own box as they are shown it, put away or not:
    /// one of something said that they no longer read, or whose message
    /// is gone, answers as a line that is not there, as one that is
    /// somebody else's does.
    pub async fn inbox_line(&self, by: &Principal, id: &str) -> Result<Shown> {
        let line = self.inbox.one(by, id).await?;
        let shown = self.shown_to(by, vec![line]).await?;
        shown.into_iter().next().ok_or_else(|| crate::inbox::no_line(id))
    }

    /// Each of these lines of `by`'s box with the message it is of as
    /// they may read it now; one whose message they may not read, or is
    /// gone, is left out.
    async fn shown_to(&self, by: &Principal, lines: Vec<Line>) -> Result<Vec<Shown>> {
        let ids: Vec<String> = lines.iter().filter_map(message_of).map(str::to_string).collect();
        let said = self.said_for(&ids).await?;
        // Each channel is asked once, as a route asks it: the scope, and
        // for one that is kept the one question.
        let mut read: HashMap<String, Option<Channel>> = HashMap::new();
        let mut shown = Vec::with_capacity(lines.len());
        for line in lines {
            if matches!(line.kind, Kind::Commented | Kind::Followed) {
                // Its words are the record's history's, as they may read
                // the record now: one they may not is passed over. A line
                // of a page somebody follows is of the change that raised
                // it last.
                let of =
                    if line.kind == Kind::Followed { line.said_by.as_deref() } else { Some(&*line.about) };
                let seq = of.and_then(|of| of.parse::<u64>().ok()).unwrap_or_default();
                let on_record = match self.memory.records.get() {
                    Some(records) => match records.said(by.clone(), line.place.clone(), seq).await {
                        Some(said) => Some(said),
                        None => continue,
                    },
                    None => None,
                };
                shown.push(Shown { line, said: None, on_record });
                continue;
            }
            let Some(about) = message_of(&line) else {
                shown.push(Shown { line, said: None, on_record: None });
                continue;
            };
            // A message that is gone took what was said with it: the
            // line says nothing any more, and is passed over.
            let Some(message) = said.get(about).cloned() else { continue };
            if !read.contains_key(&message.channel) {
                let found = match by.can(crate::kept::READ) {
                    true => self.found(&message.channel, by).await.ok(),
                    false => None,
                };
                read.insert(message.channel.clone(), found);
            }
            // One of a channel they do not read is passed over too: to
            // them the channel is not there, and nothing says it is.
            let Some(channel) = read[&message.channel].as_ref() else { continue };
            let said = Said {
                author: message.author,
                author_name: message.author_name,
                body: message.body,
                thread: message.thread,
                created: message.created,
                channel_name: channel.name.clone(),
                channel_title: channel.title.clone(),
                channel_kept: channel.kept,
                project: channel.project.clone(),
                conversation: channel.conversation,
                members: if channel.conversation { channel.members.clone() } else { Vec::new() },
                channel_archived: channel.archived.is_some(),
            };
            shown.push(Shown { said: Some(said), line, on_record: None });
        }
        Ok(shown)
    }

    /// The messages `ids` name that are there, by id, as they are
    /// stored: for this crate, which then asks who each is there for.
    async fn said_for(&self, ids: &[String]) -> Result<HashMap<String, Message>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let asked = ids.to_vec();
        let found: Vec<Message> = self
            .journal
            .db()
            .call(move |connection| {
                let places = vec!["?"; asked.len()].join(", ");
                let mut statement = connection
                    .prepare(&format!("SELECT {STORED}, 0 FROM messages WHERE id IN ({places})"))?;
                let rows = statement
                    .query_map(rusqlite::params_from_iter(asked.iter()), crate::reading::read_message)?;
                rows.collect()
            })
            .await?;
        Ok(found.into_iter().map(|message| (message.id.clone(), message)).collect())
    }
}
