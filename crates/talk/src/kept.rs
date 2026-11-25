//! A channel kept to its members: read by nobody else, by any way of
//! reading.
//!
//! **The one question.** Whether somebody reads a kept channel is one
//! expression, [`reads`], and everything asks it: a route, a blueprint's
//! action, a search, and the journal as it hands an event to a listener.
//! It is asked through three doors, which differ only in which scope is
//! asked first, in the words a scope is always refused in:
//! [`Channels::may_read`] (`channels:read`), [`Channels::may_write`]
//! (`channels:write`) and [`Channels::may_find`] (none of the channels':
//! for what asks a scope of its own, as taking a channel away asks
//! `admin`). An open channel is there for anybody the scope lets by, as
//! it always was. A kept one is there for whoever reads it, and for
//! everybody else it answers in the words a channel that does not exist
//! answers in: being told that one is there is reading that it is.
//!
//! **The reader.** What a door gives is a [`Reader`], leave for that
//! channel, and nothing of a channel is handed out without one: a
//! [`Channel`] with who is in it, a [`crate::Message`] and a
//! [`crate::Attachment`] come only from methods that take a `&Reader`
//! (or take the principal and ask a door themselves). So a new read
//! that asks nothing does not compile. What the daemon reads for itself
//! and hands to nobody (the search index, which agent a message is for)
//! says [`Channels::daemon`] with why, and each such line is on a list
//! in a test that reads the source (`crates/api/tests/suite/kept.rs`):
//! a new one fails it until it is listed with its reason. A test of this
//! crate's suite reads this crate's own source for a public method that
//! hands one of those out and takes neither.
//!
//! **The memory.** An event of a kept channel is said as taking
//! `member:<its id>` ([`Channel::takes`]), which no scope answers for,
//! and who is told it is asked of what is installed here
//! (`journal::Journal::members_from`) each time an event is handed to a
//! listener. That cannot wait for the database, so who is in each kept
//! channel is held in memory ([`Memory`]): filled from the tables as the
//! channels are opened, and never the only place anything is kept. It
//! is changed before the write for somebody leaving and after it for
//! somebody joining, so that between the two it says less than the
//! tables and never more; and one change at a time
//! ([`Memory::changing`]), or somebody leaving as they are added could
//! be written out and remembered in.
//!
//! **An agent.** An agent that is in a kept channel is of it only while
//! a turn asked there is in hand: its key opens the channel while its
//! program is at work on what somebody there asked, and at no other
//! time, so that nobody who may ask it elsewhere has it read the channel
//! out. Whether a turn asked there is in hand is whoever keeps agents'
//! to say, a crate above this one, and is asked as synchronously as who
//! is in a channel is: so it is installed here ([`Channels::asked_with`])
//! and answered from what that crate holds in memory. With nothing
//! installed no agent reads a kept channel. What the factory's owner
//! holds reads nothing for an agent either: an agent is given what it
//! holds by somebody, and is asked by anybody. And an agent comes into
//! a kept channel only by being brought in ([`Channels::bring`]), by
//! whoever keeps agents, which says first that it may be in one.
//!
//! **An agent the factory does not start** is somebody's program
//! outside it, as a person with a key is: nothing is started for it, so
//! nothing of what holds a started agent's program applies, and it is
//! of a kept channel it is in as a person is, with its own credential
//! holding the channels' scope. Which agents those are is whoever keeps
//! agents' to say too ([`Channels::outside_with`]); with nothing
//! installed none is. What the factory's owner holds still reads
//! nothing for it, and it is told nothing by being named.
//!
//! **Asked nowhere else.** A kept channel may say that its agents are
//! asked nowhere else (`channels.alone`): for what is most closely held,
//! where an agent that has read it is not to be given work anywhere a
//! word of it could follow. Whoever gives agents work asks which
//! channel an agent is so held to ([`Channels::alone_in`]), and so does
//! bringing one in, under the turn at changing who is in anything: an
//! agent is one such channel's alone, so one that is held to a channel
//! is brought into no other kept one, and one that is in a kept channel
//! is not brought into one that would hold it.
//!
//! **Who else is told.** Membership is asked as the event is handed
//! over, and by then whoever left is no member, and a channel that was
//! taken away has none: they would never learn to stop showing it. So
//! an event may name who it is also for, after the channel in what it
//! takes (`member:<id>;<actor>;<actor>`): `channel.left` names who left,
//! `channel.removed` everybody who was in it, and `channel.joined` and
//! `channel.created` who joined, since the memory has them only a moment
//! after the write that says so. It is carried by the event and not
//! done by ordering the telling before the change, because an event is
//! handed over whenever its listener comes to it, a stream read again
//! from an earlier place among them, and no order of two things in this
//! crate reaches that.
//!
//! **The door.** How a channel comes to be kept, and stops being, who
//! is in one from the first and how somebody leaves is [`crate::door`]:
//! each one write under the turn at changing who is in anything, with
//! this memory changed on the side of the write where it says less.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use journal::Journal;
use kernel::principal::{MEMBER, covers};
use kernel::{Error, Principal, Result};

