//! How one text of a blueprint differs from another, in words.
//!
//! Whoever is told that something they run is behind the text now there
//! has to decide whether to take it, and a revision tells them nothing.
//! This says what moved, by the parts a person reads a blueprint by: what
//! it is handed, its steps, what it hears, and what it does when it is
//! taken, stopped or ended. The version each text says is not among them:
//! it is said first, wherever this is said (`conductor::Moved`). A part is
//! the same when it means the same, however it is laid out and whatever
//! comments it has ([`crate::format::meaning`]); what moved inside a step
//! is not gone into: the two texts are there to be read.
//!
//! It is said two ways from one reading. [`differences`] is a sentence
//! for each part of the text itself, for whoever is told that a text
//! moved. [`between`] says the same sentences, each with what it is about
//! as the graph names it, so that a page can show where; with the steps
//! that moved inside a handler or a task under the sentence for it; and
//! with what else of the text is another, which the first leaves unsaid.
//! That one is for a text set beside the one it was made from, where
//! somebody is to see everything that was done to it
//! ([`crate::summary::forked`]).

use crate::format::write;
use crate::model::within;
use crate::syntax::{self, Entry, Node};

/// The most that is said: past it, how many more there are.
const MOST: usize = 12;

/// One thing that moved between two texts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Difference {
    /// What moved, as a sentence's middle: ``the step `build` is changed``.
    pub text: String,
    /// What it is about in the later text, as the graph names it
    /// (`main/build`, `on.changed/@`). Nothing for what is gone from it,
    /// and for what is of the whole text.
    pub at: Option<String>,
    /// What the text calls that: a step's key, a handler's or a param's name.
    pub key: Option<String>,
    /// What moved inside it: the steps of a handler, or of a task.
    pub under: Vec<Difference>,
    /// Said only where everything is ([`between`]).
    fine: bool,
}

impl Difference {
    fn of(text: String, key: &str) -> Difference {
        Difference { text, key: Some(key.to_string()), ..Difference::default() }
    }

    fn whole(text: String) -> Difference {
        Difference { text, ..Difference::default() }
    }
}

/// What a part of a text means, however it is written.
fn meant(node: &Node) -> String {
    let mut out = String::new();
    write(node, &mut out);
    out
}

/// The things a mapping names, each with what it says of it.
fn named(node: Option<&Node>) -> Vec<&Entry> {
    node.and_then(Node::as_map).map(|entries| entries.iter().collect()).unwrap_or_default()
}

/// What a mapping says under `key`.
fn under<'a>(node: &'a Node, key: &str) -> Option<&'a Node> {
    node.as_map()?.iter().find(|entry| entry.key == key).map(|entry| &entry.value)
}

/// What is said of things of one kind that were added, taken away or
/// changed between two texts, in the order of their names and what is
/// gone last. `at` says what the graph calls one that is in the later
/// text; `inside` what moved within one that is changed.
fn moved(
    said: &mut Vec<Difference>,
    what: &str,
    (was, now): (&[&Entry], &[&Entry]),
    before: &str,
    at: impl Fn(&str) -> Option<String>,
    inside: impl Fn(&Node, &Node, &str) -> Vec<Difference>,
) {
    let mut there: Vec<&&Entry> = now.iter().collect();
    there.sort_by(|one, other| one.key.cmp(&other.key));
    for entry in there {
        let name = within::name(before, &entry.key);
        let one = match was.iter().find(|one| one.key == entry.key) {
            None => Difference::of(format!("{what} `{name}` is new"), &name),
            Some(old) if meant(&old.value) != meant(&entry.value) => Difference {
                under: inside(&old.value, &entry.value, &name),
                ..Difference::of(format!("{what} `{name}` is changed"), &name)
            },
            Some(_) => continue,
        };
        said.push(Difference { at: at(&name), ..one });
    }
    let mut gone: Vec<&&Entry> =
        was.iter().filter(|old| !now.iter().any(|entry| entry.key == old.key)).collect();
    gone.sort_by(|one, other| one.key.cmp(&other.key));
    for old in gone {
        let name = within::name(before, &old.key);
        said.push(Difference::of(format!("{what} `{name}` is gone"), &name));
    }
}

/// What moved among the steps two mappings name: `group` is what the
/// graph calls what they are steps of, and `before` what goes before
/// their names where they are a task's.
fn steps(was: Option<&Node>, now: Option<&Node>, group: &str, before: &str) -> Vec<Difference> {
    let (was, now) = (named(was), named(now));
    let mut said = Vec::new();
    moved(
        &mut said,
        "the step",
        (&was, &now),
        before,
        |name| Some(format!("{group}/{name}")),
        |old, new, name| steps(under(old, "task"), under(new, "task"), group, &within::task(name)),
    );
    // The same steps one after another in another order: none of them
    // is changed, and what is done is.
    let order = |steps: &[&Entry]| steps.iter().map(|entry| entry.key.clone()).collect::<Vec<_>>();
    let (before, after) = (order(&was), order(&now));
    let same = before.len() == after.len() && before.iter().all(|name| after.contains(name));
    if same && before != after {
        said.push(Difference {
            fine: true,
            ..Difference::whole("its steps are in another order".to_string())
        });
    }
    said
}

