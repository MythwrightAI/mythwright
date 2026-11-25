//! The language as a schema an editor reads.
//!
//! A blueprint is a strict subset of YAML, and the editors people already
//! have complete and check YAML against a JSON Schema. This is that schema,
//! worked out from the language's own word list ([`crate::vocabulary`]) and
//! from the actions a program has, and never written by hand: a word that
//! is added to the list is offered by every editor with the sentence that
//! was written for it, and one that is not there is underlined.
//!
//! It says less than the reader does, on purpose. Whatever may be written
//! as an expression (`${ ... }`) is let be text here, an action or a
//! blueprint this program has not got is still a name, and what the reader
//! refuses that YAML allows (an anchor, a tag, a second document) a schema
//! cannot say at all. So it helps whoever is writing, and the check is
//! what decides.

use serde_json::{Map, Value, json};

use crate::catalog::ActionSpec;
use crate::inside::{Key, Within, Word};
use crate::template::HANDED_ON;
use crate::types::ValueType;
use crate::validate::DECISIONS;
use crate::vocabulary::{Choice, Field, Kind, SHAPES, Shape};

/// The dialect it is written in: the one the common language servers read.
/// A name, which nobody fetches.
const DIALECT: &str = "http://json-schema.org/draft-07/schema#";

/// What the three things said once are called among the definitions:
/// the name of an action, the name of one or of a decision, and what each
/// action takes. A shape's name is one word or words joined by `_`, so
/// none is called as these are.
const ACTION: &str = "an-action";
const DECISION: &str = "a-decision";
const TAKEN: &str = "what-an-action-takes";

/// Where a definition is, as a schema points at it.
fn reference(name: &str) -> Value {
    json!({ "$ref": format!("#/definitions/{name}") })
}

/// What the schema calls itself: a name and no address, which nothing
/// fetches. It says the product's name nowhere, here or in what an editor
/// shows it as, so that the one kept in the repository is not a second
/// place the name is written.
const ID: &str = "urn:blueprint:language";
const TITLE: &str = "A blueprint";

/// The schema of a blueprint, for a program that has `actions`.
pub fn schema(actions: &[ActionSpec]) -> Value {
    let mut definitions = Map::new();
    // The first shape is the file itself; the rest are what it is made of.
    for shape in &SHAPES[1..] {
        definitions.insert(shape.name.to_string(), shaped(shape));
    }
    // What this program can do is said once, and pointed at from each
    // place an action may be named: it is most of what there is to say.
    definitions.insert(ACTION.into(), named(actions, &[]));
    definitions.insert(DECISION.into(), named(actions, DECISIONS));
    definitions.insert(TAKEN.into(), json!({ "allOf": actions.iter().map(taken).collect::<Vec<_>>() }));
    let mut root = mapping(&SHAPES[0]);
    root.insert("$schema".into(), json!(DIALECT));
    root.insert("$id".into(), json!(ID));
    root.insert("title".into(), json!(TITLE));
    root.insert(
        "description".into(),
        json!(
            "A blueprint: what exists, what is heard, and what is done. This helps an editor complete and check \
             one as it is written; whether it can be applied is said by checking it."
        ),
    );
    root.insert("required".into(), json!(["blueprint"]));
    root.insert("definitions".into(), Value::Object(definitions));
    Value::Object(root)
}

/// A shape where it may stand: the mapping, or the bare value that is
/// short for one where the shape allows it (`retry: 3`).
fn shaped(shape: &Shape) -> Value {
    let whole = Value::Object(mapping(shape));
    match shape.short {
        Some(short) => json!({ "description": shape.help, "anyOf": [of(short), whole] }),
        None => whole,
    }
}