use crate::channels::{Channel, Channels, kind_of};

/// What reading channels takes of anybody.
pub const READ: &str = "channels:read";
/// What saying something in one takes.
pub const WRITE: &str = "channels:write";
/// What reads every kept channel, a member or not: the factory's owner
/// holds it. The second half of [`reads`].
const EVERYTHING: &str = "private";
/// What stands between the channel and each actor an event is also for.
pub(crate) const ALSO: char = ';';

/// What whoever keeps agents answers ([`Channels::asked_with`]): whether
/// the agent that is this actor has a turn asked in the channel of this
/// id in hand.
pub type Asked = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;

/// What whoever keeps agents answers ([`Channels::outside_with`]): whether
/// the agent that is this actor is one the factory does not start.
pub type Outside = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// **The one expression**: whether whoever holds `held` reads a kept
/// channel, being of it or not. The channels' own scope, and then being
/// of it; or, for now, holding what the factory's owner holds, which
/// reads everything. Taking the owner's half away is taking the last
/// clause away, here and nowhere else.
///
/// A key somebody made acts as its maker (`Principal::actor` is the
/// maker's) and holds what the key was given, so it is a member through
/// its maker only where the key itself holds the scope.
///
/// An `agent` is `of` it while it is in it and a turn asked there is in
/// hand, or while it is in it where it is one the factory does not
/// start ([`Memory::of`]), and the owner's half is not an agent's.
fn reads(of: bool, agent: bool, held: &[String]) -> bool {
    covers(held, READ) && (of || (!agent && covers(held, EVERYTHING)))
}

impl Memory {
    /// Whether whoever is `actor`, holding `held`, in the channel or not
    /// as `member` says, reads a channel that is kept or open as `kept`
    /// says, as it stands in a write under way: the scope every reading
    /// of a channel takes and, for one that is kept, the one expression.
    /// For what is decided of somebody in a write, where no door can be
    /// asked: whether one a message is for is told, and which lines of a
    /// box its owner is shown. An agent the factory starts is of no kept
    /// channel here: whether one is while a turn is in hand is asked
    /// where it reads, and nothing is put in its box for what is said in
    /// one. One it does not start is of a kept channel it is in.
    pub(crate) fn reads_as(&self, kept: bool, member: bool, actor: &str, held: &[String]) -> bool {
        let agent = is_agent(actor);
        let of = member && (!agent || self.outside(actor));
        covers(held, READ) && (!kept || reads(of, agent, held))
    }

    /// Whether the agent that is `actor` is one the factory does not
    /// start, as whoever keeps agents says; none where nobody says.
    pub(crate) fn outside(&self, actor: &str) -> bool {
        let outside = self.outside.read().expect("members lock").clone();
        is_agent(actor) && outside.is_some_and(|outside| outside(actor))
    }
}

