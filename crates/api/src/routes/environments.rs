//! A project's environments and their variables, read and changed.
//!
//! What an environment is, is `environments::project`, and the runner
//! keeps it with what is said of each change (`conductor::environs`).
//! What is here is who may: reading one takes reading the project, and an
//! agent is shown the variables marked for agents and nothing else of the
//! rest; changing one takes writing the project and, since a variable
//! reaches a command and the name of a secret that whoever applied runs
//! with, `shell` and `secrets:use` too (as a cell of `segments` that
//! hands on what its writer says does).
//!
//! An environment's secrets are set, replaced, taken away and marked for
//! agents by whoever holds `environments:secrets`, which the factory's
//! owner holds without being given it, and nobody is ever shown one. Their
//! names are shown beside the variables to whoever may use secrets
//! (`secrets:use`), each saying whether a value is kept for it (a copy of
//! the factory carries none). A value is written before it is listed and
//! taken away after it is not, so that a stop between the two leaves a
//! file nothing reads.
//!
//! The factory's own list of profiles (copies of the factory) is
//! `/api/profiles`, which `/api/environments` still answers as it did
//! before 0.1.0.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use conductor::environs::Environment;
use kernel::{Error, Fields, Principal, PrincipalKind, Source};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/projects/{id}/environments", get(listed).post(make))
        .route("/projects/{id}/environments/{environment}", get(one).delete(remove))
        .route("/projects/{id}/environments/{environment}/remove", get(would_remove))
        .route("/projects/{id}/environments/{environment}/vars/{var}", axum::routing::put(set).delete(unset))
        .route("/projects/{id}/environments/{environment}/secrets", get(secrets_named))
        .route(
            "/projects/{id}/environments/{environment}/secrets/{name}",
            axum::routing::put(set_secret).delete(remove_secret),
        )
        .route(
            "/projects/{id}/environments/{environment}/secrets/{name}/agents",
            axum::routing::put(mark_secret),
        )
        .route("/projects/{id}/environments/{environment}/rules", axum::routing::put(say_rules))
}

/// What setting, taking away or marking an environment's secret takes.
const SECRETS: &str = "environments:secrets";

/// What changing an environment takes.
const CHANGING: &[&str] = &["projects:write", "shell", "secrets:use"];

pub(super) fn may_change(who: &Principal) -> kernel::Result<()> {
    CHANGING.iter().try_for_each(|scope| who.require(scope))
}

/// Refuses a variable that is a feature flag's (`FLAG_<NAME>`), which is
/// changed as a flag, where a turning on waits for whom the environment's
/// rule names (`super::flags`), and never by itself.
fn flags_own(var: &str) -> kernel::Result<()> {
    match kernel::flags::is_flags(var) {
        false => Ok(()),
        true => Err(Error::conflict(format!(
            "`{var}` is a feature flag's, and is changed as the flag: `{} flag set <project> <flag> <environment> \
             <value>`, or on the project's Environments page",
            brand::CLI
        ))),
    }
}

/// Whether what is shown is for an agent, which sees its marked variables alone.
fn for_an_agent(who: &Principal) -> bool {
    who.kind == PrincipalKind::Agent
}

/// The project, refused as one that is not there where it is not.
async fn project(app: &App, id: &str) -> kernel::Result<()> {
    app.studio.projects.get(id).await.map(|_| ())
}

/// An environment as `who` is shown it: each of its secrets they may see
/// said with whether a value is kept for it.
pub(super) fn shown(app: &App, project: &str, environment: &Environment, who: &Principal) -> Value {
    // Whoever may name secrets is shown what an environment lists, and an
    // agent the ones it is to be handed, since it is told those by name.
    let agent = for_an_agent(who);
    let mut shown = environment.shown(agent, agent || who.can("secrets:use"));
    let kept = app.hosting.secrets().ok();
    if let Some(secrets) = shown.get_mut("secrets").and_then(Value::as_object_mut) {
        for (name, secret) in secrets.iter_mut() {
            let has = kept.as_ref().is_some_and(|kept| kept.has_of(project, &environment.name, name));
            secret["kept"] = json!(has);
        }
    }
    shown
}

async fn listed(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    project(&app, &id).await?;
    let environments = app.engine.environments_of(&id).await?;
    let under = super::deploys::under(&app, &who, &id).await;
    let mut shown = Vec::new();
    for environment in &environments {
        let one = self::shown(&app, &id, environment, &who);
        shown.push(super::deploys::with_ran(&app, &who, &id, under.as_ref(), one).await);
    }
    Ok(Json(json!({ "project": id, "environments": shown })))
}

