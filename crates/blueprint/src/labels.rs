//! Labels: what a blueprint says about the work it starts, so that one lot
//! of measurements can be told from another.
//!
//! The same work is often done two ways at once to see which is better: one
//! card given to two agents, two versions of a deployment, two ways of
//! handing work out. Nothing measured could tell the two apart unless the
//! blueprint that started them said which was which, and `labels:` is where
//! it says so: on a `call`, a `spawn` or a `task`, a name and a value for
//! everything measured underneath.
//!
//! A label sits in a measurement beside what the measurement says of itself
//! (its step, its blueprint, its project), and is asked for in the same way,
//! so it may not have one of those names.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::diag::Diagnostic;
use crate::model::StepKind;
use crate::syntax::Entry;
use crate::template::{self, Template, each_template};

/// The names a measurement uses for what it says of itself, somewhere in
/// the product. A label with one of these would be read as that.
pub const TAKEN: &[&str] = &[
    "action",
    "adapter",
    "backend",
    "blueprint",
    "caller",
    "collection",
    "db",
    "decision",
    "device",
    "extension",
    "flow",
    "handler",
    "kind",
    "method",
    "name",
    "op",
    "project",
    "route",
    "router",
    "service",
    "slot",
    "status",
    "step",
    "topic",
];

/// The first words of the names the system measures under, beside the
/// families of its events (`run`, `step`, `service`, ...). A measurement a
/// program or a blueprint says of its own may not begin with one of these,
/// or it would be counted with the system's.
pub const MEASURED: &[&str] = &["db", "http", "store", "snapshot", "blueprint", "library", "link", "build"];

/// Why a program or a blueprint may not say a measurement called `name`,
/// if it may not: it is not a name, or it is one the system measures
/// under.
pub fn measurement(name: &str) -> Option<String> {
    if !crate::validate::is_topic(name) {
        return Some(format!(
            "`{name}` is not a name for a measurement; use lowercase words joined by dots, such as `checkout.done`"
        ));
    }
    let first = name.split('.').next().unwrap_or_default();
    (crate::validate::RESERVED_TOPICS.contains(&first) || MEASURED.contains(&first)).then(|| {
        format!(
            "`{name}` is a name the system measures under; call your own something of your own, such as \
             `checkout.done`"
        )
    })
}

/// What a measurement's values may be of where they are not a length of
/// time: each is how its name ends. `agent.cost_usd` is an amount of
/// money, `agent.input_tokens` so many tokens, `device.free_mem_mb` so
/// many megabytes.
pub const UNITS: &[&str] = &["usd", "tokens", "bytes", "mb", "cpu", "count"];

/// What the values of the measurement `name` are of, where its name says:
/// its last word is one of [`UNITS`] or ends in one after a `_`. A
/// measurement that says nothing is a length of time, in milliseconds,
/// which is what the system measures of nearly everything. This is the
/// one place that says so: what shows a measurement adds its values up
/// and writes them as what they are by asking here, and keeps no list of
/// names of its own.
pub fn unit(name: &str) -> Option<&'static str> {
    let last = name.rsplit('.').next().unwrap_or_default();
    let ending = last.rsplit('_').next().unwrap_or_default();
    UNITS.iter().copied().find(|unit| *unit == ending)
}

/// The longest a label's value may be. A value is part of what every
/// measurement under it is kept by, so it is a word or a few, not a text.
pub const MAX_VALUE: usize = 100;

/// Labels with their values worked out: what is carried.
pub type Labels = BTreeMap<String, String>;

/// Why `name` may not be a label's name, if it may not.
pub fn refused(name: &str) -> Option<String> {
    let mut letters = name.chars();
    let word = letters.next().is_some_and(|first| first.is_ascii_lowercase())
        && letters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !word {
        return Some(format!(
            "`{name}` cannot be a label's name: a label is a lowercase word, of letters, digits and `_`, \
             starting with a letter, such as `variant`"
        ));
    }
    TAKEN.contains(&name).then(|| {
        format!(
            "`{name}` cannot be a label: measurements already say that of themselves, and asked for \
             `{name}` could not say which was meant. Taken are: {}. Call it something else, such as \
             `variant` or `experiment`",
            TAKEN.join(", ")
        )
    })
}

