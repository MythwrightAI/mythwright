//! `topic`: a forum's topics beside the channels, each a page of a wiki,
//! named by its title or its id. What a topic says (its body, its summary, where it stands)
//! is changed as any record is (`record set <topic> body=...`).

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode};
use crate::work::{list, show, text};

fn usage() -> String {
    format!(
        "usage: {} topic list | make <title...> [--body TEXT] [--status open|standing] \
         [--workspace W | --project P | --under <page>] | show <topic> | reply <topic> <words...> [--to <reply>] | \
         answer <topic> <reply>|--none | \
         move <topic> --under <page>|--top [--before <page>] | move <wiki> --workspace W|--project P | \
         follow <topic> [--with-pages] [--often once|hourly|daily|off] [--at HH:MM] [--not]",
        brand::CLI
    )
}

/// A topic's line: its id, where it stands (a blank column where it has no
/// status) and what it is called.
fn line(topic: &Value) -> String {
    let data = &topic["data"];
    let place = match (data["project"].as_str(), data["workspace"].as_str()) {
        (Some(project), _) => format!("project {project}"),
        (None, Some(workspace)) => format!("workspace {workspace}"),
        _ => "nowhere".to_string(),
    };
    let replies = match topic["replies"].as_u64() {
        Some(0) | None => String::new(),
        Some(1) => ", 1 reply".to_string(),
        Some(many) => format!(", {many} replies"),
    };
    format!("{}  {:<9} {}  ({place}{replies})", text(topic, "id"), text(data, "status"), text(data, "title"))
}

/// The replies under `parent`, each further in than what it answers.
fn replies(all: &[Value], parent: Option<u64>, depth: usize, out: &mut Vec<String>) {
    for reply in all.iter().filter(|reply| reply["parent"].as_u64() == parent) {
        let words = text(reply, "body").replace('\n', " ");
        out.push(format!("{}{}  [{}] {words}", "  ".repeat(depth), reply["seq"], text(reply, "who")));
        replies(all, reply["seq"].as_u64(), depth + 1, out);
    }
}

pub async fn topic(client: &Client, args: &Args) -> Result<(), String> {
    let named = || args.word(2).ok_or_else(usage);
    let place = || json!({ "workspace": args.value("workspace"), "project": args.value("project") });
    match args.word(1) {
        None | Some("list") => {
            let found = client.get("/api/topics").await?;
            show(client, &found, || {
                let lines: Vec<String> = list(&found["topics"]).iter().map(line).collect();
                if lines.is_empty() { "there are no topics yet".to_string() } else { lines.join("\n") }
            });
        }
        Some("make") => {
            let body = args.value("body");
            let under = args.value("under");
            let mut asked = place();
            if let Some(under) = under {
                asked["under"] = json!(under);
            }
            let title = args.rest(2);
            if title.is_empty() {
                return Err(usage());
            }
            asked["title"] = json!(title);
            asked["body"] = json!(body.unwrap_or_default());
            asked["status"] = json!(args.value("status"));
            let made = client.post("/api/topics", asked).await?;
            show(client, &made, || format!("made {}", line(&made)));
        }
        Some("show") => {
            let read = client.get(&format!("/api/topics/{}", encode(named()?))).await?;
            show(client, &read, || {
                let topic = &read["topic"];
                let mut out = Vec::new();
                let path: Vec<&str> = list(&read["path"]).iter().map(|page| text(page, "title")).collect();
                if !path.is_empty() {
                    out.push(format!("in {}", path.join(" › ")));
                }
                out.push(line(topic));
                for (part, said) in
                    [("", text(&topic["data"], "body")), ("summary: ", text(&topic["data"], "summary"))]
                {
                    if !said.is_empty() {
                        out.push(format!("{part}{said}"));
                    }
                }
                let pages = list(&read["pages"]);
                if !pages.is_empty() {
                    out.push(format!("pages under it ({}):", pages.len()));
                    for page in pages {
                        out.push(format!(
                            "  {}  {:<9} {}",
                            text(page, "id"),
                            text(page, "status"),
                            text(page, "title")
                        ));
                    }
                }
                replies(list(&read["replies"]), None, 0, &mut out);
                out.join("\n")
            });
        }
        Some("reply") => {
            let words = args.rest(3);
            if words.is_empty() {
                return Err(usage());
            }
            let parent = match args.value("to") {
                Some(to) => Some(
                    to.parse::<u64>()
                        .map_err(|_| format!("`--to` is the number a reply was said at, not `{to}`"))?,
                ),
                None => None,
            };
            let said = client
                .post(
                    &format!("/api/topics/{}/replies", encode(named()?)),
                    json!({ "body": words, "parent": parent }),
                )
                .await?;
            show(client, &said, || format!("said at {}", said["seq"]));
        }
        Some("answer") => {
            let reply = if args.has("none") { Value::Null } else { json!(args.word(3).ok_or_else(usage)?) };
            let done = client
                .post(&format!("/api/topics/{}/answer", encode(named()?)), json!({ "reply": reply }))
                .await?;
            show(client, &done, || line(&done));
        }
        Some("move") => {
            let (under, top, before) = (args.value("under"), args.has("top"), args.value("before"));
            let asked = place();
            let path = encode(named()?);
            let moved = match (under, top) {
                (Some(_), true) => return Err("say `--under <page>` or `--top`, not both".to_string()),
                (None, false) if before.is_some() => {
                    return Err("`--before` goes with `--under <page>` or `--top`".to_string());
                }
                (None, false) => client.post(&format!("/api/topics/{path}/place"), asked).await?,
                (under, _) => {
                    let asked = json!({ "under": under, "before": before });
                    client.post(&format!("/api/topics/{path}/move"), asked).await?
                }
            };
            show(client, &moved, || {
                let under = moved["data"]["parent"].as_str().map_or("the top", |page| page);
                match (args.has("top") || args.value("under").is_some(), under) {
                    (true, "the top") => format!("a wiki of its own: {}", line(&moved)),
                    (true, page) => format!("under {page}: {}", line(&moved)),
                    (false, _) => line(&moved),
                }
            });
        }
        Some("follow") => {
            let follows = !args.has("not");
            let mut asked = crate::follows::how(args);
            asked["follows"] = json!(follows);
            let done = client.post(&format!("/api/topics/{}/follow", encode(named()?)), asked).await?;
            show(client, &done, || match (follows, args.has("with-pages")) {
                (true, true) => "you follow it, with every page under it".to_string(),
                (true, false) => "you follow it".to_string(),
                (false, _) => "you follow it no longer".to_string(),
            });
        }
        Some(_) => return Err(usage()),
    }
    Ok(())
}
