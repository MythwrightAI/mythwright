//! Who is in a channel: somebody added and somebody leaving, and whoever
//! says something there being in it ([`Channels::saying`] asks
//! [`comes_in`]): a person comes into an open channel by what they say
//! there; an agent says something only in one it is in, but for its
//! answer to what it was asked there, which brings it into an open one;
//! a bot and what is said as the system are as they were. A member
//! listens, or is there to say what it is asked there
//! ([`crate::Member::listens`]).

use kernel::{Error, Result, Source};
use rusqlite::params;

use crate::channels::{Channel, Channels, Member, joined_of, kind_of, left_of};
use crate::kept::Reader;

/// Whether `author` says something in `channel` as the write under way
/// finds it, and whether they come into it by saying it: refused, said as
/// they are, or said having come in, listening or not
/// ([`Channels::saying`]). Only an open channel is come into so: one kept
/// to its members is come into by its door.
pub(crate) fn comes_in(
    transaction: &rusqlite::Connection,
    channel: &Channel,
    author: &str,
    answers: bool,
) -> rusqlite::Result<Result<Option<bool>>> {
    let kind = kind_of(author);
    if kind != "user" && kind != "agent" {
        return Ok(Ok(None));
    }
    let within = transaction
        .prepare_cached("SELECT 1 FROM channel_members WHERE channel = ?1 AND member = ?2")?
        .exists([&channel.id, author])?;
    Ok(match (within, kind, channel.kept) {
        (true, ..) => Ok(None),
        (false, "user", false) => Ok(Some(true)),
        (false, "user", true) => Ok(None),
        (false, _, false) if answers => Ok(Some(false)),
        (false, ..) => Err(not_in(channel, author)),
    })
}

/// What an agent that is not in `channel` is refused saying something
/// there with.
pub(crate) fn not_in(channel: &Channel, author: &str) -> Error {
    let name = author.strip_prefix("agent:").unwrap_or(author);
    let how = match channel.kept {
        true => "somebody who is in it brings it in".to_string(),
        false => {
            "whoever may change agents brings it in, or a person names it there, which brings it in".into()
        }
    };
    Error::denied(format!(
        "`{name}` is not in {}, and an agent says something only in a channel it is in: {how} (`{} channel add {} \
         {name}`)",
        crate::door::called(channel),
        brand::CLI,
        channel.name
    ))
}

impl Channels {
    /// Adds a member. Adding one who is already there changes nothing.
    /// The row and the saying of it are one write. An agent is not added
    /// so to a channel kept to its members: it is brought in
    /// ([`Channels::bring`]).
    pub async fn join(&self, reader: &Reader, channel: &str, actor: &str, name: &str) -> Result<()> {
        self.join_as(reader, channel, actor, name, (false, true)).await
    }

    /// The same; `brought` for an agent that whoever keeps agents says
    /// may be in a kept channel ([`Channels::bring`]), and `listens` for
    /// one that is asked by what is said there naming nobody
    /// ([`Member::listens`]). One that is there and does not listen is
    /// made one that does where `listens` says so, and never the other
    /// way: an agent somebody brought in to listen is not made quieter
    /// by being asked there.
    pub(crate) async fn join_as(
        &self,
        reader: &Reader,
        channel: &str,
        actor: &str,
        name: &str,
        (brought, listens): (bool, bool),
    ) -> Result<()> {
        reader.writing()?;
        let _turn = self.memory.changing().await;
        let channel = self.get(reader, channel).await?;
        // Under the turn: no other bringing in is between this and the write.
        if channel.kept
            && brought
            && let Some(why) = self.memory.not_brought(&channel.id, channel.alone, actor)
        {
            return Err(Error::conflict(why));
        }
        if channel.kept && !brought && kind_of(actor) == "agent" {
            return Err(Error::invalid(format!(
                "{name} is an agent, and an agent comes into a channel kept to its members by being brought in \
                 as an agent is brought into any, which says whether it may be in one: this adds a person or a bot"
            )));
        }
        let member = Member { actor: actor.to_string(), name: name.to_string(), listens, manages: false };
        let (row, now, inbox) = (channel.clone(), self.journal.now_ms(), self.inbox.clone());
        let wrote = self.journal.write(move |writing| {
            if listens {
                // Somebody brought in to listen who was there to say what
                // they were asked: nothing more is said of it.
                writing.transaction().execute(
                    "UPDATE channel_members SET listens = 1 WHERE channel = ?1 AND member = ?2 AND listens = 0",
                    [&row.id, &member.actor],
                )?;
            }
            let added = writing.transaction().execute(
                "INSERT OR IGNORE INTO channel_members (channel, member, name, joined, listens)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![row.id, member.actor, member.name, now, member.listens],
            )?;
            if added > 0 {
                // Told to whoever joined by name, too: the memory has
                // them in it only once this is written.
                let source = Source::actor(&member.actor).taking(row.takes(&[&member.actor]).as_deref());
                writing.say_here("channel.joined", source, joined_of(&row, &member))?;
                // Whoever comes into a conversation has it in their box.
                crate::inboxed::tell_joined(writing, &inbox, &row, &member.actor, now)?;
            }
            Ok(())
        });
        wrote.await?;
        // After the write: until the tables have them, the memory does not.
        self.memory.joined(&channel.id, actor);
        Ok(())
    }

