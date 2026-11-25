//! What a blueprint names: the other blueprints it runs.
//!
//! A blueprint is found by its name by everything that runs, so its name
//! is identity, and one text runs another by saying its name. Of the
//! blueprints there are, most are never applied by a person: they are
//! parts, which something a person applies runs. Which is which is not
//! written down anywhere. It is worked out here, from the texts and from
//! what each action says of itself, as functions of plain values, so
//! that whoever lists blueprints can list first the ones somebody
//! applies and under each what it runs.
//!
//! **A text names another** ([`of`]) where:
//!
//! - it attaches it (`uses`), or a step of it calls or spawns it;
//! - the default of one of its params is that name, which a step then
//!   starts by what the param came to;
//! - an action it asks reaches the blueprint by a name of the action's
//!   own, which no text writes ([`crate::ActionSpec::reaching`]);
//! - or the name is written whole as a value anywhere else in it: in a
//!   list of what is done in order, in a table of what a word comes to;
//!   but not in a list or a mapping one of its params is unless somebody
//!   says, which is a setting somebody chooses among, as a table's rows
//!   are (the rows `std/project` sets a project up with).
//!
//! A name inside a sentence, in a comment, in an expression or as a key
//! names nothing; nor does a text name itself. What is counted as a name
//! is said by whoever asks (`known`): the blueprints there are.
//!
//! **What is run is followed all the way down** ([`parts`]): what a part
//! runs is a part too. What nothing else names is what somebody applies
//! ([`tops`]), with the few that somebody applies though something else
//! runs them too, which no text says and whoever asks hands over; and a
//! blueprint is a part of each of those it is reached from
//! ([`part_of`]). They are functions of what each names and of nothing
//! else, and the client's list is the same functions over the same
//! answer (`ui/src/blueprintlist.ts`): the two are held to one table of
//! cases (`ui/src/blueprintparts.json`).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::catalog::ActionSpec;
use crate::model::StepKind;
use crate::summary::{done, steps};
use crate::syntax::{self, Node, Value};

/// How one text names another.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum How {
    /// It is attached to every instance (`uses`).
    Attaches,
    /// A step calls it.
    Calls,
    /// A step spawns it.
    Spawns,
    /// It is what the param of this name is unless somebody says.
    Default(String),
    /// The action of this name, which the text asks, reaches it.
    Asks(String),
    /// It is written whole as a value somewhere else in the text.
    Writes,
}

impl How {
    /// The way in a few words, as a table of what names what says it.
    pub fn said(&self) -> String {
        match self {
            How::Attaches => "attaches it".to_string(),
            How::Calls => "calls it".to_string(),
            How::Spawns => "spawns it".to_string(),
            How::Default(param) => format!("the default of `{param}`"),
            How::Asks(action) => format!("asks `{action}`, which reaches it"),
            How::Writes => "writes it as a value".to_string(),
        }
    }
}

/// One blueprint a text names, and how.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Named {
    pub name: String,
    pub how: How,
}

/// Every text that is written whole as a value under `node`: never a key.
fn written<'a>(node: &'a Node, found: &mut Vec<&'a str>) {
    match &node.value {
        Value::Str(text) => found.push(text),
        Value::Seq(items) => items.iter().for_each(|item| written(item, found)),
        Value::Map(entries) => entries.iter().for_each(|entry| match entry.key.as_str() {
            "params" => settings(&entry.value, found),
            _ => written(&entry.value, found),
        }),
        Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) => {}
    }
}

/// What a text's params write, but for a list or a mapping one of them
/// is unless somebody says: that is a setting, as the rows of a table
/// are, and what it names is what whoever applies the text is handed to
/// choose among (the rows `std/project` sets a project up with, each
/// applied for whoever applies it). A name a param is unless said is
/// read where the param is ([`How::Default`]).
fn settings<'a>(params: &'a Node, found: &mut Vec<&'a str>) {
    let Value::Map(params) = &params.value else { return written(params, found) };
    for param in params {
        match &param.value.value {
            Value::Map(words) => words
                .iter()
                .filter(|word| {
                    !(word.key == "default" && matches!(word.value.value, Value::Seq(_) | Value::Map(_)))
                })
                .for_each(|word| written(&word.value, found)),
            _ => written(&param.value, found),
        }
    }
}

