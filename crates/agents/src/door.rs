//! The door of a channel kept to its members, as whoever keeps agents
//! holds it: what becomes of the agents in a channel, and of what they
//! were asked there, as it is kept, opened again or told that its
//! agents are asked nowhere else; and an agent that is there brought
//! into a channel.
//!
//! Each is done holding the turn at giving agents work
//! (`Inner::choosing`): nothing is queued for an agent between what is
//! asked of its turns here and the write that changes the channel, since
//! which channel a turn is of is worked out under that turn too
//! ([`Agents::kept_of`]).
//!
//! **Keeping an open channel.** An agent that could be in no kept
//! channel as things stand (no wall, a sign-in that cannot be told from
//! what it is asked: `crate::kept`) is not taken out behind the back of
//! whoever keeps the channel: the keeping is refused, in a sentence that
//! says which agent, why, and both ways out. So is one asked while a
//! turn of the channel waits or is in hand: its program was started in
//! the home it has for everything else, and what is said of it was
//! begun as anybody's to hear. The turns that have ended are written as
//! that channel's (`turns.kept`), before the channel is, so that from
//! the write that keeps it their words are shown to whoever reads it and
//! to nobody else ([`Agents::turns_for`]); and what each agent's program
//! remembered of the conversations held there is forgotten, since its
//! next turn there is started in the home it has for that channel, where
//! nothing of them is. What its program kept in the home it has for
//! everything else, of what it was asked there while the channel was
//! open, is not taken out of that home: it is what was said while the
//! channel was open.
//!
//! **A conversation** (`talk::conversed`) is written with its agents in
//! it from the write that makes it ([`Agents::converse`]): each is asked
//! first whether it may be in a channel kept to its members, as one
//! brought into any is, and nothing is made where one may not.
//!
//! **Archiving one** (`talk::archived`), open or kept, is refused while
//! a turn of it waits or is in hand, for the same reason: its answer
//! would be refused there, with the work done for nothing.
//!
//! **Renaming one** (`talk::renamed`) that is open is refused while a
//! turn of it waits or is in hand: what it was asked may name the
//! channel by the name it has now, which is where its answer is said.
//! The turns that have ended and name it by that name are written as
//! naming it by its id first (`channel`, and a conversation held there,
//! `channel:<name>`), so that they are still its own once the name is
//! another's to take: kept later, it finds them, and whatever is said of
//! where an agent was asked says this channel. One kept to its members
//! is called its title, which no turn names, and is renamed with
//! nothing asked of its turns.
//!
//! **Opening one again.** Refused while a turn of it waits or is in
//! hand, for the same reasons the other way round. What each agent kept
//! of it as a kept channel goes: the folder that was its own for it and
//! what its program remembered there. The turns asked there while it
//! was kept stay written as its own, and are shown to whoever reads the
//! channel, which is now anybody who may read channels.

use std::collections::BTreeSet;

use kernel::{Error, Principal, Result};
use rusqlite::params;

use crate::{Agents, Slot};

/// A turn that is not over: waiting, or in hand.
const NOT_OVER: &str = "status IN ('queued', 'running', 'answered')";

/// A turn found to be of a channel: which, whose, the conversation it
/// goes on with, and whether it is over.
type Found = (String, String, String, bool);

impl Agents {
    /// The agents that are in `channel`, by name, each with its slot
    /// where it has one.
    async fn agents_in(&self, channel: &studio::Channel) -> Result<Vec<crate::Slot>> {
        let mut within = Vec::new();
        for member in &channel.members {
            if let Some(name) = member.actor.strip_prefix("agent:")
                && let Some(slot) = self.find(name).await?
            {
                within.push(slot);
            }
        }
        Ok(within)
    }

