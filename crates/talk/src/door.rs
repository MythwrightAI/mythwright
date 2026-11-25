//! The door of a channel kept to its members: how one is made, how an
//! open one comes to be kept and a kept one open again, who is in one
//! from the first, how somebody leaves, and whether its agents are asked
//! nowhere else.
//!
//! Each is one write with what is said of it, under the turn at changing
//! who is in anything ([`crate::kept`]), and the memory that answers who
//! is told an event is changed on the side of that write where it says
//! less than the tables and never more: **before** the write that keeps
//! a channel, so that from the write on its events find its members
//! there, and **after** the write that opens one again, until which its
//! events are still its members'. A message is said as its channel is in
//! the write that says it ([`as_it_is`]), so that one said in the moment
//! a channel is kept is not said to everybody.
//!
//! **What a kept channel is called.** An open channel's name is one of
//! the names channels share. Kept, it is called what its name was
//! (`title`), which another may be called too, and its name is its own
//! id: the name is free for whoever makes a channel next. Opened again
//! it takes a name among those channels share again, the one its title
//! comes to unless another is said, and is refused where that is taken.
//!
//! **Who is told.** That an open channel was kept, and that a kept one
//! is open again, is said to whoever may read channels: everybody could
//! read it until the one, and can from the other, and whoever is not in
//! it has to learn to stop showing it. What is said of a kept channel's
//! own word about its agents is its members'.
//!
//! Who may is asked by whoever holds the door open, above this crate: a
//! route, which also knows who the factory's owner is, and whoever keeps
//! agents, which says first what becomes of those in the channel.

use kernel::{Error, Principal, Result, Source};
use serde_json::json;

use crate::channels::{Channel, Channels, MAX_TITLE, Member, kind_of, taken};
use crate::kept::Reader;

/// A channel as whoever makes one asks for it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Asked<'a> {
    /// What it is called: its name where it is open, its title where it
    /// is kept.
    pub name: &'a str,
    pub project: Option<&'a str>,
    pub topic: &'a str,
    /// Kept to its members.
    pub kept: bool,
    /// Its agents are asked nowhere else: said of a kept one.
    pub alone: bool,
    /// Who is in it with its maker from the write that makes it: people
    /// and bots. An agent is brought in, by whoever keeps agents.
    pub members: &'a [Member],
}

/// What a channel is called, for a sentence: its title where it is kept.
pub fn called(channel: &Channel) -> String {
    match &channel.title {
        Some(title) if channel.kept => format!("`{title}`"),
        _ => format!("#{}", channel.name),
    }
}

/// Whether `actor` may leave `channel` by asking to: somebody who is in
/// it, and not the last person in one kept to its members, which nobody
/// who is in it could then read or add anybody to.
pub(crate) fn may_step_out(channel: &Channel, actor: &str) -> Result<()> {
    if !channel.has(actor) {
        return Err(Error::invalid(format!("whoever asks is not in {}", called(channel))));
    }
    let person = |actor: &str| kind_of(actor) == "user";
    let others = channel.members.iter().any(|member| member.actor != actor && person(&member.actor));
    if channel.kept && person(actor) && !others {
        return Err(Error::conflict(format!(
            "{} is kept to its members, and nobody else is in it: add somebody first (`{cli} channel add`), or \
             have the factory's owner open it again or remove it (`{cli} channel open`, `{cli} channel remove`)",
            called(channel),
            cli = brand::CLI
        )));
    }
    Ok(())
}

/// Whether `by` may keep the open channel `channel` to its members:
/// somebody who is in it, or they would keep it from themselves, and
/// from everybody but those who are.
pub fn may_keep(channel: &Channel, by: &Principal) -> Result<()> {
    if channel.has(&by.actor()) {
        return Ok(());
    }
    Err(Error::denied(format!(
        "{} is not in {}, and a channel is kept to its members by one of them: join it first",
        by.name,
        called(channel)
    )))
}

