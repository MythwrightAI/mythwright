//! What was said on a record, read from its history for whoever is to be
//! told it: an agent given a card again is told what others said on it
//! since its last turn (`record.said`). Which lines those are is worked
//! out of the lines by `kernel::parts::discussed`, with its cases.

use blueprint::{ActionSpec, ValueType as T};
use conductor::Registry;
use kernel::parts::{Line, discussed, said_as_text, worked};
use serde_json::{Value, json};

use crate::Studio;
use crate::actions::{add, text};
use crate::history;

/// A record's history as `kernel::parts` goes by it.
pub fn lines(history: &[history::Line]) -> Vec<Line> {
    let each = history.iter().map(|line| {
        Line::of(&line.topic, line.actor.as_deref().unwrap_or_default(), &line.who, line.ts, &line.data)
    });
    each.collect()
}

pub(crate) fn register(registry: &mut Registry, studio: &Studio) {
    add(
        registry,
        studio,
        ActionSpec::new(
            "record.said",
            "Read what was said on a record since a line of its history, as whoever is to be told it reads it.",
        )
        .example(kernel::asked::RECORD_SAID)
        .deciding()
        .required("id", T::String, "the record")
        .arg(
            "since",
            T::List,
            "topics of its history: what was said after the last line of one of them is read, and what the agent \
             that line names (its `agent` or `asker`) said is left out. Where none is there, everything said",
        )
        .arg("leaving", T::String, "words its reader is told already, left out where something said is just that")
        .capability("records:read")
        .output("said", "what was said, oldest first: each with `who`, `actor` and `words`")
        .output("text", "the same as lines, each `- who: words`; empty where nothing was said")
        .output("count", "how many things were said")
        .output("worked", "the agents that worked it, by name, whatever was asked since: none judges its own work")
        .idempotent(),
        |studio, cx, args| async move {
            let id = text(&args, "id").unwrap_or_default();
            // Read as anything of the record is, by whoever asks.
            studio.records.get(&id, cx.principal()).await?;
            let since: Vec<String> = match args.get("since") {
                Some(Value::Array(topics)) => topics.iter().filter_map(Value::as_str).map(str::to_string).collect(),
                _ => Vec::new(),
            };
            let lines = lines(&studio.collections.history(&id).await?);
            let said = discussed(&lines, &since, &text(&args, "leaving").unwrap_or_default());
            let each: Vec<Value> =
                said.iter().map(|line| json!({ "who": line.who, "actor": line.actor, "words": line.words })).collect();
            let worked: Vec<String> =
                worked(&lines).iter().map(|actor| actor.trim_start_matches("agent:").to_string()).collect();
            Ok(json!({ "said": each, "text": said_as_text(&said), "count": said.len(), "worked": worked }))
        },
    );
}
