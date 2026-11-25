//! How a project's things are arranged in the side panel, as plain
//! values: its *outline*.
//!
//! An outline is a nested list of nodes. A **thing** is one line the
//! panel draws for a project, named by a key ([`KEYS`]): its board, a
//! table, a library, mockups, its environments, its pipelines, the blueprints it uses,
//! its Git page, its Repo, and (reserved for later) one entry of its
//! folder. A **folder** is the outline's own: an id the factory gives it
//! (`f1`, `f2`, … never given twice in a project), a name, and the nodes
//! in it in order. Nothing a thing is known by anywhere else changes by
//! its being put in a folder: the outline only says where it is shown.
//!
//! A thing may be **shown as** something else for everybody reading the
//! project ([`Op::Name`]): the Board shown as Roadmap. What blueprints, the
//! daemon and every command know it by stays as it was (the table is still
//! `board`), so nothing running notices. A table is the one thing not
//! named here: it is called something else by its own title, which every
//! page reads, so that it has one name for people and not two.
//!
//! It is kept by the factory for each project (`NAMESPACE`, in the
//! factory's small durable state), and acts on nothing: nothing reads a
//! folder to decide who may read what. A thing that names nothing the
//! project has now is not refused: a table may be declared after it is
//! placed, and a file may come back from the Trash; whoever draws the
//! outline passes it over while it is not there.
//!
//! It is changed by operations and never written back whole ([`Op`],
//! [`apply`]): whoever reads it is handed less than is kept (what they
//! may not read is taken out), and an outline written back from what
//! they were shown would lose what they were not. Each operation is
//! handed `seen`, the things the page draws at the top that the outline
//! does not name, in the order drawn; those are named first, at the end
//! of the top, so that a place said by what was seen is the place meant.
//!
//! What an outline is held to is [`checked`]: the same rules, case for
//! case, as the client's (`ui/src/outline.ts`), held to one table
//! (`ui/src/outlinecases.json`). A key of a node that is not known here
//! is passed over and kept as it was, so that something said of a folder
//! later (who may read it) is a key more and not another kind of outline.

use serde_json::{Map, Value, json};

/// Where outlines are kept, in the factory's small durable state, by the
/// project's id.
pub const NAMESPACE: &str = "outline";
/// The most letters a folder's name has.
pub const LONGEST_NAME: usize = 60;
/// The most letters a thing's key has.
pub const LONGEST_KEY: usize = 2048;
/// How many folders deep an outline goes at most.
pub const DEEPEST: usize = 12;
/// The most things and folders one outline holds.
pub const MOST_NODES: usize = 2000;
/// The most folders one outline holds.
pub const MOST_FOLDERS: usize = 500;

/// The keys of a thing that are a word alone.
pub const KEYS: &[&str] = &["board", "environments", "pipelines", "blueprints", "git", "repo"];
/// The keys of a thing that are a word and then a name.
pub const NAMED: &[&str] = &["table:", "library:", "mockups:", "entry:"];

/// One node of an outline.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// One line the panel draws, by its key, and what it is shown as where somebody named it.
    Thing { key: String, name: Option<String>, more: Map<String, Value> },
    /// A folder of the outline's own.
    Folder { id: String, name: String, items: Vec<Node>, more: Map<String, Value> },
}

impl Node {
    /// What the node is named by in an operation: a thing's key, or a folder's id.
    pub fn named(&self) -> &str {
        match self {
            Node::Thing { key, .. } => key,
            Node::Folder { id, .. } => id,
        }
    }

    pub fn to_value(&self) -> Value {
        let mut written = Map::new();
        match self {
            Node::Thing { key, name, more } => {
                written.insert("thing".into(), json!(key));
                if let Some(name) = name {
                    written.insert("name".into(), json!(name));
                }
                written.extend(more.clone());
            }
            Node::Folder { id, name, items, more } => {
                written.insert("folder".into(), json!(id));
                written.insert("name".into(), json!(name));
                written.insert("items".into(), Value::Array(items.iter().map(Node::to_value).collect()));
                written.extend(more.clone());
            }
        }
        Value::Object(written)
    }
}

