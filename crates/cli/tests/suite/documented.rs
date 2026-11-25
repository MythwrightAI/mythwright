//! What the program says of itself, held to what it is: `help --json`
//! names every command it answers and every topic its help has, each flag
//! said of a command is one that command reads, every command a public
//! guide (and `AGENTS.md`) writes out is one it would read, and `docs
//! export` writes the pages a library reads, with no daemon asked.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::{run_in, scratch};

/// Nothing answers here: what is refused for a flag is refused before
/// anything is sent.
const NOWHERE: &str = "http://127.0.0.1:1";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

async fn described(dir: &Path) -> Value {
    let said = run_in(dir, NOWHERE, &[], &["help", "--json"]).await.ok();
    serde_json::from_str(&said).expect("help --json prints JSON")
}

/// Every word typed as a command, by the command it is.
fn words(described: &Value) -> BTreeMap<String, String> {
    let mut words = BTreeMap::new();
    for command in described["commands"].as_array().unwrap() {
        let name = command["name"].as_str().unwrap().to_string();
        for also in command["also"].as_array().unwrap() {
            words.insert(also.as_str().unwrap().to_string(), name.clone());
        }
        words.insert(name.clone(), name);
    }
    words
}

/// The quoted words a match arm of `text` answers (`"a" | "b" =>`, with a
/// guard or without), each arm's words together.
fn arms(text: &str) -> Vec<Vec<String>> {
    let mut found = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('"') || !line.contains("=>") {
            continue;
        }
        let pattern = line.split("=>").next().unwrap();
        let pattern = pattern.split(" if ").next().unwrap();
        let said: Vec<String> = pattern
            .split('|')
            .map(|word| word.trim().trim_matches('"').to_string())
            .filter(|word| !word.is_empty())
            .collect();
        if said.iter().all(|word| word.chars().all(|letter| letter.is_ascii_lowercase() || letter == '-')) {
            found.push(said);
        }
    }
    found
}

