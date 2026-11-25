//! Workspaces: what gathers the projects, and the channels, that belong
//! together ([`studio::workspaces`]). Grouping and nothing else: nothing
//! here lets anybody in or keeps anybody out.

use axum::extract::{Path, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use conductor::InstanceFilter;
use kernel::Error;
use serde_json::{Value, json};
use studio::Belongs;

use super::{required, text};
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/workspaces", get(listed).post(create))
        .route("/workspaces/{id}", get(one).patch(rename).delete(remove))
        .route("/projects/{id}/workspace", put(place_project))
        .route("/channels/{id}/belongs", put(place_channel))
}

/// Every workspace, by what it is called, with what is in each, and
/// which of them is the usual one (`usual`: where a project goes when
/// nobody says). Whoever may read projects reads this: it says which of
/// them go together.
async fn listed(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    let all = app.studio.workspaces.list_for(&who).await?;
    let usual = all.iter().find(|one| one.usual).map(|one| one.id.clone());
    Ok(Json(json!({ "workspaces": all, "usual": usual })))
}

async fn one(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    let workspaces = &app.studio.workspaces;
    Ok(Json(json!(workspaces.shown_to(workspaces.get(&id).await?, &who).await?)))
}

async fn create(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.workspaces.create(required(&body, "name")?, &who).await?)))
}

/// Calls one something else: `{ "name": ... }`. Its id stays.
async fn rename(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let workspaces = &app.studio.workspaces;
    let renamed = workspaces.rename(&id, required(&body, "name")?, &who).await?;
    Ok(Json(json!(workspaces.shown_to(renamed, &who).await?)))
}

/// Takes one away, and nothing that was in it: `{ "to": "<workspace>" }`
/// says where its projects and its own channels go, which one that holds
/// any is not taken away without. A request with no body is one that
/// says nowhere. The last workspace there is stays. Moving its channels
/// is asked what saying what a channel belongs to is asked, of whoever
/// removes it and for each channel it holds.
async fn remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    body: Option<Json<Value>>,
) -> ApiResult<Json<Value>> {
    let body = body.map(|Json(body)| body).unwrap_or_default();
    let to = text(&body, "to");
    let gone = app.studio.workspaces.remove(&id, to, &who).await?;
    // By its id, however it was named.
    let to = match to {
        Some(to) => app.studio.workspaces.get(to).await.ok().map(|one| one.id),
        None => None,
    };
    Ok(Json(json!({
        "id": gone.id, "name": gone.name, "projects": gone.projects, "channels": gone.channels, "to": to,
    })))
}

/// Puts a project in a workspace, out of the one it is in: `{ "workspace":
/// "<id>" }`. A project is always in one, so `null` is refused, saying so.
async fn place_project(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let Some(asked) = body.get("workspace") else {
        return Err(Error::invalid("`workspace` is needed: the workspace it is to be in").into());
    };
    let workspace = app.studio.workspaces.place_project(&id, asked.as_str(), &who).await?;
    Ok(Json(json!({ "project": id, "workspace": workspace.id })))
}

/// Says what a channel belongs to: `{ "project": ... }` or `{ "workspace":
/// ... }`. Neither is refused: a channel is always a project's or a
/// workspace's. Refused while something that is
/// running for the channel's project says it posts there: that is the
/// project's own channel, where its fleet reports and where the factory's
/// bot is asked about that project.
async fn place_channel(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let reader = app.studio.channels.may_write(&id, &who).await?;
    let belongs = belonging(&body)?;
    let channel = app.studio.channels.get(&reader, &id).await?;
    if belongs == Belongs::Nothing {
        return Err(studio::workspaces::to_nothing(&channel.name).into());
    }
    if let Some(project) = &channel.project
        && belongs != Belongs::Project(project)
    {
        let filter = InstanceFilter { live_only: true, limit: 10_000, ..InstanceFilter::default() };
        let live = app.engine.instances(filter).await?;
        let posting = super::talk::posts_in(&channel, &live);
        if let Some(first) = posting.iter().find(|one| one.project.as_deref() == Some(project.as_str())) {
            return Err(Error::conflict(format!(
                "#{} is the channel of `{project}`: `{}` ({}, {}) is running for that project and posts there. \
                 Stop it first (`{cli} stop {}`), and then say what the channel belongs to",
                channel.name,
                first.key,
                first.blueprint,
                first.id,
                first.id,
                cli = brand::CLI,
            ))
            .into());
        }
    }
    let channel = app.studio.workspaces.place_channel(&channel.id, belongs, &who).await?;
    let workspace = app.studio.workspaces.of_channel(&channel.id).await?;
    Ok(Json(
        json!({ "channel": channel.id, "name": channel.name, "project": channel.project, "workspace": workspace }),
    ))
}

/// What a request says a channel belongs to: a project or a workspace,
/// and never both. With neither, nothing is said: which a channel that
/// is being made takes for the usual workspace, and one that is there
/// is refused.
pub(super) fn belonging(body: &Value) -> Result<Belongs<'_>, Error> {
    match (text(body, "project"), text(body, "workspace")) {
        (Some(_), Some(_)) => Err(Error::invalid(
            "a channel belongs to a project or to a workspace, and not to both: say one of `project` and `workspace`",
        )),
        (Some(project), None) => Ok(Belongs::Project(project)),
        (None, Some(workspace)) => Ok(Belongs::Workspace(workspace)),
        (None, None) => Ok(Belongs::Nothing),
    }
}
