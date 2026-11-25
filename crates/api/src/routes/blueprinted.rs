//! Where a project keeps blueprints of its own, said and changed here
//! (`conductor::library::folders`): where the factory keeps them for it,
//! and before that a folder of its repository where it says one, or none;
//! and what its repository held of them moved in by one asking.
//!
//! Reading it is for whoever may read projects. It is not handed out
//! with a project, which says what it always said. Changing it takes what taking a
//! folder up as a project takes, `projects:write` and `admin`: it decides
//! which files of the project are read as its own blueprints, which is
//! which of them may stand in for what ships and be run with the
//! authority of whoever applies. It is kept and said in one write. Where
//! texts are followed, what follows them hears it and takes it as a save
//! of the files would be (`crate::following`); otherwise what an instance
//! runs of a project's own is what was vouched for when it was applied,
//! until it is applied again (`texts::vouched`).

use std::path::Path as FilePath;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use conductor::library::folders::{Folder, NAMESPACE, PLACE, USUAL};
use kernel::{Error, Source};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new().route("/projects/{id}/blueprints", get(shown).post(set))
}

async fn shown(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    let project = app.studio.projects.get(&id).await?;
    // Where the factory keeps them on this device, as a project's folder is
    // said: so that a terminal can tell a file of it for what it is.
    let at = app.studio.projects.place_of(&project.id);
    Ok(Json(
        json!({ "project": project.id, "blueprints": app.own.folders.of(&project.id).shown(), "at": at }),
    ))
}