    pub async fn leave(&self, reader: &Reader, channel: &str, actor: &str) -> Result<()> {
        self.leave_as(reader, channel, actor, false).await
    }

    /// The same; `asked` for somebody who leaves by asking to
    /// ([`Channels::step_out`]), who is in it and is not the last person
    /// in a channel kept to its members.
    pub(crate) async fn leave_as(
        &self,
        reader: &Reader,
        channel: &str,
        actor: &str,
        asked: bool,
    ) -> Result<()> {
        reader.writing()?;
        let _turn = self.memory.changing().await;
        let channel = self.get(reader, channel).await?;
        if asked {
            crate::door::may_step_out(&channel, actor)?;
        }
        // Before the write: from here the memory has let them go,
        // whatever the tables still say.
        let was = self.memory.left(&channel.id, actor);
        let (row, leaves) = (channel.clone(), actor.to_string());
        let wrote = self.journal.write(move |writing| {
            writing.transaction().execute(
                "DELETE FROM channel_members WHERE channel = ?1 AND member = ?2",
                [&row.id, &leaves],
            )?;
            // Whoever left is no member when this is handed over, and is
            // told by name: or they would never learn that they left.
            let source = Source::actor(&leaves).taking(row.takes(&[&leaves]).as_deref());
            writing.say_here("channel.left", source, left_of(&row.id, row.project.as_deref(), &leaves))?;
            Ok(())
        });
        let wrote = wrote.await;
        if wrote.is_err() && was {
            // Nothing was written: they are in it as they were.
            self.memory.joined(&channel.id, actor);
        }
        wrote.map(|_| ())
    }

    /// Takes whoever is `leaving` out of every channel they are in, in a
    /// write somebody else is making, and says of each that they left:
    /// whoever is taken away for good (an agent) is in no channel from
    /// the write that takes it away, and not a moment after. What each
    /// that is open was called. The memory let them go when `leaving` was made
    /// ([`Channels::leaving`]), which is before this write.
    pub fn leave_all(
        writing: &mut journal::Writing<'_>,
        leaving: &crate::kept::Left,
    ) -> rusqlite::Result<Vec<String>> {
        let actor = leaving.actor();
        let within: Vec<(String, String, Option<String>, bool)> = writing
            .transaction()
            .prepare(
                "SELECT id, name, project, kept FROM channels
                 WHERE id IN (SELECT channel FROM channel_members WHERE member = ?1) ORDER BY name",
            )?
            .query_map([actor], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
            .collect::<rusqlite::Result<_>>()?;
        writing.transaction().execute("DELETE FROM channel_members WHERE member = ?1", [actor])?;
        let mut left = Vec::with_capacity(within.len());
        for (id, name, project, kept) in within {
            let source = Source::actor(actor).taking(crate::kept::takes(&id, kept, &[actor]).as_deref());
            writing.say_here("channel.left", source, left_of(&id, project.as_deref(), actor))?;
            // One kept to its members is named to nobody by this: what is
            // handed back is said to whoever may see agents.
            if !kept {
                left.push(name);
            }
        }
        Ok(left)
    }
}
