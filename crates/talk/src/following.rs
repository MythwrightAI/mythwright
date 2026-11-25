//! What somebody follows, and how often they are told of what changed in
//! it: a page of a wiki (with every page under it, or alone), a board, a
//! table, a library, a channel or a project.
//!
//! **A follow** is a row of `follows`, the factory's own: who, what (its
//! kind and id, `kernel::following`), whether a page is followed with its
//! pages, how often they are told and at what time of day a daily digest
//! comes. Whether a page is followed at all is still the page's own
//! (`followers`, written by the keeper of topics); its row says how. Every
//! change of one is one write with what is said of it (`inbox.follows`),
//! heard by its owner alone.
//!
//! **At once.** A page's line is as it always was ([`crate::followed`]).
//! Anything else has one line of the kind `following` in its follower's
//! box, raised by each change and saying what changed since they last read
//! it ("Shop board: 3 cards changed"): each change is a row of
//! `follow_changes` that names the line it was told in, which is the list
//! the line opens.
//!
//! **A digest.** Followed hourly or daily, a change waits as a row with no
//! line, and once its period is over all that waits is told as one line
//! of that period ("Frogs: 6 pages changed, 3 new replies"), which names
//! each row it told, in one write: stopped before it, nothing was told and
//! everything still waits; stopped after it, nothing waits to be told
//! again. Nothing waiting, no line. Whoever keeps the time asks what waits
//! ([`Channels::waiting`]) and is woken as something comes to
//! ([`Held::wakes`]).
//!
//! **Only what they may read.** Asked in the write that tells, of what the
//! follower holds then: of the thing ([`reads_thing`]), and of the change,
//! by whoever tells, who knows what hearing it takes. A line of a thing
//! they no longer read is passed over, as a line of a record is.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use journal::{Journal, Writing, optional};
use kernel::following::{Often, key, of_key};
use kernel::principal::covers;
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde::Serialize;
use serde_json::{Value, json};

use crate::channels::Channels;
use crate::commented::reads;
use crate::inbox::{Inbox, Kind, Line, Tell, owns};
use crate::kept::Memory;

/// Why a line of something followed is in a box: told at once, or as a digest.
pub const WHY: &str = "follows";
pub const DIGEST: &str = "digest";
/// What reading a project takes, for whoever follows one.
pub const PROJECT_READ: &str = "projects:read";

pub(crate) const SCHEMA: &[&str] = &["
    CREATE TABLE follows (
        who TEXT NOT NULL,
        kind TEXT NOT NULL,
        thing TEXT NOT NULL,
        pages INTEGER NOT NULL,
        often TEXT NOT NULL,
        at TEXT NOT NULL,
        since INTEGER NOT NULL,
        told INTEGER NOT NULL,
        made INTEGER NOT NULL,
        PRIMARY KEY (who, kind, thing)
    ) WITHOUT ROWID;
    CREATE INDEX follows_thing ON follows (thing, kind);
    CREATE TABLE follow_changes (
        who TEXT NOT NULL,
        kind TEXT NOT NULL,
        thing TEXT NOT NULL,
        seq INTEGER NOT NULL,
        record TEXT NOT NULL,
        noun TEXT NOT NULL,
        verb TEXT NOT NULL,
        topic TEXT NOT NULL,
        takes TEXT,
        came_from TEXT NOT NULL,
        at INTEGER NOT NULL,
        line TEXT,
        PRIMARY KEY (who, kind, thing, seq)
    ) WITHOUT ROWID;
    CREATE INDEX follow_changes_line ON follow_changes (who, line);
"];

/// One thing somebody follows, and how.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Follow {
    /// Who, as an actor.
    pub who: String,
    pub kind: String,
    pub thing: String,
    /// For a page, whether every page under it is followed with it.
    pub pages: bool,
    #[serde(serialize_with = "often_said")]
    pub often: Often,
    /// The time of day a daily digest comes at, in the follower's zone.
    pub at: String,
    /// The journal's number as it was followed: a page's change before it
    /// is told to nobody who follows a page above it.
    #[serde(skip)]
    pub since: u64,
    /// The last change of a page a digest told.
    #[serde(skip)]
    pub told: u64,
    /// When it was followed: a change of anything else before it is told
    /// to nobody.
    pub made: i64,
}

