//! Which agents a message said in a channel is for, and the turn each is
//! given for it.

use blueprint::Labels;
use journal::optional;
use kernel::Event;

use crate::filed::NTH;
use crate::{Agents, Slot, TurnRequest};

/// How many earlier messages of a channel an agent is shown with a new one.
const CONTEXT_MESSAGES: usize = 12;

/// How a file is shown to whoever reads a channel, as an agent asked there
/// is told: attached with this program's own command. Told nothing, one
/// said where its file was, in a folder of the device the reader has no
/// way to.
fn attaching(channel: &str) -> String {
    format!(
        "To show somebody a file, attach it, since where it is on this device means nothing to whoever \
         reads the channel: `{cli} post {channel} --attach <file> <words>`",
        cli = brand::CLI
    )
}

impl Agents {
    /// Whether `slot` has a turn for `message` already, in any file that
    /// holds a turn of its. An agent has one turn for a message however
    /// many times the message is seen, which the database holds for the
    /// turns of one file; an agent given to another project has turns in
    /// two, so each is asked before one is queued. Messages are heard by
    /// one listener, one at a time: nothing queues a second in between.
    pub(crate) async fn has_turn_for(&self, slot: &str, message: &str) -> bool {
        let (lookup, message) = (slot.to_string(), message.to_string());
        let asked = move |connection: &mut rusqlite::Connection| {
            optional(connection.query_row(
                "SELECT 1 FROM turns WHERE slot = ?1 AND reply_to = ?2 AND kind = 'message'",
                [&lookup, &message],
                |row| row.get::<_, i64>(0),
            ))
        };
        let found = self.of_each(self.may_hold(slot), asked).await;
        found.is_ok_and(|found| found.into_iter().any(|found| found.is_some()))
    }

    /// The thread `slot` is being asked in just now in `channel`, if it is
    /// being asked in one: the thread the message its turn in hand there
    /// answers was said under. Nothing for an agent with nothing in hand
    /// there, one asked in the channel itself, one given work by a
    /// blueprint (which answers no message), and a channel there is not.
    ///
    /// What an agent posts for itself while it answers is said where it
    /// was asked, as its answer is: its program is told the thread, and
    /// one that posts without saying it has not thereby left the thread.
    ///
    /// `reader` is the agent's own leave to write in that channel, which
    /// whoever takes its message has asked for.
    pub async fn asked_under(&self, reader: &studio::Reader, slot: &str, channel: &str) -> Option<String> {
        let channel = self.inner.studio.channels.get(reader, channel).await.ok()?.id;
        let (lookup, held) = (slot.to_string(), self.may_hold(slot));
        let asked = move |connection: &mut rusqlite::Connection| {
            optional(connection.query_row(
                &format!(
                    "SELECT reply_to, {NTH} FROM turns WHERE slot = ?1 AND channel = ?2 AND status = 'running' \
                     AND reply_to IS NOT NULL ORDER BY rowid DESC LIMIT 1"
                ),
                [&lookup, &channel],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            ))
        };
        let newest = self.of_each(held, asked).await.ok()?;
        let (asked, _) = newest.into_iter().flatten().max_by_key(|(_, nth)| *nth)?;
        self.inner.studio.channels.message(reader, &asked).await.ok()?.thread
    }

    /// The channel `slot` has a turn in hand in just now, the newest where
    /// it has several: where what it asks for while it works is asked
    /// from, so that what comes of it is said there. Nothing where it has
    /// none in a channel (a piece of work a step gave it).
    pub async fn asking_in(&self, slot: &str) -> Option<String> {
        let (lookup, held) = (slot.to_string(), self.may_hold(slot));
        let in_hand = move |connection: &mut rusqlite::Connection| {
            optional(connection.query_row(
                &format!(
                    "SELECT channel, {NTH} FROM turns WHERE slot = ?1 AND status = 'running' \
                     AND channel IS NOT NULL ORDER BY rowid DESC LIMIT 1"
                ),
                [&lookup],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            ))
        };
        let newest = self.of_each(held, in_hand).await.ok()?;
        newest.into_iter().flatten().max_by_key(|(_, nth)| *nth).map(|(channel, _)| channel)
    }

