//! The discussions agents of a channel hold, and what each channel offers
//! of them (`kernel::discussed` for what they come to).
//!
//! A channel offers discussions by a text applied for it (`std/discuss`),
//! whose settings are kept here with the instance that offers them
//! (`discussing`: one for a channel, since its row is the channel's). A
//! discussion is a row of `discussions`, known by the message its thread
//! hangs from: where it stands, who takes part and who has spoken, who
//! asked, the stretch it is in and how many turns it has had. It is the
//! truth of a discussion, as a card's row is of a card: the texts that
//! hold its turns keep nothing of it.
//!
//! **Each change is one write with what is said of it.** What begins one
//! and what lets one go on are asked by whoever hears what is said in
//! channels (`agents`, as it works out whom a message is for), and are
//! the same done twice: a message heard again after a stop begins
//! nothing more (`began`, the message the stretch began from). What its
//! turns come to is written by the actions its texts ask, each the same
//! done twice by which turn of which stretch it is.
//!
//! **Held in memory for what is asked of every message**: which channels
//! offer discussions, with what, and how each discussion's thread stands,
//! filled from the tables as channels are opened and changed after each
//! write, so that a channel that offers none costs a message nothing.
//!
//! Its events are said as anything of its channel is (`Channel::takes`):
//! of a kept channel, to its members.

use std::collections::HashMap;
use std::sync::RwLock;

use journal::{Journal, optional};
use kernel::discussed::{Settings, Takes, Thread};
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde_json::{Value, json};

use crate::channels::{Channel, Channels};

/// What a discussion's row holds, as it is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discussion {
    pub root: String,
    pub channel: String,
    /// `going`, `waiting` or `agreed`.
    pub status: String,
    pub why: String,
    pub agents: Vec<String>,
    pub spoken: Vec<String>,
    pub asker: Option<String>,
    /// The message its stretch began from.
    pub began: String,
    pub stretch: i64,
    /// Turns it has had, over every stretch.
    pub turns: i64,
    /// The last turn: its stretch and number in it, whose it was and the word it ended with.
    pub last: Value,
}

/// What is held in memory for every message said.
#[derive(Default)]
pub(crate) struct Held {
    offered: RwLock<HashMap<String, Settings>>,
    threads: RwLock<HashMap<String, Thread>>,
}

fn list(text: &str) -> Vec<String> {
    serde_json::from_str(text).unwrap_or_default()
}

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Discussion> {
    Ok(Discussion {
        root: row.get(0)?,
        channel: row.get(1)?,
        status: row.get(2)?,
        why: row.get(3)?,
        agents: list(&row.get::<_, String>(4)?),
        spoken: list(&row.get::<_, String>(5)?),
        asker: row.get(6)?,
        began: row.get(7)?,
        stretch: row.get(8)?,
        turns: row.get(9)?,
        last: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or(Value::Null),
    })
}

const ROW: &str = "root, channel, status, why, agents, spoken, asker, began, stretch, turns, last";

fn thread_of(status: &str) -> Thread {
    match status {
        "going" => Thread::Going,
        "waiting" => Thread::Waiting,
        // One a person ended is as one that was agreed: what agents say
        // under it begins none again, and a person may begin one there.
        "agreed" | "ended" => Thread::Agreed,
        _ => Thread::Free,
    }
}

impl Held {
    pub(crate) async fn open(journal: &Journal) -> Result<Held> {
        let read = journal.db().call(|connection| {
            let mut offered = connection.prepare("SELECT channel, settings FROM discussing")?;
            let offered = offered
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let mut threads = connection.prepare("SELECT root, status FROM discussions")?;
            let threads = threads
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((offered, threads))
        });
        let (offered, threads) = read.await?;
        let held = Held::default();
        for (channel, settings) in offered {
            let read =
                serde_json::from_str::<Value>(&settings).ok().and_then(|said| Settings::read(&said).ok());
            if let Some(settings) = read {
                held.offered.write().expect("discussions lock").insert(channel, settings);
            }
        }
        let mut held_threads = held.threads.write().expect("discussions lock");
        for (root, status) in threads {
            held_threads.insert(root, thread_of(&status));
        }
        drop(held_threads);
        Ok(held)
    }

