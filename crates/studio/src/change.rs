//! What a change to a record was, in words.
//!
//! Something that follows a table's rows and could not take a change says
//! so to people, and has to say which change: the new row, the change to
//! the row or its being taken away, which row, of which table, and which
//! of its cells. That is one sentence's worth of deciding, and is
//! [`said`]: a function of the event that said the change and what the
//! row is called, which a blueprint asks as `record.change`. Why the
//! change was not taken, and what follows, is the blueprint's to say
//! around it.

use blueprint::{ActionSpec, ValueType as T};
use conductor::Registry;
use kernel::ErrorKind;
use serde_json::{Value, json};

use crate::Studio;
use crate::actions::{add, text};

blueprint::came! {
    /// Which change to a record an event said.
    pub enum Came {
        /// A record was added.
        Added => "added",
        /// A record was taken away.
        Removed => "removed",
        /// Something a record holds was changed.
        Changed => "changed",
    }
}

/// A change to a record, in words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    pub came: Came,
    /// `the change to the row `worker` of `agents` (`count`)`.
    pub says: String,
}

/// What the event `topic` said of the record called `title` in the table
/// `collection`, `changed` being the cells it changed where it says any.
/// A topic that is neither a record's adding nor its taking away is a
/// change to it.
pub fn said(topic: &str, title: &str, collection: &str, changed: &[String]) -> Said {
    let (came, which) = match topic {
        "record.created" => (Came::Added, "the new row"),
        "record.removed" => (Came::Removed, "the taking away of the row"),
        _ => (Came::Changed, "the change to the row"),
    };
    let cells = match changed.is_empty() {
        true => String::new(),
        false => format!(" (`{}`)", changed.join("`, `")),
    };
    Said { came, says: format!("{which} `{title}` of `{collection}`{cells}") }
}

pub(crate) fn register(registry: &mut Registry, studio: &Studio) {
    add(
        registry,
        studio,
        ActionSpec::new(
            "record.change",
            "Say what a change to a record was, in words: the new row, the change to the row or its taking away, \
             which row, of which table, and which of its cells.",
        )
        .required("topic", T::String, "the event that said the change: `record.created`, `record.removed`, or any other for a change to what the record holds")
        .required("id", T::String, "the record")
        .required("collection", T::String, "the table it is a record of")
        .arg("changed", T::List, "the cells that were changed, where the event says them")
        .capability("records:read")
        .output("came", &format!("{}: which of the three it was", Came::words()))
        .output("says", "the words: `the change to the row `worker` of `agents` (`count`)`")
        .output("title", "what the record is called: its `title`, or its id where it has none, is gone, or is not whoever asks's to read")
        .idempotent(),
        |studio, cx, args| async move {
            let id = text(&args, "id").unwrap_or_default();
            let topic = text(&args, "topic").unwrap_or_default();
            // One that was taken away is not there to be asked what it is
            // called, and one that cannot be read is said by its id: this
            // is for saying what went wrong, and does not go wrong itself.
            let title = match topic.as_str() {
                "record.removed" => None,
                _ => match studio.records.get(&id, cx.principal()).await {
                    Ok(record) => record.data.get("title").and_then(Value::as_str).map(str::to_string),
                    Err(gone) if matches!(gone.kind, ErrorKind::NotFound | ErrorKind::Denied) => None,
                    Err(other) => return Err(other.into()),
                },
            };
            let title = title.filter(|title| !title.is_empty()).unwrap_or(id);
            let changed: Vec<String> = args
                .get("changed")
                .and_then(Value::as_array)
                .map(|cells| cells.iter().map(blueprint::expr::display).collect())
                .unwrap_or_default();
            let said = said(&topic, &title, &text(&args, "collection").unwrap_or_default(), &changed);
            Ok(json!({ "came": said.came.word(), "says": said.says, "title": title }))
        },
    );
}
