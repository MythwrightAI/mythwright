//! What a record is made of: the fields its collection declares, and
//! whether what is written into one fits them.
//!
//! Most of what a field holds is checked by looking at the value alone: a
//! number is a number, a choice is one of what can be chosen. A `record`
//! field is the exception. It holds the id of a record of another
//! collection in the same project (a card's sprint), and whether there is
//! such a record can only be asked of where that collection is kept.
//!
//! It is asked when the field is written, and not again. The record it
//! names may be taken away afterwards, and with records kept as files a
//! person can do that without asking anybody; so a field may come to name
//! a record that is no longer there. That is read as it is written, and
//! whatever shows it says so.

use kernel::{Error, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::collections::{Collection, Collections, Store, record_id};
pub use crate::takes::Takes;
use crate::takes::listed;

/// The word a list of scopes says for what a record that holds it takes
/// of whoever writes the record: holding each one (`takes: each`). No
/// scope is called this.
pub const EACH: &str = "each";

/// What a field can hold. A `document` is Markdown kept in the row, as a
/// topic's body is, written as a library's document is written.
pub const KINDS: &[&str] =
    &["text", "choice", "actor", "list", "number", "flag", "date", "record", "records", "document"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    /// One of `KINDS`.
    pub kind: String,
    /// For a choice: what can be chosen, in order. For a record, or for
    /// records: the one collection whose records it can name.
    #[serde(default)]
    pub of: Vec<String>,
    #[serde(default)]
    pub required: bool,
    /// A change to it alone is not news of the record: no line of its
    /// history is written for one. For what a blueprint keeps on a record
    /// for its own purposes (a card's open question), where what happened
    /// is said by an event of its own and the field only shows it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quiet: bool,
    /// The scopes giving it a value takes, beside what changing the
    /// record does, every one of which is asked (`takes`, as a blueprint
    /// writes it: `takes: shell`, or `takes: [shell, secrets:use]`). For a
    /// field that is read by something with more authority than whoever
    /// may change the rest of the record: whether an agent's program has a
    /// wall; a param handed to a blueprint run as whoever applied a fleet,
    /// which may be a command or the name of a secret.
    #[serde(default, skip_serializing_if = "Takes::is_empty")]
    pub takes: Takes,
    /// The same of one value and not of the rest, by the value as it is
    /// written (`takes: { on: shell }`): for a field where only one way of
    /// filling it lets something at more than the record. Whether an
    /// agent's program goes on without the daemon: turning that on is
    /// letting a program run here with nobody watching, and turning it off
    /// again is not.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub takes_of: std::collections::BTreeMap<String, Takes>,
    /// For a list of scopes that something with more authority gives to
    /// somebody else (`takes: each`): the record gives what the list
    /// holds, so whoever adds the record, or changes anything of it,
    /// holds each scope it then says. What an agent's credential holds,
    /// in a row a fleet brings agents up by: the fleet gives it as
    /// whoever applied the fleet, and what the agent is told, how many
    /// there are of it and what runs it are cells of the same row, so
    /// what is asked of whoever applied the fleet is asked here of
    /// whoever writes the row.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub takes_each: bool,
    /// What giving it its value took is asked again of whoever adds the
    /// record or changes any cell of it, for as long as it says that value
    /// (`stands`, beside `takes`): for a cell that lets something with
    /// more authority do more, beside cells that say what that does. An
    /// agent's row with no wall: what it is told and how many there are of
    /// it is what runs with no wall ([`crate::stood`]).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stands: bool,
    /// What such a list stands for where a record says nothing of it
    /// (`unsaid`): the scopes that are given all the same, by whatever
    /// reads the record, and so are asked of whoever writes one that
    /// leaves it unsaid. Nothing here gives them: it is said so that the
    /// record is held to what it comes to. Of a choice, the one word of
    /// `of` nothing chosen stands for (a card's priority, `normal`): where
    /// it ranks, and what a number written before it was a choice is
    /// counted from (`kernel::ranked`, [`crate::worded`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unsaid: Vec<String>,
    /// Left out of a view of the collection until somebody shows it there
    /// (`hidden`, as a blueprint writes it): for what a blueprint keeps on
    /// a record for itself, which a person seldom reads. It is no more
    /// private for that. Whoever may read the collection may show it, and
    /// it is in the record wherever the record is read whole. Nothing
    /// here goes by it: it is said to whoever lays the records out.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// The words it offers whoever fills it (`suggests`): for a field
    /// that takes any word and has a few usual ones, which whoever
    /// declared it knows and whoever fills it may not. Nothing is held to
    /// them: another word is written as freely as it always was, and what
    /// a record holds is never checked against them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggests: Vec<String>,
    /// Its words are a time of day with the zone it is in after it
    /// (`zoned`, `02:00 Europe/Zagreb`): whoever fills it in the client
    /// has their own zone written after a time they write with none.
    /// Nothing here reads a zone or checks what a record holds by it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub zoned: bool,
    /// It keeps who last wrote the cell it names (`writer_of: environment`),
    /// written by the factory in every write that gives that cell a value
    /// or changes it ([`stamp`]) and by nobody else: so that whatever acts
    /// on the cell for somebody with more authority (a row of a project's
    /// triggers runs its pipeline under the environment it says, as whoever
    /// applied the triggers) can ask whether whoever wrote it may have that
    /// done (`kernel::go::first`). Whoever writes is an actor, a key that
    /// holds less being itself (`key:<id>`) and never its maker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writer_of: Option<String>,
    /// The icon its column is drawn with, as its table's declaration says
    /// (`icon`): a name ([`crate::icons`]), and nothing here knows what it
    /// is a drawing of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// The icon somebody who looks at the table chose for it, which
    /// stands over what the declaration says and is kept when the table
    /// is declared again. No declaration writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_chosen: Option<String>,
    /// Somebody who looks at the table added it ([`crate::added`]): it
    /// is the table's and no declaration's, and stands beside what is
    /// declared when the table is declared again. No declaration writes it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub added: bool,
    /// What somebody called its column, which stands over its name where
    /// the column is headed and nowhere else: a record, a file, a view
    /// and a blueprint go on calling it by its name. Kept when the table
    /// is declared again, as a chosen icon is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_chosen: Option<String>,
    /// For a choice: the order somebody put what can be chosen in, which
    /// `of` is then in. Kept so that a declaration saying its own order
    /// again does not put it back (`kernel::added::ordered`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub of_chosen: Vec<String>,
    /// For a choice somebody put in order: what they added to what it
    /// offers beyond its declaration (a board's column arranged in),
    /// which a declaration said again does not take away
    /// (`kernel::added::offered`). Nothing where that is not known, for a
    /// table kept before it was: then everything they chose stands. No
    /// declaration says it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of_added: Option<Vec<String>>,
    /// For a choice that takes a number as its word: its rows' numbers are
    /// being written as words (`crate::reworded`). Written with the
    /// declaration that begins it and taken off once every row is a
    /// word, so that whatever finds it next (a declaration, a start)
    /// finishes what a stop left. No declaration says it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub rewording: bool,
    /// Where it is among the fields a record is called by, from 1
    /// (`calls`): a record named somewhere is called by what those hold,
    /// in that order, where its `title` is what something else knows it
    /// by (an attempt of a card's A/B test, called by its card and its
    /// contender). Nothing here goes by it: it is said to whoever names a
    /// record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calls: Option<u32>,
    /// Its column is kept in order by an index from when the table is
    /// declared (`indexed`), so that a table of many rows is sorted and
    /// held by it from its first row ([`crate::indexes`]). Any column a
    /// large table is sorted by is indexed when first asked; this is said
    /// of one that is to be sorted by before anybody asks.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub indexed: bool,
    /// What every event about a record of the table says of the record
    /// carries what it holds (`said`), under `fields` of the event's
    /// data ([`crate::fieldsaid`]): so that a handler chosen by it (`for:`,
    /// a plain `when`), and whatever follows the events from outside,
    /// has it without anything being run. Never of a field that is
    /// `quiet`, or that says what giving it a value takes: what an event
    /// carries is heard by whoever may read the table.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub said: bool,
}

impl Field {
    /// For a field that names a record of a collection, or several of
    /// them (`record`, `records`), that collection.
    pub fn points_at(&self) -> Option<&str> {
        self.of.first().map(String::as_str).filter(|_| self.kind == "record" || self.kind == "records")
    }

    /// The records a value of this field names: the one, or each of the
    /// several; none, where it names nothing or is no such field.
    pub fn named<'a>(&self, value: &'a Value) -> Vec<&'a str> {
        match (self.points_at(), value) {
            (Some(_), Value::String(id)) => vec![id.as_str()],
            (Some(_), Value::Array(ids)) => ids.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        }
    }
}

