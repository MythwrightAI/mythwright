//! What this program offers a blueprint, held to what it offered before.
//!
//! A blueprint is checked against the actions and events there are when it
//! is applied. That is one direction. The other is this: a blueprint that
//! was right yesterday must not be made wrong by a build, unnoticed. So
//! everything a blueprint can depend on is written down in one file kept
//! with the code (`contract/offered.txt`): every action with what it takes,
//! gives and needs, and every event with what it carries. What a package
//! that ships brings is written down the same way in the package's own
//! folder (`extensions/<name>/contract/`), where everything else about it
//! is: the application names no package.
//!
//! A test writes what the program offers now the same way and compares.
//! Something added is told apart from something a blueprint written before
//! could break on. The first is written down by asking. The second is
//! written down only by saying that it is meant and why, and what was
//! broken is added to a list beside the file (`contract/broken.txt`), so
//! that there is a record of every time the contract was broken.
//!
//! It came out of the daemon (`api`), which offers it under the name it
//! always had (`api::contract`): it serves nothing, and reads and writes
//! the kept files and every package's manifest. It is here until there is
//! a crate of what a package says (`extensions`), which is where it goes.

use std::collections::BTreeMap;
use std::path::Path;

use blueprint::{ActionSpec, EventSpec};
use conductor::Registry;

/// The file that holds what is offered.
pub const OFFERED: &str = "offered.txt";
/// The file that lists what was broken, when, and why.
pub const BROKEN: &str = "broken.txt";

const HEAD: &str = "\
# What a blueprint can depend on: every action and every event. Written by
# a test, never by hand:
#   WRITE_CONTRACT=1 cargo test -p api the_contract
# and, where something a blueprint could break on was changed on purpose,
#   BREAK_CONTRACT=\"why\" cargo test -p api the_contract
# which adds what was broken to broken.txt.
";

const HEAD_OF_A_PACKAGE: &str = "\
# What a blueprint can depend on of this package: its actions. Written by a
# test, never by hand, as the application's own is (contract/offered.txt).
";

/// What this program offers now, as the kept file says it: the actions
/// built into it and the events it emits. What a package brings is the
/// package's to say ([`brought`]).
pub fn offered(registry: &Registry) -> String {
    let mut actions = blueprint::intrinsics();
    actions.extend(registry.specs().specs().iter().cloned());
    actions.sort_by(|a, b| a.name.cmp(&b.name));
    let events = registry.events();
    written(HEAD, &actions, events.specs())
}

/// What each package found in `packaged` offers, by the package's name:
/// its actions, written the same way. A package's own is kept in its own
/// folder, where everything else about it is.
pub fn brought(packaged: &Path) -> BTreeMap<String, String> {
    let (packages, _) = agents::extension::load_all(packaged);
    let mut found = BTreeMap::new();
    for (name, package) in packages {
        let mut actions: Vec<ActionSpec> =
            package.actions.values().map(|action| action.spec.clone()).collect();
        if actions.is_empty() {
            continue;
        }
        actions.sort_by(|a, b| a.name.cmp(&b.name));
        found.insert(name, written(HEAD_OF_A_PACKAGE, &actions, &[]));
    }
    found
}

