//! An agent taken away for good.
//!
//! Stopping an agent keeps its slot: it is there to be brought in again,
//! and is listed for ever. Taking it away is the one place its slot goes
//! ([`Agents::remove`]), and what holds something of an agent goes there
//! with it or is said here to stay.
//!
//! **Who may.** A person, who holds what changing agents takes and what
//! managing the factory takes (`agents:write`, `admin`): it cannot be put
//! back. Never an agent, whatever it holds, its own credential included,
//! and nothing the daemon does by itself.
//!
//! **What is in the way**, each said in a sentence that says what to do:
//!
//! - *Something that is running holds it.* A blueprint that brings an
//!   agent up holds it for as long as it counts it (`agent.ensure`, which
//!   is how a fleet's rows come to agents), and would bring it up again:
//!   such an agent is changed where it is counted, in its row. Which
//!   agents those are is what the instances say they hold, never what an
//!   agent is called.
//! - *It has work in hand*: waiting, at work, or answered and not yet
//!   said to have ended. Taking an agent away is not a way of calling
//!   work off; stopping it is.
//!
//! **What goes**, from the write that takes it away: its slot, with what
//! it was told, what it held and the names of the secrets and sign-ins
//! it was handed (the secrets and the sign-ins are the factory's, and
//! stay); its place in every channel it was in, each said as left; the
//! box of what was addressed to it (`studio::Inbox`); what
//! its program remembered of its conversations and the model it last
//! said it ran on. Before that write its credential is withdrawn and the
//! file it was in taken away, so the credential is nobody's from then
//! on: an agent left with none by a stop between the two is given one
//! when it next works, as one always is. After it, the folder that was
//! its own on this device: the home its program was shown behind its
//! wall, and whatever it wrote there when it worked in no project.
//!
//! That it is gone is said in the write that takes its slot away where
//! the factory's own journal keeps what is said of its project (an agent
//! that is nobody's, or of a project from before), and once that write
//! is kept where its project has a journal of its own (`Writing::say`):
//! an agent's row is the factory's, and the two are then two files. A
//! daemon stopped between them leaves it gone and not said, which
//! nothing waits to hear: a page reads the agents again as it comes back.
//!
//! **What stays** is what it said and did, which names it as text: what
//! it said in channels, the lines of history that name it, its turns as
//! what it was asked and answered, what was measured of it, and a card
//! given to it, which goes on saying so until somebody gives it to
//! another.
//!
//! **No heir.** A name is free again once its agent is gone (a fleet
//! that counts one in again calls it what it called the last), and a
//! later agent of the name is another agent. It has no credential, no
//! channel, no folder and no memory of the first one's. And the turns
//! the first one left are not its own: a slot says which turn of all
//! was the last its name had before it was made (`since`), and whatever
//! asks whose a turn is by the name goes by that: who holds a
//! conversation ([`Agents::holding`], [`Agents::beside`]), what an agent
//! was given ([`crate::filed`]), what an asking finds again
//! (`crate::asked`) and what is listed as its work ([`Agents::turns`]).
//! What names an agent as text and is read as "whoever is called that
//! now" is not held from it: a card that names it is its, and an event
//! the first one's actions said is answered for by what it holds.

use std::collections::BTreeMap;

use conductor::{Engine, InstanceFilter, InstanceInfo};
use kernel::{Error, Principal, PrincipalKind, Result, Source};
use rusqlite::params;
use serde::Serialize;
use serde_json::json;
use studio::CredentialKind;

use crate::filed::NTH;
use crate::{Agents, MODEL_REPORTED, Slot};

/// Where what an agent's program remembers of each conversation is kept
/// (`crate::turns`), under the agent's name, a `/` and the conversation.
pub(crate) const SESSIONS: &str = "agent.session";

/// What brings an agent up, of what is running ([`Agents::bringing`]).
#[derive(Clone, Debug)]
pub enum Brought {
    /// Nothing running holds it.
    Nobody,
    /// Its own text, applied for it by hand, with nothing above it.
    Own(InstanceInfo),
    /// Something that counts it and would bring it up again.
    By(InstanceInfo),
}

