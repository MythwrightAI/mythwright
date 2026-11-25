//! `agent remove`: an agent taken away for good, or with `--look` what
//! that would come to. The daemon says who may and what is in the way
//! (`agents::removed`); this says what it answered. And what a project
//! that was forgotten leaves running, which the daemon answers with.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode};
use crate::work::{list, show, text};

/// What goes with an agent and what stays, as the daemon counted it.
fn said(found: &Value) -> String {
    let name = text(found, "name");
    let names = |key: &str| -> Vec<&str> { list(&found[key]).iter().filter_map(Value::as_str).collect() };
    let mut lines = Vec::new();
    lines.push(match names("channels").as_slice() {
        [] => "its slot, its credential and the folder that was its own".to_string(),
        channels => format!(
            "its slot, its credential, the folder that was its own, and its place in #{}",
            channels.join(", #")
        ),
    });
    let turns = found["turns"].as_i64().unwrap_or(0);
    let asked = match turns {
        0 => String::new(),
        1 => ", and the one piece of work it was asked".to_string(),
        many => format!(", and the {many} pieces of work it was asked"),
    };
    lines.push(format!("what it said and did stays, under its name: what it said in channels{asked}"));
    match names("cards").as_slice() {
        [] => {}
        cards => lines.push(format!(
            "{} {} given to it, and stay{} so until somebody gives {} to another (`{} board give <card> <agent>`)",
            cards.join(", "),
            if cards.len() == 1 { "is" } else { "are" },
            if cards.len() == 1 { "s" } else { "" },
            if cards.len() == 1 { "it" } else { "them" },
            brand::CLI,
        )),
    }
    match found["withdrawn"].as_u64().unwrap_or(0) {
        0 => {}
        1 => lines.push("the one change it asked for that waited is withdrawn".to_string()),
        many => lines.push(format!("the {many} changes it asked for that waited are withdrawn")),
    }
    if let Some(left) = found["folder_left"].as_str() {
        lines.push(left.to_string());
    }
    lines.push(format!(
        "an agent that is later called `{name}` is another agent, and has none of this for its own"
    ));
    lines.join("\n  ")
}

pub async fn agent(client: &Client, args: &Args) -> Result<(), String> {
    let name = args.need(2, "the agent to take away")?;
    let path = format!("/api/agents/{}/remove", encode(name));
    if args.has("look") {
        let found = client.get(&path).await?;
        show(client, &found, || match found["refused"].as_str() {
            Some(why) => format!("{name} cannot be taken away as things stand: {why}"),
            None => format!("taking {name} away would take\n  {}\nnothing was changed", said(&found)),
        });
        return Ok(());
    }
    let gone = client.post(&path, json!({})).await?;
    show(client, &gone, || format!("{name} is taken away, for good, with\n  {}", said(&gone)));
    Ok(())
}

/// What still goes on for a project that was just forgotten, as the
/// daemon answered it (`running`), each kind with the one command that
/// ends it. Nothing, where nothing does or whoever asked may read none of
/// it. Forgetting stops nothing: this is what is left to stop.
pub fn still_running(running: &Value) -> Vec<String> {
    let cli = brand::CLI;
    let names = |key: &str| -> Vec<&str> { list(&running[key]).iter().filter_map(Value::as_str).collect() };
    let mut lines = Vec::new();
    let applied = list(&running["instances"]);
    if !applied.is_empty() {
        lines.push(format!(
            "what was applied for it is still running, each with whatever it started; `{cli} stop <id>` stops one:"
        ));
        lines.extend(applied.iter().map(|one| format!("  {}  {}", text(one, "id"), text(one, "blueprint"))));
    }
    match names("agents").as_slice() {
        [] => {}
        agents => lines.push(format!(
            "its agents are still up: {}. One a fleet brought up stops with that fleet; another is stopped with \
             `{cli} agent stop <name>`",
            agents.join(", ")
        )),
    }
    match names("channels").as_slice() {
        [] => {}
        channels => lines.push(format!(
            "its channels are still there: #{}. `{cli} channel remove <name>` moves one to the Trash, with what was said in it",
            channels.join(", #")
        )),
    }
    lines
}
