//! A blueprint said in plain words.
//!
//! A blueprint's text is written for the runner and for whoever writes
//! blueprints. Somebody who does neither should still be able to open one
//! and say what happens: what it is, what it makes, what it does when it
//! is applied, what it hears and then does, what it may do, and what it
//! does as it ends. This is that, worked out from the text as the
//! language reads it and from what each action and each event says of
//! itself, and from nothing else: no model is asked, nothing is run, and
//! the blueprint need not be running or even be installed.
//!
//! It is one function of plain values ([`summary`]), so that what a page
//! draws, what the command line prints and what somebody is shown before
//! they add a package are the same words. It lists no action: a step is
//! said by what it says of itself (`description`, as far as its first
//! full stop: [`says`]) and otherwise by its action's own sentence; what
//! is made is what an action says it makes ([`crate::catalog::Makes`]);
//! a command is an argument an action says is one. Only the actions that
//! are the language's own are said here by name ([`own`]), where they
//! are declared.
//!
//! Each part is left out where there is nothing to say, and a line says
//! what it is about as the graph names it ([`crate::graph`]), so that a
//! page can show the step it is a line of. A line holds no place in the
//! text: a text laid out another way, or given a comment, comes to the
//! same summary.
//!
//! What a text cannot say of itself is said where whoever asks knows it
//! and hands it over as plain values ([`summary_as`], [`forked`]): that
//! it is one that ships, and how what it does is changed; or that it is
//! a project's fork of one, what it was forked from, where it differs
//! and how the default has moved since.

pub mod forked;
mod modelled;
mod own;
mod words;

pub use modelled::{Field, Model};

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{ActionSpec, intrinsic};
use crate::events::{self, EventSpec};
use crate::model::*;
use crate::template::{HANDED_ON, has_holes};
use words::{called, condition, each, joined, long, says_it, value};

/// What the world outside a text says of itself: the actions there are
/// and the events, each with its own sentence, and what holding each
/// scope allows, as the list every scope is checked against says it.
pub struct Known<'a> {
    pub actions: &'a [ActionSpec],
    pub events: &'a [EventSpec],
    pub scopes: &'a [(&'a str, &'a str)],
}

/// A blueprint in words: its parts in order, each with its lines.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub name: String,
    pub parts: Vec<Part>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Part {
    /// `is`, `makes`, `does`, `hears`, `may` or `ends`.
    pub kind: String,
    /// What the part is headed by.
    pub title: String,
    /// Its lines are things done one after another.
    pub ordered: bool,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub text: String,
    /// What the line is about, as the graph names it: `main/build` for a
    /// step, `on.changed/@` for a handler. Nothing for a line about the
    /// whole of the blueprint.
    pub at: Option<String>,
    /// What the text calls that: a step's key, a handler's or a param's
    /// name, a scope.
    pub key: Option<String>,
    /// `command` or `secret`, for a line that says the blueprint runs a
    /// command or reaches a secret: what somebody who is deciding
    /// whether to apply it most needs to see.
    pub mark: Option<String>,
    /// The lines under it: a handler's steps, the steps of a task.
    pub under: Vec<Line>,
    /// Where the line leads, for a page that can go there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opens: Option<Opens>,
    /// Of a line that says what is done as the blueprint ends or is
    /// changed, which ending it is: `success` (its steps went well),
    /// `fail` (they failed), `finish` (they ended, however they went),
    /// `update` (it takes a new text while it is running) or `stop` (it
    /// is stopped). A page that plays the blueprint plays each as what it
    /// is; nothing for any other line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending: Option<String>,
    // What follows is the line taken apart, for a page that lays it out
    // and does not read it as one sentence: `text` is the whole of it,
    // and is what a terminal prints. Each is the daemon's own words.
    /// Its own words, without what holds it to when, for what or where:
    /// what a handler hears, by the event first; a step's or a hook's
    /// words; the value of a fact (`5`, beside `Version`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A word or two for what it says, where it is one fact of several
    /// about the whole (`Version`, `Permissions`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// What a fact lists, each by itself: what it keeps, what it gives
    /// out, the permissions it uses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
    /// Where a handler hears what it hears from, a sentence of `text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// What holds it to when, for what, from whom or where it is done:
    /// each a sentence of `text`, in the order `text` says them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub only: Vec<String>,
    /// A few words said of it beside its title: how a handler takes
    /// what it hears (one at a time by something, a burst once); whether
    /// what it starts is waited for, left running or attached.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// What something it starts is started under, as its text says it
    /// (`look`, `round-<the event's as>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub named: Option<String>,
    /// The handler that comes to the step it is said of, by name; nothing
    /// for a step done when it is applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// Of a line of what it makes: what is made and what it is called, for
    /// a page that lists them as a table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub made: Option<Made>,
    /// Of a line of what it makes: what that is made of, where the action
    /// that makes it says how that is read (a table's fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<Model>,
}

