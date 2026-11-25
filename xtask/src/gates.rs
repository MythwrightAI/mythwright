//! The gates.
//!
//! Each gate is a rule with its reason attached, so a failure explains
//! itself. The static gates need no compiler and run in milliseconds; they
//! are safe to run on the weakest machine in the fleet.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use crate::repo::{Repo, line_count};
use crate::suites::{self, Program, WHOLE_PRODUCT};

pub type GateResult = Result<String, Vec<String>>;

pub struct Gate {
    pub name: &'static str,
    /// Needs the compiler; skipped by `--fast`.
    pub slow: bool,
    pub why: &'static str,
    pub run: fn(&Repo, &Scope) -> GateResult,
}

pub enum Scope {
    Everything,
    /// Only these crates need compiling and testing.
    Crates(Vec<String>),
}

pub const ALL: &[Gate] = &[
    Gate {
        name: "brand",
        slow: false,
        why: "the product name lives only in brand.toml (and Markdown), so that renaming it is one command",
        run: brand,
    },
    Gate {
        name: "providers",
        slow: false,
        why: "the application knows no agent, and no other product it is set up to use, by name; \
              everything about one is in its package under extensions/, so that adding or changing one \
              never touches the application",
        run: providers,
    },
    Gate {
        name: "general",
        slow: false,
        why: "what a card is, and what a stage, a lane or a fleet does with one, is said by blueprints and rows, so \
              that it changes without a rebuild; the runner runs steps, and a building block does one thing to \
              whatever it is handed. Code of the runner, or an action or what it takes, that is named for one of \
              them has taken some of that process into the program",
        run: crate::general::general,
    },
    Gate {
        name: "portable",
        slow: false,
        why: "the blueprints that ship with the product run wherever it does; a shell script is one shell's, \
              so what they ask of a program is the program and its arguments, and a script is only ever a \
              command its user handed over",
        run: crate::portable::portable,
    },
    Gate {
        name: "manifests",
        slow: false,
        why: "one place for versions and features keeps `build` and `test` on the same feature set (a mismatch \
              recompiled 21 crates on every switch in the old codebase), and one test binary per crate keeps \
              link time down (289 binaries -> 1 cut the test build by 31%)",
        run: manifests,
    },
    Gate {
        name: "layers",
        slow: false,
        why: "dependencies point one way, so an edit to a high layer never rebuilds a low one and no crate can \
              grow into a cycle (111 of 172 modules were in one cycle in the old daemon, which is why it could \
              not be split)",
        run: layers,
    },
    Gate {
        name: "size",
        slow: false,
        why: "small files and small crates are what keep incremental builds fast and the code readable; split \
              before adding",
        run: size,
    },
    Gate {
        name: "budget",
        slow: false,
        why: "a build gets slow a little at a time and nobody sees it happen (fifty minutes, in the codebase \
              before this one); what a build takes is measured, kept in docs/build-history.csv, and held to \
              build-budget.toml",
        run: |repo, _| crate::buildtimes::gate(repo),
    },
    Gate {
        name: "sleeps",
        slow: false,
        why: "tests take time from the injected clock, never the real one (real waits were ~80% of every gate \
              in the old codebase)",
        run: sleeps,
    },
    Gate { name: "fmt", slow: false, why: "formatting is not a matter of opinion here", run: fmt },
    Gate { name: "clippy", slow: true, why: "lints are errors", run: clippy },
    Gate {
        name: "plain",
        slow: true,
        why: "a blueprint that ships is where a workflow is managed in plain words: what exists, what is heard, \
              what is done. What is decided is a building block's, written in Rust and tested as a function, \
              which the blueprint asks. A text that works things out for itself is a program in holes in \
              strings that nothing tests but playing the whole product, and is where the faults at a restart \
              were",
        run: plain,
    },
    Gate { name: "test", slow: true, why: "the tests are the specification", run: test },
    Gate {
        name: "offline",
        slow: true,
        why: "nothing the product does leaves the machine unless its user points it somewhere; people are told \
              so, and this is what holds it to that, libraries included. The tests that drive the whole \
              product are run here and nowhere else in the gate, so they are also the specification",
        run: crate::offline::offline,
    },
    Gate {
        name: "ui",
        slow: true,
        why: "the client is type-checked and tested like everything else",
        run: ui,
    },
];

