//! The commands of the command line, their flags and the topics of its
//! help, as values: what `help --json` prints and what the pages written
//! out of the program say of each command.
//!
//! The commands are a list ([`COMMANDS`]), held by a test of the command
//! line's suite to the words its `main` answers. What each is typed as,
//! and the flags it takes, are read from what the help already says of it
//! ([`commands`]): every line of the overview and of every topic that
//! writes the command out, as `help` writes it. So a flag is said once,
//! where a person reads it, and what is said of it here cannot come apart
//! from that; a test of the command line's suite holds each flag said of a
//! command to being one that command reads.

use serde::Serialize;
use serde_json::{Value, json};

/// Every command, by the word it is typed as, with the other words that
/// are the same command (`projects` for `project`).
pub const COMMANDS: &[(&str, &[&str])] = &[
    ("up", &[]),
    ("serve", &[]),
    ("down", &[]),
    ("open", &[]),
    ("status", &[]),
    ("version", &[]),
    ("help", &[]),
    ("docs", &[]),
    ("repack", &[]),
    ("apart", &[]),
    ("profiles", &["env", "environments"]),
    ("names", &["name"]),
    ("register", &[]),
    ("login", &[]),
    ("logout", &[]),
    ("whoami", &[]),
    ("key", &[]),
    ("keys", &[]),
    ("give", &[]),
    ("group", &["groups"]),
    ("flag", &["flags"]),
    ("invite", &[]),
    ("tier", &[]),
    ("project", &["projects"]),
    ("file", &["files"]),
    ("workspace", &["workspaces"]),
    ("fleet", &[]),
    ("check", &[]),
    ("explain", &[]),
    ("sandbox", &[]),
    ("graph", &[]),
    ("schema", &[]),
    ("fmt", &[]),
    ("apply", &[]),
    ("instances", &[]),
    ("instance", &[]),
    ("stop", &[]),
    ("texts", &[]),
    ("own", &[]),
    ("runs", &[]),
    ("run", &[]),
    ("logs", &[]),
    ("actions", &[]),
    ("example", &["examples"]),
    ("channel", &["channels"]),
    ("read", &[]),
    ("post", &[]),
    ("attachment", &[]),
    ("inbox", &[]),
    ("to", &[]),
    ("ask", &[]),
    ("asked", &[]),
    ("approve", &[]),
    ("decline", &[]),
    ("board", &[]),
    ("record", &["records"]),
    ("topic", &["topics"]),
    ("follow", &[]),
    ("unfollow", &[]),
    ("following", &[]),
    ("trash", &[]),
    ("agent", &["agents"]),
    ("signin", &["signins"]),
    ("extension", &["extensions"]),
    ("search", &[]),
    ("events", &[]),
    ("emit", &[]),
    ("metrics", &[]),
    ("compare", &["comparisons"]),
    ("services", &[]),
    ("secret", &["secrets"]),
    ("caller", &["callers"]),
    ("device", &["devices"]),
    ("store", &[]),
    ("credential", &[]),
];

/// Every topic of `help`, each with the other words that ask for it.
/// Held by a test of the command line's suite to the topics the help
/// answers, word for word.
pub const TOPICS: &[&[&str]] = &[
    &["blueprints", "blueprint"],
    &["views", "view"],
    &["stores", "store"],
    &["extensions", "extension"],
    &["record", "records"],
    &["topic", "topics"],
    &["apply"],
    &["check"],
    &["sandbox"],
    &["cost", "costs", "spent"],
    &["explain"],
    &["schema"],
    &["workers", "worker", "threads"],
    &["fmt"],
    &["trash", "restore"],
    &["logs", "retention"],
    &["down", "drain"],
    &["play", "handed"],
    &["actions", "action"],
    &["events"],
    &["secret", "secrets"],
    &["emit"],
    &["caller", "callers"],
    &["metrics"],
    &["own"],
    &["texts"],
    &["compare", "comparisons"],
    &["docs", "version"],
    &["profile", "profiles", "env"],
    &["apart"],
    &["repack"],
    &["agent", "agents"],
    &["wall", "walls"],
    &["web"],
    &["outlives", "restart", "restarts"],
    &["signin", "signins"],
    &["device", "devices"],
    &["key", "keys", "give"],
    &["group", "groups"],
    &["flag", "flags"],
    &["channel", "channels", "speaker", "post", "read", "thread", "threads"],
    &["inbox", "to", "conversation", "conversations"],
    &["follow", "unfollow", "following"],
    &["ask", "asked", "approve", "decline"],
    &["names", "name"],
    &["git"],
    &["environment", "environments"],
    &["project", "projects"],
    &["example", "examples"],
    &["workspace", "workspaces"],
    &["up", "serve"],
    &["open"],
    &["status"],
    &["register", "login", "logout", "whoami"],
    &["invite", "join", "leave", "external"],
    &["file", "files"],
];