/// Reads fields as a blueprint writes them: each a name and what it is.
pub(crate) fn read(written: Vec<(String, Map<String, Value>)>) -> Result<Vec<Field>> {
    // What can be chosen, and what a collection is called, are written as
    // lowercase words joined by dashes, so that each is said the same way
    // everywhere.
    let word = |value: &Value| blueprint::expr::slug(&blueprint::expr::display(value));
    let mut read = Vec::new();
    for (name, about) in written {
        let kind = about.get("type").and_then(Value::as_str).unwrap_or("text").to_string();
        if !KINDS.contains(&kind.as_str()) {
            return Err(Error::invalid(format!(
                "the field `{name}` is a `{kind}`, which is not a kind of field; there are: {}",
                KINDS.join(", ")
            )));
        }
        let of: Vec<String> = match about.get("of") {
            Some(Value::Array(options)) => options.iter().map(word).collect(),
            // One collection is written as its name, with no list about it.
            Some(one @ Value::String(_)) if kind == "record" || kind == "records" => vec![word(one)],
            _ => Vec::new(),
        };
        let of: Vec<String> = of.into_iter().filter(|option| !option.is_empty()).collect();
        if kind == "choice" && of.is_empty() {
            return Err(Error::invalid(format!(
                "the field `{name}` is a choice and needs `of`: what can be chosen"
            )));
        }
        if (kind == "record" || kind == "records") && of.len() != 1 {
            return Err(Error::invalid(format!(
                "the field `{name}` names records and needs `of`: the one collection whose records it can name, as in `of: sprints`"
            )));
        }
        let flag = |name: &str| about.get(name).and_then(Value::as_bool).unwrap_or(false);
        // Said as a scope, any value takes it; said as a mapping, each
        // value written there takes what is written beside it; said as
        // `each`, of a list of scopes, each scope written takes itself.
        let takes_each = about.get("takes").and_then(Value::as_str).map(str::trim) == Some(EACH);
        if takes_each && kind != "list" {
            return Err(Error::invalid(format!(
                "the field `{name}` says `takes: {EACH}`, which is for a list of scopes (each one written takes \
                 holding it), and it is a `{kind}`: say the scope giving it a value takes, as in `takes: shell`"
            )));
        }
        let unsaid = match about.get("unsaid") {
            None | Some(Value::Null) => Vec::new(),
            Some(said @ Value::Array(_)) if takes_each => scopes(said),
            // Of a choice, the one word nothing chosen stands for: where its
            // words are ranked, and what a number from before is counted
            // from (`kernel::ranked`).
            Some(said @ Value::String(_)) if kind == "choice" && of.contains(&word(said)) => vec![word(said)],
            Some(_) if kind == "choice" => {
                return Err(Error::invalid(format!(
                    "the field `{name}` says `unsaid`, which for a choice is the one word of `of` that nothing \
                     chosen stands for, as in `unsaid: {}`",
                    of.first().map_or("normal", String::as_str)
                )));
            }
            Some(_) => {
                return Err(Error::invalid(format!(
                    "the field `{name}` says `unsaid`, which is the scopes a list that says `takes: {EACH}` \
                     stands for where a record leaves it empty, as in `unsaid: [board, records]`"
                )));
            }
        };
        let (takes, takes_of) = match about.get("takes") {
            Some(Value::Object(each)) => (Takes::default(), by_value(&name, &kind, &of, each)?),
            _ if takes_each => (Takes::default(), std::collections::BTreeMap::new()),
            said => (Takes::read(&name, None, said)?, std::collections::BTreeMap::new()),
        };
        let stands = crate::stood::read(&name, flag("stands"), takes_each, &takes, &takes_of)?;
        let suggests = match about.get("suggests") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(words)) if kind == "text" => {
                // Each as it is written, once: a word may name a blueprint, and is not made a slug of.
                let mut offered: Vec<String> = Vec::new();
                for word in words.iter().map(blueprint::expr::display) {
                    let word = word.trim();
                    if !word.is_empty() && !offered.iter().any(|kept| kept == word) {
                        offered.push(word.to_string());
                    }
                }
                offered
            }
            Some(Value::Array(_)) => {
                return Err(Error::invalid(format!(
                    "the field `{name}` is a `{kind}` and says `suggests`, which is for a field that takes any word, a `text`; \
                     what a choice can be is said with `of`"
                )));
            }
            Some(_) => {
                return Err(Error::invalid(format!(
                    "the field `{name}` says `suggests`, which is a list of the words it offers, as in \
                     `suggests: [plan, work, review]`"
                )));
            }
        };
        let writer_of = about.get("writer_of").map(blueprint::expr::display).map(|of| of.trim().to_string());
        if writer_of.is_some() && kind != "text" {
            return Err(Error::invalid(format!(
                "the field `{name}` is a `{kind}` and says `writer_of`, which is for words that say who wrote a \
                 cell, a `text`"
            )));
        }
        let calls = match about.get("calls") {
            None | Some(Value::Null) => None,
            Some(said) => match said.as_u64().filter(|place| (1..=9).contains(place)) {
                Some(place) => Some(place as u32),
                None => {
                    return Err(Error::invalid(format!(
                        "the field `{name}` says `calls`, which is where it is among the fields a record is called                          by, a number from 1 to 9, as in `calls: 1`"
                    )));
                }
            },
        };
        if flag("indexed") && (kind == "list" || kind == "records") {
            return Err(Error::invalid(format!(
                "the field `{name}` is a `{kind}` and says `indexed`, and what holds several things is kept in \
                 order by no index: say it of a field that holds one"
            )));
        }
        if flag("said") && (flag("quiet") || about.get("takes").is_some_and(|takes| !takes.is_null())) {
            return Err(Error::invalid(format!(
                "the field `{name}` says `said`, and it is `quiet` or says what giving it a value takes: what \
                 an event about a record carries is heard by whoever may read the table, so a field kept for \
                 itself or guarded is never carried. Leave `said` out, or say it of another field"
            )));
        }
        if flag("zoned") && kind != "text" {
            return Err(Error::invalid(format!(
                "the field `{name}` is a `{kind}` and says `zoned`, which is for words that are a time of day, a `text`"
            )));
        }
        read.push(Field {
            required: flag("required"),
            quiet: flag("quiet"),
            hidden: flag("hidden"),
            zoned: flag("zoned"),
            writer_of,
            suggests,
            icon: crate::icons::declared(&name, about.get("icon"))?,
            icon_chosen: None,
            added: false,
            label_chosen: None,
            of_chosen: Vec::new(),
            of_added: None,
            rewording: false,
            calls,
            indexed: flag("indexed"),
            said: flag("said"),
            name,
            kind,
            of,
            takes,
            takes_of,
            takes_each,
            stands,
            unsaid,
        });
    }
    if read.is_empty() {
        return Err(Error::invalid("a collection needs at least one field"));
    }
    for field in &read {
        let Some(of) = &field.writer_of else { continue };
        if *of == field.name || !read.iter().any(|other| other.name == *of) {
            return Err(Error::invalid(format!(
                "the field `{}` says `writer_of: {of}`, which names another field of the table whose writer it \
                 keeps, as in `writer_of: environment`",
                field.name
            )));
        }
    }
    let indexed = read.iter().filter(|field| field.indexed).count();
    if indexed > kernel::window::MOST_INDEXED {
        return Err(Error::invalid(format!(
            "{indexed} fields say `indexed`, and a table has {} kept in order at most: say it of those it is \
             sorted or held by most",
            kernel::window::MOST_INDEXED
        )));
    }
    Ok(read)
}

