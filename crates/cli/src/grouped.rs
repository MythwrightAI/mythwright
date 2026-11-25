//! `group`: groups of people and agents, and what their members may ask
//! beside what each holds itself (`people::groups`).

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode};
use crate::work::{list, show, text};

/// The words of `value`'s list, joined.
fn words(value: &Value) -> String {
    let all: Vec<&str> = list(value).iter().filter_map(Value::as_str).collect();
    if all.is_empty() { "nothing".to_string() } else { all.join(", ") }
}

/// A group in a line: its name, what it holds where that was told, and who is in it.
fn line(group: &Value) -> String {
    let holds = match group.get("holds") {
        Some(holds) => format!(", holding {}", words(holds)),
        None => String::new(),
    };
    format!("{}{holds}: {}", text(group, "name"), words(&group["members"]))
}

pub async fn group(client: &Client, args: &Args) -> Result<(), String> {
    let (scopes, taking) = (args.values("scope"), args.has("not"));
    match args.word(1) {
        None | Some("list") => {
            let all = client.get("/api/groups").await?;
            show(client, &all, || match list(&all["groups"]) {
                [] => format!("there are no groups; `{} group make <name>` makes one", brand::CLI),
                groups => groups.iter().map(line).collect::<Vec<_>>().join("\n"),
            });
        }
        Some("make") => {
            let name = args.rest(2);
            if name.is_empty() {
                return Err("missing what the group is called".into());
            }
            let made = client.post("/api/groups", json!({ "name": name, "holds": scopes })).await?;
            show(client, &made, || format!("made {}", line(&made["group"])));
        }
        Some(verb @ ("add" | "remove")) => {
            let group = encode(args.need(2, "the group, by its name")?);
            let who = args.words_from(3);
            if who.is_empty() {
                return Err("missing who: a person's handle or email, or an agent's name".into());
            }
            let mut last = Value::Null;
            for one in who {
                last = match verb {
                    "add" => {
                        client.post(&format!("/api/groups/{group}/members"), json!({ "who": one })).await?
                    }
                    _ => {
                        let at = format!("/api/groups/{group}/members/{}", encode(&one));
                        client.call(reqwest::Method::DELETE, &at, None).await?
                    }
                };
            }
            show(client, &last, || {
                let narrowed = match last["keys"].as_u64().unwrap_or(0) {
                    0 => String::new(),
                    keys => format!(" ({keys} of their keys narrowed to what they hold now)"),
                };
                format!("{}{narrowed}", line(&last["group"]))
            });
        }
        Some("hold") => {
            let word = args.need(2, "the group, by its name")?;
            let said = args.words_from(3);
            if said.is_empty() {
                return Err("missing the scopes, such as `environments:secrets`".into());
            }
            let all = client.get("/api/groups").await?;
            let lower = word.to_lowercase();
            let found = list(&all["groups"])
                .iter()
                .find(|one| one["id"] == word || text(one, "name").to_lowercase() == lower);
            let Some(found) = found else {
                return Err(format!("there is no group `{word}`; `{} group list` lists them", brand::CLI));
            };
            let mut holds: Vec<String> =
                list(&found["holds"]).iter().filter_map(Value::as_str).map(str::to_string).collect();
            match taking {
                true => holds.retain(|one| !said.contains(one)),
                false => {
                    holds.extend(said.into_iter().filter(|one| !holds.contains(one)).collect::<Vec<_>>())
                }
            }
            let at = format!("/api/groups/{}", encode(text(found, "id")));
            let done = client.call(reqwest::Method::PATCH, &at, Some(json!({ "holds": holds }))).await?;
            show(client, &done, || line(&done["group"]));
        }
        Some("rename") => {
            let group = encode(args.need(2, "the group, by its name")?);
            let name = args.rest(3);
            if name.is_empty() {
                return Err("missing what it is to be called".into());
            }
            let at = format!("/api/groups/{group}");
            let done = client.call(reqwest::Method::PATCH, &at, Some(json!({ "name": name }))).await?;
            show(client, &done, || format!("called {} now", text(&done["group"], "name")));
        }
        Some("forget") => {
            let word = args.need(2, "the group, by its name")?;
            let at = format!("/api/groups/{}", encode(word));
            let done = client.call(reqwest::Method::DELETE, &at, None).await?;
            show(client, &done, || format!("`{word}` is gone, with everybody's place in it"));
        }
        Some(other) => {
            return Err(format!(
                "`group {other}` is not a command; `{} help group` says what is",
                brand::CLI
            ));
        }
    }
    Ok(())
}
