//! What is written of a turn and what is said of it, as one write.
//!
//! A turn is queued, begun and ended, and each is a row written and an
//! event said (`agent.turn.queued`, `.started`, `.finished`). Each pair
//! is one transaction of the journal's ([`journal::Journal::write`]): a
//! daemon stopped at any instant leaves both or neither, so no turn is
//! written as queued, in hand or ended with nobody told, and nothing has
//! to be looked for when a daemon starts. The event says whose the turn
//! is as its row does ([`crate::apart`]), read in that write.
//!
//! One thing cannot be one write. Where an agent's answer is said in a
//! channel, that is the channel's own write, and it comes before the
//! turn is said to have ended: whatever hears that a turn ended finds
//! its answer there. So the answer is kept first, in a write that says
//! nothing, with the turn still in hand (`answered`: its program is
//! done, and its answer is on its way to being said); then it is said
//! where it was asked; then the turn is written as ended and said to
//! be, in one write. A daemon stopped between those finds the turn
//! answered when it starts, and its agent's worker goes on from there:
//! the agent is not asked again, the answer is said where it was asked
//! unless it is found there already (its words, by it, in answer to the
//! same thing, said since the turn was answered), and the turn is ended
//! and said to be, once. A turn with nothing to say in a channel is never `answered`:
//! it is in hand and then ended. One asked in a topic always has something
//! to say there: its answer, or the factory's word that it gave none.

use blueprint::labels::beside;
use journal::{Writing, optional};
use kernel::{Result, Source};
use rusqlite::params;
use serde_json::{Value, json};

use crate::turns::{MOST_RESTARTS, TurnInfo, TurnRequest};
use crate::{Agents, Reply, Slot};

/// How a turn came out, as its row keeps it.
pub(crate) struct Answer {
    pub reply: Reply,
    pub error: Option<String>,
    /// For work given a choice to end on, and for a consult: what was chosen.
    pub chosen: Option<String>,
    pub finished: i64,
}

impl Answer {
    fn status(&self) -> &'static str {
        if self.error.is_none() { "done" } else { "failed" }
    }

    /// Writes it on the row of `turn`, as `status`, where the turn is
    /// still in hand: how many rows that was.
    fn keep(&self, connection: &rusqlite::Connection, turn: &str, status: &str) -> rusqlite::Result<usize> {
        connection.execute(
            "UPDATE turns SET status = ?2, reply = ?3, error = ?4, decision = ?5, cost_usd = ?6,
                    input_tokens = ?7, output_tokens = ?8, finished = ?9, refused = ?10, cost_said = ?11
             WHERE id = ?1 AND status IN ('running', 'answered')",
            params![
                turn,
                status,
                self.reply.text,
                self.error,
                self.chosen,
                self.reply.cost_usd.unwrap_or(0.0),
                self.reply.input_tokens as i64,
                self.reply.output_tokens as i64,
                self.finished,
                (!self.reply.refused.is_empty()).then(|| json!(self.reply.refused).to_string()),
                i64::from(self.reply.cost_usd.is_some()),
            ],
        )
    }
}

/// Says `topic` of the turn `id` in the write under way, as its agent,
/// with whose the turn is as its row says it there: nothing of that for
/// one that is nobody's, or whose row is gone. `kept` is the channel
/// kept to its members that the turn is of, where it is of one: what is
/// said of it is then said to that channel's members (`crate::kept`).
fn say(
    writing: &mut Writing<'_>,
    (id, slot, kept): (&str, &str, Option<&str>),
    topic: &str,
    mut data: Value,
) -> rusqlite::Result<()> {
    let reads = crate::apart::TURNS.reads();
    let whose: Option<Option<String>> = optional(writing.transaction().query_row(
        &format!("SELECT {reads} FROM turns kept WHERE kept.id = ?1"),
        [id],
        |row| row.get(0),
    ))?;
    if let Some(project) = whose.flatten() {
        data["project"] = json!(project);
    }
    let takes = kept.map(studio::Channels::kept_takes);
    writing.say(topic, Source::actor(format!("agent:{slot}")).taking(takes.as_deref()), data)?;
    Ok(())
}