/// Writes who wrote each cell a field keeps the writer of (`writer_of`)
/// into `data`, which `by` is writing: themselves, where that cell is
/// among those given (`given`, the names of the cells written), and, of a
/// record being added (`adding`), nothing where it is not. Nobody writes
/// such a field themselves: of a record being added what is given there
/// is written over, and a change of it alone is refused.
pub(crate) fn stamp(
    collection: &Collection,
    data: &mut Map<String, Value>,
    given: &[String],
    by: &kernel::Principal,
    adding: bool,
) -> Result<()> {
    let who = by.key.as_ref().map_or_else(|| by.actor(), |key| format!("key:{key}"));
    for field in &collection.fields {
        let Some(of) = &field.writer_of else { continue };
        if given.contains(of) {
            data.insert(field.name.clone(), Value::String(who.clone()));
        } else if adding {
            data.insert(field.name.clone(), Value::Null);
        } else if given.contains(&field.name) {
            return Err(Error::denied(format!(
                "`{}` of `{}` says who wrote `{of}`, which the factory writes as `{of}` is written: write `{of}` \
                 and it says you",
                field.name, collection.name
            )));
        }
    }
    Ok(())
}

/// Several ids as a field keeps them: each once, in the order given, and
/// nothing for none.
fn several(ids: impl Iterator<Item = String>) -> Value {
    let mut kept: Vec<String> = Vec::new();
    for id in ids.map(|id| id.trim().to_string()).filter(|id| !id.is_empty()) {
        if !kept.contains(&id) {
            kept.push(id);
        }
    }
    if kept.is_empty() { Value::Null } else { serde_json::json!(kept) }
}

