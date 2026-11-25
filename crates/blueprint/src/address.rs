//! Who an event is for.
//!
//! A handler says what is addressed to it with `for:`: fields of an event's
//! data, and the value each must have. What that buys over the same test in
//! `when:` is that the conductor, not the handler, decides who an event is
//! for: it keeps the handlers that are waiting by what they wait for, and an
//! event wakes only those it is for, however many others wait on its topic.
//!
//! That is only possible if the values are known before any event arrives,
//! so they are worked out once, when the handler starts to listen, and may
//! read only what cannot change while it lives.

use serde_json::{Map, Value};

use crate::diag::Diagnostic;
use crate::syntax::Entry;
use crate::template::{self, Template, each_template};

/// Reads a handler's `for:`. `source` is the field that says what fires the
/// handler: only an event can be addressed to anyone.
pub(crate) fn read(entry: &Entry, source: &str) -> (Map<String, Value>, Vec<Diagnostic>) {
    let mut problems = Vec::new();
    if source != "event" {
        problems.push(Diagnostic::error(
            "conflict",
            format!(
                "`for` says which events are addressed to a handler, and a handler with `{source}` is \
                 not fired by an event; take `for` away, and put any condition in `when`"
            ),
            entry.key_span,
        ));
        return (Map::new(), problems);
    }
    let Value::Object(address) = entry.value.to_json() else {
        problems.push(Diagnostic::error(
            "wrong-type",
            format!(
                "`for` must be a mapping from a field of the event's data to the value it must have, \
                 such as `{{ id: \"${{params.item}}\" }}`; found {}",
                entry.value.kind()
            ),
            entry.value.span,
        ));
        return (Map::new(), problems);
    };
    if address.is_empty() {
        problems.push(Diagnostic::error(
            "missing-field",
            "`for` names no field, so nothing is addressed by it; say which field of the event's data \
             must have which value, or take `for` away",
            entry.value.span,
        ));
    }
    for field in entry.value.as_map().unwrap_or_default() {
        // The left side is a field of the event's data, and not a path
        // from the event: what `event.data.id` would mean in an expression
        // is written `id` here. A field inside one is named by its path
        // from the data (`fields.sprint`, what a record's table carries).
        if let Some((_, own)) = field.key.rsplit_once('.').filter(|_| from_the_event(&field.key)) {
            problems.push(Diagnostic::error(
                "bad-name",
                format!(
                    "`{}` is not a field of the event's data; `for` names such a field by its own \
                     name, as in `{own}: ...`",
                    field.key
                ),
                field.key_span,
            ));
        }
        each_template(&field.value.to_json(), &mut |text| {
            if let Err(err) = Template::parse(text) {
                problems.push(Diagnostic::error("bad-expression", err.message, field.value.span));
            }
        });
    }
    (address, problems)
}

/// Whether a name written on the left of `for:` is a path from the event
/// rather than from its data.
fn from_the_event(name: &str) -> bool {
    matches!(name.split('.').next(), Some("event" | "data" | "source" | "topic"))
}

/// What `data` holds at `field`: a field of it, or a field inside one by
/// its path (`fields.sprint`). Nothing where any part of the way is not
/// there.
pub fn at<'a>(data: &'a Value, field: &str) -> Option<&'a Value> {
    match data.get(field) {
        Some(found) => Some(found),
        None => field.split('.').try_fold(data, |inside, part| inside.get(part)),
    }
}

/// Why a value of `for:` may not read the name `root`, if it may not, as a
/// sentence that says what to do instead.
pub(crate) fn refused(root: &str) -> Option<&'static str> {
    Some(match root {
        "event" => {
            "a value in `for` cannot read `event`: it is worked out when the handler starts to listen, \
             before any event arrives. Name the event's field on the left, as in \
             `for: { id: \"${params.item}\" }`, and put any other condition on the event in `when`"
        }
        "state" => {
            "a value in `for` cannot read `state`: it is worked out once, when the handler starts to \
             listen, and `state` can change under the handler after that. Compare with `state` in `when`"
        }
        "steps" => {
            "a value in `for` cannot read `steps`: it is worked out when the handler starts to listen, \
             before any event arrives and outside any run. Keep what the step gave in `state` and \
             compare with that in `when`"
        }
        "self" => {
            "a value in `for` cannot read `self`: that is a step, in its own hooks, and `for` is worked \
             out before any event arrives, when no step is running. Use `params` or `instance` here, \
             and `when` for anything else"
        }
        "item" => {
            "a value in `for` cannot read `item`: that is one element of a `for_each`, and `for` is \
             worked out once, before any event arrives. Use `params` or `instance` here, and `when` \
             for anything else"
        }
        "index" => {
            "a value in `for` cannot read `index`: that is one turn of a `for_each`, and `for` is \
             worked out once, before any event arrives. Use `params` or `instance` here, and `when` \
             for anything else"
        }
        _ => return None,
    })
}

/// A handler's `for:` with its values worked out: what an event's data must
/// carry for the event to be the handler's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Address {
    /// Each field, with the [`key`] of the value it must have.
    wanted: Vec<(String, String)>,
}

impl Address {
    /// Works out the values of `address`, a handler's `for:`, in `scope`.
    pub fn of(address: &Map<String, Value>, scope: &Map<String, Value>) -> Result<Address, String> {
        let mut wanted = Vec::with_capacity(address.len());
        for (field, value) in address {
            wanted.push((field.clone(), key(&template::render(value, scope)?)));
        }
        Ok(Address { wanted })
    }

    /// One field and the key of its value, for whoever keeps handlers by
    /// what they wait for: every entry must hold, so any one of them finds
    /// the handler. `None` when everything on the topic is the handler's.
    pub fn first(&self) -> Option<(&str, &str)> {
        self.wanted.first().map(|(field, value)| (field.as_str(), value.as_str()))
    }

    /// Whether an event with this data is for the handler. A field the
    /// event does not have matches nothing, not even `null`.
    pub fn carried_by(&self, data: &Value) -> bool {
        self.wanted.iter().all(|(field, value)| at(data, field).is_some_and(|found| key(found) == *value))
    }
}

/// A text two values share exactly when `==` holds between them in an
/// expression, so that being addressed means what the same test in `when:`
/// would. `==` takes `1` and `1.0` for one number, and only where the two
/// sides are numbers themselves, not inside a list or a mapping.
pub fn key(value: &Value) -> String {
    match value.as_f64() {
        // Zero has two signs that are equal to each other.
        Some(number) => format!("{:?}", if number == 0.0 { 0.0 } else { number }),
        None => {
            let mut text = String::new();
            write_key(value, &mut text);
            text
        }
    }
}

/// JSON with the names of a mapping in one order, whatever order they were
/// given in: two mappings are equal whichever way round they were written.
fn write_key(value: &Value, text: &mut String) {
    match value {
        Value::Array(items) => {
            text.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    text.push(',');
                }
                write_key(item, text);
            }
            text.push(']');
        }
        Value::Object(map) => {
            let mut names: Vec<&String> = map.keys().collect();
            names.sort_unstable();
            text.push('{');
            for (index, name) in names.into_iter().enumerate() {
                if index > 0 {
                    text.push(',');
                }
                text.push_str(&Value::String(name.clone()).to_string());
                text.push(':');
                write_key(&map[name], text);
            }
            text.push('}');
        }
        other => text.push_str(&other.to_string()),
    }
}
