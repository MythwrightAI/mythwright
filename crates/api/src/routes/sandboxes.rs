//! A sandbox test of a blueprint, by routes: made, acted on, read and
//! ended.
//!
//! Whoever may read blueprints may run one in a sandbox: the text is
//! one they send, or one they could read by its name, and nothing a
//! sandbox does takes any authority of theirs, since nothing it does is
//! real (`sandbox`, and `crate::sandboxes` for what the daemon keeps of
//! one). A sandbox is its maker's: anybody else is answered as for one
//! that is not there. Every answer says that it was a sandbox test.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kernel::Error;
use sandbox::{Act, Answers};
use serde_json::{Value, json};

use comparisons::beside;
use sandbox::played::{self, Played};

use crate::http::ApiResult;
use crate::sandboxes::{Asked, Held, MOST};
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/sandboxes", post(make))
        .route("/sandboxes/beside", post(beside))
        .route("/sandboxes/{id}", get(read).delete(end))
        .route("/sandboxes/{id}/acts", post(act))
}

/// Makes a sandbox for a text that is sent (`source`), or for a
/// blueprint by its name as it is found for a project or for none
/// (`name`, `project`), and applies the text in it with `params` unless
/// told not to (`apply: false`), after each blueprint `first` names:
/// what the text needs to be there before it, such as a board. A project's own blueprints are read
/// now, as texts, so that the one run may call them; nothing else of
/// the project is in a sandbox. `answers` says whether a stand-in waits
/// to be answered (`asked`, the usual) or answers at once (`default`),
/// and `told` what a stand-in answers each time, by its action's name.
/// A text that does not check is answered with what is wrong with it,
/// and no sandbox is made.
async fn make(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    // Before anything is read or said of any text.
    who.require("blueprints:read")?;
    let text = |name: &str| body.get(name).and_then(Value::as_str).filter(|said| !said.is_empty());
    let project = text("project");
    if let Some(project) = project {
        // Its own blueprints are files of it, read for whoever asks.
        who.require("projects:read")?;
        app.studio.projects.get(project).await?;
    }
    let library = app.engine.library();
    let source = match (body.get("source").and_then(Value::as_str), text("name")) {
        (Some(source), _) => source.to_string(),
        (None, Some(name)) => {
            library.load(name, project).ok_or_else(|| match library.refused(name, project) {
                Some(why) => Error::conflict(why),
                None => Error::not_found(format!(
                    "there is no blueprint called `{name}` here. One of a project's own is found by saying which \
                     project, and one a package brings once the package is installed"
                )),
            })?
        }
        (None, None) => {
            return Err(
                Error::invalid("give `source`, a blueprint's text, or `name`, what one is called").into()
            );
        }
    };
    let checked = app.engine.check(&source, project);
    let Some(read) = checked.blueprint.as_ref().filter(|_| !checked.has_errors()) else {
        return Ok(Json(json!({
            "ok": false,
            "said": "This text does not check, so it was not run in a sandbox: what is wrong with it is said here.",
            "diagnostics": checked.diagnostics,
        })));
    };
    let answers = match body.get("answers") {
        None | Some(Value::Null) => Answers::default(),
        Some(said) => serde_json::from_value(said.clone()).map_err(|_| {
            Error::invalid("`answers` is `asked` (a stand-in waits to be answered) or `default`")
        })?,
    };
    let asked = Asked {
        // By the name it was asked for where it was, which is what
        // other texts call it by; else what the text calls itself.
        name: text("name").map_or_else(|| read.name.clone(), str::to_string),
        source,
        own: library.own(project),
        answers,
        told: body.get("told").and_then(Value::as_object).cloned().unwrap_or_default(),
    };
    let held = app.sandboxes.make(&who, asked, app.engine.registry()).await?;
    // What is to be there before the text: each applied by its name,
    // then the text itself unless told not to. One that is refused is
    // refused as it would be anywhere, and nothing is left of a
    // sandbox whose text was never applied.
    let first = body.get("first").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut applies: Vec<Act> = first
        .iter()
        .filter_map(Value::as_str)
        .map(|name| Act::Apply { blueprint: Some(name.to_string()), params: Default::default(), key: None })
        .collect();
    if body.get("apply") != Some(&Value::Bool(false)) {
        let params = body.get("params").and_then(Value::as_object).cloned().unwrap_or_default();
        applies.push(Act::Apply { blueprint: None, params, key: text("key").map(str::to_string) });
    }
    let mut answer = held.sandbox.stands().await?;
    for apply in applies {
        answer = match held.sandbox.act(apply).await {
            Ok(applied) => applied,
            Err(refused) => {
                app.sandboxes.end(held.sandbox.id(), &who).await?;
                return Err(refused.into());
            }
        };
    }
    answer["ok"] = json!(true);
    Ok(Json(answer))
}