/// A shape written out as a mapping with a closed set of fields.
fn mapping(shape: &Shape) -> Map<String, Value> {
    let mut properties = Map::new();
    for field in shape.fields {
        properties.insert(field.name.to_string(), described(field));
    }
    let mut all = Vec::new();
    // Of the fields that stand in for each other a mapping has never two.
    for group in shape.alternatives {
        for (at, one) in group.iter().enumerate() {
            for other in &group[at + 1..] {
                all.push(json!({ "not": { "required": [one, other] } }));
            }
        }
    }
    // What an action takes, where the mapping names one beside its `with`.
    if shape.has("do") && shape.has("with") {
        all.push(reference(TAKEN));
    }
    let mut mapping = Map::new();
    mapping.insert("type".into(), json!("object"));
    mapping.insert("description".into(), json!(shape.help));
    mapping.insert("properties".into(), Value::Object(properties));
    mapping.insert("additionalProperties".into(), json!(false));
    if !all.is_empty() {
        mapping.insert("allOf".into(), Value::Array(all));
    }
    mapping
}

/// A field's value, with the sentence that says what the field is for.
fn described(field: &Field) -> Value {
    let mut value = of(field.value);
    match value.as_object_mut() {
        // A reference stands alone in this dialect: what is beside it is not read.
        Some(written) if written.contains_key("$ref") => {
            json!({ "description": field.help, "allOf": [value] })
        }
        Some(written) => {
            written.insert("description".into(), json!(field.help));
            value
        }
        None => value,
    }
}

/// A word that may be written, with what it means.
fn word(choice: &Choice) -> Value {
    json!({ "const": choice.name, "description": choice.help })
}

/// The name of an action: one of those this program has, each with what it
/// does, or any other name, since a package brings its own and a name may
/// be worked out.
fn named(actions: &[ActionSpec], also: &[&str]) -> Value {
    let mut named: Vec<Value> = also
        .iter()
        .map(|decision| json!({ "const": decision, "description": "Decides what happens to the step." }))
        .collect();
    named
        .extend(actions.iter().map(|action| json!({ "const": action.name, "description": offered(action) })));
    named.push(json!({ "type": "string" }));
    json!({ "anyOf": named })
}

/// What an action says of itself where its name is offered: what it does,
/// and after that what it makes (and where its rows set what something
/// does), which of its arguments is a command it
/// runs and which blueprints it reaches by itself, where it says any.
fn offered(action: &ActionSpec) -> String {
    let mut said = action.summary.clone();
    if let Some(makes) = &action.makes {
        said.push_str(&format!(" It makes {}, called what `{}` says.", makes.what, makes.named));
        if let Some(argument) = &makes.setting {
            said.push_str(&format!(" Where `{argument}` is said, its rows set what something does."));
        }
    }
    if !action.runs.is_empty() {
        let arguments: Vec<String> = action.runs.iter().map(|argument| format!("`{argument}`")).collect();
        said.push_str(&format!(" It runs a command, said by {}.", arguments.join(" or ")));
    }
    if !action.reaches.is_empty() {
        let names: Vec<String> = action.reaches.iter().map(|name| format!("`{name}`")).collect();
        said.push_str(&format!(" It reaches, by itself, {}.", names.join(" and ")));
    }
    said
}

/// What may be written after a field's colon, as a schema says it.
fn of(kind: Kind) -> Value {
    let names = || json!({ "anyOf": [{ "type": "string" }, { "type": "array", "items": { "type": "string" } }, { "type": "null" }] });
    match kind {
        // Text may be written as a number and is read as its digits.
        Kind::Text | Kind::Duration => json!({ "type": ["string", "integer"] }),
        Kind::Flag => json!({ "type": "boolean" }),
        // A number, or the expression that comes to one.
        Kind::Number => json!({ "type": ["number", "string"] }),
        Kind::Expression => json!({ "type": ["string", "boolean", "number"] }),
        Kind::Action => reference(ACTION),
        Kind::Decision => reference(DECISION),
        Kind::Blueprint | Kind::Agent | Kind::Device | Kind::Topic => json!({ "type": "string" }),
        Kind::Steps | Kind::Capabilities => names(),
        Kind::OneOf(choices) => json!({ "anyOf": choices.iter().map(word).collect::<Vec<_>>() }),
        Kind::Shape(name) => reference(name),
        Kind::MapOf(name) => json!({ "type": "object", "additionalProperties": reference(name) }),
        Kind::ListOf(name) => {
            json!({ "anyOf": [reference(name), { "type": "array", "items": reference(name) }] })
        }
        // What an action takes is said where the action is named (`taken`).
        Kind::Arguments(_) | Kind::Data => json!({ "type": ["object", "null"] }),
        Kind::Any => json!({}),
    }
}

