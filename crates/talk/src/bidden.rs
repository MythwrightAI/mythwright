//! What the factory's bot does when it is spoken to.
//!
//! It has no model, so it does not read. A message that names it is taken
//! as one of the commands of its list (`commanded`), word for word, or it
//! is not taken at all and is answered once with what it does take. It
//! never guesses: whoever wants to be understood in ordinary words asks an
//! agent.
//!
//! The commands are the command line's own, in the command line's words,
//! and each is done the way the command line does it: by asking one of
//! the daemon's own routes, as whoever said the message, let in as they
//! were when they said it. So the bot holds nothing. What a key may not do
//! with the command line it may not do by asking the bot to, and it is
//! refused by the same route in the same words. That is why a command is
//! a request and not a call of what the route calls: the check is the
//! route's, and there is one of it. The routes are the daemon's, so it
//! hands the asking in ([`Routes`]), and nothing here knows them but by
//! their addresses.
//!
//! Nothing said in a channel reaches a shell, or a command line a shell
//! reads. A word that becomes part of an address is held to what an id is
//! made of (`part`); everything else is sent as JSON.
//!
//! The first set is what is asked of a factory from where its work is
//! talked about: what is running, the board, a card moved, an agent
//! stopped, an event said; the rest is `asking`. What is left to
//! the command line is what takes a file or a secret, what makes or
//! removes something that is not put back by saying so again (a project,
//! a key, a record, an instance stopped), and applying a blueprint, which
//! is a decision about a text and is better made where the text can be
//! read.
//!
//! It came out of the daemon (`api::commands`) when that crate was at its
//! ceiling: it asked nothing of the routes but to be answered by them.

use kernel::{BoxFut, Principal};
use serde_json::{Map, Value, json};

use crate::commanded::{
    Command, Unread, after, beyond, count, elsewhere, help, latest, list, listed, text, value_of, words,
};
use crate::{Channels, Message};

/// How a route is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Get,
    Post,
    Patch,
    Delete,
}

/// The daemon's own routes, asked as somebody already let in: what one
/// answered, or the words it refused in.
pub trait Routes: Send + Sync {
    fn ask<'a>(
        &'a self,
        who: &'a Principal,
        verb: Verb,
        path: String,
        body: Option<Value>,
    ) -> BoxFut<'a, Result<Value, String>>;
}

/// Asks one of the daemon's routes as `who`.
pub(crate) async fn ask(
    routes: &dyn Routes,
    who: &Principal,
    verb: Verb,
    path: String,
    body: Option<Value>,
) -> Result<Value, String> {
    routes.ask(who, verb, path, body).await
}

/// Does what `message` asks, as `who`, and says what came of it under the
/// message: once, whatever it was.
pub async fn answer(channels: &Channels, routes: &dyn Routes, who: &Principal, message: &Message) {
    let name = channels.speaker.name();
    // The bot answers where it was spoken to, by leave of whoever spoke:
    // they wrote there a moment ago, and where they no longer may it
    // says nothing.
    let Ok(reader) = channels.may_write(&message.channel, who).await else { return };
    let found = channels.get(&reader, &message.channel).await;
    let project = found.ok().and_then(|channel| channel.project);
    // In a project's channel the bot is that project's: what it is asked
    // there is about that project, and it does nothing to another's.
    let within = project.as_deref();
    // What it may ask is a ceiling: it is done as whoever asked, held to
    // no more than that, and never more than they hold.
    let holds = channels.speaker.holds();
    let asking = holds.as_deref().map_or_else(|| who.clone(), |allow| who.within(allow));
    let said = match Command::read(&words(after(&message.body, &name)), within) {
        Ok(Command::Help) => help(&name, within),
        Ok(command) => {
            Doing::run(command, routes, &name, &asking, &message.channel, within).await.unwrap_or_else(
                |refused| beyond(&name, who, &asking, holds.as_deref().unwrap_or_default(), refused),
            )
        }
        Err(Unread::Elsewhere(why)) => why,
        Err(Unread::Unknown(why)) => format!("{why}\n\n{}", help(&name, within)),
    };
    let place = crate::Place {
        reply_to: Some(&message.id),
        thread: message.thread.as_deref(),
        ..crate::Place::default()
    };
    let _ = channels.post_speaker(&reader, &message.channel, &said, place).await;
}

