//! Where blueprints use each word of the language, each name an
//! expression starts with and each action: for whoever reads what a word
//! is for, a few texts that are written with it, each with the place.
//!
//! It is worked out from the texts as the reader reads them, walked by
//! the words of [`crate::vocabulary`] as [`crate::vocabulary::written`]
//! walks them, and never by looking for a word in the text: `retry` in a
//! comment, or as the name of a step, is no `retry`. Nothing is kept: it
//! is a function of the texts it is handed.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::syntax::Node;
use crate::template::{Cond, holes};
use crate::vocabulary::{Kind, SHAPES};

/// How many texts are said for each thing, at most. The rest are counted.
pub const MOST: usize = 5;

/// One place a text uses something: the text by its name, the line, and
/// where in it, as the graph names it (`fetch`, a step; `on.changed`, a
/// handler; `on.changed/post`, a step of it; `params.who`, a param),
/// empty for the text's own top.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Place {
    pub blueprint: String,
    pub line: u32,
    pub at: String,
}

/// The texts that use one thing, the shortest first, and how many more do.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Uses {
    pub places: Vec<Place>,
    pub more: usize,
}

/// Where each thing is used: a word by its mapping and its name
/// (`step/retry`, and `step` for the mapping itself), a name an
/// expression starts with (`event`), an action (`channel.post`). What no
/// text uses is not there.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Used {
    pub words: BTreeMap<String, Uses>,
    pub names: BTreeMap<String, Uses>,
    pub actions: BTreeMap<String, Uses>,
}

/// What one text uses, each thing at the first place it is written.
#[derive(Default)]
struct Found {
    words: BTreeMap<String, (u32, String)>,
    names: BTreeMap<String, (u32, String)>,
    actions: BTreeMap<String, (u32, String)>,
}

fn first(into: &mut BTreeMap<String, (u32, String)>, what: &str, line: u32, at: &str) {
    into.entry(what.to_string()).or_insert_with(|| (line, at.to_string()));
}

/// Where a thing written under `field` of a mapping, by the name `key`,
/// is: a step under its step or handler, a handler as `on.<name>`, and
/// any other thing by the field and its name.
fn within(at: &str, field: &str, key: &str) -> String {
    match field {
        "steps" | "task" if at.is_empty() => key.to_string(),
        "steps" | "task" => format!("{at}/{key}"),
        _ if at.is_empty() => format!("{field}.{key}"),
        _ => format!("{at}/{field}.{key}"),
    }
}

/// The names the expressions in some text start with.
fn names_in(text: &str, bare: bool, line: u32, at: &str, found: &mut Found) {
    let mut written: Vec<String> = Vec::new();
    if bare
        && !text.contains("${")
        && let Ok(cond) = Cond::parse(text)
    {
        written.extend(cond.expr.paths().into_iter().filter_map(|path| path.into_iter().next()));
    }
    for hole in holes(text).unwrap_or_default() {
        if let Ok(expr) = crate::expr::parse(hole) {
            written.extend(expr.paths().into_iter().filter_map(|path| path.into_iter().next()));
        }
    }
    for name in written {
        if crate::vocabulary::ROOTS.iter().any(|root| root.name == name) {
            first(&mut found.names, &name, line, at);
        }
    }
}

/// Every name an expression starts with, anywhere in what is written
/// under a word that is no mapping of the language (arguments, data).
fn names_under(node: &Node, at: &str, found: &mut Found) {
    match (node.as_str(), node.as_map(), node.as_seq()) {
        (Some(text), _, _) => names_in(text, false, node.span.start.line, at, found),
        (_, Some(entries), _) => entries.iter().for_each(|entry| names_under(&entry.value, at, found)),
        (_, _, Some(items)) => items.iter().for_each(|item| names_under(item, at, found)),
        _ => {}
    }
}

fn walk(node: &Node, shape: &'static str, line: u32, at: &str, found: &mut Found) {
    let Some(of) = SHAPES.iter().find(|one| one.name == shape) else { return };
    first(&mut found.words, shape, line, at);
    // A bare value in a mapping's place: a hook that is an action's name.
    if let (Some(text), Some(kind @ (Kind::Action | Kind::Decision))) = (node.as_str(), of.short) {
        if kind == Kind::Action || !crate::DECISIONS.contains(&text) {
            first(&mut found.actions, text, node.span.start.line, at);
        }
        return;
    }
    for entry in node.as_map().unwrap_or_default() {
        let Some(field) = of.fields.iter().find(|field| field.name == entry.key) else { continue };
        let line = entry.key_span.start.line;
        first(&mut found.words, &format!("{shape}/{}", field.name), line, at);
        match field.value {
            // `fail` is an action where an action is done, and a decision where one may be.
            kind @ (Kind::Action | Kind::Decision) => {
                let decided = |action: &&str| kind == Kind::Decision && crate::DECISIONS.contains(action);
                if let Some(action) = entry.value.as_str().filter(|action| !decided(action)) {
                    first(&mut found.actions, action, line, at);
                }
            }
            Kind::Arguments(Some(action)) => {
                first(&mut found.actions, action, line, at);
                names_under(&entry.value, at, found);
            }
            Kind::Expression => {
                if let Some(text) = entry.value.as_str() {
                    names_in(text, true, line, at, found);
                }
            }
            Kind::Shape(inside) => walk(&entry.value, inside, line, at, found),
            Kind::MapOf(inside) => {
                for one in entry.value.as_map().unwrap_or_default() {
                    let at = within(at, field.name, &one.key);
                    walk(&one.value, inside, one.key_span.start.line, &at, found);
                }
            }
            Kind::ListOf(inside) => {
                for one in entry.value.as_seq().unwrap_or_default() {
                    walk(one, inside, one.span.start.line, at, found);
                }
            }
            _ => names_under(&entry.value, at, found),
        }
    }
}

/// Where the texts handed (each by its name) use each word, name and
/// action: for each, the [`MOST`] shortest texts that use it, each at the
/// first place it does, and how many more texts use it. A text that does
/// not read uses nothing.
pub fn used(texts: &[(String, String)]) -> Used {
    let mut by_length: Vec<&(String, String)> = texts.iter().collect();
    by_length.sort_by(|one, other| one.1.len().cmp(&other.1.len()).then_with(|| one.0.cmp(&other.0)));
    let mut used = Used::default();
    for (name, source) in by_length {
        let Ok(root) = crate::syntax::parse(source) else { continue };
        let mut found = Found::default();
        walk(&root, "blueprint", 1, "", &mut found);
        for (into, from) in [
            (&mut used.words, found.words),
            (&mut used.names, found.names),
            (&mut used.actions, found.actions),
        ] {
            for (what, (line, at)) in from {
                let uses = into.entry(what).or_default();
                if uses.places.len() < MOST {
                    uses.places.push(Place { blueprint: name.clone(), line, at });
                } else {
                    uses.more += 1;
                }
            }
        }
    }
    used
}
