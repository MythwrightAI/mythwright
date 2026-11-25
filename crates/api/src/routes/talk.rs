//! Channels: what is said in them, what is attached there, and who is in
//! each.
//!
//! Every route here asks the channel itself before it reads or writes
//! anything of it (`studio::Channels::may_read`, `may_write`,
//! `may_find`, or a method that takes whoever asks): a channel kept to
//! its members answers whoever is not one of them as a channel that is
//! not there does, after the scope has refused whoever lacks it.

use agents::SlotSpec;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use conductor::{InstanceFilter, InstanceInfo};
use kernel::Error;
use serde::Deserialize;
use serde_json::{Value, json};

use super::collab::strings;
use super::{required, text};
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/channels", get(channels).post(channel_create))
        .route("/channels/{id}", get(channel))
        .route("/channels/{id}", delete(channel_remove))
        // Called something else, said what it is for, and who manages it (`routes/renamed.rs`).
        .route("/channels/{id}", patch(super::renamed::rename))
        .route("/channels/{id}/managers", post(super::renamed::managers))
        .route("/channels/{id}/messages", get(messages).post(message_post))
        // What a channel offers its agents of discussions (`talk::discussed`).
        .route("/channels/{id}/discussing", get(discussing))
        .route("/channels/{id}/discussions/{root}/end", post(discussion_end))
        // What is applied for a channel, as the Channel tab of the Tools panel lists it.
        .route("/channels/{id}/running", get(running))
        // A file is bigger than anything else that is sent here.
        .route(
            "/channels/{id}/attachments",
            post(attachment_add)
                .layer(DefaultBodyLimit::max(studio::attachments::MAX_ATTACHMENT_BYTES + 4096)),
        )
        .route("/attachments/{id}", get(attachment))
        .route("/attachments/{id}/view", get(attachment_view))
        .route("/channels/{id}/agents", post(channel_bring_in))
        .route("/channels/{id}/members", post(channel_add_person))
        .merge(super::kept::routes())
        .merge(super::archived::routes())
        .route("/people", get(people))
}

async fn channels(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    Ok(Json(json!({ "channels": app.studio.channels.readable(&who).await? })))
}

async fn channel_create(
    State(app): State<App>,
    Auth(who): Auth,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // Read before anything is made: a channel that names a workspace or
    // a project there is not, or a project and a workspace both, is not
    // made. With neither said it is the usual workspace's own: a channel
    // is a project's or a workspace's, and is written with which.
    let workspace = match super::workspaces::belonging(&body)? {
        studio::Belongs::Workspace(workspace) => {
            who.require("channels:write")?;
            Some(app.studio.workspaces.get(workspace).await?.id)
        }
        studio::Belongs::Project(project) => {
            who.require("channels:write")?;
            app.studio.projects.get(project).await?;
            None
        }
        studio::Belongs::Nothing => None,
    };
    let channels = match &workspace {
        Some(workspace) => app.studio.channels.asked_in(workspace),
        None => app.studio.channels.clone(),
    };
    let (name, topic) = (required(&body, "name")?, text(&body, "topic").unwrap_or_default());
    // Kept to its members, and whether its agents are asked nowhere
    // else, which is said of a kept one.
    let (kept, alone) = (super::kept::flag(&body, "kept")?, super::kept::flag(&body, "alone")?);
    // Who is in it from the first, beside whoever makes it: the people
    // in the write that makes it, and each agent brought in after, which
    // is asked before anything is made whether it could be.
    let (people, agents) = super::kept::members(&app, &who, &body).await?;
    app.agents.could_be_in(&agents, kept, alone, &who).await?;
    let asked =
        studio::door::Asked { name, project: text(&body, "project"), topic, kept, alone, members: &people };
    let channel = channels.create_as(asked, &who).await?;
    if agents.is_empty() {
        return Ok(Json(json!(channel)));
    }
    for agent in &agents {
        app.agents.bring_in(&channel.id, agent, &who).await.map_err(|refused| {
            let made = studio::door::called(&channel);
            Error::conflict(format!(
                "{made} was made, and `{agent}` was not brought into it: {}",
                refused.message
            ))
        })?;
    }
    let reader = app.studio.channels.may_find(&channel.id, &who).await?;
    Ok(Json(json!(app.studio.channels.get(&reader, &channel.id).await?)))
}

async fn channel(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let reader = app.studio.channels.may_read(&id, &who).await?;
    Ok(Json(json!(app.studio.channels.get(&reader, &id).await?)))
}

#[derive(Deserialize)]
struct MessagesQuery {
    before: Option<String>,
    limit: Option<usize>,
    /// A message: read the thread it is part of instead of the channel.
    thread: Option<String>,
}

