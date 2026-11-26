//! Projects and their files; blueprints, instances and runs.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use conductor::{Apply, InstanceFilter};
use kernel::Error;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{required, text};
use crate::handed;
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/projects", get(projects).post(project_create))
        .route("/projects/{id}", get(project).patch(project_rename).delete(project_forget))
        .route("/projects/{id}/trust", post(project_trust))
        .route("/projects/{id}/fleet", post(fleet_init))
        .route("/projects/{id}/starts", post(project_starts))
        .route("/starts", get(starts_settings))
        .route("/projects/{id}/scan", post(scan))
        .route("/pages/preview", post(page_preview))
        .route("/blueprints", get(blueprints))
        .route("/blueprints/used", get(blueprints_used))
        .route("/blueprints/source", get(blueprint_source))
        .route("/blueprints/check", post(check))
        .route("/blueprints/format", post(lay_out))
        .route("/blueprints/language", get(language))
        .route("/blueprints/language/used", get(language_used))
        .route("/blueprints/language/scenarios", get(language_scenarios))
        .route("/blueprints/schema", get(schema))
        .route("/actions", get(actions))
        .route("/apply", post(apply))
        .route("/instances", get(instances))
        .route("/instances/{id}", get(instance))
        .route("/instances/{id}/versions", get(instance_versions))
        .route("/instances/{id}/apply", post(instance_apply))
        .route("/instances/{id}/stop", post(instance_stop))
        .route("/runs", get(runs))
        .route("/runs/{id}", get(run))
        .route("/runs/{id}/steps/{step}/log", get(step_log))
}

async fn projects(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    Ok(Json(json!({ "projects": app.studio.projects.list().await? })))
}

async fn project_create(
    State(app): State<App>,
    Auth(who): Auth,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let path = text(&body, "path").map(std::path::PathBuf::from);
    // Read before anything is made: a request that names something no
    // project starts with adds nothing.
    let asked = crate::usual::asked(&app.engine, &body).await?;
    let documents = crate::usual::documents(asked.as_deref());
    // And the workspace it is to be in, where one was said: a project is
    // not made for one there is not. With none said it is in the usual
    // one. Either way its place is written with its row.
    let workspace = match text(&body, "workspace") {
        Some(workspace) => {
            who.require("projects:write")?;
            Some(app.studio.workspaces.get(workspace).await?.id)
        }
        None => None,
    };
    let (name, projects) = (required(&body, "name")?, &app.studio.projects);
    let (project, first_commit) =
        projects.create_begun(name, path, &who, documents, workspace.as_deref()).await?;
    let mut answer = json!(project);
    // Whether what the factory started it with is its first commit, and
    // what to do where it is not.
    if let Some(first_commit) = first_commit {
        answer["first_commit"] = json!(first_commit);
    }
    if let Some(workspace) = app.studio.workspaces.of_project(&project.id).await? {
        answer["workspace"] = json!(workspace);
    }
    // What it is to start with, where anything was asked for: each
    // applied as whoever added the project. What it was not given is
    // said, and is no reason for the project not to have been added.
    if let Some(asked) = asked {
        answer["set_up"] = crate::usual::give(&app.engine, &project, &who, &asked).await;
    }
    Ok(Json(answer))
}

async fn project(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    Ok(Json(json!(app.studio.projects.get(&id).await?)))
}

/// Calls a project something else: `{ "name": ... }`. Its id is as it was.
async fn project_rename(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let name = body
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::invalid("`name` is needed: what the project is to be called"))?;
    Ok(Json(json!(app.studio.projects.rename(&id, name, &who).await?)))
}

/// Says whether a project's own files may stand in for the blueprints
/// that ship: `{ "trusted": true }`, or `false` to take it back.
async fn project_trust(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let trusted = body
        .get("trusted")
        .and_then(Value::as_bool)
        .ok_or_else(|| Error::invalid("`trusted` is needed: true, or false to take it back"))?;
    Ok(Json(json!(app.studio.projects.trust(&id, trusted, &who).await?)))
}

