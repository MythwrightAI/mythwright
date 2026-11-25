//! Where a text stands among the blueprints that ship, said in its summary.
//!
//! A blueprint that ships is the default. A project changes what it does
//! by telling it something else, by changing the rows of a table it
//! makes, or by **forking** it: its text is copied into the project,
//! where the copy is what runs under the name and is the project's to
//! change. Both texts then have something to say that neither says of
//! itself, and this is where it is said, as lines and parts of the
//! summary ([`placed`]):
//!
//! - **One that ships** says how what it does is changed, in the order
//!   somebody should reach for them: a row of the table a list it is one
//!   of the usual things of is changed by, or that lays something into a
//!   list it goes through ([`usual`]); what it may be told; the rows of
//!   the tables it makes whose rows set what something does, each with
//!   what its step says the table is;
//!   for a rule, which is asked and never run, how a project writes its
//!   own; the files it links, each of which a project's own file of the
//!   same name is read in place of, and how many files what it runs
//!   links; and a fork. Where the project it is looked at for has a file
//!   of its own under the name, it says first where that stands ([`Own`]):
//!   what runs there; there and not in use yet, since it was written
//!   outside the factory; or passed over, since the project is not
//!   trusted.
//!   And one that another runs says, after what it says it is, what it
//!   is a part of ([`crate::names`]): the blueprints somebody applies.
//! - **A fork** says first what it was forked from; then where it
//!   differs from that; then, only where it is so, that the default has
//!   moved since and how; and last how the project goes back to the
//!   default, with what is lost by it.
//!
//! All of it is worked out from plain values that whoever asks hands
//! over ([`Standing`]): the texts, and what was kept when the fork was
//! made. No daemon is read here, nothing is merged, and nothing is made
//! up: a fork an earlier program made, of which nothing was kept, says
//! what its texts show and that the rest is not known. What moved
//! between two texts is said in the words it is said in everywhere
//! ([`crate::differs`]), each with what it is about in the fork's text
//! so that a page can show where. A text laid out another way comes to
//! the same lines.
//!
//! A line that leads somewhere says where ([`Opens`]): the default, the
//! fork, a table, or one of the three things a page does for it (apply
//! it, fork it, go back to the default). A terminal prints the words.

use serde_json::Value;

use super::{Known, Line, Opens, Part, Summary, capital, sentence, steps};
use crate::differs::{Difference, between, lead};
use crate::model::{Blueprint, StepKind};
use crate::template::has_holes;
use crate::{means_the_same, version};

