//! The setups a factory keeps (`setup::setups`): rows a new project may
//! be set up with, saved from the dialog that adds one under a name, or
//! as the factory's default for new projects (`default`).
//!
//! Read by whoever may read projects. Saved and taken away by whoever
//! may add projects and apply a blueprint, and the default by whoever
//! manages the factory: each a list of blueprints applied as whoever adds
//! a project with it, which gives nobody more than they hold.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use kernel::Error;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::setups::{self, DEFAULT};
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new().route("/setups", get(list)).route("/setups/{name}", get(one).put(save).delete(remove))
}

/// Every setup the factory keeps (`setups`), what ships as the usual
/// (`ships`), and what a project is set up with where nothing else is
/// said (`usual`): the factory's default, or what ships.
async fn list(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    let kept = setups::saved(app.engine.journal()).await?;
    let ships = crate::usual::shipped(&app.engine);
    let usual = crate::usual::the_usual(&app.engine).await?;
    Ok(Json(json!({
        "setups": kept.iter().map(setups::Setup::value).collect::<Vec<_>>(),
        "ships": crate::usual::listed(&ships),
        "usual": crate::usual::listed(&usual),
    })))
}

/// The setup called `name`: `default` is the factory's default whether
/// it keeps one or not.
async fn one(State(app): State<App>, Auth(who): Auth, Path(name): Path<String>) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    let rows = crate::usual::of_setup(&app.engine, &name).await?;
    Ok(Json(json!({ "name": name, "rows": crate::usual::listed(&rows), "default": name == DEFAULT })))
}

/// Keeps `{ "rows": [...] }` as the setup called `name`.
async fn save(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let rows = body.get("rows").ok_or_else(|| {
        Error::invalid("`rows` is needed: what a project set up with it starts with, a row for each thing")
    })?;
    Ok(Json(setups::save(app.engine.journal(), &name, rows, &who).await?.value()))
}

/// Takes the setup called `name` away; the default goes back to what ships.
async fn remove(State(app): State<App>, Auth(who): Auth, Path(name): Path<String>) -> ApiResult<Json<Value>> {
    setups::remove(app.engine.journal(), &name, &who).await?;
    Ok(Json(json!({ "removed": name })))
}