/// What a line of what a blueprint makes is about: what is made (`a
/// table`, `a blueprint`) and what it is called, as the text says it
/// (`` `agents` ``, `what the param `name` says`); nothing where its name
/// is not said.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Made {
    pub what: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its name as it is written out, where it is: what a part of another
    /// thing's model names it by ([`Field::names`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub called: Option<String>,
}

/// Where a line leads: something to open, or something a page does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opens {
    /// `blueprint`, another blueprint, by its name, which it starts;
    /// `default`, the blueprint that ships, by its name; `fork`, a
    /// project's fork of it, or its own file under the name, by its file; `table`, a table of the
    /// project, by its name; and what a page does: `apply` (apply it
    /// with something else said), `forking` (fork it) and `back` (take
    /// the fork away, behind a question).
    pub what: String,
    /// What it is called, where it is something to open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Line {
    fn of(text: impl Into<String>) -> Line {
        Line { text: text.into(), ..Line::default() }
    }

    fn at(mut self, at: impl Into<String>, key: impl Into<String>) -> Line {
        self.at = Some(at.into());
        self.key = Some(key.into());
        self
    }

    fn marked(mut self, mark: &str) -> Line {
        self.mark = Some(mark.to_string());
        self
    }

    /// A line of its own words and what qualifies them: the whole is the
    /// two said one after the other.
    fn qualified(title: String, only: Vec<String>) -> Line {
        let text = std::iter::once(&title).chain(&only).map(String::as_str).collect::<Vec<_>>().join(" ");
        Line { title: Some(title), only, ..Line::of(text) }
    }

    /// A fact about the whole: its words, and the same said short beside a label.
    fn fact(text: impl Into<String>, label: &str, title: impl Into<String>) -> Line {
        Line { label: Some(label.to_string()), title: Some(title.into()), ..Line::of(text) }
    }

    /// A fact that lists things, each set apart as written.
    fn listing(text: impl Into<String>, label: &str, items: &[String]) -> Line {
        let items = items.iter().map(|item| format!("`{item}`")).collect();
        Line { label: Some(label.to_string()), items, ..Line::of(text) }
    }
}

/// What a step says of itself, as far as its first full stop: the rest is
/// read where the step is opened. One that says nothing says nothing.
///
/// The one rule for saying a step in words, here and in the client
/// (`says` in `ui/src/graph/tile.ts`); the two are held to one table of
/// cases (`ui/src/graph/says.json`).
pub fn says(description: &str) -> String {
    let text = description.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut letters = text.char_indices().peekable();
    while let Some((at, letter)) = letters.next() {
        let ends = matches!(letter, '.' | '!' | '?');
        if ends && letters.peek().is_some_and(|(_, next)| next.is_whitespace()) {
            return text[..at + letter.len_utf8()].to_string();
        }
    }
    text
}

/// Words as a sentence: on one line, and ended.
fn sentence(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() || text.ends_with(['.', '!', '?', ':']) { text } else { format!("{text}.") }
}

/// Words that begin a sentence.
fn capital(text: &str) -> String {
    let mut letters = text.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => String::new(),
    }
}

/// A step where it stands: with its name in full, what the graph calls
/// it, and the handler it is a step of.
pub(crate) struct Placed<'a> {
    pub(crate) step: &'a Step,
    name: String,
    id: String,
    handler: Option<&'a str>,
}

fn placed<'a>(
    flow: &'a Flow,
    group: &str,
    before: &str,
    handler: Option<&'a str>,
    found: &mut Vec<Placed<'a>>,
) {
    for step in &flow.steps {
        let name = within::name(before, &step.id);
        found.push(Placed { step, id: format!("{group}/{name}"), name: name.clone(), handler });
        placed(&step.inner, group, &within::task(&name), handler, found);
    }
}

/// What the graph calls a handler's own steps, and the handler itself.
fn group(trigger: &Trigger) -> String {
    format!("on.{}", trigger.name)
}

/// Every step of a blueprint, those of its handlers and inside its tasks
/// among them, in the order they are written.
pub(crate) fn steps(blueprint: &Blueprint) -> Vec<Placed<'_>> {
    let mut found = Vec::new();
    placed(&blueprint.flow, "main", "", None, &mut found);
    for trigger in &blueprint.triggers {
        placed(&trigger.flow, &group(trigger), "", Some(&trigger.name), &mut found);
    }
    found
}