/// The flags every command takes: which daemon, as whom, how it is said,
/// and how many threads the program works on. `--env` is the old spelling
/// of `--profile`, and is taken without being said.
pub const EVERYWHERE: &[&str] = &["url", "profile", "env", "token", "as", "json", "workers", "help"];

/// One command as the help says it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Command {
    pub name: String,
    /// The other words that are the same command.
    pub also: Vec<String>,
    /// Each way it is written out, as the help writes it, with no name of
    /// the program in front.
    pub forms: Vec<String>,
    /// The flags it takes beside [`EVERYWHERE`], by name and without `--`.
    pub flags: Vec<String>,
    /// The topic of `help` that is about it, where one is.
    pub topic: Option<String>,
}

/// The command a word is, by its name: one of [`COMMANDS`] or one of
/// their other words. `alias` says whether an other word counts.
fn command_of(word: &str, alias: bool) -> Option<&'static str> {
    COMMANDS
        .iter()
        .find(|(name, also)| *name == word || (alias && also.contains(&word)))
        .map(|(name, _)| *name)
}

/// The topic a word asks for, by its first name.
pub fn topic_of(word: &str) -> Option<&'static str> {
    TOPICS.iter().find(|names| names.contains(&word)).map(|names| names[0])
}

/// What a line writes out of a command: everything before the first
/// stretch of three spaces, which is where what it says of it begins,
/// and without a topic of `help` it points to at its end
/// (`... [--web] (help agent, wall)`), which is no part of the command.
fn usage(line: &str) -> &str {
    let written = line.find("   ").map_or(line, |at| &line[..at]).trim();
    match written.rfind(" (help ") {
        Some(at) if written.ends_with(')') => written[..at].trim_end(),
        _ => written,
    }
}

/// Every flag written in `text`, by name.
fn flags_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("--") {
        let after = &rest[at + 2..];
        let before = rest[..at].chars().last();
        let name: String =
            after.chars().take_while(|letter| letter.is_ascii_lowercase() || *letter == '-').collect();
        let begins = name.chars().next().is_some_and(|letter| letter.is_ascii_lowercase());
        if begins && !before.is_some_and(|letter| letter.is_alphanumeric() || letter == '-') {
            found.push(name.trim_end_matches('-').to_string());
        }
        rest = &after[name.len().min(after.len())..];
    }
    found
}

/// The flags of [`EVERYWHERE`] that are followed by a value.
const EVERYWHERE_VALUED: &[&str] = &["url", "profile", "env", "token", "as", "workers"];

/// What is written after the program's name, without the flags every
/// command takes that stand before the command's own word.
pub fn after_everywhere(written: &str) -> &str {
    let mut rest = written.trim_start();
    loop {
        let Some(flag) = rest.strip_prefix("--") else { return rest };
        let (word, after) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        let name = flag.split(['=', ' ']).next().unwrap_or_default();
        if !EVERYWHERE.contains(&name) {
            return rest;
        }
        rest = after.trim_start();
        if EVERYWHERE_VALUED.contains(&name) && !word.contains('=') {
            rest = rest.split_once(char::is_whitespace).map_or("", |(_, after)| after).trim_start();
        }
    }
}

/// The command a line of a terminal names, after the program's name: by
/// its own word or one of its others, past what every command takes.
pub fn named(written: &str) -> Option<&'static str> {
    command_of(after_everywhere(written).split_whitespace().next()?, true)
}

/// `usage` cut where it says ` | ` outside brackets: each way a command
/// is written, `[--bare | --without board]` staying one.
fn ways(usage: &str) -> Vec<&str> {
    let (mut found, mut depth, mut from) = (Vec::new(), 0i32, 0);
    let bytes = usage.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'[' => depth += 1,
            b']' => depth -= 1,
            b' ' if depth <= 0 && usage[at..].starts_with(" | ") => {
                found.push(&usage[from..at]);
                from = at + 3;
                at += 2;
            }
            _ => {}
        }
        at += 1;
    }
    found.push(&usage[from..]);
    found
}

