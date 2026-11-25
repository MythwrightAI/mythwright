//! What becomes of a message said in a channel that offers its agents
//! discussions (`talk::discussed`, `kernel::discussed`), as whom it is for
//! is worked out (`crate::heard`): one that begins a discussion, or is
//! said in a discussion's thread, is the discussion's and gives no agent a
//! turn of its own; a person's that names no agent, where the channel has
//! a lead, is the lead's alone. What an agent asked there is told of the
//! others, and a turn that may end in a plan is offered the word.
//!
//! And what holds agents back that answer each other where no discussion
//! does: so many turns an hour that messages of agents, and apart from
//! them of bots, may start in one channel, kept across a restart, after
//! which they wait for a person, who is told in their inbox.

use kernel::discussed::Takes;
use kernel::{Event, Principal};
use serde_json::{Value, json};

use crate::Agents;

/// Turns one channel's agents may start by their own messages within an
/// hour, and its bots by theirs, each counted apart.
const MOST_AN_HOUR: usize = 30;
const HOUR: i64 = 3_600_000;

/// What a message comes to where its channel offers discussions.
pub(crate) enum Handed {
    /// Nothing: whom it is for is worked out as anywhere.
    Not,
    /// The lead's alone.
    Lead(String),
    /// The discussion's: no agent is given a turn for it.
    Taken,
}

impl Agents {
    /// What `event`, said in `channel`, comes to where it offers
    /// discussions; `agents` are the channel's agents, by name.
    pub(crate) async fn handed(&self, channel: &studio::Channel, event: &Event, agents: &[&str]) -> Handed {
        let channels = &self.inner.studio.channels;
        let Some(settings) = channels.discussing(&channel.id) else { return Handed::Not };
        let text = |key: &str| event.data[key].as_str().filter(|said| !said.is_empty()).map(str::to_string);
        let own = &studio::Channels::daemon("who a message said in a discussion's channel answers");
        let reply_to = text("reply_to");
        let answers = match &reply_to {
            Some(asked) => channels.message(own, asked).await.ok().map(|asked| asked.author),
            None => None,
        };
        let names = event.data["mentions"].as_array().into_iter().flatten().filter_map(Value::as_str);
        let message = kernel::discussed::Message {
            id: text("id").unwrap_or_default(),
            author: text("author").unwrap_or_default(),
            names: names.filter(|name| agents.contains(name)).map(str::to_string).collect(),
            mark: text("mark").unwrap_or_default(),
            thread: text("thread"),
            reply_to,
            answers: answers.filter(|author| author.starts_with("user:")),
        };
        match kernel::discussed::takes(&settings, &message, |root| channels.discussion_thread(root)) {
            Takes::Not => Handed::Not,
            Takes::Lead(lead) => Handed::Lead(lead),
            took => {
                if let Err(why) = channels.discuss(channel, &message.id, &took).await {
                    let note = format!("The discussion could not be begun: {}", why.message);
                    let place =
                        studio::Place { thread: message.thread.as_deref(), ..studio::Place::default() };
                    let _ = channels.post_speaker(own, &channel.id, &note, place).await;
                }
                Handed::Taken
            }
        }
    }

    /// The words a turn of `slot`'s for a message in `channel` may end
    /// with: `answer` or `plan`, where the channel takes plans and it
    /// takes part; none otherwise.
    pub(crate) fn plans_in(&self, channel: &str, slot: &str) -> Vec<String> {
        let Some(settings) = self.inner.studio.channels.discussing(channel) else { return Vec::new() };
        match settings.on.iter().any(|on| on == "plans") && settings.takes_part(slot) {
            true => vec!["answer".into(), "plan".into()],
            false => Vec::new(),
        }
    }

    /// What `slot` is told of the others it may talk things over with in
    /// `channel`; nothing where it offers no discussions.
    pub(crate) fn discussing_told(&self, channel: Option<&str>, slot: &str) -> String {
        let offered = channel.and_then(|channel| self.inner.studio.channels.discussing(channel));
        match offered.map(|settings| kernel::discussed::brief(&settings, slot)) {
            Some(said) if !said.is_empty() => format!("\n\n{said}"),
            _ => String::new(),
        }
    }