/// A project's outline, as it is kept.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outline {
    /// How many times it was written, which grows with each write.
    pub nth: i64,
    /// The number the next folder made is given.
    pub next: u64,
    pub items: Vec<Node>,
    /// What else it says, passed over and kept.
    pub more: Map<String, Value>,
}

impl Outline {
    pub fn to_value(&self) -> Value {
        let mut written = self.more.clone();
        written.insert("nth".into(), json!(self.nth));
        written.insert("next".into(), json!(self.next));
        written.insert("items".into(), Value::Array(self.items.iter().map(Node::to_value).collect()));
        Value::Object(written)
    }

    /// Every thing's key it names, wherever it is.
    pub fn things(&self) -> Vec<String> {
        fn walk(items: &[Node], found: &mut Vec<String>) {
            for node in items {
                match node {
                    Node::Thing { key, .. } => found.push(key.clone()),
                    Node::Folder { items, .. } => walk(items, found),
                }
            }
        }
        let mut found = Vec::new();
        walk(&self.items, &mut found);
        found
    }

    /// The outline with every thing that `keep` says no to taken out,
    /// folders and all else as they are: what is handed to somebody who
    /// may not read everything it names.
    pub fn kept_to(&self, keep: &dyn Fn(&str) -> bool) -> Outline {
        fn walk(items: &[Node], keep: &dyn Fn(&str) -> bool) -> Vec<Node> {
            items
                .iter()
                .filter_map(|node| match node {
                    Node::Thing { key, .. } => keep(key).then(|| node.clone()),
                    Node::Folder { id, name, items, more } => Some(Node::Folder {
                        id: id.clone(),
                        name: name.clone(),
                        items: walk(items, keep),
                        more: more.clone(),
                    }),
                })
                .collect()
        }
        Outline { items: walk(&self.items, keep), ..self.clone() }
    }
}

/// Whether `named` is a folder's id rather than a thing's key.
pub fn is_folder(named: &str) -> bool {
    named.len() > 1 && named.starts_with('f') && named[1..].bytes().all(|b| b.is_ascii_digit())
}

/// The number of a folder's id.
fn number_of(id: &str) -> u64 {
    id[1..].parse().unwrap_or(0)
}

/// Why `key` is no thing's key, if it is not.
pub fn unfit_key(key: &Value) -> Option<String> {
    let Some(text) = key.as_str() else {
        return Some(format!("A thing is named by its key, which is text, and {key} is not."));
    };
    if text.chars().any(char::is_control) {
        return Some("A thing's key is one line of text.".to_string());
    }
    let letters = text.chars().count();
    if letters > LONGEST_KEY {
        return Some(format!("A thing's key is {LONGEST_KEY} letters at most, and this one is {letters}."));
    }
    let named = NAMED.iter().any(|begins| text.len() > begins.len() && text.starts_with(begins));
    if KEYS.contains(&text) || named {
        return None;
    }
    Some(format!(
        "`{text}` is no thing of a project: a thing is `board`, `table:<name>`, `library:<name>`, \
         `mockups:<name>`, `environments`, `pipelines`, `blueprints`, `git`, `repo` or `entry:<path>`."
    ))
}

/// What a folder is called, trimmed, or why `name` is not that.
pub fn named_folder(name: &Value) -> Result<String, String> {
    let name = name.as_str().unwrap_or_default().trim().to_string();
    if name.is_empty() {
        return Err("A folder is called something: say what this one is called.".to_string());
    }
    let letters = name.chars().count();
    if letters > LONGEST_NAME {
        return Err(format!("A folder's name is {LONGEST_NAME} letters at most, and this one is {letters}."));
    }
    if name.chars().any(char::is_control) {
        return Err("A folder's name is one line of words.".to_string());
    }
    Ok(name)
}

/// What a thing is shown as, trimmed: nothing where it is shown as it
/// always was (nothing said, or only spaces); or why `name` is not that.
pub fn shown_name(name: &Value) -> Result<Option<String>, String> {
    let name = match name {
        Value::Null => return Ok(None),
        Value::String(name) => name.trim(),
        other => return Err(format!("What a thing is shown as is words, and {other} is not.")),
    };
    if name.is_empty() {
        return Ok(None);
    }
    let letters = name.chars().count();
    if letters > LONGEST_NAME {
        return Err(format!(
            "What a thing is shown as is {LONGEST_NAME} letters at most, and this is {letters}."
        ));
    }
    if name.chars().any(char::is_control) {
        return Err("What a thing is shown as is one line of words.".to_string());
    }
    Ok(Some(name.to_string()))
}

