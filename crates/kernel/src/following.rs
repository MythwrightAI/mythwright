//! Following something, as plain values: what may be followed, how often
//! whoever follows it is told, when a digest of what changed is due, which
//! things a change is of, and how what changed is said.
//!
//! **What is followed** is one of [`KINDS`], named by an id of its own
//! kind: a page of a wiki by its record, a board, a table or a library by
//! its project and name (`shop/board`), a channel by its id, a project by
//! its id. A page may be followed with every page under it, which then
//! takes in a page put under it later.
//!
//! **How often** ([`Often`]): at once, one line raised by each change;
//! hourly or daily, one line a period that sums up what changed in it
//! ([`due`], [`said`]); or not at all, followed and told nothing.
//!
//! Everything here is a function of what it is handed, with its cases in
//! the kernel's suite; whoever keeps follows and boxes does what it says.

use serde_json::Value;

use crate::topics::PARENT;

/// What may be followed, in the order they are listed.
pub const KINDS: &[&str] = &["page", "board", "table", "library", "channel", "project"];
/// The kinds whose things are tables of a project, named `<project>/<name>`.
pub const TABLES: &[&str] = &["board", "table", "library"];
/// The time of day a daily digest comes at where nothing is said.
pub const USUAL_AT: &str = "09:00";
/// An hour, in milliseconds.
pub const HOUR: i64 = 3_600_000;

/// How often whoever follows something is told of what changed in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Often {
    /// One line, raised by each change: what following always did.
    Once,
    /// One line an hour, summing up what changed in it.
    Hourly,
    /// One line a day, at a time of day of the follower's.
    Daily,
    /// Followed, and told nothing.
    Off,
}

impl Often {
    pub fn as_str(self) -> &'static str {
        match self {
            Often::Once => "once",
            Often::Hourly => "hourly",
            Often::Daily => "daily",
            Often::Off => "off",
        }
    }

    /// What a follow said as it is kept reads as; anything else is at once.
    pub fn kept(said: &str) -> Often {
        Often::parse(said).unwrap_or(Often::Once)
    }

    /// What somebody said, or why it is not how often one is told, in a
    /// sentence that says what is.
    pub fn parse(said: &str) -> Result<Often, String> {
        match said.trim().to_ascii_lowercase().as_str() {
            "" | "once" | "at once" | "now" => Ok(Often::Once),
            "hourly" | "hour" => Ok(Often::Hourly),
            "daily" | "day" => Ok(Often::Daily),
            "off" | "never" => Ok(Often::Off),
            _ => Err(format!(
                "`{said}` is not how often you are told: say `once` (at once), `hourly`, `daily` or `off`"
            )),
        }
    }

    /// Whether what changes waits for a digest.
    pub fn digests(self) -> bool {
        matches!(self, Often::Hourly | Often::Daily)
    }
}

/// The kind somebody said, as it is kept: one of [`KINDS`], or why not.
pub fn kind_of(said: &str) -> Result<&'static str, String> {
    let said = said.trim().to_ascii_lowercase();
    let said = if said == "wiki" || said == "topic" { "page".to_string() } else { said };
    KINDS
        .iter()
        .find(|kind| **kind == said)
        .copied()
        .ok_or_else(|| format!("`{said}` is nothing that can be followed: say {}", KINDS.join(", ")))
}

/// A time of day as `HH:MM` on a clock of twenty-four hours, or why it is
/// not one; [`USUAL_AT`] where nothing is said.
pub fn at_of(said: Option<&str>) -> Result<String, String> {
    let said = said.map(str::trim).filter(|said| !said.is_empty()).unwrap_or(USUAL_AT);
    let read = said.split_once(':').and_then(|(hour, minute)| {
        let whole = |part: &str| part.len() == 2 && part.bytes().all(|b| b.is_ascii_digit());
        (whole(hour) && whole(minute)).then(|| (hour.parse::<u8>().ok(), minute.parse::<u8>().ok()))
    });
    match read {
        Some((Some(hour), Some(minute))) if hour < 24 && minute < 60 => Ok(format!("{hour:02}:{minute:02}")),
        _ => Err(format!(
            "`{said}` is no time of day: write it as hours and minutes on a clock of twenty-four hours, as `09:00`"
        )),
    }
}