/// Where a text stands among the blueprints that ship.
pub enum Standing<'a> {
    /// One that ships, as it ships. `own` is the file of the project it
    /// is looked at for under its name, where that project has one, with
    /// where it stands there. `part_of` is each blueprint somebody
    /// applies that runs it, by name, where it is a part
    /// ([`crate::names::part_of`]); `ways` what else whoever asks knows of
    /// how what it does is changed.
    Ships { own: Option<Own<'a>>, part_of: &'a [String], ways: &'a Ways },
    /// A project's fork of one that ships.
    Fork(Fork<'a>),
    /// A project's own file under the name of one that ships that is no
    /// fork that runs: the project is not trusted, and it is passed over.
    Passed(Own<'a>),
}

/// A project's own file under the name of a text that ships, and where
/// it stands in the project.
pub struct Own<'a> {
    /// The file, within the project.
    pub file: &'a str,
    pub is: Is<'a>,
}

/// Where a project's own file under the name of a text that ships
/// stands: worked out by whoever asks, from what the runner keeps.
pub enum Is<'a> {
    /// It is what runs in the project under the name.
    InUse,
    /// It is there, and what is running in the project goes on with what
    /// was there when it was applied: the file was written outside the
    /// factory, or changed there, since. Each is what is running, with
    /// the command that has it take the project's files as they are.
    NotYet { running: &'a [Running] },
    /// It is passed over for the one that ships: the project was made of
    /// a folder that was already there and is not trusted. `trust` is the
    /// command that trusts it.
    PassedOver { trust: &'a str },
}

/// Something running in a project that goes on with another text under
/// a name than the project's own file.
pub struct Running {
    /// What it is called.
    pub key: String,
    /// The command that applies it again taking the project's files as
    /// they are.
    pub take: String,
}

/// What whoever asks knows, beside a text, of how what it does is
/// changed: plain values, from the other texts that ship.
#[derive(Clone, Debug, Default)]
pub struct Ways {
    /// Each list it is one of the usual things of: the blueprint that
    /// goes through it, and its order there ([`usual`]).
    pub usually: Vec<Usually>,
    /// The files that what it runs links, by what runs them: each with
    /// how many.
    pub linking: Vec<(String, usize)>,
    /// The file a project's own text of its name is, within the project.
    pub own_as: Option<String>,
    /// The folder the project keeps its own blueprints in, as a path
    /// from its top, where it is not `blueprints`.
    pub own_in: Option<String>,
    /// Where the project keeps no blueprints of its own, and so has no
    /// fork: the command that gives it a folder for them.
    pub keeps_none: Option<String>,
    /// The table whose rows lay something into a list, and the command
    /// that adds one, where whoever asks has them.
    pub rows: Option<(String, String)>,
}

/// One of the usual things of a list: the blueprint that goes through
/// the list, and where in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Usually {
    pub by: String,
    pub order: f64,
}

/// A fork, as plain values.
pub struct Fork<'a> {
    /// The blueprint that ships it is a fork of, by its name.
    pub of: &'a str,
    /// Its own text, as it now is.
    pub text: &'a str,
    /// What was kept when it was made; nothing for one an earlier
    /// program made.
    pub made: Option<Made<'a>>,
    /// For one of which nothing was kept: a text of the default that
    /// calls itself the version the fork calls itself, where there is
    /// one. It is what the fork is taken to have been made from, and
    /// said to be only taken so.
    pub taken: Option<&'a str>,
    /// What ships under the name now; nothing where nothing does.
    pub ships: Option<&'a str>,
    /// The file it is, with where it stands in its project, where whoever
    /// asks knows.
    pub own: Option<Own<'a>>,
}

impl Fork<'_> {
    /// Whether it is a fork of words a blueprint links, a prompt, and no
    /// blueprint: words say no version, and are the same only where they
    /// are the same words.
    fn words(&self) -> bool {
        crate::parts::is_linked(self.of)
    }

    fn same(&self, one: &str, other: &str) -> bool {
        match self.words() {
            true => one == other,
            false => means_the_same(one, other),
        }
    }

    /// The version `text` calls itself, where it is a blueprint's.
    fn version(&self, text: &str) -> Option<String> {
        version(text).filter(|_| !self.words())
    }

    /// A version as it is said after its name; nothing for a text that says none.
    fn at_version(&self, text: &str) -> String {
        self.version(text).map(|said| format!(" version {said}")).unwrap_or_default()
    }
}

/// What was kept when a fork was made.
pub struct Made<'a> {
    /// The text it was copied from, to the letter.
    pub from: &'a str,
    /// Who made it, by name.
    pub by: &'a str,
    /// When, in milliseconds since 1970.
    pub at: i64,
}

/// What an action that makes a table says it makes: the one thing made
/// whose rows are somebody's to change, and so the one that is looked
/// for among what a blueprint makes.
const TABLE: &str = "a table";

