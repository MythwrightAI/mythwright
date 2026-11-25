//! Groups of people and agents, and what their members may ask
//! (`people::groups`): each a call of what keeps them, as whoever asks.
//! Who is in each is read as who has an account here is (`/people`,
//! `channels:read`); what a group holds is told to whoever manages the
//! factory, as what each account holds is (`/accounts`).

use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use kernel::Error;
use serde_json::{Value, json};
use studio::groups::{Changed, Group};

use super::{required, text};
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/groups", get(list).post(make))
        .route("/groups/{group}", axum::routing::patch(change).delete(remove))
        .route("/groups/{group}/members", post(join))
        .route("/groups/{group}/members/{member}", delete(leave))
}

/// A group as it is handed to `who`: what it holds only to whoever
/// manages the factory.
fn shown(group: &Group, admin: bool) -> Value {
    let mut said = json!(group);
    if !admin {
        said.as_object_mut().map(|said| said.remove("holds"));
    }
    said
}

fn changed(changed: &Changed) -> Value {
    json!({ "group": changed.group.as_ref().map(|group| shown(group, true)), "changed": changed.changed, "keys": changed.keys })
}

/// What a list of scopes said in a body comes to.
fn scopes(body: &Value, key: &str) -> Option<Vec<String>> {
    body.get(key)
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(|scope| scope.as_str().map(str::to_string)).collect())
}

async fn list(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("channels:read")?;
    let admin = who.can("admin");
    let groups: Vec<Value> = app.studio.identity.groups().iter().map(|group| shown(group, admin)).collect();
    Ok(Json(json!({ "groups": groups })))
}

async fn make(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let holds = scopes(&body, "holds").unwrap_or_default();
    let group = app.studio.identity.make_group(required(&body, "name")?, &holds, &who).await?;
    Ok(Json(json!({ "group": shown(&group, true) })))
}

/// Calls a group something else, or has it hold another list, or both.
async fn change(
    State(app): State<App>,
    Auth(who): Auth,
    Path(group): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let identity = &app.studio.identity;
    let mut last = None;
    if let Some(name) = text(&body, "name") {
        last = Some(identity.rename_group(&group, name, &who).await?);
    }
    if let Some(holds) = scopes(&body, "holds") {
        let id = identity.group(&group)?.id;
        last = Some(identity.hold_group(&id, &holds, &who).await?);
    }
    let Some(done) = last else {
        return Err(Error::invalid(
            "say what a group is to be called (`name`), or what it is to hold (`holds`)",
        )
        .into());
    };
    Ok(Json(changed(&done)))
}

async fn remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(group): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(changed(&app.studio.identity.remove_group(&group, &who).await?)))
}

/// Puts somebody in a group: a person by their handle, their email or
/// `user:<account>`, an agent by its name or `agent:<name>`.
async fn join(
    State(app): State<App>,
    Auth(who): Auth,
    Path(group): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    let member = member(&app, required(&body, "who")?).await?;
    Ok(Json(changed(&app.studio.identity.join_group(&group, &member, &who).await?)))
}

async fn leave(
    State(app): State<App>,
    Auth(who): Auth,
    Path((group, member)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    let member = match member.contains(':') {
        true => member,
        false => self::member(&app, &member).await?,
    };
    Ok(Json(changed(&app.studio.identity.leave_group(&group, &member, &who).await?)))
}

/// Whom `word` names, as a card's assignee is found: an agent named so
/// is one there is.
async fn member(app: &App, word: &str) -> Result<String, Error> {
    let agents: Vec<String> = app.agents.slots().await?.into_iter().map(|slot| slot.name).collect();
    let accounts = app.studio.identity.accounts().await?;
    let actor = studio::handle::assignee(word, &agents, &accounts).map_err(Error::invalid)?;
    if let Some(name) = actor.strip_prefix("agent:")
        && !agents.iter().any(|one| one == name)
    {
        return Err(Error::not_found(format!(
            "there is no agent `{name}`: the Agents page lists every agent there is"
        )));
    }
    Ok(actor)
}
