//! Channels: where people and agents talk.
//!
//! A message posted here is an event like any other, so an agent in a channel
//! is told about it rather than having to look, and a blueprint can react to
//! a message with an ordinary handler. Members are of three kinds: people,
//! agents, and bots. A bot is a blueprint taking part under a name of its
//! own: it is told of messages by a handler and answers with `channel.post`,
//! with no model behind it. What a blueprint says without naming a bot is
//! said by one bot, the factory's own ([`crate::speaker`]).
//!
//! The tables here (`channels`, `channel_members`, `messages`, and the
//! documents a channel keeps to hand, `channel_docs`) are made
//! by this crate, and written by it, but for one column: which project a
//! channel says (`channels.project`) is written after the channel is
//! made by `studio::workspaces` and by nothing else, with statements of
//! its own on this table: where somebody says what a channel belongs to
//! (`Workspaces::place_channel`), and where a channel's project is gone
//! and the channel is made a workspace's own (a project forgotten, a
//! factory settled). It stays there because a channel is a project's or
//! a workspace's, never both and never neither, and which workspace it
//! is of is a table of the studio's: the two are one transaction, which
//! this crate, below the studio, cannot open over the studio's table.
//! For the same reason a channel that is made for no project is given
//! its place by the studio, in this crate's own write ([`Placing`]). So that column is not to be renamed, or given a
//! meaning, here alone. The studio's suite holds it:
//! `workspaces::a_channel_belongs_to_a_project_or_to_a_workspace_and_never_to_both`
//! says where a channel belongs through the studio and reads it back
//! through [`Channels::get`].
//!
//! A channel may be kept to its members ([`crate::kept`]): nothing here
//! hands out a channel, a message or what is attached without leave to
//! read it ([`Reader`]), and what is said of a kept one is said as
//! taking being a member ([`Channel::takes`]).

use std::sync::Arc;

use journal::{Journal, optional};
use kernel::{Error, Principal, Result, Source};

use crate::attachments::{Attachment, Attachments};
use crate::kept::{Memory, Reader, missing, no_message};
use crate::speaker::{SaidBy, Speaker};
use rusqlite::params;
use serde::Serialize;
use serde_json::json;

pub(crate) const SCHEMA: &[&str] = &[
    "
    CREATE TABLE channels (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        project TEXT,
        topic TEXT NOT NULL,
        created INTEGER NOT NULL
    );

    CREATE TABLE channel_members (
        channel TEXT NOT NULL,
        member TEXT NOT NULL,
        name TEXT NOT NULL,
        joined INTEGER NOT NULL,
        PRIMARY KEY (channel, member)
    );

    CREATE TABLE messages (
        id TEXT PRIMARY KEY,
        channel TEXT NOT NULL,
        author TEXT NOT NULL,
        author_name TEXT NOT NULL,
        body TEXT NOT NULL,
        reply_to TEXT,
        created INTEGER NOT NULL
    );
    CREATE INDEX messages_channel ON messages (channel, created);
",
    "
    ALTER TABLE messages ADD COLUMN model TEXT;
",
    "
    ALTER TABLE messages ADD COLUMN thread TEXT;
    CREATE INDEX messages_thread ON messages (thread);
",
    "
    ALTER TABLE messages ADD COLUMN said_by TEXT;
",
    // Kept to its members, and what such a channel is called: its `name`
    // is then its own id, so that what it is called is not among the
    // names every channel shares, where making a channel would say that
    // one of that name is there.
    "
    ALTER TABLE channels ADD COLUMN kept INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE channels ADD COLUMN title TEXT;
",
    // Whether a kept channel's agents are asked nowhere else: for what
    // is most closely held, an agent that has read it is given no work
    // anywhere a word of it could follow. Not so unless said.
    "
    ALTER TABLE channels ADD COLUMN alone INTEGER NOT NULL DEFAULT 0;
",
    // Whether a kept channel is a conversation: a message written to
    // its members, listed in each one's inbox and not among the channels
    // ([`crate::conversed`]).
    "
    ALTER TABLE channels ADD COLUMN conversation INTEGER NOT NULL DEFAULT 0;
",
    // Archived, and when and by whom: read by whoever read it, written in
    // by nobody, until it is given back ([`crate::archived`]). Nothing
    // where it is not.
    "
    ALTER TABLE channels ADD COLUMN archived TEXT;
",
    // The documents a channel keeps to hand, the same for everybody in
    // it: a library's record or a project's file an entry, by what it
    // is said as ([`crate::docs`]).
    "
    CREATE TABLE channel_docs (
        channel TEXT NOT NULL,
        doc TEXT NOT NULL,
        by TEXT NOT NULL,
        added INTEGER NOT NULL,
        PRIMARY KEY (channel, doc)
    );
",
    // A message's mark, and the discussions of a channel's agents ([`crate::discussed`]).
    "
    ALTER TABLE messages ADD COLUMN mark TEXT;
    CREATE TABLE discussing (
        channel TEXT PRIMARY KEY,
        settings TEXT NOT NULL,
        by TEXT NOT NULL,
        instance TEXT NOT NULL,
        written INTEGER NOT NULL
    );
    CREATE TABLE discussions (
        root TEXT PRIMARY KEY,
        channel TEXT NOT NULL,
        status TEXT NOT NULL,
        why TEXT NOT NULL,
        agents TEXT NOT NULL,
        spoken TEXT NOT NULL,
        asker TEXT,
        began TEXT NOT NULL,
        stretch INTEGER NOT NULL,
        turns INTEGER NOT NULL,
        last TEXT NOT NULL,
        created INTEGER NOT NULL,
        updated INTEGER NOT NULL
    );
    CREATE INDEX discussions_channel ON discussions (channel);
",
    // Whether a member is asked by what is said there naming nobody:
    // everybody is, but an agent that is there to say what it is asked
    // there, which is asked only by its name ([`Member::listens`]).
    "
    ALTER TABLE channel_members ADD COLUMN listens INTEGER NOT NULL DEFAULT 1;
",
    // Whether a member manages the channel: changes what it is called and
    // what it says it is for, and who else manages it ([`crate::managed`]).
    // Whoever makes one is from here; of one made before, nobody is, and
    // whoever manages the factory is all that may.
    "
    ALTER TABLE channel_members ADD COLUMN manages INTEGER NOT NULL DEFAULT 0;
",
];

