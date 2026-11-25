//! An event said from outside.
//!
//! A handler reacts to an event, and until now every event was said by
//! something inside: a step, an action, the program itself. What happens
//! elsewhere (a payment taken, a build finished on someone else's machine,
//! a form sent) had no way in. This is the way in: an event of a
//! blueprint's own topic, said by whoever holds a key for that topic.
//!
//! The authority is the key and nothing else. `emit:<topic>` lets its
//! bearer say that topic; `emit` any topic a blueprint may say. What the
//! system says of itself (`instance.failed`, `board.item.moved`) is its own
//! to say, and is refused whoever asks.
//!
//! Two addresses, one rule. `POST /api/emit/<topic>` is asked as every
//! other route is, with a credential in a header. `POST /in/<key>/<topic>`
//! is for what can only be given an address to call. There the key is in
//! the address, and an address is written down wherever it passes (a
//! service's own records, a proxy's log), so a key is taken there only if
//! saying events is all it can do: one found in a log lets its finder say
//! those topics, and nothing more.
//!
//! A third, for a service that signs what it sends: `POST /in/<caller>`.
//! There is no key in that address at all. The service is a caller the
//! owner described (`crate::callers`), and each call is taken only if it
//! was signed with the secret the factory keeps for it.
//!
//! Whichever way it came, the event says who said it: a person
//! (`user:<id>`), the key it was said with (`key:<id>`), or the caller that
//! signed it (`caller:<name>`). A handler that must know whose word it is
//! acting on reads `event.source.actor`, which nothing a sender writes can
//! set.

use std::time::Instant;

use axum::body::Bytes;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::HeaderMap;
use axum::http::request::Parts;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use blueprint::RESERVED_TOPICS;
use blueprint::validate::is_topic;
use kernel::{Error, Principal, Source};
use serde_json::{Value, json};

use crate::callers::{Caller, KEPT_AS};
use crate::http::{ApiError, ApiResult};
use crate::state::{App, Auth, token_of};

/// The most an event said from outside may carry.
const MOST: usize = 256 * 1024;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/emit/{topic}", post(emit))
        .route("/callers", get(callers))
        .route("/callers/{name}", put(caller_set).delete(caller_remove))
}

/// The addresses that are called from outside; beside `/api`, since what
/// calls one is told one address and no more.
pub fn ways_in() -> Router<App> {
    Router::new().route("/in/{key}/{topic}", post(emit_by_address)).route("/in/{caller}", post(called))
}

/// Whoever is saying something, and what the event will say said it.
struct Sayer {
    who: Principal,
    actor: String,
}

impl Sayer {
    async fn holding(app: &App, token: &str) -> Result<Sayer, Error> {
        let (who, key) = app.studio.identity.known(token).await?;
        let actor = match key {
            Some(id) => format!("key:{id}"),
            None => who.actor(),
        };
        Ok(Sayer { who, actor })
    }
}

impl FromRequestParts<App> for Sayer {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &App) -> Result<Self, Self::Rejection> {
        // Said inside the daemon, for somebody whose message asked for it:
        // by them, as the message was.
        if let Some(within) = parts.extensions.get::<crate::commands::Within>() {
            return Ok(Sayer { actor: within.0.actor(), who: within.0.clone() });
        }
        // Held to how the token came, as every other request is: a
        // browser's cookie by itself says nothing.
        crate::state::who(parts, app).await?;
        let token =
            token_of(parts, &app.place.cookie).ok_or_else(|| Error::unauthenticated("sign in first"))?;
        Ok(Sayer::holding(app, &token).await?)
    }
}

async fn emit(
    State(app): State<App>,
    sayer: Sayer,
    Path(topic): Path<String>,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    say(&app, &sayer, &topic, &body).await
}

async fn emit_by_address(
    State(app): State<App>,
    Path((key, topic)): Path<(String, String)>,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let sayer = Sayer::holding(&app, &key).await?;
    let scopes = &sayer.who.scopes;
    let only_says = scopes.iter().all(|scope| scope == "emit" || scope.starts_with("emit:"));
    if !only_says || scopes.is_empty() {
        return Err(Error::denied(
            "the key in this address holds more than saying events, and an address is written down wherever it \
             passes. Make one that holds only `emit:<topic>` for this, and send any other in a header",
        )
        .into());
    }
    say(&app, &sayer, &topic, &body).await
}

/// Refuses more than an event said from outside may carry.
fn sized(body: &[u8]) -> Result<(), Error> {
    match body.len() > MOST {
        true => Err(Error::invalid(format!(
            "that is {} bytes, and an event said from outside carries at most {MOST}",
            body.len()
        ))),
        false => Ok(()),
    }
}

/// What an event said from outside carries: the mapping `body` is.
fn carried(body: &[u8]) -> Result<Value, Error> {
    sized(body)?;
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(json!({}));
    }
    match serde_json::from_slice::<Value>(body) {
        Ok(data @ Value::Object(_)) => Ok(data),
        _ => Err(Error::invalid(
            "what an event carries is a mapping, written as JSON: `{ \"order\": \"o-17\" }`",
        )),
    }
}