/// Whether text is a day as a date field keeps one: `2026-11-30`.
fn is_day(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let number = |part: &str, digits: usize| {
        (part.len() == digits && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<u32>().ok())
            .flatten()
    };
    match parts.as_slice() {
        [year, month, day] => {
            number(year, 4).is_some()
                && number(month, 2).is_some_and(|month| (1..=12).contains(&month))
                && number(day, 2).is_some_and(|day| (1..=31).contains(&day))
        }
        _ => false,
    }
}

/// Makes `data` fit a collection's fields, or says why it does not. What
/// a `record` field names is looked for afterwards, by `Collections::pointed`.
pub(crate) fn check(collection: &Collection, mut data: Map<String, Value>) -> Result<Map<String, Value>> {
    for name in data.keys() {
        known(collection, name)?;
    }
    let mut checked = Map::new();
    for field in &collection.fields {
        let value = data.remove(&field.name).unwrap_or(Value::Null);
        if nothing(&value) && field.required {
            return Err(Error::invalid(format!("`{}` is needed", field.name)));
        }
        checked.insert(field.name.clone(), fit(&collection.project, &collection.name, field, value)?);
    }
    Ok(checked)
}

/// What giving a field one of its values takes, as a declaration writes
/// it (`takes: { on: shell }`, or `{ on: [shell, secrets:use] }`): each
/// value with its scopes.
///
/// Only of a field whose values are a few words that can be written one
/// way, a choice or a flag. Of anything freer the same thing written
/// another way would take nothing, and the guard would be no guard.
fn by_value(
    name: &str,
    kind: &str,
    of: &[String],
    each: &Map<String, Value>,
) -> Result<std::collections::BTreeMap<String, Takes>> {
    let values: Vec<String> = match kind {
        "choice" => of.to_vec(),
        "flag" => vec!["true".into(), "false".into()],
        _ => {
            return Err(Error::invalid(format!(
                "the field `{name}` says what giving it one value takes, which a choice or a flag can say and a \
                 `{kind}` cannot: say what giving it any value takes, as in `takes: shell`"
            )));
        }
    };
    let mut takes = std::collections::BTreeMap::new();
    for (value, scope) in each {
        if !values.contains(value) {
            return Err(Error::invalid(format!(
                "the field `{name}` says what giving it `{value}` takes, and `{value}` is not one of: {}",
                values.join(", ")
            )));
        }
        let said = Takes::read(name, Some(value), Some(scope))?;
        if said.is_empty() {
            return Err(Error::invalid(format!(
                "the field `{name}` says what giving it `{value}` takes, and that is a scope or a list of them, \
                 as in `takes: {{ {value}: shell }}`"
            )));
        }
        takes.insert(value.clone(), said);
    }
    Ok(takes)
}