    /// Whether a message by `author` in `channel` may start turns: a
    /// person's always, and lets agents and bots that were waiting go on;
    /// an agent's or a bot's while that channel's agents, or its bots,
    /// have started fewer than so many in the last hour. The one that
    /// would be the next after that is answered by nobody: it is said in
    /// the channel, once, and the person last talking there is told.
    pub(crate) async fn may_start(&self, channel: &studio::Channel, author: &str) -> bool {
        let journal = &self.inner.journal;
        let kind = match author.split_once(':').map(|(kind, _)| kind) {
            Some("user") => {
                for kind in ["agent", "bot"] {
                    let _ = journal
                        .kv_set("agents", &format!("started/{kind}/{}", channel.id), &Value::Null)
                        .await;
                }
                return true;
            }
            Some("bot") | Some("system") => "bot",
            _ => "agent",
        };
        let key = format!("started/{kind}/{}", channel.id);
        let now = journal.now_ms();
        let kept = journal.kv_get("agents", &key).await.ok().flatten().unwrap_or(Value::Null);
        let mut at: Vec<i64> =
            kept["at"].as_array().into_iter().flatten().filter_map(Value::as_i64).collect();
        at.retain(|then| now - then < HOUR);
        let noted = kept["noted"].as_i64().filter(|then| now - then < HOUR);
        if at.len() < MOST_AN_HOUR {
            at.push(now);
            let _ = journal.kv_set("agents", &key, &json!({ "at": at, "noted": noted })).await;
            return true;
        }
        if noted.is_none() {
            let _ = journal.kv_set("agents", &key, &json!({ "at": at, "noted": now })).await;
            self.waits_for_a_person(channel, kind).await;
        }
        false
    }

    /// Says in `channel`, and tells the person last talking there, that
    /// its agents (or its bots) wait for a person.
    async fn waits_for_a_person(&self, channel: &studio::Channel, kind: &str) {
        let channels = &self.inner.studio.channels;
        let own =
            &studio::Channels::daemon("who was last talking in a channel whose agents wait for a person");
        let who = if kind == "bot" { "Bots" } else { "Agents" };
        let note = format!(
            "{who} in this channel have been answering each other a great deal; they will wait for a person \
             before continuing."
        );
        let Ok(said) = channels.post_speaker(own, &channel.id, &note, studio::Place::default()).await else {
            return;
        };
        let recent = channels.messages(own, &channel.id, None, 50).await.unwrap_or_default();
        let Some(person) =
            recent.iter().rev().map(|one| &one.author).find(|author| author.starts_with("user:"))
        else {
            return;
        };
        let told = studio::telling::Telling {
            to: vec![person.clone()],
            says: format!("{note} Say something in #{} to let them go on.", channel.name),
            why: "waiting".into(),
            about: channel.id.clone(),
            place: channel.id.clone(),
            said_by: format!("answering {} {kind}", channel.id),
            parts: vec![json!({ "kind": "message", "channel": channel.id, "message": said.id })],
        };
        let _ = channels.tell(&Principal::system(), told).await;
    }

    /// What an agent asked `prompt` in the thread that hangs from `root`
    /// in `channel` is handed: the thread as it was said, oldest first,
    /// and then what it is asked.
    pub(crate) async fn thread_shown(&self, channel: &str, root: &str, prompt: String) -> String {
        let own = &studio::Channels::daemon("what an agent asked in a thread is shown of it");
        let said = self.inner.studio.channels.thread(own, root).await.unwrap_or_default();
        let shown: String = said.iter().map(|one| format!("[{}] {}\n", one.author_name, one.body)).collect();
        let name =
            self.inner.studio.channels.get(own, channel).await.map(|found| found.name).unwrap_or_default();
        format!(
            "You are in a thread in the channel #{name}. What was said in it, oldest first:\n{shown}\n{prompt}"
        )
    }
}