/// The longest message that can be posted.
const MAX_BODY: usize = 64 * 1024;
/// The longest thing a kept channel can be called.
pub(crate) const MAX_TITLE: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Channel {
    pub id: String,
    /// Lowercase words joined by dashes; unique. For a channel that is
    /// kept to its members, its own id: what it is called is `title`.
    pub name: String,
    pub project: Option<String>,
    pub topic: String,
    pub created: i64,
    pub members: Vec<Member>,
    /// Whether it is kept to its members: read by nobody else.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub kept: bool,
    /// What a kept channel is called, which another may be called too.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Whether its agents are asked nowhere else: said of a kept one.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub alone: bool,
    /// Whether it is a conversation ([`crate::conversed`]): a kept one
    /// that is listed in its members' inboxes and not among the channels.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub conversation: bool,
    /// When it was archived and by whom, where it is ([`crate::archived`]):
    /// read as it always was, and written in by nobody.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<crate::archived::Archived>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Member {
    /// `user:<id>`, `agent:<slot>` or `bot:<name>`.
    pub actor: String,
    pub name: String,
    /// Whether what is said there naming nobody may be for it: so of
    /// everybody, but an agent that is in the channel to say what it is
    /// asked there ([`Channels::bring_asked`]), which is asked there only
    /// by its name. Said only where it is not so.
    #[serde(skip_serializing_if = "listens")]
    pub listens: bool,
    /// Whether they manage the channel ([`crate::managed`]): only a
    /// person does. Said only where it is so.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub manages: bool,
}

impl Member {
    /// Somebody in a channel, as anybody is.
    pub fn new(actor: impl Into<String>, name: impl Into<String>) -> Member {
        Member { actor: actor.into(), name: name.into(), listens: true, manages: false }
    }
}

/// Whether a member listens, for leaving it unsaid where it does.
fn listens(listens: &bool) -> bool {
    *listens
}

/// The actor a bot called `name` takes part as.
pub fn bot_actor(name: &str) -> String {
    format!("bot:{}", blueprint::expr::slug(name))
}

