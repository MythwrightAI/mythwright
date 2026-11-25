//! The sign-ins the factory holds for its agents' programs.
//!
//! All of it is the owner's: a sign-in is somebody's account with a
//! provider, or a key that is paid for by use. A value comes in once, in
//! the body of the request that adds it, and is never given out again:
//! what is listed is names, who holds which, and what is set aside.

use agents::NewSignIn;
use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use super::{required, text};
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/signins", get(signins).post(signin_add))
        .route("/signins/{name}", delete(signin_remove))
        .route("/signins/{name}/restore", post(signin_restore))
}

async fn signins(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "signins": app.agents.signins(&who)? })))
}

async fn signin_add(
    State(app): State<App>,
    Auth(who): Auth,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // Before anything is read of what was sent: who may send it.
    who.require("admin")?;
    let new = NewSignIn {
        name: required(&body, "name")?.to_string(),
        package: required(&body, "for")?.to_string(),
        kind: required(&body, "kind")?.to_string(),
        value: text(&body, "value").map(str::to_string),
        folder: text(&body, "folder").map(str::to_string),
        at_once: body.get("at_once").and_then(Value::as_u64).map(|most| most.min(u64::from(u32::MAX)) as u32),
    };
    Ok(Json(json!(app.agents.signin_add(new, &who).await?)))
}

async fn signin_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    app.agents.signin_remove(&name, &who).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn signin_restore(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    app.agents.signin_restore(&name, &who).await?;
    Ok(Json(json!({ "ok": true })))
}
