//! The fleet (this device, its services and routers), search, measurements
//! and the administration of accounts.

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use conductor::InstanceFilter;
use journal::MetricQuery;
use kernel::{Error, Source};
use serde::Deserialize;
use serde_json::{Value, json};
use studio::{CredentialKind, Tier};

use super::{required, text};
use crate::http::{ApiError, ApiResult};
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/fleet", get(fleet))
        .route("/services/{name}/log", get(service_log))
        .route("/search", get(search))
        .route("/search/status", get(search_status))
        .route("/search/reindex", post(search_reindex))
        .route("/metrics", get(metrics))
        .route("/metrics/labels", get(metric_labels))
        .route("/accounts", get(accounts))
        .route("/accounts/{id}/given/{scope}", axum::routing::put(give).delete(take))
        .route("/invites", post(invite))
        .route("/tier", post(tier))
        .route("/credentials", get(credentials).post(credential_issue))
        .route("/credentials/{id}", axum::routing::delete(credential_revoke))
        .route("/credentials/{id}/withdrawing", get(credential_withdrawing))
        .route("/credentials/{id}/roll", axum::routing::post(credential_roll))
        .route("/system/stop", post(stop))
        .route("/system/drain", get(drain))
        .route("/system/names", get(names).post(name_add))
        .route("/system/names/{name}", axum::routing::delete(name_remove))
        .route("/system/speaker", get(speaker).post(speaker_call))
}

/// Stops the daemon, and with it whatever is keeping it running. What it
/// runs is left running, as when it stops for any other reason.
///
/// Told a step (`at`, and `of` for the blueprint it is a step of), it
/// stops when such a step next begins instead of now: the journal keeps
/// that step's beginning and nothing said after it, so that what the
/// daemon is started on again is what one that went at that instant would
/// have left. That is for seeing what a blueprint does about being cut
/// off there, in a factory to one side.
///
/// It is the owner's to ask. Where nobody owns the installation yet there
/// is nobody to sign in as, and whoever is at the machine it runs on could
/// make themselves its owner by registering: they may stop it as they are,
/// so that a daemon started to be looked at can be put away again.
async fn stop(
    State(app): State<App>,
    ConnectInfo(from): ConnectInfo<std::net::SocketAddr>,
    who: Result<Auth, ApiError>,
    body: Option<Json<Value>>,
) -> ApiResult<Json<Value>> {
    let source = match who {
        Ok(Auth(who)) => {
            who.require("admin")?;
            Source::actor(who.actor())
        }
        Err(_) if from.ip().is_loopback() && app.studio.identity.needs_setup().await? => Source::default(),
        Err(refused) => return Err(refused),
    };
    let body = body.map(|Json(body)| body).unwrap_or_default();
    // Once what is in hand is done (`crate::drain`).
    if body["drain"] == true {
        if text(&body, "at").is_some_and(|step| !step.is_empty()) {
            return Err(Error::invalid(
                "a drain stops once what is in hand is done, and not at a step: say one",
            )
            .into());
        }
        let within = crate::drain::within(text(&body, "within"))?;
        return Ok(Json(crate::drain::begin(&app, within, source).await?));
    }
    if let Some(step) = text(&body, "at").filter(|step| !step.is_empty()) {
        let of = text(&body, "of").filter(|of| !of.is_empty());
        let said = json!({ "at": step, "of": of });
        let _ = app.journal.emit("system.stopping", source, said.clone()).await;
        stop_at(&app, step.to_string(), of.map(str::to_string)).await?;
        return Ok(Json(json!({ "stopping": false, "at": said["at"], "of": said["of"] })));
    }
    let _ = app.journal.emit("system.stopping", source, json!({})).await;
    tokio::spawn(async move {
        // After the answer has gone out, so that whoever asked hears "yes"
        // and not a connection closing.
        app.journal.clock().sleep(std::time::Duration::from_millis(100)).await;
        app.stop.notify_one();
    });
    Ok(Json(json!({ "stopping": true })))
}

/// How a drain stands (`crate::drain`): nothing where none was asked
/// for. Told `wait`, answered once it is over, with how it came out.
async fn drain(
    State(app): State<App>,
    Auth(who): Auth,
    Query(asked): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    who.require("fleet:read")?;
    if asked.get("wait").is_some_and(|wait| wait == "true") && app.drain.asked().is_some() {
        return Ok(Json(crate::drain::over(&app).await));
    }
    Ok(Json(crate::drain::standing(&app).await?))
}

