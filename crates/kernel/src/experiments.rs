//! What is being built of this program itself and is not yet whole, offered
//! to whoever runs a factory to try: an experiment. A feature
//! that is several pieces may land piece by piece behind one, so that its
//! pieces need not wait on a long branch; each says the card it is for,
//! and is taken out of the list when the feature is whole. Most work goes
//! through whole and is behind none.
//!
//! An experiment is off unless the factory's owner turns it on, in
//! Settings, Experiments: which are on is a setting of the factory's, kept
//! in its journal ([`NAMESPACE`]) as the names of those turned on, so that
//! a name the list no longer has is passed over and a copy of the factory
//! keeps it. Nothing here reaches outside the factory: an experiment is
//! code that ships, turned on.

use serde_json::{Value, json};

/// Where which experiments are on is kept, in the factory's journal.
pub const NAMESPACE: &str = "experiments";

/// The key the names of those turned on are kept under.
pub const ON: &str = "on";

/// Something not yet whole, that a factory may turn on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Experiment {
    /// What it is known by: lowercase letters, digits and `-`.
    pub name: &'static str,
    /// The card it is for, by its number on the board this program is built from (`390`).
    pub card: &'static str,
    /// What it is, in a sentence.
    pub says: &'static str,
}

/// Every experiment there is now. None is: when a feature lands a piece
/// that is not usable alone, it is a line here until the feature is whole.
pub const LIST: &[Experiment] = &[];

/// Which of `list` are on, of the names kept (`on`): each as it is shown,
/// with whether it is on. A name kept that the list no longer has is
/// passed over.
pub fn shown(list: &[Experiment], on: &[String]) -> Vec<Value> {
    list.iter()
        .map(|one| json!({ "name": one.name, "card": one.card, "says": one.says, "on": is_on(on, one.name) }))
        .collect()
}

/// Whether the experiment `name` is on, of the names kept.
pub fn is_on(on: &[String], name: &str) -> bool {
    on.iter().any(|kept| kept == name)
}

/// The names kept once `name` is turned on or off (`turn`), of `list`;
/// refused, saying what there is, where the list has no such experiment.
/// A name kept that the list no longer has is left out.
pub fn turned(list: &[Experiment], on: &[String], name: &str, turn: bool) -> Result<Vec<String>, String> {
    if !list.iter().any(|one| one.name == name) {
        let there = match list.is_empty() {
            true => "there are none now".to_string(),
            false => format!(
                "there are {}",
                list.iter().map(|one| format!("`{}`", one.name)).collect::<Vec<_>>().join(", ")
            ),
        };
        return Err(format!("there is no experiment called `{name}`: {there}"));
    }
    let mut kept: Vec<String> = on
        .iter()
        .filter(|kept| *kept != name && list.iter().any(|one| one.name == kept.as_str()))
        .cloned()
        .collect();
    if turn {
        kept.push(name.to_string());
    }
    Ok(kept)
}

/// The names kept, read back: nothing where nothing was kept.
pub fn read(kept: Option<&Value>) -> Vec<String> {
    kept.and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}