/// Does one thing to a sandbox and answers when what came of it is at
/// rest: `{ "apply": { params } }`, `{ "say": { topic, data } }`,
/// `{ "do": { action, with } }`, `{ "advance": { by } }`,
/// `{ "answer": { asked, gives | fails } }` or
/// `{ "answers": { answers } }`.
async fn act(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let held = app.sandboxes.find(&id, &who)?;
    let act: Act = serde_json::from_value(body).map_err(|err| {
        Error::invalid(format!(
            "that is not something done to a sandbox ({err}). What is: `apply` (with `params`), `say` (a \
             `topic` and its `data`), `do` (an `action` and what it is done `with`), `advance` (`by` how \
             long), `answer` (the stand-in `asked`, and what it `gives` or why it `fails`) and `answers`"
        ))
    })?;
    Ok(Json(held.sandbox.act(act).await?))
}

/// Everything that came of a sandbox so far.
async fn read(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let held = app.sandboxes.find(&id, &who)?;
    Ok(Json(held.sandbox.read().await?))
}

/// Ends a sandbox: nothing of it is left.
async fn end(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    app.sandboxes.end(&id, &who).await?;
    Ok(Json(json!({ "ended": id, "said": sandbox::SAID })))
}

/// How many texts are played side by side at most: each is a sandbox,
/// and a daemon has [`MOST`] at once.
pub const BESIDE: usize = 4;

/// Ends the sandboxes of a side by side that is not to be: nothing is
/// left of one that could not be played.
async fn ended(app: &App, who: &kernel::Principal, columns: Vec<Column>) {
    for column in columns {
        let _ = app.sandboxes.end(column.held.sandbox.id(), who).await;
    }
}

/// What came of one text of a side by side.
struct Column {
    called: String,
    source: String,
    held: Arc<Held>,
    played: Played,
}

