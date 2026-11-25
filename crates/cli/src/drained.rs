//! `down --drain`: the daemon stopped once what it has in hand is done
//! (`api::drain`), and `status` saying so while it drains.

use serde_json::{Value, json};

use crate::client::{Client, now_ms};
use crate::work::show;

/// Asks the daemon to drain and stop, and waits until it has drained or
/// come to its bound, saying what it waits for and how it came out.
pub async fn down(client: &Client, within: Option<&str>) -> Result<(), String> {
    let asked = client.post("/api/system/stop", json!({ "drain": true, "within": within })).await?;
    show(client, &asked, || {
        let until = asked["draining"]["until"].as_i64().unwrap_or_default();
        format!(
            "draining: nothing new is begun, and the daemon stops once what is in hand is done, {} at most.{}",
            left(until),
            waits(&asked)
        )
    });
    let over = client.get("/api/system/drain?wait=true").await?;
    show(client, &over, || said(&over));
    Ok(())
}

/// What a daemon that drains says it waits for, a line each, with what
/// becomes of each where it stops at its bound.
pub fn waits(standing: &Value) -> String {
    let mut lines = Vec::new();
    for turn in list(&standing["turns"]) {
        lines.push(format!(
            "\n  {}'s turn {}: if cut off, {}",
            text(turn, "slot"),
            text(turn, "turn"),
            text(turn, "becomes")
        ));
    }
    for run in list(&standing["runs"]) {
        lines.push(format!(
            "\n  a run of {} ({}): if cut off, {}",
            text(run, "of"),
            text(run, "flow"),
            text(run, "becomes")
        ));
    }
    match lines.is_empty() {
        true => " Nothing is in hand.".to_string(),
        false => format!(" It waits for:{}", lines.concat()),
    }
}

/// How a drain came out.
fn said(over: &Value) -> String {
    let waited = over["waited_ms"].as_i64().unwrap_or_default() / 1000;
    let cut: Vec<String> =
        list(&over["cut"]).iter().map(|cut| format!("\n  {}", cut.as_str().unwrap_or_default())).collect();
    match over["drained"] == true {
        true => format!("drained after {waited}s: nothing was left in hand, and the daemon is stopping"),
        false => format!(
            "stopped at its bound after {waited}s with this still in hand:{}\nThe daemon is stopping; `{} up` starts it again.",
            cut.concat(),
            brand::CLI
        ),
    }
}

/// What `status` says of what every page says across the top of a factory
/// that is a copy of another, or was told what to say there
/// (`folders::across`): nothing where they say nothing.
pub fn across(meta: &Value) -> Option<String> {
    meta["across"]["words"].as_str().map(str::to_string)
}

/// What `status` says first of what the daemon is: [`across`], then [`status`].
pub async fn first(client: &Client, meta: &Value) -> Vec<String> {
    across(meta).into_iter().chain(status(client, meta).await).collect()
}

/// What `status` says of a daemon that drains: nothing of one that does not.
pub async fn status(client: &Client, meta: &Value) -> Option<String> {
    let until = meta["draining"]["until"].as_i64()?;
    let standing = client.get("/api/system/drain").await.unwrap_or_default();
    Some(format!(
        "Draining to stop: nothing new is begun, and it stops once what is in hand is done, {} at most.{}",
        left(until),
        waits(&standing)
    ))
}

/// How long until `until`, in words.
fn left(until: i64) -> String {
    let seconds = ((until - now_ms()) / 1000).max(0);
    match seconds {
        0..120 => format!("in {seconds}s"),
        _ => format!("in {}m", seconds / 60),
    }
}

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}