/// Has the journal shut as a step called `step` begins, of `of` if a
/// blueprint is named and of anything if none is, and the daemon stop
/// once it has.
async fn stop_at(app: &App, step: String, of: Option<String>) -> ApiResult<()> {
    // Which blueprint each instance runs: what is alive now, and whatever
    // starts between now and the step.
    let mut runs = std::collections::HashMap::new();
    if of.is_some() {
        let filter = InstanceFilter { live_only: true, limit: 100_000, ..Default::default() };
        for instance in app.engine.instances(filter).await? {
            runs.insert(instance.id, instance.blueprint);
        }
    }
    let runs = std::sync::Mutex::new(runs);
    app.journal.shut_after(move |event| {
        let Some(instance) = &event.source.instance else { return false };
        let mut runs = runs.lock().expect("stop lock");
        if event.topic == "instance.started" {
            let blueprint = event.data["blueprint"].as_str().unwrap_or_default();
            runs.insert(instance.clone(), blueprint.to_string());
        }
        event.topic == "step.started"
            && event.source.step.as_deref() == Some(step.as_str())
            && of.as_ref().is_none_or(|of| runs.get(instance) == Some(of))
    });
    let app = app.clone();
    tokio::spawn(async move {
        app.journal.shut().await;
        app.stop.notify_one();
    });
    Ok(())
}

/// Everything the fleet is (this device and the others, what runs, who
/// works here), in one read. Whatever draws it from this follows the event
/// stream after it, and reads this again only when an event says the shape
/// of the fleet changed; the client reads what this device says of itself
/// from it (Help's report). The page that drew it is taken out of the
/// application, to come back as a package.
async fn fleet(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("fleet:read")?;
    // Read before the rest, so that what happens while the rest is read is
    // heard again on the stream and not missed: whoever draws it takes an
    // event it has already drawn, and cannot take one it was never sent.
    let (now, last_seq) = (app.journal.now(), app.journal.highest());
    let data_dir = std::env::current_dir().unwrap_or_default();
    let resources =
        tokio::task::spawn_blocking(move || hosting::resources(&data_dir)).await.ok().and_then(Result::ok);
    // What is running here and who works here are each their own to read:
    // what its reader may see is handed, and the rest is not there.
    let instances = match who.can("blueprints:read") {
        true => {
            let live = InstanceFilter { live_only: true, limit: 1000, ..InstanceFilter::default() };
            app.engine.instances(live).await?
        }
        false => Vec::new(),
    };
    let slots = if who.can("agents:read") { app.agents.slots().await? } else { Vec::new() };
    // Whether a wall can be held around what is started here: asked of
    // the system the first time, which takes a moment, and kept.
    let walls = app.hosting.walls();
    let held = tokio::task::spawn_blocking(move || match walls {
        hosting::Walls::Open => json!({ "held": false, "open": true }),
        walls => match walls.can() {
            Ok(()) => json!({ "held": true }),
            Err(why) => json!({ "held": false, "why": why }),
        },
    });
    Ok(Json(json!({
        "device": {
            "name": app.journal.device(),
            "id": app.gateway.id(),
            "resources": resources,
            "address": app.addr.to_string(),
            "walls": held.await.unwrap_or_default(),
            "packages_behind": if who.can("agents:read") { app.agents.behind_what_came() } else { Vec::new() },
        },
        "instances": app.engine.shown(instances),
        "services": app.hosting.services(),
        "routers": app.hosting.routers(),
        "agents": slots,
        "devices": app.gateway.devices().await?,
        "hub": app.gateway.uplink(),
        "last_seq": last_seq,
        "cursor": crate::cursor::at(&now),
    })))
}

#[derive(Deserialize)]
struct LogQuery {
    after: Option<u64>,
    tail: Option<usize>,
}

async fn service_log(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Query(query): Query<LogQuery>,
) -> ApiResult<Json<Value>> {
    who.require("fleet:read")?;
    let logs = app.engine.logs();
    let lines = match query.after {
        Some(after) => logs.read("services", &name, after, 2000),
        None => logs.tail("services", &name, query.tail.unwrap_or(200).min(5000)),
    };
    Ok(Json(json!({ "lines": lines })))
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    kind: Option<String>,
    project: Option<String>,
    limit: Option<usize>,
}

