//! How long the detail of runs' steps is kept, and where their output is
//! written: two settings of the factory, said by its owner here and kept
//! by the journal (`journal::letgo`); and what is older than that let go
//! of, by the runner (`conductor::letgo`), as the daemon starts, once an
//! hour, and when the owner says how long. What comparisons count from
//! is asked of them and handed in, since the runner knows none.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use kernel::{Error, Result};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

/// How often what is older than the factory keeps is looked for.
const EVERY: std::time::Duration = std::time::Duration::from_secs(3600);

pub fn routes() -> Router<App> {
    Router::new().route("/system/logs", get(said).post(say))
}

/// Lets go of what is older than the factory keeps, as it starts and
/// then once an hour, by its clock. One that cannot is said once.
pub(crate) async fn keep(app: App) {
    let mut told = false;
    loop {
        if let Err(err) = pass(&app).await
            && !told
        {
            told = true;
            let effect = "what runs' steps printed is kept however old it is";
            let data = json!({ "what": "letting old logs go", "reason": err.message, "effect": effect });
            let _ = app.journal.emit("system.degraded", kernel::Source::actor("system"), data).await;
        }
        app.journal.clock().sleep(EVERY).await;
    }
}

async fn pass(app: &App) -> Result<Option<conductor::letgo::LetGo>> {
    let counted = app.comparisons.counted_from(conductor::letgo::TOPICS).await;
    app.engine.let_logs_go(&counted).await
}

/// The settings as they are said to the owner.
async fn shown(app: &App) -> Value {
    let (kept, written) = (app.engine.logs_kept().await, app.engine.logs_written().await);
    let usual = app.place.data.join("logs").to_string_lossy().into_owned();
    // Who said last, of either.
    let (by, at) = match written.at > kept.at {
        true => (written.by, written.at),
        false => (kept.by, kept.at),
    };
    json!({
        "days": kept.days, "went": kept.went, "folder": written.folder, "before": written.before,
        "usual": usual, "by": by, "at": at,
    })
}

async fn said(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    Ok(Json(shown(&app).await))
}

/// `{ "days": N | "always", "folder": "<folder>" | "usual" }`, each only
/// where it is to change. Said how long, what is older is let go of at
/// once, and the answer says what went.
async fn say(State(app): State<App>, Auth(who): Auth, Json(asked): Json<Value>) -> ApiResult<Json<Value>> {
    // Before anything is read of what was asked: whose this is to change.
    who.require("admin")?;
    let days = match asked.get("days") {
        None => None,
        Some(Value::String(always)) if always == "always" => Some(None),
        Some(Value::Number(days)) => {
            Some(Some(days.as_u64().map_or(0, |days| days.min(u32::MAX.into()) as u32)))
        }
        Some(_) => return Err(Error::invalid("`days` is a number of days, or `always`").into()),
    };
    let folder = match asked.get("folder") {
        None => None,
        Some(Value::String(usual)) if usual == "usual" => Some(None),
        Some(Value::String(folder)) => Some(Some(folder.as_str())),
        Some(_) => return Err(Error::invalid("`folder` is a folder, said whole, or `usual`").into()),
    };
    if let Some(folder) = folder {
        app.engine.write_logs_in(folder, &who).await?;
    }
    let mut let_go = None;
    if let Some(days) = days {
        app.engine.keep_logs_for(days, &who).await?;
        let_go = pass(&app).await?;
    }
    let mut answer = shown(&app).await;
    answer["let_go"] = json!(let_go);
    Ok(Json(answer))
}