#[tokio::test]
async fn help_json_names_every_command_this_program_answers_and_no_other() {
    let dir = scratch("documented-commands");
    let described = described(&dir).await;
    assert_eq!(described["version"], manual::VERSION);
    let main = std::fs::read_to_string(repo().join("crates/cli/src/main.rs")).unwrap();
    let act = &main[main.find("async fn act(").unwrap()..main.find("fn usage(").unwrap()];
    let answered: BTreeSet<String> = arms(act).into_iter().flatten().collect();
    let said: BTreeSet<String> = words(&described).into_keys().collect();
    assert_eq!(answered, said, "the words `main` answers as commands, and those `help --json` names");
    for command in described["commands"].as_array().unwrap() {
        let name = command["name"].as_str().unwrap();
        // A command nobody can read how to write is no command for a person.
        if !["credential", "tier"].contains(&name) {
            assert!(!command["forms"].as_array().unwrap().is_empty(), "`help` writes `{name}` out nowhere");
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn help_json_names_every_topic_the_help_answers_and_each_answers() {
    let dir = scratch("documented-topics");
    let described = described(&dir).await;
    let mut written = BTreeSet::new();
    for file in ["help.rs", "help_copies.rs", "help_first.rs", "help_who.rs"] {
        let text = std::fs::read_to_string(repo().join("crates/manual/src").join(file)).unwrap();
        let topic = &text[text.find("pub fn topic(").unwrap()..];
        written.extend(arms(topic));
    }
    let said: BTreeSet<Vec<String>> = described["topics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|topic| {
            let mut names = vec![topic["name"].as_str().unwrap().to_string()];
            names.extend(
                topic["also"].as_array().unwrap().iter().map(|name| name.as_str().unwrap().to_string()),
            );
            names
        })
        .collect();
    assert_eq!(written, said, "the topics the help's sources answer, and those `help --json` names");
    for names in &said {
        for name in names {
            assert!(manual::topic(name).is_some(), "`help {name}` says nothing");
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// What the head of a guide says of being public.
fn public(text: &str) -> bool {
    text.starts_with("<!-- public -->\n")
        || text
            .strip_prefix("---\n")
            .and_then(|after| after.split_once("\n---\n"))
            .is_some_and(|(head, _)| head.lines().any(|line| line.replace(' ', "") == "public:true"))
}

#[test]
fn the_guides_the_program_carries_are_those_that_say_they_are_public() {
    let mut marked = BTreeSet::new();
    for entry in std::fs::read_dir(repo().join("docs")).unwrap() {
        let path = entry.unwrap().path();
        let Some(name) =
            path.file_name().and_then(|name| name.to_str()).and_then(|name| name.strip_suffix(".md"))
        else {
            continue;
        };
        if public(&std::fs::read_to_string(&path).unwrap()) {
            marked.insert(name.to_string());
        }
    }
    // The release notes are carried apart, as what each release was.
    let mut carried: BTreeSet<String> =
        manual::guides::GUIDES.iter().map(|(name, _)| name.to_string()).collect();
    carried.insert("releases".into());
    assert!(public(manual::RELEASES));
    assert_eq!(marked, carried, "the guides of docs/ that say `public: true`, and those the program carries");
    // The design records stay the factory's own.
    for record in ["architecture", "build-times", "chain", "roadmap"] {
        assert!(!marked.contains(record), "docs/{record}.md is a design record, and no public guide");
    }
}

/// Each line of a block of code in `text` that writes a command of the
/// program out, a line that goes on with `\` joined to the next, with
/// the line it begins on.
fn written_out(text: &str) -> Vec<(usize, String)> {
    let program = format!("{} ", brand::CLI);
    let mut found: Vec<(usize, String)> = Vec::new();
    let mut going: Option<(usize, String)> = None;
    let mut fenced = false;
    for (at, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if !fenced {
            continue;
        }
        if let Some((begun, mut so_far)) = going.take() {
            so_far.push(' ');
            so_far.push_str(trimmed.trim_end_matches('\\'));
            match trimmed.ends_with('\\') {
                true => going = Some((begun, so_far)),
                false => found.push((begun, so_far)),
            }
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("$ ").unwrap_or(trimmed).strip_prefix(&program) else {
            continue;
        };
        match rest.ends_with('\\') {
            true => going = Some((at + 1, rest.trim_end_matches('\\').to_string())),
            false => found.push((at + 1, rest.to_string())),
        }
    }
    found
}

/// What a line of a shell gives the program: up to a comment, or what
/// hands its output on, with what is quoted taken as one word.
fn given(line: &str) -> Vec<String> {
    let (mut words, mut word, mut quote) = (Vec::new(), String::new(), None);
    for letter in line.chars() {
        match (quote, letter) {
            (Some(open), _) if letter == open => quote = None,
            (Some(_), _) => word.push(letter),
            (None, '"' | '\'') => quote = Some(letter),
            (None, letter) if letter.is_whitespace() => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            (None, letter) => word.push(letter),
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
        .into_iter()
        .take_while(|word| !["#", "|", "&&", "||", ">", ";", "2>&1"].contains(&word.as_str()))
        .collect()
}

#[tokio::test]
async fn every_command_a_public_guide_writes_out_is_one_the_command_line_reads() {
    let dir = scratch("documented-guides");
    let described = described(&dir).await;
    let words = words(&described);
    let everywhere: BTreeSet<String> = described["everywhere"]
        .as_array()
        .unwrap()
        .iter()
        .map(|flag| flag.as_str().unwrap().to_string())
        .collect();
    let takes: BTreeMap<String, BTreeSet<String>> = described["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|command| {
            let flags =
                command["flags"].as_array().unwrap().iter().map(|flag| flag.as_str().unwrap().to_string());
            (command["name"].as_str().unwrap().to_string(), flags.collect())
        })
        .collect();
    let mut guides: Vec<(String, &str)> =
        manual::guides::GUIDES.iter().map(|(name, text)| (format!("docs/{name}.md"), *text)).collect();
    guides.push(("docs/releases.md".into(), manual::RELEASES));
    // What an agent asked to set the program up reads first, which no copy
    // of the program carries: held from the checkout all the same.
    let for_agents = std::fs::read_to_string(repo().join("AGENTS.md")).unwrap();
    guides.push(("AGENTS.md".into(), &for_agents));
    let (mut wrong, mut read) = (Vec::new(), 0);
    for (guide, text) in &guides {
        for (line, written) in written_out(text) {
            read += 1;
            let words_given = given(&written);
            let rest = manual::commands::after_everywhere(&written);
            let Some(command) = rest.split_whitespace().next().and_then(|word| words.get(word)) else {
                wrong.push(format!("{guide}:{line}: `{written}` is no command of `{}`", brand::CLI));
                continue;
            };
            for word in &words_given {
                let Some(flag) = word.strip_prefix("--") else { continue };
                let flag = flag.split('=').next().unwrap_or_default();
                if flag.is_empty() || everywhere.contains(flag) || takes[command].contains(flag) {
                    continue;
                }
                wrong.push(format!("{guide}:{line}: `{}` takes no `--{flag}` (`{written}`)", command));
            }
        }
    }
    assert!(read > 50, "the public guides write out {read} commands: they are read here as they are written");
    assert!(
        wrong.is_empty(),
        "what the public guides write out that the command line would not read:\n{}",
        wrong.join("\n")
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Whether `form` writes `--flag` with something after it: `--at <step>`,
/// `--expires 3d`, `--why=W`, and not `[--force]` or `--bare |`.
fn takes_a_value(form: &str, flag: &str) -> bool {
    let words: Vec<&str> = form.split_whitespace().collect();
    words.iter().enumerate().any(|(at, word)| {
        let bare = word.trim_start_matches('[');
        if bare.starts_with(&format!("--{flag}=")) {
            return true;
        }
        if bare != format!("--{flag}") {
            return false;
        }
        words.get(at + 1).is_some_and(|next| !next.starts_with(['[', '-', '|']) && *next != "|")
    })
}

/// The commands that act where they are run, and are not run here to be
/// asked what they read: they start a daemon, open a browser, or work on
/// a factory's files.
const ACT_HERE: &[&str] =
    &["up", "serve", "open", "repack", "apart", "store", "credential", "profiles", "docs"];

/// What `form` comes to typed with nothing it leaves to be said: its
/// words, the first of each choice, `one` for each thing to be named, as
/// far as what it may be told.
fn typed(form: &str) -> Vec<String> {
    form.split_whitespace()
        .take_while(|word| !word.starts_with(['[', '-']) && !word.contains("...") && !word.contains('='))
        .map(|word| match word.starts_with('<') {
            true => "one".to_string(),
            false => word.split('|').next().unwrap().to_string(),
        })
        .collect()
}

#[tokio::test]
async fn each_flag_help_says_of_a_command_is_one_the_command_reads() {
    let dir = scratch("documented-flags");
    let described = described(&dir).await;
    let mut wrong = Vec::new();
    for command in described["commands"].as_array().unwrap() {
        let name = command["name"].as_str().unwrap();
        if ACT_HERE.contains(&name) {
            continue;
        }
        let forms: Vec<&str> =
            command["forms"].as_array().unwrap().iter().map(|form| form.as_str().unwrap()).collect();
        for flag in command["flags"].as_array().unwrap().iter().map(|flag| flag.as_str().unwrap()) {
            // Asked of each way of writing the command, those that write
            // the flag first, until one of them reads it: a flag is the
            // command's where one of its ways takes it.
            let written = format!("--{flag}");
            let mut ways: Vec<&str> = forms.iter().copied().filter(|form| form.contains(&written)).collect();
            ways.extend(forms.iter().copied().filter(|form| !form.contains(&written)));
            let mut tried: BTreeSet<Vec<String>> = BTreeSet::new();
            let mut refused = Vec::new();
            let read = 'read: {
                for form in ways.iter().copied().chain([name]) {
                    let words = typed(form);
                    if !tried.insert(words.clone()) {
                        continue;
                    }
                    let valued = forms.iter().any(|form| takes_a_value(form, flag));
                    for value in [valued, !valued] {
                        let mut asked = words.clone();
                        asked.push(written.clone());
                        if value {
                            asked.push("x".into());
                        }
                        let asked: Vec<&str> = asked.iter().map(String::as_str).collect();
                        let ran = run_in(&dir, NOWHERE, &[], &asked).await;
                        if ran.err.contains("needs a value") {
                            continue;
                        }
                        if !ran.err.contains(&format!("takes no `{written}`"))
                            && !ran.err.contains("takes no `--")
                            && !ran.err.contains("is not a command")
                        {
                            break 'read true;
                        }
                        refused.push(format!("`{}`: {}", asked.join(" "), ran.err.trim()));
                        break;
                    }
                }
                false
            };
            if !read {
                wrong.push(format!(
                    "`{name}` and `{written}`, refused each way it is written:\n  {}",
                    refused.join("\n  ")
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "flags `help` says of a command that the command does not read:\n{}",
        wrong.join("\n")
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// The forms whose second word is another command's own name, which the
/// line's command reads as a word of its own (`device invite`), and not
/// as that other command written after it.
const ITS_OWN_WORD: &[(&str, &str)] = &[
    ("agent", "invite"),
    ("agent", "stop"),
    ("apply", "fleet"),
    ("board", "give"),
    ("channel", "docs"),
    ("channel", "open"),
    ("device", "invite"),
    ("file", "read"),
    ("inbox", "read"),
    ("store", "serve"),
    ("texts", "follow"),
    ("topic", "follow"),
];

#[tokio::test]
async fn each_form_help_says_of_a_command_is_that_command_alone() {
    let dir = scratch("documented-forms");
    let described = described(&dir).await;
    let own: BTreeSet<String> = described["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|command| command["name"].as_str().unwrap().to_string())
        .collect();
    let mut wrong = Vec::new();
    for command in described["commands"].as_array().unwrap() {
        let name = command["name"].as_str().unwrap();
        for form in command["forms"].as_array().unwrap().iter().map(|form| form.as_str().unwrap()) {
            let words: Vec<&str> = form.split_whitespace().collect();
            if words.first() != Some(&name) {
                wrong.push(format!("`{form}` does not begin with `{name}`"));
            }
            if let Some(second) = words.get(1)
                && own.contains(*second)
                && !ITS_OWN_WORD.contains(&(name, second))
            {
                wrong.push(format!(
                    "`{form}` is said of `{name}`, and `{second}` is a command of its own: a topic writes \
                     another command on a line of its own"
                ));
            }
            if form.contains("(help ") {
                wrong.push(format!("`{form}` holds what help says beside it"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "ways help --json says a command is written that are not:\n{}",
        wrong.join("\n")
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn docs_export_writes_every_page_with_no_daemon_and_replaces_only_what_it_wrote() {
    let dir = scratch("documented-export");
    let to = dir.join("site").join("reference");
    let folder = to.to_str().unwrap();
    let said = run_in(&dir, NOWHERE, &[], &["docs", "export", folder, "--json"]).await.ok();
    let said: Value = serde_json::from_str(&said).unwrap();
    assert_eq!(said["changed"], true, "{said}");
    assert_eq!(said["version"], manual::VERSION);
    let index = std::fs::read_to_string(to.join("index.md")).unwrap();
    let head = format!("---\npublic: true\nversion: {}\n---\n", manual::VERSION);
    let listed: BTreeSet<String> =
        std::fs::read_to_string(to.join(".exported")).unwrap().lines().map(str::to_string).collect();
    assert_eq!(listed.len() as u64, said["pages"].as_u64().unwrap(), "every page");
    for page in listed.iter().filter(|page| page.ends_with(".md")) {
        let text = std::fs::read_to_string(to.join(page)).unwrap();
        assert!(text.starts_with(&head), "{page} says it is public and the version first:\n{text}");
        if page != "index.md" {
            assert!(index.contains(&format!("]({page})")), "the index links {page}");
        }
    }
    for page in [
        "commands/apply.md",
        "help/apply.md",
        "keywords/step.md",
        "actions/board.add.md",
        "actions/docs.export.md",
        "events/run.failed.md",
        "blueprints/std/board.md",
        "guides/first-run.md",
        "releases.md",
    ] {
        assert!(listed.contains(page), "{page} is written");
    }

    // A flag the command does not read is refused before anything is
    // written: a folder nothing was written to stays as it was.
    let elsewhere = dir.join("elsewhere");
    let bogus = run_in(&dir, NOWHERE, &[], &["docs", "export", elsewhere.to_str().unwrap(), "--bogus"]).await;
    assert_eq!(bogus.code, 2, "{}", bogus.err);
    assert!(bogus.err.contains("--bogus"), "{}", bogus.err);
    assert!(!elsewhere.exists(), "nothing is written for a command that is refused");

    // Written again with nothing changed, nothing is written.
    let again = run_in(&dir, NOWHERE, &[], &["docs", "export", folder, "--json"]).await.ok();
    assert!(again.contains("\"changed\": false"), "{again}");

    // A folder that holds something no export wrote is left as it is.
    std::fs::write(to.join("mine.txt"), "mine").unwrap();
    let refused = run_in(&dir, NOWHERE, &[], &["docs", "export", folder]).await;
    assert_ne!(refused.code, 0);
    assert!(refused.err.contains("which no export put there"), "{}", refused.err);
    assert_eq!(std::fs::read_to_string(to.join("mine.txt")).unwrap(), "mine");

    let version: Value =
        serde_json::from_str(&run_in(&dir, NOWHERE, &[], &["version", "--json"]).await.ok()).unwrap();
    assert_eq!(version["version"], manual::VERSION);
    let _ = std::fs::remove_dir_all(dir);
}