async fn project_forget(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    // With where it was: its channels are its workspace's own from the
    // write that forgets it, and no workspace lists it.
    let channels = channels_of(&app, &id, &who).await;
    // Deleted at once, it takes what runs for it and its agents with it:
    // whoever deletes it is asked what that takes before anything goes.
    let bin = crate::trash::of(&app);
    if bin.kept().await.at_once() {
        who.require("projects:write")?;
        bin.may_take_down(&id, &who).await?;
    }
    let trash = app.studio.projects.forget(&id, &who).await?;
    if trash.is_none() {
        // Deleted at once, as the factory keeps nothing that is deleted:
        // its environments and forks go with it now.
        crate::trash::of(&app).project_gone(&id).await?;
    }
    let mut running = still_running(&app, &id, &who).await;
    if let Some(channels) = channels {
        running["channels"] = channels;
    }
    // What it is in the Trash, where it is kept to be given back.
    let mut answer = crate::trash::moved(&app, trash).await;
    answer["running"] = running;
    Ok(Json(answer))
}

/// What goes on for a project that was just forgotten: what was applied
/// for it and is alive, and the agents that are its and are up. The
/// channels that were its are read before it is forgotten
/// ([`channels_of`]).
///
/// Forgetting a project stops nothing, and is not refused for what runs:
/// a project is let go of and taken up again with its fleet where it was,
/// and a folder is forgotten so as to be added under another name. But it
/// said only that the files were where they were, and a project forgotten
/// with its fleet up left a fleet's instances, its agents and its channel
/// with nothing to say whose they had been. So the answer says them, each
/// to whoever may read that kind of thing and nothing of it to anybody
/// else: what holds `projects:write` need not hold the rest.
async fn still_running(app: &App, project: &str, who: &kernel::Principal) -> Value {
    let mut still = json!({});
    if who.can("blueprints:read") {
        // What somebody applied for it: stopping one stops what it started.
        let filter = InstanceFilter {
            live_only: true,
            roots_only: true,
            project: Some(project.to_string()),
            limit: 200,
            ..InstanceFilter::default()
        };
        let applied = app.engine.instances(filter).await.unwrap_or_default();
        let one = |one: &conductor::InstanceInfo| json!({ "id": one.id, "blueprint": one.blueprint, "key": one.key });
        still["instances"] = json!(applied.iter().map(one).collect::<Vec<_>>());
    }
    if who.can("agents:read") {
        let slots = app.agents.slots().await.unwrap_or_default();
        let its = slots.iter().filter(|slot| slot.active && slot.project.as_deref() == Some(project));
        still["agents"] = json!(its.map(|slot| slot.name.clone()).collect::<Vec<_>>());
    }
    still
}

/// The channels that are a project's, by what each is called: only those
/// whoever asks reads, which is asked of the channels themselves, and
/// nothing where they read none. Read before the project is forgotten:
/// from that write they are its workspace's own and say no project.
async fn channels_of(app: &App, project: &str, who: &kernel::Principal) -> Option<Value> {
    let read = app.studio.channels.readable(who).await.ok()?;
    let its = read.iter().filter(|channel| channel.project.as_deref() == Some(project));
    Some(json!(its.map(|channel| channel.name.clone()).collect::<Vec<_>>()))
}

async fn fleet_init(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.projects.init_fleet(&id, &who).await?)))
}

/// What each row a project may be set up with may be handed as the
/// project is added: its blueprint's params, less what it is told of the
/// project (`crate::usual::settings`), for the dialog that adds one to
/// draw a form of under each, as the drawer that applies a blueprint
/// draws one. They are what reading the blueprints says: of the
/// factory's default, or of a setup it keeps (`?setup=<name>`).
async fn starts_settings(
    State(app): State<App>,
    Auth(who): Auth,
    Query(asked): Query<StartsQuery>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let rows = match asked.setup {
        Some(setup) => crate::usual::of_setup(&app.engine, &setup).await?,
        None => crate::usual::the_usual(&app.engine).await?,
    };
    Ok(Json(crate::usual::settings(&app.engine, &rows)))
}

#[derive(Deserialize)]
struct StartsQuery {
    setup: Option<String>,
}

/// Gives a project that is there what it may be set up with (`{ "names":
/// [...] }`, of the factory's default's rows, or rows of its own): each
/// applied as whoever asks, as it is when a project is added with it. What
/// the project has already is said to be there and is not applied again,
/// and what was left out is said with why.
async fn project_starts(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // Before anything is said of the project, or of what was named: this is applying.
    who.require("blueprints:run")?;
    let asked = crate::usual::asked_later(&app.engine, &body).await?;
    let project = app.studio.projects.get(&id).await?;
    Ok(Json(crate::usual::give_later(&app.engine, &project, &who, &asked).await?))
}

async fn scan(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    Ok(Json(json!({ "changes": app.studio.projects.scan(&id).await? })))
}

