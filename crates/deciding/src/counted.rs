//! What so many agents of one name are called, and which of those there
//! were are no longer counted.
//!
//! A blueprint that keeps a number of agents alike (`std/role`) is told
//! how many, and what they are called; it keeps how many it last brought
//! up and under what name. Brought to another count, or another name, it
//! brings up those that are counted and has those that no longer are
//! leave. Which those are is [`counted`], a function of the two counts
//! and the two names: the blueprint asks it as `agent.counted`, and then
//! does one thing for each name it is handed.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use crate::add;

/// The most of one name: something to go through one by one, and what a
/// loop of a blueprint goes to. More is a mistake in what was asked.
const MOST: i64 = 1000;

/// Those that are counted, and those that no longer are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counted {
    /// What they are called before their numbers: the name that was given,
    /// or what stands for it where none was.
    pub named: String,
    /// Each that is counted now, from the first: `<named>-1`, `<named>-2`.
    pub names: Vec<String>,
    /// Each that was there and is counted no longer: those past the count,
    /// or every one under a name that is no longer theirs.
    pub past: Vec<String>,
}

/// `count` agents called `name`, or `otherwise` where no name is given,
/// where `had` were last brought up under `called` (nothing, where none
/// were).
///
/// Under the same name, those past the count leave. Under another, all
/// that had the old name do, and as many come up under the new.
pub fn counted(name: &str, otherwise: &str, count: i64, had: i64, called: &str) -> Result<Counted, String> {
    let named = if name.is_empty() { otherwise } else { name };
    if named.is_empty() {
        return Err("what they are called is `name`, or `role` where no name is given, and neither says \
                    anything"
            .to_string());
    }
    if count < 0 {
        return Err(format!(
            "`count` is how many agents there are in the role, and is not less than none; it is {count}."
        ));
    }
    if had < 0 {
        return Err(format!("`had` is how many there were, and is not less than none; it is {had}"));
    }
    if let Some(many) = [count, had].into_iter().find(|many| *many > MOST) {
        return Err(format!("there are {MOST} of one name at most, and {many} were asked for"));
    }
    let called_as = |base: &str, number: i64| format!("{base}-{number}");
    let names = (1..=count).map(|number| called_as(named, number)).collect();
    let past = if called.is_empty() || called == named {
        (count + 1..=had).map(|number| called_as(named, number)).collect()
    } else {
        (1..=had).map(|number| called_as(called, number)).collect()
    };
    Ok(Counted { named: named.to_string(), names, past })
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "agent.counted",
            "Say what so many agents of one name are called, and which of those there were are no longer counted.",
        )
        .deciding()
        .arg("name", T::String, "what they are called, each with its number after it (`<name>-1`)")
        .arg("role", T::String, "what they are called where no `name` is given")
        .required("count", T::Int, "how many of them there are now")
        .arg("had", T::Int, "how many were last brought up (default 0)")
        .arg("called", T::String, "what those were called; empty where there were none, or it was not kept")
        .output("named", "what they are called before their numbers")
        .output("names", "each that is counted now, from the first")
        .output("past", "each that was there and is counted no longer")
        .idempotent(),
        |_, args| async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default();
            let number = |key: &str| match args.get(key) {
                None | Some(Value::Null) => Ok(0),
                Some(said) => said
                    .as_i64()
                    .ok_or_else(|| ActionError::new(format!("`{key}` is a whole number, and was given {said}"))),
            };
            let counted = counted(text("name"), text("role"), number("count")?, number("had")?, text("called"))
                .map_err(ActionError::new)?;
            Ok(json!({ "named": counted.named, "names": counted.names, "past": counted.past }))
        },
    );
}
