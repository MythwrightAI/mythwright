//! The inbox: a box of things addressed to somebody, a person or an
//! agent, with read and put away of their own.
//!
//! **A line** is a row of `inbox`, the factory's own whatever it is
//! about: that somebody was named in a channel (`named`), that something
//! was said in a conversation of theirs (`message`: one line a
//! conversation, raised by each thing somebody else says there), and
//! that a blueprint told them something (`told`, [`crate::telling`]),
//! and that they were named in something said on a record (`commented`,
//! [`crate::commented`]: about the comment, opening the record), and that
//! a page they follow changed (`followed`, [`crate::followed`]: one line a
//! page, raised by each change, opening the page). A
//! line is one for its owner, its kind, what it is about and why it is
//! there; one that is about nothing is one for each telling. It keeps no
//! words of a message: those are read from the message, as whoever
//! reads the box may read it then ([`crate::inboxed`]), so the box is no
//! second place where what was said is kept. What a blueprint told is
//! the blueprint's own words, and those the line keeps.
//!
//! **Passed over.** A line of something said in a channel its owner
//! does not read as things stand (the channel was kept to its members
//! without them, or they left it) is passed over, as one whose message
//! is gone is: not listed, not counted among what waits unread, and not
//! found by its id, so that nothing here says to anybody that something
//! is there which they may not read. It is asked each time, by the one
//! question every reading of a channel asks, and the line is as it was
//! when its owner comes to read the channel again.
//!
//! **One door.** Everything that puts a line in a box asks [`Inbox::tell`]
//! (or, inside a write of its own, [`Inbox::tell_in`]), which writes the
//! row and says `inbox.told` as one write: stopped anywhere, a line that
//! is there was said, and where it is told in the write that keeps what
//! it is about (a message that names somebody) both are there or
//! neither. It is the same done twice: a telling is known by what said
//! it ([`Tell::said_by`]; for somebody named in a message, the message
//! itself), and told again by the same it writes and says nothing. Told
//! by something else, the line there is is raised: unread again, back
//! from where it was put away, counted once more, and never a second
//! row.
//!
//! **Whose it is.** A box is read by its owner and nobody else: not the
//! factory's owner, and not whoever holds everything. So nothing here
//! takes an owner from whoever asks: the owner is who asks
//! (`Principal::actor`, which for a key somebody made is its maker), and
//! what reading and changing take ([`READ`], [`WRITE`]) is asked of what
//! they hold, so a key reads its maker's box only where the key itself
//! was given that. A line that is somebody else's answers as one that is
//! not there.
//!
//! **Who hears.** What is said of a box (`inbox.told`, `inbox.read`,
//! `inbox.unread`, `inbox.away`) carries no words, and is said as taking being of that
//! box (`member:inbox:<owner>`), which no scope answers for: who is told
//! is asked where who is in a kept channel is ([`crate::kept`]'s answer
//! for the journal), and is the owner, holding [`READ`].
//!
//! **Archived.** A line whose place is an archived channel stays,
//! listed and opening it, and waits unread for nobody: it is no part
//! of how many lines wait, and nothing said of a channel raises it
//! while the channel is archived ([`crate::archived`]).
//!
//! **What goes with what.** A channel that is taken away takes the lines
//! that open it, in the write that takes it away ([`Inbox::forget_place`]),
//! each owner told; whoever is taken away for good (an agent) takes the
//! box that was theirs, in the write that takes them
//! ([`Inbox::forget_owner`]), or a later one of the name would find it.

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use journal::{Journal, Writing, optional};
use kernel::principal::{MEMBER, covers};
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde::Serialize;
use serde_json::json;

use crate::channels::kind_of;
use crate::kept::Memory;

/// What reading one's own box takes.
pub const READ: &str = "inbox:read";
/// What saying a line was read, and putting one away, takes.
pub const WRITE: &str = "inbox:write";
/// How what follows `member:` begins in what an event of a box takes:
/// the owner, as an actor, is the rest.
const OF: &str = "inbox:";
/// The most lines one reading of a box hands out, the latest first.
pub const MOST: usize = 500;

