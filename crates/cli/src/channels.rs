//! Commands about channels: listing and making them, the door of one
//! kept to its members, what one is called, says it is for and belongs to,
//! who manages it, the documents
//! it keeps to hand, taking one away, what the
//! factory's blueprints speak as there, and reading and saying things
//! in one.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode, table};
use crate::work::{show, text};

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

pub async fn channel(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        None | Some("list") => {
            let channels = client.get("/api/channels").await?;
            show(client, &channels, || {
                // A conversation is listed in the inbox of whoever is in it.
                let rows: Vec<Vec<String>> = list(&channels["channels"])
                    .iter()
                    .filter(|channel| channel["conversation"] != true)
                    .map(|channel| {
                        let members: Vec<&str> =
                            list(&channel["members"]).iter().map(|member| text(member, "name")).collect();
                        vec![
                            called(channel) + archived(channel),
                            members.join(", "),
                            text(channel, "topic").into(),
                        ]
                    })
                    .collect();
                if rows.is_empty() { "no channels yet".to_string() } else { table(&rows) }
            });
        }
        Some("create") => {
            let name = args.need(2, "the channel's name")?;
            let mut asked = json!({
                "name": name, "project": args.value("project"), "workspace": args.value("workspace"),
                "topic": args.value("topic"), "kept": args.has("kept"), "alone": args.has("alone"),
            });
            let mut members = Vec::new();
            for who in args.values("member") {
                members.push(actor(client, who).await?);
            }
            asked["members"] = json!(members);
            let created = client.post("/api/channels", asked).await?;
            show(client, &created, || format!("created {}", called(&created)));
        }
        Some("keep" | "open" | "add" | "leave" | "alone" | "from") => return door(client, args).await,
        Some(doing @ ("archive" | "unarchive")) => {
            let channel = which(client, args.need(2, "the channel")?).await?;
            let now = client.post(&format!("/api/channels/{}/{doing}", encode(&channel)), json!({})).await?;
            show(client, &now, || match doing {
                "archive" => format!(
                    "{} is archived: whoever read it reads it, and nothing is said there until it is given back \
                     (`{} channel unarchive {}`)",
                    called(&now),
                    brand::CLI,
                    text(&now, "name")
                ),
                _ => format!("{} is back as it was", called(&now)),
            });
        }
        Some("rename") => {
            let channel = which(client, args.need(2, "the channel")?).await?;
            let to = args.need(3, "what it is to be called")?;
            let path = format!("/api/channels/{}", encode(&channel));
            let now = client.call(reqwest::Method::PATCH, &path, Some(json!({ "name": to }))).await?;
            show(client, &now, || format!("it is called {} now", called(&now)));
        }
        Some("describe") => {
            let channel = which(client, args.need(2, "the channel")?).await?;
            let topic = args.rest(3);
            let path = format!("/api/channels/{}", encode(&channel));
            let now = client.call(reqwest::Method::PATCH, &path, Some(json!({ "topic": topic }))).await?;
            show(client, &now, || match text(&now, "topic") {
                "" => format!("{} says nothing of what it is for now", called(&now)),
                said => format!("{} says it is for: {said}", called(&now)),
            });
        }
        Some("manager") => {
            let not = args.has("not");
            let channel = which(client, args.need(2, "the channel")?).await?;
            let member = actor(client, args.need(3, "who is to manage it: a person in it")?).await?;
            let path = format!("/api/channels/{}/managers", encode(&channel));
            let now = client.post(&path, json!({ "member": member, "manages": !not })).await?;
            show(client, &now, || {
                let managing: Vec<&str> = list(&now["members"])
                    .iter()
                    .filter(|one| one["manages"] == true)
                    .map(|one| text(one, "name"))
                    .collect();
                format!("{} is managed by {}", called(&now), managing.join(", "))
            });
        }
        Some("docs") => return docs(client, args).await,
        Some("end") => {
            let channel = which(client, args.need(2, "the channel")?).await?;
            let root = args.need(3, "the message the discussion's thread hangs from")?;
            let path = format!("/api/channels/{}/discussions/{}/end", encode(&channel), encode(root));
            let ended = client.post(&path, json!({})).await?;
            show(client, &ended, || {
                format!(
                    "the discussion under {root} is ended, after {} turns",
                    ended["turns"].as_i64().unwrap_or(0)
                )
            });
        }
        Some("belongs") => return crate::workspaces::belongs(client, args).await,
        Some("remove") => {
            let name = args.need(2, "the channel to remove")?.trim_start_matches('#');
            let force = if args.has("force") { "?force=true" } else { "" };
            let gone = client
                .call(reqwest::Method::DELETE, &format!("/api/channels/{}{force}", encode(name)), None)
                .await?;
            show(client, &gone, || {
                let called = match gone["title"].as_str() {
                    Some(title) => format!("`{title}`"),
                    None => format!("#{}", text(&gone, "name")),
                };
                match (gone["trash"].as_str(), gone["kept"].as_u64()) {
                    (Some(trash), days) => format!(
                        "moved {called} to the Trash as `{trash}`, with what was said in it and attached to it; {}, \
                         and `{} trash restore {trash}` gives it back",
                        days.map_or("it is kept until it is deleted for good".to_string(), |days| format!(
                            "it is kept {days} day{}",
                            if days == 1 { "" } else { "s" }
                        )),
                        brand::CLI
                    ),
                    (None, _) => format!("removed {called}, with what was said in it and attached to it"),
                }
            });
        }
        Some("speaker") => match args.word(2) {
            None => {
                let found = client.get("/api/system/speaker").await?;
                show(client, &found, || {
                    format!(
                        "what this factory's blueprints say in a channel is said by `{}`
`{} channel speaker <name>` calls it something else from then on",
                        text(&found, "name"),
                        brand::CLI
                    )
                });
            }
            Some(name) => {
                let called = client.post("/api/system/speaker", json!({ "name": name })).await?;
                show(client, &called, || {
                    format!(
                        "what this factory's blueprints say is said by `{}` from now on",
                        text(&called, "name")
                    )
                });
            }
        },
        Some(other) => {
            return Err(format!(
                "`channel {other}` is not a command; try list, create, keep, open, add, leave, alone, from, \
                 archive, unarchive, rename, describe, manager, belongs, remove or speaker"
            ));
        }
    }
    Ok(())
}

