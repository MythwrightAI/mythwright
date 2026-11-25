//! An agent this factory does not start (`agents::external`): invited by
//! whoever may add agents, let in by its invitation, and leaving by its
//! own word.

use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::post;
use kernel::Error;
use serde_json::{Value, json};

use super::collab::strings;
use super::{required, text};
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/agent/invite", post(invite))
        .route("/agent/join", post(join))
        .route("/agent/leave", post(leave))
}

/// Makes an external agent, or invites one that is there again, and
/// answers its invitation, once. What it takes is what adding an agent
/// takes, asked where the agent is made.
async fn invite(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let lasts = match text(&body, "expires").filter(|said| !said.trim().is_empty()) {
        Some(said) => Some(Duration::from_millis(blueprint::duration::parse(said).map_err(|why| {
            Error::invalid(format!(
                "`{said}` is not how long an invitation works: {why}. Say it as 12h or 3d"
            ))
        })?)),
        None => None,
    };
    let asked = agents::Invite {
        name: required(&body, "name")?.to_string(),
        project: text(&body, "project").map(str::to_string),
        scopes: strings(&body, "scopes"),
        lasts,
    };
    let invited = app.agents.invite(asked, &who).await?;
    Ok(Json(json!({
        "agent": invited.agent,
        "invitation": invited.invitation,
        "expires": invited.expires,
        // What whoever is to be that agent runs, where it can reach this
        // daemon: the invitation is read from what it is handed, and is
        // in no command.
        "join": format!("{} agent join --url <this factory's address>", brand::CLI),
    })))
}

/// Lets in whoever hands an invitation over, and answers the agent's
/// credential, once. Asked with no session: the invitation is what lets
/// its bearer in, and whoever came with a credential is not asked for it.
async fn join(State(app): State<App>, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.agents.join(required(&body, "invitation")?).await?)))
}

/// An external agent gives up the credential it asks with.
async fn leave(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    app.agents.unjoin(&who).await?;
    Ok(Json(json!({ "ok": true })))
}