async fn messages(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Query(query): Query<MessagesQuery>,
) -> ApiResult<Json<Value>> {
    // A thread is read in the channel of the path: a message of another
    // channel names nothing here.
    let (channels, reader) = (&app.studio.channels, app.studio.channels.may_read(&id, &who).await?);
    let mut messages = match &query.thread {
        Some(message) => channels.thread(&reader, message).await?,
        None => channels.messages(&reader, &id, query.before.as_deref(), query.limit.unwrap_or(100)).await?,
    };
    // A file of a project something names is read as whoever reads the message may read it.
    super::posted::seen_all(&app, &who, &mut messages).await;
    Ok(Json(json!({ "messages": messages })))
}

#[derive(Deserialize)]
struct RemoveQuery {
    /// Whatever says it posts there.
    #[serde(default)]
    force: bool,
}

/// Takes a channel away into the Trash, with what was said in it. The
/// owner's to do, and refused while something that is running says it
/// posts there, unless told to all the same: its next word would be said
/// to nothing.
async fn channel_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Query(how): Query<RemoveQuery>,
) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    // Found as whoever asks finds it: the owner's scope is asked, and of
    // a channel kept to its members whether they read it.
    let found = app.studio.channels.may_find(&id, &who).await?;
    let channel = app.studio.channels.get(&found, &id).await?;
    if !how.force {
        let filter = InstanceFilter { live_only: true, limit: 10_000, ..InstanceFilter::default() };
        let live = app.engine.instances(filter).await?;
        let posting = posts_in(&channel, &live);
        if let Some(first) = posting.first() {
            let named: Vec<String> =
                posting.iter().map(|one| format!("`{}` ({}, {})", one.key, one.blueprint, one.id)).collect();
            return Err(Error::conflict(format!(
                "#{} is where {} {}, and {} running. Stop it first (`{cli} stop {}`), or remove the channel all the \
                 same with `--force`: what it says there next will fail",
                channel.name,
                named.join(" and "),
                if posting.len() == 1 { "posts" } else { "post" },
                if posting.len() == 1 { "it is" } else { "they are" },
                first.id,
                cli = brand::CLI,
            ))
            .into());
        }
    }
    // Into the Trash, which is the one door: or for good where the
    // factory keeps nothing that is deleted.
    let (gone, trash) = crate::trash::of(&app).delete_channel(&channel.id, &who).await?;
    // What agents were asked there was kept with what had been said there.
    app.agents.forget_channel(&gone.id).await?;
    app.studio.workspaces.forgot_channel(&gone.id).await?;
    let mut answer = crate::trash::moved(&app, trash).await;
    answer["id"] = json!(gone.id);
    answer["name"] = json!(gone.name);
    if gone.kept {
        answer["title"] = json!(gone.title);
    }
    Ok(Json(answer))
}

/// What is running for a channel: the instances that say they post
/// there, as [`posts_in`] finds them, each as an instance is shown. A
/// list of every live instance is bounded (`/instances`), and which of
/// them post in a channel is a rule of the daemon's, so it is said here
/// once and not worked out by a page from part of the list. For whoever
/// reads the channel and may read what is applied.
async fn running(State(app): State<App>, Auth(who): Auth, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let channels = &app.studio.channels;
    let reader = channels.may_read(&id, &who).await?;
    who.require("blueprints:read")?;
    let channel = channels.get(&reader, &id).await?;
    let filter = InstanceFilter { live_only: true, limit: 10_000, ..InstanceFilter::default() };
    let live = app.engine.instances(filter).await?;
    let posting: Vec<InstanceInfo> = posts_in(&channel, &live).into_iter().cloned().collect();
    Ok(Json(json!({ "channel": channel.id, "instances": app.engine.shown(posting) })))
}

/// The instances that say they post in `channel`: each has a param or a
/// word of state called `channel` that names it. Of an instance and what
/// it started, the one at the top: stopping that stops the rest.
pub(super) fn posts_in<'a>(channel: &studio::Channel, live: &'a [InstanceInfo]) -> Vec<&'a InstanceInfo> {
    let names = |said: Option<&Value>| {
        said.and_then(Value::as_str).is_some_and(|said| {
            let said = said.trim_start_matches('#');
            said == channel.id || said == channel.name
        })
    };
    let posts = |one: &InstanceInfo| names(one.params.get("channel")) || names(one.state.get("channel"));
    let found: Vec<&InstanceInfo> = live.iter().filter(|one| posts(one)).collect();
    let under = |one: &InstanceInfo| {
        one.parent.as_ref().is_some_and(|parent| found.iter().any(|other| &other.id == parent))
    };
    found.iter().copied().filter(|one| !under(one)).collect()
}

