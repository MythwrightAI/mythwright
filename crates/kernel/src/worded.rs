//! A line of a record's history in words: which events a table counts
//! as the history of its records, how each is worded, and who it says did
//! what a blueprint's step did. Functions of plain values; what keeps the
//! lines and reads them is `studio::history`, which offers these under the
//! names it always had.
//!
//! What a blueprint's step does to a record is the factory's doing, and is
//! said as the factory's bot, by the name it has where the line is read
//! ([`as_called`]): the blueprint that did it is kept with the line
//! (`through`), as detail and never as who did it, and so is whose work
//! the step was told it did it from (`wrote`), said after the sentence.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event::topic_matches;
use crate::{Error, Event, Result};

/// The names an event may give the record it is about, in the order they
/// are looked for.
pub const NAMES: &[&str] = &["record", "card", "item", "id"];

/// Events that are part of a record's history, and how they are worded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// A topic, or a group of them: `board.item.moved`, `board.item.*`.
    pub topic: String,
    /// The sentence, with what the event says of itself in braces:
    /// `{actor} moved it from {from} to {to}`, and in square brackets a
    /// part said only when the event has what it names. None for the
    /// usual words.
    #[serde(default)]
    pub says: Option<String>,
}

/// Reads `history` as a blueprint writes it: a group by itself, a list of
/// groups, a map from an event to its sentence, or a list of either.
pub fn rules(written: &Value) -> Result<Vec<Rule>> {
    let mut read = Vec::new();
    let mut one = |topic: &str, says: &Value| -> Result<()> {
        let topic = topic.trim();
        if topic.is_empty() || topic.split('.').any(str::is_empty) {
            return Err(Error::invalid(format!(
                "`{topic}` is not an event; history is kept of topics such as `board.item.moved`, or groups of them such as `board.item.*`"
            )));
        }
        let says = match says {
            Value::Null => None,
            Value::String(sentence) if sentence.trim().is_empty() => None,
            Value::String(sentence) => Some(sentence.clone()),
            _ => {
                return Err(Error::invalid(format!(
                    "what is said of `{topic}` must be a sentence, such as \"{{actor}} moved it from {{from}} to {{to}}\", or left empty for the usual words"
                )));
            }
        };
        read.push(Rule { topic: topic.to_string(), says });
        Ok(())
    };
    let mut entry = |written: &Value| -> Result<()> {
        match written {
            Value::String(topic) => one(topic, &Value::Null),
            Value::Object(map) => map.iter().try_for_each(|(topic, says)| one(topic, says)),
            _ => Err(Error::invalid(
                "`history` is the events kept of a record: a list of topics, or a map from each to what is said of it",
            )),
        }
    };
    match written {
        Value::Null => {}
        Value::Array(entries) => entries.iter().try_for_each(&mut entry)?,
        other => entry(other)?,
    }
    Ok(read)
}

/// The rule that speaks for an event: the most exact of those that fit it.
pub fn rule_for<'a>(rules: &'a [Rule], topic: &str) -> Option<&'a Rule> {
    rules
        .iter()
        .filter(|rule| topic_matches(&rule.topic, topic))
        .max_by_key(|rule| (!rule.topic.contains('*'), rule.topic.len()))
}

/// Something an event says of itself, as it is read in a sentence.
fn shown(value: &Value) -> String {
    match value {
        Value::Null => "nothing".to_string(),
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(shown).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
    }
}

/// What is said of an event in a record's history.
pub fn said(rule: &Rule, event: &Event, who: &str) -> String {
    let Some(sentence) = &rule.says else {
        // The usual words: who, what happened, and what the event says of it;
        // which blueprint did it, and from whose work, are said after the line.
        let what = event.topic.rsplit('.').next().unwrap_or(&event.topic).replace('_', " ");
        let more: Vec<String> = event
            .data
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(name, value)| {
                !NAMES.contains(&name.as_str())
                    && !["project", "title", "by", "through", "wrote", "once", "doing", "asker"]
                        .contains(&name.as_str())
                    && !value.is_null()
                    && !value.is_object()
            })
            .take(4)
            .map(|(name, value)| format!("{name} {}", shown(value)))
            .collect();
        return if more.is_empty() {
            format!("{who}: {what}")
        } else {
            format!("{who}: {what} ({})", more.join(", "))
        };
    };
    let mut out = String::new();
    let mut rest = sentence.as_str();
    // A part in square brackets is said only when what it names is there.
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']') else { break };
        out.push_str(&filled(&rest[..open], event, who).0);
        let (part, all_there) = filled(&rest[open + 1..open + close], event, who);
        if all_there {
            out.push_str(&part);
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(&filled(rest, event, who).0);
    out
}

/// A piece of a sentence with what it names in braces filled in, and
/// whether the event had something to say for every name in it.
fn filled(piece: &str, event: &Event, who: &str) -> (String, bool) {
    let (mut out, mut all_there) = (String::new(), true);
    let mut rest = piece;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        let name = rest[open + 1..open + close].trim();
        match (name, event.data.get(name)) {
            ("actor", _) => out.push_str(who),
            (_, said) => {
                // An empty list says nothing, as an empty text does.
                all_there &= said.is_some_and(|value| {
                    !value.is_null()
                        && value.as_str() != Some("")
                        && value.as_array().is_none_or(|all| !all.is_empty())
                });
                out.push_str(&said.map(shown).unwrap_or_default());
            }
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    (out, all_there)
}

/// The blueprint a line says did what it is about, where a step of one
/// did: nothing for anybody else's doing.
pub fn through(data: &Value) -> Option<&str> {
    data.get("through").and_then(Value::as_str).filter(|blueprint| !blueprint.is_empty())
}

/// The agent whose decision what a blueprint's step did was (a rung of a
/// project's chain that looked at a card), by its name: such a line is
/// the agent's, with the blueprint said after it as detail. Nothing where
/// the step was told nobody's, or somebody's who is no agent.
pub fn decided(data: &Value) -> Option<&str> {
    let agent =
        data.get("decided").and_then(Value::as_str).and_then(|decided| decided.strip_prefix("agent:"));
    agent.filter(|name| !name.is_empty())
}

/// What is said after a line's sentence of whose work a blueprint's step
/// was told it did it from: ` from what shop-architect-1 wrote`, and
/// nothing where it was told nobody's, or somebody's who is no agent.
pub fn from_what(data: &Value) -> String {
    let agent = data.get("wrote").and_then(Value::as_str).and_then(|wrote| wrote.strip_prefix("agent:"));
    match agent.filter(|name| !name.is_empty()) {
        Some(name) => format!(" from what {name} wrote"),
        None => String::new(),
    }
}

/// Who a line says did what it is about, and what it says, as they are
/// read now: of what a blueprint's step did, the factory's bot by the name
/// `bot` it has now, in place of the name the line was written with (the
/// bot as it was called then, or the blueprint, as a line from before this
/// said), with whose work it was done from after the sentence; and where
/// the step did what an agent decided, that agent. Anybody else's line is
/// as it was written.
pub fn as_called(who: &str, said: &str, data: &Value, bot: &str) -> (String, String) {
    if through(data).is_none() {
        return (who.to_string(), said.to_string());
    }
    if let Some(agent) = decided(data) {
        let said = match who {
            "" => said.to_string(),
            written => said.replacen(written, agent, 1),
        };
        return (agent.to_string(), said);
    }
    let mut said = match who {
        "" => said.to_string(),
        written => said.replacen(written, bot, 1),
    };
    let after = from_what(data);
    if !after.is_empty() && !said.contains(after.trim_start()) {
        said.push_str(&after);
    }
    (bot.to_string(), said)
}