impl Agents {
    /// Writes a turn down as queued, in the journal that keeps the turns
    /// of its agent's project, and says so. Which turn it is of all
    /// there ever were (`crate::filed`).
    pub(crate) async fn queue(
        &self,
        id: &str,
        request: TurnRequest,
        slot: &Slot,
        kept: Option<String>,
    ) -> Result<i64> {
        let (id, now, outlives) = (id.to_string(), self.inner.journal.now_ms(), slot.outlives);
        let journal = self.turns_of(slot.project.as_deref());
        // A project's own journal holds no list of agents to read whose
        // the turn is from: it is that project's, which is why it is
        // kept there. In the factory's own it is its slot's project as
        // the rows say it in this write (`crate::apart`).
        let own = slot.project.clone().filter(|_| journal.name() != self.inner.journal.name());
        let (agents, name, queued) = (self.clone(), slot.name.clone(), id.clone());
        let wrote = journal.write(move |writing| {
            let nth = agents.inner.filed.next();
            writing.transaction().execute(
                &format!(
                    "INSERT INTO turns
                         (id, slot, kind, thread, prompt, channel, reply_to, options, status, created, labels,
                          cwd, outlives, again, most, until, asking, asked_by, nth, project, kept, hidden)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'queued', ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                             ?17, ?18, coalesce(?19, {}), ?20, ?21)",
                    crate::apart::of_slot(2)
                ),
                params![
                    id,
                    request.slot,
                    request.kind,
                    request.thread,
                    request.prompt,
                    request.channel,
                    request.reply_to,
                    serde_json::to_string(&request.options).unwrap_or_default(),
                    now,
                    serde_json::to_string(&request.labels).unwrap_or_default(),
                    request.cwd,
                    outlives,
                    request.again.unwrap_or(true),
                    request.most.unwrap_or(MOST_RESTARTS),
                    request.until,
                    request.asking,
                    request.asked_by,
                    nth,
                    own,
                    kept,
                    (!request.hidden.is_empty())
                        .then(|| serde_json::to_string(&request.hidden).unwrap_or_default()),
                ],
            )?;
            let data = json!({ "slot": request.slot, "turn": id });
            say(writing, (&id, &request.slot, kept.as_deref()), "agent.turn.queued", data)?;
            Ok(nth)
        });
        let (nth, _) = wrote.await?;
        self.queued_in(&name, &queued, &journal);
        Ok(nth)
    }

    /// Writes a turn down as in hand, begun at `started`, and says that
    /// it began, with `about` it. Nothing is said of one whose row is
    /// gone.
    pub(crate) async fn begin(&self, turn: &TurnInfo, started: i64, about: Value) {
        let (id, slot, restarts, kept) =
            (turn.id.clone(), turn.slot.clone(), turn.restarts, turn.kept.clone());
        // In hand from just before the write that says so: its agent's
        // key opens the kept channel it is of, where it is of one, by
        // the time anybody hears that it began. That is said in the
        // write, and heard as soon as the write is kept, which may be
        // before whoever made it goes on. Its program is not started
        // until this is done, and nothing else holds its key.
        let taking = self.inner.hand.taking(&turn.id, &turn.slot, turn.kept.as_deref());
        let wrote = self.keeps(turn).write(move |writing| {
            let begun = writing.transaction().execute(
                "UPDATE turns SET status = 'running', started = ?2, restarts = ?3 WHERE id = ?1",
                params![id, started, restarts],
            )?;
            if begun == 1 {
                say(writing, (&id, &slot, kept.as_deref()), "agent.turn.started", about)?;
            }
            Ok(begun == 1)
        });
        // Not begun after all (its row is gone, the write was not made,
        // or this was let go of before it was answered): it is in hand
        // no longer, which is what letting go of `taking` does.
        if wrote.await.is_ok_and(|(begun, _)| begun) {
            taking.begun();
        }
    }

    /// Keeps a turn's answer while it is still in hand, saying nothing:
    /// for one whose answer is to be said in a channel before the turn
    /// is said to have ended.
    pub(crate) async fn answered(&self, turn: &TurnInfo, answer: &Answer) {
        let (id, answer) = (turn.id.clone(), answer.kept());
        let wrote = self.keeps(turn).db().call(move |connection| answer.keep(connection, &id, "answered"));
        let _ = wrote.await;
    }

    /// Writes a turn down as ended, as `answer` says, and says that it
    /// finished, with `data`: one write, and nothing where the turn is
    /// no longer in hand. Whether the write was kept.
    pub(crate) async fn end(&self, turn: &TurnInfo, answer: &Answer, data: Value) -> bool {
        let (id, slot, answer, kept) = (turn.id.clone(), turn.slot.clone(), answer.kept(), turn.kept.clone());
        // In hand no longer, before its row says so: its agent's key
        // opens the kept channel it was of no more.
        self.inner.hand.gives_back(&turn.id);
        let wrote = self.keeps(turn).write(move |writing| {
            if answer.keep(writing.transaction(), &id, answer.status())? == 1 {
                say(writing, (&id, &slot, kept.as_deref()), "agent.turn.finished", data)?;
            }
            Ok(())
        });
        wrote.await.is_ok()
    }

    /// Ends turns that were never begun, saying why, and says of each
    /// that it finished, called off: one write for all of them in each
    /// file that keeps any. Those still waiting among `which`: one turn,
    /// or all a slot has. A turn that never began is over all the same
    /// and says so, or whatever shows an agent's turns would go on
    /// showing it as waiting. Which were ended.
    pub(crate) async fn call_off_unbegun(&self, which: Unbegun, why: &str) -> Vec<String> {
        let (why, now) = (why.to_string(), self.inner.journal.now_ms());
        type Waiting = (String, String, String, Option<String>, Option<String>);
        let (by, key, kept_in) = match &which {
            Unbegun::Turn(id) => ("id", id.clone(), self.may_keep(id)),
            Unbegun::Of(slot) => ("slot", slot.clone(), self.may_hold(slot)),
        };
        let mut ended = Vec::new();
        for journal in kept_in {
            let (key, why) = (key.clone(), why.clone());
            let wrote = journal.write(move |writing| {
                let waiting: Vec<Waiting> = {
                    let mut statement = writing.transaction().prepare(&format!(
                        "SELECT id, slot, kind, channel, kept FROM turns WHERE {by} = ?1 AND status = 'queued'"
                    ))?;
                    let rows = statement.query_map([&key], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
                    })?;
                    rows.collect::<rusqlite::Result<_>>()?
                };
                writing.transaction().execute(
                    &format!(
                        "UPDATE turns SET status = 'failed', error = ?2, finished = ?3
                         WHERE {by} = ?1 AND status = 'queued'"
                    ),
                    params![key, why, now],
                )?;
                let mut ended = Vec::new();
                for (id, slot, kind, channel, kept) in waiting {
                    let data = json!({
                        "slot": slot, "turn": id, "kind": kind, "channel": channel,
                        "ok": false, "error": why, "duration_ms": 0, "called_off": true,
                    });
                    say(writing, (&id, &slot, kept.as_deref()), "agent.turn.finished", data)?;
                    ended.push(id);
                }
                Ok(ended)
            });
            ended.extend(wrote.await.map(|(ended, _)| ended).unwrap_or_default());
        }
        for id in &ended {
            self.ended_in(id);
        }
        ended
    }

    /// Says a turn's answer where it was asked, as its agent: into the
    /// thread it was asked in, when it was asked in one; and where it
    /// could not answer something said to it, says so as the factory's
    /// bot. With `unless_said`, for a turn found answered by a daemon
    /// that starts, only where it is not there already.
    pub(crate) async fn say_answer(&self, turn: &TurnInfo, answer: &Answer, unless_said: bool) {
        // Asked in a topic: said there, under the reply that asked it.
        if crate::topical::topic_of(turn).is_some() {
            return self.say_in_topic(turn, answer, unless_said).await;
        }
        let Some(channel) = &turn.channel else { return };
        let channels = &self.inner.studio.channels;
        // The daemon's own: where a turn was asked is read to say its
        // answer there, and what the agent says it says as itself.
        let own = &studio::Channels::daemon(
            "where a turn was asked, to say its answer there, and the factory saying that an agent could not \
             answer",
        );
        let asked = match &turn.reply_to {
            Some(asked) => channels.message(own, asked).await.ok(),
            None => None,
        };
        // Answering a message, where that is said; asked to go on with a
        // thread of the channel and answering nothing, under that thread.
        let thread = match (&turn.reply_to, asked) {
            (Some(_), asked) => asked.and_then(|asked| asked.thread),
            (None, _) => turn.thread.strip_prefix("thread:").map(str::to_string),
        };
        match (&answer.error, self.slot(&turn.slot).await) {
            (None, Ok(slot)) if !answer.reply.text.trim().is_empty() => {
                // Its own words, by it, in answer to what this turn answers:
                // an agent has one turn for a message.
                let (author, words) = (format!("agent:{}", slot.name), &answer.reply.text);
                let its = |said: &studio::Message| {
                    said.author == author && said.body == *words && said.reply_to == turn.reply_to
                };
                if unless_said && self.was_said(channel, thread.as_deref(), answer.finished, its).await {
                    return;
                }
                // Which model wrote it: what the program said it used,
                // else what the slot runs on as far as its provider
                // has said, else not known.
                if let Some(said) = &answer.reply.model {
                    self.note_model(&slot.name, said).await;
                }
                let named = self.model_name(&slot);
                let model = answer.reply.model.as_deref().or(named.as_deref());
                // An answer the agent said is a plan is marked so, where it was offered the word.
                let mark =
                    answer.chosen.as_deref().filter(|chosen| kernel::discussed::MARKS.contains(chosen));
                // Its answer to what it was asked there: in the channel from it where it was not, so
                // that no answer is lost.
                let place = studio::Place {
                    reply_to: turn.reply_to.as_deref(),
                    thread: thread.as_deref(),
                    model,
                    mark,
                    answers: true,
                    ..studio::Place::default()
                };
                // Said by the factory for it, on the leave of whoever asked
                // it there: its own scopes are not asked, as in a topic.
                let Err(why) =
                    channels.post_answer(channel, &slot.principal(), &answer.reply.text, place).await
                else {
                    return;
                };
                // Not said, it is said why, by the factory's bot, so that
                // whoever asked is not left with nothing.
                let note = format!(
                    "{} answered, and its answer could not be said here: {}",
                    slot.name,
                    why.message.trim_end_matches('.')
                );
                let its = |said: &studio::Message| said.body == note;
                if unless_said && self.was_said(channel, thread.as_deref(), answer.finished, its).await {
                    return;
                }
                let place = studio::Place { thread: thread.as_deref(), ..studio::Place::default() };
                let _ = channels.post_speaker(own, channel, &note, place).await;
            }
            // Work given by a blueprint fails as that blueprint's step,
            // and what is said of that is the blueprint's to say; said
            // here as well it would be said twice.
            (Some(message), _) if turn.kind != "task" => {
                let note = format!("{} could not answer: {message}", turn.slot);
                let its = |said: &studio::Message| said.body == note;
                if unless_said && self.was_said(channel, thread.as_deref(), answer.finished, its).await {
                    return;
                }
                let place = studio::Place { thread: thread.as_deref(), ..studio::Place::default() };
                // Said of the agent, by the factory's bot: the agent
                // itself said nothing.
                let _ = channels.post_speaker(own, channel, &note, place).await;
            }
            _ => {}
        }
    }

    /// Whether something that `is` this is there already, in the channel
    /// or under the thread, said since the turn was answered.
    async fn was_said(
        &self,
        channel: &str,
        thread: Option<&str>,
        since: i64,
        is: impl Fn(&studio::Message) -> bool,
    ) -> bool {
        let channels = &self.inner.studio.channels;
        let own = &studio::Channels::daemon(
            "whether an answer found kept by a daemon that starts was said already, so that it is said once",
        );
        let there = match thread {
            Some(thread) => channels.thread(own, thread).await,
            None => channels.messages(own, channel, None, 500).await,
        };
        there.is_ok_and(|there| there.iter().any(|message| message.created >= since && is(message)))
    }

    /// Whether a turn that came out as `answer` has something to say in
    /// a channel before it is said to have ended.
    /// In a topic that is always so: one that gave no answer is said there
    /// by the factory to have given none.
    pub(crate) fn says_in_a_channel(turn: &TurnInfo, answer: &Answer) -> bool {
        let topic = crate::topical::topic_of(turn).is_some();
        (turn.channel.is_some() || topic)
            && match &answer.error {
                None => topic || !answer.reply.text.trim().is_empty(),
                Some(_) => turn.kind != "task",
            }
    }
}

