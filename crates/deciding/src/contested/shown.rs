//! What a contest's judges are shown, and who judges.
//!
//! Each entry whose work was kept is shown by its letter: where it was
//! done, what its agent said of it, what it changed and as much of its
//! patch as it was given; its way as well only where judging is not
//! blind; and, where its program could see the other contenders' work,
//! that it was not held apart, which is never left out. An entry whose
//! work was not kept is not shown: there is nothing of it to judge. The
//! seats are named one each, so that each judge's turn is its own and
//! what each gave out is handed back in the order they are named.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use super::{count, text};
use crate::add;

/// One entry, as its row says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    pub way: String,
    pub letter: String,
    /// `kept` where its work was kept.
    pub state: String,
    /// Its place's key: its worktree is `.lanes/<key>`, its branch `lane/<key>`.
    pub key: String,
    pub said: String,
    pub changed: String,
    pub patch: String,
    /// Its program could see the other contenders' work.
    pub unhidden: bool,
}

/// What the judges are shown of `entries`, and the letters they pick among.
pub fn shown(entries: &[Shown], blind: bool) -> (String, Vec<String>) {
    let mut text = String::new();
    let mut letters = Vec::new();
    for entry in entries.iter().filter(|entry| entry.state == "kept") {
        letters.push(entry.letter.clone());
        text.push_str(&format!("## Entry {}\n\n", entry.letter));
        if !blind {
            text.push_str(&format!("Done as `{}`.\n", entry.way));
        }
        text.push_str(&format!(
            "Done in `.lanes/{key}`, on the branch `lane/{key}`, which you may read for more.\n",
            key = entry.key
        ));
        if entry.unhidden {
            text.push_str("Not held apart: it could see the other contenders' work as it did its own.\n");
        }
        let said = if entry.said.trim().is_empty() { "(nothing)" } else { entry.said.trim() };
        text.push_str(&format!("\nWhat its agent said of it:\n\n{said}\n"));
        if !entry.changed.trim().is_empty() {
            text.push_str(&format!("\nWhat it changed:\n\n```\n{}\n```\n", entry.changed.trim_end()));
        }
        if !entry.patch.trim().is_empty() {
            text.push_str(&format!("\nIts patch:\n\n```diff\n{}\n```\n", entry.patch.trim_end()));
        }
        text.push('\n');
    }
    (text.trim_end().to_string(), letters)
}

/// What each of `seats` judges is started as, one at least.
pub fn seated(seats: i64) -> Vec<String> {
    (1..=seats.max(1)).map(|seat| format!("seat-{seat}")).collect()
}

/// Adds `contest.shown` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "contest.shown",
            "Say what a contest's judges are shown of the entries whose work was kept, by the letters they pick them by, and what each judge is started as.",
        )
        .deciding()
        .required("entries", T::List, "the entries' rows: each its `way`, `letter`, `state`, `key`, `said`, `changed`, `patch` and `not_held_apart`")
        .arg("blind", T::Bool, "whether a judge knows the entries by letters alone; true if not said")
        .arg("seats", T::Int, "how many judge; one if not said")
        .output("text", "what the judges are shown, an entry at a time")
        .output("letters", "the letters they pick among")
        .output("seated", "what each judge is started as, `seat-1` and on")
        .idempotent(),
        |_, args| async move {
            let rows = args.get("entries").and_then(Value::as_array).cloned().unwrap_or_default();
            if rows.is_empty() {
                return Err(ActionError::new("a contest's judges are shown its entries, and none is handed"));
            }
            let entries: Vec<Shown> = rows
                .iter()
                .map(|one| Shown {
                    way: text(one, "way"),
                    letter: text(one, "letter"),
                    state: text(one, "state"),
                    key: text(one, "key"),
                    said: text(one, "said"),
                    changed: text(one, "changed"),
                    patch: text(one, "patch"),
                    unhidden: one.get("not_held_apart").and_then(Value::as_bool).unwrap_or(false),
                })
                .collect();
            let blind = args.get("blind").and_then(Value::as_bool).unwrap_or(true);
            let (text, letters) = shown(&entries, blind);
            Ok(json!({ "text": text, "letters": letters, "seated": seated(count(&args, "seats")) }))
        },
    );
}