/// Refuses a collection's fields being declared again as `now` by one who
/// does not hold what a cell of `was` took and would take no longer.
///
/// A cell says `takes` because something with more authority acts on it.
/// Declared again without the word, by whoever may declare the table, it
/// would be theirs to change, and what acts on it would act for them: so
/// taking a guard off a cell takes holding what it guarded.
pub(crate) fn still_guarded(name: &str, was: &[Field], now: &[Field], by: &kernel::Principal) -> Result<()> {
    for field in was {
        let now = now.iter().find(|now| now.name == field.name);
        crate::stood::still_standing(name, field, now, by)?;
        // What every value took is still held to where every value takes
        // it, and what one value took, where that value or every value
        // does; each scope of several by itself, so that one taken off a
        // list takes holding that one.
        let any = |scope: &str| now.is_some_and(|now| now.takes.has(scope));
        let all =
            Some((None, &field.takes, field.takes.scopes().iter().filter(|scope| !any(scope)).collect()));
        let one = field.takes_of.iter().map(|(value, takes)| {
            let kept = |scope: &str| {
                any(scope) || now.is_some_and(|now| now.takes_of.get(value).is_some_and(|now| now.has(scope)))
            };
            (Some(value), takes, takes.scopes().iter().filter(|scope| !kept(scope)).collect::<Vec<_>>())
        });
        // What every scope written took is still held to where it does.
        // Otherwise any scope could then be written there by whoever may
        // change the record, and be given: that is whoever manages
        // credentials' to allow, and nobody else's.
        if field.takes_each && !now.is_some_and(|now| now.takes_each) && !by.can("admin") {
            return Err(Error::denied(format!(
                "writing a scope into `{}` of `{name}` takes holding it, and declaring `{name}` again so that it \
                 does not takes `admin`, which {} does not hold",
                field.name, by.name
            )));
        }
        // Nor is what an unsaid one stands for made less, for the same
        // reason: a record that leaves it unsaid would be held to less
        // than it gives.
        let dropped = field
            .unsaid
            .iter()
            .find(|scope| now.is_some_and(|now| now.takes_each && !now.unsaid.contains(scope)));
        if let Some(scope) = dropped.filter(|_| !by.can("admin")) {
            return Err(Error::denied(format!(
                "`{}` of `{name}` stands for `{scope}` where a record leaves it unsaid, and declaring `{name}` \
                 again so that it does not takes `admin`, which {} does not hold",
                field.name, by.name
            )));
        }
        for (value, takes, dropped) in all.into_iter().chain(one) {
            let lacking: Vec<&str> =
                dropped.into_iter().map(String::as_str).filter(|scope| !by.can(scope)).collect();
            if lacking.is_empty() {
                continue;
            }
            let what = value.map_or_else(|| "a value".to_string(), |value| format!("the value `{value}`"));
            let (took, lacking) = (listed(takes.scopes().iter().map(String::as_str)), listed(lacking));
            return Err(Error::denied(format!(
                "giving `{}` of `{name}` {what} takes {took}, and declaring `{name}` again so that it does not \
                 takes holding {lacking}, which {} does not hold",
                field.name, by.name
            )));
        }
    }
    Ok(())
}

