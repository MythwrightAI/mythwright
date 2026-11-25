//! What every page of a factory says across its top where it is not the
//! factory somebody would take it for (`folders::across`): what
//! it says, and whoever manages it saying it may stop, or what it is to
//! say instead. Every page follows `system.across` and reads nothing again.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use kernel::Source;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};
use folders::across;

pub(super) fn routes() -> Router<App> {
    Router::new().route("/system/across", get(read).post(change))
}

/// What it says, as `{ "across": ... }`, nothing where it says nothing.
async fn read(State(app): State<App>, Auth(_who): Auth) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "across": across::read(&app.journal, app.place.profile.as_deref()).await })))
}

/// Whoever manages the factory says it may stop (`{ "quiet": true }`), that
/// it is to say it again (`false`), or what it is to say instead (`says`,
/// nothing for what it would say). Said to every page as `system.across`.
async fn change(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    let mut kept = across::kept(&app.journal).await;
    if let Some(quiet) = body.get("quiet").and_then(Value::as_bool) {
        kept.quiet = quiet;
    }
    if let Some(says) = body.get("says") {
        kept.says = across::told(says.as_str().unwrap_or_default())?;
        // Told something new to say, it says it, though it was told it may stop.
        kept.quiet = kept.quiet && body.get("quiet").is_some();
    }
    across::keep(&app.journal, &kept).await?;
    let now = across::read(&app.journal, app.place.profile.as_deref()).await;
    let data = json!({ "across": now, "quiet": kept.quiet });
    app.journal.emit("system.across", Source::actor(who.actor()), data.clone()).await?;
    Ok(Json(data))
}