/// What a channel is called, for a line: an open one its name, and one
/// kept to its members its title, with the id it is named by.
fn called(channel: &Value) -> String {
    match channel["kept"] == true {
        true => {
            let alone = if channel["alone"] == true { ", its agents asked nowhere else" } else { "" };
            format!("`{}` (kept to its members{alone}: {})", text(channel, "title"), text(channel, "id"))
        }
        false => format!("#{}", text(channel, "name")),
    }
}

/// What is said of a channel after it is called, where it is archived.
fn archived(channel: &Value) -> &'static str {
    if channel["archived"].is_object() { " (archived)" } else { "" }
}

/// The channel `word` names: its id or its name, as it is written, or
/// the id of the one channel whoever asks reads that is called so, which
/// is how one kept to its members is named without its id.
async fn which(client: &Client, word: &str) -> Result<String, String> {
    let word = word.trim_start_matches('#');
    let read = client.get("/api/channels").await?;
    let channels = list(&read["channels"]);
    if channels.iter().any(|channel| channel["id"] == word || channel["name"] == word) {
        return Ok(word.to_string());
    }
    let so: Vec<&str> = channels
        .iter()
        .filter(|channel| text(channel, "title").eq_ignore_ascii_case(word))
        .map(|channel| text(channel, "id"))
        .collect();
    match so.as_slice() {
        [] => Ok(word.to_string()),
        [one] => Ok(one.to_string()),
        several => Err(format!(
            "{} channels you read are called `{word}`: say which by its id ({})",
            several.len(),
            several.join(", ")
        )),
    }
}

/// Who `word` names, as an actor: one as it is written (`user:<id>`,
/// `agent:<name>`), the person with an account here who is called so,
/// or else the agent of that name.
async fn actor(client: &Client, word: &str) -> Result<String, String> {
    if word.contains(':') {
        return Ok(word.to_string());
    }
    let people = client.get("/api/people").await?;
    let so: Vec<&str> = list(&people["people"])
        .iter()
        .filter(|person| text(person, "name").eq_ignore_ascii_case(word))
        .map(|person| text(person, "actor"))
        .collect();
    match so.as_slice() {
        [] => Ok(format!("agent:{word}")),
        [one] => Ok(one.to_string()),
        several => Err(format!(
            "{} people here are called `{word}`: say which as an actor ({})",
            several.len(),
            several.join(", ")
        )),
    }
}