#[derive(Deserialize)]
struct LibraryQuery {
    project: Option<String>,
    name: Option<String>,
}

async fn blueprints(
    State(app): State<App>,
    Auth(who): Auth,
    Query(query): Query<LibraryQuery>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let project = query.project.as_deref();
    let library = app.engine.library();
    let names = library.names(project);
    // Which of those that are no project's runs which, so that a list
    // can show what somebody applies and under each its parts.
    let listed = conductor::library::listed::listed(library.as_ref(), &app.engine.registry().all_specs());
    // Each with what it is started with and gives back, which is what
    // somebody calling it from another blueprint needs to know, and with
    // the file it is when it is the project's own: the others ship with
    // the product. One that does not read is still listed, by its name.
    let described: Vec<Value> = names
        .iter()
        .map(|name| {
            // A file of the project's own that is passed over for the one
            // that ships is not what this name is there.
            let file = project
                .filter(|_| library.passed_over(name, project).is_none())
                .and_then(|id| crate::library::own_file(&app.own, name, id));
            let read = library
                .load(name, project)
                .and_then(|source| blueprint::check(&source, &blueprint::NoCatalog).blueprint);
            let mut said = match read {
                Some(read) => {
                    let drawn = blueprint::graph(&read);
                    json!({
                        "name": name,
                        "file": file,
                        "description": drawn.description,
                        "params": drawn.params,
                        "outputs": read.outputs.keys().collect::<Vec<_>>(),
                    })
                }
                None => {
                    json!({ "name": name, "file": file, "description": null, "params": [], "outputs": [] })
                }
            };
            // What it runs is said of one that is no project's own, and
            // that it is applied though something runs it only where it is.
            if let Some(runs) = listed.runs.get(name).filter(|_| file.is_none()) {
                said["runs"] = json!(runs);
                if listed.applied.contains(name) {
                    said["applied"] = json!(true);
                }
            }
            said
        })
        .collect();
    Ok(Json(json!({ "names": names, "blueprints": described })))
}

/// What each project uses of the blueprints there are: the ones applied
/// there that are still alive, at any depth, and the ones that are files of
/// its own. This is what a project's list of blueprints is; every blueprint
/// the installation has is asked for with no project.
async fn blueprints_used(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let mut projects = serde_json::Map::new();
    for project in app.studio.projects.list().await? {
        // Its own are named as they are called, and each with its file.
        // One that is passed over for what ships is not what the project
        // uses, and is not listed as if it were.
        let library = app.engine.library();
        // With each, the files of the project's own it links, which are listed under it.
        let own: Vec<Value> = crate::library::own_names(&app.own, &project.id)
            .into_iter()
            .filter(|name| library.passed_over(name, Some(&project.id)).is_none())
            .filter_map(|name| {
                let file = crate::library::own_file(&app.own, &name, &project.id)?;
                let links = crate::library::own_links(&app.own, &name, &project.id);
                Some(json!({ "name": name, "file": file, "links": links }))
            })
            .collect();
        // Where a new one of its own is written, as a path of the project's: nothing where it keeps none.
        let written = app.own.blueprints_of(&project.id);
        projects.insert(project.id, json!({ "applied": [], "own": own, "written": written }));
    }
    for (project, blueprint) in app.engine.in_use().await? {
        // An instance may outlive its project; what it runs is then not
        // any project's.
        if let Some(applied) = projects.get_mut(&project).and_then(|uses| uses["applied"].as_array_mut()) {
            applied.push(Value::String(blueprint));
        }
    }
    Ok(Json(json!({ "projects": projects })))
}

async fn blueprint_source(
    State(app): State<App>,
    Auth(who): Auth,
    Query(query): Query<LibraryQuery>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let name = query.name.ok_or_else(|| Error::invalid("`name` is needed"))?;
    let (library, project) = (app.engine.library(), query.project.as_deref());
    // A name with more than one file is said to be that, not to be nobody's.
    let source = library.load(&name, project).ok_or_else(|| match library.refused(&name, project) {
        Some(why) => Error::conflict(why),
        None => Error::not_found(format!("there is no blueprint called `{name}`")),
    })?;
    Ok(Json(json!({ "name": name, "source": source })))
}

