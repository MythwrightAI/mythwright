//! `search`: what the factory's index finds for some words, held to a
//! kind and a project where asked.

use crate::args::Args;
use crate::client::{Client, encode};
use crate::work::{show, text};

fn list(value: &serde_json::Value) -> &[serde_json::Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

pub async fn search(client: &Client, args: &Args) -> Result<(), String> {
    let words = args.rest(1);
    if words.is_empty() {
        return Err("missing what to search for".into());
    }
    let mut path = format!("/api/search?q={}", encode(&words));
    for filter in ["kind", "project", "limit"] {
        if let Some(value) = args.value(filter) {
            path.push_str(&format!("&{filter}={}", encode(value)));
        }
    }
    let found = client.get(&path).await?;
    show(client, &found, || {
        let mut lines: Vec<String> = Vec::new();
        for hit in list(&found["hits"]) {
            // The marks around matched words are for the client; drop them here.
            let snippet: String =
                text(hit, "snippet").chars().filter(|c| !('\u{e000}'..='\u{e001}').contains(c)).collect();
            // Where it is, where that says more than what it is called: its table, its wiki, its library.
            let place = Some(text(hit, "place")).filter(|place| !place.is_empty());
            let place = place.map(|place| format!("  ({place})")).unwrap_or_default();
            lines.push(format!("{}  {}{place}", text(hit, "kind"), text(hit, "title")));
            lines.push(format!("    {}", snippet.replace('\n', " ")));
        }
        let index = &found["index"];
        if index["phase"] != "idle" {
            lines.push(format!("(still indexing: {} of {})", index["done"], index["total"]));
        }
        if lines.is_empty() { "nothing found".to_string() } else { lines.join("\n") }
    });
    Ok(())
}
