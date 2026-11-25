//! The `portable` gate: a blueprint that ships with the product runs no
//! shell script of ours.
//!
//! The product runs on Linux, Windows and macOS, and the blueprints that
//! come with it are part of it. A script is one shell's: `run:` is handed
//! to `sh` on some systems and to `cmd` on others, and what is written for
//! the one is not understood by the other. So what a shipped blueprint asks
//! of a program is said as the program and its arguments (`cmd:`, or `cli:`
//! for the product's own command line), which is the same everywhere, and
//! the deciding a script would have done (`if`, `||`, `$(...)`) is steps.
//!
//! A command that whoever applies a blueprint hands it is theirs, written
//! for the system they are on: `run: "${params.run}"` passes.
//!
//! This reads the files as text, like the other static gates: it knows how
//! a blueprint is laid out, and no more of the language than that.

use crate::gates::{GateResult, Scope};
use crate::repo::Repo;

/// Where the blueprints that ship are: the standard ones, and those a
/// package brings in its manifest.
const STANDARD: &str = "blueprints/";
const PACKAGES: &str = "extensions/";
const MANIFEST: &str = "/extension.yaml";

/// What still has a script of ours, and why. Each is a file, the step, and
/// the reason; one that is no longer true fails the gate, so that the list
/// cannot outlive what it excuses.
const KNOWN: &[(&str, &str, &str)] = &[
    (
        "blueprints/std/log-feed.bp",
        "feed",
        "following a file as it grows and adding its lines to another has no program that is on every \
         system; it waits on one of the product's own",
    ),
    (
        "blueprints/std/log-feed.bp",
        "feed_to",
        "following a file as it grows and handing its lines to a command has no program that is on every \
         system; it waits on one of the product's own",
    ),
];

/// Keys a command is written under that are not the step's own name.
const WRAPPERS: &[&str] = &["with", "service", "spec"];

struct Script {
    line: usize,
    step: String,
}

/// Whether a file is a blueprint by what it is called: the ending
/// `brand.toml` says, alone or with what marks the file as YAML after it.
fn is_blueprint(repo: &Repo, file: &str) -> bool {
    let ending = repo.brand("blueprint_ext").unwrap_or_default();
    ["", ".yml", ".yaml"].iter().any(|yaml| !ending.is_empty() && file.ends_with(&format!(".{ending}{yaml}")))
}

pub fn portable(repo: &Repo, _: &Scope) -> GateResult {
    let mut problems = Vec::new();
    let mut excused = vec![false; KNOWN.len()];
    let mut read = 0;
    for file in repo.files() {
        let manifest = file.starts_with(PACKAGES) && file.ends_with(MANIFEST);
        if !manifest && !(file.starts_with(STANDARD) && is_blueprint(repo, &file)) {
            continue;
        }
        let Some(text) = repo.read(&file) else { continue };
        read += 1;
        for script in scripts(&text, manifest) {
            match KNOWN.iter().position(|(known, step, _)| *known == file && *step == script.step) {
                Some(index) => excused[index] = true,
                None => problems.push(format!(
                    "{file}:{}: the step `{}` runs a shell script of ours. Say what it asks for as a program and \
                     its arguments (`cmd:`, or `cli:` for this program's own command line) and let steps do the \
                     deciding; a command is only written as `run:` when it is wholly a param, which is its \
                     user's to write",
                    script.line, script.step
                )),
            }
        }
    }
    for ((file, step, _), _) in KNOWN.iter().zip(&excused).filter(|(_, excused)| !**excused) {
        problems.push(format!(
            "{file}: the step `{step}` is excused a script it no longer has; take it off the list in \
             xtask/src/portable.rs"
        ));
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let known: Vec<String> =
        KNOWN.iter().map(|(file, step, why)| format!("`{step}` in {file}: {why}")).collect();
    Ok(match known.is_empty() {
        true => format!("{read} files of blueprints run no script of ours"),
        false => format!("{read} files of blueprints run no script of ours, but for {}", known.join("; ")),
    })
}

/// How far in a line begins, and what it says there.
fn indented(line: &str) -> (usize, &str) {
    let said = line.trim_start();
    (line.len() - said.len(), said)
}

/// The key a line begins with, if it is `key:` or `key: value`.
fn key(said: &str) -> Option<(&str, &str)> {
    let (name, rest) = said.split_once(':')?;
    let named = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || "_-./".contains(c));
    (named && (rest.is_empty() || rest.starts_with(' '))).then_some((name, rest.trim()))
}

/// Whether a command is wholly a param: `"${params.run}"`, with nothing of
/// ours before or after it.
fn theirs(value: &str) -> bool {
    let value = value.trim_start();
    let (quote, rest) = match value.chars().next() {
        Some(quote @ ('"' | '\'')) => (Some(quote), &value[1..]),
        _ => (None, value),
    };
    let Some(rest) = rest.strip_prefix("${") else { return false };
    let Some(end) = rest.find('}') else { return false };
    let Some(name) = rest[..end].trim().strip_prefix("params.") else { return false };
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return false;
    }
    let after = match quote {
        Some(quote) => match rest[end + 1..].strip_prefix(quote) {
            Some(after) => after,
            None => return false,
        },
        None => &rest[end + 1..],
    };
    let after = after.trim_start();
    after.is_empty() || after.starts_with([',', '}', '#'])
}

/// Each `run:` inside an inline mapping on a line (`{ run: ... }`), as
/// what comes after it. One inside a quoted string is words, not a key.
fn inline(said: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = said[from..].find("run:") {
        let at = from + at;
        from = at + 4;
        let before = said[..at].trim_end();
        let quoted = before.chars().filter(|c| *c == '"').count() % 2 == 1;
        if !quoted && before.ends_with(['{', ',']) {
            found.push(&said[at + 4..]);
        }
    }
    found
}

/// The scripts of ours in a blueprint, or in the blueprints a package's
/// manifest brings.
fn scripts(text: &str, manifest: bool) -> Vec<Script> {
    let mut found = Vec::new();
    // The keys this line is under, nearest last, each with how far in it is.
    let mut above: Vec<(usize, &str)> = Vec::new();
    // Text under a `|` or `>` is not read for keys, down to this depth.
    let mut block: Option<usize> = None;
    for (index, line) in text.lines().enumerate() {
        let (depth, said) = indented(line);
        if said.is_empty() || said.starts_with('#') {
            continue;
        }
        if block.is_some_and(|block| depth > block) {
            continue;
        }
        block = None;
        while above.last().is_some_and(|(over, _)| *over >= depth) {
            above.pop();
        }
        let step = |own: Option<&str>| -> String {
            own.into_iter()
                .chain(above.iter().rev().map(|(_, name)| *name))
                .find(|name| !WRAPPERS.contains(name))
                .unwrap_or("?")
                .to_string()
        };
        // A param called `run` is a declaration, not a command.
        let declared = above.iter().any(|(_, name)| *name == "params");
        let Some((name, value)) = key(said.trim_start_matches("- ")) else { continue };
        if !declared {
            if name == "run" && !theirs(value) {
                found.push(Script { line: index + 1, step: step(None) });
            }
            for _ in inline(value).into_iter().filter(|value| !theirs(value)) {
                found.push(Script { line: index + 1, step: step(Some(name)) });
            }
        }
        // A package's manifest holds each blueprint it brings as a block
        // of text under `blueprints:`, and that text is what is read here.
        let blueprint = manifest && above.len() == 1 && above[0].1 == "blueprints";
        if (value.starts_with('|') || value.starts_with('>')) && !blueprint {
            block = Some(depth);
        }
        above.push((depth, name));
    }
    found
}