/// Whether an actor is an agent.
fn is_agent(actor: &str) -> bool {
    kind_of(actor) == "agent"
}

/// Leave to read a channel, and to write in it where it says so. Made
/// by one of the doors, for the channel that was asked about, or by
/// [`Channels::daemon`] for the daemon's own reading, which is of any.
#[derive(Debug)]
pub struct Reader {
    /// The channel it is leave for, by id; any, where it is the daemon's.
    of: Option<String>,
    writes: bool,
}

impl Reader {
    /// The channel it is leave for, by id. Nothing for the daemon's own.
    pub fn channel(&self) -> Option<&str> {
        self.of.as_deref()
    }

    /// Whether it is leave to write there too.
    pub fn writes(&self) -> bool {
        self.writes
    }

    /// Whether it is leave for the channel with this id.
    pub(crate) fn holds(&self, channel: &str) -> bool {
        self.of.as_deref().is_none_or(|of| of == channel)
    }

    /// Refuses what is no leave to write.
    pub(crate) fn writing(&self) -> Result<()> {
        if self.writes {
            return Ok(());
        }
        Err(Error::denied("this was leave to read the channel, and something is to be written in it"))
    }
}

/// What a channel that is not there answers, and so what one that is
/// kept from whoever asks answers.
pub(crate) fn missing(lookup: &str) -> Error {
    Error::not_found(format!("there is no channel `{}`", lookup.trim_start_matches('#')))
}

/// The same of a message.
pub(crate) fn no_message(id: &str) -> Error {
    Error::not_found(format!("there is no message `{id}`"))
}

/// The same of something attached.
pub(crate) fn no_attachment(id: &str) -> Error {
    Error::not_found(format!("there is no attachment `{id}`"))
}

/// What hearing an event of the channel `id` takes ([`Channel::takes`]).
pub(crate) fn takes(id: &str, kept: bool, also: &[&str]) -> Option<String> {
    kept.then(|| {
        let mut takes = format!("{MEMBER}{id}");
        for actor in also {
            takes.push(ALSO);
            takes.push_str(actor);
        }
        takes
    })
}

impl Channel {
    /// What hearing an event of this channel takes: nothing for an open
    /// one, and being a member for one that is kept. `also` are actors
    /// the event is for whether or not they are members when it is
    /// handed over (the module's notes say which events name any).
    pub fn takes(&self, also: &[&str]) -> Option<String> {
        takes(&self.id, self.kept, also)
    }

    /// Whether `actor` is in it.
    pub(crate) fn has(&self, actor: &str) -> bool {
        self.members.iter().any(|member| member.actor == actor)
    }

    /// Whether `by` is somebody it is there for: anybody for an open
    /// one, and whoever reads it for one that is kept, as `memory` says
    /// of an agent's work in hand.
    pub(crate) fn is_there_for(&self, by: &Principal, memory: &Memory) -> bool {
        if !self.kept {
            return true;
        }
        let actor = by.actor();
        reads(memory.of(&self.id, &actor, self.has(&actor)), is_agent(&actor), &by.scopes)
    }
}

/// Who is in each kept channel, by the channel's id.
#[derive(Default)]
pub struct Memory {
    kept: RwLock<HashMap<String, HashSet<String>>>,
    /// What whoever keeps agents answers, once it is there to.
    asked: RwLock<Option<Asked>>,
    /// Which agents the factory does not start, as the same says.
    outside: RwLock<Option<Outside>>,
    /// The kept channels that say their agents are asked nowhere else.
    alone: RwLock<HashSet<String>>,
    /// The channels that are archived, each id with its name: for what
    /// is asked as an agent is given work, which no database waits on
    /// ([`Channels::is_archived`]). Kept here because this is what is
    /// filled from the tables as channels are opened; it is no part of
    /// who is told anything.
    pub(crate) archived: RwLock<HashMap<String, String>>,
    /// What answers for a line of something said on a record, where what
    /// keeps records said it ([`crate::Inbox::records_with`]): asked where
    /// a line is asked whether it is passed over, which this is handed.
    pub(crate) records: std::sync::OnceLock<Arc<dyn crate::commented::OnRecords>>,
    /// Held while who is in a channel is changed, the memory and the
    /// write together.
    changing: Arc<tokio::sync::Mutex<()>>,
}