async fn one(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, name)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    project(&app, &id).await?;
    let environment = app.engine.environment_of(&id, &name).await?.ok_or_else(|| missing(&id, &name))?;
    let under = super::deploys::under(&app, &who, &id).await;
    let shown = shown(&app, &id, &environment, &who);
    Ok(Json(super::deploys::with_ran(&app, &who, &id, under.as_ref(), shown).await))
}

async fn make(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    may_change(&who)?;
    project(&app, &id).await?;
    let name = body.get("name").and_then(Value::as_str).ok_or_else(|| {
        Error::invalid("`name` is needed: what the environment is called, such as `staging`")
    })?;
    let made = app.engine.make_environment(&id, name, &who, Source::actor(who.actor())).await?;
    Ok(Json(shown(&app, &id, &made, &who)))
}

/// Sets a variable: `value`, and `agents` where it says whether an agent
/// is handed it. With no `value` the variable is only marked, and has to
/// be set already.
async fn set(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, name, var)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    may_change(&who)?;
    project(&app, &id).await?;
    flags_own(&var)?;
    let agents = match body.get("agents") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(agents)) => Some(*agents),
        Some(_) => return Err(Error::invalid("`agents` is true or false").into()),
    };
    let value = body.get("value").cloned();
    let change = |environment: &mut Environment| match (value, agents) {
        (Some(value), agents) => {
            let mut vars = Fields::new();
            vars.insert(var.clone(), value);
            environment.set(&vars, agents)
        }
        (None, Some(agents)) => environment.mark(std::slice::from_ref(&var), agents),
        (None, None) => Err(Error::invalid("`value` is needed, or `agents` to only mark the variable")),
    };
    let source = Source::actor(who.actor());
    let (environment, changed) = app.engine.change_environment(&id, &name, change, &who, source).await?;
    Ok(Json(json!({ "environment": shown(&app, &id, &environment, &who), "changed": changed })))
}

async fn unset(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, name, var)): Path<(String, String, String)>,
) -> ApiResult<Json<Value>> {
    may_change(&who)?;
    project(&app, &id).await?;
    flags_own(&var)?;
    let change = |environment: &mut Environment| environment.unset(std::slice::from_ref(&var));
    let source = Source::actor(who.actor());
    let (environment, changed) = app.engine.change_environment(&id, &name, change, &who, source).await?;
    Ok(Json(json!({ "environment": shown(&app, &id, &environment, &who), "changed": changed })))
}

/// What taking an environment away would do, before it is done.
async fn would_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, name)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    may_change(&who)?;
    project(&app, &id).await?;
    let environment = app.engine.environment_of(&id, &name).await?.ok_or_else(|| missing(&id, &name))?;
    let vars: Vec<&String> = environment.vars.keys().collect();
    let secrets: Vec<&String> = environment.secrets.keys().collect();
    let mut running: Vec<String> =
        alive_under(&app, &id, &name).await?.iter().map(|key| format!("`{key}`")).collect();
    running.extend(agents_under(&app, &id, &name).await?.iter().map(|agent| format!("the agent `{agent}`")));
    let said = match conductor::environs::in_use(&id, &name, &running) {
        Some(refused) => refused.message,
        None => removing(&name, vars.len(), secrets.len()),
    };
    Ok(Json(
        json!({ "environment": name, "vars": vars, "secrets": secrets, "running": running, "said": said }),
    ))
}

async fn remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, name)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    may_change(&who)?;
    project(&app, &id).await?;
    // Its secrets go with it, so whoever takes away one that lists any
    // holds what taking a secret away takes.
    let listed = app.engine.environment_of(&id, &name).await?.ok_or_else(|| missing(&id, &name))?;
    if !listed.secrets.is_empty() {
        who.require(SECRETS)?;
    }
    let agents = agents_under(&app, &id, &name);
    let was = app.engine.remove_environment((&id, &name), agents, &who, Source::actor(who.actor())).await?;
    if let Ok(kept) = app.hosting.secrets() {
        kept.forget_environment(&id, &name)?;
    }
    Ok(Json(json!({ "removed": name, "said": removing(&name, was.vars.len(), was.secrets.len()) })))
}

