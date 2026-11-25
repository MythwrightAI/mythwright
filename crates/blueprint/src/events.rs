//! What can happen: the events a handler may hear, and what each carries.
//!
//! An action is described so that a step which uses it can be checked
//! before it runs. An event is described for the same reason, from the
//! other side: a handler says which events are its own (`for:`) and reads
//! what they carry (`event.data.…`), and a name that is not there is a
//! handler that never fires, or fires and reads nothing, with no error
//! anywhere. So whatever emits an event says, once, what it is.
//!
//! The description is held both ways. A blueprint is checked against it
//! when it is applied ([`crate::heard`]), and the program that emits is
//! checked against it as it runs, wherever it is run to be tested
//! ([`EventList::fault`]).

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use crate::model::{Blueprint, Flow, Hook, StepKind};
use crate::template::has_holes;
use crate::types::ValueType;
use crate::validate::RESERVED_TOPICS;

/// One kind of event.
#[derive(Clone, Debug, Serialize)]
pub struct EventSpec {
    pub topic: String,
    pub summary: String,
    pub fields: Vec<FieldSpec>,
    /// Carries more than is listed: what a blueprint's own state holds,
    /// say, whose names are the blueprint's.
    pub open: bool,
    /// Kept in the journal. One that is not is only passed on to whoever
    /// is following as it happens, and no handler hears it.
    pub kept: bool,
    /// The scope whose holder may be told of it: what reading the thing it
    /// is about needs. [`ANYONE`] for one that anybody with a credential
    /// may hear; nothing until whoever describes the event has said
    /// ([`heard`]), which a test holds every one to.
    pub hears: Option<String>,
}

/// For an event anybody with a credential may hear: that the daemon
/// started, that the client was replaced.
pub const ANYONE: &str = "";

/// Says who may hear each of `specs`: for each, the scope beside the
/// longest beginning of its topic among `by`. One that fits none is left
/// unsaid.
pub fn heard(specs: Vec<EventSpec>, by: &[(&str, &str)]) -> Vec<EventSpec> {
    specs
        .into_iter()
        .map(|mut spec| {
            let fits = by
                .iter()
                .filter(|(begins, _)| spec.topic == *begins || spec.topic.starts_with(&format!("{begins}.")))
                .max_by_key(|(begins, _)| begins.len());
            if let Some((_, scope)) = fits {
                spec.hears = Some(scope.to_string());
            }
            spec
        })
        .collect()
}

/// The scope a caller needs to be told of an event of this topic, given
/// what is described. One of this program's own that nobody described is
/// its owner's alone; one a blueprint says for itself is heard by whoever
/// may read blueprints and what they do.
pub fn scope_to_hear<'a>(described: Option<&'a EventSpec>, topic: &str) -> &'a str {
    match described.and_then(|spec| spec.hears.as_deref()) {
        Some(scope) => scope,
        None if own(topic) => "admin",
        None => "blueprints:read",
    }
}

/// One thing an event carries.
#[derive(Clone, Debug, Serialize)]
pub struct FieldSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ValueType,
    /// There every time. One that is not may be left out, or be nothing.
    pub always: bool,
    pub help: String,
}

impl EventSpec {
    pub fn new(topic: &str, summary: &str) -> Self {
        Self {
            topic: topic.to_string(),
            summary: summary.to_string(),
            fields: Vec::new(),
            open: false,
            kept: true,
            hears: None,
        }
    }

    /// A field that is there every time.
    pub fn field(mut self, name: &str, ty: ValueType, help: &str) -> Self {
        self.fields.push(FieldSpec { name: name.to_string(), ty, always: true, help: help.to_string() });
        self
    }

    /// A field that is there some of the time.
    pub fn maybe(mut self, name: &str, ty: ValueType, help: &str) -> Self {
        self.fields.push(FieldSpec { name: name.to_string(), ty, always: false, help: help.to_string() });
        self
    }

    pub fn open(mut self) -> Self {
        self.open = true;
        self
    }

    /// Passed on and not kept.
    pub fn passing(mut self) -> Self {
        self.kept = false;
        self
    }

    pub fn has(&self, field: &str) -> bool {
        self.fields.iter().any(|known| known.name == field)
    }
}

/// Whether a topic is of a family only this program emits under.
pub fn own(topic: &str) -> bool {
    let family = topic.split('.').next().unwrap_or_default();
    RESERVED_TOPICS.contains(&family)
}

/// Whether `topic` is one `pattern` takes: `*` is one word, `**` any number.
pub fn matches(pattern: &str, topic: &str) -> bool {
    fn at(pattern: &[&str], topic: &[&str]) -> bool {
        match pattern.split_first() {
            None => topic.is_empty(),
            Some((&"**", rest)) => (0..=topic.len()).any(|skip| at(rest, &topic[skip..])),
            Some((word, rest)) => match topic.split_first() {
                Some((first, remaining)) => (*word == "*" || word == first) && at(rest, remaining),
                None => false,
            },
        }
    }
    let pattern: Vec<&str> = pattern.split('.').collect();
    let topic: Vec<&str> = topic.split('.').collect();
    at(&pattern, &topic)
}

/// The events there are, by topic.
#[derive(Clone, Debug, Default, Serialize)]
pub struct EventList {
    specs: Vec<EventSpec>,
}