/// Lower layers first. A crate may depend only on crates in layers above its
/// own line here. Adding a crate means adding it to exactly one line.
const LAYERS: &[&[&str]] = &[
    &["brand", "link", "reader"],
    &["blueprint", "kernel"],
    &["journal"],
    &["runs", "people", "manual", "windowed"],
    &["texts", "environments"],
    &["conductor", "projects"],
    &["blocks", "talk", "pages"],
    &["workspaces", "search"],
    &["comparisons", "hosting", "studio"],
    &["deciding", "trash", "places", "told", "outline", "topics", "extensions"],
    &["agents", "sandbox"],
    &["gateway"],
    &["folders"],
    &["setup", "served"],
    &["api"],
    &["cli", "desktop", "xtask", "kv-cache"],
];

const MAX_FILE_LINES: usize = 1_000;
const MAX_CRATE_LINES: usize = 15_000;
/// A crate that is let past the ceiling for a while, with how far and why.
/// One that is here is owed a split, and is taken off this list by it.
///
/// None is. The runner (`conductor`) was, at 17,100, until what it keeps
/// on disk became a crate of its own (`runs`); comparisons and the
/// built-in actions came out after (`comparisons`, `blocks`), which left
/// it eight lines under the ceiling. It was here again at 15,300, for
/// what an apply meets as something is stopped (`meets`), until where
/// texts are found and what is kept of them came out below it (`texts`).
const ROOM: &[(&str, usize, &str)] = &[];
const SOURCE_EXTENSIONS: &[&str] = &["rs", "ts", "tsx", "js", "mjs", "css"];

impl Scope {
    /// The crates touched by local changes, and everything that depends on
    /// them; and, where that came to everything, the change that made it so.
    ///
    /// A change outside every crate affects everything, with two
    /// exceptions that are compiled in or checked another way. Nothing
    /// here lists which tests read which notes, packages or examples: the
    /// notes are held to the code by tests in half the crates, and a list
    /// of that kind, written by hand in the codebase before this one, was
    /// wrong for four folders at once and wrong in the direction nobody
    /// sees, a suite recorded as not needed that was the one to fail.
    pub fn changed(repo: &Repo) -> (Scope, Option<String>) {
        let members = repo.members();
        let mut touched = BTreeSet::new();
        // The client's files that changed, which its own gate checks.
        let mut client = Vec::new();
        for file in repo.changed_files() {
            let owner = members
                .iter()
                .filter(|m| file.starts_with(&format!("{}/", m.dir)))
                .max_by_key(|m| m.dir.len());
            match owner {
                Some(member) => {
                    touched.insert(member.name.clone());
                }
                // Blueprints shipped with the product are compiled into the
                // crate that embeds them.
                None if file.starts_with("blueprints/") => {
                    touched.insert("texts".to_string());
                }
                None if file.starts_with("ui/") => client.push(file),
                None => return (Scope::Everything, Some(file)),
            }
        }
        // A file of the client's that Rust reads is named where it is read,
        // by where it is: a crate that names one that changed is touched.
        if !client.is_empty() {
            for file in repo.files().iter().filter(|file| file.ends_with(".rs")) {
                let Some(member) = members.iter().find(|m| file.starts_with(&format!("{}/", m.dir))) else {
                    continue;
                };
                let text = repo.read(file).unwrap_or_default();
                if client.iter().any(|changed| text.contains(changed.as_str())) {
                    touched.insert(member.name.clone());
                }
            }
        }
        // Add dependents until nothing changes.
        loop {
            let before = touched.len();
            for member in &members {
                if member.deps.iter().any(|dep| touched.contains(&dep.name)) {
                    touched.insert(member.name.clone());
                }
            }
            if touched.len() == before {
                break;
            }
        }
        touched.remove("desktop");
        (Scope::Crates(touched.into_iter().collect()), None)
    }

    pub(crate) fn package_args(&self) -> Option<Vec<String>> {
        match self {
            Scope::Everything => Some(Vec::new()),
            Scope::Crates(crates) if crates.is_empty() => None,
            Scope::Crates(crates) => {
                Some(crates.iter().flat_map(|name| ["-p".to_string(), name.clone()]).collect())
            }
        }
    }
}