const NOT_ITEMS: &str = "`items` is a list, each a thing or a folder.";

/// What is counted as an outline is read.
#[derive(Default)]
struct Read {
    things: Vec<String>,
    folders: Vec<String>,
    nodes: usize,
    deepest: usize,
}

fn read_items(items: Option<&Value>, depth: usize, read: &mut Read) -> Result<Vec<Node>, String> {
    let items = match items {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(items)) => items,
        Some(_) => return Err(NOT_ITEMS.to_string()),
    };
    let mut nodes = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for item in items {
        let Some(item) = item.as_object() else {
            return Err(
                "Each item of an outline is a mapping: a thing (`thing`) or a folder (`folder`).".to_string()
            );
        };
        read.nodes += 1;
        let mut more = item.clone();
        match (more.remove("thing"), more.remove("folder")) {
            (Some(_), Some(_)) => {
                return Err(
                    "An item of an outline is a thing (`thing`) or a folder (`folder`), and this one says both."
                        .to_string(),
                );
            }
            (None, None) => {
                return Err(
                    "An item of an outline is a thing (`thing`) or a folder (`folder`), and this one is neither."
                        .to_string(),
                );
            }
            (Some(key), None) => {
                if let Some(wrong) = unfit_key(&key) {
                    return Err(wrong);
                }
                let key = key.as_str().unwrap_or_default().to_string();
                if read.things.contains(&key) {
                    return Err(format!("`{key}` is in the outline twice: each thing is in one place."));
                }
                read.things.push(key.clone());
                let name = shown_name(more.remove("name").as_ref().unwrap_or(&Value::Null))?;
                nodes.push(Node::Thing { key, name, more });
            }
            (None, Some(id)) => {
                let id = match id.as_str() {
                    Some(id) if is_folder(id) => id.to_string(),
                    _ => {
                        return Err(format!(
                            "A folder is named by an id the factory gives it (`f1`, `f2`, …), and {id} is not one."
                        ));
                    }
                };
                if read.folders.contains(&id) {
                    return Err(format!("Two folders are `{id}`: each folder has an id of its own."));
                }
                read.folders.push(id.clone());
                let name = named_folder(more.remove("name").as_ref().unwrap_or(&Value::Null))?;
                if names.iter().any(|one| one.to_lowercase() == name.to_lowercase()) {
                    return Err(format!(
                        "Two folders here are called {name}: each is called something of its own there."
                    ));
                }
                names.push(name.clone());
                read.deepest = read.deepest.max(depth + 1);
                let inside = more.remove("items");
                let items = read_items(inside.as_ref(), depth + 1, read)?;
                nodes.push(Node::Folder { id, name, items, more });
            }
        }
    }
    Ok(nodes)
}

fn count(value: Option<&Value>) -> Result<u64, String> {
    match value {
        None | Some(Value::Null) => Ok(0),
        Some(value) => value.as_u64().ok_or_else(|| "`nth` and `next` are counts.".to_string()),
    }
}

/// The outline `said` is, as it is kept, or why it is not one.
pub fn checked(said: &Value) -> Result<Outline, String> {
    let Some(said) = said.as_object() else {
        return Err("An outline is a mapping of its `items`, each a thing or a folder.".to_string());
    };
    let mut more = said.clone();
    let nth = count(more.remove("nth").as_ref())?;
    let next = count(more.remove("next").as_ref())?;
    let mut read = Read::default();
    let items = read_items(more.remove("items").as_ref(), 0, &mut read)?;
    if read.deepest > DEEPEST {
        return Err(format!("An outline goes {DEEPEST} folders deep at most."));
    }
    if read.folders.len() > MOST_FOLDERS {
        return Err(format!(
            "An outline holds {MOST_FOLDERS} folders at most, and this one holds {}.",
            read.folders.len()
        ));
    }
    if read.nodes > MOST_NODES {
        return Err(format!(
            "An outline holds {MOST_NODES} things and folders at most, and this one holds {}.",
            read.nodes
        ));
    }
    let past = read.folders.iter().map(|id| number_of(id) + 1).max().unwrap_or(1);
    Ok(Outline { nth: nth as i64, next: next.max(past).max(1), items, more })
}