/// What each command does, as `who`.
struct Doing;

impl Doing {
    /// Does it as `who`: what is said of it, or what refused it said.
    ///
    /// `within` is the project whose channel it was asked in, where the
    /// channel has one: what is asked there is about that project. What is
    /// running and who is up are that project's; a card, an agent or an
    /// event that is another's is left alone, and said to be. This is on
    /// top of what whoever asked holds, and never in place of it.
    async fn run(
        command: Command,
        routes: &dyn Routes,
        speaker: &str,
        who: &Principal,
        channel_id: &str,
        within: Option<&str>,
    ) -> Result<String, String> {
        let get = |path: String| ask(routes, who, Verb::Get, path, None);
        // What is alive, and the agents: the project's where there is one.
        let of = within.map(|project| format!("&project={project}")).unwrap_or_default();
        let ours = |agent: &&Value| within.is_none_or(|project| agent["project"] == project);
        Ok(match command {
            Command::Help => help(speaker, within),
            Command::Status => {
                let meta = get("/api/meta".into()).await?;
                let instances = get(format!("/api/instances?live=true&limit=1000{of}")).await?;
                let agents = get("/api/agents".into()).await?;
                let instances = list(&instances["instances"]);
                let agents: Vec<&Value> = list(&agents["agents"]).iter().filter(ours).collect();
                let behind = instances.iter().filter(|instance| !instance["behind"].is_null()).count();
                let working = agents.iter().filter(|agent| agent["active"] == true).count();
                let profile = meta["profile"].as_str().map(|name| format!(" ({name})")).unwrap_or_default();
                let whose = within.map(|project| format!("For `{project}`: ")).unwrap_or_default();
                format!(
                    "{} {}{profile} is running. {whose}{} alive, {behind} of them behind the text now \
                     there. {working} {} up.",
                    text(&meta, "name"),
                    text(&meta, "version"),
                    count(instances.len(), "instance is", "instances are"),
                    if working == 1 { "agent is" } else { "agents are" },
                )
            }
            Command::Instances => {
                let found = get(format!("/api/instances?live=true&limit=1000{of}")).await?;
                let lines = list(&found["instances"]).iter().map(|instance| {
                    let behind =
                        if instance["behind"].is_null() { "" } else { ", behind the text now there" };
                    format!(
                        "- `{}` {} as {}: {}{behind}",
                        text(instance, "id"),
                        text(instance, "blueprint"),
                        text(instance, "key"),
                        text(instance, "status"),
                    )
                });
                listed(lines.collect(), "Nothing is running.", "instances")
            }
            Command::Board { project } => {
                let board = get(format!("/api/board/{project}")).await?;
                let cards = list(&board["items"]);
                let columns = list(&board["columns"]).iter().filter_map(Value::as_str).map(|column| {
                    let held: Vec<String> = cards
                        .iter()
                        .filter(|card| card["status"] == column)
                        .map(|card| {
                            let with =
                                card["assignee"].as_str().map(|who| format!(" ({who})")).unwrap_or_default();
                            format!("`{}` {}{with}", text(card, "id"), text(card, "title"))
                        })
                        .collect();
                    match held.is_empty() {
                        true => format!("- {column}: nothing"),
                        false => format!("- {column}: {}", held.join("; ")),
                    }
                });
                listed(columns.collect(), "That board has no columns.", "board")
            }
            Command::Move { card, column, note } => {
                // Whose card it is, is the route's to say, after it has said
                // whether whoever asked may move one at all.
                let body = json!({ "status": column, "note": note, "within": within });
                let moved =
                    ask(routes, who, Verb::Patch, format!("/api/board/items/{card}"), Some(body)).await?;
                format!("`{}` is in {}.", text(&moved, "id"), text(&moved, "status"))
            }
            Command::Press { card, action, back } => {
                // As a move is: whose card it is, is the route's to say.
                let body = json!({ "action": action, "back": back, "within": within });
                let path = format!("/api/board/items/{card}/press");
                let pressed = ask(routes, who, Verb::Post, path, Some(body)).await?;
                let waits: Vec<&str> = list(&pressed["waits"]).iter().filter_map(Value::as_str).collect();
                match (pressed["released"] == true, back, waits.as_slice()) {
                    (true, ..) => format!("`{card}`: {action} is pressed, and nothing holds it now."),
                    (false, true, _) => format!("`{card}`: {action} is taken back."),
                    (false, false, []) => format!("`{card}`: {action} is pressed."),
                    (false, false, waits) => {
                        format!("`{card}`: {action} is pressed; it waits for {}.", waits.join(", "))
                    }
                }
            }
            Command::Add { project, title, body, status, labels, fields } => {
                let card = json!({ "title": title, "body": body, "status": status, "labels": labels, "fields": fields });
                let added =
                    ask(routes, who, Verb::Post, format!("/api/board/{project}/items"), Some(card)).await?;
                format!(
                    "`{}` is added to {}: {}",
                    text(&added, "id"),
                    text(&added, "status"),
                    text(&added, "title")
                )
            }
            Command::Give { card, to } => {
                // Whom a word names is the route's to say: an agent by its name, a person by their handle or email.
                let assignee = if to == "nobody" { "" } else { to.as_str() };
                let body = json!({ "assignee": assignee, "within": within });
                let given =
                    ask(routes, who, Verb::Patch, format!("/api/board/items/{card}"), Some(body)).await?;
                match given["assignee"].as_str() {
                    Some(whom) => format!("`{card}` is given to {whom}."),
                    None => format!("`{card}` is given to nobody."),
                }
            }
            Command::Set { card, pairs } => {
                let mut change = Map::new();
                let mut fields = Map::new();
                for (name, value) in pairs {
                    let value = match name.as_str() {
                        // Each is an event of its own, with a command of its own.
                        "status" => {
                            return Err(format!(
                                "Where a card is, is not set: `board move {card} <column>`."
                            ));
                        }
                        "assignee" => {
                            return Err(format!(
                                "Whose a card is, is not set: `board give {card} <agent>|<person>|nobody`."
                            ));
                        }
                        "title" | "body" | "due" => json!(value),
                        "labels" => json!(
                            value.split(',').map(str::trim).filter(|one| !one.is_empty()).collect::<Vec<_>>()
                        ),
                        _ => {
                            fields.insert(name, value_of(&value));
                            continue;
                        }
                    };
                    change.insert(name, value);
                }
                if !fields.is_empty() {
                    change.insert("fields".into(), Value::Object(fields));
                }
                change.insert("within".into(), json!(within));
                let path = format!("/api/board/items/{card}");
                let changed = ask(routes, who, Verb::Patch, path, Some(Value::Object(change))).await?;
                format!("`{}` is changed.", text(&changed, "id"))
            }
            Command::History { card } => {
                ours_only(routes, who, &card, within).await?;
                let kept = get(format!("/api/records/{card}/history")).await?;
                let lines = list(&kept["history"]).iter().map(|line| {
                    let via = kernel::worded::through(&line["data"])
                        .map(|by| format!(" · via {by}"))
                        .unwrap_or_default();
                    format!("- {}{via}", text(line, "said"))
                });
                latest(
                    lines.collect(),
                    &format!("Nothing has been kept of `{card}`."),
                    &format!("board history {card}"),
                )
            }
            Command::Cost { card } => {
                ours_only(routes, who, &card, within).await?;
                let laid = get(format!("/api/board/items/{card}/cost")).await?;
                let rounds = list(&laid["rounds"]);
                let spent = |of: &Value| {
                    let number = |key: &str| of[key].as_u64().unwrap_or(0);
                    format!(
                        "{}, {} tokens sent and {} written",
                        count(number("turns") as usize, "turn", "turns"),
                        number("sent"),
                        number("wrote")
                    )
                };
                let lines = rounds.iter().enumerate().map(|(at, round)| {
                    let came = match text(&round["decided"], "came") {
                        "" => "under way",
                        came => came,
                    };
                    format!("- round {}: {came}, {}", at + 1, spent(&round["spent"]))
                });
                match rounds.is_empty() {
                    true => format!("No agent has taken a turn on `{card}`."),
                    false => format!(
                        "`{card}`: {}, {}.\n{}",
                        count(rounds.len(), "round", "rounds"),
                        spent(&laid["spent"]),
                        listed(lines.collect(), "", &format!("board cost {card}"))
                    ),
                }
            }
            Command::Other { key, said } => {
                let here = crate::asking::Here { channel: channel_id, within };
                return crate::asking::run(routes, who, &here, &key, &said).await;
            }
            Command::Agents => {
                let found = get("/api/agents".into()).await?;
                let lines = list(&found["agents"]).iter().filter(ours).map(|agent| {
                    let role = Some(text(agent, "role")).filter(|role| !role.is_empty());
                    let role = role.map(|role| format!(", {role}")).unwrap_or_default();
                    let state = match (agent["active"] == true, agent["leaving"] == true) {
                        (false, _) => ": stopped",
                        (true, true) => ": leaving when it has finished what it has in hand",
                        (true, false) => "",
                    };
                    format!("- `{}`{role}{state}", text(agent, "name"))
                });
                listed(lines.collect(), "There are no agents.", "agent list")
            }
            Command::Stop { agent, idle } => {
                // Whose agent it is, is the route's to say, in the same order.
                let how = match (idle, within) {
                    (true, Some(project)) => format!("?idle=true&within={project}"),
                    (true, None) => "?idle=true".to_string(),
                    (false, Some(project)) => format!("?within={project}"),
                    (false, None) => String::new(),
                };
                let stopped =
                    ask(routes, who, Verb::Delete, format!("/api/agents/{agent}{how}"), None).await?;
                match stopped["leaving"] == true {
                    true => format!("`{agent}` leaves when it has finished what it has in hand."),
                    false => format!("`{agent}` is stopped."),
                }
            }
            Command::Emit { topic, mut data } => {
                // An event said in a project's channel is that project's: it
                // says so unless it already does, and may not say another's.
                if let Some(project) = within {
                    match data.get("project") {
                        None => drop(data.insert("project".into(), json!(project))),
                        Some(said) if said == project => {}
                        Some(said) => {
                            return Err(elsewhere(
                                project,
                                &format!(
                                    "an event for `{}` is another project's",
                                    blueprint::expr::display(said)
                                ),
                            ));
                        }
                    }
                }
                let said =
                    ask(routes, who, Verb::Post, format!("/api/emit/{topic}"), Some(Value::Object(data)))
                        .await?;
                format!("Said `{}` ({}).", text(&said, "topic"), said["seq"])
            }
        })
    }
}

/// Refuses a record of another project than the channel's, where the
/// channel has one, saying so; asked after whoever asks has been let read
/// it, so that nothing is said of one they may not.
async fn ours_only(
    routes: &dyn Routes,
    who: &Principal,
    record: &str,
    within: Option<&str>,
) -> Result<(), String> {
    let Some(within) = within else { return Ok(()) };
    let found = ask(routes, who, Verb::Get, format!("/api/records/{record}"), None).await?;
    match text(&found, "project") {
        project if project == within => Ok(()),
        _ => Err(elsewhere(within, &format!("`{record}` is another project's"))),
    }
}
