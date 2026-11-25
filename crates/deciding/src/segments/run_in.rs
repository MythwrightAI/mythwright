//! Where a command that is run for an item is run: where the item is
//! worked, or in the project's own folder, as whoever says the command
//! says; or nowhere, the item having no place of its own where it was to
//! be run there. A function of what it is handed, with its cases.

use blueprint::{ActionSpec, ValueType};
use conductor::Registry;
use serde_json::json;

use super::{flag, text};
use crate::add;

/// Every word `in` may be.
pub const IN_WORDS: &[&str] = &["card", "project"];

/// What [`run_in`] is asked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunIn {
    pub item: String,
    /// The command, as it was said.
    pub run: String,
    /// Where it is to be run: `card` (what nothing said means), where
    /// the item is worked; `project`, in the project's own folder.
    pub within: String,
    /// Where the item is worked, from the top of the project: nothing
    /// where it is worked in the project itself.
    pub dir: String,
    /// Whether that is there: not where the project could give the item
    /// a place of its own (a repository with history) and it has none.
    pub there: bool,
    /// Whether whoever works the item works it in a place of its own
    /// where the project can give it one. Where not (an item that is
    /// looked over or planned, and never worked here), the project's own
    /// folder stands for a place it has not got, as it always did.
    pub worktree: bool,
}

/// The folder the command is run in, from the top of the project, the
/// project's own being nothing; or why it is not run at all, which
/// leaves the item for a person. Where an item is to be worked in a
/// place of its own and has none yet (a command laid before the work),
/// or none any more, it is refused, and not run in the project's folder
/// instead: that folder is a person's, with whatever they have in hand
/// there, and a command meant for an item's own copy of the work (one
/// that tidies or rewrites files) would be run over it unasked. The row
/// that means the project's folder says so.
pub fn run_in(asked: &RunIn) -> Result<String, String> {
    let run = asked.run.trim();
    match asked.within.trim() {
        "project" => Ok(String::new()),
        "" | "card" if asked.there => Ok(asked.dir.clone()),
        "" | "card" if !asked.worktree => Ok(String::new()),
        "" | "card" => Err(format!(
            "{}: `{run}` was not run, for it has no place of its own here: it is not worked yet, or its \
             place was tidied away. A row that says `in=project` in `with` has it run in the project's \
             folder; it is left for a person",
            asked.item
        )),
        other => Err(format!(
            "{}: `{run}` was not run, for its row says `in` is `{other}`: it is `card`, where it is worked, \
             or `project`, the project's own folder; it is left for a person",
            asked.item
        )),
    }
}

/// Adds what is asked here to what a blueprint can do.
pub(super) fn register(registry: &mut Registry) {
    use ValueType as T;
    add(
        registry,
        ActionSpec::new(
            "segments.where",
            "Say where a command that is run for an item is run: where the item is worked, or in the project's \
             own folder; or that it is not run, the item having no place of its own yet.",
        )
        .deciding()
        .required("item", T::String, "the item")
        .arg("command", T::String, "the command, as it was said")
        .arg("in", T::String, "where it is to be run: `card` (if not said), where the item is worked; `project`, in the project's own folder")
        .arg("dir", T::String, "where the item is worked, from the top of the project; empty where that is the project itself")
        .arg("there", T::Bool, "whether that is there: not where the project could give the item a place of its own and it has none (true, if not said)")
        .arg("worktree", T::Bool, "whether the item is worked in a place of its own where the project can give it one: where it is not (if not said), the project's own folder stands for one it has not got")
        .output("runs", "whether the command is run")
        .output("dir", "where, from the top of the project; empty for the project's own folder")
        .output("why", "why it is not run, where it is not: the item is left for a person")
        .idempotent(),
        move |_, args| async move {
            let asked = RunIn {
                item: text(&args, "item"),
                run: text(&args, "command"),
                within: text(&args, "in"),
                dir: text(&args, "dir"),
                there: !args.contains_key("there") || flag(&args, "there"),
                worktree: flag(&args, "worktree"),
            };
            Ok(match run_in(&asked) {
                Ok(dir) => json!({ "runs": true, "dir": dir, "why": "" }),
                Err(why) => json!({ "runs": false, "dir": "", "why": why }),
            })
        },
    );
}
