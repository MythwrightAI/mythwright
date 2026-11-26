//! Mockups, served: the routes, each a call of `served::mockups`, which
//! came out of here when this crate was at its ceiling and says what a
//! way in is and what everything under one is sent with.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::http::header::HOST;
use axum::response::{Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use kernel::Error;
use serde_json::{Value, json};
use served::mockups::{mockup, way_into};

use crate::http::ApiResult;
use crate::state::{App, Auth};
use crate::through::Opened;

/// What is asked of the API about mockups.
pub fn api() -> Router<App> {
    Router::new().route("/mockups/way", post(way))
}

pub fn routes() -> Router<App> {
    Router::new()
        .route("/mockups/{id}", get(address))
        .route("/mockups/in/{name}", get(to_page))
        .route("/mockups/in/{name}/", get(page))
        .route("/mockups/in/{name}/{*path}", get(file))
}

/// Gives whoever may read a mockup a way in to its files, for showing it.
async fn way(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let id = body["id"].as_str().ok_or_else(|| Error::invalid("say which mockup, as `id`"))?;
    let (collection, folder) = mockup(&app.studio, &who, id).await?;
    let (name, until) = app.ways.open(&collection.project, &folder, app.journal.now_ms());
    Ok(Json(json!({ "path": way_into(&name), "address": format!("/mockups/{id}"), "expires": until })))
}

/// A mockup's own address: for whoever is signed in and may read it, the
/// way in to it. Opened in a browser, it is asked as a library's pages are
/// (`crate::through`).
async fn address(State(app): State<App>, opened: Opened, Path(id): Path<String>) -> ApiResult<Redirect> {
    let (collection, folder) = mockup(&app.studio, &opened.who, &id).await?;
    let (name, _) = app.ways.open(&collection.project, &folder, app.journal.now_ms());
    Ok(Redirect::to(&way_into(&name)))
}

/// A way in is a folder's address: what its page links to is beside it.
async fn to_page(Path(name): Path<String>) -> Redirect {
    Redirect::temporary(&way_into(&name))
}

async fn page(State(app): State<App>, headers: HeaderMap, Path(name): Path<String>) -> ApiResult<Response> {
    serve(&app, &headers, &name, "").await
}

async fn file(
    State(app): State<App>,
    headers: HeaderMap,
    Path((name, path)): Path<(String, String)>,
) -> ApiResult<Response> {
    serve(&app, &headers, &name, &path).await
}

async fn serve(app: &App, headers: &HeaderMap, name: &str, path: &str) -> ApiResult<Response> {
    let host = headers.get(HOST).and_then(|host| host.to_str().ok());
    Ok(served::mockups::serve(&app.studio, &app.ways, app.journal.now_ms(), host, name, path).await?)
}
