//! What somebody follows (`talk::following`): everything they follow,
//! following one thing or no longer, how often they are told, and what a
//! line of something followed told of.
//!
//! No route here takes whose follows are meant: they are always those of
//! whoever asks, a person, or a key as its maker. Following something
//! asks of the asker whether they may read it; one they may not is
//! refused, in the same words as one that is not there where that is how
//! it answers them.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use kernel::following::{Often, at_of, kind_of};
use kernel::{Error, Principal, Result};
use serde_json::{Value, json};
use talk::following::{Follow, Setting};
use topics::Topics;

use crate::followers::{table_kind, title_of};
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new().route("/following", get(list).post(follow)).route("/inbox/{id}/changes", get(changes))
}

fn keeper(app: &App) -> Topics {
    Topics::new(app.journal.clone(), app.studio.clone(), Some(app.engine.clone()))
}

/// A follow as it is handed out, with what it is called.
async fn shown(app: &App, follow: &Follow) -> Value {
    let mut shown = json!(follow);
    shown["title"] = json!(title_of(app, &follow.kind, &follow.thing).await);
    shown["follows"] = json!(true);
    shown
}

/// The thing `named` names of `kind`, where `who` may read it: its kind as
/// it is kept (a board, a library or a table, whichever was said) and id.
async fn resolve(
    app: &App,
    who: &Principal,
    kind: &'static str,
    named: &str,
) -> Result<(&'static str, String)> {
    match kind {
        "page" => Ok(("page", keeper(app).find(named, who).await?.id)),
        "channel" => Ok(("channel", app.studio.channels.found(named, who).await?.id)),
        "project" => {
            who.require(talk::following::PROJECT_READ)?;
            Ok(("project", app.studio.projects.get(named).await?.id))
        }
        _ => {
            let (project, name) = match named.split_once('/') {
                Some(both) => both,
                None if kind == "board" => (named, studio::board::BOARD),
                None => {
                    return Err(Error::invalid(format!(
                        "name a {kind} by its project and its name, as `shop/{}`",
                        if kind == "library" { "docs" } else { "tasks" }
                    )));
                }
            };
            let table = app.studio.collections.get(project, name).await?;
            table.may_read(who)?;
            let kind = table_kind(app, &table.project, &table.name).await.unwrap_or(kind);
            Ok((kind, format!("{}/{}", table.project, table.name)))
        }
    }
}

/// `who` follows what `kind` and `named` name as `body` says (`follows`,
/// `pages`, `often`, `at`), or no longer where `follows` is false: what is
/// not said is as it was. A page is followed by its own door too, which
/// says who follows it.
pub(crate) async fn followed(
    app: &App,
    who: &Principal,
    kind: &str,
    named: &str,
    body: &Value,
) -> Result<Value> {
    let kind = kind_of(kind).map_err(Error::invalid)?;
    let (kind, thing) = resolve(app, who, kind, named).await?;
    let channels = &app.studio.channels;
    if body.get("follows").and_then(Value::as_bool) == Some(false) {
        if kind == "page" {
            keeper(app).follow(&thing, false, who).await?;
        }
        channels.unfollow(who, kind, &thing).await?;
        let title = title_of(app, kind, &thing).await;
        return Ok(json!({ "kind": kind, "thing": thing, "title": title, "follows": false }));
    }
    let was = channels.follow_of(who, kind, &thing).await?;
    let often = match body.get("often").and_then(Value::as_str) {
        Some(said) => Often::parse(said).map_err(Error::invalid)?,
        None => was.as_ref().map_or(Often::Once, |was| was.often),
    };
    let pages = kind == "page"
        && body.get("pages").and_then(Value::as_bool).unwrap_or(was.as_ref().is_some_and(|was| was.pages));
    let at = at_of(body.get("at").and_then(Value::as_str).or(was.as_ref().map(|was| was.at.as_str())))
        .map_err(Error::invalid)?;
    let follow = channels.follow(who, kind, &thing, Setting { pages, often, at }).await?;
    if kind == "page" {
        keeper(app).follow(&thing, true, who).await?;
    }
    Ok(shown(app, &follow).await)
}

/// `{ "following": [...] }`: everything the asker follows, each with its
/// `kind`, `thing`, `title`, `pages`, `often` and `at`. A page followed
/// by its own door alone, as before how often was said, is followed at
/// once and alone.
async fn list(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    let mut follows = app.studio.channels.following(&who).await?;
    let actor = who.actor();
    let pages: Vec<String> = match who.can("records:read") {
        true => keeper(&app)
            .list(&who)
            .await
            .unwrap_or_default()
            .iter()
            .filter(|page| {
                let followers = page["data"]["followers"].as_array();
                followers.is_some_and(|all| all.iter().any(|one| one.as_str() == Some(&actor)))
            })
            .filter_map(|page| page["id"].as_str().map(str::to_string))
            .collect(),
        false => Vec::new(),
    };
    // A page's row says how; whether it is followed is the page's own to say.
    follows.retain(|follow| follow.kind != "page" || pages.contains(&follow.thing));
    let kept: Vec<String> =
        follows.iter().filter(|follow| follow.kind == "page").map(|follow| follow.thing.clone()).collect();
    for page in pages.iter().filter(|page| !kept.contains(page)) {
        follows.push(Follow {
            who: actor.clone(),
            kind: "page".into(),
            thing: page.clone(),
            pages: false,
            often: Often::Once,
            at: kernel::following::USUAL_AT.into(),
            since: 0,
            told: 0,
            made: 0,
        });
    }
    let mut shown_all = Vec::with_capacity(follows.len());
    for follow in &follows {
        shown_all.push(shown(&app, follow).await);
    }
    Ok(Json(json!({ "following": shown_all })))
}

/// Follows something, or no longer: `{ "kind", "thing", "follows"?,
/// "pages"?, "often"?, "at"? }`. The follow as it now is.
async fn follow(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let said = |key: &str| body.get(key).and_then(Value::as_str).filter(|said| !said.is_empty());
    let (Some(kind), Some(thing)) = (said("kind"), said("thing")) else {
        return Err(Error::invalid("say what to follow: its `kind` and which, `thing`").into());
    };
    Ok(Json(followed(&app, &who, kind, thing, &body).await?))
}

/// `{ "changes": [...] }`: what the line `id` of the asker's box told of,
/// oldest first, each what it changed, what that is called now, what it
/// was in a word or two, who made it and when. Nothing for any other line.
async fn changes(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let mut changes = Vec::new();
    for change in app.studio.channels.changes_of(&who, &id).await? {
        let mut shown = json!(change);
        // What it is called now, as the thing it is of may be read now: the line is passed over where it may not.
        shown["label"] = json!(crate::followers::label_of(&app, &change.record).await);
        changes.push(shown);
    }
    Ok(Json(json!({ "changes": changes })))
}