async fn search(
    State(app): State<App>,
    Auth(who): Auth,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<Value>> {
    who.require("search")?;
    let (kind, project) = (query.kind.as_deref(), query.project.as_deref());
    let limit = query.limit.unwrap_or(30).clamp(1, 200);
    // What is found by what it is called first, listed as its asker may
    // read it, and no more of it than leaves room for what is written.
    let named = crate::searched::named(&app, &who, kind, project, &query.q).await?;
    let mut found =
        studio::search::found(&query.q, named, if kind.is_some() { limit } else { limit.div_ceil(3) });
    // Then what is written in things, asked for more of than is handed:
    // some of it may be kept from whoever asks.
    let indexed = match kind {
        Some("project" | "channel" | "person" | "agent" | "run") => Vec::new(),
        _ => app.studio.search.query(&query.q, kind, project, (limit * 3).min(200)).await?,
    };
    // What was said in a channel kept to its members, by them.
    let indexed = app.studio.search.readable(indexed, &who).await?;
    let mut tables = std::collections::HashMap::new();
    for hit in indexed {
        if found.len() >= limit {
            break;
        }
        if readable(&app, &who, &hit, &mut tables).await {
            found.push(hit);
        }
    }
    Ok(Json(json!({ "hits": found, "index": app.studio.search.status().await })))
}

/// Whether `who` may be shown something the index found: as each kind is
/// read. A passage of a message is the channel's to show, a card's the
/// board's, a row's its table's (asked once a table, in `tables`), and a
/// file or a document is its project's, and of nobody a table that keeps
/// it private keeps it from.
async fn readable(
    app: &App,
    who: &kernel::Principal,
    hit: &studio::Hit,
    tables: &mut std::collections::HashMap<(String, String), bool>,
) -> bool {
    let reads = match hit.kind.as_str() {
        "message" => "channels:read",
        "card" => "board:read",
        "blueprint" => "blueprints:read",
        "record" | "topic" => "records:read",
        _ => "projects:read",
    };
    if !who.can(reads) {
        return false;
    }
    let link = |key: &str| hit.link.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    if matches!(hit.kind.as_str(), "record" | "topic") {
        let table = (link("at"), link("table"));
        if let Some(may) = tables.get(&table) {
            return *may;
        }
        let may = match app.studio.collections.get(&table.0, &table.1).await {
            Ok(collection) => collection.may_read(who).is_ok(),
            Err(_) => false,
        };
        tables.insert(table, may);
        return may;
    }
    match (&hit.project, hit.link.get("path").and_then(Value::as_str)) {
        (Some(project), Some(path)) => app.studio.projects.may_read_path(project, path, who).await.is_ok(),
        _ => true,
    }
}

async fn search_status(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("search")?;
    Ok(Json(json!(app.studio.search.status().await)))
}

/// Starts a full indexing pass and returns at once; progress arrives on the
/// event stream as `search.progress`.
async fn search_reindex(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("search")?;
    let search = app.studio.search.clone();
    tokio::spawn(async move {
        let _ = search.reindex().await;
    });
    Ok(Json(json!({ "started": true })))
}

#[derive(Deserialize)]
struct MetricsQuery {
    /// A measurement's name, or a prefix ending in `*`.
    name: String,
    /// How far back to look, e.g. `1h` (the default) or `7d`.
    since: Option<String>,
    /// Label keys to split by, comma-separated.
    by: Option<String>,
    /// Width of each point, e.g. `5m`. Without it only totals are returned.
    step: Option<String>,
    /// Only series with these labels: `key:value,key:value`.
    labels: Option<String>,
}

/// Reads measurements: how often something happened, how often it failed,
/// and how long it took, over a window.
async fn metrics(
    State(app): State<App>,
    Auth(who): Auth,
    Query(query): Query<MetricsQuery>,
) -> ApiResult<Json<Value>> {
    who.require("fleet:read")?;
    let window = millis(&query.since, 3_600_000)?;
    let step = millis(&query.step, 0)?;
    let now = app.journal.now_ms();
    let series = app
        .journal
        .query_metrics(MetricQuery {
            name: query.name,
            since_ms: now - window as i64,
            until_ms: 0,
            group_by: query
                .by
                .iter()
                .flat_map(|by| by.split(','))
                .map(|key| key.trim().to_string())
                .filter(|k| !k.is_empty())
                .collect(),
            step_minutes: (step / 60_000) as i64,
            labels: pairs(&query.labels),
        })
        .await?;
    Ok(Json(json!({ "series": with_units(series), "since": now - window as i64, "until": now })))
}

/// Series as they are handed out: each says what its values are of where
/// its name does (`unit`: `usd`, `tokens`, ...; [`blueprint::labels::unit`]),
/// so that whatever shows one adds it up and writes it as that, and
/// nothing that shows measurements keeps a list of names. One that says
/// nothing is a length of time, in milliseconds.
pub(crate) fn with_units(series: Vec<journal::MetricSeries>) -> Value {
    let said = |one: journal::MetricSeries| {
        let unit = blueprint::labels::unit(&one.name);
        let mut one = json!(one);
        if let Some(unit) = unit {
            one["unit"] = json!(unit);
        }
        one
    };
    series.into_iter().map(said).collect()
}

/// A length of time as it is written in a query, or `default`.
fn millis(text: &Option<String>, default: u64) -> Result<u64, Error> {
    match text {
        Some(text) => blueprint::duration::parse(text).map_err(Error::invalid),
        None => Ok(default),
    }
}

/// `key:value,key:value` as it is written in a query.
fn pairs(labels: &Option<String>) -> Vec<(String, String)> {
    labels
        .iter()
        .flat_map(|labels| labels.split(','))
        .filter_map(|pair| pair.split_once(':'))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

#[derive(Deserialize)]
struct LabelsQuery {
    /// Of which measurement, or a prefix ending in `*`. All of them unless said.
    name: Option<String>,
    /// How far back to look, e.g. `1h` (the default) or `7d`.
    since: Option<String>,
    /// Only among what carries these: `key:value,key:value`.
    labels: Option<String>,
}

/// The labels blueprints gave to what was measured, each with the values it
/// has: what there is to split measurements by and to narrow them to,
/// besides what every measurement says of itself.
async fn metric_labels(
    State(app): State<App>,
    Auth(who): Auth,
    Query(query): Query<LabelsQuery>,
) -> ApiResult<Json<Value>> {
    who.require("fleet:read")?;
    let window = millis(&query.since, 3_600_000)?;
    let asked = MetricQuery {
        name: query.name.unwrap_or_else(|| "*".to_string()),
        since_ms: app.journal.now_ms() - window as i64,
        labels: pairs(&query.labels),
        ..MetricQuery::default()
    };
    let found = app.journal.metric_labels(asked, blueprint::labels::TAKEN).await?;
    Ok(Json(json!({ "labels": found })))
}

async fn accounts(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    // Each with what it holds as things stand, its role's and what it was
    // given, so that what is offered to give it is worked out from that.
    let identity = &app.studio.identity;
    let accounts: Vec<Value> =
        identity.accounts().await?.into_iter().map(|account| listed(identity, &account)).collect();
    Ok(Json(json!({ "accounts": accounts, "tier": app.studio.identity.tier().await? })))
}

/// An account as it is listed: with what it holds as things stand, its
/// groups' among it, and which groups those are.
fn listed(identity: &studio::Identity, account: &studio::Account) -> Value {
    let mut said = json!(account);
    said["holds"] = json!(identity.principal_of(account).scopes);
    let groups: Vec<String> =
        identity.grouped().of(&format!("user:{}", account.id)).into_iter().map(|group| group.id).collect();
    said["groups"] = json!(groups);
    said
}

/// Gives a person a scope beyond what their role holds, as somebody who
/// holds it and `admin` (`people::given`).
async fn give(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, scope)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let given = app.studio.identity.give(&id, &scope, &who).await?;
    Ok(Json(json!({ "account": listed(&app.studio.identity, &given.account), "changed": given.changed })))
}

/// Takes a scope a person was given away from them, and from their keys.
async fn take(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, scope)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let taken = app.studio.identity.take(&id, &scope, &who).await?;
    Ok(Json(
        json!({ "account": listed(&app.studio.identity, &taken.account), "changed": taken.changed, "keys": taken.keys }),
    ))
}