/// What is written out of commands, read: `(command, form)` for each
/// form. In the overview (`writes` empty) a line is one or more commands,
/// each way after the first that does not begin with a command being one
/// more of the command before it, and a line that begins with what a
/// command takes goes on from the line before. In a topic a line writes a
/// command out where it begins with the program's name (`writes`), and
/// what is after a ` | ` is that command again: a topic writes another
/// command on a line of its own, since what is after a ` | ` there may be
/// a word of the line's command that is also a command (`device list |
/// invite <name>`).
fn read(text: &str, writes: &str) -> Vec<(&'static str, String)> {
    let mut found: Vec<(&'static str, String)> = Vec::new();
    let mut last: Option<&'static str> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let (written, goes_on) = if writes.is_empty() {
            // Headings and sentences begin at the margin.
            if line.len() == trimmed.len() || trimmed.is_empty() {
                last = None;
                continue;
            }
            (trimmed, false)
        } else if let Some(rest) = trimmed.strip_prefix(writes) {
            // A command written out may begin with what every command
            // takes (`mw --profile try apply ...`); one that names no
            // command is something else written after the program's name.
            let rest = after_everywhere(rest);
            if named(rest).is_none() {
                last = None;
                continue;
            }
            (rest, false)
        } else if (trimmed.starts_with("[--") || trimmed.starts_with("--")) && last.is_some() {
            (trimmed, true)
        } else {
            last = None;
            continue;
        };
        for (place, piece) in ways(usage(written)).into_iter().enumerate() {
            let piece = piece.trim();
            let first = piece.split_whitespace().next().unwrap_or_default();
            let word = first.split('|').next().unwrap_or_default();
            // The first word of a line names its command by any of its
            // words; one after it, in the overview, only by its own name,
            // since an other word there is usually what the command before
            // is asked (`secret agents`).
            let command = match (goes_on, place, writes.is_empty()) {
                (true, _, _) => None,
                (false, 0, _) => command_of(word, true),
                (false, _, true) => command_of(word, false),
                // In a topic, the line's command written again.
                (false, _, false) => command_of(word, true).filter(|command| Some(*command) == last),
            };
            match (command, last) {
                (Some(command), _) => {
                    last = Some(command);
                    found.push((command, piece.to_string()));
                }
                (None, Some(command)) if !piece.is_empty() => {
                    let before =
                        found.last().filter(|(before, _)| *before == command).map(|(_, form)| form.clone());
                    match before {
                        // What goes on from the way before it.
                        Some(_) if goes_on || piece.starts_with('[') => {
                            if let Some((_, form)) = found.last_mut() {
                                form.push(' ');
                                form.push_str(piece);
                            }
                        }
                        // Another way of what the way before it is asked
                        // (`rule <name> --grows ... | --none`).
                        Some(form) if piece.starts_with('-') => {
                            let asked: Vec<&str> = form
                                .split_whitespace()
                                .take_while(|word| !word.starts_with(['[', '-']))
                                .collect();
                            found.push((command, format!("{} {piece}", asked.join(" "))));
                        }
                        _ => found.push((command, format!("{command} {piece}"))),
                    }
                }
                _ => {}
            }
        }
    }
    found
}

/// Every command, as the help says it: its forms from the overview and
/// from every topic, and the flags written in those, with the flags a
/// topic about it explains (`--first NAME   apply ...`).
pub fn commands() -> Vec<Command> {
    let cli = format!("{} ", brand::CLI);
    let mut forms: Vec<(&'static str, String)> = read(&crate::help::overview(), "");
    let mut explained: Vec<(&'static str, String)> = Vec::new();
    for names in TOPICS {
        let Some(text) = crate::help::topic(names[0]) else { continue };
        forms.extend(read(&text, &cli));
        let about = names.iter().find_map(|name| command_of(name, true));
        if let Some(command) = about {
            for line in text
                .lines()
                .map(str::trim_start)
                .filter(|line| line.starts_with("--") && line.contains("   "))
            {
                explained.extend(flags_in(usage(line)).into_iter().map(|flag| (command, flag)));
            }
        }
    }
    COMMANDS
        .iter()
        .map(|(name, also)| {
            let mut written: Vec<String> = Vec::new();
            let mut flags: Vec<String> = Vec::new();
            for (command, form) in &forms {
                if command != name {
                    continue;
                }
                if !written.contains(form) {
                    written.push(form.clone());
                }
                flags.extend(flags_in(form));
            }
            flags.extend(
                explained.iter().filter(|(command, _)| command == name).map(|(_, flag)| flag.clone()),
            );
            flags.retain(|flag| !EVERYWHERE.contains(&flag.as_str()));
            flags.sort();
            flags.dedup();
            let topic = std::iter::once(*name).chain(also.iter().copied()).find_map(topic_of);
            Command {
                name: name.to_string(),
                also: also.iter().map(|word| word.to_string()).collect(),
                forms: written,
                flags,
                topic: topic.map(str::to_string),
            }
        })
        .collect()
}

/// What `help --json` prints: this program's version, every command with
/// its forms and flags, the flags every command takes, and every topic.
pub fn described() -> Value {
    let topics: Vec<Value> =
        TOPICS.iter().map(|names| json!({ "name": names[0], "also": &names[1..] })).collect();
    json!({
        "program": brand::CLI,
        "version": crate::VERSION,
        "commands": commands(),
        "everywhere": EVERYWHERE,
        "topics": topics,
    })
}