pub(crate) const SCHEMA: &[&str] = &[
    "
    CREATE TABLE inbox (
        id TEXT PRIMARY KEY,
        owner TEXT NOT NULL,
        kind TEXT NOT NULL,
        about TEXT NOT NULL,
        place TEXT NOT NULL,
        came_from TEXT NOT NULL,
        why TEXT NOT NULL,
        says TEXT,
        said_by TEXT,
        count INTEGER NOT NULL,
        created INTEGER NOT NULL,
        raised INTEGER NOT NULL,
        read INTEGER,
        away INTEGER
    );
    CREATE UNIQUE INDEX inbox_one ON inbox (owner, kind, about);
    CREATE INDEX inbox_place ON inbox (place);
    CREATE INDEX inbox_kept ON inbox (owner, raised) WHERE away IS NULL;
    CREATE INDEX inbox_unread ON inbox (owner) WHERE read IS NULL AND away IS NULL;
",
    "
    DROP INDEX inbox_one;
    CREATE UNIQUE INDEX inbox_one ON inbox (owner, kind, about, why) WHERE about <> '';
    CREATE INDEX inbox_alone ON inbox (owner, said_by) WHERE about = '';
",
    "
    ALTER TABLE inbox ADD COLUMN parts TEXT;
",
];

/// What kind of thing a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Its owner was named in something said: about the message.
    Named,
    /// Something was said in a conversation of its owner's.
    Message,
    /// A blueprint told its owner something, in words of its own.
    Told,
    /// Its owner was named in something said on a record: about the
    /// comment, by its number, opening the record.
    Commented,
    /// A page its owner follows changed: about the page, one line a page,
    /// raised by each change (`crate::followed`), which it opens at.
    Followed,
    /// Something else its owner follows changed, or what changed in
    /// anything they follow in an hour or a day: about the thing (and the
    /// period), opening it, in words of its own (`crate::following`).
    Following,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Named => "named",
            Kind::Message => "message",
            Kind::Told => "told",
            Kind::Commented => "commented",
            Kind::Followed => "followed",
            Kind::Following => "following",
        }
    }

    fn parse(kept: &str) -> Kind {
        match kept {
            "message" => Kind::Message,
            "told" => Kind::Told,
            "commented" => Kind::Commented,
            "followed" => Kind::Followed,
            "following" => Kind::Following,
            _ => Kind::Named,
        }
    }
}

/// One line of somebody's box.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Line {
    pub id: String,
    /// Whose box it is in: `user:<account>` or `agent:<name>`.
    pub owner: String,
    pub kind: Kind,
    /// What it is about: for somebody named, the message; for what a
    /// blueprint told, the id of what it said it is about, or nothing.
    pub about: String,
    /// What opens it: a channel's id, or for what a blueprint told
    /// whatever it said opens it.
    pub place: String,
    /// The actor it came from.
    pub from: String,
    /// Why it is in the box, in a word.
    pub why: String,
    /// Its own words, for what a blueprint told: a line of something
    /// said keeps none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    /// What said it last: which telling this is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub said_by: Option<String>,
    /// What else a blueprint told it with, beside its words: its parts,
    /// each a mapping with a `kind` (`kernel::parts`), as they were last
    /// told. Nothing, for a line of something said, or one told without.
    /// Handed out without the cards a part kept (`kernel::parts::shown`).
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "parts_shown")]
    pub parts: Option<serde_json::Value>,
    /// How many times it was told.
    pub count: u32,
    /// When it was first told, and when it was last.
    pub created: i64,
    pub raised: i64,
    /// When its owner read it, where they have since it was last told.
    pub read: Option<i64>,
    /// When its owner put it away.
    pub away: Option<i64>,
}

fn parts_shown<S: serde::Serializer>(
    parts: &Option<serde_json::Value>,
    to: S,
) -> std::result::Result<S::Ok, S::Error> {
    serde::Serialize::serialize(&parts.as_ref().map(kernel::parts::shown), to)
}