/// The turn at changing who is in anything.
pub(crate) type Changing = tokio::sync::OwnedMutexGuard<()>;

impl Memory {
    /// The memory the journal's events are answered from: installed
    /// where none was, and filled from the tables either way.
    pub(crate) async fn open(journal: &Journal) -> Result<Arc<Memory>> {
        let answer = |memory: Arc<Memory>| -> journal::Members {
            Arc::new(move |of, actor, held| memory.tells(of, actor, held))
        };
        let memory = journal.members_from(Arc::new(Memory::default()), answer).ok_or_else(|| {
            Error::internal(
                "something else already answers who is a member of what for this journal, so nobody could be \
                 told what is said in a channel kept to its members",
            )
        })?;
        let rows: Vec<(String, bool, Option<String>)> = journal
            .db()
            .call(|connection| {
                let mut statement = connection.prepare(
                    "SELECT c.id, c.alone, m.member FROM channels c
                     LEFT JOIN channel_members m ON m.channel = c.id WHERE c.kept = 1",
                )?;
                let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
                rows.collect()
            })
            .await?;
        let mut kept: HashMap<String, HashSet<String>> = HashMap::new();
        let mut alone = HashSet::new();
        for (channel, holds, member) in rows {
            if holds {
                alone.insert(channel.clone());
            }
            kept.entry(channel).or_default().extend(member);
        }
        *memory.kept.write().expect("members lock") = kept;
        *memory.alone.write().expect("members lock") = alone;
        let archived: Vec<(String, String)> = journal
            .db()
            .call(|connection| {
                let mut statement =
                    connection.prepare("SELECT id, name FROM channels WHERE archived IS NOT NULL")?;
                let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
                rows.collect()
            })
            .await?;
        *memory.archived.write().expect("members lock") = archived.into_iter().collect();
        Ok(memory)
    }

    /// The turn at changing who is in anything: taken before the memory
    /// or the tables are touched, and kept until both are done.
    pub(crate) async fn changing(&self) -> Changing {
        self.changing.clone().lock_owned().await
    }

    /// A kept channel was made, or is back: these are in it, and it
    /// says its agents are asked nowhere else or does not, as `alone` says.
    pub(crate) fn keep(&self, channel: &str, members: impl IntoIterator<Item = String>, alone: bool) {
        self.kept.write().expect("members lock").insert(channel.to_string(), members.into_iter().collect());
        if alone {
            self.alone.write().expect("members lock").insert(channel.to_string());
        }
    }

    /// Whether the kept channel's agents are asked nowhere else, as it
    /// says from here.
    pub(crate) fn alone(&self, channel: &str, alone: bool) {
        let mut held = self.alone.write().expect("members lock");
        match alone {
            true => held.insert(channel.to_string()),
            false => held.remove(channel),
        };
    }

    /// A channel is going, or is open again. Who was remembered in it,
    /// where it was kept.
    pub(crate) fn forget(&self, channel: &str) -> Option<HashSet<String>> {
        self.alone.write().expect("members lock").remove(channel);
        self.kept.write().expect("members lock").remove(channel)
    }

    /// The kept channel that holds the agent `actor` to itself, by id:
    /// one it is in that says its agents are asked nowhere else. `but`
    /// is a channel that is not looked at.
    fn holds_alone(&self, actor: &str, but: Option<&str>) -> Option<String> {
        let alone = self.alone.read().expect("members lock");
        self.within(actor).into_iter().find(|id| Some(id.as_str()) != but && alone.contains(id))
    }

