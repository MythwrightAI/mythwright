//! What a program says of its own in what it prints.
//!
//! A service is a program in any language, and the one thing every program
//! can do is print a line. So that is how one tells the factory something
//! it alone knows: a line that begins `::measure` or `::event`, with a
//! mapping written as JSON after it.
//!
//! ```text
//! ::measure {"name": "checkout.done", "value": 12.5, "ok": true, "labels": {"page": "cart"}}
//! ::event {"topic": "shop.paid", "order": "o-17"}
//! ```
//!
//! A measurement is kept as any other is, under the name given, with which
//! service said it and with the labels the step that started the service
//! carries: two versions started as `variant: a` and `variant: b` are told
//! apart by that, whatever each says of itself. An event is said for the
//! instance that owns the service, and a handler hears it as any other.
//! Neither may be the system's own: a name it measures under, a topic it
//! publishes under.
//!
//! Every other line is output and nothing more, and these are output too.

use blueprint::labels::{self, Labels};
use serde_json::{Map, Value};

/// What a line says, where it says something.
pub(crate) enum Said {
    Measure {
        name: String,
        value: f64,
        ok: bool,
        labels: Labels,
    },
    Event {
        topic: String,
        data: Value,
    },
    /// It began as one of these and is not one: why, for whoever reads the
    /// service's output.
    Wrong(String),
}

/// What `line` says to the factory, if it is addressed to it at all.
pub(crate) fn said(line: &str) -> Option<Said> {
    let (word, rest) = line.strip_prefix("::")?.split_once(' ')?;
    let read = |what: &str| match serde_json::from_str::<Value>(rest.trim()) {
        Ok(Value::Object(mapping)) => Ok(mapping),
        _ => Err(format!("what follows `::{what}` is a mapping written as JSON")),
    };
    Some(match word {
        "measure" => read(word).and_then(measure).unwrap_or_else(Said::Wrong),
        "event" => read(word).and_then(event).unwrap_or_else(Said::Wrong),
        _ => return None,
    })
}

fn measure(given: Map<String, Value>) -> Result<Said, String> {
    let name = given.get("name").and_then(Value::as_str).ok_or("a measurement has a `name`")?;
    if let Some(why) = labels::measurement(name) {
        return Err(why);
    }
    let own = match given.get("labels") {
        None | Some(Value::Null) => Labels::new(),
        Some(Value::Object(own)) => labels::given(own)?,
        Some(_) => return Err("`labels` is a mapping from a label's name to its value".into()),
    };
    Ok(Said::Measure {
        name: name.to_string(),
        value: given.get("value").and_then(Value::as_f64).unwrap_or(0.0),
        ok: given.get("ok").and_then(Value::as_bool).unwrap_or(true),
        labels: own,
    })
}

fn event(mut given: Map<String, Value>) -> Result<Said, String> {
    let Some(Value::String(topic)) = given.remove("topic") else {
        return Err("an event has a `topic`".into());
    };
    if !blueprint::validate::is_topic(&topic) {
        return Err(format!("`{topic}` is not a topic; use lowercase words joined by dots"));
    }
    if blueprint::RESERVED_TOPICS.contains(&topic.split('.').next().unwrap_or_default()) {
        return Err(format!("`{topic}` is in a namespace only the system may publish under"));
    }
    Ok(Said::Event { topic, data: Value::Object(given) })
}