/// The links a code-hosting site offers beside its issue templates, in
/// that site's own folder at the top of the repository (`.<name>/`): it
/// takes only whole addresses, and the repository's own has the product's
/// name in it. It is said to that site and nothing of the program reads it.
fn issue_links(file: &str) -> bool {
    file.starts_with('.') && file.ends_with("/ISSUE_TEMPLATE/config.yml")
}

fn brand(repo: &Repo, _: &Scope) -> GateResult {
    let (Some(display), Some(id)) = (repo.brand("display"), repo.brand("id")) else {
        return Err(vec!["brand.toml must define `display` and `id`".into()]);
    };
    let needles = [display.to_lowercase(), id.to_lowercase()];
    let mut problems = Vec::new();
    let mut scanned = 0;
    for file in repo.files() {
        if file == "brand.toml" || file == "Cargo.lock" || file.ends_with(".md") || issue_links(&file) {
            continue;
        }
        let Some(text) = repo.read(&file) else { continue };
        scanned += 1;
        for (index, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            if needles.iter().any(|needle| lower.contains(needle)) {
                problems.push(format!(
                    "{file}:{}: spells out the product name; use the `brand` crate",
                    index + 1
                ));
            }
        }
    }
    if problems.is_empty() { Ok(format!("{scanned} files")) } else { Err(problems) }
}

/// Where extension packages are kept, each a folder with a manifest.
const PACKAGES: &str = "extensions/";

/// A top-level `key: value` or indented `key: value` from a manifest, as
/// written, without its quotes.
fn manifest_value(text: &str, key: &str) -> Option<String> {
    let line = text.lines().find(|line| line.trim_start().starts_with(&format!("{key}:")))?;
    let value = line.split_once(':')?.1.trim().trim_matches(['"', '\'']);
    (!value.is_empty()).then(|| value.to_lowercase())
}

/// A name shorter than this is looked for as a word of its own. A program
/// may be called by two letters, and two letters are inside half the words
/// there are; a longer name is looked for wherever it is, inside an
/// identifier too.
const WHOLE_WORD_UNDER: usize = 4;

/// What follows a program's name in the name of a kind of file. A project
/// of anybody's may hold a file called after the program that reads it, as
/// a `Makefile` is after `make`. The editor colours such a file and the
/// search index reads it by that name, which is the file's: it no more
/// makes the application know the program than a file's ending does. So a
/// name with this straight after it is not the program being named.
const KIND_OF_FILE: &str = "file";

/// Whether a line, in lowercase, names `name`.
fn names_it(line: &str, name: &str) -> bool {
    let letter = |ch: char| ch.is_alphanumeric();
    line.match_indices(name).any(|(at, found)| {
        let after = &line[at + found.len()..];
        if after.starts_with(KIND_OF_FILE) {
            return false;
        }
        name.len() >= WHOLE_WORD_UNDER
            || (!line[..at].chars().next_back().is_some_and(letter)
                && !after.chars().next().is_some_and(letter))
    })
}

fn providers(repo: &Repo, _: &Scope) -> GateResult {
    // What must not be named is whatever the packages here are for: their
    // names and the programs they run. Nothing is listed in this file,
    // which would itself be naming them.
    let files = repo.files();
    let mut names = BTreeSet::new();
    let mut packages = 0;
    for file in files.iter().filter(|file| file.starts_with(PACKAGES) && file.ends_with("/extension.yaml")) {
        let Some(text) = repo.read(file) else { continue };
        packages += 1;
        names.extend(manifest_value(&text, "extension"));
        names.extend(manifest_value(&text, "program"));
    }
    // And whatever is kept for packages that are not written yet.
    let reserved = repo.read(&format!("{PACKAGES}reserved")).unwrap_or_default();
    names.extend(
        reserved
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_lowercase),
    );
    let mut problems = Vec::new();
    let mut scanned = 0;
    for file in &files {
        // What a tool wrote down about what was fetched is not the application speaking.
        let record = file == "Cargo.lock" || file.ends_with("package-lock.json");
        if file.starts_with(PACKAGES) || record || file.ends_with(".md") {
            continue;
        }
        let Some(text) = repo.read(file) else { continue };
        scanned += 1;
        // The one exception. A service that hosts a repository reads what
        // it is to do for it from a folder of its own name at the top
        // (`.<name>/`). What is written there is said to that service, in
        // its own words, and cannot be said without naming it; it is no
        // part of the application, which still knows none of them.
        // Its own names are the package's of that name too: the program
        // that package runs is the service's own command.
        let its_own: Vec<String> = names
            .iter()
            .find(|name| file.starts_with(&format!(".{name}/")))
            .map(|name| {
                let manifest = repo.read(&format!("{PACKAGES}{name}/extension.yaml")).unwrap_or_default();
                let mut own = vec![name.clone()];
                own.extend(manifest_value(&manifest, "extension"));
                own.extend(manifest_value(&manifest, "program"));
                own
            })
            .unwrap_or_default();
        for (index, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            if let Some(name) =
                names.iter().filter(|name| !its_own.contains(name)).find(|name| names_it(&lower, name))
            {
                problems.push(format!(
                    "{file}:{}: names `{name}`, which is an extension package's business; say it in {PACKAGES}{name}/ or not at all",
                    index + 1
                ));
            }
        }
    }
    if problems.is_empty() {
        Ok(format!("{scanned} files name none of {packages} packages"))
    } else {
        Err(problems)
    }
}