    fn thread(&self, root: &str, status: &str) {
        let mut threads = self.threads.write().expect("discussions lock");
        threads.insert(root.to_string(), thread_of(status));
    }

    /// Forgets what is held of a channel taken away.
    pub(crate) fn forget(&self, channel: &str) {
        self.offered.write().expect("discussions lock").remove(channel);
    }

    /// Holds again what a channel given back offers, as its rows say.
    pub(crate) async fn taken_up(&self, journal: &Journal, channel: &str) -> Result<()> {
        let lookup = channel.to_string();
        let read = journal.db().call(move |connection| {
            let offered: Option<String> = optional(connection.query_row(
                "SELECT settings FROM discussing WHERE channel = ?1",
                [&lookup],
                |row| row.get(0),
            ))?;
            let mut threads =
                connection.prepare("SELECT root, status FROM discussions WHERE channel = ?1")?;
            let threads = threads
                .query_map([&lookup], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok((offered, threads))
        });
        let (offered, threads) = read.await?;
        let settings = offered
            .and_then(|said| serde_json::from_str::<Value>(&said).ok())
            .and_then(|said| Settings::read(&said).ok());
        if let Some(settings) = settings {
            self.offered.write().expect("discussions lock").insert(channel.to_string(), settings);
        }
        for (root, status) in threads {
            self.thread(&root, &status);
        }
        Ok(())
    }
}

/// What a stretch of a discussion that is begun or goes on comes to: who
/// speaks first and what else its texts are handed.
fn begun(channel: &Channel, row: &Discussion, message: &str, first: &str) -> Value {
    json!({
        "channel": channel.id, "channel_name": channel.name, "project": channel.project, "root": row.root,
        "message": message, "why": row.why, "agents": row.agents, "spoken": row.spoken, "asker": row.asker,
        "stretch": row.stretch, "first": first,
    })
}

impl Channels {
    /// What `channel` offers of discussions, where it offers any.
    pub fn discussing(&self, channel: &str) -> Option<Settings> {
        self.discussed.offered.read().expect("discussions lock").get(channel).cloned()
    }

    /// How the thread that hangs from `root` stands.
    pub fn discussion_thread(&self, root: &str) -> Thread {
        self.discussed.threads.read().expect("discussions lock").get(root).copied().unwrap_or(Thread::Free)
    }

    /// Keeps what `channel` offers of discussions, as `instance` asked of
    /// it for `by`, who writes there: one text offers for a channel, and
    /// another's is refused while that one's is kept. Who takes part are
    /// agents of the channel that this factory starts.
    pub async fn offer(
        &self,
        by: &Principal,
        channel: &str,
        said: &Value,
        instance: &str,
    ) -> Result<Settings> {
        let reader = self.may_write(channel, by).await?;
        let channel = self.get(&reader, channel).await?;
        let settings = Settings::read(said).map_err(Error::invalid)?;
        for name in settings.taking() {
            let actor = format!("agent:{name}");
            if !channel.members.iter().any(|member| member.actor == actor) {
                return Err(Error::invalid(format!(
                    "`{name}` is not in #{}: bring it into the channel before it takes part in discussions there",
                    channel.name
                )));
            }
            if self.memory.outside(&actor) {
                return Err(Error::invalid(format!(
                    "`{name}` is an agent this factory does not start, and is given no turn: it cannot take part \
                     in a discussion"
                )));
            }
        }
        let (kept, by_actor, instance, now) =
            (settings.said().to_string(), by.actor(), instance.to_string(), self.journal.now_ms());
        let source = Source::actor(&by_actor).taking(channel.takes(&[]).as_deref());
        let (data, id) = (
            json!({ "channel": channel.id, "settings": settings.said(), "instance": instance }),
            channel.id.clone(),
        );
        let wrote = self.journal.write(move |writing| {
            let other: Option<String> = optional(writing.transaction().query_row(
                "SELECT instance FROM discussing WHERE channel = ?1",
                [&id],
                |row| row.get(0),
            ))?;
            if let Some(other) = other.filter(|other| *other != instance) {
                return Ok(Err(other));
            }
            writing.transaction().execute(
                "INSERT INTO discussing (channel, settings, by, instance, written) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (channel) DO UPDATE SET settings = ?2, by = ?3, written = ?5",
                params![id, kept, by_actor, instance, now],
            )?;
            writing.say_here("channel.discussion.offered", source, data)?;
            Ok(Ok(()))
        });
        if let Err(other) = wrote.await?.0 {
            return Err(Error::conflict(format!(
                "#{} offers discussions already, by `{other}`: change that, or stop it first",
                channel.name
            )));
        }
        self.discussed.offered.write().expect("discussions lock").insert(channel.id, settings.clone());
        Ok(settings)
    }