/// What kind of actor one is: what stands before its `:` (`user`, `agent`,
/// `bot`), or the whole of one that has none (`system`). An actor says its
/// kind, and whatever asks who said a thing asks this and takes no name
/// apart to know.
pub fn kind_of(actor: &str) -> &str {
    actor.split_once(':').map_or(actor, |(kind, _)| kind)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Message {
    pub id: String,
    pub channel: String,
    /// `user:<id>`, `agent:<slot>`, `bot:<name>` or `system`.
    pub author: String,
    pub author_name: String,
    pub body: String,
    pub reply_to: Option<String>,
    /// For an agent's message: the model that wrote it, when that is known.
    pub model: Option<String>,
    /// The message this one is said under, when it is part of a thread. A
    /// thread is one message and what is said under it; it is one deep.
    pub thread: Option<String>,
    /// How many messages have been said under this one.
    pub replies: u32,
    /// What is attached to it: pictures, documents, anything.
    pub attachments: Vec<Attachment>,
    /// For one a blueprint's step said: which, and in which run. The name
    /// on it is the bot's it was said as; this is what was speaking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub said_by: Option<SaidBy>,
    /// What its author marked it as, of the marks there are
    /// (`kernel::discussed::MARKS`: a plan, something to discuss), which
    /// nothing reads from its words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mark: Option<String>,
    pub created: i64,
}

/// Where a message goes and what it answers.
#[derive(Clone, Copy, Debug, Default)]
pub struct Place<'a> {
    /// The message it answers.
    pub reply_to: Option<&'a str>,
    /// The message it is said under: any message of a thread, or the one a
    /// thread is to be started under.
    pub thread: Option<&'a str>,
    /// For an agent's message, the model that wrote it.
    pub model: Option<&'a str>,
    /// What its author put here to go with it, by id.
    pub attachments: &'a [String],
    /// For one a blueprint's step says: which step it is.
    pub said_by: Option<&'a SaidBy>,
    /// What its author marks it as (`kernel::discussed::MARKS`).
    pub mark: Option<&'a str>,
    /// For an agent's answer to what it was asked in this channel: in an
    /// open one it is there from what it answers, as a person is from
    /// what they say, to say what it is asked ([`Member::listens`]).
    /// Anything else an agent says is said only where it is in already.
    pub answers: bool,
}

#[derive(Clone)]
pub struct Channels {
    pub(crate) journal: Journal,
    /// What is attached to messages.
    pub attachments: Attachments,
    /// The bot that says what the factory's blueprints say.
    pub speaker: Speaker,
    /// Who is in each channel that is kept to its members.
    pub(crate) memory: Arc<Memory>,
    /// What gives a channel its place as it is made ([`Placing`]).
    pub(crate) placing: Arc<std::sync::OnceLock<Arc<Placing>>>,
    /// Every box of things addressed to somebody ([`crate::inbox`]).
    pub inbox: crate::Inbox,
    /// What channels offer of discussions, and how each one's thread
    /// stands ([`crate::discussed`]).
    pub(crate) discussed: Arc<crate::discussed::Held>,
    /// What says who a word after an `@` names among the people
    /// ([`Channels::names_with`]).
    pub(crate) naming: Arc<std::sync::OnceLock<Arc<crate::inboxed::Naming>>>,
    /// What reads which cards a column of a board holds, for a told
    /// line's part to keep ([`Channels::columns_with`]).
    pub(crate) columns: Arc<std::sync::OnceLock<Arc<crate::telling::Columns>>>,
    /// Where whoever makes a channel through this asked for it to be
    /// ([`Channels::asked_in`]).
    place: Option<String>,
    /// What is held of what people follow ([`crate::following`]).
    pub(crate) following: Arc<crate::following::Held>,
}