/// A day as it is written, in universal time: `2026-11-20`.
pub fn day(millis: i64) -> String {
    // Days since 1970 to a date of the calendar in use, by eras of 400 years.
    let days = millis.div_euclid(86_400_000) + 719_468;
    let (era, of_era) = (days.div_euclid(146_097), days.rem_euclid(146_097));
    let year_of_era = (of_era - of_era / 1_460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * of_year + 2) / 153;
    let (day, month) = (of_year - (153 * shifted + 2) / 5 + 1, (shifted + 2) % 12 + 1);
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn opening(line: Line, what: &str, name: Option<&str>) -> Line {
    Line { opens: Some(Opens { what: what.to_string(), name: name.map(str::to_string) }), ..line }
}

/// What moved, as lines: each a sentence, about what the difference is
/// about where `placed` says the text the summary is of is the later one.
fn lines(moved: Vec<Difference>, placed: bool) -> Vec<Line> {
    // What has lines under it is said by them: each is about its own place.
    let line = |one: Difference| Line {
        text: sentence(&capital(&one.text)),
        at: one.at.filter(|_| placed && one.under.is_empty()),
        under: lines(one.under, placed),
        ..Line::default()
    };
    moved.into_iter().map(line).collect()
}

/// What moved between two texts that do not mean the same, as lines.
/// Words a blueprint links are no blueprint, and of them it is said that
/// their words changed and which blanks came or went
/// ([`crate::parts::differences`]).
fn moved(fork: &Fork, was: &str, now: &str, placed: bool) -> Vec<Line> {
    if fork.words() {
        return crate::parts::differences(was, now)
            .iter()
            .map(|one| Line::of(sentence(&capital(one))))
            .collect();
    }
    match lines(between(was, now), placed) {
        // Another text, and no part of it that is said by name is another.
        moved if moved.is_empty() => {
            vec![Line::of("Only in the version it says of itself, or in the order its parts are written.")]
        }
        moved => moved,
    }
}

/// The tables a blueprint makes whose rows set what something does, by
/// the name each is written with, each with what the step that makes it
/// says it is, as far as its first full stop. Which those are is what the
/// action says of itself ([`crate::Makes::setting`]): a table whose step
/// says the argument it names. A table whose rows are what it holds (a
/// board's cards, a library's documents) is none: changing those changes
/// nothing it does. Nor is one whose step says its rows are written only
/// by what keeps it (`by_hand: false`, as a table is told so where it is
/// made): nobody is to change them, so nobody is offered to.
fn tables(blueprint: &Blueprint, known: &Known) -> Vec<(String, Option<String>)> {
    let mut found: Vec<(String, Option<String>)> = Vec::new();
    for placed in steps(blueprint) {
        let StepKind::Do(action) = &placed.step.kind else { continue };
        let Some(makes) =
            known.action(action).and_then(|spec| spec.makes).filter(|makes| makes.what == TABLE)
        else {
            continue;
        };
        let sets = makes.setting.as_ref().and_then(|argument| placed.step.with.get(argument));
        if sets.is_none_or(Value::is_null) || placed.step.with.get("by_hand") == Some(&Value::Bool(false)) {
            continue;
        }
        let name = placed.step.with.get(&makes.named).and_then(Value::as_str);
        if let Some(name) = name.filter(|name| !has_holes(name) && !found.iter().any(|(one, _)| one == name))
        {
            let is = placed.step.description.as_deref().map(super::says).filter(|said| !said.is_empty());
            found.push((name.to_string(), is));
        }
    }
    found
}

/// One thing a list a text goes through is usually: the blueprint that
/// does it, where its name is written whole, and where in the list.
#[derive(Clone, Debug, PartialEq)]
pub struct Usual {
    /// Nothing, where what does it is worked out as the text runs: what
    /// it is told.
    pub does: Option<String>,
    pub order: f64,
}

/// The usual things of the lists a text goes through, in the order they
/// are written, each once: what a step hands as `usual`, a list of
/// things each with the blueprint that `does` it and its `order`, which
/// a row of a project's table lays something before, after or in place
/// of by an order of its own. A text that hands none goes through none.
pub fn usual(blueprint: &Blueprint) -> Vec<Usual> {
    let mut found: Vec<Usual> = Vec::new();
    for placed in steps(blueprint) {
        let Some(list) = placed.step.with.get("usual").and_then(Value::as_array) else { continue };
        for one in list {
            let (Some(order), Some(does)) =
                (one.get("order").and_then(Value::as_f64), one.get("does").and_then(Value::as_str))
            else {
                continue;
            };
            let does = Some(does.to_string()).filter(|does| !has_holes(does));
            let usual = Usual { does, order };
            if !found.contains(&usual) {
                found.push(usual);
            }
        }
    }
    found
}

/// An order as it is written: `10`, and `2.5` where it is not whole.
fn order(at: f64) -> String {
    if at.fract() == 0.0 && at.abs() < 1e15 { format!("{}", at as i64) } else { format!("{at}") }
}

/// Words joined as a sentence lists them: `a`, `a and b`, `a, b and c`.
fn listed(words: &[String]) -> String {
    match words {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// How a row of the table that lays things into a list changes what is
/// done there, as a line that opens the table: for a text that goes
/// through a list, what it does as it ships; for one that is one of the
/// usual things of a list, where. Nothing where whoever asks has no such
/// table, or the text is neither.
fn laid(blueprint: &Blueprint, ways: &Ways) -> Option<Line> {
    let (table, command) = ways.rows.as_ref()?;
    let own = usual(blueprint);
    let text = if !own.is_empty() {
        let each: Vec<String> = own
            .iter()
            .map(|one| match &one.does {
                Some(does) => format!("`{does}` at {}", order(one.order)),
                None => format!("what it is told at {}", order(one.order)),
            })
            .collect();
        format!(
            "Change what it goes through by a row of the project's `{table}` table: as it ships it does {}, and a \
             row lays something before any of them by a lesser order, in its place by the same order, or after \
             it by a greater: `{command}`.",
            listed(&each)
        )
    } else if !ways.usually.is_empty() {
        let each: Vec<String> = ways
            .usually
            .iter()
            .map(|one| format!("at {} in what `{}` goes through", order(one.order), one.by))
            .collect();
        format!(
            "Lay something before it, in its place or after it by a row of the project's `{table}` table: it is \
             done {}, so a row there at a lesser order is done before it, one at the same order in its place, \
             and one at a greater after it: `{command}`.",
            listed(&each)
        )
    } else {
        return None;
    };
    Some(opening(Line::of(text), "table", Some(table)))
}

/// Whether a text is a rule: asked and never run, it has what it gives
/// out and no steps.
fn is_rule(blueprint: &Blueprint) -> bool {
    blueprint.flow.steps.is_empty() && blueprint.triggers.is_empty() && !blueprint.outputs.is_empty()
}

/// How a project writes a rule of its own, as a line.
fn rule(blueprint: &Blueprint, ways: &Ways) -> Line {
    let names =
        |keys: Vec<&String>| listed(&keys.into_iter().map(|key| format!("`{key}`")).collect::<Vec<_>>());
    let file = match &ways.own_as {
        Some(file) => format!(", `{file}`,"),
        None => String::new(),
    };
    let shown = names(blueprint.params.iter().map(|param| &param.name).collect());
    let shown = if shown.is_empty() { String::new() } else { format!(" from what it is shown ({shown})") };
    Line::of(format!(
        "Write the project's own: it is asked and never run, so a file of the project's own under its name{file} \
         is asked in its place and says {}{shown}. Saved through the factory it is asked from then on; written in \
         an editor, once what is running there is applied again saying to take the project's files as they are; \
         and in a folder somebody gave, only once the project is trusted.",
        names(blueprint.outputs.keys().collect())
    ))
}

/// The files a text links, each with the project's own file that is
/// read in its place, as a line; and how many files what it runs links,
/// as another. Nothing where there are none.
fn linked(blueprint: &Blueprint, ways: &Ways) -> Vec<Line> {
    let mut said = Vec::new();
    let mut files: Vec<String> = Vec::new();
    for link in crate::parts::linked(blueprint) {
        let name = crate::parts::linked_as(&blueprint.name, link.from);
        if !files.contains(&name) {
            files.push(name);
        }
    }
    if !files.is_empty() {
        let mut line = Line::of(
            "Change the words it puts together: each file it links is read from a file of the project's own of \
             the same name in its place, where there is one, saved through the factory as a blueprint of the \
             project's is:",
        );
        let folder = ways.own_in.as_deref().unwrap_or("blueprints");
        line.under =
            files.iter().map(|name| Line::of(format!("`{folder}/{name}`, in place of `{name}`"))).collect();
        said.push(line);
    }
    if !ways.linking.is_empty() {
        let all: usize = ways.linking.iter().map(|(_, count)| count).sum();
        let mut line = Line::of(format!(
            "Change the words what it runs puts together: {all} files are linked by what it runs, each read from \
             a file of the project's own of the same name in its place, where there is one; the summary of each \
             names its own:"
        ));
        line.under = ways
            .linking
            .iter()
            .map(|(by, count)| {
                let files = if *count == 1 { "1 file".to_string() } else { format!("{count} files") };
                opening(Line::of(format!("`{by}`: {files}")), "default", Some(by))
            })
            .collect();
        said.push(line);
    }
    said
}

/// A way to change what it does, taken apart for a page that lists them:
/// what is done, as its label, and what is said of it after, as its
/// title, where its words say the one and then the other after a colon.
/// What is under it is taken apart the same way. Its words are as they were.
fn acting(mut line: Line) -> Line {
    if let Some((what, said)) = line.text.split_once(": ") {
        line.label = Some(what.to_string());
        line.title = Some(said.to_string());
    }
    line.under = line.under.into_iter().map(acting).collect();
    line
}

/// How what a blueprint that ships does is changed, in the order to
/// reach for them. `own` is the project's file under its name, where it
/// has one.
fn change(blueprint: &Blueprint, known: &Known, own: Option<&Own>, ways: &Ways) -> Vec<Line> {
    changes(blueprint, known, own, ways).into_iter().map(acting).collect()
}

fn changes(blueprint: &Blueprint, known: &Known, own: Option<&Own>, ways: &Ways) -> Vec<Line> {
    let mut said: Vec<Line> = laid(blueprint, ways).into_iter().collect();
    let asked = is_rule(blueprint);
    // A rule is told what it is asked with by whoever asks, and applying it does nothing.
    if !blueprint.params.is_empty() && !asked {
        let line = Line::of(
            "Tell it something else: apply it again, saying other than it was told. Nothing is copied, and it is \
             still the default.",
        );
        said.push(opening(line, "apply", None));
    }
    let tables = tables(blueprint, known);
    if !tables.is_empty() {
        let mut line = Line::of(match tables.len() {
            1 => "Change the rows of the table it makes: what it does from then on goes by what they say.",
            _ => "Change the rows of a table it makes: what it does from then on goes by what they say.",
        });
        line.under = tables
            .iter()
            .map(|(name, is)| {
                let text = match is {
                    Some(is) => format!("`{name}`: {is}"),
                    None => format!("`{name}`"),
                };
                opening(Line::of(text), "table", Some(name))
            })
            .collect();
        said.push(line);
    }
    // A project that keeps no blueprints of its own has none of them read
    // in place of what ships, and no fork.
    if let Some(command) = &ways.keeps_none {
        said.push(Line::of(format!(
            "This project keeps no blueprints of its own, so none of its files is read in place of this and it \
             has no fork of it. To have it keep them: `{command}`."
        )));
        return said;
    }
    if asked {
        said.push(rule(blueprint, ways));
    }
    said.extend(linked(blueprint, ways));
    said.push(match own.filter(|own| !matches!(own.is, Is::PassedOver { .. })) {
        Some(own) => opening(
            Line::of("Change this project's fork of it: its text is the project's own to change."),
            "fork",
            Some(own.file),
        ),
        None => opening(
            Line::of(
                "Fork it: its text is copied into a project, where the copy is what runs under the name and is \
                 that project's to change. The default stays as it is, and the fork says what it was forked from.",
            ),
            "forking",
            None,
        ),
    });
    said
}

/// Which of what is running there goes on with another text, in words:
/// `a` and `b`.
fn which(running: &[Running]) -> String {
    listed(&running.iter().map(|one| format!("`{}`", one.key)).collect::<Vec<_>>())
}

/// Where a project's own file under the name `name` of one that ships
/// stands there, as the line a summary says first: in use, not yet, or
/// passed over. It opens the file.
pub fn own_line(own: &Own, name: &str) -> Line {
    let file = own.file;
    let text = match &own.is {
        Is::InUse => format!(
            "This project runs its own fork of it, `{file}`: that file, and not this text, is what runs here \
             under the name."
        ),
        Is::NotYet { running } => not_yet(file, name, running),
        Is::PassedOver { trust } => passed(file, name, trust),
    };
    opening(Line::of(text), "fork", Some(file))
}

/// The same, as one sentence said of the file anywhere else than in a
/// summary: where it is saved, and where what a project has is listed.
pub fn own_sentence(own: &Own, name: &str) -> String {
    match &own.is {
        Is::InUse => format!(
            "`{}` is in use: it is what runs in this project under `{name}`, in place of the one that ships.",
            own.file
        ),
        Is::NotYet { running } => not_yet(own.file, name, running),
        Is::PassedOver { trust } => passed(own.file, name, trust),
    }
}

fn not_yet(file: &str, name: &str, running: &[Running]) -> String {
    let (was, goes, that) = if running.len() == 1 { ("was", "goes", "that") } else { ("were", "go", "each") };
    let take = listed(&running.iter().map(|one| format!("`{}`", one.take)).collect::<Vec<_>>());
    format!(
        "This project's own `{file}`, for `{name}`, is not in use yet: it was written outside the factory, or \
         changed there, since {} {was} applied, which {goes} on with what was there under the name then. Saved \
         here by somebody who holds what {that} runs with, it is in use; or apply {that} again saying to take the \
         project's files as they are: {take}.",
        which(running)
    )
}

fn passed(file: &str, name: &str, trust: &str) -> String {
    format!(
        "This project's own `{file}`, for `{name}`, is passed over, and the one that ships is what runs here: the \
         project was made of a folder that was already there, and its files do not stand in for what ships \
         until the factory's owner trusts it: `{trust}`."
    )
}

/// What one that ships is a part of, as a line: the blueprint somebody
/// applies that runs it, which the line opens; or, where several do,
/// each on a line of its own under it. Nothing for one that nothing runs.
fn part(of: &[String]) -> Option<Line> {
    let opens = |name: &String, text: String| opening(Line::of(text), "default", Some(name));
    match of {
        [] => None,
        [one] => Some(opens(one, format!("A part of `{one}`."))),
        several => {
            let mut line = Line::of(format!("A part of {} blueprints:", several.len()));
            line.under = several.iter().map(|name| opens(name, format!("`{name}`"))).collect();
            Some(line)
        }
    }
}

/// What a fork says first: what it was forked from, as far as is known.
fn from(fork: &Fork) -> Line {
    let of = fork.of;
    let text = match (&fork.made, fork.taken) {
        (Some(made), _) => {
            format!("Forked from `{of}`{} by {} on {}.", fork.at_version(made.from), made.by, day(made.at))
        }
        (None, Some(taken)) => format!(
            "Forked from `{of}`{}. That was before what a fork is made from was kept: by whom and when is not \
             known, and the version is the one its own text says.",
            fork.at_version(taken)
        ),
        (None, None) => format!(
            "Forked from `{of}`. That was before what a fork is made from was kept: from which version, by whom \
             and when is not known."
        ),
    };
    opening(Line::of(text), "default", Some(of))
}

/// The parts a fork has that no other text has: where it differs from
/// what it was forked from, how the default has moved since where it
/// has, and how the project goes back to the default. They are what
/// `own --say` prints and what its summary says.
pub fn parts(fork: &Fork) -> Vec<Part> {
    let part = |kind: &str, title: &str, lines: Vec<Line>| Part {
        kind: kind.to_string(),
        title: title.to_string(),
        ordered: false,
        lines,
    };
    let base = fork.made.as_ref().map(|made| made.from).or(fork.taken);
    let mut said = Vec::new();
    // Against what it was forked from, where that is known; against
    // what ships now where it is not, which is then all there is to
    // set it beside.
    let (beside, title, same) = match (base, fork.ships) {
        (Some(base), _) => (
            Some(base),
            "Where it differs from the default it was forked from",
            "Nowhere: it is the default as it was forked.",
        ),
        (None, ships) => (
            ships,
            "Where it differs from the default as it ships now",
            "Nowhere: it says what the default says.",
        ),
    };
    if let Some(beside) = beside {
        let differ = match fork.same(beside, fork.text) {
            true => vec![Line::of(same)],
            false => moved(fork, beside, fork.text, true),
        };
        said.push(part("differs", title, differ));
    }
    match (base, fork.ships) {
        (Some(base), Some(ships)) if !fork.same(base, ships) => {
            let now = lead(
                &format!("`{}`", fork.of),
                fork.version(base).as_deref(),
                fork.version(ships).as_deref(),
            );
            let first =
                Line::of(format!("{}: what ships now is not what this was forked from.", capital(&now)));
            let mut moved = vec![opening(first, "default", Some(fork.of))];
            moved.extend(self::moved(fork, base, ships, false));
            moved.push(Line::of(
                "The fork takes none of it by itself. Its text is this project's to change, with what moved said \
                 here beside it.",
            ));
            said.push(part("moved", "The default has moved since", moved));
        }
        (_, None) => said.push(part(
            "moved",
            "The default has moved since",
            vec![Line::of(format!(
                "Nothing ships as `{}` any more: the fork is all there is of it here.",
                fork.of
            ))],
        )),
        _ => {}
    }
    if let Some(ships) = fork.ships {
        let lost = match fork.same(ships, fork.text) {
            true => "Nothing is lost by it: the fork says what ships now.",
            false => "What the fork's text says is then gone, unless the project keeps it somewhere else.",
        };
        let line = Line::of(format!(
            "Take the fork away, and the one that ships is what runs here under the name again, once what is \
             running is applied. {lost}"
        ));
        said.push(part("back", "Going back to the default", vec![opening(line, "back", Some(fork.of))]));
    }
    said
}

/// A summary with what its text's standing among the blueprints that
/// ship says: the lines and parts of the head of this file.
pub fn placed(mut summary: Summary, blueprint: &Blueprint, known: &Known, standing: &Standing) -> Summary {
    let first = match standing {
        Standing::Ships { own, part_of, ways } => {
            summary.parts.push(Part {
                kind: "change".to_string(),
                title: "How to change what it does".to_string(),
                ordered: true,
                lines: change(blueprint, known, own.as_ref(), ways),
            });
            // What it is a part of is said after what it says it is.
            if let (Some(line), Some(is)) =
                (part(part_of), summary.parts.iter_mut().find(|part| part.kind == "is"))
            {
                let after =
                    usize::from(blueprint.description.as_deref().is_some_and(|said| !said.trim().is_empty()));
                is.lines.insert(after.min(is.lines.len()), line);
            }
            own.as_ref().map(|own| vec![own_line(own, &blueprint.name)])
        }
        Standing::Fork(fork) => {
            let mut more = parts(fork);
            // Going back is said last, under everything the fork is and does.
            let back = more.iter().position(|part| part.kind == "back").map(|at| more.remove(at));
            let after = summary.parts.iter().position(|part| part.kind == "is").map_or(0, |at| at + 1);
            summary.parts.splice(after..after, more);
            summary.parts.extend(back);
            // What it was forked from, and then, where it is not yet what runs, that.
            let not_yet = fork.own.as_ref().filter(|own| !matches!(own.is, Is::InUse));
            Some(std::iter::once(from(fork)).chain(not_yet.map(|own| own_line(own, fork.of))).collect())
        }
        Standing::Passed(own) => Some(vec![own_line(own, &blueprint.name)]),
    };
    if let (Some(lines), Some(is)) = (first, summary.parts.iter_mut().find(|part| part.kind == "is")) {
        is.lines.splice(0..0, lines);
    }
    summary
}
