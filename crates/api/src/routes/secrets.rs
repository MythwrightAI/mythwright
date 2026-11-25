//! The secrets the factory keeps for the programs blueprints run.
//!
//! A value comes in once, in the body of the request that sets it, and is
//! never given out again by any route: what is listed is names. Setting
//! and taking away are the owner's. Seeing which there are is for whoever
//! may use them, since a blueprint that names one has to be written by
//! somebody who knows it is there.
//!
//! The owner may also ask where each is named (`?named=true`): which
//! agents are handed it, which services are started with it, which
//! running instances' texts name it. That is what says what would break
//! before one is taken away, and it is the owner's to see: it says who
//! holds which key.

use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use conductor::InstanceFilter;
use serde::Deserialize;
use serde_json::{Value, json};

use super::required;
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/secrets", get(secrets))
        .route("/secrets/{name}", put(secret_set).delete(secret_remove))
}

#[derive(Deserialize)]
struct Asked {
    /// Each with where it is named, and who set it: the owner's to ask.
    #[serde(default)]
    named: bool,
}

async fn secrets(
    State(app): State<App>,
    Auth(who): Auth,
    Query(asked): Query<Asked>,
) -> ApiResult<Json<Value>> {
    who.require("secrets:use")?;
    let kept = app.hosting.secrets()?.list();
    if !asked.named {
        return Ok(Json(json!({ "secrets": kept })));
    }
    who.require("admin")?;
    let mut named = named(&app).await?;
    let by = set_by(&app).await;
    let mut listed: Vec<Value> = kept
        .iter()
        .map(|one| {
            json!({
                "name": one.name,
                "set": one.set,
                "set_by": by.get(&one.name),
                "kept": true,
                "named": named.remove(&one.name).unwrap_or_default().said(),
            })
        })
        .collect();
    // What is named somewhere and kept nowhere: a program that wants it
    // will not start, and whoever looks should see that before it does.
    listed.extend(named.into_iter().map(|(name, places)| {
        json!({ "name": name, "set": Value::Null, "set_by": Value::Null, "kept": false, "named": places.said() })
    }));
    Ok(Json(json!({ "secrets": listed })))
}

/// Where one secret is named.
#[derive(Default)]
struct Places {
    agents: Vec<Value>,
    services: Vec<Value>,
    instances: Vec<Value>,
}

impl Places {
    fn said(&self) -> Value {
        json!({ "agents": self.agents, "services": self.services, "instances": self.instances })
    }
}

/// Every secret that is named somewhere, with where: by an agent that is
/// handed it, by a service that is started with it, and in the text of an
/// instance that is running.
async fn named(app: &App) -> kernel::Result<BTreeMap<String, Places>> {
    let mut all: BTreeMap<String, Places> = BTreeMap::new();
    for slot in app.agents.slots().await? {
        for (variable, name) in &slot.secrets {
            all.entry(name.clone()).or_default().agents.push(json!({
                "name": slot.name, "project": slot.project, "variable": variable, "active": slot.active,
            }));
        }
    }
    let live = InstanceFilter { live_only: true, limit: 100_000, ..InstanceFilter::default() };
    let instances = app.engine.instances(live).await?;
    for instance in &instances {
        // What its environment lists is the environment's, and not the
        // factory's of the same name (`conductor::environs`).
        let theirs = match (instance.project.as_deref(), instance.environment.as_str()) {
            (Some(project), under) if !under.is_empty() => app.engine.environment_of(project, under).await?,
            _ => None,
        };
        let names = names_in(&instance.source, &instance.params).into_iter();
        for name in
            names.filter(|name| theirs.as_ref().is_none_or(|theirs| !theirs.secrets.contains_key(name)))
        {
            // A service the instance started is said as the service: once is enough.
            let places = all.entry(name).or_default();
            places.instances.push(json!({
                "id": instance.id, "blueprint": instance.blueprint, "key": instance.key, "project": instance.project,
            }));
        }
    }
    // Each once: one that is kept and named is in both.
    let names: std::collections::BTreeSet<String> =
        all.keys().cloned().chain(app.hosting.secrets()?.list().into_iter().map(|one| one.name)).collect();
    for name in names {
        for (service, variable) in app.hosting.services_handed(&name) {
            all.entry(name.clone()).or_default().services.push(json!({
                "name": service.name, "instance": service.instance, "project": service.project, "variable": variable,
            }));
        }
    }
    Ok(all)
}

/// The secrets a blueprint's text names, as far as its text says: each
/// `secret: <name>`, with a name that is a param's (`"${params.key}"`)
/// read from what the instance was given. A name worked out any other
/// way is not known until the step runs, and is left out. A line that is
/// a comment names nothing.
pub(crate) fn names_in(source: &str, params: &kernel::Fields) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for line in source.lines().filter(|line| !line.trim_start().starts_with('#')) {
        let mut rest = line;
        while let Some(at) = rest.find("secret:") {
            // `secret:` as a key, and not the end of a longer word.
            let before = rest[..at].chars().next_back();
            rest = &rest[at + "secret:".len()..];
            if before.is_some_and(|letter| letter.is_alphanumeric() || letter == '_' || letter == '-') {
                continue;
            }
            let written = rest.trim_start().trim_start_matches(['"', '\'']);
            let name = match written.strip_prefix("${") {
                // A param's, and nothing more: `"${params.key}-live"` is
                // worked out, and is not known from the text.
                Some(inner) => {
                    let (inner, after) = inner.split_once('}').unwrap_or_default();
                    let whole = after.trim_start_matches(['"', '\'']).trim_start();
                    let whole = whole.is_empty() || whole.starts_with(['}', ',', '#']);
                    inner
                        .trim()
                        .strip_prefix("params.")
                        .filter(|_| whole)
                        .and_then(|param| params.get(param))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                }
                None => written.split(['}', ',', '#', '"', '\'']).next().unwrap_or_default().trim(),
            };
            if hosting::secrets::refused(name).is_none() && !names.iter().any(|known| known == name) {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// Who last set each secret, as far as the journal still says.
async fn set_by(app: &App) -> BTreeMap<String, String> {
    let query = journal::EventQuery {
        topics: "secret.set".into(),
        limit: 10_000,
        newest_first: true,
        ..Default::default()
    };
    let mut by = BTreeMap::new();
    // Newest first: the first one said of a name is who set what is there.
    for event in app.journal.read_every(query).await.unwrap_or_default() {
        if let (Some(name), Some(who)) = (event.data["name"].as_str(), event.data["by_name"].as_str()) {
            by.entry(name.to_string()).or_insert_with(|| who.to_string());
        }
    }
    by
}

async fn secret_set(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // Before anything is read of what was sent: who may send it.
    who.require("admin")?;
    app.hosting.set_secret(&name, required(&body, "value")?, &who).await?;
    Ok(Json(json!({ "name": name })))
}

async fn secret_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    app.hosting.remove_secret(&name, &who).await?;
    Ok(Json(json!({ "ok": true })))
}