    /// The turns that are of the open channel `channel` and do not say
    /// so, in every file that keeps turns: asked there or answering
    /// there, or going on with a conversation held there, wherever its
    /// answer is said.
    async fn asked_in(&self, reader: &studio::Reader, channel: &studio::Channel) -> Result<Vec<Found>> {
        let named = [channel.id.clone(), channel.name.clone(), format!("#{}", channel.name)];
        let held: Vec<String> = named.iter().map(|named| format!("channel:{named}")).collect();
        let asked = move |connection: &mut rusqlite::Connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT id, slot, thread, {NOT_OVER},
                        channel IN (?1, ?2, ?3) OR thread IN (?4, ?5, ?6)
                 FROM turns
                 WHERE kept IS NULL AND (channel IN (?1, ?2, ?3) OR thread IN (?4, ?5, ?6)
                       OR thread LIKE 'thread:%')"
            ))?;
            let rows = statement
                .query_map(params![named[0], named[1], named[2], held[0], held[1], held[2]], |row| {
                    Ok(((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?), row.get::<_, bool>(4)?))
                })?;
            rows.collect::<rusqlite::Result<Vec<(Found, bool)>>>()
        };
        let found: Vec<(Found, bool)> =
            self.of_each(self.every_turns(), asked).await?.into_iter().flatten().collect();
        // One that goes on with a conversation named by the message it
        // hangs from is this channel's where that message is.
        let hangs_from = |thread: &str| thread.strip_prefix("thread:").unwrap_or_default().to_string();
        let messages: BTreeSet<String> =
            found.iter().filter(|(_, sure)| !sure).map(|(turn, _)| hangs_from(&turn.2)).collect();
        let messages: Vec<String> = messages.into_iter().collect();
        let mut here = BTreeSet::new();
        // A few hundred at a time: a statement names only so many things.
        for some in messages.chunks(500) {
            here.extend(self.inner.studio.channels.said_in(reader, some).await?.into_keys());
        }
        let of_it =
            |(turn, sure): (Found, bool)| (sure || here.contains(&hangs_from(&turn.2))).then_some(turn);
        Ok(found.into_iter().filter_map(of_it).collect())
    }

    /// Writes `turns` as the kept channel `kept`'s, or as no kept
    /// channel's where none is said, in whichever file keeps each.
    async fn written_as(&self, turns: &[Found], kept: Option<&str>) -> Result<()> {
        let (ids, kept) =
            (turns.iter().map(|turn| turn.0.clone()).collect::<Vec<_>>(), kept.map(str::to_string));
        if ids.is_empty() {
            return Ok(());
        }
        let write = move |connection: &mut rusqlite::Connection| {
            let mut statement = connection.prepare("UPDATE turns SET kept = ?2 WHERE id = ?1")?;
            ids.iter().try_for_each(|id| statement.execute(params![id, kept]).map(|_| ()))
        };
        self.of_each(self.every_turns(), write).await.map(|_| ())
    }

    /// The agent that has a turn of the kept channel `kept` waiting or
    /// in hand, where one has, in any file that keeps turns.
    async fn asked_now(&self, kept: &str) -> Result<Option<String>> {
        let of = kept.to_string();
        let asked = move |connection: &mut rusqlite::Connection| {
            journal::optional(connection.query_row(
                &format!("SELECT slot FROM turns WHERE kept = ?1 AND {NOT_OVER} LIMIT 1"),
                [&of],
                |row| row.get::<_, String>(0),
            ))
        };
        Ok(self.of_each(self.every_turns(), asked).await?.into_iter().flatten().next())
    }

    /// Keeps the open channel `channel` to its members, as `by`, who
    /// may write in it: whoever is in it reads it from here, and nobody
    /// else, what its agents were asked there with the rest. Refused
    /// while an agent in it could be in no kept channel, and while one
    /// is being asked something there.
    pub async fn keep_channel(&self, channel: &str, by: &Principal) -> Result<studio::Channel> {
        let channels = &self.inner.studio.channels;
        let reader = channels.may_write(channel, by).await?;
        let _held = self.inner.choosing.lock().await;
        let open = channels.get(&reader, channel).await?;
        if open.kept {
            return Ok(open);
        }
        studio::door::may_keep(&open, by)?;
        let (called, cli) = (studio::door::called(&open), brand::CLI);
        for slot in self.agents_in(&open).await? {
            if let Some((why, rule, way)) = self.kept_out(&slot) {
                return Err(Error::conflict(format!(
                    "{why}, and {rule}: {called} is not kept while it is in it. {way}; or take it out of the \
                     channel first (`{cli} channel leave {} --agent {}`)",
                    open.name, slot.name
                )));
            }
        }
        let asked = self.asked_in(&reader, &open).await?;
        if let Some((_, slot, _, _)) = asked.iter().find(|turn| turn.3) {
            return Err(Error::conflict(format!(
                "`{slot}` is being asked something in {called}, by a program started where everything it is \
                 asked elsewhere is kept: keep the channel once that is over"
            )));
        }
        // Before the channel is written as kept: until then everybody
        // reads it, and these are shown as they were.
        self.written_as(&asked, Some(&open.id)).await?;
        let kept = match channels.keep(&reader, &open.id, by).await {
            Ok(kept) => kept,
            Err(refused) => {
                let _ = self.written_as(&asked, None).await;
                return Err(refused);
            }
        };
        let slots: BTreeSet<&str> = asked.iter().map(|turn| turn.1.as_str()).collect();
        for slot in slots {
            self.forget_said(slot, &kept.id).await;
        }
        Ok(kept)
    }

    /// Archives the channel `channel` (`talk::archived`), as `by`, who
    /// may write in it. Refused while a turn of it waits or is in hand,
    /// as keeping one is: its agent would do its work and have its answer
    /// refused at the end, with nobody told. Held under the turn at
    /// giving agents work, so that none is queued between the look and
    /// the write; after the write none is ([`Agents::enqueue_chosen`]).
    pub async fn archive_channel(&self, channel: &str, by: &Principal) -> Result<studio::Channel> {
        let channels = &self.inner.studio.channels;
        let reader = channels.may_write(channel, by).await?;
        let _held = self.inner.choosing.lock().await;
        let found = channels.get(&reader, channel).await?;
        if found.archived.is_some() {
            return Ok(found);
        }
        let asked = match found.kept {
            true => self.asked_now(&found.id).await?,
            false => self.asked_in(&reader, &found).await?.into_iter().find(|turn| turn.3).map(|turn| turn.1),
        };
        if let Some(slot) = asked {
            return Err(Error::conflict(format!(
                "`{slot}` is being asked something in {}, and its answer would be refused in an archived channel: \
                 archive it once that is over, or stop the agent first (`{} agent stop {slot}`)",
                studio::door::called(&found),
                brand::CLI
            )));
        }
        channels.archive(&reader, &found.id, by).await
    }

    /// Calls the channel `channel` `to` (`talk::renamed`), as `by`, who
    /// manages it (`talk::managed`). Of an open one, refused while a turn of it waits
    /// or is in hand, and the turns that have ended and name it by its
    /// name are written as naming it by its id first. Held under the
    /// turn at giving agents work, so that none is queued between the
    /// look and the write.
    pub async fn rename_channel(&self, channel: &str, to: &str, by: &Principal) -> Result<studio::Channel> {
        let channels = &self.inner.studio.channels;
        let reader = channels.may_write(channel, by).await?;
        let _held = self.inner.choosing.lock().await;
        let found = channels.get(&reader, channel).await?;
        studio::managed::may_manage(&found, by)?;
        if found.archived.is_some() {
            return Err(studio::renamed::unarchive_first(&found));
        }
        if found.kept || studio::renamed::renaming(&found, to)?.is_none() {
            return channels.rename(&reader, &found.id, to, by).await;
        }
        let asked = self.asked_in(&reader, &found).await?;
        if let Some((_, slot, _, _)) = asked.iter().find(|turn| turn.3) {
            return Err(Error::conflict(format!(
                "`{slot}` is being asked something in {}, and may have been asked it by the name the channel has \
                 now, which is where its answer is said: rename it once that is over, or stop the agent first \
                 (`{} agent stop {slot}`)",
                studio::door::called(&found),
                brand::CLI
            )));
        }
        let ids: Vec<String> = asked.iter().map(|turn| turn.0.clone()).collect();
        if !ids.is_empty() {
            let (id, name) = (found.id.clone(), found.name.clone());
            let by_id = move |connection: &mut rusqlite::Connection| {
                let mut channel = connection
                    .prepare("UPDATE turns SET channel = ?2 WHERE id = ?1 AND channel IN (?3, '#' || ?3)")?;
                let mut thread = connection.prepare(
                    "UPDATE turns SET thread = 'channel:' || ?2
                     WHERE id = ?1 AND thread IN ('channel:' || ?3, 'channel:#' || ?3)",
                )?;
                for turn in &ids {
                    channel.execute(params![turn, id, name])?;
                    thread.execute(params![turn, id, name])?;
                }
                Ok(())
            };
            self.of_each(self.every_turns(), by_id).await?;
        }
        channels.rename(&reader, &found.id, to, by).await
    }

    /// Opens the kept channel `channel` again, as `by`, who may write in
    /// it: whether they may open one is asked by whoever asks this, and
    /// is the factory's owner's to do. `name` is what it is called once
    /// it is open, where its title is not to be. Refused while an agent
    /// is being asked something there.
    pub async fn open_channel(
        &self,
        channel: &str,
        name: Option<&str>,
        by: &Principal,
    ) -> Result<studio::Channel> {
        let channels = &self.inner.studio.channels;
        let reader = channels.may_write(channel, by).await?;
        let _held = self.inner.choosing.lock().await;
        let kept = channels.get(&reader, channel).await?;
        if !kept.kept {
            return Ok(kept);
        }
        if let Some(slot) = self.asked_now(&kept.id).await? {
            return Err(Error::conflict(format!(
                "`{slot}` is being asked something in {}, which is said to its members and kept where the \
                 agent keeps what it is asked there: open the channel once that is over",
                studio::door::called(&kept)
            )));
        }
        let opened = channels.open_again(&reader, &kept.id, name, by).await?;
        // What each agent kept of it as a kept channel goes.
        let of = opened.id.clone();
        let asked = move |connection: &mut rusqlite::Connection| {
            let mut statement = connection.prepare("SELECT DISTINCT slot FROM turns WHERE kept = ?1")?;
            let slots = statement.query_map([&of], |row| row.get::<_, String>(0))?;
            slots.collect::<rusqlite::Result<Vec<String>>>()
        };
        let asked_there = self.of_each(self.every_turns(), asked).await?;
        let mut slots: BTreeSet<String> = asked_there.into_iter().flatten().collect();
        self.with_a_place(&opened.id, &mut slots);
        for slot in slots {
            self.forget_place(&slot, &opened.id).await;
        }
        Ok(opened)
    }

    /// Says whether the agents of the kept channel `channel` are asked
    /// nowhere else, as `by`, who may write in it. Refused, to say that
    /// they are, while one of them is in another kept channel or has
    /// something asked elsewhere waiting or in hand.
    pub async fn hold_alone(&self, channel: &str, alone: bool, by: &Principal) -> Result<studio::Channel> {
        let channels = &self.inner.studio.channels;
        let reader = channels.may_write(channel, by).await?;
        let _held = self.inner.choosing.lock().await;
        let kept = channels.get(&reader, channel).await?;
        for slot in self.agents_in(&kept).await?.into_iter().filter(|_| alone && kept.kept) {
            let (name, of) = (slot.name.clone(), kept.id.clone());
            let asked = move |connection: &mut rusqlite::Connection| {
                connection
                    .prepare(&format!(
                        "SELECT 1 FROM turns WHERE slot = ?1 AND {NOT_OVER} AND kept IS NOT ?2 LIMIT 1"
                    ))?
                    .exists([&name, &of])
            };
            let elsewhere = self.of_each(self.may_hold(&slot.name), asked).await?;
            if elsewhere.contains(&true) {
                return Err(Error::conflict(format!(
                    "`{}` has something it was asked elsewhere waiting or in hand: say that the agents of {} \
                     are asked nowhere else once that is over",
                    slot.name,
                    studio::door::called(&kept)
                )));
            }
        }
        channels.hold_alone(&reader, &kept.id, alone, by).await
    }

    /// Whether each of the agents called `names` could be in a channel
    /// that is about to be made, kept to its members or not and holding
    /// its agents to itself or not: asked before the channel is made, so
    /// that none is made with somebody missing from it.
    pub async fn could_be_in(&self, names: &[String], kept: bool, alone: bool, by: &Principal) -> Result<()> {
        if names.is_empty() {
            return Ok(());
        }
        by.require("agents:write")?;
        for name in names {
            let slot = self.slot(name).await?;
            if let Some((why, rule, way)) = self.kept_out(&slot).filter(|_| kept) {
                return Err(Error::conflict(format!("{why}, and {rule}. {way}")));
            }
            if kept {
                self.inner.studio.channels.may_bring_into_new(alone, &slot.name)?;
            }
        }
        Ok(())
    }

    /// Writes a conversation as `by` to these people and to the agents
    /// called `agents`: a channel kept to them and `by` that says it is
    /// one, with what is first said in it, as one write
    /// (`Channels::converse`). An agent this factory starts is in it
    /// under what holds one in any kept channel, asked before anything
    /// is made, and is brought in by whoever may change agents; one it
    /// does not start is written to as a person is.
    pub async fn converse(
        &self,
        subject: &str,
        text: &str,
        people: &[studio::Member],
        agents: &[String],
        by: &Principal,
    ) -> Result<(studio::Channel, studio::Message)> {
        let _held = self.inner.choosing.lock().await;
        let mut brought = Vec::with_capacity(agents.len());
        for name in agents {
            let slot = self.slot(name).await?;
            if !slot.external() {
                by.require("agents:write")?;
            }
            if let Some((why, rule, way)) = self.kept_out(&slot) {
                return Err(Error::conflict(format!(
                    "{why}, and {rule}, which a conversation is: nothing was written. {way}"
                )));
            }
            brought.push(studio::Member::new(slot.principal().actor(), slot.name));
        }
        let asked = studio::Conversing { subject, text, people, agents: &brought };
        self.inner.studio.channels.converse(asked, by).await
    }

    /// Brings the agent `slot` into `channel` as `by` to say there what it is asked there
    /// (`Channels::bring_asked`), as a fleet has its agents report in its own. Into one kept to
    /// its members only where it may be in one: where it may not, it is left out, as it was.
    pub async fn reports_in(&self, channel: &str, slot: &Slot, by: &Principal) -> Result<()> {
        let channels = &self.inner.studio.channels;
        let (reader, found) = channels.writable(channel, by).await?;
        let actor = slot.principal().actor();
        if found.members.iter().any(|member| member.actor == actor) {
            return Ok(());
        }
        let _held = self.inner.choosing.lock().await;
        if found.kept && self.may_be_in(&reader, slot).is_err() {
            return Ok(());
        }
        channels.bring_asked(&reader, &found.id, &slot.name).await
    }

    /// Brings the agent called `name`, which is there, into `channel`,
    /// as `by`, who may change agents and write in that channel: of one
    /// kept to its members, one of them, and only an agent that may be
    /// in one. Nothing else of the agent changes.
    pub async fn bring_in(&self, channel: &str, name: &str, by: &Principal) -> Result<()> {
        by.require("agents:write")?;
        let channels = &self.inner.studio.channels;
        let reader = channels.may_write(channel, by).await?;
        let _held = self.inner.choosing.lock().await;
        let slot = self.slot(name).await?;
        self.may_be_in(&reader, &slot)?;
        let id = reader.channel().unwrap_or(channel).to_string();
        channels.bring(&reader, &id, &slot.name).await
    }
}
