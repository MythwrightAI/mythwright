//! Commands about the board, agents, search, events, measurements and
//! administration.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, ago, encode, now_ms, table, took};
use crate::work::{show, text};

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

pub async fn board(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        Some("add") => {
            let project = args.need(2, "the project")?;
            let title = args.rest(3);
            if title.is_empty() {
                return Err("missing the card's title".into());
            }
            let mut body = json!({ "title": title, "body": args.value("body"), "status": args.value("status"), "labels": args.values("label") });
            // What else its board keeps of a card, as `record set` takes it.
            body["fields"] = json!(match args.values("field") {
                given if given.is_empty() => serde_json::Map::new(),
                given => crate::records::held_by(client, project, "board", &given, None).await?,
            });
            let card = client.post(&format!("/api/board/{project}/items"), body).await?;
            show(client, &card, || {
                format!("added {} to {}: {}", text(&card, "id"), text(&card, "status"), text(&card, "title"))
            });
        }
        Some("cost") => return crate::spent::cost(client, args).await,
        // A board's columns, or them set in the order given: the same the
        // client's layout of a board's columns asks for.
        Some("columns") => {
            let project = args.need(2, "the project")?;
            let mut columns = Vec::new();
            while let Some(column) = args.word(3 + columns.len()) {
                columns.push(column);
            }
            let found = match columns.is_empty() {
                true => client.get(&format!("/api/board/{}", encode(project))).await?,
                false => {
                    let path = format!("/api/board/{}/columns", encode(project));
                    client.post(&path, json!({ "columns": columns })).await?
                }
            };
            show(client, &found, || {
                let named: Vec<&str> = list(&found["columns"]).iter().filter_map(Value::as_str).collect();
                named.join(" ")
            });
        }
        Some("history") => {
            let id = args.need(2, "the card")?;
            let kept = client.get(&format!("/api/records/{id}/history")).await?;
            show(client, &kept, || {
                let lines: Vec<String> = list(&kept["history"])
                    .iter()
                    .map(|line| {
                        // What a blueprint's step did is the factory's bot's doing,
                        // and the blueprint is said after it, as detail.
                        let via = kernel::worded::through(&line["data"])
                            .map(|blueprint| format!(" · via {blueprint}"));
                        format!(
                            "{}  {}{}",
                            kernel::clock::rfc3339(line["ts"].as_i64().unwrap_or(0)),
                            text(line, "said"),
                            via.unwrap_or_default()
                        )
                    })
                    .collect();
                if lines.is_empty() { format!("nothing has been kept of {id}") } else { lines.join("\n") }
            });
        }
        // Said as on any record, and not in a second way of the board's own.
        Some("comment") => {
            return Err(format!(
                "what is said on a card is said as on any record: `{} record comment <card> <words...>`",
                brand::CLI
            ));
        }
        Some("give") => {
            let (id, who) = (
                args.need(2, "the card")?,
                args.need(3, "who it is given to: an agent's name, a person's handle or email, or `nobody`")?,
            );
            // Whom a word names is the daemon's to say, as it says it for the bot and the route.
            let assignee = if who == "nobody" { "" } else { who };
            let change = json!({ "assignee": assignee });
            let card =
                client.call(reqwest::Method::PATCH, &format!("/api/board/items/{id}"), Some(change)).await?;
            show(client, &card, || match card["assignee"].as_str() {
                Some(who) => format!("{} is given to {who}", text(&card, "id")),
                None => format!("{} is given to nobody", text(&card, "id")),
            });
        }
        // What a card holds beyond where it is and whose it is: what it is
        // called and says, and whatever else its board keeps of a card.
        Some("set") => {
            let id = args.need(2, "the card")?;
            let mut pairs = Vec::new();
            while let Some(pair) = args.word(3 + pairs.len()) {
                pairs.push(pair);
            }
            if pairs.is_empty() {
                return Err(
                    "missing what it holds: one or more of `field=value`, as in `after=shop-1,shop-2`".into(),
                );
            }
            let cli = brand::CLI;
            let mut change = serde_json::Map::new();
            let mut more = Vec::new();
            // The card as it is, where something is added to what it holds or
            // what its board keeps beyond a card's own is set.
            let own = ["title", "body", "due", "status", "assignee", "labels"];
            let reads = |pair: &&str| pair.split_once('=').is_some_and(|(name, _)| !own.contains(&name));
            let was = match pairs.iter().any(reads) {
                true => Some(client.get(&format!("/api/records/{}", crate::client::encode(id))).await?),
                false => None,
            };
            for pair in pairs {
                let (name, value) = pair
                    .split_once('=')
                    .ok_or_else(|| format!("`{pair}` should be `field=value`, as in `title=A cart`"))?;
                match name {
                    // Each is an event of its own, which the rest of the
                    // factory hangs on, and has its own command.
                    "status" => {
                        return Err(format!(
                            "where a card is is not set as a field: move it with `{cli} board move {id} <column>`"
                        ));
                    }
                    "assignee" => {
                        return Err(format!(
                            "whose a card is is not set as a field: give it with `{cli} board give {id} <agent>|<person>|nobody`"
                        ));
                    }
                    "title" if value.trim().is_empty() => {
                        return Err("a card is called something: `title=` needs words after it".into());
                    }
                    "title" | "body" | "due" => {
                        change.insert(name.to_string(), json!(value));
                    }
                    "labels" => {
                        change.insert(name.to_string(), Value::Array(crate::records::between_commas(value)));
                    }
                    // One label more, whole, after those it has.
                    "labels+" => {
                        let data = was.as_ref().map(|was| &was["data"]);
                        let labels = crate::records::added(&change, data, "labels", value);
                        change.insert("labels".to_string(), labels);
                    }
                    _ => more.push(pair),
                }
            }
            // What its board keeps of a card beyond a card's own, each
            // made what its field keeps, as `record set` makes them.
            if let (false, Some(was)) = (more.is_empty(), &was) {
                let (project, collection) = (text(was, "project"), text(was, "collection"));
                let fields =
                    crate::records::held_by(client, project, collection, &more, Some(&was["data"])).await?;
                change.insert("fields".to_string(), Value::Object(fields));
            }
            let path = format!("/api/board/items/{id}");
            let card = client.call(reqwest::Method::PATCH, &path, Some(Value::Object(change))).await?;
            show(client, &card, || format!("changed {}", text(&card, "id")));
        }
        Some("move") => {
            let (id, column) = (args.need(2, "the card")?, args.need(3, "the column")?);
            // What is said as it is moved: why a card in review is sent back, say.
            let change = json!({ "status": column, "note": args.value("note") });
            let card =
                client.call(reqwest::Method::PATCH, &format!("/api/board/items/{id}"), Some(change)).await?;
            show(client, &card, || format!("{} is now in {}", text(&card, "id"), text(&card, "status")));
        }
        // Something its column offers pressed on a card, as its button is
        // pressed, or taken back: by whoever this terminal is signed in as.
        Some("press") => {
            let (id, action) =
                (args.need(2, "the card")?, args.need(3, "what is pressed: its name, as its row says")?);
            let back = args.has("back");
            let path = format!("/api/board/items/{}/press", encode(id));
            let pressing = client.post(&path, json!({ "action": action, "back": back })).await?;
            show(client, &pressing, || {
                let waits: Vec<&str> = list(&pressing["waits"]).iter().filter_map(Value::as_str).collect();
                let did = match (back, pressing["done"] == true) {
                    (false, true) => format!("pressed {action} on {id}"),
                    (true, true) => format!("took back {action} on {id}"),
                    (false, false) => format!("{action} stands pressed on {id} already"),
                    (true, false) => format!("{action} was not pressed on {id}"),
                };
                match (pressing["released"] == true, waits.as_slice()) {
                    (true, _) => format!("{did}: nothing holds it where it is now"),
                    (false, []) => did,
                    (false, waits) => format!("{did}; it waits for {}", waits.join(", ")),
                }
            });
        }
        Some(project) => {
            let board = client.get(&format!("/api/board/{project}")).await?;
            show(client, &board, || {
                let mut lines = Vec::new();
                for column in list(&board["columns"]) {
                    let cards: Vec<&Value> =
                        list(&board["items"]).iter().filter(|card| card["status"] == *column).collect();
                    lines.push(format!("{} ({})", column.as_str().unwrap_or_default(), cards.len()));
                    for card in cards {
                        let who = card["assignee"].as_str().map_or(String::new(), |who| format!("  [{who}]"));
                        // What a card says of itself: that something was
                        // asked on it, or that it waits its turn.
                        let says = |field: &str, what: &str| match card["more"][field].as_str() {
                            Some(said) if !said.is_empty() => format!("  ({what}: {said})"),
                            _ => String::new(),
                        };
                        let (asks, waits) = (says("question", "asks"), says("waiting", "waits its turn"));
                        // What was pressed on it, by name.
                        let pressed = kernel::pressed::Pressed::read(
                            &list(&card["pressed"])
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>(),
                        );
                        let pressed = match pressed
                            .presses
                            .iter()
                            .map(|press| press.action.as_str())
                            .collect::<Vec<_>>()
                        {
                            names if names.is_empty() => String::new(),
                            names => format!("  (pressed: {})", names.join(", ")),
                        };
                        lines.push(format!(
                            "  {}  {}{who}{asks}{waits}{pressed}",
                            text(card, "id"),
                            text(card, "title")
                        ));
                    }
                }
                lines.join("\n")
            });
        }
        None => {
            return Err("missing the project: `board <project>`, `board add <project> <title>`, `board move <card> <column>`, `board give <card> <agent>|<person>`, `board set <card> <field=value>...` or `board press <card> <action>`".into());
        }
    }
    Ok(())
}