/// Where something is put: into a folder (or the top), before a node of
/// that folder (or at its end).
#[derive(Clone, Debug, PartialEq)]
pub struct At {
    pub into: Option<String>,
    pub before: Option<String>,
}

/// A folder that was removed, as it is handed back so that it can be put
/// back: what it was, where it stood, and what was in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Removed {
    pub folder: String,
    pub name: String,
    pub into: Option<String>,
    pub before: Option<String>,
    pub items: Vec<String>,
}

impl Removed {
    pub fn to_value(&self) -> Value {
        json!({ "folder": self.folder, "name": self.name, "into": self.into, "before": self.before, "items": self.items })
    }
}

/// A change of an outline.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// A folder made, answered with its id.
    Make {
        name: String,
        at: At,
    },
    /// Things and folders put somewhere, in the order said.
    Put {
        nodes: Vec<String>,
        at: At,
    },
    Rename {
        folder: String,
        name: String,
    },
    /// A folder taken away, what was in it taking its place.
    Remove {
        folder: String,
    },
    /// A folder that was removed put back.
    Restore {
        removed: Removed,
    },
    /// What a thing is shown as, for everybody reading the project; nothing
    /// to show it as it always was.
    Name {
        thing: String,
        name: Option<String>,
    },
    /// The whole outline, back to the panel as it is with none.
    Forget,
}

const NOT_AN_OP: &str = "An outline is changed by one of `make`, `put`, `rename`, `remove`, `restore`, `name` and `forget`, said as `op`.";

fn text_of(said: &Map<String, Value>, key: &str) -> Option<String> {
    said.get(key).and_then(Value::as_str).map(str::to_string)
}

fn at_of(said: &Map<String, Value>) -> At {
    At { into: text_of(said, "into"), before: text_of(said, "before") }
}

fn named_list(said: Option<&Value>, what: &str) -> Result<Vec<String>, String> {
    match said {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(list)) => list
            .iter()
            .map(|one| {
                one.as_str().map(str::to_string).ok_or_else(|| format!("`{what}` is a list of names."))
            })
            .collect(),
        Some(Value::String(one)) => Ok(vec![one.clone()]),
        Some(_) => Err(format!("`{what}` is a list of names.")),
    }
}

impl Op {
    /// The change `said` asks for, or why it is none.
    pub fn read(said: &Value) -> Result<Op, String> {
        let said = said.as_object().ok_or(NOT_AN_OP)?;
        Ok(match said.get("op").and_then(Value::as_str) {
            Some("make") => {
                Op::Make { name: named_folder(said.get("name").unwrap_or(&Value::Null))?, at: at_of(said) }
            }
            Some("put") => {
                let mut nodes = named_list(said.get("nodes"), "nodes")?;
                nodes.extend(named_list(said.get("node"), "node")?);
                Op::Put { nodes, at: at_of(said) }
            }
            Some("rename") => Op::Rename {
                folder: text_of(said, "folder").unwrap_or_default(),
                name: named_folder(said.get("name").unwrap_or(&Value::Null))?,
            },
            Some("remove") => Op::Remove { folder: text_of(said, "folder").unwrap_or_default() },
            Some("restore") => {
                let removed = said.get("removed").and_then(Value::as_object).ok_or(
                    "What is put back is what removing a folder handed back, said as `removed`.".to_string(),
                )?;
                Op::Restore {
                    removed: Removed {
                        folder: text_of(removed, "folder").unwrap_or_default(),
                        name: named_folder(removed.get("name").unwrap_or(&Value::Null))?,
                        into: text_of(removed, "into"),
                        before: text_of(removed, "before"),
                        items: named_list(removed.get("items"), "items")?,
                    },
                }
            }
            Some("name") => {
                let thing = said.get("thing").cloned().unwrap_or(Value::Null);
                if let Some(wrong) = unfit_key(&thing) {
                    return Err(wrong);
                }
                let thing = thing.as_str().unwrap_or_default().to_string();
                if thing.starts_with("table:") {
                    return Err(format!(
                        "`{thing}` is a table, which is called something else by its own title, so that every page \
                         calls it by one name: rename the table itself."
                    ));
                }
                Op::Name { thing, name: shown_name(said.get("name").unwrap_or(&Value::Null))? }
            }
            Some("forget") => Op::Forget,
            _ => return Err(NOT_AN_OP.to_string()),
        })
    }
}

