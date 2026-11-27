//! A library's documents, followed where they are and served as pages:
//! the routes, each a call of `served::library`, which came out of here
//! when this crate was at its ceiling.

use axum::Router;
use axum::extract::{Path, RawQuery, State};
use axum::response::{Redirect, Response};
use axum::routing::get;

use crate::http::ApiResult;
use crate::state::App;
use crate::through::Opened;

/// Follows the folders of every library for as long as the daemon runs.
pub async fn follow(app: App) {
    served::library::follow(&app.studio, &app.journal, app.hosting.watch()).await;
}

pub fn routes() -> Router<App> {
    Router::new()
        .route("/library/{project}/{name}", get(to_front))
        .route("/library/{project}/{name}/", get(front))
        .route("/library/{project}/{name}/{*path}", get(page))
}

/// A library's address is a folder's: what its pages link to is beside it.
async fn to_front(Path((project, name)): Path<(String, String)>, RawQuery(query): RawQuery) -> Redirect {
    served::library::to_front(&project, &name, query.as_deref())
}

async fn front(
    State(app): State<App>,
    opened: Opened,
    Path((project, name)): Path<(String, String)>,
) -> ApiResult<Response> {
    Ok(served::library::front(&app.studio, &opened.who, &project, &name, &opened.said).await?)
}

async fn page(
    State(app): State<App>,
    opened: Opened,
    Path((project, name, path)): Path<(String, String, String)>,
) -> ApiResult<Response> {
    Ok(served::library::page(&app.studio, &opened.who, &project, &name, &path, &opened.said).await?)
}