/// One thing a blueprint has an action do, wherever it says so: a step,
/// what is done around one, what a handler asks on a timer, what is done
/// as the blueprint ends.
pub(crate) struct Done<'a> {
    pub(crate) action: &'a str,
    with: &'a Value,
    /// The step it is of, as the graph and the text call it.
    of: Option<(String, String)>,
}

pub(crate) fn done(blueprint: &Blueprint) -> Vec<Done<'_>> {
    let mut found = Vec::new();
    for placed in steps(blueprint) {
        let of = Some((placed.id.clone(), placed.name.clone()));
        if let StepKind::Do(action) = &placed.step.kind {
            found.push(Done { action, with: &placed.step.with, of: of.clone() });
        }
        let step = placed.step;
        let around = step.on_start.iter().chain(&step.on_success).chain(&step.on_fail);
        let around = around.chain(&step.on_timeout).chain(&step.on_finish);
        for hook in around.chain(step.watch.iter().map(|watch| &watch.hook)) {
            found.push(Done { action: &hook.action, with: &hook.with, of: of.clone() });
        }
    }
    for trigger in &blueprint.triggers {
        if let TriggerSource::Poll { action, with, .. } = &trigger.source {
            found.push(Done {
                action,
                with,
                of: Some((format!("{}/@", group(trigger)), trigger.name.clone())),
            });
        }
    }
    let ends = [&blueprint.on_success, &blueprint.on_fail, &blueprint.on_finish, &blueprint.on_stop];
    for hook in ends.into_iter().flatten().chain(blueprint.on_update.iter().flatten()) {
        found.push(Done { action: &hook.action, with: &hook.with, of: None });
    }
    found
}

impl Known<'_> {
    fn action(&self, name: &str) -> Option<ActionSpec> {
        intrinsic(name).or_else(|| self.actions.iter().find(|spec| spec.name == name).cloned())
    }

    /// What an action does with what it is handed, in a sentence: the
    /// language's own by what the step hands it, any other by what it
    /// says of itself.
    fn doing(&self, action: &str, with: &Value) -> String {
        if let Some(said) = intrinsic(action).and_then(|_| own::own(action, with)) {
            return said;
        }
        match self.action(action) {
            Some(spec) => sentence(&says(&spec.summary)),
            None => format!("Do `{action}`, an action this was read without."),
        }
    }

    /// What holding a scope allows, as a sentence: the scope's own words,
    /// or its area's where the scope is one part of an area that is said
    /// whole (`extensions:<package>`).
    fn allows(&self, scope: &str) -> String {
        let area = scope.split(':').next().unwrap_or(scope);
        let said = self.scopes.iter().find(|(known, _)| *known == scope);
        match said.or_else(|| self.scopes.iter().find(|(known, _)| *known == area)) {
            // As far as the first thing it says: what follows a `;` is how the scope is written.
            Some((_, does)) => sentence(&capital(does.split(';').next().unwrap_or(does))),
            None => format!("What holding `{scope}` allows."),
        }
    }

    /// Where a scope stands in the list, for saying them in its order.
    fn place(&self, scope: &str) -> usize {
        let area = scope.split(':').next().unwrap_or(scope);
        let found = self.scopes.iter().position(|(known, _)| *known == scope);
        found
            .or_else(|| self.scopes.iter().position(|(known, _)| *known == area))
            .unwrap_or(self.scopes.len())
    }
}

/// A blueprint by its name, as something another names: set apart where
/// the name is written out, and by what says it where it is not.
fn blueprint_called(name: &str) -> String {
    let written = Value::String(name.to_string());
    match says_it(&written) {
        Some(says) => format!("the blueprint {says} names"),
        None => value(&written),
    }
}

/// What is said after a step's own words: when it is done, for what, and where.
fn qualifiers(step: &Step) -> Vec<String> {
    let mut only = Vec::new();
    if let Some(cond) = &step.when {
        only.push(format!("Only when {}.", condition(cond)));
    }
    if let Some(over) = &step.for_each {
        only.push(format!("Once for each of {}.", each(over)));
    }
    if let Some(device) = &step.device {
        only.push(format!("On the device {}.", value(&Value::String(device.clone()))));
    }
    only
}

/// The steps of a flow in order, each as a line: by what the step says
/// of itself, and where it says nothing by what it has done.
fn step_lines(flow: &Flow, group: &str, before: &str, known: &Known) -> Vec<Line> {
    let line = |step: &Step| {
        let name = within::name(before, &step.id);
        let said = says(step.description.as_deref().unwrap_or_default());
        let text = if !said.is_empty() {
            sentence(&said)
        } else {
            match &step.kind {
                StepKind::Do(action) => known.doing(action, &step.with),
                StepKind::Call(name) => format!("Run {} and wait for it to finish.", blueprint_called(name)),
                StepKind::Spawn(name) => format!("Start {} and go on.", blueprint_called(name)),
                StepKind::Task => "Do these as one.".to_string(),
            }
        };
        let mut line = Line::qualified(text, qualifiers(step)).at(format!("{group}/{name}"), name.clone());
        line.under = step_lines(&step.inner, group, &within::task(&name), known);
        line
    };
    flow.steps.iter().map(line).collect()
}

