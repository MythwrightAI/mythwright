//! The runner and the building blocks know no process.
//!
//! What a piece of work is (a card), where it is kept (a board), what is
//! done with it and by whom (a stage, a lane, a fleet) is said by
//! blueprints and by the rows of a project's tables, so that it can be
//! changed without building the program again. The runner runs steps; a
//! building block does one thing to whatever it is handed. Neither needs
//! those words to do its job, and one that does has taken some of the
//! process into the program.
//!
//! So: the code of the runner and of the language names none of them, and
//! no action is called after one or takes or gives anything called after
//! one. Comments and notes may. What is let through is listed here with
//! the reason, and is the whole of it.

use crate::gates::{GateResult, Scope};
use crate::repo::Repo;

/// The words of a process.
const WORDS: &[&str] =
    &["card", "cards", "board", "boards", "stage", "stages", "lane", "lanes", "fleet", "fleets"];

/// Where the runner and the language are. What the runner keeps on disk
/// (`runs`) is the runner's code as much as what runs it, and so is what
/// starts one blueprint several ways (`comparisons`), which came out of
/// it and knows what the work is no more than it does; and the actions
/// it brought itself (`blocks`), which are building blocks if anything is;
/// and where its texts are found (`texts`), which names the ones that ship;
/// and what of the language is read alone (`reader`), which came out of it.
const RUNNER: &[&str] = &[
    "crates/conductor/src/",
    "crates/texts/src/",
    "crates/runs/src/",
    "crates/comparisons/src/",
    "crates/blocks/src/",
    "crates/blueprint/src/",
    "crates/reader/src/",
];

/// What a blueprint can depend on, as a test writes it from the program:
/// every action, with what it takes and gives.
const OFFERED: &str = "contract/offered.txt";

/// A line of the runner or the language that may name one: the file, what
/// the line holds, and why.
const LET_THROUGH: &[(&str, &str, &str)] = &[
    (
        "crates/texts/src/library.rs",
        "include_str!(",
        "the one table of the blueprints that ship, each by the name it has",
    ),
    (
        "crates/texts/src/library.rs",
        "\"std/prompts/",
        "the same table: a file a blueprint that ships links, by the name it has, where the line is the name's alone",
    ),
    (
        "crates/conductor/src/step.rs",
        "fleet:write",
        "a scope a key holds: sending a step to another device takes it. The word is of devices, and older than \
         a fleet of agents",
    ),
    (
        "crates/conductor/src/trigger.rs",
        "TriggerScope::Fleet",
        "the language's word for a handler that hears what every project says (`scope: fleet`)",
    ),
    ("crates/conductor/src/intrinsic.rs", "Some(\"fleet\") =>", "the same word, where a `wait` is given it"),
    (
        "crates/conductor/src/intrinsic.rs",
        "or `fleet`, every project's",
        "the same word, in what is said of a wrong one",
    ),
    (
        "crates/blueprint/src/unheard.rs",
        "`scope: fleet`",
        "the same word, in what is said to whoever applies a handler that will hear nothing without it",
    ),
    (
        "crates/blueprint/src/catalog.rs",
        "or `fleet`, every project's",
        "the same word, where a `wait` is said to take it",
    ),
    ("crates/blueprint/src/lower/triggered.rs", "TriggerScope::Fleet", "the same word, where it is read"),
    ("crates/blueprint/src/lower/triggered.rs", "\"fleet\" =>", "the same word, where it is read"),
    (
        "crates/blueprint/src/vocabulary/shown.rs",
        "scope: fleet",
        "the same word, in the whole blueprint `scope` is shown by",
    ),
    (
        "crates/blueprint/src/lower/triggered.rs",
        "expected project, fleet, own or target",
        "the same word, in what is said of a wrong one",
    ),
    ("crates/blueprint/src/model.rs", "Fleet,", "the same word, where it is declared"),
    (
        "crates/blueprint/src/summary.rs",
        "TriggerScope::Fleet",
        "the same word, where a handler that says it is said in words to hear every project",
    ),
    (
        "crates/blueprint/src/vocabulary.rs",
        "choice(\"fleet\"",
        "the same word, where the editor is told of it",
    ),
    (
        "crates/blueprint/src/validate.rs",
        "\"board\",",
        "one of the families of event that are the program's own to say and no blueprint's: a list of topics, not \
         of anything the runner does",
    ),
    (
        "crates/blueprint/src/validate.rs",
        "like `board.item.*`",
        "an example of a topic pattern, in what is said of one that is wrong",
    ),
];