/// Something to be put in a box.
#[derive(Clone, Debug)]
pub struct Tell {
    /// Whose box: `user:<account>` or `agent:<name>`, never a bot.
    pub owner: String,
    pub kind: Kind,
    pub about: String,
    pub place: String,
    pub from: String,
    pub why: String,
    pub says: Option<String>,
    /// What says it, where the same thing may be about many tellings (a
    /// step of a run, a message of a conversation): told again by the
    /// same, nothing is told. Nothing, where what it is about is itself
    /// the telling (somebody named in a message is named in it once).
    pub said_by: Option<String>,
    /// What a blueprint tells with beside its words, as they are kept.
    pub parts: Option<serde_json::Value>,
}

impl Tell {
    /// That `owner` was named in the message `message` of the channel
    /// `channel`, by `from`.
    pub fn named(owner: &str, message: &str, channel: &str, from: &str) -> Tell {
        Tell {
            owner: owner.to_string(),
            kind: Kind::Named,
            about: message.to_string(),
            place: channel.to_string(),
            from: from.to_string(),
            why: "named".to_string(),
            says: None,
            said_by: None,
            parts: None,
        }
    }

    /// That `message` was said by `from` in the conversation `channel`,
    /// which `owner` is in: one line a conversation, which each message
    /// raises.
    pub fn conversed(owner: &str, channel: &str, message: &str, from: &str) -> Tell {
        Tell {
            owner: owner.to_string(),
            kind: Kind::Message,
            about: channel.to_string(),
            place: channel.to_string(),
            from: from.to_string(),
            why: "message".to_string(),
            says: None,
            said_by: Some(message.to_string()),
            parts: None,
        }
    }
}

const LINE: &str =
    "id, owner, kind, about, place, came_from, why, says, said_by, count, created, raised, read, away, parts";

fn read_line(row: &rusqlite::Row<'_>) -> rusqlite::Result<Line> {
    Ok(Line {
        id: row.get(0)?,
        owner: row.get(1)?,
        kind: Kind::parse(&row.get::<_, String>(2)?),
        about: row.get(3)?,
        place: row.get(4)?,
        from: row.get(5)?,
        why: row.get(6)?,
        says: row.get(7)?,
        said_by: row.get(8)?,
        count: row.get(9)?,
        created: row.get(10)?,
        raised: row.get(11)?,
        read: row.get(12)?,
        away: row.get(13)?,
        parts: row.get::<_, Option<String>>(14)?.and_then(|kept| serde_json::from_str(&kept).ok()),
    })
}

/// What hearing an event of `owner`'s box takes.
pub(crate) fn takes(owner: &str) -> String {
    format!("{MEMBER}{OF}{owner}")
}

/// Whether whoever is `actor`, holding `held`, is told an event that
/// takes being a member of `of`, where that is somebody's box: nothing
/// where it is not one. The owner, holding what reading a box takes,
/// and nobody else whatever they hold.
pub(crate) fn tells(of: &str, actor: &str, held: &[String]) -> Option<bool> {
    of.strip_prefix(OF).map(|owner| owner == actor && covers(held, READ))
}

/// Whether an actor can own a box: a person or an agent.
pub(crate) fn owns(actor: &str) -> bool {
    matches!(kind_of(actor), "user" | "agent") && actor.contains(':')
}