/// Which turns that never began are called off.
pub(crate) enum Unbegun {
    /// This one, if it is still waiting.
    Turn(String),
    /// All that this slot has waiting.
    Of(String),
}

impl Answer {
    /// The same, for the write that keeps it.
    fn kept(&self) -> Answer {
        Answer {
            reply: self.reply.clone(),
            error: self.error.clone(),
            chosen: self.chosen.clone(),
            finished: self.finished,
        }
    }

    /// What a turn found answered by a daemon that starts came out as,
    /// read from its row.
    pub(crate) fn of(turn: &TurnInfo, now: i64) -> Answer {
        Answer {
            reply: Reply {
                text: turn.reply.clone(),
                cost_usd: turn.cost_said.then_some(turn.cost_usd),
                input_tokens: turn.input_tokens,
                output_tokens: turn.output_tokens,
                refused: turn.refused.clone(),
                ..Reply::default()
            },
            error: turn.error.clone(),
            chosen: turn.decision.clone(),
            finished: turn.finished.unwrap_or(now),
        }
    }
}

impl Agents {
    /// Brings a turn to its end, as `answer` says: its answer said where
    /// it was asked, what its agent changed noticed, what it took and
    /// cost measured, and the turn written as ended and said to be.
    /// `found` is for one a daemon found answered as it started, which
    /// goes on from there: its answer is said only where it is not
    /// there, and nothing is measured a second time.
    pub(crate) async fn conclude(
        &self,
        turn: &TurnInfo,
        answer: Answer,
        started: i64,
        called_off: bool,
        found: bool,
    ) {
        let journal = &self.inner.journal;
        if Agents::says_in_a_channel(turn, &answer) {
            if !found {
                self.answered(turn, &answer).await;
            }
            self.say_answer(turn, &answer, found).await;
        }
        let slot = self.slot(&turn.slot).await.ok();
        // An agent edits files directly; this is when to notice.
        if let Some(project) = slot.as_ref().and_then(|slot| slot.project.as_ref()) {
            let _ = self.inner.studio.projects.scan(project).await;
        }
        let (ok, reply) = (answer.error.is_none(), &answer.reply);
        if !found {
            let (adapter, project) = match &slot {
                Some(slot) => (slot.adapter.clone(), slot.project.clone().unwrap_or_default()),
                None => Default::default(),
            };
            // The project comes among the first three, so that what a turn cost
            // is put down to it as well.
            let mut own = vec![
                ("slot", turn.slot.as_str()),
                ("adapter", adapter.as_str()),
                ("project", project.as_str()),
                ("kind", turn.kind.as_str()),
            ];
            // What it chose, where it was given something to choose between:
            // how often each agent approves or sends back is counted from this,
            // by whoever wants it counted, in whatever its options were.
            if let Some(chosen) = &answer.chosen {
                own.push(("decision", chosen.as_str()));
            }
            // And the labels of the step it was taken for go on both: which of
            // two agents did the same work sooner, and for less. A turn
            // somebody called off is counted, and not among those that
            // failed: a reviewer still looking when enough of the others
            // had approved was counted as having failed at its card.
            let took = (answer.finished - started) as f64;
            journal.measure("agent.turn", &beside(&own, &turn.labels), took, ok || called_off);
            // What it cost and what it read and wrote, where its program says:
            // each is what its name ends in, and is added up by whoever asks.
            let spent = [
                ("agent.cost_usd", reply.cost_usd.unwrap_or(0.0)),
                ("agent.input_tokens", reply.input_tokens as f64),
                ("agent.output_tokens", reply.output_tokens as f64),
            ];
            for (name, value) in spent {
                if value > 0.0 {
                    journal.measure(name, &beside(&own[..3], &turn.labels), value, true);
                }
            }
        }
        let mut data = json!({
            "slot": turn.slot, "turn": turn.id, "kind": turn.kind, "channel": turn.channel, "ok": ok,
            "error": answer.error, "duration_ms": answer.finished - started,
            "input_tokens": reply.input_tokens, "output_tokens": reply.output_tokens, "decision": answer.chosen,
        });
        // What it cost only where its program said: left out, it was not reported, and never nought.
        if let Some(cost) = reply.cost_usd {
            data["cost_usd"] = json!(cost);
        }
        if !reply.refused.is_empty() {
            data["refused"] = json!(reply.refused);
        }
        if called_off && !ok {
            data["called_off"] = json!(true);
        }
        // What its program left for a later daemon to find is taken away
        // once the turn is kept as ended, and never before: a daemon that
        // went after reading the answer and before keeping the end left
        // the next with a turn in hand and nothing to take up, and its
        // agent was asked again.
        if self.end(turn, &answer, data).await {
            hosting::kept::clean(&self.inner.hosting.kept(&format!("turn.{}", turn.id)));
        }
        self.left_behind(turn).await;
        self.ended_in(&turn.id);
        self.notify(&turn.id);
    }

    /// Goes on with a turn that a daemon found answered as it started:
    /// an earlier one kept its answer and stopped before the turn was
    /// said to have ended.
    pub(crate) async fn end_answered(&self, turn: TurnInfo) {
        let answer = Answer::of(&turn, self.inner.journal.now_ms());
        let started = turn.started.unwrap_or(answer.finished);
        self.conclude(&turn, answer, started, false, true).await;
    }
}