pub async fn agent(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        None | Some("list") => {
            let agents = client.get("/api/agents").await?;
            show(client, &agents, || {
                let rows: Vec<Vec<String>> = list(&agents["agents"])
                    .iter()
                    .map(|agent| {
                        vec![
                            text(agent, "name").into(),
                            text(agent, "adapter").into(),
                            // Its project, and the environment of it it works under, where it says one.
                            match (agent["project"].as_str(), text(agent, "environment")) {
                                (Some(project), "") => project.to_string(),
                                (Some(project), under) => format!("{project} under {under}"),
                                (None, _) => "-".to_string(),
                            },
                            match (agent["active"] == true, agent["leaving"] == true) {
                                (true, true) => "leaving",
                                (true, false) => "active",
                                (false, _) => "stopped",
                            }
                            .into(),
                            // The role it was brought up in, where it has one.
                            match text(agent, "role") {
                                "" => String::new(),
                                // A word of the fleet's own, said as a word: `an architect`.
                                role if role.starts_with(['a', 'e', 'i', 'o', 'u']) => format!("an {role}"),
                                role => format!("a {role}"),
                            },
                            // What its program sees of this device, in the
                            // daemon's words, as walls are held while it
                            // runs. The built-in assistant is run by the
                            // daemon itself, and has no program to keep in.
                            match &agent["adapter"] {
                                adapter if adapter == "builtin" => String::new(),
                                // One the factory does not start has no
                                // program here: how it stands is said.
                                adapter if adapter == agents::EXTERNAL => {
                                    crate::external::standing(agent, now_ms())
                                }
                                _ => text(&agent["sees"], "how").into(),
                            },
                            // Whether its program is the daemon's to hold, or goes on by itself.
                            match agent["outlives"] == true && agent["adapter"] != "builtin" {
                                true => "goes on without the daemon".to_string(),
                                false => String::new(),
                            },
                            // Whether it may look things up and fetch pages by itself.
                            match agent["web"] == true && agent["adapter"] != "builtin" {
                                true => "may use the web".to_string(),
                                false => String::new(),
                            },
                            // The keys it is handed, by the names they are kept under.
                            match agent["secrets"]
                                .as_object()
                                .map(|handed| handed.values().filter_map(Value::as_str).collect::<Vec<_>>())
                            {
                                Some(names) if !names.is_empty() => format!("handed {}", names.join(", ")),
                                _ => String::new(),
                            },
                            // And what it may do with the repository there,
                            // where that is not the usual.
                            match (
                                agent["sees"]["walled"] == true && agent["adapter"] != "builtin",
                                text(agent, "git"),
                            ) {
                                (true, git) if !["", "read"].contains(&git) => format!("git {git}"),
                                _ => String::new(),
                            },
                            // What its program signs in with, when it is
                            // not whoever runs the daemon.
                            match list(&agent["signins"]) {
                                [] => String::new(),
                                draws => format!(
                                    "signs in with {}",
                                    draws
                                        .iter()
                                        .filter_map(Value::as_str)
                                        .collect::<Vec<_>>()
                                        .join(", then ")
                                ),
                            },
                        ]
                    })
                    .collect();
                if rows.is_empty() {
                    format!("no agents yet; `{} agent add <name>`", brand::CLI)
                } else {
                    table(&rows)
                }
            });
        }
        Some("adapters") => {
            let agents = client.get("/api/agents").await?;
            show(client, &agents["adapters"], || {
                let default = text(&agents, "default_adapter");
                list(&agents["adapters"])
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|name| if name == default { format!("{name}  (default)") } else { name.to_string() })
                    .collect::<Vec<_>>()
                    .join("\n")
            });
        }
        Some("add") => {
            // What its program signs in with, in order; said once for each,
            // or with commas. Left out, it stays as it was.
            let draws: Vec<&str> = args
                .values("signin")
                .into_iter()
                .flat_map(|said| said.split(','))
                .map(str::trim)
                .filter(|draw| !draw.is_empty())
                .collect();
            let signins = match (args.has("no-signin"), draws.is_empty()) {
                (true, _) => json!([]),
                (false, true) => Value::Null,
                (false, false) => json!(draws),
            };
            // Whether its program works behind a wall, and what it is shown
            // there besides its work. Left out, each stays as it was.
            let wall = match (args.has("wall"), args.has("no-wall")) {
                (true, true) => return Err("`--wall` and `--no-wall` cannot both be said".into()),
                (true, false) => json!(true),
                (false, true) => json!(false),
                (false, false) => Value::Null,
            };
            // Whether its program goes on without the daemon.
            let outlives = match (args.has("outlives"), args.has("no-outlives")) {
                (true, true) => return Err("`--outlives` and `--no-outlives` cannot both be said".into()),
                (true, false) => json!(true),
                (false, true) => json!(false),
                (false, false) => Value::Null,
            };
            // Whether its program may use the web unasked.
            let web = match (args.has("web"), args.has("no-web")) {
                (true, true) => return Err("`--web` and `--no-web` cannot both be said".into()),
                (true, false) => json!(true),
                (false, true) => json!(false),
                (false, false) => Value::Null,
            };
            let shown = |flag: &str, none: &str| match (args.has(none), args.values(flag)) {
                (true, _) => json!([]),
                (false, said) if said.is_empty() => Value::Null,
                (false, said) => json!(said),
            };
            // What it holds, whole, said once for each thing; left out, an
            // agent that is there holds what it held and a new one what
            // an agent usually does.
            let scopes = args.values("scope");
            let holds = if scopes.is_empty() { Value::Null } else { json!(scopes) };
            let body = json!({
                "name": args.need(2, "the agent's name")?,
                "environment": args.value("environment"),
                "scopes": holds,
                "wall": wall,
                "outlives": outlives,
                "web": web,
                "reads": shown("reads", "no-reads"),
                "changes": shown("changes", "no-changes"),
                "git": args.value("git"),
                "secrets": shown("secret", "no-secrets"),
                "adapter": args.value("adapter"),
                "project": args.value("project"),
                "channel": args.value("channel"),
                "model": args.value("model"),
                "instructions": args.value("instructions"),
                "permission_mode": args.value("permission"),
                "signins": signins,
                "role": args.value("role"),
            });
            let agent = client.post("/api/agents", body).await?;
            show(client, &agent, || {
                if agent["adapter"] == agents::EXTERNAL {
                    return crate::external::said_again(&agent);
                }
                // What its program sees, in the daemon's words: with no
                // wall where the factory holds none, whatever it asked for.
                let sees = text(&agent["sees"], "how");
                let alone = match agent["outlives"] == true {
                    true => {
                        "; its program goes on without the daemon, and is taken up where it is by the next"
                    }
                    false => "",
                };
                let web = match agent["web"] == true {
                    true => "; it may use the web unasked",
                    false => "",
                };
                // What it holds is said where it was said: it is the whole of it.
                let holds: Vec<&str> = list(&agent["scopes"]).iter().filter_map(Value::as_str).collect();
                let holds = match scopes.is_empty() {
                    true => String::new(),
                    false => format!("\nIt holds: {}", holds.join(", ")),
                };
                format!(
                    "{} is in, run by `{}`, {sees}{alone}{web}{holds}",
                    text(&agent, "name"),
                    text(&agent, "adapter")
                )
            });
        }
        Some("try") => {
            let name = args.need(2, "the agent's name")?;
            // In a channel kept to its members, as a turn asked there
            // starts it: in the home it has for that channel.
            let within = args.value("channel").map(|channel| channel.trim_start_matches('#'));
            let tried = client.post(&format!("/api/agents/{name}/try"), json!({ "channel": within })).await?;
            show(client, &tried, || {
                let program = text(&tried, "program");
                let wall = match (tried["walled"] == true, within) {
                    (true, Some(_)) => "behind its wall, in the home it has for that channel",
                    (true, None) => "behind its wall",
                    (false, _) => "with no wall",
                };
                match tried["ok"] == true {
                    true => format!(
                        "`{program}` started {wall} and answered: it offers {} models. No prompt was sent.",
                        tried["models"]
                    ),
                    false => format!(
                        "`{program}` did not answer {wall}: {}\n`{} help wall` says how to show it what it needs.",
                        text(&tried, "error"),
                        brand::CLI
                    ),
                }
            });
            if tried["ok"] != true {
                return Err(format!("{name}'s program did not answer"));
            }
        }
        Some("remove") => return crate::removed::agent(client, args).await,
        Some("move") => {
            let name = args.need(2, "the agent's name")?;
            let project = match (args.word(3), args.has("none")) {
                (Some(project), false) => json!(project),
                (None, true) => Value::Null,
                _ => return Err(format!("say where `{name}` is to work: a project, or `--none` for none")),
            };
            let moved = client
                .call(
                    reqwest::Method::PATCH,
                    &format!("/api/agents/{name}"),
                    Some(json!({ "project": project })),
                )
                .await?;
            show(client, &moved, || match moved["project"].as_str() {
                Some(project) => format!("{name} works in {project} from its next piece of work"),
                None => format!("{name} works in no project from its next piece of work"),
            });
        }
        Some("invite") => return crate::external::invite(client, args).await,
        Some("join") => return crate::external::join(client, args).await,
        Some("leave") => return crate::external::leave(client, args).await,
        Some("stop") => {
            let name = args.need(2, "the agent's name")?;
            let how = if args.has("idle") { "?idle=true" } else { "" };
            let stopped =
                client.call(reqwest::Method::DELETE, &format!("/api/agents/{name}{how}"), None).await?;
            if stopped["leaving"] == true {
                println!("{name} takes no more work, and stops when it has finished what it has in hand");
            } else {
                println!("{name} has stopped");
            }
        }
        Some("turns") => {
            let found =
                client.get(&format!("/api/agents/{}/turns", args.need(2, "the agent's name")?)).await?;
            show(client, &found, || {
                let now = now_ms();
                let rows: Vec<Vec<String>> = list(&found["turns"])
                    .iter()
                    .map(|turn| {
                        // One asked in a channel kept to its members comes
                        // without its words to whoever does not read it.
                        let line: String = match turn["kept_from"] == true {
                            true => "(asked in a channel kept to its members)".into(),
                            false => text(turn, "prompt")
                                .lines()
                                .last()
                                .unwrap_or_default()
                                .chars()
                                .take(60)
                                .collect(),
                        };
                        vec![
                            text(turn, "status").into(),
                            text(turn, "kind").into(),
                            ago(now, turn["created"].as_i64().unwrap_or(now)),
                            line,
                        ]
                    })
                    .collect();
                table(&rows)
            });
        }
        Some(other) => {
            return Err(format!(
                "`agent {other}` is not a command; try list, adapters, add, invite, join, leave, try, stop, \
                 remove or turns"
            ));
        }
    }
    Ok(())
}

