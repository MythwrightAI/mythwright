//! Commands about one's own inbox: what was addressed to whoever asks
//! (a person, or with `--as` an agent), saying a line was read or is to
//! wait unread again, and putting one away; and writing a conversation
//! to somebody.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode};
use crate::work::{list, show, text};

/// One line as it is printed: its id, whether it waits, where it is of
/// and what it says, as far as whoever asks may read that now.
pub(crate) fn printed(line: &Value) -> String {
    let waits = match (line["away"].is_null(), line["read"].is_null()) {
        (false, _) => "away  ",
        (true, true) => "unread",
        (true, false) => "read  ",
    };
    let times = match line["count"].as_u64().unwrap_or(1) {
        0 | 1 => String::new(),
        times => format!(" ({times} times)"),
    };
    let said = &line["said"];
    let says = if said.is_object() {
        let within = match said["channel_title"].as_str() {
            // A conversation by what it is about, who is in it, and the
            // id it is read and answered by.
            Some(subject) if said["conversation"] == true => {
                let with: Vec<&str> = list(&said["members"]).iter().map(|one| text(one, "name")).collect();
                format!("conversation `{subject}` ({}; with {})", text(line, "place"), with.join(", "))
            }
            Some(title) => format!("`{title}`"),
            None => format!("#{}", text(said, "channel_name")),
        };
        let under = if said["thread"].is_null() { "" } else { ", in a thread" };
        format!("{within}{under}  [{}] {}", text(said, "author_name"), text(said, "body"))
    } else if line["on_record"].is_object() {
        // Something said on a record that named them: the record, who said it and the words; of a
        // page they follow, the change that raised it last, its words or what it did.
        let on = &line["on_record"];
        // A page of a wiki is said by where it is now, too.
        let path: Vec<&str> =
            list(&on["path"]).iter().filter_map(Value::as_str).chain([text(on, "title")]).collect();
        let at = if path.len() > 1 { format!(" ({})", path.join(" › ")) } else { String::new() };
        let words = on["did"].as_str().unwrap_or_else(|| text(on, "body"));
        format!("on {}{at}  [{}] {words}", text(on, "record"), text(on, "who"))
    } else if line["kind"] == "following" {
        // Something followed that changed, at once or as a digest: its own words say what and how much.
        let digest = if line["why"] == "digest" { "digest: " } else { "" };
        format!("{digest}{}", text(line, "says"))
    } else {
        // What a blueprint told: from the factory's bot, by its name, in
        // the blueprint's own words, with what it is about where the
        // words do not say.
        let (from, says, about) = (text(line, "from"), text(line, "says"), text(line, "about"));
        let by = from.split_once(':').map_or(from, |(_, name)| name);
        match about.is_empty() || says.contains(about) {
            true => format!("[{by}] {says}"),
            false => format!("[{by}] {says}  (about {about})"),
        }
    };
    format!("{}  {waits}  {says}{times}", text(line, "id"))
}