/// Why `folder` cannot be where the project at `dir` keeps its own
/// blueprints: it is to be within the project's folder as the system has
/// it, links followed, as far as it is there yet; and a folder, where it
/// is there.
fn not_within(dir: &FilePath, folder: &str) -> Option<String> {
    let asked = dir.join(folder);
    if asked.is_file() {
        return Some(format!(
            "`{folder}` is a file of the project's, and its blueprints are kept in a folder"
        ));
    }
    let root = kernel::whole::whole(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut there = asked.as_path();
    while !there.exists() {
        there = there.parent()?;
    }
    let there = kernel::whole::whole(there).ok()?;
    (!kernel::whole::within(&there, &root)).then(|| {
        format!(
            "`{folder}` leads out of the project's folder ({}), by a link: a project's own blueprints are \
             files of its own. Say a folder that is within it",
            root.display()
        )
    })
}

/// What is said of `project` coming to keep its own blueprints as
/// `folder` where it kept them as `was`.
fn saying(project: &str, folder: &Folder, was: &Folder) -> Value {
    let mut said = json!({
        "project": project, "said": folder.shown()["said"], "was_said": was.shown()["said"], "usual": PLACE,
    });
    if let Some(path) = folder.repo() {
        said["folder"] = json!(path);
    }
    if let Some(path) = was.repo() {
        said["was"] = json!(path);
    }
    said
}

/// Says where the project keeps its own blueprints: `{ "folder": "<path
/// from its top>" }`, `{ "none": true }`, or `{ "usual": true }` for
/// `blueprints/`. Said again as it is, nothing is written or said.
async fn set(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // Before anything is said of the project.
    who.require("projects:write")?;
    who.require("admin")?;
    let project = app.studio.projects.get(&id).await?;
    if body.get("move_in").and_then(Value::as_bool) == Some(true) {
        return Ok(Json(moved_in(&app, &who, &project.id, body.get("from").and_then(Value::as_str)).await?));
    }
    let folder = Folder::asked(&body).map_err(Error::invalid)?;
    if let (Folder::In(path), Some(dir)) = (&folder, app.studio.projects.dir_of(&project.id))
        && let Some(why) = not_within(&dir, path)
    {
        return Err(Error::invalid(why).into());
    }
    let changed = keep(&app, &who, &project.id, &folder).await?;
    Ok(Json(json!({ "project": project.id, "blueprints": folder.shown(), "changed": changed })))
}

/// Has `project` keep its own blueprints as `folder`, kept and said in
/// one write: whether that changed anything.
async fn keep(app: &App, who: &kernel::Principal, project: &str, folder: &Folder) -> kernel::Result<bool> {
    let was = app.own.folders.of(project);
    if was == *folder {
        return Ok(false);
    }
    let (key, kept) = (project.to_string(), folder.kept());
    let said = saying(project, folder, &was);
    let source = Source::actor(who.actor());
    // Held so before the write, and put back where it is not kept: what
    // hears that it was said (`crate::following`) reads what is there
    // from then on, and may hear it before this goes on.
    app.own.folders.put(project, folder.clone());
    let written = app.journal.write(move |writing| {
        match &kept {
            Some(kept) => writing.keep(NAMESPACE, &key, kept)?,
            None => writing.forget(NAMESPACE, &key)?,
        }
        writing.say_here("project.blueprints", source, said)
    });
    if let Err(refused) = written.await {
        app.own.folders.put(project, was);
        return Err(refused);
    }
    Ok(true)
}

/// Every blueprint of the project's repository that was read as its own,
/// and every file one links, each as what it is called in the factory's
/// place and where it is in the repository: under `folder`, as what it is
/// called from there; and anywhere else, its top among them, by its whole
/// path, which was read first. What a scan never looks in (a repository's
/// own folder, what a build writes, a folder whose name begins with a dot)
/// is passed over. Refused where the repository is too large to be looked
/// through whole: some of what was read could then not be moved.
fn in_repo(dir: &FilePath, folder: &str) -> kernel::Result<Vec<(String, String)>> {
    let wanted = |path: &str, name: &str| {
        conductor::library::is_safe_name(path)
            && !path.split('/').any(|part| part.starts_with('.'))
            && (brand::is_blueprint_path(name) || conductor::library::is_linked(path))
    };
    let walked = kernel::walked::walk(&dir.join(folder));
    let all = kernel::walked::walk(dir);
    if walked.len() >= kernel::walked::MAX_ENTRIES || all.len() >= kernel::walked::MAX_ENTRIES {
        return Err(Error::invalid(format!(
            "the repository holds more than {} files and folders, so not every blueprint it was read by could be \
             found, and nothing was moved: copy them with `{} file write` where each is to be",
            kernel::walked::MAX_ENTRIES,
            brand::CLI
        )));
    }
    let mut found: Vec<(String, String)> = walked
        .into_iter()
        .filter(|entry| !entry.dir && wanted(&entry.path, &entry.name))
        .map(|entry| (entry.path.clone(), format!("{folder}/{}", entry.path)))
        .collect();
    let under = format!("{folder}/");
    for entry in all.into_iter().filter(|entry| !entry.dir && !entry.path.starts_with(&under)) {
        if wanted(&entry.path, &entry.name) {
            found.retain(|(called, _)| *called != entry.path);
            found.push((entry.path.clone(), entry.path));
        }
    }
    found.sort();
    Ok(found)
}

/// Moves a project's own blueprints from a folder of its repository into
/// the factory's place for them: each is copied there, with what a fork of
/// it was made from said to be there, the project then keeps them in the
/// factory's place alone, and each is vouched for as `who`, who could have
/// written it there. The repository's files are left as they are, for
/// whoever's they are to take away. Refused whole where the factory's
/// place has a file of the name already that says something else.
async fn moved_in(
    app: &App,
    who: &kernel::Principal,
    project: &str,
    from: Option<&str>,
) -> kernel::Result<Value> {
    let was = app.own.folders.of(project);
    let folder = from.map(|from| from.trim().trim_end_matches('/')).unwrap_or(was.repo().unwrap_or(USUAL));
    if let Some(why) = conductor::library::folders::refused(folder) {
        return Err(Error::invalid(why));
    }
    let dir = app.studio.projects.roots(project)?;
    if let Some(why) = not_within(&dir.dir, folder) {
        return Err(Error::invalid(why));
    }
    let found = in_repo(&dir.dir, folder)?;
    let mut copied = Vec::new();
    let mut taken = Vec::new();
    for (called, from) in &found {
        let text = std::fs::read(dir.dir.join(from))?;
        let to = dir.resolve(&format!("{PLACE}/{called}"))?;
        match std::fs::read(&to) {
            Ok(there) if there == text => {}
            Ok(_) => taken.push(format!("`{PLACE}/{called}`")),
            Err(_) => copied.push((to, text)),
        }
    }
    if !taken.is_empty() {
        return Err(Error::conflict(format!(
            "the factory keeps a file of `{project}`'s own already under the name of one its repository has that \
             says something else ({}), and nothing is written over: nothing was moved. Take those away, or the \
             repository's, and ask again",
            taken.join(", ")
        )));
    }
    for (to, text) in &copied {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(to, text)?;
    }
    keep(app, who, project, &Folder::Usual).await?;
    // What was copied is found, and said to have been added, by a look at the project's files.
    app.studio.projects.scan(project).await?;
    let mut names = Vec::new();
    for (called, from) in &found {
        for name in crate::library::names_at(&app.own, project, &format!("{PLACE}/{called}")) {
            app.engine.fork_moved(project, &name, (from, &format!("{PLACE}/{called}"))).await?;
            app.engine.vouch_saved(project, &name, who).await;
            names.push(name);
        }
    }
    let moved: Vec<Value> = found
        .iter()
        .map(|(called, from)| json!({ "from": from, "to": format!("{PLACE}/{called}") }))
        .collect();
    Ok(json!({
        "project": project, "blueprints": Folder::Usual.shown(), "moved": moved, "names": names,
        "left": "the files of the repository are as they were: they are the repository's to take away",
    }))
}