/// The names this daemon answers to besides the addresses it listens on.
async fn names(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("fleet:read")?;
    Ok(Json(json!({ "names": app.names.told(), "address": app.addr.to_string() })))
}

async fn name_add(
    State(app): State<App>,
    Auth(who): Auth,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let name = app.names.tell(required(&body, "name")?, &who).await?;
    Ok(Json(json!({ "name": name, "names": app.names.told() })))
}

async fn name_remove(
    State(app): State<App>,
    Auth(who): Auth,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> ApiResult<Json<Value>> {
    let name = app.names.forget(&name, &who).await?;
    Ok(Json(json!({ "name": name, "names": app.names.told() })))
}

/// What the factory's blueprints speak as in a channel: the one bot's
/// name, and what it takes when it is spoken to, for whoever shows that
/// where it is written to.
async fn speaker(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("channels:read")?;
    let commands: Vec<Value> = crate::commands::TAKES
        .iter()
        .map(|(written, does)| json!({ "written": written, "does": does }))
        .collect();
    Ok(Json(json!({
        "name": app.studio.channels.speaker.name(),
        "usual": studio::speaker::DEFAULT,
        "commands": commands,
        // The most it may ask, where somebody narrowed it; what its commands reach, which it may otherwise.
        "holds": app.studio.channels.speaker.holds(),
        "reaches": crate::commands::REACHES,
    })))
}

async fn speaker_call(
    State(app): State<App>,
    Auth(who): Auth,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    let name = blueprint::expr::slug(required(&body, "name")?);
    // What is said to a name is said to one thing: an agent is spoken to
    // by its name, and the bot by its own.
    if app.agents.slots().await?.iter().any(|slot| slot.name == name) {
        return Err(Error::conflict(format!(
            "an agent is called `{name}`, and what names it is said to the agent; choose another name"
        ))
        .into());
    }
    Ok(Json(json!({ "name": app.studio.channels.call_speaker(&name, &who).await? })))
}

/// An invitation for somebody to register with: its code, and when it
/// stops working (`expires` in the request says how long from now, as 2d
/// or 12h; a week unless said).
async fn invite(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let lasts = match text(&body, "expires").filter(|said| !said.trim().is_empty()) {
        Some(said) => {
            Some(std::time::Duration::from_millis(blueprint::duration::parse(said).map_err(|why| {
                Error::invalid(format!(
                    "`{said}` is not how long an invitation works: {why}. Say it as 2d or 12h"
                ))
            })?))
        }
        None => None,
    };
    let (code, expires) = app.studio.identity.invite_for(&who, text(&body, "email"), lasts).await?;
    Ok(Json(json!({ "code": code, "expires": expires })))
}

async fn tier(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let tier = Tier::parse(required(&body, "tier")?)
        .ok_or_else(|| Error::invalid("`tier` is free, premium or enterprise"))?;
    app.studio.identity.set_tier(&who, tier).await?;
    Ok(Json(json!({ "tier": tier })))
}

/// The credentials there are, without their tokens: all of them for
/// whoever manages the installation, and for anybody else the keys that
/// are their own.
async fn credentials(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    let mut all = app.studio.identity.credentials().await?;
    if !who.can("admin") {
        if who.kind != kernel::PrincipalKind::User {
            who.require("admin")?;
        }
        all.retain(|one| one.kind == CredentialKind::Key && one.subject == who.id);
    }
    Ok(Json(json!({ "credentials": all })))
}

#[derive(serde::Deserialize)]
struct RevokeQuery {
    /// Leave what the key applied running, with what the key held.
    #[serde(default)]
    leave: bool,
}

/// An instance a key applied, as a withdrawing says it.
fn applied_said(one: &conductor::InstanceInfo) -> Value {
    json!({ "id": one.id, "blueprint": one.blueprint, "key": one.key, "project": one.project })
}

/// What withdrawing the key `id` would come to, asked first so that
/// whoever withdraws it is asked about it (as taking a secret away says
/// what names it): the keys made with it, which go with it, and what each
/// applied that is alive, which is stopped. Nothing is changed.
async fn credential_withdrawing(
    State(app): State<App>,
    Auth(who): Auth,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<Value>> {
    let withdrawing = app.studio.identity.withdrawing(&id, &who).await?;
    let ids: Vec<String> = withdrawing.all().map(|one| one.id.clone()).collect();
    let running = app.engine.applied_with(&ids).await?;
    let keys: Vec<Value> =
        withdrawing.made_with.iter().map(|one| json!({ "id": one.id, "name": one.name })).collect();
    Ok(Json(json!({
        "key": withdrawing.key.id, "name": withdrawing.key.name, "keys": keys,
        "running": running.iter().map(applied_said).collect::<Vec<_>>(),
    })))
}

/// Withdraws a key, and every key made with it. The person whose it is
/// may, and whoever manages the installation; a key may not withdraw
/// another unless it holds `admin` (`people::keys`).
///
/// What was applied with them is stopped, as stopping it by hand does:
/// what it started with it, its services, an agent's turn it asked for.
/// What it did stays done. The answer says what was stopped (`stopped`),
/// each by the instance that was applied and not what it started in
/// turn, and one that could not be with why (`left`). Asked to leave it
/// (`leave`), it goes on with what the key held, and the answer says
/// which it is (`running`).
async fn credential_revoke(
    State(app): State<App>,
    Auth(who): Auth,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(asked): axum::extract::Query<RevokeQuery>,
) -> ApiResult<Json<Value>> {
    // Withdrawn first, so that nothing more is applied with it while what
    // it applied is stopped.
    let withdrawn = app.studio.identity.withdraw_key(&id, &who, asked.leave).await?;
    let ids: Vec<String> = withdrawn.all().map(|one| one.id.clone()).collect();
    let applied = app.engine.applied_with(&ids).await?;
    let (mut running, mut stopped, mut left) = (Vec::new(), Vec::new(), Vec::new());
    for one in &applied {
        if asked.leave {
            running.push(applied_said(one));
            continue;
        }
        match app.engine.stop_withdrawn(&one.id).await {
            Ok(()) => stopped.push(applied_said(one)),
            Err(err) => {
                let mut kept = applied_said(one);
                kept["why"] = json!(err.message);
                left.push(kept.clone());
                running.push(kept);
            }
        }
    }
    let keys: Vec<Value> =
        withdrawn.made_with.iter().map(|one| json!({ "id": one.id, "name": one.name })).collect();
    Ok(Json(json!({
        "revoked": withdrawn.key.id, "name": withdrawn.key.name, "keys": keys,
        "running": running, "stopped": stopped, "left": left,
    })))
}

/// Gives a key another secret: the token it had is refused from then,
/// and the key is the same by its id, holding what it held, so what it
/// applied goes on. Its maker may, whoever manages the installation, and
/// the key itself. The new token is in the answer and nowhere else.
async fn credential_roll(
    State(app): State<App>,
    Auth(who): Auth,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<Value>> {
    let (key, token) = app.studio.identity.roll_key(&id, &who).await?;
    Ok(Json(json!({ "token": token, "id": key.id, "name": key.name, "scopes": key.scopes })))
}

/// Issues a long-lived key for scripts, acting as the account that asks.
/// The token is in the answer and nowhere else, ever.
///
/// A key holds no more than whoever makes it. Told nothing, it holds what
/// its maker does, which for a key made with a key is that key's scopes
/// and not a person's; anything asked for that the maker lacks is refused
/// by name. Making one needs no scope of its own: nothing can be made
/// this way that the maker could not already do.
async fn credential_issue(
    State(app): State<App>,
    Auth(who): Auth,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    if who.kind != kernel::PrincipalKind::User {
        return Err(Error::denied("only a signed-in person can make a key").into());
    }
    let asked: Option<Vec<String>> = body
        .get("scopes")
        .and_then(Value::as_array)
        .map(|scopes| scopes.iter().filter_map(|scope| scope.as_str().map(str::to_string)).collect());
    if let Some(beyond) = asked.iter().flatten().find(|scope| !who.can(scope)) {
        return Err(Error::denied(format!(
            "a key holds no more than whoever makes it, and you do not hold `{beyond}`: leave it out, or have \
             the key made by someone who holds it"
        ))
        .into());
    }
    let scopes = asked.unwrap_or_else(|| who.scopes.clone());
    let name = text(&body, "name").unwrap_or("key");
    // Made with a key, it is withdrawn with that key (`people::keys`).
    let token = app
        .studio
        .identity
        .issue_with(CredentialKind::Key, &who.id, name, Some(scopes.clone()), who.key.clone())
        .await?;
    // Its id is what an event said with it is said by (`key:<id>`).
    let (_, id) = app.studio.identity.known(&token).await?;
    Ok(Json(json!({ "token": token, "id": id, "name": name, "scopes": scopes })))
}
