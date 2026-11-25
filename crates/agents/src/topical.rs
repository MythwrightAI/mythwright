//! An agent asked in a topic: named in a reply, by somebody
//! who may reply there, or talked with in the chain of replies it answered
//! in. It is given the topic as its context (title, where it stands, where
//! it is in its wiki and the pages under it by title, the summary, the body
//! and the chain it answers, `topics::context`), and
//! never a channel's history, and what it answers is said as a reply
//! under the one that asked it.
//!
//! A turn of a topic is a turn as any message's is (`kind: message`), with
//! no channel: its conversation is the chain it is asked in
//! (`topic:<topic>:<the chain's top>`, remembered as one conversation for
//! each chain), and what it answers is the reply it was asked by
//! (`<topic>#<the number it was said at>`), which is what tells one asking
//! from the next however often the reply is heard. Topics are open to
//! whoever reads records for now: who may view one, and so who of
//! those named is asked, is still to be built.

use blueprint::Labels;
use kernel::topics::{Place, TOPICS};
use kernel::{Event, Principal, PrincipalKind};
use serde_json::Value;
use studio::models::Topic;
use topics::context::{About, May, Said, prompt};

use crate::together::Answer;
use crate::{Agents, Slot, TurnInfo, TurnRequest};

/// The topic a turn was asked in, and the reply it answers: from what it
/// answers (`<topic>#<number>`).
pub(crate) fn topic_of(turn: &TurnInfo) -> Option<(String, u64)> {
    if !turn.thread.starts_with("topic:") || turn.channel.is_some() {
        return None;
    }
    let (topic, seq) = turn.reply_to.as_deref()?.rsplit_once('#')?;
    Some((topic.to_string(), seq.parse().ok()?))
}

/// Whether an event is something said in a topic.
fn of_a_topic(event: &Event) -> bool {
    event.data.get("project").and_then(Value::as_str) == Some(kernel::place::FACTORY)
        && event.data.get("collection").and_then(Value::as_str) == Some(TOPICS)
}