/// Checks blueprint text and draws it. This is what the editor calls as you
/// type and what the node view is drawn from; nothing is run.
/// What a text lists under `permissions` that `name`, holding `held`,
/// does not hold: a warning, where its `permissions` are written. A
/// permission is not given by being listed: an instance has no more than
/// whoever applied it, so it runs without, and that was said nowhere
/// until a step that needed it failed. Listed as an area, a permission
/// is held where any part of the area is: the instance has that part.
fn unheld(
    source: &str,
    blueprint: Option<&blueprint::Blueprint>,
    name: &str,
    held: &[String],
) -> Option<blueprint::Diagnostic> {
    let blueprint = blueprint?;
    let missing = |listed: &&String| {
        let part = format!("{listed}:");
        !kernel::principal::covers(held, listed) && !held.iter().any(|one| one.starts_with(&part))
    };
    let lacks: Vec<String> =
        blueprint.permissions.iter().flatten().filter(missing).map(|listed| format!("`{listed}`")).collect();
    if lacks.is_empty() {
        return None;
    }
    // Where the list is written, at the top of the text; its start, if
    // that cannot be found.
    let line = source.lines().position(|line| line.starts_with("permissions:"));
    let span = line.map_or(blueprint.span, |line| {
        let at = |col: usize| blueprint::Pos { line: line as u32 + 1, col: col as u32 };
        blueprint::Span { start: at(1), end: at("permissions".len() + 1) }
    });
    let said = format!(
        "this lists {} under `permissions`, which {name} does not hold: an instance has no more than whoever \
         applied it, so it runs without, and a step that needs it will fail",
        lacks.join(", ")
    );
    Some(blueprint::Diagnostic::warning("unheld-permission", said, span))
}

/// What a text applied for `project` listens for and will never hear,
/// each as a warning where the handler or the `wait` is written
/// (`conductor::whose`): one that says no `scope` hears its own
/// project's events and what is nobody's.
fn unheard(
    app: &App,
    blueprint: Option<&blueprint::Blueprint>,
    project: Option<&str>,
) -> Vec<blueprint::Diagnostic> {
    let Some(blueprint) = blueprint else { return Vec::new() };
    conductor::whose::unheard(blueprint, project, app.engine.registry().own_events())
        .into_iter()
        .map(|one| blueprint::Diagnostic::warning("unheard", one.said, one.span))
        .collect()
}

async fn check(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let source =
        body.get("source").and_then(Value::as_str).ok_or_else(|| Error::invalid("`source` is needed"))?;
    let mut checked = app.engine.check(source, text(&body, "project"));
    // What it would never hear, where the check says which project it is
    // for: with none said, nothing is known of where it will be applied,
    // and an apply for no project says so itself.
    if let Some(project) = text(&body, "project") {
        let silent = unheard(&app, checked.blueprint.as_ref(), Some(project));
        checked.diagnostics.extend(silent);
    }
    // Said to whoever is checking, of what they hold: applied by them it
    // would run without what it lists and they have not got.
    if let Some(said) = unheld(source, checked.blueprint.as_ref(), &who.name, &who.scopes) {
        checked.diagnostics.insert(0, said);
    }
    Ok(Json(json!({
        "ok": !checked.has_errors(),
        "diagnostics": checked.diagnostics,
        "name": checked.blueprint.as_ref().map(|blueprint| blueprint.name.clone()),
        "graph": checked.blueprint.as_ref().map(blueprint::graph),
        // What it is and does in words, where it checks: drawn from the
        // same answer as the graph, so that the two are of one text.
        "summary": super::summary::of_checked(
            &app,
            &who,
            &checked,
            (source, text(&body, "project")),
            super::summary::said(&body),
        )
        .await,
        "revision": blueprint::revision(source),
    })))
}

/// A blueprint's text laid out the one way, as the editor asks for when
/// a file is saved. For a project's own file the project may say a few
/// things of its own, in a file in its repository. Nothing is written
/// here: the text comes back, or nothing does when it does not read as a
/// blueprint, and what the reader says of it is the check's to show.
async fn lay_out(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let source =
        body.get("source").and_then(Value::as_str).ok_or_else(|| Error::invalid("`source` is needed"))?;
    let settings = match (text(&body, "project"), text(&body, "path")) {
        (Some(project), Some(path)) => {
            let top = app
                .studio
                .projects
                .dir_of(project)
                .ok_or_else(|| Error::not_found(format!("there is no project `{project}`")))?;
            // One the factory keeps for the project is looked for from where
            // it is, as far as the factory's place for them, and then at the
            // project's top.
            let place = app.studio.projects.place_of(project);
            let (top, path) = match conductor::library::folders::in_place(path) {
                Some(rest) if crate::laying::said_within(&place.join(rest), &place) => (place, rest),
                Some(_) => (top, blueprint::format::SETTINGS_FILE),
                None => (top, path),
            };
            let within = std::path::Path::new(path);
            // Only ever looked for inside the project.
            if !within.components().all(|part| matches!(part, std::path::Component::Normal(_))) {
                return Err(Error::invalid(format!("`{path}` is not a file of the project")).into());
            }
            let folder = top.join(within).parent().map_or_else(|| top.clone(), std::path::Path::to_path_buf);
            crate::laying::settings_for(&folder, Some(&top)).map_err(Error::invalid)?
        }
        _ => blueprint::Settings::default(),
    };
    Ok(Json(match blueprint::format(source, &settings) {
        Ok(formatted) => json!({ "formatted": formatted }),
        Err(left) => json!({ "formatted": null, "why": left.message() }),
    }))
}