/// Which of these lines of `owner`'s are passed over for whoever holds
/// `held`: a line of something said whose message is gone, which says
/// nothing any more, or whose channel they do not read as things
/// stand, which is to them a channel that is not there. Such a line is
/// not listed, not counted and not found by its id, and is as it was
/// when they come to read the channel again. Asked of the channel by
/// the one question every reading of it asks ([`crate::kept`]). The
/// message of a line of somebody named is what it is about; of a line
/// of a conversation, the one that raised it last.
fn passed_over(
    connection: &rusqlite::Connection,
    memory: &Memory,
    owner: &str,
    held: &[String],
    lines: &[String],
) -> rusqlite::Result<HashSet<String>> {
    let mut over = HashSet::new();
    // A line of something said on a record its owner may no longer read
    // as things stand, or that no table there is owns, is passed over the
    // same way, and so is one of a page they follow (a record too):
    // asked of what keeps records, which knows what reading
    // each takes now ([`crate::commented`]). With nothing said of that,
    // none is.
    if let Some(records) = memory.records.get() {
        let mut of_record = connection
            .prepare("SELECT place FROM inbox WHERE id = ?1 AND kind IN ('commented', 'followed')")?;
        for id in lines {
            let Some(record) = optional(of_record.query_row([id], |row| row.get::<_, String>(0)))? else {
                continue;
            };
            if !records.takes(&record).is_some_and(|takes| crate::commented::reads(held, &takes)) {
                over.insert(id.clone());
            }
        }
    }
    // A line of anything else somebody follows, of a thing they no longer
    // read as things stand ([`crate::following`]).
    let mut of_thing = connection.prepare("SELECT place FROM inbox WHERE id = ?1 AND kind = 'following'")?;
    for id in lines {
        let Some(place) = optional(of_thing.query_row([id], |row| row.get::<_, String>(0)))? else {
            continue;
        };
        if !crate::following::reads_thing(connection, memory, owner, held, &place)? {
            over.insert(id.clone());
        }
    }
    let mut of_line = connection.prepare(
        "SELECT c.kept, EXISTS (SELECT 1 FROM channel_members AS within
                                WHERE within.channel = c.id AND within.member = i.owner)
         FROM inbox i
         LEFT JOIN messages m ON m.id = CASE i.kind WHEN 'named' THEN i.about ELSE i.said_by END
         LEFT JOIN channels c ON c.id = m.channel
         WHERE i.id = ?1 AND i.kind IN ('named', 'message')",
    )?;
    for id in lines {
        let found: Option<(Option<bool>, bool)> =
            optional(of_line.query_row([id], |row| Ok((row.get(0)?, row.get(1)?))))?;
        // No row: it is no line of something said, and is never passed over.
        let Some((kept, member)) = found else { continue };
        if !kept.is_some_and(|kept| memory.reads_as(kept, member, owner, held)) {
            over.insert(id.clone());
        }
    }
    Ok(over)
}

