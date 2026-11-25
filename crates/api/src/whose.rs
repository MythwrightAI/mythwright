//! Held in a build made to be tested: an event of something that is a
//! project's says which project.
//!
//! What hears an event with nothing said hears its own project's and what
//! is nobody's (`conductor::whose`), and whose an event is, is read from
//! the event: the project its data names, or that of the instance that
//! said it. An event of an agent's or of a channel's names no instance,
//! so it has to say its project itself; one that did not was nobody's,
//! and so heard in every project.
//!
//! Which project a slot or a channel is in is learnt here from what is
//! said as each is made or changed, and every event said of it after
//! that is held to saying the same. What a table's or a board's events
//! say is held already, by their descriptions: `project`, always.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;

/// What the events said so far have told of slots and channels.
#[derive(Default)]
pub(crate) struct Whose {
    /// The project each slot is in, by its name; none for one in no project.
    slots: Mutex<HashMap<String, Option<String>>>,
    /// The project each channel belongs to, by its id.
    channels: Mutex<HashMap<String, Option<String>>>,
}

/// The agents' events that are said of a slot.
const OF_A_SLOT: &[&str] = &[
    "agent.turn.queued",
    "agent.turn.started",
    "agent.turn.finished",
    "agent.output",
    "agent.slot.leaving",
    "agent.slot.stopped",
];

/// The channels' events that are said of a channel that is there.
const OF_A_CHANNEL: &[&str] = &["channel.joined", "channel.left", "channel.message", "channel.renamed"];

impl Whose {
    /// What is wrong with an event as it is said, if anything: it is of
    /// something known to be a project's and does not say so.
    pub(crate) fn fault(&self, topic: &str, data: &Value) -> Option<String> {
        let said = data.get("project").and_then(Value::as_str).filter(|project| !project.is_empty());
        let (known, key, what) = match topic {
            // Where a slot or a channel is made or put somewhere, the
            // event says where: that is what is learnt.
            "agent.slot.created" | "agent.slot.updated" => {
                if let (Some(slot), Some(_)) = (data.get("slot").and_then(Value::as_str), data.get("project"))
                {
                    self.slots.lock().expect("whose lock").insert(slot.to_string(), said.map(str::to_string));
                }
                return None;
            }
            "channel.created" | "channel.placed" => {
                if let Some(channel) = data.get("channel").and_then(Value::as_str) {
                    self.channels
                        .lock()
                        .expect("whose lock")
                        .insert(channel.to_string(), said.map(str::to_string));
                }
                return None;
            }
            topic if OF_A_SLOT.contains(&topic) => (&self.slots, "slot", "the agent"),
            topic if OF_A_CHANNEL.contains(&topic) => (&self.channels, "channel", "the channel"),
            _ => return None,
        };
        let of = data.get(key).and_then(Value::as_str)?;
        let project = known.lock().expect("whose lock").get(of).cloned().flatten()?;
        (said != Some(project.as_str())).then(|| {
            format!(
                "`{topic}` is said of {what} `{of}`, which is in the project `{project}`, and says {}: whatever \
                 hears events by project takes it for {}",
                said.map_or("no `project`".to_string(), |said| format!("`project: {said}`")),
                said.map_or("nobody's, and so for every project's to hear", |_| "another project's"),
            )
        })
    }
}