/// What taking an agent away came to, or would come to.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Removed {
    pub name: String,
    /// The project it worked in, where it worked in one.
    pub project: Option<String>,
    /// Whether it is gone: not where this was only asked about.
    pub removed: bool,
    /// Why it cannot be taken away as things stand, where it was only
    /// asked about: taken away, that is an error and nothing is answered.
    pub refused: Option<String>,
    /// Whether it was taking work.
    pub active: bool,
    /// The channels it was in, by name: it is in none.
    pub channels: Vec<String>,
    /// How many pieces of work it was asked, which stay as they are.
    pub turns: i64,
    /// The cards given to it that whoever asks may read: they stay given
    /// to that name.
    pub cards: Vec<String>,
    /// Why the folder that was its own is still there, where it could
    /// not be taken away.
    pub folder_left: Option<String>,
    /// How many changes it had asked for and were waiting, which are
    /// withdrawn: nothing is made as an agent that is gone.
    pub withdrawn: usize,
}

impl Agents {
    /// Takes an agent away for good, or with `look` says what that would
    /// come to and why it cannot be, and changes nothing. `engine` is
    /// asked what is running that holds it.
    pub async fn remove(&self, name: &str, by: &Principal, engine: &Engine, look: bool) -> Result<Removed> {
        by.require("agents:write")?;
        if by.kind != PrincipalKind::User {
            return Err(Error::denied(format!(
                "taking an agent away is a person's to do, and {} is not one: it cannot be put back. Somebody who \
                 manages this factory can, from a terminal (`{} agent remove {name}`) or on the Agents page",
                by.name,
                brand::CLI
            )));
        }
        by.require("admin")?;
        self.taken_away(name, by, engine, look).await
    }

