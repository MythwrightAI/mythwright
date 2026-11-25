//! A change of a record asked for by whoever may not make it, and decided
//! by somebody who could (`docs/approvals.md`): `ask`, `asked`, `approve`
//! and `decline`, each a request of the daemon, which decides everything.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode};
use crate::records::held_by;
use crate::work::{list, show, text};

/// One ask as it is printed: its id, how it stands, the record and each
/// cell from what it held to what it is to hold, with what it takes.
fn printed(ask: &Value) -> String {
    if text(ask, "kind") == "go" {
        return go_printed(ask);
    }
    let cells: Vec<String> = list(&ask["cells"])
        .iter()
        .map(|cell| {
            let takes: Vec<String> = list(&cell["takes"])
                .iter()
                .map(|scope| match scope["held"] == true {
                    true => text(scope, "scope").to_string(),
                    false => format!("{} (not held)", text(scope, "scope")),
                })
                .collect();
            let takes =
                if takes.is_empty() { String::new() } else { format!("; takes {}", takes.join(", ")) };
            format!("  {}: {} -> {}{takes}", text(cell, "name"), cell["before"], cell["after"])
        })
        .collect();
    // What changing the record at all takes, where whoever asks does not hold all of it.
    let whole: Vec<String> = list(&ask["takes"])
        .iter()
        .map(|scope| match scope["held"] == true {
            true => text(scope, "scope").to_string(),
            false => format!("{} (not held)", text(scope, "scope")),
        })
        .collect();
    let whole = match whole.iter().any(|one| one.ends_with("(not held)")) {
        true => format!("\n  changing it at all takes {}", whole.join(", ")),
        false => String::new(),
    };
    let why = match text(ask, "why") {
        "" => String::new(),
        why => format!("\n  why: {why}"),
    };
    // Who decided it, and why where they said: the factory, for one stale, expired or of an asker taken away.
    let decided = match (&ask["decided"], text(ask, "status")) {
        (Value::Null, _) | (_, "waiting") => String::new(),
        (decided, status) => {
            let by = match text(decided, "name") {
                "" => "the factory",
                name => name,
            };
            let why = match text(decided, "why") {
                "" => String::new(),
                why => format!(": {why}"),
            };
            format!("\n  {status} by {by}{why}")
        }
    };
    format!(
        "{}  {}  {} of {}, asked by {}{why}\n{}{whole}{decided}",
        text(ask, "ask"),
        text(ask, "status"),
        text(ask, "record"),
        text(ask, "collection"),
        text(&ask["asker"], "name"),
        cells.join("\n")
    )
}

/// A run that waits to be let go on, as it is printed: its id, how it
/// stands, the run and what it runs, under which environment, what it says
/// is to go on, and who decided it.
fn go_printed(ask: &Value) -> String {
    let under = match text(ask, "environment") {
        "" => String::new(),
        environment => format!(" under {environment}"),
    };
    let says = match text(ask, "says") {
        "" => String::new(),
        says => format!("\n  {says}"),
    };
    let decided = match (&ask["decided"], text(ask, "status")) {
        (Value::Null, _) | (_, "waiting") => String::new(),
        (decided, status) => {
            let by = match text(decided, "name") {
                "" => "the factory",
                name => name,
            };
            let why = match text(decided, "why") {
                "" => String::new(),
                why => format!(": {why}"),
            };
            format!("\n  {status} by {by}{why}")
        }
    };
    format!(
        "{}  {}  run {} of {}{under}, started by {}{says}{decided}",
        text(ask, "ask"),
        text(ask, "status"),
        text(ask, "run"),
        text(ask, "blueprint"),
        text(&ask["asker"], "name")
    )
}

/// `ask <record> <field=value>... [--to WHO] [--why WORDS] [--channel C [--thread M]]`
pub async fn ask(client: &Client, args: &Args) -> Result<(), String> {
    // Every flag is read before anything is asked.
    let (to, why, channel, thread) =
        (args.value("to"), args.value("why"), args.value("channel"), args.value("thread"));
    let record = args.need(1, "the record whose change is asked for")?;
    let mut pairs = Vec::new();
    while let Some(pair) = args.word(2 + pairs.len()) {
        pairs.push(pair);
    }
    if pairs.is_empty() {
        return Err("missing what is to change: one or more of `field=value`, as in `wall=off`".into());
    }
    let was = client.get(&format!("/api/records/{}", encode(record))).await?;
    let set =
        held_by(client, text(&was, "project"), text(&was, "collection"), &pairs, Some(&was["data"])).await?;
    let body =
        json!({ "record": record, "set": set, "to": to, "why": why, "channel": channel, "thread": thread });
    let came = client.post("/api/asked", body).await?;
    show(client, &came, || match text(&came, "came") {
        "made" => format!("changed {record}: you may make this change, so it was made"),
        "nothing" => format!("{record} holds that already; nothing was asked"),
        _ => {
            let told: Vec<&str> = list(&came["to"]).iter().filter_map(Value::as_str).collect();
            format!(
                "{}\nasked: {} told; `{cli} asked` says how it stands",
                printed(&came["ask"]),
                told.len(),
                cli = brand::CLI
            )
        }
    });
    Ok(())
}

/// `asked [<ask>]`: what whoever asks asked for, and what waits on them to decide.
pub async fn asked(client: &Client, args: &Args) -> Result<(), String> {
    if let Some(id) = args.word(1) {
        let ask = client.get(&format!("/api/asked/{}", encode(id))).await?;
        show(client, &ask, || printed(&ask));
        return Ok(());
    }
    let found = client.get("/api/asked").await?;
    show(client, &found, || {
        let all: Vec<String> = list(&found["asked"]).iter().map(printed).collect();
        match all.is_empty() {
            true => "nothing asked by you, and nothing waiting for you to decide".to_string(),
            false => all.join("\n"),
        }
    });
    Ok(())
}

/// `approve <ask>` and `decline <ask> [--why WORDS]`.
pub async fn decide(client: &Client, args: &Args, approve: bool) -> Result<(), String> {
    let why = if approve { None } else { args.value("why") };
    let id = args.need(1, "the ask, by its id as `asked` lists it")?;
    let path = format!("/api/asked/{}/{}", encode(id), if approve { "approve" } else { "decline" });
    let ask = client.post(&path, json!({ "why": why })).await?;
    show(client, &ask, || printed(&ask));
    Ok(())
}