impl EventList {
    pub fn new(mut specs: Vec<EventSpec>) -> Self {
        specs.sort_by(|a, b| a.topic.cmp(&b.topic));
        specs.dedup_by(|later, earlier| later.topic == earlier.topic);
        Self { specs }
    }

    pub fn specs(&self) -> &[EventSpec] {
        &self.specs
    }

    pub fn get(&self, topic: &str) -> Option<&EventSpec> {
        self.specs.iter().find(|spec| spec.topic == topic)
    }

    /// What is wrong with an event as it was emitted, held to what was
    /// said of it: nothing, for one that is as described or is not this
    /// program's to describe.
    pub fn fault(&self, topic: &str, data: &Value) -> Option<String> {
        if !own(topic) {
            return None;
        }
        let Some(spec) = self.get(topic) else {
            return Some(format!(
                "the event `{topic}` was emitted and is not described: say what it is and carries where \
                 its crate lists its events (`events.rs`)"
            ));
        };
        let empty = serde_json::Map::new();
        let carried = match data {
            Value::Object(carried) => carried,
            Value::Null => &empty,
            other => {
                return Some(format!(
                    "the event `{topic}` was emitted carrying {other}, and what an event carries is named fields"
                ));
            }
        };
        if !spec.open
            && let Some(stray) = carried.keys().find(|name| !spec.has(name))
        {
            return Some(format!(
                "the event `{topic}` was emitted with `{stray}`, which its description does not list"
            ));
        }
        for field in &spec.fields {
            match carried.get(&field.name) {
                None | Some(Value::Null) if field.always => {
                    return Some(format!(
                        "the event `{topic}` was emitted without `{}`, which its description says is always there",
                        field.name
                    ));
                }
                Some(value) if !value.is_null() && !field.ty.accepts(value) => {
                    return Some(format!(
                        "the event `{topic}` was emitted with `{}` as {value}, and its description says {}",
                        field.name,
                        field.ty.name()
                    ));
                }
                _ => {}
            }
        }
        None
    }
}

/// What blueprints say to each other on a topic: the fields their `emit`
/// steps write.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Said {
    pub fields: BTreeSet<String>,
    /// One of them writes what cannot be read off its text (`with: ${ … }`).
    pub open: bool,
    /// The blueprints that say it.
    pub by: BTreeSet<String>,
}

impl Said {
    pub fn add(&mut self, other: &Said) {
        self.fields.extend(other.fields.iter().cloned());
        self.open |= other.open;
        self.by.extend(other.by.iter().cloned());
    }
}

/// Every topic written out that the steps of `flow` say with `emit`,
/// their hooks and the steps of their tasks among them.
pub(crate) fn said_in(flow: &Flow) -> Vec<String> {
    let mut found = Vec::new();
    said_by_steps(flow, "", &mut found);
    found.into_iter().map(|(topic, _)| topic).collect()
}

/// Every `emit` of a blueprint whose topic is written out: the topic, and
/// what is said on it.
pub fn emitted(blueprint: &Blueprint) -> Vec<(String, Said)> {
    let mut found = Vec::new();
    let name = blueprint.name.as_str();
    said_by_steps(&blueprint.flow, name, &mut found);
    for trigger in &blueprint.triggers {
        said_by_steps(&trigger.flow, name, &mut found);
    }
    let run = [&blueprint.on_success, &blueprint.on_fail, &blueprint.on_finish, &blueprint.on_stop];
    said_by_hooks(run.into_iter().flatten(), name, &mut found);
    said_by_hooks(blueprint.on_update.iter().flatten(), name, &mut found);
    found
}

fn said_by_hooks<'a>(hooks: impl IntoIterator<Item = &'a Hook>, name: &str, found: &mut Vec<(String, Said)>) {
    for hook in hooks {
        said_by(&hook.action, &hook.with, name, found);
    }
}

fn said_by_steps(flow: &Flow, name: &str, found: &mut Vec<(String, Said)>) {
    for step in &flow.steps {
        if let StepKind::Do(action) = &step.kind {
            said_by(action, &step.with, name, found);
        }
        let around = step.on_start.iter().chain(&step.on_success).chain(&step.on_finish).chain(&step.on_fail);
        said_by_hooks(around.chain(&step.on_timeout), name, found);
        said_by_hooks(step.watch.iter().map(|watch| &watch.hook), name, found);
        said_by_steps(&step.inner, name, found);
    }
}

/// What one `emit` says, where its topic is written out; and what a
/// `wait` says once it is listening (`say`), which is said the same way.
fn said_by(action: &str, with: &Value, name: &str, found: &mut Vec<(String, Said)>) {
    let with = match (action, with.get("say")) {
        ("emit", _) => with,
        ("wait", Some(say)) => say,
        _ => return,
    };
    let Some(topic) = with.get("topic").and_then(Value::as_str).filter(|topic| !has_holes(topic)) else {
        return;
    };
    let mut said = Said { by: BTreeSet::from([name.to_string()]), ..Said::default() };
    match with.as_object() {
        Some(given) => said.fields.extend(given.keys().filter(|key| *key != "topic").cloned()),
        None => said.open = true,
    }
    found.push((topic.to_string(), said));
}