fn event_line(event: &Value) -> String {
    let source = &event["source"];
    let from: Vec<&str> =
        ["instance", "step", "actor"].iter().filter_map(|key| source[*key].as_str()).collect();
    format!("{:>6}  {:<22} {}  {}", event["seq"], text(event, "topic"), event["data"], from.join(" "))
}

/// The kinds of event there are, or what one of them carries: what a
/// handler can be written against, as `actions` is what a step can.
async fn kinds(client: &Client, topic: Option<&str>) -> Result<(), String> {
    let catalog = client.get("/api/actions").await?;
    let all = list(&catalog["events"]);
    let kept = |event: &Value| {
        if event["kept"] == false { " (passed on, not kept: no handler hears it)" } else { "" }
    };
    let Some(topic) = topic else {
        show(client, &catalog["events"], || {
            let rows: Vec<Vec<String>> = all
                .iter()
                .map(|event| {
                    vec![text(event, "topic").into(), format!("{}{}", text(event, "summary"), kept(event))]
                })
                .collect();
            table(&rows)
        });
        return Ok(());
    };
    let event = all.iter().find(|event| event["topic"] == topic).ok_or_else(|| {
        let family = topic.split('.').next().unwrap_or_default();
        let near: Vec<&str> = all
            .iter()
            .map(|event| text(event, "topic"))
            .filter(|known| known.split('.').next() == Some(family))
            .collect();
        match near.is_empty() {
            true => format!(
                "this program emits no event called `{topic}`. What a blueprint says with `emit` is not listed here"
            ),
            false => format!("there is no event called `{topic}`; under `{family}.` there are: {}", near.join(", ")),
        }
    })?;
    show(client, event, || {
        let mut lines = vec![format!("{topic}: {}{}", text(event, "summary"), kept(event))];
        let rows: Vec<Vec<String>> = list(&event["fields"])
            .iter()
            .map(|field| {
                let always = if field["always"] == true { "always" } else { "" };
                vec![
                    format!("  {}", text(field, "name")),
                    text(field, "type").into(),
                    always.into(),
                    text(field, "help").into(),
                ]
            })
            .collect();
        match rows.is_empty() {
            true if event["open"] == true => {}
            true => lines.push("carries nothing".into()),
            false => {
                lines.push("carries, as `event.data.<name>`:".into());
                lines.push(table(&rows));
            }
        }
        if event["open"] == true {
            lines.push("and whatever else its emitter put in it, under names of its own".into());
        }
        lines.join("\n")
    });
    Ok(())
}