/// Something done at a point of a blueprint's life, as a line.
fn hook_line(hook: &Hook, known: &Known) -> Line {
    let only = hook.when.iter().map(|cond| format!("Only when {}.", condition(cond))).collect();
    Line::qualified(known.doing(&hook.action, &hook.with), only)
}

/// A default as it is said to somebody: a word, or several, or that
/// there is more than a line holds.
fn default(value: &Value) -> Option<String> {
    let plain = |item: &Value| !item.is_object() && !item.is_array();
    Some(match value {
        Value::Null => return None,
        Value::Array(items) if !items.is_empty() && items.iter().all(plain) => {
            joined(&items.iter().map(self::value).collect::<Vec<_>>(), "and")
        }
        Value::Array(items) if !items.is_empty() => {
            format!("a list of {}, written out in its text", items.len())
        }
        Value::Object(_) => "a mapping, written out in its text".to_string(),
        // Words of some length (what an agent is told, say) are read where they are written.
        Value::String(text) if text.len() > LONG || text.contains('\n') => {
            "words of its own, written out in its text".to_string()
        }
        other => self::value(other),
    })
}

/// How long a default may be and still be said whole in a line.
const LONG: usize = 80;

fn is(blueprint: &Blueprint) -> Vec<Line> {
    let mut lines = Vec::new();
    if let Some(description) = blueprint.description.as_deref().filter(|said| !said.trim().is_empty()) {
        lines.push(Line::of(sentence(description)));
    }
    if let Some(version) = &blueprint.version {
        lines.push(Line::fact(format!("This is version {version} of it."), "Version", version.clone()));
    }
    let names =
        |keys: Vec<&String>| joined(&keys.iter().map(|key| format!("`{key}`")).collect::<Vec<_>>(), "and");
    if !blueprint.state.is_empty() {
        let kept: Vec<String> = blueprint.state.keys().cloned().collect();
        lines.push(Line::listing(
            format!("It keeps, from one run to the next: {}.", names(blueprint.state.keys().collect())),
            "Keeps",
            &kept,
        ));
    }
    lines.push(match blueprint.durable {
        true => Line::fact(
            "What it is doing is written down as it goes, so that it is taken up where it was after a restart.",
            "After a restart",
            "Taken up where it was",
        ),
        false => Line::fact(
            "It writes nothing down as it goes: stopped in the middle, it is begun again from its start by \
             whatever asked for it.",
            "After a restart",
            "Begun again from its start",
        ),
    });
    if blueprint.is_long_lived() {
        lines.push(Line::fact(
            "It goes on running once its steps are done, for what it hears.",
            "Runs",
            "On, for what it hears",
        ));
    }
    if !blueprint.outputs.is_empty() {
        let given: Vec<String> = blueprint.outputs.keys().cloned().collect();
        lines.push(Line::listing(
            format!("It gives out: {}.", names(blueprint.outputs.keys().collect())),
            "Gives out",
            &given,
        ));
    }
    // What it may be told is said last: there may be a good many, and
    // what it is in a few lines is read before them.
    if !blueprint.params.is_empty() {
        let told = |param: &Param| {
            let mut said = vec![match param.description.as_deref().filter(|said| !said.trim().is_empty()) {
                Some(description) => sentence(description),
                None => "Its text does not say what it is for.".to_string(),
            }];
            if !param.choices.is_empty() {
                let choices: Vec<String> = param.choices.iter().map(value).collect();
                said.push(format!("One of {}.", joined(&choices, "or")));
            }
            match (param.required, param.default.as_ref().and_then(default)) {
                (true, _) => said.push("It has to be said.".to_string()),
                (false, Some(default)) => said.push(format!("Unless it is said: {default}.")),
                (false, None) => {}
            }
            Line { key: Some(param.name.clone()), ..Line::of(said.join(" ")) }
        };
        let many = match blueprint.params.len() {
            1 => "one thing".to_string(),
            many => format!("{many} things"),
        };
        let mut line = Line::of(format!("Whoever applies it may tell it {many}:"));
        line.under = blueprint.params.iter().map(told).collect();
        lines.push(line);
    }
    lines
}

