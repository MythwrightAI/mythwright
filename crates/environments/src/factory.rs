//! The variables of the profile, the copy of the factory a blueprint runs
//! in: what its owner has said is true of where it runs, each a name and a
//! plain value (text, a number, true or false), which every expression
//! reads as `profile.vars.<name>`. (They were `environment.vars` before
//! 0.1.0, and are kept under the name they always were.)
//!
//! They are set whole, and kept in the factory's journal's small durable
//! state before they are held, so a factory never reads what it would
//! not read again after a restart. They are held in memory from the
//! moment the first conductor is made, which is before anything is
//! resumed, and that one [`Held`] is handed to every conductor of the
//! factory. Setting them runs nothing again: an instance reads the new
//! value the next time it works an expression out.
//!
//! Changes take turns ([`Held::changing`]), and whoever changes them says
//! what changed while it holds the turn ([`Changing::set`] gives what
//! changed and nothing is said here), so what is kept, what is held and
//! the order of what is said of them agree.
//!
//! They are not for secrets. They are shown to whoever may read
//! blueprints and instances, kept in the database as they are, and may
//! appear in events and in the history of what was applied.

use std::sync::{Mutex, OnceLock};

use journal::Journal;
use kernel::{Error, Fields, Result};
use serde::Serialize;
use serde_json::{Value, json};

/// The namespace of the journal's small durable state the variables are
/// kept under. It does not go into a copy of a factory: the variables are
/// said of the factory they were set in.
pub const NAMESPACE: &str = runs::beside::ENVIRONMENT;
/// The one key there: the whole set, so that it is replaced in one write.
const VARS: &str = "vars";

/// What a factory holds of its profile: what it is called, once said,
/// and its variables as they are now.
pub struct Held {
    name: OnceLock<String>,
    vars: Mutex<Fields>,
    /// One change at a time, so that what is kept and what is held agree.
    changing: tokio::sync::Mutex<()>,
}

impl Held {
    /// What was kept of the profile the last time this factory ran.
    /// Anything kept that is not a variable (a database edited by hand) is
    /// left out, not handed to every expression.
    pub async fn load(journal: &Journal) -> Result<Held> {
        let vars = match journal.kv_get(NAMESPACE, VARS).await? {
            Some(Value::Object(kept)) => {
                kept.into_iter().filter(|(name, value)| is_name(name) && kind(value).is_none()).collect()
            }
            _ => Fields::new(),
        };
        Ok(Held { name: OnceLock::new(), vars: Mutex::new(vars), changing: tokio::sync::Mutex::new(()) })
    }

    /// Says what the factory's profile is called, once: whatever is
    /// said after the first is not taken.
    pub fn called(&self, name: String) {
        let _ = self.name.set(name);
    }

    /// What the factory's profile is called: empty for one with none.
    pub fn name(&self) -> &str {
        self.name.get().map_or("", String::as_str)
    }

    /// The variables, as they are now.
    pub fn vars(&self) -> Fields {
        self.vars.lock().expect("profile lock").clone()
    }

    /// `profile`, as every expression sees it: what it is called and its
    /// variables, which is where whatever else a profile comes to hold is
    /// added.
    pub fn view(&self) -> Value {
        json!({ "name": self.name(), "vars": self.vars() })
    }

    /// The turn at changing the variables, held until what is handed back
    /// is let go of: whatever says what changed says it before that.
    pub async fn changing(&self) -> Changing<'_> {
        Changing { held: self, _turn: self.changing.lock().await }
    }
}

/// The turn at changing the factory's variables.
pub struct Changing<'a> {
    held: &'a Held,
    _turn: tokio::sync::MutexGuard<'a, ()>,
}

impl Changing<'_> {
    /// Makes `vars` the variables, in place of whatever was set, and gives
    /// what that changed: they are kept in `journal` first and held after.
    /// Where nothing changed nothing is written.
    pub async fn set(&mut self, journal: &Journal, vars: Fields) -> Result<Changes> {
        check(&vars)?;
        let before = self.held.vars();
        let mut changes = Changes::default();
        for (name, value) in &vars {
            match before.get(name) {
                None => changes.set.push(name.clone()),
                Some(was) if was != value => changes.changed.push(name.clone()),
                Some(_) => {}
            }
        }
        changes.removed = before.keys().filter(|name| !vars.contains_key(*name)).cloned().collect();
        if changes.is_empty() {
            return Ok(changes);
        }
        // Nothing kept is how none is kept: a factory that has had its
        // variables cleared holds what one that never had any does.
        let kept = if vars.is_empty() { Value::Null } else { Value::Object(vars.clone()) };
        journal.kv_set(NAMESPACE, VARS, &kept).await?;
        *self.held.vars.lock().expect("profile lock") = vars;
        Ok(changes)
    }
}

/// What setting the variables changed, by name. A variable that is as it
/// was is in none of them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Changes {
    /// Variables that were not set before.
    pub set: Vec<String>,
    /// Variables that have another value than they had.
    pub changed: Vec<String>,
    /// Variables that are no longer set.
    pub removed: Vec<String>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.set.is_empty() && self.changed.is_empty() && self.removed.is_empty()
    }
}

/// Whether `name` can be written after `profile.vars.` in an expression.
fn is_name(name: &str) -> bool {
    let mut letters = name.bytes();
    letters.next().is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && letters.all(|letter| letter.is_ascii_alphanumeric() || letter == b'_')
}

/// What `value` is, in words, when it is not plain: `None` for text, a
/// number, true and false.
fn kind(value: &Value) -> Option<&'static str> {
    match value {
        Value::String(_) | Value::Number(_) | Value::Bool(_) => None,
        Value::Array(_) => Some("a list"),
        Value::Object(_) => Some("a map"),
        Value::Null => Some("nothing (null)"),
    }
}

/// Refuses a set of variables that could not be read back as it was meant.
fn check(vars: &Fields) -> Result<()> {
    for (name, value) in vars {
        if !is_name(name) {
            return Err(Error::invalid(format!(
                "`{name}` cannot be the name of a variable. A name is letters, digits and `_`, and does not start \
                 with a digit, so that a blueprint can read it as `profile.vars.<name>`"
            )));
        }
        if let Some(kind) = kind(value) {
            return Err(Error::invalid(format!(
                "the variable `{name}` is {kind}, and a variable is plain: text, a number, or true or false. \
                 Leave a variable out to have it not set; write anything larger as text, for the blueprint \
                 that reads it to take apart"
            )));
        }
    }
    Ok(())
}