/// What a change came to.
#[derive(Clone, Debug, PartialEq)]
pub struct Applied {
    /// The outline as it is to be kept: nothing where it is forgotten.
    pub outline: Option<Outline>,
    /// Whether that is other than what was kept.
    pub changed: bool,
    /// What is handed back: the folder made, or what was removed.
    pub done: Value,
}

/// The level of `into`: the top, or a folder's items.
fn level<'a>(items: &'a mut Vec<Node>, into: Option<&str>) -> Option<&'a mut Vec<Node>> {
    let Some(into) = into else { return Some(items) };
    for node in items.iter_mut() {
        if let Node::Folder { id, items, .. } = node {
            if id == into {
                return Some(items);
            }
            if let Some(found) = level(items, Some(into)) {
                return Some(found);
            }
        }
    }
    None
}

/// Takes the node named `named` out of wherever it is, with the folder it
/// was in and what came after it there.
fn take(
    items: &mut Vec<Node>,
    named: &str,
    into: Option<&str>,
) -> Option<(Node, Option<String>, Option<String>)> {
    if let Some(at) = items.iter().position(|node| node.named() == named) {
        let node = items.remove(at);
        let after = items.get(at).map(|next| next.named().to_string());
        return Some((node, into.map(str::to_string), after));
    }
    for node in items.iter_mut() {
        if let Node::Folder { id, items, .. } = node {
            let id = id.clone();
            if let Some(found) = take(items, named, Some(&id)) {
                return Some(found);
            }
        }
    }
    None
}

/// Whether `inner` is `outer` or under it.
fn within(items: &[Node], outer: &str, inner: &str) -> bool {
    fn find<'a>(items: &'a [Node], named: &str) -> Option<&'a Node> {
        for node in items {
            if node.named() == named {
                return Some(node);
            }
            if let Node::Folder { items, .. } = node
                && let Some(found) = find(items, named)
            {
                return Some(found);
            }
        }
        None
    }
    match find(items, outer) {
        Some(Node::Folder { items, .. }) => outer == inner || find(items, inner).is_some(),
        _ => false,
    }
}

fn no_folder(id: &str) -> String {
    format!("There is no folder `{id}` in the outline.")
}

/// Puts `nodes` at `at` in `items`.
fn put_at(items: &mut Vec<Node>, nodes: Vec<Node>, at: &At) -> Result<(), String> {
    let level =
        level(items, at.into.as_deref()).ok_or_else(|| no_folder(at.into.as_deref().unwrap_or("")))?;
    let place = match &at.before {
        None => level.len(),
        Some(before) => level.iter().position(|node| node.named() == before).ok_or_else(|| {
            format!("`{before}` is not where it is to be put, so nothing can go before it there.")
        })?,
    };
    for (n, node) in nodes.into_iter().enumerate() {
        level.insert(place + n, node);
    }
    Ok(())
}