/// Every action a blueprint can use on this device, with its arguments,
/// and every event a handler can hear, with what it carries.
async fn actions(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    Ok(Json(json!({
        "actions": app.engine.registry().all_specs(),
        "events": app.engine.registry().events().specs(),
        "functions": functions(),
        "names": names(),
    })))
}

fn functions() -> Vec<Value> {
    let example = |name: &str| {
        blueprint::vocabulary::CALLED.iter().find(|(called, _)| *called == name).map(|(_, text)| *text)
    };
    blueprint::expr::FUNCTIONS
        .iter()
        .map(|(name, usage)| json!({ "name": name, "usage": usage, "example": example(name) }))
        .collect()
}

/// The names an expression may start with, and what may follow each.
fn names() -> Vec<Value> {
    blueprint::ROOTS
        .iter()
        .map(|root| {
            let members = blueprint::vocabulary::members(root.name);
            json!({ "name": root.name, "meaning": root.meaning, "members": members, "example": root.example })
        })
        .collect()
}

/// The language as an editor needs it, in one read: every mapping a
/// blueprint can contain with its fields, the actions this device has, and
/// what an expression can say. An editor that is handed this knows what to
/// colour and what to offer without knowing anything about blueprints.
async fn language(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    Ok(Json(json!({
        "shapes": blueprint::vocabulary::SHAPES,
        "actions": app.engine.registry().all_specs(),
        "events": app.engine.registry().events().specs(),
        "functions": functions(),
        "names": names(),
        "step": blueprint::vocabulary::STEP_VIEW,
        "decisions": blueprint::DECISIONS,
    })))
}

/// Where the blueprints this factory has for no project use each word,
/// name and action, for the Reference to say beside each: worked out as it
/// is asked, from the texts as they are now (`blueprint::used`).
async fn language_used(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let library = app.engine.library();
    let texts: Vec<(String, String)> = library
        .names(None)
        .into_iter()
        .filter_map(|name| library.load(&name, None).map(|text| (name, text)))
        .collect();
    Ok(Json(json!(blueprint::used::used(&texts))))
}

/// Words shown working: each scenario's example, how its stand-ins
/// answer, what is done to it as a sandbox's acts take it, and what to
/// notice (`blueprint::vocabulary::SCENARIOS`). The Reference offers one
/// beside each example it is of, and runs it in a sandbox as asked.
async fn language_scenarios(Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let scenarios: Vec<Value> = blueprint::vocabulary::SCENARIOS
        .iter()
        .map(|one| {
            let acts: Value = serde_json::from_str(one.acts).unwrap_or(Value::Null);
            json!({ "name": one.name, "example": one.example, "answers": one.answers, "acts": acts, "notice": one.notice })
        })
        .collect();
    Ok(Json(json!(scenarios)))
}

/// The language as a schema the editors people already have complete and
/// check a blueprint against: its words, and the actions there are on
/// this device, with what its packages bring. It helps whoever writes;
/// `check` is what decides.
async fn schema(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    Ok(Json(blueprint::schema::schema(&app.engine.registry().all_specs())))
}

