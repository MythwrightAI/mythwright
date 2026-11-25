//! The notes of each release that came with the program
//! ([`crate::releases`]).

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::releases::{NOTES, page, releases};
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new().route("/releases", get(told))
}

/// Which releases there are, the newest first, and the notes as a page:
/// for anybody signed in. They came with the program, and say nothing of
/// this factory.
async fn told(Auth(_): Auth) -> Json<Value> {
    let listed: Vec<Value> =
        releases(NOTES).into_iter().map(|one| json!({ "title": one.title, "at": one.at })).collect();
    Json(json!({ "releases": listed, "html": page(NOTES) }))
}
