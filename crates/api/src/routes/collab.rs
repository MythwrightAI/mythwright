//! The board, records, and agents. Channels are in `talk`.

use agents::SlotSpec;
use axum::extract::{Path, Query, State};
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use kernel::Error;
use serde::Deserialize;
use serde_json::{Value, json};
use studio::Changes;

use super::{required, text};
use crate::http::ApiResult;
use crate::state::{App, Auth, Said};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/board/{project}", get(board))
        .route("/collections", get(collections))
        .route("/collections/{project}/{name}/records", get(records).post(record_add))
        .route("/collections/{project}/{name}/commented", get(commented))
        .route("/records/{id}", get(record).patch(record_change).delete(record_remove))
        .route("/records/{id}/history", get(record_history))
        .route("/records/{id}/comments", post(record_comment))
        .route("/collections/{project}/{name}/views", post(view_save))
        .route("/collections/{project}/{name}/views/{view}", delete(view_remove))
        .route("/collections/{project}/{name}/fields/{field}/icon", put(icon_choose).delete(icon_remove))
        .route("/collections/{project}/{name}/title", put(title_choose).delete(title_remove))
        .route("/collections/{project}/{name}/fields", post(column_add))
        .route("/collections/{project}/{name}/fields/{field}", patch(column_change).delete(column_remove))
        .route("/board/{project}/items", post(item_add))
        .route("/board/{project}/columns", post(columns_set))
        .route("/board/items/{id}", patch(item_change).delete(item_remove))
        .route("/board/items/{id}/press", post(item_press))
        .route("/board/items/{id}/cost", get(item_cost))
        .route("/agents", get(agents).post(agent_add))
        .route("/agents/{name}", delete(agent_stop).patch(agent_give))
        .route("/agents/{name}/remove", get(agent_removal).post(agent_remove))
        .route("/agents/{name}/turns", get(agent_turns))
        .route("/agents/{name}/try", post(agent_try))
        .route("/providers/{name}/models", get(provider_models).post(provider_models_again))
        .route("/extensions", get(extensions).post(extension_add))
        .route("/extensions/{name}", delete(extension_remove))
        .route("/extensions/{name}/update", post(extension_update))
}

pub(super) fn strings(body: &Value, key: &str) -> Option<Vec<String>> {
    body.get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_string)).collect())
}

#[derive(Deserialize)]
struct CollectionsQuery {
    project: Option<String>,
}

/// The collections blueprints have declared: of one project, or of all.
async fn collections(
    State(app): State<App>,
    Auth(who): Auth,
    Query(query): Query<CollectionsQuery>,
) -> ApiResult<Json<Value>> {
    who.require("projects:read")?;
    // One that is kept private is not there for whoever may not read it.
    let mut collections = app.studio.collections.list(query.project.as_deref()).await?;
    collections.retain(|collection| collection.open_to(&who).is_ok());
    Ok(Json(json!({ "collections": collections })))
}

/// The records of one collection, in the order they were made: what a
/// field that names one of them is chosen from, and shown by.
async fn records(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let listed = app.studio.records.list(&project, &name, &who).await;
    // A table the project has not got, which a project may start with:
    // whoever asked for its records is told how it is given, as whoever
    // asks to add one is. A project an agent added has no library, and
    // the first thing the notes have somebody ask of one is its documents.
    Ok(Json(
        json!({ "records": listed.map_err(|refused| crate::usual::lacking(&crate::usual::shipped(&app.engine), &project, &name, refused))? }),
    ))
}

/// How many things were said on each record of one collection, by the
/// record's id, with nothing for a record nothing was said on: what a
/// list of them shows of each, asked once for the table and not of each
/// record's history. It is read by whoever may read the records.
async fn commented(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "commented": app.studio.records.commented(&project, &name, &who).await? })))
}

/// What a record is given or changed to: `fields`, each by its name.
fn fields_of(body: &Value) -> Result<serde_json::Map<String, Value>, Error> {
    body.get("fields")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| Error::invalid("`fields` is needed: what the record holds, each field by its name"))
}