/// What `with` may hold where `do` names this action: its arguments, each
/// with what it is for. An argument may always be an expression, and all
/// of them may be handed on at once, so nothing is said to be missing.
fn taken(action: &ActionSpec) -> Value {
    let mut arguments = Map::new();
    for argument in &action.args {
        let mut said = typed(argument.ty);
        if let Some(written) = said.as_object_mut() {
            let needed = if argument.required { " Needed." } else { "" };
            written.insert("description".into(), json!(format!("{}{needed}", argument.help)));
            if let Some(within) = &argument.within {
                inside(written, within, argument.ty);
            }
        }
        arguments.insert(argument.name.clone(), said);
    }
    let mut with = Map::new();
    with.insert("properties".into(), Value::Object(arguments));
    if !action.open {
        // Everything it was handed may be handed on under this one name.
        let handed_on: String = HANDED_ON.chars().map(|dot| format!("\\{dot}")).collect();
        with.insert("patternProperties".into(), json!({ format!("^{handed_on}$"): {} }));
        with.insert("additionalProperties".into(), json!(false));
    }
    json!({
        "if": { "properties": { "do": { "const": action.name } }, "required": ["do"] },
        "then": { "properties": { "with": with } },
    })
}

/// Adds to what is said of a value what the action says is written
/// inside it: each key with its sentence, and each of a key's few words
/// with its own. It is said and held to nothing: a key that is not named
/// here may be written, and so may a value that is none of the words,
/// since the action is what takes or refuses either, and a text may be
/// older than what the action now says of itself.
fn inside(said: &mut Map<String, Value>, within: &Within, ty: ValueType) {
    let mut held = Map::new();
    let keys: Map<String, Value> = within.keys.iter().map(|key| (key.name.clone(), keyed(key))).collect();
    match &within.each {
        // Things by a name of their writer's choosing: any name, each this.
        Some(each) => {
            let mut thing = Map::new();
            thing.insert("description".into(), json!(each));
            if !within.words.is_empty() {
                thing.insert("anyOf".into(), one_of(&within.words));
            }
            if !keys.is_empty() {
                thing.insert("properties".into(), Value::Object(keys));
            }
            held.insert("additionalProperties".into(), Value::Object(thing));
        }
        None => {
            held.insert("properties".into(), Value::Object(keys));
        }
    }
    // A list of mappings is said as one of them is.
    if ty == ValueType::List {
        said.insert("items".into(), Value::Object(held));
    } else {
        said.extend(held);
    }
}

/// One key of what an action reads: its sentence, its words, what it holds.
fn keyed(key: &Key) -> Value {
    let mut said = Map::new();
    said.insert("description".into(), json!(key.help));
    if !key.words.is_empty() {
        said.insert("anyOf".into(), one_of(&key.words));
    }
    if let Some(within) = &key.within {
        inside(&mut said, within, key.ty);
    }
    Value::Object(said)
}

/// A few words a value may be, each with what it means where it says,
/// and anything else: an expression, or a word the action has since
/// come to take.
fn one_of(words: &[Word]) -> Value {
    let mut all: Vec<Value> = words
        .iter()
        .map(|word| match word.help.is_empty() {
            true => json!({ "const": word.name }),
            false => json!({ "const": word.name, "description": word.help }),
        })
        .collect();
    all.push(json!({}));
    Value::Array(all)
}

/// What an argument of a type may be written as: the type, or text, which
/// an expression is.
fn typed(ty: ValueType) -> Value {
    match ty {
        ValueType::String | ValueType::Duration => json!({ "type": ["string", "number", "boolean"] }),
        ValueType::Int => json!({ "type": ["integer", "string"] }),
        ValueType::Number => json!({ "type": ["number", "string"] }),
        ValueType::Bool => json!({ "type": ["boolean", "string"] }),
        ValueType::List => json!({ "type": ["array", "string"] }),
        ValueType::Object => json!({ "type": ["object", "string"] }),
        ValueType::Any => json!({}),
    }
}