    /// Why the agent `actor` is not brought into the kept channel `id`,
    /// which says its agents are asked nowhere else or does not as
    /// `alone` says, where it is not: an agent is one such channel's
    /// alone.
    pub(crate) fn not_brought(&self, id: &str, alone: bool, actor: &str) -> Option<String> {
        // One the factory does not start is given no work anywhere:
        // nothing is held by its being asked nowhere else.
        if self.outside(actor) {
            return None;
        }
        let name = actor.strip_prefix("agent:").unwrap_or(actor);
        if self.holds_alone(actor, Some(id)).is_some() {
            return Some(format!(
                "`{name}` is in a channel it is asked nowhere but in, and so is brought into no other that is \
                 kept to its members"
            ));
        }
        let elsewhere = self.within(actor).into_iter().any(|within| within != id);
        (alone && elsewhere).then(|| {
            format!(
                "`{name}` would be asked nowhere but here, and it is in another channel kept to its members: \
                 an agent is held so to one channel, and is then in no other that is kept"
            )
        })
    }

    /// Somebody is in a kept channel: said once the tables say so.
    pub(crate) fn joined(&self, channel: &str, actor: &str) {
        if let Some(members) = self.kept.write().expect("members lock").get_mut(channel) {
            members.insert(actor.to_string());
        }
    }

    /// Somebody is leaving a kept channel: said before the tables are
    /// told. Whether they were remembered there.
    pub(crate) fn left(&self, channel: &str, actor: &str) -> bool {
        self.kept.write().expect("members lock").get_mut(channel).is_some_and(|members| members.remove(actor))
    }

    /// Somebody is leaving every channel. The kept ones they were
    /// remembered in.
    fn left_all(&self, actor: &str) -> Vec<String> {
        let mut kept = self.kept.write().expect("members lock");
        let within =
            kept.iter_mut().filter_map(|(channel, members)| members.remove(actor).then_some(channel));
        within.cloned().collect()
    }

    /// Whether `actor`, in the kept channel or not as `member` says, is
    /// of it: anybody who is in it, but an agent the factory starts,
    /// which is of it only while a turn asked there is in hand, as
    /// whoever keeps agents says; and no agent where nobody says.
    fn of(&self, channel: &str, actor: &str, member: bool) -> bool {
        if !member || !is_agent(actor) || self.outside(actor) {
            return member;
        }
        let asked = self.asked.read().expect("members lock").clone();
        asked.is_some_and(|asked| asked(channel, actor))
    }

    /// Whether any channel is kept.
    fn keeps_any(&self) -> bool {
        !self.kept.read().expect("members lock").is_empty()
    }

    /// Whether there is a kept channel of this id.
    pub(crate) fn is_kept(&self, channel: &str) -> bool {
        self.kept.read().expect("members lock").contains_key(channel)
    }

    /// The kept channels `actor` is remembered in, by id.
    pub(crate) fn within(&self, actor: &str) -> Vec<String> {
        let kept = self.kept.read().expect("members lock");
        let mut within: Vec<String> =
            kept.iter().filter(|(_, members)| members.contains(actor)).map(|(id, _)| id.clone()).collect();
        within.sort();
        within
    }

    /// Whether whoever is `actor`, holding `held`, is told an event that
    /// takes being a member of `of`: what follows `member:` there, a
    /// channel's id and then anybody the event is also for; or
    /// somebody's box ([`crate::inbox`]), which is its owner's.
    fn tells(&self, of: &str, actor: &str, held: &[String]) -> bool {
        // Somebody's box is its owner's, and no channel's to answer for.
        if let Some(told) = crate::inbox::tells(of, actor, held) {
            return told;
        }
        let mut said = of.split(ALSO);
        let channel = said.next().unwrap_or_default();
        let member = || {
            let within = self
                .kept
                .read()
                .expect("members lock")
                .get(channel)
                .is_some_and(|members| members.contains(actor));
            self.of(channel, actor, within)
        };
        // An agent is told nothing by being named: that it is in a
        // kept channel, or was, is the channel's to say while a turn
        // asked there is in hand, and at no other time.
        let agent = is_agent(actor);
        let also = !agent && said.any(|also| also == actor);
        reads(also || member(), agent, held)
    }
}

