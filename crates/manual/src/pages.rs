//! Everything the program says of itself, as pages of Markdown that a
//! library reads and a site is written from: every command and topic of
//! its help, every keyword of the language, every action and event, the
//! summary of every blueprint that ships, the release notes and the
//! public guides, with an index that lists and links all of them.
//!
//! It is a function of plain values ([`pages`]): what is not the
//! language's own (the actions and events of a device, the blueprints
//! that ship) is handed in, so that the command that writes it with no
//! daemon and the action a daemon runs come to the same pages. Every page
//! says `public: true` and this program's version at its head, and
//! nothing on one is fetched or run. Which pages there are is held to a
//! kept list by a test of the API suite (`contract/docs.txt`), so a page
//! somebody links to is not taken away unnoticed.

use std::collections::BTreeMap;

use blueprint::summary::{Known, plain, summary};
use blueprint::vocabulary::{self, Field, Shape};
use blueprint::{ActionSpec, EventSpec};

use crate::commands::{Command, EVERYWHERE, TOPICS, commands};
use crate::guides::{GUIDES, body};

/// The head every page is written with.
fn head() -> String {
    format!("---\npublic: true\nversion: {}\n---\n\n", crate::VERSION)
}

/// A fence that nothing in `text` closes, around it.
fn fenced(text: &str, kind: &str) -> String {
    let mut fence = String::from("```");
    while text.contains(fence.as_str()) {
        fence.push('`');
    }
    format!("{fence}{kind}\n{}\n{fence}\n", text.trim_end())
}

/// The first heading of a Markdown text, or `fallback`.
fn heading(text: &str, fallback: &str) -> String {
    body(text)
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .map_or_else(|| fallback.to_string(), |title| title.trim().to_string())
}

/// What a page of each kind is called, from the top of what is written.
pub fn command_page(name: &str) -> String {
    format!("commands/{name}.md")
}
pub fn topic_page(name: &str) -> String {
    format!("help/{name}.md")
}
pub fn guide_page(name: &str) -> String {
    format!("guides/{name}.md")
}
pub fn keyword_page(shape: &str) -> String {
    format!("keywords/{shape}.md")
}
pub fn action_page(name: &str) -> String {
    format!("actions/{name}.md")
}
pub fn event_page(topic: &str) -> String {
    format!("events/{topic}.md")
}
pub fn blueprint_page(name: &str) -> String {
    format!("blueprints/{name}.md")
}

/// What is handed in that the language does not say itself.
pub struct Device<'a> {
    /// Every action, the language's own among them.
    pub actions: &'a [ActionSpec],
    pub events: &'a [EventSpec],
    /// The blueprints that ship, by name, with their texts.
    pub blueprints: &'a [(String, String)],
}