/// Several sandboxes from one set of acts: each text sent (`texts`, each
/// `called` something and either its `source` or `ships`, what ships
/// under `name`) is run in a sandbox of its own, made as one is made
/// alone (`project`, `first`, `params`, `answers`, `told`), and each has
/// `acts` played against it in order, as a sandbox takes them, beside
/// an answer to a stand-in `usual`, which each sandbox answers with what
/// it usually answers. What each came to is handed whole, and laid out
/// beside the others as a comparison's columns are (`beside`): counts,
/// never scores, and the columns in the order they were asked for. The
/// sandboxes are kept as any are, each its maker's, until they are ended
/// by their own route or are asked nothing for a while. Where any text
/// does not check, what is wrong with each is said and none is made.
async fn beside(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    who.require("blueprints:read")?;
    let text = |name: &str| body.get(name).and_then(Value::as_str).filter(|said| !said.is_empty());
    let project = text("project");
    if let Some(project) = project {
        who.require("projects:read")?;
        app.studio.projects.get(project).await?;
    }
    let asked = body.get("texts").and_then(Value::as_array).cloned().unwrap_or_default();
    if asked.len() < 2 || asked.len() > BESIDE {
        return Err(Error::invalid(format!(
            "side by side is two texts of a blueprint, or up to {BESIDE}: `texts`, each `called` something and \
             either its `source` or `ships`"
        ))
        .into());
    }
    let library = app.engine.library();
    let mut texts: Vec<(String, String)> = Vec::new();
    for one in &asked {
        let called = one.get("called").and_then(Value::as_str).map(str::trim).unwrap_or_default();
        if called.is_empty() || texts.iter().any(|(other, _)| other == called) {
            return Err(Error::invalid(
                "each text side by side is `called` something of its own: it heads its column",
            )
            .into());
        }
        let source = match (one.get("source").and_then(Value::as_str), one.get("ships")) {
            (Some(source), _) => source.to_string(),
            (None, Some(Value::Bool(true))) => {
                let Some(name) = text("name") else {
                    return Err(Error::invalid("what ships is found by the blueprint's `name`").into());
                };
                library.load(name, None).ok_or_else(|| {
                    Error::not_found(format!("there is no blueprint called `{name}` that ships here"))
                })?
            }
            _ => {
                return Err(Error::invalid(format!(
                    "`{called}` says neither its `source` nor that it is what `ships`"
                ))
                .into());
            }
        };
        texts.push((called.to_string(), source));
    }
    let done: Vec<played::Done> = body
        .get("acts")
        .and_then(Value::as_array)
        .map(|acts| acts.iter().map(played::Done::read).collect::<kernel::Result<_>>())
        .transpose()?
        .unwrap_or_default();
    // Each is checked before anything is made: what is wrong with one is
    // said with the column it would have had.
    let mut wrong = Vec::new();
    let mut names = Vec::new();
    for (called, source) in &texts {
        let checked = app.engine.check(source, project);
        match checked.blueprint.as_ref().filter(|_| !checked.has_errors()) {
            Some(read) => names.push(text("name").map_or_else(|| read.name.clone(), str::to_string)),
            None => wrong.push(json!({ "called": called, "diagnostics": checked.diagnostics })),
        }
    }
    if !wrong.is_empty() {
        return Ok(Json(json!({
            "ok": false,
            "said": "A text side by side does not check, so none was run in a sandbox: what is wrong with each is said here.",
            "texts": wrong,
        })));
    }
    if app.sandboxes.count() + texts.len() > MOST {
        return Err(Error::unavailable(format!(
            "{} sandbox tests are under way here, and {} more would be more than the {MOST} there are at once. \
             End one, or wait: one that nobody asks anything is ended after {} minutes",
            app.sandboxes.count(),
            texts.len(),
            crate::sandboxes::UNASKED.as_secs() / 60
        ))
        .into());
    }
    let answers = match body.get("answers") {
        None | Some(Value::Null) => Answers::default(),
        Some(said) => serde_json::from_value(said.clone()).map_err(|_| {
            Error::invalid("`answers` is `asked` (a stand-in waits to be answered) or `default`")
        })?,
    };
    let first: Vec<String> = body
        .get("first")
        .and_then(Value::as_array)
        .map(|first| first.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();
    let params = body.get("params").and_then(Value::as_object).cloned().unwrap_or_default();
    let told = body.get("told").and_then(Value::as_object).cloned().unwrap_or_default();
    let mut columns: Vec<Column> = Vec::new();
    for ((called, source), name) in texts.into_iter().zip(names) {
        let asked =
            Asked { name, source: source.clone(), own: library.own(project), answers, told: told.clone() };
        let held = match app.sandboxes.make(&who, asked, app.engine.registry()).await {
            Ok(held) => held,
            Err(why) => {
                ended(&app, &who, columns).await;
                return Err(why.into());
            }
        };
        columns.push(Column { called, source, held, played: Played::default() });
    }
    // What goes first, and the text itself, are applied in each as one
    // made alone applies them: one refused is refused for all.
    let mut applies: Vec<Act> = first
        .iter()
        .map(|name| Act::Apply { blueprint: Some(name.clone()), params: Default::default(), key: None })
        .collect();
    if body.get("apply") != Some(&Value::Bool(false)) {
        applies.push(Act::Apply { blueprint: None, params, key: text("key").map(str::to_string) });
    }
    for column in &columns {
        for apply in &applies {
            if let Err(refused) = column.held.sandbox.act(apply.clone()).await {
                ended(&app, &who, columns).await;
                return Err(refused.into());
            }
        }
    }
    // Each sandbox has a thread of its own: they are played side by side.
    let done = Arc::new(done);
    let mut playing = tokio::task::JoinSet::new();
    for (at, mut column) in columns.into_iter().enumerate() {
        let done = done.clone();
        playing.spawn(async move {
            column.played = played::play(&column.held.sandbox, &done).await;
            (at, column)
        });
    }
    let mut columns: Vec<Option<Column>> = Vec::new();
    while let Some(played) = playing.join_next().await {
        let (at, column) =
            played.map_err(|err| Error::internal(format!("a sandbox side by side crashed: {err}")))?;
        if columns.len() <= at {
            columns.resize_with(at + 1, || None);
        }
        columns[at] = Some(column);
    }
    let columns: Vec<Column> = columns.into_iter().flatten().collect();
    let mut shown = Vec::new();
    for column in &columns {
        shown.push(column.held.sandbox.read().await?);
    }
    let each: Vec<beside::Played> = columns
        .iter()
        .zip(&shown)
        .map(|(column, shown)| beside::Played { called: &column.called, source: &column.source, shown })
        .collect();
    let laid = beside::sandboxes(&each);
    let sandboxes: Vec<Value> = columns
        .iter()
        .zip(&shown)
        .map(|(column, shown)| {
            let mut said = column.played.said();
            said["called"] = json!(column.called);
            said["sandbox"] = json!(column.held.sandbox.id());
            said["shown"] = shown.clone();
            said
        })
        .collect();
    Ok(Json(json!({ "ok": true, "said": sandbox::SAID, "beside": laid, "sandboxes": sandboxes })))
}