    /// The same, asked of nobody: for what takes an agent away with
    /// something it was part of (a project deleted for good, `forgotten`),
    /// once whoever asked for that was held to what it takes.
    pub(crate) async fn taken_away(
        &self,
        name: &str,
        by: &Principal,
        engine: &Engine,
        look: bool,
    ) -> Result<Removed> {
        let brought = self.brought_by(&blueprint::expr::slug(name), engine).await?;
        // One added by hand is taken away with its own text, which is
        // stopped first, as somebody would stop it: once nothing is in
        // its way, and before the turn is held that its stopping asks.
        if let Brought::Own(own) = &brought
            && !look
        {
            let slot = self.slot(name).await?;
            if let Some(why) = self.in_hand_of(&slot).await? {
                return Err(Error::conflict(why));
            }
            engine.stop(&own.id).await?;
        }
        // Held as it is while one is brought up or given work: nothing
        // is asked of it, and nobody is called it, in between.
        let _held = self.inner.choosing.lock().await;
        let slot = self.slot(name).await?;
        let actor = slot.principal().actor();
        let refused = match self.held_by(&slot, &brought) {
            Some(why) => Some(why),
            None => self.in_hand_of(&slot).await?,
        };
        let turns = self.inner.given.lock().expect("agents lock").get(&slot.name).map_or(0, |given| given.0);
        let mut removed = Removed {
            name: slot.name.clone(),
            project: slot.project.clone(),
            active: slot.active,
            channels: self.channels_of(&actor).await?,
            turns,
            cards: self.cards_of(&actor, by).await,
            ..Removed::default()
        };
        match refused {
            Some(why) if look => removed.refused = Some(why),
            Some(why) => return Err(Error::conflict(why)),
            None => {}
        }
        if look {
            return Ok(removed);
        }
        // Its credential is nobody's before anything of it goes: stopped
        // here, it is an agent with none, which is given one when it
        // next works.
        self.inner.studio.identity.revoke_subject(CredentialKind::Agent, &slot.name).await?;
        self.inner.studio.identity.uninvite(CredentialKind::Agent, &slot.name).await?;
        let _ = std::fs::remove_file(self.token_path(&slot.name));
        let mut data = json!({ "slot": slot.name, "turns": turns });
        if let Some(project) = &slot.project {
            data["project"] = json!(project);
        }
        let (gone, by_whom) = (slot.name.clone(), Source::actor(by.actor()));
        // What answers who is in a kept channel lets it go before the
        // write that takes it out of each, and keeps that turn until the
        // write is made.
        let leaving = self.inner.studio.channels.leaving(&actor).await;
        let left = leaving.left();
        // So does what answers what a group's members hold, which a later
        // agent of the name is no heir to.
        let grouped = self.inner.studio.identity.grouped().letting_go(&actor);
        let member = actor.clone();
        let wrote = self.inner.journal.write(move |writing| {
            let within = writing.transaction();
            within.execute("DELETE FROM slots WHERE name = ?1", [&gone])?;
            // Each conversation its program remembered: the name, a `/`
            // and the conversation, and a name has no `/` in it.
            within.execute(
                "DELETE FROM kv WHERE namespace = ?1 AND key >= ?2 AND key < ?3",
                params![SESSIONS, format!("{gone}/"), format!("{gone}0")],
            )?;
            within
                .execute("DELETE FROM kv WHERE namespace = ?1 AND key = ?2", params![MODEL_REPORTED, gone])?;
            let left = studio::Channels::leave_all(writing, &left)?;
            // And the box of what was addressed to it: a later agent of
            // the name is no heir to it.
            studio::Inbox::forget_owner(writing, &format!("agent:{gone}"))?;
            studio::groups::forget_member(writing, &member, by_whom.clone())?;
            data["channels"] = json!(left);
            writing.say("agent.slot.removed", by_whom, data)?;
            Ok(left)
        });
        removed.channels = match wrote.await {
            Ok((left, _)) => left,
            Err(error) => {
                // Nothing was written: it is in each as it was.
                self.inner.studio.channels.stayed(leaving);
                self.inner.studio.identity.grouped().stayed(grouped);
                return Err(error);
            }
        };
        drop(leaving);
        // What it asked for and waits is withdrawn, saying why; where this
        // could not be done, an approval of it is refused all the same, its
        // credential being gone.
        let asks = told::Asks::new(&self.inner.journal, &self.inner.studio);
        let why = format!("{} was taken away", slot.name);
        removed.withdrawn = asks.withdraw_all_of(&actor, &why).await.unwrap_or_default();
        // What is kept of it in memory: its worker, which ends by being
        // let go of, what it was given and what its program said it ran on.
        self.inner.workers.lock().expect("agents lock").remove(&slot.name);
        self.inner.given.lock().expect("agents lock").remove(&slot.name);
        self.inner.reported.lock().expect("agents lock").remove(&slot.name);
        self.inner.hand.is_outside(&slot.name, false);
        self.inner.outliving.is(&slot.name, false);
        // And the folder that was its own, which no later agent of the
        // name is to find anything in.
        let folder = self.inner.config.home.join(&slot.name);
        if let Err(err) = std::fs::remove_dir_all(&folder)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            removed.folder_left = Some(format!(
                "the folder that was its own, `{}`, could not be taken away ({err}): take it away by hand, or \
                 an agent that is later called `{}` finds what is in it",
                folder.display(),
                slot.name
            ));
        }
        removed.removed = true;
        Ok(removed)
    }

    /// Why `slot` cannot be taken away for something running that holds
    /// it, if anything does: read from what the instances say they hold.
    fn held_by(&self, slot: &Slot, brought: &Brought) -> Option<String> {
        let Brought::By(holder) = brought else { return None };
        let table = match &holder.project {
            Some(project) => format!("the `agents` table of the project `{project}`"),
            None => "its project's `agents` table".to_string(),
        };
        Some(format!(
            "`{name}` is brought up by `{key}` ({blueprint}, {id}), which is running, counts it, and would bring \
             it up again. An agent of a fleet is changed in its row: in {table}, lower the row's count or take \
             the row away, and an agent that is no longer counted can be taken away. One that something else \
             brought up goes when that is stopped (`{cli} stop {id}`)",
            name = slot.name,
            key = holder.key,
            blueprint = holder.blueprint,
            id = holder.id,
            cli = brand::CLI,
        ))
    }

    /// What is running that brings `name` up, if anything does: read from
    /// what the instances say they hold and never from what the agent is
    /// called.
    pub async fn brought_by(&self, name: &str, engine: &Engine) -> Result<Brought> {
        Ok(self.bringing(engine).await?.remove(name).unwrap_or(Brought::Nobody))
    }

    /// What brings each agent up that something running holds, by name.
    /// An instance that holds an agent and is keyed by its name is the
    /// agent's own (`std/agent`, which `agent.ensure` runs in): under
    /// something else, that is what brings it up (a role, by its row);
    /// with nothing above it, the agent was added by hand. Anything else
    /// that holds one brings it up, as before agents had a text of their
    /// own, and is what counts.
    pub async fn bringing(&self, engine: &Engine) -> Result<BTreeMap<String, Brought>> {
        let filter = InstanceFilter { live_only: true, limit: 100_000, ..InstanceFilter::default() };
        let live = engine.instances(filter).await?;
        let mut brought = BTreeMap::new();
        for one in &live {
            for name in one.holds.iter().filter_map(|hold| hold.strip_prefix("agent:")) {
                let said = match (&one.parent, one.key == name) {
                    (_, false) => Brought::By(one.clone()),
                    (Some(parent), true) => match live.iter().find(|above| &above.id == parent) {
                        Some(above) => Brought::By(above.clone()),
                        None => Brought::Own(one.clone()),
                    },
                    (None, true) => Brought::Own(one.clone()),
                };
                let had = brought.entry(name.to_string()).or_insert(Brought::Nobody);
                if !matches!(had, Brought::By(_)) {
                    *had = said;
                }
            }
        }
        Ok(brought)
    }

    /// Why `slot` cannot be taken away for the work it has in hand, if it
    /// has any: waiting, at work, or answered and not yet said to have
    /// ended.
    async fn in_hand_of(&self, slot: &Slot) -> Result<Option<String>> {
        let in_hand = self.in_hand(std::iter::once(slot.name.as_str())).await?;
        let in_hand = in_hand.get(&slot.name).copied().unwrap_or(0);
        if in_hand == 0 {
            return Ok(None);
        }
        Ok(Some(format!(
            "`{name}` has {in_hand} piece{s} of work in hand, and taking an agent away is not a way of calling \
             work off. Stop it first, which ends what it is doing and calls off what waits behind that \
             (`{cli} agent stop {name}`), or let it finish what it has (`{cli} agent stop {name} --idle`); \
             then take it away",
            name = slot.name,
            s = if in_hand == 1 { "" } else { "s" },
            cli = brand::CLI,
        )))
    }

    /// The channels `actor` is in, by name: those that are open. One kept
    /// to its members is named to nobody here, as it is not where the
    /// agent is said to have been taken away.
    async fn channels_of(&self, actor: &str) -> Result<Vec<String>> {
        let own = studio::Channels::daemon(
            "which channels an agent that is to be taken away is in, said to whoever asks what taking it away \
             would come to",
        );
        let channels = self.inner.studio.channels.list(&own).await?;
        let within =
            channels.into_iter().filter(|one| !one.kept && one.members.iter().any(|one| one.actor == actor));
        Ok(within.map(|channel| channel.name).collect())
    }

    /// The cards given to `actor`, of the boards `by` may read.
    async fn cards_of(&self, actor: &str, by: &Principal) -> Vec<String> {
        let studio = &self.inner.studio;
        let mut cards = Vec::new();
        for collection in studio.collections.list(None).await.unwrap_or_default() {
            if collection.name != studio::board::BOARD || collection.may_read(by).is_err() {
                continue;
            }
            let listed = studio.board.list(&collection.project).await.unwrap_or_default();
            cards.extend(
                listed.into_iter().filter(|card| card.assignee.as_deref() == Some(actor)).map(|c| c.id),
            );
        }
        cards
    }

    /// Which turn of all was the last an agent called `name` was asked,
    /// in any file that keeps turns: what a slot made now says its own
    /// turns come after (`Slot::since`). Nought where nobody called that
    /// was ever asked anything. Asked of the index of turns by agent, for
    /// the newest row of the name in each file.
    pub(crate) async fn last_of_name(&self, name: &str) -> Result<i64> {
        let lookup = name.to_string();
        let asked = move |connection: &mut rusqlite::Connection| {
            journal::optional(connection.query_row(
                &format!("SELECT {NTH} FROM turns WHERE slot = ?1 ORDER BY rowid DESC LIMIT 1"),
                [&lookup],
                |row| row.get::<_, i64>(0),
            ))
        };
        let newest = self.of_each(self.may_hold(name), asked).await?;
        Ok(newest.into_iter().flatten().max().unwrap_or(0))
    }

    /// Which turn of all an agent's own turns come after, by its name:
    /// nought for one whose name nobody had before, and for a name that
    /// is nobody's now.
    pub(crate) async fn since(&self, name: &str) -> i64 {
        self.find(name).await.ok().flatten().map_or(0, |slot| slot.since)
    }
}
