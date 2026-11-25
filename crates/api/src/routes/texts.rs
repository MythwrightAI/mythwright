//! How what is running comes by new texts: the factory's setting, said
//! and changed here.
//!
//! What it means is the conductor's (`conductor::taking`). Reading it is
//! for whoever may read blueprints; changing it is the owner's, and
//! having texts followed takes `shell` as well, since it gives up what
//! vouching is for.
//!
//! And a project's own copy of a blueprint that ships (`own`), which is
//! a fork of it: the text what is running there goes on with, written
//! where a project's own is looked for. From then it is a file of the
//! project's like any other: vouched for by whoever wrote it, and no
//! longer the program's to move. What it was copied from is kept as it
//! is made and forgotten as it is taken away (`conductor::forked`), each
//! said as it is done, and where it then differs from that, and how
//! what ships has moved since, is said as its summary says it
//! (`blueprint::summary::forked`).

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use conductor::forked::Origin;
use conductor::library::Named;
use conductor::taking::{Come, GIVES_UP, Take, Taking};
use kernel::{Error, Source};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new().route("/system/texts", get(texts).post(texts_set)).route("/projects/{id}/own", post(own))
}

/// The setting as it is said to people: each part, what holds where
/// nothing was said, and what following gives up.
fn shown(app: &App, now: &Taking) -> Value {
    // What came with the program is held unless somebody says otherwise; a
    // checkout's blueprints are files, and are read where they are.
    let built_in = app.engine.library().ships("std/board", None);
    let come = match now.come {
        Some(Come::Held) => "held",
        Some(Come::Follow) => "follow",
        None => "usual",
    };
    json!({
        "come": come,
        "land": now.land,
        "usual": if built_in { "held" } else { "read" },
        "by": now.by,
        "at": now.at,
        "gives_up": GIVES_UP,
    })
}

async fn texts(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    Ok(Json(shown(&app, &app.engine.taking())))
}

async fn texts_set(
    State(app): State<App>,
    Auth(who): Auth,
    Json(asked): Json<Take>,
) -> ApiResult<Json<Value>> {
    // Before anything is read of what was asked: whose this is to change.
    who.require("admin")?;
    let (now, left) = app.engine.set_taking(asked, &who).await?;
    let mut answer = shown(&app, &now);
    answer["not_taken"] = json!(left);
    crate::following::told(&app, left).await;
    Ok(Json(answer))
}

/// What is said of where a project's own and the one that ships differ:
/// of a blueprint by the version each says and what moved in it, of words
/// a blueprint links that they changed and which blanks came or went
/// (`conductor::Moved`), and whether they are one to what runs them. It is
/// the project's own set against what ships (`was` is the version of what
/// ships, `now` the fork's), as the fork's summary sets it against what it
/// was forked from: what only the fork has is new, as it is said there.
fn differs(name: &str, own: &str, ships: &str) -> Value {
    let moved = conductor::Moved::between(name, Some(ships), own);
    json!({
        "name": name,
        "was": moved.was,
        "now": moved.now,
        "changed": moved.changed,
        "same": conductor::library::same(name, own, ships),
    })
}

