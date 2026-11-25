//! The factory's bot, read about: what it did lately, and what it may ask.

use axum::extract::{Query, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new().route("/system/speaker/history", get(history)).route("/system/speaker/holds", put(hold))
}

#[derive(Deserialize)]
struct Asked {
    limit: Option<usize>,
}

/// What the factory's bot did lately, for whoever reads about it, newest
/// first and `limit` at most (fifty unless said, two hundred at most):
/// each line of a record's history that a blueprint's step wrote, which
/// is the bot's doing, with the blueprint it was done through; and each
/// thing it said in a channel, with the blueprint that had it say so.
/// Each is held to what whoever asks may read as that record's history
/// and that channel are: nothing of a record or a channel kept from them
/// is handed, nor said to be there.
async fn history(
    State(app): State<App>,
    Auth(who): Auth,
    Query(asked): Query<Asked>,
) -> ApiResult<Json<Value>> {
    let most = asked.limit.unwrap_or(50).clamp(1, 200);
    let done = app.studio.collections.done_through(&who, most).await?;
    let said = app.studio.channels.said_by_the_bot(&who, most).await?;
    let mut lines: Vec<Value> = done
        .into_iter()
        .map(|done| {
            json!({ "kind": "record", "ts": done.line.ts, "record": done.record, "project": done.project,
                    "collection": done.collection, "topic": done.line.topic, "said": done.line.said,
                    "via": kernel::worded::through(&done.line.data) })
        })
        .chain(said.into_iter().map(|message| {
            json!({ "kind": "said", "ts": message.created, "channel": message.channel, "message": message.id,
                    "thread": message.thread, "said": message.body,
                    "via": message.said_by.map(|by| by.blueprint) })
        }))
        .collect();
    lines.sort_by_key(|line| std::cmp::Reverse(line["ts"].as_i64().unwrap_or(0)));
    lines.truncate(most);
    Ok(Json(json!({ "name": app.studio.channels.speaker.name(), "done": lines })))
}

/// Narrows what the factory's bot may ask to `holds`, a list of scopes, or
/// gives it everything its commands reach again with `null`: by whoever
/// manages the factory, giving only what they hold. What it is asked is
/// then done as whoever asked, held to no more than that.
async fn hold(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let holds = match &body["holds"] {
        Value::Null => None,
        Value::Array(scopes) => Some(
            scopes
                .iter()
                .map(|scope| scope.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    kernel::Error::invalid("`holds` is a list of scopes, or null for the usual")
                })?,
        ),
        _ => return Err(kernel::Error::invalid("`holds` is a list of scopes, or null for the usual").into()),
    };
    let holds = app.studio.channels.hold_speaker(holds, &who).await?;
    Ok(Json(json!({ "name": app.studio.channels.speaker.name(), "holds": holds })))
}
