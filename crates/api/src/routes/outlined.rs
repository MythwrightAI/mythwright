//! How a project's things are arranged in the side panel (`outline`):
//! read as whoever asks may see it, and changed one operation at a time
//! by whoever may write the project's files.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use kernel::Error;
use kernel::outline::Op;
use outline::Outlines;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new().route("/projects/{id}/outline", get(read).post(change))
}

fn outlines(app: &App) -> Outlines {
    Outlines::new(app.journal.clone(), app.studio.clone())
}

/// `{ "project": ..., "outline": { "nth", "next", "items": [...] } }`, the
/// outline nothing where nobody arranged the project.
async fn read(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let outline = outlines(&app).read(&id, &who).await?;
    Ok(Json(json!({ "project": id, "outline": outline.map(|outline| outline.to_value()) })))
}

/// One change: `{ "op": "make" | "put" | "rename" | "remove" | "restore" |
/// "forget", ..., "seen": [<what the page drew at the top that the outline
/// does not name, in order>] }`. Answers with the outline as it is now,
/// as the asker may see it, and what was done.
async fn change(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let op = Op::read(&body).map_err(Error::invalid)?;
    let seen = match body.get("seen") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(seen)) => seen.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Some(_) => return Err(Error::invalid("`seen` is a list of the keys of what was drawn.").into()),
    };
    let changed = outlines(&app).change(&id, op, seen, &who).await?;
    Ok(Json(json!({
        "project": id,
        "outline": changed.outline.map(|outline| outline.to_value()),
        "changed": changed.changed,
        "nth": changed.nth,
        "done": changed.done,
    })))
}