fn manifests(repo: &Repo, _: &Scope) -> GateResult {
    let mut problems = Vec::new();
    let members = repo.members();
    for member in &members {
        let at = |line: usize| format!("{}/Cargo.toml:{line}", member.dir);
        for dep in &member.deps {
            let plain = dep.spec.replace(' ', "");
            if plain != "{workspace=true}" {
                problems.push(format!(
                    "{}: `{}` under [{}] must be `{{ workspace = true }}`; declare versions and features once, in the root manifest",
                    at(dep.line),
                    dep.name,
                    dep.section
                ));
            }
        }
        if member.manifest.contains("[dependencies.") || member.manifest.contains("[dev-dependencies.") {
            problems.push(format!(
                "{}: use inline `name = {{ workspace = true }}`, not a dependency table",
                at(1)
            ));
        }
        if member.manifest.contains("\n[features]") {
            problems.push(format!("{}: member crates do not define features", at(1)));
        }
        let has_lib = repo.root.join(&member.dir).join("src/lib.rs").is_file();
        if has_lib
            && !(member.manifest.contains("test = false") && member.manifest.contains("doctest = false"))
        {
            problems.push(format!(
                "{}: a library sets `test = false` and `doctest = false` under `[lib]`; tests go in tests/suite",
                at(1)
            ));
        }
        // One test binary: tests/suite/main.rs, with modules beside it.
        let tests_dir = format!("{}/tests/", member.dir);
        for file in repo.files() {
            let Some(rest) = file.strip_prefix(&tests_dir) else { continue };
            let is_extra_target = (rest.ends_with(".rs") && !rest.contains('/'))
                || (rest.ends_with("/main.rs") && rest != "suite/main.rs");
            if is_extra_target {
                problems
                    .push(format!("{file}: a second test binary; make it a module of tests/suite instead"));
            }
        }
    }
    if problems.is_empty() { Ok(format!("{} crates", members.len())) } else { Err(problems) }
}

fn layers(repo: &Repo, _: &Scope) -> GateResult {
    let layer_of: BTreeMap<&str, usize> = LAYERS
        .iter()
        .enumerate()
        .flat_map(|(index, names)| names.iter().map(move |name| (*name, index)))
        .collect();
    let members = repo.members();
    let names: BTreeSet<&str> = members.iter().map(|member| member.name.as_str()).collect();
    let mut problems = Vec::new();
    for member in &members {
        let Some(&layer) = layer_of.get(member.name.as_str()) else {
            problems.push(format!(
                "{}/Cargo.toml: `{}` is not in the layer table in xtask/src/gates.rs; add it to one layer",
                member.dir, member.name
            ));
            continue;
        };
        for dep in member.deps.iter().filter(|dep| names.contains(dep.name.as_str())) {
            match layer_of.get(dep.name.as_str()) {
                Some(&below) if below < layer => {}
                _ => problems.push(format!(
                    "{}/Cargo.toml:{}: `{}` may not depend on `{}`; dependencies only point down the layer table",
                    member.dir, dep.line, member.name, dep.name
                )),
            }
        }
    }
    if problems.is_empty() { Ok(format!("{} layers", LAYERS.len())) } else { Err(problems) }
}