fn often_said<S: serde::Serializer>(often: &Often, to: S) -> std::result::Result<S::Ok, S::Error> {
    to.serialize_str(often.as_str())
}

const FOLLOW: &str = "who, kind, thing, pages, often, at, since, told, made";

fn read_follow(row: &rusqlite::Row<'_>) -> rusqlite::Result<Follow> {
    Ok(Follow {
        who: row.get(0)?,
        kind: row.get(1)?,
        thing: row.get(2)?,
        pages: row.get(3)?,
        often: Often::kept(&row.get::<_, String>(4)?),
        at: row.get(5)?,
        since: row.get::<_, i64>(6)? as u64,
        told: row.get::<_, i64>(7)? as u64,
        made: row.get(8)?,
    })
}

/// What a follow is said to be: how often, and of a page whether with its pages.
#[derive(Clone, Debug)]
pub struct Setting {
    pub pages: bool,
    pub often: Often,
    pub at: String,
}

/// One change of something followed, as it is kept to be told and listed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Pended {
    /// Its number in the journal.
    pub seq: u64,
    /// What it changed: the page, the row, the message. What that is
    /// called is read as the list is, of what is there then: nothing of
    /// what was said is kept here.
    pub record: String,
    /// What it was, in a word or two (`kernel::following::said`).
    pub noun: String,
    pub verb: String,
    /// What was said of it, and what hearing that takes beside its topic.
    pub topic: String,
    #[serde(skip)]
    pub takes: Option<String>,
    /// Who made it, as an actor.
    pub from: String,
    pub at: i64,
}

const PENDED: &str = "seq, record, noun, verb, topic, takes, came_from, at";

fn read_pended(row: &rusqlite::Row<'_>) -> rusqlite::Result<Pended> {
    Ok(Pended {
        seq: row.get::<_, i64>(0)? as u64,
        record: row.get(1)?,
        noun: row.get(2)?,
        verb: row.get(3)?,
        topic: row.get(4)?,
        takes: row.get(5)?,
        from: row.get(6)?,
        at: row.get(7)?,
    })
}

/// What is held of follows beside the tables: what wakes whoever keeps the
/// time, and which things anybody follows, so that a change of nothing
/// followed costs no reading.
pub struct Held {
    wakes: Arc<tokio::sync::Notify>,
    followed: RwLock<HashSet<String>>,
}

impl Held {
    pub(crate) async fn open(journal: &Journal) -> Result<Held> {
        journal.db().migrate("follows", SCHEMA).await?;
        let keys = journal
            .db()
            .call(|connection| {
                let mut statement = connection.prepare("SELECT DISTINCT kind, thing FROM follows")?;
                let found = statement
                    .query_map([], |row| Ok(grouped(&row.get::<_, String>(0)?, &row.get::<_, String>(1)?)))?;
                found.collect::<rusqlite::Result<HashSet<String>>>()
            })
            .await?;
        Ok(Held { wakes: Arc::default(), followed: RwLock::new(keys) })
    }

    /// What is woken as a change comes to wait for a digest.
    pub fn wakes(&self) -> Arc<tokio::sync::Notify> {
        self.wakes.clone()
    }

    /// Whether anybody follows the thing `kind` and `thing` name (a
    /// table's rows by `table`, whichever kind of table it is).
    pub fn anybody(&self, kind: &str, thing: &str) -> bool {
        self.followed.read().expect("follows lock").contains(&key(kind, thing))
    }
}

/// The key a thing is held by in memory: a board, a table and a library are
/// one kind of thing to a change of a row.
fn grouped(kind: &str, thing: &str) -> String {
    let kind = if kernel::following::TABLES.contains(&kind) { "table" } else { kind };
    key(kind, thing)
}

/// How `who` follows the thing, as the write under way finds it.
pub(crate) fn how(
    connection: &rusqlite::Connection,
    who: &str,
    kind: &str,
    thing: &str,
) -> rusqlite::Result<Option<Follow>> {
    optional(connection.query_row(
        &format!("SELECT {FOLLOW} FROM follows WHERE who = ?1 AND kind = ?2 AND thing = ?3"),
        params![who, kind, thing],
        read_follow,
    ))
}

