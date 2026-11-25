//! Somebody's inbox: the box of things addressed to them
//! (`talk::inbox`), read, marked read and put away by its owner.
//!
//! No route here takes whose box is meant: it is the box of whoever
//! asks, a person, an agent with its own credential, or a key as its
//! maker where the key itself was given the scope. So a line that is
//! somebody else's is one that is not there, in the same words, for
//! whoever asks and whatever they hold. What a line says of a message
//! is read from the message as whoever asks may read it now
//! (`Channels::inbox_of`).

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kernel::Error;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route("/inbox", get(inbox))
        .route("/inbox/read", post(read_in))
        .route("/inbox/{id}", get(line))
        .route("/inbox/{id}/read", post(read))
        .route("/inbox/{id}/unread", post(unread))
        .route("/inbox/{id}/away", post(away))
}

#[derive(Deserialize)]
struct Asked {
    /// What was put away too.
    #[serde(default)]
    all: bool,
}

/// The lines of whoever asks, the latest first, and how many wait unread.
async fn inbox(
    State(app): State<App>,
    Auth(who): Auth,
    Query(asked): Query<Asked>,
) -> ApiResult<Json<Value>> {
    let (lines, unread) = app.studio.channels.inbox_of(&who, asked.all).await?;
    Ok(Json(json!({ "lines": lines, "unread": unread })))
}

/// One line of whoever asks, put away or not, as they are shown it: for
/// a page of the line itself. Somebody else's answers as one that is
/// not there.
async fn line(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.channels.inbox_line(&who, &id).await?)))
}

/// Says one line that was read waits unread again.
async fn unread(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let inbox = &app.studio.channels.inbox;
    let line = inbox.unread_again(&who, &id).await?;
    Ok(Json(json!({ "line": line, "unread": inbox.unread(&who).await? })))
}

/// Says one line was read.
async fn read(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let inbox = &app.studio.channels.inbox;
    let line = inbox.read(&who, &id).await?;
    Ok(Json(json!({ "line": line, "unread": inbox.unread(&who).await? })))
}

/// Says every line was read, or every line that opens one place
/// (`place`, a channel's id).
async fn read_in(
    State(app): State<App>,
    Auth(who): Auth,
    body: Option<Json<Value>>,
) -> ApiResult<Json<Value>> {
    let body = body.map(|Json(body)| body).unwrap_or_default();
    let place = match body.get("place") {
        None | Some(Value::Null) => None,
        Some(Value::String(place)) if !place.is_empty() => Some(place.as_str()),
        Some(_) => {
            return Err(Error::invalid("`place` is the id of the channel whose lines were read").into());
        }
    };
    let inbox = &app.studio.channels.inbox;
    let read = inbox.read_in(&who, place).await?;
    Ok(Json(json!({ "read": read, "unread": inbox.unread(&who).await? })))
}

/// Puts one line away.
async fn away(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let inbox = &app.studio.channels.inbox;
    let line = inbox.away(&who, &id).await?;
    Ok(Json(json!({ "line": line, "unread": inbox.unread(&who).await? })))
}
