//! A project's feature flags, as plain values: what a flag is
//! called and the variable it is read by, what it may say in an
//! environment, whether a change of it there waits for somebody the
//! environment's rule names as approving, which flags are past the day
//! they were to go, and a percentage flag started as a comparison.
//!
//! A flag is a row of the project's `flags` table (`std/flags`): its name,
//! what it is for, whose it is, the day it is to go and what kind of value
//! it says. **What it says in an environment is kept in one place, that
//! environment's variables** (`FLAG_<NAME>`, [`variable`]), which is what
//! an app the project deploys reads as it reads any setting, and what a
//! blueprint run under the environment reads as `environment.vars`: so
//! nothing is fetched from anywhere and nothing is sent, and there is no
//! second copy to come apart from it. The row's history keeps each change.
//!
//! A value is one of three kinds ([`Kind`]): on or off; a percentage, a
//! whole number from 0 to 100; or a word. Where an environment says
//! nothing of a flag it is off ([`off`]), as an app reading a variable
//! that is not set takes its own default.
//!
//! Turning a flag on, or wider ([`widens`]: off to on, a larger
//! percentage, another word), where the environment's rule names who
//! approves what goes under it, waits for one of them unless whoever asks
//! is one of them ([`asks`]); turning one off, or narrower, never waits,
//! since that is what a flag is for when something goes wrong.

use serde_json::{Value, json};

use crate::compared::{Compare, Counted, Way};

/// The table a project's flags are rows of.
pub const TABLE: &str = "flags";

/// What every flag's variable begins with. An environment's variables of
/// this name are the flags' own, and are changed as a flag and never by
/// hand ([`is_flags`]).
pub const PREFIX: &str = "FLAG_";

/// The longest a flag's name may come to, as its key.
pub const LONGEST: usize = 60;

/// The kinds of value a flag says, as its row says them.
pub const KINDS: [&str; 3] = ["switch", "percentage", "word"];

/// What kind of value a flag says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// On or off.
    Switch,
    /// A whole number from 0 to 100: how much of something it is on for.
    Percentage,
    /// A word: which of several ways something is done.
    Word,
}

impl Kind {
    /// As a row says it; a row that says nothing, or what is not a kind,
    /// is a switch.
    pub fn read(word: &str) -> Kind {
        match word.trim() {
            "percentage" => Kind::Percentage,
            "word" => Kind::Word,
            _ => Kind::Switch,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Kind::Switch => "switch",
            Kind::Percentage => "percentage",
            Kind::Word => "word",
        }
    }
}

/// What a flag is known by: its name written lowercase, each run of what
/// is not a letter or a digit a `-` (`New checkout` is `new-checkout`).
/// Nothing, where the name has no letter or digit.
pub fn key(name: &str) -> String {
    let mut key = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            key.push(ch.to_ascii_lowercase());
        } else if !key.is_empty() && !key.ends_with('-') {
            key.push('-');
        }
    }
    key.trim_end_matches('-').to_string()
}

/// Why `name` cannot be a flag's name, if it cannot.
pub fn refused_name(name: &str) -> Option<String> {
    let key = key(name);
    if key.is_empty() {
        return Some(format!(
            "`{name}` cannot be a flag's name: a flag is called by letters and digits, such as `new-checkout`"
        ));
    }
    (key.len() > LONGEST)
        .then(|| format!("`{name}` is too long a name for a flag: {LONGEST} letters at most"))
}

/// The variable an environment says `name`'s value in: `FLAG_NEW_CHECKOUT`.
pub fn variable(name: &str) -> String {
    format!("{PREFIX}{}", key(name).to_ascii_uppercase().replace('-', "_"))
}

/// Whether `variable` is a flag's, which is changed as a flag and never
/// as a variable of the environment by itself.
pub fn is_flags(variable: &str) -> bool {
    variable.to_ascii_uppercase().starts_with(PREFIX)
}

/// What a flag of `kind` says where its environment says nothing.
pub fn off(kind: Kind) -> Value {
    match kind {
        Kind::Switch => json!("off"),
        Kind::Percentage => json!(0),
        Kind::Word => json!(""),
    }
}