fn written(head: &str, actions: &[ActionSpec], events: &[EventSpec]) -> String {
    let mut text = String::from(head);
    for spec in actions {
        text.push_str(&format!("\naction {}\n", spec.name));
        if let Some(capability) = &spec.capability {
            text.push_str(&format!("  needs {capability}\n"));
        }
        if spec.open {
            text.push_str("  open\n");
        }
        // What it says it makes, by the argument that names it, and which
        // of its arguments it says are a command it runs: what a summary
        // of a blueprint is made from (`blueprint::summary`).
        if let Some(makes) = &spec.makes {
            text.push_str(&format!("  makes {} {}\n", makes.named, makes.what));
            // The argument that, said, has what it makes set what
            // something does: what a summary offers to change the rows of.
            if let Some(argument) = &makes.setting {
                text.push_str(&format!("  setting {argument}\n"));
            }
        }
        for argument in &spec.runs {
            text.push_str(&format!("  runs {argument}\n"));
        }
        // The blueprints it reaches by a name of its own: what a list of
        // which blueprint runs which is made from (`blueprint::names`).
        for name in &spec.reaches {
            text.push_str(&format!("  reaches {name}\n"));
        }
        for arg in &spec.args {
            let needed = if arg.required { " needed" } else { "" };
            text.push_str(&format!("  takes {} {}{needed}\n", arg.name, arg.ty.name()));
        }
        // What an argument says is written inside it, each key and each
        // word by where it is: what an editor explains and offers by, and
        // what a form of a step's arguments is made from. Nothing is
        // checked by it, so no blueprint breaks on it.
        for arg in &spec.args {
            for (path, kind, _) in arg.within.iter().flat_map(|within| within.rows()) {
                text.push_str(&format!("  within {}.{path} {kind}\n", arg.name));
            }
        }
        for (output, _) in &spec.outputs {
            text.push_str(&format!("  gives {output}\n"));
        }
    }
    for spec in events {
        text.push_str(&format!("\nevent {}\n", spec.topic));
        if !spec.kept {
            text.push_str("  passing\n");
        }
        if spec.open {
            text.push_str("  open\n");
        }
        // Who is told of it: the scope a caller of the event stream needs.
        match spec.hears.as_deref() {
            Some(blueprint::events::ANYONE) => text.push_str("  heard anyone\n"),
            Some(scope) => text.push_str(&format!("  heard {scope}\n")),
            None => {}
        }
        for field in &spec.fields {
            let always = if field.always { " always" } else { "" };
            text.push_str(&format!("  carries {} {}{always}\n", field.name, field.ty.name()));
        }
    }
    text
}

/// One action or event as the file says it: its own lines, by their first
/// two words.
type Item = BTreeMap<(String, String), String>;

fn read(text: &str) -> BTreeMap<(String, String), Item> {
    let mut items: BTreeMap<(String, String), Item> = BTreeMap::new();
    let mut at: Option<(String, String)> = None;
    for line in text.lines().filter(|line| !line.starts_with('#') && !line.trim().is_empty()) {
        let mut words = line.split_whitespace();
        let (first, second) = (words.next().unwrap_or_default(), words.next().unwrap_or_default());
        let rest = words.collect::<Vec<_>>().join(" ");
        if line.starts_with(' ') {
            if let Some(item) = at.as_ref().and_then(|at| items.get_mut(at)) {
                item.insert((first.to_string(), second.to_string()), rest);
            }
        } else {
            let key = (first.to_string(), second.to_string());
            items.insert(key.clone(), Item::new());
            at = Some(key);
        }
    }
    items
}

/// How what is offered now differs from what was kept.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Differences {
    /// What there is now that there was not: nothing written before
    /// depends on it.
    pub added: Vec<String>,
    /// What a blueprint written before could break on, each as what it is
    /// (`action board.comment`) and what became of it.
    pub broken: Vec<(String, String)>,
}

impl Differences {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.broken.is_empty()
    }
}

