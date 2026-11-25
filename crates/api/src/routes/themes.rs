//! The themes somebody makes for the client, kept with their account
//! (`people::themes`), and those offered to everybody.
//!
//! No route here takes whose themes are meant: they are always those of
//! whoever asks, a person, or a key as its maker where the key itself
//! was given the scope. So a theme that is somebody else's is one that
//! is not there, in the same words, whoever asks and whatever they hold.
//! Offering one to everybody and withdrawing one take `admin` beside
//! what changing one's own takes.

use axum::extract::{Path, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use kernel::Error;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route("/themes", get(listed).post(make))
        .route("/themes/chosen", put(choose))
        .route("/themes/{id}", put(change).delete(take_away))
        .route("/themes/{id}/offer", post(offer).delete(withdraw))
}

/// What a theme is called, its colours, its light version and its look,
/// as a request says them: what is not said is checked as nothing, so
/// that it is refused in the sentence for it, and a light version or a
/// look not said, or said as nothing, is none.
fn said(body: &Value) -> (&str, &Value, Option<&Value>, Option<&Value>) {
    (
        body.get("name").and_then(Value::as_str).unwrap_or_default(),
        body.get("colours").unwrap_or(&Value::Null),
        body.get("light").filter(|light| !light.is_null()),
        body.get("look").filter(|look| !look.is_null()),
    )
}

/// The asker's own themes, those offered to everybody that are not
/// theirs, and the one they chose.
async fn listed(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.themes.of(&who).await?)))
}

/// Makes one: `{ "name": ..., "colours": { "ground": "#rrggbb", ... } }`,
/// with `"light": { ... }` beside them where it has a light version, and
/// `"look": { "size": "larger", ... }` where it says how the client looks.
async fn make(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let (name, colours, light, look) = said(&body);
    Ok(Json(json!(app.studio.themes.make(&who, name, colours, light, look).await?)))
}

/// Changes one of the asker's own, whole: its name, its colours, its
/// light version and its look, each of the last two none where it is not
/// said.
async fn change(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let (name, colours, light, look) = said(&body);
    Ok(Json(json!(app.studio.themes.change(&who, &id, name, colours, light, look).await?)))
}

async fn take_away(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    app.studio.themes.take_away(&who, &id).await?;
    Ok(Json(json!({ "gone": id })))
}

/// Says which theme the asker has chosen: `{ "theme": <id> }`, or
/// `{ "theme": null }` for the usual one.
async fn choose(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let theme = match body.get("theme") {
        None | Some(Value::Null) => None,
        Some(Value::String(theme)) if !theme.is_empty() => Some(theme.as_str()),
        Some(_) => {
            return Err(Error::invalid("`theme` is the id of a theme, or nothing for the usual one").into());
        }
    };
    Ok(Json(json!({ "chosen": app.studio.themes.choose(&who, theme).await? })))
}

async fn offer(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.themes.offer(&who, &id).await?)))
}

async fn withdraw(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.themes.withdraw(&who, &id).await?)))
}
