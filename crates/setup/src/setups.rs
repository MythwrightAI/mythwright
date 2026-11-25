//! The setups a factory keeps: rows a new project is set up with, saved
//! from the dialog that adds one, each under a name ("a research
//! project"), and one under [`DEFAULT`], which is what a project is set
//! up with where whoever adds it says nothing else of it.
//!
//! Each is kept in the factory's small durable state ([`NAMESPACE`],
//! which a copy takes: `studio::copy::KEPT`), as its rows as they were
//! written, who saved it and when, and is saved or taken away in one
//! write with what is said of it (`project.setup.saved`,
//! `project.setup.removed`, the factory's own for good). With none saved
//! as the default, the default is what ships: `std/project`'s own rows
//! ([`crate::usual::shipped`]).
//!
//! The factory's default is for whoever manages it (`admin`), and a
//! setup with a name for whoever may add projects and apply a blueprint
//! (`projects:write` and `blueprints:run`): it is a list of blueprints
//! that are applied as whoever adds a project with it, which saving one
//! gives nobody any more than they hold.

use deciding::setups::{Row, rows};
use journal::Journal;
use kernel::{Error, Principal, Result, Source};
use serde_json::{Value, json};

/// Where the setups are kept, in the factory's small durable state.
pub const NAMESPACE: &str = "project.setups";

/// What the factory's own default for new projects is called.
pub const DEFAULT: &str = "default";

/// One setup as it is kept.
#[derive(Clone, Debug)]
pub struct Setup {
    pub name: String,
    pub rows: Vec<Row>,
    /// Who saved it, as an actor, and when, in milliseconds.
    pub by: String,
    pub at: i64,
}

impl Setup {
    /// As it is handed out.
    pub fn value(&self) -> Value {
        let rows: Vec<Value> = self.rows.iter().map(Row::value).collect();
        json!({ "name": self.name, "rows": rows, "by": self.by, "at": self.at, "default": self.name == DEFAULT })
    }

    fn read(name: &str, kept: &Value) -> Option<Setup> {
        Some(Setup {
            name: name.to_string(),
            rows: rows(kept.get("rows")?).ok()?,
            by: kept.get("by").and_then(Value::as_str).unwrap_or_default().to_string(),
            at: kept.get("at").and_then(Value::as_i64).unwrap_or_default(),
        })
    }
}

/// Every setup the factory keeps, by name; one that can no longer be
/// read is passed over.
pub async fn saved(journal: &Journal) -> Result<Vec<Setup>> {
    let kept = journal.kv_list(NAMESPACE).await?;
    Ok(kept.iter().filter_map(|(name, kept)| Setup::read(name, kept)).collect())
}

/// The setup called `name`, where the factory keeps one.
pub async fn one(journal: &Journal, name: &str) -> Result<Option<Setup>> {
    Ok(journal.kv_get(NAMESPACE, name).await?.and_then(|kept| Setup::read(name, &kept)))
}

/// What a name may be: small letters, digits and dashes, as a row's.
fn named(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 60
        || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(Error::invalid(format!(
            "`{name}` is no name for a setup: a name is small letters, digits and dashes, such as `research`"
        )));
    }
    Ok(())
}

/// What saving or taking away the setup called `name` takes.
fn may(who: &Principal, name: &str) -> Result<()> {
    if name == DEFAULT {
        // What every new project is set up with: the factory's to say.
        return who.require("admin");
    }
    who.require("projects:write")?;
    who.require("blueprints:run")
}

/// Keeps `rows` as the setup called `name`, as `who`, in one write with
/// what is said of it; rows that cannot be read are refused in the
/// sentence that says why, and nothing is kept.
pub async fn save(journal: &Journal, name: &str, value: &Value, who: &Principal) -> Result<Setup> {
    named(name)?;
    may(who, name)?;
    let read = rows(value).map_err(Error::invalid)?;
    if read.is_empty() {
        return Err(Error::invalid(
            "a setup has at least one row: a project set up with nothing is added alone",
        ));
    }
    let by = who.actor();
    let written: Vec<Value> = read.iter().map(Row::value).collect();
    let (key, said_by) = (name.to_string(), by.clone());
    let (at, _) = journal
        .write(move |writing| {
            let at = writing.ts();
            writing.keep(NAMESPACE, &key, &json!({ "rows": written, "by": said_by, "at": at }))?;
            writing.say_here(
                "project.setup.saved",
                Source::actor(said_by.clone()),
                json!({ "name": key, "rows": written.len(), "by": said_by }),
            )?;
            Ok(at)
        })
        .await?;
    Ok(Setup { name: name.to_string(), rows: read, by, at })
}

/// Takes the setup called `name` away, as `who`: the default goes back
/// to what ships. One that is not kept is refused in a sentence.
pub async fn remove(journal: &Journal, name: &str, who: &Principal) -> Result<()> {
    may(who, name)?;
    if one(journal, name).await?.is_none() {
        return Err(Error::not_found(format!("there is no setup called `{name}`")));
    }
    let (key, by) = (name.to_string(), who.actor());
    journal
        .write(move |writing| {
            writing.forget(NAMESPACE, &key)?;
            writing.say_here(
                "project.setup.removed",
                Source::actor(by.clone()),
                json!({ "name": key, "by": by }),
            )?;
            Ok(())
        })
        .await?;
    Ok(())
}