    /// The model that writes what `slot` posts for itself in `channel` while
    /// a turn of it is in hand there, as its answer is put down with: what
    /// its program last said it used, else what the slot runs on as far as
    /// its provider has said. Nothing with no turn in hand there, so that a
    /// message the agent posts beside its answer says what wrote it as the
    /// answer does, and the two read as one.
    pub async fn model_in_hand(&self, reader: &studio::Reader, slot: &str, channel: &str) -> Option<String> {
        let channel = self.inner.studio.channels.get(reader, channel).await.ok()?.id;
        let (lookup, held) = (slot.to_string(), self.may_hold(slot));
        let in_hand = move |connection: &mut rusqlite::Connection| {
            optional(connection.query_row(
                "SELECT 1 FROM turns WHERE slot = ?1 AND channel = ?2 AND status IN ('running', 'answered') LIMIT 1",
                [&lookup, &channel],
                |row| row.get::<_, i64>(0),
            ))
        };
        let found = self.of_each(held, in_hand).await.ok()?;
        if !found.into_iter().any(|one| one.is_some()) {
            return None;
        }
        let said = self.inner.reported.lock().expect("agents lock").get(slot).cloned();
        match said {
            Some(said) => Some(said),
            None => self.model_name(&self.find(slot).await.ok()??),
        }
    }

    /// The model `slot`'s program last said it used, where it has said one.
    pub fn model_reported(&self, slot: &str) -> Option<String> {
        self.inner.reported.lock().expect("agents lock").get(slot).cloned()
    }

