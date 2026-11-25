//! What of a blueprint's text is done, or settled, when it is applied.
//!
//! An instance runs its steps once, when it is applied, with the params it
//! was given; after that it does what its handlers say, and what it says
//! it does when it is stopped. A text that differs from the one an
//! instance runs only in that later part can take its place where the
//! instance is, with nothing run again: the steps it would run are the
//! ones that ran. One that differs in the first part cannot, because what
//! was done was done by other steps, or with other params.
//!
//! [`outset`] is that first part, written out so that two texts can be
//! compared by it. It is read as a blueprint and not as characters:
//! comments, spacing, how a value is quoted and the spaces just inside a
//! `${ }` do not count ([`crate::format::meaning`]), and neither does the
//! order of the fields at the top. Left out of it are the fields
//! that say what happens later (`on`, `on_stop`, `on_update`, `one_at_a_time`), what the text may use
//! (`permissions`, which is held to whoever applies it), and what it says
//! of itself (`version`, and a `description` of the blueprint, of a param
//! or of a step). Everything else is in it: the params, the steps and
//! their order, and whatever else is said at the top.

use crate::format::write;
use crate::syntax::{self, Entry, Node, Value};

/// Fields at the top that are not part of how a blueprint begins.
const NOT_THE_OUTSET: &[&str] =
    &["on", "on_stop", "on_update", "one_at_a_time", "permissions", "version", "description"];

/// What `source` does and settles when it is applied, in a form two texts
/// can be compared by. `None` for a text that does not read as a mapping.
pub fn outset(source: &str) -> Option<String> {
    let root = syntax::parse(source).ok()?;
    let mut entries: Vec<&Entry> =
        root.as_map()?.iter().filter(|entry| !NOT_THE_OUTSET.contains(&entry.key.as_str())).collect();
    entries.sort_by(|one, other| one.key.cmp(&other.key));
    let mut out = String::new();
    for entry in entries {
        out.push_str(&entry.key);
        out.push(':');
        match entry.key.as_str() {
            "steps" => steps(&entry.value, &mut out),
            "params" => params(&entry.value, &mut out),
            _ => write(&entry.value, &mut out),
        }
        out.push('\n');
    }
    Some(out)
}

/// What `source` calls its own version, where it says.
pub fn version(source: &str) -> Option<String> {
    let root = syntax::parse(source).ok()?;
    let said = &root.as_map()?.iter().find(|entry| entry.key == "version")?.value;
    match &said.value {
        Value::Str(text) => Some(text.clone()),
        Value::Int(number) => Some(number.to_string()),
        Value::Float(number) => Some(number.to_string()),
        _ => None,
    }
}

/// A mapping of steps: each without its description, and a task's steps
/// the same way.
fn steps(node: &Node, out: &mut String) {
    named(
        node,
        out,
        |field, out| {
            if field.key == "task" { steps(&field.value, out) } else { write(&field.value, out) }
        },
    );
}

/// A mapping of params: each without its description.
fn params(node: &Node, out: &mut String) {
    named(node, out, |field, out| write(&field.value, out));
}

/// A mapping from names to things, each of which may say a `description`
/// of itself, which is left out; `field` writes one of its other fields.
fn named(node: &Node, out: &mut String, field: impl Fn(&Entry, &mut String)) {
    let Some(entries) = node.as_map() else { return write(node, out) };
    out.push('{');
    for entry in entries {
        out.push_str(&format!("{:?}:", entry.key));
        match entry.value.as_map() {
            Some(fields) => {
                out.push('{');
                for one in fields.iter().filter(|one| one.key != "description") {
                    out.push_str(&format!("{:?}:", one.key));
                    field(one, out);
                    out.push(',');
                }
                out.push('}');
            }
            None => write(&entry.value, out),
        }
        out.push(',');
    }
    out.push('}');
}