/// Each of an environment's secrets with where it is named: by what is
/// running under the environment whose text names it, and by a service
/// started with it, so that what would fail is seen before one is taken
/// away. Its owner's to ask, as where the factory's are named is
/// (`environments:secrets`), and never with a value; and by an agent that
/// works under it, which is handed one marked for agents and one its own
/// slot names.
async fn secrets_named(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, name)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    who.require(SECRETS)?;
    project(&app, &id).await?;
    let environment = app.engine.environment_of(&id, &name).await?.ok_or_else(|| missing(&id, &name))?;
    let live = conductor::InstanceFilter {
        live_only: true,
        project: Some(id.clone()),
        limit: 100_000,
        ..conductor::InstanceFilter::default()
    };
    let under: Vec<conductor::InstanceInfo> = app
        .engine
        .instances(live)
        .await?
        .into_iter()
        .filter(|instance| instance.environment == name)
        .collect();
    let kept = app.hosting.secrets().ok();
    // The agents that work under it: each is handed what is marked for
    // agents, and what its own slot names that the environment lists.
    let slots: Vec<agents::Slot> = app
        .agents
        .slots()
        .await?
        .into_iter()
        .filter(|slot| slot.project.as_deref() == Some(id.as_str()) && slot.environment == name)
        .collect();
    let listed: Vec<Value> = environment
        .secrets
        .iter()
        .map(|(secret, listed)| {
            let instances: Vec<Value> = under
                .iter()
                .filter(|instance| super::secrets::names_in(&instance.source, &instance.params).contains(secret))
                .map(|instance| json!({ "id": instance.id, "blueprint": instance.blueprint, "key": instance.key }))
                .collect();
            let sought = conductor::environs::sought(&id, &name, secret);
            let services: Vec<Value> = app
                .hosting
                .services_handed(&sought)
                .into_iter()
                .map(|(service, variable)| json!({ "name": service.name, "instance": service.instance, "variable": variable }))
                .collect();
            let handed: Vec<Value> = slots
                .iter()
                .filter_map(|slot| {
                    let own = slot.secrets.iter().find(|(_, named)| *named == secret).map(|(variable, _)| variable);
                    own.or(listed.agents.as_ref()).map(|variable| json!({ "name": slot.name, "variable": variable }))
                })
                .collect();
            json!({
                "name": secret,
                "set": listed.set,
                "by_name": listed.by_name,
                "agents": listed.agents,
                "kept": kept.as_ref().is_some_and(|kept| kept.has_of(&id, &name, secret)),
                "named": { "instances": instances, "services": services, "agents": handed },
            })
        })
        .collect();
    Ok(Json(json!({ "project": id, "environment": name, "secrets": listed })))
}

/// The agents of `project` that work under its environment `name`, by name.
async fn agents_under(app: &App, project: &str, name: &str) -> kernel::Result<Vec<String>> {
    let slots = app.agents.slots().await?.into_iter();
    let under = slots.filter(|slot| slot.project.as_deref() == Some(project) && slot.environment == name);
    Ok(under.map(|slot| slot.name).collect())
}

/// The instances of `project` alive under its environment `name`, by key.
async fn alive_under(app: &App, project: &str, name: &str) -> kernel::Result<Vec<String>> {
    let live = conductor::InstanceFilter {
        live_only: true,
        project: Some(project.to_string()),
        limit: 100_000,
        ..conductor::InstanceFilter::default()
    };
    let instances = app.engine.instances(live).await?;
    Ok(instances
        .into_iter()
        .filter(|instance| instance.environment == name)
        .map(|instance| instance.key)
        .collect())
}

/// What taking an environment away does, in a sentence.
fn removing(name: &str, vars: usize, secrets: usize) -> String {
    let counted = |many: usize, one: &str, more: &str| match many {
        0 => None,
        1 => Some(format!("its one {one}")),
        many => Some(format!("its {many} {more}")),
    };
    let values = match secrets {
        0 => "",
        1 => ", with its value",
        _ => ", with their values",
    };
    let what = match (counted(vars, "variable", "variables"), counted(secrets, "secret", "secrets")) {
        (None, None) => "it has no variables and no secrets".to_string(),
        (Some(one), None) | (None, Some(one)) if vars + secrets == 1 => format!("{one} goes with it{values}"),
        (Some(those), None) | (None, Some(those)) => format!("{those} go with it{values}"),
        (Some(vars), Some(secrets)) => format!("{vars} and {secrets} go with it{values}"),
    };
    format!("The environment `{name}` is taken away for good: {what}. Nothing runs under it.")
}

/// Sets an environment's secret to `value`, in place of whatever it was,
/// and hands it to agents as `agents` where that is said.
async fn set_secret(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, environment, name)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // Before anything is read of what was sent: who may send it.
    who.require(SECRETS)?;
    project(&app, &id).await?;
    app.engine.environment_of(&id, &environment).await?.ok_or_else(|| missing(&id, &environment))?;
    if let Some(why) = conductor::environs::secret_refused(&name) {
        return Err(Error::invalid(why).into());
    }
    let value = super::required(&body, "value")?;
    let agents = handed_as(&body)?;
    // The value first: a stop before it is listed leaves a file nothing reads.
    app.hosting.secrets()?.set_of(&id, &environment, &name, value)?;
    let at = app.journal.now_ms();
    let change = |listed: &mut Environment| {
        let mut changed = listed.keep_secret(&name, &who, at)?;
        if let Some(agents) = agents {
            changed.secrets_marked = listed.mark_secret(&name, agents.as_deref())?.secrets_marked;
        }
        Ok(changed)
    };
    let source = Source::actor(who.actor());
    let (listed, changed) = app.engine.change_environment(&id, &environment, change, &who, source).await?;
    Ok(Json(json!({ "environment": shown(&app, &id, &listed, &who), "changed": changed })))
}

