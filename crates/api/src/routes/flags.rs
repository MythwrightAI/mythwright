//! A project's feature flags, read and changed.
//!
//! A flag is a row of the project's `flags` table (`std/flags`); what it
//! says in each environment is that environment's variable of it
//! (`FLAG_<NAME>`), written with what is said of it as one write
//! (`Engine::set_flag`). What a change comes to is `kernel::flags::change`:
//! one that turns a flag on, or wider, in an environment whose rule names
//! who approves, asked by somebody it does not name, is said to what looks
//! after the project's flags (`project.flag.asked`), which asks one of them
//! to let it go on (`approval.ask`) and writes it once one does; anything
//! else is written now, as whoever asks.
//!
//! Reading them takes reading the project and its records; changing one
//! takes writing the project, and no more, since what a flag says is only
//! ever `on`, `off`, a percentage or a plain word (`kernel::flags::said`),
//! where a variable of the environment written by itself, which may hold
//! anything, takes `shell` and `secrets:use` too. A variable that is a
//! flag's is changed as a flag and never by itself
//! (`super::environments`).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use conductor::environs::{Environment, Flagging};
use kernel::flags::{self, Came, Kind};
use kernel::{Error, Principal, PrincipalKind, Source};
use serde_json::{Value, json};
use studio::collections::Record;

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/projects/{id}/flags", get(listed))
        .route("/projects/{id}/flags/{flag}/{environment}", put(set).delete(taken_away))
        .route("/projects/{id}/flags/{flag}/compare", post(compare))
}

/// What changing a flag takes.
const CHANGING: &str = "projects:write";

/// Who asks, as a flag's change is said and held to an environment's rule:
/// a key that holds less is itself (`key:<id>`), never its maker.
fn asker(who: &Principal) -> String {
    who.key.as_ref().map_or_else(|| who.actor(), |key| format!("key:{key}"))
}

/// The project's flags, refused, with the command that gives them, where
/// it has none.
async fn rows(app: &App, project: &str, who: &Principal) -> kernel::Result<Vec<Record>> {
    match app.studio.records.list(project, flags::TABLE, who).await {
        Ok(rows) => Ok(rows),
        Err(refused) if refused.kind == kernel::ErrorKind::NotFound => {
            let starts = setup::usual::the_usual(&app.engine).await.unwrap_or_default();
            Err(setup::usual::lacking(&starts, project, flags::TABLE, refused))
        }
        Err(refused) => Err(refused),
    }
}

fn cell(row: &Record, name: &str) -> String {
    row.data.get(name).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// The row of the flag called `flag`, by its name written plainly.
fn row_of<'a>(rows: &'a [Record], project: &str, flag: &str) -> kernel::Result<&'a Record> {
    let key = flags::key(flag);
    let mut found = rows.iter().filter(|row| flags::key(&cell(row, "name")) == key);
    match (found.next(), found.next()) {
        (Some(row), None) => Ok(row),
        (Some(_), Some(_)) => Err(Error::conflict(format!(
            "two rows of `{project}`'s flags are called `{key}`: call one of them something else first"
        ))),
        (None, _) => Err(Error::not_found(format!(
            "`{project}` has no flag called `{flag}`. `{} flag list {project}` lists those it has",
            brand::CLI
        ))),
    }
}

async fn environment(app: &App, project: &str, name: &str) -> kernel::Result<Environment> {
    app.engine.environment_of(project, name).await?.ok_or_else(|| super::environments::missing(project, name))
}