impl Made {
    /// Another blueprint this one brings into being, by its name.
    fn blueprint(name: &str) -> Made {
        let called = (!has_holes(name)).then(|| name.to_string());
        Made { what: "a blueprint".to_string(), name: Some(blueprint_called(name)), called }
    }
}

/// Another blueprint as something this one brings into being.
fn another(name: &str) -> String {
    let called = blueprint_called(name);
    if called.starts_with('`') { format!("Another blueprint, {called}") } else { capital(&called) }
}

/// Another blueprint by its name, as something to open where the name is written out.
fn opens_blueprint(name: &str) -> Option<Opens> {
    (!has_holes(name)).then(|| Opens { what: "blueprint".to_string(), name: Some(name.to_string()) })
}

fn makes(blueprint: &Blueprint, known: &Known) -> Vec<Line> {
    let mut lines = Vec::new();
    for attachment in &blueprint.uses {
        let text = format!("{}, attached to it as `{}`.", another(&attachment.blueprint), attachment.key);
        let line = Line {
            title: Some(blueprint_called(&attachment.blueprint)),
            named: Some(format!("`{}`", attachment.key)),
            tags: vec!["attached".to_string()],
            opens: opens_blueprint(&attachment.blueprint),
            made: Some(Made::blueprint(&attachment.blueprint)),
            ..Line::of(text)
        };
        lines.push(line.at(format!("uses/{}", attachment.key), attachment.key.clone()));
    }
    for placed in steps(blueprint) {
        let step = placed.step;
        let key = step.key.as_ref().map(|key| value(&Value::String(key.clone())));
        let mut named = key.as_ref().map(|key| format!(" as {key}")).unwrap_or_default();
        if let Some(environment) = &step.environment {
            named.push_str(&format!(
                " in the project's environment {}",
                value(&Value::String(environment.clone()))
            ));
        }
        // A thing an action makes is said short too, where nothing more is said of it: a fact of the whole.
        let mut made = None;
        let (mut text, short, started) = match &step.kind {
            StepKind::Do(action) => match known.action(action).and_then(|spec| spec.makes) {
                Some(makes) => {
                    let name = step.with.get(&makes.named).filter(|name| !name.is_null());
                    made = Some((
                        Made {
                            what: makes.what.clone(),
                            name: name.map(|name| says_it(name).unwrap_or_else(|| value(name))),
                            called: name
                                .and_then(Value::as_str)
                                .filter(|name| !has_holes(name))
                                .map(str::to_string),
                        },
                        makes.model.map(|how| modelled::model(&step.with, &how)),
                    ));
                    match name {
                        Some(name) => {
                            let short = format!("{} {}", makes.what, called(name));
                            (format!("{}.", capital(&short)), short, None)
                        }
                        None => (format!("{}.", capital(&makes.what)), makes.what.clone(), None),
                    }
                }
                None => continue,
            },
            StepKind::Spawn(name) => (
                format!("{}, started under it{named} and left running.", another(name)),
                blueprint_called(name),
                Some((name, "left running")),
            ),
            StepKind::Call(name) => (
                format!("{}, run under it{named} and waited for.", another(name)),
                blueprint_called(name),
                Some((name, "waited for")),
            ),
            StepKind::Task => continue,
        };
        let mut only = Vec::new();
        if let Some(over) = &step.for_each {
            only.push(format!("One for each of {}.", each(over)));
        }
        for said in &only {
            text.push_str(&format!(" {said}"));
        }
        if let Some(handler) = placed.handler {
            text.push_str(&format!(" Each time its handler `{handler}` comes to it."));
        }
        let mut line = match started {
            Some((name, how)) => Line {
                title: Some(short),
                named: key,
                tags: vec![how.to_string()],
                opens: opens_blueprint(name),
                only,
                by: placed.handler.map(str::to_string),
                made: Some(Made::blueprint(name)),
                ..Line::of(text)
            },
            None if only.is_empty() && placed.handler.is_none() => Line::fact(text, "Makes", short),
            None => {
                Line { title: Some(short), only, by: placed.handler.map(str::to_string), ..Line::of(text) }
            }
        };
        if let Some((made, model)) = made {
            line.made = Some(made);
            line.model = model;
        }
        lines.push(line.at(placed.id, placed.name));
    }
    lines
}

