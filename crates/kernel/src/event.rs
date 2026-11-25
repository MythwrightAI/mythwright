//! The one event shape.
//!
//! Everything that happens is one of these: a step finishing, a message
//! posted, a service going unhealthy, a card moving on the board. Hooks and
//! triggers react to them, the client is drawn from them, and every metric is
//! a query over them. There is deliberately no second schema.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Position in this device's journal. Strictly increasing; `0` for an
    /// event that was broadcast but not kept (see [`Event::is_durable`]).
    pub seq: u64,
    /// Milliseconds since the Unix epoch.
    pub ts: i64,
    /// Dotted lowercase words, e.g. `step.finished`.
    pub topic: String,
    #[serde(default, skip_serializing_if = "Source::is_empty")]
    pub source: Source,
    #[serde(default)]
    pub data: Value,
}

/// Where an event came from. Every field is optional because events come from
/// many places; a trigger's `scope` and the client's filters match on these.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    /// Who caused it: `user:<id>`, `agent:<slot>`, `device:<name>` or `system`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    /// What hearing it takes, besides what its topic does: a scope. Said
    /// by whoever says an event of something that is not everybody's to
    /// read (a record of a table that is kept private), and held to
    /// wherever an event is handed to somebody.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub takes: Option<String>,
}

impl Source {
    pub fn is_empty(&self) -> bool {
        *self == Source::default()
    }

    pub fn actor(actor: impl Into<String>) -> Self {
        Source { actor: Some(actor.into()), ..Source::default() }
    }

    pub fn instance(instance: impl Into<String>) -> Self {
        Source { instance: Some(instance.into()), ..Source::default() }
    }

    /// The same, of something that hearing of takes `scope`; nothing said
    /// leaves it as it was.
    pub fn taking(self, scope: Option<&str>) -> Self {
        match scope {
            Some(scope) => Source { takes: Some(scope.to_string()), ..self },
            None => self,
        }
    }
}

impl Event {
    /// Whether the event was written to the journal. Ephemeral events (log
    /// lines, progress ticks) are broadcast to whoever is listening and then
    /// forgotten, so a chatty build cannot grow the journal.
    pub fn is_durable(&self) -> bool {
        self.seq != 0
    }
}

/// Whether `topic` matches `pattern`, where `*` is exactly one word and `**`
/// is any number of words (including none).
pub fn topic_matches(pattern: &str, topic: &str) -> bool {
    fn matches(pattern: &[&str], topic: &[&str]) -> bool {
        match pattern.split_first() {
            None => topic.is_empty(),
            Some((&"**", rest)) => (0..=topic.len()).any(|skip| matches(rest, &topic[skip..])),
            Some((word, rest)) => match topic.split_first() {
                Some((first, remaining)) => (*word == "*" || word == first) && matches(rest, remaining),
                None => false,
            },
        }
    }
    let pattern: Vec<&str> = pattern.split('.').collect();
    let topic: Vec<&str> = topic.split('.').collect();
    matches(&pattern, &topic)
}

/// Whether `topic` matches any of a comma-separated list of patterns.
pub fn topic_matches_any(patterns: &str, topic: &str) -> bool {
    patterns.split(',').map(str::trim).filter(|p| !p.is_empty()).any(|pattern| topic_matches(pattern, topic))
}