/// A label's value as it is carried: text as it is, a number as it is
/// written. Anything else is not something measurements can be told apart by.
pub fn value(name: &str, value: &Value) -> Result<String, String> {
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        other => {
            let what = match other {
                Value::Null => "nothing",
                Value::Bool(_) => "true or false",
                Value::Array(_) => "a list",
                _ => "a mapping",
            };
            return Err(format!("the label `{name}` must be text or a number, and came out as {what}"));
        }
    };
    if text.chars().count() > MAX_VALUE {
        return Err(format!(
            "the label `{name}` is {} characters long; a label is a word or a few, at most {MAX_VALUE} \
             characters, since every measurement under it is kept by it",
            text.chars().count()
        ));
    }
    Ok(text)
}

/// Reads a step's `labels:`. `kind` is what the step does: only a step that
/// starts something has anything underneath it to label.
pub(crate) fn read(entry: &Entry, kind: &StepKind) -> (Map<String, Value>, Vec<Diagnostic>) {
    let mut problems = Vec::new();
    if matches!(kind, StepKind::Do(_)) {
        problems.push(Diagnostic::error(
            "conflict",
            "`labels` says what everything started under a step is measured as, and a step that is one \
             `do` starts nothing under it; put `labels` on the `call`, `spawn` or `task` around it",
            entry.key_span,
        ));
        return (Map::new(), problems);
    }
    let Value::Object(labels) = entry.value.to_json() else {
        problems.push(Diagnostic::error(
            "wrong-type",
            format!(
                "`labels` must be a mapping from a label's name to its value, such as \
                 `{{ experiment: cart, variant: a }}`; found {}",
                entry.value.kind()
            ),
            entry.value.span,
        ));
        return (Map::new(), problems);
    };
    for field in entry.value.as_map().unwrap_or_default() {
        if let Some(why) = refused(&field.key) {
            problems.push(Diagnostic::error("bad-name", why, field.key_span));
        }
        let written = field.value.to_json();
        let mut holes = false;
        each_template(&written, &mut |text| {
            holes = true;
            if let Err(err) = Template::parse(text) {
                problems.push(Diagnostic::error("bad-expression", err.message, field.value.span));
            }
        });
        // What is written out is known now; what text with a `${ }` in it
        // comes to is checked when the step starts.
        let later = holes && written.is_string();
        if !later && let Err(why) = value(&field.key, &written) {
            problems.push(Diagnostic::error("wrong-type", why, field.value.span));
        }
    }
    (labels, problems)
}

/// Works out a step's `labels:` in `scope` and adds them to `around`, the
/// labels the step is itself under. A label said here wins over one of the
/// same name from further out; one that comes out empty says nothing, and
/// leaves what was there.
pub fn under(
    around: &Labels,
    labels: &Map<String, Value>,
    scope: &Map<String, Value>,
) -> Result<Labels, String> {
    let mut carried = around.clone();
    for (name, written) in labels {
        let text = value(name, &template::render(written, scope)?)?;
        if !text.is_empty() {
            carried.insert(name.clone(), text);
        }
    }
    Ok(carried)
}

/// Labels given from outside a blueprint, to whatever applies it: each
/// name and value is held to what `labels:` is held to.
pub fn given(labels: &Map<String, Value>) -> Result<Labels, String> {
    let mut carried = Labels::new();
    for (name, written) in labels {
        if let Some(why) = refused(name) {
            return Err(why);
        }
        let text = value(name, written)?;
        if !text.is_empty() {
            carried.insert(name.clone(), text);
        }
    }
    Ok(carried)
}

/// What a measurement says of itself and, beside it, the labels it is
/// under, as they are handed to whatever keeps measurements. What it says
/// of itself is never replaced: a label that got this far with such a name
/// is left out.
pub fn beside<'a>(own: &[(&'a str, &'a str)], carried: &'a Labels) -> Vec<(&'a str, &'a str)> {
    let mut all = own.to_vec();
    for (name, value) in carried {
        if !own.iter().any(|(taken, _)| taken == name) {
            all.push((name.as_str(), value.as_str()));
        }
    }
    all
}
