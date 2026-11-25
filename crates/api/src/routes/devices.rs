//! The other devices of a fleet: inviting and removing them on a hub,
//! joining and leaving on a member, and the link itself.
//!
//! The link's three paths take no credential of the usual kind, and are
//! not left open by that: what is sent to them is sealed for this hub by
//! a device proving its key, or it is refused (`link::sealed`).

use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::header::{CONTENT_TYPE, HOST, RETRY_AFTER};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use gateway::Shut;
use gateway::wire::{HELLO_PATH, LINK_PATH, REPORT_PATH};
use kernel::Error;
use serde_json::{Value, json};

use super::required;
use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    // The link's paths are named by the gateway, which is also what calls
    // them from the member's side.
    let under_api = |path: &'static str| path.strip_prefix("/api").unwrap_or(path);
    Router::new()
        .route("/devices", get(devices).post(device_invite))
        .route("/devices/{name}", delete(device_remove))
        .route("/gateway/join", post(join))
        .route("/gateway/leave", post(leave))
        .route(under_api(HELLO_PATH), post(hello))
        .route(under_api(LINK_PATH), post(link))
        .route(under_api(REPORT_PATH), post(report))
}

/// The fleet's members as this device knows them, and this device's own
/// link to a hub, if it has joined one. `id` is the short name of the key
/// this installation proves on a link: what a hub or a member sees of it.
async fn devices(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    who.require("fleet:read")?;
    Ok(Json(json!({
        "this": app.journal.device(),
        "id": app.gateway.id(),
        "devices": app.gateway.devices().await?,
        "hub": app.gateway.uplink(),
        // That the hub it had joined took this device out, where it did.
        "unjoined": app.gateway.unjoined().await,
    })))
}

/// Where this hub can be reached, as far as it can tell: the address
/// whoever is asking used, the address it listens on, and each name it has
/// been told is its own, at the port it listens on. An address that is
/// every address (`0.0.0.0`, `[::]`) is where it listens and nowhere it is
/// reached: it is left out, and where nothing else is known, this machine's
/// own address is said, as the daemon says where it is.
fn reached_at(app: &App, headers: &HeaderMap) -> Vec<String> {
    let asked = headers.get(HOST).and_then(|host| host.to_str().ok()).map(str::to_string);
    let mut places: Vec<String> = asked.into_iter().collect();
    places.push(app.addr.to_string());
    places.extend(app.names.told().into_iter().map(|name| format!("{name}:{}", app.addr.port())));
    let mut at: Vec<String> = Vec::new();
    for place in places.into_iter().filter(|place| !everywhere(place)) {
        let place = format!("http://{place}");
        if !at.contains(&place) {
            at.push(place);
        }
    }
    if at.is_empty() {
        at.push(folders::own_url(app.addr));
    }
    at
}

/// Whether a place is every address of the machine rather than one of them.
fn everywhere(place: &str) -> bool {
    place.parse::<SocketAddr>().is_ok_and(|addr| addr.ip().is_unspecified())
}

/// Whether a place is reached only from this machine.
fn only_here(place: &str) -> bool {
    let host = place.trim_start_matches("http://");
    match host.parse::<SocketAddr>() {
        Ok(addr) => addr.ip().is_loopback(),
        Err(_) => host.rsplit_once(':').map_or(host, |(name, _)| name).eq_ignore_ascii_case("localhost"),
    }
}

/// Makes an invitation for a device and returns it, once, with the
/// command to run on that device.
async fn device_invite(
    State(app): State<App>,
    Auth(who): Auth,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let name = required(&body, "name")?;
    let expires = match body.get("expires").and_then(Value::as_str).filter(|said| !said.trim().is_empty()) {
        Some(said) => Some(Duration::from_millis(blueprint::duration::parse(said).map_err(|why| {
            Error::invalid(format!("`{said}` is not how long an invitation works: {why}. Say it as 2h or 3d"))
        })?)),
        None => None,
    };
    // Said, where it is said; otherwise as well as this daemon can tell.
    let said: Vec<String> = match body.get("at") {
        Some(Value::String(place)) => vec![place.clone()],
        Some(Value::Array(places)) => {
            places.iter().filter_map(|place| place.as_str().map(str::to_string)).collect()
        }
        _ => Vec::new(),
    };
    let guessed = said.is_empty();
    let at = if guessed { reached_at(&app, &headers) } else { said };
    let local = guessed && at.iter().all(|place| only_here(place));
    let invited = app.gateway.invite(name, expires, at.clone(), &who).await?;
    Ok(Json(json!({
        "name": invited.name,
        "invitation": invited.invitation,
        "expires": invited.expires,
        "at": at,
        "join": format!("{} device join {}", brand::CLI, invited.invitation),
        "note": local.then(|| match app.addr.ip().is_unspecified() {
            false => format!(
                "this daemon only listens on {}, so the invitation says to reach it there: another device can once this one is started with `--listen` on an address it can see, or the invitation is made with `--at http://host:port`",
                app.addr
            ),
            true => format!(
                "this daemon listens on every address of this machine and was asked at {}, which another device cannot reach: make the invitation with `--at http://host:port`, saying an address of this machine another device can see",
                at.join(", ")
            ),
        }),
    })))
}