/// Says that `name` is a field of a collection, or which ones are.
pub(crate) fn known(collection: &Collection, name: &str) -> Result<()> {
    if collection.field(name).is_some() {
        return Ok(());
    }
    let known: Vec<&str> = collection.fields.iter().map(|field| field.name.as_str()).collect();
    Err(Error::invalid(format!(
        "`{name}` is not a field of `{}`; its fields are: {}",
        collection.name,
        known.join(", ")
    )))
}

/// Refuses unless `by` holds what giving each of these fields its value
/// takes, and what the record then gives. Asked wherever a record is
/// added or changed, of the fields that are being given something, each
/// with what it is being given and what it held before (nothing, for a
/// record that is being added), and of the record as it is to be
/// written.
pub(crate) fn may_set<'a>(
    collection: &Collection,
    record: &Map<String, Value>,
    fields: impl IntoIterator<Item = (&'a str, &'a Value, Option<&'a Value>)>,
    by: &kernel::Principal,
) -> Result<()> {
    // What each list of scopes that is being written held before.
    let mut held = std::collections::BTreeMap::new();
    for (name, value, kept) in fields {
        let Some(field) = collection.field(name) else { continue };
        if let Some(why) = field.takes.refused(by) {
            return Err(Error::denied(format!("changing `{name}` of `{}` {why}", collection.name)));
        }
        // What only one of its values takes is asked of that value, as it
        // is written: any other, and nothing at all, takes no more than
        // changing the record does.
        let written = blueprint::expr::display(value);
        if let Some(why) = field.takes_of.get(&written).and_then(|takes| takes.refused(by)) {
            return Err(Error::denied(format!(
                "giving `{name}` of `{}` the value `{written}` {why}",
                collection.name
            )));
        }
        if field.takes_each {
            held.insert(name, kept.map(scopes).unwrap_or_default());
        }
    }
    // A list of scopes that says each takes holding it is what the record
    // gives somebody, through something with more authority than whoever
    // may write the record. So whoever writes the record, whichever of
    // its cells, holds each scope it then says: what the one it is given
    // to is told, and how many of them there are, are cells beside it,
    // and writing those is acting with what it holds. A record made to
    // say no more than its writer holds, in the same write, is theirs:
    // what it gives from there is theirs to give. Where it says nothing,
    // it gives what the list stands for unsaid.
    for field in collection.fields.iter().filter(|field| field.takes_each) {
        let (name, table, who) = (&field.name, &collection.name, &by.name);
        let said = record.get(name).map(scopes).unwrap_or_default();
        let gives = if said.is_empty() { &field.unsaid } else { &said };
        let Some(scope) = gives.iter().find(|scope| !by.can(scope)) else { continue };
        let written = held.get(name.as_str()).is_some_and(|was| !kernel::principal::covers(was, scope));
        let why = if said.is_empty() {
            format!(
                "a record of `{table}` that says nothing of `{name}` gives `{scope}`, and is added or changed only \
                 by whoever holds `{scope}`, which {who} does not"
            )
        } else if written {
            format!(
                "writing `{scope}` into `{name}` of `{table}` takes holding `{scope}`, which {who} does not hold"
            )
        } else {
            format!(
                "a record of `{table}` whose `{name}` says `{scope}` is changed only by whoever holds `{scope}`, \
                 which {who} does not"
            )
        };
        return Err(Error::denied(format!("{why}: nobody gives what they do not hold")));
    }
    // And what a cell took, while it says so, as `takes: each` is of a list.
    crate::stood::held(collection, record, by)
}

/// The scopes a value of a list names, each as it is written.
fn scopes(value: &Value) -> Vec<String> {
    let words = |word: &Value| blueprint::expr::display(word).trim().to_string();
    let all = match value {
        Value::Null => Vec::new(),
        Value::Array(each) => each.iter().map(words).collect(),
        one => vec![words(one)],
    };
    all.into_iter().filter(|scope| !scope.is_empty()).collect()
}