/// The lines that wait unread in `owner`'s box, by id, for whoever
/// holds `held`: those that would be listed, but for a line whose place
/// is an archived channel, which is listed and opens it, and waits for
/// nobody ([`crate::archived`]).
fn unread_of(
    connection: &rusqlite::Connection,
    memory: &Memory,
    owner: &str,
    held: &[String],
) -> rusqlite::Result<Vec<String>> {
    let mut unread = connection.prepare(
        "SELECT id FROM inbox WHERE owner = ?1 AND read IS NULL AND away IS NULL
         AND place NOT IN (SELECT id FROM channels WHERE archived IS NOT NULL) ORDER BY rowid",
    )?;
    let lines: Vec<String> = unread.query_map([owner], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    let over = passed_over(connection, memory, owner, held, &lines)?;
    Ok(lines.into_iter().filter(|id| !over.contains(id)).collect())
}

/// Whether `place` is an archived channel, as the write under way finds it.
fn archived(connection: &rusqlite::Connection, place: &str) -> rusqlite::Result<bool> {
    let found =
        connection.query_row("SELECT archived IS NOT NULL FROM channels WHERE id = ?1", [place], |row| {
            row.get::<_, bool>(0)
        });
    Ok(optional(found)?.unwrap_or(false))
}

/// One of `owner`'s lines by its id, as the write under way finds it.
fn line_of(connection: &rusqlite::Connection, owner: &str, id: &str) -> rusqlite::Result<Option<Line>> {
    optional(connection.query_row(
        &format!("SELECT {LINE} FROM inbox WHERE id = ?1 AND owner = ?2"),
        [id, owner],
        read_line,
    ))
}

/// What a line that is not there answers, and so what somebody else's
/// answers.
pub(crate) fn no_line(id: &str) -> Error {
    Error::not_found(format!("there is no line `{id}` in your inbox"))
}

/// The keeper of every box.
#[derive(Clone)]
pub struct Inbox {
    journal: Journal,
    /// What says what somebody holds now ([`Inbox::holds_with`]).
    holding: Arc<OnceLock<Arc<Holding>>>,
    /// Who is in each kept channel, and which agents are of one as a
    /// person is: what whether a line's channel is read is asked of.
    pub(crate) memory: Arc<Memory>,
}

/// What says what an actor holds now (an account by what it is, an
/// agent by its credential), read in a write that is under way: for how
/// many lines wait unread in a box, which is said with whatever changes
/// the box and is of what its owner may read then.
pub type Holding = dyn Fn(&rusqlite::Connection, &str) -> rusqlite::Result<Vec<String>> + Send + Sync;

impl Inbox {
    pub(crate) async fn open(journal: Journal, memory: Arc<Memory>) -> Result<Inbox> {
        journal.db().migrate("inbox", SCHEMA).await?;
        Ok(Inbox { journal, holding: Arc::default(), memory })
    }

    /// What `owner` holds now, as the write under way reads it.
    pub(crate) fn held(
        &self,
        connection: &rusqlite::Connection,
        owner: &str,
    ) -> rusqlite::Result<Vec<String>> {
        match self.holding.get() {
            Some(holding) => holding(connection, owner),
            None => Ok(vec![crate::kept::READ.to_string()]),
        }
    }

    /// Says what answers what an actor holds now, for every keeper of
    /// these boxes. Said once; a second is not taken. Until it is said
    /// an owner is taken to read channels and nothing kept to its
    /// members but as one of them.
    pub fn holds_with(&self, holding: Arc<Holding>) {
        let _ = self.holding.set(holding);
    }

    /// Says what answers for a line of something said on a record (what
    /// reading the record takes now, and what was said as somebody may
    /// read it), for every keeper of these boxes: said by what keeps
    /// records. Said once; a second is not taken.
    pub fn records_with(&self, records: Arc<dyn crate::commented::OnRecords>) {
        let _ = self.memory.records.set(records);
    }

    /// What `owner` holds now, as the write under way finds it: nothing
    /// said, where nothing answers that here.
    pub(crate) fn holds_now(
        &self,
        connection: &rusqlite::Connection,
        owner: &str,
    ) -> rusqlite::Result<Option<Vec<String>>> {
        self.holding.get().map(|holding| holding(connection, owner)).transpose()
    }

    /// How many lines wait unread in `owner`'s box, as its owner may
    /// read things in the write under way.
    fn waiting(&self, connection: &rusqlite::Connection, owner: &str) -> rusqlite::Result<usize> {
        let held = self.held(connection, owner)?;
        Ok(unread_of(connection, &self.memory, owner, &held)?.len())
    }

    /// Says of lines that were read or put away which they were, where
    /// they were all of one place that place, and how many now wait.
    fn said_of(
        &self,
        writing: &mut Writing<'_>,
        topic: &str,
        owner: &str,
        lines: &[String],
        place: Option<&str>,
    ) -> rusqlite::Result<()> {
        let unread = self.waiting(writing.transaction(), owner)?;
        let mut data = json!({ "owner": owner, "lines": lines, "unread": unread });
        if let Some(place) = place {
            data["place"] = json!(place);
        }
        writing.say_here(topic, Source::actor(owner).taking(Some(&takes(owner))), data).map(|_| ())
    }

    /// Puts a line in a box and says so, as one write; the line as it
    /// then is, or nothing where this very telling was told before.
    pub async fn tell(&self, tell: Tell) -> Result<Option<Line>> {
        if !owns(&tell.owner) {
            return Err(Error::invalid(format!(
                "`{}` has no inbox: a box is a person's (`user:<account>`) or an agent's (`agent:<name>`)",
                tell.owner
            )));
        }
        let (inbox, now) = (self.clone(), self.journal.now_ms());
        Ok(self.journal.write(move |writing| inbox.tell_in(writing, &tell, now)).await?.0)
    }

    /// The same, in a write somebody else is making: what is told is
    /// kept with whatever that write keeps, or not at all. Nothing is
    /// told an actor that can own no box.
    pub fn tell_in(
        &self,
        writing: &mut Writing<'_>,
        tell: &Tell,
        now: i64,
    ) -> rusqlite::Result<Option<Line>> {
        if !owns(&tell.owner) {
            return Ok(None);
        }
        let transaction = writing.transaction();
        // What is said in a channel puts nothing in a box while it is
        // archived, and raises no line there is of it: nothing is said
        // there, and whoever comes into one is told nothing of it.
        if matches!(tell.kind, Kind::Named | Kind::Message) && archived(transaction, &tell.place)? {
            return Ok(None);
        }
        // The line there is: of what this is about and why, or, about
        // nothing, the one this very telling put there. Each says what
        // its index says, word for word.
        let found = |row: &rusqlite::Row<'_>| Ok((row.get(0)?, row.get(1)?));
        let there: Option<(String, Option<String>)> = optional(match tell.about.is_empty() {
            false => transaction.query_row(
                "SELECT id, said_by FROM inbox
                 WHERE owner = ?1 AND kind = ?2 AND about = ?3 AND why = ?4 AND about <> ''",
                params![tell.owner, tell.kind.as_str(), tell.about, tell.why],
                found,
            ),
            true => transaction.query_row(
                "SELECT id, said_by FROM inbox
                 WHERE owner = ?1 AND about = '' AND said_by IS ?2 AND kind = ?3 AND why = ?4",
                params![tell.owner, tell.said_by, tell.kind.as_str(), tell.why],
                found,
            ),
        })?;
        let parts = tell.parts.as_ref().map(serde_json::Value::to_string);
        let id = match there {
            // This very telling, done twice: told once.
            Some((_, said_by)) if said_by == tell.said_by => return Ok(None),
            Some((id, _)) => {
                transaction.execute(
                    "UPDATE inbox SET place = ?2, came_from = ?3, why = ?4, says = ?5, said_by = ?6,
                            count = count + 1, raised = ?7, read = NULL, away = NULL, parts = ?8
                     WHERE id = ?1",
                    params![id, tell.place, tell.from, tell.why, tell.says, tell.said_by, now, parts],
                )?;
                id
            }
            None => {
                let id = kernel::id::new("inb", now);
                transaction.execute(
                    &format!(
                        "INSERT INTO inbox ({LINE}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?10, NULL, NULL, ?11)"
                    ),
                    params![
                        id,
                        tell.owner,
                        tell.kind.as_str(),
                        tell.about,
                        tell.place,
                        tell.from,
                        tell.why,
                        tell.says,
                        tell.said_by,
                        now,
                        parts
                    ],
                )?;
                id
            }
        };
        let line = line_of(transaction, &tell.owner, &id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let data = json!({
            "owner": line.owner,
            "line": line.id,
            "kind": line.kind,
            "place": line.place,
            "about": line.about,
            "count": line.count,
            "unread": self.waiting(transaction, &line.owner)?,
        });
        writing.say_here("inbox.told", Source::actor(&line.from).taking(Some(&takes(&line.owner))), data)?;
        Ok(Some(line))
    }

    /// The lines of `by`'s own box as they may read things now, the
    /// latest told first, [`MOST`] at most: those not put away, or with
    /// `all` every one. A line that is passed over is not among them.
    pub async fn list(&self, by: &Principal, all: bool) -> Result<Vec<Line>> {
        by.require(READ)?;
        let (owner, held, memory) = (by.actor(), by.scopes.clone(), self.memory.clone());
        self.journal
            .db()
            .call(move |connection| {
                // Each way says what its index says, word for word.
                let which = if all { "" } else { "AND away IS NULL" };
                let mut statement = connection.prepare(&format!(
                    "SELECT {LINE} FROM inbox WHERE owner = ?1 {which} ORDER BY raised DESC, rowid DESC LIMIT {MOST}"
                ))?;
                let lines: Vec<Line> = statement.query_map([&owner], read_line)?.collect::<rusqlite::Result<_>>()?;
                let ids: Vec<String> = lines.iter().map(|line| line.id.clone()).collect();
                let over = passed_over(connection, &memory, &owner, &held, &ids)?;
                Ok(lines.into_iter().filter(|line| !over.contains(&line.id)).collect())
            })
            .await
    }

    /// How many lines wait unread in `by`'s own box, of those they are
    /// shown.
    pub async fn unread(&self, by: &Principal) -> Result<u64> {
        by.require(READ)?;
        let (owner, held, memory) = (by.actor(), by.scopes.clone(), self.memory.clone());
        let waiting = self.journal.db().call(move |connection| unread_of(connection, &memory, &owner, &held));
        Ok(waiting.await?.len() as u64)
    }

    /// The line `id` of `owner`'s box as the write under way finds it,
    /// for whoever holds `held`: nothing for one that is passed over.
    fn theirs(
        &self,
        connection: &rusqlite::Connection,
        owner: &str,
        held: &[String],
        id: &str,
    ) -> rusqlite::Result<Option<Line>> {
        let Some(line) = line_of(connection, owner, id)? else { return Ok(None) };
        let over = passed_over(connection, &self.memory, owner, held, std::slice::from_ref(&line.id))?;
        Ok(over.is_empty().then_some(line))
    }

    /// Says that `by` read the line `id` of their own box. One that was
    /// read already is as it was, and nothing is said.
    pub async fn read(&self, by: &Principal, id: &str) -> Result<Line> {
        by.require(WRITE)?;
        let (inbox, owner, held) = (self.clone(), by.actor(), by.scopes.clone());
        let (lookup, now) = (id.to_string(), self.journal.now_ms());
        let wrote = self.journal.write(move |writing| {
            let found = inbox.theirs(writing.transaction(), &owner, &held, &lookup)?;
            let Some(mut line) = found else { return Ok(None) };
            if line.read.is_none() {
                writing
                    .transaction()
                    .execute("UPDATE inbox SET read = ?2 WHERE id = ?1", params![line.id, now])?;
                line.read = Some(now);
                inbox.said_of(
                    writing,
                    "inbox.read",
                    &owner,
                    std::slice::from_ref(&line.id),
                    Some(&line.place),
                )?;
            }
            Ok(Some(line))
        });
        wrote.await?.0.ok_or_else(|| no_line(id))
    }

    /// The line `id` of `by`'s own box as they may read things now, put
    /// away or not: one that is passed over, or is somebody else's,
    /// answers as one that is not there.
    pub async fn one(&self, by: &Principal, id: &str) -> Result<Line> {
        by.require(READ)?;
        let (inbox, owner, held, lookup) = (self.clone(), by.actor(), by.scopes.clone(), id.to_string());
        let found =
            self.journal.db().call(move |connection| inbox.theirs(connection, &owner, &held, &lookup));
        found.await?.ok_or_else(|| no_line(id))
    }

    /// Says that `by` would have the line `id` of their own box wait
    /// unread again, as though it had not been read: it is counted among
    /// what waits, and stays where it is listed. One that waits unread
    /// already is as it was, and nothing is said.
    pub async fn unread_again(&self, by: &Principal, id: &str) -> Result<Line> {
        by.require(WRITE)?;
        let (inbox, owner, held) = (self.clone(), by.actor(), by.scopes.clone());
        let lookup = id.to_string();
        let wrote = self.journal.write(move |writing| {
            let found = inbox.theirs(writing.transaction(), &owner, &held, &lookup)?;
            let Some(mut line) = found else { return Ok(None) };
            if line.read.is_some() {
                writing.transaction().execute("UPDATE inbox SET read = NULL WHERE id = ?1", [&line.id])?;
                line.read = None;
                inbox.said_of(
                    writing,
                    "inbox.unread",
                    &owner,
                    std::slice::from_ref(&line.id),
                    Some(&line.place),
                )?;
            }
            Ok(Some(line))
        });
        wrote.await?.0.ok_or_else(|| no_line(id))
    }

    /// Says that `by` read every line of their own box that opens
    /// `place`, or with no place every line there is: those they are
    /// shown. How many were unread until now; where none was, nothing
    /// is written or said.
    pub async fn read_in(&self, by: &Principal, place: Option<&str>) -> Result<usize> {
        by.require(WRITE)?;
        let (inbox, owner, held) = (self.clone(), by.actor(), by.scopes.clone());
        let (place, now) = (place.map(str::to_string), self.journal.now_ms());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let mut unread = transaction.prepare(
                "SELECT id FROM inbox WHERE owner = ?1 AND read IS NULL AND (?2 IS NULL OR place = ?2) ORDER BY rowid",
            )?;
            let lines: Vec<String> =
                unread.query_map(params![owner, place], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
            let over = passed_over(transaction, &inbox.memory, &owner, &held, &lines)?;
            let lines: Vec<String> = lines.into_iter().filter(|id| !over.contains(id)).collect();
            let mut mark = transaction.prepare("UPDATE inbox SET read = ?2 WHERE id = ?1")?;
            for id in &lines {
                mark.execute(params![id, now])?;
            }
            if !lines.is_empty() {
                inbox.said_of(writing, "inbox.read", &owner, &lines, place.as_deref())?;
            }
            Ok(lines.len())
        });
        Ok(wrote.await?.0)
    }

    /// Puts the line `id` of `by`'s own box away: it is listed no longer
    /// unless everything is asked for, waits unread no longer, and is
    /// back where it is told again. One that was put away already is as
    /// it was, and nothing is said.
    pub async fn away(&self, by: &Principal, id: &str) -> Result<Line> {
        by.require(WRITE)?;
        let (inbox, owner, held) = (self.clone(), by.actor(), by.scopes.clone());
        let (lookup, now) = (id.to_string(), self.journal.now_ms());
        let wrote = self.journal.write(move |writing| {
            let found = inbox.theirs(writing.transaction(), &owner, &held, &lookup)?;
            let Some(mut line) = found else { return Ok(None) };
            if line.away.is_none() {
                writing
                    .transaction()
                    .execute("UPDATE inbox SET away = ?2 WHERE id = ?1", params![line.id, now])?;
                line.away = Some(now);
                inbox.said_of(
                    writing,
                    "inbox.away",
                    &owner,
                    std::slice::from_ref(&line.id),
                    Some(&line.place),
                )?;
            }
            Ok(Some(line))
        });
        wrote.await?.0.ok_or_else(|| no_line(id))
    }

    /// Takes away every line that opens `place`, in the write that takes
    /// the place away and before anything of the place is gone, and
    /// tells each owner which of theirs went: of the lines they were
    /// shown, so that somebody the place was kept from is told nothing
    /// of it going. How many lines went.
    pub(crate) fn forget_place(&self, writing: &mut Writing<'_>, place: &str) -> rusqlite::Result<usize> {
        let transaction = writing.transaction();
        let mut of_place =
            transaction.prepare("SELECT owner, id FROM inbox WHERE place = ?1 ORDER BY owner, rowid")?;
        let found: Vec<(String, String)> = of_place
            .query_map([place], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        if found.is_empty() {
            return Ok(0);
        }
        let mut owners: Vec<(String, Vec<String>)> = Vec::new();
        for (owner, id) in &found {
            match owners.last_mut() {
                Some((last, lines)) if last == owner => lines.push(id.clone()),
                _ => owners.push((owner.clone(), vec![id.clone()])),
            }
        }
        // Which of them each owner was shown, asked while the place is
        // still there to be asked of.
        for (owner, lines) in &mut owners {
            let held = self.held(transaction, owner)?;
            let over = passed_over(transaction, &self.memory, owner, &held, lines)?;
            lines.retain(|id| !over.contains(id));
        }
        transaction.execute("DELETE FROM inbox WHERE place = ?1", [place])?;
        for (owner, lines) in owners.iter().filter(|(_, lines)| !lines.is_empty()) {
            self.said_of(writing, "inbox.away", owner, lines, Some(place))?;
        }
        Ok(found.len())
    }

    /// Takes away the box that was `owner`'s, in the write that takes
    /// its owner away for good, so that nobody later called the same
    /// finds it. Nobody is told: whose it was is gone. How many lines
    /// went.
    pub fn forget_owner(writing: &Writing<'_>, owner: &str) -> rusqlite::Result<usize> {
        writing.transaction().execute("DELETE FROM inbox WHERE owner = ?1", [owner])
    }

    /// Tells the owner of the line `id`, which was just put back as it
    /// was with the place it opens (a channel given back from the Trash),
    /// that it is in their box again, as a line told is.
    pub(crate) fn told_back(&self, writing: &mut Writing<'_>, owner: &str, id: &str) -> rusqlite::Result<()> {
        let transaction = writing.transaction();
        let Some(line) = line_of(transaction, owner, id)? else { return Ok(()) };
        let data = json!({
            "owner": line.owner,
            "line": line.id,
            "kind": line.kind,
            "place": line.place,
            "about": line.about,
            "count": line.count,
            "unread": self.waiting(transaction, &line.owner)?,
        });
        writing.say_here("inbox.told", Source::actor(&line.from).taking(Some(&takes(&line.owner))), data)?;
        Ok(())
    }
}