    /// Takes away what `instance` offered of discussions in `channel`.
    /// What is going on there ends as it comes to its end.
    pub async fn withdraw(&self, channel: &str, instance: &str) -> Result<()> {
        let own = &Channels::daemon("which channel a text that offered discussions stops offering them in");
        let Ok(channel) = self.get(own, channel).await else { return Ok(()) };
        let source = Source::actor("system").taking(channel.takes(&[]).as_deref());
        let (id, instance) = (channel.id.clone(), instance.to_string());
        let data = json!({ "channel": id });
        let wrote = self.journal.write(move |writing| {
            let gone = writing.transaction().execute(
                "DELETE FROM discussing WHERE channel = ?1 AND instance = ?2",
                params![id, instance],
            )?;
            if gone > 0 {
                writing.say_here("channel.discussion.withdrawn", source, data)?;
            }
            Ok(gone > 0)
        });
        if wrote.await?.0 {
            self.discussed.forget(&channel.id);
        }
        Ok(())
    }

    /// The discussion of the thread that hangs from `root`, where there is one.
    pub async fn discussion(&self, root: &str) -> Result<Option<Discussion>> {
        let root = root.to_string();
        let found = self.journal.db().call(move |connection| {
            optional(connection.query_row(
                &format!("SELECT {ROW} FROM discussions WHERE root = ?1"),
                [&root],
                read_row,
            ))
        });
        found.await
    }