    /// The agent a person in a thread is already talking to: the one that
    /// spoke last in it; failing that, one the message it hangs from was
    /// for, by name or by having answered it in the channel.
    async fn agent_in_thread<'a>(
        &self,
        reader: &studio::Reader,
        root: &str,
        said: &[studio::Message],
        newest: &str,
        members: &[&'a str],
    ) -> Option<&'a str> {
        let member = |actor: &str| {
            let name = actor.strip_prefix("agent:")?;
            members.iter().copied().find(|one| *one == name)
        };
        if let Some(spoke) =
            said.iter().rev().filter(|message| message.id != newest).find_map(|m| member(&m.author))
        {
            return Some(spoke);
        }
        let named = said.first().map(|first| studio::channels::mentions(&first.body)).unwrap_or_default();
        if let Some(named) = members.iter().copied().find(|one| named.iter().any(|name| name == one)) {
            return Some(named);
        }
        let answered = self.inner.studio.channels.answered_by(reader, root).await.unwrap_or_default();
        answered.iter().find_map(|actor| member(actor))
    }

    /// Decides which agents, if any, a channel message is for, and queues a
    /// turn for each.
    pub(crate) async fn dispatch(&self, event: &Event) {
        let studio = &self.inner.studio;
        let text = |key: &str| event.data[key].as_str().unwrap_or_default().to_string();
        let (channel_id, author, message_id) = (text("channel"), text("author"), text("id"));
        // The daemon's own reading: which agent a message is for, and
        // what that agent is shown of where it was asked, is worked out
        // from the channel as it is, whoever said the message.
        let own = &studio::Channels::daemon(
            "which agents a message is for, what each is shown of where it was asked, and what the factory says \
             there of one that was not asked",
        );
        let Ok(channel) = studio.channels.get(own, &channel_id).await else { return };
        // Nobody is asked anything in an archived channel: nothing is said
        // there, and what was said as it was archived is answered by nobody.
        if channel.archived.is_some() {
            return;
        }
        let mentioned: Vec<String> = event.data["mentions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|m| m.as_str().map(str::to_string))
            .collect();
        let mut members: Vec<String> = channel
            .members
            .iter()
            .filter_map(|member| member.actor.strip_prefix("agent:").map(str::to_string))
            .collect();
        if members.is_empty() && mentioned.is_empty() {
            return;
        }
        let Ok(slots) = self.slots().await else { return };
        let from_person = author.starts_with("user:");
        // A person who names an agent that is up in an open channel it is not in brings it in, as
        // their saying something brings them in, to say what it is asked there.
        if from_person && !channel.kept {
            let up = |slot: &&Slot| slot.active && !slot.leaving;
            for slot in slots.iter().filter(up).filter(|slot| mentioned.contains(&slot.name)) {
                if !members.contains(&slot.name)
                    && studio.channels.bring_asked(own, &channel.id, &slot.name).await.is_ok()
                {
                    members.push(slot.name.clone());
                }
            }
        }
        // One this factory does not start is asked nothing by what is
        // said: a message that names it is there for it to read, and
        // starts nothing. Nor is it the channel's one agent, to whom
        // what names nobody is said.
        let outside: Vec<Slot> = slots.into_iter().filter(|slot| slot.external()).collect();
        // What names it is put in its box, in the channel or not, where
        // it may read the channel: that is how it comes to hear of it.
        self.tell_named(&outside, &mentioned, &channel, &message_id, &author).await;
        let outside: Vec<String> = outside.into_iter().map(|slot| slot.name).collect();
        let members: Vec<&str> = members
            .iter()
            .map(String::as_str)
            .filter(|name| !outside.iter().any(|one| one == name))
            .collect();
        if members.is_empty() {
            return;
        }
        // A channel that offers discussions takes what begins one, and
        // what is said in one's thread, for itself; and where it has a
        // lead, a person who names no agent is talking to the lead.
        let lead = match self.handed(&channel, event, &members).await {
            crate::discussing::Handed::Taken => return,
            crate::discussing::Handed::Lead(lead) => members.iter().copied().find(|one| *one == lead),
            crate::discussing::Handed::Not => None,
        };
        // Asked by what names nobody: those in it to listen.
        let listens = |name: &str| {
            channel.members.iter().any(|member| member.listens && member.actor == format!("agent:{name}"))
        };
        let listening: Vec<&str> = members.iter().copied().filter(|name| listens(name)).collect();
        let thread = event.data["thread"].as_str().map(str::to_string);
        let said = match &thread {
            Some(root) => studio.channels.thread(own, root).await.unwrap_or_default(),
            None => Vec::new(),
        };
        let mut targets: Vec<&str> =
            members.iter().copied().filter(|name| mentioned.iter().any(|m| m == name)).collect();
        // What names a bot of the channel and no agent is said to the bot,
        // which has its own way of hearing it: no agent takes it up.
        let to_a_bot = channel.members.iter().any(|member| {
            member.actor.strip_prefix("bot:").is_some_and(|bot| mentioned.iter().any(|named| named == bot))
        }) || mentioned.contains(&studio.channels.speaker.name());
        // Nor is what names an agent this factory does not start said to
        // whoever else is there: it was said to that one.
        let to_outside = mentioned.iter().any(|named| outside.contains(named));
        if targets.is_empty() && from_person && !to_a_bot && !to_outside {
            // With nobody named: in a thread, a person is talking to the
            // agent they were already talking to there; in the channel, to
            // its agent when it has only one.
            let already = match (&thread, lead) {
                (_, Some(lead)) => Some(lead),
                (Some(root), None) => self.agent_in_thread(own, root, &said, &message_id, &members).await,
                (None, None) => None,
            };
            match already {
                Some(agent) => targets.push(agent),
                None if listening.len() == 1 => targets.push(listening[0]),
                None => {}
            }
        }
        targets.retain(|name| author != format!("agent:{name}"));
        if targets.is_empty() {
            return;
        }
        // Agents and bots that answer one another are held back after so
        // many turns an hour, where no discussion holds them.
        if !self.may_start(&channel, &author).await {
            return;
        }
        // What the agent is shown: in a thread, the thread; otherwise the
        // channel's recent messages. An agent in a thread can still read
        // the channel for itself when it needs to.
        let (mut prompt, history) = match &thread {
            Some(root) => {
                let earlier: Vec<_> =
                    said.iter().filter(|message| message.id != message_id).cloned().collect();
                // The message the thread hangs from is always shown, with
                // the most recent of what was said under it.
                let skip = earlier.len().saturating_sub(CONTEXT_MESSAGES).max(1).min(earlier.len());
                let mut shown: Vec<_> = earlier.first().cloned().into_iter().collect();
                shown.extend(earlier.into_iter().skip(skip));
                // Which thread, and how something is put in it: its answer
                // is said here for it, and a file it has made it posts
                // itself, which lands here while it answers and in the
                // channel at any other time unless it says the thread.
                let opening = format!(
                    "You are in a thread in the channel #{name}. Only the thread is shown here; \
                     `{cli} read {name}` shows what was said in the channel itself, if you need it. \
                     What you answer is said in this thread, and so is what you post in #{name} while you \
                     answer. {attach} (with `--thread {root}` at any other time, and `--top` to say it in \
                     the channel itself).",
                    name = channel.name,
                    cli = brand::CLI,
                    attach = attaching(&channel.name),
                );
                (opening, shown)
            }
            None => (
                format!("You are in the channel #{}. {}.", channel.name, attaching(&channel.name)),
                studio
                    .channels
                    .messages(own, &channel_id, Some(&message_id), CONTEXT_MESSAGES)
                    .await
                    .unwrap_or_default(),
            ),
        };
        // What is attached to a message is said with the command that
        // reads it, as the agent may read the channel, since where it is on
        // this device means nothing behind a wall or on another; a
        // project's file posted as that file is said as one, with what
        // reads it as the agent may.
        let files = |attached: &[studio::Attachment]| -> String {
            attached
                .iter()
                .map(|one| match &one.file {
                    // A file of a project posted as that file: read as that project's file is.
                    Some(file) => format!(
                        "\n  attached: {} ({}), the file `{}` of `{}` (`{cli} file read {} {}`)",
                        one.name,
                        one.media,
                        file.path,
                        file.project,
                        file.project,
                        file.path,
                        cli = brand::CLI
                    ),
                    // A copy is read through this program, as whoever the agent is, wherever it runs.
                    None => format!(
                        "\n  attached: {} ({}), read with `{cli} attachment {} --out <file>`",
                        one.name,
                        one.media,
                        one.id,
                        cli = brand::CLI
                    ),
                })
                .collect()
        };
        if !history.is_empty() {
            prompt.push_str(" Earlier messages, oldest first:\n");
            for message in &history {
                let attached = files(&message.attachments);
                prompt.push_str(&format!("[{}] {}{attached}\n", message.author_name, message.body));
            }
        }
        let newest = studio.channels.message(own, &message_id).await.map(|message| message.attachments);
        prompt.push_str(&format!(
            "\nThe newest message, from {}, is for you:\n{}{}",
            text("author_name"),
            text("body"),
            files(&newest.unwrap_or_default())
        ));
        // A thread is a conversation of its own, and remembered as one.
        let conversation = match &thread {
            Some(root) => format!("thread:{root}"),
            None => format!("channel:{channel_id}"),
        };
        for name in targets {
            // Seeing a message a second time (after a restart) must not
            // start a second turn for it.
            if self.has_turn_for(name, &message_id).await {
                continue;
            }
            let request = TurnRequest {
                slot: name.to_string(),
                kind: "message".into(),
                thread: conversation.clone(),
                prompt: prompt.clone(),
                channel: Some(channel_id.clone()),
                reply_to: Some(message_id.clone()),
                // Where the channel takes plans, the agent says whether its answer is one.
                options: self.plans_in(&channel_id, name),
                labels: Labels::new(),
                cwd: None,
                hidden: Vec::new(),
                again: None,
                most: None,
                until: None,
                // Asked by a message, which is itself what says which asking it is.
                asking: None,
                asked_by: None,
            };
            // Said where the message was, when its agent will not take it
            // (it is leaving): whoever spoke to it is not left with silence.
            if let Err(refused) = self.enqueue(request).await {
                let place = studio::Place { thread: thread.as_deref(), ..studio::Place::default() };
                let note = format!("{name} was not asked: {}", refused.message);
                let _ = studio.channels.post_speaker(own, &channel_id, &note, place).await;
            }
        }
    }

    /// Forgets what agents were asked and what they answered in a channel
    /// that has been taken away: each such turn was kept with what had
    /// been said there. One that is in hand is left to end; what it says
    /// then has nowhere to go. Work given by a blueprint is not a
    /// channel's, and stays.
    ///
    /// Of one that was kept to its members, what each agent kept of it
    /// goes too, and the words of the work that stays (`crate::kept`).
    pub async fn forget_channel(&self, channel: &str) -> kernel::Result<usize> {
        self.forget_kept(channel).await?;
        let channel = channel.to_string();
        let asked = move |connection: &mut rusqlite::Connection| {
            connection.execute(
                "DELETE FROM turns WHERE channel = ?1 AND kind = 'message' \
                 AND status NOT IN ('running', 'answered')",
                [&channel],
            )
        };
        let forgotten: usize = self.of_each(self.every_turns(), asked).await?.into_iter().sum();
        // What each agent was given is counted from the turns there are.
        self.count_given().await?;
        Ok(forgotten)
    }
}