/// Every page, by where it is from the top of the folder, with what it
/// says.
pub fn pages(device: &Device) -> BTreeMap<String, String> {
    let mut written = BTreeMap::new();
    let mut index = Index::default();
    let cli = brand::CLI;

    for (name, text) in GUIDES {
        let title = heading(text, name);
        written.insert(guide_page(name), format!("{}{}", head(), body(text)));
        index.guides.push((title, guide_page(name)));
    }

    for command in commands() {
        written.insert(command_page(&command.name), command_text(&command));
        index.commands.push((format!("{cli} {}", command.name), command_page(&command.name)));
    }

    for names in TOPICS {
        let Some(text) = crate::help::topic(names[0]) else { continue };
        written.insert(topic_page(names[0]), topic_text(names, &text));
        index.topics.push((format!("{cli} help {}", names[0]), topic_page(names[0])));
    }

    for shape in vocabulary::SHAPES {
        written.insert(keyword_page(shape.name), shape_text(shape));
        for field in shape.fields {
            index.keywords.push((format!("{}: {}", shape.name, field.name), keyword_page(shape.name)));
        }
    }
    written.insert(keyword_page("names"), names_text());
    index.keywords.push(("what an expression begins with".into(), keyword_page("names")));
    written.insert(keyword_page("functions"), functions_text());
    index.keywords.push(("the functions of an expression".into(), keyword_page("functions")));

    let mut actions: Vec<&ActionSpec> = device.actions.iter().collect();
    actions.sort_by(|a, b| a.name.cmp(&b.name));
    actions.dedup_by(|a, b| a.name == b.name);
    for action in actions {
        written.insert(action_page(&action.name), action_text(action));
        index.actions.push((action.name.clone(), action_page(&action.name)));
    }

    let mut events: Vec<&EventSpec> = device.events.iter().collect();
    events.sort_by(|a, b| a.topic.cmp(&b.topic));
    events.dedup_by(|a, b| a.topic == b.topic);
    for event in events {
        written.insert(event_page(&event.topic), event_text(event));
        index.events.push((event.topic.clone(), event_page(&event.topic)));
    }

    let known = Known { actions: device.actions, events: device.events, scopes: kernel::principal::SCOPES };
    let mut blueprints: Vec<&(String, String)> = device.blueprints.iter().collect();
    blueprints.sort();
    for (name, text) in blueprints {
        written.insert(blueprint_page(name), blueprint_text(name, text, &known));
        index.blueprints.push((name.clone(), blueprint_page(name)));
    }

    written.insert("releases.md".into(), format!("{}{}", head(), body(crate::RELEASES)));
    written.insert("index.md".into(), index.text());
    written
}

/// What the index lists, a heading each.
#[derive(Default)]
struct Index {
    guides: Vec<(String, String)>,
    commands: Vec<(String, String)>,
    topics: Vec<(String, String)>,
    keywords: Vec<(String, String)>,
    actions: Vec<(String, String)>,
    events: Vec<(String, String)>,
    blueprints: Vec<(String, String)>,
}

impl Index {
    fn text(&self) -> String {
        let (product, cli) = (brand::DISPLAY, brand::CLI);
        let mut text = format!(
            "{}# {product} {version}\n\nEverything {product} {version} says of itself: its guides, every command \
             of `{cli}` and every topic of its help, every word of the language blueprints are written in, \
             every action and event, every blueprint that ships, and what each release was. Written out by \
             the program itself (`{cli} docs export`), so that it is what this version does.\n",
            head(),
            version = crate::VERSION,
        );
        let parts: [(&str, &[(String, String)]); 7] = [
            ("Guides", &self.guides),
            ("Commands", &self.commands),
            ("Help", &self.topics),
            ("Keywords", &self.keywords),
            ("Actions", &self.actions),
            ("Events", &self.events),
            ("Blueprints that ship", &self.blueprints),
        ];
        for (title, listed) in parts {
            text.push_str(&format!("\n## {title}\n\n"));
            for (called, page) in listed {
                text.push_str(&format!("- [{called}]({page})\n"));
            }
        }
        text.push_str("\n## Releases\n\n- [What each release was](releases.md)\n");
        text
    }
}

fn flags(names: &[String]) -> String {
    names.iter().map(|name| format!("`--{name}`")).collect::<Vec<_>>().join(", ")
}

fn command_text(command: &Command) -> String {
    let cli = brand::CLI;
    let mut text = format!("{}# {cli} {}\n\n", head(), command.name);
    if !command.also.is_empty() {
        let also: Vec<String> = command.also.iter().map(|word| format!("`{cli} {word}`")).collect();
        text.push_str(&format!("Also typed {}.\n\n", also.join(", ")));
    }
    if !command.forms.is_empty() {
        let forms: Vec<String> = command.forms.iter().map(|form| format!("{cli} {form}")).collect();
        text.push_str(&format!("## How it is written\n\n{}\n", fenced(&forms.join("\n"), "console")));
    }
    if !command.flags.is_empty() {
        text.push_str(&format!("## Its flags\n\n{}\n\n", flags(&command.flags)));
    }
    let everywhere: Vec<String> =
        EVERYWHERE.iter().filter(|name| **name != "env").map(|name| name.to_string()).collect();
    text.push_str(&format!("And what every command takes: {}.\n", flags(&everywhere)));
    if let Some(topic) = &command.topic {
        text.push_str(&format!(
            "\n`{cli} help {topic}` says more of it: [{cli} help {topic}](../{}).\n",
            topic_page(topic)
        ));
    }
    text
}