fn size(repo: &Repo, _: &Scope) -> GateResult {
    let mut problems = Vec::new();
    let mut per_crate: BTreeMap<String, usize> = BTreeMap::new();
    let members = repo.members();
    let mut largest = (0, String::new());
    for file in repo.files() {
        let extension = file.rsplit('.').next().unwrap_or_default();
        if !SOURCE_EXTENSIONS.contains(&extension) || file.contains("/dist/") || file.contains("/vendor/") {
            continue;
        }
        let lines = line_count(&repo.root.join(&file));
        if lines > largest.0 {
            largest = (lines, file.clone());
        }
        if lines > MAX_FILE_LINES {
            problems.push(format!("{file}: {lines} lines; the ceiling is {MAX_FILE_LINES}. Split it."));
        }
        if let Some(member) = members.iter().find(|member| file.starts_with(&format!("{}/src/", member.dir)))
        {
            *per_crate.entry(member.name.clone()).or_default() += lines;
        }
    }
    for (name, lines) in &per_crate {
        let most = ROOM.iter().find(|(one, ..)| one == name).map_or(MAX_CRATE_LINES, |(_, most, _)| *most);
        if *lines > most {
            problems
                .push(format!("crate `{name}`: {lines} lines of source; the ceiling is {most}. Split it."));
        }
    }
    if problems.is_empty() {
        Ok(format!("largest file {} lines ({})", largest.0, largest.1))
    } else {
        Err(problems)
    }
}

fn sleeps(repo: &Repo, _: &Scope) -> GateResult {
    let mut problems = Vec::new();
    for file in repo.files() {
        if !file.contains("/tests/") || !file.ends_with(".rs") {
            continue;
        }
        let Some(text) = repo.read(&file) else { continue };
        for (index, line) in text.lines().enumerate() {
            let sleeps = line.contains("thread::sleep(") || line.contains("time::sleep(");
            if sleeps && !line.contains("allow-sleep:") {
                problems.push(format!(
                    "{file}:{}: a test waits on the real clock; use the manual clock",
                    index + 1
                ));
            }
        }
    }
    if problems.is_empty() { Ok(String::new()) } else { Err(problems) }
}

fn cargo(repo: &Repo, args: &[&str], extra: &[String]) -> GateResult {
    let output = Command::new("cargo").args(args).args(extra).current_dir(&repo.root).output();
    match output {
        Ok(output) if output.status.success() => Ok(String::new()),
        Ok(output) => Err(failures(&format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))),
        Err(err) => Err(vec![format!("could not run cargo: {err}")]),
    }
}

/// What a person needs from the output of a build or a test run that
/// failed: errors, failed tests and what they said.
///
/// What a test said as it failed is the lines after the one that says
/// where, up to the note about backtraces. They are kept whole: a failure
/// read on another machine, where it cannot be run again to see, has to
/// say what was waited for and not only on which line.
pub fn failures(text: &str) -> Vec<String> {
    let keep = |line: &str| {
        line.starts_with("error")
            || line.starts_with("warning: unused")
            || line.starts_with("-->")
            || line.starts_with("Diff in")
            || line.starts_with("test ") && line.ends_with("FAILED")
            || line.starts_with("left:")
            || line.starts_with("right:")
            || line.starts_with("test result: FAILED")
    };
    let mut lines = Vec::new();
    // How many more lines of what a failed test said are still to be kept.
    let mut said = 0;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("thread '") {
            lines.push(line.to_string());
            said = 70;
        } else if said > 0 && (trimmed.is_empty() || trimmed.starts_with("note: run with")) {
            said = 0;
        } else if said > 0 {
            lines.push(line.to_string());
            said -= 1;
        } else if keep(trimmed) {
            lines.push(line.to_string());
        }
    }
    if lines.is_empty() {
        lines = text.lines().rev().take(20).map(str::to_string).collect();
        lines.reverse();
    }
    lines
}

fn fmt(repo: &Repo, _: &Scope) -> GateResult {
    cargo(repo, &["fmt", "--all", "--check"], &[])
}

fn clippy(repo: &Repo, scope: &Scope) -> GateResult {
    match scope.package_args() {
        Some(packages) => {
            let mut args = packages;
            args.extend(["--all-targets", "--", "-D", "warnings"].map(String::from));
            cargo(repo, &["clippy", "--quiet"], &args)
        }
        None => Ok("nothing to lint".into()),
    }
}