/// Somebody who is leaving every channel there is, in a write somebody
/// else makes ([`Channels::leave_all`]): the memory has let them go, and
/// nothing else changes who is in anything until this is dropped, which
/// is after that write.
pub struct Leaving {
    actor: String,
    /// The kept channels the memory had them in: put back by
    /// [`Channels::stayed`] where the write was not made.
    within: Vec<String>,
    _turn: Changing,
}

/// Who is leaving every channel, for the write that takes them out of
/// each: made only of a [`Leaving`], so that no such write is made
/// before the memory has let them go.
pub struct Left(String);

impl Left {
    pub(crate) fn actor(&self) -> &str {
        &self.0
    }
}

impl Leaving {
    /// What the write that takes them out is handed.
    pub fn left(&self) -> Left {
        Left(self.actor.clone())
    }
}

impl Channels {
    /// The channel `lookup` names, where it is there for `by`: the row
    /// as the tables have it, and for one that is kept the one question.
    pub async fn found(&self, lookup: &str, by: &Principal) -> Result<Channel> {
        let channel = self.row(lookup).await?;
        if channel.is_there_for(by, &self.memory) { Ok(channel) } else { Err(missing(lookup)) }
    }

    /// Leave for `by` to read `channel`, named by id or by name.
    pub async fn may_read(&self, channel: &str, by: &Principal) -> Result<Reader> {
        by.require(READ)?;
        let found = self.found(channel, by).await?;
        Ok(Reader { of: Some(found.id), writes: false })
    }

    /// Leave for `by` to say something in `channel`, and to change who
    /// is in it. In one that is kept that takes reading it.
    pub async fn may_write(&self, channel: &str, by: &Principal) -> Result<Reader> {
        self.writable(channel, by).await.map(|(reader, _)| reader)
    }

    /// The same, with the channel as it was found, who is in it with it:
    /// for what asks whether somebody is in it as it asks for leave.
    pub async fn writable(&self, channel: &str, by: &Principal) -> Result<(Reader, Channel)> {
        by.require(WRITE)?;
        let found = self.found(channel, by).await?;
        Ok((Reader { of: Some(found.id.clone()), writes: true }, found))
    }

    /// Leave for `by` to be told that `channel` is there and who is in
    /// it, with no scope of the channels' asked first: for what asks a
    /// scope of its own and always went by that alone (taking a channel
    /// away, bringing an agent into one, saying which agents are in
    /// one). An open channel is found as it always was, and written in
    /// as it always was; a kept one is found by whoever reads it, and
    /// written in only by one of them who also holds what writing takes.
    pub async fn may_find(&self, channel: &str, by: &Principal) -> Result<Reader> {
        let found = self.found(channel, by).await?;
        Ok(Reader { of: Some(found.id), writes: !found.kept || by.can(WRITE) })
    }

    /// Leave for `by` to read the channel the message `id` was said in,
    /// for what is handed a message and no channel (a conversation an
    /// agent is asked to go on with, named by the message it hangs
    /// from). A message of a channel kept from them is not there.
    pub async fn may_read_message(&self, id: &str, by: &Principal) -> Result<Reader> {
        by.require(READ)?;
        let lookup = id.to_string();
        let within = move |connection: &mut rusqlite::Connection| {
            journal::optional(connection.query_row(
                "SELECT channel FROM messages WHERE id = ?1",
                [&lookup],
                |row| row.get::<_, String>(0),
            ))
        };
        let channel = self.journal.db().call(within).await?.ok_or_else(|| no_message(id))?;
        match self.found(&channel, by).await {
            Ok(found) => Ok(Reader { of: Some(found.id), writes: false }),
            Err(error) if error.kind == kernel::ErrorKind::NotFound => Err(no_message(id)),
            Err(error) => Err(error),
        }
    }

