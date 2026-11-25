//! What the factory's bot does for the commands of its list beyond the
//! board and the agents it always took: records, a channel's own talk,
//! the inbox, what is running and applying it, environments' runs,
//! comparisons, sandbox tests and search. Each is read by its line
//! (`taken`), keyed by the words it is written with as they are, and done
//! as `bidden` does every command: by asking the daemon's own routes as
//! whoever said the message (already held to the bot's ceiling), with what
//! goes into an address held to what a name is made of (`part`), and held
//! to the channel's project where the channel has one.
//!
//! What would put something into a channel that its members may not
//! read is not said there: what somebody's inbox holds is counted and
//! not read out, a channel kept to its members is read only from inside
//! it, a table kept private is not listed, and search finds no messages.

use kernel::Principal;
use serde_json::{Map, Value, json};

use crate::bidden::{Routes, Verb, ask};
use crate::commanded::{
    LINES, brief, count, elsewhere, encode, held, latest, list, listed, named_project, part, plain, project,
    text, value_of,
};
use crate::taken::Said;

/// Where a command was said: the channel, and its project where it has one.
pub(crate) struct Here<'a> {
    pub(crate) channel: &'a str,
    pub(crate) within: Option<&'a str>,
}

/// Does the command `key` with what was `said`, as `who`.
pub(crate) async fn run(
    routes: &dyn Routes,
    who: &Principal,
    here: &Here<'_>,
    key: &str,
    said: &Said,
) -> Result<String, String> {
    let get = |path: String| ask(routes, who, Verb::Get, path, None);
    let within = here.within;
    let word = |name: &str, what: &str| part(said.word(name), what).map(str::to_string);
    Ok(match key {
        "instance" => {
            let id = word("id", "an instance")?;
            let found = get(format!("/api/instances/{id}")).await?;
            let one = &found["instance"];
            held(within, one, &format!("`{id}` is another project's"))?;
            let children = list(&found["children"]).len();
            format!(
                "`{id}` runs {} as {}: {}{}.",
                text(one, "blueprint"),
                text(one, "key"),
                text(one, "status"),
                match children {
                    0 => String::new(),
                    many => format!(", with {}", count(many, "instance under it", "instances under it")),
                }
            )
        }
        "stop" => {
            let id = word("id", "an instance")?;
            if within.is_some() {
                let found = get(format!("/api/instances/{id}")).await?;
                held(within, &found["instance"], &format!("`{id}` is another project's"))?;
            }
            ask(routes, who, Verb::Post, format!("/api/instances/{id}/stop"), Some(json!({}))).await?;
            format!("`{id}` is stopped, and what it holds with it.")
        }
        "apply" => {
            // A blueprint by its name: a file on the device is nothing a channel can name.
            let name = said.word("file|name");
            if brand::is_blueprint_path(name) || name.starts_with(['.', '/', '~']) || name.contains('\\') {
                return Err(format!(
                    "`{name}` is a file: here a blueprint is named, as `std/board` or one of the project's own."
                ));
            }
            let mut params = Map::new();
            for pair in said.every("param") {
                let (key, value) = pair
                    .split_once('=')
                    .ok_or_else(|| format!("`--param {pair}` should be `--param k=v`."))?;
                params.insert(key.to_string(), value_of(value));
            }
            let body = json!({
                "blueprint": name, "project": project(said, within)?, "environment": said.flag("environment"),
                "params": params, "wait": true,
            });
            let applied = ask(routes, who, Verb::Post, "/api/apply".into(), Some(body)).await?;
            let how = match (applied["created"] == true, applied["changed"] == true) {
                (true, _) => "made",
                (false, true) => "changed",
                (false, false) => "as it was",
            };
            let ended = applied["finished"]["status"].as_str().map(|status| format!(", its steps {status}"));
            format!(
                "`{name}` is applied: `{}`, {how}{}.",
                text(&applied, "instance"),
                ended.unwrap_or_default()
            )
        }
        "board columns" => {
            let project = named_project(said, within)?;
            let asked: Vec<&str> = said.word("column").split(' ').filter(|one| !one.is_empty()).collect();
            let columns = match asked.is_empty() {
                true => get(format!("/api/board/{project}")).await?["columns"].clone(),
                false => {
                    let body = json!({ "columns": asked });
                    ask(routes, who, Verb::Post, format!("/api/board/{project}/columns"), Some(body)).await?["columns"]
                        .clone()
                }
            };
            let named: Vec<&str> = list(&columns).iter().filter_map(Value::as_str).collect();
            format!(
                "The board of `{project}` has {}: {}.",
                count(named.len(), "column", "columns"),
                named.join(", ")
            )
        }
        "record list" => {
            let project = named_project(said, within)?;
            let collection = word("collection", "a table")?;
            shown_here(routes, who, &project, &collection).await?;
            let found = get(format!("/api/collections/{project}/{collection}/records")).await?;
            let lines = list(&found["records"])
                .iter()
                .map(|record| format!("- `{}` {}", text(record, "id"), brief(&record["data"])));
            listed(
                lines.collect(),
                &format!("`{collection}` has no records yet."),
                &format!("record list {project} {collection}"),
            )
        }
        "record show" => {
            let id = word("record", "a record")?;
            let found = get(format!("/api/records/{id}")).await?;
            held(within, &found, &format!("`{id}` is another project's"))?;
            shown_here(routes, who, text(&found, "project"), text(&found, "collection")).await?;
            let fields = found["data"].as_object().into_iter().flatten();
            let lines: Vec<String> =
                fields.map(|(name, value)| format!("- {name}: {}", plain(value))).collect();
            format!("`{id}`, of {}:\n{}", text(&found, "collection"), lines.join("\n"))
        }
        "record add" => {
            let (project, collection) = (named_project(said, within)?, word("collection", "a table")?);
            let fields: Map<String, Value> =
                said.pairs.iter().map(|(name, value)| (name.clone(), value_of(value))).collect();
            let path = format!("/api/collections/{project}/{collection}/records");
            let added = ask(routes, who, Verb::Post, path, Some(json!({ "fields": fields }))).await?;
            format!("`{}` is added to {collection}.", text(&added, "id"))
        }
        "record set" | "record comment" | "record remove" => {
            let id = word("record", "a record")?;
            if within.is_some() {
                let found = get(format!("/api/records/{id}")).await?;
                held(within, &found, &format!("`{id}` is another project's"))?;
            }
            let path = format!("/api/records/{id}");
            match key {
                "record set" => {
                    let fields: Map<String, Value> =
                        said.pairs.iter().map(|(name, value)| (name.clone(), value_of(value))).collect();
                    ask(routes, who, Verb::Patch, path, Some(json!({ "fields": fields }))).await?;
                    format!("`{id}` is changed.")
                }
                "record comment" => {
                    let body = json!({ "body": said.word("words") });
                    ask(routes, who, Verb::Post, format!("{path}/comments"), Some(body)).await?;
                    format!("Said on `{id}`.")
                }
                _ => {
                    ask(routes, who, Verb::Delete, path, None).await?;
                    format!("`{id}` is moved to the Trash.")
                }
            }
        }
        "agent move" => {
            let name = word("name", "an agent")?;
            if let Some(project) = within {
                return Err(elsewhere(project, "moving an agent is between projects"));
            }
            let to = match said.has("none") {
                true => Value::Null,
                false => json!(part(said.word("project"), "a project")?),
            };
            let moved =
                ask(routes, who, Verb::Patch, format!("/api/agents/{name}"), Some(json!({ "project": to })))
                    .await?;
            match moved["project"].as_str() {
                Some(project) => format!("`{name}` works in `{project}` from its next piece of work."),
                None => format!("`{name}` works in no project from its next piece of work."),
            }
        }
        "read" | "post" | "channel rename" => {
            let channel = word("channel", "a channel")?;
            let found = get(format!("/api/channels/{channel}")).await?;
            held(within, &found, &format!("#{channel} is another project's"))?;
            let id = text(&found, "id").to_string();
            match key {
                "read" => {
                    // What is said in a channel kept to its members is theirs: read here, it would be told to this one's.
                    if found["kept"] == true && id != here.channel {
                        return Err(format!(
                            "#{channel} is kept to its members, and what is said there is read there and not here."
                        ));
                    }
                    let most = said
                        .flag("limit")
                        .and_then(|limit| limit.parse::<usize>().ok())
                        .unwrap_or(10)
                        .min(LINES);
                    let read = get(format!("/api/channels/{id}/messages?limit={most}")).await?;
                    let lines = list(&read["messages"]).iter().map(|message| {
                        let first = text(message, "body").lines().next().unwrap_or_default().to_string();
                        format!(
                            "- {}: {}",
                            text(message, "author_name"),
                            if first.is_empty() { "something attached".into() } else { first }
                        )
                    });
                    latest(lines.collect(), "Nothing has been said there yet.", &format!("read {channel}"))
                }
                "post" => {
                    let body = json!({ "body": said.word("message") });
                    ask(routes, who, Verb::Post, format!("/api/channels/{id}/messages"), Some(body)).await?;
                    format!("Said in #{}.", text(&found, "name"))
                }
                _ => {
                    let name = said.word("name");
                    let now = ask(
                        routes,
                        who,
                        Verb::Patch,
                        format!("/api/channels/{id}"),
                        Some(json!({ "name": name })),
                    )
                    .await?;
                    format!(
                        "#{channel} is called {} now.",
                        now["title"].as_str().unwrap_or(text(&now, "name"))
                    )
                }
            }
        }
        "inbox" => {
            let all = if said.has("all") { "?all=true" } else { "" };
            let found = get(format!("/api/inbox{all}")).await?;
            let lines = list(&found["lines"]).len();
            format!(
                "You have {} in your inbox, {} unread. What they say is yours, so it is read in your inbox and not said here.",
                count(lines, "line", "lines"),
                found["unread"].as_u64().unwrap_or(0)
            )
        }
        "inbox read" => match said.has("all") {
            true => {
                ask(routes, who, Verb::Post, "/api/inbox/read".into(), Some(json!({}))).await?;
                "Everything in your inbox is read.".to_string()
            }
            false => {
                let line = word("line", "a line of your inbox")?;
                ask(routes, who, Verb::Post, format!("/api/inbox/{line}/read"), Some(json!({}))).await?;
                format!("`{line}` is read.")
            }
        },
        "inbox away" => {
            let line = word("line", "a line of your inbox")?;
            ask(routes, who, Verb::Post, format!("/api/inbox/{line}/away"), Some(json!({}))).await?;
            format!("`{line}` is put away.")
        }
        "project environment" => {
            let project = named_project(said, within)?;
            let found = get(format!("/api/projects/{project}/environments")).await?;
            let names: Vec<&str> = list(&found["environments"]).iter().map(|one| text(one, "name")).collect();
            match names.is_empty() {
                true => format!("`{project}` has no environments."),
                false => format!(
                    "`{project}` has {}: {}.",
                    count(names.len(), "environment", "environments"),
                    names.join(", ")
                ),
            }
        }
        "project environment run" => {
            let (project, environment) = (named_project(said, within)?, word("env", "an environment")?);
            let path = format!("/api/projects/{project}/environments/{environment}/run");
            let ran =
                ask(routes, who, Verb::Post, path, Some(json!({ "branch": said.word("branch") }))).await?;
            let last = &ran["environment"]["ran"];
            format!(
                "`{environment}` is being loaded from `{}` by `{}`: its run is `{}`.",
                text(last, "branch"),
                text(last, "blueprint"),
                text(last, "run")
            )
        }
        "compare" => {
            let of = within.map(|project| format!("?project={project}")).unwrap_or_default();
            let found = get(format!("/api/comparisons{of}")).await?;
            let lines = list(&found["comparisons"]).iter().map(|one| {
                let state = if one["ended"].is_null() { "going" } else { "ended" };
                format!("- `{}` of {}: {state}", text(one, "name"), text(one, "of"))
            });
            listed(lines.collect(), "Nothing is being compared.", "compare list")
        }
        "compare show" | "compare end" => {
            let name = word("name", "a comparison")?;
            let found = get(format!("/api/comparisons/{name}")).await?;
            let one = &found["comparison"];
            held(within, one, &format!("`{name}` is another project's"))?;
            match key {
                "compare show" => {
                    let ways: Vec<&str> = list(&one["ways"]).iter().map(|way| text(way, "name")).collect();
                    let state = if one["ended"].is_null() { "going" } else { "ended" };
                    format!(
                        "`{name}` compares {} ways of {} ({}): {state}. `{} compare show {name}` lays out what each came to.",
                        ways.len(),
                        text(one, "of"),
                        ways.join(", "),
                        brand::CLI
                    )
                }
                _ => {
                    let body = json!({ "goes": said.flag("goes") });
                    ask(routes, who, Verb::Post, format!("/api/comparisons/{name}/end"), Some(body)).await?;
                    match said.flag("goes") {
                        Some(way) => format!("`{name}` is ended; everything goes `{way}`."),
                        None => format!("`{name}` is ended; everything goes as it is written."),
                    }
                }
            }
        }
        "sandbox" => {
            let name = said.word("file|name");
            if brand::is_blueprint_path(name) || name.starts_with(['.', '/', '~']) || name.contains('\\') {
                return Err(format!("`{name}` is a file: here a blueprint is named, as `std/board`."));
            }
            let mut params = Map::new();
            for pair in said.every("param") {
                let (key, value) = pair
                    .split_once('=')
                    .ok_or_else(|| format!("`--param {pair}` should be `--param k=v`."))?;
                params.insert(key.to_string(), value_of(value));
            }
            let body = json!({ "name": name, "project": project(said, within)?, "params": params, "answers": "default" });
            let made = ask(routes, who, Verb::Post, "/api/sandboxes".into(), Some(body)).await?;
            if made["ok"] != true {
                return Err(format!(
                    "`{name}` does not check, so it was not run: `{} check` says why.",
                    brand::CLI
                ));
            }
            let path = format!("/api/sandboxes/{}", text(&made, "sandbox"));
            // Read and ended whatever became of the reading: a sandbox is nobody's to keep.
            let shown = get(path.clone()).await;
            let _ = ask(routes, who, Verb::Delete, path, None).await;
            let shown = shown?;
            let tables = list(&shown["tables"]).iter().map(|table| {
                format!(
                    "`{}` {}",
                    text(table, "name"),
                    count(table["records"].as_array().map_or(0, Vec::len), "row", "rows")
                )
            });
            let tables: Vec<String> = tables.collect();
            let made =
                if tables.is_empty() { String::new() } else { format!(" It made {}.", tables.join(", ")) };
            format!(
                "A sandbox test of `{name}`, where nothing is real.{made} {} {}",
                text(&shown, "stood_said"),
                text(&shown, "said")
            )
        }
        "search" => {
            // Messages are found where they are read: a channel they were said in may be kept from this one's members.
            if said.flag("kind") == Some("message") {
                return Err("Messages are searched where you read them, and not said in a channel.".into());
            }
            let words: Vec<String> = said.word("words").split(' ').map(|one| one.to_string()).collect();
            let mut path = format!("/api/search?q={}", encode(&words.join(" ")));
            if let Some(kind) = said.flag("kind") {
                path.push_str(&format!("&kind={}", part(kind, "a kind")?));
            }
            if let Some(project) = project(said, within)? {
                path.push_str(&format!("&project={project}"));
            }
            let found = get(path).await?;
            let hits = list(&found["hits"]).iter().filter(|hit| hit["kind"] != "message");
            let lines = hits.map(|hit| format!("- {} {}", text(hit, "kind"), text(hit, "title")));
            listed(lines.collect(), "Nothing was found.", &format!("search {}", words.join(" ")))
        }
        other => return Err(format!("`{other}` is written down and not done yet.")),
    })
}

/// Refuses to list or show the records of a table kept private: whoever is
/// in the channel may not read them, and what the bot says there is read by
/// everybody in it.
async fn shown_here(
    routes: &dyn Routes,
    who: &Principal,
    project: &str,
    collection: &str,
) -> Result<(), String> {
    let tables = ask(routes, who, Verb::Get, format!("/api/collections?project={project}"), None).await?;
    let kept = list(&tables["collections"])
        .iter()
        .any(|table| text(table, "name") == collection && !table["reads"].is_null());
    match kept {
        true => Err(format!(
            "`{collection}` is kept private: its records are read where you read them, and not said in a channel."
        )),
        false => Ok(()),
    }
}