/// Adds a record to a collection. A board's cards are added to the board.
async fn record_add(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let before = crate::taken::mark(&app, &project);
    let added = app.studio.records.add(&project, &name, fields_of(&body)?, &who, None).await;
    // A table the project has not got, which a project may start with:
    // whoever asked is told how it is given.
    let added = added.map_err(|refused| {
        crate::usual::lacking(&crate::usual::shipped(&app.engine), &project, &name, refused)
    })?;
    let mut answer = json!(added);
    crate::taken::said(&app, (&project, &name), before, &mut answer).await;
    Ok(Json(answer))
}

async fn record(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.records.get(&id, &who).await?)))
}

/// Changes the fields named, and leaves the others as they are.
async fn record_change(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let fields = fields_of(&body)?;
    let (collection, _) = app.studio.collections.record(&id).await?;
    let before = crate::taken::mark(&app, &collection.project);
    let mut answer = json!(app.studio.records.update(&id, fields, &who, None).await?);
    crate::taken::said(&app, (&collection.project, &collection.name), before, &mut answer).await;
    Ok(Json(answer))
}

async fn record_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    // What it is in the Trash, where it is kept to be given back, and how
    // many records went with it there (the pages under a page).
    let (collection, _) = app.studio.collections.record(&id).await?;
    let before = crate::taken::mark(&app, &collection.project);
    let gone = app.studio.records.removing(&id, &who, None).await?;
    let mut answer = crate::trash::moved(&app, gone.trash).await;
    if gone.with > 0 {
        answer["with"] = json!(gone.with);
    }
    crate::taken::said(&app, (&collection.project, &collection.name), before, &mut answer).await;
    Ok(Json(answer))
}

/// Says something on a record, a card included. It is kept in the
/// record's history; what follows from it is its blueprint's to say.
/// `public` beside the words marks it so, and it is private without: a
/// mark that is neither yes nor no is refused, since what was meant to be
/// kept in must not go out on a guess, nor the other way. `anchor` says
/// which words of a document it is about (`kernel::anchor`); without it,
/// it is about the whole record.
async fn record_comment(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let public = match body.get("public") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(public)) => *public,
        Some(other) => {
            return Err(Error::invalid(format!(
                "`public` says whether what is said is marked public, and is `true` or `false`; it is {other}.                  Left out, it is private."
            ))
            .into());
        }
    };
    // What it is about, where it is a few words of a document: each part checked here, before anything is kept.
    let anchor = kernel::anchor::read(body.get("anchor"))?;
    let said =
        app.studio.records.comment_on(&id, required(&body, "body")?, public, anchor.as_ref(), &who, None);
    Ok(Json(json!(said.await?)))
}

/// What has happened to a record, oldest first. It is there whether or not the record still is.
async fn record_history(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    // A record's history is the record's: a card's is read by whoever may
    // read the board, any other by whoever may read records. Of one that
    // is gone, which of the two it was is no longer known, and it takes both.
    if !who.can("board:read") {
        who.require("records:read")?;
    }
    match app.studio.collections.record(&id).await {
        Ok((collection, _)) => collection.may_read(&who)?,
        Err(_) => {
            who.require("board:read")?;
            who.require("records:read")?;
            // And of one that was a private table's, what that table takes.
            for collection in app.studio.collections.list(None).await? {
                if collection.owns(&id) {
                    collection.open_to(&who)?;
                }
            }
        }
    }
    Ok(Json(json!({ "history": app.studio.collections.history(&id).await? })))
}

/// Saves a view of a collection: a way of laying it out, under a name.
async fn view_save(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let view = studio::View {
        name: text(&body, "name").unwrap_or_default().to_string(),
        kind: text(&body, "kind").unwrap_or("table").to_string(),
        by: text(&body, "by").map(str::to_string),
        saved: true,
        // The fields it leaves out; said or not, a view somebody saves is kept with which.
        hides: body
            .get("hides")
            .and_then(Value::as_array)
            .map(|names| names.iter().filter_map(Value::as_str).map(str::to_string).collect()),
        group: text(&body, "group").filter(|field| !field.is_empty()).map(str::to_string),
        done: text(&body, "done").filter(|field| !field.is_empty()).map(str::to_string),
        only: studio::View::held_to(text(&body, "name").unwrap_or_default(), body.get("only"))?,
        // The order of its columns, which it is frozen up to and the figures at their feet, as a blueprint says them.
        shown: match body.as_object() {
            Some(said) => kernel::columns::Columns::read(text(&body, "name").unwrap_or_default(), said)?,
            None => Default::default(),
        },
    };
    // A view that was saved under another name until now is that one, renamed.
    if let Some(was) = text(&body, "was").filter(|was| *was != view.name) {
        let kept = app.studio.collections.save_view(&project, &name, view, &who).await?;
        if kept.views.iter().any(|one| one.name == was) {
            return Ok(Json(json!(app.studio.collections.remove_view(&project, &name, was, &who).await?)));
        }
        return Ok(Json(json!(kept)));
    }
    Ok(Json(json!(app.studio.collections.save_view(&project, &name, view, &who).await?)))
}