/// What a text says at its top that is said here by its own sentence, or
/// is not something that moved: what it is called, and the version it
/// says, which is said first wherever this is said.
const SAID_APART: &[&str] =
    &["blueprint", "version", "params", "steps", "on", "on_update", "on_stop", "permissions", "durable"];

/// Everything that moved between the text `was` and the text `now`, in
/// the order a blueprint is read, each with what it is about; nothing
/// where either cannot be read as a blueprint.
pub fn between(was: &str, now: &str) -> Vec<Difference> {
    let read =
        |text: &str| Some((crate::check(text, &crate::NoCatalog).blueprint?, syntax::parse(text).ok()?));
    let (Some((before, old)), Some((after, new))) = (read(was), read(now)) else { return Vec::new() };
    let mut said = Vec::new();
    let both = |key: &str| (named(under(&old, key)), named(under(&new, key)));
    let none = |_: &Node, _: &Node, _: &str| Vec::new();
    // The version each calls itself is said apart from this, and first
    // (`conductor::Moved`): what is said here is what moved between them.
    let (was_params, now_params) = both("params");
    moved(&mut said, "the param", (&was_params, &now_params), "", |_| None, none);
    said.extend(steps(under(&old, "steps"), under(&new, "steps"), "main", ""));
    let (was_handlers, now_handlers) = both("on");
    moved(
        &mut said,
        "the handler",
        (&was_handlers, &now_handlers),
        "",
        |name| Some(format!("on.{name}/@")),
        |old, new, name| steps(under(old, "steps"), under(new, "steps"), &format!("on.{name}"), ""),
    );
    let whole = |text: String| Difference::whole(text);
    for (what, key, was, now) in [
        (
            "when it is taken by one that is running (`on_update`)",
            "on_update",
            before.on_update.is_some(),
            after.on_update.is_some(),
        ),
        ("when it is stopped (`on_stop`)", "on_stop", !before.on_stop.is_empty(), !after.on_stop.is_empty()),
    ] {
        match (was, now) {
            (false, true) => said.push(whole(format!("it now says what it does {what}"))),
            (true, false) => said.push(whole(format!("it no longer says what it does {what}"))),
            (true, true) if under(&old, key).map(meant) != under(&new, key).map(meant) => {
                said.push(Difference { fine: true, ..whole(format!("what it does {what} is changed")) })
            }
            _ => {}
        }
    }
    if before.permissions != after.permissions {
        said.push(whole("what it may use (`permissions`) is changed".to_string()));
    }
    if before.durable != after.durable {
        said.push(whole(match after.durable {
            true => "what it is doing is now written down as it goes".to_string(),
            false => "what it is doing is no longer written down as it goes (`durable: false`)".to_string(),
        }));
    }
    // Whatever else either says at its top, by the word it is said under.
    let tops =
        |node: &'_ Node| named(Some(node)).into_iter().map(|entry| entry.key.clone()).collect::<Vec<_>>();
    let mut others = tops(&new);
    others.extend(tops(&old).into_iter().filter(|key| under(&new, key).is_none()));
    for key in others.iter().filter(|key| !SAID_APART.contains(&key.as_str())) {
        let text = match (under(&old, key).map(meant), under(&new, key).map(meant)) {
            (None, Some(_)) => format!("it now says `{key}`"),
            (Some(_), None) => format!("it no longer says `{key}`"),
            (Some(was), Some(now)) if was != now => format!("what it says under `{key}` is changed"),
            _ => continue,
        };
        said.push(Difference { fine: true, ..whole(text) });
    }
    said
}

/// What two texts of `name` call themselves, as it is said before what
/// moved between them: `std/lane-round 3 to 4`. The version is said
/// first wherever what moved is said, and this is the one way it is.
pub fn lead(name: &str, was: Option<&str>, now: Option<&str>) -> String {
    match (was, now) {
        (Some(was), Some(now)) => format!("{name} {was} to {now}"),
        (None, Some(now)) => format!("{name}, now version {now}"),
        (Some(was), None) => format!("{name}, which was version {was} and now says none"),
        (None, None) => name.to_string(),
    }
}

/// What moved between the text `was` and the text `now`, a sentence for
/// each thing, in the order a blueprint is read; nothing where either
/// cannot be read as one.
pub fn differences(was: &str, now: &str) -> Vec<String> {
    let mut said: Vec<String> =
        between(was, now).into_iter().filter(|one| !one.fine).map(|one| one.text).collect();
    if said.len() > MOST {
        let more = said.len() - MOST;
        said.truncate(MOST);
        said.push(format!("and {more} more"));
    }
    said
}
