//! An environment loaded from a branch: the branches of a project's
//! repository to choose from, fetched fresh when a person asks, the
//! blueprint an environment says it is loaded by, and Run.
//!
//! What a deploy does is the blueprint's, the project's own or one that
//! ships, and nothing here knows it: Run applies that blueprint for the
//! project under the environment, as whoever pressed it, through the one
//! door every apply goes through (`Engine::apply`), handed the branch as
//! its param `branch` (and the commit as `commit`, where it declares one).
//! It is applied under a key of the environment's own (`deploy-<env>`), so
//! that each environment has its one deploy, and with its steps run again
//! each time (`force`) taking the project's own blueprints as they are
//! then (`take`): Run is somebody asking, now. What the environment was
//! last loaded with is kept with it (`Environment::ran`); how that went
//! is the run's to say, and is read from the run as the environment is
//! shown ([`with_ran`]), never kept a second time.
//!
//! Git is asked the one way there is (`kernel::git`). Listing reads the
//! repository; fetching speaks to its remotes, which is the network, and
//! is done only when a person presses for it, as whoever runs the daemon,
//! with nobody prompted: a remote that wants a password is said to have
//! refused, in git's words. A fetch runs what the repository's settings
//! name for reaching a remote (a program to connect with, a helper), so it
//! takes `shell` beside writing the project, as a command does.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use axum::extract::{Path as Route, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use conductor::Apply;
use conductor::environs::{Environment, Ran, last};
use kernel::git::{self, Branch};
use kernel::{Error, Fields, Source};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/projects/{id}/branches", get(branches))
        .route("/projects/{id}/branches/fetch", post(fetch))
        .route("/projects/{id}/environments/{environment}/deploy", put(say_deploy))
        .route("/projects/{id}/environments/{environment}/run", post(run))
}

/// How long git is given to list a repository's branches, and to fetch.
const LISTING: Duration = Duration::from_secs(20);
const FETCHING: Duration = Duration::from_secs(300);

/// The key an environment's deploy is applied under in its project.
pub fn key_of(environment: &str) -> String {
    format!("deploy-{environment}")
}

/// The project's folder where it is a repository of its own; why not where
/// it is not. A folder inside another repository is not one: git asked
/// there would answer for the repository around it.
fn repository(app: &App, project: &str) -> Result<PathBuf, Error> {
    let dir = app
        .studio
        .projects
        .dir_of(project)
        .ok_or_else(|| Error::not_found(format!("there is no project `{project}`")))?;
    match dir.join(".git").exists() {
        true => Ok(dir),
        false => Err(Error::invalid(format!(
            "`{project}` is no repository: its folder has no git history of its own, so it has no branch to run. \
             Make it one (`git init` in {}), or add the project from a repository",
            dir.display()
        ))),
    }
}

/// What git said, as text, or why it said nothing.
struct Said {
    ok: bool,
    out: String,
    err: String,
}

impl Said {
    /// The last of what it said of why, in its words.
    fn why(&self) -> String {
        let said = self.err.lines().rev().find(|line| !line.trim().is_empty());
        said.or_else(|| self.out.lines().last()).unwrap_or("git ended without saying why").trim().to_string()
    }
}

/// Asks git `asked` of the repository in `dir`, for at most `within`.
async fn ask(dir: &Path, asked: &[&str], within: Duration) -> Result<Said, Error> {
    let mut command = tokio::process::Command::from(git::git(dir));
    command.args(asked).kill_on_drop(true);
    match tokio::time::timeout(within, command.output()).await {
        Ok(Ok(output)) => Ok(Said {
            ok: output.status.success(),
            out: String::from_utf8_lossy(&output.stdout).into_owned(),
            err: String::from_utf8_lossy(&output.stderr).into_owned(),
        }),
        Ok(Err(err)) => {
            Err(Error::unavailable(format!("git could not be started: {err}. Is git installed?")))
        }
        Err(_) => Err(Error::unavailable(format!(
            "git did not answer `git {}` in {} seconds",
            asked.first().copied().unwrap_or_default(),
            within.as_secs()
        ))),
    }
}

/// The branches of the repository in `dir`, local and as each remote holds them.
async fn listed(dir: &Path) -> Result<Vec<Branch>, Error> {
    let said = ask(dir, git::BRANCHES, LISTING).await?;
    match said.ok {
        true => Ok(git::branches(&said.out)),
        false => Err(Error::unavailable(format!("git could not list the branches: {}", said.why()))),
    }
}