async fn view_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name, view)): Path<(String, String, String)>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.collections.remove_view(&project, &name, &view, &who).await?)))
}

/// Chooses the icon a field's column is drawn with, by its name: the
/// field's, for everybody who opens the table.
async fn icon_choose(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name, field)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // Whatever text was sent is the name's own check to refuse, in its own words.
    let icon = body.get("icon").and_then(Value::as_str).ok_or_else(|| {
        Error::invalid(
            "`icon` is needed: the name of the icon the column is drawn with, as in `calendar-days`. \
             A choice is taken away by asking the same address with DELETE",
        )
    })?;
    Ok(Json(json!(app.studio.collections.choose_icon(&project, &name, &field, Some(icon), &who).await?)))
}

/// Takes the choice away: the column is drawn as its table's declaration says.
async fn icon_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name, field)): Path<(String, String, String)>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.collections.choose_icon(&project, &name, &field, None, &who).await?)))
}

/// Calls a table something else for people, for everybody: `{ "title": ... }`. Its name is as it was.
async fn title_choose(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let title = body.get("title").and_then(Value::as_str).ok_or_else(|| {
        Error::invalid(
            "`title` is needed: what the table is to be called. It is called by its name again by asking the same \
             address with DELETE",
        )
    })?;
    Ok(Json(json!(app.studio.collections.title(&project, &name, Some(title), &who).await?)))
}

/// Takes that away: the table is called what it is declared to be, or by its name.
async fn title_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.collections.title(&project, &name, None, &who).await?)))
}

/// Adds a column to a table, for everybody who opens it: what it is
/// called and what kind of thing it holds (`label`, `kind`, and `of` for
/// what a choice offers or the table a record is of), or the column it
/// is a copy of (`like`); and where it stands in a view (`place`).
async fn column_add(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let place = body.get("place").filter(|place| place.is_object()).map(|place| studio::added::Place {
        view: text(place, "view").unwrap_or_default().to_string(),
        kind: text(place, "kind").unwrap_or_default().to_string(),
        order: strings(place, "order").unwrap_or_default(),
        before: text(place, "before").filter(|before| !before.is_empty()).map(str::to_string),
    });
    let new = studio::added::New {
        label: text(&body, "label").unwrap_or_default().to_string(),
        kind: text(&body, "kind").unwrap_or("text").to_string(),
        of: strings(&body, "of").unwrap_or_default(),
        like: text(&body, "like").filter(|like| !like.is_empty()).map(str::to_string),
        place,
    };
    let (table, field) = app.studio.collections.add_column(&project, &name, new, &who).await?;
    Ok(Json(json!({ "table": table, "field": field })))
}

/// Changes a column, saying one thing of it: what it is called (`label`,
/// or `null` for its name again), what kind of thing it holds (`kind`,
/// with `of`), what a choice offers and in what order (`of` alone), or
/// one thing it offers called something else (`rename`: `from`, `to`).
async fn column_change(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name, field)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    use studio::added::Change;
    let of = || strings(&body, "of").unwrap_or_default();
    let change = match (body.get("label"), text(&body, "kind"), body.get("rename")) {
        (Some(Value::Null), None, None) => Change::Label(None),
        (Some(Value::String(label)), None, None) => Change::Label(Some(label.clone())),
        (None, Some(kind), None) => Change::Kind { kind: kind.to_string(), of: of() },
        (None, None, Some(rename)) => Change::Renamed {
            from: required(rename, "from")?.to_string(),
            to: required(rename, "to")?.to_string(),
        },
        (None, None, None) if body.get("of").is_some_and(Value::is_array) => Change::Options(of()),
        _ => {
            return Err(Error::invalid(
                "say one thing of the column: `label` (what it is called, or null for its name again), `kind` (what it \
                 holds, with `of` where that kind needs it), `of` (what a choice offers, in order) or `rename` (one \
                 thing it offers called something else: `from` and `to`)",
            )
            .into());
        }
    };
    Ok(Json(json!(app.studio.collections.change_column(&project, &name, &field, change, &who).await?)))
}

