//! What a program was handed, taken out of what it said.
//!
//! A secret goes from its file into the environment of one program. What
//! that program does with it is the program's: one that prints its
//! environment, or fails with the key it was refused with in its last
//! line, has said its secret. What it said is written down (a step's log,
//! what the step gives, a service's log, an error's sentence), and from
//! there it is in the journal, in whatever is said in a channel of it, and
//! in a copy of the factory. So wherever a program that was handed
//! something says it again, it is written as the secret's name in a mark
//! that no program prints by itself, and never as the value.
//!
//! Two things do that, and both are this:
//!
//! * A program whose output goes to files (a step's command, a service,
//!   an agent's program that goes on without the daemon) has it taken
//!   out by its wrapper ([`crate::wrapped`], which stands between every
//!   such program and its files, and is told what the program was
//!   handed): what reaches the disk is already without it, whether or
//!   not a daemon is there to read it.
//! * A program whose output the daemon reads itself, over pipes, has each
//!   line taken through [`Unsaid::from`] before anything is done with it.
//!
//! A value too short to be told from ordinary words is too short to be a
//! key, and is left ([`SHORTEST`]): taking `on` or `1234` out of every line
//! a program prints would say more about the secret than leaving it does.
//! A value of several lines (a key in its armour) is looked for whole and
//! line by line, since that is how a program prints it.

use std::collections::BTreeMap;

use serde_json::Value;

/// The fewest letters a value has to have to be looked for in what a
/// program said.
pub const SHORTEST: usize = 6;

/// What a secret is written as where its value was said.
pub fn mark(name: &str) -> String {
    // A project's environment's is said by its name and its environment's,
    // as it is known where it was set.
    match name.split('/').collect::<Vec<_>>()[..] {
        [_, environment, name] => format!("[the secret `{name}` of the `{environment}` environment]"),
        _ => format!("[the secret `{name}`]"),
    }
}

/// What one program was handed, and what each is written as in its place.
#[derive(Clone, Default)]
pub struct Unsaid {
    /// What to look for, longest first so that a value that holds another
    /// is taken whole, and what is written instead.
    held: Vec<(String, String)>,
}

impl Unsaid {
    /// From each value and what it is said as.
    pub fn of(handed: impl IntoIterator<Item = (String, String)>) -> Unsaid {
        let mut held: Vec<(String, String)> = Vec::new();
        for (value, said) in handed {
            let lines = value.lines().map(str::trim).filter(|line| *line != value);
            for part in std::iter::once(value.as_str()).chain(lines) {
                if part.len() >= SHORTEST && !held.iter().any(|(kept, _)| kept == part) {
                    held.push((part.to_string(), said.clone()));
                }
            }
        }
        held.sort_by_key(|one| std::cmp::Reverse(one.0.len()));
        Unsaid { held }
    }

    /// From the variables of an environment that hold a secret, each with
    /// the name it is kept under: what [`crate::Hosting::dress_handed`]
    /// put there.
    pub fn named(named: &BTreeMap<String, String>, env: &BTreeMap<String, String>) -> Unsaid {
        Unsaid::of(named.iter().filter_map(|(variable, name)| Some((env.get(variable)?.clone(), mark(name)))))
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// `said` without what was handed.
    pub fn from(&self, said: &str) -> String {
        let mut said = said.to_string();
        for (value, name) in &self.held {
            if said.contains(value.as_str()) {
                said = said.replace(value.as_str(), name);
            }
        }
        said
    }

    /// The same of everything said in `value`, however deep.
    pub fn from_all(&self, value: &mut Value) {
        if self.held.is_empty() {
            return;
        }
        match value {
            Value::String(said) => *said = self.from(said),
            Value::Array(items) => items.iter_mut().for_each(|item| self.from_all(item)),
            Value::Object(fields) => fields.values_mut().for_each(|field| self.from_all(field)),
            _ => {}
        }
    }
}
