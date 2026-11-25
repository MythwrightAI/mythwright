//! The tickets a document lists, asked as actions: `tickets.read` and
//! `tickets.filed`.
//!
//! What each works out is a function of text and plain values and is the
//! kernel's ([`kernel::tickets`]), offered here under the names it always
//! had; this is how a blueprint asks it.
//!
//! It was the studio's (`studio::tickets`), and came here with nothing of a
//! table in it: a fleet's blueprints ask it as they ask the rest here.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use kernel::BoxFut;
use serde_json::{Value, json};

pub use kernel::tickets::*;

pub(crate) fn register(registry: &mut Registry) {
    let spec = ActionSpec::new("tickets.read", "Read the tickets a document lists in its block of them.")
        .arg("content", T::String, "the document; one that is not there lists none")
        .output(
            "came",
            &format!(
                "{}: it has no block; they were read; or its block is not a list of tickets",
                Came::words()
            ),
        )
        .output("tickets", "the tickets, in the order they are listed")
        .output("why", "why they could not be read, where they could not")
        .idempotent();
    registry.add_fn(spec, |_cx, args| -> BoxFut<'static, conductor::ActionResult> {
        Box::pin(async move {
            let read = read(args.get("content").and_then(Value::as_str).unwrap_or_default());
            Ok(json!({ "came": read.came.word(), "tickets": read.tickets, "why": read.why }))
        })
    });

    let spec = ActionSpec::new(
        "tickets.filed",
        "Say what a document's tickets came to, once each is a thing with an id: what each waits on, and the \
         document with that written where its block was.",
    )
    .required("content", T::String, "the document they were read from")
    .required("tickets", T::List, "the tickets, as they were read")
    .required("made", T::List, "what each came to, in the same order, each with its `id`")
    .arg("in_place", T::String, "what the document says where its block was; `{ids}` in it is the ids")
    .output(
        "links",
        "for each ticket in turn, the `id` of what it came to and `after`, the ids of what it waits on",
    )
    .output("ids", "the ids, in the order of the tickets")
    .output("listed", "the ids as a sentence lists them")
    .output("content", "the document, saying `in_place` where its block was")
    .idempotent();
    registry.add_fn(spec, |_cx, args| -> BoxFut<'static, conductor::ActionResult> {
        Box::pin(async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default();
            let list = |key: &str| args.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
            let filed = filed(text("content"), &list("tickets"), &list("made"), text("in_place"))
                .map_err(ActionError::new)?;
            Ok(json!({
                "links": filed.links,
                "listed": filed.ids.join(", "),
                "ids": filed.ids,
                "content": filed.content,
            }))
        })
    });
}
