//! How somebody arranged the client's side panel for themselves, kept
//! with their account (`people::arranged`): their groups of channels,
//! the channels pinned to the top and what is pinned in Resources.
//!
//! No route here takes whose arrangement is meant: it is always that of
//! whoever asks, a person, or a key as its maker where the key itself was
//! given the scope. The daemon keeps what it is handed and acts on none
//! of it.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new().route("/arranged", get(read).put(keep)).route("/arranged/first", post(first))
}

/// What the asker keeps: `{ "arrangement": ..., "nth": ..., "changed": ... }`,
/// the arrangement nothing where they never kept one.
async fn read(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.arranged.of(&who).await?)))
}

/// Keeps the asker's arrangement whole:
/// `{ "groups": [{ "id", "name", "channels": [...] }], "pinned": [...], "things": [...] }`.
async fn keep(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.arranged.keep(&who, &body, false).await?)))
}

/// Keeps it only where the asker keeps none yet, which is how what a
/// browser kept before is taken up: answered with what is kept either way.
async fn first(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.arranged.keep(&who, &body, true).await?)))
}
