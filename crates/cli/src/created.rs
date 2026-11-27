//! What `project create` says under a project it has just added: what
//! it was given beside its files, each with the command that reads it,
//! and what it was not, with why.

use serde_json::Value;

use crate::work::{list, text};

/// What a project that was just added was given beside its files, and
/// what it was not, as lines to go under where it is.
pub fn given(project: &Value, bare: bool) -> String {
    // Whether what it was started with is its first commit, said first:
    // without one a card's work has no branch of its own.
    let first = match text(&project["first_commit"], "says") {
        "" => String::new(),
        says => format!("\n{says}"),
    };
    format!("{first}{}", beside(project, bare))
}

/// What it was given beside its files.
fn beside(project: &Value, bare: bool) -> String {
    let (id, cli) = (text(project, "id"), brand::CLI);
    if bare {
        return format!(
            "\nIt is bare, as asked: a board is `{cli} apply std/board --project {id}`, a fleet `{cli} fleet init {id}` and then `{cli} apply fleet --project {id}`, and a library `{cli} apply std/docs --project {id}`"
        );
    }
    let set_up = &project["set_up"];
    let mut lines = String::new();
    match (set_up["board"].is_string(), set_up["fleet"].is_string()) {
        (true, true) => lines.push_str(&format!(
            "\nIt has a board (`{cli} board {id}`) and a fleet, whose agents are `{cli} agent list` and whose tables are `{cli} record list {id} agents` and `{cli} record list {id} stages`"
        )),
        (true, false) => lines.push_str(&format!("\nIt has a board (`{cli} board {id}`)")),
        _ => {}
    }
    if set_up["docs"].is_string() {
        lines
            .push_str(&format!("\nIt has a library, its documents as pages (`{cli} record list {id} docs`)"));
    }
    if set_up["features"].is_string() {
        lines.push_str(&format!(
            "\nIt has features, for work bigger than a card and for cards put in order together (`{cli} record list {id} features`)"
        ));
    }
    if set_up["environments"].is_string() {
        lines.push_str(&format!(
            "\nIt has environments, and whatever is applied for it says which it runs under (`{cli} project environment {id}`)"
        ));
    }
    for left in list(&set_up["left"]) {
        lines.push_str(&format!("\n{}", left.as_str().unwrap_or_default()));
    }
    lines
}
