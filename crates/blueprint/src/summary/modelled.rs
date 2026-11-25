//! What a thing a blueprint makes is made of, said in words: a table's
//! fields, each by its name with its kind, what it may be or names,
//! whether it is needed, what giving it a value takes and what stands for
//! it where nothing is said; and how the thing stands among others (what
//! it sits beside, whether its things are written by hand).
//!
//! It is read by the words the action that makes the thing says it reads
//! ([`Modelled`]), so that nothing here knows an action or a table: a
//! model is what an action says its thing is made of, and an action that
//! says nothing of one has none.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::words::{joined, says_it, value};
use crate::catalog::Modelled;
use crate::template::has_holes;

/// Whether a value is words with a hole in them.
fn has_holes_in(value: &Value) -> bool {
    value.as_str().is_some_and(has_holes)
}

/// What a thing made is made of, as a page draws it and a terminal prints it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Each of its parts (a table's fields) by its name, in the order of their names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    /// Where its parts are not written out in the text: what says them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_said: Option<String>,
    /// What is said of the whole beside its parts, each a sentence: what it
    /// sits beside and what each of its rows is about there, and that
    /// its things are not written by hand.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub said: Vec<String>,
}

/// One part of what is made: a field of a table.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    /// Its kind in a word or a few: `number`, `choice of `a` or `b``,
    /// `record of `agents``; as written where the text works it out.
    pub kind: String,
    /// Where it names the things of another (`record`), which: by its name, where it is written out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub names: Option<String>,
    /// What else is said of it, each a sentence: that it is needed, what
    /// giving it a value takes, what stands for it where nothing is said,
    /// the words it is usually given.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub about: Vec<String>,
    /// The whole of it in a line, as a terminal prints it.
    pub text: String,
}

/// Words written out, each set apart, joined by `or`.
fn either(items: &[Value]) -> String {
    joined(&items.iter().map(value).collect::<Vec<_>>(), "or")
}

/// What giving a part a value takes, in a sentence: a scope, several, or
/// for some of its values each its own.
fn takes(takes: &Value, how: &Modelled) -> Option<String> {
    let scopes = |said: &Value| match said {
        Value::Array(all) => joined(&all.iter().map(value).collect::<Vec<_>>(), "and"),
        one => value(one),
    };
    Some(match takes {
        Value::Null => return None,
        Value::String(word) if word == how.takes.1 => format!("{}.", capital(how.takes.2)),
        Value::Object(each) => {
            let said: Vec<String> =
                each.iter().map(|(of, needs)| format!("giving it `{of}` takes {}", scopes(needs))).collect();
            format!("{}.", capital(&joined(&said, "and")))
        }
        other => format!("Giving it a value takes {}.", scopes(other)),
    })
}

fn capital(text: &str) -> String {
    let mut letters = text.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => String::new(),
    }
}

/// One part as it is written: a mapping of what it is, or a word for its kind alone.
fn field(name: &str, written: &Value, how: &Modelled) -> Field {
    let said = |key: &str| written.get(key).filter(|one| !one.is_null());
    let kind_word = match written {
        Value::String(_) => written.clone(),
        _ => said(how.kind.0).cloned().unwrap_or_else(|| Value::String(how.kind.1.to_string())),
    };
    let kind_said = match kind_word.as_str().filter(|_| says_it(&kind_word).is_none()) {
        Some(word) => word.to_string(),
        // What the text works out is said as it is written.
        None => format!("what {} says", value(&kind_word)),
    };
    let names = kind_word.as_str().is_some_and(|word| how.naming.contains(&word));
    let (kind, named) = match said(how.of) {
        Some(Value::Array(choices)) if !choices.is_empty() => {
            (format!("{kind_said} of {}", either(choices)), None)
        }
        Some(of) if names => (
            format!("{kind_said} of {}", value(of)),
            of.as_str().filter(|_| says_it(of).is_none() && !has_holes_in(of)).map(str::to_string),
        ),
        Some(of) => (format!("{kind_said} of {}", value(of)), None),
        None => (kind_said, None),
    };
    let mut about = Vec::new();
    if said(how.required).and_then(Value::as_bool) == Some(true) {
        about.push("A row cannot be without it.".to_string());
    }
    if let Some(sentence) = said(how.takes.0).and_then(|one| takes(one, how)) {
        about.push(sentence);
    }
    match said(how.unsaid) {
        Some(Value::Array(all)) if !all.is_empty() => about.push(format!(
            "Where nothing is said: {}.",
            joined(&all.iter().map(value).collect::<Vec<_>>(), "and")
        )),
        Some(Value::Array(_)) | None => {}
        Some(one) => about.push(format!("Where nothing is said: {}.", value(one))),
    }
    if let Some(Value::Array(usual)) =
        said(how.suggests).filter(|one| one.as_array().is_some_and(|all| !all.is_empty()))
    {
        about.push(format!("Usually {}.", either(usual)));
    }
    let text = std::iter::once(format!("`{name}`: {kind}."))
        .chain(about.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ");
    Field { name: name.to_string(), kind, names: named, about, text }
}

/// What a step makes is made of, from what it hands the action that makes
/// it, by the words that action says it reads; nothing where it says none.
pub(crate) fn model(with: &Value, how: &Modelled) -> Model {
    let mut model = Model::default();
    match with.get(how.parts) {
        Some(Value::Object(parts)) => {
            model.fields = parts.iter().map(|(name, written)| field(name, written, how)).collect();
        }
        Some(other) if !other.is_null() => {
            model.as_said = Some(format!("What it is made of is what {} says.", value(other)));
        }
        _ => {}
    }
    if let Some(beside) = with.get(how.beside.0).filter(|one| !one.is_null()) {
        let about = match with.get(how.beside.1).filter(|one| !one.is_null()) {
            Some(about) => format!(", and each of its rows is about what its {} says of that", value(about)),
            None => String::new(),
        };
        model.said.push(format!("It sits beside {}{about}.", value(beside)));
    }
    if with.get(how.by_hand) == Some(&Value::Bool(false)) {
        model.said.push("Its rows are not written by hand: what keeps it writes them.".to_string());
    }
    model
}
