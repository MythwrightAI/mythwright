//! The Trash over HTTP (`studio::trash`): what is in it for whoever asks,
//! giving something back, deleting it for good, and how long what is
//! deleted is kept, which is the owner's to say. And what empties it, as
//! the daemon starts, once an hour by its clock, and when the owner says
//! how long.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kernel::{Error, Result};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

/// How often what has been in the Trash long enough is looked for.
const EVERY: std::time::Duration = std::time::Duration::from_secs(3600);

/// The Trash of the factory the app is.
pub(crate) fn of(app: &App) -> trash::Trash {
    // What runs for a project deleted for good, and its agents, go with it
    // (`agents::forgotten`), as the system: whoever deleted it was held to
    // what stopping each and taking each away takes, before anything was
    // deleted (`asks`), and the Trash empties itself as the system.
    let (agents, engine) = (app.agents.clone(), app.engine.clone());
    let also: trash::Also = std::sync::Arc::new(move |project: String| {
        let (agents, engine) = (agents.clone(), engine.clone());
        Box::pin(async move {
            agents.forget_project(&project, &engine, &kernel::Principal::system()).await.map(drop)
        })
    });
    let (agents, engine) = (app.agents.clone(), app.engine.clone());
    let asks: trash::Asks = std::sync::Arc::new(move |project: String, by: kernel::Principal| {
        let (agents, engine) = (agents.clone(), engine.clone());
        Box::pin(async move { agents.may_forget_project(&project, &engine, &by).await })
    });
    trash::Trash::of(&app.studio, &app.journal)
        .with_secrets(app.hosting.secrets().ok())
        .with_engine(app.engine.clone())
        .with_also(also, asks)
}

pub fn routes() -> Router<App> {
    Router::new()
        .route("/trash", get(list))
        .route("/trash/{id}", axum::routing::delete(forget))
        .route("/trash/{id}/restore", post(restore))
        .route("/system/trash", get(said).post(say))
}

/// What a route that deleted something answers: what it is in the
/// Trash, and how long it is kept there; nothing of either where it went
/// at once.
pub(crate) async fn moved(app: &App, trash: Option<String>) -> Value {
    let days = of(app).kept().await.days;
    json!({ "ok": true, "trash": trash, "kept": trash.as_ref().map(|_| days) })
}

/// Finishes what a stop left half done, then empties what is due, as the
/// daemon starts and once an hour after that. One that cannot is said once.
pub(crate) async fn keep(app: App) {
    if let Err(err) = of(&app).settle().await {
        degraded(&app, &err).await;
    }
    let mut told = false;
    loop {
        if let Err(err) = empty(&app).await
            && !told
        {
            told = true;
            degraded(&app, &err).await;
        }
        app.journal.clock().sleep(EVERY).await;
    }
}

async fn degraded(app: &App, err: &Error) {
    let effect = "what is in the Trash is kept however long it has been there";
    let data = json!({ "what": "emptying the Trash", "reason": err.message, "effect": effect });
    let _ = app.journal.emit("system.degraded", kernel::Source::actor("system"), data).await;
}

/// Empties what is due, and measures how long that took: quietly, since
/// it is asked whether or not anything is happening.
async fn empty(app: &App) -> Result<usize> {
    let began = std::time::Instant::now();
    let gone = of(app).empty().await;
    let took = began.elapsed().as_secs_f64() * 1000.0;
    app.journal.measure_quietly("trash.emptied", &[], took, gone.is_ok());
    gone
}

async fn list(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    let kept = of(&app).kept().await;
    Ok(Json(json!({ "trash": of(&app).list(&who).await?, "days": kept.days })))
}

/// `{ "beside": true }` gives a file back beside whatever is at its place
/// now, where something is.
async fn restore(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    body: Option<Json<Value>>,
) -> ApiResult<Json<Value>> {
    let beside = body.and_then(|Json(body)| body.get("beside").and_then(Value::as_bool)).unwrap_or(false);
    let (entry, at, said) = of(&app).restore(&id, beside, &who).await?;
    if entry.kind == kernel::trash::Kind::Channel {
        app.agents.channel_back(&at, entry.at).await?;
    }
    let mut answer = json!({ "restored": entry, "at": at });
    if let Some(said) = said {
        answer["said"] = json!(said);
    }
    Ok(Json(answer))
}

async fn forget(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "gone": of(&app).forget(&id, &who).await? })))
}

async fn said(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    Ok(Json(json!(of(&app).kept().await)))
}

/// `{ "days": N | "always" }`. What has been there longer is emptied at
/// once, and the answer says how many went.
async fn say(State(app): State<App>, Auth(who): Auth, Json(asked): Json<Value>) -> ApiResult<Json<Value>> {
    // Before anything is read of what was asked: whose this is to change.
    who.require("admin")?;
    let days = match asked.get("days") {
        Some(Value::String(always)) if always == "always" => None,
        Some(Value::Number(days)) => {
            Some(days.as_u64().map_or(u32::MAX, |days| days.min(u32::MAX.into()) as u32))
        }
        _ => {
            return Err(
                Error::invalid("`days` is a number of days, nought to delete at once, or `always`").into()
            );
        }
    };
    let kept = of(&app).keep_for(days, &who).await?;
    let gone = empty(&app).await?;
    let mut answer = json!(kept);
    answer["gone"] = json!(gone);
    Ok(Json(answer))
}
