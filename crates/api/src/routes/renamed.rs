//! A channel's details changed: called something else (`talk::renamed`),
//! an open one by its name and one kept to its members by its title, and
//! what it says it is for (`talk::managed`); and who manages it, which is
//! who may do either. Each is asked of whoever manages it
//! (`talk::managed::may_manage`) beside its door.
//!
//! It asks the channel first, as every route of a channel does: one kept
//! from whoever asks answers as a channel that is not there.

use axum::Json;
use axum::extract::{Path, State};
use conductor::{InstanceFilter, InstanceInfo};
use kernel::Error;
use serde_json::{Value, json};

use super::{required, text};
use crate::http::ApiResult;
use crate::state::{App, Auth};

/// Calls a channel what `name` says, and has it say it is for what
/// `topic` says, either or both, as whoever manages it. A name is refused
/// while something that is running names it by what it is called, and
/// while an agent is asked something there (`Agents::rename_channel`).
pub(super) async fn rename(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let (to, topic) = (text(&body, "name"), body.get("topic").and_then(Value::as_str));
    let (reader, channel) = app.studio.channels.writable(&id, &who).await?;
    studio::managed::may_manage(&channel, &who)?;
    let Some(to) = to else {
        let Some(topic) = topic else {
            return Err(Error::invalid(
                "say what the channel is to be called (`name`), or what it is for (`topic`)",
            )
            .into());
        };
        return Ok(Json(json!(app.studio.channels.describe(&reader, &channel.id, topic, &who).await?)));
    };
    if channel.archived.is_some() {
        return Err(studio::renamed::unarchive_first(&channel).into());
    }
    if studio::renamed::renaming(&channel, to)?.is_some() {
        let filter = InstanceFilter { live_only: true, limit: 10_000, ..InstanceFilter::default() };
        let live = app.engine.instances(filter).await?;
        let naming = named_by_name(&channel, &live);
        if let Some(first) = naming.first() {
            let named: Vec<String> =
                naming.iter().map(|one| format!("`{}` ({}, {})", one.key, one.blueprint, one.id)).collect();
            return Err(Error::conflict(format!(
                "{} is where {} {}, and {} running: it finds the channel by what it is called, and would make \
                 another of that name. Stop it first (`{cli} stop {}`), then rename the channel",
                studio::door::called(&channel),
                named.join(" and "),
                if naming.len() == 1 { "posts" } else { "post" },
                if naming.len() == 1 { "it is" } else { "they are" },
                first.id,
                cli = brand::CLI,
            ))
            .into());
        }
    }
    let renamed = app.agents.rename_channel(&channel.id, to, &who).await?;
    match topic {
        Some(topic) => {
            Ok(Json(json!(app.studio.channels.describe(&reader, &renamed.id, topic, &who).await?)))
        }
        None => Ok(Json(json!(renamed))),
    }
}

/// Makes somebody in a channel one of its managers (`manages: true`) or
/// no longer one, as whoever manages it.
pub(super) async fn managers(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let member = required(&body, "member")?;
    let Some(manages) = body.get("manages").and_then(Value::as_bool) else {
        return Err(Error::invalid("say whether they are to manage it (`manages`: true or false)").into());
    };
    let reader = app.studio.channels.may_write(&id, &who).await?;
    Ok(Json(json!(app.studio.channels.manage(&reader, &id, member, manages, &who).await?)))
}

/// What says it posts in `channel`, as `posts_in` finds it, naming it by
/// what it is called and not by its id: an open one by its name, and one
/// kept to its members by what its title comes to, which is how making
/// sure of a kept channel finds it again (`Channels::ensure_as`).
fn named_by_name<'a>(channel: &studio::Channel, live: &'a [InstanceInfo]) -> Vec<&'a InstanceInfo> {
    let by_name = |said: Option<&Value>| {
        said.and_then(Value::as_str).is_some_and(|said| {
            let said = said.trim_start_matches('#');
            match (channel.kept, &channel.title) {
                _ if said == channel.id => false,
                (true, Some(title)) => blueprint::expr::slug(title) == blueprint::expr::slug(said),
                _ => said == channel.name,
            }
        })
    };
    let names = |one: &InstanceInfo| by_name(one.params.get("channel")) || by_name(one.state.get("channel"));
    let found: Vec<&InstanceInfo> = live.iter().filter(|one| names(one)).collect();
    // Of an instance and what it started, the one at the top: stopping that stops the rest.
    let under = |one: &InstanceInfo| {
        one.parent.as_ref().is_some_and(|parent| found.iter().any(|other| &other.id == parent))
    };
    found.iter().copied().filter(|one| !under(one)).collect()
}