/// The project's branches: for whoever may read the project. One that is
/// no repository says so, and has none.
async fn branches(
    State(app): State<App>,
    Auth(who): Auth,
    Route(id): Route<String>,
) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    app.studio.projects.get(&id).await?;
    match repository(&app, &id) {
        Ok(dir) => {
            // Its remotes by name, so that whoever shows Fetch shows it only where there is one to ask.
            let remotes = ask(&dir, &["remote"], LISTING).await?;
            let remotes: Vec<&str> =
                remotes.out.lines().map(str::trim).filter(|name| !name.is_empty()).collect();
            let branches = listed(&dir).await?;
            Ok(Json(json!({ "project": id, "repository": true, "remotes": remotes, "branches": branches })))
        }
        Err(not) => Ok(Json(
            json!({ "project": id, "repository": false, "remotes": [], "branches": [], "said": not.message }),
        )),
    }
}

/// Asks every remote of the project's repository what it has now, as a
/// person pressed for it, and says what that brought: by the remotes'
/// branches before and after, never by what git printed.
async fn fetch(State(app): State<App>, Auth(who): Auth, Route(id): Route<String>) -> ApiResult<Json<Value>> {
    who.require("projects:write")?;
    who.require("shell")?;
    app.studio.projects.get(&id).await?;
    let dir = repository(&app, &id)?;
    let remotes = ask(&dir, &["remote"], LISTING).await?;
    if remotes.out.trim().is_empty() {
        return Err(Error::invalid(format!(
            "`{id}`'s repository has no remote to fetch from: its branches are all here (`git remote add origin <address>` gives it one)"
        ))
        .into());
    }
    let before = listed(&dir).await?;
    let started = Instant::now();
    let mut said =
        ask(&dir, &["fetch", "--all", "--prune", "--no-recurse-submodules"], LISTING.max(FETCHING)).await?;
    // Another git holding a lock had done nothing: it is asked again, for a while.
    // In the world's time, as that git keeps it (`git::Waited`).
    let mut waited = git::Waited::default();
    while let Some(lock) = git::lock_met(&said.err) {
        if waited.spent() {
            said.err = format!("{}\n{}", said.err, git::lock_kept(&lock));
            break;
        }
        waited.moment().await;
        said = ask(&dir, &["fetch", "--all", "--prune", "--no-recurse-submodules"], FETCHING).await?;
    }
    let took = started.elapsed().as_secs_f64() * 1000.0;
    app.journal.measure("project.fetched", &[("project", id.as_str())], took, said.ok);
    if !said.ok {
        return Err(Error::unavailable(format!(
            "`{id}`'s remotes could not be asked what they have: {}. Git is asked as whoever runs the daemon and \
             prompts nobody, so a remote that wants a password or a key is reached only where git can sign in \
             without asking (a key of its own, or a helper it is told of)",
            said.why()
        ))
        .into());
    }
    let after = listed(&dir).await?;
    let came = git::fetched(&before, &after);
    let remotes: Vec<&str> = remotes.out.lines().map(str::trim).filter(|name| !name.is_empty()).collect();
    Ok(Json(
        json!({ "project": id, "said": came.said(), "fetched": came, "remotes": remotes, "branches": after }),
    ))
}