/// Starts or updates an instance. The blueprint is given by name, as text, or
/// as a file in a project.
async fn apply(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    // Before anything is read or said: applying takes this, and a file of
    // a project is read by whoever may read the project's files.
    who.require("blueprints:run")?;
    if text(&body, "path").is_some() {
        who.require("projects:read")?;
    }
    let project = text(&body, "project").map(str::to_string);
    // A project that is not there is said so now. Applied all the same,
    // the instance was started under a name nothing answers to: a
    // misspelt project ran, and what it did went nowhere anybody looks.
    if let Some(project) = &project
        && app.studio.projects.get(project).await.is_err()
    {
        return Err(Error::not_found(format!(
            "there is no project `{project}`, so nothing was applied; `{} project list` says which there are",
            brand::CLI
        ))
        .into());
    }
    // What whoever applies holds, for what the text lists and they do not.
    let (applier, held, asks) = (who.name.clone(), who.scopes.clone(), who.clone());
    let (name, source) =
        match (text(&body, "path"), body.get("source").and_then(Value::as_str), text(&body, "blueprint")) {
            (Some(path), _, _) => {
                let project = project.as_deref().ok_or_else(|| Error::invalid("`path` needs `project`"))?;
                app.studio.projects.may_read_path(project, path, &who).await?;
                let source = app.studio.projects.read(project, path).await?;
                (None, Some(source))
            }
            (None, Some(source), name) => (name.map(str::to_string), Some(source.to_string())),
            (None, None, Some(name)) => (Some(name.to_string()), None),
            (None, None, None) => {
                return Err(Error::invalid("give one of `blueprint`, `source` or `path`").into());
            }
        };
    // Text is filed under the name it gives itself.
    let name = match (&name, &source) {
        (Some(name), _) => name.clone(),
        (None, Some(source)) => blueprint::check(source, &blueprint::NoCatalog)
            .blueprint
            .map(|blueprint| blueprint.name)
            .unwrap_or_else(|| "blueprint".to_string()),
        (None, None) => unreachable!("one of the three was given"),
    };
    // Said by whoever applies it, of everything measured under it: what a
    // blueprint that called this one would say with `labels:`.
    let labels = match body.get("labels") {
        None | Some(Value::Null) => Default::default(),
        Some(Value::Object(labels)) => blueprint::labels::given(labels).map_err(Error::invalid)?,
        Some(_) => {
            return Err(Error::invalid(
                "`labels` must be a mapping from a label's name to its value, such as { \"experiment\": \"cart\" }",
            )
            .into());
        }
    };
    let request = Apply {
        blueprint: name,
        source,
        params: body.get("params").and_then(Value::as_object).cloned().unwrap_or_default(),
        key: text(&body, "key").map(str::to_string),
        parent: None,
        target: None,
        project,
        principal: who,
        force: body.get("force").and_then(Value::as_bool).unwrap_or(false),
        take: body.get("take").and_then(Value::as_bool).unwrap_or(false),
        labels,
        // The empty name is the daemon's own none on purpose, and no request's.
        environment: text(&body, "environment").filter(|name| !name.is_empty()).map(str::to_string),
    };
    // Put back on the texts it had before it was last moved (`back`), or
    // only told what that would put back (`say`); else applied.
    let flag = |name: &str| body.get(name).and_then(Value::as_bool).unwrap_or(false);
    let applied = match flag("back") {
        true => app.engine.put_back(request, flag("say")).await?,
        false => app.engine.apply(request).await?,
    };
    let mut answer = json!(applied);
    if flag("back") {
        (answer["back"], answer["said_only"]) = (json!(true), json!(flag("say")));
    }
    // Where the project has a file of its own for this name, or for one it
    // reaches, that was not the text used, whoever applied is told.
    if let Ok(instance) = app.engine.instance(&applied.instance).await {
        // Which instance it is, as everything else that names one says it:
        // the blueprint it runs and its key. The id alone says nothing, and
        // whoever applied a text that was there already was told so of a
        // name they had not got (`null is already running this`).
        (answer["blueprint"], answer["key"]) = (json!(instance.blueprint), json!(instance.key));
        let mut said = app.engine.passed_over(&instance.source, instance.project.as_deref());
        // And what the project has not got that this cannot work without:
        // a fleet applied for a project with no board.
        said.extend(
            crate::usual::lacks(&app.engine, &app.studio, &instance.blueprint, instance.project.as_deref())
                .await,
        );
        if !said.is_empty() {
            answer["said"] = json!(said);
        }
        // What the check has to say of the text short of refusing it. An
        // apply goes ahead over a warning, and whoever applied may never
        // have asked for the check: a handler nothing wakes is said here.
        let checked = app.engine.check(&instance.source, instance.project.as_deref());
        let mut warnings: Vec<blueprint::Diagnostic> =
            checked.diagnostics.iter().filter(|diagnostic| !diagnostic.is_error()).cloned().collect();
        warnings.extend(unheard(&app, checked.blueprint.as_ref(), instance.project.as_deref()));
        // What it lists and whoever applied does not hold, said first.
        if let Some(said) = unheld(&instance.source, checked.blueprint.as_ref(), &applier, &held) {
            warnings.insert(0, said);
        }
        if !warnings.is_empty() {
            answer["warnings"] = json!(warnings);
        }
    }
    if body.get("wait").and_then(Value::as_bool).unwrap_or(false)
        && let Some(run) = &applied.run
    {
        answer["finished"] = handed::run(&app, &asks, app.engine.wait_run(run).await?);
    }
    Ok(Json(answer))
}