    /// Begins the discussion `message` of `channel` comes to, where it
    /// takes `takes` (`kernel::discussed::takes`), and says so; or lets
    /// one that waits go on. The same done twice: a stretch is begun from
    /// a message once.
    pub async fn discuss(&self, channel: &Channel, message: &str, takes: &Takes) -> Result<()> {
        let (root, start) = match takes {
            Takes::Starts { root, agents, spoken, asker, why } => {
                (root.clone(), Some((agents.clone(), spoken.clone(), asker.clone(), *why)))
            }
            Takes::Within { resumes: true } => match self.discussion_root(message).await? {
                Some(root) => (root, None),
                None => return Ok(()),
            },
            _ => return Ok(()),
        };
        let source = Source::actor("system").taking(channel.takes(&[]).as_deref());
        let (now, said_channel, message) = (self.journal.now_ms(), channel.clone(), message.to_string());
        let wrote = self.journal.write(move |writing| {
            let was = optional(writing.transaction().query_row(
                &format!("SELECT {ROW} FROM discussions WHERE root = ?1"),
                [&root],
                read_row,
            ))?;
            if was.as_ref().is_some_and(|was| was.began == message) {
                return Ok(None);
            }
            let (row, topic, first) = match (start, was) {
                // One begun where none was, or where one ended agreed.
                (Some((agents, spoken, asker, why)), was) if was.as_ref().is_none_or(|was| was.status == "agreed") => {
                    let stretch = was.as_ref().map_or(1, |was| was.stretch + 1);
                    let first = agents.first().cloned().unwrap_or_default();
                    let row = Discussion {
                        root: root.clone(),
                        channel: said_channel.id.clone(),
                        status: "going".into(),
                        why: why.into(),
                        agents,
                        spoken,
                        asker,
                        began: message.clone(),
                        stretch,
                        turns: was.as_ref().map_or(0, |was| was.turns),
                        last: Value::Null,
                    };
                    (row, "channel.discussion.begun", first)
                }
                // A person's word in one that waits lets it go on, from whoever spoke last.
                (None, Some(was)) if was.status == "waiting" => {
                    let first = was.last["slot"].as_str().unwrap_or_default().to_string();
                    let row = Discussion { status: "going".into(), began: message.clone(), stretch: was.stretch + 1, ..was };
                    (row, "channel.discussion.resumed", first)
                }
                _ => return Ok(None),
            };
            writing.transaction().execute(
                &format!(
                    "INSERT INTO discussions ({ROW}, created, updated) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
                     ON CONFLICT (root) DO UPDATE SET status = ?3, why = ?4, agents = ?5, spoken = ?6, asker = ?7,
                     began = ?8, stretch = ?9, turns = ?10, last = ?11, updated = ?12"
                ),
                params![
                    row.root,
                    row.channel,
                    row.status,
                    row.why,
                    json!(row.agents).to_string(),
                    json!(row.spoken).to_string(),
                    row.asker,
                    row.began,
                    row.stretch,
                    row.turns,
                    row.last.to_string(),
                    now
                ],
            )?;
            let said = begun(&said_channel, &row, &message, &first);
            // Each topic written out where it is said, so that it is held to being described.
            match topic {
                "channel.discussion.resumed" => writing.say_here("channel.discussion.resumed", source, said)?,
                _ => writing.say_here("channel.discussion.begun", source, said)?,
            };
            Ok(Some(row.root))
        });
        if let Some(root) = wrote.await?.0 {
            self.discussed.thread(&root, "going");
        }
        Ok(())
    }

    /// The message the thread `message` is in hangs from.
    async fn discussion_root(&self, message: &str) -> Result<Option<String>> {
        let message = message.to_string();
        let found = self.journal.db().call(move |connection| {
            optional(connection.query_row("SELECT thread FROM messages WHERE id = ?1", [&message], |row| {
                row.get::<_, Option<String>>(0)
            }))
        });
        Ok(found.await?.flatten())
    }

    /// Writes that turn `nth` of `stretch` of the discussion under `root`
    /// was `slot`'s and ended with `word`: the same done twice.
    pub async fn discussion_turned(
        &self,
        root: &str,
        stretch: i64,
        nth: i64,
        slot: &str,
        word: &str,
    ) -> Result<Discussion> {
        let found = self.discussion(root).await?.ok_or_else(|| no_discussion(root))?;
        let channel = self.row(&found.channel).await?;
        let source = Source::actor("system").taking(channel.takes(&[]).as_deref());
        let (root, slot, word, now) =
            (root.to_string(), slot.to_string(), word.to_string(), self.journal.now_ms());
        let wrote = self.journal.write(move |writing| {
            let mut row = writing
                .transaction()
                .query_row(&format!("SELECT {ROW} FROM discussions WHERE root = ?1"), [&root], read_row)?;
            let at = (row.last["stretch"].as_i64().unwrap_or(0), row.last["nth"].as_i64().unwrap_or(0));
            if (stretch, nth) <= at {
                return Ok(row);
            }
            row.turns += 1;
            if !row.spoken.contains(&slot) {
                row.spoken.push(slot.clone());
            }
            row.last = json!({ "stretch": stretch, "nth": nth, "slot": slot, "word": word });
            writing.transaction().execute(
                "UPDATE discussions SET turns = ?2, spoken = ?3, last = ?4, updated = ?5 WHERE root = ?1",
                params![root, row.turns, json!(row.spoken).to_string(), row.last.to_string(), now],
            )?;
            let data = json!({
                "channel": row.channel, "root": root, "stretch": stretch, "nth": nth, "slot": slot, "word": word,
                "turns": row.turns,
            });
            writing.say_here("channel.discussion.turned", source, data)?;
            Ok(row)
        });
        Ok(wrote.await?.0)
    }