async fn message_post(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // What goes with it was put here first, and is named by id. With
    // something attached, a message need say nothing.
    let attached = strings(&body, "attachments").unwrap_or_default();
    // What an agent posts for itself while it is answering somebody in a
    // thread of this channel is said in that thread, as its answer will
    // be, unless it says a thread or says the channel itself (`top`): a
    // page it made for whoever asked is theirs to find where they asked.
    // Anybody else, an agent with nothing in hand here, and one asked in
    // the channel itself, post where they say, as they always did.
    let top = body.get("top").and_then(Value::as_bool).unwrap_or(false);
    let reader = app.studio.channels.may_write(&id, &who).await?;
    let asked = match (&who.kind, text(&body, "thread")) {
        (kernel::PrincipalKind::Agent, None) if !top => app.agents.asked_under(&reader, &who.id, &id).await,
        _ => None,
    };
    // An agent with a turn in hand here says what wrote it as its answer will.
    let model = match who.kind {
        kernel::PrincipalKind::Agent => app.agents.model_in_hand(&reader, &who.id, &id).await,
        _ => None,
    };
    let place = studio::Place {
        reply_to: text(&body, "reply_to"),
        thread: text(&body, "thread").or(asked.as_deref()),
        model: model.as_deref(),
        attachments: &attached,
        // What its author marks it as: a plan, something to discuss.
        mark: text(&body, "mark"),
        ..studio::Place::default()
    };
    let said = if attached.is_empty() { required(&body, "body")? } else { text(&body, "body").unwrap_or("") };
    let message = app.studio.channels.post_at(&id, &who, said, place).await?;
    // What a file of a project it names is now, as whoever posted it may read it.
    let mut answered = [message];
    super::posted::seen_all(&app, &who, &mut answered).await;
    Ok(Json(json!(answered[0])))
}

/// What a channel offers its agents of discussions, as whoever reads it
/// may see: nothing where it offers none.
async fn discussing(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let channels = &app.studio.channels;
    let reader = channels.may_read(&id, &who).await?;
    let channel = channels.get(&reader, &id).await?;
    let offered = channels.discussing(&channel.id);
    let open = channels.discussions_in(&reader, &channel.id).await?;
    let open: Vec<Value> = open
        .iter()
        .map(|one| json!({ "root": one.root, "status": one.status, "stretch": one.stretch }))
        .collect();
    Ok(Json(json!({
        "channel": channel.id,
        "settings": offered.as_ref().map(|settings| settings.said()),
        "taking": offered.map(|settings| settings.taking()).unwrap_or_default(),
        "discussions": open,
    })))
}