/// What `asked` comes to as a value of `kind`, written as it is kept: a
/// switch `on` or `off` (true and false read as those), a percentage a
/// whole number from 0 to 100 (`25` or `25%`), a word lowercase letters,
/// digits, `-`, `_` and `.`, or nothing for none. Refused, in a sentence,
/// where it is none of those.
pub fn said(kind: Kind, asked: &Value) -> Result<Value, String> {
    let text = match asked {
        Value::String(text) => text.trim().to_string(),
        Value::Bool(true) => "on".to_string(),
        Value::Bool(false) => "off".to_string(),
        Value::Number(number) => number.to_string(),
        _ => String::new(),
    };
    match kind {
        Kind::Switch => match text.to_ascii_lowercase().as_str() {
            "on" | "true" => Ok(json!("on")),
            "off" | "false" => Ok(json!("off")),
            _ => Err(format!("a flag that is a switch is `on` or `off`, and `{text}` is neither")),
        },
        Kind::Percentage => {
            let number =
                text.trim_end_matches('%').trim().parse::<u64>().ok().filter(|number| *number <= 100);
            number.map(|number| json!(number)).ok_or_else(|| {
                format!("a flag that is a percentage is a whole number from 0 to 100, such as `25`, and `{text}` is not")
            })
        }
        Kind::Word => {
            let shaped = text.len() <= 40
                && text.chars().all(|ch| {
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_' | '.')
                });
            match shaped {
                true => Ok(json!(text)),
                false => Err(format!(
                    "a flag that is a word says lowercase letters, digits, `-`, `_` and `.`, 40 at most, such as \
                     `blue`, and `{text}` is not that"
                )),
            }
        }
    }
}

/// How a value of `kind` is said to people: `on`, `off`, `25%`, a word,
/// or `nothing` for an empty word.
pub fn shown(kind: Kind, value: &Value) -> String {
    match (kind, value) {
        (Kind::Percentage, Value::Number(number)) => format!("{number}%"),
        (Kind::Word, Value::String(word)) if word.is_empty() => "nothing".to_string(),
        (_, Value::String(text)) => text.clone(),
        (_, other) => other.to_string(),
    }
}

/// Whether `now` turns a flag of `kind` on, or wider, from `was` (what
/// its environment says, where it says anything): off to on, a larger
/// percentage, or another word than it said. Turning it off, a smaller
/// percentage and taking a word away (none) do not.
pub fn widens(kind: Kind, was: Option<&Value>, now: &Value) -> bool {
    let was = was.cloned().unwrap_or_else(|| off(kind));
    match kind {
        Kind::Switch => now.as_str() == Some("on") && was.as_str() != Some("on"),
        Kind::Percentage => now.as_u64().unwrap_or(0) > was.as_u64().unwrap_or(0),
        Kind::Word => now.as_str() != Some("") && *now != was,
    }
}

/// Whether a change of a flag in an environment whose rule names `rule`
/// as approving, asked by `by` (an actor; a key is `key:<id>`, never its
/// maker), waits for one of them: where it widens it and `by` is not one
/// of them, and never where the rule names nobody.
pub fn asks(rule: &[String], by: &str, widens: bool) -> bool {
    widens && crate::go::first(rule, &[by])
}

/// What a change of a flag comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Came {
    /// It says that already: nothing is written.
    Same,
    /// It is written now, as whoever asked.
    Set(Value),
    /// It waits for one of those the environment's rule names.
    Asks(Value),
}

/// What asking a flag of `kind` to say `asked` in an environment that says
/// `was` of it, whose rule names `rule`, comes to for `by`: refused where
/// it is not a value of its kind.
pub fn change(
    kind: Kind,
    was: Option<&Value>,
    asked: &Value,
    rule: &[String],
    by: &str,
) -> Result<Came, String> {
    let now = said(kind, asked)?;
    let as_it_is = was.cloned().unwrap_or_else(|| off(kind));
    if now == as_it_is {
        return Ok(Came::Same);
    }
    match asks(rule, by, widens(kind, was, &now)) {
        true => Ok(Came::Asks(now)),
        false => Ok(Came::Set(now)),
    }
}

