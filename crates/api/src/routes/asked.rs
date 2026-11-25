//! A change of a record asked for by whoever may not make it, and decided
//! by a person who could (`told::asks`, `docs/approvals.md`), and a run
//! that waits for a person to let it go on, decided at the same door
//! (`told::goes`): each route a call of it as whoever asks, with nothing
//! decided here but where an agent asked from.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kernel::{Error, PrincipalKind};
use serde_json::{Map, Value, json};
use told::{Asks, Came};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route("/asked", get(listed).post(ask))
        .route("/asked/{id}", get(shown))
        .route("/asked/{id}/approve", post(approve))
        .route("/asked/{id}/decline", post(decline))
        .route("/asked/{id}/withdraw", post(withdraw))
}

fn asks(app: &App) -> Asks {
    Asks::new(&app.journal, &app.studio).reading(&app.engine)
}

fn text<'a>(body: &'a Value, key: &str) -> Option<&'a str> {
    body.get(key).and_then(Value::as_str).filter(|text| !text.trim().is_empty())
}

/// The asks whoever asks made, and those waiting that they could decide;
/// or, said `run`, that run's asks to be let go on, as they are shown to
/// whoever asks, for the run's page.
async fn listed(
    State(app): State<App>,
    Auth(who): Auth,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    if let Some(run) = query.get("run") {
        return Ok(Json(json!({ "run": run, "asked": asks(&app).of_run(&who, run).await? })));
    }
    Ok(Json(json!({ "asked": asks(&app).listed(&who).await? })))
}

/// One ask, as whoever asks is shown it.
async fn shown(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(asks(&app).shown(&who, &id).await?))
}

/// Asks for `set` of `record`. Made at once where whoever asks may make
/// it; kept and told otherwise, where only scopes are lacking. Where it
/// was asked from is the channel (and thread) said, or, for an agent that
/// says none, the one its turn in hand is in and the thread it was asked
/// under there.
async fn ask(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let record = text(&body, "record").ok_or_else(|| Error::invalid("say which record, as `record`"))?;
    let set: Map<String, Value> = match body.get("set") {
        Some(Value::Object(set)) if !set.is_empty() => set.clone(),
        _ => return Err(Error::invalid("say what is to be set, as `set`: a field and its value each").into()),
    };
    let channel = match (text(&body, "channel"), &who.kind) {
        (Some(channel), _) => Some(channel.to_string()),
        (None, PrincipalKind::Agent) => app.agents.asking_in(&who.id).await,
        (None, _) => None,
    };
    let mut place = json!({});
    if let Some(channel) = channel {
        let reader = app.studio.channels.may_write(&channel, &who).await?;
        let thread = match (text(&body, "thread"), &who.kind) {
            (Some(thread), _) => Some(thread.to_string()),
            (None, PrincipalKind::Agent) => app.agents.asked_under(&reader, &who.id, &channel).await,
            (None, _) => None,
        };
        let found = app.studio.channels.get(&reader, &channel).await?;
        place = json!({ "channel": found.id, "thread": thread });
    }
    let why = text(&body, "why").unwrap_or_default();
    let came = asks(&app).ask(&who, record, set, why, text(&body, "to"), place).await?;
    Ok(Json(match came {
        Came::Nothing(record) => json!({ "came": "nothing", "record": record }),
        Came::Made(record) => json!({ "came": "made", "record": record }),
        Came::Asked(ask) => {
            json!({ "came": "asked", "ask": asks(&app).shown(&who, &ask.id).await?, "to": ask.named })
        }
    }))
}

async fn approve(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    asks(&app).decide(&who, &id, true, "").await?;
    Ok(Json(asks(&app).shown(&who, &id).await?))
}

/// Declined, with why where it is said.
async fn decline(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    body: Option<Json<Value>>,
) -> ApiResult<Json<Value>> {
    let body = body.map(|Json(body)| body).unwrap_or_default();
    asks(&app).decide(&who, &id, false, text(&body, "why").unwrap_or_default()).await?;
    Ok(Json(asks(&app).shown(&who, &id).await?))
}

/// Taken back by whoever asked for it.
async fn withdraw(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    asks(&app).withdraw(&who, &id).await?;
    Ok(Json(asks(&app).shown(&who, &id).await?))
}
