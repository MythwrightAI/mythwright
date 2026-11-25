//! A file of a project posted as that file, and a copy made one.
//!
//! **Recognised here.** Whoever attaches something may say where they
//! have it (`from`, the whole path as their system has it: the command
//! line says it of a file it is handed). Where that is a file of the
//! channel's project (of a channel that is no project's, of the project
//! whose folder holds it), which whoever attaches may read, and the bytes
//! they sent are what that file holds as the daemon reads it, nothing is
//! kept beside the database: the attachment names the file by its project
//! and path (`talk::attachments::Posted`). Anything else is kept as a copy,
//! as it always was. The bytes are held to the file so that a path that
//! only reads the same on another device names nothing of this one's.
//!
//! **Read as whoever reads.** What such a file is now, and how big, is
//! read when the message is read and as the reader may read that file
//! ([`seen`]): `there` with its size, `gone`, or `kept` from them, which
//! hands them nothing of the file but what the message says (its name and
//! where it is). Its bytes are handed through the same question a
//! project's file always asks, `projects:read` and
//! `Projects::may_read_path`.
//!
//! **A copy made a file of a project** (`POST /attachments/{id}/project`):
//! written among the project's files through the door that writes a new
//! one (`Projects::write_new`, which asks `projects:write` and refuses
//! where anything is there), and the attachment then made to name it in
//! one write with what is said of it (`Channels::placed_file`), so that
//! the message names the file from then on.

use std::path::Path as FsPath;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kernel::{Error, Principal};
use serde_json::{Value, json};
use studio::{Attachment, Message, Posted};

use super::required;
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route("/attachments/{id}/seen", get(seen_one))
        .route("/attachments/{id}/project", post(to_project))
}

/// The file of `project` and its path that `from` is, where it is one of
/// `channel`'s project (or of a channel that is no project's, the project
/// whose folder holds it, the deepest), `who` may read it, and what it
/// holds is `bytes`.
pub(super) async fn recognised(
    app: &App,
    who: &Principal,
    channel: &studio::Channel,
    from: &str,
    bytes: &[u8],
) -> Option<(String, String)> {
    who.require("projects:read").ok()?;
    let from = FsPath::new(from);
    let candidates: Vec<String> = match &channel.project {
        Some(project) => vec![project.clone()],
        None => app.studio.projects.list().await.ok()?.into_iter().map(|project| project.id).collect(),
    };
    let mut found: Option<(String, String)> = None;
    for project in candidates {
        let Some(root) = app.studio.projects.dir_of(&project).and_then(|dir| kernel::whole::whole(&dir).ok())
        else {
            continue;
        };
        let Some(rest) = kernel::whole::under(from, &root) else { continue };
        let names: Vec<String> =
            rest.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
        if names.is_empty() {
            continue;
        }
        let path = names.join("/");
        // The deepest folder that holds it, where projects' folders are one in another.
        if found.as_ref().is_some_and(|(_, was)| was.len() <= path.len()) {
            continue;
        }
        found = Some((project, path));
    }
    let (project, path) = found?;
    app.studio.projects.may_read_path(&project, &path, who).await.ok()?;
    let there = app.studio.projects.read_bytes(&project, &path).await.ok()?;
    (there == bytes).then_some((project, path))
}

/// What the file a posted one names is now, as `who` may read it: its
/// state and, where it is there, its size. Nothing is read of one they
/// may not read.
async fn standing(app: &App, who: &Principal, file: &Posted) -> (&'static str, u64) {
    let projects = &app.studio.projects;
    if who.require("projects:read").is_err() {
        return ("kept", 0);
    }
    if projects.dir_of(&file.project).is_none() {
        return ("gone", 0);
    }
    if projects.may_read_path(&file.project, &file.path, who).await.is_err() {
        return ("kept", 0);
    }
    match projects.stat(&file.project, &file.path).await {
        Ok(entry) if !entry.dir => ("there", entry.size),
        _ => ("gone", 0),
    }
}

/// Says of something attached that names a file what that file is now,
/// as `who` may read it. A copy is as it is.
pub(crate) async fn seen(app: &App, who: &Principal, attachment: &mut Attachment) {
    let Some(file) = attachment.file.as_mut() else { return };
    let (state, size) = standing(app, who, file).await;
    file.state = Some(state);
    attachment.size = size;
}

/// The same of everything attached to each of `messages`.
pub(crate) async fn seen_all(app: &App, who: &Principal, messages: &mut [Message]) {
    for message in messages {
        for attachment in &mut message.attachments {
            seen(app, who, attachment).await;
        }
    }
}

/// What the file something attached names holds, for `who`: refused as
/// reading that project's file is refused them, and said to be gone where
/// it is.
pub(super) async fn bytes_of(app: &App, who: &Principal, found: &Attachment) -> Result<Vec<u8>, Error> {
    let file = found.file.as_ref().expect("a file of a project");
    who.require("projects:read")?;
    let projects = &app.studio.projects;
    let gone = || Error::not_found(format!("`{}` of `{}` is no longer there", file.path, file.project));
    if projects.dir_of(&file.project).is_none() {
        return Err(gone());
    }
    projects.may_read_path(&file.project, &file.path, who).await?;
    projects.read_bytes(&file.project, &file.path).await.map_err(|error| match error.kind {
        kernel::ErrorKind::NotFound => gone(),
        _ => error,
    })
}

/// Something attached, as whoever asks may see it: for a message that
/// came by what was said of it, whose file was not looked at for them.
async fn seen_one(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let (mut found, _) = app.studio.channels.attached(&id, &who).await?;
    seen(&app, &who, &mut found).await;
    Ok(Json(json!(found)))
}

/// Writes a copy attached to a message among the files of `project`, at
/// `path`, and has the message name that file from then on.
async fn to_project(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let channels = &app.studio.channels;
    let channel = channels.where_attached(&id, &who).await?;
    let reader = channels.may_write(&channel, &who).await?;
    who.require("projects:write")?;
    let (project, path) = (required(&body, "project")?, required(&body, "path")?);
    let path = path.trim().trim_start_matches('/');
    let (found, _) = channels.attachment_in(&reader, &id).await?;
    if let Some(file) = &found.file {
        return Err(Error::conflict(format!(
            "`{}` is the file `{}` of `{}` already, and is not added to a project again",
            found.name, file.path, file.project
        ))
        .into());
    }
    // Said plainly before anything is written: a path that leads out of the project is no file of it.
    studio::docs::Doc::file(project, path)?;
    app.studio.projects.get(project).await?;
    let bytes = tokio::fs::read(channels.attachments.path(&found.id))
        .await
        .map_err(|_| Error::not_found(format!("the file of `{}` is no longer here", found.name)))?;
    app.studio.projects.write_new(project, path, bytes, &who).await?;
    let mut placed = channels.placed_file(&reader, &id, project, path, &who).await?;
    seen(&app, &who, &mut placed).await;
    Ok(Json(json!(placed)))
}