/// What the blueprints that ship work out for themselves, held to what is
/// still let through (`xtask::plain`). The texts are read by a test of the
/// runner's suite, which reads an expression as the runner does; the
/// tooling cannot, and counts what that test says.
fn plain(repo: &Repo, scope: &Scope) -> GateResult {
    let (reader, test) = xtask::plain::READER;
    let covers = |name: &str| match scope {
        Scope::Everything => true,
        Scope::Crates(crates) => crates.iter().any(|one| one == name),
    };
    // The texts are compiled into the runner, and the language is under
    // it: a change to either has the runner's tests built. A change to
    // what is still let through is a change to the tooling, and has them
    // built for this.
    let programs = if covers(reader) {
        suites::built(repo, scope)?
    } else if covers("xtask") {
        suites::built_whole(repo)?
    } else {
        return Ok(
            "not looked at: neither the blueprints that ship, the language, nor what is still let through \
             was changed"
                .into(),
        );
    };
    let Some(program) = programs.iter().find(|program| program.krate == reader) else {
        return Err(vec![format!(
            "the tests of `{reader}`, which read the blueprints that ship, were not built"
        )]);
    };
    let printed = suites::asked_of(program, &["--exact", test, "--nocapture", "--test-threads=1"])
        .map_err(|said| failures(&said))?;
    let (read, found) = xtask::plain::heard(&printed).map_err(|why| vec![why])?;
    // What the program's own blueprints are still let is written in the
    // tooling; a package's is the package's to keep, beside its manifest.
    let mut still = xtask::plain::still();
    for file in repo.files() {
        let Some(package) = file.strip_suffix(xtask::plain::PACKAGES_OWN) else { continue };
        if !package.starts_with("extensions/") {
            continue;
        }
        let Some(text) = repo.read(&file) else { continue };
        let manifest = format!("{package}extension.yaml");
        still.push(xtask::plain::of_package(&manifest, &file, &text).map_err(|why| vec![why])?);
    }
    xtask::plain::held(read, &found, &still)
}

/// Every crate's tests but those of the two that drive the whole product,
/// which `offline` runs: once, where there is no network.
fn test(repo: &Repo, scope: &Scope) -> GateResult {
    let programs: Vec<Program> = suites::built(repo, scope)?
        .into_iter()
        .filter(|program| !WHOLE_PRODUCT.contains(&program.krate.as_str()))
        .collect();
    if programs.is_empty() {
        return Ok("nothing to test".into());
    }
    crate::leftover::held(repo, || {
        let outcome = suites::run_all(repo, &programs);
        if outcome.problems.is_empty() {
            Ok(format!("{}; `offline` runs the rest", outcome.timed))
        } else {
            Err(outcome.problems)
        }
    })
}

fn ui(repo: &Repo, _: &Scope) -> GateResult {
    if !repo.root.join("ui/package.json").is_file() {
        return Ok("no client yet".into());
    }
    if !repo.root.join("ui/node_modules").is_dir() {
        return Ok("skipped: run `npm install` in ui/ to enable".into());
    }
    // The client's tests are TypeScript that Node runs as it is. A Node
    // built without that, as some systems' own packages are, fails every
    // one of them with a line about a file's ending and nothing of why.
    let asked = Command::new("node").args(["-p", "process.features.typescript || ''"]).output();
    if let Ok(asked) = asked
        && asked.status.success()
        && String::from_utf8_lossy(&asked.stdout).trim().is_empty()
    {
        let version = Command::new("node").arg("--version").output();
        let version = version.map(|said| String::from_utf8_lossy(&said.stdout).trim().to_string());
        return Err(vec![format!(
            "the `node` on the path ({}) does not run TypeScript itself, which the client's tests need: it is \
             older than 22.18, or was built without it, as a system's own package may be. Use one from \
             nodejs.org, 22.18 or newer, and run the gate again.",
            version.unwrap_or_default()
        )]);
    }
    let output = Command::new(xtask::tools::here("npm"))
        .args(["run", "--silent", "check"])
        .current_dir(repo.root.join("ui"))
        .output();
    match output {
        Ok(output) if output.status.success() => Ok(String::new()),
        Ok(output) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            Err(text
                .lines()
                .rev()
                .take(30)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(str::to_string)
                .collect())
        }
        Err(err) => Err(vec![format!("could not run npm: {err}")]),
    }
}