/// Whether a value is no value: nothing at all, or words with nothing in them.
pub(crate) fn nothing(value: &Value) -> bool {
    value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty())
}

/// Makes one value what a field of the collection `name` keeps, or says
/// why it cannot be that.
fn fit(project: &str, name: &str, field: &Field, value: Value) -> Result<Value> {
    let wrong = |what: &str| Error::invalid(format!("`{}` must be {what}", field.name));
    Ok(match (field.kind.as_str(), value) {
        (_, Value::Null) => Value::Null,
        ("text" | "actor", Value::String(text)) => Value::String(text),
        ("document", Value::String(text)) if text.len() > kernel::topics::MOST_DOCUMENT => {
            return Err(wrong(&format!("a document of {} KB at most", kernel::topics::MOST_DOCUMENT / 1024)));
        }
        ("document", Value::String(text)) => Value::String(text),
        // A cell that was emptied is kept as empty text, and says nothing
        // whatever its field then holds: a column that was words and is
        // now a number has rows that were emptied while it was words.
        (_, Value::String(text)) if text.trim().is_empty() => Value::Null,
        ("text" | "actor", _) => return Err(wrong("text")),
        ("document", _) => return Err(wrong("a document, its Markdown as text")),
        ("choice", Value::String(chosen)) if field.of.contains(&chosen) => Value::String(chosen),
        // A number to a choice that says what nothing stands for is the
        // word it is counted to from there, as one written before the field
        // was a choice is (`crate::reworded`).
        ("choice", Value::Number(number)) if !field.unsaid.is_empty() => {
            let unsaid = field.unsaid.first().map(String::as_str);
            match number.as_f64().and_then(|number| kernel::ranked::word_of(number, &field.of, unsaid)) {
                Some(word) => Value::String(word),
                None => return Err(wrong(&format!("one of: {}", field.of.join(", ")))),
            }
        }
        ("choice", other) => {
            return Err(Error::invalid(format!(
                "`{}` is not a `{}` of `{name}`; there are: {}",
                blueprint::expr::display(&other),
                field.name,
                field.of.join(", ")
            )));
        }
        ("list", Value::Array(items)) => {
            Value::Array(items.iter().map(|item| Value::String(blueprint::expr::display(item))).collect())
        }
        ("list", _) => return Err(wrong("a list")),
        ("number", Value::Number(number)) => Value::Number(number),
        ("number", _) => return Err(wrong("a number")),
        ("date", Value::String(day)) if is_day(&day) => Value::String(day),
        ("date", _) => return Err(wrong("a day written year-month-day, like 2026-11-30")),
        ("record", Value::String(id)) => Value::String(id.trim().to_string()),
        // Several: a list of ids, or one line of them with commas between,
        // as they are written on a command line. Each is named once.
        ("records", Value::String(ids)) => several(ids.split(',').map(str::to_string)),
        ("records", Value::Array(ids)) if ids.iter().all(Value::is_string) => {
            several(ids.iter().filter_map(Value::as_str).map(str::to_string))
        }
        ("records", _) => {
            let of = field.points_at().unwrap_or_default();
            return Err(wrong(&format!(
                "a list of the ids of records of `{of}`, {}",
                written(project, of, None)
            )));
        }
        ("record", _) => {
            let of = field.points_at().unwrap_or_default();
            return Err(wrong(&format!("the id of a record of `{of}`, {}", written(project, of, None))));
        }
        ("flag", Value::Bool(flag)) => Value::Bool(flag),
        ("flag", _) => return Err(wrong("true or false")),
        (_, other) => other,
    })
}

/// How many of these values a table's rows hold of `field` are what it,
/// as it is now said to be, does not take. Nothing, and a cell that was
/// emptied, is taken by any field.
pub(crate) fn misfits(
    project: &str,
    name: &str,
    field: &Field,
    held: impl IntoIterator<Item = Value>,
) -> usize {
    held.into_iter()
        .filter(|value| !nothing(value) && fit(project, name, field, value.clone()).is_err())
        .count()
}

/// How the ids of a collection's records are written, for whoever gave
/// something else.
fn written(project: &str, collection: &str, store: Option<Store>) -> String {
    match store {
        // A document is known by where its file is, and a mockup by its folder.
        Some(Store::Documents) => format!(
            "which are known by where their files are: `{project}-{collection}:guides:install` is `guides/install.md` in its folder"
        ),
        Some(Store::Mockups) => format!(
            "which are known by their folders: `{project}-{collection}:pricing` is the folder `pricing`"
        ),
        Some(_) => format!(
            "which are written `{}`, `{}` and so on",
            record_id(project, collection, 1),
            record_id(project, collection, 2)
        ),
        // Where it is kept is not known here: its own list says how they are known.
        None => format!("as `{} record list {project} {collection}` shows them", brand::CLI),
    }
}