/// Everybody who follows the thing `kind` names (a table's rows by `table`).
fn of_thing(connection: &rusqlite::Connection, kind: &str, thing: &str) -> rusqlite::Result<Vec<Follow>> {
    let kinds: Vec<&str> = if kind == "table" { kernel::following::TABLES.to_vec() } else { vec![kind] };
    let mut statement = connection.prepare(&format!(
        "SELECT {FOLLOW} FROM follows WHERE thing = ?1 AND kind IN ('{}') ORDER BY who",
        kinds.join("', '")
    ))?;
    statement.query_map([thing], read_follow)?.collect()
}

/// Whether whoever is `owner`, holding `held`, reads the thing `key` names,
/// as the write under way finds it: a page and a table as what keeps
/// records says reading it takes, a channel as every reading of one asks,
/// and a project by the scope reading projects takes.
pub(crate) fn reads_thing(
    connection: &rusqlite::Connection,
    memory: &Memory,
    owner: &str,
    held: &[String],
    key: &str,
) -> rusqlite::Result<bool> {
    let Some((kind, thing)) = of_key(key) else { return Ok(false) };
    let records = memory.records.get();
    Ok(match kind {
        "page" => records.and_then(|records| records.takes(thing)).is_some_and(|takes| reads(held, &takes)),
        "project" => covers(held, PROJECT_READ),
        "channel" => {
            let found: Option<(bool, bool)> = optional(connection.query_row(
                "SELECT kept, EXISTS (SELECT 1 FROM channel_members WHERE channel = id AND member = ?2)
                 FROM channels WHERE id = ?1",
                params![thing, owner],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ))?;
            found.is_some_and(|(kept, member)| memory.reads_as(kept, member, owner, held))
        }
        _ => thing
            .split_once('/')
            .and_then(|(project, name)| records?.table_takes(project, name))
            .is_some_and(|takes| reads(held, &takes)),
    })
}

/// Keeps a change for `follow`'s next digest, where it is not kept or told
/// already. Whether it was kept.
pub(crate) fn pend(
    connection: &rusqlite::Connection,
    follow: &Follow,
    pended: &Pended,
) -> rusqlite::Result<bool> {
    if follow.kind == "page" && pended.seq <= follow.told {
        return Ok(false);
    }
    keep_change(connection, follow, pended, None)
}

fn keep_change(
    connection: &rusqlite::Connection,
    follow: &Follow,
    pended: &Pended,
    line: Option<&str>,
) -> rusqlite::Result<bool> {
    let kept = connection.execute(
        &format!("INSERT OR IGNORE INTO follow_changes (who, kind, thing, line, {PENDED}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"),
        params![
            follow.who,
            follow.kind,
            follow.thing,
            line,
            pended.seq as i64,
            pended.record,
            pended.noun,
            pended.verb,
            pended.topic,
            pended.takes,
            pended.from,
            pended.at
        ],
    )?;
    Ok(kept > 0)
}

/// The changes a line names, in the order they were made.
fn of_line(connection: &rusqlite::Connection, who: &str, line: &str) -> rusqlite::Result<Vec<Pended>> {
    let mut statement = connection.prepare(&format!(
        "SELECT {PENDED} FROM follow_changes WHERE who = ?1 AND line = ?2 ORDER BY at, seq"
    ))?;
    statement.query_map(params![who, line], read_pended)?.collect()
}

/// Tells `follow`'s owner at once of a change of what they follow, called
/// `title`: one line of the thing, raised, saying what changed since they
/// last read it. Whether they were told: not where this very change was.
pub(crate) fn tell_at_once(
    writing: &mut Writing<'_>,
    inbox: &Inbox,
    follow: &Follow,
    pended: &Pended,
    title: &str,
) -> rusqlite::Result<bool> {
    let about = key(&follow.kind, &follow.thing);
    let transaction = writing.transaction();
    // What they read is said no more: the line counts from what came after.
    let seen: Option<bool> = optional(transaction.query_row(
        "SELECT read IS NOT NULL OR away IS NOT NULL FROM inbox WHERE owner = ?1 AND kind = ?2 AND about = ?3 AND why = ?4 AND about <> ''",
        params![follow.who, Kind::Following.as_str(), about, WHY],
        |row| row.get(0),
    ))?;
    if seen == Some(true) {
        transaction
            .execute("DELETE FROM follow_changes WHERE who = ?1 AND line = ?2", params![follow.who, about])?;
    }
    if !keep_change(transaction, follow, pended, Some(&about))? {
        return Ok(false);
    }
    let changes = of_line(transaction, &follow.who, &about)?;
    let counts = kernel::following::counted(changes.iter().map(|one| (one.noun.as_str(), one.verb.as_str())));
    let tell = Tell {
        owner: follow.who.clone(),
        kind: Kind::Following,
        about: about.clone(),
        place: about,
        from: pended.from.clone(),
        why: WHY.to_string(),
        says: Some(kernel::following::line_said(title, &counts)),
        said_by: Some(pended.seq.to_string()),
        parts: None,
    };
    Ok(inbox.tell_in(writing, &tell, pended.at)?.is_some())
}