/// Says the blueprint an environment is loaded by, or none
/// (`{ "blueprint": null }`): for whoever may change the environment. A
/// name is taken only where there is a blueprint of that name for the
/// project, so that a name misspelt is said now and not at Run.
async fn say_deploy(
    State(app): State<App>,
    Auth(who): Auth,
    Route((id, name)): Route<(String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    super::environments::may_change(&who)?;
    app.studio.projects.get(&id).await?;
    let blueprint = match body.get("blueprint") {
        None => return Err(Error::invalid("`blueprint` is needed: its name, or null for none").into()),
        Some(Value::Null) => None,
        Some(Value::String(said)) if said.trim().is_empty() => None,
        Some(Value::String(said)) => Some(said.trim().to_string()),
        Some(_) => return Err(Error::invalid("`blueprint` is a blueprint's name, or null for none").into()),
    };
    if let Some(blueprint) = &blueprint {
        if let Some(why) = conductor::environs::deploy_refused(blueprint) {
            return Err(Error::invalid(why).into());
        }
        if app.engine.library().load(blueprint, Some(&id)).is_none() {
            return Err(Error::not_found(format!(
                "there is no blueprint `{blueprint}` for `{id}`: write one among the project's own blueprints, \
                 or name one that ships, such as `std/board`"
            ))
            .into());
        }
    }
    let change = |environment: &mut Environment| environment.say_deploy(blueprint.as_deref());
    let source = Source::actor(who.actor());
    let (environment, changed) = app.engine.change_environment(&id, &name, change, &who, source).await?;
    let under = under(&app, &who, &id).await;
    let shown = super::environments::shown(&app, &id, &environment, &who);
    let shown = with_ran(&app, &who, &id, under.as_ref(), shown).await;
    Ok(Json(json!({ "environment": shown, "changed": changed })))
}

/// Loads an environment from a branch: its deploy blueprint applied for
/// the project under it, as whoever pressed Run, handed the branch.
async fn run(
    State(app): State<App>,
    Auth(who): Auth,
    Route((id, name)): Route<(String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // What applying takes, before anything is read: the apply asks the rest.
    who.require("blueprints:run")?;
    who.require("projects:read")?;
    app.studio.projects.get(&id).await?;
    let environment = app.engine.environment_of(&id, &name).await?.ok_or_else(|| missing(&id, &name))?;
    let Some(blueprint) = environment.deploy.clone() else {
        return Err(Error::invalid(format!(
            "`{name}` says no blueprint it is loaded by, so there is nothing to run: `{} project environment {id} \
             {name} deploy <blueprint>` says one",
            brand::CLI
        ))
        .into());
    };
    let branch = super::required(&body, "branch")?.to_string();
    let dir = repository(&app, &id)?;
    let branches = listed(&dir).await?;
    let chosen = branches.iter().find(|one| one.name == branch).ok_or_else(|| {
        Error::not_found(format!(
            "`{id}` has no branch `{branch}`. `{} project branches {id}` lists those it has, and `--fetch` asks \
             its remotes what they have now",
            brand::CLI
        ))
    })?;
    // The branch is handed as `branch`, which the blueprint has to say it
    // takes: one that does not would be refused by the apply in words that
    // say nothing of why it was handed one.
    let source = app.engine.library().load(&blueprint, Some(&id)).ok_or_else(|| {
        Error::not_found(format!(
            "there is no blueprint `{blueprint}` for `{id}` any more, which `{name}` says it is loaded by: write it \
             again, or say another (`{} project environment {id} {name} deploy <blueprint>`)",
            brand::CLI
        ))
    })?;
    let declared: Vec<String> = app
        .engine
        .check(&source, Some(&id))
        .blueprint
        .map(|text| text.params.into_iter().map(|param| param.name).collect())
        .unwrap_or_else(|| vec!["branch".to_string()]);
    if !declared.iter().any(|param| param == "branch") {
        return Err(Error::invalid(format!(
            "`{blueprint}` does not declare a param `branch`, which running an environment hands it: say \
             `branch: {{ type: string }}` under its `params:`, and read it as `${{params.branch}}`"
        ))
        .into());
    }
    let mut params = Fields::new();
    params.insert("branch".to_string(), json!(chosen.name));
    if declared.iter().any(|param| param == "commit") {
        params.insert("commit".to_string(), json!(chosen.commit));
    }
    let key = key_of(&name);
    let request = Apply {
        params,
        key: Some(key.clone()),
        project: Some(id.clone()),
        force: true,
        take: true,
        environment: Some(name.clone()),
        ..Apply::new(blueprint.clone(), who.clone())
    };
    let applied = app.engine.apply(request).await?;
    let ran = Ran {
        branch: chosen.name.clone(),
        commit: chosen.commit.clone(),
        blueprint,
        at: app.journal.now_ms(),
        by: who.actor(),
        by_name: who.name.clone(),
        instance: applied.instance.clone(),
        run: applied.run.clone(),
        ended: None,
        down: None,
    };
    let environment = app.engine.environment_ran(&id, &name, ran, Source::actor(who.actor())).await?;
    let under = under(&app, &who, &id).await;
    let shown = super::environments::shown(&app, &id, &environment, &who);
    let shown = with_ran(&app, &who, &id, under.as_ref(), shown).await;
    // Which instance that is, as everything that names one says it.
    let mut applied = json!(applied);
    (applied["blueprint"], applied["key"]) = (json!(environment.deploy), json!(key));
    Ok(Json(json!({ "applied": applied, "environment": shown })))
}

/// An environment as it is shown to `who`, with how what it was last
/// loaded with went, read from the run as it is handed to them
/// (`handed::run`): `status`, when it ended, and why it failed where they
/// may be told. A run that is no longer there says nothing of it. Beside
/// it `last`, what it last ran in words (`environments::last`), and `runs`,
/// the runs that acted under it, from what was read of the project for
/// them ([`under`]).
///
/// What the environment keeps of its last run is written once the apply
/// has answered, and a daemon stopped between the two leaves it naming the
/// run before. So the run of its deploy that is there (the newest of its
/// key's instance) stands over what is kept where it is newer
/// ([`newer_than_kept`]): what is shown is never older than what ran.
pub async fn with_ran(
    app: &App,
    who: &kernel::Principal,
    project: &str,
    under: Option<&last::Read>,
    mut shown: Value,
) -> Value {
    if let Some(newer) = newer_than_kept(app, project, &shown).await {
        shown["ran"] = newer;
    }
    if let Some(run) = shown["ran"]["run"].as_str().map(str::to_string)
        && let Ok(read) = app.engine.run(&run).await.map(|read| crate::handed::run(app, who, read))
    {
        shown["ran"]["status"] = read["status"].clone();
        shown["ran"]["finished"] = read["finished"].clone();
        shown["ran"]["error"] = read["error"].clone();
    }
    let ran: Option<Ran> = serde_json::from_value(shown["ran"].clone()).ok();
    let loaded = ran.as_ref().map(|ran| (ran, shown["ran"]["status"].as_str()));
    let name = shown["name"].as_str().unwrap_or_default().to_string();
    let name = name.as_str();
    let said = last::last(project, name, shown["deploy"].as_str(), loaded, under);
    shown["last"] = json!(said);
    // The runs that acted under it, the newest first, each with its own
    // page: only to whoever may be told what runs, as `under` is read.
    if let Some(read) = under {
        shown["runs"] = json!(last::runs(name, read));
    }
    shown
}

/// What is read of `project` once to say what each of its environments
/// last ran: its latest runs, and the instances alive and lately made,
/// each as far as `last` bounds it. Only for whoever may be told what
/// runs (`blueprints:read`, as listing it takes); nothing for anybody else.
pub async fn under(app: &App, who: &kernel::Principal, project: &str) -> Option<last::Read> {
    if !who.can("blueprints:read") {
        return None;
    }
    let of = |live_only| conductor::InstanceFilter {
        live_only,
        project: Some(project.to_string()),
        limit: last::THINGS,
        ..conductor::InstanceFilter::default()
    };
    let newest = app.engine.instances(of(false)).await.ok()?;
    let runs = app.engine.recent_runs(last::RUNS, Some(project.to_string())).await.ok()?;
    let whole = newest.len() < last::THINGS && runs.len() < last::RUNS;
    let mut seen = std::collections::HashSet::new();
    let things = app.engine.instances(of(true)).await.ok()?.into_iter().chain(newest);
    let things = things.filter(|one| seen.insert(one.id.clone())).map(|one| last::Thing {
        live: one.status.is_live(),
        by_name: one.principal.name,
        id: one.id,
        parent: one.parent,
        environment: one.environment,
        blueprint: one.blueprint,
        key: one.key,
    });
    let runs = runs.into_iter().map(|run| last::Once {
        status: run.status.as_str().to_string(),
        id: run.id,
        instance: run.instance,
        started: run.started,
    });
    Some(last::Read { runs: runs.collect(), things: things.collect(), whole })
}

/// The run of an environment's deploy that is newer than what the
/// environment keeps of its last, said as what it keeps is: the newest
/// instance under its key (`deploy-<env>`) of the blueprint it says or
/// last ran, and the newest run of that instance's steps, where that is
/// not the run kept. The branch is what the instance was handed, and the
/// commit too where it takes one; otherwise the commit is not known, and
/// is said as nothing.
async fn newer_than_kept(app: &App, project: &str, shown: &Value) -> Option<Value> {
    let key = key_of(shown["name"].as_str()?);
    let kept = &shown["ran"];
    let mut blueprints: Vec<&str> =
        [shown["deploy"].as_str(), kept["blueprint"].as_str()].into_iter().flatten().collect();
    blueprints.dedup();
    let mut newest: Option<conductor::InstanceInfo> = None;
    for blueprint in blueprints {
        let filter = conductor::InstanceFilter {
            project: Some(project.to_string()),
            blueprint: Some(blueprint.to_string()),
            roots_only: true,
            limit: 50,
            ..conductor::InstanceFilter::default()
        };
        for one in app.engine.instances(filter).await.ok()? {
            if one.key == key && newest.as_ref().is_none_or(|was| one.created > was.created) {
                newest = Some(one);
            }
        }
    }
    let instance = newest?;
    let runs = app.engine.runs(&instance.id, 20).await.ok()?;
    let run = runs.into_iter().find(|run| run.flow == "main")?;
    let same = kept["instance"].as_str() == Some(instance.id.as_str())
        && kept["run"].as_str() == Some(run.id.as_str());
    if same {
        return None;
    }
    let param =
        |name: &str| instance.params.get(name).and_then(Value::as_str).unwrap_or_default().to_string();
    Some(json!({
        "branch": param("branch"),
        "commit": param("commit"),
        "blueprint": instance.blueprint,
        "at": run.started,
        "by": instance.principal.actor(),
        "by_name": instance.principal.name,
        "instance": instance.id,
        "run": run.id,
    }))
}

fn missing(project: &str, name: &str) -> Error {
    Error::not_found(format!(
        "`{project}` has no environment called `{name}`. `{} project environment {project}` lists those it has",
        brand::CLI
    ))
}
