//! How a time is written for somebody, kept with their account
//! (`people::clocked`): the zone and a clock of twelve hours or
//! twenty-four.
//!
//! No route here takes whose setting is meant: it is always that of
//! whoever asks, a person, or a key as its maker where the key itself was
//! given the scope. The daemon holds a zone to the form a zone's name has
//! and looks nothing up: which zones there are is the client's to know.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new().route("/clock", get(read).put(keep)).route("/clock/first", post(first))
}

/// What the asker keeps: `{ "clock": ..., "nth": ..., "changed": ... }`,
/// the setting nothing where they never kept one.
async fn read(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.clocked.of(&who).await?)))
}

/// Keeps the asker's setting whole: `{ "zone": "America/Denver", "hours": "12h" }`.
async fn keep(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.clocked.keep(&who, &body, false).await?)))
}

/// Keeps it only where the asker keeps none yet, which is how the zone a
/// browser says it is in is taken up: answered with what is kept either way.
async fn first(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.clocked.keep(&who, &body, true).await?)))
}
