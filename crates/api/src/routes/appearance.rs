//! How what somebody reads looks for them, kept with their account
//! (`people::appearance`): the size of its text, its width, and light or
//! dark.
//!
//! No route here takes whose setting is meant: it is always that of
//! whoever asks, a person, or a key as its maker where the key itself was
//! given the scope.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new().route("/appearance", get(read).put(keep))
}

/// What the asker keeps: `{ "appearance": ..., "nth": ..., "changed": ... }`,
/// the setting nothing where they never kept one.
async fn read(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.appearance.of(&who).await?)))
}

/// Keeps the asker's setting whole: `{ "text": "larger", "width": "wide", "colour": "dark", "contents": "hidden", "typeface": "serif" }`.
async fn keep(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.appearance.keep(&who, &body).await?)))
}