/// Each flag with what it says in each environment, the environments, and
/// each variable of a flag no row names any more, which is taken away
/// there as a flag is.
async fn listed(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    app.studio.projects.get(&id).await?;
    let rows = rows(&app, &id, &who).await?;
    let environments = app.engine.environments_of(&id).await?;
    // An agent is shown what is marked for agents, as of any variable.
    let agent = who.kind == PrincipalKind::Agent;
    let shown_of = |environment: &Environment, variable: &str| {
        environment.vars.get(variable).filter(|var| !agent || var.agents).map(|var| var.value.clone())
    };
    let mut named = Vec::new();
    let listed: Vec<Value> = rows
        .iter()
        .map(|row| {
            let name = cell(row, "name");
            let kind = Kind::read(&cell(row, "kind"));
            let variable = flags::variable(&name);
            named.push(variable.clone());
            let says: serde_json::Map<String, Value> = environments
                .iter()
                .map(|environment| {
                    let value = shown_of(environment, &variable);
                    let shown = flags::shown(kind, value.as_ref().unwrap_or(&flags::off(kind)));
                    (environment.name.clone(), json!({ "value": value, "shown": shown }))
                })
                .collect();
            json!({
                "id": row.id, "name": name, "flag": flags::key(&name), "variable": variable,
                "about": cell(row, "about"), "owner": cell(row, "owner"), "until": cell(row, "until"),
                "kind": kind.word(), "says": says,
            })
        })
        .collect();
    let left: Vec<Value> = environments
        .iter()
        .flat_map(|environment| {
            environment
                .vars
                .iter()
                .filter(|(variable, var)| flags::is_flags(variable) && !named.contains(variable) && (!agent || var.agents))
                .map(|(variable, var)| json!({ "environment": environment.name, "variable": variable, "value": var.value }))
                .collect::<Vec<_>>()
        })
        .collect();
    let names: Vec<&String> = environments.iter().map(|environment| &environment.name).collect();
    Ok(Json(json!({ "project": id, "flags": listed, "environments": names, "left": left })))
}

/// Says what a flag says in an environment (`value`): written now, or,
/// where it turns the flag on or wider under a rule that names others as
/// approving, asked of them (202, `came: asks`), only where it is still the
/// change asked last of the flag there and the flag says what it said when
/// it was asked (`environments::flagged`).
async fn set(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, flag, name)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    who.require(CHANGING)?;
    app.studio.projects.get(&id).await?;
    let rows = rows(&app, &id, &who).await?;
    let row = row_of(&rows, &id, &flag)?;
    let environment = environment(&app, &id, &name).await?;
    let (called, kind) = (cell(row, "name"), Kind::read(&cell(row, "kind")));
    let asked = body.get("value").cloned().unwrap_or(Value::Null);
    let was = environment.vars.get(&flags::variable(&called)).map(|var| var.value.clone());
    let by = asker(&who);
    let mut came =
        flags::change(kind, was.as_ref(), &asked, &environment.rules.approve, &by).map_err(Error::invalid)?;
    // Saying what it says already is a change all the same where a change
    // of it waits: written, it takes that one's place, and its approval
    // writes nothing.
    if came == Came::Same && environment.asked.contains_key(&flags::variable(&called)) {
        came = Came::Set(flags::said(kind, &asked).map_err(Error::invalid)?);
    }
    let flagging = |value: Value| Flagging {
        project: &id,
        environment: &name,
        flag: &called,
        record: &row.id,
        kind,
        value: Some(value),
        asked_by: None,
        approved_by: None,
        expected: None,
    };
    match came {
        Came::Same => Ok((
            StatusCode::OK,
            Json(json!({ "came": "same", "flag": flags::key(&called), "environment": name })),
        )),
        Came::Set(value) => {
            let shown = flags::shown(kind, &value);
            app.engine.set_flag(flagging(value.clone()), &who, Source::actor(by)).await?;
            let said = json!({ "came": "set", "flag": flags::key(&called), "environment": name, "value": value, "shown": shown });
            Ok((StatusCode::OK, Json(said)))
        }
        Came::Asks(value) => {
            looked_after(&app, &id).await?;
            let shown = flags::shown(kind, &value);
            // Kept on the environment as the change asked last of the flag
            // there, with what the flag says now, in the write that says it.
            let id_of = kernel::id::new("fa", app.journal.now_ms());
            app.engine.ask_flag(flagging(value.clone()), &id_of, &who, Source::actor(by)).await?;
            let words = format!(
                "Setting {called} to {shown} in {name} waits for a go-ahead from whom its rule names as approving \
                 what goes under it, who are told in their inbox"
            );
            let answer = json!({ "came": "asks", "flag": flags::key(&called), "environment": name, "value": value, "shown": shown, "said": words, "asked": id_of });
            Ok((StatusCode::ACCEPTED, Json(answer)))
        }
    }
}