fn topic_text(names: &[&str], said: &str) -> String {
    let cli = brand::CLI;
    let mut text = format!("{}# {cli} help {}\n\n", head(), names[0]);
    if names.len() > 1 {
        let also: Vec<String> = names[1..].iter().map(|word| format!("`{cli} help {word}`")).collect();
        text.push_str(&format!("Also asked as {}.\n\n", also.join(", ")));
    }
    // A topic that is a guide says where the guide is, not the guide again.
    match GUIDES.iter().find(|(_, guide)| body(guide) == said) {
        Some((guide, written)) => text.push_str(&format!(
            "It prints the guide [{}](../{}).\n",
            heading(written, guide),
            guide_page(guide)
        )),
        None => text.push_str(&fenced(said, "text")),
    }
    text
}

fn example(text: &str) -> String {
    format!("Written in a whole blueprint:\n\n{}\n", fenced(text, "yaml"))
}

fn field_text(field: &Field) -> String {
    let mut text = format!("## `{}`\n\n{}: {}\n\n", field.name, vocabulary::after(field.value), field.help);
    if let Some(shown) = field.example {
        text.push_str(&example(shown));
    }
    text
}

fn shape_text(shape: &Shape) -> String {
    let mut text = format!("{}# `{}`: {}\n\n{}\n\n", head(), shape.name, shape.what, shape.help);
    if let Some(short) = shape.short {
        text.push_str(&format!("In its place, {} may be written alone.\n\n", vocabulary::after(short)));
    }
    for group in shape.alternatives {
        let words: Vec<String> = group.iter().map(|word| format!("`{word}`")).collect();
        text.push_str(&format!("One of {} and never two.\n\n", words.join(", ")));
    }
    if let Some(shown) = shape.example {
        text.push_str(&example(shown));
    }
    for field in shape.fields {
        text.push_str(&field_text(field));
    }
    text
}

fn names_text() -> String {
    let mut text = format!("{}# What an expression begins with\n\n", head());
    for root in blueprint::ROOTS {
        text.push_str(&format!("## `{}`\n\n{}\n\n", root.name, root.meaning));
        for member in vocabulary::members(root.name) {
            text.push_str(&format!("- `{}.{}`: {}\n", root.name, member.name, member.help));
        }
        text.push('\n');
        text.push_str(&example(root.example));
    }
    text
}

fn functions_text() -> String {
    let mut text = format!("{}# The functions of an expression\n\n", head());
    for (name, usage) in blueprint::expr::FUNCTIONS {
        text.push_str(&format!("## `{name}`\n\n{usage}\n\n"));
        if let Some((_, shown)) = vocabulary::CALLED.iter().find(|(called, _)| called == name) {
            text.push_str(&example(shown));
        }
    }
    text
}

fn action_text(action: &ActionSpec) -> String {
    let mut text = format!("{}# `{}`\n\n{}\n\n", head(), action.name, action.summary);
    if let Some(needs) = &action.capability {
        text.push_str(&format!("A blueprint that asks it holds `{needs}`.\n\n"));
    }
    if let Some(makes) = &action.makes {
        text.push_str(&format!("It makes {}, called by `{}`.\n\n", makes.what, makes.named));
    }
    if !action.runs.is_empty() {
        let runs: Vec<String> = action.runs.iter().map(|arg| format!("`{arg}`")).collect();
        text.push_str(&format!("It runs a command on the device: {}.\n\n", runs.join(", ")));
    }
    if action.idempotent {
        text.push_str("Done again after a stop in the middle of it, it leaves nothing twice.\n\n");
    }
    if !action.args.is_empty() || action.open {
        text.push_str("## What it is handed\n\n");
        for arg in &action.args {
            let needed = if arg.required { ", needed" } else { "" };
            text.push_str(&format!("- `{}` ({}{needed}): {}\n", arg.name, arg.ty.name(), arg.help));
        }
        if action.open {
            text.push_str("- and anything else, as its own\n");
        }
        text.push('\n');
    }
    if !action.outputs.is_empty() {
        text.push_str("## What it gives\n\n");
        for (field, meaning) in &action.outputs {
            text.push_str(&format!("- `{field}`: {meaning}\n"));
        }
        text.push('\n');
    }
    if let Some(shown) = &action.example {
        text.push_str(&example(shown));
    }
    text
}