/// A change of anything but a page, as whoever tells of it hands it over.
pub struct Told {
    /// What it is of, each its kind (`table` for a table's rows, whichever
    /// kind of table it is), its id and what that is called.
    pub things: Vec<(String, String, String)>,
    pub pended: Pended,
    /// Whose doing it was, which is never told it.
    pub by: String,
    /// When it was said, which only those who followed by then are told of.
    pub ts: i64,
}

/// What says whether whoever is an actor, holding what they hold, may be
/// told of a change (what was said of it and what hearing that takes).
pub type May = dyn Fn(&str, &[String], &str, Option<&str>) -> bool + Send + Sync;

impl Channels {
    /// What is held of follows.
    pub fn follows(&self) -> &Held {
        &self.following
    }

    /// Whoever follows the things `told` is of is told, each as they follow
    /// it, where they may read it and the change; and `keep`, a place, is
    /// kept in the same write, so that a change is told once whatever
    /// stops. How many were told or kept a change for a digest.
    pub async fn tell_things(
        &self,
        told: Told,
        may: Arc<May>,
        keep: Option<(&'static str, &'static str, Value)>,
    ) -> Result<usize> {
        let (inbox, memory) = (self.inbox.clone(), self.memory.clone());
        let wrote = self.journal.write(move |writing| {
            let (mut told_now, mut waits) = (0, false);
            for (kind, thing, title) in &told.things {
                for follow in of_thing(writing.transaction(), kind, thing)? {
                    let follower = follow.who.as_str();
                    if !owns(follower)
                        || follower == told.by
                        || follower == told.pended.from
                        || follow.made > told.ts
                    {
                        continue;
                    }
                    let held = inbox.held(writing.transaction(), follower)?;
                    let place = key(&follow.kind, &follow.thing);
                    if !may(follower, &held, &told.pended.topic, told.pended.takes.as_deref())
                        || !reads_thing(writing.transaction(), &memory, follower, &held, &place)?
                    {
                        continue;
                    }
                    let pended = &told.pended;
                    match follow.often {
                        Often::Off => {}
                        Often::Once => {
                            told_now += usize::from(tell_at_once(writing, &inbox, &follow, pended, title)?)
                        }
                        Often::Hourly | Often::Daily => {
                            let kept = pend(writing.transaction(), &follow, pended)?;
                            waits |= kept;
                            told_now += usize::from(kept);
                        }
                    }
                }
            }
            if let Some((namespace, name, place)) = &keep {
                writing.keep(namespace, name, place)?;
            }
            Ok((told_now, waits))
        });
        let (told, waits) = wrote.await?.0;
        if waits {
            self.following.wakes.notify_one();
        }
        Ok(told)
    }