/// The door of a channel kept to its members: `channel keep`, `open`,
/// `add`, `leave` and `alone`; and `channel from`, which opens a
/// conversation as a channel.
async fn door(client: &Client, args: &Args) -> Result<(), String> {
    let what = args.word(1).unwrap_or_default();
    // What each door is told beside the channel, read before the channel
    // is asked for (`crate::args`).
    let asked = match what {
        "open" => json!({ "name": args.value("name") }),
        "from" => json!({
            "name": args.value("name"), "workspace": args.value("workspace"), "project": args.value("project"),
        }),
        "leave" => json!({ "agent": args.value("agent") }),
        _ => json!({}),
    };
    let channel = which(client, args.need(2, "the channel")?).await?;
    let at = |door: &str| format!("/api/channels/{}/{door}", encode(&channel));
    match what {
        "keep" => {
            let kept = client.post(&at("keep"), json!({})).await?;
            show(client, &kept, || {
                format!(
                    "{} is read by whoever is in it and nobody else from here: it is named by its id",
                    called(&kept)
                )
            });
        }
        "open" => {
            let opened = client.post(&at("open"), asked).await?;
            show(client, &opened, || {
                format!("{} is open again: anybody who may read channels reads it", called(&opened))
            });
        }
        "add" => {
            let who = actor(client, args.need(3, "who to add: a person's name, or an agent's")?).await?;
            let with = client.post(&at("members"), json!({ "actor": who })).await?;
            show(client, &with, || format!("{who} is in {}", called(&with)));
        }
        "from" => {
            let path = format!("/api/inbox/conversations/{}/channel", encode(&channel));
            let opened = client.post(&path, asked).await?;
            show(client, &opened, || {
                format!(
                    "{} is a channel from here, with everything said in it so far: listed among the channels of \
                     whoever is in it",
                    called(&opened["channel"])
                )
            });
        }
        "leave" => {
            let left = client.post(&at("leave"), asked).await?;
            show(client, &left, || match args.value("agent") {
                Some(agent) => format!("{agent} is out of the channel, with what it kept of it"),
                None => "you have left the channel".to_string(),
            });
        }
        _ => {
            let alone = match args.word(3) {
                Some("on") => true,
                Some("off") => false,
                _ => return Err("say `on` or `off`: whether its agents are asked nowhere else".into()),
            };
            let now =
                client.call(reqwest::Method::PUT, &at("alone"), Some(json!({ "alone": alone }))).await?;
            show(client, &now, || match alone {
                true => format!("the agents of {} are asked nowhere else from here", called(&now)),
                false => format!("the agents of {} are asked anywhere again", called(&now)),
            });
        }
    }
    Ok(())
}

pub async fn read(client: &Client, args: &Args) -> Result<(), String> {
    let channel = args.need(1, "the channel")?.trim_start_matches('#');
    let limit = args.value("limit").unwrap_or("30");
    let which = match args.value("thread") {
        Some(message) => format!("thread={}", encode(message)),
        None => format!("limit={limit}"),
    };
    let found = client.get(&format!("/api/channels/{}/messages?{which}", encode(channel))).await?;
    show(client, &found, || {
        let messages = list(&found["messages"]);
        let lines = messages
            .iter()
            .map(|message| {
                // A message with a thread under it says so, and how to read it.
                let under = match message["replies"].as_u64().unwrap_or(0) {
                    0 => String::new(),
                    1 => format!("  (1 reply: --thread {})", text(message, "id")),
                    n => format!("  ({n} replies: --thread {})", text(message, "id")),
                };
                // What is attached is named with its id, by which its file is the daemon's to give.
                let names: Vec<String> = list(&message["attachments"])
                    .iter()
                    .filter_map(|one| Some(format!("{} ({})", one["name"].as_str()?, one["id"].as_str()?)))
                    .collect();
                let attached = if names.is_empty() {
                    String::new()
                } else {
                    format!(" [attached: {}]", names.join(", "))
                };
                format!("[{}] {}{attached}{under}", text(message, "author_name"), text(message, "body"))
            })
            .collect::<Vec<_>>();
        // Said once, where anything is attached: how a file is read by its id.
        let attached = messages.iter().any(|message| !list(&message["attachments"]).is_empty());
        let how = attached.then(|| {
            format!("(what is attached is read by its id: {} attachment <id> [--out FILE])", brand::CLI)
        });
        lines.into_iter().chain(how).collect::<Vec<_>>().join("\n")
    });
    Ok(())
}

