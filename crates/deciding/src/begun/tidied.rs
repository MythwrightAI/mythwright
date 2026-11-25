//! What is said of an item's place having been tidied away, where there
//! is anything to say.
//!
//! An item that is finished has its place to work in removed, and its
//! branch with it when nothing would be lost (`place.clear`). Mostly that
//! is done and nothing is said. Where something was kept, or git would
//! not do what was asked, whoever reports the work says so in a sentence:
//! [`tidied`] is that sentence, a function of what clearing answered.

use blueprint::{ActionSpec, ValueType as T};
use conductor::Registry;
use serde_json::{Value, json};

use crate::add;

/// What clearing an item's place came to, as whoever asked it says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tidied {
    pub item: String,
    /// What clearing answered: `cleared`, `nothing`, `place_kept`,
    /// `branch_kept` or `untidied`; nothing, where it was not asked or
    /// failed outright.
    pub came: String,
    /// Why, where it says; and the error, where asking failed.
    pub why: String,
    pub failed: bool,
    pub error: String,
    /// The remote the branch may have been sent to.
    pub remote: String,
}

/// The sentence for where the work is reported; nothing, where the place
/// was cleared, there was none, or it was not asked.
pub fn tidied(asked: &Tidied) -> String {
    let item = &asked.item;
    let why = [asked.why.as_str(), asked.error.as_str(), "the command failed"]
        .into_iter()
        .find(|said| !said.is_empty())
        .unwrap_or_default();
    match asked.came.as_str() {
        "place_kept" => format!(
            "{item}: its worktree .lanes/{item} and its branch lane/{item} were kept, because the worktree could \
             not be removed without losing something: {}",
            asked.why
        ),
        "branch_kept" => format!(
            "{item}: its branch lane/{item} was kept, because it has work that is neither in the project's own \
             branch nor on {}",
            asked.remote
        ),
        "untidied" => format!("{item}: its worktree and branch could not be tidied away: {why}"),
        _ if asked.failed => format!("{item}: its worktree and branch could not be tidied away: {why}"),
        _ => String::new(),
    }
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "place.tidied",
            "Say, in a sentence for where the work is reported, what clearing an item's place came to where \
             something was kept or could not be done; nothing, where there is nothing to say.",
        )
        .deciding()
        .required("item", T::String, "the item")
        .arg("came", T::String, "what `place.clear` answered")
        .arg("why", T::String, "why, where it said")
        .arg("failed", T::Bool, "asking it failed outright")
        .arg("error", T::String, "what it failed with")
        .arg("remote", T::String, "the remote the item's branch may have been sent to")
        .output("says", "the sentence; empty where nothing is to be said")
        .idempotent(),
        move |_, args| async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let asked = Tidied {
                item: text("item"),
                came: text("came"),
                why: text("why"),
                failed: args.get("failed").and_then(Value::as_bool).unwrap_or(false),
                error: text("error"),
                remote: text("remote"),
            };
            Ok(json!({ "says": tidied(&asked) }))
        },
    );
}
