//! What is attached to a record, a card included (`studio::attaching`):
//! added by whoever may change the record, listed and read by whoever may
//! read it, and taken away into the Trash by whoever may change it. Its file is read at
//! the address every attachment is (`/attachments/{id}`, in `talk.rs`),
//! which goes by the id.

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::routing::{delete, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route(
            "/records/{id}/attachments",
            post(add)
                .get(list)
                .layer(DefaultBodyLimit::max(studio::attachments::MAX_ATTACHMENT_BYTES + 4096)),
        )
        .route("/records/{id}/attachments/{attachment}", delete(remove))
}

#[derive(Deserialize)]
struct Named {
    name: String,
}

/// Attaches the body, the file as it is, to the record: what kind of
/// thing it is, is what the request says it is, or else what its name says.
async fn add(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Query(named): Query<Named>,
    headers: axum::http::HeaderMap,
    bytes: axum::body::Bytes,
) -> ApiResult<Json<Value>> {
    let media = headers.get(axum::http::header::CONTENT_TYPE).and_then(|value| value.to_str().ok());
    let records = &app.studio.records;
    Ok(Json(json!(records.attach(&id, (&named.name, media), bytes.to_vec(), &who, None).await?)))
}

async fn list(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "attachments": app.studio.records.attachments(&id, &who).await? })))
}

async fn remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, attachment)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    // Into the Trash, with how long it is kept there, as anything deleted is.
    let (gone, trash) = app.studio.records.detach(&id, &attachment, &who, None).await?;
    let mut answer = crate::trash::moved(&app, trash).await;
    answer["attachment"] = json!(gone);
    Ok(Json(answer))
}