/// Takes away a column somebody added, with what the rows hold there.
async fn column_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path((project, name, field)): Path<(String, String, String)>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.studio.collections.remove_column(&project, &name, &field, &who).await?)))
}

async fn board(
    State(app): State<App>,
    Auth(who): Auth,
    Path(project): Path<String>,
) -> ApiResult<Json<Value>> {
    who.require("board:read")?;
    Ok(Json(json!({
        "project": project,
        "columns": app.studio.board.columns(&project).await?,
        "items": app.studio.board.list(&project).await?,
    })))
}

async fn item_add(
    State(app): State<App>,
    Said { who, actor }: Said,
    Path(project): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let card = studio::board::New {
        title: required(&body, "title")?.to_string(),
        body: text(&body, "body").unwrap_or_default().to_string(),
        status: text(&body, "status").map(str::to_string),
        labels: strings(&body, "labels").unwrap_or_default(),
        // What else the board keeps of a card, each field by its name.
        more: body.get("fields").and_then(Value::as_object).cloned().unwrap_or_default(),
    };
    // Added with a key somebody made, it is the key that added it, as it
    // is the key that moves one: what follows may rest on what whoever
    // did it holds.
    Ok(Json(json!(app.studio.board.add_said(&project, card, &who, &actor).await?)))
}

async fn columns_set(
    State(app): State<App>,
    Auth(who): Auth,
    Path(project): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let columns = strings(&body, "columns").ok_or_else(|| Error::invalid("`columns` is needed"))?;
    Ok(Json(json!({ "columns": app.studio.board.set_columns(&project, columns, &who).await? })))
}

/// Changes a card: its content, its column (with a `note`, what is said as
/// it is moved), or its place in the column.
/// What a change to a card says by name beside a card's own fields
/// (`studio::models::CARD`): how it is moved, and the fields its board
/// declared beyond a card's own, given together.
const MOVING: &[&str] = &["before", "note", "fields", "within", "give"];

/// What a card cost, by round: each time it was worked and looked over,
/// the agents' turns of it, what they say they took, and what was
/// decided at its end (`crate::spent`). For whoever may read the board.
async fn item_cost(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(crate::spent::of(&app, &id, &who).await?))
}

async fn item_change(
    State(app): State<App>,
    Said { who, actor }: Said,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // Asked to be done only if the card is one project's (`within`: the
    // factory's bot, asked in that project's channel). Whether whoever
    // asks may change a card at all is said first, in the words it always
    // was; only somebody who may is told whose the card is.
    if let Some(project) = text(&body, "within") {
        who.require("board:write")?;
        if app.studio.board.get(&id).await?.project != project {
            return Err(Error::invalid(format!(
                "`{id}` is not a card of `{project}`, and only that project's were asked for"
            ))
            .into());
        }
    }
    // `"assignee": null` or `""` clears it; leaving it out keeps it. A
    // bare word is an agent by its name or a person by their handle or
    // email, and refused where it is nobody's.
    let assignee = match body.get("assignee").map(|value| value.as_str().filter(|who| !who.is_empty())) {
        Some(Some(word)) => {
            let agents: Vec<String> = app.agents.slots().await?.into_iter().map(|slot| slot.name).collect();
            let accounts = app.studio.identity.accounts().await?;
            Some(Some(studio::handle::assignee(word, &agents, &accounts).map_err(Error::invalid)?))
        }
        said => said.map(|_| None),
    };
    let changes = Changes {
        title: text(&body, "title").map(str::to_string),
        body: body.get("body").and_then(Value::as_str).map(str::to_string),
        assignee,
        labels: strings(&body, "labels"),
        // The same for the day it is wanted by.
        due: body.get("due").map(|value| value.as_str().filter(|day| !day.is_empty()).map(str::to_string)),
        // Anything else is a field the board's blueprint declared beyond
        // a card's own, by its name; the board says so if it is not. They
        // may also be given together as `fields`, as they are when a card
        // is added.
        more: body
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(name, _)| !studio::models::CARD.has(name) && !MOVING.contains(&name.as_str()))
            .chain(body.get("fields").and_then(Value::as_object).into_iter().flatten())
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
    };
    let mut item = app.studio.board.update_said(&id, changes, &who, &actor).await?;
    if let Some(status) = text(&body, "status") {
        // What is said as it is moved goes with the move: `note`.
        let (before, note) = (text(&body, "before"), text(&body, "note"));
        // And how many more rounds than its stage allows it is given:
        // `give`, a count, written as a number or as words.
        let give = match body.get("give") {
            None | Some(Value::Null) => Some(0),
            Some(Value::Number(count)) => count.as_u64(),
            Some(Value::String(count)) => count.trim().parse().ok(),
            Some(_) => None,
        }
        .ok_or_else(|| {
            Error::invalid("`give` is how many more rounds the card is given: a count, such as 1")
        })?;
        // Moved with a key somebody made, it is the key that moved it:
        // what follows a move may rest on what the mover holds.
        item = app.studio.board.move_said(&id, status, (before, note, give), &who, &actor).await?;
    }
    Ok(Json(json!(item)))
}