pub fn differences(kept: &str, now: &str) -> Differences {
    let (kept, now) = (read(kept), read(now));
    let mut found = Differences::default();
    for (key, was) in &kept {
        let (kind, name) = key;
        let what = format!("{kind} {name}");
        let Some(is) = now.get(key) else {
            let gone = if kind == "event" { "is no longer emitted" } else { "was taken away" };
            found.broken.push((what, gone.to_string()));
            continue;
        };
        for ((word, part), before) in was {
            let of = format!("{what}: `{part}`");
            match (word.as_str(), is.get(&(word.clone(), part.clone()))) {
                ("needs", None) if !is.keys().any(|(word, _)| word == "needs") => {
                    found.added.push(format!("{what} no longer needs `{part}`"));
                }
                ("needs", None) => {}
                ("open", None) | ("passing", Some(_)) | ("needs", Some(_)) => {}
                ("passing", None) => {
                    found.added.push(format!("{what} is kept now, so a handler can hear it"))
                }
                ("heard", None) => {
                    let now = is.keys().find(|(word, _)| word == "heard");
                    let now = now.map_or("nobody in particular".to_string(), |(_, with)| format!("`{with}`"));
                    found.broken.push((
                        what.clone(),
                        format!("is told to whoever holds {now}, and was told to whoever held `{part}`"),
                    ));
                }
                // What an action says of itself for whoever reads a summary:
                // no blueprint written before depends on it.
                ("makes", None) => found.added.push(format!("{what} no longer says what it makes")),
                ("runs", None) => found.added.push(format!("{of} is no longer said to be a command it runs")),
                ("setting", None) => found
                    .added
                    .push(format!("{of} no longer says that the rows of what it makes set something")),
                ("reaches", None) => {
                    found.added.push(format!("{of} is no longer a blueprint it is said to reach"))
                }
                ("within", None) => {
                    found.added.push(format!("{of} is no longer said of what it takes within"))
                }
                ("takes", None) => found.broken.push((of, "is no longer an argument".into())),
                ("gives", None) => found.broken.push((of, "is no longer given out".into())),
                ("carries", None) => found.broken.push((of, "is no longer carried".into())),
                (_, Some(after)) if before == after => {}
                ("makes", Some(after)) => {
                    found.added.push(format!("{what} now says it makes {after}, named by `{part}`"));
                }
                ("within", Some(after)) => {
                    found.added.push(format!("{of} is said to be {after} within, and was {before}"));
                }
                ("takes", Some(after)) => match (kind_of(before), kind_of(after)) {
                    (a, b) if a != b => found.broken.push((of, format!("was {a} and is {b}"))),
                    _ if after.ends_with("needed") => found.broken.push((of, "must now be given".into())),
                    _ => found.added.push(format!("{of} need no longer be given")),
                },
                ("carries", Some(after)) => match (kind_of(before), kind_of(after)) {
                    (a, b) if a != b => found.broken.push((of, format!("was {a} and is {b}"))),
                    _ if before.ends_with("always") => {
                        found.broken.push((of, "is no longer always there".into()));
                    }
                    _ => found.added.push(format!("{of} is always there now")),
                },
                _ => {}
            }
        }
        // What an action accepted beyond its list, and what an event
        // carried beyond its own, could be relied on.
        if was.contains_key(&("open".into(), String::new()))
            && !is.contains_key(&("open".into(), String::new()))
        {
            let closed = if kind == "event" {
                "no longer carries anything beyond what is listed"
            } else {
                "no longer takes arguments beyond those listed"
            };
            found.broken.push((what.clone(), closed.into()));
        }
        for ((word, part), after) in is {
            if was.contains_key(&(word.clone(), part.clone())) {
                continue;
            }
            let of = format!("{what}: `{part}`");
            match word.as_str() {
                "needs" => {
                    let before = was.keys().find(|(word, _)| word == "needs").map(|(_, part)| part.as_str());
                    let had = before.map_or("nothing".to_string(), |before| format!("`{before}`"));
                    found.broken.push((what.clone(), format!("needs `{part}` now, and needed {had}")));
                }
                "passing" => {
                    found.broken.push((what.clone(), "is no longer kept, so no handler hears it".into()));
                }
                "takes" if after.ends_with("needed") => {
                    found.broken.push((of, "is a new argument that must be given".into()));
                }
                "makes" => found.added.push(format!("{what} says it makes {after}, named by `{part}`")),
                "runs" => found.added.push(format!("{of} is said to be a command it runs")),
                "setting" => {
                    found.added.push(format!("{of} says that the rows of what it makes set something"))
                }
                "reaches" => found.added.push(format!("{of} is said to be a blueprint it reaches")),
                "within" => found.added.push(format!("{of} is said of what it takes within: {after}")),
                "takes" => found.added.push(format!("{of}, a new argument")),
                "gives" => found.added.push(format!("{of}, newly given out")),
                "carries" => found.added.push(format!("{of}, newly carried")),
                "open" => found.added.push(format!("{what} takes more than is listed now")),
                // Said for the first time. One that was said before and is
                // said otherwise now is a change, found above.
                "heard" if !was.keys().any(|(word, _)| word == "heard") => {
                    found.added.push(format!("{what} is told to whoever holds `{part}`"));
                }
                _ => {}
            }
        }
    }
    for (kind, name) in now.keys().filter(|key| !kept.contains_key(*key)) {
        found.added.push(format!("{kind} {name}, new"));
    }
    found
}