    /// Installs what answers whether an agent has a turn asked in a
    /// channel in hand, for whoever keeps agents: asked each time an
    /// agent reads a kept channel or is handed an event of one. What was
    /// installed before is answered no longer.
    pub fn asked_with(&self, asked: Asked) {
        *self.memory.asked.write().expect("members lock") = Some(asked);
    }

    /// Installs what answers whether an agent is one the factory does
    /// not start, for whoever keeps agents: such a one is of a kept
    /// channel it is in as a person is, and is told in its box of what
    /// is said to it. What was installed before is answered no longer.
    pub fn outside_with(&self, outside: Outside) {
        *self.memory.outside.write().expect("members lock") = Some(outside);
    }

    /// Whether any channel is kept to its members: asked first by what
    /// would otherwise ask something of every turn or every channel.
    pub fn keeps_any(&self) -> bool {
        self.memory.keeps_any()
    }

    /// What hearing something said of the kept channel of this id takes
    /// ([`Channel::takes`]), for what is said of it by another crate
    /// that holds its id and not its row (an agent's turn asked there).
    pub fn kept_takes(channel: &str) -> String {
        format!("{MEMBER}{channel}")
    }

    /// Whether the channel of this id is kept to its members. For what
    /// the daemon does differently for one (where an agent's program
    /// keeps what it reads there), and never an answer to anybody.
    pub fn is_kept(&self, channel: &str) -> bool {
        self.memory.is_kept(channel.trim_start_matches('#'))
    }

    /// The kept channels `actor` is in, by id: for whoever keeps agents,
    /// to do what an agent being in one takes, and never to be handed to
    /// anybody.
    pub fn kept_with(&self, actor: &str) -> Vec<String> {
        self.memory.within(actor)
    }

    /// Brings the agent called `agent` into `channel`: the one way an
    /// agent comes to be in a channel kept to its members, for whoever
    /// keeps agents, which has said first that this one may be in one.
    /// Refused where that would have it held to one channel and in
    /// another. In an open channel it is a member like any.
    pub async fn bring(&self, reader: &Reader, channel: &str, agent: &str) -> Result<()> {
        self.join_as(reader, channel, &format!("agent:{agent}"), agent, (true, true)).await
    }

    /// Brings the agent called `agent` into `channel` to say there what
    /// it is asked there: a member, listed with the rest, whose words are
    /// its own there, and which is asked there by its name and not by
    /// what names nobody ([`crate::Member::listens`]). For whatever has an
    /// agent report in a channel (a fleet's), and whatever asks it there:
    /// an agent says something only in a channel it is in. Into one kept
    /// to its members only as [`Channels::bring`] brings one, by whoever
    /// keeps agents, which has said first that it may be in one.
    pub async fn bring_asked(&self, reader: &Reader, channel: &str, agent: &str) -> Result<()> {
        self.join_as(reader, channel, &format!("agent:{agent}"), agent, (true, false)).await
    }

    /// The channels the agent called `agent` is in to say what it is
    /// asked there, by id ([`Channels::bring_asked`]): for whoever keeps
    /// agents, which takes it out of each as it stops, and never to be
    /// handed to anybody.
    pub async fn asked_into(&self, agent: &str) -> Result<Vec<String>> {
        let actor = format!("agent:{agent}");
        let asked = move |connection: &mut rusqlite::Connection| {
            let mut statement = connection.prepare(
                "SELECT channel FROM channel_members WHERE member = ?1 AND listens = 0 ORDER BY channel",
            )?;
            let rows = statement.query_map([&actor], |row| row.get(0))?;
            rows.collect::<rusqlite::Result<Vec<String>>>()
        };
        self.journal.db().call(asked).await
    }