/// Presses something a card's column offers on it, or takes a press back
/// (`back`): as whoever asks, and with a key, as that key. Who may is what
/// the action's row says (`studio::pressed`).
async fn item_press(
    State(app): State<App>,
    Said { who, actor }: Said,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let action =
        text(&body, "action").ok_or_else(|| Error::invalid("which action? give `action`, its name"))?;
    // As for a change (`within`): only one project's card was asked for.
    if let Some(project) = text(&body, "within") {
        who.require("board:write")?;
        if app.studio.board.get(&id).await?.project != project {
            return Err(Error::invalid(format!(
                "`{id}` is not a card of `{project}`, and only that project's were asked for"
            ))
            .into());
        }
    }
    let back = body.get("back").and_then(Value::as_bool).unwrap_or(false);
    let pressing = app.studio.board.press(&id, action, back, &who, (None, Some(&actor))).await?;
    Ok(Json(json!(pressing)))
}

async fn item_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let trash = app.studio.board.remove(&id, &who).await?;
    Ok(Json(crate::trash::moved(&app, trash).await))
}

async fn agents(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("agents:read")?;
    // Each with what its model is called, as far as is known without asking.
    let mut slots: Vec<Value> = Vec::new();
    // What is running that brings each up and counts it, where something
    // does: its project is that one's, and it is not moved from here.
    let brought = app.agents.bringing(&app.engine).await?;
    for slot in app.agents.slots().await? {
        let mut described = json!(slot);
        described["model_name"] = json!(app.agents.model_name(&slot));
        // The id its program last said it used, so that a message that names it is said by the name above.
        described["model_reported"] = json!(app.agents.model_reported(&slot.name));
        // What its program sees of this device, as walls are held while the daemon runs.
        described["sees"] = json!(app.agents.sees(&slot));
        match brought.get(&slot.name) {
            Some(agents::Brought::By(by)) => described["held_by"] = json!(by.key),
            // One added by hand, with the text of its own it was applied as.
            Some(agents::Brought::Own(own)) => described["instance"] = json!(own.id),
            _ => {}
        }
        // Of one this factory does not start: whether it has joined, and
        // whether an invitation waits for it.
        if slot.external() {
            described["external"] = json!(app.agents.standing(&slot).await?);
        }
        slots.push(described);
    }
    Ok(Json(json!({
        "agents": slots,
        "adapters": app.agents.adapters(),
        "default_adapter": app.agents.default_adapter(),
        // The same, with what a person choosing between them needs to know.
        "providers": app.agents.providers(),
        "permissions": agents::PERMISSIONS.iter().map(|(name, means)| json!({ "name": name, "means": means })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct ModelsQuery {
    /// Ask the program again instead of answering from memory.
    #[serde(default)]
    fresh: bool,
}

/// The models one provider offers. For a package that asks its program this
/// can take a few seconds the first time, which is why it is its own read
/// and not part of the list of agents.
async fn provider_models(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Query(query): Query<ModelsQuery>,
) -> ApiResult<Json<Value>> {
    who.require("agents:read")?;
    // Reading never runs a program a second time: a page that is only
    // looked at must not be able to set one going with its reader's session.
    if query.fresh {
        return Err(Error::invalid("to have the program asked again, send this as `POST`").into());
    }
    // Asked for the first time, its program is run: that is done for
    // whoever may change agents, and read afterwards by whoever may see them.
    if !app.agents.knows_models(&name) {
        who.require("agents:write")?;
    }
    Ok(Json(json!({ "models": app.agents.models(&name, false).await? })))
}

/// Asks a provider's program again which models it offers.
async fn provider_models_again(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    who.require("agents:write")?;
    Ok(Json(json!({ "models": app.agents.models(&name, true).await? })))
}

fn extension_info(extension: &agents::Extension) -> Value {
    json!({
        "name": extension.name,
        "title": extension.title,
        "description": extension.description,
        "program": extension.program,
        "available": extension.available(),
        "models": extension.models_kind(),
        // What the package has to say to someone who does not have its program.
        "install": extension.install,
        // What it brings: an agent, actions, blueprints, or several of them.
        // An action is described as every action is (`/api/actions`).
        "agent": extension.agent,
        "actions": extension.actions.values().map(|action| &action.spec).collect::<Vec<_>>(),
        "blueprints": extension.blueprints.keys().collect::<Vec<_>>(),
    })
}

async fn extensions(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("agents:read")?;
    // Which of them are not the ones of their names that came with this
    // installation: installed as a copy, each says what it said then.
    // Where each differs is said with it; which of the two is the earlier
    // is not known, and is not said.
    let behind = app.agents.behind_by();
    let installed: Vec<Value> = app
        .agents
        .extensions()
        .iter()
        .map(|extension| {
            let mut info = extension_info(extension);
            let differs = behind.get(&extension.name);
            info["behind"] = json!(differs.is_some());
            if let Some(differs) = differs {
                info["differs"] = differs.to_json();
            }
            info
        })
        .collect();
    // What came with the product and could be installed with one request.
    let offered: Vec<Value> =
        app.agents.offered().iter().map(|extension| extension_info(extension)).collect();
    Ok(Json(json!({ "extensions": installed, "offered": offered })))
}

/// Installs a package: one that came with the product, by `{ "name": ... }`,
/// or any other from its manifest, `{ "source": "<the file's text>" }`.
async fn extension_add(
    State(app): State<App>,
    Auth(who): Auth,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let extension = match text(&body, "name") {
        Some(name) => app.agents.add_offered(name, &who).await?,
        None => app.agents.add_extension(required(&body, "source")?, &who).await?,
    };
    Ok(Json(extension_info(&extension)))
}

/// Takes the package that came with this installation in place of the
/// installed copy of that name: `updated` says whether anything was
/// replaced, and `differs` where the one that came differed from the copy.
/// It is asked, and never done unasked.
async fn extension_update(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    let taken = app.agents.take_what_came(&name, &who).await?;
    let mut info = extension_info(&taken.extension);
    info["updated"] = json!(taken.differs.is_some());
    if let Some(differs) = &taken.differs {
        info["differs"] = differs.to_json();
    }
    Ok(Json(info))
}

async fn extension_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    app.agents.remove_extension(&name, &who).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn agent_add(
    State(app): State<App>,
    Auth(who): Auth,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let spec = SlotSpec {
        name: required(&body, "name")?.to_string(),
        adapter: text(&body, "adapter").map(str::to_string),
        model: text(&body, "model").map(str::to_string),
        project: text(&body, "project").map(str::to_string),
        channel: text(&body, "channel").map(str::to_string),
        instructions: text(&body, "instructions").map(str::to_string),
        scopes: strings(&body, "scopes"),
        permission_mode: text(&body, "permission_mode").map(str::to_string),
        signins: strings(&body, "signins"),
        role: body.get("role").and_then(Value::as_str).map(str::to_string),
        rank: body.get("rank").and_then(Value::as_i64),
        wall: agents::walled::said(body.get("wall"))?,
        reads: strings(&body, "reads"),
        changes: strings(&body, "changes"),
        git: text(&body, "git").map(str::to_string),
        secrets: strings(&body, "secrets"),
        outlives: agents::running::said(body.get("outlives"))?,
        web: agents::running::said_of_the_web(body.get("web"))?,
        environment: text(&body, "environment").map(str::to_string),
    };
    let more = crate::byhand::more_of(&body);
    let before = app.agents.find(&blueprint::expr::slug(&spec.name)).await?;
    let slot = crate::byhand::add(&app.engine, &app.agents, spec, more, &who).await?;
    Ok(Json(seen(&app, crate::byhand::answered(before.as_ref(), &slot), &slot)))
}

/// Changes what an agent holds, whole, the project it works in, or both,
/// and nothing else of it: one that is stopped stays stopped. Whoever
/// asks gives nothing they do not hold. The answer says what the change
/// comes to for the work it has in hand (`said`).
async fn agent_give(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // What it holds, the project it works in, or both, the first first.
    let before = app.agents.find(&blueprint::expr::slug(&name)).await?;
    let project = match body.get("project") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(project)) => Some(Some(project.clone())),
        Some(_) => {
            return Err(
                Error::invalid("`project` is the project it is to work in, or `null` for none").into()
            );
        }
    };
    let scopes = strings(&body, "scopes");
    let beside =
        body.as_object().is_some_and(|said| said.keys().any(|key| key != "scopes" && key != "project"));
    if (scopes.is_none() && project.is_none()) || beside {
        return Err(Error::invalid(
            "an agent is changed here by `scopes`, what it is to hold, all of it, as a list; by `project`, the \
             project it is to work in, or `null` for none; or by both; nothing else of it is changed",
        )
        .into());
    }
    let mut slot = None;
    if let Some(scopes) = scopes {
        slot = Some(crate::byhand::give(&app.engine, &app.agents, &name, scopes, &who).await?);
    }
    if let Some(project) = project {
        slot = Some(crate::byhand::move_to(&app.engine, &app.agents, &name, project, &who).await?);
    }
    let slot = slot.expect("one of the two was said");
    Ok(Json(seen(&app, crate::byhand::answered(before.as_ref(), &slot), &slot)))
}

#[derive(Deserialize)]
struct StopQuery {
    /// Let it finish what it has in hand first.
    #[serde(default)]
    idle: bool,
    /// Only if it is this project's: asked by the factory's bot, of what
    /// it is told in that project's channel.
    within: Option<String>,
}

/// Stops an agent: at once, with what it was doing, or with `?idle=true`
/// when it has finished what it has in hand.
async fn agent_stop(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Query(how): Query<StopQuery>,
) -> ApiResult<Json<Value>> {
    who.require("agents:write")?;
    if let Some(project) = &how.within
        && app.agents.slot(&name).await?.project.as_deref() != Some(project.as_str())
    {
        return Err(Error::invalid(format!(
            "`{name}` is not one of `{project}`'s agents, and only that project's were asked for"
        ))
        .into());
    }
    let leaving = if how.idle {
        !app.agents.leave(&name).await?
    } else {
        app.agents.deactivate(&name).await?;
        false
    };
    Ok(Json(json!({ "ok": true, "leaving": leaving })))
}

/// What taking an agent away would come to, asked before it is done:
/// what goes with it, and why it cannot be as things stand. The same
/// people may ask as may do it.
async fn agent_removal(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.agents.remove(&name, &who, &app.engine, true).await?)))
}

/// Takes an agent away for good (`agents::removed`): a person's to do,
/// refused for one that something running counts and for one with work
/// in hand. `DELETE` of an agent stops it and keeps it, as it always did.
async fn agent_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(json!(app.agents.remove(&name, &who, &app.engine, false).await?)))
}

/// Starts an agent's program as a turn of its would, and asks it what it
/// offers: whether it works where it will work, without giving it any.
/// With a `channel` kept to its members, as a turn asked there would
/// start it: in the home it has for that channel.
async fn agent_try(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    body: Option<Json<Value>>,
) -> ApiResult<Json<Value>> {
    let body = body.map(|Json(body)| body).unwrap_or_default();
    Ok(Json(json!(app.agents.try_program_in(&name, text(&body, "channel"), &who).await?)))
}

async fn agent_turns(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    who.require("agents:read")?;
    // What an agent was asked holds what was said where it listens, and
    // what it answered was said there: both are the channels' to show.
    who.require("channels:read")?;
    // One asked in a channel kept to its members is shown whole to whoever
    // reads that channel, and to anybody else without its words.
    Ok(Json(json!({ "turns": app.agents.turns_for(&name, 50, &who).await? })))
}

/// An agent as it is answered, with what its program sees of this device as walls are held now.
fn seen(app: &App, mut answer: Value, slot: &agents::Slot) -> Value {
    answer["sees"] = json!(app.agents.sees(slot));
    answer
}