/// Says `topic` for `sayer`, carrying what `body` is.
async fn say(app: &App, sayer: &Sayer, topic: &str, body: &[u8]) -> ApiResult<Json<Value>> {
    if !is_topic(topic) {
        return Err(
            Error::invalid(format!("`{topic}` is not a topic; use lowercase words joined by dots")).into()
        );
    }
    if RESERVED_TOPICS.contains(&topic.split('.').next().unwrap_or_default()) {
        return Err(Error::invalid(format!(
            "`{topic}` is in a namespace only the system may publish under; say a topic of your blueprint's own"
        ))
        .into());
    }
    sayer.who.require(&format!("emit:{topic}"))?;
    let data = carried(body)?;
    let event = app.journal.emit(topic, Source::actor(&sayer.actor), data).await?;
    Ok(Json(json!({ "topic": topic, "seq": event.seq })))
}

async fn caller_of(app: &App, name: &str) -> Result<Caller, Error> {
    let kept = app.journal.kv_get(KEPT_AS, name).await?;
    kept.and_then(|kept| serde_json::from_value(kept).ok()).ok_or_else(|| {
        Error::not_found(format!(
            "there is no caller called `{name}`; whoever owns the factory describes one with `{} caller set {name}`",
            brand::CLI
        ))
    })
}

/// A call from a service that signs what it sends.
async fn called(
    State(app): State<App>,
    Path(name): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let began = Instant::now();
    let taken = take(&app, &name, &headers, &body).await;
    // Each caller's calls are counted of their own, taken and refused: a
    // service whose calls have begun to be refused is seen there.
    let took = began.elapsed().as_secs_f64() * 1000.0;
    app.journal.measure("caller.call", &[("caller", name.as_str())], took, taken.is_ok());
    Ok(Json(taken?))
}

async fn take(app: &App, name: &str, headers: &HeaderMap, body: &[u8]) -> Result<Value, Error> {
    let caller = caller_of(app, name).await?;
    // Before the signature is worked out over it: how much there is. What
    // it says is read only once it is known whose it is.
    sized(body)?;
    let secret = app.hosting.secrets()?.get(&caller.secret)?;
    let now = app.journal.now_ms();
    let proved = caller
        .proves(&secret, |name| headers.get(name).and_then(|value| value.to_str().ok()), body, now)
        .map_err(|why| Error::unauthenticated(format!("caller `{name}`: {why}")))?;
    // Held from here until the call is said: sent twice at once, it is
    // still said once.
    let mut seen = app.seen.lock().await;
    if proved.at.is_some()
        && let Some(seq) = seen.before(name, &proved.signature, now)
    {
        return Ok(json!({ "topic": caller.topic, "seq": seq, "again": true }));
    }
    let data = carried(body)?;
    let event = app.journal.emit(&caller.topic, Source::actor(format!("caller:{name}")), data).await?;
    if let Some(at) = proved.at {
        seen.took(name, proved.signature, at + caller.within_ms().unwrap_or_default(), event.seq);
    }
    Ok(json!({ "topic": caller.topic, "seq": event.seq }))
}

async fn callers(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    let callers: Vec<Value> = app
        .journal
        .kv_list(KEPT_AS)
        .await?
        .into_iter()
        .map(|(name, mut caller)| {
            caller["name"] = json!(name);
            caller
        })
        .collect();
    Ok(Json(json!({ "callers": callers })))
}

/// Describes a caller, or describes one again. Who may be let in from
/// outside, and on the strength of which secret, is the owner's to say.
async fn caller_set(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    if let Some(why) = crate::callers::misnamed(&name) {
        return Err(Error::invalid(why).into());
    }
    let caller: Caller = serde_json::from_value(body).map_err(|err| {
        Error::invalid(format!(
            "that does not describe a caller ({err}). One says `topic`, `secret` and `header`, and where they \
             differ from the usual `prefix`, `encoding`, `key`, `signed`, `time_header`, `time_prefix` and `within`"
        ))
    })?;
    if let Some(why) = caller.refused() {
        return Err(Error::invalid(why).into());
    }
    // It cannot check anything without its secret: said now, to whoever is
    // setting it up, and not to the service on its first call.
    app.hosting.secrets()?.get(&caller.secret)?;
    app.journal.kv_set(KEPT_AS, &name, &json!(caller)).await?;
    app.seen.lock().await.forget(&name);
    let said = json!({ "name": name, "topic": caller.topic, "by_name": who.name });
    app.journal.emit("caller.set", Source::actor(who.actor()), said).await?;
    let mut answer = json!(caller);
    answer["name"] = json!(name);
    answer["address"] = json!(format!("/in/{name}"));
    Ok(Json(answer))
}

async fn caller_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    who.require("admin")?;
    let caller = caller_of(&app, &name).await?;
    app.journal.kv_set(KEPT_AS, &name, &Value::Null).await?;
    app.seen.lock().await.forget(&name);
    let said = json!({ "name": name, "topic": caller.topic, "by_name": who.name });
    app.journal.emit("caller.removed", Source::actor(who.actor()), said).await?;
    Ok(Json(json!({ "ok": true })))
}