/// What gives a channel that is no project's its place, in the write
/// that makes its row, so that none is ever written that belongs to
/// nothing: said by what keeps what channels belong to, which is above
/// this crate (`studio::workspaces`: this one knows no workspace). It is
/// handed the write, the channel as it is written, where whoever makes
/// it asked for it to be, and how what it says of that is said (as
/// whoever makes the channel, to its members where it is kept), and says
/// in that write where it put the channel.
pub type Placing =
    dyn Fn(&mut journal::Writing<'_>, &Channel, Option<&str>, Source) -> rusqlite::Result<()> + Send + Sync;

/// What making or naming a channel is refused with where another has
/// the name: the names open channels share are each one channel's.
pub(crate) fn taken(refused: Error, name: &str) -> Error {
    match refused.kind {
        kernel::ErrorKind::Conflict => Error::conflict(format!("there is already a channel called `{name}`")),
        _ => refused,
    }
}

/// Names mentioned as `@name` in `body`, lowercase, in order, without repeats.
pub fn mentions(body: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut rest = body;
    while let Some(at) = rest.find('@') {
        let before = rest[..at].chars().next_back();
        let name: String = rest[at + 1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        // `user@example.com` is not a mention.
        if !name.is_empty() && !before.is_some_and(|c| c.is_alphanumeric()) {
            let name = name.to_lowercase();
            if !found.contains(&name) {
                found.push(name);
            }
        }
        rest = &rest[at + 1..];
    }
    found
}

/// What is said of a message as it is said.
pub(crate) fn said_of(channel: &Channel, message: &Message) -> serde_json::Value {
    json!({
        "channel": channel.id,
        "channel_name": channel.name,
        "project": channel.project,
        "id": message.id,
        "author": message.author,
        "author_kind": kind_of(&message.author),
        "author_name": message.author_name,
        "body": message.body,
        "reply_to": message.reply_to,
        "model": message.model,
        "thread": message.thread,
        "attachments": message.attachments,
        "mentions": mentions(&message.body),
        "said_by": message.said_by,
        "mark": message.mark,
    })
}

/// What is stored of a message.
pub(crate) const STORED: &str =
    "id, channel, author, author_name, body, reply_to, created, model, thread, said_by, mark";
/// A channel's row, as it is read.
const CHANNEL: &str = "id, name, project, topic, created, kept, title, alone, conversation, archived";

fn read_channel(row: &rusqlite::Row<'_>) -> rusqlite::Result<Channel> {
    Ok(Channel {
        id: row.get(0)?,
        name: row.get(1)?,
        project: row.get(2)?,
        topic: row.get(3)?,
        created: row.get(4)?,
        // Read separately, for all the channels asked for at once.
        members: Vec::new(),
        kept: row.get(5)?,
        title: row.get(6)?,
        alone: row.get(7)?,
        conversation: row.get(8)?,
        archived: crate::archived::read(row.get(9)?),
    })
}

/// The channels `chosen` picks (`WHERE …`, or nothing for all), by name,
/// each with who is in it.
pub(crate) fn read_channels(
    connection: &rusqlite::Connection,
    chosen: &str,
    by: &[&str],
) -> rusqlite::Result<Vec<Channel>> {
    let mut statement =
        connection.prepare(&format!("SELECT {CHANNEL} FROM channels {chosen} ORDER BY name"))?;
    let mut channels: Vec<Channel> = statement
        .query_map(rusqlite::params_from_iter(by), read_channel)?
        .collect::<rusqlite::Result<_>>()?;
    let mut members = connection.prepare(
        "SELECT member, name, listens, manages FROM channel_members WHERE channel = ?1 ORDER BY rowid",
    )?;
    for channel in &mut channels {
        let member = |row: &rusqlite::Row<'_>| {
            Ok(Member { actor: row.get(0)?, name: row.get(1)?, listens: row.get(2)?, manages: row.get(3)? })
        };
        let within = members.query_map([&channel.id], member)?;
        channel.members = within.collect::<rusqlite::Result<_>>()?;
    }
    Ok(channels)
}

/// What is said of somebody joining a channel.
pub(crate) fn joined_of(channel: &Channel, member: &Member) -> serde_json::Value {
    let mut data = json!({
        "channel": channel.id, "name": channel.name, "member": member.actor, "member_name": member.name,
    });
    // Whose it is, for whatever hears events by project.
    if let Some(project) = &channel.project {
        data["project"] = json!(project);
    }
    data
}

/// What is said of somebody leaving one.
pub(crate) fn left_of(channel: &str, project: Option<&str>, actor: &str) -> serde_json::Value {
    let mut data = json!({ "channel": channel, "member": actor });
    if let Some(project) = project {
        data["project"] = json!(project);
    }
    data
}

impl Channels {
    pub async fn open(journal: Journal, attachments: Attachments) -> Result<Channels> {
        journal.db().migrate("channels", SCHEMA).await?;
        let speaker = Speaker::open(journal.clone()).await?;
        // Who is in each kept channel answers for the journal from here:
        // before anything of one can be said.
        let memory = Memory::open(&journal).await?;
        let inbox = crate::Inbox::open(journal.clone(), memory.clone()).await?;
        let (placing, naming, columns) = (Arc::default(), Arc::default(), Arc::default());
        let discussed = Arc::new(crate::discussed::Held::open(&journal).await?);
        let place = None;
        let following = Arc::new(crate::following::Held::open(&journal).await?);
        Ok(Channels {
            following,
            journal,
            attachments,
            speaker,
            memory,
            placing,
            inbox,
            naming,
            columns,
            place,
            discussed,
        })
    }

    /// Says what gives a channel that is no project's its place as one
    /// is made, for every keeper of these channels. Said once; a second
    /// is not taken.
    pub fn makes_with(&self, placing: Arc<Placing>) {
        let _ = self.placing.set(placing);
    }

    /// The same channels, for making one in the place `place` names:
    /// handed as it is to what places a channel as it is made, which is
    /// what knows what it names.
    pub fn asked_in(&self, place: &str) -> Channels {
        Channels { place: Some(place.to_string()), ..self.clone() }
    }

    /// Creates a channel and makes its creator a member.
    pub async fn create(
        &self,
        name: &str,
        project: Option<&str>,
        topic: &str,
        by: &Principal,
    ) -> Result<Channel> {
        by.require(crate::kept::WRITE)?;
        let name = blueprint::expr::slug(name);
        if name.is_empty() {
            return Err(Error::invalid("a channel needs a name with at least one letter or digit"));
        }
        let channel = Channel {
            id: self.journal.new_id("ch"),
            name: name.clone(),
            project: project.map(str::to_string),
            topic: topic.to_string(),
            created: self.journal.now_ms(),
            members: Vec::new(),
            kept: false,
            title: None,
            alone: false,
            conversation: false,
            archived: None,
        };
        self.make(channel, Vec::new(), by).await.map_err(|err| taken(err, &name))
    }

    /// Writes a new channel with whoever makes it in it, and `others`
    /// with them, and says each, as one write: stopped anywhere, a
    /// channel that is there has its maker and everybody it was made
    /// with in it, was said to have been made, and is a project's or
    /// has the place it was given ([`Placing`]).
    pub(crate) async fn make(
        &self,
        mut channel: Channel,
        others: Vec<Member>,
        by: &Principal,
    ) -> Result<Channel> {
        // Whoever makes it manages it, where that is a person.
        let maker =
            Member { manages: kind_of(&by.actor()) == "user", ..Member::new(by.actor(), by.name.clone()) };
        channel.members = vec![maker.clone()];
        for other in others {
            if !channel.has(&other.actor) {
                channel.members.push(other);
            }
        }
        let _turn = self.memory.changing().await;
        let made = self.making(&channel, &maker);
        self.journal.write(made).await?;
        if channel.kept {
            let within = channel.members.iter().map(|member| member.actor.clone());
            self.memory.keep(&channel.id, within, channel.alone);
        }
        Ok(channel)
    }

    /// What writes the new channel `channel`, with everybody it says is
    /// in it, and says each, in a write: for [`Channels::make`], and for
    /// what makes a channel and says something in it as one write
    /// ([`crate::conversed`]).
    pub(crate) fn making(
        &self,
        channel: &Channel,
        maker: &Member,
    ) -> impl FnOnce(&mut journal::Writing<'_>) -> rusqlite::Result<()> + Send + 'static {
        let (row, maker) = (channel.clone(), maker.clone());
        let (placing, asked) = (self.placing.get().cloned(), self.place.clone());
        move |writing| {
            writing.transaction().execute(
                &format!("INSERT INTO channels ({CHANNEL}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"),
                params![
                    row.id,
                    row.name,
                    row.project,
                    row.topic,
                    row.created,
                    row.kept,
                    row.title,
                    row.alone,
                    row.conversation,
                    crate::archived::written(row.archived.as_ref())
                ],
            )?;
            // Whoever a kept channel is made with is told of it by name:
            // the memory has them in it only once this is written.
            let within: Vec<&str> = row.members.iter().map(|member| member.actor.as_str()).collect();
            let source = |actor: &str| Source::actor(actor).taking(row.takes(&within).as_deref());
            for member in &row.members {
                writing.transaction().execute(
                    "INSERT INTO channel_members (channel, member, name, joined, manages) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![row.id, member.actor, member.name, row.created, member.manages],
                )?;
                writing.say_here("channel.joined", source(&member.actor), joined_of(&row, member))?;
            }
            let data = json!({ "channel": row.id, "name": row.name, "project": row.project });
            writing.say_here("channel.created", source(&maker.actor), data)?;
            // One that is no project's is given its place, with what is
            // said of that: all of it or none.
            if let (None, Some(placing)) = (&row.project, placing) {
                placing(writing, &row, asked.as_deref(), source(&maker.actor))?;
            }
            Ok(())
        }
    }

    /// The channel called `name`, created if it does not exist: an open
    /// one, or the kept one of that project that is called so and has
    /// `by` in it ([`Channels::ensure_as`]).
    pub async fn ensure(
        &self,
        name: &str,
        project: Option<&str>,
        topic: &str,
        by: &Principal,
    ) -> Result<Channel> {
        let asked = crate::door::Asked { name, project, topic, ..Default::default() };
        self.ensure_as(asked, by).await
    }

    /// A channel's row by id or by name, with who is in it, whoever
    /// asks: for this crate, which then asks who it is there for.
    pub(crate) async fn row(&self, id_or_name: &str) -> Result<Channel> {
        let lookup = id_or_name.trim_start_matches('#').to_string();
        let named = lookup.clone();
        let found = self
            .journal
            .db()
            .call(move |connection| read_channels(connection, "WHERE id = ?1 OR name = ?1", &[&named]))
            .await?;
        found.into_iter().next().ok_or_else(|| missing(&lookup))
    }

    /// Every channel's row, by name: for this crate, the same way.
    pub(crate) async fn rows(&self) -> Result<Vec<Channel>> {
        self.journal.db().call(|connection| read_channels(connection, "", &[])).await
    }

    /// A channel by id or by name, with who is in it.
    pub async fn get(&self, reader: &Reader, channel: &str) -> Result<Channel> {
        let found = self.row(channel).await?;
        if reader.holds(&found.id) { Ok(found) } else { Err(missing(channel)) }
    }

    /// The channels `reader` is leave for: every one for the daemon's own.
    pub async fn list(&self, reader: &Reader) -> Result<Vec<Channel>> {
        Ok(self.rows().await?.into_iter().filter(|channel| reader.holds(&channel.id)).collect())
    }

    /// Takes a channel away, with everything said in it and everything
    /// attached to it. It cannot be put back, so it is the owner's to do.
    /// What the journal kept of what was said goes too, or a copy of the
    /// factory would carry a channel nobody can open.
    pub async fn remove(&self, channel: &str, by: &Principal) -> Result<Channel> {
        by.require("admin")?;
        let found = self.found(channel, by).await?;
        let _turn = self.memory.changing().await;
        // Who is in it as it goes, which is who is told that it went.
        let channel = self.row(&found.id).await?;
        let remembered = self.memory.forget(&channel.id);
        let id = channel.id.clone();
        // With every line of anybody's box that opens it, each owner
        // told in the same write: a line outlives no channel.
        let inbox = self.inbox.clone();
        let removed = self.journal.write(move |writing| {
            // First, while the channel is there to say who read it.
            inbox.forget_place(writing, &id)?;
            let transaction = writing.transaction();
            let messages = transaction.execute("DELETE FROM messages WHERE channel = ?1", [&id])?;
            transaction.execute("DELETE FROM channel_members WHERE channel = ?1", [&id])?;
            transaction.execute("DELETE FROM channel_docs WHERE channel = ?1", [&id])?;
            transaction.execute("DELETE FROM discussing WHERE channel = ?1", [&id])?;
            transaction.execute("DELETE FROM discussions WHERE channel = ?1", [&id])?;
            transaction.execute("DELETE FROM channels WHERE id = ?1", [&id])?;
            Ok(messages)
        });
        let messages = match removed.await {
            Ok((messages, _)) => messages,
            Err(error) => {
                // It is still there, and so is everybody in it.
                if let Some(members) = remembered {
                    self.memory.keep(&channel.id, members, channel.alone);
                }
                return Err(error);
            }
        };
        // Nor is it archived: another may be called what it was.
        self.memory.archived.write().expect("members lock").remove(&channel.id);
        self.discussed.forget(&channel.id);
        let attachments = self.attachments.forget_channel(&channel.id).await?;
        self.journal.forget_in_every("channel.message", "channel", &channel.id).await?;
        let data = json!({
            "channel": channel.id,
            "name": channel.name,
            "project": channel.project,
            "messages": messages,
            "attachments": attachments,
        });
        // Nobody is in it any more: whoever was is told by name.
        let were: Vec<&str> = channel.members.iter().map(|member| member.actor.as_str()).collect();
        let source = Source::actor(by.actor()).taking(channel.takes(&were).as_deref());
        let _ = self.journal.emit_here("channel.removed", source, data).await;
        Ok(channel)
    }

    /// Posts a message as `by`.
    pub async fn post(
        &self,
        channel: &str,
        by: &Principal,
        body: &str,
        reply_to: Option<&str>,
    ) -> Result<Message> {
        self.post_at(channel, by, body, Place { reply_to, ..Place::default() }).await
    }

    /// Posts a message as `by`, in a thread or in answer to another, and
    /// for an agent saying which model wrote it.
    pub async fn post_at(
        &self,
        channel: &str,
        by: &Principal,
        body: &str,
        place: Place<'_>,
    ) -> Result<Message> {
        let reader = self.may_write(channel, by).await?;
        let message = self.insert(&reader, channel, &by.actor(), &by.name, body, place).await?;
        // One that names the factory's bot is said to it, by whoever this is.
        self.speaker.heard(by, &message);
        Ok(message)
    }

    /// Says an agent's answer to what it was asked in `channel`, as the
    /// agent (`agent`): the factory says it for the agent it started, on
    /// the leave of whoever asked it there, who said that there. So the
    /// agent is not asked whether it may write in channels, and is given
    /// nothing by it: whatever it posts by its own commands is asked of it
    /// as ever ([`Channels::post_at`]). Who is in the channel is asked as
    /// for any answer (`place.answers`): one kept to its members is said
    /// in only by an agent that is in it.
    pub async fn post_answer(
        &self,
        channel: &str,
        agent: &Principal,
        body: &str,
        place: Place<'_>,
    ) -> Result<Message> {
        let own = Channels::daemon("an agent's answer to what it was asked in a channel, said there for it");
        let place = Place { answers: true, ..place };
        let message = self.insert(&own, channel, &agent.actor(), &agent.name, body, place).await?;
        self.speaker.heard(agent, &message);
        Ok(message)
    }

    /// Posts as the factory's bot, which is in the channel from the first
    /// thing it says there: what a blueprint's step says that names no bot
    /// (`place.said_by` says which step), and what the bot answers.
    pub async fn post_speaker(
        &self,
        reader: &Reader,
        channel: &str,
        body: &str,
        place: Place<'_>,
    ) -> Result<Message> {
        let name = self.speaker.name();
        // Asked before the bot is put in it: nothing is said in an
        // archived channel, and the bot comes to one by what it says.
        let found = self.get(reader, channel).await?;
        if found.archived.is_some() {
            return Err(crate::archived::refused(&found));
        }
        self.join(reader, channel, &bot_actor(&name), &name).await?;
        self.insert(reader, channel, &bot_actor(&name), &name, body, place).await
    }

    /// Calls the factory's bot something else. Where it was a member it is
    /// one under its new name; what it said keeps the name it was said under.
    pub async fn call_speaker(&self, name: &str, by: &Principal) -> Result<String> {
        let (was, name) = self.speaker.call(name, by).await?;
        if was != name {
            let own =
                Channels::daemon("the factory's bot is called something else in every channel it is in");
            for channel in self.list(&own).await? {
                if channel.has(&bot_actor(&was)) {
                    self.leave(&own, &channel.id, &bot_actor(&was)).await?;
                    self.join(&own, &channel.id, &bot_actor(&name), &name).await?;
                }
            }
        }
        Ok(name)
    }

    /// Narrows what the factory's bot may ask, or gives it the usual again
    /// ([`Speaker::hold`]).
    pub async fn hold_speaker(
        &self,
        holds: Option<Vec<String>>,
        by: &Principal,
    ) -> Result<Option<Vec<String>>> {
        self.speaker.hold(holds, by).await
    }

    /// Posts a message as the system's, under any name. Nothing the factory
    /// says now is said this way: it speaks as its bot ([`Self::post_speaker`]).
    /// It is how what was said before there was one is kept, and read.
    pub async fn post_system(
        &self,
        reader: &Reader,
        channel: &str,
        name: &str,
        body: &str,
    ) -> Result<Message> {
        self.insert(reader, channel, "system", name, body, Place::default()).await
    }

    /// Posts as a bot: a blueprint taking part in the channel under `name`.
    pub async fn post_bot(
        &self,
        reader: &Reader,
        channel: &str,
        name: &str,
        body: &str,
        place: Place<'_>,
    ) -> Result<Message> {
        self.insert(reader, channel, &bot_actor(name), name, body, place).await
    }

    /// What a message of `channel` is said under, if it is under
    /// anything: looked for in that channel and nowhere else, so that a
    /// message of another channel is as much there as one that was never
    /// said.
    async fn within(&self, channel: &str, message: &str) -> Result<Option<String>> {
        let (of, lookup) = (channel.to_string(), message.to_string());
        let found: Option<Option<String>> = self
            .journal
            .db()
            .call(move |connection| {
                optional(connection.query_row(
                    "SELECT thread FROM messages WHERE id = ?1 AND channel = ?2",
                    [&lookup, &of],
                    |row| row.get(0),
                ))
            })
            .await?;
        found.ok_or_else(|| no_message(message))
    }

    async fn insert(
        &self,
        reader: &Reader,
        channel: &str,
        author: &str,
        author_name: &str,
        body: &str,
        place: Place<'_>,
    ) -> Result<Message> {
        reader.writing()?;
        let channel = self.get(reader, channel).await?;
        let body = Channels::may_say(body, place.attachments)?;
        // The message a thread hangs from, given any message of it: the
        // one it is under, or itself when it is under none. And what it
        // answers is a message of this channel too.
        let thread = match place.thread {
            Some(under) => Some(self.within(&channel.id, under).await?.unwrap_or_else(|| under.to_string())),
            None => None,
        };
        if let Some(answers) = place.reply_to {
            self.within(&channel.id, answers).await?;
        }
        let mark = crate::discussed::marked(place.mark)?;
        let message = Message {
            id: self.journal.new_id("msg"),
            channel: channel.id.clone(),
            author: author.to_string(),
            author_name: author_name.to_string(),
            body: body.to_string(),
            reply_to: place.reply_to.map(str::to_string),
            model: place.model.map(str::to_string),
            thread,
            replies: 0,
            attachments: Vec::new(),
            said_by: place.said_by.cloned(),
            mark,
            created: self.journal.now_ms(),
        };
        // Somebody who comes into the channel by what they say comes in
        // under the turn at changing who is in anything, as anybody
        // does: not as it is kept.
        let _turn = match channel.has(author) {
            false if kind_of(author) == "user" || place.answers => Some(self.memory.changing().await),
            _ => None,
        };
        let said = self.saying(channel, message, place.attachments.to_vec(), place.answers);
        self.journal.write(said).await?.0
    }

    /// What can be said: some words, or something attached, and no more
    /// words than a message holds.
    pub(crate) fn may_say<'a>(body: &'a str, attached: &[String]) -> Result<&'a str> {
        let body = body.trim();
        if body.is_empty() && attached.is_empty() {
            return Err(Error::invalid("a message needs some text, or something attached"));
        }
        if body.len() > MAX_BODY {
            return Err(Error::invalid("that message is too long; put it in a file and link to it"));
        }
        Attachments::may_carry(attached)?;
        Ok(body)
    }

    /// What writes `message` in `channel` and says it, in a write: what
    /// goes with it is made part of it, the message is written and it
    /// is said, with the lines it puts in anybody's box, as one write.
    /// If any of what goes with it cannot be part of it, or the channel
    /// is archived as the write finds it, nothing is written and nothing
    /// said, and the refusal is what the write gives; and stopped
    /// anywhere, a message that is there was said.
    ///
    /// **Whoever says something is in the channel.** A person comes into
    /// an open one by what they say, in this write; an agent says
    /// something only in one it is in, and is refused otherwise, but for
    /// its answer to what it was asked there (`answers`), by which it
    /// comes into an open one as a person does, to say what it is asked
    /// there. A bot, and what is said as the system, are as they were.
    pub(crate) fn saying(
        &self,
        channel: Channel,
        message: Message,
        attached: Vec<String>,
        answers: bool,
    ) -> impl FnOnce(&mut journal::Writing<'_>) -> rusqlite::Result<Result<Message>> + Send + 'static {
        let (naming, inbox) = (self.naming.get().cloned(), self.inbox.clone());
        move |writing| {
            // To whom it is said is read in this write, before anything is
            // written: a channel kept, opened again or archived since it
            // was read above is as it is now.
            let channel = crate::door::as_it_is(writing.transaction(), channel)?;
            if channel.archived.is_some() {
                return Ok(Err(crate::archived::refused(&channel)));
            }
            let mut row = message;
            let comes = match crate::joined::comes_in(writing.transaction(), &channel, &row.author, answers)?
            {
                Ok(comes) => comes,
                Err(refused) => return Ok(Err(refused)),
            };
            let claimed =
                Attachments::claim_in(writing.transaction(), &attached, &row.channel, &row.author, &row.id)?;
            let Some(claimed) = claimed else { return Ok(Err(Attachments::not_theirs())) };
            row.attachments = claimed;
            // Last of what may refuse it: in from here, with what is said
            // of that before what they said.
            if let Some(listens) = comes {
                let member = Member {
                    actor: row.author.clone(),
                    name: row.author_name.clone(),
                    listens,
                    manages: false,
                };
                writing.transaction().execute(
                    "INSERT INTO channel_members (channel, member, name, joined, listens) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![channel.id, member.actor, member.name, row.created, member.listens],
                )?;
                writing.say_here(
                    "channel.joined",
                    Source::actor(&member.actor),
                    joined_of(&channel, &member),
                )?;
            }
            writing.transaction().execute(
                &format!(
                    "INSERT INTO messages ({STORED}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
                ),
                params![
                    row.id,
                    row.channel,
                    row.author,
                    row.author_name,
                    row.body,
                    row.reply_to,
                    row.created,
                    row.model,
                    row.thread,
                    row.said_by.as_ref().map(|said| json!(said).to_string()),
                    row.mark
                ],
            )?;
            let source = Source::actor(&row.author).taking(channel.takes(&[]).as_deref());
            writing.say_here("channel.message", source, said_of(&channel, &row))?;
            // Whoever it is for is told in their box, in this write: each
            // member of a conversation, and whoever it names.
            crate::inboxed::tell_of(writing, &inbox, naming.as_deref(), &channel, &row)?;
            Ok(Ok(row))
        }
    }
}