/// What a handler hears, in words: the event by what whoever says it
/// says of it, and a timer by how often. As a sentence, and as a title,
/// which says the event first and what it is after it.
fn heard(trigger: &Trigger, blueprint: &Blueprint, known: &Known) -> (String, String) {
    let pattern = match &trigger.source {
        TriggerSource::Event(pattern) => pattern,
        TriggerSource::Every(ms) => {
            let said = format!("Every {}.", every(*ms));
            return (said.clone(), said);
        }
        TriggerSource::At(at) => {
            let said = match crate::at::read(at) {
                Ok(read) => format!("{}.", capital(&read.said())),
                Err(_) => format!("At the time of day {} says.", value(&Value::String(at.clone()))),
            };
            return (said.clone(), said);
        }
        TriggerSource::Poll { every_ms, action, with } => {
            let said = format!(
                "Every {} it asks this, and goes on when the answer is another: {}",
                every(*every_ms),
                known.doing(action, with)
            );
            return (said.clone(), said);
        }
    };
    if has_holes(pattern) {
        let said = format!("When the event {} names is said.", value(&Value::String(pattern.clone())));
        return (said.clone(), said);
    }
    let exact: Vec<&EventSpec> = known.events.iter().filter(|spec| spec.topic == *pattern).collect();
    let fits: Vec<&EventSpec> = match exact.is_empty() {
        true => known.events.iter().filter(|spec| events::matches(pattern, &spec.topic)).collect(),
        false => exact,
    };
    let said = |spec: &EventSpec| small(says(&spec.summary).trim_end_matches(['.', '!', '?']));
    match fits.as_slice() {
        [] => {
            let title = match says_itself(blueprint, pattern) {
                true => format!("`{pattern}`, which it says itself"),
                false => format!("`{pattern}`"),
            };
            (format!("When `{pattern}` is said."), title)
        }
        [one] => (
            format!("When this happens: {} (`{pattern}`).", said(one)),
            format!("`{pattern}`: {}", said(one)),
        ),
        many => {
            // A few are said; a pattern that takes a great many is said by those and how many more.
            let mut some: Vec<String> = many.iter().take(MOST_SAID).map(|spec| said(spec)).collect();
            if many.len() > MOST_SAID {
                some.push(format!("{} more of the kind", many.len() - MOST_SAID));
            }
            (
                format!("When any of these happens: {} (`{pattern}`).", some.join("; ")),
                format!("`{pattern}`: any of {}", some.join("; ")),
            )
        }
    }
}

/// Whether a text says the topic it hears itself, by an `emit` of it written out.
fn says_itself(blueprint: &Blueprint, topic: &str) -> bool {
    done(blueprint)
        .iter()
        .any(|one| one.action == "emit" && one.with.get("topic").and_then(Value::as_str) == Some(topic))
}

/// How often, after `every`: `minute` for one of them, `5 minutes` for more.
fn every(millis: u64) -> String {
    let said = long(millis);
    match said.strip_prefix("1 ") {
        Some(unit) if !unit.contains(' ') => unit.to_string(),
        _ => said,
    }
}

/// Words that began a sentence, to go on in the middle of one: a word
/// that is a name or a mark is left as it is written.
fn small(text: &str) -> String {
    let mut letters = text.chars();
    match (letters.next(), letters.clone().next()) {
        (Some(first), Some(second)) if first.is_uppercase() && (second.is_lowercase() || second == ' ') => {
            first.to_lowercase().chain(letters).collect()
        }
        _ => text.to_string(),
    }
}

/// How many of the events a pattern takes are said by their own words.
const MOST_SAID: usize = 4;

fn hears(blueprint: &Blueprint, known: &Known) -> Vec<Line> {
    let mut lines = Vec::new();
    if blueprint.one_at_a_time && !blueprint.triggers.is_empty() {
        lines.push(Line::of("Its handlers take turns: one of them runs at a time."));
    }
    for trigger in &blueprint.triggers {
        let (said, title) = heard(trigger, blueprint, known);
        let from = matches!(trigger.source, TriggerSource::Event(_)).then(|| {
            match trigger.scope {
                TriggerScope::Project => "Heard from its own project, and from the factory itself.",
                TriggerScope::Fleet => "Heard from every project.",
                TriggerScope::Own => "Heard only from itself and what it started.",
                TriggerScope::Target => "Heard only from the instance it is attached to.",
            }
            .to_string()
        });
        let mut only = Vec::new();
        let address: Vec<String> =
            trigger.address.iter().map(|(field, is)| format!("`{field}` is {}", value(is))).collect();
        if !address.is_empty() {
            only.push(format!("Only where its {}.", joined(&address, "and")));
        }
        if let Some(cond) = &trigger.when {
            only.push(format!("Only when {}.", condition(cond)));
        }
        if trigger.outside {
            only.push("It is said from outside: by a person, a key or a caller.".to_string());
        }
        // How it takes what it hears: said whole in the text, and in a few words beside.
        let mut tags = Vec::new();
        let mut taking = Vec::new();
        if let Some(turn) = &trigger.turn {
            let by = value(&Value::String(turn.clone()));
            taking.push(format!("Of those that share {by}, one is taken at a time."));
            tags.push(format!("one at a time by {by}"));
        }
        if let Some(quiet) = trigger.debounce_ms {
            taking.push(format!("A burst of them is taken once, after {} of quiet.", long(quiet)));
            tags.push(format!("a burst taken once, after {} of quiet", long(quiet)));
        }
        let whole = std::iter::once(&said).chain(&from).chain(&only).chain(&taking);
        let text = whole.map(String::as_str).collect::<Vec<_>>().join(" ");
        let group = group(trigger);
        let mut line = Line { title: Some(title), from, only, tags, ..Line::of(text) }
            .at(format!("{group}/@"), trigger.name.clone());
        line.under = step_lines(&trigger.flow, &group, "", known);
        lines.push(line);
    }
    lines
}