/// Takes an environment's secret away: off its list first, then its value.
async fn remove_secret(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, environment, name)): Path<(String, String, String)>,
) -> ApiResult<Json<Value>> {
    who.require(SECRETS)?;
    project(&app, &id).await?;
    let change = |listed: &mut Environment| listed.drop_secret(&name);
    let source = Source::actor(who.actor());
    let (listed, changed) = app.engine.change_environment(&id, &environment, change, &who, source).await?;
    app.hosting.secrets()?.remove_of(&id, &environment, &name)?;
    Ok(Json(json!({ "environment": shown(&app, &id, &listed, &who), "changed": changed })))
}

/// Says what an agent is handed an environment's secret as: `agents`, a
/// variable, or nothing for not at all.
async fn mark_secret(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, environment, name)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require(SECRETS)?;
    project(&app, &id).await?;
    let agents = handed_as(&body)?.unwrap_or(None);
    let change = |listed: &mut Environment| listed.mark_secret(&name, agents.as_deref());
    let source = Source::actor(who.actor());
    let (listed, changed) = app.engine.change_environment(&id, &environment, change, &who, source).await?;
    Ok(Json(json!({ "environment": shown(&app, &id, &listed, &who), "changed": changed })))
}

/// What `agents` of a body says: not said, said as nothing (not handed),
/// or a variable.
fn handed_as(body: &Value) -> kernel::Result<Option<Option<String>>> {
    match body.get("agents") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(variable)) => Ok(Some(Some(variable.clone()))),
        Some(_) => Err(Error::invalid(
            "`agents` is the variable an agent is handed the secret as, such as `STRIPE_KEY`, or null for none",
        )),
    }
}

pub(super) fn missing(project: &str, name: &str) -> Error {
    Error::not_found(format!(
        "`{project}` has no environment called `{name}`. `{} project environment {project}` lists those it has",
        brand::CLI
    ))
}

/// Says what an environment holds to (`environments::rules`): who
/// approves what goes under it (`approve`, each a person by their id, name
/// or email, or `user:<id>`; an empty list for nobody), and which branches
/// may be run under it (`branches`, each a branch's name or a beginning of
/// names ended by `*`; an empty list for any). Either or both; what is not
/// said is left as it is. For whoever may change its secrets, since what
/// it holds to is what stands between a run and what it holds.
async fn say_rules(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, environment)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require(SECRETS)?;
    project(&app, &id).await?;
    let listed = |key: &str| match body.get(key) {
        None => Ok(None),
        Some(Value::Array(said)) => Ok(Some(
            said.iter().map(|one| one.as_str().unwrap_or_default().trim().to_string()).collect::<Vec<_>>(),
        )),
        Some(_) => Err(Error::invalid(format!("`{key}` is a list, empty for none"))),
    };
    let (said, branches) = (listed("approve")?, listed("branches")?);
    if said.is_none() && branches.is_none() {
        return Err(Error::invalid(
            "say `approve` (a list of people, empty for nobody), `branches` (a list of branches, empty for any), or both",
        )
        .into());
    }
    let accounts = app.studio.identity.accounts().await?;
    let mut approve = Vec::new();
    for word in said.iter().flatten() {
        let bare = word.trim_start_matches("user:").trim_start_matches('@');
        let found = accounts.iter().find(|account| {
            account.id == bare
                || account.name.eq_ignore_ascii_case(bare)
                || account.email.eq_ignore_ascii_case(bare)
        });
        match found {
            Some(account) => approve.push(format!("user:{}", account.id)),
            None => {
                return Err(Error::not_found(format!(
                    "nobody here is `{word}`: who approves is a person with an account, by their id, name or email"
                ))
                .into());
            }
        }
    }
    let change = |listed: &mut Environment| {
        let mut changed = match said.is_some() {
            true => listed.say_approve(&approve)?,
            false => Default::default(),
        };
        if let Some(branches) = &branches {
            changed.rules |= listed.say_branches(branches)?.rules;
        }
        Ok(changed)
    };
    let source = Source::actor(who.actor());
    let (listed, changed) = app.engine.change_environment(&id, &environment, change, &who, source).await?;
    let under = super::deploys::under(&app, &who, &id).await;
    let shown =
        super::deploys::with_ran(&app, &who, &id, under.as_ref(), shown(&app, &id, &listed, &who)).await;
    Ok(Json(json!({ "environment": shown, "changed": changed })))
}
