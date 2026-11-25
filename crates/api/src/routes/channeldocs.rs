//! The documents a channel keeps to hand (`talk::docs`), as whoever asks
//! may see them (`agents::docs`): listed to whoever reads the channel,
//! added to and taken from by whoever may write in it.
//!
//! Each asks the channel first, as every route of a channel does: one
//! kept from whoever asks answers as a channel that is not there. An
//! entry is added only by somebody who may read what it names, which is
//! asked here, since the channel knows no library and no project's files.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use kernel::{Error, Principal};
use serde::Deserialize;
use serde_json::{Value, json};
use studio::docs::Doc;

use super::text;
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new().route("/channels/{id}/docs", get(listed).post(add).delete(remove))
}

/// The channel's documents as `who` may see them.
async fn seen(app: &App, id: &str, who: &Principal) -> ApiResult<Json<Value>> {
    let reader = app.studio.channels.may_read(id, who).await?;
    let kept = app.studio.channels.docs(&reader, id).await?;
    Ok(Json(json!({ "docs": app.agents.docs_seen(kept, who).await })))
}

async fn listed(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    seen(&app, &id, &who).await
}

/// What a request names: `doc` as it is said, `record`, or `project` and
/// `path`.
fn named(body: &Value) -> Result<Doc, Error> {
    if let Some(doc) = text(body, "doc") {
        return Doc::parse(doc);
    }
    if let Some(record) = text(body, "record") {
        return match record.contains('/') {
            true => Err(Error::invalid(format!("`{record}` is no library's record"))),
            false => Doc::parse(record),
        };
    }
    match (text(body, "project"), text(body, "path")) {
        (Some(project), Some(path)) => Doc::file(project, path),
        _ => Err(Error::invalid(
            "say the document: `record` (a library's, by its id), or `project` and `path` (a project's file)",
        )),
    }
}

/// Adds a document, which whoever asks has to be able to read.
async fn add(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let reader = app.studio.channels.may_write(&id, &who).await?;
    let doc = named(&body)?;
    app.agents.may_add_doc(&doc, &who).await?;
    app.studio.channels.add_doc(&reader, &id, &doc, &who).await?;
    seen(&app, &id, &who).await
}

#[derive(Deserialize)]
struct Taken {
    doc: String,
}

/// Takes an entry away, as it is said: whatever it names now, or nothing.
async fn remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Query(taken): Query<Taken>,
) -> ApiResult<Json<Value>> {
    let reader = app.studio.channels.may_write(&id, &who).await?;
    app.studio.channels.remove_doc(&reader, &id, &taken.doc, &who).await?;
    seen(&app, &id, &who).await
}