/// The secrets an environment names, each with the variable it is handed as.
fn secrets(env: &serde_json::Map<String, Value>) -> impl Iterator<Item = (&String, &str)> {
    env.iter().filter_map(|(variable, value)| Some((variable, crate::secrets::named(value)?)))
}

fn may(blueprint: &Blueprint, known: &Known) -> Vec<Line> {
    let mut lines = vec![Line::of("It runs as whoever applies it, and can do no more than they can.")];
    match blueprint.permissions.as_deref() {
        Some([]) => lines.push(Line::fact(
            "It lists nothing it may use, so it can do nothing that takes a permission.",
            "Permissions",
            "None",
        )),
        Some(listed) => {
            let set: Vec<String> = listed.iter().map(|one| format!("`{one}`")).collect();
            lines.push(Line::listing(
                format!("Of what they can do, it uses only what it lists: {}.", joined(&set, "and")),
                "Permissions",
                listed,
            ));
        }
        None => {}
    }
    let all = done(blueprint);
    let found = steps(blueprint);
    let mut needed = BTreeSet::new();
    let mut others = Vec::new();
    for spec in all.iter().filter_map(|one| known.action(one.action)) {
        needed.extend(spec.capability);
    }
    for placed in &found {
        if let StepKind::Call(name) | StepKind::Spawn(name) = &placed.step.kind {
            needed.insert("blueprints:run".to_string());
            let said = blueprint_called(name);
            if !others.contains(&said) {
                others.push(said);
            }
        }
    }
    let handed: Vec<(Option<&Placed>, &String, &str)> = secrets(&blueprint.env)
        .map(|(variable, secret)| (None, variable, secret))
        .chain(found.iter().flat_map(|placed| {
            secrets(&placed.step.env).map(move |(variable, secret)| (Some(placed), variable, secret))
        }))
        .collect();
    if !handed.is_empty() {
        needed.insert("secrets:use".to_string());
    }
    let mut needed: Vec<String> = needed.into_iter().collect();
    needed.sort_by_key(|scope| (known.place(scope), scope.clone()));
    for scope in needed {
        lines.push(Line { key: Some(scope.clone()), ..Line::of(known.allows(&scope)) });
    }
    for one in &all {
        let Some(spec) = known.action(one.action).filter(|spec| !spec.runs.is_empty()) else { continue };
        let command =
            spec.runs.iter().find_map(|argument| one.with.get(argument).filter(|said| !said.is_null()));
        let tall = |command: &Value| command.as_str().map_or(1, |text| text.trim().lines().count());
        let text = match command {
            Some(command) => match says_it(command) {
                Some(says) => format!("It runs a command that {says} says."),
                // A script is read where it is written: a line is no place for it.
                None if tall(command) > 1 => {
                    format!("It runs a command of {} lines, written out in its text.", tall(command))
                }
                None => format!("It runs a command: {}.", value(command)),
            },
            // Handed whole (`with: ${ … }`, or all of what it was handed), or said by another argument.
            None if !one.with.is_object() || one.with.get(HANDED_ON).is_some() => {
                "It runs a command it is handed as it runs, which its text does not write out.".to_string()
            }
            None => continue,
        };
        let line = Line::of(text).marked("command");
        lines.push(match &one.of {
            Some((at, key)) => line.at(at.clone(), key.clone()),
            None => line,
        });
    }
    for (placed, variable, secret) in handed {
        let secret = Value::String(secret.to_string());
        let which = match says_it(&secret) {
            Some(says) => format!("the secret {says} names"),
            None => format!("the secret {}", value(&secret)),
        };
        let line = Line::of(format!("It hands {which} to what it runs, as the variable `{variable}`."))
            .marked("secret");
        lines.push(match placed {
            Some(placed) => line.at(placed.id.clone(), placed.name.clone()),
            None => line,
        });
    }
    if !others.is_empty() {
        lines.push(Line::of(format!(
            "What the blueprints it starts may do is said of each of them: {}.",
            joined(&others, "and")
        )));
    }
    lines
}