/// The blueprints `source` names, each way it names each, in the order
/// of the head of this file; one that is only written as a value is said
/// so once, and one named another way is not also said to be written.
/// `known` says which names are blueprints, and `actions` what each
/// action says of itself. A text that does not read names nothing.
pub fn of(source: &str, known: &dyn Fn(&str) -> bool, actions: &[ActionSpec]) -> Vec<Named> {
    let Ok(tree) = syntax::parse(source) else { return Vec::new() };
    let (Some(blueprint), _) = crate::lower::lower(&tree) else { return Vec::new() };
    // Everything said, in the order of the ways; what is no blueprint,
    // the text itself and what was said already are left out after.
    let mut said: Vec<(&str, How)> = Vec::new();
    for attached in &blueprint.uses {
        said.push((&attached.blueprint, How::Attaches));
    }
    let placed = steps(&blueprint);
    for placed in &placed {
        match &placed.step.kind {
            StepKind::Call(name) => said.push((name, How::Calls)),
            StepKind::Spawn(name) => said.push((name, How::Spawns)),
            StepKind::Do(_) | StepKind::Task => {}
        }
    }
    for param in &blueprint.params {
        if let Some(name) = param.default.as_ref().and_then(serde_json::Value::as_str) {
            said.push((name, How::Default(param.name.clone())));
        }
    }
    let done = done(&blueprint);
    for done in &done {
        let reaches = actions.iter().find(|spec| spec.name == done.action).map(|spec| &spec.reaches);
        for name in reaches.into_iter().flatten() {
            said.push((name, How::Asks(done.action.to_string())));
        }
    }
    let mut values = Vec::new();
    written(&tree, &mut values);
    said.extend(values.into_iter().map(|name| (name, How::Writes)));
    let mut found: Vec<Named> = Vec::new();
    for (name, how) in said {
        let named = Named { name: name.to_string(), how };
        let again = match named.how {
            How::Writes => found.iter().any(|one| one.name == name),
            _ => found.contains(&named),
        };
        if name != blueprint.name && known(name) && !again {
            found.push(named);
        }
    }
    found
}

/// What each blueprint names, by name: the plain values the rest is a
/// function of. A name that is no key of it is passed over wherever it
/// is read.
pub type Runs = BTreeMap<String, Vec<String>>;

/// Whether anything but itself names `name`.
fn is_named(name: &str, runs: &Runs) -> bool {
    runs.iter().any(|(by, names)| by != name && names.iter().any(|one| one == name))
}

/// What somebody applies, by name: the blueprints nothing else names,
/// and those of `applied`, which somebody applies though something else
/// runs them too. That is nothing a text says, so whoever asks says it.
pub fn tops(runs: &Runs, applied: &[String]) -> Vec<String> {
    runs.keys().filter(|name| applied.contains(name) || !is_named(name, runs)).cloned().collect()
}

/// Every part of `name`, by name: what it names, and what those name,
/// all the way down. Never itself, though something it runs names it.
pub fn parts(name: &str, runs: &Runs) -> Vec<String> {
    let mut found = BTreeSet::new();
    let mut next = vec![name];
    while let Some(one) = next.pop() {
        for part in runs.get(one).into_iter().flatten() {
            if part != name && runs.contains_key(part) && found.insert(part.as_str()) {
                next.push(part);
            }
        }
    }
    found.into_iter().map(str::to_string).collect()
}

/// What `name` is a part of: each blueprint somebody applies that it is
/// reached from, by name. Nothing for one that nothing runs.
pub fn part_of(name: &str, runs: &Runs, applied: &[String]) -> Vec<String> {
    let reached = |top: &String| parts(top, runs).iter().any(|part| part == name);
    tops(runs, applied).into_iter().filter(reached).collect()
}

/// The blueprints that are in no list: neither what somebody applies
/// nor a part of any of those. There is one where texts name each other
/// in a ring that nothing outside it names.
pub fn unreached(runs: &Runs, applied: &[String]) -> Vec<String> {
    let tops = tops(runs, applied);
    let mut reached: BTreeSet<String> = tops.iter().flat_map(|top| parts(top, runs)).collect();
    reached.extend(tops);
    runs.keys().filter(|name| !reached.contains(*name)).cloned().collect()
}
