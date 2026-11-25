//! A project's files: listed, read, written, made new, and taken away
//! into the Trash, as text through `file` and as they are through `raw`,
//! and its folders made, by the client and by the command line (`mw
//! file`) alike.

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kernel::Error;
use serde::Deserialize;
use serde_json::{Value, json};
use studio::projects::MAX_RAW_BYTES;

use super::required;
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/projects/{id}/tree", get(tree))
        .route("/projects/{id}/file", get(file_read).put(file_write).delete(file_remove))
        .route("/projects/{id}/folders", post(folder_make))
        // What is written as it is may be a picture, bigger than anything else sent here.
        .route(
            "/projects/{id}/raw",
            get(file_raw).put(file_put).layer(DefaultBodyLimit::max(MAX_RAW_BYTES as usize + 4096)),
        )
}

async fn tree(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    // Less what a table of the project keeps private from whoever asks.
    let mut entries = app.studio.projects.tree(&id).await?;
    entries.retain(|entry| !app.studio.projects.is_kept_from(&id, &entry.path, &who));
    Ok(Json(json!({ "entries": entries })))
}

#[derive(Deserialize)]
struct FileQuery {
    path: String,
    /// Said (`&head`, with anything or nothing after it): the file's entry and nothing of what is in it, for
    /// a page that asks whether a path written in somebody's words is a file it may open (`ui/src/pathsknown.ts`).
    head: Option<String>,
}

async fn file_read(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Query(query): Query<FileQuery>,
) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    app.studio.projects.may_read_path(&id, &query.path, &who).await?;
    if query.head.is_some() {
        return Ok(Json(json!(app.studio.projects.stat(&id, &query.path).await?)));
    }
    let content = app.studio.projects.read(&id, &query.path).await?;
    Ok(Json(json!({ "path": query.path, "content": content })))
}

/// A file's own bytes, for what is not text: a picture is shown from here.
///
/// It is somebody's file and may be anything, so the browser is told not
/// to guess what it is and not to run it: opened by itself, a page or a
/// drawing with a script in it stays inert.
async fn file_raw(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Query(query): Query<FileQuery>,
) -> ApiResult<axum::response::Response> {
    use axum::http::header::{
        CONTENT_SECURITY_POLICY, CONTENT_TYPE, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS,
    };
    use axum::response::IntoResponse;
    who.require("projects:read")?;
    app.studio.projects.may_read_path(&id, &query.path, &who).await?;
    let bytes = app.studio.projects.read_bytes(&id, &query.path).await?;
    let ending = query.path.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    let kind = match ending.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    };
    let headers = [
        (CONTENT_TYPE, kind),
        (X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (CONTENT_SECURITY_POLICY, "sandbox; default-src 'none'; style-src 'unsafe-inline'"),
        // Its address says which page asked (`by`): nothing it leads to is told where it was led from.
        (REFERRER_POLICY, "no-referrer"),
    ];
    Ok((headers, bytes).into_response())
}

async fn file_write(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let content =
        body.get("content").and_then(Value::as_str).ok_or_else(|| Error::invalid("`content` is needed"))?;
    let path = required(&body, "path")?;
    // A file that is to be new is refused where anything is there under its name, and never written over.
    let new = body.get("new").and_then(Value::as_bool).unwrap_or(false);
    Ok(Json(written(&app, &id, path, content.as_bytes(), new, &who).await?))
}

/// An empty folder of a project, made with the folders it is in, for
/// whoever may write the project's files; refused where anything is there
/// under its name, and where its path leads out of the project.
async fn folder_make(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("projects:write")?;
    let path = required(&body, "path")?;
    Ok(Json(json!(app.studio.projects.make_folder(&id, path, &who).await?)))
}

/// A file written as it is, whatever is in it, from the request's own
/// bytes: what is not text (a picture) as well as what is, so that it is
/// on the disk byte for byte. What it is called says what it is, as where
/// it is read back (`file_raw`): nothing sent beside it is believed.
async fn file_put(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Query(query): Query<FileQuery>,
    bytes: Bytes,
) -> ApiResult<Json<Value>> {
    Ok(Json(written(&app, &id, &query.path, &bytes, false, &who).await?))
}

/// What either way of writing a file does, and answers.
async fn written(
    app: &App,
    id: &str,
    path: &str,
    content: &[u8],
    new: bool,
    who: &kernel::Principal,
) -> kernel::Result<Value> {
    who.require("projects:write")?;
    app.studio.projects.may_read_path(id, path, who).await?;
    // A file that is a library's document: what it held is read first, so
    // that what of the document changed can be said of it, by whom. One
    // that is not text is no document's words, and is written and no more.
    let text = std::str::from_utf8(content).ok();
    let library =
        text.is_some() && app.studio.collections.document_at(id, path).await.ok().flatten().is_some();
    let was = if library { app.studio.projects.read(id, path).await.ok() } else { None };
    let written = match new {
        true => app.studio.projects.write_new(id, path, content, who).await?,
        false => app.studio.projects.write(id, path, content, who).await?,
    };
    if let (true, Some(text)) = (library, text) {
        app.studio.records.file_written(id, &written.path, was.as_deref(), text, who).await;
    }
    saved(app, id, &written.path, who).await;
    // Where a file under the name of a text that ships stands, now that it is saved.
    let mut answer = json!(written);
    if let Some(said) = crate::forked::said_of_file(app.knowing(), who, id, &written.path).await {
        answer["stands"] = said;
    }
    Ok(answer)
}

/// Whoever writes or takes away a blueprint of a project's through the
/// daemon has vouched for it as it now is, where they hold what was
/// vouched with (`conductor`'s `vouched`): they could have applied it.
async fn saved(app: &App, project: &str, path: &str, who: &kernel::Principal) {
    for name in crate::library::names_at(&app.own, project, path) {
        app.engine.vouch_saved(project, &name, who).await;
    }
}

async fn file_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Query(query): Query<FileQuery>,
) -> ApiResult<Json<Value>> {
    // Before anything is said of the file: taking one away needs what writing one does.
    who.require("projects:write")?;
    app.studio.projects.may_read_path(&id, &query.path, &who).await?;
    let file = app.studio.projects.stat(&id, &query.path).await.ok().filter(|entry| !entry.dir);
    // Into the Trash, where the factory keeps what is deleted.
    let trash = app.studio.projects.delete(&id, &query.path, &who).await?;
    // Deleted at once, a document or a mockup takes what is attached to it with it, as from the Trash.
    if trash.is_none() {
        app.studio.collections.forget_attached_at(&id, &query.path).await?;
    }
    if let Some(file) = file {
        app.studio.records.file_removed(&id, &file.path, &who).await;
        saved(&app, &id, &file.path, &who).await;
    }
    Ok(Json(crate::trash::moved(&app, trash).await))
}