/// `emit <topic> [key=value]... [--data JSON]`: says an event of a
/// blueprint's own topic, as whoever the key in hand lets it be said by.
pub async fn emit(client: &Client, args: &Args) -> Result<(), String> {
    let topic = args.need(1, "the topic to say, such as `shop.paid`")?;
    let mut data = match args.value("data") {
        None => serde_json::Map::new(),
        Some(text) => match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(data)) => data,
            _ => {
                return Err(
                    "`--data` takes a mapping written as JSON, such as '{\"order\": \"o-17\"}'".into()
                );
            }
        },
    };
    let mut index = 2;
    while let Some(pair) = args.word(index) {
        index += 1;
        let (key, value) = pair.split_once('=').ok_or_else(|| {
            format!("`{pair}` should be `key=value`: what the event carries, a pair at a time")
        })?;
        data.insert(
            key.to_string(),
            serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.into())),
        );
    }
    let said = client.post(&format!("/api/emit/{}", encode(topic)), Value::Object(data)).await?;
    show(client, &said, || format!("said {} ({})", text(&said, "topic"), said["seq"]));
    Ok(())
}

pub async fn events(client: &Client, args: &Args) -> Result<(), String> {
    if args.has("kinds") {
        return kinds(client, args.value("kinds").or(args.word(1))).await;
    }
    let topics = args.value("topics").map(|topics| format!("&topics={}", encode(topics))).unwrap_or_default();
    let limit = args.value("limit").unwrap_or("30");
    let follow = args.has("follow");
    let found = client.get(&format!("/api/events/recent?limit={limit}{topics}")).await?;
    let mut recent: Vec<&Value> = list(&found["events"]).iter().collect();
    recent.reverse();
    for event in &recent {
        println!("{}", if client.json { event.to_string() } else { event_line(event) });
    }
    // Asked for more than the daemon hands at once, a list cut there says so.
    let asked: u64 = limit.parse().unwrap_or(0);
    if let Some(most) = found["most"].as_u64().filter(|most| found["more"] == true && asked > *most) {
        eprintln!(
            "The daemon hands at most {most} events at once: these are the newest it read. `--topics` narrows what is read, and `--follow` goes on from here."
        );
    }
    if !follow {
        return Ok(());
    }
    // The place handed out with what was read, handed back as it was.
    let since = encode(text(&found, "cursor"));
    let mut response = client.stream(&format!("/api/events?since={since}{topics}")).await?;
    let mut buffer = String::new();
    while let Some(chunk) = response.chunk().await.map_err(|err| format!("the event stream broke: {err}"))? {
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(end) = buffer.find("\n\n") {
            let frame: String = buffer.drain(..end + 2).collect();
            // Said where what is followed may have missed what the runner
            // passed on and did not keep: there is no event to print.
            if frame.lines().any(|line| line.trim() == "event: behind") {
                if !client.json {
                    println!(
                        "(what was said of steps and runs meanwhile was passed on and not kept, and is not here)"
                    );
                }
                continue;
            }
            let data = frame.lines().find_map(|line| line.strip_prefix("data:"));
            if let Some(event) = data.and_then(|data| serde_json::from_str::<Value>(data.trim()).ok()) {
                println!("{}", if client.json { event.to_string() } else { event_line(&event) });
            }
        }
    }
    Ok(())
}