/// The outline `kept` (nothing where none is kept), with `seen` named at
/// the end of its top and then `op` done: what it comes to, or why it is
/// refused.
pub fn apply(kept: Option<&Outline>, op: &Op, seen: &[String]) -> Result<Applied, String> {
    if *op == Op::Forget {
        return Ok(Applied { outline: None, changed: kept.is_some(), done: json!({}) });
    }
    let mut outline = kept.cloned().unwrap_or(Outline { next: 1, ..Outline::default() });
    let mut named = outline.things();
    for key in seen {
        if let Some(wrong) = unfit_key(&json!(key)) {
            return Err(wrong);
        }
        if !named.contains(key) {
            named.push(key.clone());
            outline.items.push(Node::Thing { key: key.clone(), name: None, more: Map::new() });
        }
    }
    let before = outline.clone();
    let mut done = json!({});
    match op {
        Op::Make { name, at } => {
            let id = format!("f{}", outline.next);
            outline.next += 1;
            let made =
                Node::Folder { id: id.clone(), name: name.clone(), items: Vec::new(), more: Map::new() };
            put_at(&mut outline.items, vec![made], at)?;
            done = json!({ "made": id });
        }
        Op::Put { nodes, at } => {
            if nodes.is_empty() {
                return Err("Say what is to be put: a thing's key, or a folder's id.".to_string());
            }
            let mut moving = Vec::new();
            for (n, named) in nodes.iter().enumerate() {
                if nodes[..n].contains(named) {
                    return Err(format!("`{named}` is said twice in what is to be put."));
                }
                if at.before.as_deref() == Some(named.as_str()) {
                    return Err(format!("`{named}` cannot be put before itself."));
                }
                if is_folder(named) {
                    if let Some(into) = &at.into
                        && within(&outline.items, named, into)
                    {
                        return Err("A folder cannot be put inside itself.".to_string());
                    }
                } else if let Some(wrong) = unfit_key(&json!(named)) {
                    return Err(wrong);
                }
            }
            for named in nodes {
                match take(&mut outline.items, named, None) {
                    Some((node, ..)) => moving.push(node),
                    None if is_folder(named) => return Err(no_folder(named)),
                    None => moving.push(Node::Thing { key: named.clone(), name: None, more: Map::new() }),
                }
            }
            put_at(&mut outline.items, moving, at)?;
        }
        Op::Rename { folder, name: new } => {
            let found = find_folder(&mut outline.items, folder).ok_or_else(|| no_folder(folder))?;
            if let Node::Folder { name, .. } = found {
                *name = new.clone();
            }
        }
        Op::Remove { folder } => {
            let (node, into, after) = match take(&mut outline.items, folder, None) {
                Some(found @ (Node::Folder { .. }, ..)) => found,
                _ => return Err(no_folder(folder)),
            };
            let Node::Folder { id, name, items, .. } = node else { unreachable!() };
            let removed = Removed {
                folder: id,
                name,
                into: into.clone(),
                before: after.clone(),
                items: items.iter().map(|node| node.named().to_string()).collect(),
            };
            put_at(&mut outline.items, items, &At { into, before: after })?;
            done = json!({ "removed": removed.to_value() });
        }
        Op::Restore { removed } => {
            if !is_folder(&removed.folder) {
                return Err(format!(
                    "A folder is named by an id the factory gives it (`f1`, `f2`, …), and \"{}\" is not one.",
                    removed.folder
                ));
            }
            if find_folder(&mut outline.items, &removed.folder).is_some() {
                return Err(format!(
                    "There is a folder `{}` already: what was removed is put back only where its id is free.",
                    removed.folder
                ));
            }
            // Where it stood, as far as that is still there: in the folder it was in, before
            // what came after it, or after the last of what was in it, or at the end.
            let into = removed.into.clone().filter(|into| level(&mut outline.items, Some(into)).is_some());
            let there = level(&mut outline.items, into.as_deref()).and_then(|level| {
                let names: Vec<String> = level.iter().map(|node| node.named().to_string()).collect();
                let last = names.iter().rposition(|name| removed.items.contains(name));
                let after = last.and_then(|last| names.get(last + 1).cloned());
                removed.before.clone().filter(|before| names.contains(before)).or(after)
            });
            let mut inside = Vec::new();
            for named in &removed.items {
                match take(&mut outline.items, named, None) {
                    Some((node, ..)) => inside.push(node),
                    None if is_folder(named) => {}
                    None => inside.push(Node::Thing { key: named.clone(), name: None, more: Map::new() }),
                }
            }
            let at =
                At { into: into.clone(), before: there.filter(|before| !removed.items.contains(before)) };
            let back = Node::Folder {
                id: removed.folder.clone(),
                name: removed.name.clone(),
                items: inside,
                more: Map::new(),
            };
            put_at(&mut outline.items, vec![back], &at)?;
            outline.next = outline.next.max(number_of(&removed.folder) + 1);
        }
        Op::Name { thing, name } => match find_node(&mut outline.items, thing) {
            Some(Node::Thing { name: shown, .. }) => *shown = name.clone(),
            // A thing the outline does not name yet is named at the end of the top, as it is put there.
            _ if name.is_some() => {
                outline.items.push(Node::Thing { key: thing.clone(), name: name.clone(), more: Map::new() })
            }
            _ => {}
        },
        Op::Forget => unreachable!("forgetting is done before"),
    }
    let outline = checked(&outline.to_value())?;
    if outline.items == before.items && outline.more == before.more {
        // What it was asked comes to nothing: what is kept stays, seen or not.
        return Ok(Applied { outline: kept.cloned(), changed: false, done });
    }
    Ok(Applied { outline: Some(outline), changed: true, done })
}