pub async fn post(client: &Client, args: &Args) -> Result<(), String> {
    let channel = args.need(1, "the channel")?.trim_start_matches('#');
    // What it is marked as: a plan, or something for agents to discuss.
    let mark = args.value("mark");
    let body = args.rest(2);
    let files = args.values("attach");
    if body.is_empty() && files.is_empty() {
        return Err("missing the message".into());
    }
    // In the channel itself: what an agent says while it is answering in
    // a thread is otherwise said in that thread.
    let top = args.has("top");
    let (reply_to, thread) = (args.value("reply-to"), args.value("thread"));
    if top && thread.is_some() {
        return Err(
            "`--top` says it in the channel itself and `--thread` under a message: say one of them".into()
        );
    }
    // A file of a project is posted as that file, not a copy, unless a
    // copy is asked for: the daemon is told where the file is, whole,
    // and says whether it is one (`api::posted`).
    let copy = args.has("copy");
    // What goes with it is sent first, each file as it is, and then named in the message.
    let (mut attachments, mut named) = (Vec::new(), Vec::new());
    for file in files {
        let bytes = std::fs::read(file).map_err(|err| format!("cannot read {file} to attach it: {err}"))?;
        let name = std::path::Path::new(file).file_name().map(|name| name.to_string_lossy().into_owned());
        let mut path = format!(
            "/api/channels/{}/attachments?name={}",
            encode(channel),
            encode(&name.unwrap_or_else(|| file.to_string()))
        );
        let whole = (!copy).then(|| kernel::whole::whole(std::path::Path::new(file)).ok()).flatten();
        if let Some(whole) = whole.as_ref().and_then(|whole| whole.to_str()) {
            path.push_str(&format!("&from={}", encode(whole)));
        }
        let kept = client.upload(&path, bytes).await?;
        if let (Some(project), Some(path)) = (kept["file"]["project"].as_str(), kept["file"]["path"].as_str())
        {
            named.push(format!("`{path}` of {project}"));
        }
        attachments.push(kept["id"].clone());
    }
    let posted = client
        .post(
            &format!("/api/channels/{}/messages", encode(channel)),
            json!({
                "body": body, "reply_to": reply_to, "thread": thread,
                "attachments": attachments, "top": top, "mark": mark,
            }),
        )
        .await?;
    // Where it went: under a message, where it was said to be or where
    // whoever posts is being asked.
    let files = match named.is_empty() {
        true => String::new(),
        false => format!(", with the project's file {} as it is and no copy", named.join(" and ")),
    };
    show(client, &posted, || match posted["thread"].as_str() {
        Some(under) => format!("posted to #{channel}, in the thread under {under}{files}"),
        None => format!("posted to #{channel}{files}"),
    });
    Ok(())
}

/// The documents a channel keeps to hand, the same for everybody in it:
/// listed, one added (a library's record by its id, or a project's file
/// as `<project>/<path>`) or one taken away.
async fn docs(client: &Client, args: &Args) -> Result<(), String> {
    let channel = which(client, args.need(2, "the channel")?).await?;
    let path = format!("/api/channels/{}/docs", encode(&channel));
    let now = match args.word(3) {
        None | Some("list") => client.get(&path).await?,
        Some("add") => {
            let doc =
                args.need(4, "the document: a library's record, or a project's file as <project>/<path>")?;
            client.post(&path, json!({ "doc": doc })).await?
        }
        Some("remove") => {
            let doc = args.need(4, "the document, as it is listed")?;
            client.call(reqwest::Method::DELETE, &format!("{path}?doc={}", encode(doc)), None).await?
        }
        Some(other) => {
            return Err(format!(
                "`{other}` is not something done to a channel's documents: it is `add` or `remove`, or nothing to \
                 list them"
            ));
        }
    };
    show(client, &now, || {
        let rows: Vec<Vec<String>> = list(&now["docs"])
            .iter()
            .map(|doc| {
                let said = match text(doc, "state") {
                    "there" => text(doc, "title").to_string(),
                    "kept" => "kept from you".to_string(),
                    _ => "gone".to_string(),
                };
                vec![text(doc, "doc").to_string(), said]
            })
            .collect();
        if rows.is_empty() { "the channel keeps no documents to hand".to_string() } else { table(&rows) }
    });
    Ok(())
}