pub async fn metrics(client: &Client, args: &Args) -> Result<(), String> {
    // `--project shop` is `--label project=shop`, said the way the rest of
    // the command line says it. A label is written `name=value` here, as a
    // param is, and `name:value` as the daemon is asked.
    let project = args.value("project").map(|project| format!("project:{project}"));
    let labels: Vec<String> =
        args.values("label").into_iter().map(|label| label.replacen('=', ":", 1)).chain(project).collect();
    let narrowed =
        if labels.is_empty() { String::new() } else { format!("&labels={}", encode(&labels.join(","))) };
    if args.has("labels") {
        return labels_there_are(client, args, &narrowed).await;
    }
    let name = args.need(1, "what to measure, e.g. `step` or `http`")?;
    let mut path = format!("/api/metrics?name={}{narrowed}", encode(name));
    for (flag, key) in [("since", "since"), ("by", "by"), ("step", "step")] {
        if let Some(value) = args.value(flag) {
            path.push_str(&format!("&{key}={}", encode(value)));
        }
    }
    let found = client.get(&path).await?;
    show(client, &found, || {
        let mut rows = vec![vec![
            "what".to_string(),
            "count".into(),
            "failed".into(),
            "typical".into(),
            "p95".into(),
            "worst".into(),
        ]];
        if list(&found["series"]).iter().any(|series| series["unit"].is_string()) {
            rows[0].push("in all".into());
        }
        for series in list(&found["series"]) {
            let labels: Vec<String> = series["labels"]
                .as_object()
                .into_iter()
                .flatten()
                .map(|(k, v)| format!("{k}={}", v.as_str().unwrap_or_default()))
                .collect();
            let what = if labels.is_empty() { text(series, "name").to_string() } else { labels.join(" ") };
            let total = &series["total"];
            // A length of time unless the daemon says what it is of (a cost, a size), which is read
            // from its name in one place; then it is a number of that.
            let unit = series["unit"].as_str();
            let amount = |key: &str| {
                let value = total[key].as_f64().unwrap_or(0.0);
                match unit {
                    None => took(value as i64),
                    Some(unit) => crate::client::amount(value, unit),
                }
            };
            let failed = match total["errors"].as_u64().unwrap_or(0) {
                0 => "-".to_string(),
                errors => format!("{errors} ({:.0}%)", total["error_rate"].as_f64().unwrap_or(0.0) * 100.0),
            };
            let mut row =
                vec![what, total["count"].to_string(), failed, amount("p50"), amount("p95"), amount("max")];
            // What is not a time is worth adding up: what it came to in all.
            if unit.is_some() {
                row.push(amount("sum"));
            }
            rows.push(row);
        }
        if rows.len() == 1 { format!("nothing measured as `{name}` in that window") } else { table(&rows) }
    });
    Ok(())
}