fn ends(blueprint: &Blueprint, known: &Known) -> Vec<Line> {
    let said = |ending: &str, words: &str, hooks: &[Hook]| {
        let mut line = Line { ending: Some(ending.to_string()), ..Line::of(words) };
        line.under = hooks.iter().map(|hook| hook_line(hook, known)).collect();
        line
    };
    let mut lines = Vec::new();
    if !blueprint.on_success.is_empty() {
        lines.push(said("success", "When its steps went well:", &blueprint.on_success));
    }
    if !blueprint.on_fail.is_empty() {
        lines.push(said("fail", "When its steps fail:", &blueprint.on_fail));
    }
    if !blueprint.on_finish.is_empty() {
        lines.push(said("finish", "When its steps have ended, however they went:", &blueprint.on_finish));
    }
    match blueprint.on_update.as_deref() {
        Some([]) => lines.push(said(
            "update",
            "When it takes a new text while it is running, it goes on where it is: its steps are not run again.",
            &[],
        )),
        Some(hooks) => lines.push(said(
            "update",
            "When it takes a new text while it is running, in place of running its steps again:",
            hooks,
        )),
        None => {}
    }
    if !blueprint.on_stop.is_empty() {
        lines.push(said("stop", "When it is stopped:", &blueprint.on_stop));
    }
    lines
}

/// What a blueprint is and does, in words, from the text as it was read
/// and from what the actions and events there are say of themselves.
pub fn summary(blueprint: &Blueprint, known: &Known) -> Summary {
    let part = |kind: &str, title: &str, ordered: bool, lines: Vec<Line>| Part {
        kind: kind.to_string(),
        title: title.to_string(),
        ordered,
        lines,
    };
    let parts = vec![
        part("is", "What it is", false, is(blueprint)),
        part("makes", "What it makes", false, makes(blueprint, known)),
        part("does", "What it does when it is applied", true, step_lines(&blueprint.flow, "main", "", known)),
        part("hears", "What it hears, and what it then does", false, hears(blueprint, known)),
        part("may", "What it may do", false, may(blueprint, known)),
        part("ends", "What it does as it ends or is changed", false, ends(blueprint, known)),
    ];
    Summary {
        name: blueprint.name.clone(),
        parts: parts.into_iter().filter(|part| !part.lines.is_empty()).collect(),
    }
}

/// A summary with what its text's standing among the blueprints that
/// ship says, where whoever asks knows it ([`forked`]): one that ships,
/// or a project's fork of one.
pub fn summary_as(blueprint: &Blueprint, known: &Known, standing: Option<&forked::Standing>) -> Summary {
    let said = summary(blueprint, known);
    match standing {
        Some(standing) => forked::placed(said, blueprint, known, standing),
        None => said,
    }
}

fn written(lines: &[Line], ordered: bool, leads: bool, depth: usize, out: &mut String) {
    for (index, line) in lines.iter().enumerate() {
        let lead = match (&line.mark, ordered) {
            (Some(_), _) => "! ".to_string(),
            (None, true) => format!("{}. ", index + 1),
            (None, false) if depth > 1 => "- ".to_string(),
            (None, false) => String::new(),
        };
        // What a line is about goes after it, and before it where it is
        // what the line is known by: a param's name.
        let (before, after) = match &line.key {
            Some(key) if leads => (format!("{key}: "), String::new()),
            Some(key) => (String::new(), format!(" [{key}]")),
            None => (String::new(), String::new()),
        };
        out.push_str(&format!("{}{lead}{before}{}{after}\n", "  ".repeat(depth), line.text));
        // What a thing made is made of, under it: each of its parts, and what is said of the whole.
        if let Some(model) = &line.model {
            let pad = "  ".repeat(depth + 1);
            for said in
                model.as_said.iter().chain(model.fields.iter().map(|field| &field.text)).chain(&model.said)
            {
                out.push_str(&format!("{pad}| {said}\n"));
            }
        }
        // What is under a handler or a task is its steps, one after
        // another; what is under anything else is said in no order.
        written(&line.under, line.at.is_some(), leads, depth + 1, out);
    }
}

/// A summary as plain text, for a terminal: each part under its heading,
/// what a line is about after it in brackets (a step's key, a handler's
/// name, a scope), a param's name before its own, and `!` before a line
/// that says a command is run or a secret is reached.
pub fn plain(summary: &Summary) -> String {
    let mut out = format!("{}\n", summary.name);
    for part in &summary.parts {
        out.push_str(&format!("\n{}\n", part.title));
        written(&part.lines, part.ordered, part.kind == "is", 1, &mut out);
    }
    out
}
