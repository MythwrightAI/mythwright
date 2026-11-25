//! A project's repository as its Git page shows it (`places::seen`), and
//! what an installed package says it shows of a repository it serves
//! (`agents::shown`), each for whoever may read the project; and being
//! told that a repository may have changed, which the page reads again by.
//!
//! The first only reads the repository, the one way git is asked. The
//! second runs a package's program, which reaches the service it is for:
//! it is done only when a page asks for that one part, and takes what
//! using the package's actions takes (`extensions:<package>`) of whoever
//! asks.

use std::time::Instant;

use axum::extract::{Path as Route, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use kernel::{Error, Source};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/projects/{id}/git", get(seen))
        .route("/projects/{id}/git/{package}/{part}", get(part))
}

#[derive(serde::Deserialize)]
struct Asked {
    /// The branch whose commits are listed, whole as a branch is listed.
    branch: Option<String>,
}

/// The project's folder, for whoever may read the project.
async fn folder(app: &App, who: &kernel::Principal, id: &str) -> Result<std::path::PathBuf, Error> {
    who.require("projects:read")?;
    app.studio.projects.get(id).await?;
    app.studio.projects.dir_of(id).ok_or_else(|| Error::not_found(format!("there is no project `{id}`")))
}

async fn seen(
    State(app): State<App>,
    Auth(who): Auth,
    Route(id): Route<String>,
    Query(asked): Query<Asked>,
) -> ApiResult<Json<Value>> {
    let dir = folder(&app, &who, &id).await?;
    app.agents.looked().ask(&id);
    if !places::seen::is_repository(&dir) {
        let said =
            format!("`{id}`'s folder is no git repository of its own, so there is nothing of one to show");
        return Ok(Json(json!({ "project": id, "repository": false, "said": said })));
    }
    let started = Instant::now();
    let seen = places::seen::seen(&dir, asked.branch.as_deref()).await;
    let took = started.elapsed().as_secs_f64() * 1000.0;
    app.journal.measure("project.git_read", &[("project", id.as_str())], took, seen.is_ok());
    let seen = seen?;
    let remotes: Vec<(String, String)> =
        seen.remotes.iter().map(|remote| (remote.name.clone(), remote.fetch.clone())).collect();
    let mut answer = json!(seen);
    answer["project"] = json!(id);
    answer["repository"] = json!(true);
    answer["shows"] = json!(app.agents.shown_for(&remotes, &who));
    Ok(Json(answer))
}

async fn part(
    State(app): State<App>,
    Auth(who): Auth,
    Route((id, package, part)): Route<(String, String, String)>,
) -> ApiResult<Json<Value>> {
    let dir = folder(&app, &who, &id).await?;
    let started = Instant::now();
    let read = app.agents.shown(&package, &part, &dir, &who).await;
    let took = started.elapsed().as_secs_f64() * 1000.0;
    let labels = [("project", id.as_str()), ("package", package.as_str())];
    app.journal.measure("project.git_shown", &labels, took, read.is_ok());
    Ok(Json(json!(read?)))
}

/// Follows the repository of every project somebody has looked at, of
/// every project something asked to have its branches said
/// (`branches.follow`, `places::branches`), and of every other that a
/// repository is made in, for as long as the daemon runs: one follower for
/// a project's repository, whatever it is followed for
/// (`places::seen::followed`). It says `project.git.changed` of one each
/// time the system says any of it may have changed, where somebody looked
/// at it, and each branch of it that moved (`project.branch.moved`, with
/// where each was said to be last kept in the project's journal in the
/// write that says the moves, so that a branch that moved while the daemon
/// was away is said once it is back), where its branches are followed.
/// Which projects there are is looked at again whenever one is added or
/// taken away, whenever somebody looks at one nobody had, and whenever one
/// more has its branches followed.
pub async fn follow(app: App) {
    let (watch, looked, branched) = (app.hosting.watch().clone(), app.agents.looked(), app.agents.branches());
    let mut heard = app.journal.follow_every(app.journal.now(), "project.created,project.removed");
    // What was followed is let go of first, wholly: each follower begins
    // by saying what changed since, so nothing is missed between, and two
    // never say one repository's branches at once.
    let mut following = tokio::task::JoinSet::new();
    loop {
        following.abort_all();
        while following.join_next().await.is_some() {}
        let followed: Vec<String> = branched.projects();
        for project in app.studio.projects.list().await.unwrap_or_default() {
            let wants =
                places::seen::For { page: looked.has(&project.id), branches: followed.contains(&project.id) };
            let (dir, was) = (std::path::PathBuf::from(&project.path), project.repository);
            let (journal, id) = (app.journal.clone(), project.id.clone());
            let said = move |heard: places::seen::Heard| {
                let (journal, id, dir) = (journal.clone(), id.clone(), dir.clone());
                async move {
                    let changed = match heard {
                        // One that was no repository as the projects were listed, and is one now, was made.
                        places::seen::Heard::Began => wants.page || !was,
                        places::seen::Heard::Made => true,
                        places::seen::Heard::Changed => wants.page,
                    };
                    if changed {
                        let source = Source::actor("system");
                        journal.emit_ephemeral("project.git.changed", source, json!({ "project": id }));
                    }
                    if wants.branches {
                        places::branches::said(&journal.of(Some(&id)), &id, &dir, false).await;
                    }
                }
            };
            following.spawn(places::seen::followed(watch.clone(), project.path.into(), wants, said));
        }
        tokio::select! {
            () = looked.more() => {}
            () = branched.more() => {}
            event = heard.next() => {
                if event.is_err() {
                    return;
                }
            }
        }
    }
}
