//! Conversations: a message written to one or several people and agents
//! (`talk::conversed`), and one opened as a channel.
//!
//! A conversation is a channel kept to its members that says it is one,
//! so reading it, saying something in it, attaching to it and leaving it
//! are the routes of any channel, asked of the channel as they always
//! are. What is here is what is a conversation's own: writing one, which
//! makes it with who is in it and says its first message in one request,
//! so that nothing that fails leaves one with nothing said; and opening
//! one as a channel, which one of its members does.

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use kernel::{Error, Principal};
use serde_json::{Value, json};

use super::{required, text};
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route("/inbox/conversations", post(write))
        .route("/inbox/conversations/{id}/channel", post(open_as_channel))
}

/// Who a conversation is written to (`to`): each an actor
/// (`user:<account>`, `agent:<name>`), or a handle with or without its
/// `@`, which is the one person or agent here that is called so. The
/// people, and the agents by name.
async fn to(app: &App, who: &Principal, body: &Value) -> Result<(Vec<studio::Member>, Vec<String>), Error> {
    let said = match body.get("to") {
        Some(Value::Array(said)) if !said.is_empty() => said,
        _ => {
            return Err(Error::invalid(
                "`to` is needed: who the conversation is written to, each a handle (`@nell`) or an actor \
                 (`user:<account>`, `agent:<name>`)",
            ));
        }
    };
    let accounts = app.studio.identity.accounts().await?;
    let (mut people, mut agents): (Vec<studio::Member>, Vec<String>) = (Vec::new(), Vec::new());
    for one in said {
        let word = one.as_str().unwrap_or_default().trim();
        let actor = match word.split_once(':') {
            Some(("user" | "agent", _)) => word.to_string(),
            Some(_) => {
                return Err(Error::invalid(format!(
                    "`{word}` is nobody a conversation is written to: it is to people and to agents, and never \
                     to a bot"
                )));
            }
            None => {
                let handle = word.trim_start_matches('@').to_lowercase();
                let mut called: Vec<String> = accounts
                    .iter()
                    .filter(|account| !handle.is_empty() && studio::handle::handle(&account.name) == handle)
                    .map(|account| account.principal().actor())
                    .collect();
                let agent = app.agents.slots().await?.into_iter().find(|slot| slot.name == handle);
                called.extend(agent.map(|slot| format!("agent:{}", slot.name)));
                match called.as_slice() {
                    [one] => one.clone(),
                    [] => {
                        return Err(Error::not_found(format!(
                            "nobody here is called `@{handle}`: a conversation is written to a person with an \
                             account here or to an agent, by its handle or as an actor (`user:<account>`, \
                             `agent:<name>`)"
                        )));
                    }
                    several => {
                        return Err(Error::conflict(format!(
                            "{} here are called `@{handle}`: say which as an actor ({})",
                            several.len(),
                            several.join(", ")
                        )));
                    }
                }
            }
        };
        match actor.split_once(':') {
            Some(("agent", name)) if !agents.iter().any(|agent| agent == name) => {
                agents.push(name.to_string())
            }
            Some(("agent", _)) => {}
            _ if actor == who.actor() || people.iter().any(|one| one.actor == actor) => {}
            _ => people.push(super::kept::person(app, &actor).await?),
        }
    }
    Ok((people, agents))
}

/// Writes a conversation: `{ "to": [...], "subject": ..., "text": ... }`.
/// Its channel, with everybody it is to in it, and its first message,
/// as one write: what is refused leaves nothing behind.
async fn write(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    who.require("inbox:write")?;
    who.require("channels:write")?;
    let (people, agents) = to(&app, &who, &body).await?;
    let (subject, said) = (text(&body, "subject").unwrap_or_default(), required(&body, "text")?);
    let (channel, message) = app.agents.converse(subject, said, &people, &agents, &who).await?;
    Ok(Json(json!({ "channel": channel, "message": message })))
}

/// Opens a conversation as a channel, which one of its members does:
/// `{ "name": ..., "workspace": ... }` or `{ "name": ..., "project": ... }`.
/// It is called `name`, or what it was about, and belongs to what is
/// said, or stays the own of the workspace it was made in. Everything
/// said so far goes with it; it is not undone.
async fn open_as_channel(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    body: Option<Json<Value>>,
) -> ApiResult<Json<Value>> {
    let channels = &app.studio.channels;
    // Asked of the channel first: one kept from whoever asks is not there.
    let reader = channels.may_write(&id, &who).await?;
    let found = channels.get(&reader, &id).await?;
    if !found.conversation {
        return Err(Error::invalid(format!(
            "{} is no conversation: it is a channel already, listed among the channels of whoever is in it",
            studio::door::called(&found)
        ))
        .into());
    }
    studio::conversed::may_open_as_channel(&found, &who)?;
    let body = body.map(|Json(body)| body).unwrap_or_default();
    // What it is to belong to is said first, while it is still a
    // conversation and nothing shows where one is: stopped between the
    // two, it is asked again and nothing is half done.
    match super::workspaces::belonging(&body)? {
        studio::Belongs::Nothing => {}
        belongs => {
            app.studio.workspaces.place_channel(&found.id, belongs, &who).await?;
        }
    }
    let opened = channels.open_as_channel(&reader, &found.id, text(&body, "name"), &who).await?;
    let workspace = app.studio.workspaces.of_channel(&opened.id).await?;
    Ok(Json(json!({ "channel": opened, "workspace": workspace })))
}
