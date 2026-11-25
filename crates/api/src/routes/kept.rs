//! The door of a channel kept to its members: an open channel kept, a
//! kept one opened again, somebody leaving, and whether its agents are
//! asked nowhere else (`talk::door`, `agents::door`).
//!
//! Each asks the channel first, as every route of a channel does
//! (`Channels::may_write`): one kept from whoever asks answers as a
//! channel that is not there does, whatever else would have refused
//! them.

use axum::extract::{Path, State};
use axum::routing::{post, put};
use axum::{Json, Router};
use kernel::{Error, Principal};
use serde_json::{Value, json};

use super::{required, text};
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route("/channels/{id}/keep", post(keep))
        .route("/channels/{id}/open", post(open))
        .route("/channels/{id}/alone", put(alone))
        .route("/channels/{id}/leave", post(leave))
}

/// Whether `key` of a request says so: said as anything but so or not
/// so, it is not taken for not so, or an open channel would be made
/// where a kept one was asked for.
pub(super) fn flag(body: &Value, key: &str) -> Result<bool, Error> {
    match body.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(said)) => Ok(*said),
        Some(_) => Err(Error::invalid(format!("`{key}` is true or false"))),
    }
}

/// The person with an account here who is `actor`.
pub(super) async fn person(app: &App, actor: &str) -> Result<studio::Member, Error> {
    let accounts = app.studio.identity.accounts().await?;
    let found = accounts.into_iter().find(|account| account.principal().actor() == actor).ok_or_else(|| {
        Error::not_found(format!(
            "nobody with an account here is `{actor}`; a person joins the installation first, by invitation"
        ))
    })?;
    Ok(studio::Member::new(actor, found.name))
}

/// Who a request says is in a channel that is being made, beside
/// whoever makes it (`members`, each an actor): the people, and the
/// agents by name.
pub(super) async fn members(
    app: &App,
    who: &Principal,
    body: &Value,
) -> Result<(Vec<studio::Member>, Vec<String>), Error> {
    let said = match body.get("members") {
        None | Some(Value::Null) => return Ok((Vec::new(), Vec::new())),
        Some(Value::Array(said)) => said,
        Some(_) => {
            return Err(Error::invalid("`members` is a list of actors: `user:<id>` or `agent:<name>`"));
        }
    };
    let (mut people, mut agents): (Vec<studio::Member>, Vec<String>) = (Vec::new(), Vec::new());
    for one in said {
        let actor = one.as_str().unwrap_or_default();
        match actor.split_once(':') {
            Some(("agent", name)) if !agents.iter().any(|agent| agent == name) => {
                agents.push(name.to_string())
            }
            Some(("agent", _)) => {}
            Some(("user", _)) if actor == who.actor() || people.iter().any(|one| one.actor == actor) => {}
            Some(("user", _)) => people.push(person(app, actor).await?),
            _ => {
                return Err(Error::invalid(format!(
                    "`{one}` is not somebody a channel is made with: a member is `user:<id>` or `agent:<name>`"
                )));
            }
        }
    }
    Ok((people, agents))
}

/// Keeps an open channel to its members: whoever may write in it.
async fn keep(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.agents.keep_channel(&id, &who).await?)))
}

/// Opens a kept channel again, which is the factory's owner's to do and
/// nobody else's. `name` is what it is called once it is open, where
/// what it was called while it was kept is not to be.
async fn open(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    body: Option<Json<Value>>,
) -> ApiResult<Json<Value>> {
    // Asked of the channel first: one kept from whoever asks is not
    // there, whoever they are.
    app.studio.channels.may_write(&id, &who).await?;
    let accounts = app.studio.identity.accounts().await?;
    let owner =
        |account: &studio::Account| account.role == "owner" && account.principal().actor() == who.actor();
    if !accounts.iter().any(owner) {
        return Err(Error::denied(
            "only the factory's owner opens a channel again that is kept to its members: what was said in it \
             was said to its members",
        )
        .into());
    }
    // And a key of the owner's only where it was given what managing
    // the factory takes.
    who.require("admin")?;
    let body = body.map(|Json(body)| body).unwrap_or_default();
    Ok(Json(json!(app.agents.open_channel(&id, text(&body, "name"), &who).await?)))
}

/// Says whether a kept channel's agents are asked nowhere else: whoever
/// may write in it.
async fn alone(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    app.studio.channels.may_write(&id, &who).await?;
    let alone = match body.get("alone") {
        Some(Value::Bool(alone)) => *alone,
        _ => return Err(Error::invalid("`alone` is needed, and is true or false").into()),
    };
    Ok(Json(json!(app.agents.hold_alone(&id, alone, &who).await?)))
}

/// Takes whoever asks out of a channel, or the agent they name
/// (`agent`), which whoever may write there and change agents does.
async fn leave(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    body: Option<Json<Value>>,
) -> ApiResult<Json<Value>> {
    let reader = app.studio.channels.may_write(&id, &who).await?;
    let channel = reader.channel().unwrap_or(&id).to_string();
    let body = body.map(|Json(body)| body).unwrap_or_default();
    let left = match body.get("agent") {
        None | Some(Value::Null) => {
            app.studio.channels.step_out(&reader, &channel, &who).await?;
            who.actor()
        }
        Some(_) => {
            who.require("agents:write")?;
            let actor = format!("agent:{}", blueprint::expr::slug(required(&body, "agent")?));
            let within = app.studio.channels.get(&reader, &channel).await?;
            let Some(agent) = within.members.iter().find(|member| member.actor == actor) else {
                let called = studio::door::called(&within);
                return Err(Error::invalid(format!("no agent called that is in {called}")).into());
            };
            app.agents.leave_channel(&channel, &agent.name, &who).await?;
            actor
        }
    };
    Ok(Json(json!({ "channel": channel, "left": left })))
}