/// The labels blueprints gave to what was measured, and the values of each:
/// what `--by` and `--label` can be given besides what every measurement
/// says of itself.
async fn labels_there_are(client: &Client, args: &Args, narrowed: &str) -> Result<(), String> {
    let mut path = format!("/api/metrics/labels?name={}{narrowed}", encode(args.word(1).unwrap_or("*")));
    if let Some(since) = args.value("since") {
        path.push_str(&format!("&since={}", encode(since)));
    }
    let found = client.get(&path).await?;
    show(client, &found, || {
        let lines: Vec<String> = found["labels"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(name, values)| {
                let values: Vec<&str> = list(values).iter().filter_map(Value::as_str).collect();
                format!("{name}  {}", values.join(", "))
            })
            .collect();
        if lines.is_empty() {
            "nothing measured in that window carries a label; a blueprint gives them with `labels:`".into()
        } else {
            lines.join("\n")
        }
    });
    Ok(())
}

pub async fn services(client: &Client) -> Result<(), String> {
    let fleet = client.get("/api/fleet").await?;
    show(client, &json!({ "services": fleet["services"], "routers": fleet["routers"] }), || {
        let mut lines: Vec<String> = Vec::new();
        let rows: Vec<Vec<String>> = list(&fleet["services"])
            .iter()
            .map(|service| {
                vec![
                    text(service, "name").into(),
                    text(service, "status").into(),
                    format!("port {}", service["port"]),
                    format!("pid {}", service["pid"]),
                    format!("{} restarts", service["restarts"]),
                ]
            })
            .collect();
        lines.push(if rows.is_empty() { "no services".to_string() } else { table(&rows) });
        for router in list(&fleet["routers"]) {
            let backends: Vec<String> = list(&router["backends"])
                .iter()
                .map(|backend| {
                    format!(
                        "{} weight {} ({} connections, {} failed)",
                        text(backend, "target"),
                        backend["weight"],
                        backend["connections"],
                        backend["failures"]
                    )
                })
                .collect();
            lines.push(format!(
                "router {} on port {} -> {}",
                text(router, "name"),
                router["listen"],
                backends.join("; ")
            ));
        }
        lines.join("\n")
    });
    Ok(())
}

