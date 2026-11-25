//! What is said of an apply that was asked for and answered: what it left
//! or moved of the texts it reaches, and how the instance came to its own.

use serde_json::Value;

use crate::client::{Client, took};
use crate::work::{list, plain, show, step_rows, text};

/// One text as it moved, by version first: `std/lane-round 3 to 4: the
/// step `work` is changed`.
pub fn moved(one: &Value) -> String {
    let lead = blueprint::differs::lead(text(one, "name"), one["was"].as_str(), one["now"].as_str());
    let changed: Vec<&str> = list(&one["changed"]).iter().filter_map(Value::as_str).collect();
    match changed.is_empty() {
        true => lead,
        false => format!("{lead}: {}", changed.join("; ")),
    }
}

/// What is said above the texts an apply moved a tree to.
fn applied_how(applied: &Value) -> &'static str {
    match (applied["back"] == true, applied["said_only"] == true) {
        (true, true) => "putting it back would put it on (nothing was changed)",
        (true, false) => "put back on the texts it had",
        _ => "moved to what ships now",
    }
}

/// What is printed of an apply that was asked for and answered.
pub fn applied_as(client: &Client, applied: &Value) -> Result<(), String> {
    let finished = &applied["finished"];
    // What became of the project's own blueprints that have changed since
    // this was last applied: left as they were, unless whoever applies
    // said to take them, and said either way.
    let named = |of: &str, which: &str| -> Vec<String> {
        let names = list(&applied[of][which]).iter().filter_map(Value::as_str);
        names.map(|name| format!("`{name}`")).collect()
    };
    let (taken, left) = (named("vouched", "taken"), named("vouched", "left"));
    if !client.json && !left.is_empty() {
        println!(
            "{}",
            plain(&format!(
                "the project's own {} changed since this was last applied, and {} left as {} then. Read what \
                 changed, and apply with `--take` to have the project's blueprints run as they are now",
                left.join(", "),
                if left.len() == 1 { "is" } else { "are" },
                if left.len() == 1 { "it was" } else { "they were" }
            ))
        );
    }
    if !client.json && !taken.is_empty() {
        println!("{}", plain(&format!("took up the project's own {} as now written", taken.join(", "))));
    }
    // The same of what ships, which is another when the program is: what
    // is running goes on with it as it was reached until told otherwise.
    let ships = |which: &str| -> Vec<String> {
        list(&applied["reached"][which]).iter().map(|one| format!("  {}", moved(one))).collect()
    };
    let (taken, left) = (ships("taken"), ships("left"));
    if !client.json && !left.is_empty() {
        let cli = brand::CLI;
        println!(
            "what ships with this program is other than what this was applied with, and what is running goes \
             on with it as it was. `{cli} apply ... --take` moves it to what ships now:\n{}",
            left.join("\n")
        );
    }
    if !client.json && !taken.is_empty() {
        println!("{}:\n{}", applied_how(applied), taken.join("\n"));
    }
    // Asked only what putting it back would put back: that was the answer.
    if !client.json && applied["said_only"] == true {
        if taken.is_empty() {
            println!("it is on the texts it had; putting it back would change nothing");
        }
        return Ok(());
    }
    show(client, applied, || {
        let instance = text(applied, "instance");
        // Asked for what it is in the middle of: its steps were not started
        // over, and the run it is given is the one that was going.
        let under_way = applied["changed"] == false && !applied["run"].is_null();
        // Its own text is what it was, and what it reaches moved: what is
        // under it is brought along, which is not nothing.
        let moved = !list(&applied["reached"]["taken"]).is_empty();
        if applied["changed"] == false && !under_way && moved {
            return format!(
                "instance {instance} runs this text already; what is under it is brought to what it \
                 now goes by, each part's steps run again where it begins otherwise"
            );
        }
        if applied["changed"] == false && !under_way {
            return format!(
                "instance {instance} is already running this; nothing to do (`--force` to run it again)"
            );
        }
        // How one that was there came to what was asked is said, since
        // each is another thing to have happened: it took the text where
        // it is and nothing ran; it was carried over and none of its steps
        // ran (or could not be, and is back as it was); or its steps ran
        // again.
        let how = applied["took"].as_str().unwrap_or_default();
        if how == "in_place" {
            return format!(
                "instance {instance} took the text now there where it is; none of its steps ran again"
            );
        }
        let carried = how == "carried" || finished["flow"] == "on_update";
        let verb = match (applied["created"] == true, carried, finished["status"] == "ok") {
            _ if under_way => "is already in the middle of this, so nothing was started over",
            (true, _, _) => "started",
            (false, true, _) if finished.is_null() => "being carried over",
            (false, true, true) => "carried over",
            (false, true, false) => "not carried over",
            (false, false, _) => "updated, its steps run again",
        };
        if finished.is_null() {
            return format!("instance {instance} {verb}; run {}", text(applied, "run"));
        }
        let duration =
            took(finished["finished"].as_i64().unwrap_or(0) - finished["started"].as_i64().unwrap_or(0));
        let mut lines = vec![format!(
            "instance {instance} {verb}; run {} {} in {duration}",
            text(finished, "id"),
            text(finished, "status")
        )];
        lines.push(step_rows(finished));
        if finished["outputs"].as_object().is_some_and(|outputs| !outputs.is_empty()) {
            lines.push(format!("outputs: {}", finished["outputs"]));
        }
        lines.join("\n")
    });
    // What the daemon had to say of the apply beside its outcome: a file
    // of the project's own that was passed over for the one that ships.
    if !client.json {
        for said in list(&applied["said"]) {
            eprintln!("note: {}", said.as_str().unwrap_or_default());
        }
        // And what the check says of the text short of refusing it, which
        // `check` would have said: the apply went ahead all the same.
        for warning in list(&applied["warnings"]) {
            let start = &warning["span"]["start"];
            eprintln!("warning: line {}: {}", start["line"], text(warning, "message"));
        }
    }
    match finished["status"].as_str() {
        Some("ok") | None => Ok(()),
        Some(_) => Err(text(finished, "error").to_string()),
    }
}

/// Whether an instance still hears every project's events where its
/// handlers and waits say no `scope`: one a program from before applied,
/// which goes on hearing as it was written to until it is applied again.
/// A daemon from before says nothing of it, and nothing is said.
pub fn hears_every_project(instance: &Value) -> bool {
    matches!(instance["status"].as_str(), Some("starting" | "active"))
        && instance.get("heard").is_some_and(Value::is_null)
}

/// What is said of such an instance, after what it is called.
pub const HEARS_EVERY_PROJECT: &str = "every project's events where it says no `scope`, as when it was applied; \
                                       applied again (`--force` for the same text) it hears its own project's \
                                       and the factory's own";

/// What is said under a list of instances of those among them that still
/// hear every project's, if any do.
pub fn hearing_every_project(instances: &[Value]) -> Option<String> {
    let ids: Vec<&str> = instances
        .iter()
        .filter(|one| hears_every_project(one))
        .filter_map(|one| one["id"].as_str())
        .collect();
    let (first, more) = ids.split_at(ids.len().min(8));
    let more = if more.is_empty() { String::new() } else { format!(" and {} more", more.len()) };
    (!ids.is_empty())
        .then(|| format!("\n{}{more}: {} hear {HEARS_EVERY_PROJECT}", first.join(", "), ids.len()))
}