/// The node named `named` (a thing's key or a folder's id), wherever it is.
fn find_node<'a>(items: &'a mut [Node], named: &str) -> Option<&'a mut Node> {
    for node in items.iter_mut() {
        if node.named() == named {
            return Some(node);
        }
        if let Node::Folder { items, .. } = node
            && let Some(found) = find_node(items, named)
        {
            return Some(found);
        }
    }
    None
}

fn find_folder<'a>(items: &'a mut [Node], folder: &str) -> Option<&'a mut Node> {
    find_node(items, folder).filter(|node| matches!(node, Node::Folder { .. }))
}

/// The folder at `path` (names from the top, each matched whatever its
/// capitals), by id, where every folder of it is there.
pub fn folder_at(items: &[Node], path: &[String]) -> Option<String> {
    let (mut level, mut found) = (items, None);
    for name in path {
        let (id, inside) = level.iter().find_map(|node| match node {
            Node::Folder { id, name: called, items, .. } if called.to_lowercase() == name.to_lowercase() => {
                Some((id, items))
            }
            _ => None,
        })?;
        (level, found) = (inside.as_slice(), Some(id.clone()));
    }
    found
}

/// `things` put in the folder at `path` (names from the top; the top
/// where it is empty), each folder of the path that is not there made at
/// the end of the one before it, as a blueprint asks it: the same done
/// twice. A thing that is in that folder already stays where it is there,
/// so that asking again moves nothing. What is handed back says the
/// folder they were put in (`folder`, nothing for the top).
pub fn placed_in(kept: Option<&Outline>, path: &[String], things: &[String]) -> Result<Applied, String> {
    if things.is_empty() && path.is_empty() {
        return Err("Say what is to be put: a thing's key, or the folder it is to be put in.".to_string());
    }
    let names = path.iter().map(|name| named_folder(&json!(name))).collect::<Result<Vec<_>, _>>()?;
    let (mut now, mut changed, mut into) = (kept.cloned(), false, None::<String>);
    for (n, name) in names.iter().enumerate() {
        let items = now.as_ref().map(|outline| outline.items.as_slice()).unwrap_or_default();
        if let Some(there) = folder_at(items, &names[..=n]) {
            into = Some(there);
            continue;
        }
        let made = apply(now.as_ref(), &Op::Make { name: name.clone(), at: At { into, before: None } }, &[])?;
        into = made.done["made"].as_str().map(str::to_string);
        (now, changed) = (made.outline, true);
    }
    let level: Vec<String> = match (&now, &into) {
        (Some(outline), Some(folder)) => {
            let mut items = outline.items.clone();
            level(&mut items, Some(folder))
                .map(|level| level.iter().map(|node| node.named().to_string()).collect())
        }
        (Some(outline), None) => Some(outline.items.iter().map(|node| node.named().to_string()).collect()),
        _ => None,
    }
    .unwrap_or_default();
    let nodes: Vec<String> = things.iter().filter(|thing| !level.contains(thing)).cloned().collect();
    if !nodes.is_empty() {
        let put = apply(now.as_ref(), &Op::Put { nodes, at: At { into: into.clone(), before: None } }, &[])?;
        changed |= put.changed;
        now = put.outline;
    }
    Ok(Applied {
        outline: if changed { now } else { kept.cloned() },
        changed,
        done: json!({ "folder": into }),
    })
}