    /// Ends `stretch` of the discussion under `root` as `came` (`agreed`,
    /// `needs`, `capped`), leaving it `status`, and says so; the same done
    /// twice. Answers the discussion, and whom its ending is told to:
    /// the person who asked, else `applier`, and in a channel kept to its
    /// members only one of them; and whether it is `by`'s to say how it
    /// ended: the run that ended it, done again after a stop, says it,
    /// and another run of the same steps, begun again once that one was
    /// over, does not say it twice.
    pub async fn discussion_ended(
        &self,
        root: &str,
        stretch: i64,
        came: &str,
        status: &str,
        (applier, by): (&str, &str),
    ) -> Result<(Discussion, Option<String>, bool)> {
        let found = self.discussion(root).await?.ok_or_else(|| no_discussion(root))?;
        let channel = self.row(&found.channel).await?;
        let source = Source::actor("system").taking(channel.takes(&[]).as_deref());
        let label = came.to_string();
        let (root_id, came, status, now) =
            (root.to_string(), came.to_string(), status.to_string(), self.journal.now_ms());
        let by = by.to_string();
        let wrote = self.journal.write(move |writing| {
            let mut row = writing.transaction().query_row(
                &format!("SELECT {ROW} FROM discussions WHERE root = ?1"),
                [&root_id],
                read_row,
            )?;
            if row.stretch != stretch || row.status != "going" {
                let ours = row.stretch == stretch && row.last["ended_by"].as_str() == Some(by.as_str());
                return Ok((row, false, ours));
            }
            row.status = status.clone();
            row.last["ended_by"] = json!(by);
            writing.transaction().execute(
                "UPDATE discussions SET status = ?2, last = ?3, updated = ?4 WHERE root = ?1",
                params![root_id, status, row.last.to_string(), now],
            )?;
            let data = json!({
                "channel": row.channel, "root": root_id, "stretch": stretch, "came": came, "status": status,
                "turns": row.turns, "agents": row.agents,
            });
            writing.say_here("channel.discussion.ended", source, data)?;
            Ok((row, true, true))
        });
        let (row, ended, says) = wrote.await?.0;
        if ended {
            self.discussed.thread(&row.root, &row.status);
            self.journal.measure(
                "channel.discussion_turns_count",
                &[("channel", &row.channel), ("came", &label)],
                row.last["nth"].as_f64().unwrap_or(0.0),
                true,
            );
        }
        let to = row
            .asker
            .clone()
            .filter(|asker| asker.starts_with("user:"))
            .unwrap_or_else(|| applier.to_string());
        let told = (!channel.kept || channel.members.iter().any(|member| member.actor == to)).then_some(to);
        Ok((row, told.filter(|to| to.starts_with("user:")), says))
    }
}

impl Channels {
    /// The discussions of `channel` that are going or wait for a person,
    /// as `reader` may see them.
    pub async fn discussions_in(&self, reader: &crate::Reader, channel: &str) -> Result<Vec<Discussion>> {
        let id = self.get(reader, channel).await?.id;
        let read = self.journal.db().call(move |connection| {
            let mut found = connection.prepare(&format!(
                "SELECT {ROW} FROM discussions WHERE channel = ?1 AND status IN ('going', 'waiting') ORDER BY created"
            ))?;
            let rows = found.query_map([&id], read_row)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        });
        read.await
    }