/// `channel` as its row says it in the write under way: what it is
/// called, whether it is kept and whether it is archived may be other
/// than when it was read.
pub(crate) fn as_it_is(
    transaction: &rusqlite::Connection,
    mut channel: Channel,
) -> rusqlite::Result<Channel> {
    type Now = (String, bool, Option<String>, bool, bool, Option<String>);
    let now: Option<Now> = journal::optional(transaction.query_row(
        "SELECT name, kept, title, alone, conversation, archived FROM channels WHERE id = ?1",
        [&channel.id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    ))?;
    if let Some((name, kept, title, alone, conversation, archived)) = now {
        (channel.name, channel.kept, channel.title, channel.alone) = (name, kept, title, alone);
        (channel.conversation, channel.archived) = (conversation, crate::archived::read(archived));
    }
    Ok(channel)
}

/// What is said of a channel kept, opened again, or told whether its
/// agents are asked nowhere else.
pub(crate) fn kept_of(channel: &Channel, was: Option<&str>) -> serde_json::Value {
    let mut data = json!({
        "channel": channel.id, "name": channel.name, "kept": channel.kept, "alone": channel.alone,
    });
    for (key, said) in
        [("title", channel.title.as_deref()), ("was", was), ("project", channel.project.as_deref())]
    {
        if let Some(said) = said {
            data[key] = json!(said);
        }
    }
    data
}

impl Channels {
    /// Creates a channel as `asked`, with whoever makes it and everybody
    /// it is asked with in it, as one write.
    pub async fn create_as(&self, asked: Asked<'_>, by: &Principal) -> Result<Channel> {
        by.require(crate::kept::WRITE)?;
        if asked.alone && !asked.kept {
            return Err(Error::invalid(
                "`alone` is said of a channel kept to its members: its agents are then asked nowhere else",
            ));
        }
        if let Some(agent) = asked.members.iter().find(|member| kind_of(&member.actor) == "agent") {
            return Err(Error::invalid(format!(
                "{} is an agent, and an agent comes into a channel by being brought in, which says whether it \
                 may be in one: a channel is made with people and bots",
                agent.name
            )));
        }
        let (title, slug) = (asked.name.trim(), blueprint::expr::slug(asked.name));
        if asked.kept && (title.is_empty() || title.chars().count() > MAX_TITLE) {
            return Err(Error::invalid(format!(
                "a channel is called something, in {MAX_TITLE} letters at most"
            )));
        }
        if !asked.kept && slug.is_empty() {
            return Err(Error::invalid("a channel needs a name with at least one letter or digit"));
        }
        let id = self.journal.new_id("ch");
        let channel = Channel {
            // A kept channel's name is its own id: what it is called is
            // not among the names channels share.
            name: if asked.kept { id.clone() } else { slug.clone() },
            id,
            project: asked.project.map(str::to_string),
            topic: asked.topic.to_string(),
            created: self.journal.now_ms(),
            members: Vec::new(),
            kept: asked.kept,
            title: asked.kept.then(|| title.to_string()),
            alone: asked.alone,
            conversation: false,
            archived: None,
        };
        self.make(channel, asked.members.to_vec(), by).await.map_err(|refused| taken(refused, &slug))
    }

    /// Creates a channel kept to its members, of whom its creator is the
    /// first: read by nobody else, by any way of reading
    /// ([`crate::kept`]). It is called `title`, which another may be
    /// called too: its name, among the names channels share, is its own
    /// id, so that nobody who makes a channel is told one of that name
    /// is there.
    pub async fn create_kept(
        &self,
        title: &str,
        project: Option<&str>,
        topic: &str,
        by: &Principal,
    ) -> Result<Channel> {
        let asked = Asked { name: title, project, topic, kept: true, ..Asked::default() };
        self.create_as(asked, by).await
    }

    /// The same, of a channel whose agents are asked nowhere else
    /// ([`crate::kept`]): an agent that is in it is given no work
    /// anywhere but there.
    pub async fn create_kept_alone(
        &self,
        title: &str,
        project: Option<&str>,
        topic: &str,
        by: &Principal,
    ) -> Result<Channel> {
        let asked = Asked { name: title, project, topic, kept: true, alone: true, ..Asked::default() };
        self.create_as(asked, by).await
    }

    /// The channel `asked` names, created if it is not there: what a
    /// blueprint's step asks, which is the same done twice. An open one
    /// is the one of that name. A kept one is called nothing among the
    /// names channels share, so the one such a step made before, or one
    /// somebody kept since, is found as the kept channel of the same
    /// project that is called so and has `by` in it: the first made,
    /// where several are. Nothing of a channel that is there is changed:
    /// asked for kept where an open one has the name, it is refused, so
    /// that nothing meant for a kept channel is said in an open one.
    pub async fn ensure_as(&self, asked: Asked<'_>, by: &Principal) -> Result<Channel> {
        let slug = blueprint::expr::slug(asked.name);
        match self.found(&slug, by).await {
            Ok(open) if asked.kept && !open.kept => {
                return Err(Error::conflict(format!(
                    "#{} is an open channel, and this asks for one kept to its members: keep that one (`{} \
                     channel keep {}`), or ask for another name",
                    open.name,
                    brand::CLI,
                    open.name
                )));
            }
            Ok(open) => return Ok(open),
            Err(refused) if refused.kind != kernel::ErrorKind::NotFound => return Err(refused),
            Err(_) => {}
        }
        // The kept channels of that project, as they were made: which is
        // called so is asked of each as `by` finds it.
        let project = asked.project.map(str::to_string);
        let kept: Vec<(String, String)> = self
            .journal
            .db()
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT id, title FROM channels
                     WHERE kept = 1 AND conversation = 0 AND project IS ?1 AND title IS NOT NULL
                     ORDER BY created, rowid",
                )?;
                let rows = statement.query_map([&project], |row| Ok((row.get(0)?, row.get(1)?)))?;
                rows.collect()
            })
            .await?;
        let actor = by.actor();
        for (id, _) in kept.iter().filter(|(_, title)| blueprint::expr::slug(title) == slug) {
            if let Ok(found) = self.found(id, by).await
                && found.has(&actor)
            {
                return Ok(found);
            }
        }
        let open = !asked.kept;
        match self.create_as(Asked { members: &[], ..asked }, by).await {
            // Somebody made it in the moment between the look and the
            // making: two texts that make sure of one channel as they
            // are applied together. It is there, which is what was asked.
            Err(taken) if open && taken.kind == kernel::ErrorKind::Conflict => {
                self.found(&slug, by).await.map_err(|_| taken)
            }
            made => made,
        }
    }

    /// Keeps the open channel `reader` is leave to write in to its
    /// members, as `by`: whoever is in it reads it from here, and nobody
    /// else. It is called what its name was, and its name is its own id.
    /// Refused while an agent in it is held to another channel. One
    /// that is kept already is as it was.
    pub async fn keep(&self, reader: &Reader, channel: &str, by: &Principal) -> Result<Channel> {
        reader.writing()?;
        let _turn = self.memory.changing().await;
        let mut channel = self.get(reader, channel).await?;
        if channel.kept {
            return Ok(channel);
        }
        may_keep(&channel, by)?;
        let agents = channel.members.iter().filter(|member| kind_of(&member.actor) == "agent");
        for agent in agents {
            if let Some(why) = self.memory.not_brought(&channel.id, false, &agent.actor) {
                return Err(Error::conflict(format!(
                    "{why}: take it out of {} first (`{} channel leave {} --agent {}`)",
                    called(&channel),
                    brand::CLI,
                    channel.name,
                    agent.name
                )));
            }
        }
        let was = std::mem::replace(&mut channel.name, channel.id.clone());
        (channel.kept, channel.title) = (true, Some(was.clone()));
        // Before the write: from the write on, what is said of it is its
        // members', and the memory is where they are found.
        self.memory.keep(&channel.id, channel.members.iter().map(|member| member.actor.clone()), false);
        let (row, actor) = (channel.clone(), by.actor());
        let wrote = self.journal.write(move |writing| {
            let kept = writing.transaction().execute(
                "UPDATE channels SET kept = 1, title = name, name = id WHERE id = ?1 AND kept = 0",
                [&row.id],
            )?;
            if kept == 1 {
                // Said to whoever may read channels: everybody read it
                // until now, and whoever is not in it stops showing it.
                writing.say_here("channel.kept", Source::actor(&actor), kept_of(&row, Some(&was)))?;
            }
            Ok(())
        });
        if let Err(refused) = wrote.await {
            // It is open as it was.
            self.memory.forget(&channel.id);
            return Err(refused);
        }
        // An archived one is known by the name it has now.
        self.memory.archived_as(&channel);
        Ok(channel)
    }

    /// Opens the kept channel `reader` is leave to write in again, as
    /// `by`: anybody who may read channels reads it from here, what was
    /// said in it while it was kept with the rest. It is called `name`
    /// among the names channels share, or what its title comes to, and
    /// is refused where another has that. Whether `by` may is asked by
    /// whoever asks this: it is the factory's owner's to do. One that
    /// is open already is as it was.
    pub async fn open_again(
        &self,
        reader: &Reader,
        channel: &str,
        name: Option<&str>,
        by: &Principal,
    ) -> Result<Channel> {
        reader.writing()?;
        let _turn = self.memory.changing().await;
        let mut channel = self.get(reader, channel).await?;
        if !channel.kept {
            return Ok(channel);
        }
        if channel.conversation {
            return Err(Error::conflict(format!(
                "{} is a conversation, which is its members' and listed among no channels: one of them opens it \
                 as a channel first (`{} channel from {}`), and that channel is then opened to everybody",
                called(&channel),
                brand::CLI,
                channel.id
            )));
        }
        let was = channel.title.take().unwrap_or_default();
        let slug = blueprint::expr::slug(name.unwrap_or(&was));
        if slug.is_empty() {
            return Err(Error::invalid(
                "an open channel needs a name with at least one letter or digit: say what this one is to be \
                 called",
            ));
        }
        (channel.name, channel.kept, channel.alone) = (slug.clone(), false, false);
        let (row, actor) = (channel.clone(), by.actor());
        let wrote = self.journal.write(move |writing| {
            let opened = writing.transaction().execute(
                "UPDATE channels SET kept = 0, alone = 0, name = ?2, title = NULL WHERE id = ?1 AND kept = 1",
                [&row.id, &row.name],
            )?;
            if opened == 1 {
                // What it was called while it was kept was its
                // members': said of it now is what it is called open.
                writing.say_here("channel.kept", Source::actor(&actor), kept_of(&row, None))?;
            }
            Ok(())
        });
        wrote.await.map_err(|refused| match refused.kind {
            kernel::ErrorKind::Conflict => Error::conflict(format!(
                "there is already a channel called `{slug}`: say what this one is to be called once it is \
                 open (`{} channel open {} --name <name>`)",
                brand::CLI,
                channel.id
            )),
            _ => refused,
        })?;
        // After the write: until it, what was said of it was its members'.
        self.memory.forget(&channel.id);
        self.memory.archived_as(&channel);
        Ok(channel)
    }

    /// Says whether the agents of the kept channel `reader` is leave to
    /// write in are asked nowhere else, as `by`. Refused, to say that
    /// they are, while one of them is in another channel kept to its
    /// members: an agent is held so to one channel.
    pub async fn hold_alone(
        &self,
        reader: &Reader,
        channel: &str,
        alone: bool,
        by: &Principal,
    ) -> Result<Channel> {
        reader.writing()?;
        let _turn = self.memory.changing().await;
        let mut channel = self.get(reader, channel).await?;
        if !channel.kept {
            return Err(Error::invalid(format!(
                "{} is open, and `alone` is said of a channel kept to its members: its agents are then asked \
                 nowhere else",
                called(&channel)
            )));
        }
        if channel.alone == alone {
            return Ok(channel);
        }
        if alone {
            let agents = channel.members.iter().filter(|member| kind_of(&member.actor) == "agent");
            for agent in agents {
                if let Some(why) = self.memory.not_brought(&channel.id, true, &agent.actor) {
                    return Err(Error::conflict(why));
                }
            }
            // Before the write: held from here, whatever the row says.
            self.memory.alone(&channel.id, true);
        }
        channel.alone = alone;
        let (row, actor) = (channel.clone(), by.actor());
        let wrote = self.journal.write(move |writing| {
            let said = writing.transaction().execute(
                "UPDATE channels SET alone = ?2 WHERE id = ?1 AND kept = 1",
                rusqlite::params![row.id, row.alone],
            )?;
            if said == 1 {
                let source = Source::actor(&actor).taking(row.takes(&[]).as_deref());
                writing.say_here("channel.kept", source, kept_of(&row, None))?;
            }
            Ok(())
        });
        let wrote = wrote.await;
        // Let go after the write that says so, and where the write that
        // was to hold them was not made.
        if wrote.is_err() == alone {
            self.memory.alone(&channel.id, false);
        }
        wrote.map(|_| channel)
    }

    /// Takes `by` out of the channel `reader` is leave to write in, at
    /// their own asking. Refused for somebody who is not in it, and for
    /// the last person in one kept to its members.
    pub async fn step_out(&self, reader: &Reader, channel: &str, by: &Principal) -> Result<()> {
        self.leave_as(reader, channel, &by.actor(), true).await
    }
}