/// When a digest of what changed is due, the first change it holds made
/// at `first`: hourly, at the first whole hour after it; daily, at the
/// first time of day after it (`next_daily`, which knows the follower's
/// zone). Nothing for what is told at once or not at all, and nothing
/// where no time of day comes.
pub fn due(often: Often, first: i64, next_daily: impl Fn(i64) -> Option<i64>) -> Option<i64> {
    match often {
        Often::Hourly => Some((first.div_euclid(HOUR) + 1) * HOUR),
        Often::Daily => next_daily(first).filter(|at| *at > first),
        Often::Once | Often::Off => None,
    }
}

/// What a change of a page is to whoever follows it (`own`), and to
/// whoever follows a page above it with every page under it (`above`), and
/// what it is in a word or two (`noun`, `verb`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PageChange {
    pub own: bool,
    pub above: bool,
    pub noun: &'static str,
    pub verb: &'static str,
}

/// What an event of a page (`topic` and what it carries, `data`) is to
/// whoever follows it, where the page is `under` another now: whoever
/// follows the page is told what they always were
/// (`crate::topics::tells_followers`), and whoever follows a page above
/// it with its pages is told the same and of a page made under it, once:
/// not of the first revision that goes with its making.
pub fn page_change(topic: &str, data: &Value, under: bool) -> Option<PageChange> {
    // A page's first revision is its making, which its being made says.
    if topic == "topic.revised" && data.get("nth").and_then(Value::as_u64) == Some(1) {
        return None;
    }
    let own = crate::topics::tells_followers(topic, data, under);
    let (noun, verb) = match topic {
        "record.created" => ("page", "added"),
        _ if !own => return None,
        "record.commented" => ("reply", "said"),
        "topic.moved" | "topic.placed" => ("page", "moved"),
        _ => ("page", "changed"),
    };
    let above = own || topic == "record.created";
    Some(PageChange { own, above, noun, verb })
}

/// What an event that is no page's is to whoever follows what it is of:
/// the things it is of, each its kind and id (a table's rows are
/// `table`, whichever of [`TABLES`] it is), the table it is of where it is
/// of one, and what it did. Nothing for what nobody follows it for: what
/// is said of the factory's own tables (its topics, whose pages are
/// followed as pages), and anything not said here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThingChange {
    pub things: Vec<(&'static str, String)>,
    pub table: Option<(String, String)>,
    /// What it was to a row: nothing where that is the table's kind's
    /// (`row`, `card`, `document`), and a comment where something was
    /// said on one.
    pub noun: Option<&'static str>,
    pub verb: &'static str,
}

pub fn thing_change(topic: &str, data: &Value) -> Option<ThingChange> {
    let said = |key: &str| data.get(key).and_then(Value::as_str).filter(|said| !said.is_empty());
    let project = said("project");
    match topic {
        "record.created" | "record.updated" | "record.removed" | "record.commented" => {
            let (project, collection) = (project?, said("collection")?);
            if project == crate::place::FACTORY {
                return None;
            }
            let (noun, verb) = match topic {
                "record.created" => (None, "added"),
                "record.updated" => (None, "changed"),
                "record.removed" => (None, "removed"),
                _ => (Some("comment"), "said"),
            };
            Some(ThingChange {
                things: vec![("table", format!("{project}/{collection}")), ("project", project.to_string())],
                table: Some((project.to_string(), collection.to_string())),
                noun,
                verb,
            })
        }
        "channel.message" => {
            let mut things = vec![("channel", said("channel")?.to_string())];
            things.extend(
                project.filter(|project| *project != crate::place::FACTORY).map(|p| ("project", p.into())),
            );
            Some(ThingChange { things, table: None, noun: Some("message"), verb: "said" })
        }
        "project.renamed" | "project.placed" => Some(ThingChange {
            things: vec![("project", project?.to_string())],
            table: None,
            noun: Some("project"),
            verb: if topic == "project.renamed" { "renamed" } else { "moved" },
        }),
        _ => None,
    }
}