impl Agents {
    /// Decides which agents, if any, something said in a topic is for,
    /// and queues a turn for each.
    pub(crate) async fn topical(&self, event: &Event) {
        if !of_a_topic(event) {
            return;
        }
        let text = |key: &str| event.data.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
        let (id, body, author) = (text("id"), text("body"), text("by"));
        let collections = &self.inner.studio.collections;
        let Ok((_, record)) = collections.record(&id).await else { return };
        // Its replies as the topic's page reads them: those the keeper of
        // histories has written, and those said a moment ago that it has not
        // come to yet, which may be the chain this one answers.
        let keeper = topics::Topics::new(self.inner.journal.clone(), self.inner.studio.clone(), None);
        let read = keeper.read(&id, &Principal::system()).await.unwrap_or_default();
        let mut replies: Vec<(u64, Option<u64>, String, String)> = read["replies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|reply| {
                let seq = reply["seq"].as_u64().filter(|seq| *seq != event.seq)?;
                let text = |key: &str| reply[key].as_str().unwrap_or_default().to_string();
                Some((seq, reply["parent"].as_u64(), text("who"), text("body")))
            })
            .collect();
        let parent = event.data.get("parent").and_then(Value::as_u64);
        replies.push((event.seq, parent, author.clone(), body.clone()));
        let tree: Vec<(u64, Option<u64>)> = replies.iter().map(|(seq, parent, ..)| (*seq, *parent)).collect();
        let chain = kernel::topics::chain(&tree, event.seq);
        let Ok(slots) = self.slots().await else { return };
        let up = |slot: &&Slot| slot.active && !slot.leaving && !slot.external();
        let named = studio::channels::mentions(&body);
        let mut targets: Vec<&Slot> =
            slots.iter().filter(up).filter(|slot| named.contains(&slot.name)).collect();
        // Named by nobody, a person is talking to the agent that spoke last
        // in the chain, as in a thread of a channel.
        if targets.is_empty() && author.starts_with("user:") {
            let spoke = chain.iter().rev().skip(1).find_map(|seq| {
                let by = &replies.iter().find(|reply| reply.0 == *seq)?.2;
                slots.iter().filter(up).find(|slot| *by == slot.name || *by == format!("agent:{}", slot.name))
            });
            targets.extend(spoke);
        }
        // What an agent says asks no agent: in a channel agents that answer
        // one another are held to so many turns an hour, which nothing
        // holds in a topic yet, and two that named each other would go on.
        if author.starts_with("agent:") {
            return;
        }
        if targets.is_empty() {
            return;
        }
        let topic = Topic::read(&record.data);
        let place = match Place::of(&record.data) {
            Some(Place::Project(project)) => format!("the project {project}"),
            Some(Place::Workspace(workspace)) => format!("the workspace {workspace}"),
            None => "the factory".to_string(),
        };
        // Where it is in its wiki and the pages under it, by title.
        let (above, under) = keeper.whereabouts(&id).await.unwrap_or_default();
        let titles = |pages: Vec<Value>| -> Vec<String> {
            pages.iter().map(|page| page["title"].as_str().unwrap_or_default().to_string()).collect()
        };
        let about = About {
            id: id.clone(),
            title: topic.title,
            status: topic.status.filter(|status| !status.is_empty()),
            place,
            summary: topic.summary,
            body: topic.body,
            path: titles(above),
            pages: titles(under),
        };
        let said = |seq: &u64| {
            let (_, _, who, body) = replies.iter().find(|reply| reply.0 == *seq)?;
            Some(Said { who: who.clone(), body: body.clone() })
        };
        let earlier: Vec<Said> = chain.iter().filter(|seq| **seq != event.seq).filter_map(said).collect();
        let who = event.source.actor.clone().unwrap_or(author);
        let newest = Said { who, body };
        let top = chain.first().copied().unwrap_or(event.seq);
        let reply_to = format!("{id}#{}", event.seq);
        for slot in targets {
            // Heard a second time (after a restart), it starts no second turn.
            if self.has_turn_for(&slot.name, &reply_to).await {
                continue;
            }
            // Offered only what it may run: what it is refused it would
            // try, and fill its answer with having been refused.
            let principal = slot.principal();
            let may = May { read: principal.can("records:read"), write: principal.can("records:write") };
            let request = TurnRequest {
                slot: slot.name.clone(),
                kind: "message".into(),
                thread: format!("topic:{id}:{top}"),
                prompt: prompt(&about, &earlier, &newest, may),
                channel: None,
                reply_to: Some(reply_to.clone()),
                options: Vec::new(),
                labels: Labels::from([("topic".to_string(), id.clone())]),
                cwd: None,
                hidden: Vec::new(),
                again: None,
                most: None,
                until: None,
                asking: None,
                asked_by: None,
            };
            let _ = self.enqueue(request).await;
        }
    }

    /// Says a turn's answer in the topic it was asked in, as its agent,
    /// under the reply that asked it. The factory says it for the agent it
    /// started, on the leave of whoever asked it there, as an answer in a
    /// channel comes into the channel by being one: the agent is not asked
    /// whether it may change records, and is given nothing
    /// ([`studio::Records::answer`]). Where it could not answer, gave no
    /// answer, or its answer could not be said there, the factory says so
    /// there instead, so that whoever asked is never left with nothing.
    /// With `unless_said`, each is said only where it is not there already.
    pub(crate) async fn say_in_topic(&self, turn: &TurnInfo, answer: &Answer, unless_said: bool) {
        let Some((topic, parent)) = topic_of(turn) else { return };
        let began = self.inner.journal.now_ms();
        let name = &turn.slot;
        let note = match &answer.error {
            Some(message) => format!("{name} could not answer: {message}"),
            None if answer.reply.text.trim().is_empty() => format!("{name} gave no answer."),
            None => {
                let agent = match self.slot(name).await {
                    Ok(slot) => slot.principal(),
                    // Taken away since it answered: its answer is its all the same.
                    Err(_) => Principal {
                        kind: PrincipalKind::Agent,
                        id: name.clone(),
                        name: name.clone(),
                        scopes: Vec::new(),
                        key: None,
                    },
                };
                match self.say_once(&topic, parent, &agent, &answer.reply.text, unless_said).await {
                    Ok(()) => return self.measured(name, began, true),
                    Err(why) => format!(
                        "{name} answered, and its answer could not be said here: {}",
                        why.message.trim_end_matches('.')
                    ),
                }
            }
        };
        let said = self.say_once(&topic, parent, &Principal::system(), &note, unless_said).await;
        // Where not even that could be said (the topic is gone), it is
        // counted as failed, which is all that is left to do.
        self.measured(name, began, said.is_ok());
    }

    /// Says `words` in `topic` under `parent`, as `by`: an agent's answer
    /// as the factory says one, anything else as `by` may say it. With
    /// `unless_said`, nothing where `by` said the same there already.
    async fn say_once(
        &self,
        topic: &str,
        parent: u64,
        by: &Principal,
        words: &str,
        unless_said: bool,
    ) -> kernel::Result<()> {
        if unless_said {
            let keeper = topics::Topics::new(self.inner.journal.clone(), self.inner.studio.clone(), None);
            let read = keeper.read(topic, &Principal::system()).await.unwrap_or_default();
            // What is said is kept trimmed, so it is found so.
            let there = read["replies"].as_array().into_iter().flatten().any(|reply| {
                reply["by"].as_str() == Some(by.actor().as_str())
                    && reply["body"].as_str().map(str::trim) == Some(words.trim())
                    && reply["parent"].as_u64() == Some(parent)
            });
            if there {
                return Ok(());
            }
        }
        let records = &self.inner.studio.records;
        match by.kind {
            PrincipalKind::Agent => records.answer(topic, words, parent, by).await?,
            _ => records.reply(topic, words, Some(parent), by, None).await?,
        };
        Ok(())
    }

    /// Counts a turn's answer said in a topic, or what was said instead,
    /// and whether that could be said.
    fn measured(&self, slot: &str, began: i64, ok: bool) {
        let took = (self.inner.journal.now_ms() - began) as f64;
        self.inner.journal.measure("agent.topic.answer", &[("slot", slot)], took, ok);
    }
}