pub async fn inbox(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        None | Some("list") if args.has("follow") => return crate::followed::follow(client).await,
        None | Some("list") => {
            let all = if args.has("all") { "?all=true" } else { "" };
            let found = client.get(&format!("/api/inbox{all}")).await?;
            show(client, &found, || {
                let lines: Vec<String> = list(&found["lines"]).iter().map(printed).collect();
                match (lines.is_empty(), args.has("all")) {
                    (true, true) => "nothing was ever addressed to you here".to_string(),
                    (true, false) => {
                        "nothing waits in your inbox; --all lists what was put away too".to_string()
                    }
                    (false, _) => format!("{}\n{} unread", lines.join("\n"), found["unread"]),
                }
            });
        }
        Some("read") => {
            let read = match (args.word(2), args.has("all")) {
                (Some(line), false) => {
                    client.post(&format!("/api/inbox/{}/read", encode(line)), json!({})).await?
                }
                (None, true) => client.post("/api/inbox/read", json!({})).await?,
                _ => {
                    return Err(format!(
                        "say which line was read, by its id as `{cli} inbox` lists it, or --all for every one",
                        cli = brand::CLI
                    ));
                }
            };
            show(client, &read, || match read["read"].as_u64() {
                Some(1) => format!("1 line read; {} unread", read["unread"]),
                Some(lines) => format!("{lines} lines read; {} unread", read["unread"]),
                None => format!("read {}; {} unread", text(&read["line"], "id"), read["unread"]),
            });
        }
        Some("unread") => {
            let line = args.need(2, "the line to have wait unread again, by its id")?;
            let unread = client.post(&format!("/api/inbox/{}/unread", encode(line)), json!({})).await?;
            show(client, &unread, || {
                format!("{} waits unread again; {} unread", text(&unread["line"], "id"), unread["unread"])
            });
        }
        Some("parts") => {
            let line = args.need(2, "the line whose parts are shown, by its id")?;
            let read = client.get(&format!("/api/inbox/{}/parts", encode(line))).await?;
            show(client, &read, || {
                list(&read["parts"]).iter().enumerate().map(part_printed).collect::<Vec<_>>().join("\n")
            });
        }
        Some(verdict @ ("pass" | "fail")) => {
            // Every flag is read before anything is asked.
            let (reason, part) = (args.value("reason").unwrap_or_default(), args.value("part"));
            let line = args.need(2, "the line whose part lists the card, by its id")?;
            let card = args.need(3, "the card, by its id")?;
            let part = match part {
                Some(part) => part.to_string(),
                None => {
                    // The first part of the line that lists the card.
                    let read = client.get(&format!("/api/inbox/{}/parts", encode(line))).await?;
                    let lists = |part: &Value| list(&part["cards"]).iter().any(|one| text(one, "id") == card);
                    let found = list(&read["parts"]).iter().position(lists);
                    let found = found.ok_or_else(|| {
                        format!(
                            "no part of {line} lists {card}; `{} inbox parts {line}` says what each lists",
                            brand::CLI
                        )
                    })?;
                    (found + 1).to_string()
                }
            };
            let path = format!("/api/inbox/{}/parts/{}/cards/{}", encode(line), encode(&part), encode(card));
            let done = client.post(&path, json!({ "verdict": verdict, "reason": reason })).await?;
            show(client, &done, || match done["did"]["moved"].as_str() {
                Some(to) => format!("{card} {verdict}ed: moved to {to}"),
                None => format!("{card} passed: that is said on it"),
            });
        }
        Some("away") => {
            let line = args.need(2, "the line to put away, by its id")?;
            let away = client.post(&format!("/api/inbox/{}/away", encode(line)), json!({})).await?;
            show(client, &away, || {
                format!("put {} away; {} unread", text(&away["line"], "id"), away["unread"])
            });
        }
        Some(other) => {
            return Err(format!(
                "`{other}` is not something done with an inbox; `{} help inbox` says what is",
                brand::CLI
            ));
        }
    }
    Ok(())
}

/// Writes a conversation to one or several people and agents: its
/// subject, and what is first said in it.
pub async fn to(client: &Client, args: &Args) -> Result<(), String> {
    let who = args.need(1, "who it is to: a handle, or several with commas between (`nell,scout`)")?;
    let to: Vec<&str> = who.split(',').map(str::trim).filter(|one| !one.is_empty()).collect();
    let words = args.rest(2);
    if words.is_empty() {
        return Err("missing what is said: the words after who it is to".into());
    }
    let asked = json!({ "to": to, "subject": args.value("subject"), "text": words });
    let written = client.post("/api/inbox/conversations", asked).await?;
    show(client, &written, || {
        let channel = &written["channel"];
        let with: Vec<&str> = list(&channel["members"]).iter().map(|one| text(one, "name")).collect();
        format!(
            "wrote `{}` to {}: it is in the inbox of each, and is read and answered by its id\n\
             {cli} read {id} | {cli} post {id} <words...>",
            text(channel, "title"),
            with.join(", "),
            id = text(channel, "id"),
            cli = brand::CLI
        )
    });
    Ok(())
}

/// One part of a line as it is printed: its number, what it is, and of a
/// part of cards each card with where it is and the last thing said on it.
fn part_printed((at, part): (usize, &Value)) -> String {
    let number = at + 1;
    match text(part, "kind") {
        "links" => {
            let links: Vec<&str> = list(&part["links"]).iter().filter_map(Value::as_str).collect();
            format!("{number}. links: {}", links.join(" "))
        }
        _ => {
            let (column, kept) = (text(part, "column"), part["kept"] == true);
            let listed = match column {
                "" => format!("labelled {}", text(part, "label")),
                column if kept => format!("that were in {column} as it was told"),
                column => format!("in {column}"),
            };
            let head = format!(
                "{number}. the cards of {} {listed}; failed go to {}",
                text(part, "project"),
                text(part, "fail_to")
            );
            if let Some(refused) = part["refused"].as_str() {
                return format!("{head}\n   {refused}");
            }
            let cards = list(&part["cards"]).iter().map(|card| {
                let last = list(&card["thread"])
                    .last()
                    .map(|said| format!("  [{}] {}", text(said, "who"), text(said, "words")))
                    .unwrap_or_default();
                // One the line was told of that moved on says where it is now; one taken off the board, that it is gone.
                let now = match (card["gone"] == true, text(card, "status")) {
                    (true, _) => "  (gone)".to_string(),
                    (false, status) if !column.is_empty() && status != column => format!("  (in {status})"),
                    _ => String::new(),
                };
                format!("   {}  {}{now}{last}", text(card, "id"), text(card, "title"))
            });
            std::iter::once(head).chain(cards).collect::<Vec<_>>().join("\n")
        }
    }
}
