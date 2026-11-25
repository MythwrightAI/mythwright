//! Which agent work goes to, and how an agent stops being given any.
//!
//! Work is given to an agent by name, to whoever a conversation was held
//! with, or to whichever agent is available. Which one that is, is said
//! here once. So is how an agent leaves: at once, with what it had in hand
//! stopped where it was, or when it has finished what it has.

use std::collections::HashMap;

use kernel::{Result, Source};
use rusqlite::params;
use serde_json::json;
use studio::CredentialKind;

use crate::filed::NTH;
use crate::{Agents, Order, Slot};

/// An agent that could be given a piece of work, and what choosing between
/// it and the others could turn on.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub slot: Slot,
    /// How many turns it has in hand: at work, or waiting behind that.
    pub in_hand: usize,
    /// How many turns it was ever given, and which was the last of them,
    /// counted from the first there was; 0 for one never given any.
    pub given: i64,
    pub last: i64,
    /// Whether a conversation beside this one is held with it.
    pub apart: bool,
}

/// How many turns an agent has in hand in one file, which the index of
/// turns by agent and status answers at once.
const IN_HAND: &str =
    "SELECT COUNT(*) FROM turns WHERE slot = ?1 AND status IN ('queued', 'running', 'answered')";

impl Agents {
    /// An active agent for `project`, if there is one: the answer to "is
    /// anyone available to look at this?". One of the project's own before
    /// any other, and of those the one with least in hand: an agent does
    /// one thing at a time, so what is given to a busy one waits behind
    /// what it has. With a `role`, one of the agents brought up in that
    /// role and no other. One that is leaving is never it.
    pub async fn available(&self, project: Option<&str>, role: Option<&str>) -> Result<Option<Slot>> {
        self.available_apart(project, role, &[], Order::First).await
    }

    /// The same, and not one of `apart` while there is anyone else of the
    /// project's own: so that several asked about one thing are each a
    /// different agent. Where there are fewer agents than askers, one is
    /// asked twice. An agent of another project is never it: what is
    /// asked of a role is that project's fleet's to do, and nobody
    /// else's. One that is no project's is anybody's. Among those that
    /// are left with as little in hand as each other, it is the one
    /// `order` puts first.
    pub async fn available_apart(
        &self,
        project: Option<&str>,
        role: Option<&str>,
        apart: &[String],
        order: Order,
    ) -> Result<Option<Slot>> {
        Ok(self.could_take(project, role, apart, order).await?.into_iter().next().map(|one| one.slot))
    }

    /// Everyone who could be given work asked of `role` in `project`, the
    /// one `order` would give it to first, each with what a choice between
    /// them could turn on. What [`crate::dispatch`] shows the rule.
    pub async fn could_take(
        &self,
        project: Option<&str>,
        role: Option<&str>,
        apart: &[String],
        order: Order,
    ) -> Result<Vec<Candidate>> {
        let slots = self.slots().await?;
        // Every turn there was: what the orders that go by it read, and
        // what a rule of somebody's own may go by.
        let given = self.given().await?;
        let in_role = |slot: &&Slot| role.is_none_or(|role| slot.role == role);
        let theirs =
            |slot: &&Slot| project.is_some() && slot.project.is_some() && slot.project.as_deref() != project;
        let mut active: Vec<&Slot> = slots
            .iter()
            .filter(|slot| slot.active && !slot.leaving)
            // One this factory does not start is given nothing.
            .filter(|slot| !slot.external())
            .filter(in_role)
            .filter(|slot| !theirs(slot))
            // One that is a kept channel's alone is asked nowhere else:
            // what is asked of a role is asked elsewhere.
            .filter(|slot| self.alone_in(slot).is_none())
            .collect();
        // Asked of those who could take it and of nobody else: what
        // another project's agents have in hand is in another file.
        let in_hand = self.in_hand(active.iter().map(|slot| slot.name.as_str())).await?;
        // Stable: among equals, the one that was there first.
        active.sort_by_key(|slot| {
            let elsewhere = !(project.is_some() && slot.project.as_deref() == project);
            let place = order.place(slot.rank, given.get(&slot.name).copied().unwrap_or((0, 0)));
            (elsewhere, apart.contains(&slot.name), in_hand.get(&slot.name).copied().unwrap_or(0), place)
        });
        let facts = |slot: &Slot| {
            let (given, last) = given.get(&slot.name).copied().unwrap_or((0, 0));
            Candidate {
                in_hand: in_hand.get(&slot.name).copied().unwrap_or(0),
                given,
                last,
                apart: apart.contains(&slot.name),
                slot: slot.clone(),
            }
        };
        Ok(active.into_iter().map(facts).collect())
    }

