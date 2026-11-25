//! A forum's topics (`topics`), each a page of a wiki: made, under another or as a wiki of its own, read whole with
//! their replies, replied to, answered, moved in its wiki's tree, put
//! somewhere else and followed, each as whoever asks, and the pages that link to one. What a topic says (its title, body, summary and status)
//! is changed at the door any record is changed by
//! (`PATCH /api/records/{id}`), and goes with its revision all the same.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kernel::Error;
use serde_json::{Value, json};
use topics::Topics;

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub(super) fn routes() -> Router<App> {
    Router::new()
        .route("/topics", get(list).post(make))
        .route("/topics/{id}", get(read))
        .route("/topics/{id}/replies", post(reply))
        .route("/topics/{id}/answer", post(answer))
        .route("/topics/{id}/place", post(place))
        .route("/topics/{id}/move", post(move_to))
        .route("/topics/{id}/follow", post(follow))
        .route("/topics/{id}/revisions", get(revisions))
        .route("/topics/{id}/linked", get(linked))
}

fn keeper(app: &App) -> Topics {
    Topics::new(app.journal.clone(), app.studio.clone(), Some(app.engine.clone()))
}

fn said<'a>(body: &'a Value, key: &str) -> Option<&'a str> {
    body.get(key).and_then(Value::as_str)
}

fn place_of(body: &Value) -> (Option<&str>, Option<&str>) {
    (said(body, "workspace"), said(body, "project"))
}

/// `{ "topics": [...] }`: every topic, each its row, where it is and how many replies it has.
async fn list(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "topics": keeper(&app).list(&who).await? })))
}

/// `{ "title", "body"?, "status"?, "workspace"? | "project"? | "under"? }`:
/// a topic, made a wiki of its own where it is said to be, or in the usual
/// workspace; or a page under the page `under` names, of its wiki's place.
/// It has no status unless `status` says `open` or `standing`.
async fn make(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let title =
        said(&body, "title").ok_or_else(|| Error::invalid("a topic is called something: say its `title`"))?;
    let words = said(&body, "body").unwrap_or_default();
    let status = match body.get("status") {
        None | Some(Value::Null) => None,
        Some(Value::String(status)) => Some(status.as_str()),
        Some(_) => return Err(Error::invalid("`status` is `open` or `standing`, or null for none").into()),
    };
    let made = match said(&body, "under") {
        Some(under) => {
            if place_of(&body) != (None, None) {
                return Err(Error::invalid(
                    "a page made under another is where that page's wiki is: say `under`, or where it is \
                     (`workspace` or `project`), and not both",
                )
                .into());
            }
            keeper(&app).make_under(title, words, under, status, &who).await?
        }
        None => keeper(&app).make(title, words, place_of(&body), status, &who).await?,
    };
    Ok(Json(made))
}

/// `{ "topic": {...}, "replies": [...] }`, by its id or its title.
async fn read(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(keeper(&app).read(&id, &who).await?))
}

/// `{ "body", "parent"? }`: a reply, under the one it answers: `{ "seq" }`.
async fn reply(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let words = said(&body, "body").ok_or_else(|| Error::invalid("a reply says something: its `body`"))?;
    let parent = match body.get("parent") {
        None | Some(Value::Null) => None,
        Some(parent) => Some(
            parent
                .as_u64()
                .ok_or_else(|| Error::invalid("`parent` is the number the reply it answers was said at"))?,
        ),
    };
    Ok(Json(json!({ "seq": keeper(&app).reply(&id, words, parent, &who).await? })))
}

/// `{ "reply": <number> | null }`: the reply that answered it, or none, which opens it again; refused
/// of a topic with no status, which is no question.
async fn answer(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let reply = match body.get("reply") {
        None | Some(Value::Null) => None,
        Some(reply) => {
            Some(reply.as_u64().or_else(|| reply.as_str().and_then(|said| said.parse().ok())).ok_or_else(
                || Error::invalid("`reply` is the number the reply that answered it was said at"),
            )?)
        }
    };
    Ok(Json(keeper(&app).answered(&id, reply, &who).await?))
}

/// `{ "workspace" | "project" }`: a wiki, with every page under it, put
/// somewhere else.
async fn place(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    Ok(Json(keeper(&app).place(&id, place_of(&body), &who).await?))
}

/// `{ "under": <page> | null, "before"?: <page> | null }`: moved under the
/// page `under` names, or to the top with `null`, where it is a wiki of its
/// own; before the page `before` names, or at the end of that level.
async fn move_to(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let named = |key: &str| match body.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(page)) => Ok(Some(page.as_str())),
        Some(_) => Err(Error::invalid(format!("`{key}` names a page, by its id or its title, or is null"))),
    };
    if body.get("under").is_none() {
        return Err(Error::invalid(
            "say where it goes: `under`, the page it goes under, or `null` for the top, where it is a wiki of its own",
        )
        .into());
    }
    Ok(Json(keeper(&app).move_to(&id, named("under")?, named("before")?, &who).await?))
}

/// `{ "follows": true | false, "pages"?, "often"?, "at"? }`, of whoever
/// asks: the page as it then is. How it is followed is kept as anything
/// followed is (`follows::followed`).
async fn follow(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    super::follows::followed(&app, &who, "page", &id, &body).await?;
    Ok(Json(topics::shown(&keeper(&app).find(&id, &who).await?)))
}

/// `{ "revisions": [...] }`, oldest first.
async fn revisions(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "revisions": keeper(&app).revisions(&id, &who).await? })))
}

/// `{ "linked": [...] }`: the pages that link to it, each its id, its title and the wiki it is in, by title.
async fn linked(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "linked": keeper(&app).linked(&id, &who).await? })))
}

/// Keeps every topic somewhere there is as projects and workspaces are
/// taken away: told by what is said of each, and asked again at once.
pub(crate) async fn follow_places(app: App) {
    let mut heard = app.journal.follow_every(app.journal.now(), "project.removed,workspace.removed");
    while let Ok(event) = heard.next().await {
        let gone = event.data.get("workspace").and_then(Value::as_str).zip(said(&event.data, "to"));
        let _ = keeper(&app).settle(gone.filter(|_| event.topic == "workspace.removed")).await;
    }
}