    /// Everything `by` follows, in the order they followed each.
    pub async fn following(&self, by: &Principal) -> Result<Vec<Follow>> {
        by.require(crate::inbox::READ)?;
        let who = by.actor();
        let found = self.journal.db().call(move |connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT {FOLLOW} FROM follows WHERE who = ?1 ORDER BY made, kind, thing"
            ))?;
            statement.query_map([&who], read_follow)?.collect::<rusqlite::Result<Vec<Follow>>>()
        });
        found.await
    }

    /// How `by` follows one thing, where they do.
    pub async fn follow_of(&self, by: &Principal, kind: &str, thing: &str) -> Result<Option<Follow>> {
        by.require(crate::inbox::READ)?;
        let (who, kind, thing) = (by.actor(), kind.to_string(), thing.to_string());
        self.journal.db().call(move |connection| how(connection, &who, &kind, &thing)).await
    }

    /// `by` follows the thing as `setting` says, from now on: what they
    /// followed already is followed as it says now, and what waited for a
    /// digest that is told no more goes. Whether they may read it is asked
    /// of what they hold in the same write; one they may not is refused.
    pub async fn follow(&self, by: &Principal, kind: &str, thing: &str, setting: Setting) -> Result<Follow> {
        by.require(crate::inbox::WRITE)?;
        let who = by.actor();
        if !owns(&who) || by.kind == kernel::PrincipalKind::Agent {
            return Err(Error::invalid("only a person follows something: an agent is told by being named"));
        }
        let (memory, now) = (self.memory.clone(), self.journal.now_ms());
        let (kind, thing, held) = (kind.to_string(), thing.to_string(), by.scopes.clone());
        let (asked, place) = (by.clone(), key(&kind, &thing));
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            if !reads_thing(transaction, &memory, &who, &held, &place)? {
                return Ok(Err(Error::denied(format!(
                    "{} may not read `{thing}`, and so cannot follow it",
                    asked.name
                ))));
            }
            let was = how(transaction, &who, &kind, &thing)?;
            let often = setting.often.as_str();
            if was.as_ref().is_some_and(|was| {
                was.pages == setting.pages && was.often == setting.often && was.at == setting.at
            }) {
                return Ok(Ok(was.expect("followed")));
            }
            if !setting.often.digests() {
                transaction.execute(
                    "DELETE FROM follow_changes WHERE who = ?1 AND kind = ?2 AND thing = ?3 AND line IS NULL",
                    params![who, kind, thing],
                )?;
            }
            let data = json!({
                "owner": who, "kind": kind, "thing": thing, "follows": true,
                "pages": setting.pages, "often": often, "at": setting.at,
            });
            let seq = writing.say_here(
                "inbox.follows",
                Source::actor(&who).taking(Some(&crate::inbox::takes(&who))),
                data,
            )?;
            let (since, made) = was.as_ref().map_or((seq, now), |was| (was.since, was.made));
            writing.transaction().execute(
                &format!(
                    "INSERT INTO follows ({FOLLOW}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)
                     ON CONFLICT (who, kind, thing) DO UPDATE
                     SET pages = excluded.pages, often = excluded.often, at = excluded.at"
                ),
                params![who, kind, thing, setting.pages, often, setting.at, since as i64, made],
            )?;
            Ok(Ok(how(writing.transaction(), &who, &kind, &thing)?.expect("just written")))
        });
        let follow = wrote.await?.0?;
        self.following.followed.write().expect("follows lock").insert(grouped(&follow.kind, &follow.thing));
        self.following.wakes.notify_one();
        Ok(follow)
    }

    /// `by` follows the thing no longer: its row goes, and every change it
    /// kept, waiting or told. Whether they followed it.
    pub async fn unfollow(&self, by: &Principal, kind: &str, thing: &str) -> Result<bool> {
        by.require(crate::inbox::WRITE)?;
        let (who, kind, thing) = (by.actor(), kind.to_string(), thing.to_string());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let gone = transaction.execute(
                "DELETE FROM follows WHERE who = ?1 AND kind = ?2 AND thing = ?3",
                params![who, kind, thing],
            )?;
            if gone == 0 {
                return Ok(None);
            }
            transaction.execute(
                "DELETE FROM follow_changes WHERE who = ?1 AND kind = ?2 AND thing = ?3",
                params![who, kind, thing],
            )?;
            let still: bool = transaction.query_row(
                "SELECT EXISTS (SELECT 1 FROM follows WHERE thing = ?1 AND kind = ?2)",
                params![thing, kind],
                |row| row.get(0),
            )?;
            let data = json!({ "owner": who, "kind": kind, "thing": thing, "follows": false });
            writing.say_here(
                "inbox.follows",
                Source::actor(&who).taking(Some(&crate::inbox::takes(&who))),
                data,
            )?;
            Ok(Some((grouped(&kind, &thing), still)))
        });
        let Some((grouped, still)) = wrote.await?.0 else { return Ok(false) };
        if !still {
            self.following.followed.write().expect("follows lock").remove(&grouped);
        }
        Ok(true)
    }

    /// Every follow that has changes waiting for a digest, each with when the
    /// first of them was made.
    pub async fn waiting(&self) -> Result<Vec<(Follow, i64)>> {
        let found = self.journal.db().call(|connection| {
            let mut statement = connection.prepare(
                "SELECT f.who, f.kind, f.thing, f.pages, f.often, f.at, f.since, f.told, f.made, MIN(c.at)
                 FROM follows f JOIN follow_changes c ON c.who = f.who AND c.kind = f.kind AND c.thing = f.thing
                 WHERE c.line IS NULL GROUP BY f.who, f.kind, f.thing",
            )?;
            let rows = statement.query_map([], |row| Ok((read_follow(row)?, row.get::<_, i64>(9)?)))?;
            rows.collect::<rusqlite::Result<Vec<(Follow, i64)>>>()
        });
        found.await
    }

    /// Tells `follow`'s owner everything that waited for its digest of the
    /// period that ended at `due` (what came since waits for the next), as one line saying what changed in what
    /// is called `title`, in one write: what they may not read now is
    /// told nothing of and waits no more. The line, where anything was told.
    pub async fn digest(
        &self,
        follow: Follow,
        due: i64,
        title: String,
        may: Arc<May>,
    ) -> Result<Option<Line>> {
        let (inbox, memory, now) = (self.inbox.clone(), self.memory.clone(), self.journal.now_ms());
        let wrote = self.journal.write(move |writing| {
            let transaction = writing.transaction();
            let place = key(&follow.kind, &follow.thing);
            let mut statement = transaction.prepare(&format!(
                "SELECT {PENDED} FROM follow_changes
                 WHERE who = ?1 AND kind = ?2 AND thing = ?3 AND line IS NULL AND at < ?4 ORDER BY at, seq"
            ))?;
            let waited: Vec<Pended> = statement
                .query_map(params![follow.who, follow.kind, follow.thing, due], read_pended)?
                .collect::<rusqlite::Result<_>>()?;
            let held = inbox.held(transaction, &follow.who)?;
            let thing = reads_thing(transaction, &memory, &follow.who, &held, &place)?;
            let (told, not): (Vec<Pended>, Vec<Pended>) = waited
                .into_iter()
                .partition(|one| thing && may(&follow.who, &held, &one.topic, one.takes.as_deref()));
            let about = format!("{place}#{due}");
            for one in &not {
                transaction.execute(
                    "DELETE FROM follow_changes WHERE who = ?1 AND kind = ?2 AND thing = ?3 AND seq = ?4",
                    params![follow.who, follow.kind, follow.thing, one.seq as i64],
                )?;
            }
            for one in &told {
                transaction.execute(
                    "UPDATE follow_changes SET line = ?5 WHERE who = ?1 AND kind = ?2 AND thing = ?3 AND seq = ?4",
                    params![follow.who, follow.kind, follow.thing, one.seq as i64, about],
                )?;
            }
            if let Some(last) = told.iter().map(|one| one.seq).max().filter(|_| follow.kind == "page") {
                transaction.execute(
                    "UPDATE follows SET told = MAX(told, ?4) WHERE who = ?1 AND kind = ?2 AND thing = ?3",
                    params![follow.who, follow.kind, follow.thing, last as i64],
                )?;
            }
            let Some(last) = told.last() else { return Ok(None) };
            let counts = kernel::following::counted(told.iter().map(|one| (one.noun.as_str(), one.verb.as_str())));
            let tell = Tell {
                owner: follow.who.clone(),
                kind: Kind::Following,
                about,
                place,
                from: last.from.clone(),
                why: DIGEST.to_string(),
                says: Some(kernel::following::line_said(&title, &counts)),
                said_by: Some(due.to_string()),
                parts: None,
            };
            inbox.tell_in(writing, &tell, now)
        });
        Ok(wrote.await?.0)
    }

    /// The changes the line `id` of `by`'s box told of, oldest first: what
    /// it opens. Nothing for a line of anything else.
    pub async fn changes_of(&self, by: &Principal, id: &str) -> Result<Vec<Pended>> {
        let line = self.inbox.one(by, id).await?;
        if line.kind != Kind::Following {
            return Ok(Vec::new());
        }
        let owner = line.owner.clone();
        self.journal.db().call(move |connection| of_line(connection, &owner, &line.about)).await
    }
}