    /// Ends the discussion under `root` in `channel` for `by`, who writes
    /// there, whether it is going or waits for a person, and says so,
    /// crediting them, as one write. What its stretch says as it ends is
    /// then said by nobody (`says` of [`Channels::discussion_ended`]), and
    /// what is in hand of it is whoever asked's to call off.
    pub async fn end_discussion(&self, by: &Principal, channel: &str, root: &str) -> Result<Discussion> {
        let reader = self.may_write(channel, by).await?;
        let channel = self.get(&reader, channel).await?;
        let found = self.discussion(root).await?.filter(|found| found.channel == channel.id);
        if !found.is_some_and(|found| matches!(found.status.as_str(), "going" | "waiting")) {
            return Err(Error::not_found(format!(
                "there is no discussion going on, or waiting, under `{root}` in #{}",
                channel.name
            )));
        }
        let source = Source::actor(by.actor()).taking(channel.takes(&[]).as_deref());
        let (actor, name, now, root) = (by.actor(), by.name.clone(), self.journal.now_ms(), root.to_string());
        let wrote = self.journal.write(move |writing| {
            let mut row = writing
                .transaction()
                .query_row(&format!("SELECT {ROW} FROM discussions WHERE root = ?1"), [&root], read_row)?;
            if !matches!(row.status.as_str(), "going" | "waiting") {
                return Ok(Err(row));
            }
            row.status = "ended".into();
            row.last["ended_by"] = json!(actor);
            writing.transaction().execute(
                "UPDATE discussions SET status = 'ended', last = ?2, updated = ?3 WHERE root = ?1",
                params![root, row.last.to_string(), now],
            )?;
            let data = json!({
                "channel": row.channel, "root": root, "stretch": row.stretch, "came": "stopped", "status": "ended",
                "turns": row.turns, "agents": row.agents, "by": actor, "by_name": name,
            });
            writing.say_here("channel.discussion.ended", source, data)?;
            Ok(Ok(row))
        });
        let row = wrote.await?.0.map_err(|_| Error::conflict("that discussion has just ended"))?;
        self.discussed.thread(&row.root, &row.status);
        self.journal.measure(
            "channel.discussion_turns_count",
            &[("channel", &row.channel), ("came", "stopped")],
            row.last["nth"].as_f64().unwrap_or(0.0),
            true,
        );
        Ok(row)
    }
}

/// What a message is marked as, where it is marked as one of the marks
/// there are; refused in a sentence otherwise.
pub(crate) fn marked(mark: Option<&str>) -> Result<Option<String>> {
    let Some(mark) = mark.map(str::trim).filter(|mark| !mark.is_empty()) else { return Ok(None) };
    if !kernel::discussed::MARKS.contains(&mark) {
        let marks: Vec<String> = kernel::discussed::MARKS.iter().map(|one| format!("`{one}`")).collect();
        return Err(Error::invalid(format!("`{mark}` is no mark a message carries: {}", marks.join(" or "))));
    }
    Ok(Some(mark.to_string()))
}

fn no_discussion(root: &str) -> Error {
    Error::not_found(format!("there is no discussion under `{root}`"))
}

/// The actions a channel's discussions are kept by: offered and taken
/// away by the text that offers them, and a turn and an ending written by
/// the texts that hold the turns.
fn number(args: &kernel::Fields, key: &str) -> i64 {
    args.get(key).and_then(Value::as_i64).unwrap_or(0)
}

pub(crate) fn register(registry: &mut conductor::Registry, channels: &Channels) {
    use blueprint::{ActionSpec, ValueType as T};

    use crate::actions::{add, text};

    add(
        registry,
        channels,
        ActionSpec::new("discussion.offer", "Have agents of a channel talk things over in its threads, as this says.")
            .required("channel", T::String, "the channel's id or name")
            .arg("agents", T::List, "who discuss as equals: each a name, or `{ name, does }` with what it is for, said to the others")
            .arg("lead", T::String, "the agent a person talks to: what a person says naming no agent is its alone")
            .arg("advisors", T::List, "whom the lead consults: each `{ name, for }`, with what it is for")
            .arg("on", T::List, "what begins one, any of: tagged (an agent names another), plans (an agent marks what it said as a plan), asked (a person marks what they said for two or more to discuss); all of them unless said")
            .arg("turns", T::Int, "how many turns agents take in a stretch before a person is asked (6 unless said, 20 at most)")
            .arg("agreeing", T::String, "`one`: one agrees, once everyone has spoken; or `all`")
            .arg("tell_on", T::List, "which endings the person is told of in their inbox: any of agree, needs, cap")
            .capability("channels:write")
            .output("channel", "the channel's id")
            .output("taking", "everybody who takes part, the lead first")
            .output("settings", "what it offers, as it is kept and handed on to `discussion.next`")
            .idempotent(),
        |channels, cx, args| async move {
            let channel = text(&args, "channel").unwrap_or_default();
            let said = Value::Object(args.clone().into_iter().collect());
            let settings = channels.offer(cx.principal(), &channel, &said, cx.instance()).await?;
            let reader = channels.may_find(&channel, cx.principal()).await?;
            let id = channels.get(&reader, &channel).await?.id;
            Ok(json!({ "channel": id, "taking": settings.taking(), "settings": settings.said() }))
        },
    );
    add(
        registry,
        channels,
        ActionSpec::new(
            "discussion.withdraw",
            "Stop a channel offering discussions, as this text offered them.",
        )
        .required("channel", T::String, "the channel's id or name")
        .capability("channels:write")
        .idempotent(),
        |channels, cx, args| async move {
            channels.withdraw(&text(&args, "channel").unwrap_or_default(), cx.instance()).await?;
            Ok(json!({}))
        },
    );
    add(
        registry,
        channels,
        ActionSpec::new(
            "discussion.turned",
            "Write down whose a turn of a discussion was, and how it ended.",
        )
        .required("root", T::String, "the message the discussion's thread hangs from")
        .required("stretch", T::Int, "which stretch of it")
        .required("nth", T::Int, "which turn of the stretch")
        .required("slot", T::String, "whose turn it was")
        .arg("word", T::String, "what it ended with")
        .capability("channels:write")
        .output("turns", "how many turns the discussion has had")
        .idempotent(),
        |channels, _cx, args| async move {
            let (root, slot) =
                (text(&args, "root").unwrap_or_default(), text(&args, "slot").unwrap_or_default());
            let word = text(&args, "word").unwrap_or_default();
            let row = channels
                .discussion_turned(&root, number(&args, "stretch"), number(&args, "nth"), &slot, &word)
                .await?;
            Ok(json!({ "turns": row.turns }))
        },
    );
    add(
        registry,
        channels,
        ActionSpec::new("discussion.end", "Write down how a stretch of a discussion ended, and whom it is told to.")
            .required("root", T::String, "the message the discussion's thread hangs from")
            .required("stretch", T::Int, "which stretch of it")
            .required("came", T::String, "`agreed`, `needs` or `capped`")
            .required("status", T::String, "what it is left as: `agreed`, or `waiting` for a person")
            .capability("channels:write")
            .output("to", "whom its ending is told to, as an actor: the person who asked, else whoever applied this; empty for nobody")
            .output("agents", "who took part")
            .output("turns", "how many turns it has had")
            .output("says", "whether this is what says how it ended: not where it ended by another run of these steps")
            .idempotent(),
        |channels, cx, args| async move {
            let (root, came) = (text(&args, "root").unwrap_or_default(), text(&args, "came").unwrap_or_default());
            let status = text(&args, "status").unwrap_or_default();
            if !["agreed", "waiting"].contains(&status.as_str()) {
                return Err(conductor::ActionError::new("`status` is `agreed` or `waiting`"));
            }
            let applier = cx.principal().actor();
            let by = (applier.as_str(), cx.run());
            let (row, to, says) = channels.discussion_ended(&root, number(&args, "stretch"), &came, &status, by).await?;
            Ok(json!({ "to": to.unwrap_or_default(), "agents": row.agents, "turns": row.turns, "says": says }))
        },
    );
}