fn event_text(event: &EventSpec) -> String {
    let mut text = format!("{}# `{}`\n\n{}\n\n", head(), event.topic, event.summary);
    text.push_str(if event.kept {
        "It is kept in the journal, and a handler hears it.\n\n"
    } else {
        "It is only passed on to whoever is following as it happens: it is not kept, and no handler hears it.\n\n"
    });
    match event.hears.as_deref() {
        Some("") => text.push_str("Anybody with a credential may be told of it.\n\n"),
        Some(scope) => text.push_str(&format!("Whoever holds `{scope}` may be told of it.\n\n")),
        None => {}
    }
    if !event.fields.is_empty() || event.open {
        text.push_str("## What it carries\n\n");
        for field in &event.fields {
            let always = if field.always { ", always" } else { "" };
            text.push_str(&format!("- `{}` ({}{always}): {}\n", field.name, field.ty.name(), field.help));
        }
        if event.open {
            text.push_str("- and more besides, whose names are its own\n");
        }
        text.push('\n');
    }
    text
}

fn blueprint_text(name: &str, source: &str, known: &Known) -> String {
    let cli = brand::CLI;
    let mut text = format!("{}# `{name}`\n\n", head());
    let checked = blueprint::check(source, &blueprint::NoCatalog);
    let Some(read) = checked.blueprint.as_ref().filter(|_| !checked.has_errors()) else {
        text.push_str("It does not read as a blueprint here, so there is nothing to say of it.\n");
        return text;
    };
    if let Some(description) = &read.description {
        text.push_str(&format!("{description}\n\n"));
    }
    if let Some(version) = &read.version {
        text.push_str(&format!("Version {version}. `{cli} explain {name}` says the same of it.\n\n"));
    }
    if !read.params.is_empty() {
        text.push_str("## What it may be told\n\n");
        for param in &read.params {
            let needed = if param.required { ", needed" } else { "" };
            let default =
                param.default.as_ref().map(|value| format!(", `{value}` unless said")).unwrap_or_default();
            let said = param.description.as_deref().unwrap_or_default();
            text.push_str(&format!("- `{}` ({}{needed}{default}): {said}\n", param.name, param.ty.name()));
        }
        text.push('\n');
    }
    text.push_str(&format!("## In plain words\n\n{}", fenced(&plain(&summary(read, known)), "text")));
    text
}

/// What the kept list of pages says of `pages` (`contract/docs.txt`): the
/// head every page is written with, and each page by where it is.
pub fn listed(pages: &BTreeMap<String, String>) -> String {
    let mut text = String::from(
        "# The pages the program writes out of itself (`docs export`, `docs.export`), each by where it\n\
         # is from the top of the folder, and the head every page says. Written by a test, never by\n\
         # hand: a page added is written down by\n\
         #   WRITE_DOCS=1 cargo test -p api the_docs\n\
         # and one taken away, which somebody may link to, only by saying why:\n\
         #   BREAK_DOCS=\"why\" cargo test -p api the_docs\n\
         # which keeps the reason in docs-broken.txt.\n\nhead public version\n\n",
    );
    for path in pages.keys() {
        text.push_str(&format!("page {path}\n"));
    }
    text
}
