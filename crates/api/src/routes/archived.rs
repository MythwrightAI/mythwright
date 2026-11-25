//! A channel archived and given back (`talk::archived`): by whoever may
//! write in it, which for one kept to its members is one of them or the
//! factory's owner, as its door says.
//!
//! Each asks the channel first, as every route of a channel does: one
//! kept from whoever asks answers as a channel that is not there.

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use conductor::InstanceFilter;
use kernel::Error;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route("/channels/{id}/archive", post(archive))
        .route("/channels/{id}/unarchive", post(unarchive))
}

/// Archives a channel. Refused while something that is running says it
/// posts there, as taking one away is asked about, and while an agent is
/// asked something there: what either says there next would be refused.
async fn archive(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let reader = app.studio.channels.may_write(&id, &who).await?;
    let channel = app.studio.channels.get(&reader, &id).await?;
    if channel.archived.is_none() {
        let filter = InstanceFilter { live_only: true, limit: 10_000, ..InstanceFilter::default() };
        let live = app.engine.instances(filter).await?;
        let posting = super::talk::posts_in(&channel, &live);
        if let Some(first) = posting.first() {
            let named: Vec<String> =
                posting.iter().map(|one| format!("`{}` ({}, {})", one.key, one.blueprint, one.id)).collect();
            return Err(Error::conflict(format!(
                "{} is where {} {}, and {} running, and nothing is said in an archived channel. Stop it first \
                 (`{cli} stop {}`), then archive the channel",
                studio::door::called(&channel),
                named.join(" and "),
                if posting.len() == 1 { "posts" } else { "post" },
                if posting.len() == 1 { "it is" } else { "they are" },
                first.id,
                cli = brand::CLI,
            ))
            .into());
        }
    }
    // And while no agent is asked anything there, under the turn at giving agents work.
    Ok(Json(json!(app.agents.archive_channel(&channel.id, &who).await?)))
}

/// Gives an archived channel back, as it was.
async fn unarchive(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let reader = app.studio.channels.may_write(&id, &who).await?;
    let found = reader.channel().unwrap_or(&id).to_string();
    Ok(Json(json!(app.studio.channels.unarchive(&reader, &found, &who).await?)))
}
