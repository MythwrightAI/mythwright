//! What is being built of this program itself and may be turned on before
//! it is whole: Settings, Experiments. The list is the
//! program's (`kernel::experiments::LIST`), each naming the card it is
//! for, and which are on is a setting of the factory, kept in its journal
//! and changed with what is said of it as one write. Reading it is for
//! anybody signed in, since a page goes by it; turning one on or off is
//! for whoever manages the factory.

use axum::extract::{Path, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use kernel::experiments::{self, LIST, NAMESPACE, ON};
use kernel::{Error, Source};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new().route("/experiments", get(listed)).route("/experiments/{name}", put(turn))
}

/// The names of those turned on.
pub(super) async fn on(app: &App) -> kernel::Result<Vec<String>> {
    Ok(experiments::read(app.journal.kv_get(NAMESPACE, ON).await?.as_ref()))
}

async fn listed(State(app): State<App>, Auth(_who): Auth) -> ApiResult<Json<Value>> {
    let on = on(&app).await?;
    Ok(Json(json!({ "experiments": experiments::shown(LIST, &on) })))
}

/// Turns one on or off (`on`), as whoever manages the factory.
async fn turn(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    let turn =
        body.get("on").and_then(Value::as_bool).ok_or_else(|| Error::invalid("`on` is true or false"))?;
    let kept = experiments::turned(LIST, &on(&app).await?, &name, turn).map_err(Error::not_found)?;
    let said = json!({ "name": name, "on": turn, "by": who.actor(), "by_name": who.name });
    let value = json!(kept);
    let source = Source::actor(who.actor());
    app.journal
        .write(move |writing| {
            writing.keep(NAMESPACE, ON, &value)?;
            writing.say("system.experiment", source, said)
        })
        .await?;
    Ok(Json(json!({ "experiments": experiments::shown(LIST, &kept) })))
}