/// Makes a blueprint that ships the project's own, by copying it into the
/// project: a fork of it, with what it was copied from kept; or says how
/// the fork and the one that ships now differ (`say`), and where the fork
/// differs from what it was forked from; or takes the fork away again
/// (`back`), which is refused for one that says something else unless
/// whoever asks says to lose that (`force`).
async fn own(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let flag = |name: &str| body.get(name).and_then(Value::as_bool).unwrap_or(false);
    // Before anything is said of the project: this writes a file of it, or
    // takes one away; only saying where two texts differ is a reading.
    who.require(if flag("say") { "projects:read" } else { "projects:write" })?;
    let name = body.get("blueprint").and_then(Value::as_str).unwrap_or_default().to_string();
    let path = crate::library::own_path(&app.own, &name, &id);
    if let Some(path) = &path {
        app.studio.projects.may_read_path(&id, path, &who).await?;
    }
    let Some(ships) = app.engine.library().shipped(&name, Some(&id)) else {
        return Err(Error::not_found(format!(
            "no blueprint called `{name}` ships here, so there is none to make `{id}`'s own. A file of the \
             project's is its own already"
        ))
        .into());
    };
    // A project that keeps no blueprints of its own has no fork, and
    // nothing of its files is read in place of what ships.
    let Some(path) = path else {
        app.studio.projects.get(&id).await?;
        return Err(Error::conflict(format!(
            "`{id}` keeps no blueprints of its own, so it has no fork of `{name}` and none is made: what ships \
             is what runs there. Nothing was written or taken away. To have it keep them, in the factory's \
             place for them: `{cli} project blueprints {id} --usual`",
            cli = brand::CLI
        ))
        .into());
    };
    // A name the project has two files for is nobody's to copy over, say
    // of or take away until it is one: neither of them is read.
    let has = match crate::library::own_files(&app.own, &name, &id) {
        Named::Nothing => None,
        Named::File(file) => Some(file),
        Named::Several(files) => {
            return Err(Error::conflict(format!(
                "in `{id}` {}. Nothing was written or taken away",
                conductor::library::several(&name, &files)
            ))
            .into());
        }
    };
    // Who did it, said so that whoever may not read the file is not told of it.
    let said_by = |file: &str| {
        Source::actor(who.actor()).taking(app.studio.projects.takes_to_read(&id, file).as_deref())
    };
    if flag("say") || flag("back") {
        let Some(file) = has else {
            // A fork whose file went as a daemon stopped has nothing left to say.
            if flag("back") && app.engine.fork_leaving(&id, &name, true).await.is_some() {
                app.engine.fork_gone(&id, &name, (&path, false), None, (&who, said_by(&path))).await?;
            }
            return Err(Error::conflict(format!(
                "`{id}` has no file of its own for `{name}`: what ships is what runs there"
            ))
            .into());
        };
        // The file may be at the top of the project and not where a copy is put.
        app.studio.projects.may_read_path(&id, &file, &who).await?;
        let text = app.studio.projects.read(&id, &file).await?;
        let said = differs(&name, &text, &ships);
        if flag("say") {
            let known = crate::forked::known(app.knowing(), &id, &name, &text).await;
            return Ok(Json(json!({
                "blueprint": name, "path": file, "differs": said,
                // What it was forked from, where that was kept, and what
                // its summary says of where it stands.
                "origin": known.made(),
                "said": blueprint::summary::forked::parts(&known.fork(&text)),
            })));
        }
        if said["same"] != true && !flag("force") {
            return Err(Error::conflict(format!(
                "`{file}` does not say what `{name}` ships as: it was changed since it was copied, or what \
                 ships has moved on. Nothing was taken away. Say `--say` to see where they differ, and \
                 `--back --force` to take the file away all the same: what it says is then gone, unless the \
                 project keeps it somewhere else"
            ))
            .into());
        }
        // What was kept of it says nothing from here: its file is going.
        let was = app.engine.fork_leaving(&id, &name, true).await;
        if let Err(refused) = app.studio.projects.remove(&id, &file, &who).await {
            app.engine.fork_leaving(&id, &name, false).await;
            return Err(refused.into());
        }
        for one in crate::library::names_at(&app.own, &id, &file) {
            app.engine.vouch_saved(&id, &one, &who).await;
        }
        let lost = said["same"] != true;
        app.engine.fork_gone(&id, &name, (&file, lost), was.as_ref(), (&who, said_by(&file))).await?;
        return Ok(Json(json!({ "blueprint": name, "removed": file, "differs": said })));
    }
    if let Some(file) = has {
        return Err(
            Error::conflict(format!("`{id}` already has a file of its own for `{name}`: `{file}`")).into()
        );
    }
    // Written in the factory's place for the project's own, and never in
    // its folder: read there whether or not the project is trusted, since
    // nothing is there but what was written through the factory.
    // The text what is running there goes on with, where a tree is held to
    // one: a later one that nobody took would not be what they run.
    let held = app.engine.reached_in(&id, &name).await;
    let text = held.clone().unwrap_or_else(|| ships.clone());
    let written = app.studio.projects.write(&id, &path, &text, &who).await?;
    for one in crate::library::names_at(&app.own, &id, &written.path) {
        app.engine.vouch_saved(&id, &one, &who).await;
    }
    // What it was copied from, to the letter, kept with the saying of it.
    let origin = Origin::of(&name, &text, &written.path, &who, app.journal.now_ms());
    app.engine.fork_made(&id, origin.clone(), said_by(&written.path)).await?;
    Ok(Json(json!({
        "blueprint": name,
        "project": id,
        "path": written.path,
        "version": origin.version.clone(),
        "origin": {
            "blueprint": origin.blueprint, "version": origin.version, "revision": origin.revision,
            "by": origin.by, "by_name": origin.by_name, "at": origin.at,
        },
        // Whether what was copied is the one a running tree is held to and not the one that ships now.
        "held": held.is_some_and(|held| !conductor::library::same(&name, &held, &ships)),
        "differs": differs(&name, &text, &ships),
    })))
}