/// Ends a discussion of a channel, going or waiting for a person, as
/// whoever writes there: what its stretch has in hand is called off, as a
/// stop calls it off, once it is said to have been ended by them.
async fn discussion_end(
    State(app): State<App>,
    Auth(who): Auth,
    Path((id, root)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let ended = app.studio.channels.end_discussion(&who, &id, &root).await?;
    let filter = InstanceFilter {
        live_only: true,
        blueprint: Some("std/discussion".into()),
        limit: 100_000,
        ..Default::default()
    };
    let stretches = app.engine.instances(filter).await?;
    for stretch in stretches.iter().filter(|one| one.params["root"] == json!(root)) {
        app.engine.stop(&stretch.id).await?;
    }
    Ok(Json(
        json!({ "root": ended.root, "status": ended.status, "turns": ended.turns, "stretch": ended.stretch }),
    ))
}

#[derive(Deserialize)]
struct AttachQuery {
    name: String,
    /// Where whoever attaches it has it, as their system has it whole: a
    /// file of a project there is posted as that file (`posted`).
    from: Option<String>,
}

/// Keeps a file to go with a message that is about to be posted in this
/// channel. The body is the file as it is; what kind of thing it is, is
/// what the request says it is, or else what its name says.
async fn attachment_add(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Query(query): Query<AttachQuery>,
    headers: axum::http::HeaderMap,
    bytes: axum::body::Bytes,
) -> ApiResult<Json<Value>> {
    let channels = &app.studio.channels;
    let reader = channels.may_write(&id, &who).await?;
    let channel = channels.get(&reader, &id).await?;
    let media = headers.get(axum::http::header::CONTENT_TYPE).and_then(|value| value.to_str().ok());
    let file = match &query.from {
        Some(from) => super::posted::recognised(&app, &who, &channel, from, &bytes).await,
        None => None,
    };
    if let Some((project, path)) = file {
        who.require("channels:write")?;
        let mut kept =
            channels.attachments.add_file(&reader, &channel.id, &who.actor(), &project, &path).await?;
        super::posted::seen(&app, &who, &mut kept).await;
        return Ok(Json(json!(kept)));
    }
    let kept =
        channels.attachments.add(&reader, &channel.id, &who, &query.name, media, bytes.to_vec()).await?;
    Ok(Json(json!(kept)))
}

/// What a view is held to, wherever it is opened: it runs in a box with no
/// origin of its own, so it has nobody's session and can read nothing of
/// the client; it may run its own scripts and those that are served beside
/// the client; and it reaches nothing over the network.
const VIEW_POLICY: &str = "sandbox allow-scripts; default-src 'none'; script-src 'unsafe-inline' 'self'; \
                           style-src 'unsafe-inline'; img-src data: blob:; media-src data: blob:; \
                           font-src data:; form-action 'none'; base-uri 'none'";

/// Put ahead of a view's own page. It gives the page the reader's colours
/// as they are told to it (`--text`, `--accent` and the rest, and a
/// `theme` event when they change), a plain starting look, and tells the
/// channel how tall the page is so that it is given that much room. It
/// can say nothing to the channel but that. It asks for the colours only
/// once the page's own scripts have run, so that one listening for them
/// is listening by the time they come.
const VIEW_START: &str = r#"<!doctype html><meta charset="utf-8">
<style>html{color-scheme:light dark}body{margin:0;padding:12px;font:13px/1.5 system-ui,sans-serif;color:var(--text,CanvasText);background:transparent}</style>
<script>(()=>{const id="__ID__",root=document.documentElement,say=(m)=>parent.postMessage(Object.assign({view:id},m),"*"),
size=()=>say({height:Math.ceil(root.getBoundingClientRect().height)});
addEventListener("message",(e)=>{const t=e.source===parent&&e.data&&e.data.theme;if(!t)return;
for(const k in t.colours)root.style.setProperty("--"+k,t.colours[k]);root.style.colorScheme=t.scheme;
window.theme=t;dispatchEvent(new CustomEvent("theme",{detail:t}))});
addEventListener("DOMContentLoaded",()=>{new ResizeObserver(size).observe(root);size();say({ready:true})});
addEventListener("load",size)})();</script>
"#;

/// An attachment that is a page, as it is shown in a channel: its own
/// HTML, with a start put ahead of it, held to [`VIEW_POLICY`].
async fn attachment_view(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<axum::response::Response> {
    use axum::http::header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS};
    use axum::response::IntoResponse;
    let not_a_page =
        |name: &str| Error::invalid(format!("`{name}` is not a page, so there is nothing to show of it"));
    let gone = |name: &str| Error::not_found(format!("the file of `{name}` is no longer here"));
    // A page attached to a record (a topic's, put in its text) is shown as that record is read, by whoever may read it.
    let (id, page) = if studio::attaching::is_of_a_record(&id) {
        let (found, path) = app.studio.records.attachment(&id, &who).await?;
        if found.media != studio::attachments::VIEW {
            return Err(not_a_page(&found.name).into());
        }
        (found.id, tokio::fs::read(&path).await.map_err(|_| gone(&found.name))?)
    } else {
        let (found, path) = app.studio.channels.attached(&id, &who).await?;
        if found.media != studio::attachments::VIEW {
            return Err(not_a_page(&found.name).into());
        }
        let page = match found.file {
            Some(_) => super::posted::bytes_of(&app, &who, &found).await?,
            None => tokio::fs::read(&path).await.map_err(|_| gone(&found.name))?,
        };
        (found.id, page)
    };
    let body = format!("{}{}", VIEW_START.replace("__ID__", &id), String::from_utf8_lossy(&page));
    let headers = [
        (CONTENT_TYPE, "text/html; charset=utf-8"),
        (X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (CONTENT_SECURITY_POLICY, VIEW_POLICY),
        (CACHE_CONTROL, "private, max-age=3600"),
    ];
    Ok((headers, body).into_response())
}

/// What is shown in the page as it is: pictures a browser draws without
/// running anything. Everything else is handed over to be saved. An SVG is
/// drawn by an `<img>`, which runs nothing in it, and opened by itself is
/// held by the policy `file` sends (`sandbox`, nothing loaded): it is how a
/// diagram is shown in a topic.
const SHOWN: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/avif",
    "image/svg+xml",
    "application/pdf",
    "video/mp4",
    "video/webm",
    "text/plain",
];

/// The file of an attachment.
async fn attachment(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
) -> ApiResult<axum::response::Response> {
    // Something attached to a record is read as that record is, by whoever may read it.
    if studio::attaching::is_of_a_record(&id) {
        let (found, path) = app.studio.records.attachment(&id, &who).await?;
        let bytes = tokio::fs::read(&path).await;
        let bytes =
            bytes.map_err(|_| Error::not_found(format!("the file of `{}` is no longer here", found.name)))?;
        return Ok(file(&found.name, &found.media, bytes));
    }
    let (found, path) = app.studio.channels.attached(&id, &who).await?;
    // A file of a project is read as that project's file, as whoever asks may read it.
    let bytes = match found.file {
        Some(_) => super::posted::bytes_of(&app, &who, &found).await?,
        None => tokio::fs::read(&path)
            .await
            .map_err(|_| Error::not_found(format!("the file of `{}` is no longer here", found.name)))?,
    };
    Ok(file(&found.name, &found.media, bytes))
}

/// A file attached, as it is handed to whoever may read it: shown where a
/// browser draws it without running anything, and otherwise to be saved.
fn file(name: &str, media: &str, bytes: Vec<u8>) -> axum::response::Response {
    use axum::http::header::{
        CONTENT_DISPOSITION, CONTENT_SECURITY_POLICY, CONTENT_TYPE, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS,
    };
    use axum::response::IntoResponse;
    let shown = SHOWN.contains(&media);
    // A name is put in a header: only what cannot break out of it.
    let name: String =
        name.chars().map(|c| if c.is_ascii_graphic() && c != '"' && c != '\\' { c } else { '_' }).collect();
    let headers = [
        (CONTENT_TYPE, if shown { media.to_string() } else { "application/octet-stream".to_string() }),
        (X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
        (CONTENT_SECURITY_POLICY, "sandbox; default-src 'none'; style-src 'unsafe-inline'".to_string()),
        // Its address says which page asked (`by`): nothing it leads to is told where it was led from.
        (REFERRER_POLICY, "no-referrer".to_string()),
        (
            CONTENT_DISPOSITION,
            format!("{}; filename=\"{name}\"", if shown { "inline" } else { "attachment" }),
        ),
    ];
    (headers, bytes).into_response()
}

/// The people with an account on this installation: who can be added to a
/// channel. Names only; an address is its owner's and an administrator's.
async fn people(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("channels:read")?;
    let people: Vec<Value> = app
        .studio
        .identity
        .accounts()
        .await?
        .into_iter()
        .map(|account| json!({ "actor": account.principal().actor(), "name": account.name }))
        .collect();
    Ok(Json(json!({ "people": people })))
}

/// Adds somebody to a channel: a person who has an account here, or an
/// agent that is there (`agent:<its name>`), which is brought in as an
/// agent is brought into any channel.
async fn channel_add_person(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // Whoever may write in a channel adds to it: of one kept to its
    // members, one of them, and nobody adds themselves to one.
    let reader = app.studio.channels.may_write(&id, &who).await?;
    let actor = required(&body, "actor")?;
    match actor.strip_prefix("agent:") {
        Some(agent) => app.agents.bring_in(&id, agent, &who).await?,
        None => {
            let person = super::kept::person(&app, actor).await?;
            app.studio.channels.join(&reader, &id, actor, &person.name).await?;
        }
    }
    Ok(Json(json!(app.studio.channels.get(&reader, &id).await?)))
}

/// Brings an agent into a channel, creating its slot if it has none. This is
/// what "invite an agent" does in the client.
async fn channel_bring_in(
    State(app): State<App>,
    Auth(who): Auth,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    // An open channel is found by anybody who may change agents, as it
    // always was. One kept to its members is found by one of them, and
    // an agent is brought into it by one who may write there.
    let reader = app.studio.channels.may_find(&id, &who).await?;
    if !reader.writes() {
        who.require("channels:write")?;
    }
    let channel = app.studio.channels.get(&reader, &id).await?;
    let spec = SlotSpec {
        name: required(&body, "name")?.to_string(),
        adapter: text(&body, "adapter").map(str::to_string),
        model: text(&body, "model").map(str::to_string),
        project: text(&body, "project").map(str::to_string).or(channel.project.clone()),
        channel: Some(channel.id.clone()),
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
    // Brought in by what an agent is set up by, as one added by hand is.
    let slot =
        crate::byhand::add(&app.engine, &app.agents, spec, crate::byhand::more_of(&body), &who).await?;
    Ok(Json(json!({ "agent": slot, "channel": app.studio.channels.get(&reader, &channel.id).await? })))
}