/// The type a `takes` or `carries` line says, without `needed` or `always`.
fn kind_of(rest: &str) -> &str {
    rest.split_whitespace().next().unwrap_or_default()
}

/// What was asked of the kept contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Only that it be what the program offers.
    Hold,
    /// That what was added be written down.
    Write,
    /// That what was broken be written down too, having been meant: why.
    Break(String),
}

/// Holds the contract kept in `dir` to what is `offered` now, writing it
/// again when that was asked and may be done. `today` is the date a break
/// is written down under.
pub fn hold(dir: &Path, offered: &str, asked: &Asked, today: &str) -> Result<(), String> {
    let file = dir.join(OFFERED);
    let kept = std::fs::read_to_string(&file).unwrap_or_default();
    if kept == offered {
        return Ok(());
    }
    let write = |text: &str| {
        std::fs::write(&file, text).map_err(|err| format!("could not write {}: {err}", file.display()))
    };
    let found = differences(&kept, offered);
    if found.is_empty() {
        // Written otherwise (its heading, say) and saying the same:
        // nothing a blueprint depends on has moved.
        return if *asked == Asked::Hold { Ok(()) } else { write(offered) };
    }
    let list = |lines: &[String]| lines.iter().map(|line| format!("  {line}")).collect::<Vec<_>>().join("\n");
    let broken: Vec<String> = found.broken.iter().map(|(what, became)| format!("{what} {became}")).collect();
    if !found.broken.is_empty() {
        let Asked::Break(why) = asked else {
            return Err(format!(
                "This breaks blueprints written for the program as it was:\n{}\n\
                 If it is meant, say so and why, and it is written down in contract/{BROKEN}:\n  \
                 BREAK_CONTRACT=\"why\" cargo test -p api the_contract",
                list(&broken)
            ));
        };
        let why = why.trim();
        if why.is_empty() || why == "1" {
            return Err(
                "BREAK_CONTRACT is the sentence written down beside what was broken: say why it was meant"
                    .into(),
            );
        }
        let path = dir.join(BROKEN);
        let mut kept = std::fs::read_to_string(&path).unwrap_or_default();
        for (what, became) in &found.broken {
            kept.push_str(&format!("{today}\t{what}\t{became}\t{why}\n"));
        }
        std::fs::write(&path, kept).map_err(|err| format!("could not write {}: {err}", path.display()))?;
        return write(offered);
    }
    match asked {
        Asked::Hold => Err(format!(
            "This program offers what the kept contract (contract/{OFFERED}) does not say:\n{}\n\
             Nothing written before breaks on these. Write the contract again:\n  \
             WRITE_CONTRACT=1 cargo test -p api the_contract",
            list(&found.added)
        )),
        Asked::Write | Asked::Break(_) => write(offered),
    }
}

/// The day a moment falls on, as `2026-11-12`, from milliseconds since
/// 1970: for the date a break is written down under.
pub fn day(ms: i64) -> String {
    // Days to a date in the calendar as it is counted now (Howard Hinnant's
    // `civil_from_days`).
    let days = ms.div_euclid(86_400_000) + 719_468;
    let era = days.div_euclid(146_097);
    let of_era = days.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let day_of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 { shifted + 3 } else { shifted - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