/// Refused where nothing looks after the project's flags, which is what
/// asks a go-ahead and writes a change once it is let go on.
async fn looked_after(app: &App, project: &str) -> kernel::Result<()> {
    let filter = conductor::InstanceFilter {
        live_only: true,
        project: Some(project.to_string()),
        blueprint: Some("std/flags".to_string()),
        limit: 1,
        ..conductor::InstanceFilter::default()
    };
    match app.engine.instances(filter).await?.is_empty() {
        false => Ok(()),
        true => Err(Error::conflict(format!(
            "nothing looks after `{project}`'s flags, which is what asks for a go-ahead: \
             `{} apply std/flags --project {project}` starts it again",
            brand::CLI
        ))),
    }
}

/// Takes a flag's variable away in an environment, which narrows it and so
/// never waits: also of a flag no row names any more.
async fn taken_away(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, flag, name)): Path<(String, String, String)>,
) -> ApiResult<Json<Value>> {
    who.require(CHANGING)?;
    app.studio.projects.get(&id).await?;
    let rows = rows(&app, &id, &who).await?;
    let row = row_of(&rows, &id, &flag).ok();
    let called = row.map(|row| cell(row, "name")).unwrap_or_else(|| flag.clone());
    let kind = row.map_or(Kind::Switch, |row| Kind::read(&cell(row, "kind")));
    environment(&app, &id, &name).await?;
    let flagging = Flagging {
        project: &id,
        environment: &name,
        flag: &called,
        record: row.map(|row| row.id.as_str()).unwrap_or_default(),
        kind,
        value: None,
        asked_by: None,
        approved_by: None,
        expected: None,
    };
    let flagged = app.engine.set_flag(flagging, &who, Source::actor(asker(&who))).await?;
    Ok(Json(json!({ "flag": flags::key(&called), "environment": name, "taken_away": flagged.written })))
}

/// A percentage flag started as a comparison on real work: the blueprint
/// started for each thing its param `per` names, two ways, `on` for the
/// flag's part of the things in `environment` and `off` for the rest, each
/// handed its word as the blueprint's param `param`. It is counted as
/// `counts` says, and never scored (`/api/comparisons`).
async fn compare(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, flag)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:run")?;
    app.studio.projects.get(&id).await?;
    let text = |key: &str| body.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    let rows = rows(&app, &id, &who).await?;
    let row = row_of(&rows, &id, &flag)?;
    let called = cell(row, "name");
    let name = text("environment");
    let environment = environment(&app, &id, &name).await?;
    let says =
        environment.vars.get(&flags::variable(&called)).map(|var| var.value.clone()).unwrap_or(Value::Null);
    let counts = match body.get("counts") {
        None | Some(Value::Null) => Vec::new(),
        Some(counts) => serde_json::from_value(counts.clone())
            .map_err(|wrong| Error::invalid(format!("`counts` is each thing to count: {wrong}")))?,
    };
    let (blueprint, per, param) = (text("blueprint"), text("per"), text("param"));
    let asked = flags::compared(flags::Compared {
        flag: &called,
        environment: &name,
        percentage: &says,
        project: &id,
        blueprint: &blueprint,
        per: &per,
        param: &param,
        counts,
    })
    .map_err(Error::invalid)?;
    Ok(Json(json!(app.comparisons.compare(asked, &who).await?)))
}