/// A flag past the day it was to go.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Due {
    /// The flag's row, by its id, and what it is called.
    pub record: String,
    pub name: String,
    /// Whose it is, as an actor; nothing where its row names nobody.
    pub owner: String,
    /// The day it was to go, as its row says it (`2026-11-30`).
    pub until: String,
    /// What whose it is is told.
    pub says: String,
}

/// Which of a project's flags (`rows`, as `rows.read` gives them: each its
/// `id`, `name`, `owner` and `until`) are past the day they were to go on
/// `today` (`2026-11-30`): those whose day is before it, each with the
/// sentence its owner is told. A row with no day, or one that cannot be
/// read as a day, is never due.
pub fn due(rows: &[Value], today: &str) -> Vec<Due> {
    let text =
        |row: &Value, key: &str| row.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_string();
    rows.iter()
        .filter_map(|row| {
            let until = text(row, "until");
            let day = until.get(..10).filter(|day| is_day(day))?.to_string();
            if day.as_str() >= today {
                return None;
            }
            let name = text(row, "name");
            Some(Due {
                record: text(row, "id"),
                says: format!(
                    "The flag {name} was to be taken out by {day}, and is still there. Take it out of the code \
                     that reads {var}, and then its row; or give it a later day.",
                    var = variable(&name)
                ),
                owner: text(row, "owner"),
                until: day,
                name,
            })
        })
        .collect()
}

fn is_day(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes.iter().enumerate().all(|(at, byte)| at == 4 || at == 7 || byte.is_ascii_digit())
}

/// The day of `ms` (milliseconds since 1970 began, in UTC), as a row
/// writes a day: `2026-11-30`.
pub fn day_of(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    // Days to a civil date (Howard Hinnant's way, for the proleptic
    // Gregorian calendar).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// A percentage flag started as a comparison on real work: `blueprint`,
/// started for each thing its param `per` names, two ways, `on` for the
/// flag's part of the things and `off` for the rest, each handed its word
/// as the blueprint's param `param`, and counted as `counts` says. What
/// comes of each way is counted and never scored.
pub struct Compared<'a> {
    pub flag: &'a str,
    pub environment: &'a str,
    /// What the flag says there: a whole number from 0 to 100.
    pub percentage: &'a Value,
    pub project: &'a str,
    pub blueprint: &'a str,
    pub per: &'a str,
    pub param: &'a str,
    pub counts: Vec<Counted>,
}

/// What [`Compared`] asks for as a comparison, or why it cannot be one:
/// the flag is no percentage here, or is all or nothing (one way would be
/// given every thing), or what the blueprint is handed is not said.
pub fn compared(asked: Compared<'_>) -> Result<Compare, String> {
    let Some(part) = asked.percentage.as_u64().filter(|part| *part <= 100) else {
        return Err(format!(
            "`{}` says no percentage in {}, and only a percentage is started as a comparison",
            asked.flag, asked.environment
        ));
    };
    if part == 0 || part == 100 {
        return Err(format!(
            "`{}` is {part}% in {}: every thing would go one way, and there would be nothing to put beside it. \
             Give it a part between 1% and 99% first",
            asked.flag, asked.environment
        ));
    }
    for (what, said) in [("blueprint", asked.blueprint), ("per", asked.per), ("param", asked.param)] {
        if said.trim().is_empty() {
            return Err(format!(
                "a flag started as a comparison says `{what}`: the blueprint started for each thing (`blueprint`), \
                 its param that says which thing (`per`) and its param handed `on` or `off` (`param`)"
            ));
        }
    }
    let way = |name: &str, share: u64| {
        let mut with = crate::Fields::new();
        with.insert(asked.param.to_string(), json!(name));
        serde_json::from_value::<Way>(json!({ "name": name, "share": share, "with": with }))
            .map_err(|wrong| wrong.to_string())
    };
    Ok(Compare {
        name: format!("flag-{}-{}", key(asked.flag), asked.environment),
        project: Some(asked.project.to_string()),
        of: asked.blueprint.to_string(),
        per: Some(asked.per.to_string()),
        also: Vec::new(),
        also_per: None,
        ways: vec![way("on", part)?, way("off", 100 - part)?],
        counts: asked.counts,
        asked: false,
        rule: None,
    })
}
