//! The parts of a line a blueprint told, as whoever's line it is reads
//! them now, and a card such a part lists passed or failed from there:
//! what each comes to is the `told` crate's.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::Value;

use crate::http::ApiResult;
use crate::state::{App, Auth, Said};

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route("/inbox/{id}/parts", get(parts))
        .route("/inbox/{id}/parts/{part}/cards/{card}", post(press))
}

async fn parts(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(told::parts((&app.studio, &app.journal, &app.engine), &who, &id).await?))
}

async fn press(
    State(app): State<App>,
    Said { who, actor }: Said,
    Path((id, part, card)): Path<(String, usize, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let done = told::press((&app.studio, &app.journal), (&who, &actor), (&id, part, &card), &body).await?;
    Ok(Json(done))
}