pub async fn admin(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(0) {
        Some("invite") => {
            let asked = json!({ "email": args.word(1), "expires": args.value("expires") });
            let made = client.post("/api/invites", asked).await?;
            show(client, &made, || {
                format!(
                    "invitation code: {}\nThey register with `{} register --invite <code>`. It works until {}.",
                    text(&made, "code"),
                    brand::CLI,
                    kernel::clock::rfc3339(made["expires"].as_i64().unwrap_or(0))
                )
            });
        }
        Some("give") => {
            let email = args.need(1, "whose account, by its email")?;
            let scope = args.need(2, "the scope, such as `environments:secrets`")?;
            let taking = args.has("not");
            let accounts = client.get("/api/accounts").await?;
            let found = list(&accounts["accounts"]).iter().find(|one| one["email"] == email).cloned();
            let Some(account) = found else {
                return Err(format!("there is no account for {email}; `{} whoami` says yours", brand::CLI));
            };
            let at = format!("/api/accounts/{}/given/{}", text(&account, "id"), crate::client::encode(scope));
            let method = if taking { reqwest::Method::DELETE } else { reqwest::Method::PUT };
            let done = client.call(method, &at, None).await?;
            show(client, &done, || {
                let name = text(&done["account"], "name");
                let given: Vec<&str> =
                    list(&done["account"]["given"]).iter().filter_map(Value::as_str).collect();
                let beyond = match given.is_empty() {
                    true => "nothing beyond what their role holds".to_string(),
                    false => format!("beyond what their role holds: {}", given.join(", ")),
                };
                match (taking, done["changed"] == true) {
                    (true, _) => format!(
                        "{name} holds `{scope}` no longer, and neither do {} of their keys. They hold {beyond}",
                        done["keys"].as_u64().unwrap_or(0)
                    ),
                    (false, true) => format!("{name} holds `{scope}` from now on. They hold {beyond}"),
                    (false, false) => format!("{name} holds `{scope}` already; nothing was changed"),
                }
            });
        }
        Some("tier") => {
            let set = client
                .post("/api/tier", json!({ "tier": args.need(1, "free, premium or enterprise")? }))
                .await?;
            show(client, &set, || format!("this installation is now on the {} tier", text(&set, "tier")));
        }
        Some("keys") => {
            let found = client.get("/api/credentials").await?;
            show(client, &found, || {
                let rows: Vec<Vec<String>> = list(&found["credentials"])
                    .iter()
                    .filter(|one| one["kind"] == "key")
                    .map(|one| {
                        let holds: Vec<&str> =
                            list(&one["scopes"]).iter().filter_map(Value::as_str).collect();
                        let state = if one["revoked"].is_null() { "in use" } else { "withdrawn" };
                        vec![text(one, "id").into(), text(one, "name").into(), state.into(), holds.join(", ")]
                    })
                    .collect();
                if rows.is_empty() { "no keys have been made".to_string() } else { table(&rows) }
            });
        }
        Some("key") if args.word(1) == Some("roll") => {
            let id = args.need(2, "the key's id, as `keys` lists it")?;
            let rolled = client.post(&format!("/api/credentials/{id}/roll"), json!({})).await?;
            show(client, &rolled, || {
                format!(
                    "{}\nThis is `{}`'s new secret, shown once: the one it had is refused from now on. What it \
                     applied goes on.",
                    text(&rolled, "token"),
                    text(&rolled, "name")
                )
            });
        }
        Some("key") if args.word(1) == Some("revoke") => {
            let id = args.need(2, "the key's id, as `keys` lists it")?;
            let leave = if args.has("leave") { "?leave=true" } else { "" };
            let gone =
                client.call(reqwest::Method::DELETE, &format!("/api/credentials/{id}{leave}"), None).await?;
            show(client, &gone, || {
                let named = |all: &Value| -> Vec<String> {
                    list(all)
                        .iter()
                        .map(|one| {
                            let project = one["project"]
                                .as_str()
                                .map_or(String::new(), |project| format!(" in {project}"));
                            let why = one["why"].as_str().map_or(String::new(), |why| format!(": {why}"));
                            format!("  {}  {}{project}{why}", text(one, "id"), text(one, "blueprint"))
                        })
                        .collect()
                };
                let mut lines = vec![format!(
                    "`{}` is withdrawn: it is nobody's credential any more",
                    text(&gone, "name")
                )];
                let with: Vec<String> = list(&gone["keys"])
                    .iter()
                    .map(|one| format!("  {}  {}", text(one, "id"), text(one, "name")))
                    .collect();
                if !with.is_empty() {
                    lines.push("So are the keys made with it:".into());
                    lines.extend(with);
                }
                let (running, stopped) = (named(&gone["running"]), named(&gone["stopped"]));
                if !stopped.is_empty() {
                    lines.push("What it applied was stopped:".into());
                    lines.extend(stopped);
                }
                if !running.is_empty() {
                    lines.push(format!(
                        "What it applied is still running, with what the key held. Stop one with `{} stop <id>`:",
                        brand::CLI
                    ));
                    lines.extend(running);
                }
                lines.join("\n")
            });
        }
        Some("key") => {
            // Told no scope, a key holds what whoever makes it does. That
            // is left for the daemon to say, which knows what it is: a
            // list with nothing in it asks for a key that can do nothing.
            let mut asked = json!({ "name": args.word(1).unwrap_or("key") });
            let scopes = args.values("scope");
            if !scopes.is_empty() {
                asked["scopes"] = json!(scopes);
            }
            let made = client.post("/api/credentials", asked).await?;
            show(client, &made, || {
                let holds: Vec<&str> = list(&made["scopes"]).iter().filter_map(Value::as_str).collect();
                format!(
                    "{}\nThis is shown once. Keep it somewhere safe.\nIt holds: {}",
                    text(&made, "token"),
                    holds.join(", ")
                )
            });
        }
        _ => return Err("not a command".into()),
    }
    Ok(())
}