/// What a row of a table of `kind` is called, and what was done to it,
/// in words: a library's documents are written, whether added or changed.
pub fn worded(kind: &str, noun: Option<&'static str>, verb: &'static str) -> (&'static str, &'static str) {
    let noun = noun.unwrap_or(match kind {
        "board" => "card",
        "library" => "document",
        _ => "row",
    });
    match (noun, verb) {
        ("document", "added" | "changed") => ("document", "written"),
        other => other,
    }
}

/// The order what changed is said in.
const VERBS: &[&str] = &["changed", "said", "added", "written", "moved", "removed", "renamed"];

fn plural(noun: &str, count: usize) -> String {
    match (noun, count) {
        (_, 1) => noun.to_string(),
        ("reply", _) => "replies".to_string(),
        _ => format!("{noun}s"),
    }
}

/// What changed, from each kind of change with how many there were
/// (`noun`, `verb`, how many), in a few words: `6 pages changed, 3 new
/// replies, 1 page added`. What a project itself had done to it is said
/// alone (`renamed`). Nothing, where nothing changed.
pub fn said(counts: &[(String, String, usize)]) -> String {
    let mut ordered: Vec<&(String, String, usize)> = counts.iter().filter(|(.., count)| *count > 0).collect();
    ordered.sort_by_key(|(noun, verb, _)| {
        (VERBS.iter().position(|one| one == verb).unwrap_or(VERBS.len()), noun.clone())
    });
    let words: Vec<String> = ordered
        .iter()
        .map(|(noun, verb, count)| match (noun.as_str(), verb.as_str()) {
            ("project", verb) => verb.to_string(),
            (noun, "said") => format!("{count} new {}", plural(noun, *count)),
            (noun, verb) => format!("{count} {} {verb}", plural(noun, *count)),
        })
        .collect();
    words.join(", ")
}

/// Each kind of change among `changes` (each its noun and verb), counted,
/// in the order each first came.
pub fn counted<'a>(changes: impl IntoIterator<Item = (&'a str, &'a str)>) -> Vec<(String, String, usize)> {
    let mut counts: Vec<(String, String, usize)> = Vec::new();
    for (noun, verb) in changes {
        match counts.iter_mut().find(|(one, did, _)| one == noun && did == verb) {
            Some((.., count)) => *count += 1,
            None => counts.push((noun.to_string(), verb.to_string(), 1)),
        }
    }
    counts
}

/// What a line of something followed says: what it is called, and what
/// changed in it.
pub fn line_said(title: &str, counts: &[(String, String, usize)]) -> String {
    match (title.trim(), said(counts)) {
        ("", changed) => changed,
        (title, changed) if changed.is_empty() => title.to_string(),
        (title, changed) => format!("{title}: {changed}"),
    }
}

/// The key a thing is told by in a box, its kind and id: `table:shop/board`.
pub fn key(kind: &str, thing: &str) -> String {
    format!("{kind}:{thing}")
}

/// A key's kind and id, where it is one: of a digest's key too, which is
/// the key and the period it is of after a `#`.
pub fn of_key(key: &str) -> Option<(&str, &str)> {
    let (kind, thing) = key.split_once(':')?;
    let thing = thing.split_once('#').map_or(thing, |(thing, _)| thing);
    (KINDS.contains(&kind) && !thing.is_empty()).then_some((kind, thing))
}

/// The page a change of a page is under, as the page holds it: nothing for a wiki.
pub fn parent_of(data: &serde_json::Map<String, Value>) -> Option<&str> {
    data.get(PARENT).and_then(Value::as_str).filter(|said| !said.is_empty())
}