/// What a list is asked for beside `project`, which is read apart
/// ([`named`]): it may be said several times over.
#[derive(Deserialize)]
struct InstancesQuery {
    live: Option<bool>,
    parent: Option<String>,
    roots: Option<bool>,
    /// Only those of this blueprint (what a project's pipeline ran as),
    /// and only those under this environment of their project's.
    blueprint: Option<String>,
    environment: Option<String>,
    limit: Option<usize>,
}

/// The projects a query names: `project` said once, as it always was, or
/// once for each of several (`project=a&project=b`), each named once
/// however often it was said. None is every project and what is nobody's.
/// It is several projects and never something that gathers them: which
/// are asked for together is whoever asks's to say, each time.
fn named(said: Vec<(String, String)>) -> Vec<String> {
    let mut named: Vec<String> = Vec::new();
    for (_, project) in said.into_iter().filter(|(word, _)| word == "project") {
        if !named.contains(&project) {
            named.push(project);
        }
    }
    named
}

async fn instances(
    State(app): State<App>,
    Auth(who): Auth,
    Query(query): Query<InstancesQuery>,
    Query(said): Query<Vec<(String, String)>>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    // As many as are handed: what was asked for, or as many as a list of one project's hands out at most.
    let listed = match query.limit.unwrap_or(200) {
        0 => 500,
        limit => limit,
    };
    let of = |project: Option<String>| InstanceFilter {
        live_only: query.live.unwrap_or(false),
        project,
        parent: query.parent.clone(),
        roots_only: query.roots.unwrap_or(false),
        blueprint: query.blueprint.clone(),
        environment: query.environment.clone(),
        // One more than is handed, so that whether there are more is said and not guessed.
        limit: listed + 1,
    };
    let mut named = named(said);
    let found = match named.len() {
        0 | 1 => app.engine.instances(of(named.pop())).await?,
        // Several projects' are each one's put together as a list of
        // everything is: the newest first, and as many as were asked for.
        _ => {
            let mut all = Vec::new();
            for project in named {
                all.extend(app.engine.instances(of(Some(project))).await?);
            }
            all.sort_by_key(|one| std::cmp::Reverse(one.created));
            all
        }
    };
    // A list cut at so many says so, or whoever reads it takes it for all
    // there is (`mw instances` said 200 and nothing more).
    let more = found.len() > listed;
    let mut found = found;
    found.truncate(listed);
    // Each with the version its text says, and whether the text now there
    // for its blueprint is another: worked out now, from the texts.
    Ok(Json(json!({ "instances": app.engine.shown(found), "more": more })))
}

/// One instance with what is needed to show it: its recent runs, its
/// children and the graph of the blueprint it is running.
async fn instance(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let info = app.engine.instance(&id).await?;
    let children = app
        .engine
        .instances(InstanceFilter { parent: Some(id.clone()), ..InstanceFilter::default() })
        .await?;
    let graph =
        blueprint::check(&info.source, &blueprint::NoCatalog).blueprint.as_ref().map(blueprint::graph);
    Ok(Json(json!({
        "source": info.source,
        "instance": app.engine.shown_one(info),
        "runs": handed::runs(&app, &who, app.engine.runs(&id, 20).await?),
        "children": app.engine.shown(children),
        "graph": graph,
    })))
}

/// Every definition an instance has been given, the latest first: what
/// it was applied with each time, by whom, and whether it held.
async fn instance_versions(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    app.engine.instance(&id).await?;
    Ok(Json(json!({ "versions": app.engine.versions(&id, 100).await? })))
}