impl Collections {
    /// Says that each collection a `record` field of `name` is to point at
    /// is there: one the project already has, or `name` itself. This is
    /// asked when the collection is declared, so that a mistake is found
    /// when its blueprint is applied and not at the first record.
    pub(crate) async fn pointing(&self, project: &str, name: &str, fields: &[Field]) -> Result<()> {
        for field in fields {
            if let Some(of) = field.points_at()
                && of != name
                && self.find(project, of).await?.is_none()
            {
                return Err(Error::invalid(format!(
                    "the field `{}` of `{name}` names records of `{of}`, and the project `{project}` has no collection called `{of}`; declare `{of}` first, in a step before this one",
                    field.name
                )));
            }
        }
        Ok(())
    }

    /// Refuses unless `by` may read each collection a record is newly
    /// named of: naming a record takes being able to read it. What a
    /// record names is shown with it and followed by whatever reads it,
    /// with that reader's authority, so somebody who may not read a table
    /// could otherwise have its records read for them by naming them; and
    /// being told whether an id is one of its records is reading it. So
    /// it is asked before [`Collections::pointed`], and answers the same
    /// whether or not there is such a record. `was` is what each changed
    /// cell held, for a record that is there: a cell that was not changed,
    /// or names only what it already named, asks nothing, so a row that
    /// names something its writer cannot read can still be changed by
    /// them in its other cells.
    pub(crate) async fn may_name(
        &self,
        collection: &Collection,
        data: &Map<String, Value>,
        was: Option<&Map<String, Value>>,
        by: &kernel::Principal,
    ) -> Result<()> {
        for field in &collection.fields {
            let Some(of) = field.points_at() else { continue };
            let before = match was {
                None => Vec::new(),
                Some(was) => match was.get(&field.name) {
                    Some(held) => field.named(held),
                    None => continue,
                },
            };
            let now = data.get(&field.name).map(|value| field.named(value)).unwrap_or_default();
            if now.iter().all(|id| before.contains(id)) {
                continue;
            }
            let Some(named) = self.find(&collection.project, of).await? else { continue };
            named.may_read(by).map_err(|refused| {
                Error::denied(format!(
                    "the `{}` of `{}` names a record of `{of}`, and naming one takes being able to read it: {}",
                    field.name, collection.name, refused.message
                ))
            })?;
        }
        Ok(())
    }

    /// Says that every record `data` newly points at is there. `kept` is
    /// the id of the record being written back, for one that exists: a
    /// field that names what it already named is left as it is, because
    /// the record it named may have been taken away since, and that must
    /// not stop the one that named it from being moved or changed.
    pub(crate) async fn pointed(
        &self,
        collection: &Collection,
        data: &Map<String, Value>,
        kept: Option<&str>,
    ) -> Result<()> {
        let mut before: Option<Map<String, Value>> = None;
        for field in &collection.fields {
            let Some(of) = field.points_at() else { continue };
            for id in data.get(&field.name).map(|value| field.named(value)).unwrap_or_default() {
                if self.has(&collection.project, of, id).await? {
                    continue;
                }
                if let Some(kept) = kept {
                    if before.is_none() {
                        let was = self.record_of(collection, kept).await?;
                        before = Some(was.map(|one| one.data).unwrap_or_default());
                    }
                    let named = before.as_ref().and_then(|was| was.get(&field.name));
                    if named.is_some_and(|was| field.named(was).contains(&id)) {
                        continue;
                    }
                }
                let store = self.find(&collection.project, of).await?.map(|named| named.store);
                return Err(Error::invalid(format!(
                    "`{id}` is not a record of `{of}`, which is what the `{}` of `{}` names. Give the id of one that is there, {}, or leave it empty",
                    field.name,
                    collection.name,
                    written(&collection.project, of, store)
                )));
            }
        }
        Ok(())
    }

    /// Whether a project's collection has a record with this id.
    async fn has(&self, project: &str, name: &str, id: &str) -> Result<bool> {
        let Some(collection) = self.find(project, name).await? else { return Ok(false) };
        if collection.store != Store::Local {
            return Ok(self.records(&collection).await?.iter().any(|record| record.id == id));
        }
        let (project, name, id) = (project.to_string(), name.to_string(), id.to_string());
        self.rows(&project)
            .db()
            .call(move |connection| {
                connection
                    .prepare("SELECT 1 FROM records WHERE id = ?1 AND project = ?2 AND collection = ?3")?
                    .exists(params![id, project, name])
            })
            .await
    }
}