/// An action that may be called after one: its name up to the dot, and why.
const NAMED_FOR: &[(&str, &str)] = &[(
    "board",
    "a board is a table the program keeps a model of (`studio::board`): a record with a status, moved between \
     columns. These are that table's own actions and were in the contract before this rule; what a card does \
     beyond being moved is blueprints",
)];

/// The words of `text` as a reader of code takes them: at anything that is
/// not a letter or a digit, and where a small letter is followed by a
/// capital, as in `CardId`.
fn words(text: &str) -> Vec<String> {
    let mut words = vec![String::new()];
    let mut before = ' ';
    for letter in text.chars() {
        let breaks = !letter.is_alphanumeric() || (before.is_lowercase() && letter.is_uppercase());
        if breaks && !words.last().is_some_and(String::is_empty) {
            words.push(String::new());
        }
        if letter.is_alphanumeric() {
            words.last_mut().expect("there is always a word being read").push(letter.to_ascii_lowercase());
        }
        before = letter;
    }
    words.retain(|word| !word.is_empty());
    words
}

/// The first of the words that `text` names, if it names one.
fn names(text: &str) -> Option<String> {
    words(text).into_iter().find(|word| WORDS.contains(&word.as_str()))
}

/// The code of a line: nothing where the whole line is a comment, and
/// otherwise what stands before a comment at its end.
fn code(line: &str) -> &str {
    let line = line.trim_start();
    if line.starts_with("//") {
        return "";
    }
    line.split(" // ").next().unwrap_or(line)
}

pub fn general(repo: &Repo, _: &Scope) -> GateResult {
    let mut problems = Vec::new();
    let (mut lines, mut actions) = (0, 0);
    for file in repo.files() {
        if !file.ends_with(".rs") || !RUNNER.iter().any(|at| file.starts_with(at)) {
            continue;
        }
        let Some(text) = repo.read(&file) else { continue };
        for (index, line) in text.lines().enumerate() {
            lines += 1;
            let Some(word) = names(code(line)) else { continue };
            if LET_THROUGH.iter().any(|(at, holds, _)| *at == file && line.contains(holds)) {
                continue;
            }
            problems.push(format!(
                "{file}:{}: the runner names a {word} in its code: `{}`. What it does there is a blueprint's to say, \
                 or a block's that is handed what it works on",
                index + 1,
                line.trim()
            ));
        }
    }
    let mut action = String::new();
    for line in repo.read(OFFERED).unwrap_or_default().lines() {
        let said: Vec<&str> = line.split_whitespace().collect();
        match said.as_slice() {
            ["action", name] => {
                actions += 1;
                action = (*name).to_string();
                let family = name.split('.').next().unwrap_or_default();
                if let Some(word) = names(name)
                    && !NAMED_FOR.iter().any(|(named, _)| *named == family)
                {
                    problems.push(format!(
                        "the action `{name}` is called after a {word}: a block is named for what it does to whatever \
                         it is handed, so that something else can be handed it"
                    ));
                }
            }
            ["takes" | "gives", name, ..] => {
                if let Some(word) = names(name) {
                    problems.push(format!(
                        "`{name}` of the action `{action}` is called after a {word}: call it for what it is to the \
                         block (an `item`, a `column`, a `name`), and let the blueprint say that it is a {word}"
                    ));
                }
            }
            _ => {}
        }
    }
    if problems.is_empty() {
        Ok(format!(
            "{lines} lines of the runner and the language and {actions} actions name no card, board, stage, lane or \
             fleet, but for {} places and the actions of `{}`, each let through with its reason",
            LET_THROUGH.len(),
            NAMED_FOR[0].0
        ))
    } else {
        Err(problems)
    }
}
