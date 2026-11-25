//! A blueprint said in plain words, handed out.
//!
//! The words are the language's to make (`blueprint::summary`), from a
//! text and from what each action and event says of itself. What is the
//! daemon's is what it knows of those: the actions and events there are
//! on this device, with what its packages bring, and what holding each
//! scope allows. Nothing is run and nothing is written, and the
//! blueprint need not be applied anywhere.
//!
//! It is handed to whoever may read blueprints, which is who may read
//! the text it is made from and be told whether it checks: it says
//! nothing the text does not, and a text that does not check has none.
//!
//! But for one thing the text cannot say: where it stands among the
//! blueprints that ship (`crate::forked`). One that ships says how what
//! it does is changed, and that the project it is looked at for runs a
//! fork of it where it does; a fork says what it was forked from, where
//! it differs and how the default has moved since. That is said where
//! whoever asks says what the text is (a file of the project, or what
//! ships under its name; a blueprint asked for by name is whichever was
//! found), and only to whoever may read the project's files.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use blueprint::summary::{Known, Summary};
use kernel::Error;
use serde_json::{Value, json};

use crate::forked::Said;
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new().route("/blueprints/summary", post(summary)).route("/blueprints/stands", get(stands))
}

/// Every project's own file under the name of a text that ships, a
/// blueprint or a file one links, with where it stands there and that
/// in a sentence (`crate::forked::own_of`): for `mw status`, which says
/// those that are not in use. Only what `who` may read is listed.
async fn stands(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    let mut found = Vec::new();
    for project in app.studio.projects.list().await? {
        for name in crate::library::every_own_name(&app.own, &project.id) {
            let Some(own) = crate::forked::own_of(app.knowing(), &who, &project.id, &name).await else {
                continue;
            };
            // One file is two names; it is said once, by the name that ships.
            if found
                .iter()
                .any(|one: &Value| one["project"] == project.id.as_str() && one["file"] == own.file.as_str())
            {
                continue;
            }
            found.push(json!({
                "project": project.id,
                "name": name,
                "file": own.file,
                "in_use": own.in_use(),
                "said": blueprint::summary::forked::own_sentence(&own.own(), &name),
            }));
        }
    }
    Ok(Json(json!({ "own": found })))
}

/// The summary of a text that was checked, where it checks: one with a
/// problem that would keep it from being applied has none, and what is
/// wrong with it is the check's to say. `said` is what whoever asks
/// says the text is, by which it is said where it stands for `who`.
pub(crate) async fn of_checked(
    app: &App,
    who: &kernel::Principal,
    checked: &blueprint::Checked,
    (source, project): (&str, Option<&str>),
    said: Said<'_>,
) -> Option<Summary> {
    let read = checked.blueprint.as_ref().filter(|_| !checked.has_errors())?;
    let registry = app.engine.registry();
    let (actions, events) = (registry.all_specs(), registry.events());
    let known = Known { actions: &actions, events: events.specs(), scopes: kernel::principal::SCOPES };
    let stands = crate::forked::stands(app.knowing(), who, project, &read.name, source, said).await;
    let standing = stands.as_ref().map(|stands| stands.standing(source));
    Some(blueprint::summary::summary_as(read, &known, standing.as_ref()))
}

/// What a request says the text it sends is: the file at `path` of the
/// project, or what ships under its name (`ships`).
pub(crate) fn said(body: &Value) -> Said<'_> {
    Said {
        path: body.get("path").and_then(Value::as_str).filter(|path| !path.is_empty()),
        ships: body.get("ships").and_then(Value::as_bool).unwrap_or(false),
    }
}

/// The summary of a text that is sent (`source`), or of a blueprint by
/// its name as it is found for a project or for none (`name`,
/// `project`): one of the project's own, a shared one, or one that
/// ships. A text that does not check is answered with what is wrong
/// with it and no summary.
async fn summary(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    // Before anything is read or said of any text.
    who.require("blueprints:read")?;
    let text = |name: &str| body.get(name).and_then(Value::as_str).filter(|said| !said.is_empty());
    let project = text("project");
    // What a blueprint found by its name is: the project's own file,
    // where that is what was found, and otherwise what ships.
    let mut found = None;
    let source = match (body.get("source").and_then(Value::as_str), text("name")) {
        (Some(source), _) => source.to_string(),
        (None, Some(name)) => {
            let library = app.engine.library();
            // A name with more than one file is said to be that, not to be nobody's.
            let text = library.load(name, project).ok_or_else(|| match library.refused(name, project) {
                Some(why) => Error::conflict(why),
                // One of a project's own is found for its project, and a
                // package's once the package is installed.
                None => Error::not_found(format!(
                    "there is no blueprint called `{name}` here. One of a project's own is found by saying which \
                     project, and one a package brings once the package is installed"
                )),
            })?;
            let own = project.filter(|_| library.is_own(name, project));
            found = Some(own.and_then(|project| crate::library::own_file(&app.own, name, project)));
            text
        }
        (None, None) => {
            return Err(
                Error::invalid("give `source`, a blueprint's text, or `name`, what one is called").into()
            );
        }
    };
    let checked = app.engine.check(&source, project);
    let said = match &found {
        Some(file) => Said { path: file.as_deref(), ships: file.is_none() },
        None => said(&body),
    };
    Ok(Json(json!({
        "ok": !checked.has_errors(),
        "name": checked.blueprint.as_ref().map(|read| read.name.clone()),
        "summary": of_checked(&app, &who, &checked, (&source, project), said).await,
        "diagnostics": checked.diagnostics,
    })))
}