async fn device_remove(
    State(app): State<App>,
    Auth(who): Auth,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    app.gateway.remove(&name, &who).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Makes this device a member of the fleet whose hub made `invitation`.
async fn join(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let allow: Option<Vec<String>> = body
        .get("allow")
        .and_then(Value::as_array)
        .map(|scopes| scopes.iter().filter_map(|scope| scope.as_str().map(str::to_string)).collect());
    if body.get("token").is_some() || body.get("url").is_some() {
        return Err(Error::invalid(format!(
            "a device no longer joins by an address and a credential, which crossed in the clear. \
             On the hub, run `{0} device invite <name>`; here, `{0} device join <invitation>`",
            brand::CLI
        ))
        .into());
    }
    app.gateway.join(required(&body, "invitation")?, allow, &who).await?;
    Ok(Json(json!({ "hub": app.gateway.uplink() })))
}

async fn leave(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    app.gateway.leave(&who).await?;
    Ok(Json(json!({ "ok": true })))
}

/// On its way down: a member is asked to come back shortly rather than
/// find out by being cut off.
fn restarting() -> Response {
    let wait = [(RETRY_AFTER, "5")];
    let body = json!({ "error": { "kind": "unavailable", "message": "the hub is restarting" } });
    (StatusCode::SERVICE_UNAVAILABLE, wait, Json(body)).into_response()
}

fn sealed(bytes: Vec<u8>) -> Response {
    ([(CONTENT_TYPE, "application/octet-stream")], bytes).into_response()
}

/// A device's first message of the handshake; the answer is this hub's.
/// Whether the device is let in is said inside the answer, sealed for it.
async fn hello(
    State(app): State<App>,
    ConnectInfo(from): ConnectInfo<SocketAddr>,
    body: Bytes,
) -> ApiResult<Response> {
    if app.engine.is_halting() {
        return Ok(restarting());
    }
    match app.gateway.hello(&body, from.ip()).await {
        Ok(answer) => Ok(sealed(answer)),
        // Nothing can be sealed for whoever sent this, so it is told in
        // words; a device takes those as proof of nothing.
        Err(Shut::Unread(message)) => Err(Error::denied(message).into()),
        Err(Shut::Later { wait, message }) => {
            let seconds = [(RETRY_AFTER, wait.as_secs().max(1).to_string())];
            let body = json!({ "error": { "kind": "unavailable", "message": message } });
            Ok((StatusCode::TOO_MANY_REQUESTS, seconds, Json(body)).into_response())
        }
        Err(Shut::Failed(err)) => Err(err.into()),
    }
}

/// A member's link: the request stays open and commands are sent down it
/// as they arise, each a sealed frame on a line of its own. Closing it,
/// from either end, is the link going down.
async fn link(State(app): State<App>, body: Bytes) -> ApiResult<Response> {
    if app.engine.is_halting() {
        return Ok(restarting());
    }
    let stream = app.gateway.sealed_link(&body).await?;
    let events = futures_util::stream::unfold(stream, |mut stream| async move {
        let line = stream.next().await?;
        let frame: Result<SseEvent, Infallible> = Ok(SseEvent::default().data(line));
        Some((frame, stream))
    });
    let alive = KeepAlive::new().interval(Duration::from_secs(15)).text("still here");
    Ok(Sse::new(events).keep_alive(alive).into_response())
}

async fn report(State(app): State<App>, body: Bytes) -> ApiResult<Response> {
    Ok(sealed(app.gateway.sealed_report(&body).await?))
}