/// Applies an instance again as it is: the text now there for its
/// blueprint, with the params, the key and the place it has, for whoever
/// asks. That is how one a handler started is brought up to date by hand:
/// nobody but what started it knows what it was started with. It is an
/// apply like any other, and the answer says how the instance came to the
/// text (`took`): it took it where it is and nothing ran, it was carried
/// over (`on_update`), or its steps ran again.
async fn instance_apply(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    body: Option<Json<Value>>,
) -> ApiResult<Json<Value>> {
    let body = body.map(|Json(body)| body).unwrap_or_default();
    let flag = |name: &str| body.get(name).and_then(Value::as_bool).unwrap_or(false);
    let asks = who.clone();
    let applied = match flag("back") {
        true => app.engine.put_back_instance(&id, who, flag("say")).await?,
        false => app.engine.apply_again_taking(&id, who, flag("force"), flag("take")).await?,
    };
    let mut answer = json!(applied);
    if flag("back") {
        (answer["back"], answer["said_only"]) = (json!(true), json!(flag("say")));
    }
    if flag("wait")
        && let Some(run) = &applied.run
    {
        answer["finished"] = handed::run(&app, &asks, app.engine.wait_run(run).await?);
    }
    Ok(Json(answer))
}

async fn instance_stop(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:run")?;
    app.engine.stop(&id).await?;
    Ok(Json(json!({ "ok": true })))
}

/// `project` beside these is only the runs of what was applied for that
/// project, or for any of several ([`named`]).
#[derive(Deserialize)]
struct RunsQuery {
    instance: Option<String>,
    limit: Option<usize>,
}

async fn runs(
    State(app): State<App>,
    Auth(who): Auth,
    Query(query): Query<RunsQuery>,
    Query(said): Query<Vec<(String, String)>>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let limit = query.limit.unwrap_or(50).min(500);
    let mut named = named(said);
    let runs = match (query.instance, named.len()) {
        (Some(instance), _) => app.engine.runs(&instance, limit).await?,
        (None, 0 | 1) => app.engine.recent_runs(limit, named.pop()).await?,
        // The most recent of several projects': each one's most recent
        // put together, the latest begun first, as those of everything are.
        (None, _) => {
            let mut all = Vec::new();
            for project in named {
                all.extend(app.engine.recent_runs(limit, Some(project)).await?);
            }
            all.sort_by_key(|one| std::cmp::Reverse(one.started));
            all.truncate(limit);
            all
        }
    };
    Ok(Json(json!({ "runs": handed::runs(&app, &who, runs) })))
}

async fn run(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    Ok(Json(handed::run(&app, &who, app.engine.run(&id).await?)))
}

/// `tail` is the last so many lines, for a page that draws a long output
/// from its end and asks for what came before it by `after` and `limit`.
#[derive(Deserialize)]
struct LogQuery {
    after: Option<u64>,
    limit: Option<usize>,
    tail: Option<usize>,
}

async fn step_log(
    State(app): State<App>,
    Auth(who): Auth,
    Path((run, step)): Path<(String, String)>,
    Query(query): Query<LogQuery>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    // What a step printed is of what set its run off, as what it gave
    // is: kept from whoever may not be told that, in the words for a run
    // that is not there. A log with no run to ask is read as it was.
    let of = app.engine.run(&run).await.ok();
    if of.as_ref().is_some_and(|of| !app.tells_of(&who, of)) {
        return Err(Error::not_found(format!("there is no run `{run}`")).into());
    }
    let (after, limit) = (query.after.unwrap_or(0), query.limit.unwrap_or(2000).min(10_000));
    let lines = match query.tail {
        // Read from the end of its file and not from its beginning: a step's log is the runner's, kept as `<run>.<step>`.
        Some(tail) => app.engine.logs().tail("runs", &format!("{run}.{step}"), tail.min(10_000)),
        None => app.engine.step_log(&run, &step, after, limit),
    };
    // Where it printed nothing that is kept, whether that was let go.
    let let_go = of.filter(|_| lines.is_empty()).and_then(|of| app.engine.let_go_said(of.finished));
    Ok(Json(json!({ "lines": lines, "let_go": let_go })))
}

/// A document as the page it would be published as: `{ "source": "<its
/// Markdown>", "name": "<what to call it if it has no heading>" }`. The text
/// is sent, not a path, so that what is previewed is what is in the editor,
/// saved or not. Nothing is written or published by this.
async fn page_preview(
    State(_app): State<App>,
    Auth(who): Auth,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    let source = body.get("source").and_then(Value::as_str).unwrap_or_default();
    let name = text(&body, "name").unwrap_or("Untitled");
    Ok(Json(json!({ "html": studio::pages::page(source, name), "title": studio::pages::title(source) })))
}