    /// Whether the agent called `agent` would be brought into the channel
    /// `reader` is leave for, as things stand ([`Channels::bring`], which
    /// asks again under its turn): for whoever keeps agents to ask
    /// before it writes anything of one it is about to bring in.
    pub fn may_bring(&self, reader: &Reader, agent: &str) -> Result<()> {
        let Some(channel) = reader.channel().filter(|channel| self.memory.is_kept(channel)) else {
            return Ok(());
        };
        let alone = self.memory.alone.read().expect("members lock").contains(channel);
        match self.memory.not_brought(channel, alone, &format!("agent:{agent}")) {
            Some(why) => Err(Error::conflict(why)),
            None => Ok(()),
        }
    }

    /// Whether the agent called `agent` would be brought into a kept
    /// channel that is about to be made, holding its agents to itself
    /// or not as `alone` says: asked before one is made with it.
    pub fn may_bring_into_new(&self, alone: bool, agent: &str) -> Result<()> {
        match self.memory.not_brought("", alone, &format!("agent:{agent}")) {
            Some(why) => Err(Error::conflict(why)),
            None => Ok(()),
        }
    }

    /// The kept channel the agent `actor` is asked nowhere but in, by
    /// its id: one it is in that says so. For whoever gives agents work,
    /// and never to be handed to anybody.
    pub fn alone_in(&self, actor: &str) -> Option<String> {
        self.memory.holds_alone(actor, None)
    }

    /// Leave for the daemon itself to read and write any channel: for
    /// what it reads to do its own work and hands to nobody as it is.
    /// `why` says which work; every line that says this is on a list,
    /// with its reason, in a test that reads the source.
    pub fn daemon(why: &'static str) -> Reader {
        debug_assert!(!why.trim().is_empty(), "the daemon reads a channel for a reason, which is said");
        Reader { of: None, writes: true }
    }

    /// The channels `by` reads, by what each is called: an open one its
    /// name, one kept to its members its title.
    pub async fn readable(&self, by: &Principal) -> Result<Vec<Channel>> {
        by.require(READ)?;
        let rows = self.rows().await?.into_iter();
        let mut read: Vec<Channel> = rows.filter(|channel| channel.is_there_for(by, &self.memory)).collect();
        let called = |channel: &Channel| channel.title.as_deref().unwrap_or(&channel.name).to_lowercase();
        read.sort_by_cached_key(called);
        Ok(read)
    }

    /// The kept channels `by` does not read, by id; every kept one,
    /// where nobody is said. For leaving them out of something handed to
    /// `by` that lists channels by id (a workspace, what a search
    /// found), and never to be handed to anybody itself.
    pub async fn kept_from(&self, by: Option<&Principal>) -> Result<HashSet<String>> {
        let kept = self.rows().await?.into_iter().filter(|channel| channel.kept);
        let from = |channel: &Channel| by.is_none_or(|by| !channel.is_there_for(by, &self.memory));
        Ok(kept.filter(from).map(|one| one.id).collect())
    }

    /// Something attached and where its file is, for `by`, who reads the
    /// channel it was put in. One of a channel kept from them is not
    /// there.
    pub async fn attached(
        &self,
        id: &str,
        by: &Principal,
    ) -> Result<(crate::Attachment, std::path::PathBuf)> {
        by.require(READ)?;
        let (found, channel, path) = self.attachments.get(id).await?;
        match self.found(&channel, by).await {
            Ok(_) => Ok((found, path)),
            Err(error) if error.kind == kernel::ErrorKind::NotFound => Err(no_attachment(id)),
            Err(error) => Err(error),
        }
    }

    /// Has the memory let `actor` go from every channel, before a write
    /// somebody else makes takes them out of each
    /// ([`Channels::leave_all`], which is handed this).
    pub async fn leaving(&self, actor: &str) -> Leaving {
        let turn = self.memory.changing().await;
        Leaving { actor: actor.to_string(), within: self.memory.left_all(actor), _turn: turn }
    }

    /// The write that was to take somebody out of every channel was not
    /// made: they are in each as they were.
    pub fn stayed(&self, leaving: Leaving) {
        for channel in &leaving.within {
            self.memory.joined(channel, &leaving.actor);
        }
    }
}