    /// How many turns each agent has been given, and which was its last,
    /// counted from the first there ever was: kept in memory, read once
    /// as agents are opened from every file that keeps turns
    /// (`crate::filed`), and again where turns are taken away. Each
    /// choosing used to ask the database, which took as long as there
    /// were turns.
    async fn given(&self) -> Result<HashMap<String, (i64, i64)>> {
        Ok(self.inner.given.lock().expect("agents lock").clone())
    }

    /// Counts a turn that has just been queued for `slot`, the `nth`
    /// there ever was.
    pub(crate) fn was_given(&self, slot: &str, nth: i64) {
        let mut given = self.inner.given.lock().expect("agents lock");
        let (count, last) = given.entry(slot.to_string()).or_insert((0, 0));
        (*count, *last) = (*count + 1, (*last).max(nth));
    }

    /// How many turns each of `slots` has in hand: at work, or waiting
    /// behind what it is at work on. Asked of each agent by itself, in
    /// each file that holds a turn of its, which the index of turns by
    /// slot and status answers at once however many turns have been
    /// kept: asked of all of them together it read every turn there
    /// ever was, on each choosing.
    pub async fn in_hand<'a>(&self, slots: impl Iterator<Item = &'a str>) -> Result<HashMap<String, usize>> {
        // Which agents each file is asked of.
        let mut asked: Vec<(journal::Journal, Vec<String>)> = Vec::new();
        for slot in slots {
            for journal in self.may_hold(slot) {
                match asked.iter_mut().find(|(one, _)| one.name() == journal.name()) {
                    Some((_, of)) => of.push(slot.to_string()),
                    None => asked.push((journal, vec![slot.to_string()])),
                }
            }
        }
        let mut in_hand: HashMap<String, usize> = HashMap::new();
        for (journal, of) in asked {
            let counted: Vec<(String, i64)> = journal
                .db()
                .call(move |connection| {
                    let mut statement = connection.prepare(IN_HAND)?;
                    let mut counted = Vec::new();
                    for slot in of {
                        let count = statement.query_row([&slot], |row| row.get(0))?;
                        counted.push((slot, count));
                    }
                    Ok(counted)
                })
                .await?;
            for (slot, count) in counted.into_iter().filter(|(_, count)| *count > 0) {
                *in_hand.entry(slot).or_default() += count as usize;
            }
        }
        Ok(in_hand)
    }

    /// The active agent a conversation was last held with, if there is
    /// one: what is said in it next is that agent's to hear, since nobody
    /// else was there for what went before. One that is leaving hears no
    /// more of it. A conversation's turns may be in any file that keeps
    /// turns (an agent of one project, and one that is nobody's, asked
    /// in one thread), so each is asked for its newest, and the newest
    /// of those is it.
    pub async fn holding(&self, thread: &str) -> Result<Option<Slot>> {
        let thread = thread.to_string();
        let asked = move |connection: &mut rusqlite::Connection| {
            journal::optional(connection.query_row(
                &format!("SELECT slot, {NTH} FROM turns WHERE thread = ?1 ORDER BY rowid DESC LIMIT 1"),
                [&thread],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            ))
        };
        let newest = self.of_each(self.every_turns(), asked).await?;
        let Some((name, nth)) = newest.into_iter().flatten().max_by_key(|(_, nth)| *nth) else {
            return Ok(None);
        };
        // Held with an agent that was taken away, it is held with
        // nobody, whoever is called that now (`crate::removed`).
        let theirs = |slot: &Slot| slot.active && !slot.leaving && nth > slot.since;
        Ok(self.slot(&name).await.ok().filter(theirs))
    }

    /// Who the other conversations are held with whose names begin as
    /// `prefix` does: the others asked about the same thing as `thread`.
    /// Each file is asked who its newest turn of each such conversation
    /// was of, and a conversation with turns in several is held with
    /// whoever the newest of them all was of.
    pub async fn beside(&self, prefix: &str, thread: &str) -> Result<Vec<String>> {
        // Everything that begins with the prefix sorts before the prefix
        // followed by the last character there is.
        let (from, to, own) = (prefix.to_string(), format!("{prefix}\u{10FFFF}"), thread.to_string());
        let asked = move |connection: &mut rusqlite::Connection| {
            let mut statement = connection.prepare(&format!(
                "SELECT thread, slot, {NTH} FROM turns AS last
                 WHERE thread >= ?1 AND thread < ?2 AND thread != ?3
                   AND rowid = (SELECT MAX(rowid) FROM turns WHERE thread = last.thread)",
            ))?;
            let rows = statement.query_map(params![from, to, own], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        };
        let mut held: HashMap<String, (i64, String)> = HashMap::new();
        for (thread, slot, nth) in self.of_each(self.every_turns(), asked).await?.into_iter().flatten() {
            if held.get(&thread).is_none_or(|(newest, _)| nth > *newest) {
                held.insert(thread, (nth, slot));
            }
        }
        // One held with an agent that was taken away is held with nobody
        // who is called that now.
        let since: HashMap<String, i64> =
            self.slots().await?.into_iter().map(|slot| (slot.name, slot.since)).collect();
        held.retain(|_, (nth, slot)| since.get(slot).is_none_or(|since| *nth > *since));
        let mut with: Vec<String> = held.into_values().map(|(_, slot)| slot).collect();
        with.sort();
        with.dedup();
        Ok(with)
    }

    /// Stops a slot at once: what it was doing is stopped where it was,
    /// what waited behind that is not begun, and its credential is
    /// revoked. Its history stays.
    pub async fn deactivate(&self, name: &str) -> Result<()> {
        let held = self.inner.choosing.lock().await;
        self.put_out(name).await?;
        drop(held);
        self.stop_turns_of(name, "its agent was stopped").await;
        Ok(())
    }

    /// Has a slot leave when it has nothing in hand: it is given no more
    /// work from now, and is stopped when what it has is finished. One
    /// with nothing in hand is stopped at once. Whether it is gone.
    pub async fn leave(&self, name: &str) -> Result<bool> {
        let _held = self.inner.choosing.lock().await;
        let slot = self.slot(name).await?;
        if !slot.active {
            return Ok(true);
        }
        let in_hand = self.in_hand(std::iter::once(name)).await?.get(&slot.name).copied().unwrap_or(0);
        if in_hand == 0 {
            self.put_out(&slot.name).await?;
            return Ok(true);
        }
        let (lookup, now) = (slot.name.clone(), self.inner.journal.now_ms());
        self.inner
            .journal
            .db()
            .call(move |c| {
                c.execute("UPDATE slots SET leaving = 1, updated = ?2 WHERE name = ?1", params![lookup, now])
            })
            .await?;
        if !slot.leaving {
            let mut data = json!({ "slot": slot.name, "in_hand": in_hand });
            if let Some(project) = &slot.project {
                data["project"] = json!(project);
            }
            let _ = self.inner.journal.emit("agent.slot.leaving", Source::actor("system"), data).await;
        }
        Ok(false)
    }

    /// Stops a slot that is leaving, once it has nothing in hand. Asked by
    /// its worker each time it finds nothing more to do.
    pub(crate) async fn gone_if_done(&self, name: &str) {
        let _held = self.inner.choosing.lock().await;
        let leaving = self.slot(name).await.is_ok_and(|slot| slot.active && slot.leaving);
        let in_hand = self.in_hand(std::iter::once(name)).await;
        if leaving && in_hand.is_ok_and(|in_hand| !in_hand.contains_key(name)) {
            let _ = self.put_out(name).await;
        }
    }

    /// Marks a slot as stopped and takes its credential away. Whoever
    /// calls holds `choosing`: nothing is given to it in between.
    async fn put_out(&self, name: &str) -> Result<()> {
        let slot = self.slot(name).await?;
        let (lookup, now) = (slot.name.clone(), self.inner.journal.now_ms());
        self.inner
            .journal
            .db()
            .call(move |c| {
                c.execute(
                    "UPDATE slots SET active = 0, leaving = 0, updated = ?2 WHERE name = ?1",
                    params![lookup, now],
                )
            })
            .await?;
        self.inner.studio.identity.revoke_subject(CredentialKind::Agent, &slot.name).await?;
        // And whatever invitation waited to let it in from outside.
        self.inner.studio.identity.uninvite(CredentialKind::Agent, &slot.name).await?;
        let _ = std::fs::remove_file(self.token_path(&slot.name));
        // Stopped, it leaves each channel it was in to say what it was asked there; where somebody
        // brought it in to listen it stays.
        let channels = &self.inner.studio.channels;
        let own = studio::Channels::daemon(
            "which channels an agent that stops was in to say what it was asked there",
        );
        for channel in channels.asked_into(&slot.name).await.unwrap_or_default() {
            let _ = channels.leave(&own, &channel, &slot.principal().actor()).await;
        }
        let mut data = json!({ "slot": slot.name });
        if let Some(project) = &slot.project {
            data["project"] = json!(project);
        }
        let _ = self.inner.journal.emit("agent.slot.stopped", Source::actor("system"), data).await;
        Ok(())
    }
}
