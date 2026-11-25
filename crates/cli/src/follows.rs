//! `follow`, `unfollow` and `following`: being told in one's inbox when a
//! page, a wiki, a board, a table, a library, a channel or a project
//! changes, and how often (`help follow`).

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::Client;
use crate::work::{list, show, text};

fn usage() -> String {
    format!(
        "usage: {cli} follow <kind>:<thing> [--with-pages] [--often once|hourly|daily|off] [--at HH:MM] | \
         {cli} unfollow <kind>:<thing> | {cli} following",
        cli = brand::CLI
    )
}

/// What a follow is in a line: what it is, what it is called and how often.
fn line(follow: &Value) -> String {
    let often = match (text(follow, "often"), text(follow, "at")) {
        ("hourly", _) => "hourly".to_string(),
        ("daily", at) => format!("daily at {at}"),
        ("off", _) => "followed, told nothing".to_string(),
        _ => "at once".to_string(),
    };
    let pages = if follow["pages"] == json!(true) { ", with every page under it" } else { "" };
    format!("{}:{}  {}  ({often}{pages})", text(follow, "kind"), text(follow, "thing"), text(follow, "title"))
}

/// What `<kind>:<thing>` names, as the daemon is asked it.
fn named(args: &Args) -> Result<(String, String), String> {
    let said = args.word(1).ok_or_else(usage)?;
    match said.split_once(':') {
        Some((kind, thing)) if !kind.is_empty() && !thing.is_empty() => {
            Ok((kind.to_string(), thing.to_string()))
        }
        _ => Err(format!(
            "say what to follow as `<kind>:<thing>`, as `board:shop` or `page:Frogs`; {}",
            usage()
        )),
    }
}

/// What a page is followed with, as it is said on the command line.
pub fn how(args: &Args) -> Value {
    let mut asked = json!({});
    if args.has("with-pages") {
        asked["pages"] = json!(true);
    }
    if let Some(often) = args.value("often") {
        asked["often"] = json!(often);
    }
    if let Some(at) = args.value("at") {
        asked["at"] = json!(at);
    }
    asked
}

pub async fn follow(client: &Client, args: &Args, follows: bool) -> Result<(), String> {
    let (kind, thing) = named(args)?;
    let mut asked = how(args);
    asked["kind"] = json!(kind);
    asked["thing"] = json!(thing);
    asked["follows"] = json!(follows);
    let done = client.post("/api/following", asked).await?;
    show(client, &done, || match follows {
        true => format!("you follow {}", line(&done)),
        false => format!("you follow {} no longer", text(&done, "title")),
    });
    Ok(())
}

pub async fn following(client: &Client) -> Result<(), String> {
    let found = client.get("/api/following").await?;
    show(client, &found, || {
        let lines: Vec<String> = list(&found["following"]).iter().map(line).collect();
        if lines.is_empty() { "you follow nothing".to_string() } else { lines.join("\n") }
    });
    Ok(())
}
