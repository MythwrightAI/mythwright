//! The examples that came with the program: which there are, and one put
//! among a project's own blueprints ([`crate::examples`]).

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kernel::Error;
use serde_json::{Value, json};

use crate::examples::{OFFERED, put};
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new().route("/examples", get(listed)).route("/projects/{id}/examples", post(add))
}

/// Which there are, for anybody signed in: they came with the program,
/// and say nothing of this factory.
async fn listed(Auth(_): Auth) -> Json<Value> {
    Json(json!(OFFERED.iter().map(|one| one.listed()).collect::<Vec<_>>()))
}

async fn add(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // Before anything is said of the project: this writes files of it.
    who.require("projects:write")?;
    let name = body
        .get("example")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::invalid("`example` is needed: the name of the one to put there"))?;
    Ok(Json(put(&app.engine, &app.studio, &app.own, &id, name, &who).await?))
}
